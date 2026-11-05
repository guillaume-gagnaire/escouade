//! The fake Escouade (PLG-3, ESC-175): a plugin's program started as the app starts it (in its
//! folder, its three variables, in a Job object that takes its tree), spoken to in JSON-RPC 2.0, one
//! message per line, without the app. It sends what it is told to (`call`, `notify`) and keeps what
//! the plugin does on its own: the secrets it asks for (served from the session's, its own keys
//! only, as `runtime` serves them), its `log` lines, its standard error, what it sends that answers
//! nothing. The bench plays its scenarios through it; `escouade-plugin dev` lets an author drive it
//! by hand.
//!
//! A `run.js` plugin is played in WASM (ESC-322), in the app's own host (`plugins::wasm`): its module, an instance on a
//! thread of its own, `net/fetch` served by the host and closed to what `permissions.network` does not name; no Node.
//! As in the app, its instance ends once it answered `shutdown`, and a fault (its time, its memory, an exception) ends
//! it as a crash ends a process. Its `fs/*` and `proc/*` are the app's (ESC-333), on the manifest as it is, around one agent
//! of the bench, `BENCH_AGENT`, in a scratch folder that goes with the session; offline, it runs no program.

use crate::job::Job;
use crate::plugins::manifest::Manifest;
use crate::plugins::runtime::{
    matches, notification_line, request_line, METHOD_NOT_FOUND, NOT_YOURS, PROTOCOL,
};
use crate::plugins::{files, procs, wasm};
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot, Notify};

/// The key of the bench's agent, for `fs/*` and `proc/*`.
pub const BENCH_AGENT: &str = "bench";

/// What a plugin in WASM reaches in the bench: the app's rules, around a scratch folder and its one agent. `back` is the
/// way to its channel; offline, the manifest's commands are not given.
fn capabilities(
    shared: Arc<Shared>,
    manifest: &Manifest,
    folder: &Path,
    scratch: &Path,
    offline: bool,
    back: mpsc::WeakUnboundedSender<wasm::Frame>,
) -> Arc<wasm::Capabilities> {
    for d in ["agent", "repo", "home", "project", "temp"] {
        let _ = std::fs::create_dir_all(scratch.join(d));
    }
    let agents: files::Agents = Arc::default();
    agents.lock().insert(
        BENCH_AGENT.to_string(),
        files::AgentPlace {
            cwd: scratch.join("agent"),
            repo: Some(scratch.join("repo")),
            home: Some(scratch.join("home")),
            fence: crate::fence::Fence::default(),
            project: "bench".into(),
        },
    );
    let told: procs::Told = Arc::new(move |_, line| shared.tell(line.to_string()));
    let project = scratch.join("project");
    let procs = procs::Procs::new(procs::ProcsOf {
        commands: if offline {
            Vec::new()
        } else {
            manifest.permissions.commands.clone()
        },
        temp: scratch.join("temp"),
        agents: agents.clone(),
        projects: Arc::new(move |_| Ok(project.clone())),
        agent_env: Vec::new(),
        back,
        told: told.clone(),
    });
    let access = files::Access::new(manifest, folder, scratch.join("temp"), agents);
    wasm::Capabilities::new(access, procs, told)
}

/// How many lines of its standard error an end tells.
const TAIL: usize = 10;

/// An error the plugin answered.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RemoteError {
    pub code: i64,
    pub message: String,
}

/// What came of a call.
#[derive(Debug, Clone, PartialEq)]
pub enum Reply {
    /// An answer, and how long it took.
    Answered {
        result: Result<Value, RemoteError>,
        elapsed: Duration,
    },
    /// No answer within the patience given.
    Silent,
    /// Its process ended before it answered: its code, its last lines of standard error.
    Ended {
        code: Option<i32>,
        tail: Vec<String>,
    },
}

/// Something the plugin did on its own, in the order it came.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Seen {
    /// It asked for its secret `key`; `own` when the key is one of its `permissions.secrets`.
    Secret { key: String, own: bool },
    /// A `log` notification.
    Log { level: String, message: String },
    /// A line of its standard error.
    Stderr { line: String },
    /// A request for a method Escouade does not serve.
    Unserved { method: String },
    /// Another notification.
    Notification { method: String },
    /// A line that is no JSON-RPC, or an answer to no call of ours (to a notification).
    Stray { line: String },
}

