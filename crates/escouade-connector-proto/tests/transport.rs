//! The outer transport, for real on 127.0.0.1: TLS signed by an authority made in the test (given to the connector by
//! a file of authorities), a fake company proxy (`CONNECT`, Basic), and a fake proxy that inspects TLS (it re-signs
//! with its own authority and reads everything in clear, as Zscaler does): the channel holds, and it reads nothing but
//! the WebSocket's upgrade. No network but the loopback; no key on disk but the test's temporary files.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use escouade_connector_proto::transport::{self, ClientConfig, Proxy};
use escouade_connector_proto::*;
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, DnType, IsCa, KeyPair, KeyUsagePurpose,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

const V: Version = Version::new(2, 3, 0);
const SECRET: &[u8] = b"a payload no proxy reads";

fn authority(name: &str) -> CertifiedIssuer<'static, KeyPair> {
    let mut p = CertificateParams::new(Vec::<String>::new()).unwrap();
    p.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    p.distinguished_name.push(DnType::CommonName, name);
    p.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    CertifiedIssuer::self_signed(p, KeyPair::generate().unwrap()).unwrap()
}

/// A certificate for `localhost` signed by `ca`, and its key, in PEM.
fn leaf(ca: &CertifiedIssuer<'static, KeyPair>) -> (String, String) {
    let key = KeyPair::generate().unwrap();
    let cert = CertificateParams::new(vec!["localhost".to_string()])
        .unwrap()
        .signed_by(&key, ca)
        .unwrap();
    (cert.pem(), key.serialize_pem())
}

struct Server {
    port: u16,
    public: PublicKey,
    channels: mpsc::UnboundedReceiver<ServerChannel>,
    refused: mpsc::UnboundedReceiver<AcceptError>,
}

/// `escouade-server`'s entry, as RP-4 will hold it: TLS, the upgrade on `/connector`, the handshake.
async fn server(ca: &CertifiedIssuer<'static, KeyPair>, pinned: PublicKey) -> Server {
    let (chain, key) = leaf(ca);
    let acceptor = transport::tls_acceptor(chain.as_bytes(), key.as_bytes()).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let keys = Arc::new(Keypair::generate());
    let public = keys.public();
    let (tx, channels) = mpsc::unbounded_channel();
    let (refused_tx, refused) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        loop {
            let (tcp, _) = listener.accept().await.unwrap();
            let (acceptor, keys, tx, refused_tx) = (
                acceptor.clone(),
                keys.clone(),
                tx.clone(),
                refused_tx.clone(),
            );
            tokio::spawn(async move {
                let Ok(tls) = acceptor.accept(tcp).await else {
                    return;
                };
                let ws = match transport::accept_websocket(tls, Duration::from_secs(10)).await {
                    Ok(ws) => ws,
                    Err(e) => {
                        let _ = refused_tx.send(e);
                        return;
                    }
                };
                match accept_connector(ws, &keys, |k| *k == pinned, V, Limits::default()).await {
                    Ok(ch) => {
                        let _ = tx.send(ch);
                    }
                    Err(e) => {
                        let _ = refused_tx.send(e);
                    }
                }
            });
        }
    });
    Server {
        port,
        public,
        channels,
        refused,
    }
}

fn ceiling() -> Ceiling {
    Ceiling {
        rules: vec!["gitlab.acme.internal:22".into()],
    }
}

/// The connector: connects as configured, then echoes every stream.
async fn connector(
    config: ClientConfig,
    keys: &Keypair,
    server: PublicKey,
) -> Result<(), ConnectError> {
    let ws = transport::connect(&config).await?;
    let mut ch = connect_server(ws, keys, &server, V, &ceiling(), Limits::default()).await?;
    tokio::spawn(async move {
        while let Some(incoming) = ch.next_open().await {
            let stream = incoming.accept();
            tokio::spawn(async move {
                let (mut r, mut w) = tokio::io::split(stream);
                let _ = tokio::io::copy(&mut r, &mut w).await;
                let _ = w.shutdown().await;
            });
        }
    });
    Ok(())
}

/// The server opens a stream through the connector, and the secret comes back.
async fn round_trip(server: &mut Server) {
    let ch = tokio::time::timeout(Duration::from_secs(20), server.channels.recv())
        .await
        .expect("a channel")
        .unwrap();
    ch.watch_ceiling()
        .wait_for(|c| c.as_ref() == Some(&ceiling()))
        .await
        .unwrap();
    let mut stream = ch
        .open("gitlab.acme.internal", 22, "agent", "Démo")
        .await
        .unwrap();
    stream.write_all(SECRET).await.unwrap();
    stream.shutdown().await.unwrap();
    let mut back = Vec::new();
    stream.read_to_end(&mut back).await.unwrap();
    assert_eq!(back, SECRET);
}

