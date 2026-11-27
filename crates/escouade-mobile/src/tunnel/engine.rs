//! The tunnel's thread: the UDP socket and WireGuard on one side, smoltcp and the proxy's connections on the other.
//!
//! Each turn: read what the UDP socket holds and open it (WireGuard), give the packets to smoltcp; accept the
//! proxy's connections; read the local connections into their tunnel sockets; let smoltcp work; write what it
//! received to the local connections; seal what it sends and write it to the UDP socket. `mio` wakes the thread when
//! a socket is ready, and smoltcp and WireGuard's timers say how long it may sleep. A connection's buffers are
//! bounded on both sides: a slow WebView slows the tunnel's TCP window, never the phone's memory.

use std::collections::HashMap;
use std::io::{self, ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr, SocketAddrV4};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use boringtun::noise::{Tunn, TunnResult};
use boringtun::x25519::{PublicKey, StaticSecret};
use mio::net::{TcpListener, TcpStream, UdpSocket};
use mio::{Events, Interest, Poll, Token, Waker};
use smoltcp::iface::{Config, Interface, SocketHandle, SocketSet};
use smoltcp::socket::tcp;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr};

use super::device::Queues;
use super::gate::{self, Gate};
use super::{Plan, Shared, TunnelError};

const UDP: Token = Token(0);
const LISTENER: Token = Token(1);
const WAKE: Token = Token(2);
const FIRST_CONN: usize = 16;
/// WireGuard's timers want a call every 250 ms or so (boringtun's own device ticks at that pace).
const TIMER_TICK: Duration = Duration::from_millis(250);
/// The tunnel sockets' buffers, each way.
const TCP_BUFFER: usize = 128 * 1024;
/// What a connection holds at most on our side, each way, before it stops reading.
const PENDING_MAX: usize = 64 * 1024;
/// The proxy's connections at a time (the app opens one WebSocket per space, a few fetches).
const MAX_CONNECTIONS: usize = 64;
/// How long a connection has to pass the gate (its first line, with the secret): past it, it is closed (SR1-08 of the
/// 2.3's security review). Another app of the phone that opens connections and says nothing holds no place for long.
const GATE_WAIT: Duration = Duration::from_secs(5);
/// A connection into the tunnel with no answer for this long is given up (the WebView then sees it close).
const TCP_TIMEOUT: Duration = Duration::from_secs(30);
/// A WireGuard session older than this is no longer valid (REJECT_AFTER_TIME).
const SESSION_LIFE: Duration = Duration::from_secs(180);
/// The first local port of the tunnel's connections.
const FIRST_PORT: u16 = 49152;

struct Conn {
    stream: TcpStream,
    /// When it was accepted: it has `GATE_WAIT` to pass the gate.
    opened: Instant,
    /// The tunnel's socket, once the gate let the connection in.
    socket: Option<SocketHandle>,
    /// What was read before the first line passed the gate.
    gate: Option<Vec<u8>>,
    to_tunnel: Vec<u8>,
    to_local: Vec<u8>,
    readable: bool,
    writable: bool,
    local_eof: bool,
    fin_sent: bool,
    local_shut: bool,
    /// Refused by the gate: the refusal is written, then the connection closes.
    refused: bool,
    dead: bool,
}

pub(super) struct Engine {
    poll: Poll,
    udp: UdpSocket,
    listener: TcpListener,
    tunn: Box<Tunn>,
    iface: Interface,
    device: Queues,
    sockets: SocketSet<'static>,
    /// Sockets of closed connections, kept until smoltcp has sent their RST or FIN.
    dying: Vec<SocketHandle>,
    conns: HashMap<usize, Conn>,
    next_id: usize,
    next_port: u16,
    endpoint: SocketAddr,
    api: SocketAddrV4,
    secret: String,
    /// The device's token, written in each request (ESC-570).
    token: Option<zeroize::Zeroizing<String>>,
    shared: Arc<Mutex<Shared>>,
    stop: Arc<AtomicBool>,
    inbuf: Vec<u8>,
    outbuf: Vec<u8>,
}

fn io_error(e: io::Error) -> TunnelError {
    TunnelError::Io(e.to_string())
}

fn random_u64() -> u64 {
    let mut b = [0u8; 8];
    getrandom::getrandom(&mut b).expect("the system's randomness");
    u64::from_le_bytes(b)
}

fn smol_now() -> smoltcp::time::Instant {
    smoltcp::time::Instant::now()
}

