//! The fake Escouade (PLG-3, ESC-175): a plugin's module started as the app starts it, spoken to in JSON-RPC 2.0, one
//! message per line, without the app. It sends what it is told to (`call`, `notify`) and keeps what
//! the plugin does on its own: the secrets it asks for (served from the session's, its own keys
//! only, as `runtime` serves them), its `log` lines, its standard error, what it sends that answers
//! nothing. The bench plays its scenarios through it; `escouade-plugin dev` lets an author drive it
//! by hand.
//!
//! A plugin is played in WASM (`run.js`, ESC-322; `run.wasm`, ESC-331), in the app's own host (`plugins::wasm`): its
//! module, an instance on a thread of its own, `net/fetch` served by the host and closed to what `permissions.network` does not name; no Node.
//! As in the app, its instance ends once it answered `shutdown`, and a fault (its time, its memory, an exception) ends
//! it as a crash ends a process. Its `fs/*` and `proc/*` are the app's (ESC-333), on the manifest as it is, around one agent
//! of the bench, `BENCH_AGENT`, in a scratch folder that goes with the session; offline, it runs no program.
//!
//! It serves the contract of 2.2.5 as the app does (ESC-531): `settings/get` by the ranks of `profiles::settings_for`,
//! one profile of the bench by kind of `contributes.profiles` (`bench_profiles`), chosen by the bench's project,
//! `profiles/list`, `profiles/get` and the secret fields of a profile; a secret on several lines is given on several
//! lines (`multiline_secret`). What it refuses is seen (`Seen::Refused`), the programs' refusals among them.

use crate::journal::Kind;
use crate::manifest::Manifest;
use crate::runtime::{
    matches, notification_line, request_line, INVALID_PARAMS, METHOD_NOT_FOUND, NOT_YOURS,
};
use crate::{files, procs, profiles, wasm, Choices};
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot, Notify};

/// The key of the bench's agent, for `fs/*` and `proc/*`.
pub const BENCH_AGENT: &str = "bench";
/// The id of the bench's project, that of its facts and decisions (`bench::subjects`): its profiles are chosen there.
pub const BENCH_PROJECT: &str = "bench-project";

/// A secret on several lines as the bench gives it, for the key `key`: a configuration whose lines each say `made`'s
/// nonce in their own way, so that a line written out alone is found (`bench::Scenario::Multiline`).
pub fn multiline_secret(made: &str, key: &str) -> String {
    let nonce = made.rsplit('-').next().unwrap_or(made);
    format!(
        "# {made}-{key}\nremote {key}.{nonce}.bench.example 1194\n<ca>\nMIIB{nonce}{key}certificate\n</ca>\n"
    )
}

/// The lines of a secret on several lines that would tell it, written out: those of more than 8 characters.
pub fn telling_lines(value: &str) -> Vec<&str> {
    value
        .lines()
        .map(str::trim)
        .filter(|l| l.chars().count() > 8)
        .collect()
}

/// Whether a field (a secret of `contributes.secrets`, a field of a profile) is on several lines.
fn multiline(field: &serde_json::Map<String, Value>) -> bool {
    field.get("multiline").and_then(Value::as_bool) == Some(true)
}

