//! The tunnel of the phone (ESC-567, `docs/REMOTE.md`, « Le téléphone », « Architecture retenue »): a WireGuard peer
//! in user space (`boringtun`, the protocol alone) on a UDP socket of the process, and a TCP/IP stack of its own
//! inside the tunnel (`smoltcp`). No interface of the system (`wg0`), no driver (Wintun), no Network Extension, no
//! capability: nothing asks for an administrator, and only Escouade's traffic goes through it.
//!
//! - **One task** (the pump) owns the socket, the peers' sessions and the stack: a datagram, a timer, a write of a
//!   connection or an order (`Order`) wakes it; nothing else touches them.
//! - **Peers** are known by their public key: a handshake whose key is no peer's is dropped without an answer (a scan
//!   sees nothing), and counted (`Counts::unknown`). A packet out of the tunnel is accepted from its peer's address in
//!   the tunnel only (`AllowedIPs` of one address).
//! - **Connections**: the stack listens on one TCP port of the tunnel's address (`Config::listen`), and each
//!   connection a peer opens is handed out as a [`TunnelStream`] (tokio's `AsyncRead`/`AsyncWrite`) with the peer it
//!   comes from. The same stack connects out ([`Tunnel::connect`]): the tests' phone, and later the phone's own side.
//! - **Bounded**: a connection's bytes wait in two channels of [`CHUNKS`] chunks; when the reader lags, the stack stops
//!   reading and TCP's window holds the sender back. [`MAX_CONNECTIONS`] connections at once.

use anyhow::{anyhow, Result};
use boringtun::noise::handshake::parse_handshake_anon;
use boringtun::noise::rate_limiter::RateLimiter;
use boringtun::noise::{Packet, Tunn, TunnResult};
use boringtun::x25519::{PublicKey, StaticSecret};
use smoltcp::iface::{Config as IfaceConfig, Interface, SocketHandle, SocketSet};
use smoltcp::phy::{Device, DeviceCapabilities, Medium};
use smoltcp::socket::tcp;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr, IpEndpoint};
use std::collections::VecDeque;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, oneshot, Notify};
use tokio_util::sync::PollSender;

/// The tunnel's MTU, WireGuard's usual one.
pub const MTU: usize = 1420;
/// The chunks a connection's bytes wait in, each way.
pub const CHUNKS: usize = 32;
/// The connections at once, all peers together.
pub const MAX_CONNECTIONS: usize = 64;
/// The sockets kept listening, for connections that open together.
const LISTENING: usize = 4;
/// The bytes of a chunk, at most.
const CHUNK: usize = 16 * 1024;
/// A TCP socket's buffers, each way.
const TCP_BUFFER: usize = 64 * 1024;
/// The datagrams taken at once before the stack turns.
const BURST: usize = 256;
/// The UDP socket's buffers, each way: a TCP window's burst in flight, with room to spare.
const UDP_BUFFER: usize = 4 * 1024 * 1024;
/// How often the sessions' timers are looked at (WireGuard's own tick).
const TICK: Duration = Duration::from_millis(250);
/// The handshakes a second before cookies are asked (boringtun's default for a device).
const HANDSHAKE_RATE: u64 = 100;

/// A peer of the tunnel.
#[derive(Clone)]
pub struct PeerConfig {
    /// Its id, as the stream of its connections says it.
    pub id: String,
    pub public: [u8; 32],
    /// Its address in the tunnel: the only one its packets may come from (its `AllowedIPs`).
    pub address: Ipv4Addr,
    /// Where it is, when it is known before it speaks (a client's server).
    pub endpoint: Option<SocketAddr>,
    /// Seconds between keepalives (a client behind a NAT); none on a server.
    pub keepalive: Option<u16>,
}

/// What the tunnel is.
pub struct Config {
    pub private: StaticSecret,
    /// Its own address in the tunnel (`10.77.0.1` on the core).
    pub address: Ipv4Addr,
    /// The prefix of the tunnel's network.
    pub prefix: u8,
    /// The TCP port it listens on in the tunnel; none for a client.
    pub listen: Option<u16>,
    pub peers: Vec<PeerConfig>,
}