/// Sends a datagram; one the system cannot take now is dropped, as UDP would (TCP in the tunnel sends it again).
fn send_udp(udp: &UdpSocket, to: SocketAddr, packet: &[u8]) -> io::Result<()> {
    match udp.send_to(packet, to) {
        Ok(_) => Ok(()),
        Err(e) if is_transient(&e) => Ok(()),
        Err(e) => Err(e),
    }
}

/// The errors a socket gives without being broken: nothing to do now, an interrupted call, and the ICMP « port
/// unreachable » that Windows and Linux hand to the next call on a UDP socket while the peer is not there yet.
fn is_transient(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        ErrorKind::WouldBlock
            | ErrorKind::Interrupted
            | ErrorKind::ConnectionRefused
            | ErrorKind::ConnectionReset
    )
}

impl Engine {
    pub(super) fn new(
        plan: Plan,
        secret: String,
        shared: Arc<Mutex<Shared>>,
        stop: Arc<AtomicBool>,
    ) -> Result<(Engine, Arc<Waker>, SocketAddr), TunnelError> {
        let poll = Poll::new().map_err(io_error)?;
        let any: SocketAddr = if plan.endpoint.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        }
        .parse()
        .expect("an address");
        let mut udp = UdpSocket::bind(any).map_err(io_error)?;
        poll.registry()
            .register(&mut udp, UDP, Interest::READABLE)
            .map_err(io_error)?;
        let mut listener =
            TcpListener::bind("127.0.0.1:0".parse().expect("an address")).map_err(io_error)?;
        let proxy = listener.local_addr().map_err(io_error)?;
        poll.registry()
            .register(&mut listener, LISTENER, Interest::READABLE)
            .map_err(io_error)?;
        let waker = Arc::new(Waker::new(poll.registry(), WAKE).map_err(io_error)?);

        let tunn = Box::new(Tunn::new(
            StaticSecret::from(*plan.private),
            PublicKey::from(plan.peer),
            plan.preshared.map(|k| *k),
            plan.keepalive,
            (random_u64() as u32) & 0x00ff_ffff,
            None,
        ));

        let mut device = Queues::default();
        let mut config = Config::new(HardwareAddress::Ip);
        config.random_seed = random_u64();
        let mut iface = Interface::new(config, &mut device, smol_now());
        iface.update_ip_addrs(|addrs| {
            addrs
                .push(IpCidr::new(IpAddress::Ipv4(plan.address), plan.prefix))
                .expect("one address fits");
        });
        // The API may sit outside the phone's prefix: everything goes to the core's peer.
        iface
            .routes_mut()
            .add_default_ipv4_route(*plan.api.ip())
            .expect("one route fits");

