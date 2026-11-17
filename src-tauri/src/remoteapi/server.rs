//! The WebSocket of the remote API (`docs/REMOTE.md`, « La fondation commune », points 2 to 6; « La fondation 2b »).
//!
//! - **Off unless asked**: a build of development of the desktop app serves it when `ESCOUADE_REMOTE_API` gives a
//!   port (never a release: a variable opens nothing there), a process without a window when it asks
//!   (`Headless::serve_remote`); never while the policies of the device forbid it (`servers.remoteApi`), checked at
//!   the start and at each connection.
//! - **On the loopback alone**: it listens on 127.0.0.1, and is reached through a tunnel (SSH, WireGuard: the server
//!   mode's). No TLS here.
//! - **Before the WebSocket opens**: the path, a `Host` of the loopback (DNS rebinding), and no `Origin` but the one of
//!   Escouade's own window (a web page of the machine is refused). Bounded ([`Limits`]): the handshakes under way, and
//!   the failed ones in a minute (past them, a connection is closed as soon as it is accepted, for the rest of the
//!   minute).
//! - **The handshake**: the first message gives the device's token (compared in constant time, `devices`), the
//!   client's version, which must be the server's (« Mettre à jour le serveur »), and the session it had before a cut
//!   (`sessions`). Bounded: the connections of a device, and in all.
//! - **Then** the calls, each on the lane of its command (`windowthread::lane_of`, as the IPC: a terminal's
//!   keystrokes keep their order), as the client of this connection (`remoteapi::Client`), so many at once at most;
//!   their raw bodies before them in binary frames; the channels and the events pushed on the same connection,
//!   through a bounded queue (`outbox`): a client that reads too slowly gets a re-read of the state and resumes its
//!   terminals; one that reads nothing is closed. A message past `proto::MAX_MESSAGE` closes it.
//! - **The log** (`app.log`) says each connection, its device's name, each refusal and why: never a token, a session
//!   id, a path of the disk, an argument nor what a terminal printed.

use super::devices::Devices;
use super::outbox::{Bounds, Lane, Outbox};
use super::proto::{self, Frame, ToClient, ToServer};
use super::sessions::Sessions;
use super::{codes, Call, Channels, Client, Failure, Outcome, RawBody, Reply, Services};
use crate::termlog::{Replay, Sink};
use crate::windowthread::{lane_of, Lanes, Running};
use anyhow::{Context, Result};
use futures_util::{FutureExt, SinkExt, StreamExt};
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::net::{Ipv4Addr, SocketAddr};
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};
use tauri::ipc::InvokeResponseBody;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Notify;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, WebSocketConfig};
use tokio_tungstenite::tungstenite::Message;

/// The variable that turns the API on in a build of development of the desktop app: the port to listen on (0: any
/// free one). Read in a build of development alone ([`port_asked`]).
pub const PORT_VAR: &str = "ESCOUADE_REMOTE_API";
/// In a debug build only, a device token the tests give (fondation 2b's e2e): a device « test » kept in memory.
pub const TEST_TOKEN_VAR: &str = "ESCOUADE_REMOTE_API_TOKEN";

/// The port `ESCOUADE_REMOTE_API` gives, in a build of development; never in a release (a variable opens nothing
/// there, as SEC-13 of the audit asks of the policies).
pub fn port_asked() -> Option<u16> {
    if !cfg!(debug_assertions) {
        return None;
    }
    std::env::var(PORT_VAR).ok()?.trim().parse().ok()
}

/// The bounds of the API (`docs/REMOTE.md`, « La fondation 2b »).
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// The send queue of a connection.
    pub queue: Bounds,
    /// The connections of one device at once.
    pub per_device: usize,
    /// The connections in all at once.
    pub total: usize,
    /// The connections whose handshake is under way at once.
    pub pending: usize,
    /// The failed handshakes in a minute before the next connections are closed at once, for the rest of it.
    pub failures_per_minute: usize,
    /// The calls of one connection waiting for their answer at once.
    pub in_flight: usize,
    /// The raw bodies of one connection waiting for their call at once, and their bytes.
    pub bodies: usize,
    pub body_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            queue: Bounds::default(),
            per_device: 4,
            total: 16,
            pending: 8,
            failures_per_minute: 20,
            in_flight: 256,
            bodies: 8,
            body_bytes: proto::MAX_BODY + (8 << 20),
        }
    }
}

