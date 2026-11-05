//! The loop as a module (docs/PLUGINS.md §8.3): the same `Plugin` and the same `Cx` as in process, on one thread, without
//! tokio. A call is a task; a task that asks the host (`secrets/get`, `net/fetch`, `timer/set`) gives the thread back and
//! waits for a line of a later turn. Each turn reads what came, gives the answers to the tasks that wait for them, starts
//! the new calls, polls them all, and writes their lines.

use super::{arrived, called, Cx, Plugin};
use parking_lot::Mutex;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Waker};

type Task = Pin<Box<dyn Future<Output = ()> + Send>>;

/// A plugin kept between its turns.
pub(crate) struct Reactor {
    plugin: Arc<dyn Plugin>,
    cx: Arc<Cx>,
    out: Arc<Mutex<Vec<String>>>,
    tasks: Mutex<Vec<Task>>,
}

impl Reactor {
    pub(crate) fn new(plugin: impl Plugin) -> Self {
        let out: Arc<Mutex<Vec<String>>> = Arc::default();
        let written = out.clone();
        let cx = Cx::new(move |line| {
            written.lock().push(line);
            true
        });
        Self {
            plugin: Arc::new(plugin),
            cx: Arc::new(cx),
            out,
            tasks: Mutex::new(Vec::new()),
        }
    }

    pub(crate) fn cx(&self) -> Arc<Cx> {
        self.cx.clone()
    }

    /// One turn on what came: the lines to write.
    ///
    /// Nothing wakes a task but a line, and a line makes a turn: every task is polled, so no waker is needed. A timer that
    /// rang has been seen by now, or nobody waits for it any more.
    pub(crate) fn turn(&self, lines: impl IntoIterator<Item = String>) -> Vec<String> {
        let mut tasks = self.tasks.lock();
        for line in lines {
            let Some(msg) = arrived(&self.cx, &line) else {
                continue;
            };
            let (plugin, cx) = (self.plugin.clone(), self.cx.clone());
            tasks.push(Box::pin(async move {
                let (out, _) = called(&*plugin, &cx, msg).await;
                if let Some(out) = out {
                    cx.send(out.to_string());
                }
            }));
        }
        let mut task = Context::from_waker(Waker::noop());
        tasks.retain_mut(|t| t.as_mut().poll(&mut task).is_pending());
        drop(tasks);
        self.cx.fired.lock().clear();
        std::mem::take(&mut *self.out.lock())
    }
}

/// The plugin of this module, once `run` has kept it: tests make their own.
static HOST: std::sync::OnceLock<Reactor> = std::sync::OnceLock::new();

/// The `Cx` of this module, for what has none at hand (a client, a timer).
pub(crate) fn cx() -> Option<Arc<Cx>> {
    HOST.get().map(Reactor::cx)
}

#[cfg(target_family = "wasm")]
fn turn() {
    let Some(host) = HOST.get() else { return };
    for line in host.turn(escouade_plugin_protocol::turn::lines()) {
        escouade_plugin_protocol::turn::write(&line);
    }
    escouade_plugin_protocol::turn::flush();
}

