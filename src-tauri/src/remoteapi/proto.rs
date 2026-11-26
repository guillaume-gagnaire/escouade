//! The messages of the remote API (`docs/REMOTE.md`, « L'API distante » of the spec): JSON in text frames, the bytes
//! of the channels (the terminals), of the raw answers and of the raw bodies in binary frames, all on one WebSocket.
//!
//! - The client's first message is the handshake ([`ToServer::Hello`]): its version, its device token, and the session
//!   it had before a cut, if any. The server answers [`ToClient::Welcome`] (the session, kept or new), or
//!   [`ToClient::Refused`] and closes.
//! - A call ([`ToServer::Call`]) has the id its answer takes back ([`ToClient::Reply`]); its arguments are the IPC's,
//!   by the same names; a channel is given as the IPC gives it, `__CHANNEL__:<id>`, an id the client chooses (unique in
//!   its session). A raw body (`attach_chunk`, `fs_write_bytes`) comes first in binary frames ([`Frame::Body`]), then
//!   the call says its length and its headers.
//! - What a channel is sent comes back as [`ToClient::Channel`] (JSON: the events of `subscribe`), or in a binary
//!   frame: a terminal's output, numbered ([`Frame::Term`]), or bytes outside the numbering ([`Frame::Channel`]: a
//!   replay of a screen, then [`ToClient::Mark`]).
//! - After a cut, the client resumes its terminals ([`ToServer::Resume`]: the end of the last bytes it received from
//!   each); a gap the server made when the client read too slowly is said ([`ToClient::Gap`]), as is a state to read
//!   again whole ([`ToClient::Resync`]).
//! - What the host publishes (a notification, the count of what waits: `host::HostEvent`) comes as
//!   [`ToClient::Host`].

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::time::Duration;

/// The path of the WebSocket: `ws://127.0.0.1:<port>/remote`.
pub const PATH: &str = "/remote";

/// The largest message the server reads, and the largest it sends: a message past it closes the connection; an answer
/// or a body past it goes in parts.
pub const MAX_MESSAGE: usize = 16 << 20;

/// The largest raw body of a call (`fs_write_bytes`: a file of the editor, 64 MB at most, as `fs_read_bytes`).
pub const MAX_BODY: usize = 64 << 20;

/// The time the client has to send its handshake once the WebSocket is open.
pub const HELLO_WAIT: Duration = Duration::from_secs(10);

/// The longest session id a client may give back.
pub const MAX_SESSION: usize = 64;

/// The most channels a client resumes in one message.
pub const MAX_RESUME: usize = 1024;

/// The most headers of a raw call, and their sizes.
pub const MAX_HEADERS: usize = 32;
pub const MAX_HEADER_NAME: usize = 64;
pub const MAX_HEADER_VALUE: usize = 8 << 10;

/// The close code of a connection that read nothing for too long, or let too much wait unread.
pub const CLOSE_SLOW: u16 = 4002;

/// The `Origin` a connection may come with: none (a program of its own), or the page of Escouade's own window
/// (`tauri://localhost` under macOS and Linux, `http://tauri.localhost` under Windows). Any other is a web page of the
/// machine: refused.
pub const ORIGINS: &[&str] = &[
    "tauri://localhost",
    "http://tauri.localhost",
    "https://tauri.localhost",
];

/// The origin of the Android app's WebView (SR1-09, ESC-631): a name of its own (`apps/mobile/scripts/android-hostname.mjs`),
/// under `.invalid`, which no site has (RFC 6761), in place of `https://localhost`, which any page served on port 443
/// of a machine has.
pub const ANDROID_ORIGIN: &str = "https://app.escouade.invalid";

/// The `Origin`s an app of the phone comes with besides (ESC-567), through the phone's tunnel alone: its WebView's
/// (Capacitor's, under iOS and Android), those of an app built before its own name too. Never on the loopback, where
/// they would be a page of the machine.
pub const PHONE_ORIGINS: &[&str] = &[
    "capacitor://localhost",
    ANDROID_ORIGIN,
    "https://localhost",
    "http://localhost",
];

/// The `Origin`s an app of the phone comes with through Tailscale (SR1-09, ESC-631): its WebView's alone, never a
/// `localhost` of any kind, which a web page of another machine of the tailnet may be served from. (No browser makes
/// `capacitor://`.)
pub const TAILSCALE_ORIGINS: &[&str] = &["capacitor://localhost", ANDROID_ORIGIN];

/// A terminal to resume: its channel, and the end of the last bytes the client received of it (none: nothing yet).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resume {
    pub channel: u32,
    #[serde(default)]
    pub after: Option<u64>,
}