/// A connection that passed the handshake.
struct Conn {
    client: Client,
    device: String,
    session: String,
    sessions: Arc<Sessions>,
    outbox: Outbox,
    lanes: Arc<Lanes>,
    in_flight: AtomicUsize,
    limit: usize,
    /// Told to end: its device revoked, the server stopped.
    kicked: Notify,
}

impl Conn {
    fn push(&self, message: Message, lane: Lane) {
        self.outbox.push(message, lane);
    }

    fn send(&self, message: &ToClient) {
        match serde_json::to_string(message) {
            Ok(text) => self.push(Message::text(text), Lane::Control),
            Err(e) => log::warn!("remote API: a message not written: {e}"),
        }
    }

    fn answer(&self, id: u64, out: Outcome) {
        self.in_flight.fetch_sub(1, Ordering::AcqRel);
        match out {
            Ok(InvokeResponseBody::Json(json)) => {
                self.push(Message::text(proto::reply_ok(id, &json)), Lane::Control)
            }
            Ok(InvokeResponseBody::Raw(bytes)) => {
                for frame in proto::reply_frames(id, &bytes) {
                    self.push(Message::binary(frame), Lane::Control);
                }
            }
            Err(Failure::Command(error)) => self.send(&ToClient::Reply {
                id,
                ok: None,
                error: Some(error),
                code: None,
            }),
            Err(Failure::Refused { code, message }) => self.send(&ToClient::Reply {
                id,
                ok: None,
                error: Some(serde_json::Value::String(message)),
                code: Some(code.to_string()),
            }),
        }
    }
}

/// The channels of a connection: what they are sent goes to its client, tagged with their id.
struct ConnChannels(Arc<Conn>);

impl Channels for ConnChannels {
    fn open(&self, id: u32) -> Box<dyn Fn(InvokeResponseBody) -> tauri::Result<()> + Send + Sync> {
        let conn = Arc::downgrade(&self.0);
        Box::new(move |body| {
            // A client gone: what its channels are sent goes nowhere (its terminals run on).
            let Some(conn) = conn.upgrade() else {
                return Ok(());
            };
            match body {
                // The events of the state (`subscribe`).
                InvokeResponseBody::Json(json) => {
                    conn.push(Message::text(proto::channel_json(id, &json)), Lane::State)
                }
                InvokeResponseBody::Raw(bytes) => conn.push(
                    Message::binary(proto::frame(Frame::Channel, u64::from(id), &bytes)),
                    Lane::Control,
                ),
            }
            Ok(())
        })
    }

    fn term(&self, id: u32) -> Box<dyn Sink> {
        Box::new(TermSink::new(&self.0, id, false))
    }
}

/// The channel of a terminal of a client: its output numbered (`Frame::Term`), its replays and marks, kept in the
/// session for the client to resume it after a cut.
struct TermSink {
    conn: Weak<Conn>,
    channel: u32,
    /// The replay of a resume under way: never dropped (it is what makes up for a drop).
    replaying: AtomicBool,
}

impl TermSink {
    fn new(conn: &Arc<Conn>, channel: u32, replaying: bool) -> Self {
        Self {
            conn: Arc::downgrade(conn),
            channel,
            replaying: AtomicBool::new(replaying),
        }
    }

    fn json(&self, message: &ToClient) {
        if let Some(conn) = self.conn.upgrade() {
            conn.send(message);
        }
    }
}

impl Sink for TermSink {
    fn send(&self, end: u64, data: &[u8]) {
        let Some(conn) = self.conn.upgrade() else {
            return;
        };
        let lane = if self.replaying.load(Ordering::Acquire) {
            Lane::Control
        } else {
            Lane::Term(self.channel)
        };
        conn.push(
            Message::binary(proto::term_frame(self.channel, end, data)),
            lane,
        );
    }

    fn replay(&self, data: &[u8]) {
        if let Some(conn) = self.conn.upgrade() {
            conn.push(
                Message::binary(proto::frame(Frame::Channel, u64::from(self.channel), data)),
                Lane::Control,
            );
        }
    }

    fn mark(&self, end: u64) {
        self.json(&ToClient::Mark {
            channel: self.channel,
            end,
        });
    }

    fn lost(&self, bytes: u64) {
        self.json(&ToClient::Lost {
            channel: self.channel,
            bytes,
        });
    }

