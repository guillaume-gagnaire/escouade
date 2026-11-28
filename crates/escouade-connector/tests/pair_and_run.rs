//! End to end over the real transport on 127.0.0.1: a fake `escouade-server` entry in TLS (an authority made in the
//! test, given to the connector by `connector.toml`), `pair` with a code, then `run` and a stream to a host of the
//! ceiling. Neither the code nor the private key is in the journal, in what `pair` says, nor in a file but the key's.

mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::*;
use escouade_connector::commands;
use escouade_connector::config;
use escouade_connector::guard::Guards;
use escouade_connector::journal::Journal;
use escouade_connector::keys::State;
use escouade_connector::run::{self, Options, Wss};
use escouade_connector_proto::pairing::{self, CodeKey};
use escouade_connector_proto::transport::{self, ws_config};
use escouade_connector_proto::*;
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, DnType, IsCa, KeyPair, KeyUsagePurpose,
};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

const CODE: &str = "tstc-odex";

fn authority() -> CertifiedIssuer<'static, KeyPair> {
    let mut p = CertificateParams::new(Vec::<String>::new()).unwrap();
    p.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    p.distinguished_name
        .push(DnType::CommonName, "Escouade test");
    p.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    CertifiedIssuer::self_signed(p, KeyPair::generate().unwrap()).unwrap()
}

struct TlsServer {
    port: u16,
    channels: mpsc::UnboundedReceiver<ServerChannel>,
    paired: Arc<Mutex<Option<PublicKey>>>,
}

/// The entry of RP-4, as small as it can be: TLS, `/connector/pair` (one pending code) and `/connector`.
#[allow(clippy::result_large_err)]
async fn tls_server(ca: &CertifiedIssuer<'static, KeyPair>) -> TlsServer {
    let key = KeyPair::generate().unwrap();
    let cert = CertificateParams::new(vec!["localhost".to_string()])
        .unwrap()
        .signed_by(&key, ca)
        .unwrap();
    let acceptor =
        transport::tls_acceptor(cert.pem().as_bytes(), key.serialize_pem().as_bytes()).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let keys = Arc::new(Keypair::generate());
    let paired: Arc<Mutex<Option<PublicKey>>> = Arc::default();
    let (tx, channels) = mpsc::unbounded_channel();
    let p = paired.clone();
    tokio::spawn(async move {
        loop {
            let Ok((tcp, _)) = listener.accept().await else {
                return;
            };
            let (acceptor, keys, tx, paired) =
                (acceptor.clone(), keys.clone(), tx.clone(), p.clone());
            tokio::spawn(async move {
                let Ok(tls) = acceptor.accept(tcp).await else {
                    return;
                };
                let path = Arc::new(Mutex::new(String::new()));
                let seen = path.clone();
                let ws = tokio_tungstenite::accept_hdr_async_with_config(
                    tls,
                    move |req: &Request, resp: Response| {
                        *seen.lock().unwrap() = req.uri().path().to_string();
                        Ok(resp)
                    },
                    Some(ws_config()),
                )
                .await;
                let Ok(ws) = ws else { return };
                let path = path.lock().unwrap().clone();
                if path == pairing::pair_path(PATH) {
                    let pending = [CodeKey::from_code("ABCD-EFGH"), CodeKey::from_code(CODE)];
                    let pin = paired.clone();
                    let _ = pairing::accept_pairing(
                        ws,
                        &keys,
                        SERVER_V,
                        &pending,
                        move |_, k, _| {
                            *pin.lock().unwrap() = Some(*k);
                            Ok(())
                        },
                        Duration::from_secs(10),
                    )
                    .await;
                } else if path == PATH {
                    let known = |k: &PublicKey| *paired.lock().unwrap() == Some(*k);
                    if let Ok(ch) =
                        accept_connector(ws, &keys, known, SERVER_V, Limits::default()).await
                    {
                        let _ = tx.send(ch);
                    }
                }
            });
        }
    });
    TlsServer {
        port,
        channels,
        paired,
    }
}

