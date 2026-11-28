//! The connectors' entry (RP-4), for real on 127.0.0.1, against a fake connector made of the protocol's crate (RP-2):
//! it pairs by a code, connects, sends its ceiling, echoes the flows the gate opens through it. No network but the
//! loopback; no key on disk (the secrets in memory, the certificate of the TLS test made in it).

use escouade_connector_proto::pairing::{self, PairError};
use escouade_connector_proto::transport::{self, ws_config, ClientConfig};
use escouade_connector_proto::{
    connect_server, Ceiling, ConnectError, ConnectorChannel, Keypair, Limits as ChannelLimits,
    PublicKey, Version,
};
use escouade_gate::entry::{
    self, Entry, Limits, MemoryStore, Registry, State, Store, VersionState,
};
use escouade_gate::lists::{Allowed, ProjectList};
use escouade_gate::{Config as GateConfig, Connectors, Gate, Holder, Lang, Lists, Outside, Reach};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const SERVER: Version = Version::new(2, 3, 0);
const DEMO: &str = "p-demo";

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

struct Fixture {
    entry: Entry,
    registry: Arc<Registry>,
    store: Arc<MemoryStore>,
}

async fn fixture_with(limits: Limits) -> Fixture {
    let store = Arc::new(MemoryStore::default());
    let registry = Registry::new(store.clone(), Lang::Fr, SERVER);
    let entry = Entry::start(entry::Config {
        listen: "127.0.0.1:0".parse().unwrap(),
        tls: None,
        registry: registry.clone(),
        proxies: Vec::new(),
        bounds: limits,
        channel: ChannelLimits::default(),
    })
    .await
    .unwrap();
    Fixture {
        entry,
        registry,
        store,
    }
}

async fn fixture() -> Fixture {
    fixture_with(Limits::default()).await
}

/// A WebSocket to the entry, on `path`, without TLS (the entry behind a TLS proxy).
async fn ws(
    f: &Fixture,
    path: &str,
) -> Result<tokio_tungstenite::WebSocketStream<TcpStream>, String> {
    let tcp = TcpStream::connect(f.entry.addr())
        .await
        .map_err(|e| e.to_string())?;
    let url = format!("ws://{}{path}", f.entry.addr());
    tokio_tungstenite::client_async_with_config(url, tcp, Some(ws_config()))
        .await
        .map(|(ws, _)| ws)
        .map_err(|e| e.to_string())
}

async fn pair(
    f: &Fixture,
    keys: &Keypair,
    code: &str,
    version: Version,
) -> Result<pairing::Paired, PairError> {
    let ws = ws(f, "/connector/pair").await.map_err(PairError::Net)?;
    pairing::pair_server(ws, keys, code, version, Duration::from_secs(10)).await
}

fn ceiling() -> Ceiling {
    Ceiling {
        rules: vec![
            "gitlab.acme.internal:22".into(),
            "*.npm.acme.internal:443".into(),
            "10.20.0.0/16".into(),
        ],
    }
}

async fn connect(
    f: &Fixture,
    keys: &Keypair,
    server: &PublicKey,
    version: Version,
) -> Result<ConnectorChannel, ConnectError> {
    let ws = ws(f, "/connector").await.map_err(ConnectError::Net)?;
    connect_server(
        ws,
        keys,
        server,
        version,
        &ceiling(),
        ChannelLimits::default(),
    )
    .await
}

/// The connector echoes every flow it is asked to open, until it is cut (`Echo::cut`: its channel closed).
struct Echo(Option<tokio::sync::oneshot::Sender<()>>);

impl Echo {
    fn cut(&mut self) {
        if let Some(t) = self.0.take() {
            let _ = t.send(());
        }
    }
}

fn echo_flows(mut ch: ConnectorChannel) -> Echo {
    let (tx, mut rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                incoming = ch.next_open() => {
                    let Some(incoming) = incoming else { return };
                    let stream = incoming.accept();
                    tokio::spawn(async move {
                        let (mut r, mut w) = tokio::io::split(stream);
                        let _ = tokio::io::copy(&mut r, &mut w).await;
                        let _ = w.shutdown().await;
                    });
                }
                _ = &mut rx => {
                    ch.close();
                    return;
                }
            }
        }
    });
    Echo(Some(tx))
}