/// What `run` is in a module: the plugin kept for its turns. Its language comes with `initialize`.
#[cfg(target_family = "wasm")]
pub fn run<P: Plugin>(_name: &str, plugin: P) -> i32 {
    let _ = HOST.set(Reactor::new(plugin));
    escouade_plugin_protocol::turn::set_turn(turn);
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::BoxFuture;
    use crate::http::fetch;
    use crate::time::{sleep_on, timeout_on};
    use escouade_plugin_protocol::{code, Refusal};
    use serde_json::{json, Value};
    use std::time::Duration;

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
                    "secret" => Ok(json!(cx.secret("account:x").await?)),
                    "nap" => {
                        sleep_on(cx, Duration::from_millis(1500)).await;
                        Ok(json!("rested"))
                    }
                    "patient" => {
                        let secret = cx.secret("account:x");
                        match timeout_on(cx, Duration::from_millis(50), secret).await {
                            Ok(v) => Ok(json!(v?)),
                            Err(e) => Ok(json!(e.to_string())),
                        }
                    }
                    "fetch" => {
                        let resp = fetch::Client::new()
                            .post("https://api.example.dev/graphql")
                            .bearer_auth("k")
                            .json(&json!({ "a": 1 }))
                            .send()
                            .await
                            .map_err(|e| Refusal {
                                code: -32000,
                                message: e.to_string(),
                            })?;
                        let status = resp.status().as_u16();
                        Ok(json!([status, resp.text().await.unwrap()]))
                    }
                    other => Err(Refusal::not_found(other)),
                }
            })
        }
    }

    fn parsed(lines: Vec<String>) -> Vec<Value> {
        lines
            .iter()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    fn call(id: u64, method: &str) -> String {
        json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": { "a": id } }).to_string()
    }

    fn result(id: &Value, result: Value) -> String {
        json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
    }

    fn fired(timer: &Value) -> String {
        json!({ "jsonrpc": "2.0", "method": "timer/fired", "params": { "timer": timer } })
            .to_string()
    }

    #[test]
    fn a_call_that_waits_gives_the_turn_back_and_holds_back_nothing() {
        let r = Reactor::new(Echo);
        let out = parsed(r.turn([call(1, "secret"), call(2, "echo")]));
        assert_eq!(out.len(), 2, "{out:?}");
        assert_eq!(out[0]["method"], "secrets/get");
        assert_eq!(
            (out[1]["id"].clone(), out[1]["result"]["a"].clone()),
            (json!(2), json!(2))
        );
        assert!(r.turn([]).is_empty(), "nothing came, nothing moves");
        let done = parsed(r.turn([result(&out[0]["id"], json!("ghp_x"))]));
        assert_eq!(
            (done[0]["id"].clone(), done[0]["result"].clone()),
            (json!(1), json!("ghp_x"))
        );
    }

    #[test]
    fn a_refusal_of_escouade_reaches_the_call() {
        let r = Reactor::new(Echo);
        let ask = parsed(r.turn([call(1, "secret")]));
        let no = json!({ "jsonrpc": "2.0", "id": ask[0]["id"], "error": { "code": -32003, "message": "no" } });
        let done = parsed(r.turn([no.to_string()]));
        assert_eq!(done[0]["error"]["code"], -32003);
    }

    /// As in process (`the_handshake_garbage_and_shutdown`).
    #[test]
    fn the_handshake_garbage_and_a_batch() {
        let r = Reactor::new(Echo);
        let out =
            parsed(
                r.turn([
                    json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} })
                        .to_string(),
                    "not json".to_string(),
                    String::new(),
                    format!("[{},{}]", call(3, "echo"), call(4, "nope")),
                    call(9, "shutdown"),
                ]),
            );
        assert_eq!(out.len(), 4, "{out:?}");
        assert_eq!(out[0]["error"]["code"], code::PARSE);
        assert_eq!(out[1]["result"]["protocolVersion"], 1);
        assert_eq!(out[2][0]["result"]["a"], 3);
        assert_eq!(out[2][1]["error"]["code"], code::METHOD_NOT_FOUND);
        assert_eq!(out[3]["id"], 9);
    }

    #[test]
    fn a_batch_of_answers_reaches_its_calls() {
        let r = Reactor::new(Echo);
        let asks = parsed(r.turn([call(1, "secret"), call(2, "secret")]));
        let batch = format!(
            "[{},{}]",
            result(&asks[0]["id"], json!("a")),
            result(&asks[1]["id"], json!("b"))
        );
        let done = parsed(r.turn([batch]));
        assert_eq!(
            (done[0]["result"].clone(), done[1]["result"].clone()),
            (json!("a"), json!("b"))
        );
    }

    #[test]
    fn a_sleep_asks_a_timer_and_ends_when_it_rings() {
        let r = Reactor::new(Echo);
        let set = parsed(r.turn([call(1, "nap")]));
        assert_eq!(set[0]["method"], "timer/set");
        assert_eq!(set[0]["params"]["ms"], 1500);
        assert!(r
            .turn([result(&set[0]["id"], json!({ "timer": 7 }))])
            .is_empty());
        assert!(r.turn([fired(&json!(99))]).is_empty(), "another timer");
        let done = parsed(r.turn([fired(&json!(7))]));
        assert_eq!(done[0]["result"], "rested");
    }

    /// The host may write both before the turn: the timer rings before the task knows it.
    #[test]
    fn a_timer_that_rings_with_its_answer_is_not_lost() {
        let r = Reactor::new(Echo);
        let set = parsed(r.turn([call(1, "nap")]));
        let done = parsed(r.turn([
            result(&set[0]["id"], json!({ "timer": "t1" })),
            fired(&json!("t1")),
        ]));
        assert_eq!(done[0]["result"], "rested");
    }

    #[test]
    fn a_host_without_timers_does_not_hold_a_sleep_for_ever() {
        let r = Reactor::new(Echo);
        let set = parsed(r.turn([call(1, "nap")]));
        let no = json!({ "jsonrpc": "2.0", "id": set[0]["id"], "error": { "code": -32601, "message": "no method" } });
        let out = parsed(r.turn([no.to_string()]));
        assert_eq!(out[0]["method"], "log", "{out:?}");
        assert_eq!(out[1]["result"], "rested");
    }

    #[test]
    fn a_timeout_ends_the_call_and_cancels_nothing_it_does_not_hold() {
        let r = Reactor::new(Echo);
        let first = parsed(r.turn([call(1, "patient")]));
        let (ask, set) = (&first[0], &first[1]);
        assert_eq!(ask["method"], "secrets/get");
        assert_eq!(
            (set["method"].clone(), set["params"]["ms"].clone()),
            (json!("timer/set"), json!(50))
        );
        assert!(r
            .turn([result(&set["id"], json!({ "timer": 3 }))])
            .is_empty());
        let done = parsed(r.turn([fired(&json!(3))]));
        assert_eq!(done[0]["result"], "timed out");
    }

    #[test]
    fn what_ends_in_time_cancels_its_timer() {
        let r = Reactor::new(Echo);
        let first = parsed(r.turn([call(1, "patient")]));
        assert!(r
            .turn([result(&first[1]["id"], json!({ "timer": 3 }))])
            .is_empty());
        let out = parsed(r.turn([result(&first[0]["id"], json!("ghp_x"))]));
        assert_eq!(out[0]["method"], "timer/cancel", "{out:?}");
        assert_eq!(out[0]["params"]["timer"], 3);
        assert_eq!(out[1]["result"], "ghp_x");
    }

    /// The one test that keeps the module's plugin: the client finds it as a module does.
    #[test]
    fn a_client_goes_through_net_fetch() {
        let host = HOST.get_or_init(|| Reactor::new(Echo));
        let ask = parsed(host.turn([call(1, "fetch")]));
        assert_eq!(ask[0]["method"], "net/fetch");
        let p = &ask[0]["params"];
        assert_eq!(p["url"], "https://api.example.dev/graphql");
        assert_eq!(p["method"], "POST");
        assert_eq!(p["headers"]["Authorization"], "Bearer k");
        assert_eq!(p["headers"]["Content-Type"], "application/json");
        assert_eq!(p["body"], r#"{"a":1}"#);
        let done = parsed(host.turn([result(
            &ask[0]["id"],
            json!({ "status": 201, "headers": {}, "body": "ok" }),
        )]));
        assert_eq!(done[0]["result"], json!([201, "ok"]));
    }
}
