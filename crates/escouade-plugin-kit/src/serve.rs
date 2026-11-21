//! The loop of a plugin (docs/PLUGINS-DESIGN.md §3, PLG-2), asynchronous: JSON-RPC 2.0, one message a line. Each request is
//! answered when it ends, several at once (a slow call does not hold back the others); a batch is answered as a batch.
//! While it answers, the plugin may ask Escouade (`Cx::request`: its secrets, its settings) and write to its journal
//! (`Cx::log`). `initialize` and `shutdown` are answered here.
//!
//! The same [`Plugin`] and the same [`Cx`] on both targets. In process, the loop is on tokio's threads (`process`); in a
//! module, a reactor on one thread, without tokio (`reactor`, docs/PLUGINS-DESIGN.md §8.3). A module whose tasks outlive its
//! calls, or that reads a stream, has [`crate::host`].

use crate::forge::BoxFuture;
use escouade_plugin_protocol::{code, greeted, initialized, method, Greeting, ESCOUADE_MIN};
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::future::poll_fn;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::task::{Poll, Waker};

#[cfg(not(target_family = "wasm"))]
mod process;
#[cfg(any(test, target_family = "wasm"))]
pub(crate) mod reactor;
pub(crate) mod tasks;

#[cfg(not(target_family = "wasm"))]
pub use process::{run, serve_on};
#[cfg(target_family = "wasm")]
pub use reactor::run;

/// A refusal: its code and its message (`escouade_plugin_protocol::code`).
pub use escouade_plugin_protocol::Refusal;

/// A plugin: what it can do, and its answer to each method but the handshake's.
pub trait Plugin: Send + Sync + 'static {
    /// What `initialize` answers it can do.
    fn capabilities(&self) -> Value {
        json!({})
    }

    fn call<'a>(
        &'a self,
        cx: &'a Cx,
        method: &'a str,
        params: Value,
    ) -> BoxFuture<'a, Result<Value, Refusal>>;
}

/// A call to Escouade that waits: its answer when it came, and who to tell.
#[derive(Default)]
struct Waiting {
    answer: Option<Result<Value, Refusal>>,
    waker: Option<Waker>,
}

/// How a line goes out; false when it cannot any more.
type Out = Box<dyn Fn(String) -> bool + Send + Sync>;

/// What a call may ask of Escouade while it runs.
pub struct Cx {
    out: Out,
    pending: Mutex<HashMap<u64, Arc<Mutex<Waiting>>>>,
    next: AtomicU64,
    /// The timers that rang in this turn, by what `timer/set` answered (a module's; `crate::time`).
    fired: Mutex<HashSet<String>>,
    /// What Escouade said at `initialize`: its version and what it serves.
    greeting: OnceLock<Greeting>,
}

/// What an Escouade may serve or not, which the kit checks before it asks ([`Cx::require`]): an older Escouade would
/// ignore a parameter it does not know, and answer something else in silence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    /// The profiles and the settings of each project (`capabilities.profiles`).
    Profiles,
    /// The jokers of `permissions.commands` and the `pid` of `proc/spawn` (`capabilities.typedCommands`).
    TypedCommands,
}

impl Capability {
    fn word(self) -> &'static str {
        match self {
            Self::Profiles => "profiles",
            Self::TypedCommands => "typedCommands",
        }
    }

    fn what(self) -> &'static str {
        match self {
            Self::Profiles => "the profiles and the settings of each project",
            Self::TypedCommands => "the jokers of permissions.commands",
        }
    }

    fn served(self, g: &Greeting) -> bool {
        match self {
            Self::Profiles => g.profiles,
            Self::TypedCommands => g.typed_commands,
        }
    }
}

/// A program `proc/spawn` started: what `proc/write`, `proc/kill`, `proc/out` and `proc/exit` name it by, and its
/// pid, which `{pid}` of `permissions.commands` admits while it runs (none from an Escouade without
/// `capabilities.typedCommands`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spawned {
    pub proc: u64,
    pub pid: Option<u32>,
}

impl Cx {
    fn new(out: impl Fn(String) -> bool + Send + Sync + 'static) -> Self {
        Self {
            out: Box::new(out),
            pending: Mutex::new(HashMap::new()),
            next: AtomicU64::new(1),
            fired: Mutex::new(HashSet::new()),
            greeting: OnceLock::new(),
        }
    }

    /// What Escouade said at `initialize` (its version, what it serves): none before it.
    pub fn greeting(&self) -> Option<&Greeting> {
        self.greeting
            .get()
            .or_else(|| escouade_plugin_protocol::greeting())
    }

