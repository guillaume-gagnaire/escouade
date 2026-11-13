//! The trust of the app's connections (audit SEC-07): the file of extra authorities, and
//! « Ignorer la vérification » held to the proxy.

use super::*;
use crate::integrations::fake::FakeServer;
use crate::model::Settings;
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

/// The PoC W-28 of the audit (SEC-28): the setting on, a proxy set, and a server of the machine excluded by a range of
/// addresses. Who decides the route and who decides the check read the exclusions alike (`proxy_for`): the server is
/// reached directly, and its certificate is checked, so refused.
#[tokio::test]
async fn a_host_excluded_by_a_range_goes_around_the_proxy_and_its_certificate_is_checked() {
    let server = FakeServer::start_tls().await;
    server.on("GET", "/ok", 200, serde_json::json!({}));
    let url = format!("{}/ok", server.url);
    let (proxy, opened) = tunnel_proxy().await;
    let mut settings = Settings {
        proxy_url: proxy,
        insecure_tls: true,
        ..Settings::default()
    };
    for around in [
        "127.0.0.0/8",
        "127.0.0.1/32",
        "10.0.0.0/8, 127.0.0.0/8",
        "127.0.0.1:443",
        "[127.0.0.1]",
    ] {
        settings.no_proxy = around.into();
        assert_eq!(status(&settings, &url).await, None, "{around}: unchecked");
        assert_eq!(*opened.lock().unwrap(), 0, "{around}: through the proxy");
    }
    // Out of the range, through the proxy: unchecked, as the setting says.
    settings.no_proxy = "10.0.0.0/8".into();
    assert_eq!(status(&settings, &url).await, Some(200));
    assert_eq!(*opened.lock().unwrap(), 1);
}

/// The one reading of the exclusions (SEC-28): what a proxy's tools write, and what is no exclusion of a host.
#[test]
fn the_exclusions_are_read_as_written_by_the_tools_of_a_proxy() {
    for (no_proxy, host) in [
        ("*", "example.com"),
        ("internal.example", "internal.example"),
        ("internal.example", "INTERNAL.example"),
        ("internal.example", "a.internal.example"),
        (".corp.example", "x.corp.example"),
        ("*.corp.example", "x.corp.example"),
        ("internal.example:8443", "internal.example"),
        ("internal.example.", "internal.example"),
        ("internal.example", "internal.example."),
        ("10.0.0.0/8", "10.1.2.3"),
        ("127.0.0.0/8", "127.0.0.1"),
        ("10.1.2.3", "10.1.2.3"),
        ("10.1.2.3:8080", "10.1.2.3"),
        ("[::1]", "::1"),
        ("::1", "[::1]"),
        ("[::1]:443", "::1"),
        ("fd00::/8", "fd12::1"),
        ("10.0.0.0/8", "::ffff:10.1.2.3"),
        ("a.example;b.example 10.0.0.0/8,c.example", "10.200.0.1"),
    ] {
        assert!(
            excluded(no_proxy, host),
            "{no_proxy} does not exclude {host}"
        );
    }
    for (no_proxy, host) in [
        ("", "example.com"),
        ("internal.example", "notinternal.example"),
        ("internal.example", "internal.example.evil.test"),
        ("10.0.0.0/8", "11.1.2.3"),
        ("10.0.0.0/8", "10.example"),
        ("10.1.2.3", "10.1.2.30"),
        ("10.0.0.0/33", "10.1.2.3"),
        ("fd00::/8", "fe80::1"),
        ("*.corp.example", "corp.example.evil"),
        ("10.0.0.0", "10.0.0.0.evil.test"),
    ] {
        assert!(!excluded(no_proxy, host), "{no_proxy} excludes {host}");
    }
    // The route and the check ask the same: through the proxy what is not excluded, around it what is.
    let net = Net {
        proxy: "http://127.0.0.1:9000".into(),
        no_proxy: "10.0.0.0/8".into(),
        insecure: true,
        extra_ca: String::new(),
    };
    assert!(proxy_for(&net, "10.1.2.3").is_none());
    assert!(proxy_for(&net, "example.com").is_some());
    assert!(proxy(&net).is_some());
    assert!(proxy(&Net::default()).is_none());
}
