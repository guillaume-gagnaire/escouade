//! The client of an `opencode serve` (OpenCode 1.19, HTTP + SSE, docs/PROTOCOL.md, « OpenCode serve »), which the core
//! runs for the agents of an account (`Launch::shared`): one for all of them (0,7 Go a server, and one serves every
//! worktree by `?directory=`).
//!
//! It listens on 127.0.0.1, on the port it picks (`--port 0`: 4096 when it is free, so its address is read on its
//! output), behind a password drawn at random (`OPENCODE_SERVER_PASSWORD`, Basic `opencode:<password>`) that never leaves
//! Escouade. Its events are read on `/global/event` (all its folders) and handed to the session they are about: each
//! agent is a session of its own, and the sessions its subagents open (`parentID`) go to the agent whose session opened
//! them.

use crate::rules::tr;
use anyhow::{anyhow, bail, Context, Result};
use parking_lot::Mutex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::watch;

/// How long a server may take to say where it listens, its events read.
const START_LIMIT: Duration = Duration::from_secs(60);
/// How long a request may take.
pub(crate) const REQUEST_LIMIT: Duration = Duration::from_secs(60);

/// What hears the events of a session (and of its subagents').
pub(crate) trait Listener: Send + Sync {
    fn event(&self, frame: Value);
}

pub(crate) struct Server {
    /// Its address.
    base: String,
    password: String,
    http: reqwest::Client,
    /// Its event stream is read.
    connected: watch::Receiver<bool>,
    /// Who hears each agent's session, by its id.
    listeners: Mutex<HashMap<String, Arc<dyn Listener>>>,
    /// The agent's session each session of a subagent belongs to.
    owners: Mutex<HashMap<String, String>>,
    /// The models of its connected providers, once read (`GET /config/providers`).
    pub(crate) catalog: tokio::sync::Mutex<Option<Value>>,
}

impl Server {
    /// The client of the server listening at `base`, its events read from now on.
    pub(crate) fn connect(base: String, password: String) -> Result<Arc<Server>> {
        let (connected_tx, connected) = watch::channel(false);
        let http = reqwest::Client::builder()
            .no_proxy()
            .build()
            .context("http client")?;
        let server = Arc::new(Server {
            base: base.clone(),
            password,
            http,
            connected,
            listeners: Mutex::default(),
            owners: Mutex::default(),
            catalog: tokio::sync::Mutex::new(None),
        });
        tokio::spawn(read_events(Arc::downgrade(&server), base, connected_tx));
        Ok(server)
    }

    /// Waits until its events are read.
    pub(crate) async fn ready(&self, lang: &str) -> Result<()> {
        let mut connected = self.connected.clone();
        let wait = async {
            while !*connected.borrow() {
                if connected.changed().await.is_err() {
                    return false;
                }
            }
            true
        };
        match tokio::time::timeout(START_LIMIT, wait).await {
            Ok(true) => Ok(()),
            _ => bail!(tr(
                lang,
                "Le flux d’événements d’OpenCode ne répond pas".into(),
                "OpenCode’s event stream doesn’t answer".into()
            )),
        }
    }

    /// The password of its requests (`OPENCODE_SERVER_PASSWORD`).
    pub(crate) fn password(&self) -> &str {
        &self.password
    }

    /// Its address.
    pub(crate) fn base(&self) -> &str {
        &self.base
    }

    /// A request to the server, in the folder `dir`; its answer (null for none).
    pub(crate) async fn call(
        &self,
        method: reqwest::Method,
        path: &str,
        dir: &str,
        body: Option<Value>,
    ) -> Result<Value> {
        let url = format!("{}{path}", self.base);
        let mut req = self
            .http
            .request(method.clone(), &url)
            .basic_auth("opencode", Some(&self.password))
            .timeout(REQUEST_LIMIT);
        if !dir.is_empty() {
            req = req.query(&[("directory", dir)]);
        }
        if let Some(b) = body {
            req = req.json(&b);
        }
        let res = req
            .send()
            .await
            .with_context(|| format!("{method} {path}"))?;
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        if !status.is_success() {
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
            return Err(
                Status(status.as_u16(), format!("{method} {path}: {status} {said}")).into(),
            );
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

/// Reads `/global/event` while the client lives, again when it drops.
async fn read_events(server: Weak<Server>, base: String, connected: watch::Sender<bool>) {
    loop {
        let Some(s) = server.upgrade() else {
            return;
        };
        let req = s
            .http
            .get(format!("{base}/global/event"))
            .basic_auth("opencode", Some(&s.password))
            .header("Accept", "text/event-stream");
        drop(s);
        match req.send().await {
            Ok(mut res) if res.status().is_success() => {
                let _ = connected.send(true);
                let mut buf: Vec<u8> = Vec::new();
                while let Ok(Some(chunk)) = res.chunk().await {
                    buf.extend_from_slice(&chunk);
                    while let Some(end) = block_end(&buf) {
                        let block: Vec<u8> = buf.drain(..end.0).collect();
                        buf.drain(..end.1);
                        if let (Some(v), Some(s)) = (sse_data(&block), server.upgrade()) {
                            s.dispatch(v);
                        }
                    }
                }
            }
            Ok(res) => eprintln!("opencode: /global/event: {}", res.status()),
            Err(_) => {}
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
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

/// 24 bytes from the system's random generator, in hex.
pub(crate) fn new_password() -> String {
    let mut bytes = [0u8; 24];
    getrandom::fill(&mut bytes).expect("the system's random generator");
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
}
