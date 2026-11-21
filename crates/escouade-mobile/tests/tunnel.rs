//! The phone's peer joins a peer of test (the core's place) and carries a request, then the API's WebSocket, through
//! the local proxy; an unknown phone, a wrong key of the core, and a request without the proxy's secret get nothing.

mod common;

use std::io::{Read, Write};
use std::net::{TcpStream, UdpSocket};
use std::sync::atomic::Ordering;
use std::time::Duration;

use common::{eventually, phone_config, EchoBackend, HttpBackend, Keys, TestPeer};
use escouade_mobile::tunnel::{Tunnel, TunnelState};
use tungstenite::Message;

const UP: Duration = Duration::from_secs(10);

fn read_all(mut s: TcpStream) -> String {
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let mut out = Vec::new();
    let _ = s.read_to_end(&mut out);
    String::from_utf8_lossy(&out).into_owned()
}

#[test]
fn a_request_goes_through_the_proxy_and_the_tunnel() {
    let backend = HttpBackend::start();
    let phone = Keys::new();
    let core = TestPeer::start(&phone.public, backend.addr);
    let tunnel = Tunnel::start(&phone_config(&phone, &core.keys.public, core.endpoint)).unwrap();
    assert!(tunnel.wait_up(UP), "{:?}", tunnel.status());

    let status = tunnel.status();
    assert_eq!(status.state, TunnelState::Up);
    assert!(status.proxy_url.starts_with("http://127.0.0.1:"));
    assert!(status.proxy_url.ends_with(tunnel.secret()));
    assert_eq!(
        status.websocket_url,
        format!("ws://{}/{}/remote", tunnel.proxy_addr(), tunnel.secret())
    );
    assert!(status.last_handshake_secs.is_some());
    assert_eq!(core.handshakes.load(Ordering::SeqCst), 1);

    let mut s = TcpStream::connect(tunnel.proxy_addr()).unwrap();
    write!(
        s,
        "GET /{}/hello?x=1 HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
        tunnel.secret()
    )
    .unwrap();
    let answer = read_all(s);
    assert!(answer.starts_with("HTTP/1.1 200 OK\r\n"), "{answer:?}");
    assert!(answer.ends_with("\r\n\r\nbonjour"), "{answer:?}");
    // The API saw the path without the secret.
    assert_eq!(
        *backend.lines.lock().unwrap(),
        vec!["GET /hello?x=1 HTTP/1.1".to_string()]
    );
    let status = tunnel.status();
    assert!(status.tx_bytes > 0 && status.rx_bytes > 0, "{status:?}");
    // The connection, over, is let go on both sides.
    assert!(eventually(Duration::from_secs(5), || tunnel
        .status()
        .connections
        == 0));
}