    fn resumed(&self, ended: bool) {
        self.replaying.store(false, Ordering::Release);
        self.json(&ToClient::Resumed {
            channel: self.channel,
            ended,
            gone: false,
        });
    }

    fn bound(&self, replay: Arc<dyn Replay>) {
        if let Some(conn) = self.conn.upgrade() {
            conn.sessions.bind(&conn.session, self.channel, replay);
        }
    }
}

/// What the server counts, for the tests and the log.
#[derive(Debug, Default)]
pub struct Counts {
    /// The most bytes that waited in a connection's queue at once.
    pub peak_queued: AtomicUsize,
    /// The connections closed because they read too slowly.
    pub closed_slow: AtomicU64,
    /// The connections closed as soon as they were accepted (too many handshakes, or failed ones).
    pub turned_away: AtomicU64,
}

/// The remote API, served.
pub struct RemoteApi {
    services: Arc<Services>,
    devices: Devices,
    sessions: Arc<Sessions>,
    limits: Limits,
    addr: SocketAddr,
    next: AtomicU64,
    live: Mutex<HashMap<u64, Arc<Conn>>>,
    pending: AtomicUsize,
    failures: Mutex<VecDeque<Instant>>,
    counts: Counts,
    stopped: Notify,
}

/// Where the API listens: a port of 127.0.0.1 (0: a free one), or a socket bound already (the window's, which is told
/// its address before the core starts).
pub enum Listen {
    Port(u16),
    Bound(std::net::TcpListener),
}

impl RemoteApi {
    /// The API served on 127.0.0.1:`port` (0: a free port, `addr` says which), for the core of `services`; refused
    /// when the policies of the device forbid it. On a runtime of Tokio.
    pub async fn start(services: Arc<Services>, port: u16) -> Result<Arc<Self>> {
        Self::start_with(services, Listen::Port(port), Limits::default()).await
    }

    /// The same, where `listen` says, within `limits`.
    pub async fn start_with(
        services: Arc<Services>,
        listen: Listen,
        limits: Limits,
    ) -> Result<Arc<Self>> {
        let core = services.core().clone();
        core.locks.read().check_remote_api()?;
        #[cfg(debug_assertions)]
        let devices = match std::env::var(TEST_TOKEN_VAR) {
            Ok(token) if !token.is_empty() => Devices::of_test("test", token, core.secrets.clone()),
            _ => Devices::load(&core.data, core.secrets.clone()),
        };
        #[cfg(not(debug_assertions))]
        let devices = Devices::load(&core.data, core.secrets.clone());
        let listener = match listen {
            Listen::Port(port) => TcpListener::bind((Ipv4Addr::LOCALHOST, port))
                .await
                .with_context(|| format!("remote API: port {port}"))?,
            Listen::Bound(socket) => {
                socket.set_nonblocking(true)?;
                TcpListener::from_std(socket)?
            }
        };
        let addr = listener.local_addr()?;
        let api = Arc::new(Self {
            services,
            devices,
            sessions: Arc::new(Sessions::default()),
            limits,
            addr,
            next: AtomicU64::new(1),
            live: Mutex::new(HashMap::new()),
            pending: AtomicUsize::new(0),
            failures: Mutex::new(VecDeque::new()),
            counts: Counts::default(),
            stopped: Notify::new(),
        });
        log::info!("remote API: listening on {addr}");
        let me = api.clone();
        tokio::spawn(async move { me.accept(listener).await });
        Ok(api)
    }

    /// Where it listens: 127.0.0.1 and its port.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// The devices that may connect.
    pub fn devices(&self) -> &Devices {
        &self.devices
    }

    /// How many clients are connected.
    pub fn clients(&self) -> usize {
        self.live.lock().len()
    }

    /// What the server counted.
    pub fn counts(&self) -> &Counts {
        &self.counts
    }

    /// How many sessions it keeps.
    pub fn sessions(&self) -> usize {
        self.sessions.len()
    }

    /// The device `id` revoked: its token forgotten, its connections closed, its sessions forgotten.
    pub fn revoke(&self, id: &str) -> Result<bool> {
        let gone = self.devices.revoke(id)?;
        for conn in self.live.lock().values().filter(|c| c.device == id) {
            conn.kicked.notify_one();
        }
        self.sessions.forget_device(id);
        if gone {
            log::info!("remote API: device {id} revoked");
        }
        Ok(gone)
    }

