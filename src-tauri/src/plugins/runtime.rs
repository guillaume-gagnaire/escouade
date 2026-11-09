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
    /// What the instance of a `run.js` plugin may take (its memory, its turns).
    pub wasm: super::wasm::Limits,
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
            wasm: super::wasm::Limits::default(),
        }
    }
}

/// The delay of a call to `method`: short for a decision that holds an agent, long for an import.
pub fn timeout_of(method: &str) -> Duration {
    let ms = match method {
        "initialize" => 10_000,
        "shutdown" => 1_000,
        m if m.starts_with("permissions/") => 500,
        // isola's own limit (60 s) comes first, with its message.
        m if m.starts_with("launch/") => 65_000,
        m if m.starts_with("tickets/")
            || m.starts_with("forge/")
            || m.starts_with("workflow/")
            || m.starts_with("voice/")
            || m == "account/me"
            || m == "engine/oneShot" =>
        {
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

/// The program of `run` for this system: in the plugin's `folder`, or in `escouade_dir` (the
/// folder of the app's programs) for `{escouade}/…`. Always absolute: the process starts in the
/// plugin's folder, and a relative program would be looked for from there under Unix
/// (`escouade-plugin test slack` would start `slack/slack/slack`).
pub(crate) fn program_path(folder: &Path, run: &manifest::Run, escouade_dir: &Path) -> PathBuf {
    let folder = std::path::absolute(folder).unwrap_or_else(|_| folder.to_path_buf());
    let escouade_dir =
        std::path::absolute(escouade_dir).unwrap_or_else(|_| escouade_dir.to_path_buf());
    let sys = if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };
    let p = run.by_system.get(sys).unwrap_or(&run.command);
    match p.strip_prefix(manifest::ESCOUADE_DIR) {
        Some(rest) => escouade_dir.join(rest.trim_start_matches(['/', '\\'])),
        None => folder.join(p),
    }
}

/// Whether the program of `run` for this system is one of the app's (`{escouade}/…`).
fn app_program(run: &manifest::Run) -> bool {
    let sys = if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };
    run.by_system
        .get(sys)
        .unwrap_or(&run.command)
        .starts_with(manifest::ESCOUADE_DIR)
}

/// Whether `word` is one of `patterns`: the same, or one a `prefix*` takes in.
pub(crate) fn matches(pattern: &str, word: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => word.starts_with(prefix),
        None => pattern == word,
    }
}

