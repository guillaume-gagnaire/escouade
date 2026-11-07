//! The one thread of a module whose tasks are not its calls (docs/PLUGINS.md §8.3, §8.4), without tokio: what were tasks of
//! a process (reading a stream, a socket, the requests that wait, a timer) are events of the host, each arriving as a line
//! of a turn. `serve::run` is the simple way, a task a call; this is for a plugin that reads a stream, keeps a socket, or
//! starts a task no call began, and serves its lines itself.
//!
//! The host writes what came on the module's input, then calls one turn. A turn gives each line to whom it is for
//! (`arrived`), polls the tasks (`run`) and writes their lines (`lines`): a task that needs the host writes a line, gives
//! the thread back, and goes on at a later turn. Nothing waits inside a turn. A task that waits for what another sets is
//! woken like any future, or by `stir` when what it waits for has no waker: nothing posted in a turn sleeps until the next
//! line.
//!
//! Everything here is plain code on lines: `cargo test` plays it natively, without a module.

use crate::serve::tasks::Tasks;
use base64::Engine;
use escouade_plugin_protocol::Refusal;
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::future::{poll_fn, Future};
use std::pin::pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;

/// What goes to a socket.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    Text(String),
    Binary(Vec<u8>),
}

impl Frame {
    pub fn text(text: impl Into<String>) -> Self {
        Frame::Text(text.into())
    }
}

/// What the host refuses while its queue is full (`ws/send`): the code of a refusal.
const REFUSED: i64 = -32003;
/// How long a socket whose queue is full waits before it sends again.
const FULL_WAIT: Duration = Duration::from_millis(10);

/// What came on a stream and was not read yet.
#[derive(Default)]
struct Flow {
    data: Vec<u8>,
    /// It ended (`net/end`), and why when it broke.
    ended: bool,
    error: Option<String>,
    /// The bytes read since the last `net/credit`.
    read: u64,
}

/// What a socket said.
#[derive(Debug, Clone, PartialEq)]
pub enum SocketEvent {
    Text(String),
    Binary,
    Closed { code: u16, reason: String },
}

/// What a call given up leaves open when its answer comes all the same.
#[derive(Clone, Copy)]
enum Left {
    Stream,
    Socket,
}

#[derive(Default)]
pub struct Host {
    /// The lines of this turn.
    out: Mutex<Vec<String>>,
    next: AtomicU64,
    /// The calls to the host that wait, their answer once it came.
    waiting: Mutex<HashMap<u64, Option<Result<Value, Refusal>>>>,
    /// The calls given up: what they open is shut as soon as it says its name.
    left: Mutex<HashMap<u64, Left>>,
    /// The timers that rang in this turn.
    fired: Mutex<HashSet<String>>,
    /// The streams open, by what `net/stream` answered.
    flows: Mutex<HashMap<String, Flow>>,
    /// The streams let go: what was on its way when they were is dropped.
    cancelled: Mutex<HashSet<String>>,
    /// What the sockets said and was not read yet; those let go.
    sockets: Mutex<HashMap<String, VecDeque<SocketEvent>>>,
    gone: Mutex<HashSet<String>>,
    tasks: Tasks,
}

impl Host {
    pub fn write(&self, line: String) {
        self.out.lock().push(line);
    }

    /// Tells the host or the core something, without waiting for an answer.
    pub fn notify(&self, method: &str, params: impl Serialize) {
        let line = json!({ "jsonrpc": "2.0", "method": method, "params": params });
        self.write(line.to_string());
    }

    /// Starts `task`: it is polled in this turn.
    pub fn spawn(&self, task: impl Future<Output = ()> + Send + 'static) {
        self.tasks.spawn(Box::pin(task));
    }

    /// What a task waits for changed, and nothing wakes it: the tasks are polled once more in this turn.
    pub fn stir(&self) {
        self.tasks.stir();
    }