/// What a plugin in WASM reaches on the bench.
#[derive(Debug, Clone, Default)]
pub struct Reach {
    /// No host at all (`--offline`).
    pub offline: bool,
    /// The hosts `{account}` opens (`--account`).
    pub accounts: Vec<String>,
}

struct Shared {
    tx: Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>>,
    pending: Mutex<HashMap<u64, oneshot::Sender<Result<Value, RemoteError>>>>,
    next: AtomicU64,
    seen: Mutex<Vec<Seen>>,
    tail: Mutex<VecDeque<String>>,
    /// The values kept for its own keys.
    secrets: HashMap<String, String>,
    /// For a key of its own with no value kept: `<made>-<key>` (the bench's), else `null`.
    made: Option<String>,
    patterns: Vec<String>,
    /// Its folder, for `files/read`.
    folder: std::path::PathBuf,
    /// Its exit code, once it ended (`Some(None)`: no code, killed).
    ended: Mutex<Option<Option<i32>>>,
    end: Notify,
    /// Each new thing seen: for `escouade-plugin dev`, which shows them as they come.
    watch: Mutex<Option<mpsc::UnboundedSender<Seen>>>,
    /// An instance in WASM: the id of the `shutdown` asked, whose answer ends it, as Escouade ends it.
    wasm: bool,
    shutdown: Mutex<Option<u64>>,
}

impl Shared {
    fn new(
        tx: mpsc::UnboundedSender<Vec<u8>>,
        folder: &Path,
        manifest: &Manifest,
        secrets: HashMap<String, String>,
        made: Option<String>,
        wasm: bool,
    ) -> Arc<Self> {
        Arc::new(Shared {
            tx: Mutex::new(Some(tx)),
            pending: Mutex::new(HashMap::new()),
            next: AtomicU64::new(1),
            seen: Mutex::new(Vec::new()),
            tail: Mutex::new(VecDeque::new()),
            secrets,
            made,
            patterns: manifest.permissions.secrets.clone(),
            folder: folder.to_path_buf(),
            ended: Mutex::new(None),
            end: Notify::new(),
            watch: Mutex::new(None),
            wasm,
            shutdown: Mutex::new(None),
        })
    }

    fn see(&self, s: Seen) {
        if let Some(w) = self.watch.lock().as_ref() {
            let _ = w.send(s.clone());
        }
        self.seen.lock().push(s);
    }

    fn tell(&self, line: String) {
        {
            let mut t = self.tail.lock();
            t.push_back(line.clone());
            while t.len() > TAIL {
                t.pop_front();
            }
        }
        self.see(Seen::Stderr { line });
    }

    /// It ended (`code`: none when it was killed): what still waits gets no answer.
    fn end(&self, code: Option<i32>) {
        self.tx.lock().take();
        *self.ended.lock() = Some(code);
        self.pending.lock().clear();
        self.end.notify_waiters();
    }

    fn write(&self, line: &str) -> bool {
        let mut bytes = Vec::with_capacity(line.len() + 1);
        bytes.extend_from_slice(line.as_bytes());
        bytes.push(b'\n');
        self.tx
            .lock()
            .as_ref()
            .is_some_and(|tx| tx.send(bytes).is_ok())
    }
}

/// A plugin's program, run by the fake Escouade.
pub struct Session {
    shared: Arc<Shared>,
    job: Option<Job>,
    kill: Mutex<Option<oneshot::Sender<()>>>,
    pub pid: Option<u32>,
}