/// A request on one line. `params` is raw JSON: a newline in it can only be whitespace (a string
/// of JSON holds none), so it is made a space.
pub(crate) fn request_line(id: u64, method: &str, params: &RawValue) -> String {
    let method = serde_json::to_string(method).unwrap_or_else(|_| "\"\"".into());
    let params = one_line(params.get());
    format!(r#"{{"jsonrpc":"2.0","id":{id},"method":{method},"params":{params}}}"#)
}

pub(crate) fn notification_line(method: &str, params: &RawValue) -> String {
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

/// A line for a plugin's input, and who waits for it to be written.
type Frame = super::wasm::Frame;

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
    tx: Mutex<Option<mpsc::UnboundedSender<Frame>>>,
    job: Option<Job>,
    kill_tx: Mutex<Option<oneshot::Sender<()>>>,
    exited: AtomicBool,
}

impl Conn {
    /// Writes `line` on its input; false when it is closed.
    fn write(&self, line: &str) -> bool {
        self.write_then(line, None)
    }

    /// Writes `line` on its input; `written` is told once it was (dropped if it never is).
    fn write_then(&self, line: &str, written: Option<oneshot::Sender<()>>) -> bool {
        let mut bytes = Vec::with_capacity(line.len() + 1);
        bytes.extend_from_slice(line.as_bytes());
        bytes.push(b'\n');
        self.tx
            .lock()
            .as_ref()
            .is_some_and(|tx| tx.send((bytes, written)).is_ok())
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
    journal: Arc<Journal>,
    /// The agents it was given, for `fs/*` and `proc/*`.
    agents: super::files::Agents,
}

/// The plugins' processes of the app.
pub struct Runtime {
    plugins: Arc<Plugins>,
    secrets: Arc<dyn SecretStore>,
    logs: PathBuf,
    /// The temporary folders of the plugins, one each.
    temps: PathBuf,
    app_version: String,
    /// The folder of the app's programs (`{escouade}` of a plugin of its own).
    escouade_dir: PathBuf,
    limits: Limits,
    hosts: Mutex<HashMap<String, Arc<Host>>>,
    /// Every plugin's journal, a declarative one's too (its webhooks, its commands).
    journals: Mutex<HashMap<String, Arc<Journal>>>,
    quitting: AtomicBool,
    /// Where the projects are (the core): what `settings/get` reads of a project's file.
    place: parking_lot::RwLock<Option<Weak<dyn super::events::Place>>>,
    /// What each account has left of its calls, as its plugin's last answer said it (INT-6): by
    /// `accounts::budget_key`, shared by all the points of the plugin that serves the account.
    budgets: Mutex<HashMap<String, crate::forge::RateLimit>>,
    /// Who listens to the notifications a plugin sends beyond `log` (`on_notification`): the voice services' text and
    /// sound while they work (INT-13).
    listeners: Mutex<HashMap<String, Listener>>,
    /// The modules of the `run.js` plugins, in the app's cache (ESC-320).
    modules: Arc<super::wasm::Modules>,
    /// What the tests change of `net/fetch`'s network.
    net: parking_lot::RwLock<super::wasm::Net>,
    /// The secrets the core lends a plugin the app gives, by plugin and key (`account:github.com`): the token of an
    /// saved account of the first hand as the core holds it, the user's or its CLI's (`gh auth token`), read before
    /// the keychain. Never another plugin's.
    lent: Mutex<HashMap<(String, String), String>>,
    /// What the tries of a connection under way lend, the latest last (`Trial`).
    tried: Mutex<Vec<Tried>>,
    /// The number of the next `Trial`.
    trials: AtomicU64,
    /// What a plugin the app gives is told when it starts (`ESCOUADE_LANG`, the network of Réglages › Avancé › Réseau).
    builtin_env: parking_lot::RwLock<Vec<(String, String)>>,
    /// What a program of a plugin gets as an agent does (`set_agent_env`).
    agent_env: parking_lot::RwLock<Vec<(String, String)>>,
    me: Weak<Self>,
}

/// What a try of a connection lends a plugin.
struct Tried {
    trial: u64,
    plugin: String,
    key: String,
    secret: String,
    host: String,
}

/// The try of a connection, whose account is not saved yet: the secret and the host it lends end with it, won, lost or
/// dropped half way. Each try has its own, so two tries at once, or an account that runs on the same secret, keep theirs.
pub struct Trial {
    rt: Weak<Runtime>,
    number: u64,
}

impl Trial {
    /// Lends the plugin `plugin` the secret `key` and the host the user typed (`{account}` reaches it, empty for none).
    pub fn lend(&self, plugin: &str, key: &str, secret: &str, host: &str) {
        if let Some(rt) = self.rt.upgrade() {
            rt.tried.lock().push(Tried {
                trial: self.number,
                plugin: plugin.to_string(),
                key: key.to_string(),
                secret: secret.trim().to_string(),
                host: host.to_string(),
            });
        }
    }

    /// Won: its account is saved, and its secrets go on lent for it.
    pub fn keep(self) {
        if let Some(rt) = self.rt.upgrade() {
            for t in rt.tried.lock().iter().filter(|t| t.trial == self.number) {
                rt.lend(&t.plugin, &t.key, &t.secret);
            }
        }
    }
}

impl Drop for Trial {
    fn drop(&mut self) {
        if let Some(rt) = self.rt.upgrade() {
            rt.tried.lock().retain(|t| t.trial != self.number);
        }
    }
}

/// What listens to a plugin's notifications.
type Listener = Arc<dyn Fn(&str, Value) + Send + Sync>;

impl Runtime {
    /// Listens to the notifications of the plugin `id` (but `log`), in place of the one before. The listener runs on
    /// the thread that reads the plugin's output, in the order the plugin wrote: it must not wait.
    pub fn on_notification(&self, id: &str, f: impl Fn(&str, Value) + Send + Sync + 'static) {
        self.listeners.lock().insert(id.to_string(), Arc::new(f));
    }

    /// What the account `key` (`accounts::budget_key`) has left of its calls, as last said.
    pub fn budget(&self, key: &str) -> Option<crate::forge::RateLimit> {
        self.budgets.lock().get(key).copied()
    }

    /// The account `key` has `left` of its calls: whichever of its plugin's points was answered.
    pub fn spend(&self, key: &str, left: crate::forge::RateLimit) {
        self.budgets.lock().insert(key.to_string(), left);
    }

    /// Lends the plugin of the app `id` its secret `key` (`account:<host>`) for a saved account: what `secrets/get`
    /// gives it first, until the account goes (`take_back`). An empty one takes the loan back.
    pub fn lend(&self, id: &str, key: &str, secret: &str) {
        let at = (id.to_string(), key.to_string());
        if secret.trim().is_empty() {
            self.lent.lock().remove(&at);
        } else {
            self.lent.lock().insert(at, secret.trim().to_string());
        }
    }

    /// The account of the secret `key` went: no plugin keeps it. What a try under way lends ends with that try.
    pub fn take_back(&self, key: &str) {
        self.lent.lock().retain(|(_, k), _| k != key);
    }

    /// The try of a connection: what it lends ends with it.
    pub fn trial(&self) -> Trial {
        Trial {
            rt: self.me.clone(),
            number: self.trials.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// The hosts the tries under way lend the plugin `id`: a saved account's needs no loan (`accounts::hosts_of`).
    pub fn lent_hosts(&self, id: &str) -> Vec<String> {
        let mut hosts: Vec<String> = self
            .tried
            .lock()
            .iter()
            .filter(|t| t.plugin == id && !t.host.is_empty())
            .map(|t| t.host.clone())
            .collect();
        hosts.sort();
        hosts.dedup();
        hosts
    }

    /// The secret `key` lent to the plugin `id`: the latest try's while one lends it, else the saved account's.
    pub fn lent_secret(&self, id: &str, key: &str) -> Option<String> {
        let tried = self.tried.lock();
        let trying = tried
            .iter()
            .rev()
            .find(|t| t.plugin == id && t.key == key && !t.secret.is_empty());
        if let Some(t) = trying {
            return Some(t.secret.clone());
        }
        drop(tried);
        let at = (id.to_string(), key.to_string());
        self.lent.lock().get(&at).cloned()
    }

    /// The network its plugins get.
    #[cfg(test)]
    pub fn net(&self) -> super::wasm::Net {
        self.net.read().clone()
    }

    /// What a plugin the app gives is told when it starts: the interface's language and the network's settings. A
    /// change applies to its next process: the core stops the running ones (`forget`).
    /// What a program of a plugin gets as an agent does (`proc/spawn`): the network of Réglages.
    pub fn set_agent_env(&self, env: Vec<(String, String)>) {
        *self.agent_env.write() = env;
    }

    pub fn set_builtin_env(&self, env: Vec<(String, String)>) {
        *self.builtin_env.write() = env;
    }

    pub fn builtin_env(&self) -> Vec<(String, String)> {
        self.builtin_env.read().clone()
    }

    /// The language `initialize` says: the one the plugins the app gives are told, or the backend's. A module has no
    /// environment, and a plugin of a third party gets none of `builtin_env` (docs/PLUGINS.md §8.4.8).
    fn lang(&self) -> String {
        let env = self.builtin_env.read();
        let said = env.iter().find(|(k, _)| k == "ESCOUADE_LANG");
        said.map_or_else(
            || crate::i18n::kit(crate::i18n::ui()).code().to_string(),
            |(_, code)| code.clone(),
        )
    }

    /// The core, once made: the projects whose settings a plugin may ask for.
    pub fn place_with(&self, p: Weak<dyn super::events::Place>) {
        *self.place.write() = Some(p);
    }

    /// The settings of the plugin `id` (`settings/get`): the user's, and in the project `project`
    /// those its `.escouade.yml` gives it (PLG-14), only where the plugin is on (a private project
    /// gives nothing to a plugin not turned on for it).
    pub fn settings_of(&self, id: &str, project: Option<&str>) -> Value {
        let Some(choices) = self.plugins.choices(id) else {
            return json!({});
        };
        let from_file = project.and_then(|p| {
            let place = self.place.read().as_ref().and_then(Weak::upgrade)?;
            let (private, file) = place.project_plugin(p, id)?;
            super::active_in(&choices, p, private).then_some(file)
        });
        let manifest = self.plugins.get(id, Lang::En).and_then(|v| v.manifest);
        match (from_file, manifest) {
            (Some(file), Some(m)) => {
                Value::Object(super::project_settings(&m, &choices.settings, &file))
            }
            _ => Value::Object(choices.settings),
        }
    }

    pub fn new(
        plugins: Arc<Plugins>,
        secrets: Arc<dyn SecretStore>,
        data: &DataDir,
        app_version: &str,
        limits: Limits,
    ) -> Arc<Self> {
        let escouade_dir = plugins.app_dir().to_path_buf();
        let rt = Arc::new_cyclic(|me| Self {
            plugins: plugins.clone(),
            secrets,
            logs: data.plugin_logs_dir(),
            temps: data.plugin_temp_dir(),
            app_version: app_version.to_string(),
            escouade_dir,
            limits,
            hosts: Mutex::new(HashMap::new()),
            journals: Mutex::new(HashMap::new()),
            budgets: Mutex::new(HashMap::new()),
            listeners: Mutex::new(HashMap::new()),
            quitting: AtomicBool::new(false),
            place: parking_lot::RwLock::new(None),
            modules: Arc::new(super::wasm::Modules::new(data, app_version)),
            net: parking_lot::RwLock::new(super::wasm::Net::default()),
            lent: Mutex::new(HashMap::new()),
            tried: Mutex::new(Vec::new()),
            trials: AtomicU64::new(0),
            builtin_env: parking_lot::RwLock::new(Vec::new()),
            agent_env: parking_lot::RwLock::new(Vec::new()),
            me: me.clone(),
        });
        // A folder in development read again: its process stops, and the next one is given the
        // manifest of now (its permissions of secrets included), at once if it decides.
        let weak = Arc::downgrade(&rt);
        plugins.on_reload(move |id| {
            if let Some(rt) = weak.upgrade() {
                rt.forget(id);
                super::decide::warm_one(&rt.plugins, &rt, id);
            }
        });
        rt
    }

    /// The modules of the plugins in WASM.
    pub fn modules(&self) -> &Arc<super::wasm::Modules> {
        &self.modules
    }

    /// Réglages › Avancé › Réseau, for the plugins in WASM: their next call follows it.
    pub fn set_network(&self, network: super::socket::Net) {
        *self.net.read().network.write() = network;
    }

    /// Who tells the hosts the user typed for the accounts of a plugin (`{account}`).
    pub fn set_accounts(&self, accounts: super::wasm::HostsOf) {
        self.net.write().accounts = Some(accounts);
    }

    /// The fake services of a build of development or a test (`Bases::faked`), for the instances to come.
    #[cfg(any(debug_assertions, test))]
    pub fn set_routes(&self, routes: std::collections::HashMap<String, String>) {
        self.net.write().routes = routes;
    }

    /// The tests' network for `net/fetch` (a fake service, the resolver's answer), for the instances to come.
    #[cfg(test)]
    pub fn set_net(&self, net: super::wasm::Net) {
        *self.net.write() = net;
    }

    /// Makes the module of the plugin in WASM `id` ready in the cache, on a thread of its own (its
    /// installation, an update of the app): none for a plugin of another kind.
    pub fn prepare(
        &self,
        id: &str,
    ) -> Option<std::thread::JoinHandle<Result<super::wasm::Made, String>>> {
        let view = self.plugins.get(id, Lang::En)?;
        let run = view.manifest?.run.filter(|r| r.in_wasm().is_some())?;
        let folder = PathBuf::from(&view.folder);
        let modules = self.modules.clone();
        let journal = self.journal_of(id);
        let id = id.to_string();
        std::thread::Builder::new()
            .name(format!("plugin {id}: its module"))
            .spawn(move || {
                let made = modules.of(&folder, &run).map(|(_, made)| made);
                match &made {
                    Ok(super::wasm::Made::Cache) => {}
                    Ok(m) => journal.push(Kind::Message, format!("ready: {}", m.describe())),
                    Err(e) => {
                        log::warn!("plugin {id}: its module: {e}");
                        journal.push(Kind::Refused, format!("its module: {e}"));
                    }
                }
                made
            })
            .ok()
    }

    /// Makes ready the modules of every plugin in WASM installed, those of another version of the app taken
    /// out of the cache: at the start of the app, once its version may have changed.
    pub fn prepare_all(&self) {
        self.modules.prune();
        for v in self.plugins.list(Lang::En) {
            let in_wasm = v
                .manifest
                .as_ref()
                .and_then(|m| m.run.as_ref())
                .is_some_and(|r| r.in_wasm().is_some());
            // A plugin of the seed is precompiled at its first activation: a start does not pay for those that do not serve.
            // Those of the registry in a module are the seed to come (a harness's is first used by its first agent).
            let seeded = matches!(
                v.origin,
                super::Origin::Store { seed: true, .. } | super::Origin::Builtin
            );
            if v.state == Loaded::Ready && in_wasm && !seeded {
                if let Some(h) = self.prepare(&v.id) {
                    let _ = h.join();
                }
            }
        }
    }

    /// The plugin `id`'s host, made at its first use: only a plugin that loaded, with a program.
    fn host(&self, id: &str) -> Result<Arc<Host>, CallError> {
        if let Some(h) = self.hosts.lock().get(id) {
            return Ok(h.clone());
        }
        let (folder, manifest) = self.runnable(id)?;
        let host = Arc::new(Host {
            id: id.to_string(),
            folder,
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
            journal: self.journal_of(id),
            agents: Arc::default(),
        });
        Ok(self
            .hosts
            .lock()
            .entry(id.to_string())
            .or_insert(host)
            .clone())
    }

    /// Why the plugin `id` cannot run now (switched off, not installed, not loaded, the app quitting), if it cannot;
    /// nothing is made for it.
    pub fn refusal(&self, id: &str) -> Option<CallError> {
        if self.hosts.lock().contains_key(id) {
            return None;
        }
        self.runnable(id).err()
    }

    /// The folder and the granted manifest of the plugin `id`, when it may run.
    fn runnable(&self, id: &str) -> Result<(PathBuf, Manifest), CallError> {
        if self.quitting.load(Ordering::Acquire) {
            return Err(CallError::Stopped(Stop::Shutdown));
        }
        let view = self.plugins.get(id, Lang::En).ok_or_else(|| {
            CallError::Unavailable(if self.plugins.uninstalled(id) {
                tr!(
                    "il n’est plus installé : la vue Plugins propose de l’installer.",
                    "it is no longer installed: the Plugins view offers to install it."
                )
            } else {
                format!("{id} is not installed")
            })
        })?;
        match &view.state {
            Loaded::Ready => {}
            Loaded::Disabled { reason } | Loaded::Invalid { reason, .. } => {
                return Err(CallError::Unavailable(reason.clone()))
            }
        }
        if !view.choices.enabled {
            return Err(CallError::Unavailable(format!("{id} is switched off")));
        }
        // What it is given: its permissions but those the user took back.
        let manifest = view
            .manifest
            .filter(|m| m.run.is_some())
            .map(|m| super::granted(&m, &view.choices.revoked))
            .ok_or_else(|| CallError::Unavailable(format!("{id} is declarative: nothing runs")))?;
        Ok((PathBuf::from(&view.folder), manifest))
    }

    /// Starts the plugins `word` activates (`event:ticket.column`, `account:github`), those that
    /// do not run yet; gives their ids.
    pub fn activate(&self, word: &str) -> Vec<String> {
        let mut started = Vec::new();
        for v in self.plugins.list(Lang::En) {
            let Some(m) = &v.manifest else { continue };
            // Off for the app: nothing of it runs.
            if v.state != Loaded::Ready
                || !v.choices.enabled
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

    /// An instance of the module of the plugin `id`, for its views (`plugins/view.rs`): greeted, and asking nothing of the
    /// machine.
    pub fn view_instance(&self, id: &str) -> Result<super::wasm::Instance, String> {
        let (folder, manifest) = self.runnable(id).map_err(|e| e.to_string())?;
        let run = manifest.run.as_ref().ok_or("no program")?;
        if run.in_wasm().is_none() {
            return Err(format!("{id} is no module"));
        }
        let (module, _) = self.modules.of(&folder, run)?;
        let limits = self.limits.wasm.asked(manifest.limits);
        let files = super::wasm::files_of(&folder);
        let mut instance = super::wasm::Instance::start_with(&module, &files, limits, None)
            .map_err(|f| f.to_string())?;
        let hello = json!({
            "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {
                "protocolVersion": PROTOCOL,
                "escouade": { "version": self.app_version },
                "plugin": { "id": id, "version": manifest.version },
                "lang": self.lang(),
                "capabilities": {},
            },
        });
        instance.push(format!("{hello}\n").as_bytes());
        instance.turn(limits.turn).map_err(|f| f.to_string())?;
        Ok(instance)
    }

    /// A crash of the instance of the plugin `id`, which has no process to kill: as a fault of its module.
    #[cfg(test)]
    pub fn crash_instance(&self, id: &str) {
        let Some(h) = self.hosts.lock().get(id).cloned() else {
            return;
        };
        let conn = h.inner.lock().conn.clone();
        if let Some(c) = conn {
            c.kill();
            h.exited(&c, None, vec!["a crash of the tests".to_string()]);
        }
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

    /// What the plugin `id`'s process uses now (its job's memory and CPU), when it runs.
    pub fn usage(&self, id: &str) -> Option<JobUsage> {
        let h = self.hosts.lock().get(id).cloned()?;
        let conn = h.inner.lock().conn.clone()?;
        conn.usage()
    }

    /// The plugin `id`'s journal, the oldest entry first: what its process wrote since Escouade
    /// started, or, before it ran, what its file kept of the earlier runs.
    pub fn journal(&self, id: &str) -> Vec<Entry> {
        let held = self.journals.lock().get(id).map(|j| j.entries());
        match held {
            Some(e) if !e.is_empty() => e,
            _ => super::journal::read_file(&self.journal_file(id)),
        }
    }

    /// The file of the plugin `id`'s journal.
    pub fn journal_file(&self, id: &str) -> PathBuf {
        self.logs.join(format!("{id}.log"))
    }

    /// Starts the plugin `id`'s process now, if it is idle (« Démarrer »).
    pub fn start(&self, id: &str) -> Result<(), CallError> {
        self.host(id)?.wake();
        Ok(())
    }

    /// Stops the plugin `id`'s process and forgets its host and its manifest: switched off,
    /// uninstalled, updated, a permission taken back or given again. The next use reads it anew.
    pub fn forget(&self, id: &str) {
        // Its journal too, closed once its stop is noted: the next one starts from its file.
        let journal = self.journals.lock().remove(id);
        let Some(h) = self.hosts.lock().remove(id) else {
            if let Some(j) = journal {
                j.close();
            }
            return;
        };
        let idle = {
            let inner = h.inner.lock();
            inner.conn.is_none() && matches!(inner.phase, Phase::Idle | Phase::Stopped(_))
        };
        if !idle {
            h.stop_with(Stop::Asked);
        }
        h.journal.close();
    }

    /// The plugin `id`'s journal, made at its first use.
    pub fn journal_of(&self, id: &str) -> Arc<Journal> {
        self.journals
            .lock()
            .entry(id.to_string())
            .or_insert_with(|| Arc::new(Journal::new(self.journal_file(id))))
            .clone()
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

    /// Tells the plugin `id` something, without waiting for an answer; the receiver is told once
    /// the message is on its input (the pipe took it), dropped when it waits for a restart. What
    /// sends many keeps its own queue, and sends the next once the last one was taken (`events`).
    pub fn deliver(
        &self,
        id: &str,
        method: &str,
        params: &RawValue,
    ) -> Result<oneshot::Receiver<()>, CallError> {
        let host = self.host(id)?;
        let (tx, rx) = oneshot::channel();
        host.send_then(
            Queued {
                ids: Vec::new(),
                line: notification_line(method, params),
            },
            Some(tx),
        )?;
        Ok(rx)
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

    /// `attach`, within `within` (a first call that starts the process).
    pub async fn attach_within(
        &self,
        id: &str,
        key: &str,
        method: &str,
        params: Value,
        within: Duration,
    ) -> Result<Value, CallError> {
        let host = self.host(id)?;
        host.inner
            .lock()
            .attached
            .insert(key.to_string(), (method.to_string(), raw(&params)));
        self.call_within(id, method, params, within).await
    }

    /// Forgets what `key` attached.
    pub fn detach(&self, id: &str, key: &str) {
        if let Some(h) = self.hosts.lock().get(id) {
            h.inner.lock().attached.remove(key);
            h.agents.lock().remove(key);
        }
    }

    /// How many places the plugin `id` was given and still holds.
    #[cfg(test)]
    pub fn lent_places(&self, id: &str) -> usize {
        self.hosts
            .lock()
            .get(id)
            .map_or(0, |h| h.agents.lock().len())
    }

    /// Where the agent `key` of the plugin `id` lives, as the core resolved it: its `fs/*` and `proc/*` are judged on it.
    /// The roots it declares, what `agent/attach` tells it.
    pub fn place_agent(
        &self,
        id: &str,
        key: &str,
        place: super::files::AgentPlace,
    ) -> Result<BTreeMap<String, String>, CallError> {
        let host = self.host(id)?;
        let roots = host.access(self).roots_of(&place);
        host.agents.lock().insert(key.to_string(), place);
        Ok(roots)
    }

    /// Whether the plugin `id` reaches the project `project`, by its `choices`: a private project only when it is turned
    /// on there. A project that is not known any more (one being closed) is not private.
    pub fn reaches(&self, id: &str, project: &str, choices: &super::Choices) -> bool {
        let place = self.place.read().as_ref().and_then(Weak::upgrade);
        let private = place
            .and_then(|p| p.project_plugin(project, id))
            .is_some_and(|(private, _)| private);
        super::active_in(choices, project, private)
    }

    /// The folder of the project `project` for the plugin `id`: a private project refuses a plugin not turned on for it.
    fn project_folder(&self, id: &str, project: &str) -> Result<PathBuf, String> {
        let place = self.place.read().as_ref().and_then(Weak::upgrade);
        let place = place.ok_or("Escouade is closing")?;
        let unknown = || format!("no project {project}");
        let (private, _) = place.project_plugin(project, id).ok_or_else(unknown)?;
        let choices = self.plugins.choices(id).ok_or("no such plugin")?;
        if !super::active_in(&choices, project, private) {
            return Err(format!(
                "the project {project} is private, and this plugin is not turned on for it"
            ));
        }
        place
            .project_path(project)
            .map(PathBuf::from)
            .ok_or_else(unknown)
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
        program_path(folder, run, &self.escouade_dir)
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
        self.send_then(q, None)
    }

    /// `send`, `written` told once it is on its input; dropped when it waits in the queue.
    fn send_then(
        self: &Arc<Self>,
        q: Queued,
        written: Option<oneshot::Sender<()>>,
    ) -> Result<(), CallError> {
        let mut start = false;
        {
            let mut inner = self.inner.lock();
            match &inner.phase {
                Phase::Stopped(s) => return Err(CallError::Stopped(s.clone())),
                Phase::Running => {
                    if inner
                        .conn
                        .as_ref()
                        .is_some_and(|c| c.write_then(&q.line, written))
                    {
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
        let conn = if run.in_wasm().is_some() {
            // In WASM (ESC-320, ESC-331): its module from the cache, or made now, off the runtime's threads.
            let js = run.js.as_deref().unwrap_or(super::wasm::MODULE_FILE);
            let (modules, folder, program) = (rt.modules.clone(), self.folder.clone(), run.clone());
            let asked = Instant::now();
            let prepared = tokio::task::spawn_blocking(move || modules.of(&folder, &program))
                .await
                .unwrap_or_else(|e| Err(e.to_string()));
            // How long its module took at this activation (ESC-354): the first one of a plugin of the seed precompiles it.
            let took = asked.elapsed().as_millis();
            let (module, made) = match prepared {
                Ok(m) => m,
                Err(e) => {
                    self.stop_with(Stop::Refused {
                        reason: format!("cannot make the module of {js}: {e}"),
                    });
                    return;
                }
            };
            let conn = match self.spawn_wasm(&rt, module, generation) {
                Ok(Some(c)) => c,
                Ok(None) => return,
                Err(e) => {
                    self.stop_with(Stop::Refused {
                        reason: format!("cannot start {js} in WASM: {e}"),
                    });
                    return;
                }
            };
            self.journal.push(
                Kind::Start,
                format!(
                    "started {} in WASM, isolated: {} ({took} ms)",
                    self.folder.join(js).display(),
                    made.describe()
                ),
            );
            log::info!("plugin {}: {} in {took} ms", self.id, made.describe());
            conn
        } else {
            let program = rt.program_of(&self.folder, run);
            // The app starts no program of its own but its own: that of a plugin it gives is checked before it runs (INT-4).
            if app_program(run) {
                if let Err(reason) = super::builtin::check_program(&program, &rt.escouade_dir) {
                    self.stop_with(Stop::Refused {
                        reason: format!("not the app's own program: {reason}"),
                    });
                    return;
                }
            }
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
            conn
        };
        let hello = json!({
            "protocolVersion": PROTOCOL,
            "escouade": { "version": rt.app_version },
            "plugin": { "id": self.id, "version": self.manifest.version },
            "lang": rt.lang(),
            "capabilities": { "batch": true, "secrets": true, "settings": true, "log": true, "attach": true },
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
            .envs(
                rt.builtin_env
                    .read()
                    .iter()
                    .filter(|_| super::builtin::is_builtin(&self.id))
                    .cloned(),
            )
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
        let (tx, mut rx) = mpsc::unbounded_channel::<Frame>();
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
            while let Some((bytes, written)) = rx.recv().await {
                if stdin.write_all(&bytes).await.is_err() || stdin.flush().await.is_err() {
                    break;
                }
                if let Some(w) = written {
                    let _ = w.send(());
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

    /// Its instance in WASM (`run.js`, `run.wasm`), on a thread of its own, which reads what comes for it and sends
    /// `net/fetch` on its way; none when it was stopped meanwhile. A fault of the instance ends it as a crash ends a
    /// process: `exited` makes another while its budget of restarts lasts.
    fn spawn_wasm(
        self: &Arc<Self>,
        rt: &Runtime,
        module: wasmtime::Module,
        generation: u64,
    ) -> std::io::Result<Option<Arc<Conn>>> {
        let (tx, rx) = mpsc::unbounded_channel::<Frame>();
        let back = tx.downgrade();
        let capabilities = self.capabilities(rt, back.clone());
        let conn = Arc::new(Conn {
            generation,
            pid: None,
            tx: Mutex::new(Some(tx)),
            job: None,
            kill_tx: Mutex::new(None),
            exited: AtomicBool::new(false),
        });
        {
            let mut inner = self.inner.lock();
            if inner.generation != generation
                || !matches!(inner.phase, Phase::Starting | Phase::Restarting)
            {
                return Ok(None);
            }
            inner.conn = Some(conn.clone());
        }
        let files = super::wasm::files_of(&self.folder);
        let mut net = rt.net.read().clone();
        if net.shared.is_none() {
            // As `harness.rs` keys them: the processes of this runtime.
            let owner = rt as *const Runtime as usize;
            net.shared = Some(Arc::new(move |plugin| {
                super::shared::ports_of(owner, plugin)
            }));
        }
        let fetcher = super::wasm::Fetcher::new(&self.id, &self.manifest.permissions.network, &net)
            .limited(self.manifest.limits);
        let limits = rt.limits.wasm.asked(self.manifest.limits);
        let (host, c) = (self.clone(), conn.clone());
        std::thread::Builder::new()
            .name(format!("plugin {}", self.id))
            .spawn(move || {
                let calls = Some(capabilities.calls());
                let fault = match super::wasm::Instance::start_with(&module, &files, limits, calls)
                {
                    Err(f) => Some(f),
                    Ok(instance) => {
                        let (h1, c1) = (host.clone(), c.clone());
                        let (h2, h3, h4) = (host.clone(), host.clone(), host.clone());
                        let wiring = super::wasm::Wiring {
                            rx,
                            back,
                            line: Box::new(move |l| h1.incoming(&c1, l)),
                            stderr: Box::new(move |l| h2.journal.push(Kind::Stderr, l)),
                            refused: Arc::new(move |l| h3.journal.push(Kind::Refused, l)),
                            said: Arc::new(move |l| h4.journal.push(Kind::Message, l)),
                            capabilities: Some(capabilities.clone()),
                        };
                        super::wasm::serve(instance, fetcher, wiring)
                    }
                };
                c.exited.store(true, Ordering::Release);
                // Its programs end with it.
                capabilities.end();
                // Its input closed: it was stopped, and that was dealt with.
                if let Some(f) = fault {
                    host.exited(&c, None, vec![f.to_string()]);
                }
            })?;
        Ok(Some(conn))
    }

    /// What it may reach of its folders (`permissions.files`).
    fn access(&self, rt: &Runtime) -> super::files::Access {
        super::files::Access::new(
            &self.manifest,
            &self.folder,
            rt.temps.join(&self.id),
            self.agents.clone(),
        )
    }

    /// What an instance reaches of the machine.
    fn capabilities(
        self: &Arc<Self>,
        rt: &Runtime,
        back: mpsc::WeakUnboundedSender<Frame>,
    ) -> Arc<super::wasm::Capabilities> {
        let journal = self.journal.clone();
        let told: super::procs::Told = Arc::new(move |kind, line| journal.push(kind, line));
        let (me, id) = (self.rt.clone(), self.id.clone());
        let procs = super::procs::Procs::new(super::procs::ProcsOf {
            commands: self.manifest.permissions.commands.clone(),
            temp: rt.temps.join(&self.id),
            agents: self.agents.clone(),
            projects: Arc::new(move |project| {
                me.upgrade()
                    .ok_or_else(|| "Escouade is closing".to_string())?
                    .project_folder(&id, project)
            }),
            agent_env: rt.agent_env.read().clone(),
            back,
            told: told.clone(),
        });
        super::wasm::Capabilities::new(self.access(rt), procs, told)
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
            // The values of its settings, as the user set them (Réglages of its page), with those
            // of the project's .escouade.yml when it names a project (PLG-14).
            "settings/get" => match params.get("project") {
                None | Some(Value::Null) | Some(Value::String(_)) => Ok(self
                    .rt
                    .upgrade()
                    .map(|rt| {
                        rt.settings_of(&self.id, params.get("project").and_then(Value::as_str))
                    })
                    .unwrap_or_else(|| json!({}))),
                Some(_) => Err((
                    INVALID_PARAMS,
                    "settings/get takes a project's id, or none".into(),
                )),
            },
            // A file of its own folder (`p.file`), for a process as for an instance in WASM (ESC-321).
            "files/read" => {
                super::files::read(&self.folder, &params).inspect_err(|(code, message)| {
                    if *code == super::files::REFUSED {
                        self.journal
                            .push(Kind::Refused, format!("files/read: {message}"));
                    }
                })
            }
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
        if let Some(s) = rt.lent_secret(&self.id, key) {
            self.journal.mask(&s);
            self.journal
                .push(Kind::Message, format!("was given its secret {key:?}"));
            return Ok(Some(s));
        }
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
            // What an account has left, said before a refusal (which carries none): kept as an answer's
            // `rateLimit` is (INT-8.1).
            "account/budget" => {
                let account = params.get("account").and_then(Value::as_str);
                let left = params
                    .get("rateLimit")
                    .cloned()
                    .and_then(|r| serde_json::from_value::<crate::forge::RateLimit>(r).ok());
                if let (Some(account), Some(left), Some(rt)) = (account, left, self.rt.upgrade()) {
                    rt.spend(&super::accounts::budget_key(&self.id, account), left);
                }
            }
            _ => {
                let listener = self
                    .rt
                    .upgrade()
                    .and_then(|rt| rt.listeners.lock().get(&self.id).cloned());
                match listener {
                    Some(f) => f(method, params),
                    None => self
                        .journal
                        .push(Kind::Message, format!("a notification {method}")),
                }
            }
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
        super::view::forget(&self.id);
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
