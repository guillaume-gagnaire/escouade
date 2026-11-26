//! The phone's tunnel (ESC-567, MO-2): a WireGuard peer of test, in Rust (the same stack the phone's side will use),
//! against the core without a window: the handshake, the API reached in the tunnel and refused out of it, an unknown
//! key unanswered, the key turned, the public address read again, the policy that turns it off, Tailscale. Every
//! address is the loopback: no test reaches the user's box (its gateway and STUN server are fakes here).

use crate::headless::Headless;
use crate::headless_tests::{data, wait};
use crate::phone::{nat, stack, tailscale, API_PORT, TUNNEL_ADDRESS};
use crate::remoteapi::proto::{self, ToClient, ToServer};
use crate::remoteapi::server::{Limits, Listen, RemoteApi};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A free UDP port of the loopback (taken, then let go).
/// Free in TCP too: Tailscale's API takes the same number (another test of the suite may hold it).
fn free_udp() -> u16 {
    loop {
        let port = std::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        if std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_ok() {
            return port;
        }
    }
}

/// A core without a window, its API served as `listen` says, the phone's access attached to it, the network's
/// probes off; and a device « Téléphone » and its token.
async fn core(name: &str, listen: Listen) -> (Headless, Arc<RemoteApi>, String) {
    let (_dir, data) = data(name);
    let h = Headless::start(data).unwrap();
    let api = h.serve_remote_on(listen, Limits::default()).await.unwrap();
    h.core.phone.set_probes(nat::Probes::off());
    h.core
        .phone
        .set_tailscale_probe(tailscale::Probe::Given(None));
    h.core.phone.attach(&api);
    let (_, token) = api.devices().add("Téléphone").unwrap();
    (h, api, token)
}

/// The access turned on, on a free port: where its UDP socket is.
async fn turn_on(h: &Headless) -> SocketAddr {
    let port = free_udp();
    h.core
        .phone
        .set(&h.core, Some(true), Some(port), None)
        .await
        .unwrap();
    assert!(h.core.phone.running());
    SocketAddr::from((Ipv4Addr::LOCALHOST, port))
}

/// The core's public key, as the phone's config gives it.
fn core_key(h: &Headless) -> [u8; 32] {
    let key = crate::phone::stack::key_of(
        &std::fs::read_to_string(h.core.data.root().join(crate::phone::FILE))
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|v| v["publicKey"].as_str().map(str::to_string))
            .expect("the tunnel's public key"),
    );
    key.expect("a key of 32 bytes")
}

/// A phone of test: its own tunnel on a socket of the loopback, the core as its one peer.
struct Phone {
    tunnel: stack::Tunnel,
    public: [u8; 32],
}

impl Phone {
    fn new(core: SocketAddr, core_public: [u8; 32], address: Ipv4Addr) -> Self {
        let (private, public) = stack::new_key();
        Self::with_key(core, core_public, address, private, public)
    }

    fn with_key(
        core: SocketAddr,
        core_public: [u8; 32],
        address: Ipv4Addr,
        private: boringtun::x25519::StaticSecret,
        public: [u8; 32],
    ) -> Self {
        let std_socket = std::net::UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        std_socket.set_nonblocking(true).unwrap();
        stack::tune_udp(&std_socket);
        let socket = tokio::net::UdpSocket::from_std(std_socket).unwrap();
        let tunnel = stack::Tunnel::start(
            socket,
            stack::Config {
                private,
                address,
                prefix: 24,
                listen: None,
                peers: vec![stack::PeerConfig {
                    id: "core".into(),
                    public: core_public,
                    address: TUNNEL_ADDRESS,
                    endpoint: Some(core),
                    keepalive: Some(25),
                }],
            },
            None,
        )
        .unwrap();
        Self { tunnel, public }
    }

    fn key(&self) -> String {
        stack::key_text(&self.public)
    }

    /// A TCP connection to the API in the tunnel, in `secs` at most.
    async fn reach(&self, secs: u64) -> Option<stack::TunnelStream> {
        tokio::time::timeout(
            Duration::from_secs(secs),
            self.tunnel
                .connect(SocketAddrV4::new(TUNNEL_ADDRESS, API_PORT)),
        )
        .await
        .ok()?
        .ok()
    }
}

type Ws = tokio_tungstenite::WebSocketStream<stack::TunnelStream>;

/// The WebSocket opened on `stream`, its `Host` and `Origin` as given: the socket, or the HTTP status of the refusal.
async fn open(stream: stack::TunnelStream, host: &str, origin: Option<&str>) -> Result<Ws, u16> {
    let mut req = format!("ws://{TUNNEL_ADDRESS}{}", proto::PATH)
        .into_client_request()
        .unwrap();
    req.headers_mut().insert("host", host.parse().unwrap());
    if let Some(o) = origin {
        req.headers_mut().insert("origin", o.parse().unwrap());
    }
    tokio::time::timeout(
        Duration::from_secs(20),
        tokio_tungstenite::client_async(req, stream),
    )
    .await
    .expect("the WebSocket's handshake in 20 s")
    .map(|(ws, _)| ws)
    .map_err(|e| match e {
        tokio_tungstenite::tungstenite::Error::Http(resp) => resp.status().as_u16(),
        other => panic!("{other}"),
    })
}