/// What the tunnel counted.
#[derive(Default)]
pub struct Counts {
    /// Handshakes of a key that is no peer's, dropped without an answer.
    pub unknown: AtomicU64,
    /// Datagrams that are not WireGuard, or whose MAC is wrong: dropped.
    pub invalid: AtomicU64,
    /// Packets out of the tunnel from an address that is not their peer's: dropped.
    pub spoofed: AtomicU64,
    /// Handshakes completed.
    pub handshakes: AtomicU64,
    /// Connections handed out.
    pub accepted: AtomicU64,
}

/// A peer, as the tunnel sees it now.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerState {
    pub id: String,
    /// Milliseconds since its last handshake; none when it never made one (or it expired).
    pub since_handshake: Option<u64>,
    pub endpoint: Option<String>,
    pub rx: u64,
    pub tx: u64,
}

/// A connection the tunnel accepted: its stream, and the id of the peer it comes from.
pub struct Accepted {
    pub stream: TunnelStream,
    pub peer: String,
    /// The address it was opened to, in the tunnel.
    pub local: SocketAddrV4,
}

enum Order {
    Peers(Vec<PeerConfig>),
    Key(StaticSecret),
    Connect(SocketAddrV4, oneshot::Sender<Result<TunnelStream>>),
    State(oneshot::Sender<Vec<PeerState>>),
}

/// The tunnel, running: dropped or stopped, its task ends and its socket closes.
pub struct Tunnel {
    orders: mpsc::UnboundedSender<Order>,
    stop: Arc<Notify>,
    wake: Arc<Notify>,
    counts: Arc<Counts>,
    local: SocketAddr,
}

impl Drop for Tunnel {
    fn drop(&mut self) {
        self.stop.notify_one();
    }
}

impl Tunnel {
    /// The tunnel on `socket`, its connections sent to `accepted` (a client gives none). On a runtime of Tokio.
    pub fn start(
        socket: UdpSocket,
        config: Config,
        accepted: Option<mpsc::Sender<Accepted>>,
    ) -> Result<Self> {
        let local = socket.local_addr()?;
        let (orders, rx) = mpsc::unbounded_channel();
        let stop = Arc::new(Notify::new());
        let wake = Arc::new(Notify::new());
        let counts = Arc::new(Counts::default());
        let pump = Pump::new(socket, config, accepted, wake.clone(), counts.clone());
        let stopped = stop.clone();
        tokio::spawn(async move { pump.run(rx, stopped).await });
        Ok(Self {
            orders,
            stop,
            wake,
            counts,
            local,
        })
    }

    /// Where its UDP socket is bound.
    pub fn local_addr(&self) -> SocketAddr {
        self.local
    }

    pub fn counts(&self) -> &Counts {
        &self.counts
    }

    /// Its peers from now on: a peer kept (same key) keeps its session, one gone loses its connections.
    pub fn set_peers(&self, peers: Vec<PeerConfig>) {
        let _ = self.orders.send(Order::Peers(peers));
        self.wake.notify_one();
    }

    /// Its own key from now on: every session is dropped, and a peer that still knows the old public key is no
    /// longer answered.
    pub fn set_key(&self, private: StaticSecret) {
        let _ = self.orders.send(Order::Key(private));
        self.wake.notify_one();
    }

    /// A TCP connection to `to` in the tunnel (a client's).
    pub async fn connect(&self, to: SocketAddrV4) -> Result<TunnelStream> {
        let (tx, rx) = oneshot::channel();
        self.orders
            .send(Order::Connect(to, tx))
            .map_err(|_| anyhow!("tunnel stopped"))?;
        self.wake.notify_one();
        rx.await.map_err(|_| anyhow!("tunnel stopped"))?
    }

    /// Its peers, now.
    pub async fn peers(&self) -> Vec<PeerState> {
        let (tx, rx) = oneshot::channel();
        if self.orders.send(Order::State(tx)).is_err() {
            return Vec::new();
        }
        self.wake.notify_one();
        rx.await.unwrap_or_default()
    }

    /// Stopped: its task ends, its connections close.
    pub fn stop(&self) {
        self.stop.notify_one();
    }
}

// ---------- a connection, on the side of who uses it ----------

/// A TCP connection in the tunnel, as tokio's `AsyncRead` and `AsyncWrite`.
pub struct TunnelStream {
    rx: mpsc::Receiver<Vec<u8>>,
    unread: Vec<u8>,
    at: usize,
    tx: PollSender<Vec<u8>>,
    wake: Arc<Notify>,
}

