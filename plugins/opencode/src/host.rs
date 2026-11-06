//! The one thread of the plugin (docs/PLUGINS.md §8.3, §8.4), without tokio: what were three tasks of a process (reading
//! the event stream, the requests that wait, a timer) are events of the host, each arriving as a line of a turn.
//!
//! The host writes what came on the module's input, then calls one turn. A turn gives each line to whom it is for, polls
//! every task, and writes their lines: a task that needs the host (`net/fetch`, `net/stream`, `timer/set`) writes a line
//! and gives the thread back, and goes on at a later turn. Nothing waits inside a turn. Nothing wakes a task but a turn,
//! so a turn polls them all, as the kit's reactor does; the kit's has no way to start a task that no call of the core
//! began, which the event stream is, nor takes a stream, hence this one.
//!
//! Everything here is plain code on lines: `cargo test` plays it natively, without a module.

use base64::Engine;
use escouade_plugin_protocol::Refusal;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::future::{poll_fn, Future};
use std::pin::{pin, Pin};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

type Task = Pin<Box<dyn Future<Output = ()> + Send>>;

/// What came on a stream and was not read yet.
#[derive(Default)]
struct Flow {
    data: Vec<u8>,
    /// It ended (`net/end`).
    ended: bool,
    /// The bytes read since the last `net/credit`.
    read: u64,
}

#[derive(Default)]
pub(crate) struct Host {
    /// The lines of this turn.
    out: Mutex<Vec<String>>,
    next: AtomicU64,
    /// The calls to the host that wait, their answer once it came.
    waiting: Mutex<HashMap<u64, Option<Result<Value, Refusal>>>>,
    /// The timers that rang in this turn.
    fired: Mutex<HashSet<String>>,
    /// The streams open, by what `net/stream` answered.
    flows: Mutex<HashMap<String, Flow>>,
    /// The streams let go: what was on its way when they were is dropped.
    cancelled: Mutex<HashSet<String>>,
    /// The tasks waiting for a turn; those started in it, while the others are polled.
    tasks: Mutex<Vec<Task>>,
    born: Mutex<Vec<Task>>,
    /// Something changed that a task polled earlier in this turn may wait for.
    stirred: AtomicBool,
}

impl Host {
    pub(crate) fn write(&self, line: String) {
        self.out.lock().push(line);
    }

    /// Tells the host or the core something, without waiting for an answer.
    pub(crate) fn notify(&self, method: &str, params: &impl serde::Serialize) {
        let line = json!({ "jsonrpc": "2.0", "method": method, "params": params });
        self.write(line.to_string());
    }

    /// Starts `task`: it is polled in this turn.
    pub(crate) fn spawn(&self, task: impl Future<Output = ()> + Send + 'static) {
        self.born.lock().push(Box::pin(task));
    }

    /// What a task waits for changed: the tasks are polled once more in this turn.
    pub(crate) fn stir(&self) {
        self.stirred.store(true, Ordering::Release);
    }

    /// A line that came: what the plugin has to serve, when it is a call. The rest is dealt with here.
    pub(crate) fn arrived(&self, line: &str) -> Option<Value> {
        if line.trim().is_empty() {
            return None;
        }
        let Ok(msg) = serde_json::from_str::<Value>(line) else {
            return Some(Value::Null);
        };
        if msg.get("method").is_none() && msg.get("id").is_some() {
            self.answered(&msg);
            return None;
        }
        if let Value::Array(items) = &msg {
            if !items.is_empty() && items.iter().all(|m| m.get("method").is_none()) {
                items.iter().for_each(|m| self.answered(m));
                return None;
            }
        }
        let params = &msg["params"];
        match msg["method"].as_str() {
            Some("timer/fired") => {
                self.fired.lock().insert(params["timer"].to_string());
                None
            }
            Some("net/chunk") => {
                self.chunk(params);
                None
            }
            Some("net/end") => {
                let stream = params["stream"].to_string();
                if !self.cancelled.lock().contains(&stream) {
                    self.flows.lock().entry(stream).or_default().ended = true;
                }
                None
            }
            _ => Some(msg),
        }
    }