/// The profiles of the bench for `manifest`: one by kind, its simple fields from their `default` or made of their type,
/// chosen by every `profile` setting in the bench's project; and the values of their secret fields (`made`'s, on several
/// lines for a `multiline` one; none without `made`), by profile then key.
pub fn bench_profiles(
    manifest: &Manifest,
    made: Option<&str>,
) -> (Choices, HashMap<(String, String), String>) {
    let mut choices = Choices::default();
    let mut secrets = HashMap::new();
    for kind in profiles::kinds(manifest) {
        let id = profiles::new_id();
        let mut values = serde_json::Map::new();
        for f in &kind.fields {
            let Some(key) = f.get("key").and_then(Value::as_str) else {
                continue;
            };
            if f.get("secret").and_then(Value::as_bool) == Some(true) {
                if let Some(made) = made {
                    let value = if multiline(f) {
                        multiline_secret(made, key)
                    } else {
                        format!("{made}-{key}")
                    };
                    secrets.insert((id.clone(), key.to_string()), value);
                }
                continue;
            }
            let value = match f.get("default") {
                Some(d) if profiles::fits(f, d) => d.clone(),
                _ => match f.get("type").and_then(Value::as_str) {
                    Some("number") => json!(1),
                    Some("boolean") => json!(true),
                    Some("select") => f
                        .get("options")
                        .and_then(Value::as_array)
                        .and_then(|o| o.first())
                        .and_then(|o| o.as_str().or_else(|| o.get("value")?.as_str()))
                        .map_or(Value::Null, |o| json!(o)),
                    _ if key == kind.name_field => json!("Banc"),
                    _ => json!(format!("bench-{key}")),
                },
            };
            if !value.is_null() {
                values.insert(key.to_string(), value);
            }
        }
        for d in profiles::declared(manifest) {
            let of_kind = d.get("type").and_then(Value::as_str) == Some("profile")
                && d.get("of").and_then(Value::as_str) == Some(kind.id.as_str());
            if let (true, Some(key)) = (of_kind, d.get("key").and_then(Value::as_str)) {
                let place = if profiles::of_project(&d) {
                    choices
                        .project_settings
                        .entry(BENCH_PROJECT.to_string())
                        .or_default()
                } else {
                    &mut choices.settings
                };
                place.insert(key.to_string(), json!(id));
            }
        }
        choices
            .profiles
            .entry(kind.id.clone())
            .or_default()
            .push(profiles::Profile { id, values });
    }
    (choices, secrets)
}