    /// No more connections, and the ones open closed.
    pub fn stop(&self) {
        self.stopped.notify_one();
        for conn in self.live.lock().values() {
            conn.kicked.notify_one();
        }
    }

    /// Too many failed handshakes in the last minute: connections are turned away.
    fn failing(&self, now: Instant) -> bool {
        let mut failures = self.failures.lock();
        while failures
            .front()
            .is_some_and(|t| now.duration_since(*t) >= Duration::from_secs(60))
        {
            failures.pop_front();
        }
        failures.len() >= self.limits.failures_per_minute
    }

    fn failed(&self) {
        let now = Instant::now();
        let mut failures = self.failures.lock();
        failures.push_back(now);
        if failures.len() == self.limits.failures_per_minute {
            log::warn!(
                "remote API: {} failed handshakes in a minute: connections turned away for the rest of it",
                failures.len()
            );
        }
        // Never more kept than the bound needs.
        while failures.len() > self.limits.failures_per_minute {
            failures.pop_front();
        }
    }

    async fn accept(self: Arc<Self>, listener: TcpListener) {
        loop {
            tokio::select! {
                _ = self.stopped.notified() => break,
                accepted = listener.accept() => match accepted {
                    Ok((stream, _)) => {
                        let now = Instant::now();
                        if self.failing(now)
                            || self.pending.load(Ordering::Acquire) >= self.limits.pending
                        {
                            self.counts.turned_away.fetch_add(1, Ordering::Relaxed);
                            drop(stream);
                            continue;
                        }
                        self.pending.fetch_add(1, Ordering::AcqRel);
                        let me = self.clone();
                        tokio::spawn(async move { me.connection(stream).await });
                    }
                    Err(e) => {
                        log::warn!("remote API: accept: {e}");
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                },
            }
        }
        log::info!("remote API: stopped");
    }

    async fn connection(self: Arc<Self>, stream: TcpStream) {
        // The handshake under way counts until it is over, whatever its end.
        struct Pending<'a>(&'a AtomicUsize, bool);
        impl Pending<'_> {
            fn over(&mut self) {
                if !self.1 {
                    self.1 = true;
                    self.0.fetch_sub(1, Ordering::AcqRel);
                }
            }
        }
        impl Drop for Pending<'_> {
            fn drop(&mut self) {
                self.over();
            }
        }
        let mut pending = Pending(&self.pending, false);
        let config = WebSocketConfig::default()
            .max_message_size(Some(proto::MAX_MESSAGE))
            .max_frame_size(Some(proto::MAX_MESSAGE));
        // The callback's type is tungstenite's: its error, an HTTP answer, is large.
        #[allow(clippy::result_large_err)]
        let guard = |req: &Request, resp: Response| match guard(req) {
            Ok(()) => Ok(resp),
            Err((status, why)) => {
                log::warn!("remote API: refused a connection: {why}");
                let mut refused = ErrorResponse::new(Some(why));
                *refused.status_mut() = status;
                Err(refused)
            }
        };
        let opened = tokio::time::timeout(
            proto::HELLO_WAIT,
            tokio_tungstenite::accept_hdr_async_with_config(stream, guard, Some(config)),
        )
        .await;
        let ws = match opened {
            Ok(Ok(ws)) => ws,
            Ok(Err(e)) => {
                self.failed();
                log::debug!("remote API: handshake of the WebSocket: {e}");
                return;
            }
            Err(_) => {
                self.failed();
                log::debug!("remote API: the WebSocket did not open in time");
                return;
            }
        };
        let (mut sink, mut stream) = ws.split();
        // Before the handshake, the few messages of a refusal: a queue of their own, never past its bounds.
        let refuse = |code: &'static str,
                      error: String,
                      server_version: Option<String>,
                      update: Option<String>|
         -> Vec<Message> {
            log::warn!("remote API: refused a connection ({code}): {error}");
            let mut out = Vec::new();
            if let Ok(text) = serde_json::to_string(&ToClient::Refused {
                code: code.into(),
                error,
                server_version,
                update,
            }) {
                out.push(Message::text(text));
            }
            out.push(Message::Close(Some(CloseFrame {
                code: CloseCode::Policy,
                reason: code.into(),
            })));
            out
        };

        // The handshake: the first message, in time.
        let hello = tokio::time::timeout(proto::HELLO_WAIT, stream.next()).await;
        let (version, token, asked) = match hello {
            Ok(Some(Ok(Message::Text(text)))) => match serde_json::from_str::<ToServer>(&text) {
                Ok(ToServer::Hello {
                    version,
                    token,
                    session,
                }) => (version, token, session),
                _ => {
                    self.failed();
                    send_all(
                        &mut sink,
                        refuse(codes::HANDSHAKE, handshake_text(), None, None),
                    )
                    .await;
                    return;
                }
            },
            _ => {
                self.failed();
                send_all(
                    &mut sink,
                    refuse(codes::HANDSHAKE, handshake_text(), None, None),
                )
                .await;
                return;
            }
        };
        let core = self.services.core().clone();
        let server_version = core.host.version();
        let refused = if let Err(e) = core.locks.read().check_remote_api() {
            Err(refuse(codes::POLICY, e.to_string(), None, None))
        } else if let Some(device) = self.devices.find(&token) {
            if version != server_version {
                let update = proto::to_update(&server_version, &version);
                Err(refuse(
                    codes::VERSION,
                    version_text(&server_version, &version, update),
                    Some(server_version.clone()),
                    Some(update.to_string()),
                ))
            } else {
                Ok(device)
            }
        } else {
            self.failed();
            Err(refuse(
                codes::TOKEN,
                tr!(
                    "Jeton d’appareil absent, inconnu ou révoqué.",
                    "Device token missing, unknown or revoked."
                ),
                None,
                None,
            ))
        };
        let device = match refused {
            Ok(device) => device,
            Err(messages) => {
                send_all(&mut sink, messages).await;
                return;
            }
        };
        // Room for it: the device's connections, and all.
        let client = Client::Remote(self.next.fetch_add(1, Ordering::Relaxed));
        let n = match client {
            Client::Remote(n) => n,
            Client::Window => 0,
        };
        let room = {
            let mut live = self.live.lock();
            let of_device = live.values().filter(|c| c.device == device.id).count();
            if of_device >= self.limits.per_device || live.len() >= self.limits.total {
                Err((of_device, live.len()))
            } else {
                let (session, resumed) =
                    self.sessions
                        .open(&device.id, asked.as_deref(), Instant::now());
                let conn = Arc::new(Conn {
                    client: client.clone(),
                    device: device.id.clone(),
                    session,
                    sessions: self.sessions.clone(),
                    outbox: Outbox::new(self.limits.queue),
                    lanes: Lanes::new(Arc::new(Running::default())),
                    in_flight: AtomicUsize::new(0),
                    limit: self.limits.in_flight,
                    kicked: Notify::new(),
                });
                live.insert(n, conn.clone());
                Ok((conn, resumed))
            }
        };
        match room {
            Ok((conn, resumed)) => {
                pending.over();
                self.serve(conn, resumed, &device, sink, stream).await;
            }
            Err((of_device, all)) => {
                send_all(
                    &mut sink,
                    refuse(
                        codes::TOO_MANY,
                        tr!(
                            "Trop de connexions : {of_device} de cet appareil, {all} en tout.",
                            "Too many connections: {of_device} of this device, {all} in all."
                        ),
                        None,
                        None,
                    ),
                )
                .await
            }
        }
    }

    /// The connection `conn`, its handshake passed: served until it ends.
    async fn serve(
        self: &Arc<Self>,
        conn: Arc<Conn>,
        resumed: bool,
        device: &super::devices::Device,
        mut sink: WsSink,
        mut stream: WsStream,
    ) {
        let core = self.services.core().clone();
        let client = conn.client.clone();
        self.devices.seen(&device.id);
        log::info!(
            "remote API: device « {} » connected ({}{})",
            device.name,
            client.label(),
            if resumed { ", session resumed" } else { "" }
        );
        let writer_conn = conn.clone();
        let writer = tokio::spawn(async move {
            while let Some(message) = writer_conn.outbox.next().await {
                let closing = matches!(message, Message::Close(_));
                if sink.send(message).await.is_err() {
                    break;
                }
                writer_conn.outbox.sent();
                if closing {
                    break;
                }
            }
            let _ = tokio::time::timeout(Duration::from_secs(2), sink.close()).await;
        });
        conn.send(&ToClient::Welcome {
            version: core.host.version(),
            client: client.label(),
            device: device.name.clone(),
            session: conn.session.clone(),
            resumed,
            desk: super::list::COMMANDS
                .iter()
                .filter(|c| c.class == super::Class::Desk)
                .map(|c| c.name.to_string())
                .collect(),
        });
        // What the host publishes (a process without a window), pushed as it comes.
        let host_events = core.host.events().map(|mut events| {
            let conn = Arc::downgrade(&conn);
            tokio::spawn(async move {
                while let Some(event) = events.recv().await {
                    let Some(conn) = conn.upgrade() else { break };
                    match serde_json::to_string(&ToClient::Host {
                        event: serde_json::to_value(&event).unwrap_or_default(),
                    }) {
                        Ok(text) => conn.push(Message::text(text), Lane::State),
                        Err(e) => log::warn!("remote API: a host event not written: {e}"),
                    }
                }
            })
        });

        // The raw bodies received, waiting for their call.
        let mut bodies: HashMap<u64, Vec<u8>> = HashMap::new();
        let mut watch = tokio::time::interval(Duration::from_millis(500));
        let mut slow = None;
        loop {
            tokio::select! {
                _ = conn.kicked.notified() => break,
                _ = watch.tick() => {
                    if let Some(why) = conn.outbox.stuck(Instant::now()) {
                        slow = Some(why);
                        break;
                    }
                }
                message = stream.next() => match message {
                    None | Some(Ok(Message::Close(_))) => break,
                    Some(Err(e)) => {
                        log::warn!("remote API: {} closed: {e}", client.label());
                        break;
                    }
                    Some(Ok(Message::Text(text))) => match serde_json::from_str::<ToServer>(&text) {
                        Ok(ToServer::Call { id, cmd, args, body, headers }) => {
                            let body = match body {
                                None => None,
                                Some(len) => match raw_body(&mut bodies, id, len, &headers) {
                                    Ok(body) => Some(body),
                                    Err(why) => {
                                        conn.in_flight.fetch_add(1, Ordering::AcqRel);
                                        conn.answer(id, Err(Failure::refused(codes::TOO_LARGE, why)));
                                        continue;
                                    }
                                },
                            };
                            self.dispatch(&conn, id, &cmd, args, body)
                        }
                        Ok(ToServer::Resume { channels }) if channels.len() <= proto::MAX_RESUME => {
                            for r in channels {
                                match conn.sessions.find(&conn.session, &conn.device, r.channel) {
                                    Some(replay) => replay.resume(
                                        r.after,
                                        Box::new(TermSink::new(&conn, r.channel, true)),
                                    ),
                                    None => conn.send(&ToClient::Resumed {
                                        channel: r.channel,
                                        ended: false,
                                        gone: true,
                                    }),
                                }
                            }
                        }
                        _ => conn.send(&ToClient::Error {
                            code: codes::BAD_MESSAGE.into(),
                            error: "not a message of the remote API".into(),
                        }),
                    },
                    Some(Ok(Message::Binary(bytes))) => {
                        if let Err(why) = body_part(&mut bodies, &bytes, &self.limits) {
                            conn.send(&ToClient::Error {
                                code: codes::TOO_LARGE.into(),
                                error: why,
                            });
                        }
                    }
                    Some(Ok(_)) => {}
                },
            }
        }

        if let Some(task) = host_events {
            task.abort();
        }
        self.live.lock().retain(|_, c| !Arc::ptr_eq(c, &conn));
        self.sessions.leave(&conn.session, Instant::now());
        core.forget_client(&client);
        self.counts
            .peak_queued
            .fetch_max(conn.outbox.peak(), Ordering::Relaxed);
        if let Some(why) = slow {
            // It reads nothing: its queue is let go, the close said if it ever reads again, and the socket closed
            // whatever it does.
            self.counts.closed_slow.fetch_add(1, Ordering::Relaxed);
            log::warn!(
                "remote API: device « {} » closed ({}): {why} ({} bytes waiting)",
                device.name,
                client.label(),
                conn.outbox.waiting()
            );
            conn.outbox.finish(None);
            writer.abort();
        } else {
            conn.outbox.finish(Some(Message::Close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: "".into(),
            }))));
            log::info!(
                "remote API: device « {} » disconnected ({})",
                device.name,
                client.label()
            );
            let _ = tokio::time::timeout(Duration::from_secs(5), writer).await;
        }
        drop(conn);
    }

    /// The call `id` of the command `name`, run as the IPC runs it, on its lane; its answer sent back.
    fn dispatch(
        &self,
        conn: &Arc<Conn>,
        id: u64,
        name: &str,
        args: serde_json::Map<String, serde_json::Value>,
        body: Option<RawBody>,
    ) {
        // Every path below answers once, which counts it out.
        if conn.in_flight.fetch_add(1, Ordering::AcqRel) >= conn.limit {
            return conn.answer(
                id,
                Err(Failure::refused(
                    codes::BUSY,
                    tr!(
                        "Trop d’appels en attente de réponse sur cette connexion ({n}).",
                        "Too many calls waiting for their answer on this connection ({n}).",
                        n = conn.limit
                    ),
                )),
            );
        }
        let Some(command) = super::find(name) else {
            return conn.answer(
                id,
                Err(Failure::refused(
                    codes::UNKNOWN_COMMAND,
                    format!("Command {} not found", shown(name)),
                )),
            );
        };
        if !command.class.served() {
            log::info!(
                "remote API: {} asked {}, refused ({})",
                conn.client.label(),
                command.name,
                match command.class {
                    super::Class::Desk => "the desktop's",
                    _ => "not served yet",
                }
            );
            return conn.answer(id, Err(refusal(command)));
        }
        let call = Call {
            command: command.name,
            args,
            services: self.services.clone(),
            channels: Arc::new(ConnChannels(conn.clone())),
            body,
        };
        let reply = (command.call)(call);
        let (done, client) = (conn.clone(), conn.client.clone());
        let name = command.name;
        let panicked = move || {
            Err(Failure::refused(
                codes::PANICKED,
                format!("{name}: the command panicked"),
            ))
        };
        match reply {
            Reply::Refused(why) => conn.answer(
                id,
                Err(Failure::refused(
                    codes::UNAVAILABLE,
                    format!("{name} is not served at a distance ({why})"),
                )),
            ),
            Reply::Blocking(job) => conn.lanes.submit(
                lane_of(name),
                Box::new(move || {
                    let out = std::panic::catch_unwind(AssertUnwindSafe(|| {
                        super::as_client(client, job)
                    }))
                    .unwrap_or_else(|_| panicked());
                    done.answer(id, out);
                }),
            ),
            Reply::Future(future) => conn.lanes.submit(
                lane_of(name),
                Box::new(move || {
                    // Started in its turn on its lane, as the IPC starts it; it runs on the runtime.
                    tokio::runtime::Handle::current().spawn(async move {
                        let out = AssertUnwindSafe(super::scoped(client, future))
                            .catch_unwind()
                            .await
                            .unwrap_or_else(|_| panicked());
                        done.answer(id, out);
                    });
                }),
            ),
        }
    }
}