    /// Whether this Escouade serves `what`, or the refusal that says so (`-32601`, never sent to Escouade): an Escouade
    /// older than [`ESCOUADE_MIN`] would ignore what the call adds and answer something else, so the kit refuses first.
    ///
    /// ```
    /// use escouade_plugin_kit::serve::{Capability, Cx, Refusal};
    ///
    /// /// The tunnel of a project, when this Escouade serves the jokers its command takes.
    /// async fn tunnel(cx: &Cx, port: u16) -> Result<u64, Refusal> {
    ///     cx.require(Capability::TypedCommands)?;
    ///     let args = ["--port".to_string(), port.to_string()];
    ///     let spawned = cx.spawn(serde_json::json!({ "command": "tunnel", "args": args })).await?;
    ///     Ok(spawned.proc)
    /// }
    /// ```
    pub fn require(&self, what: Capability) -> Result<(), Refusal> {
        let g = self.greeting();
        if g.is_some_and(|g| what.served(g)) {
            return Ok(());
        }
        let version = g
            .map(|g| g.version.as_str())
            .filter(|v| !v.is_empty())
            .unwrap_or("before initialize");
        Err(Refusal {
            code: code::METHOD_NOT_FOUND,
            message: format!(
                "this Escouade ({version}) does not serve {} (capabilities.{}): Escouade {ESCOUADE_MIN} does, the \
                 escouade.min the plugin's manifest should say",
                what.what(),
                what.word()
            ),
        })
    }

    fn send(&self, line: String) -> bool {
        (self.out)(line)
    }