#[tokio::test]
async fn pair_then_run_over_tls_and_no_secret_written() {
    let ca = authority();
    let mut server = tls_server(&ca).await;
    let echo = echo().await;
    let dir = tempfile::tempdir().unwrap();
    let authorities = dir.path().join("acme-root.pem");
    std::fs::write(&authorities, ca.pem()).unwrap();
    let address = format!("wss://localhost:{}/connector", server.port);
    let config_path = dir.path().join("connector.toml");
    std::fs::write(
        &config_path,
        format!(
            "server = '{address}'\nauthorities = '{}'\nallow = [\"git.acme.test:{}\"]\n",
            authorities.display(),
            echo.port
        ),
    )
    .unwrap();
    let (config, _) = config::load(&config_path).unwrap();
    let state = State::new(dir.path().join("state"));
    let version = escouade_connector::version();

    // A wrong code: said, nothing kept.
    let wrong = commands::pair(
        &address,
        "WXYZ-WXYZ",
        Some(&config),
        &state,
        version,
        false,
        &Wss,
    )
    .await
    .unwrap_err();
    assert!(wrong.contains("wrong, expired"), "{wrong}");
    assert!(state.load().unwrap().is_none());
    assert!(server.paired.lock().unwrap().is_none());

    // The right one, typed in lower case with its dash.
    let said = commands::pair(&address, CODE, Some(&config), &state, version, false, &Wss)
        .await
        .unwrap();
    assert!(said.contains(&format!("paired with {address}")), "{said}");
    let (pairing, keys) = state.load().unwrap().unwrap();
    assert_eq!(pairing.server, address);
    assert_eq!(Some(keys.public()), *server.paired.lock().unwrap());
    let again = commands::pair(&address, CODE, Some(&config), &state, version, false, &Wss)
        .await
        .unwrap_err();
    assert!(again.contains("already paired"), "{again}");

    // The service, over the same TLS.
    let dns = FakeDns::new();
    dns.set("git.acme.test", &[&["127.0.0.1"]]);
    let (journal, lines) = Journal::memory();
    let (stop, stop_rx) = tokio::sync::watch::channel(false);
    let task = tokio::spawn(run::run(
        Options {
            config_path: config_path.clone(),
            state: state.clone(),
            journal,
            resolver: dns,
            guards: Guards {
                loopback_for_tests: true,
            },
            version,
            system_roots: false,
            reload_every: Duration::from_millis(50),
        },
        Wss,
        stop_rx,
    ));
    let ch = tokio::time::timeout(Duration::from_secs(30), server.channels.recv())
        .await
        .expect("a channel over TLS")
        .unwrap();
    assert_eq!(ch.peer_key(), keys.public());
    assert_eq!(
        round_trip(&ch, "git.acme.test", echo.port, b"clone").await,
        b"clone"
    );
    wait_event(&lines, "closed", |_| true).await;
    let _ = stop.send(true);
    let _ = tokio::time::timeout(Duration::from_secs(20), task).await;

    // No secret anywhere but the key's own file.
    let private = hex::encode(keys.private_bytes());
    let code_forms = [
        CODE.to_string(),
        CODE.to_uppercase(),
        pairing::normalize_code(CODE),
        pairing::normalize_code(CODE).to_lowercase(),
    ];
    let mut written = vec![
        ("journal".to_string(), lines.text()),
        ("pair".to_string(), said),
    ];
    for entry in std::fs::read_dir(&state.dir).unwrap() {
        let path = entry.unwrap().path();
        written.push((
            path.display().to_string(),
            String::from_utf8_lossy(&std::fs::read(&path).unwrap()).into_owned(),
        ));
    }
    for (what, text) in &written {
        for code in &code_forms {
            assert!(!text.contains(code.as_str()), "the code in {what}");
        }
        let is_key_file = what.ends_with(escouade_connector::keys::KEY_FILE);
        if !is_key_file || cfg!(windows) {
            assert!(
                !text.to_lowercase().contains(&private),
                "the private key in {what}"
            );
        }
    }
    let text = lines.text();
    assert!(text.contains("\"event\":\"connected\""), "{text}");
    assert!(text.contains("\"event\":\"allowed\""));
}