type WsSink =
    futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<TcpStream>, Message>;
type WsStream = futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<TcpStream>>;

/// The few messages of a refusal, before the connection closes: 2 s each at most.
async fn send_all(sink: &mut WsSink, messages: Vec<Message>) {
    for m in messages {
        if tokio::time::timeout(Duration::from_secs(2), sink.send(m))
            .await
            .map_or(true, |r| r.is_err())
        {
            break;
        }
    }
}

/// A part of a raw body (`Frame::Body`): added to the body of its call, within the bounds.
fn body_part(
    bodies: &mut HashMap<u64, Vec<u8>>,
    bytes: &[u8],
    limits: &Limits,
) -> std::result::Result<(), String> {
    let Some((Frame::Body, id, part)) = proto::read_frame(bytes) else {
        return Err("a client's binary message is a part of a raw body (Frame::Body)".into());
    };
    let waiting: usize = bodies.values().map(Vec::len).sum();
    if !bodies.contains_key(&id) && bodies.len() >= limits.bodies {
        return Err(format!("more than {} raw bodies waiting", limits.bodies));
    }
    let body = bodies.entry(id).or_default();
    if body.len() + part.len() > proto::MAX_BODY || waiting + part.len() > limits.body_bytes {
        bodies.remove(&id);
        return Err(format!(
            "a raw body past {} MB, or too many waiting",
            proto::MAX_BODY >> 20
        ));
    }
    body.extend_from_slice(part);
    Ok(())
}

