//! The client of an `opencode serve` (OpenCode 1.19, HTTP + SSE, docs/PROTOCOL.md, « OpenCode serve »), which the core
//! runs for the agents of an account (`Launch::shared`): one for all of them (0,7 Go a server, and one serves every
//! worktree by `?directory=`).
//!
//! It listens on 127.0.0.1, on the port it picks (`--port 0`: 4096 when it is free, so its address is read on its
//! output, by the core: that port is all `local:shared` opens to this plugin), behind a password drawn at random
//! (`OPENCODE_SERVER_PASSWORD`, Basic `opencode:<password>`) that never leaves Escouade. Its events are read on
//! `/global/event` (all its folders) and handed to the session they are about: each agent is a session of its own, and
//! the sessions its subagents open (`parentID`) go to the agent whose session opened them.

use crate::rules::tr;
use anyhow::{anyhow, bail, Result};
use base64::Engine;
use escouade_plugin_kit::host::{Host, Stream};
use parking_lot::Mutex;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::future::poll_fn;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::task::Poll;
use std::time::Duration;

/// How long a server may take to say where it listens, its events read.
const START_LIMIT: Duration = Duration::from_secs(60);
/// How long before the event stream is opened again.
const AGAIN_IN: Duration = Duration::from_millis(200);

/// What hears the events of a session (and of its subagents').
pub(crate) trait Listener: Send + Sync {
    fn event(&self, frame: Value);
}

pub(crate) struct Server {
    host: Arc<Host>,
    /// Its address.
    base: String,
    password: String,
    /// Its event stream is read.
    connected: AtomicBool,
    /// Who hears each agent's session, by its id.
    listeners: Mutex<HashMap<String, Arc<dyn Listener>>>,
    /// The agent's session each session of a subagent belongs to.
    owners: Mutex<HashMap<String, String>>,
    /// The models of its connected providers, once read (`GET /config/providers`).
    catalog: Mutex<Option<Value>>,
    /// One session is reading them.
    reading: AtomicBool,
}

/// The reading of the models a session took on; over, the others look again.
pub(crate) struct Reading(Arc<Server>);

impl Reading {
    pub(crate) fn keep(&self, catalog: Value) {
        *self.0.catalog.lock() = Some(catalog);
    }
}

impl Drop for Reading {
    fn drop(&mut self) {
        self.0.reading.store(false, Ordering::Release);
        self.0.host.stir();
    }
}

impl Server {
    /// The client of the server listening at `base`, its events read from now on.
    pub(crate) fn connect(host: &Arc<Host>, base: String, password: String) -> Arc<Server> {
        let server = Arc::new(Server {
            host: host.clone(),
            base,
            password,
            connected: AtomicBool::new(false),
            listeners: Mutex::default(),
            owners: Mutex::default(),
            catalog: Mutex::new(None),
            reading: AtomicBool::new(false),
        });
        host.spawn(read_events(host.clone(), Arc::downgrade(&server)));
        server
    }

    /// Waits until its events are read.
    pub(crate) async fn ready(&self, lang: &str) -> Result<()> {
        let wait = poll_fn(|_| {
            if self.connected.load(Ordering::Acquire) {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        });
        match self.host.timeout(START_LIMIT, wait).await {
            Some(()) => Ok(()),
            None => bail!(tr(
                lang,
                "Le flux d’événements d’OpenCode ne répond pas".into(),
                "OpenCode’s event stream doesn’t answer".into()
            )),
        }
    }

    /// The models read, or the right to read them: one session reads, the others wait for it.
    pub(crate) async fn catalog(self: &Arc<Self>) -> Result<Value, Reading> {
        poll_fn(|_| {
            if let Some(known) = &*self.catalog.lock() {
                return Poll::Ready(Ok(known.clone()));
            }
            if self.reading.swap(true, Ordering::AcqRel) {
                return Poll::Pending;
            }
            Poll::Ready(Err(Reading(self.clone())))
        })
        .await
    }

    /// The password of its requests (`OPENCODE_SERVER_PASSWORD`).
    pub(crate) fn password(&self) -> &str {
        &self.password
    }

    /// Its address.
    pub(crate) fn base(&self) -> &str {
        &self.base
    }

    fn authorization(&self) -> String {
        let pair = format!("opencode:{}", self.password);
        let pair = base64::engine::general_purpose::STANDARD.encode(pair);
        format!("Basic {pair}")
    }

    /// A request to the server, in the folder `dir`; its answer (null for none). How long it may take is the host's
    /// (`limits.fetchSeconds` of the manifest).
    pub(crate) async fn call(
        &self,
        method: &str,
        path: &str,
        dir: &str,
        body: Option<Value>,
    ) -> Result<Value> {
        let mut url = format!("{}{path}", self.base);
        if !dir.is_empty() {
            url.push_str("?directory=");
            url.push_str(&form(dir));
        }
        let mut headers = Map::new();
        headers.insert("Authorization".into(), json!(self.authorization()));
        let mut request = json!({ "url": url, "method": method });
        if let Some(b) = body {
            headers.insert("Content-Type".into(), json!("application/json"));
            request["body"] = json!(b.to_string());
        }
        request["headers"] = Value::Object(headers);
        let (status, text) = self.host.fetch(request).await.map_err(|e| Unsent {
            said: format!("{method} {path}: {}", e.message),
            late: e.message.contains("no answer within"),
        })?;
        if !(200..300).contains(&status) {
            let said = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| {
                    v["data"]["message"]
                        .as_str()
                        .or(v["message"].as_str())
                        .or(v["error"].as_str())
                        .map(str::to_string)
                })
                .unwrap_or_else(|| truncate(text.trim(), 300));
            let words = format!("{method} {path}: {status} {} {said}", reason(status));
            return Err(Status(status, words).into());
        }
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        Ok(serde_json::from_str(&text).unwrap_or(Value::String(text)))
    }