impl AsyncRead for TunnelStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let me = &mut *self;
        if me.at >= me.unread.len() {
            match me.rx.poll_recv(cx) {
                Poll::Ready(Some(chunk)) => {
                    me.unread = chunk;
                    me.at = 0;
                    // Room again in the channel: the stack may read the socket on.
                    me.wake.notify_one();
                }
                // The other side closed: the end of the stream.
                Poll::Ready(None) => return Poll::Ready(Ok(())),
                Poll::Pending => return Poll::Pending,
            }
        }
        let n = buf.remaining().min(me.unread.len() - me.at);
        buf.put_slice(&me.unread[me.at..me.at + n]);
        me.at += n;
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for TunnelStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let me = &mut *self;
        match me.tx.poll_reserve(cx) {
            Poll::Ready(Ok(())) => {
                let n = buf.len().min(CHUNK);
                if me.tx.send_item(buf[..n].to_vec()).is_err() {
                    return Poll::Ready(Err(std::io::ErrorKind::BrokenPipe.into()));
                }
                me.wake.notify_one();
                Poll::Ready(Ok(n))
            }
            Poll::Ready(Err(_)) => Poll::Ready(Err(std::io::ErrorKind::BrokenPipe.into())),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.tx.close();
        self.wake.notify_one();
        Poll::Ready(Ok(()))
    }
}

impl Drop for TunnelStream {
    fn drop(&mut self) {
        self.wake.notify_one();
    }
}

// ---------- the stack's side ----------

/// The IP packets between the stack and WireGuard.
#[derive(Default)]
struct Queues {
    rx: VecDeque<Vec<u8>>,
    tx: VecDeque<Vec<u8>>,
}

struct RxToken(Vec<u8>);
struct TxToken<'a>(&'a mut VecDeque<Vec<u8>>);

impl smoltcp::phy::RxToken for RxToken {
    fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R {
        f(&self.0)
    }
}

impl smoltcp::phy::TxToken for TxToken<'_> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        let mut packet = vec![0; len];
        let out = f(&mut packet);
        self.0.push_back(packet);
        out
    }
}

impl Device for Queues {
    type RxToken<'a> = RxToken;
    type TxToken<'a> = TxToken<'a>;

    fn receive(
        &mut self,
        _: smoltcp::time::Instant,
    ) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let packet = self.rx.pop_front()?;
        Some((RxToken(packet), TxToken(&mut self.tx)))
    }

    fn transmit(&mut self, _: smoltcp::time::Instant) -> Option<Self::TxToken<'_>> {
        Some(TxToken(&mut self.tx))
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ip;
        caps.max_transmission_unit = MTU;
        caps
    }
}

struct Peer {
    config: PeerConfig,
    tunn: Tunn,
    index: u32,
    endpoint: Option<SocketAddr>,
    /// Whether its session was up at the last look, to count a new handshake once.
    up: bool,
}

/// A connection, on the stack's side.
struct Conn {
    socket: SocketHandle,
    peer: String,
    /// Toward who uses it; dropped once the peer closed its side and everything was handed over.
    to_app: Option<mpsc::Sender<Vec<u8>>>,
    from_app: mpsc::Receiver<Vec<u8>>,
    /// What the user wrote that the socket has not taken yet.
    pending: Option<(Vec<u8>, usize)>,
    app_closed: bool,
}

/// A connection asked for (`connect`), until it is established or fails.
struct Opening {
    socket: SocketHandle,
    peer: String,
    reply: oneshot::Sender<Result<TunnelStream>>,
    asked: Instant,
}

struct Pump {
    socket: UdpSocket,
    private: StaticSecret,
    public: PublicKey,
    limiter: Arc<RateLimiter>,
    address: Ipv4Addr,
    listen: Option<u16>,
    peers: Vec<Peer>,
    next_index: u32,
    iface: Interface,
    queues: Queues,
    sockets: SocketSet<'static>,
    listening: Vec<SocketHandle>,
    conns: Vec<Conn>,
    opening: Vec<Opening>,
    accepted: Option<mpsc::Sender<Accepted>>,
    wake: Arc<Notify>,
    counts: Arc<Counts>,
    started: Instant,
    next_port: u16,
}

fn now_of(started: Instant) -> smoltcp::time::Instant {
    smoltcp::time::Instant::from_millis(started.elapsed().as_millis() as i64)
}

fn random_u32() -> u32 {
    let mut b = [0u8; 4];
    let _ = getrandom::fill(&mut b);
    u32::from_le_bytes(b)
}

