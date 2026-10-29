//! The plugins' processes (PLG-2, ESC-174; docs/PLUGINS.md §2.4, §3.5, §4.6): a plugin with a
//! program (`run`) runs as a process of its own, started at its first activation and never before,
//! in a Job object (`job.rs`) that takes its whole tree with it. The two speak JSON-RPC 2.0 on its
//! standard input and output, one message per line; a batch (a JSON array, on one line) goes both
//! ways.
//!
//! - `initialize` comes first: Escouade gives the version of the protocol it speaks and its
//!   capabilities, the plugin answers the version it speaks and its own (as in ACP); another version
//!   stops it, with the reason.
//! - Every call has a delay (`timeout_of`, by method: short for a decision, long for an import);
//!   past it, the caller's default answer applies (`call_or`).
//! - One process serves every agent of its plugin: the calls are told apart by their ids, and what
//!   an agent attached (`attach`) is given again to the next process after a restart.
//! - A crash restarts it, three times within five minutes, then stops it. While it starts again,
//!   what is sent to it waits in a bounded queue (the oldest dropped first, and said in its journal),
//!   then goes in order.
//! - Its memory, read from its job as `resources.rs` reads the agents', stops it past a limit.
//! - It gets its secrets by `secrets/get`, only its own (`plugin:<id>:<key>`, a key of its
//!   `permissions.secrets`), never another's; the values it was given are masked in its journal.
//! - Its standard error and its messages go to its journal (`journal`).
//! - When Escouade closes, each running plugin is asked to `shutdown`, then its job ends.
//!
//! The parameters and results travel as raw JSON (`RawValue`): a frame a harness wrote is handed
//! on as it came, never read again on the way.

use super::journal::{Entry, Journal, Kind};
use super::manifest::{self, Manifest};
use super::{Plugins, State as Loaded};
use crate::i18n::Lang;
use crate::integrations::secrets::SecretStore;
use crate::job::{Job, JobUsage};
use crate::paths::DataDir;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

/// The version of the protocol this Escouade speaks: the major version of the plugins' API.
pub const PROTOCOL: u64 = manifest::API;

/// The error of a method Escouade does not serve (JSON-RPC).
pub const METHOD_NOT_FOUND: i64 = -32601;
/// The error of parameters that do not hold (JSON-RPC).
pub const INVALID_PARAMS: i64 = -32602;
/// The error of a secret that is not the plugin's own.
pub const NOT_YOURS: i64 = -32001;
/// The error of a keychain out of reach.
pub const KEYCHAIN: i64 = -32002;

/// How many lines of its standard error a crash tells in the journal.
const TAIL: usize = 10;

/// What a plugin's process may do before it is stopped.
#[derive(Debug, Clone)]
pub struct Limits {
    /// Its memory (its job's working sets), in bytes.
    pub memory: u64,
    /// How often its memory is read.
    pub sample: Duration,
    /// How many restarts within `window`; one more crash stops it.
    pub restarts: usize,
    pub window: Duration,
    /// How many messages wait while it starts again.
    pub queue: usize,
    /// How long it has to answer `initialize`.
    pub initialize: Duration,
    /// How long it has to answer `shutdown` before its job ends.
    pub shutdown: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            memory: 1 << 30,
            sample: Duration::from_secs(2),
            restarts: 3,
            window: Duration::from_secs(5 * 60),
            queue: 256,
            initialize: Duration::from_secs(10),
            shutdown: Duration::from_secs(1),
        }
    }
}

/// The delay of a call to `method`: short for a decision that holds an agent, long for an import.
pub fn timeout_of(method: &str) -> Duration {
    let ms = match method {
        "initialize" => 10_000,
        "shutdown" => 1_000,
        m if m.starts_with("permissions/") => 500,
        m if m.starts_with("tickets/") || m.starts_with("forge/") || m == "engine/oneShot" => {
            60_000
        }
        _ => 5_000,
    };
    Duration::from_millis(ms)
}

/// Why a plugin's process was stopped and does not start again by itself.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Stop {
    /// Asked (Réglages › Plugins, a permission taken back).
    Asked,
    /// Escouade closed.
    Shutdown,
    /// It crashed once more than it may restart within the window.
    Crashed { times: usize },
    /// Its memory went past the limit.
    Memory { bytes: u64, limit: u64 },
    /// It could not start, or what it answered to `initialize` does not hold.
    Refused { reason: String },
}

impl Stop {
    /// In words, for its journal.
    pub fn describe(&self, window: Duration) -> String {
        match self {
            Stop::Asked => "stopped".into(),
            Stop::Shutdown => "stopped: Escouade closed".into(),
            Stop::Crashed { times } => format!(
                "stopped after {times} crashes within {} minutes",
                window.as_secs().div_ceil(60)
            ),
            Stop::Memory { bytes, limit } => format!(
                "stopped: {} MB of memory, past the limit of {} MB",
                bytes >> 20,
                limit >> 20
            ),
            Stop::Refused { reason } => format!("stopped: {reason}"),
        }
    }
}

/// Why a call got no answer of the plugin.
#[derive(Debug, Clone, PartialEq)]
pub enum CallError {
    /// No such plugin, or one that does not run (declarative, disabled, invalid).
    Unavailable(String),
    /// Its process is stopped.
    Stopped(Stop),
    /// No answer in time.
    Timeout,
    /// Its process ended before it answered.
    Crashed,
    /// Dropped from the queue kept while it started again.
    Dropped,
    /// It answered an error.
    Remote { code: i64, message: String },
    /// Its answer could not be read.
    Invalid(String),
}