    /// The events of the session `session` (and of its subagents') go to `listener` from now on.
    pub(crate) fn listen(&self, session: &str, listener: Arc<dyn Listener>) {
        self.listeners.lock().insert(session.to_string(), listener);
    }

    pub(crate) fn forget(&self, session: &str) {
        self.listeners.lock().remove(session);
        self.owners.lock().retain(|_, root| root != session);
    }

    /// One event of `/global/event` (`{directory, payload}`), to the session it is about.
    fn dispatch(&self, event: Value) {
        let mut frame = if event["payload"].is_object() {
            event["payload"].clone()
        } else {
            event.clone()
        };
        if let Some(dir) = event.get("directory").filter(|d| d.is_string()) {
            frame["directory"] = dir.clone();
        }
        let props = &frame["properties"];
        let Some(session) = props["sessionID"]
            .as_str()
            .or(props["info"]["sessionID"].as_str())
            .or(props["part"]["sessionID"].as_str())
            .map(str::to_string)
        else {
            return;
        };
        let root = {
            let mut owners = self.owners.lock();
            if frame["type"] == "session.created" {
                if let Some(parent) = props["info"]["parentID"].as_str() {
                    let root = owners
                        .get(parent)
                        .cloned()
                        .unwrap_or_else(|| parent.to_string());
                    owners.insert(session.clone(), root);
                }
            }
            owners.get(&session).cloned().unwrap_or(session)
        };
        let listener = self.listeners.lock().get(&root).cloned();
        if let Some(l) = listener {
            l.event(frame);
        }
    }
}

/// `text` kept to `max` characters.
fn truncate(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((at, _)) => format!("{}…", &text[..at]),
        None => text.to_string(),
    }
}

/// A request the server answered with an error status.
#[derive(Debug)]
pub(crate) struct Status(pub u16, pub String);

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.1)
    }
}

impl std::error::Error for Status {}

/// Whether `e` is the server's 404.
pub(crate) fn not_found(e: &anyhow::Error) -> bool {
    e.downcast_ref::<Status>().is_some_and(|s| s.0 == 404)
}

/// A request that had no answer: why, and whether it ran out of time.
#[derive(Debug)]
struct Unsent {
    said: String,
    late: bool,
}

impl std::fmt::Display for Unsent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.said)
    }
}

impl std::error::Error for Unsent {}

/// Whether `e` is a request that ran out of time.
pub(crate) fn late(e: &anyhow::Error) -> bool {
    e.downcast_ref::<Unsent>().is_some_and(|u| u.late)
}

/// The words of an HTTP status, which an error of the server is told with.
fn reason(status: u16) -> &'static str {
    match status {
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        413 => "Payload Too Large",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "<unknown status code>",
    }
}

