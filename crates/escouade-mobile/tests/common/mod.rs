//! A WireGuard peer of test, in the core's place (MO-2 makes the real one): `boringtun` as the responder, `smoltcp`
//! listening on the API's address in the tunnel, each connection carried to a real TCP server of the test (the
//! « API »). Written apart from the crate's engine, plainly (a loop that sleeps 1 ms), so that the tests do not check
//! the engine against itself. The keys are drawn by each test.

#![allow(dead_code)]

use std::collections::VecDeque;
use std::io::{ErrorKind, Read, Write};
use std::net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use boringtun::noise::{Tunn, TunnResult};
use boringtun::x25519::{PublicKey, StaticSecret};
use escouade_mobile::keys::{encode_key, random_key};
use escouade_mobile::tunnel::TunnelConfig;
use smoltcp::iface::{Config, Interface, SocketHandle, SocketSet};
use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::socket::tcp;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr};

pub const CORE_IP: Ipv4Addr = Ipv4Addr::new(10, 77, 0, 1);
pub const PHONE_IP: &str = "10.77.0.2/24";
pub const API_PORT: u16 = 7878;

/// A key pair, drawn now.
pub struct Keys {
    pub private: [u8; 32],
    pub public: [u8; 32],
}

impl Keys {
    pub fn new() -> Self {
        let private = *random_key();
        let public = PublicKey::from(&StaticSecret::from(private)).to_bytes();
        Keys { private, public }
    }
}

/// The phone's configuration for a peer at `endpoint`.
pub fn phone_config(phone: &Keys, core_public: &[u8; 32], endpoint: SocketAddr) -> TunnelConfig {
    TunnelConfig {
        private_key: encode_key(&phone.private),
        peer_public_key: encode_key(core_public),
        preshared_key: None,
        endpoint: endpoint.to_string(),
        address: PHONE_IP.into(),
        api: format!("{CORE_IP}:{API_PORT}"),
        keepalive_secs: None,
    }
}

#[derive(Default)]
struct Queues {
    rx: VecDeque<Vec<u8>>,
    tx: VecDeque<Vec<u8>>,
}

struct Rx(Vec<u8>);
struct Tx<'a>(&'a mut VecDeque<Vec<u8>>);

impl RxToken for Rx {
    fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R {
        f(&self.0)
    }
}

impl TxToken for Tx<'_> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        let mut p = vec![0u8; len];
        let r = f(&mut p);
        self.0.push_back(p);
        r
    }
}

impl Device for Queues {
    type RxToken<'a> = Rx;
    type TxToken<'a> = Tx<'a>;
    fn receive(&mut self, _: smoltcp::time::Instant) -> Option<(Rx, Tx<'_>)> {
        let p = self.rx.pop_front()?;
        Some((Rx(p), Tx(&mut self.tx)))
    }
    fn transmit(&mut self, _: smoltcp::time::Instant) -> Option<Tx<'_>> {
        Some(Tx(&mut self.tx))
    }
    fn capabilities(&self) -> DeviceCapabilities {
        let mut c = DeviceCapabilities::default();
        c.medium = Medium::Ip;
        c.max_transmission_unit = 1420;
        c
    }
}

struct Bridge {
    socket: SocketHandle,
    backend: TcpStream,
    to_backend: Vec<u8>,
    backend_eof: bool,
    backend_shut: bool,
}