/// The raw body of the call `id`, of `len` bytes, received before it, with its headers.
fn raw_body(
    bodies: &mut HashMap<u64, Vec<u8>>,
    id: u64,
    len: u64,
    headers: &serde_json::Map<String, serde_json::Value>,
) -> std::result::Result<RawBody, String> {
    let bytes = bodies.remove(&id).unwrap_or_default();
    if bytes.len() as u64 != len {
        return Err(format!(
            "the raw body is {} bytes, its call says {len}",
            bytes.len()
        ));
    }
    if headers.len() > proto::MAX_HEADERS {
        return Err(format!("more than {} headers", proto::MAX_HEADERS));
    }
    let mut map = tauri::http::HeaderMap::new();
    for (name, value) in headers {
        let value = value.as_str().ok_or("a header that is not a text")?;
        if name.len() > proto::MAX_HEADER_NAME || value.len() > proto::MAX_HEADER_VALUE {
            return Err("a header too long".into());
        }
        let name = tauri::http::HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| "a header's name that is not one".to_string())?;
        let value = tauri::http::HeaderValue::from_str(value)
            .map_err(|_| "a header's value that is not one".to_string())?;
        map.insert(name, value);
    }
    Ok(RawBody {
        bytes,
        headers: map,
    })
}

/// Why the command `command`, not served, is refused.
fn refusal(command: &super::Command) -> Failure {
    let name = command.name;
    match command.class {
        super::Class::Desk => Failure::refused(
            codes::DESK_ONLY,
            tr!(
                "« {name} » est une commande du poste : elle ne se sert pas à distance.",
                "“{name}” is a command of the desktop: it is not served at a distance."
            ),
        ),
        _ => Failure::refused(
            codes::NOT_SERVED_YET,
            tr!(
                "« {name} » change de machine en mode serveur : pas encore servie à distance.",
                "“{name}” changes machines in server mode: not served at a distance yet."
            ),
        ),
    }
}

