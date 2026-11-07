//! The loop of an asynchronous plugin as a module (ESC-330, docs/PLUGINS.md §8.3): the same `Plugin` and the same `Cx` as
//! `escouade_plugin_kit::serve`, on one thread. A call is a task; a task that asks the host (`secrets/get`, `net/fetch`)
//! waits for a line of a later turn, and the turn ends. Each turn reads what came, gives the answers to the tasks that
//! wait for them, starts the new calls, polls them all once, and writes their lines.

use crate::forge::BoxFuture;
use escouade_plugin_protocol::{code, initialized, method};
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::{poll_fn, Future};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::task::{Context, Poll, Waker};

pub use escouade_plugin_protocol::Refusal;

pub trait Plugin: Send + Sync + 'static {
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

type Slot = Arc<Mutex<Option<Result<Value, Refusal>>>>;

/// What a call may ask of Escouade while it runs.
#[derive(Default)]
pub struct Cx {
    pending: Mutex<HashMap<u64, Slot>>,
    next: AtomicU64,
}

impl Cx {
    /// Asks Escouade `method`: its result, or its refusal.
    pub async fn request(&self, method: &str, params: Value) -> Result<Value, Refusal> {
        let id = self.next.fetch_add(1, Ordering::Relaxed) + 1;
        let slot: Slot = Arc::default();
        self.pending.lock().insert(id, slot.clone());
        let line = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        escouade_turn::write(&line.to_string());
        poll_fn(move |_| match slot.lock().take() {
            Some(answer) => Poll::Ready(answer),
            None => Poll::Pending,
        })
        .await
    }

    pub async fn secret(&self, key: &str) -> Result<Option<String>, Refusal> {
        let v = self.request("secrets/get", json!({ "key": key })).await?;
        Ok(v.as_str().map(str::to_string))
    }

    pub fn notify(&self, method: &str, params: Value) {
        let line = json!({ "jsonrpc": "2.0", "method": method, "params": params });
        escouade_turn::write(&line.to_string());
    }

    pub fn log(&self, level: &str, message: &str) {
        self.notify("log", json!({ "level": level, "message": message }));
    }

    fn answered(&self, msg: &Value) {
        let Some(slot) = msg
            .get("id")
            .and_then(Value::as_u64)
            .and_then(|id| self.pending.lock().remove(&id))
        else {
            return;
        };
        *slot.lock() = Some(match msg.get("error") {
            Some(e) => Err(Refusal {
                code: e["code"].as_i64().unwrap_or(-32000),
                message: e["message"].as_str().unwrap_or("").to_string(),
            }),
            None => Ok(msg.get("result").cloned().unwrap_or(Value::Null)),
        });
    }
}

type Task = Pin<Box<dyn Future<Output = ()> + Send>>;

struct Host {
    plugin: Arc<dyn Plugin>,
    cx: Arc<Cx>,
    tasks: Mutex<Vec<Task>>,
}

static HOST: OnceLock<Host> = OnceLock::new();

fn fetch(params: Value) -> reqwest::Answer {
    Box::pin(async move {
        let cx = HOST.get().ok_or("no host")?.cx.clone();
        cx.request("net/fetch", params).await.map_err(|e| e.message)
    })
}

/// What `escouade_plugin_kit::serve::run` is here: the plugin kept for its turns.
pub fn run<P: Plugin>(_name: &str, plugin: P) -> i32 {
    crate::i18n::from_env();
    reqwest::transport(fetch);
    let _ = HOST.set(Host {
        plugin: Arc::new(plugin),
        cx: Arc::default(),
        tasks: Mutex::new(Vec::new()),
    });
    0
}

fn answer(id: &Value, result: Result<Value, Refusal>) -> String {
    escouade_plugin_protocol::answer(id, result)
}

async fn one(plugin: Arc<dyn Plugin>, cx: Arc<Cx>, msg: Value) {
    let id = msg.get("id").cloned();
    let Some(name) = msg
        .get("method")
        .and_then(Value::as_str)
        .map(str::to_string)
    else {
        let refusal = Refusal {
            code: code::PARSE,
            message: "not a JSON-RPC request".into(),
        };
        return escouade_turn::write(&answer(&Value::Null, Err(refusal)));
    };
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    let result = match name.as_str() {
        method::INITIALIZE => Ok(initialized(plugin.capabilities())),
        method::SHUTDOWN => Ok(Value::Null),
        other => plugin.call(&cx, other, params).await,
    };
    if let Some(id) = id {
        escouade_turn::write(&answer(&id, result));
    }
}

/// A turn: what came, then each call polled once.
pub fn turn() {
    let Some(host) = HOST.get() else { return };
    let mut tasks = host.tasks.lock();
    for line in escouade_turn::lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if msg.get("method").is_none() && msg.get("id").is_some() {
            host.cx.answered(&msg);
        } else {
            tasks.push(Box::pin(one(host.plugin.clone(), host.cx.clone(), msg)));
        }
    }
    let mut cx = Context::from_waker(Waker::noop());
    tasks.retain_mut(|t| t.as_mut().poll(&mut cx).is_pending());
    drop(tasks);
    escouade_turn::flush();
}
