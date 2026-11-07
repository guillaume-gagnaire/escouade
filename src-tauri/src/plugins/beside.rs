//! What runs beside the thread of a plugin in WASM (ESC-331, ESC-332, docs/PLUGINS.md §8.3 and §8.4): the capabilities of
//! the host, for `run.js` and `run.wasm` alike. A request is a line the module wrote; what it waits for comes back on the
//! one channel of its thread, as a line, a turn later. All of it goes with the thread: nothing outlives its instance.
//!
//! - **`net/fetch`**: a call, judged by [`Fetcher`], 16 at once.
//! - **`net/stream`**: a response in pieces (`net/chunk`, `net/end`), the same rules at its opening. A plugin holds 256 KiB
//!   it has not consumed (`net/credit`): past it the host does not read the socket, and a slow plugin slows the server,
//!   not the app's memory. 8 streams; 120 s without a byte end one, never one declared `sse`.
//! - **`ws/*`**: a WebSocket, `wss` to the same hosts. 4 sockets, 1 MiB a message, 256 KiB waiting to be sent. Each message
//!   received waits to be on the module's input before the next is read.
//! - **`timer/*`**: 10 ms at least, 64 at once.
//! - **`fs/*` and `proc/*`** ([`Capabilities`]): the same service as `escouade.call`, for what asks by a line (`run.js`).
//!
//! What it reached goes to its journal by its method and its host alone: a query and headers hold keys.

use super::wasm::{headers_of, net, refusal_of, Capabilities, Fetcher, Frame, FAILED, REFUSED};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender, WeakUnboundedSender};
use tokio::sync::{oneshot, Notify};
use tokio::task::{AbortHandle, JoinSet};
use tokio_tungstenite::tungstenite::{protocol::frame::coding::CloseCode, Message};

/// How many streams a plugin has open.
const STREAMS: usize = 8;
/// What a stream holds that the plugin has not consumed.
pub(super) const CREDIT: usize = 256 << 10;
/// How many sockets a plugin has open.
const SOCKETS: usize = 4;
/// A message of a socket.
pub(super) const WS_MESSAGE: usize = 1 << 20;
/// What waits to be sent on a socket.
const WS_QUEUE: usize = 256 << 10;
/// How long a socket waits for the other side's end once it is asked to close.
const WS_CLOSING: Duration = Duration::from_secs(5);
/// How many timers a plugin has set, and the least one.
const TIMERS: usize = 64;
const TIMER_MIN_MS: f64 = 10.0;

/// A failure of a capability: a code of the protocol, and why.
type Failure = (i64, String);

/// A request of the module the host serves itself.
pub struct Request {
    id: Option<Value>,
    method: String,
    params: Value,
}

/// The request of `line`, when it is one of the methods served here.
pub fn request_of(line: &str) -> Option<Request> {
    if !(line.contains("net/")
        || line.contains("ws/")
        || line.contains("timer/")
        || line.contains("fs/")
        || line.contains("proc/"))
    {
        return None;
    }
    let v: Value = serde_json::from_str(line).ok()?;
    let method = v.get("method")?.as_str()?;
    matches!(
        method,
        "net/fetch"
            | "net/stream"
            | "net/credit"
            | "net/cancel"
            | "ws/open"
            | "ws/send"
            | "ws/close"
            | "timer/set"
            | "timer/cancel"
            | "fs/read"
            | "fs/write"
            | "fs/stat"
            | "fs/list"
            | "fs/canon"
            | "proc/spawn"
            | "proc/write"
            | "proc/kill"
    )
    .then(|| Request {
        id: v.get("id").cloned(),
        method: method.to_string(),
        params: v.get("params").cloned().unwrap_or(Value::Null),
    })
}

/// The way back to the module.
#[derive(Clone)]
pub(super) struct Back(pub(super) WeakUnboundedSender<Frame>);

impl Back {
    /// Whether the thread is still there to take it.
    pub(super) fn send(&self, line: &Value, written: Option<oneshot::Sender<()>>) -> bool {
        match self.0.upgrade() {
            Some(tx) => tx.send((format!("{line}\n").into_bytes(), written)).is_ok(),
            None => false,
        }
    }

    fn reply(&self, id: Option<Value>, answer: Result<Value, Failure>) {
        let Some(id) = id else { return };
        self.send(
            &match answer {
                Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                Err((code, message)) => {
                    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
                }
            },
            None,
        );
    }