impl Pump {
    fn new(
        socket: UdpSocket,
        config: Config,
        accepted: Option<mpsc::Sender<Accepted>>,
        wake: Arc<Notify>,
        counts: Arc<Counts>,
    ) -> Self {
        let public = PublicKey::from(&config.private);
        let limiter = Arc::new(RateLimiter::new(&public, HANDSHAKE_RATE));
        let mut queues = Queues::default();
        let started = Instant::now();
        let mut iface_config = IfaceConfig::new(HardwareAddress::Ip);
        iface_config.random_seed = u64::from(random_u32()) << 32 | u64::from(random_u32());
        let mut iface = Interface::new(iface_config, &mut queues, now_of(started));
        iface.update_ip_addrs(|addrs| {
            let _ = addrs.push(IpCidr::new(IpAddress::Ipv4(config.address), config.prefix));
        });
        let mut pump = Self {
            socket,
            private: config.private,
            public,
            limiter,
            address: config.address,
            listen: config.listen,
            peers: Vec::new(),
            // A peer's index is its 24 high bits of the session's index (boringtun shifts it by 8).
            next_index: random_u32() & 0x00ff_ffff,
            iface,
            queues,
            sockets: SocketSet::new(Vec::new()),
            listening: Vec::new(),
            conns: Vec::new(),
            opening: Vec::new(),
            accepted,
            wake,
            counts,
            started,
            next_port: 49152 + (random_u32() % 16000) as u16,
        };
        pump.set_peers(config.peers);
        pump.refill_listening();
        pump
    }