impl std::fmt::Display for CallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallError::Unavailable(why) => write!(f, "plugin unavailable: {why}"),
            CallError::Stopped(s) => write!(f, "plugin {}", s.describe(Limits::default().window)),
            CallError::Timeout => write!(f, "no answer in time"),
            CallError::Crashed => write!(f, "the plugin ended before it answered"),
            CallError::Dropped => write!(f, "dropped while the plugin started again"),
            CallError::Remote { code, message } => write!(f, "{message} ({code})"),
            CallError::Invalid(why) => write!(f, "unreadable answer: {why}"),
        }
    }
}

impl std::error::Error for CallError {}

/// Where a plugin's process stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PhaseName {
    Idle,
    Starting,
    Running,
    Restarting,
    Stopped,
}

/// A plugin's process, as Réglages › Plugins shows it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub phase: PhaseName,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Stop>,
    /// Its restarts within the window.
    pub restarts: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    /// What it said it can do, in its answer to `initialize`.
    pub capabilities: Value,
}

/// The name of a plugin's secret in the keychain (§3.5).
pub fn secret_name(plugin: &str, key: &str) -> String {
    format!("plugin:{plugin}:{key}")
}

/// Whether `word` is one of `patterns`: the same, or one a `prefix*` takes in.
fn matches(pattern: &str, word: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => word.starts_with(prefix),
        None => pattern == word,
    }
}

/// A request on one line. `params` is raw JSON: a newline in it can only be whitespace (a string
/// of JSON holds none), so it is made a space.
fn request_line(id: u64, method: &str, params: &RawValue) -> String {
    let method = serde_json::to_string(method).unwrap_or_else(|_| "\"\"".into());
    let params = one_line(params.get());
    format!(r#"{{"jsonrpc":"2.0","id":{id},"method":{method},"params":{params}}}"#)
}

fn notification_line(method: &str, params: &RawValue) -> String {
    let method = serde_json::to_string(method).unwrap_or_else(|_| "\"\"".into());
    let params = one_line(params.get());
    format!(r#"{{"jsonrpc":"2.0","method":{method},"params":{params}}}"#)
}

fn one_line(json: &str) -> std::borrow::Cow<'_, str> {
    if json.contains(['\n', '\r']) {
        json.replace(['\n', '\r'], " ").into()
    } else {
        json.into()
    }
}

fn raw(v: &Value) -> Box<RawValue> {
    serde_json::value::to_raw_value(v).unwrap_or_else(|_| null())
}

fn null() -> Box<RawValue> {
    RawValue::from_string("null".into()).expect("null is JSON")
}

type Answer = Result<Box<RawValue>, CallError>;

/// A message the plugin sent: an answer, a request or a notification.
#[derive(Deserialize)]
struct Incoming {
    #[serde(default)]
    id: Option<Value>,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    params: Option<Box<RawValue>>,
    #[serde(default)]
    result: Option<Box<RawValue>>,
    #[serde(default)]
    error: Option<RpcError>,
}

#[derive(Deserialize)]
struct RpcError {
    code: i64,
    #[serde(default)]
    message: String,
}

/// A line to send, and the ids of the calls it carries (none for a notification).
struct Queued {
    ids: Vec<u64>,
    line: String,
}

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Idle,
    Starting,
    Running,
    Restarting,
    Stopped(Stop),
}

struct Inner {
    phase: Phase,
    /// The process of now, from its start (before `initialize`) to its end.
    conn: Option<Arc<Conn>>,
    /// Counts the processes started: what an older one says is not listened to.
    generation: u64,
    /// When it restarted, within the window.
    restarts: VecDeque<Instant>,
    queue: VecDeque<Queued>,
    /// What was attached to it, by key (an agent's id): given again after a restart.
    attached: BTreeMap<String, (String, Box<RawValue>)>,
    capabilities: Value,
}

/// A running process of a plugin.
struct Conn {
    generation: u64,
    pid: Option<u32>,
    tx: Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>>,
    job: Option<Job>,
    kill_tx: Mutex<Option<oneshot::Sender<()>>>,
    exited: AtomicBool,
}

impl Conn {
    /// Writes `line` on its input; false when it is closed.
    fn write(&self, line: &str) -> bool {
        let mut bytes = Vec::with_capacity(line.len() + 1);
        bytes.extend_from_slice(line.as_bytes());
        bytes.push(b'\n');
        self.tx
            .lock()
            .as_ref()
            .is_some_and(|tx| tx.send(bytes).is_ok())
    }

    /// Ends it and its whole tree.
    fn kill(&self) {
        self.tx.lock().take();
        if let Some(job) = &self.job {
            job.terminate();
        }
        if let Some(k) = self.kill_tx.lock().take() {
            let _ = k.send(());
        }
    }

    fn usage(&self) -> Option<JobUsage> {
        self.job.as_ref().and_then(Job::usage)
    }
}

/// A plugin with a program, and its process when it runs.
struct Host {
    id: String,
    folder: PathBuf,
    manifest: Manifest,
    rt: Weak<Runtime>,
    inner: Mutex<Inner>,
    pending: Mutex<HashMap<u64, oneshot::Sender<Answer>>>,
    next_id: AtomicU64,
    journal: Journal,
}