/// The core's peer, while it lives.
pub struct TestPeer {
    pub endpoint: SocketAddr,
    pub keys: Keys,
    /// Connections that came out of the tunnel to the API.
    pub accepted: Arc<AtomicUsize>,
    /// Handshakes answered.
    pub handshakes: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl TestPeer {
    /// A peer on a port of its own, that lets in `phone` only, its API carried to `backend`.
    pub fn start(phone: &[u8; 32], backend: SocketAddr) -> TestPeer {
        Self::start_on(
            UdpSocket::bind("127.0.0.1:0").unwrap(),
            Keys::new(),
            phone,
            backend,
        )
    }

    pub fn start_on(udp: UdpSocket, keys: Keys, phone: &[u8; 32], backend: SocketAddr) -> TestPeer {
        let endpoint = udp.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let accepted = Arc::new(AtomicUsize::new(0));
        let handshakes = Arc::new(AtomicUsize::new(0));
        let tunn = Tunn::new(
            StaticSecret::from(keys.private),
            PublicKey::from(*phone),
            None,
            None,
            7,
            None,
        );
        let (s, a, h) = (stop.clone(), accepted.clone(), handshakes.clone());
        let thread = std::thread::spawn(move || run(udp, tunn, backend, s, a, h));
        TestPeer {
            endpoint,
            keys,
            accepted,
            handshakes,
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for TestPeer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn listening() -> tcp::Socket<'static> {
    let mut s = tcp::Socket::new(
        tcp::SocketBuffer::new(vec![0; 65536]),
        tcp::SocketBuffer::new(vec![0; 65536]),
    );
    s.listen(API_PORT).unwrap();
    s
}

fn run(
    udp: UdpSocket,
    mut tunn: Tunn,
    backend: SocketAddr,
    stop: Arc<AtomicBool>,
    accepted: Arc<AtomicUsize>,
    handshakes: Arc<AtomicUsize>,
) {
    udp.set_nonblocking(true).unwrap();
    let mut dev = Queues::default();
    let mut iface = Interface::new(
        Config::new(HardwareAddress::Ip),
        &mut dev,
        smoltcp::time::Instant::now(),
    );
    iface.update_ip_addrs(|a| a.push(IpCidr::new(IpAddress::Ipv4(CORE_IP), 24)).unwrap());
    let mut sockets = SocketSet::new(Vec::new());
    let mut listener = sockets.add(listening());
    let mut bridges: Vec<Bridge> = Vec::new();
    let mut phone: Option<SocketAddr> = None;
    let mut inbuf = vec![0u8; 65536];
    let mut out = vec![0u8; 65536];
    let mut chunk = vec![0u8; 16384];
    while !stop.load(Ordering::SeqCst) {
        let mut idle = true;
        // The network.
        loop {
            let (n, from) = match udp.recv_from(&mut inbuf) {
                Ok(r) => r,
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(_) => continue,
            };
            idle = false;
            let mut r = tunn.decapsulate(Some(from.ip()), &inbuf[..n], &mut out);
            loop {
                match r {
                    TunnResult::WriteToNetwork(p) => {
                        phone = Some(from);
                        if p.len() == 92 {
                            // A handshake response is 92 bytes.
                            handshakes.fetch_add(1, Ordering::SeqCst);
                        }
                        let _ = udp.send_to(p, from);
                        r = tunn.decapsulate(None, &[], &mut out);
                    }
                    TunnResult::WriteToTunnelV4(p, _) => {
                        phone = Some(from);
                        dev.rx.push_back(p.to_vec());
                        break;
                    }
                    _ => break,
                }
            }
        }
        if let (TunnResult::WriteToNetwork(p), Some(to)) = (tunn.update_timers(&mut out), phone) {
            let _ = udp.send_to(p, to);
        }
        iface.poll(smoltcp::time::Instant::now(), &mut dev, &mut sockets);
        // A connection came in: carry it to the backend, listen again.
        if sockets.get::<tcp::Socket>(listener).state() != tcp::State::Listen {
            let backend = TcpStream::connect(backend).unwrap();
            backend.set_nonblocking(true).unwrap();
            backend.set_nodelay(true).unwrap();
            accepted.fetch_add(1, Ordering::SeqCst);
            bridges.push(Bridge {
                socket: listener,
                backend,
                to_backend: Vec::new(),
                backend_eof: false,
                backend_shut: false,
            });
            listener = sockets.add(listening());
        }
        for b in &mut bridges {
            let s = sockets.get_mut::<tcp::Socket>(b.socket);
            while s.can_recv() {
                let n = s.recv_slice(&mut chunk).unwrap_or(0);
                if n == 0 {
                    break;
                }
                b.to_backend.extend_from_slice(&chunk[..n]);
            }
            while !b.to_backend.is_empty() {
                match b.backend.write(&b.to_backend) {
                    Ok(n) if n > 0 => {
                        b.to_backend.drain(..n);
                    }
                    _ => break,
                }
            }
            if !b.backend_shut
                && b.to_backend.is_empty()
                && !s.may_recv()
                && s.state() != tcp::State::SynReceived
            {
                let _ = b.backend.shutdown(Shutdown::Write);
                b.backend_shut = true;
            }
            while !b.backend_eof && s.can_send() {
                let room = s.send_capacity() - s.send_queue();
                if room == 0 {
                    break;
                }
                match b.backend.read(&mut chunk[..room.min(16384)]) {
                    Ok(0) => b.backend_eof = true,
                    Ok(n) => {
                        s.send_slice(&chunk[..n]).unwrap();
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                    Err(_) => b.backend_eof = true,
                }
            }
            if b.backend_eof && s.send_queue() == 0 && s.may_send() {
                s.close();
            }
        }
        bridges.retain(|b| {
            let closed = sockets.get::<tcp::Socket>(b.socket).state() == tcp::State::Closed;
            if closed {
                sockets.remove(b.socket);
            }
            !closed
        });
        iface.poll(smoltcp::time::Instant::now(), &mut dev, &mut sockets);
        while let Some(p) = dev.tx.pop_front() {
            idle = false;
            if let (TunnResult::WriteToNetwork(d), Some(to)) =
                (tunn.encapsulate(&p, &mut out), phone)
            {
                let _ = udp.send_to(d, to);
            }
        }
        // Busy, at once again; idle, a short sleep (1 ms asks for up to 15 under Windows).
        if idle {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

/// An HTTP server of test: answers `bonjour` to each request, and notes their first lines.
pub struct HttpBackend {
    pub addr: SocketAddr,
    pub lines: Arc<Mutex<Vec<String>>>,
}

impl HttpBackend {
    pub fn start() -> HttpBackend {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let lines = Arc::new(Mutex::new(Vec::new()));
        let seen = lines.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let seen = seen.clone();
                std::thread::spawn(move || {
                    stream.set_nonblocking(false).unwrap();
                    let mut req = Vec::new();
                    let mut b = [0u8; 1024];
                    while !req.windows(4).any(|w| w == b"\r\n\r\n") {
                        match stream.read(&mut b) {
                            Ok(0) | Err(_) => return,
                            Ok(n) => req.extend_from_slice(&b[..n]),
                        }
                    }
                    let text = String::from_utf8_lossy(&req);
                    seen.lock()
                        .unwrap()
                        .push(text.lines().next().unwrap_or_default().to_string());
                    let body = "bonjour";
                    let _ = write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                });
            }
        });
        HttpBackend { addr, lines }
    }
}

/// A WebSocket server of test: sends back each message, and notes the paths asked for.
pub struct EchoBackend {
    pub addr: SocketAddr,
    pub paths: Arc<Mutex<Vec<String>>>,
}

impl EchoBackend {
    pub fn start() -> EchoBackend {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let paths = Arc::new(Mutex::new(Vec::new()));
        let seen = paths.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let seen = seen.clone();
                std::thread::spawn(move || {
                    stream.set_nonblocking(false).unwrap();
                    // tungstenite's callback type: its error is an HTTP response, large by design.
                    #[allow(clippy::result_large_err)]
                    let callback =
                        |req: &tungstenite::handshake::server::Request,
                         resp: tungstenite::handshake::server::Response| {
                            seen.lock().unwrap().push(req.uri().to_string());
                            Ok(resp)
                        };
                    let Ok(mut ws) = tungstenite::accept_hdr(stream, callback) else {
                        return;
                    };
                    while let Ok(msg) = ws.read() {
                        if msg.is_close() {
                            break;
                        }
                        if (msg.is_text() || msg.is_binary()) && ws.send(msg).is_err() {
                            break;
                        }
                    }
                });
            }
        });
        EchoBackend { addr, paths }
    }
}

/// Waits for `f` to hold, up to `timeout`.
pub fn eventually(timeout: Duration, mut f: impl FnMut() -> bool) -> bool {
    let until = Instant::now() + timeout;
    while Instant::now() < until {
        if f() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    f()
}
