//! The WebSocket of the remote API (`docs/REMOTE.md`, « La fondation commune », points 2 to 6; « La fondation 2b »).
//!
//! - **Off unless asked**: a build of development of the desktop app serves it when `ESCOUADE_REMOTE_API` gives a
//!   port (never a release: a variable opens nothing there), a process without a window when it asks
//!   (`Headless::serve_remote`); never while the policies of the device forbid it (`servers.remoteApi`), checked at
//!   the start and at each connection.
//! - **On the loopback alone**: it listens on 127.0.0.1, and is reached through a tunnel (SSH, WireGuard: the server
//!   mode's). No TLS here.
//! - **Before the WebSocket opens**: the path, a `Host` of the loopback (DNS rebinding), and no `Origin` but the one of
//!   Escouade's own window (a web page of the machine is refused). Bounded ([`Limits`]): the handshakes under way (all
//!   taken, the oldest is let go for the newest: connections that say nothing never hold the door, ESC-654), and the
//!   failed ones of a minute, counted by source (its address, the tunnel's peer, the client a declared proxy forwards):
//!   past them, the pairing codes of that source are refused unread for the rest of the minute; past the wrong codes of
//!   a minute from elsewhere in all, those from elsewhere, never those of this machine; a right token always passes
//!   (SR1-04, ESC-654, ESC-631). Through Tailscale, no `Origin` of a `localhost`: the phone app's own alone (SR1-09).
//!   A `Host` or an `Origin` given twice, or an absolute URI whose authority is not the `Host`, is refused (400): a
//!   proxy and the server never read two names (ESC-688).
//! - **The handshake**: the first message gives the device's token (compared in constant time, `devices`), the
//!   client's version, which must be the server's (« Mettre à jour le serveur »), and the session it had before a cut
//!   (`sessions`). Bounded: the connections of a device, and in all.
//! - **Then** the calls, each on the lane of its command (`windowthread::lane_of`, as the IPC: a terminal's
//!   keystrokes keep their order), as the client of this connection (`remoteapi::Client`), so many at once at most;
//!   their raw bodies before them in binary frames; a call under way or waiting on its lane let go when its connection
//!   ends (closed, its device revoked: ESC-654); the channels and the events pushed on the same connection,
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
use tokio::net::TcpListener;
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

/// How often the devices' file is looked at, for a device another process revoked (`escouade-server devices`).
const DEVICES_EVERY: Duration = Duration::from_secs(2);

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
    /// The connections in all at once (and `reserve` more, for a device's first).
    pub total: usize,
    /// The connections whose handshake is under way at once: past them, the oldest is let go.
    pub pending: usize,
    /// The failed handshakes of one source (its address, the tunnel's peer) in a minute, past which its pairing codes
    /// are refused unread for the rest of it.
    pub failures_per_minute: usize,
    /// The calls of one connection waiting for their answer at once.
    pub in_flight: usize,
    /// The raw bodies of one connection waiting for their call at once, and their bytes.
    pub bodies: usize,
    pub body_bytes: usize,
    /// The wrong pairing codes of a minute, from all sources of a reach (this machine, or elsewhere), past which any code
    /// of that reach is refused unread for the rest of it.
    pub pairing_per_minute: usize,
    /// The places beyond `total` kept for the first connection of a device: one that is not connected always gets in,
    /// to revoke the device that took the others.
    pub reserve: usize,
}

/// The sources whose failures are kept at most: past them, those whose last failure is the oldest are forgotten.
const MAX_SOURCES: usize = 4096;