/// What a pairing refused unread gives the connector: closed without a word (or reset, under Windows).
fn refused_unread(got: Result<pairing::Paired, PairError>) {
    match got {
        Err(PairError::WrongCode) | Err(PairError::Net(_)) => {}
        other => panic!("{:?}", other.map(|_| ())),
    }
}

async fn until(what: &str, mut ok: impl FnMut() -> bool) {
    for _ in 0..500 {
        if ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("never: {what}");
}

fn view(f: &Fixture, name: &str) -> entry::View {
    f.registry
        .views()
        .into_iter()
        .find(|v| v.name == name)
        .unwrap_or_else(|| panic!("no connector {name}"))
}

/// A connector paired as `name`: its keys, and the server's key it pinned.
async fn paired(f: &Fixture, name: &str) -> (Keypair, PublicKey) {
    let code = f.registry.create_code(name, now_ms()).unwrap();
    let keys = Keypair::generate();
    let got = pair(f, &keys, &code.text, SERVER).await.unwrap();
    (keys, got.server)
}

#[tokio::test]
async fn a_connector_pairs_by_its_code_connects_and_sends_its_ceiling() {
    let f = fixture().await;
    let code = f.registry.create_code("Acme", now_ms()).unwrap();
    assert_eq!(code.text.len(), 9, "ABCD-EFGH");
    assert_eq!(code.text.as_bytes()[4], b'-');
    assert!(code
        .text
        .chars()
        .filter(|c| *c != '-')
        .all(|c| "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".contains(c)));
    let left = code.expires_at - now_ms();
    assert!(left > 9 * 60_000 && left <= 10 * 60_000, "10 minutes");
    // Only the code's pre-shared key is kept, never the code.
    let secrets = f.store.secrets();
    assert!(!format!("{secrets:?}").contains(&code.text));
    assert!(!format!("{secrets:?}").contains(&code.text.replace('-', "")));

    let keys = Keypair::generate();
    // Typed in lower case, without its dash: the same code.
    let typed = code.text.to_lowercase().replace('-', "");
    let got = pair(&f, &keys, &typed, SERVER).await.unwrap();
    assert_eq!(got.server, f.registry.server_keys().unwrap().public());
    let v = view(&f, "Acme");
    assert_eq!(v.state, State::Never);
    assert_eq!(v.version.as_deref(), Some("2.3.0"));
    // The connector's key in the secrets, the code gone.
    let secrets = f.store.secrets();
    assert_eq!(
        secrets.get(&entry::store::key_secret(&v.id)),
        Some(&keys.public().to_hex())
    );
    assert!(f.registry.pending(now_ms()).is_empty());

    // Once: the same code again is a wrong code.
    let again = pair(&f, &Keypair::generate(), &code.text, SERVER).await;
    assert_eq!(again.unwrap_err(), PairError::WrongCode);

    // It connects, sends its ceiling: kept, shown, given to the gate (its ranges of addresses kept, not as patterns).
    let ch = connect(&f, &keys, &got.server, SERVER).await.unwrap();
    until("connected", || view(&f, "Acme").state == State::Connected).await;
    until("its ceiling", || {
        view(&f, "Acme").ceiling == ceiling().rules
    })
    .await;
    let v = view(&f, "Acme");
    assert_eq!(v.version_state, Some(VersionState::Current));
    let known = f.registry.known();
    assert_eq!(known.len(), 1);
    assert_eq!(known[0].reach, Reach::Up);
    assert_eq!(
        known[0].ceiling.len(),
        2,
        "the range is the connector's alone"
    );

    // A flow through it.
    let mut echo = echo_flows(ch);
    let mut flow = f
        .registry
        .open(
            &v.id,
            escouade_gate::Ask {
                host: "gitlab.acme.internal".into(),
                port: 22,
                agent: "agent-1".into(),
                project: DEMO.into(),
            },
        )
        .await
        .unwrap();
    flow.write_all(b"ping").await.unwrap();
    let mut got = [0u8; 4];
    flow.read_exact(&mut got).await.unwrap();
    assert_eq!(&got, b"ping");
    drop(flow);

    // Cut: down at once, and back when it reconnects.
    echo.cut();
    until("cut", || view(&f, "Acme").state == State::Down).await;
    assert!(matches!(f.registry.known()[0].reach, Reach::Down { .. }));
    let _ch = connect(
        &f,
        &keys,
        &f.registry.server_keys().unwrap().public(),
        SERVER,
    )
    .await
    .unwrap();
    until("back", || view(&f, "Acme").state == State::Connected).await;
}

#[tokio::test]
async fn a_wrong_code_is_closed_without_a_word_counted_and_never_ends_the_code() {
    let f = fixture().await;
    let code = f.registry.create_code("Acme", now_ms()).unwrap();
    for _ in 0..4 {
        let got = pair(&f, &Keypair::generate(), "ZZZZ-ZZZZ", SERVER).await;
        assert_eq!(got.unwrap_err(), PairError::WrongCode);
    }
    // Four wrong codes: the code still holds.
    assert_eq!(f.registry.pending(now_ms()).len(), 1);
    pair(&f, &Keypair::generate(), &code.text, SERVER)
        .await
        .unwrap();
    assert!(f.registry.views().iter().any(|v| v.name == "Acme"));
}

#[tokio::test]
async fn five_wrong_codes_close_the_pairing_to_their_source_the_code_waiting_on() {
    let f = fixture().await;
    let code = f.registry.create_code("Acme", now_ms()).unwrap();
    for _ in 0..5 {
        let got = pair(&f, &Keypair::generate(), "ZZZZ-ZZZZ", SERVER).await;
        assert_eq!(got.unwrap_err(), PairError::WrongCode);
    }
    assert!(!f.entry.bounds().may_pair("127.0.0.1"));
    // Even the right code is refused unread from that source now; the code waits on (another source may give it).
    refused_unread(pair(&f, &Keypair::generate(), &code.text, SERVER).await);
    assert_eq!(f.registry.pending(now_ms()).len(), 1);
    assert!(f.registry.views().is_empty());
}

#[tokio::test]
async fn an_expired_code_is_a_wrong_code() {
    let f = fixture().await;
    let long_ago = now_ms() - 11 * 60_000;
    let code = f.registry.create_code("Acme", long_ago).unwrap();
    assert!(f.registry.pending(now_ms()).is_empty());
    let got = pair(&f, &Keypair::generate(), &code.text, SERVER).await;
    assert_eq!(got.unwrap_err(), PairError::WrongCode);
    assert!(f.registry.views().is_empty());
}

#[tokio::test]
async fn a_new_code_for_a_name_takes_the_place_of_the_one_before_and_a_paired_name_is_refused() {
    let f = fixture().await;
    let first = f.registry.create_code("Acme", now_ms()).unwrap();
    let second = f.registry.create_code("acme", now_ms()).unwrap();
    assert_eq!(f.registry.pending(now_ms()).len(), 1);
    let got = pair(&f, &Keypair::generate(), &first.text, SERVER).await;
    assert_eq!(got.unwrap_err(), PairError::WrongCode);
    pair(&f, &Keypair::generate(), &second.text, SERVER)
        .await
        .unwrap();
    let err = f.registry.create_code("ACME", now_ms()).unwrap_err();
    assert!(err.contains("déjà appairé"), "{err}");
    assert!(f.registry.create_code("", now_ms()).is_err());
    // Two networks, two connectors (R12).
    f.registry.create_code("Client d'Acme", now_ms()).unwrap();
}

#[tokio::test]
async fn an_unknown_key_is_closed_at_its_first_message_and_twenty_close_the_source() {
    let f = fixture().await;
    let server = f.registry.server_keys().unwrap().public();
    for i in 0..20 {
        let got = connect(&f, &Keypair::generate(), &server, SERVER).await;
        match got {
            Err(ConnectError::NotPaired) => {}
            Err(ConnectError::Net(_)) if i == 19 => {}
            other => panic!("{i}: {:?}", other.map(|_| ())),
        }
    }
    until("closed", || f.entry.bounds().closed("127.0.0.1")).await;
    // Closed unread: not even the WebSocket's upgrade.
    assert!(ws(&f, "/connector").await.is_err());
    // Nothing paired came of it.
    assert!(f.registry.views().is_empty());
}

#[tokio::test]
async fn a_paired_connector_still_gets_in_when_the_pairing_is_closed_to_all() {
    let f = fixture_with(Limits {
        failures_per_minute: 100,
        ..Limits::default()
    })
    .await;
    let (keys, server) = paired(&f, "Acme").await;
    let code = f.registry.create_code("Other", now_ms()).unwrap();
    for _ in 0..20 {
        let _ = connect(&f, &Keypair::generate(), &server, SERVER).await;
    }
    until("pairing closed", || !f.entry.bounds().may_pair("10.9.9.9")).await;
    refused_unread(pair(&f, &Keypair::generate(), &code.text, SERVER).await);
    connect(&f, &keys, &server, SERVER).await.unwrap();
    until("connected", || view(&f, "Acme").state == State::Connected).await;
}

#[tokio::test]
async fn removed_its_key_no_longer_holds_its_channel_is_closed_and_a_trace_is_kept() {
    let f = fixture().await;
    let (keys, server) = paired(&f, "Acme").await;
    let ch = connect(&f, &keys, &server, SERVER).await.unwrap();
    until("connected", || view(&f, "Acme").state == State::Connected).await;
    let id = view(&f, "Acme").id;
    let trace = f
        .registry
        .remove("acme", entry::registry::BY_COMMAND, now_ms())
        .unwrap()
        .unwrap();
    assert_eq!(trace.name, "Acme");
    assert_eq!(trace.by, "escouade-server");
    assert!(!trace.key_hash.contains(&keys.public().to_hex()));
    tokio::time::timeout(Duration::from_secs(5), ch.closed())
        .await
        .expect("its channel closed");
    assert!(!f
        .store
        .secrets()
        .contains_key(&entry::store::key_secret(&id)));
    assert_eq!(f.registry.removed().len(), 1);
    assert!(f.registry.known().is_empty());
    // It no longer gets in.
    let again = connect(&f, &keys, &server, SERVER).await;
    assert_eq!(again.err(), Some(ConnectError::NotPaired));
    assert!(f.registry.remove("Acme", "x", now_ms()).unwrap().is_none());
}

#[tokio::test]
async fn removed_by_another_process_its_channel_is_closed_within_seconds() {
    let f = fixture().await;
    let (keys, server) = paired(&f, "Acme").await;
    let ch = connect(&f, &keys, &server, SERVER).await.unwrap();
    until("connected", || view(&f, "Acme").state == State::Connected).await;
    // `escouade-server connectors remove` beside the server: another registry on the same store.
    let beside = Registry::new(f.store.clone() as Arc<dyn Store>, Lang::Fr, SERVER);
    beside
        .remove("Acme", entry::registry::BY_COMMAND, now_ms())
        .unwrap()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(6), ch.closed())
        .await
        .expect("its channel closed");
}

