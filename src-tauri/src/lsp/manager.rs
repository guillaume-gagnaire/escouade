//! The language servers running (ESC-446): one instance by root (a project's folder or a worktree's) and server,
//! shared by the views that need it, its process and what it starts in a Job (`job::Job`: a Job object on Windows,
//! a process group on Unix), so that they end with it, and with the app however it ends.
//!
//! - **Shared**: `acquire` by a view gives the instance of its root and server, started if need be; `release` lets go
//!   of it. When no view holds it any more it stops, after `Policy::idle_stop`: a view closed then opened again (a
//!   tab moved, the page reloaded) finds it running.
//! - **Restarted**: a server that ends without being asked is started again after a delay that doubles each time, at
//!   most `Policy::max_restarts` times in a row (a run longer than `Policy::stable_after` counts the crashes again from
//!   zero). Past it, it is `Failed`, said, until `restart` is asked.
//! - **Carried**: its messages go to a `Sink` (the window's events, `TauriSink`; the remote API of docs/REMOTE.md
//!   another), and come in by `send`, framed and bounded by `transport`. A stream the server breaks (a length too big,
//!   a bad header) ends its process: it is not read further.
//! - **Logged**: what it writes on stderr, and what the core did with it (started, ended, restarted), in a bounded
//!   log, `log`.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Result};
use parking_lot::Mutex;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, Notify};

use super::transport::{frame, read_frame};

/// Where the messages and the states of the servers go.
pub trait Sink: Send + Sync + 'static {
    /// A message of the server `id` (JSON text).
    fn message(&self, id: u32, msg: &str);
    /// The server `id` changed state.
    fn state(&self, info: &Info);
}

/// What runs: found and allowed beforehand (`lsp::launch_for`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub program: PathBuf,
    pub args: Vec<String>,
    /// Its working folder: the root.
    pub cwd: PathBuf,
    /// The repository chose it (its program, its arguments): it ran because the user trusts the project, and stops
    /// when they no longer do (`Manager::stop_of_project`).
    pub of_project: bool,
}

/// How long and how often.
#[derive(Debug, Clone, Copy)]
pub struct Policy {
    /// The restarts in a row after a crash, past which it is `Failed`.
    pub max_restarts: u32,
    /// The delay before the first restart, doubled at each next one (up to `max_delay`).
    pub first_delay: Duration,
    pub max_delay: Duration,
    /// A run at least this long counts the crashes again from zero.
    pub stable_after: Duration,
    /// How long it stays once no view holds it.
    pub idle_stop: Duration,
}

impl Default for Policy {
    fn default() -> Self {
        Policy {
            max_restarts: 5,
            first_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            stable_after: Duration::from_secs(120),
            idle_stop: Duration::from_secs(60),
        }
    }
}

impl Policy {
    /// The delay before the restart number `n` (from 1).
    pub fn delay(&self, n: u32) -> Duration {
        let factor = 1u32.checked_shl(n.saturating_sub(1)).unwrap_or(u32::MAX);
        self.first_delay
            .checked_mul(factor)
            .unwrap_or(self.max_delay)
            .min(self.max_delay)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum State {
    Starting,
    Running,
    /// Ended without being asked: started again after a delay.
    Restarting,
    /// Ended too often in a row: no longer started until `restart`.
    Failed,
    Stopped,
}

/// A server running, as the window sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub id: u32,
    /// The project it serves.
    pub project: String,
    /// Its id in the catalog.
    pub server: String,
    pub root: String,
    pub program: String,
    pub state: State,
    /// Its process, from 1: a view that sees it grow initializes the server again.
    pub run: u32,
    /// The restarts in a row so far.
    pub restarts: u32,
    /// Why it ended, or failed.
    pub error: Option<String>,
    /// The delay before the next start, while `Restarting`, in ms.
    pub retry_in_ms: Option<u64>,
}

/// A line of a server's log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    /// Milliseconds since the epoch.
    pub at: u64,
    /// `stderr`: the server's; `core`: what the core did.
    pub source: &'static str,
    pub text: String,
}

/// The lines kept by server, the oldest dropped past it.
const LOG_LINES: usize = 2000;
/// The longest line kept (a longer one is cut).
const LOG_LINE: usize = 4096;
/// The messages waiting for the server's stdin.
const QUEUE: usize = 1024;

