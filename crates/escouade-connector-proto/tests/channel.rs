//! A server and a connector over an in-memory WebSocket: the handshake (pinned keys, versions), the streams and their
//! windows, the life of the channel (in simulated time) and its bounds. Keys are made in each test.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use escouade_connector_proto::channel::CeilingError;
use escouade_connector_proto::frame::Frame;
use escouade_connector_proto::limits::{DEAD_AFTER, INITIAL_WINDOW, PING_EVERY};
use escouade_connector_proto::noise::{Initiator, Transport};
use escouade_connector_proto::transport::ws_config;
use escouade_connector_proto::version::Hello;
use escouade_connector_proto::*;
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::protocol::Role;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

const V: Version = Version::new(2, 3, 0);

async fn sockets() -> (WebSocketStream<DuplexStream>, WebSocketStream<DuplexStream>) {
    let (a, b) = tokio::io::duplex(256 << 10);
    let server = WebSocketStream::from_raw_socket(a, Role::Server, Some(ws_config())).await;
    let client = WebSocketStream::from_raw_socket(b, Role::Client, Some(ws_config())).await;
    (server, client)
}

fn ceiling() -> Ceiling {
    Ceiling {
        rules: vec![
            "gitlab.acme.internal:22".into(),
            "nexus.acme.internal:443".into(),
        ],
    }
}

struct Setup {
    server_limits: Limits,
    connector_limits: Limits,
    server_version: Version,
    connector_version: Version,
}

impl Default for Setup {
    fn default() -> Setup {
        Setup {
            server_limits: Limits::default(),
            connector_limits: Limits::default(),
            server_version: V,
            connector_version: V,
        }
    }
}

type Pair = (
    Result<ServerChannel, AcceptError>,
    Result<ConnectorChannel, ConnectError>,
);

async fn handshake(setup: Setup) -> Pair {
    let (ws_server, ws_client) = sockets().await;
    let server_keys = Keypair::generate();
    let connector_keys = Keypair::generate();
    let pinned = connector_keys.public();
    let server_public = server_keys.public();
    let server = tokio::spawn(async move {
        accept_connector(
            ws_server,
            &server_keys,
            |k| *k == pinned,
            setup.server_version,
            setup.server_limits,
        )
        .await
    });
    let connector = connect_server(
        ws_client,
        &connector_keys,
        &server_public,
        setup.connector_version,
        &ceiling(),
        setup.connector_limits,
    )
    .await;
    (server.await.unwrap(), connector)
}

async fn pair(setup: Setup) -> (ServerChannel, ConnectorChannel) {
    let (s, c) = handshake(setup).await;
    (s.unwrap(), c.unwrap())
}

/// The connector's side of the tests: accepts every stream and echoes it, as a host of the company would.
fn echo(mut connector: ConnectorChannel) -> tokio::task::JoinHandle<ConnectorChannel> {
    tokio::spawn(async move {
        while let Some(incoming) = connector.next_open().await {
            let stream = incoming.accept();
            tokio::spawn(async move {
                let (mut r, mut w) = tokio::io::split(stream);
                let _ = tokio::io::copy(&mut r, &mut w).await;
                let _ = w.shutdown().await;
            });
        }
        connector
    })
}