#[tokio::test]
async fn versions_two_minors_behind_is_to_update_beyond_is_refused_and_said() {
    let f = fixture().await;
    let (keys, server) = paired(&f, "Acme").await;
    // Two minors behind: accepted, « to update ».
    let ch = connect(&f, &keys, &server, Version::new(2, 1, 4))
        .await
        .unwrap();
    until("connected", || view(&f, "Acme").state == State::Connected).await;
    let v = view(&f, "Acme");
    assert_eq!(v.version_state, Some(VersionState::Outdated));
    assert!(v.advice.unwrap().contains("update"));
    ch.close();
    until("cut", || view(&f, "Acme").state == State::Down).await;
    // Too old: refused, with what to do, kept for the space.
    let old = connect(&f, &keys, &server, Version::new(2, 0, 0)).await;
    assert!(
        matches!(old, Err(ConnectError::Version { .. })),
        "{:?}",
        old.map(|_| ())
    );
    until("too old", || {
        view(&f, "Acme").version.as_deref() == Some("2.0.0")
    })
    .await;
    let v = view(&f, "Acme");
    assert_eq!(v.version_state, Some(VersionState::TooOld));
    assert!(v
        .advice
        .unwrap()
        .contains("escouade-connector update --to 2.3.0"));
    // A refused version is no failed attempt of a stranger.
    assert!(f.entry.bounds().may_pair("127.0.0.1"));
    // At the pairing too: refused, the code not used.
    let code = f.registry.create_code("Old", now_ms()).unwrap();
    let got = pair(&f, &Keypair::generate(), &code.text, Version::new(1, 9, 0)).await;
    assert!(matches!(got, Err(PairError::Version { .. })));
    assert_eq!(f.registry.pending(now_ms()).len(), 1);
}