    fn new_tcp() -> tcp::Socket<'static> {
        let mut socket = tcp::Socket::new(
            tcp::SocketBuffer::new(vec![0; TCP_BUFFER]),
            tcp::SocketBuffer::new(vec![0; TCP_BUFFER]),
        );
        socket.set_nagle_enabled(false);
        // A phone gone without a word (out of coverage) does not hold a connection for ever.
        socket.set_keep_alive(Some(smoltcp::time::Duration::from_secs(20)));
        socket.set_timeout(Some(smoltcp::time::Duration::from_secs(90)));
        socket
    }

    fn refill_listening(&mut self) {
        let Some(port) = self.listen else {
            return;
        };
        while self.listening.len() < LISTENING
            && self.conns.len() + self.listening.len() < MAX_CONNECTIONS + LISTENING
        {
            let mut socket = Self::new_tcp();
            if socket
                .listen(IpEndpoint::new(IpAddress::Ipv4(self.address), port))
                .is_err()
            {
                return;
            }
            self.listening.push(self.sockets.add(socket));
        }
    }

    fn set_peers(&mut self, wanted: Vec<PeerConfig>) {
        let mut kept = Vec::new();
        let mut old: Vec<Peer> = std::mem::take(&mut self.peers);
        for config in wanted {
            if let Some(at) = old.iter().position(|p| p.config.public == config.public) {
                let mut peer = old.swap_remove(at);
                if config.endpoint.is_some() {
                    peer.endpoint = config.endpoint;
                }
                peer.config = config;
                kept.push(peer);
            } else {
                let index = self.next_index;
                self.next_index = (self.next_index + 1) & 0x00ff_ffff;
                let tunn = Tunn::new(
                    self.private.clone(),
                    PublicKey::from(config.public),
                    None,
                    config.keepalive,
                    index,
                    Some(self.limiter.clone()),
                );
                kept.push(Peer {
                    endpoint: config.endpoint,
                    config,
                    tunn,
                    index,
                    up: false,
                });
            }
        }
        // The connections of a peer gone are cut.
        for gone in old {
            self.cut_peer(&gone.config.id);
        }
        self.peers = kept;
    }

    fn cut_peer(&mut self, id: &str) {
        let mut i = 0;
        while i < self.conns.len() {
            if self.conns[i].peer == id {
                let conn = self.conns.swap_remove(i);
                self.sockets.get_mut::<tcp::Socket>(conn.socket).abort();
                self.sockets.remove(conn.socket);
            } else {
                i += 1;
            }
        }
    }

    fn set_key(&mut self, private: StaticSecret) {
        self.public = PublicKey::from(&private);
        self.limiter = Arc::new(RateLimiter::new(&self.public, HANDSHAKE_RATE));
        self.private = private;
        for peer in &mut self.peers {
            peer.tunn.set_static_private(
                self.private.clone(),
                self.public,
                Some(self.limiter.clone()),
            );
            peer.up = false;
        }
        let ids: Vec<String> = self.peers.iter().map(|p| p.config.id.clone()).collect();
        for id in ids {
            self.cut_peer(&id);
        }
    }

    async fn run(mut self, mut orders: mpsc::UnboundedReceiver<Order>, stop: Arc<Notify>) {
        let mut buf = vec![0u8; 65536];
        let mut tick = tokio::time::interval(TICK);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut limiter_reset = Instant::now();
        loop {
            let delay = self
                .iface
                .poll_delay(now_of(self.started), &self.sockets)
                .map(|d| Duration::from_micros(d.total_micros()))
                .unwrap_or(TICK)
                .min(TICK);
            tokio::select! {
                _ = stop.notified() => break,
                received = self.socket.recv_from(&mut buf) => match received {
                    Ok((n, from)) => {
                        let datagram = buf[..n].to_vec();
                        self.datagram(&datagram, from).await;
                        // What came meanwhile, taken in the same turn: a burst of TCP is never left to overflow the
                        // socket's buffer while the stack turns once per datagram.
                        for _ in 0..BURST {
                            let Ok((n, from)) = self.socket.try_recv_from(&mut buf) else {
                                break;
                            };
                            let datagram = buf[..n].to_vec();
                            self.datagram(&datagram, from).await;
                        }
                    }
                    // A port unreachable of an earlier send (Windows says it on the next receive): nothing to do.
                    Err(e) => log::debug!("phone tunnel: receive: {e}"),
                },
                order = orders.recv() => match order {
                    Some(order) => self.order(order),
                    None => break,
                },
                _ = self.wake.notified() => {},
                _ = tick.tick() => {
                    self.timers().await;
                    if limiter_reset.elapsed() >= Duration::from_secs(1) {
                        self.limiter.reset_count();
                        limiter_reset = Instant::now();
                    }
                },
                _ = tokio::time::sleep(delay) => {},
            }
            self.turn().await;
        }
        for conn in self.conns.drain(..) {
            self.sockets.get_mut::<tcp::Socket>(conn.socket).abort();
        }
        log::debug!("phone tunnel: stopped");
    }

    fn order(&mut self, order: Order) {
        match order {
            Order::Peers(peers) => self.set_peers(peers),
            Order::Key(private) => self.set_key(private),
            Order::State(reply) => {
                let states = self
                    .peers
                    .iter()
                    .map(|p| {
                        let (since, tx, rx, _, _) = p.tunn.stats();
                        PeerState {
                            id: p.config.id.clone(),
                            since_handshake: since.map(|d| d.as_millis() as u64),
                            endpoint: p.endpoint.map(|e| e.to_string()),
                            rx: rx as u64,
                            tx: tx as u64,
                        }
                    })
                    .collect();
                let _ = reply.send(states);
            }
            Order::Connect(to, reply) => {
                let Some(peer) = self.peer_for(*to.ip()) else {
                    let _ = reply.send(Err(anyhow!("no peer for {to}")));
                    return;
                };
                let peer = peer.config.id.clone();
                let mut socket = Self::new_tcp();
                self.next_port = if self.next_port >= 65000 {
                    49152
                } else {
                    self.next_port + 1
                };
                let port = self.next_port;
                match socket.connect(
                    self.iface.context(),
                    IpEndpoint::new(IpAddress::Ipv4(*to.ip()), to.port()),
                    port,
                ) {
                    Ok(()) => {
                        let handle = self.sockets.add(socket);
                        self.opening.push(Opening {
                            socket: handle,
                            peer,
                            reply,
                            asked: Instant::now(),
                        });
                    }
                    Err(e) => {
                        let _ = reply.send(Err(anyhow!("connect: {e}")));
                    }
                }
            }
        }
    }

    fn peer_for(&self, address: Ipv4Addr) -> Option<&Peer> {
        self.peers.iter().find(|p| p.config.address == address)
    }

    /// A datagram from the network: a handshake, a cookie, or a packet of a session.
    async fn datagram(&mut self, datagram: &[u8], from: SocketAddr) {
        let mut cookie = [0u8; 256];
        let which = match self
            .limiter
            .verify_packet(Some(from.ip()), datagram, &mut cookie)
        {
            Ok(packet) => match packet {
                Packet::HandshakeInit(ref init) => {
                    match parse_handshake_anon(&self.private, &self.public, init) {
                        Ok(half) => self
                            .peers
                            .iter()
                            .position(|p| p.config.public == half.peer_static_public)
                            .ok_or(true),
                        // Encrypted to another key (an old one, after a rotation): no peer of ours.
                        Err(_) => Err(true),
                    }
                }
                Packet::HandshakeResponse(r) => self.by_index(r.receiver_idx).ok_or(false),
                Packet::PacketCookieReply(c) => self.by_index(c.receiver_idx).ok_or(false),
                Packet::PacketData(d) => self.by_index(d.receiver_idx).ok_or(false),
            },
            Err(TunnResult::WriteToNetwork(reply)) => {
                // Under load: a cookie to the one who asks, as WireGuard says.
                let _ = self.socket.send_to(reply, from).await;
                return;
            }
            Err(_) => {
                self.counts.invalid.fetch_add(1, Ordering::Relaxed);
                return;
            }
        };
        let at = match which {
            Ok(at) => at,
            Err(unknown) => {
                if unknown {
                    self.counts.unknown.fetch_add(1, Ordering::Relaxed);
                    log::debug!("phone tunnel: a handshake of an unknown key from {from}, dropped");
                } else {
                    self.counts.invalid.fetch_add(1, Ordering::Relaxed);
                }
                return;
            }
        };
        let mut out = vec![0u8; 65536];
        let mut first = true;
        loop {
            let peer = &mut self.peers[at];
            let result = if first {
                peer.tunn.decapsulate(Some(from.ip()), datagram, &mut out)
            } else {
                peer.tunn.decapsulate(None, &[], &mut out)
            };
            first = false;
            match result {
                TunnResult::WriteToNetwork(bytes) => {
                    peer.endpoint = Some(from);
                    let bytes = bytes.to_vec();
                    let _ = self.socket.send_to(&bytes, from).await;
                    // Packets that waited for the session go now.
                    continue;
                }
                TunnResult::WriteToTunnelV4(packet, source) => {
                    peer.endpoint = Some(from);
                    if source == peer.config.address {
                        self.queues.rx.push_back(packet.to_vec());
                    } else {
                        self.counts.spoofed.fetch_add(1, Ordering::Relaxed);
                    }
                }
                TunnResult::WriteToTunnelV6(..) => {
                    self.counts.spoofed.fetch_add(1, Ordering::Relaxed);
                }
                TunnResult::Done => {
                    peer.endpoint = Some(from);
                }
                TunnResult::Err(e) => {
                    log::debug!("phone tunnel: a packet from {from} refused: {e:?}");
                    self.counts.invalid.fetch_add(1, Ordering::Relaxed);
                }
            }
            break;
        }
        self.note_handshakes();
    }

    fn by_index(&self, receiver: u32) -> Option<usize> {
        let index = receiver >> 8;
        self.peers.iter().position(|p| p.index == index)
    }

    fn note_handshakes(&mut self) {
        for peer in &mut self.peers {
            let up = peer.tunn.time_since_last_handshake().is_some();
            if up && !peer.up {
                self.counts.handshakes.fetch_add(1, Ordering::Relaxed);
                log::info!("phone tunnel: handshake with peer {}", peer.config.id);
            }
            peer.up = up;
        }
    }

    async fn timers(&mut self) {
        let mut out = vec![0u8; 2048];
        for at in 0..self.peers.len() {
            let peer = &mut self.peers[at];
            if let TunnResult::WriteToNetwork(bytes) = peer.tunn.update_timers(&mut out) {
                if let Some(to) = peer.endpoint {
                    let bytes = bytes.to_vec();
                    let _ = self.socket.send_to(&bytes, to).await;
                }
            }
        }
        self.note_handshakes();
    }

    /// The stack turned: the packets in taken, the connections moved, the packets out sent.
    async fn turn(&mut self) {
        for _ in 0..8 {
            self.iface
                .poll(now_of(self.started), &mut self.queues, &mut self.sockets);
            let moved = self.connections();
            self.iface
                .poll(now_of(self.started), &mut self.queues, &mut self.sockets);
            self.send_out().await;
            if !moved {
                break;
            }
        }
    }

    /// The IP packets the stack wrote, each through its peer's session.
    async fn send_out(&mut self) {
        let mut out = vec![0u8; MTU + 256];
        while let Some(packet) = self.queues.tx.pop_front() {
            let Some(IpAddr::V4(to)) = Tunn::dst_address(&packet) else {
                continue;
            };
            let Some(peer) = self.peers.iter_mut().find(|p| p.config.address == to) else {
                continue;
            };
            if let TunnResult::WriteToNetwork(bytes) = peer.tunn.encapsulate(&packet, &mut out) {
                if let Some(endpoint) = peer.endpoint {
                    let bytes = bytes.to_vec();
                    let _ = self.socket.send_to(&bytes, endpoint).await;
                }
            }
        }
    }

    /// The connections moved: the new ones handed out, the bytes passed each way, the closed ones forgotten. True
    /// when something moved.
    fn connections(&mut self) -> bool {
        let mut moved = false;
        // New connections on the listening sockets.
        let mut i = 0;
        while i < self.listening.len() {
            let handle = self.listening[i];
            let socket = self.sockets.get_mut::<tcp::Socket>(handle);
            // Handed out once established: a SYN alone (or one reset) stays the listener's.
            if matches!(socket.state(), tcp::State::Listen | tcp::State::SynReceived) {
                i += 1;
                continue;
            }
            self.listening.swap_remove(i);
            moved = true;
            let remote = socket.remote_endpoint();
            let local = socket.local_endpoint();
            let peer = remote.and_then(|r| match r.addr {
                IpAddress::Ipv4(a) => self.peer_for(a).map(|p| p.config.id.clone()),
            });
            let (Some(peer), Some(accepted), Some(local)) = (peer, self.accepted.clone(), local)
            else {
                self.sockets.get_mut::<tcp::Socket>(handle).abort();
                self.sockets.remove(handle);
                continue;
            };
            if self.conns.len() >= MAX_CONNECTIONS {
                log::warn!("phone tunnel: {MAX_CONNECTIONS} connections already: one refused");
                self.sockets.get_mut::<tcp::Socket>(handle).abort();
                self.sockets.remove(handle);
                continue;
            }
            let (stream, conn) = self.pair(handle, peer.clone());
            let local = match local.addr {
                IpAddress::Ipv4(a) => SocketAddrV4::new(a, local.port),
            };
            match accepted.try_send(Accepted {
                stream,
                peer,
                local,
            }) {
                Ok(()) => {
                    self.counts.accepted.fetch_add(1, Ordering::Relaxed);
                    self.conns.push(conn);
                }
                Err(_) => {
                    self.sockets.get_mut::<tcp::Socket>(handle).abort();
                    self.sockets.remove(handle);
                }
            }
        }
        self.refill_listening();
        // Connections asked for, established or failed.
        let mut i = 0;
        while i < self.opening.len() {
            let handle = self.opening[i].socket;
            let state = self.sockets.get::<tcp::Socket>(handle).state();
            let late = self.opening[i].asked.elapsed() > Duration::from_secs(20);
            if matches!(state, tcp::State::Established) {
                let opening = self.opening.swap_remove(i);
                let (stream, conn) = self.pair(handle, opening.peer);
                self.conns.push(conn);
                let _ = opening.reply.send(Ok(stream));
                moved = true;
            } else if matches!(state, tcp::State::Closed) || late {
                let opening = self.opening.swap_remove(i);
                self.sockets.remove(handle);
                let _ = opening
                    .reply
                    .send(Err(anyhow!("the connection did not open")));
            } else {
                i += 1;
            }
        }
        // Bytes each way.
        let mut i = 0;
        while i < self.conns.len() {
            let conn = &mut self.conns[i];
            let socket = self.sockets.get_mut::<tcp::Socket>(conn.socket);
            if conn.to_app.is_none() {
                // Nobody reads: dropped, so that the peer's window never stays shut.
                while socket.can_recv() {
                    if socket.recv(|data| (data.len(), ())).is_err() {
                        break;
                    }
                }
            }
            // In: from the peer to who uses it, while there is room.
            if let Some(to_app) = &conn.to_app {
                while socket.can_recv() {
                    let Ok(permit) = to_app.try_reserve() else {
                        break;
                    };
                    match socket.recv(|data| {
                        let n = data.len().min(CHUNK);
                        (n, data[..n].to_vec())
                    }) {
                        Ok(chunk) if !chunk.is_empty() => {
                            permit.send(chunk);
                            moved = true;
                        }
                        _ => break,
                    }
                }
                if to_app.is_closed() {
                    // Who used it no longer reads: what comes is dropped, what it wrote still goes, then the end.
                    conn.to_app = None;
                    moved = true;
                } else if !socket.may_recv()
                    && !socket.can_recv()
                    && !matches!(
                        socket.state(),
                        tcp::State::SynSent | tcp::State::SynReceived
                    )
                {
                    // The peer closed its side, and everything was handed over: the end of the stream.
                    conn.to_app = None;
                    moved = true;
                }
            }
            // Out: from who uses it to the peer, while the socket takes it.
            loop {
                if let Some((data, at)) = &mut conn.pending {
                    if !socket.can_send() {
                        break;
                    }
                    match socket.send_slice(&data[*at..]) {
                        Ok(n) => {
                            *at += n;
                            moved |= n > 0;
                            if *at < data.len() {
                                break;
                            }
                            conn.pending = None;
                        }
                        Err(_) => {
                            conn.pending = None;
                            break;
                        }
                    }
                } else if conn.app_closed {
                    break;
                } else {
                    match conn.from_app.try_recv() {
                        Ok(data) => conn.pending = Some((data, 0)),
                        Err(mpsc::error::TryRecvError::Empty) => break,
                        Err(mpsc::error::TryRecvError::Disconnected) => {
                            conn.app_closed = true;
                            socket.close();
                            moved = true;
                            break;
                        }
                    }
                }
            }
            let done = matches!(socket.state(), tcp::State::Closed | tcp::State::TimeWait)
                || (conn.app_closed && conn.to_app.is_none() && !socket.is_active());
            if done {
                let conn = self.conns.swap_remove(i);
                self.sockets.remove(conn.socket);
                moved = true;
            } else {
                i += 1;
            }
        }
        moved
    }

    fn pair(&self, socket: SocketHandle, peer: String) -> (TunnelStream, Conn) {
        let (to_app, rx) = mpsc::channel(CHUNKS);
        let (tx, from_app) = mpsc::channel(CHUNKS);
        let stream = TunnelStream {
            rx,
            unread: Vec::new(),
            at: 0,
            tx: PollSender::new(tx),
            wake: self.wake.clone(),
        };
        let conn = Conn {
            socket,
            peer,
            to_app: Some(to_app),
            from_app,
            pending: None,
            app_closed: false,
        };
        (stream, conn)
    }
}