fn authorities_file(
    dir: &tempfile::TempDir,
    ca: &CertifiedIssuer<'static, KeyPair>,
) -> std::path::PathBuf {
    let path = dir.path().join("authorities.pem");
    std::fs::write(&path, ca.pem()).unwrap();
    path
}

fn config(port: u16, authorities: Option<std::path::PathBuf>) -> ClientConfig {
    let mut c = ClientConfig::new(format!("wss://localhost:{port}/connector"));
    c.system_roots = false;
    c.authorities = authorities;
    c.timeout = Duration::from_secs(20);
    c
}

#[tokio::test]
async fn the_connector_reaches_its_server_over_tls_with_a_file_of_authorities() {
    let ca = authority("Escouade test");
    let keys = Keypair::generate();
    let mut server = server(&ca, keys.public()).await;
    let dir = tempfile::tempdir().unwrap();
    connector(
        config(server.port, Some(authorities_file(&dir, &ca))),
        &keys,
        server.public,
    )
    .await
    .unwrap();
    round_trip(&mut server).await;

    // Without the authority: TLS refuses (the system's and Mozilla's roots do not know it).
    let mut bare = config(server.port, None);
    bare.system_roots = true;
    let refused = connector(bare, &keys, server.public).await.unwrap_err();
    assert!(
        matches!(&refused, ConnectError::Net(e) if e.contains("TLS")),
        "{refused:?}"
    );
}

#[tokio::test]
async fn another_path_is_not_the_entry() {
    let ca = authority("Escouade test");
    let keys = Keypair::generate();
    let mut server = server(&ca, keys.public()).await;
    let dir = tempfile::tempdir().unwrap();
    let mut c = config(server.port, Some(authorities_file(&dir, &ca)));
    c.url = format!("wss://localhost:{}/v1/live", server.port);
    let refused = connector(c, &keys, server.public).await.unwrap_err();
    assert!(
        matches!(&refused, ConnectError::Net(e) if e.contains("404")),
        "{refused:?}"
    );
    assert!(matches!(
        server.refused.recv().await,
        Some(AcceptError::Network(_))
    ));
}

#[tokio::test]
async fn a_key_the_server_does_not_know_gets_nothing_over_the_real_transport() {
    let ca = authority("Escouade test");
    let paired = Keypair::generate();
    let mut server = server(&ca, paired.public()).await;
    let dir = tempfile::tempdir().unwrap();
    let stranger = Keypair::generate();
    let refused = connector(
        config(server.port, Some(authorities_file(&dir, &ca))),
        &stranger,
        server.public,
    )
    .await
    .unwrap_err();
    assert_eq!(refused, ConnectError::NotPaired);
    let counted = server.refused.recv().await.unwrap();
    assert_eq!(counted, AcceptError::UnknownKey(stranger.public()));
    assert!(counted.is_failed_attempt());
}

// === The company's proxies ===

/// What a fake proxy saw.
#[derive(Default)]
struct Seen {
    heads: Vec<String>,
    clear: Vec<u8>,
}

async fn read_head(tcp: &mut TcpStream) -> String {
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        let mut b = [0u8; 1];
        if tcp.read(&mut b).await.unwrap() == 0 {
            break;
        }
        head.push(b[0]);
    }
    String::from_utf8_lossy(&head).into_owned()
}

/// Copies `from` to `to`, keeping what goes by in `seen`.
async fn tee<R, W>(mut from: R, mut to: W, seen: Arc<Mutex<Seen>>)
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut buf = vec![0u8; 16 << 10];
    loop {
        let n = match from.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        seen.lock().unwrap().clear.extend_from_slice(&buf[..n]);
        if to.write_all(&buf[..n]).await.is_err() {
            break;
        }
    }
    let _ = to.shutdown().await;
}