#[tokio::test]
async fn the_entry_speaks_the_connectors_protocol_alone_never_the_remote_api() {
    let f = fixture().await;
    // The remote API's path, or any other: no upgrade.
    for path in ["/", "/api", "/remote", "/connector/x", "/connector/pair/x"] {
        let got = ws(&f, path).await;
        assert!(
            got.as_ref().err().is_some_and(|e| e.contains("404")),
            "{path}: {:?}",
            got.map(|_| ())
        );
    }
    // A plain request: no answer of an API.
    let mut s = TcpStream::connect(f.entry.addr()).await.unwrap();
    s.write_all(b"GET /api/welcome HTTP/1.1\r\nHost: x\r\n\r\n")
        .await
        .unwrap();
    let mut got = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(5), s.read_to_end(&mut got)).await;
    let got = String::from_utf8_lossy(&got);
    assert!(!got.contains("200 OK") && !got.contains("101"), "{got}");
    // A remote API's handshake as text over the connectors' WebSocket: closed.
    use futures_util::SinkExt;
    let mut w = ws(&f, "/connector").await.unwrap();
    let _ = w
        .send(tokio_tungstenite::tungstenite::Message::Text(
            r#"{"type":"hello","token":"x"}"#.into(),
        ))
        .await;
    let mut buf = Vec::new();
    use futures_util::StreamExt;
    let next = tokio::time::timeout(Duration::from_secs(5), w.next()).await;
    if let Ok(Some(Ok(m))) = next {
        buf.extend_from_slice(&m.into_data());
    }
    assert!(buf.is_empty(), "nothing answered");
}