    fn answered(&self, msg: &Value) {
        let Some(id) = msg.get("id").and_then(Value::as_u64) else {
            return;
        };
        let answer = match msg.get("error") {
            Some(e) => Err(Refusal {
                code: e["code"].as_i64().unwrap_or(-32000),
                message: e["message"].as_str().unwrap_or("").to_string(),
            }),
            None => Ok(msg.get("result").cloned().unwrap_or(Value::Null)),
        };
        if let Some(slot) = self.waiting.lock().get_mut(&id) {
            *slot = Some(answer);
        }
    }

    fn chunk(&self, params: &Value) {
        let stream = params["stream"].to_string();
        if self.cancelled.lock().contains(&stream) {
            return;
        }
        let mut flows = self.flows.lock();
        let flow = flows.entry(stream).or_default();
        if let Some(text) = params["data"].as_str() {
            flow.data.extend_from_slice(text.as_bytes());
        } else if let Some(text) = params["dataBase64"].as_str() {
            // What is not UTF-8: the event of a broken harness, kept for what it can still say.
            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(text) {
                flow.data.extend_from_slice(&bytes);
            }
        }
    }

    /// Polls every task, again while one of them changed what another waits for; then gives the host back what the
    /// streams consumed, one line a stream a turn.
    pub(crate) fn run(&self) {
        let mut waker = Context::from_waker(Waker::noop());
        let mut tasks = std::mem::take(&mut *self.tasks.lock());
        loop {
            self.stirred.store(false, Ordering::Release);
            tasks.append(&mut self.born.lock());
            tasks.retain_mut(|t| t.as_mut().poll(&mut waker).is_pending());
            if !self.stirred.load(Ordering::Acquire) && self.born.lock().is_empty() {
                break;
            }
        }
        *self.tasks.lock() = tasks;
        self.fired.lock().clear();
        let mut flows = self.flows.lock();
        for (stream, flow) in flows.iter_mut().filter(|(_, f)| f.read > 0) {
            let params =
                json!({ "stream": stream.parse::<u64>().unwrap_or(0), "bytes": flow.read });
            self.notify("net/credit", &params);
            flow.read = 0;
        }
    }

    /// The lines of this turn.
    pub(crate) fn lines(&self) -> Vec<String> {
        std::mem::take(&mut *self.out.lock())
    }

    /// Asks the host `method`: its answer, at a later turn.
    pub(crate) async fn request(&self, method: &str, params: Value) -> Result<Value, Refusal> {
        let id = self.next.fetch_add(1, Ordering::Relaxed) + 1;
        self.waiting.lock().insert(id, None);
        let _asked = Asked { host: self, id };
        let line = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        self.write(line.to_string());
        poll_fn(|_| {
            let mut waiting = self.waiting.lock();
            match waiting.get_mut(&id).and_then(Option::take) {
                Some(answer) => Poll::Ready(answer),
                None => Poll::Pending,
            }
        })
        .await
    }

    /// An HTTP request (`net/fetch`): its status and its text.
    pub(crate) async fn fetch(&self, request: Value) -> Result<(u16, String), Refusal> {
        let v = self.request("net/fetch", request).await?;
        let status = u16::try_from(v["status"].as_u64().unwrap_or(0)).unwrap_or(0);
        let text = match (v["body"].as_str(), v["bodyBase64"].as_str()) {
            (Some(text), _) => text.to_string(),
            (None, Some(bytes)) => {
                let bytes = base64::engine::general_purpose::STANDARD.decode(bytes);
                String::from_utf8_lossy(&bytes.unwrap_or_default()).into_owned()
            }
            (None, None) => String::new(),
        };
        Ok((status, text))
    }

    /// An HTTP response in pieces (`net/stream`).
    pub(crate) async fn stream(self: &Arc<Self>, request: Value) -> Result<Stream, Refusal> {
        let v = self.request("net/stream", request).await?;
        Ok(Stream {
            host: self.clone(),
            key: v["stream"].to_string(),
            id: v["stream"].clone(),
            status: u16::try_from(v["status"].as_u64().unwrap_or(0)).unwrap_or(0),
            ended: false,
        })
    }