    /// Asks Escouade `method`: its result, or its refusal (one that does not come back: Escouade closed).
    pub async fn request(&self, method: &str, params: Value) -> Result<Value, Refusal> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let waiting = Arc::new(Mutex::new(Waiting::default()));
        self.pending.lock().insert(id, waiting.clone());
        let line = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        if !self.send(line.to_string()) {
            self.pending.lock().remove(&id);
            return Err(Refusal {
                code: -32000,
                message: "Escouade is not answering".into(),
            });
        }
        poll_fn(move |task| {
            let mut w = waiting.lock();
            match w.answer.take() {
                Some(answer) => Poll::Ready(answer),
                None => {
                    w.waker = Some(task.waker().clone());
                    Poll::Pending
                }
            }
        })
        .await
    }

    /// The secret `key` of the plugin (`permissions.secrets`): none when Escouade keeps none.
    pub async fn secret(&self, key: &str) -> Result<Option<String>, Refusal> {
        let v = self.request("secrets/get", json!({ "key": key })).await?;
        Ok(v.as_str().map(str::to_string))
    }

    /// The secret field `key` of the profile `profile` (its id, from [`Cx::profile_of`] or [`Cx::profiles`]), which
    /// `permissions.secrets` holds by `profile:<kind>`: none when Escouade keeps none. Refused before the call by an
    /// Escouade without `capabilities.profiles`, which would answer another secret ([`Cx::require`]).
    ///
    /// ```
    /// use escouade_plugin_kit::serve::{Cx, Refusal};
    ///
    /// /// The password of the VPN profile a project chose: none when it chose none, or none is kept.
    /// async fn password(cx: &Cx, project: &str) -> Result<Option<String>, Refusal> {
    ///     let Some(profile) = cx.profile_of(project, "profile").await? else {
    ///         return Ok(None);
    ///     };
    ///     cx.secret_of(&profile, "password").await
    /// }
    /// ```
    pub async fn secret_of(&self, profile: &str, key: &str) -> Result<Option<String>, Refusal> {
        self.require(Capability::Profiles)?;
        let v = self
            .request("secrets/get", json!({ "key": key, "profile": profile }))
            .await?;
        Ok(v.as_str().map(str::to_string))
    }

    /// Its profiles of the kind `kind` (`contributes.profiles`): their ids and names, in the order the user made them.
    /// Refused before the call by an Escouade without `capabilities.profiles`.
    ///
    /// ```
    /// use escouade_plugin_kit::serve::{Cx, Refusal};
    ///
    /// /// A line of its journal that names its VPN profiles.
    /// async fn say_profiles(cx: &Cx) -> Result<(), Refusal> {
    ///     let names: Vec<String> = cx.profiles("vpn").await?.into_iter().map(|(_, name)| name).collect();
    ///     cx.log("info", &format!("profiles: {}", names.join(", ")));
    ///     Ok(())
    /// }
    /// ```
    pub async fn profiles(&self, kind: &str) -> Result<Vec<(String, String)>, Refusal> {
        self.require(Capability::Profiles)?;
        let v = self
            .request("profiles/list", json!({ "kind": kind }))
            .await?;
        Ok(v.as_array()
            .into_iter()
            .flatten()
            .filter_map(|p| {
                Some((
                    p.get("id")?.as_str()?.to_string(),
                    p.get("name")?.as_str()?.to_string(),
                ))
            })
            .collect())
    }

    /// The simple fields of its profile `profile` of the kind `kind` (never a secret): none when it has no such profile.
    /// Refused before the call by an Escouade without `capabilities.profiles`.
    ///
    /// ```
    /// use escouade_plugin_kit::serve::{Cx, Refusal};
    ///
    /// /// The server of a VPN profile.
    /// async fn server(cx: &Cx, profile: &str) -> Result<Option<String>, Refusal> {
    ///     let fields = cx.profile("vpn", profile).await?;
    ///     Ok(fields.and_then(|f| f["server"].as_str().map(str::to_string)))
    /// }
    /// ```
    pub async fn profile(&self, kind: &str, profile: &str) -> Result<Option<Value>, Refusal> {
        self.require(Capability::Profiles)?;
        let v = self
            .request("profiles/get", json!({ "kind": kind, "profile": profile }))
            .await?;
        Ok((!v.is_null()).then_some(v))
    }

    /// Its settings (`contributes.settings`): as the user set them, or, with `project` (the `project.id` of a fact), for
    /// that project, each one by its ranks (this machine's value for the project, then its `.escouade.yml`, then the
    /// manifest's `default`). A `profile` setting gives the id of the profile the project chose, or null.
    ///
    /// ```
    /// use escouade_plugin_kit::serve::{Cx, Refusal};
    ///
    /// /// The routes a project sends through the tunnel (a setting of `"scope": "project"`).
    /// async fn routes(cx: &Cx, project: &str) -> Result<String, Refusal> {
    ///     let settings = cx.settings(Some(project)).await?;
    ///     Ok(settings.get("routes").and_then(|r| r.as_str()).unwrap_or_default().to_string())
    /// }
    /// ```
    pub async fn settings(
        &self,
        project: Option<&str>,
    ) -> Result<serde_json::Map<String, Value>, Refusal> {
        let params = match project {
            Some(p) => json!({ "project": p }),
            None => json!({}),
        };
        let v = self.request("settings/get", params).await?;
        Ok(match v {
            Value::Object(m) => m,
            _ => serde_json::Map::new(),
        })
    }

    /// The profile the project `project` chose by its setting `setting` (`"type": "profile"`): its id, none while it chose
    /// none. Only the user chooses it, never a `.escouade.yml`. Refused before the call by an Escouade without
    /// `capabilities.profiles`, whose `settings/get` knows no profile.
    pub async fn profile_of(
        &self,
        project: &str,
        setting: &str,
    ) -> Result<Option<String>, Refusal> {
        self.require(Capability::Profiles)?;
        let settings = self.settings(Some(project)).await?;
        Ok(settings
            .get(setting)
            .and_then(Value::as_str)
            .map(str::to_string))
    }

    /// Runs a program of `permissions.commands` (`proc/spawn`, a module's): `params` are the call's (`command`, `args`,
    /// `dir`, `env`). Its output comes as `proc/out`, its end as `proc/exit`, to the plugin's `call`. Its pid comes from
    /// an Escouade that serves the jokers: [`Cx::spawn_typed`] wants it.
    pub async fn spawn(&self, params: Value) -> Result<Spawned, Refusal> {
        let v = self.request("proc/spawn", params).await?;
        let proc = v["proc"].as_u64().ok_or_else(|| Refusal {
            code: code::PARSE,
            message: format!("proc/spawn answered no proc: {v}"),
        })?;
        Ok(Spawned {
            proc,
            pid: v["pid"].as_u64().and_then(|p| u32::try_from(p).ok()),
        })
    }

    /// Runs a program whose entry of `permissions.commands` has jokers (`{int}`, `{port}`, `{name}`, `{pid}`,
    /// `{path:temp}`, `{path:plugin}`), and gives its `proc` and its pid. Refused before the call by an Escouade
    /// without `capabilities.typedCommands`, which would refuse the entry anyway.
    ///
    /// ```
    /// use escouade_plugin_kit::serve::{Cx, Refusal};
    /// use serde_json::json;
    ///
    /// /// The tunnel's namespace, then its network, given the pid of the first program (`{pid}`).
    /// async fn tunnel(cx: &Cx) -> Result<(u64, u64), Refusal> {
    ///     let vpn = json!({ "command": "unshare", "args": ["-Urn", "openvpn", "--config", "vpn.ovpn"] });
    ///     let (vpn, pid) = cx.spawn_typed(vpn).await?;
    ///     let net = json!({ "command": "slirp4netns", "args": ["--configure", pid.to_string(), "tap0"] });
    ///     let (slirp, _) = cx.spawn_typed(net).await?;
    ///     Ok((vpn, slirp))
    /// }
    /// ```
    pub async fn spawn_typed(&self, params: Value) -> Result<(u64, u32), Refusal> {
        self.require(Capability::TypedCommands)?;
        let s = self.spawn(params).await?;
        let pid = s.pid.ok_or_else(|| Refusal {
            code: code::PARSE,
            message: "proc/spawn answered no pid".into(),
        })?;
        Ok((s.proc, pid))
    }

    /// A notification to Escouade (`method`, no answer).
    pub fn notify(&self, method: &str, params: Value) {
        let line = json!({ "jsonrpc": "2.0", "method": method, "params": params });
        self.send(line.to_string());
    }

    /// A line of the plugin's journal.
    pub fn log(&self, level: &str, message: &str) {
        self.notify("log", json!({ "level": level, "message": message }));
    }

    /// Whether the timer `key` rang; said once.
    #[cfg(any(test, target_family = "wasm"))]
    pub(crate) fn rang(&self, key: &str) -> bool {
        self.fired.lock().remove(key)
    }

    fn answered(&self, msg: &Value) {
        let Some(id) = msg.get("id").and_then(Value::as_u64) else {
            return;
        };
        let Some(waiting) = self.pending.lock().remove(&id) else {
            return;
        };
        let answer = match msg.get("error") {
            Some(e) => Err(Refusal {
                code: e["code"].as_i64().unwrap_or(-32000),
                message: e["message"].as_str().unwrap_or("").to_string(),
            }),
            None => Ok(msg.get("result").cloned().unwrap_or(Value::Null)),
        };
        let mut w = waiting.lock();
        w.answer = Some(answer);
        if let Some(waker) = w.waker.take() {
            waker.wake();
        }
    }
}