#[tokio::test]
async fn a_stream_goes_and_comes_back_and_the_ceiling_is_known() {
    let (server, mut connector) = pair(Setup::default()).await;
    assert_eq!(server.peer_version(), V);
    assert_eq!(server.verdict(), Verdict::Current);
    let opening = tokio::spawn(async move {
        let stream = server
            .open("gitlab.acme.internal", 22, "agent 3", "Démo")
            .await;
        (server, stream)
    });
    let incoming = connector.next_open().await.unwrap();
    let open = incoming.open().clone();
    assert_eq!(
        (
            open.host.as_str(),
            open.port,
            open.agent.as_str(),
            open.project.as_str()
        ),
        ("gitlab.acme.internal", 22, "agent 3", "Démo")
    );
    let mut at_connector = incoming.accept();
    let (server, stream) = opening.await.unwrap();
    let mut at_server = stream.unwrap();
    assert_eq!(at_server.id(), open.id);

    at_server
        .write_all(b"SSH-2.0-OpenSSH_9.6\r\n")
        .await
        .unwrap();
    let mut buf = [0u8; 21];
    at_connector.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"SSH-2.0-OpenSSH_9.6\r\n");
    at_connector.write_all(b"SSH-2.0-GitLab\r\n").await.unwrap();
    let mut back = [0u8; 16];
    at_server.read_exact(&mut back).await.unwrap();
    assert_eq!(&back, b"SSH-2.0-GitLab\r\n");

    // Each end closes its direction (FIN); the other still writes.
    at_server.shutdown().await.unwrap();
    let mut rest = Vec::new();
    assert_eq!(at_connector.read_to_end(&mut rest).await.unwrap(), 0);
    at_connector.write_all(b"bye").await.unwrap();
    at_connector.shutdown().await.unwrap();
    let mut bye = Vec::new();
    at_server.read_to_end(&mut bye).await.unwrap();
    assert_eq!(bye, b"bye");

    // The ceiling came with the connection, and again when it changes.
    assert_eq!(server.ceiling(), Some(ceiling()));
    let mut watch = server.watch_ceiling();
    let narrower = Ceiling {
        rules: vec!["gitlab.acme.internal:22".into()],
    };
    connector.send_ceiling(&narrower).unwrap();
    watch
        .wait_for(|c| c.as_ref() == Some(&narrower))
        .await
        .unwrap();
    let huge = Ceiling {
        rules: vec!["x".repeat(1000); 40],
    };
    assert_eq!(connector.send_ceiling(&huge), Err(CeilingError::TooLarge));
}

#[tokio::test]
async fn a_refusal_reaches_the_server_with_its_reason() {
    let (server, mut connector) = pair(Setup::default()).await;
    tokio::spawn(async move {
        while let Some(incoming) = connector.next_open().await {
            let host = incoming.open().host.clone();
            incoming.refuse(Refusal::new(
                RefusalCode::NotAllowed,
                format!("{host}:22 is not allowed by the connector of Acme"),
            ));
        }
    });
    match server.open("intranet.acme.internal", 22, "a", "p").await {
        Err(OpenError::Refused(r)) => {
            assert_eq!(r.code, RefusalCode::NotAllowed);
            assert_eq!(
                r.reason,
                "intranet.acme.internal:22 is not allowed by the connector of Acme"
            );
        }
        other => panic!("{:?}", other.map(|s| s.id())),
    }
    assert_eq!(
        server.open(&"h".repeat(300), 22, "a", "p").await.err(),
        Some(OpenError::Invalid)
    );
}