/// A company proxy on 127.0.0.1: `CONNECT` with Basic `alice:s3cret` alone; with `inspect`, it opens the TLS with its
/// own authority, as an inspection of TLS does, and reads all.
async fn proxy(
    inspect: Option<((String, String), rustls::RootCertStore)>,
) -> (u16, Arc<Mutex<Seen>>) {
    use base64::Engine as _;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen = Arc::new(Mutex::new(Seen::default()));
    let expected = format!(
        "Proxy-Authorization: Basic {}",
        base64::engine::general_purpose::STANDARD.encode("alice:s3cret")
    );
    let inspect = inspect.map(|((chain, key), upstream_roots)| {
        let acceptor = transport::tls_acceptor(chain.as_bytes(), key.as_bytes()).unwrap();
        let client = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(upstream_roots)
        .with_no_client_auth();
        (acceptor, tokio_rustls::TlsConnector::from(Arc::new(client)))
    });
    let record = seen.clone();
    tokio::spawn(async move {
        loop {
            let (mut tcp, _) = listener.accept().await.unwrap();
            let (seen, expected, inspect) = (record.clone(), expected.clone(), inspect.clone());
            tokio::spawn(async move {
                let head = read_head(&mut tcp).await;
                seen.lock().unwrap().heads.push(head.clone());
                if !head.lines().any(|l| l == expected) {
                    let _ = tcp
                        .write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\nProxy-Authenticate: Basic\r\n\r\n")
                        .await;
                    return;
                }
                let target = head.split_whitespace().nth(1).unwrap().to_string();
                let Ok(upstream) = TcpStream::connect(&target).await else {
                    let _ = tcp.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
                    return;
                };
                tcp.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
                    .await
                    .unwrap();
                match inspect {
                    None => {
                        let (a, b) = tokio::io::split(tcp);
                        let (c, d) = tokio::io::split(upstream);
                        tokio::join!(tee(a, d, seen.clone()), tee(c, b, seen));
                    }
                    Some((acceptor, connector)) => {
                        let Ok(inner) = acceptor.accept(tcp).await else {
                            return;
                        };
                        let name = rustls_pki_types::ServerName::try_from("localhost").unwrap();
                        let outer = connector.connect(name, upstream).await.unwrap();
                        let (a, b) = tokio::io::split(inner);
                        let (c, d) = tokio::io::split(outer);
                        tokio::join!(tee(a, d, seen.clone()), tee(c, b, seen));
                    }
                }
            });
        }
    });
    (port, seen)
}

#[tokio::test]
async fn the_connector_goes_through_a_company_proxy_with_basic() {
    let ca = authority("Escouade test");
    let keys = Keypair::generate();
    let mut server = server(&ca, keys.public()).await;
    let (proxy_port, seen) = proxy(None).await;
    let dir = tempfile::tempdir().unwrap();
    let mut c = config(server.port, Some(authorities_file(&dir, &ca)));
    c.proxy = Some(Proxy::parse(&format!("http://alice:s3cret@127.0.0.1:{proxy_port}")).unwrap());
    connector(c.clone(), &keys, server.public).await.unwrap();
    round_trip(&mut server).await;
    {
        let seen = seen.lock().unwrap();
        assert!(
            seen.heads[0].starts_with(&format!("CONNECT localhost:{} HTTP/1.1\r\n", server.port))
        );
        // Through the TLS, nothing in clear.
        let clear = String::from_utf8_lossy(&seen.clear);
        assert!(!clear.contains("connector") && !clear.contains("gitlab"));
    }

    // A wrong password: the proxy's 407 is said.
    c.proxy = Some(Proxy::parse(&format!("http://alice:wrong@127.0.0.1:{proxy_port}")).unwrap());
    let refused = connector(c, &keys, server.public).await.unwrap_err();
    assert!(
        matches!(&refused, ConnectError::Net(e) if e.contains("407") && e.contains("Basic")),
        "{refused:?}"
    );
}

#[tokio::test]
async fn an_inspection_of_tls_reads_nothing_but_the_upgrade() {
    let ca = authority("Escouade test");
    let acme = authority("Acme TLS inspection");
    let keys = Keypair::generate();
    let mut server = server(&ca, keys.public()).await;
    // The proxy trusts the server's real authority; the connector, Acme's alone (its file of authorities).
    let mut upstream = rustls::RootCertStore::empty();
    upstream.add(ca.der().clone()).unwrap();
    let (proxy_port, seen) = proxy(Some((leaf(&acme), upstream))).await;
    let dir = tempfile::tempdir().unwrap();
    let mut c = config(server.port, Some(authorities_file(&dir, &acme)));
    c.proxy = Some(Proxy::parse(&format!("http://alice:s3cret@127.0.0.1:{proxy_port}")).unwrap());
    connector(c, &keys, server.public).await.unwrap();
    round_trip(&mut server).await;

    let seen = seen.lock().unwrap();
    let clear = String::from_utf8_lossy(&seen.clear);
    // It reads the upgrade, as any HTTP…
    assert!(clear.contains("GET /connector HTTP/1.1"), "{clear}");
    // …then Noise alone: neither the host, nor the ceiling, nor the payload, nor a key.
    for hidden in ["gitlab.acme.internal", "Démo"] {
        assert!(!clear.contains(hidden), "{hidden} in clear");
    }
    assert!(!seen.clear.windows(SECRET.len()).any(|w| w == SECRET));
    assert!(!seen.clear.windows(32).any(|w| w == keys.public().0));
}