impl Session {
    /// Starts `program` for the plugin of `manifest` in `folder`; `secrets` are the values kept for
    /// its keys; a key of its own with none gets `<made>-<key>`, or `null` as in the app.
    pub fn start(
        folder: &Path,
        manifest: &Manifest,
        program: &Path,
        app_version: &str,
        secrets: HashMap<String, String>,
        made: Option<String>,
    ) -> std::io::Result<Self> {
        let run = manifest.run.as_ref().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "a declarative plugin")
        })?;
        let mut cmd = Command::new(program);
        cmd.args(&run.args)
            .current_dir(folder)
            .env("ESCOUADE_PLUGIN_ID", &manifest.id)
            .env("ESCOUADE_PLUGIN_API", PROTOCOL.to_string())
            .env("ESCOUADE_VERSION", app_version)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(windows)]
        cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
        crate::job::isolate(&mut cmd);
        crate::proc::no_inherited_fds(&mut cmd);
        let mut child = cmd.spawn()?;
        let job = Job::for_child(&child);
        let pid = child.id();
        let mut stdin = child.stdin.take().expect("piped");
        let stdout = child.stdout.take().expect("piped");
        let stderr = child.stderr.take().expect("piped");
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let (kill_tx, mut kill_rx) = oneshot::channel::<()>();
        let shared = Shared::new(tx, folder, manifest, secrets, made, false);

        tokio::spawn(async move {
            while let Some(bytes) = rx.recv().await {
                if stdin.write_all(&bytes).await.is_err() || stdin.flush().await.is_err() {
                    break;
                }
            }
        });

        let (err_tx, err_rx) = oneshot::channel::<()>();
        {
            let shared = shared.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).split(b'\n');
                while let Ok(Some(line)) = lines.next_segment().await {
                    let line = String::from_utf8_lossy(&line)
                        .trim_end_matches('\r')
                        .to_string();
                    shared.tell(line);
                }
                let _ = err_tx.send(());
            });
        }

        {
            let shared = shared.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stdout).split(b'\n');
                loop {
                    tokio::select! {
                        next = lines.next_segment() => match next {
                            Ok(Some(bytes)) => incoming(&shared, &String::from_utf8_lossy(&bytes)),
                            _ => break,
                        },
                        _ = &mut kill_rx => {
                            let _ = child.start_kill();
                            break;
                        }
                    }
                }
                let code = child.wait().await.ok().and_then(|s| s.code());
                let _ = tokio::time::timeout(Duration::from_millis(500), err_rx).await;
                shared.end(code);
            });
        }

        Ok(Self {
            shared,
            job,
            kill: Mutex::new(Some(kill_tx)),
            pid,
        })
    }

    /// Starts the instance of `module`, the WASM of the plugin of `manifest` in `folder`, in Escouade's host,
    /// with its limits; `secrets` and `made` as for a process.
    pub fn start_wasm(
        folder: &Path,
        manifest: &Manifest,
        module: wasmtime::Module,
        secrets: HashMap<String, String>,
        made: Option<String>,
        limits: wasm::Limits,
        reach: Reach,
    ) -> std::io::Result<Self> {
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let shared = Shared::new(tx, folder, manifest, secrets, made, true);
        let (ftx, frames) = mpsc::unbounded_channel::<wasm::Frame>();
        let back = ftx.downgrade();
        // What the bench writes, to the instance's thread; closed when the bench closes it (`kill`, `shutdown`).
        std::thread::Builder::new()
            .name(format!("bench {} input", manifest.id))
            .spawn(move || {
                while let Some(bytes) = rx.blocking_recv() {
                    if ftx.send((bytes, None)).is_err() {
                        break;
                    }
                }
            })?;
        let files = wasm::files_of(folder);
        let offline = reach.offline;
        // Offline (the review's bench), no host is covered: every fetch is refused, as the container's `--network=none`.
        let fetcher = wasm::Fetcher::new(
            &manifest.id,
            if reach.offline {
                &[]
            } else {
                &manifest.permissions.network
            },
            &wasm::Net {
                accounts: Some(Arc::new(move |_| reach.accounts.clone())),
                ..wasm::Net::default()
            },
        )
        .limited(manifest.limits);
        let scratch =
            std::env::temp_dir().join(format!("escouade-bench-{}", uuid::Uuid::new_v4().simple()));
        let reached = capabilities(
            shared.clone(),
            manifest,
            folder,
            &scratch,
            offline,
            back.clone(),
        );
        let (s1, s2, s3, s4, s5) = (
            shared.clone(),
            shared.clone(),
            shared.clone(),
            shared.clone(),
            shared.clone(),
        );
        std::thread::Builder::new()
            .name(format!("bench {}", manifest.id))
            .spawn(move || {
                let calls = Some(reached.calls());
                let fault = match wasm::Instance::start_with(&module, &files, limits, calls) {
                    Err(f) => Some(f),
                    Ok(instance) => {
                        let wiring = wasm::Wiring {
                            rx: frames,
                            back,
                            line: Box::new(move |l| incoming(&s1, l)),
                            stderr: Box::new(move |l| s2.tell(l.to_string())),
                            refused: Arc::new(move |l| s3.tell(l.to_string())),
                            said: Arc::new(move |l| s5.tell(l.to_string())),
                            capabilities: Some(reached.clone()),
                        };
                        wasm::serve(instance, fetcher, wiring)
                    }
                };
                reached.end();
                let _ = std::fs::remove_dir_all(&scratch);
                // A fault ends it as a crash ends a process (code 1); its input closed, it was stopped.
                let code = fault.map(|f| {
                    s4.tell(f.to_string());
                    1
                });
                s4.end(code);
            })?;
        Ok(Self {
            shared,
            job: None,
            kill: Mutex::new(None),
            pid: None,
        })
    }

    /// The `initialize` Escouade sends, offering `version` of the protocol.
    pub fn hello(manifest: &Manifest, app_version: &str, version: u64) -> Value {
        json!({
            "protocolVersion": version,
            "escouade": { "version": app_version },
            "plugin": { "id": manifest.id, "version": manifest.version },
            "lang": "fr",
            "capabilities": { "batch": true, "secrets": true, "log": true, "attach": true },
        })
    }

    /// Calls `method`, waiting `patience` at most for its answer.
    pub async fn call(&self, method: &str, params: &Value, patience: Duration) -> Reply {
        let rx = match self.send(method, params) {
            Some(rx) => rx,
            None => return self.ended_reply(),
        };
        let started = Instant::now();
        match tokio::time::timeout(patience, rx).await {
            Ok(Ok(result)) => Reply::Answered {
                result,
                elapsed: started.elapsed(),
            },
            Ok(Err(_)) => self.ended_reply(),
            Err(_) => Reply::Silent,
        }
    }

    /// Sends the request `method` and gives where its answer comes; none when its input is
    /// closed. A call whose answer is not waited for (a crash in the middle of it).
    pub fn send(
        &self,
        method: &str,
        params: &Value,
    ) -> Option<oneshot::Receiver<Result<Value, RemoteError>>> {
        let params = serde_json::value::to_raw_value(params).ok()?;
        let id = self.shared.next.fetch_add(1, Ordering::Relaxed);
        if method == "shutdown" && self.shared.wasm {
            *self.shared.shutdown.lock() = Some(id);
        }
        let (tx, rx) = oneshot::channel();
        self.shared.pending.lock().insert(id, tx);
        if self.shared.write(&request_line(id, method, &params)) {
            Some(rx)
        } else {
            self.shared.pending.lock().remove(&id);
            None
        }
    }

    /// Tells it something, without waiting; false when its input is closed.
    pub fn notify(&self, method: &str, params: &Value) -> bool {
        match serde_json::value::to_raw_value(params) {
            Ok(p) => self.shared.write(&notification_line(method, &p)),
            Err(_) => false,
        }
    }

    fn ended_reply(&self) -> Reply {
        Reply::Ended {
            code: self.shared.ended.lock().flatten(),
            tail: self.tail(),
        }
    }

    /// Its last lines of standard error.
    pub fn tail(&self) -> Vec<String> {
        self.shared.tail.lock().iter().cloned().collect()
    }

    /// What it did on its own so far.
    pub fn seen(&self) -> Vec<Seen> {
        self.shared.seen.lock().clone()
    }

    /// What it does on its own from now on, as it comes.
    pub fn watch(&self) -> mpsc::UnboundedReceiver<Seen> {
        let (tx, rx) = mpsc::unbounded_channel();
        *self.shared.watch.lock() = Some(tx);
        rx
    }

    /// Whether its process ended, and its code.
    pub fn ended(&self) -> Option<Option<i32>> {
        *self.shared.ended.lock()
    }

    /// Waits `within` at most for its process to end; gives its code when it did.
    pub async fn wait_end(&self, within: Duration) -> Option<Option<i32>> {
        let deadline = Instant::now() + within;
        loop {
            let notified = self.shared.end.notified();
            if let Some(code) = self.ended() {
                return Some(code);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() || tokio::time::timeout(left, notified).await.is_err() {
                return self.ended();
            }
        }
    }

    /// Ends it and its whole tree, as a crash would.
    pub fn kill(&self) {
        if let Some(job) = &self.job {
            job.terminate();
        }
        if let Some(k) = self.kill.lock().take() {
            let _ = k.send(());
        }
        // An instance: its input closed, its thread ends it.
        if self.shared.wasm {
            self.shared.tx.lock().take();
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.kill();
    }
}

/// A line of its output: a message, or a batch of them.
fn incoming(shared: &Shared, text: &str) {
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Array(batch)) => {
            for m in batch {
                message(shared, m);
            }
        }
        Ok(m @ Value::Object(_)) => message(shared, m),
        _ => shared.see(Seen::Stray { line: cut(text) }),
    }
}

