//! A `claude` process driven over the stream-json protocol (see docs/PROTOCOL.md).

use crate::engine::Program;
use crate::i18n::{self, Lang};
use crate::job::{Job, JobUsage};
use crate::text::truncate;
use anyhow::{anyhow, Context, Result};
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

type Pending = Arc<Mutex<HashMap<String, oneshot::Sender<Result<Value, String>>>>>;

pub struct SpawnOpts {
    pub program: Program,
    pub cwd: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

pub struct ClaudeProcess {
    tx: Mutex<Option<mpsc::UnboundedSender<String>>>,
    pending: Pending,
    next_id: AtomicU64,
    kill_tx: Mutex<Option<oneshot::Sender<()>>>,
    exited: Arc<AtomicBool>,
    job: Option<Arc<Job>>,
}

impl ClaudeProcess {
    /// Spawns the process. `on_frame` receives every stdout frame except responses to our own
    /// control requests; `on_exit` runs once with the exit code and the tail of stderr.
    pub fn spawn<F, E>(opts: SpawnOpts, on_frame: F, on_exit: E) -> Result<Arc<Self>>
    where
        F: Fn(Value) + Send + Sync + 'static,
        E: FnOnce(Option<i32>, String) + Send + 'static,
    {
        let launch = opts.program.launch(&opts.cwd, &opts.args, &opts.env)?;
        let mut cmd = Command::new(&launch.program);
        cmd.args(&launch.args)
            .current_dir(&opts.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        for (k, v) in &launch.env {
            cmd.env(k, v);
        }
        #[cfg(windows)]
        cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
        crate::job::isolate(&mut cmd);
        crate::proc::no_inherited_fds(&mut cmd);

        let mut child = cmd
            .spawn()
            .with_context(|| cannot_start(i18n::ui(), &opts.program.describe()))?;
        let job = Job::for_child(&child).map(Arc::new);
        if job.is_none() {
            // Killing it would then leave what it started running.
            log::warn!(
                "claude (pid {:?}) is not in a job: its tree is not killed as a whole",
                child.id()
            );
        }
        let mut stdin = child.stdin.take().context("stdin")?;
        let stdout = child.stdout.take().context("stdout")?;
        let stderr = child.stderr.take().context("stderr")?;

        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        tokio::spawn(async move {
            while let Some(line) = rx.recv().await {
                if stdin.write_all(line.as_bytes()).await.is_err()
                    || stdin.write_all(b"\n").await.is_err()
                    || stdin.flush().await.is_err()
                {
                    break;
                }
            }
            // Dropping stdin closes the input: the CLI exits once the current turn is over.
        });

        // What the process said besides its frames, for its exit: its stderr, and the lines of its
        // stdout that are no frame (`wsl.exe` writes its own errors there). Lines are read as
        // bytes: one that is not UTF-8 must not stop the reading, or the process would block.
        let stderr_tail = Arc::new(Mutex::new(VecDeque::<String>::new()));
        {
            let tail = stderr_tail.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).split(b'\n');
                while let Ok(Some(line)) = lines.next_segment().await {
                    let line = text_line(&line);
                    log::debug!("claude stderr: {line}");
                    note(&tail, line);
                }
            });
        }
        let stdout_tail = stderr_tail.clone();