/// A new key pair: the private key, and its public key.
pub fn new_key() -> (StaticSecret, [u8; 32]) {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("the system's random generator");
    let private = StaticSecret::from(bytes);
    let public = PublicKey::from(&private).to_bytes();
    (private, public)
}

/// A key as WireGuard's files write it (base64).
pub fn key_text(key: &[u8; 32]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(key)
}

/// A key written as WireGuard's files write it.
pub fn key_of(text: &str) -> Option<[u8; 32]> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(text.trim())
        .ok()?;
    bytes.try_into().ok()
}

/// A UDP socket on `port` of every address of the machine, IPv6 and IPv4 together where the system allows it.
pub fn bind_udp(port: u16) -> std::io::Result<UdpSocket> {
    use socket2::{Domain, Protocol, Socket, Type};
    // The port of IPv4 taken by another program: said, rather than a socket of IPv6 that would never see IPv4 (Windows
    // lets both be bound).
    drop(std::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port))?);
    let dual = (|| -> std::io::Result<std::net::UdpSocket> {
        let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))?;
        socket.set_only_v6(false)?;
        socket.bind(&SocketAddr::from((std::net::Ipv6Addr::UNSPECIFIED, port)).into())?;
        Ok(socket.into())
    })();
    let std_socket = match dual {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => return Err(e),
        Err(_) => std::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port))?,
    };
    std_socket.set_nonblocking(true)?;
    tune_udp(&std_socket);
    UdpSocket::from_std(std_socket)
}

/// Room in a UDP socket's buffers for a TCP window's burst (the system's default, small under Windows, dropped
/// datagrams under load, and TCP waited its retransmission). Best effort.
pub fn tune_udp(socket: &std::net::UdpSocket) {
    let s = socket2::SockRef::from(socket);
    let _ = s.set_recv_buffer_size(UDP_BUFFER);
    let _ = s.set_send_buffer_size(UDP_BUFFER);
}