/// The plugins' processes of the app.
pub struct Runtime {
    plugins: Arc<Plugins>,
    secrets: Arc<dyn SecretStore>,
    logs: PathBuf,
    app_version: String,
    /// The folder of the app's programs (`{escouade}` of a plugin of its own).
    escouade_dir: PathBuf,
    limits: Limits,
    hosts: Mutex<HashMap<String, Arc<Host>>>,
    quitting: AtomicBool,
    me: Weak<Self>,
}

impl Runtime {
    pub fn new(
        plugins: Arc<Plugins>,
        secrets: Arc<dyn SecretStore>,
        data: &DataDir,
        app_version: &str,
        limits: Limits,
    ) -> Arc<Self> {
        let escouade_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_default();
        Arc::new_cyclic(|me| Self {
            plugins,
            secrets,
            logs: data.plugin_logs_dir(),
            app_version: app_version.to_string(),
            escouade_dir,
            limits,
            hosts: Mutex::new(HashMap::new()),
            quitting: AtomicBool::new(false),
            me: me.clone(),
        })
    }

    /// The plugin `id`'s host, made at its first use: only a plugin that loaded, with a program.
    fn host(&self, id: &str) -> Result<Arc<Host>, CallError> {
        if let Some(h) = self.hosts.lock().get(id) {
            return Ok(h.clone());
        }
        if self.quitting.load(Ordering::Acquire) {
            return Err(CallError::Stopped(Stop::Shutdown));
        }
        let view = self
            .plugins
            .get(id, Lang::En)
            .ok_or_else(|| CallError::Unavailable(format!("{id} is not installed")))?;
        match &view.state {
            Loaded::Ready => {}
            Loaded::Disabled { reason } | Loaded::Invalid { reason, .. } => {
                return Err(CallError::Unavailable(reason.clone()))
            }
        }
        let manifest = view
            .manifest
            .filter(|m| m.run.is_some())
            .ok_or_else(|| CallError::Unavailable(format!("{id} is declarative: nothing runs")))?;
        let host = Arc::new(Host {
            id: id.to_string(),
            folder: PathBuf::from(&view.folder),
            manifest,
            rt: self.me.clone(),
            inner: Mutex::new(Inner {
                phase: Phase::Idle,
                conn: None,
                generation: 0,
                restarts: VecDeque::new(),
                queue: VecDeque::new(),
                attached: BTreeMap::new(),
                capabilities: Value::Null,
            }),
            pending: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            journal: Journal::new(self.logs.join(format!("{id}.log"))),
        });
        Ok(self
            .hosts
            .lock()
            .entry(id.to_string())
            .or_insert(host)
            .clone())
    }

    /// Starts the plugins `word` activates (`event:ticket.column`, `account:github`), those that
    /// do not run yet; gives their ids.
    pub fn activate(&self, word: &str) -> Vec<String> {
        let mut started = Vec::new();
        for v in self.plugins.list(Lang::En) {
            let Some(m) = &v.manifest else { continue };
            if v.state != Loaded::Ready
                || m.run.is_none()
                || !m.activation.iter().any(|a| matches(a, word))
            {
                continue;
            }
            if let Ok(h) = self.host(&v.id) {
                if h.wake() {
                    started.push(v.id.clone());
                }
            }
        }
        started
    }

    /// Where the plugin `id`'s process stands; idle for one that never started.
    pub fn status(&self, id: &str) -> Status {
        let Some(h) = self.hosts.lock().get(id).cloned() else {
            return Status {
                phase: PhaseName::Idle,
                stop: None,
                restarts: 0,
                pid: None,
                capabilities: Value::Null,
            };
        };
        let inner = h.inner.lock();
        let (phase, stop) = match &inner.phase {
            Phase::Idle => (PhaseName::Idle, None),
            Phase::Starting => (PhaseName::Starting, None),
            Phase::Running => (PhaseName::Running, None),
            Phase::Restarting => (PhaseName::Restarting, None),
            Phase::Stopped(s) => (PhaseName::Stopped, Some(s.clone())),
        };
        Status {
            phase,
            stop,
            restarts: inner.restarts.len(),
            pid: inner.conn.as_ref().and_then(|c| c.pid),
            capabilities: inner.capabilities.clone(),
        }
    }

    /// The plugin `id`'s journal, the oldest entry first.
    pub fn journal(&self, id: &str) -> Vec<Entry> {
        self.hosts
            .lock()
            .get(id)
            .map(|h| h.journal.entries())
            .unwrap_or_default()
    }

    /// Calls `method` of the plugin `id`, within its delay (`timeout_of`); starts it if it does
    /// not run.
    pub async fn call(&self, id: &str, method: &str, params: Value) -> Result<Value, CallError> {
        self.call_within(id, method, params, timeout_of(method))
            .await
    }

    /// `call`, within `within`.
    pub async fn call_within(
        &self,
        id: &str,
        method: &str,
        params: Value,
        within: Duration,
    ) -> Result<Value, CallError> {
        let answer = self.call_raw(id, method, &raw(&params), within).await?;
        serde_json::from_str(answer.get()).map_err(|e| CallError::Invalid(e.to_string()))
    }

