//! A `claude` process driven over the stream-json protocol (see docs/PROTOCOL.md).

use crate::i18n::{self, Lang};
use crate::job::{Job, JobUsage};
use anyhow::{anyhow, Context, Result};
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

type Pending = Arc<Mutex<HashMap<String, oneshot::Sender<Result<Value, String>>>>>;

pub struct SpawnOpts {
    pub program: PathBuf,
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
        let mut cmd = Command::new(&opts.program);
        cmd.args(&opts.args)
            .current_dir(&opts.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        for (k, v) in &opts.env {
            cmd.env(k, v);
        }
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);
        crate::job::isolate(&mut cmd);

        let mut child = cmd
            .spawn()
            .with_context(|| cannot_start(i18n::ui(), &opts.program.display().to_string()))?;
        let job = Job::for_child(&child).map(Arc::new);
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

        let stderr_tail = Arc::new(Mutex::new(VecDeque::<String>::new()));
        {
            let tail = stderr_tail.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    log::debug!("claude stderr: {line}");
                    let mut t = tail.lock();
                    t.push_back(line);
                    if t.len() > 30 {
                        t.pop_front();
                    }
                }
            });
        }

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
            let mut lines = BufReader::new(stdout).lines();
            loop {
                tokio::select! {
                    line = lines.next_line() => match line {
                        Ok(Some(line)) => {
                            if line.trim().is_empty() {
                                continue;
                            }
                            match serde_json::from_str::<Value>(&line) {
                                Ok(frame) => {
                                    if frame["type"] == "control_response" {
                                        resolve(&pending, &frame["response"]);
                                    } else {
                                        on_frame(frame);
                                    }
                                }
                                Err(e) => log::warn!("unparsable frame ({e}): {}", truncate(&line, 200)),
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

pub fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

/// Locates the `claude` executable: explicit setting, PATH, then the native installer location.
pub fn resolve_binary(configured: &str) -> Option<PathBuf> {
    if !configured.trim().is_empty() {
        let p = PathBuf::from(configured.trim());
        return p.exists().then_some(p);
    }
    let exts: &[&str] = if cfg!(windows) {
        &["exe", "cmd"]
    } else {
        &[""]
    };
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            for ext in exts {
                let mut candidate = dir.join("claude");
                if !ext.is_empty() {
                    candidate.set_extension(ext);
                }
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    let home = dirs::home_dir()?;
    let candidates: Vec<PathBuf> = if cfg!(windows) {
        vec![
            home.join(".local").join("bin").join("claude.exe"),
            home.join(".claude").join("local").join("claude.exe"),
            home.join("AppData")
                .join("Roaming")
                .join("npm")
                .join("claude.cmd"),
        ]
    } else {
        // The native installer, the old local install, Homebrew (Apple Silicon, then Intel).
        vec![
            home.join(".local").join("bin").join("claude"),
            home.join(".claude").join("local").join("claude"),
            PathBuf::from("/opt/homebrew/bin/claude"),
            PathBuf::from("/usr/local/bin/claude"),
        ]
    };
    candidates.into_iter().find(|p| p.is_file())
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
    fn truncate_respects_char_boundaries() {
        assert_eq!(truncate("héllo", 2), "h…");
        assert_eq!(truncate("abc", 5), "abc");
    }
}