struct Instance {
    id: u32,
    key: (String, String),
    launch: Launch,
    views: Mutex<HashSet<String>>,
    info: Mutex<Info>,
    log: Mutex<VecDeque<LogLine>>,
    /// The stdin of the process running.
    stdin: Mutex<Option<mpsc::Sender<Vec<u8>>>>,
    /// Its Job: terminated to end it with what it started.
    job: Mutex<Option<crate::job::Job>>,
    stopping: AtomicBool,
    /// `restart` asked: the next end is not a crash.
    asked: AtomicBool,
    /// Wakes the supervisor out of its delay, or tells it the process is to end.
    wake: Notify,
    /// Bumped by each `acquire` and `release`: an idle stop planned before is called off.
    generation: AtomicU64,
}

impl Instance {
    fn note(&self, source: &'static str, text: &str) {
        let at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let mut text = text.trim_end().to_string();
        if text.len() > LOG_LINE {
            let mut cut = LOG_LINE;
            while !text.is_char_boundary(cut) {
                cut -= 1;
            }
            text.truncate(cut);
            text.push('…');
        }
        let mut log = self.log.lock();
        if log.len() == LOG_LINES {
            log.pop_front();
        }
        log.push_back(LogLine { at, source, text });
    }

    fn kill(&self) {
        if let Some(job) = self.job.lock().take() {
            job.terminate();
        }
        *self.stdin.lock() = None;
        self.wake.notify_waiters();
    }
}