    pub(super) fn tell(&self, method: &str, params: Value) -> bool {
        self.send(
            &json!({ "jsonrpc": "2.0", "method": method, "params": params }),
            None,
        )
    }

    /// `tell`, once it is on the module's input.
    async fn tell_and_wait(&self, method: &str, params: Value) -> bool {
        let (written, done) = oneshot::channel();
        self.send(
            &json!({ "jsonrpc": "2.0", "method": method, "params": params }),
            Some(written),
        ) && done.await.is_ok()
    }
}

/// What a stream may still hand the plugin.
struct Credit {
    left: AtomicUsize,
    more: Notify,
}

impl Credit {
    fn new() -> Self {
        Self {
            left: AtomicUsize::new(CREDIT),
            more: Notify::new(),
        }
    }

    /// `bytes` consumed: never more than it was handed.
    fn give(&self, bytes: usize) {
        let _ = self
            .left
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |l| {
                Some(l.saturating_add(bytes).min(CREDIT))
            });
        self.more.notify_one();
    }

    /// Up to `at_most` bytes, when there are some.
    async fn take(&self, at_most: usize) -> usize {
        loop {
            let got = self
                .left
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |l| {
                    (l > 0).then(|| l - l.min(at_most))
                });
            if let Ok(left) = got {
                return left.min(at_most);
            }
            self.more.notified().await;
        }
    }
}

/// The parameters of a `net/chunk` of `bytes`: text when they are UTF-8, a character the read cut waiting in `tail` for
/// the rest of it; base64 when they are not. `last`: nothing more will come.
fn chunk_of(stream: u64, tail: &mut Vec<u8>, bytes: &[u8], last: bool) -> Option<Value> {
    tail.extend_from_slice(bytes);
    if tail.is_empty() {
        return None;
    }
    let good = match std::str::from_utf8(tail) {
        Ok(_) => tail.len(),
        Err(e) if e.error_len().is_none() && !last => e.valid_up_to(),
        Err(_) => {
            let data = BASE64.encode(&*tail);
            tail.clear();
            return Some(json!({ "stream": stream, "dataBase64": data }));
        }
    };
    if good == 0 {
        return None;
    }
    let rest = tail.split_off(good);
    let text = String::from_utf8_lossy(tail).into_owned();
    *tail = rest;
    Some(json!({ "stream": stream, "data": text }))
}

/// Whether a message of `size` bytes may wait behind `waiting`. A long one goes when nothing waits: the queue bounds what
/// piles up, not what a message may hold.
fn fits(waiting: usize, size: usize) -> bool {
    waiting == 0 || waiting + size <= WS_QUEUE
}

/// A stream under way, or opening.
struct OpenStream {
    credit: Arc<Credit>,
    task: AbortHandle,
}

/// What a socket is asked to do.
enum Out {
    Send(Message, usize),
    Close,
}

/// A socket under way, or opening.
struct OpenSocket {
    out: UnboundedSender<Out>,
    queued: Arc<AtomicUsize>,
}

/// Reads the response into `net/chunk`s while the plugin has credit: why it ended, if it did not end well.
async fn pump(
    mut res: reqwest::Response,
    stream: u64,
    silent: Option<Duration>,
    credit: &Credit,
    back: &Back,
    name: &str,
) -> Option<String> {
    let mut tail = Vec::new();
    loop {
        let read = match silent {
            Some(d) => match tokio::time::timeout(d, res.chunk()).await {
                Ok(r) => r,
                Err(_) => return Some(format!("{name}: no byte for {} s", d.as_secs())),
            },
            None => res.chunk().await,
        };
        let bytes = match read {
            Ok(Some(b)) => b,
            Ok(None) => break,
            Err(e) => return Some(refusal_of(name, e, Duration::ZERO).1),
        };
        let mut rest = &bytes[..];
        while !rest.is_empty() {
            let (now, later) = rest.split_at(credit.take(rest.len()).await);
            rest = later;
            if let Some(chunk) = chunk_of(stream, &mut tail, now, false) {
                if !back.tell("net/chunk", chunk) {
                    return None;
                }
            }
        }
    }
    if let Some(chunk) = chunk_of(stream, &mut tail, &[], true) {
        back.tell("net/chunk", chunk);
    }
    None
}