impl Default for Limits {
    fn default() -> Self {
        Self {
            queue: Bounds::default(),
            per_device: 4,
            total: 16,
            pending: 8,
            failures_per_minute: 20,
            pairing_per_minute: 120,
            reserve: 4,
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
    /// What its device may call (ESC-570): checked in `dispatch` before the command's class.
    scope: super::Scope,
    session: String,
    sessions: Arc<Sessions>,
    outbox: Outbox,
    lanes: Arc<Lanes>,
    in_flight: AtomicUsize,
    limit: usize,
    /// Told to end: its device revoked, the server stopped.
    kicked: Notify,
    /// True once it ended: its calls under way are let go, those waiting on a lane never run (ESC-654).
    ended: tokio::sync::watch::Sender<bool>,
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
    /// The handshakes let go for newer ones, all places of a handshake taken (ESC-654).
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
    /// The number of the next handshake (apart from the clients' numbers, which the clients are told).
    handshakes: AtomicU64,
    live: Mutex<HashMap<u64, Arc<Conn>>>,
    /// The handshakes under way, oldest first: each its number and what lets it go.
    pending: Mutex<VecDeque<(u64, Arc<Notify>)>>,
    /// The failed handshakes of the last minute, by source.
    failures: Mutex<HashMap<String, VecDeque<Instant>>>,
    /// The wrong pairing codes of the last minute, from all sources elsewhere than this machine.
    wrong_codes: Mutex<VecDeque<Instant>>,
    /// The wrong pairing codes of the last minute, from this machine (the loopback).
    wrong_codes_here: Mutex<VecDeque<Instant>>,
    /// The proxies whose `X-Forwarded-For` is read (`escouade-server --trust-proxy`, SR1-04).
    proxies: Mutex<Arc<Vec<std::net::IpAddr>>>,
    counts: Counts,
    stopped: Notify,
    /// The names taken in a `Host` besides the loopback's (`escouade-server --allow-host`: a TLS proxy's).
    hosts: Mutex<Arc<Vec<String>>>,
}

/// Where the API listens: a port of 127.0.0.1 (0: a free one), or a socket bound already (the window's, which is told
/// its address before the core starts).
pub enum Listen {
    Port(u16),
    Bound(std::net::TcpListener),
    /// No socket of its own: it serves the connections it is handed (`serve_stream`), the phone's tunnel's
    /// (ESC-567: on the desktop, the API is served in the tunnel alone).
    Nothing,
}

/// Where a connection comes from (ESC-567), which says the `Host` and the `Origin` it may give.
#[derive(Clone)]
pub enum Via {
    /// A port of the loopback: the window, another machine's client through SSH.
    Loopback,
    /// The phone's WireGuard tunnel, from one of its peers.
    Tunnel(Arc<dyn TunnelPeer>),
    /// The machine's Tailscale address, from a machine of the tailnet: the names it is reached by.
    Tailscale(Arc<Vec<String>>),
}

/// A peer of the phone's tunnel, as the API sees it (`phone`).
pub trait TunnelPeer: Send + Sync {
    /// The names a `Host` may give through it: the tunnel's address.
    fn hosts(&self) -> Vec<String>;
    /// The device it belongs to, once paired: the only token taken through it.
    fn device(&self) -> Option<String>;
    /// A device paired through it: it belongs to it from now on.
    fn paired(&self, device: &str);
    /// As the log says it.
    fn label(&self) -> String;
}

impl Via {
    fn hosts(&self) -> Vec<String> {
        match self {
            Via::Loopback => Vec::new(),
            Via::Tunnel(peer) => peer.hosts(),
            Via::Tailscale(names) => names.to_vec(),
        }
    }
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
            Listen::Port(port) => Some(
                TcpListener::bind((Ipv4Addr::LOCALHOST, port))
                    .await
                    .with_context(|| format!("remote API: port {port}"))?,
            ),
            Listen::Bound(socket) => {
                socket.set_nonblocking(true)?;
                Some(TcpListener::from_std(socket)?)
            }
            Listen::Nothing => None,
        };
        let addr = match &listener {
            Some(l) => l.local_addr()?,
            None => SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        };
        let api = Arc::new(Self {
            services,
            devices,
            sessions: Arc::new(Sessions::default()),
            limits,
            addr,
            next: AtomicU64::new(1),
            handshakes: AtomicU64::new(1),
            live: Mutex::new(HashMap::new()),
            pending: Mutex::new(VecDeque::new()),
            failures: Mutex::new(HashMap::new()),
            wrong_codes: Mutex::new(VecDeque::new()),
            wrong_codes_here: Mutex::new(VecDeque::new()),
            proxies: Mutex::new(Arc::new(Vec::new())),
            counts: Counts::default(),
            stopped: Notify::new(),
            hosts: Mutex::new(Arc::new(Vec::new())),
        });
        match &listener {
            Some(_) => log::info!("remote API: listening on {addr}"),
            None => log::info!("remote API: served in the phone's tunnel alone"),
        }
        let me = api.clone();
        tokio::spawn(async move { me.accept(listener).await });
        Ok(api)
    }

    /// The names it takes in a `Host` besides the loopback's, from now on (lower case): the name a TLS proxy in front of
    /// it serves it under (`escouade-server --allow-host`). Any other name is still refused (DNS rebinding).
    pub fn allow_hosts(&self, hosts: Vec<String>) {
        *self.hosts.lock() = Arc::new(hosts.into_iter().map(|h| h.to_ascii_lowercase()).collect());
    }

