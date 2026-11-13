//! The trust of the app's connections (audit SEC-07): the file of extra authorities, and
//! « Ignorer la vérification » held to the proxy.

use super::*;
use crate::integrations::fake::FakeServer;
use crate::model::Settings;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// A proxy that opens the tunnels it is asked for (`CONNECT host:port`), as Zscaler's local
/// agent does, and counts them.
async fn tunnel_proxy() -> (String, Arc<Mutex<u32>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let at = format!("http://{}", listener.local_addr().unwrap());
    let opened = Arc::new(Mutex::new(0));
    let count = opened.clone();
    tokio::spawn(async move {
        while let Ok((mut client, _)) = listener.accept().await {
            let count = count.clone();
            tokio::spawn(async move {
                let mut head = Vec::new();
                let mut byte = [0u8; 1];
                while !head.ends_with(b"\r\n\r\n") {
                    if client.read(&mut byte).await.unwrap_or(0) == 0 {
                        return;
                    }
                    head.push(byte[0]);
                }
                let head = String::from_utf8_lossy(&head).to_string();
                let Some(target) = head
                    .strip_prefix("CONNECT ")
                    .and_then(|r| r.split(' ').next())
                else {
                    return;
                };
                let Ok(mut server) = tokio::net::TcpStream::connect(target).await else {
                    return;
                };
                *count.lock().unwrap() += 1;
                client
                    .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
                    .await
                    .unwrap();
                let _ = tokio::io::copy_bidirectional(&mut client, &mut server).await;
            });
        }
    });
    (at, opened)
}

async fn status(settings: &Settings, url: &str) -> Option<u16> {
    let client = app_builder(settings).unwrap().build().unwrap();
    client
        .get(url)
        .send()
        .await
        .ok()
        .map(|r| r.status().as_u16())
}

fn pem_file(pem: &str) -> String {
    let dir = std::env::temp_dir().join(format!("tls-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("zscaler-root.pem");
    std::fs::write(&path, pem).unwrap();
    path.to_string_lossy().into_owned()
}

#[tokio::test]
async fn the_file_of_extra_authorities_makes_the_app_trust_a_proxys_root() {
    let server = FakeServer::start_tls().await;
    server.on("GET", "/ok", 200, serde_json::json!({}));
    let url = format!("{}/ok", server.url);
    let mut settings = Settings {
        no_proxy: String::new(),
        ..Settings::default()
    };
    // Unknown to every store: refused.
    assert_eq!(status(&settings, &url).await, None);
    settings.extra_ca_file = pem_file(&server.cert_pem);
    assert_eq!(status(&settings, &url).await, Some(200));
    // The usual authorities still hold with it: the roots are added, not replaced.
    let net = Net::of(&settings);
    assert!(config(&net).alpn_protocols == vec![b"http/1.1".to_vec()]);
}

#[test]
fn a_file_with_no_certificate_is_told() {
    let path = pem_file("pas un certificat");
    let e = read_authorities(&path).unwrap_err().to_string();
    assert!(e.contains(&path), "{e}");
    assert!(read_authorities(&format!("{path}.absent")).is_err());
}

#[tokio::test]
async fn ignoring_the_certificates_does_nothing_without_a_proxy() {
    let server = FakeServer::start_tls().await;
    server.on("GET", "/ok", 200, serde_json::json!({}));
    let settings = Settings {
        insecure_tls: true,
        no_proxy: String::new(),
        ..Settings::default()
    };
    assert!(!Net::of(&settings).unchecked_by_proxy());
    assert_eq!(status(&settings, &format!("{}/ok", server.url)).await, None);
}

#[tokio::test]
async fn ignoring_the_certificates_holds_through_the_proxy_alone() {
    let server = FakeServer::start_tls().await;
    server.on("GET", "/ok", 200, serde_json::json!({}));
    let url = format!("{}/ok", server.url);
    let (proxy, opened) = tunnel_proxy().await;
    let mut settings = Settings {
        proxy_url: proxy,
        no_proxy: String::new(),
        insecure_tls: false,
        ..Settings::default()
    };
    // Through the proxy, checked: refused.
    assert_eq!(status(&settings, &url).await, None);
    // Through the proxy, unchecked: answered, by the tunnel.
    settings.insecure_tls = true;
    assert_eq!(status(&settings, &url).await, Some(200));
    let through = *opened.lock().unwrap();
    assert!(through >= 1);
    // A host of the exclusions goes around the proxy, and its certificate is checked again.
    settings.no_proxy = "127.0.0.1".into();
    assert_eq!(status(&settings, &url).await, None);
    assert_eq!(*opened.lock().unwrap(), through);
}

#[test]
fn the_agents_get_the_file_and_the_unchecked_tls_only_with_a_proxy() {
    let has = |env: &[(String, String)], k: &str| env.iter().any(|(n, _)| n == k);
    let mut s = Settings {
        insecure_tls: true,
        ..Settings::default()
    };
    // No proxy: the setting does nothing, for the agents as for the app.
    assert!(!has(&s.agent_env(), "NODE_TLS_REJECT_UNAUTHORIZED"));
    s.proxy_url = "http://127.0.0.1:9000".into();
    assert!(has(&s.agent_env(), "NODE_TLS_REJECT_UNAUTHORIZED"));
    // The file goes to Node's harnesses and to Codex, and to the plugins' programs.
    s.insecure_tls = false;
    s.extra_ca_file = r"C:\certs\zscaler.pem".into();
    for env in [s.agent_env(), s.claude_env(), s.network_env()] {
        for k in crate::model::EXTRA_CA_VARS {
            assert!(
                env.contains(&(k.to_string(), r"C:\certs\zscaler.pem".to_string())),
                "{k}"
            );
        }
        assert!(!has(&env, "NODE_TLS_REJECT_UNAUTHORIZED"));
    }
}