/// What a plugin holds beside its thread.
pub struct Beside {
    fetcher: Arc<Fetcher>,
    back: Back,
    refused: Arc<dyn Fn(&str) + Send + Sync>,
    said: Arc<dyn Fn(&str) + Send + Sync>,
    capabilities: Option<Arc<Capabilities>>,
    set: JoinSet<()>,
    streams: Arc<Mutex<HashMap<u64, OpenStream>>>,
    sockets: Arc<Mutex<HashMap<u64, OpenSocket>>>,
    timers: Arc<Mutex<HashMap<u64, AbortHandle>>>,
    next: u64,
}

fn locked<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// What `id` of `params` names.
fn number(params: &Value, key: &str) -> Result<u64, Failure> {
    params.get(key).and_then(Value::as_u64).ok_or_else(|| {
        (
            super::runtime::INVALID_PARAMS,
            format!("{key} is not a number"),
        )
    })
}

impl Beside {
    pub fn new(
        fetcher: Arc<Fetcher>,
        back: WeakUnboundedSender<Frame>,
        refused: Arc<dyn Fn(&str) + Send + Sync>,
        said: Arc<dyn Fn(&str) + Send + Sync>,
        capabilities: Option<Arc<Capabilities>>,
    ) -> Self {
        Self {
            fetcher,
            back: Back(back),
            refused,
            said,
            capabilities,
            set: JoinSet::new(),
            streams: Arc::default(),
            sockets: Arc::default(),
            timers: Arc::default(),
            next: 0,
        }
    }

    /// Serves a request of the module; what lasts goes beside the thread.
    pub fn serve(&mut self, r: Request) {
        let Request { id, method, params } = r;
        // Refused or served, its journal is told by the capability.
        if Capabilities::serves(&method) {
            let answer = match &self.capabilities {
                Some(c) => c.call(&method, &params),
                None => Err((
                    super::runtime::METHOD_NOT_FOUND,
                    format!("unknown method {method}"),
                )),
            };
            return self.back.reply(id, answer);
        }
        let answer = match method.as_str() {
            "net/fetch" => return self.fetch(id, params),
            "net/stream" => return self.stream(id, params),
            "ws/open" => return self.open(id, params),
            "net/credit" => self.credit(&params),
            "net/cancel" => self.cancel(&params),
            "ws/send" => self.send(&params),
            "ws/close" => self.close(&params),
            "timer/set" => return self.timer(id, &params),
            _ => self.untimer(&params),
        };
        if let Err((_, message)) = &answer {
            (self.refused)(&format!("{method}: {message}"));
        }
        self.back.reply(id, answer);
    }

    /// Lets go of what ended.
    pub fn reap(&mut self) {
        while self.set.try_join_next().is_some() {}
    }

    fn fetch(&mut self, id: Option<Value>, params: Value) {
        let (fetcher, back) = (self.fetcher.clone(), self.back.clone());
        let (refused, said) = (self.refused.clone(), self.said.clone());
        let id = Some(id.unwrap_or(Value::Null));
        self.set.spawn_on(
            async move {
                let method = params.get("method").and_then(Value::as_str);
                let what = fetcher.describe(&params, method.unwrap_or("GET"));
                let answer = fetcher.fetch(&params).await;
                match &answer {
                    Ok(r) => said(&format!("net/fetch {what}: {}", r["status"])),
                    Err((_, message)) => refused(&format!("net/fetch: {message}")),
                }
                back.reply(id, answer);
            },
            net(),
        );
    }