    /// Waits `delay` on a timer of the host. One it refuses (past its 64 timers) ends at once, the journal told.
    pub(crate) async fn sleep(&self, delay: Duration) {
        let ms = u64::try_from(delay.as_millis()).unwrap_or(u64::MAX);
        let timer = match self.request("timer/set", json!({ "ms": ms })).await {
            Ok(v) => v.get("timer").cloned(),
            Err(e) => {
                eprintln!("opencode: no timer: {}", e.message);
                return;
            }
        };
        let Some(timer) = timer else { return };
        let key = timer.to_string();
        let mut ringing = Ringing {
            host: self,
            timer,
            rung: false,
        };
        poll_fn(|_| {
            if self.fired.lock().contains(&key) {
                ringing.rung = true;
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;
    }

    /// `work`, or None past `limit`; dropped then.
    pub(crate) async fn timeout<F: Future>(&self, limit: Duration, work: F) -> Option<F::Output> {
        let mut work = pin!(work);
        let mut limit = pin!(self.sleep(limit));
        poll_fn(|task| {
            if let Poll::Ready(done) = work.as_mut().poll(task) {
                return Poll::Ready(Some(done));
            }
            limit.as_mut().poll(task).map(|()| None)
        })
        .await
    }
}

/// A call to the host in progress; dropped before its answer, its answer goes nowhere.
struct Asked<'a> {
    host: &'a Host,
    id: u64,
}

impl Drop for Asked<'_> {
    fn drop(&mut self) {
        self.host.waiting.lock().remove(&self.id);
    }
}

/// A timer of the host being waited for; let go before it rang, it is cancelled.
struct Ringing<'a> {
    host: &'a Host,
    timer: Value,
    rung: bool,
}

impl Drop for Ringing<'_> {
    fn drop(&mut self) {
        if !self.rung {
            self.host
                .notify("timer/cancel", &json!({ "timer": self.timer }));
        }
    }
}

/// A response in pieces; let go before its end, it is cancelled.
pub(crate) struct Stream {
    host: Arc<Host>,
    key: String,
    id: Value,
    status: u16,
    ended: bool,
}

impl Stream {
    pub(crate) fn status(&self) -> u16 {
        self.status
    }