fn handshake_text() -> String {
    tr!(
        "Le premier message doit être la poignée de main (version et jeton), dans les {} s.",
        "The first message must be the handshake (version and token), within {} s.",
        proto::HELLO_WAIT.as_secs()
    )
}

/// « Le serveur est en 2.3.0, ce client en 2.2.1 : mettre à jour le client. »
fn version_text(server: &str, client: &str, update: &str) -> String {
    let (server, client) = (shown(server), shown(client));
    if update == "client" {
        tr!(
            "Le serveur est en {server}, ce client en {client} : mettre à jour le client.",
            "The server runs {server}, this client {client}: update the client."
        )
    } else {
        tr!(
            "Le serveur est en {server}, ce client en {client} : mettre à jour le serveur.",
            "The server runs {server}, this client {client}: update the server."
        )
    }
}

/// The request that opens the WebSocket, before it opens: its path, a `Host` of the loopback, no `Origin` but
/// Escouade's own window's. Else its HTTP status and why (shown in the log: no secret there).
fn guard(req: &Request) -> std::result::Result<(), (tauri::http::StatusCode, String)> {
    use tauri::http::StatusCode;
    if req.uri().path() != proto::PATH {
        return Err((
            StatusCode::NOT_FOUND,
            format!("path {}", shown(req.uri().path())),
        ));
    }
    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    if !loopback_host(host) {
        return Err((
            StatusCode::FORBIDDEN,
            format!("a Host that is not the loopback ({})", shown(host)),
        ));
    }
    if let Some(origin) = req.headers().get("origin") {
        let origin = origin.to_str().unwrap_or("");
        if !proto::ORIGINS.contains(&origin) {
            return Err((
                StatusCode::FORBIDDEN,
                format!("the Origin of a web page ({})", shown(origin)),
            ));
        }
    }
    Ok(())
}

/// `127.0.0.1`, `localhost` or `[::1]`, with any port: through a tunnel, the client's port is its own.
fn loopback_host(host: &str) -> bool {
    let name = if let Some(rest) = host.strip_prefix('[') {
        rest.split(']')
            .next()
            .map(|h| format!("[{h}]"))
            .unwrap_or_default()
    } else {
        host.split(':').next().unwrap_or("").to_string()
    };
    matches!(
        name.to_ascii_lowercase().as_str(),
        "127.0.0.1" | "localhost" | "[::1]"
    )
}

/// A text a client gave, as the log or an answer may show it: 80 characters at most, printable ones.
fn shown(value: &str) -> String {
    value.chars().filter(|c| !c.is_control()).take(80).collect()
}