    fn stream(&mut self, id: Option<Value>, params: Value) {
        let streams = self.streams.clone();
        let mut open = locked(&streams);
        if open.len() >= STREAMS {
            drop(open);
            let message = format!("{STREAMS} streams are open");
            (self.refused)(&format!("net/stream: {message}"));
            return self.back.reply(id, Err((REFUSED, message)));
        }
        self.next += 1;
        let stream = self.next;
        let credit = Arc::new(Credit::new());
        let (fetcher, back) = (self.fetcher.clone(), self.back.clone());
        let (refused, said) = (self.refused.clone(), self.said.clone());
        let (own, gone) = (credit.clone(), streams.clone());
        let task = self.set.spawn_on(
            async move {
                let what = fetcher.describe(&params, "net/stream");
                let res = match fetcher.open_stream(&params).await {
                    Ok(r) => r,
                    Err((code, message)) => {
                        locked(&gone).remove(&stream);
                        refused(&format!("net/stream: {message}"));
                        return back.reply(id, Err((code, message)));
                    }
                };
                let (status, headers) = (res.status().as_u16(), headers_of(res.headers()));
                let name = res.url().host_str().unwrap_or_default().to_string();
                back.reply(
                    id,
                    Ok(json!({ "stream": stream, "status": status, "headers": headers })),
                );
                said(&format!("{what}: {status}"));
                let sse = params.get("sse").and_then(Value::as_bool) == Some(true);
                let silent = (!sse).then(|| fetcher.idle());
                let error = pump(res, stream, silent, &own, &back, &name).await;
                locked(&gone).remove(&stream);
                match error {
                    Some(error) => {
                        refused(&format!("net/stream: {error}"));
                        back.tell("net/end", json!({ "stream": stream, "error": error }));
                    }
                    None => {
                        back.tell("net/end", json!({ "stream": stream }));
                    }
                }
            },
            net(),
        );
        open.insert(stream, OpenStream { credit, task });
    }

    fn credit(&self, params: &Value) -> Result<Value, Failure> {
        let stream = number(params, "stream")?;
        let bytes = number(params, "bytes")?;
        if let Some(s) = locked(&self.streams).get(&stream) {
            s.credit.give(usize::try_from(bytes).unwrap_or(usize::MAX));
        }
        Ok(Value::Null)
    }

    fn cancel(&self, params: &Value) -> Result<Value, Failure> {
        let stream = number(params, "stream")?;
        if let Some(s) = locked(&self.streams).remove(&stream) {
            s.task.abort();
        }
        Ok(Value::Null)
    }

    fn open(&mut self, id: Option<Value>, params: Value) {
        let sockets = self.sockets.clone();
        let mut open = locked(&sockets);
        if open.len() >= SOCKETS {
            drop(open);
            let message = format!("{SOCKETS} sockets are open");
            (self.refused)(&format!("ws/open: {message}"));
            return self.back.reply(id, Err((REFUSED, message)));
        }
        self.next += 1;
        let socket = self.next;
        let (out, rx) = unbounded_channel();
        let queued = Arc::new(AtomicUsize::new(0));
        let (fetcher, back) = (self.fetcher.clone(), self.back.clone());
        let (refused, said) = (self.refused.clone(), self.said.clone());
        let (waiting, gone) = (queued.clone(), sockets.clone());
        self.set.spawn_on(
            async move {
                let what = fetcher.describe(&params, "ws/open");
                let opened =
                    match tokio::time::timeout(fetcher.seconds(), fetcher.open_socket(&params))
                        .await
                    {
                        Ok(o) => o,
                        Err(_) => Err((FAILED, "no answer in time".to_string())),
                    };
                let ws = match opened {
                    Ok(ws) => ws,
                    Err((code, message)) => {
                        locked(&gone).remove(&socket);
                        refused(&format!("ws/open: {message}"));
                        return back.reply(id, Err((code, message)));
                    }
                };
                back.reply(id, Ok(json!({ "socket": socket })));
                said(&what);
                let (code, reason) = carry(ws, socket, rx, &waiting, &back).await;
                locked(&gone).remove(&socket);
                said(&format!("ws/closed {socket}: {code}"));
                back.tell(
                    "ws/closed",
                    json!({ "socket": socket, "code": code, "reason": reason }),
                );
            },
            net(),
        );
        open.insert(socket, OpenSocket { out, queued });
    }

    fn send(&self, params: &Value) -> Result<Value, Failure> {
        let socket = number(params, "socket")?;
        let invalid = |m: &str| (super::runtime::INVALID_PARAMS, m.to_string());
        let message = match (
            params.get("text").and_then(Value::as_str),
            params.get("dataBase64").and_then(Value::as_str),
        ) {
            (Some(text), None) => Message::text(text),
            (None, Some(data)) => Message::binary(
                BASE64
                    .decode(data)
                    .map_err(|_| invalid("dataBase64 is not base64"))?,
            ),
            _ => return Err(invalid("text or dataBase64")),
        };
        let size = message.len();
        if size > WS_MESSAGE {
            return Err((REFUSED, format!("a message over {WS_MESSAGE} bytes")));
        }
        let sockets = locked(&self.sockets);
        let Some(s) = sockets.get(&socket) else {
            return Err((REFUSED, format!("no socket {socket}")));
        };
        let waiting = s.queued.load(Ordering::Acquire);
        if !fits(waiting, size) {
            return Err((REFUSED, format!("{waiting} bytes wait to be sent")));
        }
        s.queued.fetch_add(size, Ordering::AcqRel);
        let _ = s.out.send(Out::Send(message, size));
        Ok(Value::Null)
    }

