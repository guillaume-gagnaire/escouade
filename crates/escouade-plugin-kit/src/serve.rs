//! The loop of a plugin in process (docs/PLUGINS.md §3, PLG-2), asynchronous: JSON-RPC 2.0, one message a line, on the
//! standard input and output. Each request is answered when it ends, several at once (a slow call does not hold back
//! the others); a batch is answered as a batch. While it answers, the plugin may ask Escouade (`Cx::request`: its
//! secrets, its settings) and write to its journal (`Cx::log`). `initialize` and `shutdown` are answered here.

use crate::forge::BoxFuture;
use escouade_plugin_protocol::{code, initialized, method};
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};

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

/// What a call may ask of Escouade while it runs.
pub struct Cx {
    out: mpsc::UnboundedSender<String>,
    pending: Mutex<HashMap<u64, oneshot::Sender<Result<Value, Refusal>>>>,
    next: AtomicU64,
}

impl Cx {
    fn new(out: mpsc::UnboundedSender<String>) -> Self {
        Self {
            out,
            pending: Mutex::new(HashMap::new()),
            next: AtomicU64::new(1),
        }
    }

    /// Asks Escouade `method`: its result, or its refusal (one that does not come back: Escouade closed).
    pub async fn request(&self, method: &str, params: Value) -> Result<Value, Refusal> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().insert(id, tx);
        let line = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        let gone = || Refusal {
            code: -32000,
            message: "Escouade is not answering".into(),
        };
        if self.out.send(line.to_string()).is_err() {
            self.pending.lock().remove(&id);
            return Err(gone());
        }
        rx.await.unwrap_or_else(|_| Err(gone()))
    }

    /// The secret `key` of the plugin (`permissions.secrets`): none when Escouade keeps none.
    pub async fn secret(&self, key: &str) -> Result<Option<String>, Refusal> {
        let v = self.request("secrets/get", json!({ "key": key })).await?;
        Ok(v.as_str().map(str::to_string))
    }

    /// A notification to Escouade (`method`, no answer).
    pub fn notify(&self, method: &str, params: Value) {
        let line = json!({ "jsonrpc": "2.0", "method": method, "params": params });
        let _ = self.out.send(line.to_string());
    }

    /// A line of the plugin's journal.
    pub fn log(&self, level: &str, message: &str) {
        let line = json!({
            "jsonrpc": "2.0",
            "method": "log",
            "params": { "level": level, "message": message },
        });
        let _ = self.out.send(line.to_string());
    }

    fn answered(&self, msg: &Value) {
        let Some(id) = msg.get("id").and_then(Value::as_u64) else {
            return;
        };
        let Some(tx) = self.pending.lock().remove(&id) else {
            return;
        };
        let answer = match msg.get("error") {
            Some(e) => Err(Refusal {
                code: e["code"].as_i64().unwrap_or(-32000),
                message: e["message"].as_str().unwrap_or("").to_string(),
            }),
            None => Ok(msg.get("result").cloned().unwrap_or(Value::Null)),
        };
        let _ = tx.send(answer);
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

/// What a request is answered (none for a notification), and whether it was `shutdown`.
async fn one<P: Plugin>(plugin: &P, cx: &Cx, msg: Value) -> (Option<Value>, bool) {
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
        method::INITIALIZE => Ok(initialized(plugin.capabilities())),
        method::SHUTDOWN => Ok(Value::Null),
        other => plugin.call(cx, other, params).await,
    };
    (id.map(|id| answer(&id, result)), name == method::SHUTDOWN)
}

/// Runs `plugin` on `input` and `output` until `input` ends or `shutdown` is answered.
pub async fn serve_on<P, R, W>(plugin: Arc<P>, input: R, mut output: W) -> std::io::Result<()>
where
    P: Plugin,
    R: tokio::io::AsyncRead + Unpin,
    W: AsyncWrite + Unpin + Send + 'static,
{
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let writer = tokio::spawn(async move {
        while let Some(line) = rx.recv().await {
            if output.write_all(line.as_bytes()).await.is_err()
                || output.write_all(b"\n").await.is_err()
                || output.flush().await.is_err()
            {
                break;
            }
        }
    });
    let cx = Arc::new(Cx::new(tx.clone()));
    let (stop_tx, mut stop_rx) = mpsc::unbounded_channel::<()>();
    let mut lines = BufReader::new(input).lines();
    loop {
        let line = tokio::select! {
            line = lines.next_line() => line?,
            _ = stop_rx.recv() => break,
        };
        let Some(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<Value>(&line) else {
            let _ = tx.send(answer(&Value::Null, Err(parse_error())).to_string());
            continue;
        };
        if is_answer(&msg) {
            cx.answered(&msg);
            continue;
        }
        if let Value::Array(items) = &msg {
            if !items.is_empty() && items.iter().all(is_answer) {
                items.iter().for_each(|m| cx.answered(m));
                continue;
            }
        }
        let (plugin, cx, tx, stop) = (plugin.clone(), cx.clone(), tx.clone(), stop_tx.clone());
        tokio::spawn(async move {
            let (out, shut) = match msg {
                Value::Array(items) => {
                    let mut out = Vec::new();
                    let mut shut = false;
                    for m in items {
                        let (a, s) = one(&*plugin, &cx, m).await;
                        out.extend(a);
                        shut |= s;
                    }
                    ((!out.is_empty()).then_some(Value::Array(out)), shut)
                }
                m => one(&*plugin, &cx, m).await,
            };
            if let Some(out) = out {
                let _ = tx.send(out.to_string());
            }
            if shut {
                let _ = stop.send(());
            }
        });
    }
    drop(tx);
    drop(cx);
    // What was answered is written before the plugin ends.
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), writer).await;
    Ok(())
}