fn cut(text: &str) -> String {
    text.chars().take(300).collect()
}

fn message(shared: &Shared, msg: Value) {
    let method = msg.get("method").and_then(Value::as_str).map(String::from);
    let id = msg.get("id").filter(|v| !v.is_null()).cloned();
    match (method, id) {
        (None, Some(id)) => {
            let waiting = id.as_u64().and_then(|n| shared.pending.lock().remove(&n));
            let Some(tx) = waiting else {
                shared.see(Seen::Stray {
                    line: cut(&msg.to_string()),
                });
                return;
            };
            let result = match msg.get("error") {
                Some(e) => Err(RemoteError {
                    code: e.get("code").and_then(Value::as_i64).unwrap_or(0),
                    message: e
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                }),
                None => Ok(msg.get("result").cloned().unwrap_or(Value::Null)),
            };
            let _ = tx.send(result);
            // Its answer to `shutdown`: Escouade ends its instance.
            if shared.wasm && id.as_u64().is_some() && *shared.shutdown.lock() == id.as_u64() {
                shared.tx.lock().take();
            }
        }
        (Some(method), Some(id)) => {
            let params = msg.get("params").cloned().unwrap_or(Value::Null);
            let answer = request(shared, &method, &params);
            let line = match answer {
                Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                Err((code, message)) => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": code, "message": message },
                }),
            };
            shared.write(&line.to_string());
        }
        (Some(method), None) => {
            let params = msg.get("params").cloned().unwrap_or(Value::Null);
            if method == "log" {
                let level = params
                    .get("level")
                    .and_then(Value::as_str)
                    .unwrap_or("info");
                let message = params.get("message").and_then(Value::as_str).unwrap_or("");
                shared.see(Seen::Log {
                    level: level.to_string(),
                    message: message.to_string(),
                });
            } else {
                shared.see(Seen::Notification { method });
            }
        }
        // An answer without id: to a notification, or an error of a line it could not read.
        (None, None) => shared.see(Seen::Stray {
            line: cut(&msg.to_string()),
        }),
    }
}

/// A request of the plugin, served as the app serves it.
fn request(shared: &Shared, method: &str, params: &Value) -> Result<Value, (i64, String)> {
    match method {
        "secrets/get" => {
            let key = params.get("key").and_then(Value::as_str).unwrap_or("");
            let own = !key.is_empty()
                && !key.chars().any(char::is_control)
                && shared.patterns.iter().any(|p| matches(p, key));
            shared.see(Seen::Secret {
                key: key.to_string(),
                own,
            });
            if own {
                let value = shared
                    .secrets
                    .get(key)
                    .cloned()
                    .or_else(|| shared.made.as_ref().map(|m| format!("{m}-{key}")));
                Ok(value.map_or(Value::Null, Value::String))
            } else {
                Err((
                    NOT_YOURS,
                    format!("the secret {key:?} is not one of this plugin's (permissions.secrets)"),
                ))
            }
        }
        // As the app serves it (ESC-321).
        "files/read" => crate::plugins::files::read(&shared.folder, params),
        _ => {
            shared.see(Seen::Unserved {
                method: method.to_string(),
            });
            Err((METHOD_NOT_FOUND, format!("unknown method {method}")))
        }
    }
}