/// A query value, as a form writes it: a folder, with its spaces and its `\`.
fn form(value: &str) -> String {
    let mut out = String::new();
    for b in value.bytes() {
        match b {
            b' ' => out.push('+'),
            b if b.is_ascii_alphanumeric() || b"*-._".contains(&b) => out.push(b as char),
            b => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Reads `/global/event` while the client lives, again when it drops: a task woken by the pieces of its stream
/// (`net/stream`) and by a timer. OpenCode does not replay what it sent: what a cut lost is lost, as in a process.
async fn read_events(host: Arc<Host>, server: Weak<Server>) {
    loop {
        let Some(s) = server.upgrade() else {
            return;
        };
        let request = json!({
            "url": format!("{}/global/event", s.base),
            "method": "GET",
            "headers": {
                "Authorization": s.authorization(),
                "Accept": "text/event-stream",
            },
            "sse": true,
        });
        drop(s);
        match host.stream(request).await {
            Ok(stream) if (200..300).contains(&stream.status()) => {
                if let Some(s) = server.upgrade() {
                    s.connected.store(true, Ordering::Release);
                    host.stir();
                }
                read_stream(stream, &server).await;
                if server.strong_count() == 0 {
                    return;
                }
            }
            Ok(stream) => eprintln!("opencode: /global/event: {}", stream.status()),
            Err(_) => {}
        }
        host.sleep(AGAIN_IN).await;
    }
}

/// Hands the events of `stream` to their sessions, until it ends or the client drops.
async fn read_stream(mut stream: Stream, server: &Weak<Server>) {
    let mut buf: Vec<u8> = Vec::new();
    loop {
        let piece = poll_fn(|_| {
            if server.strong_count() == 0 {
                return Poll::Ready(None);
            }
            stream.poll_next()
        })
        .await;
        let Some(Ok(piece)) = piece else {
            return;
        };
        buf.extend_from_slice(&piece);
        let mut done = 0;
        while let Some(end) = block_end(&buf[done..]) {
            if let (Some(v), Some(s)) = (sse_data(&buf[done..done + end.0]), server.upgrade()) {
                s.dispatch(v);
            }
            done += end.0 + end.1;
        }
        buf.drain(..done);
    }
}

/// Where the first event of `buf` ends: its length, and the length of the blank line after it.
fn block_end(buf: &[u8]) -> Option<(usize, usize)> {
    let lf = buf.windows(2).position(|w| w == b"\n\n").map(|i| (i, 2));
    let crlf = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|i| (i, 4));
    match (lf, crlf) {
        (Some(a), Some(b)) => Some(if a.0 < b.0 { a } else { b }),
        (a, b) => a.or(b),
    }
}

/// The JSON of an event of the stream (its `data:` lines).
fn sse_data(block: &[u8]) -> Option<Value> {
    let text = String::from_utf8_lossy(block);
    let data: Vec<&str> = text
        .lines()
        .filter_map(|l| l.strip_prefix("data:"))
        .map(|l| l.strip_prefix(' ').unwrap_or(l))
        .collect();
    if data.is_empty() {
        return None;
    }
    serde_json::from_str(&data.join("\n")).ok()
}

/// The address of `opencode server listening on http://127.0.0.1:4096`.
pub(crate) fn listening(line: &str) -> Result<String> {
    let at = line
        .find(crate::LISTENING)
        .ok_or_else(|| anyhow!("no address in « {line} »"))?;
    let url = line[at + crate::LISTENING.len()..].trim();
    if !url.starts_with("http") {
        bail!("no address in « {line} »");
    }
    Ok(url.trim_end_matches('/').to_string())
}

/// `bytes` from the system's random generator.
#[cfg(not(target_family = "wasm"))]
fn random(bytes: &mut [u8]) {
    getrandom::fill(bytes).expect("the system's random generator");
}

/// See the native `random`: WASI's, which the host serves.
#[cfg(target_family = "wasm")]
fn random(bytes: &mut [u8]) {
    #[link(wasm_import_module = "wasi_snapshot_preview1")]
    extern "C" {
        fn random_get(buf: *mut u8, len: usize) -> i32;
    }
    // SAFETY: the buffer lives through the call, which fills its length.
    let failed = unsafe { random_get(bytes.as_mut_ptr(), bytes.len()) };
    assert_eq!(failed, 0, "the system's random generator");
}

/// 24 bytes from the system's random generator, in hex.
pub(crate) fn new_password() -> String {
    let mut bytes = [0u8; 24];
    random(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_address_is_read_on_its_output_and_its_events_in_their_blocks() {
        assert_eq!(
            listening("opencode server listening on http://127.0.0.1:4096").unwrap(),
            "http://127.0.0.1:4096"
        );
        assert!(listening("Warning: OPENCODE_SERVER_PASSWORD is not set").is_err());
        let buf = b"data: {\"a\":1}\n\nevent: x\r\ndata: {\"b\":\r\ndata: 2}\r\n\r\nrest";
        let (end, gap) = block_end(buf).unwrap();
        assert_eq!(sse_data(&buf[..end]), Some(json!({ "a": 1 })));
        let rest = &buf[end + gap..];
        let (end, _) = block_end(rest).unwrap();
        assert_eq!(sse_data(&rest[..end]), Some(json!({ "b": 2 })));
        assert_eq!(new_password().len(), 48);
    }

    #[test]
    fn a_folder_goes_in_the_query_as_a_form_writes_it() {
        assert_eq!(form(r"C:\wt a/é-1_.*"), "C%3A%5Cwt+a%2F%C3%A9-1_.*");
    }
}