/// The whole life of a plugin of `escouade-plugins`: its language, then its loop on the standard input and output, on
/// a runtime of its own. Its exit code.
pub fn run<P: Plugin>(name: &str, plugin: P) -> i32 {
    crate::i18n::from_env();
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("escouade-plugins {name}: {e}");
            return 1;
        }
    };
    let result = runtime.block_on(serve_on(
        Arc::new(plugin),
        tokio::io::stdin(),
        tokio::io::stdout(),
    ));
    // A call still running (a slow host) does not keep the process.
    runtime.shutdown_timeout(std::time::Duration::from_millis(200));
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("escouade-plugins {name}: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Echo;

    impl Plugin for Echo {
        fn call<'a>(
            &'a self,
            cx: &'a Cx,
            method: &'a str,
            params: Value,
        ) -> BoxFuture<'a, Result<Value, Refusal>> {
            Box::pin(async move {
                match method {
                    "echo" => Ok(params),
                    "slow" => {
                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                        Ok(json!("slow"))
                    }
                    "secret" => Ok(json!(cx.secret("account:x").await?)),
                    other => Err(Refusal::not_found(other)),
                }
            })
        }
    }

    /// The plugin's lines, as Escouade reads them, and what Escouade writes to it.
    async fn session() -> (
        tokio::io::DuplexStream,
        tokio::io::Lines<BufReader<tokio::io::DuplexStream>>,
    ) {
        let (to_plugin, plugin_in) = tokio::io::duplex(1 << 16);
        let (plugin_out, from_plugin) = tokio::io::duplex(1 << 16);
        tokio::spawn(serve_on(Arc::new(Echo), plugin_in, plugin_out));
        (to_plugin, BufReader::new(from_plugin).lines())
    }

    async fn next(lines: &mut tokio::io::Lines<BufReader<tokio::io::DuplexStream>>) -> Value {
        serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap()
    }

    #[tokio::test]
    async fn a_slow_call_does_not_hold_back_the_next() {
        let (mut w, mut r) = session().await;
        w.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"slow\"}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"echo\",\"params\":{\"a\":1}}\n")
            .await
            .unwrap();
        let first = next(&mut r).await;
        assert_eq!(first["id"], 2, "{first}");
        assert_eq!(next(&mut r).await["id"], 1);
    }

    #[tokio::test]
    async fn the_plugin_asks_escouade_while_it_answers() {
        let (mut w, mut r) = session().await;
        w.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"secret\"}\n")
            .await
            .unwrap();
        let ask = next(&mut r).await;
        assert_eq!(ask["method"], "secrets/get");
        assert_eq!(ask["params"]["key"], "account:x");
        let reply = json!({ "jsonrpc": "2.0", "id": ask["id"], "result": "ghp_x" });
        w.write_all(format!("{reply}\n").as_bytes()).await.unwrap();
        let done = next(&mut r).await;
        assert_eq!(
            (done["id"].clone(), done["result"].clone()),
            (json!(7), json!("ghp_x"))
        );
    }

    #[tokio::test]
    async fn the_handshake_garbage_and_shutdown() {
        let (mut w, mut r) = session().await;
        w.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\nnot json\n[{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"echo\",\"params\":1},{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"nope\"}]\n")
            .await
            .unwrap();
        // Answered as each ends: in any order.
        let mut got = [next(&mut r).await, next(&mut r).await, next(&mut r).await];
        got.sort_by_key(|v| (v.is_array(), v["id"].as_u64()));
        assert_eq!(got[0]["error"]["code"], code::PARSE);
        assert_eq!(got[1]["result"]["protocolVersion"], 1);
        let batch = got[2].clone();
        assert_eq!(batch[0]["result"], 1);
        assert_eq!(batch[1]["error"]["code"], code::METHOD_NOT_FOUND);
        w.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"shutdown\"}\n")
            .await
            .unwrap();
        assert_eq!(next(&mut r).await["id"], 9);
        assert!(
            r.next_line().await.unwrap().is_none(),
            "it ends after shutdown"
        );
    }
}