    /// `call`, or `default` when the plugin does not answer in time, fails or does not run.
    pub async fn call_or(&self, id: &str, method: &str, params: Value, default: Value) -> Value {
        match self.call(id, method, params).await {
            Ok(v) => v,
            Err(e) => {
                log::debug!("plugin {id}: {method}: {e}; the default answer applies");
                default
            }
        }
    }

    /// `call` with raw parameters and result: they are handed on as they are, never read again.
    pub async fn call_raw(
        &self,
        id: &str,
        method: &str,
        params: &RawValue,
        within: Duration,
    ) -> Answer {
        let host = self.host(id)?;
        let call = host.next_id.fetch_add(1, Ordering::Relaxed);
        let rx = host.register(call);
        host.send(Queued {
            ids: vec![call],
            line: request_line(call, method, params),
        })
        .inspect_err(|_| {
            host.pending.lock().remove(&call);
        })?;
        host.wait(call, method, rx, within).await
    }

    /// Several calls in one batch (one line), their answers in the same order, all within
    /// `within`.
    pub async fn call_batch(
        &self,
        id: &str,
        calls: Vec<(String, Value)>,
        within: Duration,
    ) -> Vec<Result<Value, CallError>> {
        // An empty batch is no JSON-RPC.
        if calls.is_empty() {
            return Vec::new();
        }
        let host = match self.host(id) {
            Ok(h) => h,
            Err(e) => return calls.iter().map(|_| Err(e.clone())).collect(),
        };
        let mut ids = Vec::with_capacity(calls.len());
        let mut lines = Vec::with_capacity(calls.len());
        let mut waits = Vec::with_capacity(calls.len());
        for (method, params) in &calls {
            let call = host.next_id.fetch_add(1, Ordering::Relaxed);
            ids.push(call);
            lines.push(request_line(call, method, &raw(params)));
            waits.push((call, method.clone(), host.register(call)));
        }
        let line = format!("[{}]", lines.join(","));
        if let Err(e) = host.send(Queued {
            ids: ids.clone(),
            line,
        }) {
            let mut pending = host.pending.lock();
            for id in &ids {
                pending.remove(id);
            }
            return calls.iter().map(|_| Err(e.clone())).collect();
        }
        let deadline = Instant::now() + within;
        let mut out = Vec::with_capacity(waits.len());
        for (call, method, rx) in waits {
            let left = deadline.saturating_duration_since(Instant::now());
            out.push(host.wait(call, &method, rx, left).await.and_then(|a| {
                serde_json::from_str(a.get()).map_err(|e| CallError::Invalid(e.to_string()))
            }));
        }
        out
    }

    /// Tells the plugin `id` something, without waiting for an answer.
    pub fn notify(&self, id: &str, method: &str, params: Value) -> Result<(), CallError> {
        let host = self.host(id)?;
        host.send(Queued {
            ids: Vec::new(),
            line: notification_line(method, &raw(&params)),
        })
    }

    /// Calls `method` with `params` for `key` (an agent's id), and keeps it: the next process,
    /// after a restart, is given it again before anything else.
    pub async fn attach(
        &self,
        id: &str,
        key: &str,
        method: &str,
        params: Value,
    ) -> Result<Value, CallError> {
        let host = self.host(id)?;
        host.inner
            .lock()
            .attached
            .insert(key.to_string(), (method.to_string(), raw(&params)));
        self.call(id, method, params).await
    }

    /// Forgets what `key` attached.
    pub fn detach(&self, id: &str, key: &str) {
        if let Some(h) = self.hosts.lock().get(id) {
            h.inner.lock().attached.remove(key);
        }
    }

    /// Stops the plugin `id`'s process; it does not start again until `restart`.
    pub fn stop(&self, id: &str) {
        if let Some(h) = self.hosts.lock().get(id).cloned() {
            h.stop_with(Stop::Asked);
        }
    }

    /// Starts again a plugin that was stopped (« Relancer »), its crashes forgotten.
    pub fn restart(&self, id: &str) -> Result<(), CallError> {
        let host = self.host(id)?;
        {
            let mut inner = host.inner.lock();
            if !matches!(inner.phase, Phase::Stopped(_)) {
                return Ok(());
            }
            inner.phase = Phase::Idle;
            inner.restarts.clear();
        }
        host.wake();
        Ok(())
    }

    /// Escouade closes: each running plugin is asked to `shutdown`, given `Limits::shutdown` in
    /// all to answer, then its job ends. Blocks the calling thread for that long at most.
    pub fn shutdown(&self) {
        self.quitting.store(true, Ordering::Release);
        let hosts: Vec<Arc<Host>> = self.hosts.lock().values().cloned().collect();
        let mut asked = Vec::new();
        let mut conns = Vec::new();
        for h in &hosts {
            let (conn, rx) = h.begin_shutdown();
            if let Some(c) = conn {
                conns.push((h.clone(), c));
            }
            if let Some(rx) = rx {
                asked.push((h.clone(), rx));
            }
        }
        let deadline = Instant::now() + self.limits.shutdown;
        let mut waiting = asked;
        while !waiting.is_empty() && Instant::now() < deadline {
            waiting.retain_mut(|(h, rx)| match rx.try_recv() {
                Err(oneshot::error::TryRecvError::Empty) => true,
                Ok(Ok(_)) => {
                    h.journal.push(Kind::Stop, "answered shutdown");
                    false
                }
                _ => false,
            });
            std::thread::sleep(Duration::from_millis(10));
        }
        for (h, _) in &waiting {
            h.journal.push(
                Kind::Timeout,
                format!(
                    "shutdown: no answer within {} ms; its job ends",
                    self.limits.shutdown.as_millis()
                ),
            );
        }
        for (h, c) in conns {
            c.kill();
            h.fail_all(CallError::Stopped(Stop::Shutdown), &HashSet::new());
        }
    }