/// The command that starts the server of `l`: in its root, its pipes, out of sight, ready for its Job.
///
/// Under Windows, `NoDefaultCurrentDirectoryInExePath`: the server found on the `PATH` is often an npm shim (`.cmd`)
/// that calls `node` by its name, and `cmd` would look for it in its current folder first, the root of the worktree. A
/// `node.exe`, `node.cmd` or `node.bat` of a cloned repository would run as the file is opened, the project trusted or
/// not (as `escouade_acp::terminal` does for the hooks, ESC-430).
pub(crate) fn command(l: &Launch) -> Command {
    let mut cmd = Command::new(&l.program);
    cmd.args(&l.args)
        .current_dir(&l.cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    crate::job::isolate(&mut cmd);
    #[cfg(windows)]
    {
        cmd.env(escouade_acp::terminal::NO_CURRENT_DIR, "1");
        cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
    }
    cmd
}

/// The servers running.
pub struct Manager {
    sink: Arc<dyn Sink>,
    policy: Policy,
    instances: Mutex<HashMap<u32, Arc<Instance>>>,
    next: Mutex<u32>,
}

impl Manager {
    pub fn new(sink: Arc<dyn Sink>, policy: Policy) -> Arc<Self> {
        Arc::new(Manager {
            sink,
            policy,
            instances: Mutex::new(HashMap::new()),
            next: Mutex::new(0),
        })
    }

    fn get(&self, id: u32) -> Result<Arc<Instance>> {
        self.instances
            .lock()
            .get(&id)
            .cloned()
            .ok_or_else(|| anyhow!("no language server {id}"))
    }

    /// The instance of `server` in `root` for `view`: the one running, or one started with `launch`.
    pub fn acquire(
        self: &Arc<Self>,
        view: &str,
        project: &str,
        server: &str,
        root: &str,
        launch: Launch,
    ) -> Info {
        let key = (root.to_string(), server.to_string());
        let mut instances = self.instances.lock();
        if let Some(i) = instances.values().find(|i| i.key == key) {
            i.views.lock().insert(view.to_string());
            i.generation.fetch_add(1, Ordering::SeqCst);
            return i.info.lock().clone();
        }
        let id = {
            let mut next = self.next.lock();
            *next += 1;
            *next
        };
        let info = Info {
            id,
            project: project.to_string(),
            server: server.to_string(),
            root: root.to_string(),
            program: launch.program.display().to_string(),
            state: State::Starting,
            run: 0,
            restarts: 0,
            error: None,
            retry_in_ms: None,
        };
        let i = Arc::new(Instance {
            id,
            key,
            launch,
            views: Mutex::new(HashSet::from([view.to_string()])),
            info: Mutex::new(info.clone()),
            log: Mutex::new(VecDeque::new()),
            stdin: Mutex::new(None),
            job: Mutex::new(None),
            stopping: AtomicBool::new(false),
            asked: AtomicBool::new(false),
            wake: Notify::new(),
            generation: AtomicU64::new(0),
        });
        instances.insert(id, i.clone());
        drop(instances);
        let me = self.clone();
        tokio::spawn(async move { me.supervise(i).await });
        info
    }

    /// A message for the server `id`.
    pub async fn send(&self, id: u32, msg: &str) -> Result<()> {
        let i = self.get(id)?;
        let bytes = frame(msg)?;
        let tx = i.stdin.lock().clone();
        let Some(tx) = tx else {
            let state = i.info.lock().state;
            bail!("language server {id} is not running ({state:?})");
        };
        tx.send(bytes)
            .await
            .map_err(|_| anyhow!("language server {id} ended"))
    }

    /// `view` no longer needs the server `id`: stopped after `Policy::idle_stop` once no view does.
    pub fn release(self: &Arc<Self>, view: &str, id: u32) {
        let Ok(i) = self.get(id) else { return };
        let empty = {
            let mut views = i.views.lock();
            views.remove(view);
            views.is_empty()
        };
        if !empty {
            return;
        }
        let generation = i.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let me = self.clone();
        let wait = self.policy.idle_stop;
        tokio::spawn(async move {
            tokio::time::sleep(wait).await;
            if i.generation.load(Ordering::SeqCst) == generation && i.views.lock().is_empty() {
                i.note("core", "stopped: no view uses it any more");
                me.stop(i.id);
            }
        });
    }

    /// `view` is gone (its page closed or reloaded): it holds no server any more.
    pub fn release_view(self: &Arc<Self>, view: &str) {
        let held: Vec<u32> = self
            .instances
            .lock()
            .values()
            .filter(|i| i.views.lock().contains(view))
            .map(|i| i.id)
            .collect();
        for id in held {
            self.release(view, id);
        }
    }

    /// Ends the server `id` now, with what it started.
    pub fn stop(&self, id: u32) {
        let Some(i) = self.instances.lock().remove(&id) else {
            return;
        };
        i.stopping.store(true, Ordering::SeqCst);
        i.kill();
        let info = {
            let mut info = i.info.lock();
            info.state = State::Stopped;
            info.retry_in_ms = None;
            info.clone()
        };
        self.sink.state(&info);
    }

    /// Ends the servers of `project` that the repository chose: the user no longer trusts it.
    pub fn stop_of_project(&self, project: &str) {
        let ids: Vec<u32> = self
            .instances
            .lock()
            .values()
            .filter(|i| i.launch.of_project && i.info.lock().project == project)
            .map(|i| i.id)
            .collect();
        for id in ids {
            self.stop(id);
        }
    }

    /// Starts the server `id` again now, its crashes counted from zero: one `Failed` too.
    pub fn restart(self: &Arc<Self>, id: u32) -> Result<()> {
        let i = self.get(id)?;
        i.note("core", "restart asked");
        let failed = i.info.lock().state == State::Failed;
        if failed {
            i.info.lock().restarts = 0;
            let me = self.clone();
            tokio::spawn(async move { me.supervise(i).await });
        } else {
            i.asked.store(true, Ordering::SeqCst);
            i.kill();
        }
        Ok(())
    }

    /// The servers running.
    pub fn list(&self) -> Vec<Info> {
        let mut out: Vec<Info> = self
            .instances
            .lock()
            .values()
            .map(|i| i.info.lock().clone())
            .collect();
        out.sort_by_key(|i| i.id);
        out
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn info(&self, id: u32) -> Result<Info> {
        Ok(self.get(id)?.info.lock().clone())
    }

    /// The log of the server `id`, the oldest line first.
    pub fn log(&self, id: u32) -> Result<Vec<LogLine>> {
        Ok(self.get(id)?.log.lock().iter().cloned().collect())
    }

    /// The ids of the processes of the server `id` (its Job's), for the tests and the resources.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn pids(&self, id: u32) -> Vec<u32> {
        self.get(id)
            .ok()
            .and_then(|i| i.job.lock().as_ref().map(|j| j.pids()))
            .unwrap_or_default()
    }

    /// Ends every server: the app ends.
    pub fn shutdown(&self) {
        let ids: Vec<u32> = self.instances.lock().keys().copied().collect();
        for id in ids {
            self.stop(id);
        }
    }

    fn set(&self, i: &Instance, change: impl FnOnce(&mut Info)) {
        let info = {
            let mut info = i.info.lock();
            change(&mut info);
            info.clone()
        };
        if !i.stopping.load(Ordering::SeqCst) {
            self.sink.state(&info);
        }
    }

    /// Runs the server of `i`, and again after each crash, while the policy allows.
    async fn supervise(self: Arc<Self>, i: Arc<Instance>) {
        loop {
            if i.stopping.load(Ordering::SeqCst) {
                return;
            }
            let started = Instant::now();
            let ended = self.run_once(&i).await;
            if i.stopping.load(Ordering::SeqCst) {
                return;
            }
            if i.asked.swap(false, Ordering::SeqCst) {
                self.set(&i, |info| {
                    info.restarts = 0;
                    info.error = None;
                });
                continue;
            }
            i.note("core", &format!("ended: {ended}"));
            let restarts = {
                let mut info = i.info.lock();
                if started.elapsed() >= self.policy.stable_after {
                    info.restarts = 0;
                }
                info.restarts += 1;
                info.restarts
            };
            if restarts > self.policy.max_restarts {
                let max = self.policy.max_restarts;
                i.note(
                    "core",
                    &format!(
                        "failed: ended {max} times in a row after its restarts, not started again"
                    ),
                );
                self.set(&i, |info| {
                    info.restarts = max;
                    info.state = State::Failed;
                    info.error = Some(ended);
                    info.retry_in_ms = None;
                });
                return;
            }
            let delay = self.policy.delay(restarts);
            i.note(
                "core",
                &format!("restart {restarts} in {} ms", delay.as_millis()),
            );
            self.set(&i, |info| {
                info.state = State::Restarting;
                info.error = Some(ended);
                info.retry_in_ms = Some(delay.as_millis() as u64);
            });
            tokio::select! {
                _ = tokio::time::sleep(delay) => {}
                _ = i.wake.notified() => {}
            }
            if i.asked.swap(false, Ordering::SeqCst) {
                self.set(&i, |info| info.restarts = 0);
            }
        }
    }

    /// One process of the server, until it ends: why.
    async fn run_once(&self, i: &Arc<Instance>) -> String {
        let l = &i.launch;
        let mut cmd = command(l);
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let why = format!("could not start {}: {e}", l.program.display());
                i.note("core", &why);
                return why;
            }
        };
        *i.job.lock() = crate::job::Job::for_child(&child);
        let (Some(mut stdin), Some(stdout), Some(stderr)) =
            (child.stdin.take(), child.stdout.take(), child.stderr.take())
        else {
            return "no pipes".into();
        };
        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(QUEUE);
        *i.stdin.lock() = Some(tx);
        let pid = child.id().unwrap_or(0);
        i.note(
            "core",
            &format!(
                "started {} {} (pid {pid})",
                l.program.display(),
                l.args.join(" ")
            ),
        );
        self.set(i, |info| {
            info.state = State::Running;
            info.run += 1;
            info.retry_in_ms = None;
        });
        let writer = tokio::spawn(async move {
            while let Some(bytes) = rx.recv().await {
                if stdin.write_all(&bytes).await.is_err() || stdin.flush().await.is_err() {
                    break;
                }
            }
        });
        let errs = {
            let i = i.clone();
            tokio::spawn(async move {
                let mut r = BufReader::new(stderr);
                let mut line = Vec::new();
                loop {
                    line.clear();
                    // A line bounded as the log keeps it: the rest of a longer one comes as the next.
                    match (&mut r)
                        .take(LOG_LINE as u64)
                        .read_until(b'\n', &mut line)
                        .await
                    {
                        Ok(0) | Err(_) => break,
                        Ok(_) => i.note("stderr", &String::from_utf8_lossy(&line)),
                    }
                }
            })
        };
        let mut out = BufReader::new(stdout);
        let broken = loop {
            match read_frame(&mut out).await {
                Ok(Some(msg)) => self.sink.message(i.id, &msg),
                Ok(None) => break None,
                Err(e) => break Some(e.to_string()),
            }
        };
        if let Some(why) = &broken {
            // Nothing more of this stream is read: the process ends.
            i.note("core", &format!("stream refused: {why}"));
            if let Some(job) = i.job.lock().as_ref() {
                job.terminate();
            }
            let _ = child.start_kill();
        }
        let status = tokio::select! {
            s = child.wait() => s.ok(),
            _ = i.wake.notified() => {
                let _ = child.start_kill();
                child.wait().await.ok()
            }
        };
        *i.stdin.lock() = None;
        writer.abort();
        let _ = errs.await;
        *i.job.lock() = None;
        let code = match status.and_then(|s| s.code()) {
            Some(c) => format!("exit code {c}"),
            None => "killed".to_string(),
        };
        match broken {
            Some(why) => format!("{why} ({code})"),
            None => code,
        }
    }
}