/// What the client sends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum ToServer {
    /// The handshake: the client's version (the app's, `2.3.0`), its device's token, and the session it had before a
    /// cut (its terminals are resumed in it).
    Hello {
        version: String,
        token: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session: Option<String>,
    },
    /// A command, `cmd`, with the IPC's arguments. `body`: the length of its raw body, sent before in binary frames
    /// ([`Frame::Body`]), with its `headers`.
    Call {
        id: u64,
        cmd: String,
        #[serde(default)]
        args: Map<String, Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        body: Option<u64>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        headers: Map<String, Value>,
    },
    /// The terminals of the session to take over again, each from where the client stopped.
    Resume { channels: Vec<Resume> },
    /// In place of the handshake, a new device's first connection (ESC-514): the pairing code `escouade-server pair`
    /// showed (`remoteapi::pairing`), and the name the user gives the device. Answered by [`ToClient::Paired`] (its
    /// token, given once) or [`ToClient::Refused`] (`pairing`), then closed: the device connects again with its token.
    Pair { code: String, name: String },
    /// In place of the handshake, a device that forgets the server (ESC-537, « Retirer un serveur de ce poste »): its
    /// token, which the server revokes (its own name said as who revoked it); answered by [`ToClient::Unpaired`] or
    /// [`ToClient::Refused`] (`token`), then closed.
    Unpair { token: String },
    /// After the handshake, a measure of the way to the server and back (ESC-537: the latency the status bar says):
    /// answered at once by [`ToClient::Pong`] with the same `id`.
    Ping { id: u64 },
}

/// The machine a server runs on (ESC-537), as the client says it: « escouade@vps.example.net ».
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Machine {
    /// Its name on its network (`gethostname`).
    pub host: String,
    /// The account the server runs as.
    pub user: String,
}

/// A device's token refused because it was revoked (ESC-537): when, and by whom (the name of the device that asked
/// for it; none: the server itself, `escouade-server devices revoke`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Revoked {
    /// The name of the device revoked.
    pub device: String,
    /// When (ms since the epoch).
    pub at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
}

/// What the server sends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum ToClient {
    /// The handshake passed: the server's version, the client's name for this connection, its device's name, its
    /// session (`resumed`: the one it gave, whose terminals it may resume), the desktop's commands.
    Welcome {
        version: String,
        client: String,
        device: String,
        #[serde(default)]
        session: String,
        #[serde(default)]
        resumed: bool,
        /// The desktop's commands (`desk-only` here): a client that has a desktop of its own does them there.
        #[serde(default)]
        desk: Vec<String>,
        /// The server keeps its secrets (the keys of the integrations, of Cursor, of the voices and of the plugins) in a
        /// file of its data readable by its owner alone, not in a keychain (`escouade-server`, ESC-514): the client says
        /// so where they are given.
        #[serde(default, rename = "secretsInFile")]
        secrets_in_file: bool,
        /// The machine the server runs on (ESC-537): what the title bar and a terminal's head say. None from a window's
        /// own core.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        machine: Option<Machine>,
        /// A session taken again (`resumed`): the events of the state emitted since its client left (ESC-538,
        /// « Reconnecté à Travail · 12 événements rattrapés »). The state itself is read again whole.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        missed: Option<u64>,
    },
    /// The device forgot the server (ESC-537): its token is revoked. The connection then closes.
    Unpaired {},
    /// The answer to [`ToServer::Ping`].
    Pong { id: u64 },
    /// The pairing passed (ESC-514): the new device and its token, which the client keeps and gives at each handshake
    /// from now on. Given once; the connection then closes.
    Paired {
        device: super::devices::Device,
        token: String,
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
        /// With `update` at `server` (ESC-515): how to put the server at the client's version, for the client to offer
        /// it (the screen is the client's).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        server_update: Option<ServerUpdate>,
        /// For `pairing` (ESC-537): why, for the client to say it in its words (`none`, `expired`, `wrong`, `tries`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        /// For `token` (ESC-537): the token was a device's, revoked; when and by whom.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        revoked: Option<Revoked>,
    },
    /// The answer to the call `id`: `ok` (its value, `null` for none), or `error` (as the IPC gives it: a text for
    /// most) with `code` when the remote API refused it. An answer in bytes comes in binary frames instead.
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
    /// The output of the terminal of `channel` is numbered from `end` on, after a replay of its screen.
    Mark { channel: u32, end: u64 },
    /// `bytes` of the terminal of `channel` left its ring before the client could have them: what follows is all
    /// that is kept.
    Lost { channel: u32, bytes: u64 },
    /// The replay of a resume of `channel` is over (`ended`: its program ended; `gone`: no such terminal in the
    /// session, nothing to resume).
    Resumed {
        channel: u32,
        #[serde(default)]
        ended: bool,
        #[serde(default)]
        gone: bool,
    },
    /// The client read too slowly: some output of the terminal of `channel` was not sent. To resume.
    Gap { channel: u32 },
    /// The client read too slowly: events of the state were not sent. To read it again whole (`subscribe`).
    Resync {},
    /// What the host published (`host::HostEvent`).
    Host { event: Value },
    /// A message the server could not take (no call to answer): `code` says why.
    Error { code: String, error: String },
}