        let pending: Pending = Arc::default();
        let (kill_tx, mut kill_rx) = oneshot::channel::<()>();
        let exited = Arc::new(AtomicBool::new(false));
        let proc = Arc::new(Self {
            tx: Mutex::new(Some(tx)),
            pending: pending.clone(),
            next_id: AtomicU64::new(1),
            kill_tx: Mutex::new(Some(kill_tx)),
            exited: exited.clone(),
            job: job.clone(),
        });

        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).split(b'\n');
            loop {
                tokio::select! {
                    line = lines.next_segment() => match line {
                        Ok(Some(line)) => {
                            let line = text_line(&line);
                            if line.trim().is_empty() {
                                continue;
                            }
                            match crate::text::json_line(&line) {
                                Ok(frame) => {
                                    if frame["type"] == "control_response" {
                                        resolve(&pending, &frame["response"]);
                                    } else {
                                        on_frame(frame);
                                    }
                                }
                                Err(e) => {
                                    log::warn!("unparsable frame ({e}): {}", truncate(&line, 200));
                                    note(&stdout_tail, truncate(&line, 500));
                                }
                            }
                        }
                        _ => break,
                    },
                    _ = &mut kill_rx => {
                        let _ = child.start_kill();
                        break;
                    }
                }
            }
            let code = child.wait().await.ok().and_then(|s| s.code());
            exited.store(true, Ordering::Release);
            let tail = stderr_tail
                .lock()
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join("\n");
            // The exit is handled before pending requests fail, so a caller seeing a failed
            // request also sees the consequences of the exit (status, cleared session…).
            on_exit(code, tail);
            for (_, tx) in pending.lock().drain() {
                let _ = tx.send(Err("process exited".into()));
            }
            // Closing the job ends whatever the process left running.
            drop(job);
        });

        Ok(proc)
    }

    pub fn is_alive(&self) -> bool {
        !self.exited.load(Ordering::Acquire)
    }

    /// What the process and everything it started use.
    pub fn usage(&self) -> Option<JobUsage> {
        self.job.as_ref()?.usage()
    }

    /// How many processes of the tree run: those of its job, and at least one while the process
    /// itself has not been reaped.
    pub fn still_running(&self) -> u32 {
        let in_job = self.usage().map_or(0, |u| u.processes);
        in_job.max(u32::from(self.is_alive()))
    }

    /// The ids of the processes of its job (to say what keeps a folder).
    pub fn pids(&self) -> Vec<u32> {
        self.job.as_ref().map(|j| j.pids()).unwrap_or_default()
    }

    pub fn send(&self, frame: &Value) -> Result<()> {
        if !self.is_alive() {
            return Err(anyhow!("process exited"));
        }
        let guard = self.tx.lock();
        let tx = guard
            .as_ref()
            .ok_or_else(|| anyhow!("process input closed"))?;
        tx.send(frame.to_string())
            .map_err(|_| anyhow!("process input closed"))
    }

    pub async fn control(&self, request: Value, timeout: Duration) -> Result<Value> {
        let id = format!("ccm_{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let (tx, rx) = oneshot::channel();
        self.pending.lock().insert(id.clone(), tx);
        if let Err(e) =
            self.send(&json!({ "type": "control_request", "request_id": id, "request": request }))
        {
            self.pending.lock().remove(&id);
            return Err(e);
        }
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(v))) => Ok(v),
            Ok(Ok(Err(e))) => Err(anyhow!(e)),
            Ok(Err(_)) => Err(anyhow!("control request dropped")),
            Err(_) => {
                self.pending.lock().remove(&id);
                Err(anyhow!("control request timed out"))
            }
        }
    }

    /// Answers a control_request issued by the CLI.
    pub fn respond(&self, request_id: &str, response: Value) -> Result<()> {
        self.send(&json!({
            "type": "control_response",
            "response": { "subtype": "success", "request_id": request_id, "response": response }
        }))
    }

    pub fn respond_error(&self, request_id: &str, error: &str) -> Result<()> {
        self.send(&json!({
            "type": "control_response",
            "response": { "subtype": "error", "request_id": request_id, "error": error }
        }))
    }

    /// Closes stdin: the CLI finishes its current work and exits (session stays resumable).
    pub fn close_input(&self) {
        self.tx.lock().take();
    }

    /// Waits until the process is gone (its output read to the end and the process reaped), at
    /// most `within`; false when it still runs.
    pub async fn wait_exit(&self, within: Duration) -> bool {
        let started = std::time::Instant::now();
        while self.is_alive() {
            if started.elapsed() >= within {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        true
    }

    /// Waits until the process and everything it started are gone, at most `within`; false when
    /// something still runs. Under Windows, a process that lives keeps its folder (and the files in
    /// it) however the process that started it ended, and a killed tree takes a moment to go: the
    /// job must be empty before the folder is removed. (Not elsewhere: a folder can be removed
    /// while a program has it open, and `ps` is too heavy to ask every few milliseconds.)
    pub async fn wait_gone(&self, within: Duration) -> bool {
        let started = std::time::Instant::now();
        if !self.wait_exit(within).await {
            return false;
        }
        #[cfg(windows)]
        while self.usage().is_some_and(|u| u.processes > 0) {
            if started.elapsed() >= within {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        #[cfg(not(windows))]
        let _ = started;
        true
    }

    /// Kills the process and its whole tree.
    pub fn kill(&self) {
        self.tx.lock().take();
        if let Some(job) = &self.job {
            job.terminate();
        }
        if let Some(k) = self.kill_tx.lock().take() {
            let _ = k.send(());
        }
    }
}

fn resolve(pending: &Pending, response: &Value) {
    let Some(id) = response["request_id"].as_str() else {
        return;
    };
    let Some(tx) = pending.lock().remove(id) else {
        return;
    };
    let result = if response["subtype"] == "success" {
        Ok(response
            .get("response")
            .cloned()
            .unwrap_or_else(|| json!({})))
    } else {
        Err(response["error"].as_str().unwrap_or("error").to_string())
    };
    let _ = tx.send(result);
}

/// A line a process wrote, as text: invalid UTF-8 replaced, without its `\r` nor the NULs of a
/// message written in UTF-16.
fn text_line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim_end_matches('\r')
        .replace('\0', "")
}

/// Keeps `line` among the last 30 of `tail`.
fn note(tail: &Mutex<VecDeque<String>>, line: String) {
    let mut t = tail.lock();
    t.push_back(line);
    if t.len() > 30 {
        t.pop_front();
    }
}

/// The program of an agent could not be started (not there, not allowed to run).
fn cannot_start(lang: Lang, program: &str) -> String {
    tr_in!(
        lang,
        "impossible de lancer {program}",
        "can’t start {program}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_program_that_does_not_start_is_named_in_english() {
        assert_eq!(
            cannot_start(crate::i18n::Lang::En, "C:/bin/claude.exe"),
            "can’t start C:/bin/claude.exe"
        );
    }

    #[test]
    fn a_line_is_read_whatever_its_bytes() {
        assert_eq!(text_line(b"{\"type\":\"x\"}\r"), "{\"type\":\"x\"}");
        // Not UTF-8 (a message of wsl.exe in UTF-16: « détectée »): read all the same.
        assert_eq!(text_line(b"d\0\xe9\0t\0"), "d\u{FFFD}t");
        let tail = Mutex::new(VecDeque::new());
        for n in 0..35 {
            note(&tail, n.to_string());
        }
        assert_eq!(tail.lock().len(), 30);
        assert_eq!(tail.lock().front().map(String::as_str), Some("5"));
    }
}