    /// The program of `run` for this system, in the plugin's folder or the app's.
    fn program_of(&self, folder: &Path, run: &manifest::Run) -> PathBuf {
        let sys = if cfg!(windows) {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else {
            "linux"
        };
        let p = run.by_system.get(sys).unwrap_or(&run.command);
        match p.strip_prefix(manifest::ESCOUADE_DIR) {
            Some(rest) => self.escouade_dir.join(rest.trim_start_matches(['/', '\\'])),
            None => folder.join(p),
        }
    }
}

impl Host {
    fn register(&self, id: u64) -> oneshot::Receiver<Answer> {
        let (tx, rx) = oneshot::channel();
        self.pending.lock().insert(id, tx);
        rx
    }

    fn limits(&self) -> Limits {
        self.rt
            .upgrade()
            .map(|rt| rt.limits.clone())
            .unwrap_or_default()
    }

    /// Waits for the answer to `id`; past `within`, it is no longer waited for.
    async fn wait(
        &self,
        id: u64,
        method: &str,
        rx: oneshot::Receiver<Answer>,
        within: Duration,
    ) -> Answer {
        match tokio::time::timeout(within, rx).await {
            Ok(Ok(answer)) => answer,
            Ok(Err(_)) => Err(CallError::Crashed),
            Err(_) => {
                self.pending.lock().remove(&id);
                self.journal.push(
                    Kind::Timeout,
                    format!("{method}: no answer within {} ms", within.as_millis()),
                );
                Err(CallError::Timeout)
            }
        }
    }

    /// Gives `answer` to the call `id`, if it still waits.
    fn answer(&self, id: u64, answer: Answer) {
        if let Some(tx) = self.pending.lock().remove(&id) {
            let _ = tx.send(answer);
        }
    }

    /// Fails every call still waiting but those of `kept` (queued for the next process).
    fn fail_all(&self, err: CallError, kept: &HashSet<u64>) {
        let failed: Vec<_> = {
            let mut pending = self.pending.lock();
            let ids: Vec<u64> = pending
                .keys()
                .filter(|id| !kept.contains(id))
                .copied()
                .collect();
            ids.into_iter()
                .filter_map(|id| pending.remove(&id))
                .collect()
        };
        for tx in failed {
            let _ = tx.send(Err(err.clone()));
        }
    }

    /// Starts it if it is idle; true when it did.
    fn wake(self: &Arc<Self>) -> bool {
        {
            let mut inner = self.inner.lock();
            if inner.phase != Phase::Idle {
                return false;
            }
            inner.phase = Phase::Starting;
        }
        self.start();
        true
    }

    /// Sends `q` now, or once it runs; starts it if it is idle.
    fn send(self: &Arc<Self>, q: Queued) -> Result<(), CallError> {
        let mut start = false;
        {
            let mut inner = self.inner.lock();
            match &inner.phase {
                Phase::Stopped(s) => return Err(CallError::Stopped(s.clone())),
                Phase::Running => {
                    if inner.conn.as_ref().is_some_and(|c| c.write(&q.line)) {
                        return Ok(());
                    }
                    // Its input closed: it is ending, and its end says what comes next.
                }
                Phase::Idle => {
                    inner.phase = Phase::Starting;
                    start = true;
                }
                Phase::Starting | Phase::Restarting => {}
            }
            self.enqueue(&mut inner, q);
        }
        if start {
            self.start();
        }
        Ok(())
    }

    fn enqueue(&self, inner: &mut Inner, q: Queued) {
        if inner.queue.len() >= self.limits().queue {
            if let Some(old) = inner.queue.pop_front() {
                self.journal.push(
                    Kind::Dropped,
                    "the queue kept while it starts is full: the oldest message is dropped",
                );
                for id in old.ids {
                    self.answer(id, Err(CallError::Dropped));
                }
            }
        }
        inner.queue.push_back(q);
    }

    fn start(self: &Arc<Self>) {
        let me = self.clone();
        tauri::async_runtime::spawn(async move { me.run_start().await });
    }