#[test]
fn the_websocket_of_the_api_goes_through_two_at_once() {
    let backend = EchoBackend::start();
    let phone = Keys::new();
    let core = TestPeer::start(&phone.public, backend.addr);
    let tunnel = Tunnel::start(&phone_config(&phone, &core.keys.public, core.endpoint)).unwrap();
    assert!(tunnel.wait_up(UP));
    let url = tunnel.status().websocket_url;

    let open = || {
        let stream = TcpStream::connect(tunnel.proxy_addr()).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(20)))
            .unwrap();
        tungstenite::client(url.as_str(), stream).unwrap().0
    };
    let mut a = open();
    let mut b = open();
    assert!(eventually(Duration::from_secs(5), || tunnel
        .status()
        .connections
        == 2));

    a.send(Message::text(r#"{"id":1,"cmd":"subscribe"}"#))
        .unwrap();
    b.send(Message::text(r#"{"id":1,"cmd":"space_glance"}"#))
        .unwrap();
    assert_eq!(
        a.read().unwrap(),
        Message::text(r#"{"id":1,"cmd":"subscribe"}"#)
    );
    assert_eq!(
        b.read().unwrap(),
        Message::text(r#"{"id":1,"cmd":"space_glance"}"#)
    );

    // Larger than every buffer on the way (smoltcp's, the proxy's), both ways at once: the windows hold.
    let big: Vec<u8> = (0..700_000u32).map(|i| (i * 7 % 251) as u8).collect();
    a.send(Message::binary(big.clone())).unwrap();
    b.send(Message::text("é".repeat(100_000))).unwrap();
    assert_eq!(a.read().unwrap().into_data().as_ref(), big.as_slice());
    assert_eq!(
        b.read().unwrap().into_text().unwrap().as_str(),
        "é".repeat(100_000)
    );

    a.close(None).unwrap();
    while a.read().is_ok() {}
    assert_eq!(
        *backend.paths.lock().unwrap(),
        vec!["/remote".to_string(), "/remote".to_string()]
    );
    assert_eq!(core.accepted.load(Ordering::SeqCst), 2);
    b.send(Message::text("encore là")).unwrap();
    assert_eq!(b.read().unwrap(), Message::text("encore là"));
}

#[test]
fn the_proxy_refuses_a_request_without_its_secret() {
    let backend = HttpBackend::start();
    let phone = Keys::new();
    let core = TestPeer::start(&phone.public, backend.addr);
    let tunnel = Tunnel::start(&phone_config(&phone, &core.keys.public, core.endpoint)).unwrap();
    assert!(tunnel.wait_up(UP));

    for first in [
        "GET /remote HTTP/1.1".to_string(),
        format!("GET /{}x/remote HTTP/1.1", tunnel.secret()),
    ] {
        let mut s = TcpStream::connect(tunnel.proxy_addr()).unwrap();
        write!(s, "{first}\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
        let answer = read_all(s);
        assert!(
            answer.starts_with("HTTP/1.1 403 Forbidden\r\n"),
            "{answer:?}"
        );
    }
    // A connection that says nothing and leaves is let go too.
    drop(TcpStream::connect(tunnel.proxy_addr()).unwrap());
    assert!(eventually(Duration::from_secs(5), || tunnel
        .status()
        .connections
        == 0));
    assert_eq!(
        core.accepted.load(Ordering::SeqCst),
        0,
        "nothing went into the tunnel"
    );
    assert!(backend.lines.lock().unwrap().is_empty());
}

#[test]
fn an_unknown_phone_or_a_wrong_core_key_never_gets_in() {
    let backend = HttpBackend::start();
    let phone = Keys::new();
    let stranger = Keys::new();
    // The core knows another phone.
    let core = TestPeer::start(&stranger.public, backend.addr);
    let unknown = Tunnel::start(&phone_config(&phone, &core.keys.public, core.endpoint)).unwrap();
    // The phone has the wrong key of the core.
    let core2 = TestPeer::start(&phone.public, backend.addr);
    let wrong = Tunnel::start(&phone_config(&phone, &Keys::new().public, core2.endpoint)).unwrap();

    for t in [&unknown, &wrong] {
        let mut s = TcpStream::connect(t.proxy_addr()).unwrap();
        write!(s, "GET /{}/hello HTTP/1.1\r\n\r\n", t.secret()).unwrap();
    }
    assert!(!unknown.wait_up(Duration::from_secs(2)));
    assert!(!wrong.wait_up(Duration::from_millis(100)));
    for t in [&unknown, &wrong] {
        let status = t.status();
        assert_eq!(status.state, TunnelState::Connecting);
        assert_eq!(status.last_handshake_secs, None);
        assert_eq!(status.rx_bytes, 0);
    }
    for c in [&core, &core2] {
        assert_eq!(c.handshakes.load(Ordering::SeqCst), 0);
        assert_eq!(c.accepted.load(Ordering::SeqCst), 0);
    }
    assert!(backend.lines.lock().unwrap().is_empty());
}

#[test]
fn the_tunnel_comes_up_when_the_core_comes_late() {
    // The port is taken back by the core a little later: until then, the phone's handshakes meet a closed port.
    let port = UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    let phone = Keys::new();
    let core_keys = Keys::new();
    let tunnel = Tunnel::start(&phone_config(&phone, &core_keys.public, port)).unwrap();
    assert!(!tunnel.wait_up(Duration::from_millis(700)));
    assert_eq!(
        tunnel.status().state,
        TunnelState::Connecting,
        "a closed port is not an error"
    );

    let backend = HttpBackend::start();
    let core = TestPeer::start_on(
        UdpSocket::bind(port).unwrap(),
        core_keys,
        &phone.public,
        backend.addr,
    );
    // WireGuard tries again every 5 s.
    assert!(
        tunnel.wait_up(Duration::from_secs(15)),
        "{:?}",
        tunnel.status()
    );
    let mut s = TcpStream::connect(tunnel.proxy_addr()).unwrap();
    write!(s, "GET /{}/late HTTP/1.1\r\n\r\n", tunnel.secret()).unwrap();
    assert!(read_all(s).ends_with("bonjour"));
    drop(core);
}

#[test]
fn stop_closes_the_proxy_and_its_connections() {
    let backend = EchoBackend::start();
    let phone = Keys::new();
    let core = TestPeer::start(&phone.public, backend.addr);
    let tunnel = Tunnel::start(&phone_config(&phone, &core.keys.public, core.endpoint)).unwrap();
    assert!(tunnel.wait_up(UP));
    let stream = TcpStream::connect(tunnel.proxy_addr()).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let (mut ws, _) = tungstenite::client(tunnel.status().websocket_url.as_str(), stream).unwrap();
    ws.send(Message::text("ping")).unwrap();
    assert_eq!(ws.read().unwrap(), Message::text("ping"));

    tunnel.stop();
    tunnel.stop();
    assert_eq!(tunnel.status().state, TunnelState::Stopped);
    // The WebSocket learns it at once, and the proxy is gone.
    assert!(ws.read().is_err());
    assert!(TcpStream::connect_timeout(&tunnel.proxy_addr(), Duration::from_secs(2)).is_err());
}