        let engine = Engine {
            poll,
            udp,
            listener,
            tunn,
            iface,
            device,
            sockets: SocketSet::new(Vec::new()),
            dying: Vec::new(),
            conns: HashMap::new(),
            next_id: FIRST_CONN,
            next_port: FIRST_PORT + (random_u64() % 4096) as u16,
            endpoint: plan.endpoint,
            api: plan.api,
            secret,
            token: plan.token.clone(),
            shared,
            stop,
            inbuf: vec![0u8; 65536],
            outbuf: vec![0u8; 65536],
        };
        Ok((engine, waker, proxy))
    }

    pub(super) fn run(mut self) {
        let outcome = self.run_loop();
        let mut s = self.shared.lock().unwrap_or_else(|e| e.into_inner());
        s.up = false;
        s.connections = 0;
        s.done = true;
        if let Err(e) = outcome {
            if !self.stop.load(Ordering::SeqCst) {
                s.error = Some(e.to_string());
            }
        }
        // Dropping self closes the UDP socket, the listener and every connection.
    }

    fn run_loop(&mut self) -> io::Result<()> {
        let mut events = Events::with_capacity(256);
        self.handshake()?;
        let mut next_timer = Instant::now() + TIMER_TICK;
        let mut busy = false;
        loop {
            let delay = self
                .iface
                .poll_delay(smol_now(), &self.sockets)
                .map(|d| Duration::from_micros(d.total_micros()))
                .unwrap_or(TIMER_TICK);
            let timeout = if busy {
                Duration::ZERO
            } else {
                delay.min(next_timer.saturating_duration_since(Instant::now()))
            };
            match self.poll.poll(&mut events, Some(timeout)) {
                Ok(()) => {}
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
            if self.stop.load(Ordering::SeqCst) {
                return Ok(());
            }
            let (mut udp_ready, mut accept) = (false, false);
            for ev in events.iter() {
                match ev.token() {
                    UDP => udp_ready = true,
                    LISTENER => accept = true,
                    WAKE => {}
                    Token(id) => {
                        if let Some(c) = self.conns.get_mut(&id) {
                            c.readable |= ev.is_readable() || ev.is_read_closed() || ev.is_error();
                            c.writable |= ev.is_writable() || ev.is_write_closed() || ev.is_error();
                        }
                    }
                }
            }
            if udp_ready {
                self.read_udp()?;
            }
            if Instant::now() >= next_timer {
                self.timers()?;
                next_timer = Instant::now() + TIMER_TICK;
            }
            if accept {
                self.accept();
            }
            self.expire_gates();
            let mut moved = self.local_to_tunnel();
            self.iface
                .poll(smol_now(), &mut self.device, &mut self.sockets);
            moved |= self.tunnel_to_local();
            self.iface
                .poll(smol_now(), &mut self.device, &mut self.sockets);
            self.flush_tx()?;
            self.reap();
            self.publish();
            busy = moved;
        }
    }

    fn handshake(&mut self) -> io::Result<()> {
        if let TunnResult::WriteToNetwork(p) = self
            .tunn
            .format_handshake_initiation(&mut self.outbuf, false)
        {
            send_udp(&self.udp, self.endpoint, p)?;
        }
        Ok(())
    }

    /// Opens the datagrams waiting on the UDP socket.
    fn read_udp(&mut self) -> io::Result<()> {
        loop {
            let (n, from) = match self.udp.recv_from(&mut self.inbuf) {
                Ok(r) => r,
                Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(()),
                Err(e) if is_transient(&e) => continue,
                Err(e) => return Err(e),
            };
            // Only the core's peer: anything else would fail WireGuard's checks anyway.
            if from != self.endpoint {
                continue;
            }
            let mut result =
                self.tunn
                    .decapsulate(Some(from.ip()), &self.inbuf[..n], &mut self.outbuf);
            loop {
                match result {
                    TunnResult::WriteToNetwork(p) => {
                        send_udp(&self.udp, self.endpoint, p)?;
                        // What WireGuard queued while there was no session goes now.
                        result = self.tunn.decapsulate(None, &[], &mut self.outbuf);
                    }
                    TunnResult::WriteToTunnelV4(p, src) => {
                        // AllowedIPs: the API's address alone.
                        if src == *self.api.ip() {
                            self.device.rx.push_back(p.to_vec());
                        }
                        break;
                    }
                    TunnResult::WriteToTunnelV6(..) | TunnResult::Done | TunnResult::Err(_) => {
                        break
                    }
                }
            }
        }
    }

    fn timers(&mut self) -> io::Result<()> {
        if let TunnResult::WriteToNetwork(p) = self.tunn.update_timers(&mut self.outbuf) {
            send_udp(&self.udp, self.endpoint, p)?;
        }
        Ok(())
    }

    fn accept(&mut self) {
        loop {
            match self.listener.accept() {
                Ok((mut stream, _)) => {
                    // At the ceiling, the oldest connection still at the gate makes room (SR1-08): connections that
                    // say nothing never keep the WebView out. Without one, the new connection is closed.
                    if self.conns.len() >= MAX_CONNECTIONS && !self.evict_one_at_the_gate() {
                        continue;
                    }
                    let _ = stream.set_nodelay(true);
                    let id = self.next_id;
                    self.next_id += 1;
                    if self
                        .poll
                        .registry()
                        .register(
                            &mut stream,
                            Token(id),
                            Interest::READABLE | Interest::WRITABLE,
                        )
                        .is_err()
                    {
                        continue;
                    }
                    self.conns.insert(
                        id,
                        Conn {
                            stream,
                            opened: Instant::now(),
                            socket: None,
                            gate: Some(Vec::new()),
                            to_tunnel: Vec::new(),
                            to_local: Vec::new(),
                            readable: true,
                            writable: true,
                            local_eof: false,
                            fin_sent: false,
                            local_shut: false,
                            refused: false,
                            dead: false,
                        },
                    );
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => return,
                // A connection gone before it was accepted, or the system short of descriptors: try at the next
                // readiness.
                Err(e)
                    if e.kind() == ErrorKind::Interrupted
                        || e.kind() == ErrorKind::ConnectionAborted =>
                {
                    continue
                }
                Err(_) => return,
            }
        }
    }

    /// The oldest connection that has not passed the gate, closed at once to make room. Says whether there was one.
    fn evict_one_at_the_gate(&mut self) -> bool {
        let Some(id) = oldest_at_the_gate(
            self.conns
                .iter()
                .map(|(&id, c)| (id, c.opened, c.gate.is_some() && !c.refused)),
        ) else {
            return false;
        };
        if let Some(mut c) = self.conns.remove(&id) {
            let _ = self.poll.registry().deregister(&mut c.stream);
            let _ = c.stream.shutdown(Shutdown::Both);
        }
        true
    }

    /// The connections that did not pass the gate in `GATE_WAIT`, closed (at the next reap).
    fn expire_gates(&mut self) {
        let now = Instant::now();
        for c in self.conns.values_mut() {
            if c.gate.is_some() && !c.refused && now.duration_since(c.opened) >= GATE_WAIT {
                c.dead = true;
            }
        }
    }

    /// Reads the local connections into their tunnel sockets. Says whether bytes moved.
    fn local_to_tunnel(&mut self) -> bool {
        let mut moved = false;
        let mut chunk = [0u8; 16 * 1024];
        for c in self.conns.values_mut() {
            while c.readable && !c.local_eof && !c.refused && !c.dead {
                let room = match &c.gate {
                    Some(g) => gate::MAX_LINE.saturating_sub(g.len()),
                    None => PENDING_MAX.saturating_sub(c.to_tunnel.len()),
                };
                if room == 0 {
                    break;
                }
                let want = room.min(chunk.len());
                match c.stream.read(&mut chunk[..want]) {
                    Ok(0) => c.local_eof = true,
                    Ok(n) => {
                        moved = true;
                        match &mut c.gate {
                            Some(g) => g.extend_from_slice(&chunk[..n]),
                            None => c.to_tunnel.extend_from_slice(&chunk[..n]),
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => c.readable = false,
                    Err(e) if e.kind() == ErrorKind::Interrupted => {}
                    Err(_) => c.dead = true,
                }
                if let Some(g) = &c.gate {
                    match gate::check(g, &self.secret, self.token.as_deref().map(String::as_str)) {
                        Gate::NeedMore if c.local_eof => c.dead = true,
                        Gate::NeedMore => {}
                        Gate::Pass(bytes) => {
                            c.gate = None;
                            c.to_tunnel = bytes;
                            let mut socket = tcp::Socket::new(
                                tcp::SocketBuffer::new(vec![0u8; TCP_BUFFER]),
                                tcp::SocketBuffer::new(vec![0u8; TCP_BUFFER]),
                            );
                            socket.set_nagle_enabled(false);
                            socket.set_timeout(Some(smoltcp::time::Duration::from_secs(
                                TCP_TIMEOUT.as_secs(),
                            )));
                            let port = self.next_port;
                            self.next_port = if port == u16::MAX {
                                FIRST_PORT
                            } else {
                                port + 1
                            };
                            let remote = (IpAddress::Ipv4(*self.api.ip()), self.api.port());
                            if socket.connect(self.iface.context(), remote, port).is_ok() {
                                c.socket = Some(self.sockets.add(socket));
                            } else {
                                c.dead = true;
                            }
                        }
                        Gate::Refuse => {
                            c.gate = None;
                            c.refused = true;
                            c.to_local = gate::REFUSAL.to_vec();
                        }
                    }
                }
            }
            let Some(h) = c.socket else { continue };
            let s = self.sockets.get_mut::<tcp::Socket>(h);
            if !c.to_tunnel.is_empty() && s.can_send() {
                if let Ok(n) = s.send_slice(&c.to_tunnel) {
                    c.to_tunnel.drain(..n);
                    moved |= n > 0;
                }
            }
            // The WebView is done sending: a FIN, once all is in and the connection is up.
            if c.local_eof && c.to_tunnel.is_empty() && !c.fin_sent && s.may_send() {
                s.close();
                c.fin_sent = true;
            }
        }
        moved
    }

    /// Writes what the tunnel sockets received to the local connections. Says whether bytes moved.
    fn tunnel_to_local(&mut self) -> bool {
        let mut moved = false;
        let mut chunk = [0u8; 16 * 1024];
        for c in self.conns.values_mut() {
            let mut remote_done = false;
            if let Some(h) = c.socket {
                let s = self.sockets.get_mut::<tcp::Socket>(h);
                while c.to_local.len() < PENDING_MAX && s.can_recv() {
                    let want = (PENDING_MAX - c.to_local.len()).min(chunk.len());
                    match s.recv_slice(&mut chunk[..want]) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            c.to_local.extend_from_slice(&chunk[..n]);
                            moved = true;
                        }
                    }
                }
                remote_done = !s.can_recv()
                    && matches!(
                        s.state(),
                        tcp::State::CloseWait
                            | tcp::State::LastAck
                            | tcp::State::Closing
                            | tcp::State::TimeWait
                            | tcp::State::Closed
                    );
            }
            while c.writable && !c.to_local.is_empty() && !c.dead {
                match c.stream.write(&c.to_local) {
                    Ok(0) => c.dead = true,
                    Ok(n) => {
                        c.to_local.drain(..n);
                        moved = true;
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => c.writable = false,
                    Err(e) if e.kind() == ErrorKind::Interrupted => {}
                    Err(_) => c.dead = true,
                }
            }
            // The core is done sending: the WebView reads the end of the stream.
            if remote_done && c.to_local.is_empty() && !c.local_shut {
                let _ = c.stream.shutdown(Shutdown::Write);
                c.local_shut = true;
            }
        }
        moved
    }

    /// Seals smoltcp's packets and sends them to the core's peer.
    fn flush_tx(&mut self) -> io::Result<()> {
        while let Some(packet) = self.device.tx.pop_front() {
            if let TunnResult::WriteToNetwork(p) = self.tunn.encapsulate(&packet, &mut self.outbuf)
            {
                send_udp(&self.udp, self.endpoint, p)?;
            }
        }
        Ok(())
    }

    /// Drops the connections that are over, and the sockets smoltcp is done with.
    fn reap(&mut self) {
        let mut over = Vec::new();
        for (&id, c) in &self.conns {
            let socket_closed = c.socket.is_some_and(|h| {
                let s = self.sockets.get::<tcp::Socket>(h);
                matches!(s.state(), tcp::State::Closed | tcp::State::TimeWait) && !s.can_recv()
            });
            let done = c.dead
                || (c.refused && c.to_local.is_empty())
                || (socket_closed && c.to_local.is_empty())
                || (c.local_eof && c.fin_sent && c.local_shut && c.to_local.is_empty());
            if done {
                over.push(id);
            }
        }
        for id in over {
            if let Some(mut c) = self.conns.remove(&id) {
                let _ = self.poll.registry().deregister(&mut c.stream);
                if let Some(h) = c.socket {
                    let s = self.sockets.get_mut::<tcp::Socket>(h);
                    if c.dead {
                        s.abort();
                    } else if s.may_send() {
                        s.close();
                    }
                    self.dying.push(h);
                }
            }
        }
        let sockets = &mut self.sockets;
        self.dying.retain(|&h| {
            let gone = matches!(
                sockets.get::<tcp::Socket>(h).state(),
                tcp::State::Closed | tcp::State::TimeWait
            );
            if gone {
                sockets.remove(h);
            }
            !gone
        });
    }

    fn publish(&mut self) {
        let (since, tx, rx, _, _) = self.tunn.stats();
        let mut s = self.shared.lock().unwrap_or_else(|e| e.into_inner());
        s.up = since.is_some_and(|d| d < SESSION_LIFE);
        s.last_handshake_secs = since.map(|d| d.as_secs());
        s.tx_bytes = tx as u64;
        s.rx_bytes = rx as u64;
        s.connections = self.conns.len() as u32;
    }
}

/// Of the connections (`id`, when accepted, still at the gate), the oldest still at the gate.
fn oldest_at_the_gate(conns: impl Iterator<Item = (usize, Instant, bool)>) -> Option<usize> {
    conns
        .filter(|&(_, _, at_gate)| at_gate)
        .min_by_key(|&(id, opened, _)| (opened, id))
        .map(|(id, _, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_oldest_connection_at_the_gate_makes_room_never_one_let_in() {
        let t = Instant::now();
        let later = t + Duration::from_millis(10);
        // 16 passed the gate (the WebView's), 17 and 18 still at it: 17, the oldest of them, goes.
        let conns = [(16, t, false), (18, later, true), (17, t, true)];
        assert_eq!(oldest_at_the_gate(conns.into_iter()), Some(17));
        // All let in: none goes, the new one is closed.
        assert_eq!(
            oldest_at_the_gate([(16, t, false), (17, later, false)].into_iter()),
            None
        );
    }

    #[test]
    fn transient_errors_are_not_fatal() {
        for kind in [
            ErrorKind::WouldBlock,
            ErrorKind::Interrupted,
            ErrorKind::ConnectionRefused,
            ErrorKind::ConnectionReset,
        ] {
            assert!(is_transient(&io::Error::from(kind)));
        }
        assert!(!is_transient(&io::Error::from(ErrorKind::PermissionDenied)));
        // An address the socket's family cannot reach is an error of the configuration, said in the status.
        let udp = UdpSocket::bind("127.0.0.1:0".parse().unwrap()).unwrap();
        assert!(send_udp(&udp, "[::1]:9".parse().unwrap(), b"x").is_err());
    }
}