    /// Starts its process, then `initialize`, what was attached, and what waited in the queue.
    async fn run_start(self: Arc<Self>) {
        let Some(rt) = self.rt.upgrade() else { return };
        let generation = {
            let mut inner = self.inner.lock();
            inner.generation += 1;
            inner.generation
        };
        let run = self.manifest.run.as_ref().expect("a host has a program");
        let program = rt.program_of(&self.folder, run);
        let conn = match self.spawn(&rt, &program, generation) {
            Ok(Some(c)) => c,
            // Stopped meanwhile.
            Ok(None) => return,
            Err(e) => {
                self.stop_with(Stop::Refused {
                    reason: format!("cannot start {}: {e}", program.display()),
                });
                return;
            }
        };
        self.journal.push(
            Kind::Start,
            format!(
                "started {} (pid {})",
                program.display(),
                conn.pid.map(|p| p.to_string()).unwrap_or_default()
            ),
        );
        let hello = json!({
            "protocolVersion": PROTOCOL,
            "escouade": { "version": rt.app_version },
            "plugin": { "id": self.id, "version": self.manifest.version },
            "capabilities": { "batch": true, "secrets": true, "log": true, "attach": true },
        });
        let answer = match self
            .call_on(&conn, "initialize", &raw(&hello), rt.limits.initialize)
            .await
        {
            Ok(a) => a,
            Err(CallError::Timeout) => {
                self.stop_with(Stop::Refused {
                    reason: format!(
                        "no answer to initialize within {} s",
                        rt.limits.initialize.as_secs_f32()
                    ),
                });
                return;
            }
            Err(CallError::Remote { message, .. }) => {
                self.stop_with(Stop::Refused {
                    reason: format!("initialize refused: {message}"),
                });
                return;
            }
            // It ended: its end says what comes next.
            Err(_) => return,
        };
        let answer: Value = serde_json::from_str(answer.get()).unwrap_or(Value::Null);
        let version = answer.get("protocolVersion").and_then(Value::as_u64);
        if version != Some(PROTOCOL) {
            let said = version.map_or("no version".to_string(), |v| format!("version {v}"));
            self.stop_with(Stop::Refused {
                reason: format!(
                    "it speaks {said} of the protocol; this Escouade speaks version {PROTOCOL}"
                ),
            });
            return;
        }
        let capabilities = answer.get("capabilities").cloned().unwrap_or(json!({}));
        self.journal.push(
            Kind::Ready,
            format!("initialized: protocol {PROTOCOL}, capabilities {capabilities}"),
        );
        let attached: Vec<(String, Box<RawValue>)> =
            self.inner.lock().attached.values().cloned().collect();
        for (method, params) in attached {
            if let Err(e) = self
                .call_on(&conn, &method, &params, timeout_of(&method))
                .await
            {
                self.journal
                    .push(Kind::Refused, format!("{method} given again: {e}"));
            }
        }
        let mut inner = self.inner.lock();
        let current = inner.generation == generation
            && matches!(inner.phase, Phase::Starting | Phase::Restarting);
        if !current {
            return;
        }
        inner.capabilities = capabilities;
        for q in inner.queue.drain(..) {
            conn.write(&q.line);
        }
        inner.phase = Phase::Running;
    }

    /// A call straight on `conn`, past the queue (`initialize`, what is given again).
    async fn call_on(
        &self,
        conn: &Conn,
        method: &str,
        params: &RawValue,
        within: Duration,
    ) -> Answer {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let rx = self.register(id);
        if !conn.write(&request_line(id, method, params)) {
            self.pending.lock().remove(&id);
            return Err(CallError::Crashed);
        }
        self.wait(id, method, rx, within).await
    }

