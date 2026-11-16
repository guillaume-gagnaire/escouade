//! The messages of the remote API (`docs/REMOTE.md`, « L'API distante » of the spec): JSON in text frames, the bytes
//! of the channels (the terminals) and of the raw answers in binary frames, all on one WebSocket.
//!
//! - The client's first message is the handshake ([`ToServer::Hello`]): its version and its device token. The server
//!   answers [`ToClient::Welcome`], or [`ToClient::Refused`] and closes.
//! - A call ([`ToServer::Call`]) has the id its answer takes back ([`ToClient::Reply`]); its arguments are the IPC's,
//!   by the same names; a channel is given as the IPC gives it, `__CHANNEL__:<id>`, an id the client chooses.
//! - What a channel is sent comes back as [`ToClient::Channel`] (JSON: the events of `subscribe`), or in a binary
//!   frame ([`Frame`]: a terminal's output).
//! - What the host publishes (a notification, the count of what waits: `host::HostEvent`) comes as
//!   [`ToClient::Host`].

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::time::Duration;

/// The path of the WebSocket: `ws://127.0.0.1:<port>/remote`.
pub const PATH: &str = "/remote";

/// The largest message the server reads, and the largest frame: a message past it closes the connection.
pub const MAX_MESSAGE: usize = 16 << 20;

/// The time the client has to send its handshake once the WebSocket is open.
pub const HELLO_WAIT: Duration = Duration::from_secs(10);

/// The `Origin` a connection may come with: none (a program of its own), or the page of Escouade's own window
/// (`tauri://localhost` under macOS and Linux, `http://tauri.localhost` under Windows), which fondation 2b connects.
/// Any other is a web page of the machine: refused.
pub const ORIGINS: &[&str] = &[
    "tauri://localhost",
    "http://tauri.localhost",
    "https://tauri.localhost",
];

/// What the client sends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum ToServer {
    /// The handshake: the client's version (the app's, `2.3.0`) and its device's token.
    Hello { version: String, token: String },
    /// A command, `cmd`, with the IPC's arguments.
    Call {
        id: u64,
        cmd: String,
        #[serde(default)]
        args: Map<String, Value>,
    },
}

/// What the server sends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum ToClient {
    /// The handshake passed: the server's version, the client's name for this connection, its device's name.
    Welcome {
        version: String,
        client: String,
        device: String,
    },
    /// The connection refused, then closed: `code` says why (`remoteapi::codes`). For `version`, both versions and
    /// which one to update (`server` or `client`), for the client to offer « Mettre à jour le serveur ».
    #[serde(rename_all = "camelCase")]
    Refused {
        code: String,
        error: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        server_version: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        update: Option<String>,
    },
    /// The answer to the call `id`: `ok` (its value, `null` for none), or `error` (as the IPC gives it: a text for
    /// most) with `code` when the remote API refused it. An answer in bytes comes in a binary frame instead.
    Reply {
        id: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ok: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        code: Option<String>,
    },
    /// A message of the channel `channel`, in JSON.
    Channel { channel: u32, data: Value },
    /// What the host published (`host::HostEvent`).
    Host { event: Value },
    /// A message the server could not take (no call to answer): `code` says why.
    Error { code: String, error: String },
}

/// The kinds of the binary frames: a byte, then the id (8 bytes, big-endian), then the bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frame {
    /// The bytes sent on a channel (a terminal's output).
    Channel = 1,
    /// The answer in bytes to a call (`tauri::ipc::Response`).
    Reply = 2,
}

/// The length of a binary frame's head.
pub const FRAME_HEAD: usize = 9;

/// A binary frame of `kind` for `id`.
pub fn frame(kind: Frame, id: u64, bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(FRAME_HEAD + bytes.len());
    out.push(kind as u8);
    out.extend_from_slice(&id.to_be_bytes());
    out.extend_from_slice(bytes);
    out
}

/// A binary frame read back: its kind, its id, its bytes.
pub fn read_frame(bytes: &[u8]) -> Option<(Frame, u64, &[u8])> {
    if bytes.len() < FRAME_HEAD {
        return None;
    }
    let kind = match bytes[0] {
        1 => Frame::Channel,
        2 => Frame::Reply,
        _ => return None,
    };
    let id = u64::from_be_bytes(bytes[1..FRAME_HEAD].try_into().ok()?);
    Some((kind, id, &bytes[FRAME_HEAD..]))
}

/// The text of a `Reply` whose value `json` is already JSON (Tauri's answer): put in as it is, never parsed again.
pub fn reply_ok(id: u64, json: &str) -> String {
    format!(r#"{{"t":"reply","id":{id},"ok":{json}}}"#)
}

/// The text of a `Channel` message whose data `json` is already JSON (an event, serialized once).
pub fn channel_json(channel: u32, json: &str) -> String {
    format!(r#"{{"t":"channel","channel":{channel},"data":{json}}}"#)
}

/// Which side to update when the versions differ: the older one (the server unless the client is older).
pub fn to_update(server: &str, client: &str) -> &'static str {
    let parse = |v: &str| -> Vec<u64> {
        v.split(['.', '-', '+'])
            .take(3)
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    if parse(client) < parse(server) {
        "client"
    } else {
        "server"
    }
}