#[tokio::test]
async fn the_gate_goes_through_the_real_entry_and_refuses_at_once_when_the_channel_is_cut() {
    let f = fixture().await;
    let (keys, server) = paired(&f, "Acme").await;
    let ch = connect(&f, &keys, &server, SERVER).await.unwrap();
    until("ceiling", || view(&f, "Acme").ceiling == ceiling().rules).await;
    let id = view(&f, "Acme").id;
    let mut echo = echo_flows(ch);
    let mut projects = BTreeMap::new();
    projects.insert(
        DEMO.to_string(),
        ProjectList {
            hosts: vec![Allowed {
                host: "gitlab.acme.internal".into(),
                port: 22,
                connector: id.clone(),
            }],
        },
    );
    let gate = Gate::start(GateConfig {
        lang: Lang::Fr,
        connectors: f.registry.clone(),
        lists: Lists { projects },
        outside: Outside::default(),
        port: 0,
    })
    .await
    .unwrap();
    let token = gate.issue(Holder {
        who: "agent-1".into(),
        project: DEMO.into(),
        project_name: "Démo".into(),
    });
    let (status, _, mut s) = gate_connect(&gate, &token, "gitlab.acme.internal:22").await;
    assert_eq!(status, 200);
    s.write_all(b"hello").await.unwrap();
    let mut got = [0u8; 5];
    tokio::time::timeout(Duration::from_secs(5), s.read_exact(&mut got))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(&got, b"hello");
    // Out of its ceiling: refused by the gate itself.
    let (status, body, _) = gate_connect(&gate, &token, "gitlab.acme.internal:443").await;
    assert_eq!(status, 403, "{body}");
    // Cut: refused at once, said.
    echo.cut();
    until("cut", || view(&f, "Acme").state == State::Down).await;
    let started = std::time::Instant::now();
    let (status, body, _) = gate_connect(&gate, &token, "gitlab.acme.internal:22").await;
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("injoignable"), "{body}");
    assert!(started.elapsed() < Duration::from_secs(2), "at once");
}