    /// Starts the process, in its job, with what reads its outputs and watches its memory; none
    /// when it was stopped meanwhile.
    fn spawn(
        self: &Arc<Self>,
        rt: &Runtime,
        program: &Path,
        generation: u64,
    ) -> std::io::Result<Option<Arc<Conn>>> {
        let run = self.manifest.run.as_ref().expect("a host has a program");
        let mut cmd = Command::new(program);
        cmd.args(&run.args)
            .current_dir(&self.folder)
            .env("ESCOUADE_PLUGIN_ID", &self.id)
            .env("ESCOUADE_PLUGIN_API", PROTOCOL.to_string())
            .env("ESCOUADE_VERSION", &rt.app_version)
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
        if job.is_none() {
            log::warn!(
                "plugin {} (pid {:?}) is not in a job: its tree is not killed as a whole",
                self.id,
                child.id()
            );
        }
        let mut stdin = child.stdin.take().expect("piped");
        let stdout = child.stdout.take().expect("piped");
        let stderr = child.stderr.take().expect("piped");
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let (kill_tx, mut kill_rx) = oneshot::channel::<()>();
        let conn = Arc::new(Conn {
            generation,
            pid: child.id(),
            tx: Mutex::new(Some(tx)),
            job,
            kill_tx: Mutex::new(Some(kill_tx)),
            exited: AtomicBool::new(false),
        });
        // Its process of now before anything reads it: an end at once is this one's.
        {
            let mut inner = self.inner.lock();
            if inner.generation != generation
                || !matches!(inner.phase, Phase::Starting | Phase::Restarting)
            {
                // Its job and `kill_on_drop` end it.
                return Ok(None);
            }
            inner.conn = Some(conn.clone());
        }

        tauri::async_runtime::spawn(async move {
            while let Some(bytes) = rx.recv().await {
                if stdin.write_all(&bytes).await.is_err() || stdin.flush().await.is_err() {
                    break;
                }
            }
        });

        let tail = Arc::new(Mutex::new(VecDeque::<String>::new()));
        let err_done = {
            let (done_tx, done_rx) = oneshot::channel::<()>();
            let host = self.clone();
            let tail = tail.clone();
            tauri::async_runtime::spawn(async move {
                let mut lines = BufReader::new(stderr).split(b'\n');
                while let Ok(Some(line)) = lines.next_segment().await {
                    let line = String::from_utf8_lossy(&line)
                        .trim_end_matches('\r')
                        .to_string();
                    host.journal.push(Kind::Stderr, line.clone());
                    let mut t = tail.lock();
                    t.push_back(line);
                    while t.len() > TAIL {
                        t.pop_front();
                    }
                }
                let _ = done_tx.send(());
            });
            done_rx
        };

        {
            let host = self.clone();
            let conn = conn.clone();
            tauri::async_runtime::spawn(async move {
                let mut lines = BufReader::new(stdout).split(b'\n');
                loop {
                    tokio::select! {
                        next = lines.next_segment() => match next {
                            Ok(Some(bytes)) => host.incoming(&conn, &String::from_utf8_lossy(&bytes)),
                            _ => break,
                        },
                        _ = &mut kill_rx => {
                            let _ = child.start_kill();
                            break;
                        }
                    }
                }
                let code = child.wait().await.ok().and_then(|s| s.code());
                // What it wrote last on its standard error, before its end is told.
                let _ = tokio::time::timeout(Duration::from_millis(500), err_done).await;
                conn.exited.store(true, Ordering::Release);
                let said: Vec<String> = tail.lock().iter().cloned().collect();
                host.exited(&conn, code, said);
            });
        }

        {
            let host = self.clone();
            let conn = conn.clone();
            let limits = rt.limits.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(limits.sample).await;
                    if conn.exited.load(Ordering::Acquire) {
                        break;
                    }
                    // `ps` on macOS and Linux: off the runtime's threads.
                    let c = conn.clone();
                    let Ok(Some(u)) = tokio::task::spawn_blocking(move || c.usage()).await else {
                        continue;
                    };
                    if u.memory > limits.memory {
                        if host.is_current(&conn) {
                            host.stop_with(Stop::Memory {
                                bytes: u.memory,
                                limit: limits.memory,
                            });
                        }
                        break;
                    }
                }
            });
        }
        Ok(Some(conn))
    }

    fn is_current(&self, conn: &Conn) -> bool {
        self.inner
            .lock()
            .conn
            .as_ref()
            .is_some_and(|c| c.generation == conn.generation)
    }

    /// A line of its output: a message, or a batch of them.
    fn incoming(self: &Arc<Self>, conn: &Arc<Conn>, text: &str) {
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        if text.starts_with('[') {
            match serde_json::from_str::<Vec<Box<RawValue>>>(text) {
                Ok(batch) => {
                    for m in batch {
                        self.message(conn, m.get());
                    }
                }
                Err(e) => self
                    .journal
                    .push(Kind::Refused, format!("an unreadable batch ({e})")),
            }
        } else {
            self.message(conn, text);
        }
    }

    fn message(self: &Arc<Self>, conn: &Arc<Conn>, text: &str) {
        let msg: Incoming = match serde_json::from_str(text) {
            Ok(m) => m,
            Err(e) => {
                self.journal.push(
                    Kind::Refused,
                    format!("an unreadable message ({e}), {} bytes", text.len()),
                );
                return;
            }
        };
        match (msg.method, msg.id) {
            (None, Some(id)) => {
                let Some(id) = id.as_u64() else { return };
                let answer = match msg.error {
                    Some(e) => Err(CallError::Remote {
                        code: e.code,
                        message: e.message,
                    }),
                    None => Ok(msg.result.unwrap_or_else(null)),
                };
                self.answer(id, answer);
            }
            (Some(method), Some(id)) => self.request(conn, &method, id, msg.params),
            (Some(method), None) => self.notification(&method, msg.params),
            (None, None) => match msg.error {
                Some(e) => self.journal.push(
                    Kind::Message,
                    format!("an error without id: {} ({})", e.message, e.code),
                ),
                None => self
                    .journal
                    .push(Kind::Refused, "a message with neither method nor id"),
            },
        }
    }

    /// A request of the plugin to Escouade.
    fn request(&self, conn: &Conn, method: &str, id: Value, params: Option<Box<RawValue>>) {
        let params: Value = params
            .and_then(|p| serde_json::from_str(p.get()).ok())
            .unwrap_or(Value::Null);
        let answer: Result<Value, (i64, String)> = match method {
            "secrets/get" => match params.get("key").and_then(Value::as_str) {
                Some(key) => self.secret(key).map(|s| json!(s)),
                None => Err((INVALID_PARAMS, "secrets/get takes a key".into())),
            },
            _ => {
                self.journal.push(
                    Kind::Refused,
                    format!("asked for {method}, which Escouade does not serve"),
                );
                Err((METHOD_NOT_FOUND, format!("unknown method {method}")))
            }
        };
        let line = match answer {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err((code, message)) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": code, "message": message },
            }),
        };
        conn.write(&line.to_string());
    }

    /// Its secret `key`: only one of its own (`permissions.secrets`), under its own name.
    fn secret(&self, key: &str) -> Result<Option<String>, (i64, String)> {
        let own = !key.is_empty()
            && !key.chars().any(char::is_control)
            && self
                .manifest
                .permissions
                .secrets
                .iter()
                .any(|p| matches(p, key));
        if !own {
            self.journal.push(
                Kind::Refused,
                format!("asked for the secret {key:?}, which is not one of its own"),
            );
            return Err((
                NOT_YOURS,
                format!("the secret {key:?} is not one of this plugin's (permissions.secrets)"),
            ));
        }
        let Some(rt) = self.rt.upgrade() else {
            return Err((KEYCHAIN, "Escouade is closing".into()));
        };
        match rt.secrets.get(&secret_name(&self.id, key)) {
            Ok(Some(s)) => {
                self.journal.mask(&s);
                self.journal
                    .push(Kind::Message, format!("was given its secret {key:?}"));
                Ok(Some(s))
            }
            Ok(None) => {
                self.journal.push(
                    Kind::Message,
                    format!("asked for its secret {key:?}: none is kept"),
                );
                Ok(None)
            }
            Err(e) => {
                log::warn!("plugin {}: the keychain: {e}", self.id);
                Err((KEYCHAIN, "the keychain is out of reach".into()))
            }
        }
    }

    /// A notification of the plugin: a line of its journal (`log`).
    fn notification(&self, method: &str, params: Option<Box<RawValue>>) {
        let params: Value = params
            .and_then(|p| serde_json::from_str(p.get()).ok())
            .unwrap_or(Value::Null);
        match method {
            "log" => {
                let level = params
                    .get("level")
                    .and_then(Value::as_str)
                    .unwrap_or("info");
                let message = params.get("message").and_then(Value::as_str).unwrap_or("");
                self.journal
                    .push(Kind::Message, format!("[{level}] {message}"));
            }
            _ => self
                .journal
                .push(Kind::Message, format!("a notification {method}")),
        }
    }

    /// Its process ended: asked to (stopped), or a crash, which starts it again while it may.
    fn exited(self: &Arc<Self>, conn: &Conn, code: Option<i32>, tail: Vec<String>) {
        let limits = self.limits();
        let quitting = self
            .rt
            .upgrade()
            .is_none_or(|rt| rt.quitting.load(Ordering::Acquire));
        let mut restart = None;
        let mut stopped = None;
        let kept: HashSet<u64>;
        {
            let mut inner = self.inner.lock();
            // Only the process of now: one stopped (its connection taken) was dealt with already.
            if !inner
                .conn
                .as_ref()
                .is_some_and(|c| c.generation == conn.generation)
            {
                return;
            }
            inner.conn = None;
            let now = Instant::now();
            while inner
                .restarts
                .front()
                .is_some_and(|t| now.duration_since(*t) > limits.window)
            {
                inner.restarts.pop_front();
            }
            if quitting {
                inner.phase = Phase::Stopped(Stop::Shutdown);
            } else if inner.restarts.len() >= limits.restarts {
                let stop = Stop::Crashed {
                    times: inner.restarts.len() + 1,
                };
                inner.phase = Phase::Stopped(stop.clone());
                stopped = Some(stop);
            } else {
                inner.restarts.push_back(now);
                inner.phase = Phase::Restarting;
                restart = Some(inner.restarts.len());
            }
            kept = if restart.is_some() {
                inner.queue.iter().flat_map(|q| q.ids.clone()).collect()
            } else {
                inner.queue.clear();
                HashSet::new()
            };
        }
        let code = code.map_or("no code".to_string(), |c| format!("code {c}"));
        let said = if tail.is_empty() {
            String::new()
        } else {
            format!(": {}", tail.join(" | "))
        };
        self.journal.push(
            Kind::Crash,
            format!("ended ({code}) without being asked to{said}"),
        );
        self.fail_all(CallError::Crashed, &kept);
        if let Some(stop) = stopped {
            self.journal.push(Kind::Stop, stop.describe(limits.window));
        }
        if let Some(n) = restart {
            self.journal.push(
                Kind::Start,
                format!(
                    "starting again ({n} of {} within {} minutes)",
                    limits.restarts,
                    limits.window.as_secs().div_ceil(60)
                ),
            );
            self.start();
        }
    }

    /// Stops its process for `stop`: what waits fails, and it does not start again by itself.
    fn stop_with(&self, stop: Stop) {
        let conn = {
            let mut inner = self.inner.lock();
            if matches!(inner.phase, Phase::Stopped(_)) && inner.conn.is_none() {
                return;
            }
            inner.phase = Phase::Stopped(stop.clone());
            inner.queue.clear();
            inner.conn.take()
        };
        self.journal
            .push(Kind::Stop, stop.describe(self.limits().window));
        if let Some(c) = conn {
            c.kill();
        }
        self.fail_all(CallError::Stopped(stop), &HashSet::new());
    }

    /// Marks it stopped for Escouade's closing, and asks a running process to `shutdown`: gives
    /// the process to end and where its answer comes.
    #[allow(clippy::type_complexity)]
    fn begin_shutdown(&self) -> (Option<Arc<Conn>>, Option<oneshot::Receiver<Answer>>) {
        let (conn, running) = {
            let mut inner = self.inner.lock();
            let running = inner.phase == Phase::Running;
            if !matches!(inner.phase, Phase::Stopped(_)) {
                inner.phase = Phase::Stopped(Stop::Shutdown);
            }
            inner.queue.clear();
            (inner.conn.take(), running)
        };
        let Some(conn) = conn else {
            return (None, None);
        };
        if !running {
            return (Some(conn), None);
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let rx = self.register(id);
        if !conn.write(&request_line(id, "shutdown", &null())) {
            self.pending.lock().remove(&id);
            return (Some(conn), None);
        }
        (Some(conn), Some(rx))
    }
}