async fn send(ws: &mut Ws, message: &ToServer) {
    ws.send(Message::text(serde_json::to_string(message).unwrap()))
        .await
        .unwrap();
}

async fn next(ws: &mut Ws) -> Option<ToClient> {
    loop {
        match tokio::time::timeout(Duration::from_secs(30), ws.next()).await {
            Ok(Some(Ok(Message::Text(t)))) => return serde_json::from_str(&t).ok(),
            Ok(Some(Ok(Message::Close(_)))) | Ok(None) | Ok(Some(Err(_))) => return None,
            Ok(Some(Ok(_))) => continue,
            Err(_) => panic!("nothing came in 30 s"),
        }
    }
}

/// The handshake of the API through the tunnel: its first answer.
async fn hello(ws: &mut Ws, token: &str) -> ToClient {
    send(
        ws,
        &ToServer::Hello {
            version: VERSION.into(),
            token: token.into(),
            session: None,
        },
    )
    .await;
    next(ws).await.expect("an answer to the handshake")
}

fn refused_code(told: &ToClient) -> Option<&str> {
    match told {
        ToClient::Refused { code, .. } => Some(code),
        _ => None,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_phone_shakes_hands_with_the_core_and_reaches_the_api_in_the_tunnel_only() {
    // The desktop's way: the API has no socket of its own, it is served in the tunnel alone.
    let (h, api, token) = core("phone-handshake", Listen::Nothing).await;
    assert_eq!(api.addr().port(), 0, "no TCP port of the machine");
    let udp = turn_on(&h).await;
    let phone = Phone::new(udp, core_key(&h), Ipv4Addr::new(10, 77, 0, 2));
    let peer = h
        .core
        .phone
        .add_peer("Pixel de test", &phone.key())
        .unwrap();
    assert_eq!(peer.address, Ipv4Addr::new(10, 77, 0, 2));

    let stream = phone
        .reach(20)
        .await
        .expect("the API reached in the tunnel");
    let tunnel = h.core.phone.tunnel().unwrap();
    assert!(tunnel.counts().handshakes.load(Ordering::Relaxed) >= 1);
    let mut ws = open(
        stream,
        &TUNNEL_ADDRESS.to_string(),
        Some("capacitor://localhost"),
    )
    .await
    .expect("the WebSocket opens in the tunnel");
    let welcome = hello(&mut ws, &token).await;
    assert!(matches!(welcome, ToClient::Welcome { .. }), "{welcome:?}");
    send(
        &mut ws,
        &ToServer::Call {
            id: 1,
            cmd: "phone_status".into(),
            args: Default::default(),
            body: None,
            headers: Default::default(),
        },
    )
    .await;
    let reply = loop {
        match next(&mut ws).await.expect("a reply") {
            ToClient::Reply { id: 1, ok, .. } => break ok.unwrap(),
            _ => continue,
        }
    };
    assert_eq!(reply["state"], "on");
    assert_eq!(reply["peers"][0]["name"], "Pixel de test");
    assert!(reply["peers"][0]["sinceHandshake"].is_u64(), "{reply}");
    // The peer, linked to nothing yet (paired by code in MO-4), sees its last handshake.
    let status = h.core.phone.status(&h.core).await;
    assert_eq!(status.handshakes, 1);
    assert!(status.bound.is_some());
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn out_of_the_tunnel_nothing_answers_and_in_it_a_host_or_a_page_of_elsewhere_is_refused() {
    let (h, _api, token) = core("phone-outside", Listen::Nothing).await;
    let udp = turn_on(&h).await;
    // No TCP port where the tunnel listens: the API is on no address of the machine.
    assert!(
        tokio::net::TcpStream::connect(SocketAddr::from((Ipv4Addr::LOCALHOST, udp.port())))
            .await
            .is_err()
    );
    // A datagram that is not WireGuard: no answer, counted.
    let probe = tokio::net::UdpSocket::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    probe.send_to(b"GET / HTTP/1.1\r\n\r\n", udp).await.unwrap();
    let mut buf = [0u8; 256];
    assert!(
        tokio::time::timeout(Duration::from_secs(1), probe.recv_from(&mut buf))
            .await
            .is_err(),
        "a scan sees nothing"
    );
    let tunnel = h.core.phone.tunnel().unwrap();
    wait("the bad datagram counted", || {
        tunnel.counts().invalid.load(Ordering::Relaxed) >= 1
    })
    .await;
    // In the tunnel: a Host of the loopback (DNS rebinding through the tunnel) and a web page are refused.
    let phone = Phone::new(udp, core_key(&h), Ipv4Addr::new(10, 77, 0, 2));
    h.core.phone.add_peer("Téléphone", &phone.key()).unwrap();
    let s = phone.reach(20).await.unwrap();
    assert_eq!(open(s, "127.0.0.1", None).await.err(), Some(403));
    let s = phone.reach(20).await.unwrap();
    assert_eq!(
        open(s, &TUNNEL_ADDRESS.to_string(), Some("https://example.com"))
            .await
            .err(),
        Some(403)
    );
    // The device token is asked in the tunnel too.
    let s = phone.reach(20).await.unwrap();
    let mut ws = open(s, &TUNNEL_ADDRESS.to_string(), None).await.unwrap();
    let told = hello(&mut ws, "not-a-token").await;
    assert_eq!(refused_code(&told), Some("token"), "{told:?}");
    // The token of the device is taken there.
    let s = phone.reach(20).await.unwrap();
    let mut ws = open(s, &TUNNEL_ADDRESS.to_string(), None).await.unwrap();
    assert!(matches!(
        hello(&mut ws, &token).await,
        ToClient::Welcome { .. }
    ));
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn on_the_loopback_the_origin_of_the_phones_app_is_refused() {
    // A page of the machine that says it is Capacitor's: refused on the loopback, the phone's origins being for the
    // tunnel and Tailscale alone.
    let (h, api, _token) = core("phone-loopback-origin", Listen::Port(0)).await;
    let mut req = format!("ws://{}{}", api.addr(), proto::PATH)
        .into_client_request()
        .unwrap();
    req.headers_mut()
        .insert("origin", "capacitor://localhost".parse().unwrap());
    let tcp = tokio::net::TcpStream::connect(api.addr()).await.unwrap();
    let refused = tokio_tungstenite::client_async(req, tcp).await;
    assert!(
        matches!(refused, Err(tokio_tungstenite::tungstenite::Error::Http(ref r)) if r.status() == 403),
        "{refused:?}"
    );
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_phone_of_an_unknown_key_gets_no_answer() {
    let (h, _api, _token) = core("phone-unknown", Listen::Nothing).await;
    let udp = turn_on(&h).await;
    let known = Phone::new(udp, core_key(&h), Ipv4Addr::new(10, 77, 0, 2));
    h.core.phone.add_peer("Téléphone", &known.key()).unwrap();
    // A key the core does not know, at the same address.
    let stranger = Phone::new(udp, core_key(&h), Ipv4Addr::new(10, 77, 0, 2));
    assert!(
        stranger.reach(3).await.is_none(),
        "no session for a stranger"
    );
    let tunnel = h.core.phone.tunnel().unwrap();
    assert!(tunnel.counts().unknown.load(Ordering::Relaxed) >= 1);
    assert_eq!(tunnel.counts().handshakes.load(Ordering::Relaxed), 0);
    // The known phone still gets in; then removed, it no longer does.
    assert!(known.reach(20).await.is_some());
    let id = h.core.phone.peers()[0].id.clone();
    assert!(h.core.phone.remove_peer(&id).unwrap());
    // Its session gone with it: neither its packets nor a new handshake are answered.
    assert!(known.reach(3).await.is_none());
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_key_turned_a_phone_of_the_old_key_is_no_longer_answered() {
    let (h, _api, token) = core("phone-rotate", Listen::Nothing).await;
    let udp = turn_on(&h).await;
    let old = core_key(&h);
    let (private, public) = stack::new_key();
    h.core
        .phone
        .add_peer("Téléphone", &stack::key_text(&public))
        .unwrap();
    let before = Phone::with_key(
        udp,
        old,
        Ipv4Addr::new(10, 77, 0, 2),
        private.clone(),
        public,
    );
    assert!(before.reach(20).await.is_some());
    drop(before);
    let new = h.core.phone.rotate_key().unwrap();
    let new = stack::key_of(&new).unwrap();
    assert_ne!(new, old);
    assert_eq!(core_key(&h), new, "the new public key kept");
    // The same phone with the old key of the core: its handshake is to a key no longer there.
    let stale = Phone::with_key(
        udp,
        old,
        Ipv4Addr::new(10, 77, 0, 2),
        private.clone(),
        public,
    );
    assert!(stale.reach(3).await.is_none());
    // With the new one (its QR code made again), it gets in.
    let fresh = Phone::with_key(udp, new, Ipv4Addr::new(10, 77, 0, 2), private, public);
    let s = fresh.reach(20).await.expect("in with the new key");
    let mut ws = open(s, &TUNNEL_ADDRESS.to_string(), None).await.unwrap();
    assert!(matches!(
        hello(&mut ws, &token).await,
        ToClient::Welcome { .. }
    ));
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_peer_paired_as_a_device_takes_that_devices_token_alone() {
    let (h, api, token) = core("phone-bound", Listen::Nothing).await;
    let udp = turn_on(&h).await;
    let phone = Phone::new(udp, core_key(&h), Ipv4Addr::new(10, 77, 0, 2));
    h.core.phone.add_peer("Téléphone", &phone.key()).unwrap();
    // Paired through the tunnel (the QR code's code, MO-4): the peer belongs to the new device.
    let code = crate::remoteapi::pairing::create(&h.core.data, crate::model::now_ms()).unwrap();
    let s = phone.reach(20).await.unwrap();
    let mut ws = open(s, &TUNNEL_ADDRESS.to_string(), None).await.unwrap();
    send(
        &mut ws,
        &ToServer::Pair {
            code: code.text.clone(),
            name: "Pixel".into(),
        },
    )
    .await;
    let Some(ToClient::Paired { device, token: own }) = next(&mut ws).await else {
        panic!("paired")
    };
    assert_eq!(
        h.core.phone.peers()[0].device.as_deref(),
        Some(device.id.as_str())
    );
    // Another device's token through this peer: refused; its own: taken.
    let s = phone.reach(20).await.unwrap();
    let mut ws = open(s, &TUNNEL_ADDRESS.to_string(), None).await.unwrap();
    assert_eq!(refused_code(&hello(&mut ws, &token).await), Some("token"));
    let s = phone.reach(20).await.unwrap();
    let mut ws = open(s, &TUNNEL_ADDRESS.to_string(), None).await.unwrap();
    assert!(matches!(
        hello(&mut ws, &own).await,
        ToClient::Welcome { .. }
    ));
    // The device revoked: its peer goes with it.
    api.revoke(&device.id).unwrap();
    wait("the peer of the revoked device removed", || {
        h.core.phone.peers().is_empty()
    })
    .await;
    h.stop();
}

/// A fake STUN server on the loopback: it says the address `seen` holds.
fn fake_stun(seen: Arc<parking_lot::Mutex<SocketAddrV4>>) -> SocketAddr {
    let socket = std::net::UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let addr = socket.local_addr().unwrap();
    std::thread::spawn(move || {
        let mut buf = [0u8; 512];
        while let Ok((n, from)) = socket.recv_from(&mut buf) {
            if n < 20 {
                continue;
            }
            let at = *seen.lock();
            let magic: u32 = 0x2112_A442;
            let mut out = vec![0x01, 0x01, 0, 12];
            out.extend_from_slice(&buf[4..20]);
            out.extend_from_slice(&[0x00, 0x20, 0, 8, 0, 1]);
            out.extend_from_slice(&(at.port() ^ (magic >> 16) as u16).to_be_bytes());
            out.extend_from_slice(&(u32::from(*at.ip()) ^ magic).to_be_bytes());
            let _ = socket.send_to(&out, from);
        }
    });
    addr
}

/// A fake box speaking NAT-PMP on the loopback, whose address on the Internet is `wan`: it maps what it is asked.
fn fake_natpmp(wan: Ipv4Addr, mapped: Arc<parking_lot::Mutex<Vec<(u16, u32)>>>) -> SocketAddr {
    let socket = std::net::UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let addr = socket.local_addr().unwrap();
    std::thread::spawn(move || {
        let mut buf = [0u8; 64];
        while let Ok((n, from)) = socket.recv_from(&mut buf) {
            let mut out = vec![0u8, 128 + buf[1], 0, 0, 0, 0, 0, 1];
            match (n, buf[1]) {
                (2, 0) => out.extend_from_slice(&wan.octets()),
                (12, 1) => {
                    let internal = u16::from_be_bytes([buf[4], buf[5]]);
                    let lifetime = u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]);
                    mapped.lock().push((internal, lifetime));
                    out.extend_from_slice(&buf[4..6]);
                    out.extend_from_slice(&buf[4..6]);
                    out.extend_from_slice(&lifetime.to_be_bytes());
                }
                _ => continue,
            }
            let _ = socket.send_to(&out, from);
        }
    });
    addr
}

#[tokio::test(flavor = "multi_thread")]
async fn the_box_opens_the_port_and_the_public_address_read_again_says_it_changed_and_a_cgnat() {
    let (h, _api, _token) = core("phone-net", Listen::Nothing).await;
    let seen = Arc::new(parking_lot::Mutex::new(SocketAddrV4::new(
        Ipv4Addr::new(203, 0, 113, 7),
        40000,
    )));
    let stun = fake_stun(seen.clone());
    let mapped = Arc::new(parking_lot::Mutex::new(Vec::new()));
    // The box's own address is in the operators' shared space: a CGNAT.
    let natpmp = fake_natpmp(Ipv4Addr::new(100, 72, 3, 9), mapped.clone());
    h.core.phone.set_probes(nat::Probes {
        upnp: None,
        natpmp: Some(natpmp),
        natpmp_on: true,
        stun: vec![stun.to_string()],
        wait: Duration::from_secs(2),
        local: Some(Ipv4Addr::new(192, 168, 1, 20).into()),
    });
    let udp = turn_on(&h).await;
    wait("the first look at the network", || {
        h.core.phone.checked_at().is_some()
    })
    .await;
    let status = h.core.phone.status(&h.core).await;
    let net = status.net.clone().unwrap();
    assert_eq!(net.opened.as_ref().map(|o| o.by), Some(nat::Opener::Natpmp));
    assert_eq!(
        net.opened.as_ref().map(|o| o.external_port),
        Some(udp.port())
    );
    assert_eq!(net.wan, Some(Ipv4Addr::new(100, 72, 3, 9).into()));
    assert_eq!(net.public, Some(Ipv4Addr::new(203, 0, 113, 7).into()));
    assert!(net.cgnat, "{net:?}");
    assert!(!net.direct);
    assert_eq!(mapped.lock().first(), Some(&(udp.port(), nat::LEASE)));
    let first = status.public.clone().unwrap();
    assert_eq!(first.address, "203.0.113.7");
    assert!(first.changed_at.is_none());
    // The public address changes (a new lease of the operator): read again, it is said.
    *seen.lock() = SocketAddrV4::new(Ipv4Addr::new(198, 51, 100, 23), 40000);
    h.core.phone.check().await;
    let public = h.core.phone.status(&h.core).await.public.unwrap();
    assert_eq!(public.address, "198.51.100.23");
    assert_eq!(public.previous.as_deref(), Some("203.0.113.7"));
    assert!(public.changed_at.is_some());
    // Kept in the file: read again after a restart.
    let saved = std::fs::read_to_string(h.core.data.root().join(crate::phone::FILE)).unwrap();
    assert!(saved.contains("198.51.100.23"), "{saved}");
    // Off: the mapping removed from the box (a lifetime of 0).
    h.core
        .phone
        .set(&h.core, Some(false), None, None)
        .await
        .unwrap();
    assert!(!h.core.phone.running());
    assert_eq!(mapped.lock().last(), Some(&(udp.port(), 0)));
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn stun_turned_off_sends_nothing_to_its_servers_and_the_box_still_says_the_address() {
    let (h, _api, _token) = core("phone-stun-off", Listen::Nothing).await;
    // A STUN server that only listens: what it receives is read after.
    let stun = std::net::UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    stun.set_nonblocking(true).unwrap();
    let mapped = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let natpmp = fake_natpmp(Ipv4Addr::new(198, 51, 100, 40), mapped.clone());
    h.core.phone.set_probes(nat::Probes {
        upnp: None,
        natpmp: Some(natpmp),
        natpmp_on: true,
        stun: vec![stun.local_addr().unwrap().to_string()],
        wait: Duration::from_millis(500),
        local: Some(Ipv4Addr::new(192, 168, 1, 20).into()),
    });
    assert!(h.core.phone.status(&h.core).await.stun_on, "on by default");
    h.core.phone.set_stun(false).unwrap();
    let udp = turn_on(&h).await;
    wait("the first look at the network", || {
        h.core.phone.checked_at().is_some()
    })
    .await;
    h.core.phone.check().await;
    let mut buf = [0u8; 64];
    assert_eq!(
        stun.recv_from(&mut buf).map_err(|e| e.kind()).err(),
        Some(std::io::ErrorKind::WouldBlock),
        "nothing sent to the STUN server"
    );
    let status = h.core.phone.status(&h.core).await;
    assert!(!status.stun_on);
    let net = status.net.unwrap();
    assert_eq!(net.public, None);
    // The box still opens the port, and its own address on the Internet is the one kept.
    assert_eq!(
        net.opened.as_ref().map(|o| o.external_port),
        Some(udp.port())
    );
    assert_eq!(status.public.unwrap().address, "198.51.100.40");
    // Kept in the file.
    let saved = std::fs::read_to_string(h.core.data.root().join(crate::phone::FILE)).unwrap();
    assert!(saved.contains("\"stun\": false"), "{saved}");
    // On again: asked again at the next look.
    h.core.phone.set_stun(true).unwrap();
    h.core.phone.check().await;
    assert!(stun.recv_from(&mut buf).is_ok(), "asked once on again");
    h.stop();
}

#[test]
fn a_stun_answer_reads_back_and_the_cgnat_is_told_from_what_was_seen() {
    let id = [3u8; 12];
    let magic: u32 = 0x2112_A442;
    let mut answer = vec![0x01, 0x01, 0, 12, 0x21, 0x12, 0xA4, 0x42];
    answer.extend_from_slice(&id);
    answer.extend_from_slice(&[0x00, 0x20, 0, 8, 0, 1]);
    answer.extend_from_slice(&(51820u16 ^ (magic >> 16) as u16).to_be_bytes());
    answer.extend_from_slice(&(u32::from(Ipv4Addr::new(203, 0, 113, 7)) ^ magic).to_be_bytes());
    assert_eq!(
        nat::stun_answer(&answer, &id),
        Some(SocketAddr::from(([203, 0, 113, 7], 51820)))
    );
    assert_eq!(
        nat::stun_answer(&answer, &[4u8; 12]),
        None,
        "another request's"
    );
    // The box says the public address: no CGNAT.
    let mut net = nat::NetState {
        local: Some(Ipv4Addr::new(192, 168, 1, 20).into()),
        wan: Some(Ipv4Addr::new(203, 0, 113, 7).into()),
        public: Some(Ipv4Addr::new(203, 0, 113, 7).into()),
        ..Default::default()
    };
    nat::classify(&mut net);
    assert!(!net.cgnat && !net.direct);
    // The box's address is private (a box behind a box): a double NAT, said like a CGNAT.
    net.wan = Some(Ipv4Addr::new(192, 168, 0, 2).into());
    nat::classify(&mut net);
    assert!(net.cgnat);
    // A VPS: the machine has the public address itself.
    let mut vps = nat::NetState {
        local: Some(Ipv4Addr::new(203, 0, 113, 50).into()),
        public: Some(Ipv4Addr::new(203, 0, 113, 50).into()),
        ..Default::default()
    };
    nat::classify(&mut vps);
    assert!(vps.direct && !vps.cgnat);
    assert!(nat::shared(Ipv4Addr::new(100, 64, 0, 1)));
    assert!(!nat::shared(Ipv4Addr::new(100, 128, 0, 1)));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_policy_turns_the_tunnel_off_at_once_and_refuses_to_turn_it_on() {
    let (h, _api, _token) = core("phone-policy", Listen::Nothing).await;
    let udp = turn_on(&h).await;
    let phone = Phone::new(udp, core_key(&h), Ipv4Addr::new(10, 77, 0, 2));
    h.core.phone.add_peer("Téléphone", &phone.key()).unwrap();
    assert!(phone.reach(20).await.is_some());
    h.core.set_managed(crate::locks::Managed {
        values: [(
            escouade_policy::list::PHONE.to_string(),
            (Value::Bool(false), "test".to_string()),
        )]
        .into(),
        warnings: Vec::new(),
    });
    assert!(!h.core.phone.running(), "off at once");
    let later = Phone::new(udp, core_key(&h), Ipv4Addr::new(10, 77, 0, 3));
    assert!(later.reach(3).await.is_none());
    let status = h.core.phone.status(&h.core).await;
    assert!(status.denied.unwrap().contains("servers.phone"));
    let err = h
        .core
        .phone
        .set(&h.core, Some(true), None, None)
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("servers.phone"), "{err}");
    // The remote API denied: the phone with it.
    h.core.set_managed(crate::locks::Managed {
        values: [(
            escouade_policy::list::REMOTE_API.to_string(),
            (Value::Bool(false), "test".to_string()),
        )]
        .into(),
        warnings: Vec::new(),
    });
    assert!(h.core.locks.read().check_phone().is_err());
    // Lifted: on again, as it was left.
    h.core.set_managed(crate::locks::Managed::default());
    wait("on again", || h.core.phone.running()).await;
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn tailscale_found_the_api_is_served_on_its_address_to_the_tailnet_alone() {
    let (h, _api, _token) = core("phone-tailscale", Listen::Nothing).await;
    // The tests' Tailscale is the loopback: a connection from it is not of a tailnet, so it is dropped.
    h.core
        .phone
        .set_tailscale_probe(tailscale::Probe::Given(Some(tailscale::Found {
            address: Ipv4Addr::LOCALHOST,
            name: Some("poste.tail1234.ts.net".into()),
        })));
    let udp = turn_on(&h).await;
    h.core.phone.check().await;
    // Found, but not served until the user turns it on (SR1-05).
    let status = h.core.phone.status(&h.core).await;
    assert!(status.tailscale.is_some());
    assert!(!status.tailscale_on);
    assert!(status.tailscale_served.is_none());
    h.core
        .phone
        .set(&h.core, None, None, Some(true))
        .await
        .unwrap();
    let status = h.core.phone.status(&h.core).await;
    assert_eq!(
        status.tailscale.as_ref().unwrap().address,
        Ipv4Addr::LOCALHOST
    );
    let served = status
        .tailscale_served
        .clone()
        .expect("served on Tailscale");
    assert_eq!(served, format!("127.0.0.1:{}", udp.port()));
    let mut tcp = tokio::net::TcpStream::connect(served.as_str())
        .await
        .unwrap();
    let mut buf = [0u8; 16];
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let _ = tcp.write_all(b"GET /remote HTTP/1.1\r\n\r\n").await;
    let read = tokio::time::timeout(Duration::from_secs(5), tcp.read(&mut buf)).await;
    assert!(matches!(read, Ok(Ok(0)) | Ok(Err(_))), "closed: {read:?}");
    assert!(tailscale::tailnet(Ipv4Addr::new(100, 101, 102, 103).into()));
    assert!(!tailscale::tailnet(Ipv4Addr::LOCALHOST.into()));
    // Turned off in Réglages › Téléphone: no longer served there.
    h.core
        .phone
        .set(&h.core, None, None, Some(false))
        .await
        .unwrap();
    assert!(h
        .core
        .phone
        .status(&h.core)
        .await
        .tailscale_served
        .is_none());
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_port_already_taken_is_said_and_the_access_stays_off() {
    let (h, _api, _token) = core("phone-port-taken", Listen::Nothing).await;
    let taken = std::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).unwrap();
    let port = taken.local_addr().unwrap().port();
    let err = h
        .core
        .phone
        .set(&h.core, Some(true), Some(port), None)
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains(&port.to_string()), "{err}");
    let status = h.core.phone.status(&h.core).await;
    assert_eq!(status.state, crate::phone::State::Error);
    assert!(!h.core.phone.running());
    drop(taken);
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_stack_carries_megabytes_each_way_in_order() {
    // Two tunnels of the stack alone, face to face: the bytes cross whole and in order, the slow reader holding the
    // writer back (the channels' bounds, then TCP's window).
    let (core_private, core_public) = stack::new_key();
    let (phone_private, phone_public) = stack::new_key();
    let std_udp = std::net::UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    std_udp.set_nonblocking(true).unwrap();
    stack::tune_udp(&std_udp);
    let core_udp = tokio::net::UdpSocket::from_std(std_udp).unwrap();
    let core_addr = core_udp.local_addr().unwrap();
    let (tx, mut accepted) = tokio::sync::mpsc::channel(4);
    let core = stack::Tunnel::start(
        core_udp,
        stack::Config {
            private: core_private,
            address: TUNNEL_ADDRESS,
            prefix: 24,
            listen: Some(API_PORT),
            peers: vec![stack::PeerConfig {
                id: "phone".into(),
                public: phone_public,
                address: Ipv4Addr::new(10, 77, 0, 2),
                endpoint: None,
                keepalive: None,
            }],
        },
        Some(tx),
    )
    .unwrap();
    let phone = Phone::with_key(
        core_addr,
        core_public,
        Ipv4Addr::new(10, 77, 0, 2),
        phone_private,
        phone_public,
    );
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let echo = tokio::spawn(async move {
        let a = accepted.recv().await.unwrap();
        assert_eq!(a.peer, "phone");
        assert_eq!(a.local, SocketAddrV4::new(TUNNEL_ADDRESS, API_PORT));
        let (mut r, mut w) = tokio::io::split(a.stream);
        // Slow at first: the writer is held back meanwhile.
        tokio::time::sleep(Duration::from_millis(300)).await;
        tokio::io::copy(&mut r, &mut w).await.unwrap();
        w.shutdown().await.unwrap();
    });
    let stream = phone.reach(20).await.expect("connected");
    let (mut r, mut w) = tokio::io::split(stream);
    let sent: Vec<u8> = (0..2 * 1024 * 1024u32).map(|i| (i % 251) as u8).collect();
    let expected = sent.clone();
    let writer = tokio::spawn(async move {
        w.write_all(&sent).await.unwrap();
        w.shutdown().await.unwrap();
    });
    let mut got = Vec::new();
    tokio::time::timeout(Duration::from_secs(60), r.read_to_end(&mut got))
        .await
        .expect("all back in 60 s")
        .unwrap();
    writer.await.unwrap();
    echo.await.unwrap();
    assert_eq!(got.len(), expected.len());
    assert!(got == expected, "the bytes in order");
    assert_eq!(core.counts().accepted.load(Ordering::Relaxed), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_phone_added_by_the_servers_command_reaches_the_core_that_runs() {
    // `escouade-server phone add` from another process: the running core reads the file again and takes the peer.
    let (h, _api, token) = core("phone-cli", Listen::Nothing).await;
    let udp = turn_on(&h).await;
    let other = crate::phone::Phone::load(&h.core.data, h.core.secrets.clone());
    let (config, peer) = other
        .add_phone("Pixel de l'essai", Some("vps.example.net"))
        .unwrap();
    assert!(config.contains("[Interface]\nPrivateKey = "), "{config}");
    assert!(config.contains(&format!("Address = {}/32", peer.address)));
    assert!(config.contains("AllowedIPs = 10.77.0.1/32"));
    assert!(config.contains(&format!("Endpoint = vps.example.net:{}", udp.port())));
    assert!(config.contains("PersistentKeepalive = 25"));
    let field = |name: &str| {
        config
            .lines()
            .find_map(|l| l.strip_prefix(&format!("{name} = ")))
            .map(str::to_string)
            .unwrap()
    };
    assert_eq!(stack::key_of(&field("PublicKey")), Some(core_key(&h)));
    assert!(
        h.core.phone.reload_if_changed(),
        "the file changed by another process"
    );
    assert!(!h.core.phone.reload_if_changed(), "read once");
    assert_eq!(h.core.phone.peers().len(), 1);
    let private = stack::key_of(&field("PrivateKey")).unwrap();
    let private = boringtun::x25519::StaticSecret::from(private);
    let public = boringtun::x25519::PublicKey::from(&private).to_bytes();
    let phone = Phone::with_key(udp, core_key(&h), peer.address, private, public);
    let s = phone
        .reach(20)
        .await
        .expect("the peer of the command gets in");
    let mut ws = open(s, &TUNNEL_ADDRESS.to_string(), None).await.unwrap();
    assert!(matches!(
        hello(&mut ws, &token).await,
        ToClient::Welcome { .. }
    ));
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_key_kept_in_the_keychain_is_read_again_never_made_anew() {
    // The file lost (or written before its public key): the key of the keychain stays the tunnel's.
    let (h, _api, _token) = core("phone-key-again", Listen::Nothing).await;
    let _ = turn_on(&h).await;
    let key = core_key(&h);
    h.core
        .phone
        .set(&h.core, Some(false), None, None)
        .await
        .unwrap();
    std::fs::remove_file(h.core.data.root().join(crate::phone::FILE)).unwrap();
    let again = crate::phone::Phone::load(&h.core.data, h.core.secrets.clone());
    again.set_saved(Some(true), None).unwrap();
    assert_eq!(
        again
            .summary()
            .public_key
            .as_deref()
            .and_then(stack::key_of),
        Some(key)
    );
    let (config, _) = again.add_phone("Pixel", None).unwrap();
    assert!(
        config.contains(&format!("PublicKey = {}", stack::key_text(&key))),
        "{config}"
    );
    h.stop();
}

// ---------- « Retirer » a phone, Unpair through a peer (SR1-03, SR1-10) ----------

/// The handshake of `token` on a connection handed to the API as Tailscale's listener hands one: its first answer.
async fn hello_by_tailscale(api: &Arc<RemoteApi>, token: &str) -> ToClient {
    let (ours, theirs) = tokio::io::duplex(1 << 20);
    let names = Arc::new(vec!["100.101.102.103".to_string()]);
    assert!(api.serve_stream(
        Box::new(theirs),
        crate::remoteapi::server::Via::Tailscale(names)
    ));
    let req = format!("ws://100.101.102.103{}", proto::PATH)
        .into_client_request()
        .unwrap();
    let (mut ws, _) = tokio_tungstenite::client_async(req, ours).await.unwrap();
    ws.send(Message::text(
        serde_json::to_string(&ToServer::Hello {
            version: VERSION.into(),
            token: token.into(),
            session: None,
        })
        .unwrap(),
    ))
    .await
    .unwrap();
    loop {
        match tokio::time::timeout(Duration::from_secs(30), ws.next()).await {
            Ok(Some(Ok(Message::Text(t)))) => return serde_json::from_str(&t).unwrap(),
            Ok(Some(Ok(_))) => continue,
            other => panic!("no answer to the handshake: {other:?}"),
        }
    }
}

/// A phone paired through the tunnel (its peer linked to the new device): the peer's id, the device and its token.
async fn paired_phone(h: &Headless, name: &str) -> (Phone, String, String, String) {
    let udp = turn_on(h).await;
    let phone = Phone::new(udp, core_key(h), Ipv4Addr::new(10, 77, 0, 2));
    let peer = h.core.phone.add_peer(name, &phone.key()).unwrap();
    let code = crate::remoteapi::pairing::create(&h.core.data, crate::model::now_ms()).unwrap();
    let s = phone.reach(20).await.unwrap();
    let mut ws = open(s, &TUNNEL_ADDRESS.to_string(), None).await.unwrap();
    send(
        &mut ws,
        &ToServer::Pair {
            code: code.text,
            name: name.into(),
        },
    )
    .await;
    let Some(ToClient::Paired { device, token }) = next(&mut ws).await else {
        panic!("paired")
    };
    (phone, peer.id, device.id, token)
}

#[tokio::test(flavor = "multi_thread")]
async fn removing_a_phone_revokes_its_device_and_its_token_passes_no_other_way() {
    let (h, api, _token) = core("phone-remove-revokes", Listen::Nothing).await;
    let (_phone, peer, device, token) = paired_phone(&h, "Pixel").await;
    assert!(matches!(
        hello_by_tailscale(&api, &token).await,
        ToClient::Welcome { .. }
    ));
    // « Retirer » in Réglages › Téléphone: the peer gone, and its device's token revoked, signed by the window.
    let state = crate::remoteapi::State::of_test(&h.core);
    let removed = crate::commands::phone_remove(state, peer).unwrap();
    assert!(
        removed.removed && removed.revoked && !removed.never_linked,
        "{removed:?}"
    );
    assert!(h.core.phone.peers().is_empty());
    api.devices().refresh();
    assert!(!api.devices().list().iter().any(|d| d.id == device));
    // Its token no longer passes by Tailscale; it is told revoked, by whom.
    match hello_by_tailscale(&api, &token).await {
        ToClient::Refused { code, revoked, .. } => {
            assert_eq!(code, "token");
            let window = tr!("la fenêtre d’Escouade", "Escouade’s window");
            assert_eq!(revoked.and_then(|r| r.by), Some(window));
        }
        other => panic!("a removed phone's token still welcome: {other:?}"),
    }
    // The other device stays.
    assert!(api.devices().list().iter().any(|d| d.name == "Téléphone"));
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn removing_a_phone_never_paired_as_a_device_says_so_and_revokes_nothing() {
    let (h, api, _token) = core("phone-remove-unlinked", Listen::Nothing).await;
    let (_, public) = stack::new_key();
    let peer = h
        .core
        .phone
        .add_peer("iPhone", &stack::key_text(&public))
        .unwrap();
    let state = crate::remoteapi::State::of_test(&h.core);
    let removed = crate::commands::phone_remove(state.clone(), peer.id.clone()).unwrap();
    assert!(
        removed.removed && !removed.revoked && removed.never_linked,
        "{removed:?}"
    );
    assert_eq!(api.devices().list().len(), 1);
    // None any more: nothing done.
    let again = crate::commands::phone_remove(state, peer.id).unwrap();
    assert!(!again.removed && !again.revoked && !again.never_linked);
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn unpair_through_the_peer_of_another_device_is_refused_as_the_handshake_is() {
    let (h, api, other) = core("phone-unpair-bound", Listen::Nothing).await;
    let (phone, _peer, device, own) = paired_phone(&h, "Pixel").await;
    // Another device's token through this phone's peer: refused, that device kept.
    let s = phone.reach(20).await.unwrap();
    let mut ws = open(s, &TUNNEL_ADDRESS.to_string(), None).await.unwrap();
    send(
        &mut ws,
        &ToServer::Unpair {
            token: other.clone(),
        },
    )
    .await;
    assert_eq!(refused_code(&next(&mut ws).await.unwrap()), Some("token"));
    assert!(
        api.devices().find(&other).is_some(),
        "another device unpaired through this peer"
    );
    // Its own: unpaired.
    let s = phone.reach(20).await.unwrap();
    let mut ws = open(s, &TUNNEL_ADDRESS.to_string(), None).await.unwrap();
    send(&mut ws, &ToServer::Unpair { token: own }).await;
    assert!(matches!(next(&mut ws).await, Some(ToClient::Unpaired {})));
    assert!(!api.devices().holds(&device));
    h.stop();
}