    /// What came since the last time, counted for its credit; `Ready(None)` once it ended and all of it was read.
    pub(crate) fn poll_read(&mut self) -> Poll<Option<Vec<u8>>> {
        let mut flows = self.host.flows.lock();
        let flow = flows.entry(self.key.clone()).or_default();
        if !flow.data.is_empty() {
            flow.read += flow.data.len() as u64;
            return Poll::Ready(Some(std::mem::take(&mut flow.data)));
        }
        if flow.ended {
            self.ended = true;
            return Poll::Ready(None);
        }
        Poll::Pending
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        self.host.flows.lock().remove(&self.key);
        if !self.ended {
            self.host.cancelled.lock().insert(self.key.clone());
            self.host
                .notify("net/cancel", &json!({ "stream": self.id }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(id: u64, result: Value) -> String {
        json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
    }

    fn turn(host: &Host, lines: &[String]) -> Vec<Value> {
        for line in lines {
            assert!(host.arrived(line).is_none(), "{line}");
        }
        host.run();
        host.lines()
            .iter()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    fn fired(timer: u64) -> String {
        json!({ "method": "timer/fired", "params": { "timer": timer } }).to_string()
    }

    #[test]
    fn a_sleep_ends_when_its_timer_rings_and_another_timer_does_nothing() {
        let host = Arc::new(Host::default());
        let done = Arc::new(AtomicBool::new(false));
        let (h, d) = (host.clone(), done.clone());
        host.spawn(async move {
            h.sleep(Duration::from_millis(200)).await;
            d.store(true, Ordering::Release);
        });
        let out = turn(&host, &[]);
        assert_eq!(out[0]["method"], "timer/set");
        assert_eq!(out[0]["params"]["ms"], 200);
        assert!(turn(&host, &[answer(1, json!({ "timer": 7 })), fired(9)]).is_empty());
        assert!(!done.load(Ordering::Acquire));
        // Ringing with its answer would be the same.
        assert!(turn(&host, &[fired(7)]).is_empty());
        assert!(done.load(Ordering::Acquire));
    }

    #[test]
    fn what_ends_in_time_cancels_its_timer_and_what_does_not_is_dropped() {
        let host = Arc::new(Host::default());
        let said = Arc::new(Mutex::new(Vec::new()));
        let (h, s) = (host.clone(), said.clone());
        host.spawn(async move {
            let quick = h.request("slow/one", json!({}));
            let first = h.timeout(Duration::from_millis(50), quick).await;
            s.lock().push(first.is_some());
            let slow = h.request("slow/two", json!({}));
            let second = h.timeout(Duration::from_millis(50), slow).await;
            s.lock().push(second.is_some());
        });
        let out = turn(&host, &[]);
        assert_eq!(out[0]["method"], "slow/one");
        assert_eq!(out[1]["method"], "timer/set");
        turn(&host, &[answer(2, json!({ "timer": 3 }))]);
        let out = turn(&host, &[answer(1, json!("ok"))]);
        assert_eq!(out[0]["method"], "timer/cancel");
        assert_eq!(out[1]["method"], "slow/two");
        assert_eq!(out[2]["method"], "timer/set");
        turn(&host, &[answer(4, json!({ "timer": 5 }))]);
        turn(&host, &[fired(5)]);
        assert_eq!(*said.lock(), [true, false]);
        assert!(host.waiting.lock().is_empty(), "nothing waits any more");
        // The answer of what was given up goes nowhere.
        assert!(turn(&host, &[answer(3, json!("late"))]).is_empty());
    }

    #[test]
    fn a_stream_is_read_as_it_comes_and_its_credit_given_back_once_a_turn() {
        let host = Arc::new(Host::default());
        let read = Arc::new(Mutex::new(Vec::new()));
        let (h, r) = (host.clone(), read.clone());
        host.spawn(async move {
            let mut stream = h.stream(json!({ "url": "u" })).await.unwrap();
            assert_eq!(stream.status(), 200);
            while let Some(bytes) = poll_fn(|_| stream.poll_read()).await {
                r.lock().extend(bytes);
            }
        });
        let out = turn(&host, &[]);
        assert_eq!(out[0]["method"], "net/stream");
        let chunk = |data: Value| json!({ "method": "net/chunk", "params": data }).to_string();
        // A chunk may come in the turn of its opening; `é` is two bytes.
        let out = turn(
            &host,
            &[
                answer(1, json!({ "stream": 4, "status": 200, "headers": {} })),
                chunk(json!({ "stream": 4, "data": "é" })),
                chunk(json!({ "stream": 4, "data": "a" })),
            ],
        );
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0]["method"], "net/credit");
        assert_eq!(out[0]["params"], json!({ "stream": 4, "bytes": 3 }));
        let out = turn(
            &host,
            &[
                chunk(json!({ "stream": 4, "dataBase64": "/w==" })),
                json!({ "method": "net/end", "params": { "stream": 4 } }).to_string(),
            ],
        );
        assert!(out.is_empty(), "no credit nor net/cancel after its end");
        assert_eq!(*read.lock(), [0xc3, 0xa9, b'a', 0xff]);
        assert!(host.flows.lock().is_empty());
        assert!(host.tasks.lock().is_empty());
    }

    #[test]
    fn a_stream_let_go_is_cancelled_and_what_was_on_its_way_dropped() {
        let host = Arc::new(Host::default());
        let h = host.clone();
        host.spawn(async move {
            let _stream = h.stream(json!({ "url": "u" })).await.unwrap();
        });
        turn(&host, &[]);
        let out = turn(
            &host,
            &[answer(
                1,
                json!({ "stream": 4, "status": 500, "headers": {} }),
            )],
        );
        assert_eq!(out[0]["method"], "net/cancel");
        let late = json!({ "method": "net/chunk", "params": { "stream": 4, "data": "x" } });
        assert!(turn(&host, &[late.to_string()]).is_empty());
        assert!(host.flows.lock().is_empty());
    }

    #[test]
    fn a_task_waiting_for_another_goes_on_in_the_same_turn() {
        let host = Arc::new(Host::default());
        let opened = Arc::new(AtomicBool::new(false));
        let done = Arc::new(AtomicBool::new(false));
        let (o, d) = (opened.clone(), done.clone());
        host.spawn(async move {
            poll_fn(|_| {
                if o.load(Ordering::Acquire) {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            })
            .await;
            d.store(true, Ordering::Release);
        });
        let (h, o) = (host.clone(), opened.clone());
        host.spawn(async move {
            o.store(true, Ordering::Release);
            h.stir();
        });
        turn(&host, &[]);
        assert!(done.load(Ordering::Acquire));
    }
}