    /// The proxies in front of it, from now on (`escouade-server --trust-proxy`, SR1-04): a connection of one of them
    /// that says whom it forwards (`X-Forwarded-For`, its last address) counts its failures as that client's, elsewhere
    /// than this machine; one of them that says none, as its own address's. Of any other address, the header is never
    /// read: a client cannot choose where its failures go.
    pub fn trust_proxies(&self, proxies: Vec<std::net::IpAddr>) {
        *self.proxies.lock() = Arc::new(proxies);
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
    /// The devices paired from it go with it (SR1-02).
    pub fn revoke(&self, id: &str) -> Result<bool> {
        let gone = self.devices.revoke_with(id, None)?;
        self.cut(id, &gone);
        Ok(!gone.is_empty())
    }

    /// The connections of the devices `gone` (revoked with `id`) closed, their sessions forgotten.
    fn cut(&self, id: &str, gone: &[super::devices::Device]) {
        let ids: Vec<&str> = std::iter::once(id)
            .chain(gone.iter().map(|d| d.id.as_str()))
            .collect();
        for conn in self
            .live
            .lock()
            .values()
            .filter(|c| ids.contains(&c.device.as_str()))
        {
            conn.kicked.notify_one();
        }
        for id in &ids {
            self.sessions.forget_device(id);
        }
        for d in gone {
            log::info!("remote API: device {} revoked", d.id);
        }
    }

    /// A new device paired by the code `code` (`pairing`), named `name`, from `source`: it and its token, or the
    /// refusal's code and text. A refused code counts as a failed handshake of its source; past the failures of its
    /// source, or the wrong codes of all those of its reach (this machine, or elsewhere) in the minute, the code is
    /// refused unread (`Wait`), the code waiting on (ESC-654, SR1-04). The log says neither the code nor the token.
    fn pair(
        &self,
        code: &str,
        name: &str,
        key: Option<&str>,
        tunnel: bool,
        source: &Source,
    ) -> std::result::Result<Paired, (&'static str, String, Option<&'static str>)> {
        let core = self.services.core().clone();
        if let Err(e) = core.locks.read().check_remote_api() {
            return Err((codes::POLICY, e.to_string(), None));
        }
        let now = Instant::now();
        if self.failing(source, now) || self.codes_failing(source.reach, now) {
            log::warn!("remote API: pairing refused from {source}: too many wrong tries in the minute, the code not read");
            let refusal = super::pairing::Refusal::Wait;
            return Err((codes::PAIRING, refusal.text(), Some(refusal.code())));
        }
        // A phone's key, read before the code is: a key that is none leaves the code waiting (ESC-570).
        let derived = match key {
            None => None,
            Some(key) => match super::pairing::derive(key) {
                Some(d) => Some(d),
                None => {
                    self.failed(source);
                    log::warn!(
                        "remote API: pairing refused from {source}: the phone's key is not one"
                    );
                    return Err((
                        codes::PAIRING,
                        tr!(
                            "La clé de ce téléphone est illisible : mets l’app à jour.",
                            "This phone's key is unreadable: update the app."
                        ),
                        Some("key"),
                    ));
                }
            },
        };
        let data = core.data.clone();
        let redeemed = match super::pairing::redeem_with(&data, code, crate::model::now_ms()) {
            Ok(r) => r,
            Err(refusal) => {
                self.failed(source);
                if refusal == super::pairing::Refusal::Wrong {
                    self.wrong_code(source.reach);
                }
                log::warn!(
                    "remote API: pairing refused from {source}: {}",
                    refusal.reason()
                );
                return Err((codes::PAIRING, refusal.text(), Some(refusal.code())));
            }
        };
        let paired_by = redeemed.by;
        // A code asked by a device revoked since: no device is born of it (SR1-02).
        self.devices.refresh();
        if let Some(by) = paired_by.as_deref().filter(|by| !self.devices.holds(by)) {
            self.failed(source);
            log::warn!("remote API: pairing refused: the code's device ({by}) was revoked");
            let refusal = super::pairing::Refusal::NoCode;
            return Err((codes::PAIRING, refusal.text(), Some(refusal.code())));
        }
        // The name, as the user typed it: printable, 64 characters at most.
        let name: String = name.chars().filter(|c| !c.is_control()).take(64).collect();
        let name = if name.trim().is_empty() {
            tr!("Appareil", "Device")
        } else {
            name
        };
        // A phone's scope (ESC-570): its QR code's, and always through its tunnel or with a key of its own; never more
        // than its code says.
        let scope = if tunnel || derived.is_some() {
            super::Scope::Phone
        } else {
            redeemed.scope
        };
        let given = derived.as_ref().map(|d| d.token.clone());
        match self
            .devices
            .add_as(&name, paired_by.as_deref(), scope, given)
        {
            Ok((device, token)) => {
                log::info!(
                    "remote API: device « {} » paired ({})",
                    device.name,
                    device.scope.as_str()
                );
                match derived {
                    // The token never sent: the phone derives it. Its pairing key kept, for its notifications.
                    Some(d) => {
                        if let Err(e) = self.devices.keep_key(&device.id, &d.key) {
                            log::warn!("remote API: a phone's pairing key not kept: {e:#}");
                        }
                        Ok((device, String::new(), Some(d.public)))
                    }
                    None => Ok((device, token, None)),
                }
            }
            Err(e) => {
                log::error!("remote API: a paired device not kept: {e:#}");
                Err((codes::PAIRING, e.to_string(), None))
            }
        }
    }

    /// The device whose token `token` is, forgetting this server (ESC-537): revoked, its own name said as who asked;
    /// its connections closed, its sessions forgotten, and the devices paired from it with it. None when the token is
    /// no device's, or not the device of the tunnel's peer it comes through (as the handshake, SR1-10).
    fn unpair(&self, token: &str, via: &Via) -> Option<super::devices::Device> {
        self.devices.refresh();
        let device = self.devices.find(token)?;
        if let Via::Tunnel(peer) = via {
            if let Some(bound) = peer.device().filter(|d| *d != device.id) {
                log::warn!(
                    "remote API: « {} » forgetting this server through the tunnel's peer of another device ({bound}): refused",
                    device.name
                );
                return None;
            }
        }
        match self.devices.revoke_with(&device.id, Some(&device.name)) {
            Ok(gone) => {
                self.cut(&device.id, &gone);
                log::info!(
                    "remote API: device « {} » forgot this server: revoked",
                    device.name
                );
                Some(device)
            }
            Err(e) => {
                log::error!("remote API: a device that forgot this server not revoked: {e:#}");
                None
            }
        }
    }

    /// The devices read again when another process changed them (`escouade-server devices revoke`): the connections
    /// of a device revoked there are closed, its sessions forgotten.
    fn sync_devices(&self) {
        // Every live connection looked at, whoever read the file again first (a handshake or a pairing reads it too:
        // a change it read was never told to the connections of a revoked device, ESC-570).
        self.devices.refresh();
        let gone: Vec<String> = self
            .live
            .lock()
            .values()
            .filter(|c| !self.devices.holds(&c.device))
            .map(|c| {
                c.kicked.notify_one();
                c.device.clone()
            })
            .collect();
        for id in gone {
            self.sessions.forget_device(&id);
            log::info!("remote API: device {id} revoked");
        }
    }

    /// No more connections, and the ones open closed.
    pub fn stop(&self) {
        self.stopped.notify_one();
        for conn in self.live.lock().values() {
            conn.kicked.notify_one();
        }
    }

    /// Too many failed handshakes of `source` in the last minute: its pairing codes are refused unread.
    fn failing(&self, source: &Source, now: Instant) -> bool {
        let mut failures = self.failures.lock();
        let Some(times) = failures.get_mut(&source.key) else {
            return false;
        };
        within_a_minute(times, now);
        if times.is_empty() {
            failures.remove(&source.key);
            return false;
        }
        times.len() >= self.limits.failures_per_minute
    }

    /// A failed handshake of `source`.
    fn failed(&self, source: &Source) {
        let now = Instant::now();
        let mut failures = self.failures.lock();
        if !failures.contains_key(&source.key) && failures.len() >= MAX_SOURCES {
            // Too many sources kept: those of no failure in the minute go, then the one whose last failure is oldest.
            failures.retain(|_, times| {
                within_a_minute(times, now);
                !times.is_empty()
            });
            if failures.len() >= MAX_SOURCES {
                if let Some(oldest) = failures
                    .iter()
                    .min_by_key(|(_, times)| times.back().copied())
                    .map(|(s, _)| s.clone())
                {
                    failures.remove(&oldest);
                }
            }
        }
        let times = failures.entry(source.key.clone()).or_default();
        within_a_minute(times, now);
        times.push_back(now);
        if times.len() == self.limits.failures_per_minute {
            log::warn!(
                "remote API: {} failed handshakes from {source} in a minute: its pairing codes refused unread for the rest of it",
                times.len()
            );
        }
        // Never more kept than the bound needs.
        while times.len() > self.limits.failures_per_minute {
            times.pop_front();
        }
    }

    /// The wrong pairing codes of the last minute from all sources of `reach`.
    fn wrong_codes(&self, reach: Reach) -> &Mutex<VecDeque<Instant>> {
        match reach {
            Reach::Here => &self.wrong_codes_here,
            Reach::Elsewhere => &self.wrong_codes,
        }
    }

    /// Too many wrong pairing codes of all sources of `reach` in the last minute: any code of that reach is refused
    /// unread. Those from elsewhere never refuse this machine's, nor the other way (SR1-04).
    fn codes_failing(&self, reach: Reach, now: Instant) -> bool {
        let mut wrong = self.wrong_codes(reach).lock();
        within_a_minute(&mut wrong, now);
        wrong.len() >= self.limits.pairing_per_minute
    }

    fn wrong_code(&self, reach: Reach) {
        let now = Instant::now();
        let mut wrong = self.wrong_codes(reach).lock();
        within_a_minute(&mut wrong, now);
        wrong.push_back(now);
        if wrong.len() == self.limits.pairing_per_minute {
            let n = wrong.len();
            match reach {
                Reach::Here => log::warn!(
                    "remote API: {n} wrong pairing codes from this machine in a minute: its codes refused unread for the rest of it"
                ),
                Reach::Elsewhere => {
                    log::warn!(
                        "remote API: {n} wrong pairing codes from elsewhere than this machine in a minute: their codes refused unread for the rest of it, the code waiting on"
                    );
                    // Said to the window too: never a code burnt, nor a pairing held up, behind the user's back.
                    self.services.core().hub.emit(crate::model::UiEvent::Toast {
                        text: tr!(
                            "{n} codes d’appairage faux en une minute venus du réseau : les codes venus d’ailleurs que cette machine sont refusés jusqu’à la fin de la minute. Le code qui attend vaut toujours, et se donne depuis cette machine ou par SSH.",
                            "{n} wrong pairing codes in a minute from the network: codes from elsewhere than this machine are refused until the minute ends. The code that waits still holds, and can be given from this machine or through SSH."
                        ),
                    });
                }
            }
        }
        while wrong.len() > self.limits.pairing_per_minute {
            wrong.pop_front();
        }
    }

    /// A connection handed to it, from `via` (the phone's tunnel, Tailscale) and the address `from` (the other end's,
    /// when it has one): served as one of its own socket's, within the same bounds. Always taken: all places of a
    /// handshake taken, the oldest handshake is let go for it.
    pub fn serve_stream_from(
        self: &Arc<Self>,
        stream: Box<dyn Io>,
        via: Via,
        from: Option<std::net::IpAddr>,
    ) -> bool {
        let source = Source::of(&via, from);
        let id = self.handshakes.fetch_add(1, Ordering::Relaxed);
        let cancel = Arc::new(Notify::new());
        {
            let mut pending = self.pending.lock();
            while pending.len() >= self.limits.pending.max(1) {
                if let Some((_, oldest)) = pending.pop_front() {
                    oldest.notify_one();
                    self.counts.turned_away.fetch_add(1, Ordering::Relaxed);
                }
            }
            pending.push_back((id, cancel.clone()));
        }
        let me = self.clone();
        tokio::spawn(async move {
            // The handshake under way counts until it is over, whatever its end.
            let _over = HandshakeOver(me.clone(), id);
            me.connection(stream, via, source, from, id, cancel).await
        });
        true
    }

    /// As `serve_stream_from`, with no address of the other end.
    pub fn serve_stream(self: &Arc<Self>, stream: Box<dyn Io>, via: Via) -> bool {
        self.serve_stream_from(stream, via, None)
    }

    /// The handshake `id` over: its place is free.
    fn handshake_over(&self, id: u64) {
        self.pending.lock().retain(|(n, _)| *n != id);
    }

    async fn accept(self: Arc<Self>, listener: Option<TcpListener>) {
        let mut devices = tokio::time::interval(DEVICES_EVERY);
        loop {
            let accepting = async {
                match &listener {
                    Some(l) => l.accept().await,
                    None => std::future::pending().await,
                }
            };
            tokio::select! {
                _ = self.stopped.notified() => break,
                _ = devices.tick() => self.sync_devices(),
                accepted = accepting => match accepted {
                    Ok((stream, from)) => {
                        let _ = self.serve_stream_from(Box::new(stream), Via::Loopback, Some(from.ip()));
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

    /// The connection `id` from `source` (the address `from`): its handshake, within [`proto::HELLO_WAIT`] in all (the
    /// WebSocket opened and its first message) and until `cancel` lets it go for a newer one; then served.
    async fn connection(
        self: Arc<Self>,
        stream: Box<dyn Io>,
        via: Via,
        mut source: Source,
        from: Option<std::net::IpAddr>,
        id: u64,
        cancel: Arc<Notify>,
    ) {
        let deadline = tokio::time::Instant::now() + proto::HELLO_WAIT;
        let config = WebSocketConfig::default()
            .max_message_size(Some(proto::MAX_MESSAGE))
            .max_frame_size(Some(proto::MAX_MESSAGE));
        // The callback's type is tungstenite's: its error, an HTTP answer, is large.
        let hosts = match &via {
            Via::Loopback => self.hosts.lock().clone(),
            other => Arc::new(other.hosts()),
        };
        let way = Way::of(&via);
        let through = way != Way::Loopback;
        // A connection of a proxy declared (`--trust-proxy`) that says whom it forwards: that client is the source.
        let proxied = matches!(via, Via::Loopback)
            && from.is_some_and(|ip| self.proxies.lock().contains(&ip));
        let forwarded: Mutex<Option<std::net::IpAddr>> = Mutex::new(None);
        // A phone's token, written by its native proxy in the request (ESC-570, SR1-15): the WebView's JavaScript never
        // holds it, and says an empty one in its handshake.
        let header_token: Mutex<Option<String>> = Mutex::new(None);
        // The callback's type is tungstenite's: its error, an HTTP answer, is large.
        #[allow(clippy::result_large_err)]
        let guard = |req: &Request, resp: Response| match guard(req, &hosts, way) {
            Ok(()) => {
                if proxied {
                    *forwarded.lock() = forwarded_for(req);
                }
                *header_token.lock() = device_header(req);
                Ok(resp)
            }
            Err((status, why)) => {
                log::warn!("remote API: refused a connection: {why}");
                let mut refused = ErrorResponse::new(Some(why));
                *refused.status_mut() = status;
                Err(refused)
            }
        };
        let opened = tokio::select! {
            opened = tokio::time::timeout_at(
                deadline,
                tokio_tungstenite::accept_hdr_async_with_config(stream, guard, Some(config)),
            ) => opened,
            _ = cancel.notified() => {
                log::debug!("remote API: a handshake from {source} let go for a newer one");
                return;
            }
        };
        let ws = match opened {
            Ok(Ok(ws)) => ws,
            Ok(Err(e)) => {
                self.failed(&source);
                if through {
                    // Said in the log of a server: what the trial in 4G looks for (a page opened in the tunnel).
                    log::info!("remote API: a connection through the phone's tunnel or Tailscale that is no WebSocket ({e})");
                } else {
                    log::debug!("remote API: handshake of the WebSocket: {e}");
                }
                return;
            }
            Err(_) => {
                self.failed(&source);
                log::debug!("remote API: the WebSocket did not open in time");
                return;
            }
        };
        if let Some(client) = forwarded.into_inner() {
            source = Source::forwarded(client);
        }
        let header_token = header_token.into_inner().unwrap_or_default();
        // The token of the handshake, else the one the phone's proxy wrote in the request.
        let token_of = |given: String| -> String {
            if given.is_empty() {
                header_token.clone()
            } else {
                given
            }
        };
        let (mut sink, mut stream) = ws.split();
        // Before the handshake, the few messages of a refusal: a queue of their own, never past its bounds.
        let refuse_with = |code: &'static str,
                           error: String,
                           server_version: Option<String>,
                           update: Option<String>,
                           reason: Option<&'static str>,
                           revoked: Option<proto::Revoked>,
                           server_update: Option<proto::ServerUpdate>|
         -> Vec<Message> {
            log::warn!("remote API: refused a connection ({code}): {error}");
            let mut out = Vec::new();
            if let Ok(text) = serde_json::to_string(&ToClient::Refused {
                code: code.into(),
                error,
                server_version,
                update,
                reason: reason.map(str::to_string),
                revoked,
                server_update,
            }) {
                out.push(Message::text(text));
            }
            out.push(Message::Close(Some(CloseFrame {
                code: CloseCode::Policy,
                reason: code.into(),
            })));
            out
        };
        let refuse = |code: &'static str,
                      error: String,
                      server_version: Option<String>,
                      update: Option<String>|
         -> Vec<Message> {
            refuse_with(code, error, server_version, update, None, None, None)
        };

        // The handshake: the first message, in time.
        let hello = tokio::select! {
            hello = tokio::time::timeout_at(deadline, stream.next()) => hello,
            _ = cancel.notified() => {
                log::debug!("remote API: a handshake from {source} let go for a newer one");
                return;
            }
        };
        let (version, token, asked) = match hello {
            Ok(Some(Ok(Message::Text(text)))) => match serde_json::from_str::<ToServer>(&text) {
                Ok(ToServer::Hello {
                    version,
                    token,
                    session,
                }) => (version, token_of(token), session),
                // A new device's code, in place of the handshake (ESC-514): its token, or why not; closed either way.
                Ok(ToServer::Pair { code, name, key }) => {
                    let tunnel = matches!(via, Via::Tunnel(_));
                    let messages = match self.pair(&code, &name, key.as_deref(), tunnel, &source) {
                        Ok((device, token, key)) => {
                            // Through the phone's tunnel: its peer belongs to this device from now on.
                            if let Via::Tunnel(peer) = &via {
                                peer.paired(&device.id);
                            }
                            let mut out = Vec::new();
                            if let Ok(text) =
                                serde_json::to_string(&ToClient::Paired { device, token, key })
                            {
                                out.push(Message::text(text));
                            }
                            out.push(Message::Close(Some(CloseFrame {
                                code: CloseCode::Normal,
                                reason: "paired".into(),
                            })));
                            out
                        }
                        Err((code, error, reason)) => {
                            refuse_with(code, error, None, None, reason, None, None)
                        }
                    };
                    send_all(&mut sink, messages).await;
                    return;
                }
                // A device that forgets this server (ESC-537): its token revoked; closed either way.
                Ok(ToServer::Unpair { token }) => {
                    let messages = match self.unpair(&token_of(token), &via) {
                        Some(_) => {
                            let mut out = Vec::new();
                            if let Ok(text) = serde_json::to_string(&ToClient::Unpaired {}) {
                                out.push(Message::text(text));
                            }
                            out.push(Message::Close(Some(CloseFrame {
                                code: CloseCode::Normal,
                                reason: "unpaired".into(),
                            })));
                            out
                        }
                        None => {
                            self.failed(&source);
                            refuse(
                                codes::TOKEN,
                                tr!(
                                    "Jeton d’appareil absent, inconnu ou révoqué.",
                                    "Device token missing, unknown or revoked."
                                ),
                                None,
                                None,
                            )
                        }
                    };
                    send_all(&mut sink, messages).await;
                    return;
                }
                _ => {
                    self.failed(&source);
                    send_all(
                        &mut sink,
                        refuse(codes::HANDSHAKE, handshake_text(), None, None),
                    )
                    .await;
                    return;
                }
            },
            _ => {
                self.failed(&source);
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
        // A device paired or revoked by another process meanwhile (`escouade-server pair`, `devices revoke`).
        self.devices.refresh();
        let refused = if let Err(e) = core.locks.read().check_remote_api() {
            Err(refuse(codes::POLICY, e.to_string(), None, None))
        } else if let Some(device) = self.devices.find(&token) {
            if version != server_version {
                let update = proto::to_update(&server_version, &version);
                // How to put the server at the client's version (ESC-515), for the client to offer it.
                let server_update = (update == "server").then(|| {
                    proto::server_update(
                        &version,
                        crate::server::status::supervisor()
                            == Some(crate::server::status::Supervisor::Docker),
                    )
                });
                Err(refuse_with(
                    codes::VERSION,
                    version_text(&server_version, &version, update),
                    Some(server_version.clone()),
                    Some(update.to_string()),
                    None,
                    None,
                    server_update,
                ))
            } else if let Some(bound) = match &via {
                Via::Tunnel(peer) => peer.device().filter(|d| *d != device.id),
                _ => None,
            } {
                // Through a peer of another device: its key and this token are not the same phone's.
                self.failed(&source);
                log::warn!(
                    "remote API: the token of « {} » through the tunnel's peer of another device ({bound})",
                    device.name
                );
                Err(refuse(
                    codes::TOKEN,
                    tr!(
                        "Jeton d’appareil absent, inconnu ou révoqué.",
                        "Device token missing, unknown or revoked."
                    ),
                    None,
                    None,
                ))
            } else {
                Ok(device)
            }
        } else {
            self.failed(&source);
            // A device revoked says so, when and by whom (ESC-537): « Ce poste n'est plus appairé ».
            let revoked = self.devices.revoked(&token);
            // Its version said (ESC-537): a client not paired yet learns that Escouade answers, and which.
            Err(refuse_with(
                codes::TOKEN,
                tr!(
                    "Jeton d’appareil absent, inconnu ou révoqué.",
                    "Device token missing, unknown or revoked."
                ),
                Some(server_version.clone()),
                None,
                None,
                revoked,
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
        // Room for it: the device's connections, and all; a device's first has the places kept for it (ESC-654).
        let client = Client::Remote(self.next.fetch_add(1, Ordering::Relaxed));
        let n = match client {
            Client::Remote(n) => n,
            Client::Window => 0,
        };
        let room = {
            let mut live = self.live.lock();
            let of_device = live.values().filter(|c| c.device == device.id).count();
            let total = if of_device == 0 {
                self.limits.total + self.limits.reserve
            } else {
                self.limits.total
            };
            if of_device >= self.limits.per_device || live.len() >= total {
                Err((of_device, live.len()))
            } else {
                let (session, resumed) =
                    self.sessions
                        .open(&device.id, asked.as_deref(), Instant::now());
                let conn = Arc::new(Conn {
                    client: client.clone(),
                    device: device.id.clone(),
                    scope: device.scope,
                    session,
                    sessions: self.sessions.clone(),
                    outbox: Outbox::new(self.limits.queue),
                    lanes: Lanes::new(Arc::new(Running::default())),
                    in_flight: AtomicUsize::new(0),
                    limit: self.limits.in_flight,
                    kicked: Notify::new(),
                    ended: tokio::sync::watch::channel(false).0,
                });
                live.insert(n, conn.clone());
                Ok((conn, resumed))
            }
        };
        match room {
            Ok((conn, resumed)) => {
                self.handshake_over(id);
                // Revoked between its token read and its place taken: put out as the revocation puts the others.
                if !self.devices.holds(&device.id) {
                    conn.kicked.notify_one();
                }
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
            secrets_in_file: core.host.secrets_in_file(),
            machine: core.host.machine(),
            missed: if resumed {
                self.sessions.missed(&conn.session, core.hub.emitted())
            } else {
                None
            },
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
                        // The way there and back (ESC-537): answered at once, before any call that waits.
                        Ok(ToServer::Ping { id }) => conn.send(&ToClient::Pong { id }),
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

        // Its calls under way let go, those waiting on its lanes never run (ESC-654).
        conn.ended.send_replace(true);
        // Counted before it leaves the live ones: who sees it gone sees why.
        self.counts
            .peak_queued
            .fetch_max(conn.outbox.peak(), Ordering::Relaxed);
        if slow.is_some() {
            self.counts.closed_slow.fetch_add(1, Ordering::Relaxed);
        }
        if let Some(task) = host_events {
            task.abort();
        }
        self.live.lock().retain(|_, c| !Arc::ptr_eq(c, &conn));
        self.sessions
            .note_seen(&conn.session, core.hub.emitted().max(1));
        self.sessions.leave(&conn.session, Instant::now());
        core.forget_client(&client);
        if let Some(why) = slow {
            // It reads nothing: its queue is let go, the close said if it ever reads again, and the socket closed
            // whatever it does.
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
        // The device's scope first (ESC-570, SR1-01), whatever the command's class: a phone never reaches a computer's
        // command, served or not.
        if !conn.scope.allows(command.scope) {
            log::warn!(
                "remote API: {} ({}) asked {}, refused: a command of a computer",
                conn.client.label(),
                conn.scope.as_str(),
                command.name
            );
            return conn.answer(
                id,
                Err(Failure::refused(
                    codes::SCOPE,
                    tr!(
                        "Un téléphone n’a pas accès à {name} : il répond, il ne touche ni au terminal, ni aux fichiers, ni aux secrets, ni aux plugins, ni aux appareils, ni au réseau.",
                        "A phone has no access to {name}: it answers, it touches neither the terminal, nor the files, the secrets, the plugins, the devices or the network.",
                        name = command.name
                    ),
                )),
            );
        }
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
        let (done, client, device) = (conn.clone(), conn.client.clone(), conn.device.clone());
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
            // A call waiting on its lane once its connection ended never runs; one under way goes to its end, its answer
            // to no one (a thread is not stopped from outside) (ESC-654).
            Reply::Blocking(job) => conn.lanes.submit(
                lane_of(name),
                Box::new(move || {
                    if *done.ended.borrow() {
                        return;
                    }
                    let out = std::panic::catch_unwind(AssertUnwindSafe(|| {
                        super::as_caller(client, device, job)
                    }))
                    .unwrap_or_else(|_| panicked());
                    done.answer(id, out);
                }),
            ),
            // A future is let go when its connection ends: closed, its device revoked (ESC-654).
            Reply::Future(future) => conn.lanes.submit(
                lane_of(name),
                Box::new(move || {
                    let mut ended = done.ended.subscribe();
                    if *ended.borrow() {
                        return;
                    }
                    // Started in its turn on its lane, as the IPC starts it; it runs on the runtime.
                    tokio::runtime::Handle::current().spawn(async move {
                        let call = AssertUnwindSafe(super::scoped_caller(client, device, future))
                            .catch_unwind();
                        tokio::select! {
                            out = call => done.answer(id, out.unwrap_or_else(|_| panicked())),
                            _ = ended.wait_for(|ended| *ended) => {
                                log::info!("remote API: {name} let go, its connection ended");
                            }
                        }
                    });
                }),
            ),
        }
    }
}

/// A device paired: it, the token to answer (empty for a phone, which derives it), and the core's public half of a
/// phone's pairing (ESC-570).
type Paired = (super::devices::Device, String, Option<String>);

/// The handshake of this number over, whatever its end: its place is free.
struct HandshakeOver(Arc<RemoteApi>, u64);

impl Drop for HandshakeOver {
    fn drop(&mut self) {
        self.0.handshake_over(self.1);
    }
}

/// `times` keeps those of the last minute alone.
fn within_a_minute(times: &mut VecDeque<Instant>, now: Instant) {
    while times
        .front()
        .is_some_and(|t| now.duration_since(*t) >= Duration::from_secs(60))
    {
        times.pop_front();
    }
}

/// What a connection runs on: a TCP connection of the loopback or of Tailscale, or one of the phone's tunnel.
pub trait Io: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static {}
impl<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static> Io for T {}

type WsSink =
    futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<Box<dyn Io>>, Message>;
type WsStream = futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<Box<dyn Io>>>;

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
/// Escouade's own window's (and, through the tunnel or Tailscale, the phone app's: [`Way::origins`]); a single `Host`
/// and `Origin`, and the authority of an absolute URI the `Host`'s. Else its HTTP status and why (shown in the log: no
/// secret there).
fn guard(
    req: &Request,
    hosts: &[String],
    way: Way,
) -> std::result::Result<(), (tauri::http::StatusCode, String)> {
    let through = way != Way::Loopback;
    use tauri::http::StatusCode;
    if req.uri().path() != proto::PATH {
        return Err((
            StatusCode::NOT_FOUND,
            format!("path {}", shown(req.uri().path())),
        ));
    }
    // One name alone: a `Host` or an `Origin` given twice, a proxy and this server might each read another (ESC-688).
    for name in ["host", "origin"] {
        if req.headers().get_all(name).iter().nth(1).is_some() {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("more than one {name} header"),
            ));
        }
    }
    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    // An absolute URI names the server too: the same as its `Host`, or refused.
    if let Some(authority) = req.uri().authority() {
        if !authority.as_str().eq_ignore_ascii_case(host) {
            return Err((
                StatusCode::BAD_REQUEST,
                format!(
                    "an absolute URI of another authority ({}) than its Host ({})",
                    shown(authority.as_str()),
                    shown(host)
                ),
            ));
        }
    }
    // Through the tunnel or Tailscale, the address the phone reached alone; on the loopback, the loopback (or a name
    // asked for in so many words).
    let host_ok = if through {
        allowed_host(host, hosts)
    } else {
        loopback_host(host) || allowed_host(host, hosts)
    };
    if !host_ok {
        return Err((
            StatusCode::FORBIDDEN,
            format!("a Host that is not the loopback ({})", shown(host)),
        ));
    }
    if let Some(origin) = req.headers().get("origin") {
        let origin = origin.to_str().unwrap_or("");
        if !proto::ORIGINS.contains(&origin) && !way.origins().contains(&origin) {
            return Err((
                StatusCode::FORBIDDEN,
                format!("the Origin of a web page ({})", shown(origin)),
            ));
        }
    }
    Ok(())
}

/// Where a connection comes from, as its failures are counted (SR1-04): its key (the address of the other end, the
/// client a declared proxy forwards, the tunnel's peer: every phone has its own), and whether it is this machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Source {
    key: String,
    reach: Reach,
}

/// Whether a source is this machine (the loopback: its window, its command line, a client through SSH) or anywhere
/// else (the tunnel, Tailscale, the network of `--listen`, a client behind a proxy): the wrong pairing codes from
/// elsewhere are bounded apart, and never refuse those of this machine (SR1-04).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reach {
    Here,
    Elsewhere,
}

impl Source {
    fn of(via: &Via, from: Option<std::net::IpAddr>) -> Self {
        let (key, reach) = match (via, from) {
            (Via::Tunnel(peer), _) => (format!("peer {}", peer.label()), Reach::Elsewhere),
            (Via::Loopback, Some(ip)) if ip.is_loopback() => (ip.to_string(), Reach::Here),
            (_, Some(ip)) => (ip.to_string(), Reach::Elsewhere),
            (Via::Loopback, None) => (Ipv4Addr::LOCALHOST.to_string(), Reach::Here),
            (Via::Tailscale(_), None) => ("tailscale".to_string(), Reach::Elsewhere),
        };
        Self { key, reach }
    }

    /// The client a declared proxy forwards: elsewhere, whatever its address says.
    fn forwarded(client: std::net::IpAddr) -> Self {
        Self {
            key: format!("{client} (forwarded)"),
            reach: Reach::Elsewhere,
        }
    }
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.key)
    }
}

/// The device's token a phone's native proxy wrote in the request (`proto::DEVICE_HEADER`, ESC-570): one, not empty;
/// none else (given twice, it is none).
fn device_header(req: &Request) -> Option<String> {
    let mut all = req.headers().get_all(proto::DEVICE_HEADER).iter();
    let first = all.next()?;
    if all.next().is_some() {
        return None;
    }
    let text = first.to_str().ok()?.trim();
    (!text.is_empty() && text.len() <= 256).then(|| text.to_string())
}

/// The client a proxy says it forwards: the last address of its last `X-Forwarded-For` (the one the proxy wrote; those
/// before it, the client may have written itself). None when it says none, or not an address.
fn forwarded_for(req: &Request) -> Option<std::net::IpAddr> {
    let last = req
        .headers()
        .get_all("x-forwarded-for")
        .iter()
        .next_back()?;
    let value = last.to_str().ok()?;
    let ip = value.rsplit(',').next()?.trim();
    // An address of IPv6 may come in brackets, with a port or not; one of IPv4 with a port.
    let ip = ip
        .strip_prefix('[')
        .and_then(|rest| rest.split(']').next())
        .unwrap_or(ip);
    ip.parse()
        .ok()
        .or_else(|| ip.parse::<SocketAddr>().ok().map(|a| a.ip()))
}

/// The way of a connection, as the guard of its request reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Way {
    Loopback,
    Tunnel,
    Tailscale,
}

impl Way {
    fn of(via: &Via) -> Self {
        match via {
            Via::Loopback => Way::Loopback,
            Via::Tunnel(_) => Way::Tunnel,
            Via::Tailscale(_) => Way::Tailscale,
        }
    }

    /// The `Origin`s of the phone app taken this way, besides the window's (SR1-09): none on the loopback, where they
    /// would be a page of the machine; through Tailscale, the app's own alone, never a `localhost` of another machine.
    fn origins(self) -> &'static [&'static str] {
        match self {
            Way::Loopback => &[],
            Way::Tunnel => proto::PHONE_ORIGINS,
            Way::Tailscale => proto::TAILSCALE_ORIGINS,
        }
    }
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

/// `host` (its port aside) is one of `hosts`, the names asked for in so many words.
fn allowed_host(host: &str, hosts: &[String]) -> bool {
    if host.starts_with('[') {
        return false;
    }
    let name = host.split(':').next().unwrap_or("").to_ascii_lowercase();
    !name.is_empty() && hosts.contains(&name)
}

/// A text a client gave, as the log or an answer may show it: 80 characters at most, printable ones.
fn shown(value: &str) -> String {
    value.chars().filter(|c| !c.is_control()).take(80).collect()
}