/// The kinds of the binary frames: a byte, then the id (8 bytes, big-endian), then the bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frame {
    /// The bytes sent on a channel outside the numbering (a replay of a screen).
    Channel = 1,
    /// The answer in bytes to a call (`tauri::ipc::Response`), or its last part.
    Reply = 2,
    /// A terminal's output, numbered: after the id, the end of these bytes (8 bytes, big-endian), then the bytes.
    Term = 3,
    /// A part of an answer in bytes past [`MAX_MESSAGE`], before its last ([`Frame::Reply`]).
    ReplyPart = 4,
    /// From the client: a part of the raw body of the call of this id, before the call.
    Body = 5,
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

/// A terminal's output on `channel`, its bytes ending at `end`.
pub fn term_frame(channel: u32, end: u64, bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(FRAME_HEAD + 8 + bytes.len());
    out.push(Frame::Term as u8);
    out.extend_from_slice(&u64::from(channel).to_be_bytes());
    out.extend_from_slice(&end.to_be_bytes());
    out.extend_from_slice(bytes);
    out
}

/// A binary frame read back: its kind, its id, its bytes (for [`Frame::Term`], the end first).
pub fn read_frame(bytes: &[u8]) -> Option<(Frame, u64, &[u8])> {
    if bytes.len() < FRAME_HEAD {
        return None;
    }
    let kind = match bytes[0] {
        1 => Frame::Channel,
        2 => Frame::Reply,
        3 => Frame::Term,
        4 => Frame::ReplyPart,
        5 => Frame::Body,
        _ => return None,
    };
    let id = u64::from_be_bytes(bytes[1..FRAME_HEAD].try_into().ok()?);
    Some((kind, id, &bytes[FRAME_HEAD..]))
}

/// The end and the bytes of a [`Frame::Term`]'s rest.
pub fn read_term(rest: &[u8]) -> Option<(u64, &[u8])> {
    let end = u64::from_be_bytes(rest.get(..8)?.try_into().ok()?);
    Some((end, &rest[8..]))
}

/// The frames of an answer in bytes to `id`: parts of [`MAX_MESSAGE`] at most, the last one a [`Frame::Reply`].
pub fn reply_frames(id: u64, bytes: &[u8]) -> Vec<Vec<u8>> {
    let piece = MAX_MESSAGE - FRAME_HEAD;
    if bytes.len() <= piece {
        return vec![frame(Frame::Reply, id, bytes)];
    }
    let pieces: Vec<&[u8]> = bytes.chunks(piece).collect();
    let last = pieces.len() - 1;
    pieces
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let kind = if i == last {
                Frame::Reply
            } else {
                Frame::ReplyPart
            };
            frame(kind, id, p)
        })
        .collect()
}

/// The text of a `Reply` whose value `json` is already JSON (Tauri's answer): put in as it is, never parsed again.
pub fn reply_ok(id: u64, json: &str) -> String {
    format!(r#"{{"t":"reply","id":{id},"ok":{json}}}"#)
}

/// The text of a `Channel` message whose data `json` is already JSON (an event, serialized once).
pub fn channel_json(channel: u32, json: &str) -> String {
    format!(r#"{{"t":"channel","channel":{channel},"data":{json}}}"#)
}

/// How a server is put at a client's version (ESC-515): `command`, `escouade-server update --to <version>` run on the
/// server (it checks the signature and waits for the turns), or `image`, a new image of the container (the server
/// never changes itself there), and what to run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerUpdate {
    pub how: String,
    pub command: String,
}

/// The image of the server (`deploy/Dockerfile`, published by the Release).
pub const SERVER_IMAGE: &str = "ghcr.io/guillaume-gagnaire/escouade/escouade-server";

/// How to put this server at `client`'s version: `in_image`, it runs in the image of Escouade.
pub fn server_update(client: &str, in_image: bool) -> ServerUpdate {
    let version: String = client
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
        .take(32)
        .collect();
    if in_image {
        ServerUpdate {
            how: "image".into(),
            command: format!("docker pull {SERVER_IMAGE}:{version} && docker compose up -d"),
        }
    } else {
        ServerUpdate {
            how: "command".into(),
            command: format!("escouade-server update --to {version}"),
        }
    }
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
