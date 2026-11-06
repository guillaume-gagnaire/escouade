//! The loop of a plugin (docs/PLUGINS.md §3, PLG-2), asynchronous: JSON-RPC 2.0, one message a line. Each request is
//! answered when it ends, several at once (a slow call does not hold back the others); a batch is answered as a batch.
//! While it answers, the plugin may ask Escouade (`Cx::request`: its secrets, its settings) and write to its journal
//! (`Cx::log`). `initialize` and `shutdown` are answered here.
//!
//! The same [`Plugin`] and the same [`Cx`] on both targets. In process, the loop is on tokio's threads (`process`); in a
//! module, a reactor on one thread, without tokio (`reactor`, docs/PLUGINS.md §8.3). A module whose tasks outlive its
//! calls, or that reads a stream, has [`crate::host`].

use crate::forge::BoxFuture;
use escouade_plugin_protocol::{code, greeted, initialized, method};
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::future::poll_fn;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
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
}

impl Cx {
    fn new(out: impl Fn(String) -> bool + Send + Sync + 'static) -> Self {
        Self {
            out: Box::new(out),
            pending: Mutex::new(HashMap::new()),
            next: AtomicU64::new(1),
            fired: Mutex::new(HashSet::new()),
        }
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