    fn close(&self, params: &Value) -> Result<Value, Failure> {
        let socket = number(params, "socket")?;
        match locked(&self.sockets).get(&socket) {
            Some(s) => {
                let _ = s.out.send(Out::Close);
                Ok(Value::Null)
            }
            None => Err((REFUSED, format!("no socket {socket}"))),
        }
    }

    fn timer(&mut self, id: Option<Value>, params: &Value) {
        let ms = params.get("ms").and_then(Value::as_f64);
        let Some(ms) = ms.filter(|m| m.is_finite() && *m >= 0.0) else {
            let failure = (
                super::runtime::INVALID_PARAMS,
                "ms is not a number".to_string(),
            );
            (self.refused)(&format!("timer/set: {}", failure.1));
            return self.back.reply(id, Err(failure));
        };
        let timers = self.timers.clone();
        let mut set = locked(&timers);
        if set.len() >= TIMERS {
            drop(set);
            let message = format!("{TIMERS} timers are set");
            (self.refused)(&format!("timer/set: {message}"));
            return self.back.reply(id, Err((REFUSED, message)));
        }
        self.next += 1;
        let timer = self.next;
        // Its answer first: `timer/fired` never comes before the plugin knows the timer.
        self.back.reply(id, Ok(json!({ "timer": timer })));
        let (back, gone) = (self.back.clone(), timers.clone());
        let wait = Duration::from_secs_f64((ms.max(TIMER_MIN_MS) / 1000.0).min(3e9));
        let task = self.set.spawn_on(
            async move {
                tokio::time::sleep(wait).await;
                locked(&gone).remove(&timer);
                back.tell("timer/fired", json!({ "timer": timer }));
            },
            net(),
        );
        set.insert(timer, task);
    }

    fn untimer(&self, params: &Value) -> Result<Value, Failure> {
        let timer = number(params, "timer")?;
        if let Some(t) = locked(&self.timers).remove(&timer) {
            t.abort();
        }
        Ok(Value::Null)
    }
}

/// Carries a socket's two directions until it ends: its code, and why.
async fn carry(
    ws: super::socket::Ws,
    socket: u64,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<Out>,
    queued: &AtomicUsize,
    back: &Back,
) -> (u16, String) {
    let (mut sink, mut read) = ws.split();
    let writing = async {
        while let Some(out) = rx.recv().await {
            match out {
                Out::Send(message, size) => {
                    let sent = sink.send(message).await;
                    queued.fetch_sub(size, Ordering::AcqRel);
                    if let Err(e) = sent {
                        return (1006, e.to_string());
                    }
                }
                Out::Close => {
                    let _ = sink.send(Message::Close(None)).await;
                    tokio::time::sleep(WS_CLOSING).await;
                    return (1000, "the other side did not answer".to_string());
                }
            }
        }
        std::future::pending().await
    };
    let reading = async {
        loop {
            let params = match read.next().await {
                Some(Ok(Message::Text(text))) => {
                    json!({ "socket": socket, "text": text.as_str() })
                }
                Some(Ok(Message::Binary(data))) => {
                    json!({ "socket": socket, "dataBase64": BASE64.encode(&data) })
                }
                Some(Ok(Message::Close(frame))) => {
                    return frame.map_or((1005, String::new()), |f| {
                        (u16::from(f.code), f.reason.to_string())
                    });
                }
                Some(Ok(_)) => continue,
                Some(Err(tokio_tungstenite::tungstenite::Error::Capacity(e))) => {
                    return (u16::from(CloseCode::Size), e.to_string());
                }
                Some(Err(e)) => return (1006, e.to_string()),
                None => return (1006, "the connection closed".to_string()),
            };
            // One at a time: a server faster than its plugin waits on its socket, not in the app's memory.
            if !back.tell_and_wait("ws/message", params).await {
                return (1001, "the plugin is gone".to_string());
            }
        }
    };
    tokio::select! {
        end = writing => end,
        end = reading => end,
    }
}

#[cfg(test)]
#[path = "beside_tests.rs"]
mod tests;