fn answer(id: &Value, result: Result<Value, Refusal>) -> Value {
    match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err(e) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": e.code, "message": e.message },
        }),
    }
}

fn parse_error() -> Refusal {
    Refusal {
        code: code::PARSE,
        message: "not a JSON-RPC request".into(),
    }
}

/// An answer of Escouade to one of the plugin's requests.
fn is_answer(m: &Value) -> bool {
    m.get("method").is_none() && m.get("id").is_some()
}

/// A line that came: what the loop has to run, when it is a call. The rest is dealt with here: an answer of Escouade
/// goes to who waits for it, a timer of a module rings, what is no JSON is refused.
fn arrived(cx: &Cx, line: &str) -> Option<Value> {
    if line.trim().is_empty() {
        return None;
    }
    let Ok(msg) = serde_json::from_str::<Value>(line) else {
        cx.send(answer(&Value::Null, Err(parse_error())).to_string());
        return None;
    };
    if is_answer(&msg) {
        cx.answered(&msg);
        return None;
    }
    if let Value::Array(items) = &msg {
        if !items.is_empty() && items.iter().all(is_answer) {
            items.iter().for_each(|m| cx.answered(m));
            return None;
        }
    }
    if msg["method"] == "timer/fired" {
        cx.fired.lock().insert(msg["params"]["timer"].to_string());
        return None;
    }
    Some(msg)
}

/// What a request is answered (none for a notification), and whether it was `shutdown`.
async fn one<P: Plugin + ?Sized>(plugin: &P, cx: &Cx, msg: Value) -> (Option<Value>, bool) {
    let id = msg.get("id").cloned();
    let Some(name) = msg
        .get("method")
        .and_then(Value::as_str)
        .map(str::to_string)
    else {
        return (Some(answer(&Value::Null, Err(parse_error()))), false);
    };
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    let result = match name.as_str() {
        method::INITIALIZE => {
            greeted(&params);
            let _ = cx.greeting.set(Greeting::of(&params));
            crate::i18n::from_greeting();
            Ok(initialized(plugin.capabilities()))
        }
        method::SHUTDOWN => Ok(Value::Null),
        other => plugin.call(cx, other, params).await,
    };
    (id.map(|id| answer(&id, result)), name == method::SHUTDOWN)
}

/// A call or a batch to its end: what goes out, and whether it was `shutdown`.
async fn called<P: Plugin + ?Sized>(plugin: &P, cx: &Cx, msg: Value) -> (Option<Value>, bool) {
    match msg {
        Value::Array(items) => {
            let mut out = Vec::new();
            let mut shut = false;
            for m in items {
                let (a, s) = one(plugin, cx, m).await;
                out.extend(a);
                shut |= s;
            }
            ((!out.is_empty()).then_some(Value::Array(out)), shut)
        }
        m => one(plugin, cx, m).await,
    }
}