/// The values the bench keeps for the plugin's own secrets: `made`'s for each key of `permissions.secrets`, on several
/// lines for a `multiline` one of `contributes.secrets`.
pub fn bench_secrets(manifest: &Manifest, made: &str) -> HashMap<String, String> {
    let lines: Vec<String> = manifest
        .contributes
        .get("secrets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .filter(|s| multiline(s))
        .filter_map(|s| s.get("key").and_then(Value::as_str).map(str::to_string))
        .collect();
    manifest
        .permissions
        .secrets
        .iter()
        .filter(|k| !k.ends_with('*') && !k.starts_with("profile:"))
        .map(|k| {
            let value = if lines.contains(k) {
                multiline_secret(made, k)
            } else {
                format!("{made}-{k}")
            };
            (k.clone(), value)
        })
        .collect()
}

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
    // What the host refuses is seen as such: the bench judges the programs on it.
    let told: procs::Told = Arc::new(move |kind, line| {
        if matches!(kind, Kind::Refused) {
            let (method, message) = line.split_once(": ").unwrap_or(("", line));
            shared.see(Seen::Refused {
                method: method.to_string(),
                message: message.to_string(),
            });
        }
        shared.tell(line.to_string())
    });
    let project = scratch.join("project");
    let procs = procs::Procs::new(procs::ProcsOf {
        commands: if offline {
            Vec::new()
        } else {
            manifest.permissions.commands.clone()
        },
        env: manifest.permissions.env.clone(),
        first_hand: crate::builtin::first_hand(&manifest.id),
        temp: scratch.join("temp"),
        plugin: folder.to_path_buf(),
        agents: agents.clone(),
        projects: Arc::new(move |_| Ok(project.clone())),
        agent_env: Vec::new(),
        back,
        told: told.clone(),
        processes: manifest.limits.processes,
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
    /// It asked for its profiles (`profiles/list`, `profiles/get`, a secret of a profile, `settings/get`): what, in words.
    Profile { method: String, said: String },
    /// Escouade refused one of its calls (a profile's secret, a program): the method and the refusal.
    Refused { method: String, message: String },
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
    /// Its manifest: its kinds of profile, its settings.
    manifest: Manifest,
    /// The bench's choices of it: its profiles, chosen in the bench's project (`bench_profiles`).
    choices: Choices,
    /// The secret fields of its profiles, by profile then key.
    profile_secrets: HashMap<(String, String), String>,
    /// Its exit code, once it ended (`Some(None)`: no code, killed).
    ended: Mutex<Option<Option<i32>>>,
    end: Notify,
    /// Each new thing seen: for `escouade-plugin dev`, which shows them as they come.
    watch: Mutex<Option<mpsc::UnboundedSender<Seen>>>,
    /// An instance in WASM: the id of the `shutdown` asked, whose answer ends it, as Escouade ends it.
    wasm: bool,
    shutdown: Mutex<Option<u64>>,
    /// Its calls to the host under way (`escouade.call`: a program started, a file read): the host's work, not quiet.
    host_calls: AtomicU64,
    /// What the host serves it, for the programs it runs.
    reached: std::sync::OnceLock<Arc<wasm::Capabilities>>,
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
        let (choices, profile_secrets) = bench_profiles(manifest, made.as_deref());
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
            manifest: manifest.clone(),
            choices,
            profile_secrets,
            ended: Mutex::new(None),
            end: Notify::new(),
            watch: Mutex::new(None),
            wasm,
            shutdown: Mutex::new(None),
            host_calls: AtomicU64::new(0),
            reached: std::sync::OnceLock::new(),
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
}

impl Session {
    /// Starts the instance of `module`, the WASM of the plugin of `manifest` in `folder`, in Escouade's host,
    /// with its limits; `secrets` are the values kept for its keys; a key of its own with none gets `<made>-<key>`, or
    /// `null` as in the app.
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
        .limited(manifest.limits.clone());
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
        let _ = shared.reached.set(reached.clone());
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
                // Each call counted while the host serves it (`Session::busy`).
                let calls: Option<wasm::Calls> = {
                    let (served, counted) = (reached.calls(), s1.clone());
                    Some(Arc::new(move |method, params| {
                        counted.host_calls.fetch_add(1, Ordering::AcqRel);
                        let r = served(method, params);
                        counted.host_calls.fetch_sub(1, Ordering::AcqRel);
                        r
                    }))
                };
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
        Ok(Self { shared })
    }

    /// The `initialize` Escouade sends, offering `version` of the protocol.
    pub fn hello(manifest: &Manifest, app_version: &str, version: u64) -> Value {
        json!({
            "protocolVersion": version,
            "escouade": { "version": app_version },
            "plugin": { "id": manifest.id, "version": manifest.version },
            "lang": "fr",
            "capabilities": { "batch": true, "secrets": true, "log": true, "attach": true, "typedCommands": true, "profiles": true },
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
    /// Whether the host works for it now: a call of it under way, or a program of it running (ESC-650: under load,
    /// starting a program outlasts the bench's quiet of 300 ms, which is no quiet of the plugin's).
    pub fn busy(&self) -> bool {
        self.shared.host_calls.load(Ordering::Acquire) > 0
            || self.shared.reached.get().is_some_and(|r| r.programs() > 0)
    }

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

    /// Ends it, as a crash would: its input closed, its thread ends it.
    pub fn kill(&self) {
        self.shared.tx.lock().take();
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

/// A request of the plugin, served as the app serves it; what it refuses, seen.
fn request(shared: &Shared, method: &str, params: &Value) -> Result<Value, (i64, String)> {
    let answer = served(shared, method, params);
    if let Err((code, message)) = &answer {
        if *code != METHOD_NOT_FOUND {
            shared.see(Seen::Refused {
                method: method.to_string(),
                message: message.clone(),
            });
        }
    }
    answer
}

/// The kind of profile a call names: one the manifest declares, else `-32602` (`runtime`'s `kind_of`).
fn kind_of(shared: &Shared, params: &Value) -> Result<profiles::Kind, (i64, String)> {
    let kind = params
        .get("kind")
        .and_then(Value::as_str)
        .ok_or((INVALID_PARAMS, "takes a kind of profile".to_string()))?;
    profiles::kind(&shared.manifest, kind).ok_or_else(|| {
        (
            INVALID_PARAMS,
            format!("{kind:?} is no kind of profile of this plugin (contributes.profiles)"),
        )
    })
}

/// The secret field `key` of the profile `profile`, by the rules of `runtime`'s `profile_secret`.
fn profile_secret(shared: &Shared, key: &str, profile: &str) -> Result<Value, (i64, String)> {
    if !profiles::is_id(profile) {
        return Err((INVALID_PARAMS, format!("{profile:?} is no profile's id")));
    }
    let kind = profiles::find(&shared.choices, profile)
        .and_then(|(k, _)| profiles::kind(&shared.manifest, k))
        .ok_or_else(|| (INVALID_PARAMS, format!("no profile {profile:?}")))?;
    if !kind.has_secret(key) {
        return Err((
            INVALID_PARAMS,
            format!("{key:?} is no secret field of the profiles {:?}", kind.id),
        ));
    }
    let word = format!("profile:{}", kind.id);
    if !shared.manifest.permissions.secrets.contains(&word) {
        return Err((
            NOT_YOURS,
            format!(
                "the secrets of the profiles {:?} are not this plugin's (permissions.secrets: {word})",
                kind.id
            ),
        ));
    }
    shared.see(Seen::Profile {
        method: "secrets/get".into(),
        said: format!("{key} of a profile {}", kind.id),
    });
    Ok(shared
        .profile_secrets
        .get(&(profile.to_string(), key.to_string()))
        .map_or(Value::Null, |v| json!(v)))
}

fn served(shared: &Shared, method: &str, params: &Value) -> Result<Value, (i64, String)> {
    match method {
        "secrets/get" if params.get("profile").is_some_and(|p| !p.is_null()) => {
            let key = params.get("key").and_then(Value::as_str).unwrap_or("");
            match params.get("profile") {
                Some(Value::String(profile)) => profile_secret(shared, key, profile),
                _ => Err((
                    INVALID_PARAMS,
                    "secrets/get takes a profile's id, or none".into(),
                )),
            }
        }
        "secrets/get"
            if {
                let key = params.get("key").and_then(Value::as_str).unwrap_or("");
                let own = crate::secret_keys(&shared.manifest)
                    .iter()
                    .any(|k| k == key)
                    || shared
                        .patterns
                        .iter()
                        .any(|p| !p.starts_with("profile:") && matches(p, key));
                !own && profiles::kinds(&shared.manifest)
                    .iter()
                    .any(|k| k.has_secret(key))
            } =>
        {
            let key = params.get("key").and_then(Value::as_str).unwrap_or("");
            Err((
                INVALID_PARAMS,
                format!("{key:?} is a field of a profile: secrets/get takes its profile"),
            ))
        }
        "settings/get" => {
            let project = match params.get("project") {
                None | Some(Value::Null) => None,
                Some(Value::String(p)) => Some(p.as_str()),
                Some(_) => {
                    return Err((
                        INVALID_PARAMS,
                        "settings/get takes a project's id, or none".into(),
                    ))
                }
            };
            if project.is_some() && !shared.choices.profiles.is_empty() {
                shared.see(Seen::Profile {
                    method: method.to_string(),
                    said: "the settings of a project".into(),
                });
            }
            Ok(Value::Object(profiles::settings_for(
                Some(&shared.manifest),
                &shared.choices,
                project,
                project == Some(BENCH_PROJECT),
                None,
            )))
        }
        "profiles/list" => {
            let kind = kind_of(shared, params)?;
            shared.see(Seen::Profile {
                method: method.to_string(),
                said: format!("the profiles {}", kind.id),
            });
            Ok(Value::Array(
                shared
                    .choices
                    .profiles
                    .get(&kind.id)
                    .into_iter()
                    .flatten()
                    .map(|p| json!({ "id": p.id, "name": kind.name_of(p) }))
                    .collect(),
            ))
        }
        "profiles/get" => {
            let kind = kind_of(shared, params)?;
            let Some(profile) = params.get("profile").and_then(Value::as_str) else {
                return Err((INVALID_PARAMS, "profiles/get takes a profile's id".into()));
            };
            shared.see(Seen::Profile {
                method: method.to_string(),
                said: format!("the fields of a profile {}", kind.id),
            });
            Ok(shared
                .choices
                .profiles
                .get(&kind.id)
                .and_then(|l| l.iter().find(|p| p.id == profile))
                .map_or(Value::Null, |p| {
                    Value::Object(kind.simple_values(&p.values))
                }))
        }
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
        "files/read" => crate::files::read(&shared.folder, params),
        _ => {
            shared.see(Seen::Unserved {
                method: method.to_string(),
            });
            Err((METHOD_NOT_FOUND, format!("unknown method {method}")))
        }
    }
}