    /// A line that came: what the plugin has to serve, when it is a call. The rest is dealt with here.
    pub fn arrived(&self, line: &str) -> Option<Value> {
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
                    let mut flows = self.flows.lock();
                    let flow = flows.entry(stream).or_default();
                    flow.ended = true;
                    flow.error = params["error"].as_str().map(String::from);
                }
                None
            }
            Some("ws/message") => {
                let event = match params["text"].as_str() {
                    Some(text) => SocketEvent::Text(text.to_string()),
                    None => SocketEvent::Binary,
                };
                self.socket_said(params, event);
                None
            }
            Some("ws/closed") => {
                let code = u16::try_from(params["code"].as_u64().unwrap_or(0)).unwrap_or(0);
                let reason = params["reason"].as_str().unwrap_or("").to_string();
                self.socket_said(params, SocketEvent::Closed { code, reason });
                None
            }
            _ => Some(msg),
        }
    }

    fn socket_said(&self, params: &Value, event: SocketEvent) {
        let socket = params["socket"].to_string();
        if self.gone.lock().contains(&socket) {
            if matches!(event, SocketEvent::Closed { .. }) {
                self.gone.lock().remove(&socket);
            }
            return;
        }
        self.sockets
            .lock()
            .entry(socket)
            .or_default()
            .push_back(event);
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
        if let Some(left) = self.left.lock().remove(&id) {
            let opened = answer.unwrap_or(Value::Null);
            match left {
                Left::Stream if opened.get("stream").is_some() => {
                    self.cancelled.lock().insert(opened["stream"].to_string());
                    self.notify("net/cancel", json!({ "stream": opened["stream"] }));
                }
                Left::Socket if opened.get("socket").is_some() => {
                    self.gone.lock().insert(opened["socket"].to_string());
                    self.notify("ws/close", json!({ "socket": opened["socket"] }));
                }
                _ => {}
            }
            return;
        }
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
            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(text) {
                flow.data.extend_from_slice(&bytes);
            }
        }
    }

    /// Polls every task, again those that were woken or started meanwhile; then gives the host back what the streams
    /// consumed, one line a stream a turn. Tasks that a turn could not bring to rest get the next one from an empty timer.
    pub fn run(&self) {
        if self.tasks.run() {
            self.notify("timer/set", json!({ "ms": 0 }));
        }
        self.fired.lock().clear();
        let mut flows = self.flows.lock();
        for (stream, flow) in flows.iter_mut().filter(|(_, f)| f.read > 0) {
            let params =
                json!({ "stream": stream.parse::<u64>().unwrap_or(0), "bytes": flow.read });
            self.notify("net/credit", params);
            flow.read = 0;
        }
    }

    /// The lines of this turn.
    pub fn lines(&self) -> Vec<String> {
        std::mem::take(&mut *self.out.lock())
    }

    /// Asks the host `method`: its answer, at a later turn.
    pub async fn request(&self, method: &str, params: Value) -> Result<Value, Refusal> {
        self.asked(method, params, None).await
    }

    async fn asked(
        &self,
        method: &str,
        params: Value,
        opens: Option<Left>,
    ) -> Result<Value, Refusal> {
        let id = self.next.fetch_add(1, Ordering::Relaxed) + 1;
        self.waiting.lock().insert(id, None);
        let mut asked = Asked {
            host: self,
            id,
            opens,
        };
        let line = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        self.write(line.to_string());
        let answer = poll_fn(|_| {
            let mut waiting = self.waiting.lock();
            match waiting.get_mut(&id).and_then(Option::take) {
                Some(answer) => Poll::Ready(answer),
                None => Poll::Pending,
            }
        })
        .await;
        asked.opens = None;
        answer
    }

    /// An HTTP request (`net/fetch`): its status and its text, what is not UTF-8 replaced.
    pub async fn fetch(&self, request: Value) -> Result<(u16, String), Refusal> {
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
    pub async fn stream(self: &Arc<Self>, request: Value) -> Result<Stream, Refusal> {
        let v = self
            .asked("net/stream", request, Some(Left::Stream))
            .await?;
        Ok(Stream {
            host: self.clone(),
            key: v["stream"].to_string(),
            id: v["stream"].clone(),
            status: u16::try_from(v["status"].as_u64().unwrap_or(0)).unwrap_or(0),
            ended: false,
        })
    }

    /// A WebSocket (`ws/open`), `header` carrying the key.
    pub async fn open_socket(
        self: &Arc<Self>,
        url: &str,
        header: &str,
        value: &str,
    ) -> Result<Socket, Refusal> {
        let params = json!({ "url": url, "headers": { header: value } });
        let v = self.asked("ws/open", params, Some(Left::Socket)).await?;
        Ok(Socket {
            host: self.clone(),
            key: v["socket"].to_string(),
            id: v["socket"].clone(),
            closed: false,
        })
    }

    /// Waits `delay` on a timer of the host. One it refuses (past its 64 timers) ends at once, the journal told.
    pub async fn sleep(&self, delay: Duration) {
        let ms = u64::try_from(delay.as_millis()).unwrap_or(u64::MAX);
        let timer = match self.request("timer/set", json!({ "ms": ms })).await {
            Ok(v) => v.get("timer").cloned(),
            Err(e) => {
                let message = format!("no timer: {}", e.message);
                self.notify("log", json!({ "level": "warn", "message": message }));
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
    pub async fn timeout<F: Future>(&self, limit: Duration, work: F) -> Option<F::Output> {
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

/// A call to the host in progress; dropped before its answer, its answer goes nowhere, and what it opens is shut.
struct Asked<'a> {
    host: &'a Host,
    id: u64,
    opens: Option<Left>,
}

impl Drop for Asked<'_> {
    fn drop(&mut self) {
        self.host.waiting.lock().remove(&self.id);
        if let Some(left) = self.opens {
            self.host.left.lock().insert(self.id, left);
        }
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
                .notify("timer/cancel", json!({ "timer": self.timer }));
        }
    }
}

/// A response in pieces; let go before its end, it is cancelled.
pub struct Stream {
    host: Arc<Host>,
    key: String,
    id: Value,
    status: u16,
    ended: bool,
}

impl Stream {
    pub fn status(&self) -> u16 {
        self.status
    }

    /// What came since the last time, counted for its credit; None once it ended and all of it was read; an error once it
    /// broke.
    pub fn poll_next(&mut self) -> Poll<Option<Result<Vec<u8>, String>>> {
        let mut flows = self.host.flows.lock();
        let flow = flows.entry(self.key.clone()).or_default();
        if !flow.data.is_empty() {
            flow.read += flow.data.len() as u64;
            return Poll::Ready(Some(Ok(std::mem::take(&mut flow.data))));
        }
        if flow.ended {
            self.ended = true;
            return Poll::Ready(flow.error.take().map(Err));
        }
        Poll::Pending
    }

    /// `poll_next`, waited for.
    pub async fn next(&mut self) -> Option<Result<Vec<u8>, String>> {
        poll_fn(|_| self.poll_next()).await
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        self.host.flows.lock().remove(&self.key);
        if !self.ended {
            self.host.cancelled.lock().insert(self.key.clone());
            self.host.notify("net/cancel", json!({ "stream": self.id }));
        }
    }
}

/// A WebSocket; let go, it is closed, and what it still says goes nowhere.
pub struct Socket {
    host: Arc<Host>,
    key: String,
    id: Value,
    closed: bool,
}

impl Socket {
    /// Sends `frame`, in its order, once the host took it: a queue that is full (a stream that opened with sound waiting)
    /// is waited for. What fails says why, or nothing when the socket is gone: its `ws/closed` comes then.
    pub async fn send(&mut self, frame: Frame) -> Result<(), String> {
        let mut params = match &frame {
            Frame::Text(text) => json!({ "text": text }),
            Frame::Binary(bytes) => {
                let data = base64::engine::general_purpose::STANDARD.encode(bytes);
                json!({ "dataBase64": data })
            }
        };
        params["socket"] = self.id.clone();
        loop {
            match self.host.request("ws/send", params.clone()).await {
                Ok(_) => return Ok(()),
                Err(e) if e.code == REFUSED && e.message.contains("wait to be sent") => {
                    self.host.sleep(FULL_WAIT).await;
                }
                Err(e) if e.code == REFUSED && e.message.starts_with("no socket") => {
                    return Ok(());
                }
                Err(e) => return Err(e.message),
            }
        }
    }

    /// What it said next.
    pub fn poll_event(&mut self) -> Poll<SocketEvent> {
        let mut sockets = self.host.sockets.lock();
        match sockets.get_mut(&self.key).and_then(VecDeque::pop_front) {
            Some(event) => {
                if matches!(event, SocketEvent::Closed { .. }) {
                    self.closed = true;
                    sockets.remove(&self.key);
                }
                Poll::Ready(event)
            }
            None => Poll::Pending,
        }
    }

    /// Closes it, from this side.
    pub fn close(&mut self) {
        if !std::mem::replace(&mut self.closed, true) {
            self.host.sockets.lock().remove(&self.key);
            self.host.gone.lock().insert(self.key.clone());
            self.host.notify("ws/close", json!({ "socket": self.id }));
        }
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    fn answer(id: u64, result: Value) -> String {
        json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
    }

    fn refusal(id: u64, code: i64, message: &str) -> String {
        json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
            .to_string()
    }

    fn event(method: &str, params: Value) -> String {
        json!({ "method": method, "params": params }).to_string()
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

    #[test]
    fn a_task_woken_by_a_later_one_goes_on_in_the_same_turn() {
        let host = Arc::new(Host::default());
        let waker = Arc::new(Mutex::new(None::<std::task::Waker>));
        let opened = Arc::new(AtomicBool::new(false));
        let done = Arc::new(AtomicBool::new(false));
        let (o, d, w) = (opened.clone(), done.clone(), waker.clone());
        host.spawn(async move {
            poll_fn(|task| {
                if o.load(Ordering::Acquire) {
                    return Poll::Ready(());
                }
                *w.lock() = Some(task.waker().clone());
                Poll::Pending
            })
            .await;
            d.store(true, Ordering::Release);
        });
        host.spawn(async move {
            opened.store(true, Ordering::Release);
            if let Some(waker) = waker.lock().take() {
                waker.wake();
            }
        });
        turn(&host, &[]);
        assert!(done.load(Ordering::Acquire));
    }

    #[test]
    fn a_task_that_wakes_itself_asks_for_another_turn() {
        let host = Host::default();
        host.spawn(poll_fn(|task| {
            task.waker().wake_by_ref();
            Poll::<()>::Pending
        }));
        let out = turn(&host, &[]);
        assert_eq!(out[0]["method"], "timer/set");
        assert_eq!(out[0]["params"]["ms"], 0);
        assert!(out[0].get("id").is_none());
    }

    #[test]
    fn a_host_that_refuses_a_timer_does_not_hold_a_sleep_and_the_journal_says_it() {
        let host = Arc::new(Host::default());
        let (h, slept) = (host.clone(), Arc::new(AtomicBool::new(false)));
        let s = slept.clone();
        host.spawn(async move {
            h.sleep(Duration::from_secs(1)).await;
            s.store(true, Ordering::Release);
        });
        turn(&host, &[]);
        let out = turn(&host, &[refusal(1, REFUSED, "64 timers")]);
        assert_eq!(out[0]["method"], "log");
        assert!(slept.load(Ordering::Acquire));
    }

    #[test]
    fn a_batch_of_answers_reaches_its_calls() {
        let host = Arc::new(Host::default());
        let said = Arc::new(Mutex::new(Vec::new()));
        for method in ["a", "b"] {
            let (h, s) = (host.clone(), said.clone());
            host.spawn(async move {
                let v = h.request(method, json!({})).await.unwrap();
                s.lock().push(v);
            });
        }
        turn(&host, &[]);
        let batch = format!("[{},{}]", answer(1, json!(1)), answer(2, json!(2)));
        turn(&host, &[batch]);
        assert_eq!(*said.lock(), [json!(1), json!(2)]);
    }

    #[test]
    fn what_is_not_utf8_reads_with_its_replacement() {
        let host = Arc::new(Host::default());
        let said = Arc::new(Mutex::new(None));
        let (h, s) = (host.clone(), said.clone());
        host.spawn(async move {
            *s.lock() = h.fetch(json!({ "url": "u" })).await.ok();
        });
        turn(&host, &[]);
        let out = json!({ "status": 200, "bodyBase64": "Yf8=" });
        turn(&host, &[answer(1, out)]);
        assert_eq!(*said.lock(), Some((200, "a\u{fffd}".to_string())));
    }

    #[test]
    fn a_timeout_cancels_its_timer_and_drops_what_it_waited_for() {
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
        turn(&host, &[event("timer/fired", json!({ "timer": 5 }))]);
        assert_eq!(*said.lock(), [true, false]);
        assert!(host.waiting.lock().is_empty(), "nothing waits any more");
    }

    #[test]
    fn a_sleep_ends_when_its_timer_rings_and_another_timer_does_nothing() {
        let fired = |timer: u64| event("timer/fired", json!({ "timer": timer }));
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
        assert!(turn(&host, &[fired(7)]).is_empty());
        assert!(done.load(Ordering::Acquire));
    }

    #[test]
    fn a_stream_gives_its_credit_back_once_a_turn() {
        let host = Arc::new(Host::default());
        let read = Arc::new(Mutex::new(Vec::new()));
        let (h, r) = (host.clone(), read.clone());
        host.spawn(async move {
            let mut stream = h.stream(json!({ "url": "u" })).await.unwrap();
            assert_eq!(stream.status(), 200);
            while let Some(Ok(bytes)) = poll_fn(|_| stream.poll_next()).await {
                r.lock().extend(bytes);
            }
        });
        let out = turn(&host, &[]);
        assert_eq!(out[0]["method"], "net/stream");
        let chunk = |data: Value| event("net/chunk", data);
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
        let end = event("net/end", json!({ "stream": 4 }));
        let out = turn(
            &host,
            &[chunk(json!({ "stream": 4, "dataBase64": "/w==" })), end],
        );
        assert!(out.is_empty(), "no credit nor net/cancel after its end");
        assert_eq!(*read.lock(), [0xc3, 0xa9, b'a', 0xff]);
        assert!(host.flows.lock().is_empty());
    }

    #[test]
    fn a_stream_let_go_is_cancelled_and_what_was_on_its_way_dropped() {
        let host = Arc::new(Host::default());
        let h = host.clone();
        host.spawn(async move {
            let _stream = h.stream(json!({ "url": "u" })).await.unwrap();
        });
        turn(&host, &[]);
        let opened = answer(1, json!({ "stream": 4, "status": 500, "headers": {} }));
        let out = turn(&host, &[opened]);
        assert_eq!(out[0]["method"], "net/cancel");
        let late = event("net/chunk", json!({ "stream": 4, "data": "x" }));
        assert!(turn(&host, &[late]).is_empty());
        assert!(host.flows.lock().is_empty());
    }

    #[test]
    fn a_stream_is_read_as_it_comes_and_says_how_it_broke() {
        let host = Arc::new(Host::default());
        let read = Arc::new(Mutex::new(Vec::new()));
        let (h, r) = (host.clone(), read.clone());
        host.spawn(async move {
            let mut stream = h.stream(json!({ "url": "u" })).await.unwrap();
            while let Some(piece) = stream.next().await {
                r.lock().push(piece);
            }
        });
        turn(&host, &[]);
        let chunk = event("net/chunk", json!({ "stream": 4, "dataBase64": "/w==" }));
        let out = turn(
            &host,
            &[answer(1, json!({ "stream": 4, "status": 200 })), chunk],
        );
        assert_eq!(out[0]["method"], "net/credit");
        assert_eq!(out[0]["params"], json!({ "stream": 4, "bytes": 1 }));
        let end = event("net/end", json!({ "stream": 4, "error": "cut" }));
        assert!(
            turn(&host, &[end]).is_empty(),
            "no net/cancel after its end"
        );
        assert_eq!(*read.lock(), [Ok(vec![0xff]), Err("cut".to_string())],);
    }

    #[test]
    fn what_was_opened_for_a_call_given_up_is_shut_when_it_answers() {
        let host = Arc::new(Host::default());
        let (h, g) = (host.clone(), host.clone());
        host.spawn(async move {
            let open = h.open_socket("wss://x", "authorization", "Bearer k");
            assert!(h.timeout(Duration::from_millis(50), open).await.is_none());
            let stream = h.stream(json!({ "url": "u" }));
            assert!(h.timeout(Duration::from_millis(50), stream).await.is_none());
            g.stir();
        });
        let out = turn(&host, &[]);
        assert_eq!(out[0]["method"], "ws/open");
        turn(&host, &[answer(2, json!({ "timer": 1 }))]);
        let out = turn(&host, &[event("timer/fired", json!({ "timer": 1 }))]);
        assert_eq!(out[0]["method"], "net/stream");
        turn(&host, &[answer(4, json!({ "timer": 2 }))]);
        turn(&host, &[event("timer/fired", json!({ "timer": 2 }))]);
        // Both come late: what they opened is shut, and nothing they say is kept.
        let out = turn(
            &host,
            &[
                answer(1, json!({ "socket": 7 })),
                answer(3, json!({ "stream": 8, "status": 200 })),
                event("ws/message", json!({ "socket": 7, "text": "late" })),
                event("net/chunk", json!({ "stream": 8, "data": "late" })),
            ],
        );
        assert_eq!(out[0]["method"], "ws/close");
        assert_eq!(out[0]["params"]["socket"], 7);
        assert_eq!(out[1]["method"], "net/cancel");
        assert!(host.sockets.lock().is_empty());
        assert!(host.flows.lock().is_empty());
    }

    #[test]
    fn a_socket_sends_in_its_order_waits_for_a_full_queue_and_closes_once() {
        let host = Arc::new(Host::default());
        let said = Arc::new(Mutex::new(Vec::new()));
        let (h, s) = (host.clone(), said.clone());
        host.spawn(async move {
            let mut socket = h
                .open_socket("wss://x", "xi-api-key", "secret")
                .await
                .unwrap();
            socket.send(Frame::Text("one".into())).await.unwrap();
            socket.send(Frame::Binary(vec![1, 2])).await.unwrap();
            let event = poll_fn(|_| socket.poll_event()).await;
            s.lock().push(event);
            socket.close();
        });
        let out = turn(&host, &[]);
        assert_eq!(out[0]["method"], "ws/open");
        assert_eq!(out[0]["params"]["headers"]["xi-api-key"], "secret");
        let out = turn(&host, &[answer(1, json!({ "socket": 3 }))]);
        assert_eq!(out[0]["method"], "ws/send");
        assert_eq!(out[0]["params"], json!({ "socket": 3, "text": "one" }));
        // Refused while the queue is full: sent again, in its place.
        let full = refusal(2, REFUSED, "300000 bytes wait to be sent");
        let out = turn(&host, &[full]);
        assert_eq!(out[0]["method"], "timer/set");
        turn(&host, &[answer(3, json!({ "timer": 9 }))]);
        let out = turn(&host, &[event("timer/fired", json!({ "timer": 9 }))]);
        assert_eq!(out[0]["params"]["text"], "one");
        let out = turn(&host, &[answer(4, Value::Null)]);
        assert_eq!(out[0]["params"]["dataBase64"], "AQI=");
        let out = turn(
            &host,
            &[
                answer(5, Value::Null),
                event("ws/message", json!({ "socket": 3, "text": "heard" })),
            ],
        );
        assert_eq!(
            out[0]["method"], "ws/close",
            "one close, its drop adds none"
        );
        assert_eq!(out.len(), 1);
        assert_eq!(*said.lock(), [SocketEvent::Text("heard".into())]);
        // The close of the other side forgets it.
        let closed = event(
            "ws/closed",
            json!({ "socket": 3, "code": 1000, "reason": "" }),
        );
        turn(&host, &[closed]);
        assert!(host.gone.lock().is_empty());
    }
}