#[tokio::test]
async fn an_incoming_dropped_unanswered_is_refused() {
    let (server, mut connector) = pair(Setup::default()).await;
    tokio::spawn(async move {
        while let Some(incoming) = connector.next_open().await {
            drop(incoming);
        }
    });
    match server.open("nexus.acme.internal", 443, "a", "p").await {
        Err(OpenError::Refused(r)) => assert_eq!(r.code, RefusalCode::Closing),
        other => panic!("{:?}", other.map(|s| s.id())),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_slow_stream_does_not_hold_another() {
    let (server, mut connector) = pair(Setup::default()).await;
    let server = Arc::new(server);
    // The connector: the first stream is never read (a slow `git clone`), the others are echoed.
    let held = Arc::new(tokio::sync::Mutex::new(None));
    let keep = held.clone();
    tokio::spawn(async move {
        let mut first = true;
        while let Some(incoming) = connector.next_open().await {
            let stream = incoming.accept();
            if std::mem::take(&mut first) {
                *keep.lock().await = Some(stream);
                continue;
            }
            tokio::spawn(async move {
                let (mut r, mut w) = tokio::io::split(stream);
                let _ = tokio::io::copy(&mut r, &mut w).await;
                let _ = w.shutdown().await;
            });
        }
    });
    let slow = server
        .open("gitlab.acme.internal", 22, "a", "p")
        .await
        .unwrap();
    let written = Arc::new(AtomicUsize::new(0));
    let count = written.clone();
    let (_slow_read, mut slow_write) = slow.into_split();
    let writer = tokio::spawn(async move {
        let chunk = vec![7u8; 8 << 10];
        loop {
            if slow_write.write_all(&chunk).await.is_err() {
                return slow_write;
            }
            count.fetch_add(chunk.len(), Ordering::SeqCst);
        }
    });

    // Meanwhile, 4 MiB go and come back on another stream: sixteen times the window.
    let fast = server
        .open("nexus.acme.internal", 443, "a", "p")
        .await
        .unwrap();
    let (mut r, mut w) = fast.into_split();
    let total = 4 << 20;
    let sending = tokio::spawn(async move {
        let block: Vec<u8> = (0..64 << 10).map(|i| (i % 251) as u8).collect();
        for _ in 0..total / block.len() {
            w.write_all(&block).await.unwrap();
        }
        w.shutdown().await.unwrap();
    });
    let mut got = Vec::with_capacity(total);
    tokio::time::timeout(Duration::from_secs(60), r.read_to_end(&mut got))
        .await
        .expect("the fast stream is not held")
        .unwrap();
    sending.await.unwrap();
    assert_eq!(got.len(), total);
    assert!(got
        .iter()
        .enumerate()
        .all(|(i, &b)| b == ((i % (64 << 10)) % 251) as u8));

    // The slow writer stopped at its window, exactly.
    assert_eq!(written.load(Ordering::SeqCst), INITIAL_WINDOW as usize);
    assert!(!writer.is_finished());
    // Once read, it goes on.
    let mut slow_at_connector = held.lock().await.take().unwrap();
    let mut buf = vec![0u8; 512 << 10];
    slow_at_connector.read_exact(&mut buf).await.unwrap();
    tokio::time::timeout(Duration::from_secs(30), async {
        while written.load(Ordering::SeqCst) < (512 << 10) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the window granted again");
    writer.abort();
}

#[tokio::test]
async fn a_dropped_stream_is_reset_at_the_other_end_and_a_closed_channel_resets_all() {
    let (server, mut connector) = pair(Setup::default()).await;
    let opening = tokio::spawn(async move {
        let s = server.open("h", 1, "a", "p").await.unwrap();
        (server, s)
    });
    let at_connector = connector.next_open().await.unwrap().accept();
    let (server, mut at_server) = opening.await.unwrap();
    drop(at_connector);
    let mut buf = [0u8; 4];
    let err = at_server.read(&mut buf).await.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::ConnectionReset);

    let opening = tokio::spawn(async move {
        let s = server.open("h", 2, "a", "p").await.unwrap();
        (server, s)
    });
    let mut at_connector = connector.next_open().await.unwrap().accept();
    let (server, _at_server) = opening.await.unwrap();
    server.close();
    assert_eq!(server.closed().await, Closed::Local);
    assert!(at_connector.read(&mut buf).await.is_err());
    assert!(connector.next_open().await.is_none());
    assert_eq!(connector.closed().await, Closed::ByPeer);
    assert_eq!(
        server.open("h", 3, "a", "p").await.err(),
        Some(OpenError::Closed)
    );
}

// === The handshake ===

#[tokio::test]
async fn an_unknown_key_is_closed_after_the_first_message_without_a_word() {
    let (ws_server, ws_client) = sockets().await;
    let server_keys = Keypair::generate();
    let paired = Keypair::generate();
    let stranger = Keypair::generate();
    let pinned = paired.public();
    let server_public = server_keys.public();
    let server = tokio::spawn(async move {
        accept_connector(
            ws_server,
            &server_keys,
            |k| *k == pinned,
            V,
            Limits::default(),
        )
        .await
    });
    let connector = connect_server(
        ws_client,
        &stranger,
        &server_public,
        V,
        &ceiling(),
        Limits::default(),
    )
    .await;
    let refused = server.await.unwrap().err().unwrap();
    assert_eq!(refused, AcceptError::UnknownKey(stranger.public()));
    assert!(refused.is_failed_attempt());
    assert_eq!(connector.err(), Some(ConnectError::NotPaired));
}

#[tokio::test]
async fn a_changed_key_is_refused_on_either_side() {
    // The connector's key changed (a new pair, not paired again): unknown to the server.
    let (ws_server, ws_client) = sockets().await;
    let server_keys = Keypair::generate();
    let old = Keypair::generate();
    let new = Keypair::generate();
    let pinned = old.public();
    let server_public = server_keys.public();
    let server = tokio::spawn(async move {
        accept_connector(
            ws_server,
            &server_keys,
            |k| *k == pinned,
            V,
            Limits::default(),
        )
        .await
    });
    let _ = connect_server(
        ws_client,
        &new,
        &server_public,
        V,
        &ceiling(),
        Limits::default(),
    )
    .await;
    assert!(matches!(server.await.unwrap(), Err(AcceptError::UnknownKey(k)) if k == new.public()));

    // The server's key changed: the connector's first message is made for the old one, and does not open.
    let (ws_server, ws_client) = sockets().await;
    let old_server = Keypair::generate();
    let new_server = Keypair::generate();
    let connector = Keypair::generate();
    let pinned = connector.public();
    let server = tokio::spawn(async move {
        accept_connector(
            ws_server,
            &new_server,
            |k| *k == pinned,
            V,
            Limits::default(),
        )
        .await
    });
    let told = connect_server(
        ws_client,
        &connector,
        &old_server.public(),
        V,
        &ceiling(),
        Limits::default(),
    )
    .await;
    let refused = server.await.unwrap().err().unwrap();
    assert_eq!(refused, AcceptError::NotNoise);
    assert!(refused.is_failed_attempt());
    assert_eq!(told.err(), Some(ConnectError::NotPaired));
}

#[tokio::test]
async fn garbage_and_silence_are_failed_attempts() {
    let (ws_server, mut ws_client) = sockets().await;
    let server_keys = Keypair::generate();
    let server = tokio::spawn(async move {
        accept_connector(ws_server, &server_keys, |_| true, V, Limits::default()).await
    });
    ws_client
        .send(Message::Binary(b"GET / HTTP/1.1\r\n\r\n".to_vec().into()))
        .await
        .unwrap();
    let refused = server.await.unwrap().err().unwrap();
    assert_eq!(refused, AcceptError::NotNoise);
    assert!(refused.is_failed_attempt());
    // Nothing comes back but the end.
    assert!(!matches!(
        ws_client.next().await,
        Some(Ok(Message::Binary(_)))
    ));

    tokio::time::pause();
    let (ws_server, _ws_client) = sockets().await;
    let server_keys = Keypair::generate();
    let start = Instant::now();
    let refused = accept_connector(ws_server, &server_keys, |_| true, V, Limits::default())
        .await
        .err()
        .unwrap();
    assert_eq!(refused, AcceptError::Timeout);
    assert!(refused.is_failed_attempt());
    assert_eq!(start.elapsed().as_secs(), 10);
}

#[tokio::test]
async fn the_server_accepts_its_minor_and_the_two_before() {
    let with = |server_version, connector_version| Setup {
        server_version,
        connector_version,
        ..Setup::default()
    };
    // Current, a newer patch, one minor before.
    for c in [
        Version::new(2, 3, 0),
        Version::new(2, 3, 9),
        Version::new(2, 2, 4),
    ] {
        let (s, k) = pair(with(V, c)).await;
        assert_eq!(s.verdict(), Verdict::Current);
        assert_eq!(s.peer_version(), c);
        assert_eq!(k.welcome().verdict, Verdict::Current);
        assert_eq!(k.welcome().version, V);
    }
    // Two before: accepted, to update.
    let (s, k) = pair(with(V, Version::new(2, 1, 0))).await;
    assert_eq!(s.verdict(), Verdict::Outdated);
    assert!(k
        .welcome()
        .advice(Version::new(2, 1, 0))
        .unwrap()
        .contains("to update"));
    // Further, another major, newer: refused, and told what to do.
    for (c, verdict) in [
        (Version::new(2, 0, 7), Verdict::TooOld),
        (Version::new(1, 9, 0), Verdict::TooOld),
        (Version::new(2, 4, 0), Verdict::TooNew),
        (Version::new(3, 0, 0), Verdict::TooNew),
    ] {
        let (s, k) = handshake(with(V, c)).await;
        let refused = s.err().unwrap();
        assert_eq!(
            refused,
            AcceptError::Version {
                connector: c,
                verdict
            }
        );
        assert!(!refused.is_failed_attempt(), "a paired key");
        match k.err().unwrap() {
            ConnectError::Version { welcome, ours } => {
                assert_eq!((welcome.verdict, welcome.version, ours), (verdict, V, c));
                assert_eq!(welcome.oldest, Version::new(2, 1, 0));
                let advice = ConnectError::Version { welcome, ours }.to_string();
                assert!(advice.contains("2.1 to 2.3"), "{advice}");
                let todo = if verdict == Verdict::TooOld {
                    "escouade-connector update --to 2.3.0"
                } else {
                    "update escouade-server"
                };
                assert!(advice.contains(todo), "{advice}");
            }
            other => panic!("{other:?}"),
        }
    }
}

// === The life ===

/// A connector played by hand: the handshake, then frames sealed one by one; it answers nothing unless told.
struct RawConnector {
    ws: WebSocketStream<DuplexStream>,
    transport: Transport,
}

impl RawConnector {
    async fn send(&mut self, frame: &Frame) {
        let plain = frame.encode().unwrap();
        self.send_plain(&plain).await;
    }

    async fn send_plain(&mut self, plain: &[u8]) {
        let sealed = self.transport.seal(plain).unwrap();
        self.ws.send(Message::Binary(sealed.into())).await.unwrap();
    }

    async fn next(&mut self) -> Option<Frame> {
        loop {
            match self.ws.next().await? {
                Ok(Message::Binary(b)) => {
                    return Some(Frame::decode(&self.transport.open(&b).unwrap()).unwrap());
                }
                Ok(Message::Close(_)) | Err(_) => return None,
                Ok(_) => continue,
            }
        }
    }
}

async fn raw(limits: Limits) -> (ServerChannel, RawConnector) {
    let (ws_server, mut ws) = sockets().await;
    let server_keys = Keypair::generate();
    let connector = Keypair::generate();
    let server_public = server_keys.public();
    let server = tokio::spawn(async move {
        accept_connector(ws_server, &server_keys, |_| true, V, limits).await
    });
    let (initiator, first) =
        Initiator::start(&connector, &server_public, &Hello { version: V }).unwrap();
    ws.send(Message::Binary(first.into())).await.unwrap();
    let second = match ws.next().await.unwrap().unwrap() {
        Message::Binary(b) => b,
        other => panic!("{other:?}"),
    };
    let (_, transport) = initiator.finish(&second).unwrap();
    (
        server.await.unwrap().unwrap(),
        RawConnector { ws, transport },
    )
}

#[tokio::test(start_paused = true)]
async fn a_ping_every_15_s_and_silent_45_s_is_cut() {
    let (server, mut peer) = raw(Limits::default()).await;
    let start = Instant::now();
    assert_eq!(peer.next().await, Some(Frame::Ping { nonce: 1 }));
    assert_eq!(start.elapsed(), PING_EVERY);
    assert_eq!(peer.next().await, Some(Frame::Ping { nonce: 2 }));
    assert_eq!(start.elapsed(), PING_EVERY * 2);
    // The peer said nothing since the handshake: cut at 45 s.
    assert_eq!(server.closed().await, Closed::Silent);
    assert_eq!(start.elapsed(), DEAD_AFTER);
}

#[tokio::test(start_paused = true)]
async fn a_peer_that_answers_keeps_the_channel() {
    let (server, mut peer) = raw(Limits::default()).await;
    let start = Instant::now();
    // Answered pings for 10 minutes.
    while start.elapsed() < Duration::from_secs(600) {
        match peer.next().await {
            Some(Frame::Ping { nonce }) => peer.send(&Frame::Pong { nonce }).await,
            other => panic!("{other:?}"),
        }
    }
    assert!(!server.is_closed());
    // A ping answered with a pong, and a frame of a later minor ignored.
    peer.send(&Frame::Ping { nonce: 99 }).await;
    assert_eq!(peer.next().await, Some(Frame::Pong { nonce: 99 }));
    peer.send_plain(&[0x42, 1, 2, 3]).await;
    peer.send(&Frame::Ping { nonce: 100 }).await;
    assert_eq!(peer.next().await, Some(Frame::Pong { nonce: 100 }));
    assert!(!server.is_closed());
}

#[tokio::test(start_paused = true)]
async fn two_channels_keep_each_other_alive() {
    let (server, connector) = pair(Setup::default()).await;
    tokio::time::sleep(Duration::from_secs(3600)).await;
    assert!(!server.is_closed() && !connector.is_closed());
}

// === The bounds ===

#[tokio::test(start_paused = true)]
async fn the_connector_refuses_beyond_its_streams_and_its_rate() {
    let connector_limits = Limits {
        max_streams: 2,
        opens_per_second: 3,
        ..Limits::default()
    };
    let (server, connector) = pair(Setup {
        connector_limits,
        ..Setup::default()
    })
    .await;
    let _echo = echo(connector);
    let a = server.open("h", 1, "a", "p").await.unwrap();
    let _b = server.open("h", 2, "a", "p").await.unwrap();
    match server.open("h", 3, "a", "p").await {
        Err(OpenError::Refused(r)) => assert_eq!(r.code, RefusalCode::Limits, "{r}"),
        other => panic!("{:?}", other.map(|s| s.id())),
    }
    // One closed: room again, but this second has seen its three opens.
    drop(a);
    tokio::task::yield_now().await;
    match server.open("h", 4, "a", "p").await {
        Err(OpenError::Refused(r)) => assert_eq!(r.code, RefusalCode::Limits),
        other => panic!("{:?}", other.map(|s| s.id())),
    }
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(server.open("h", 5, "a", "p").await.is_ok());
}

#[tokio::test(start_paused = true)]
async fn the_server_keeps_its_own_bounds() {
    let server_limits = Limits {
        max_streams: 1,
        ..Limits::default()
    };
    let (server, connector) = pair(Setup {
        server_limits,
        ..Setup::default()
    })
    .await;
    let _echo = echo(connector);
    let _a = server.open("h", 1, "a", "p").await.unwrap();
    assert_eq!(
        server.open("h", 2, "a", "p").await.err(),
        Some(OpenError::Limits)
    );
}

#[tokio::test(start_paused = true)]
async fn a_silent_stream_is_closed_after_10_min() {
    let (server, mut connector) = pair(Setup::default()).await;
    let opening = tokio::spawn(async move {
        let s = server.open("h", 1, "a", "p").await.unwrap();
        (server, s)
    });
    let mut at_connector = connector.next_open().await.unwrap().accept();
    let (server, mut at_server) = opening.await.unwrap();
    let start = Instant::now();
    let mut buf = [0u8; 1];
    assert!(at_server.read(&mut buf).await.is_err());
    assert!(at_connector.read(&mut buf).await.is_err());
    let after = start.elapsed();
    assert!(
        after >= Duration::from_secs(600) && after <= Duration::from_secs(660),
        "{after:?}"
    );
    assert!(!server.is_closed(), "the channel stays");
}

#[tokio::test]
async fn a_frame_beyond_the_bound_closes_the_channel() {
    let (server, mut peer) = raw(Limits {
        max_frame: 32 << 10,
        ..Limits::default()
    })
    .await;
    peer.send(&Frame::Ceiling(Ceiling {
        rules: vec!["x".repeat(1000); 40],
    }))
    .await;
    assert!(matches!(server.closed().await, Closed::Protocol(e) if e.contains("bytes")));
}

#[tokio::test]
async fn data_beyond_the_window_closes_the_channel() {
    let (server, mut peer) = raw(Limits::default()).await;
    let opening = tokio::spawn(async move {
        let s = server.open("h", 1, "a", "p").await;
        (server, s)
    });
    let id = match peer.next().await {
        Some(Frame::Open(o)) => o.id,
        other => panic!("{other:?}"),
    };
    peer.send(&Frame::Opened { id }).await;
    let (server, stream) = opening.await.unwrap();
    let _stream = stream.unwrap();
    // The window, then one byte more, never read.
    for _ in 0..INITIAL_WINDOW as usize / (16 << 10) {
        peer.send(&Frame::Data {
            id,
            bytes: vec![1; 16 << 10],
        })
        .await;
    }
    peer.send(&Frame::Data { id, bytes: vec![1] }).await;
    assert!(matches!(server.closed().await, Closed::Protocol(e) if e.contains("window")));
}

#[tokio::test]
async fn frames_out_of_their_role_close_the_channel() {
    let (server, mut peer) = raw(Limits::default()).await;
    peer.send(&Frame::Open(Open {
        id: 1,
        host: "h".into(),
        port: 1,
        agent: "a".into(),
        project: "p".into(),
    }))
    .await;
    assert!(matches!(server.closed().await, Closed::Protocol(e) if e.contains("open")));
}