#[tokio::test]
async fn the_entry_serves_tls_of_its_own() {
    use rcgen::{
        BasicConstraints, CertificateParams, CertifiedIssuer, DnType, IsCa, KeyPair,
        KeyUsagePurpose,
    };
    let mut p = CertificateParams::new(Vec::<String>::new()).unwrap();
    p.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    p.distinguished_name
        .push(DnType::CommonName, "test authority");
    p.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    let ca = CertifiedIssuer::self_signed(p, KeyPair::generate().unwrap()).unwrap();
    let leaf_key = KeyPair::generate().unwrap();
    let leaf = CertificateParams::new(vec!["localhost".to_string()])
        .unwrap()
        .signed_by(&leaf_key, &ca)
        .unwrap();
    let tls = transport::tls_acceptor(leaf.pem().as_bytes(), leaf_key.serialize_pem().as_bytes())
        .unwrap();
    let store = Arc::new(MemoryStore::default());
    let registry = Registry::new(store.clone(), Lang::En, SERVER);
    let entry = Entry::start(entry::Config {
        listen: "127.0.0.1:0".parse().unwrap(),
        tls: Some(tls),
        registry: registry.clone(),
        proxies: Vec::new(),
        bounds: Limits::default(),
        channel: ChannelLimits::default(),
    })
    .await
    .unwrap();
    let dir = std::env::temp_dir().join(format!("entry-tls-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let authorities = dir.join("ca.pem");
    std::fs::write(&authorities, ca.pem()).unwrap();
    let url = format!("wss://localhost:{}/connector", entry.addr().port());
    let client = |url: String| {
        let mut c = ClientConfig::new(url);
        c.system_roots = false;
        c.authorities = Some(authorities.clone());
        c
    };
    let code = registry.create_code("Acme", now_ms()).unwrap();
    let keys = Keypair::generate();
    let ws = transport::connect(&client(pairing::pairing_url(&url).unwrap()))
        .await
        .unwrap();
    let got = pairing::pair_server(ws, &keys, &code.text, SERVER, Duration::from_secs(10))
        .await
        .unwrap();
    let ws = transport::connect(&client(url)).await.unwrap();
    let _ch = connect_server(
        ws,
        &keys,
        &got.server,
        SERVER,
        &ceiling(),
        ChannelLimits::default(),
    )
    .await
    .unwrap();
    for _ in 0..500 {
        if registry.views()[0].state == State::Connected {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(registry.views()[0].state, State::Connected);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn stopped_the_entry_closes_its_port_and_every_channel() {
    let f = fixture().await;
    let (keys, server) = paired(&f, "Acme").await;
    let ch = connect(&f, &keys, &server, SERVER).await.unwrap();
    until("connected", || view(&f, "Acme").state == State::Connected).await;
    let addr = f.entry.addr();
    let Fixture {
        entry, registry, ..
    } = f;
    entry.stop();
    tokio::time::timeout(Duration::from_secs(5), ch.closed())
        .await
        .expect("closed");
    assert!(registry.views()[0].state != State::Connected);
    assert!(TcpStream::connect(addr).await.is_err(), "its port closed");
}

/// A `CONNECT` to the gate with `token`: its status, its body, and the stream.
async fn gate_connect(gate: &Gate, token: &str, target: &str) -> (u16, String, TcpStream) {
    use base64::Engine as _;
    let mut s = TcpStream::connect(gate.addr()).await.unwrap();
    let auth = base64::engine::general_purpose::STANDARD.encode(format!("{token}:"));
    let ask = format!(
        "CONNECT {target} HTTP/1.1\r\nHost: {target}\r\nProxy-Authorization: Basic {auth}\r\n\r\n"
    );
    s.write_all(ask.as_bytes()).await.unwrap();
    let (head, rest) = escouade_gate::pipe::read_head(&mut s).await.unwrap();
    let head = String::from_utf8(head).unwrap();
    let status: u16 = head[9..12].parse().unwrap();
    let mut body = rest;
    if status != 200 {
        let _ = tokio::time::timeout(Duration::from_secs(2), s.read_to_end(&mut body)).await;
    }
    (status, String::from_utf8(body).unwrap(), s)
}
