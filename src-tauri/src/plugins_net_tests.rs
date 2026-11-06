//! The network of the host (ESC-332, docs/PLUGINS.md §8.4): `net/fetch` revised (`{account}`, the proxy and the TLS of the
//! settings, `bodyBase64`, the limits), `net/stream`, the WebSocket, `local:shared` and the timers, each rule and each
//! refusal, on fake local servers. The module of tests/fixtures/plugins/reactor (`probe/host`) plays what lasts: `run.js`
//! is played by `plugins_wasm_tests.rs`, on the same host.

use crate::plugins::bench::fake::{Reach, Seen, Session};
use crate::plugins::manifest::{self, Hand, Limits as Declared};
use crate::plugins::socket::Net as Settings;
use crate::plugins::wasm::{self, Dns, Fetcher, HostsOf, Net, PortsOf, FAILED, REFUSED};
use crate::plugins_reactor_tests::{fixture, Served};
use futures_util::{SinkExt, StreamExt};
use parking_lot::RwLock;
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio_tungstenite::tungstenite::Message;

const ID: &str = "dev.test.reactor";

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn hosts(list: &[String]) -> HostsOf {
    let list = list.to_vec();
    Arc::new(move |_| list.clone())
}

fn to_machine() -> Option<Dns> {
    Some(Arc::new(|_| vec!["127.0.0.1".parse().unwrap()]))
}

/// A fake server: `serve` gets the first line of each request, its body, and the connection.
fn server(serve: impl Fn(String, Vec<u8>, TcpStream) + Send + Sync + 'static) -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let serve = Arc::new(serve);
    std::thread::spawn(move || {
        for stream in l.incoming().flatten() {
            let serve = serve.clone();
            std::thread::spawn(move || {
                let mut r = BufReader::new(stream.try_clone().unwrap());
                let (mut first, mut len) = (String::new(), 0);
                if r.read_line(&mut first).unwrap_or(0) == 0 {
                    return;
                }
                loop {
                    let mut h = String::new();
                    if r.read_line(&mut h).unwrap_or(0) == 0 || h.trim().is_empty() {
                        break;
                    }
                    if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap_or(0);
                    }
                }
                let mut body = vec![0; len];
                let _ = r.read_exact(&mut body);
                serve(first, body, stream);
            });
        }
    });
    port
}

fn answer(mut s: TcpStream, body: &[u8]) {
    let _ = write!(
        s,
        "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    );
    let _ = s.write_all(body);
}

/// A server that answers `ok`, and tells what it was asked.
fn plain() -> (u16, Arc<Mutex<Vec<String>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let told = seen.clone();
    let port = server(move |first, _, s| {
        told.lock().unwrap().push(first);
        answer(s, b"ok");
    });
    (port, seen)
}

fn get(url: &str) -> Value {
    json!({ "url": url })
}

// ---- {account} ---------------------------------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn the_host_of_an_account_is_reached_in_http_on_the_local_network_and_it_alone() {
    let (port, _) = plain();
    let net = Net {
        accounts: Some(hosts(&[format!("127.0.0.1:{port}")])),
        ..Net::default()
    };
    let f = Fetcher::new(ID, &words(&["{account}"]), &net);
    let at = format!("http://127.0.0.1:{port}");
    let r = f.fetch(&get(&format!("{at}/api"))).await.unwrap();
    assert_eq!(
        (r["status"].as_u64(), r["body"].as_str()),
        (Some(200), Some("ok"))
    );
    // Another port of that host, another address of the machine.
    let (other, _) = plain();
    for url in [
        format!("http://127.0.0.1:{other}/"),
        "http://127.0.0.1/".to_string(),
        format!("http://localhost:{port}/"),
        "http://192.168.1.1/".to_string(),
    ] {
        let (code, why) = f.fetch(&get(&url)).await.unwrap_err();
        assert_eq!(code, REFUSED, "{url}: {why}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn an_account_opens_a_name_of_the_local_network_but_none_of_its_subdomains() {
    let (port, _) = plain();
    let net = Net {
        accounts: Some(hosts(&[format!("git.acme.test:{port}")])),
        dns: to_machine(),
        ..Net::default()
    };
    let f = Fetcher::new(ID, &words(&["{account}", "acme.dev"]), &net);
    let ok = f
        .fetch(&get(&format!("http://git.acme.test:{port}/")))
        .await;
    assert_eq!(ok.unwrap()["status"], 200);
    let (code, _) = f
        .fetch(&get(&format!("http://sub.git.acme.test:{port}/")))
        .await
        .unwrap_err();
    assert_eq!(code, REFUSED);
    // A host of the manifest gets nothing of it: no http, no address of the machine.
    let (code, why) = f.fetch(&get("http://api.acme.dev/")).await.unwrap_err();
    assert_eq!(code, REFUSED);
    assert!(why.contains("only https"), "{why}");
    let (code, why) = f.fetch(&get("https://api.acme.dev/")).await.unwrap_err();
    assert_eq!(code, REFUSED);
    assert!(why.contains("127.0.0.1"), "{why}");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_account_opens_nothing_unless_declared_and_only_while_it_is_there() {
    let (port, _) = plain();
    let typed = Arc::new(Mutex::new(vec![format!("127.0.0.1:{port}")]));
    let now = typed.clone();
    let net = Net {
        accounts: Some(Arc::new(move |_| now.lock().unwrap().clone())),
        ..Net::default()
    };
    let url = format!("http://127.0.0.1:{port}/");
    let undeclared = Fetcher::new(ID, &words(&["acme.dev"]), &net);
    assert_eq!(undeclared.fetch(&get(&url)).await.unwrap_err().0, REFUSED);
    let f = Fetcher::new(ID, &words(&["{account}"]), &net);
    assert_eq!(f.fetch(&get(&url)).await.unwrap()["status"], 200);
    typed.lock().unwrap().clear();
    assert_eq!(f.fetch(&get(&url)).await.unwrap_err().0, REFUSED);
}

#[test]
fn the_hosts_of_an_account_are_those_the_user_typed_for_this_plugin() {
    use crate::forge::accounts::ForgeAccount;
    use crate::integrations::{Account, Accounts};
    use crate::plugins::accounts::hosts_of;
    let forge = |kind: &str, url: &str, plugin: &str| ForgeAccount {
        kind: kind.into(),
        url: url.into(),
        plugin: plugin.into(),
        ..ForgeAccount::default()
    };
    let accounts = Accounts {
        forges: vec![
            forge("gitlab", "http://git.acme.dev:8080/", ""),
            forge("gitlab", "https://gitlab.exemple.dev", "dev.exemple.gitlab"),
        ],
        jira: Some(Account {
            site: "https://Atlas.atlassian.net/".into(),
            ..Account::default()
        }),
        ..Accounts::default()
    };
    assert_eq!(
        hosts_of(&accounts, "dev.escouade.gitlab"),
        ["git.acme.dev:8080"]
    );
    assert_eq!(
        hosts_of(&accounts, "dev.exemple.gitlab"),
        ["gitlab.exemple.dev"]
    );
    assert_eq!(
        hosts_of(&accounts, "dev.escouade.jira"),
        ["atlas.atlassian.net"]
    );
    assert!(hosts_of(&accounts, "dev.exemple.other").is_empty());
}

// ---- The settings' network -------------------------------------------------------------------------------------

/// A proxy that says what it was asked, then what the test wants.
fn fake_proxy(reply: &'static [u8]) -> (u16, Arc<Mutex<Vec<String>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let told = seen.clone();
    let port = server(move |first, _, mut s| {
        told.lock().unwrap().push(first.trim().to_string());
        let _ = s.write_all(reply);
    });
    (port, seen)
}

const OK: &[u8] = b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\nby";

fn proxied(proxy: &str, no_proxy: &str) -> Net {
    Net {
        network: Arc::new(RwLock::new(Settings {
            proxy: proxy.into(),
            no_proxy: no_proxy.into(),
            insecure: false,
        })),
        ..Net::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_call_goes_through_the_proxy_of_the_settings_and_the_journal_says_its_address_was_not_judged(
) {
    let (port, seen) = fake_proxy(OK);
    let mut net = proxied(&format!("127.0.0.1:{port}"), "");
    net.accounts = Some(hosts(&["git.acme.test".to_string()]));
    let f = Fetcher::new(ID, &words(&["{account}", "api.exemple.dev"]), &net);
    let r = f.fetch(&get("http://git.acme.test/x")).await.unwrap();
    assert_eq!(r["body"], "by");
    assert_eq!(
        seen.lock().unwrap()[0],
        "GET http://git.acme.test/x HTTP/1.1"
    );
    // In https: a tunnel to its name, which the proxy resolves.
    let (code, why) = f.fetch(&get("https://api.exemple.dev/")).await.unwrap_err();
    assert_eq!(code, FAILED, "{why}");
    assert_eq!(
        seen.lock().unwrap()[1],
        "CONNECT api.exemple.dev:443 HTTP/1.1"
    );
    let said = f.describe(&get("https://api.exemple.dev/?key=s3cret"), "GET");
    assert_eq!(
        said,
        "GET api.exemple.dev (by the proxy: its name judged, not its addresses)"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_hosts_no_proxy_names_go_straight_and_their_addresses_are_judged() {
    let (port, seen) = fake_proxy(OK);
    let mut net = proxied(&format!("127.0.0.1:{port}"), "exemple.dev");
    net.dns = to_machine();
    let f = Fetcher::new(ID, &words(&["api.exemple.dev"]), &net);
    let (code, why) = f.fetch(&get("https://api.exemple.dev/")).await.unwrap_err();
    assert_eq!(code, REFUSED, "{why}");
    assert!(seen.lock().unwrap().is_empty());
    assert!(!f
        .describe(&get("https://api.exemple.dev/"), "GET")
        .contains("proxy"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_proxy_of_the_local_network_is_reached_by_its_name_but_never_lets_localhost_through() {
    let (port, seen) = fake_proxy(OK);
    let mut net = proxied(&format!("http://proxy.corp.test:{port}"), "");
    net.dns = to_machine();
    let f = Fetcher::new(ID, &words(&["{account}", "exemple.dev", "localhost"]), &net);
    let (code, why) = f.fetch(&get("https://api.exemple.dev/")).await.unwrap_err();
    assert_eq!(code, FAILED, "{why}");
    assert_eq!(seen.lock().unwrap().len(), 1, "{why}");
    let (code, why) = f.fetch(&get("https://localhost/")).await.unwrap_err();
    assert_eq!(code, REFUSED, "{why}");
    assert_eq!(seen.lock().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_change_of_the_settings_reaches_the_next_call_of_an_instance_that_runs() {
    let (port, seen) = fake_proxy(OK);
    let (direct, _) = plain();
    let net = Net {
        accounts: Some(hosts(&[format!("127.0.0.1:{direct}")])),
        ..Net::default()
    };
    let settings = net.network.clone();
    let f = Fetcher::new(ID, &words(&["{account}"]), &net);
    let url = format!("http://127.0.0.1:{direct}/");
    assert_eq!(f.fetch(&get(&url)).await.unwrap()["body"], "ok");
    settings.write().proxy = format!("127.0.0.1:{port}");
    assert_eq!(f.fetch(&get(&url)).await.unwrap()["body"], "by");
    assert_eq!(seen.lock().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_proxy_that_is_no_address_fails_and_says_where_to_look() {
    let net = proxied("http://exa mple", "");
    let f = Fetcher::new(ID, &words(&["exemple.dev"]), &net);
    let (code, why) = f.fetch(&get("https://api.exemple.dev/")).await.unwrap_err();
    assert_eq!(code, FAILED);
    assert!(why.contains("Réglages"), "{why}");
}

/// A server of the certificate a corporate proxy would show: self-signed, for `localhost`.
fn tls_acceptor() -> tokio_rustls::TlsAcceptor {
    use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
    let made = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(made.key_pair.serialize_der()));
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(vec![made.cert.der().clone()], key)
    .unwrap();
    tokio_rustls::TlsAcceptor::from(Arc::new(config))
}

#[tokio::test(flavor = "multi_thread")]
async fn the_certificates_are_checked_unless_the_settings_say_not_to() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let tls = tls_acceptor();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let tls = tls.clone();
            tokio::spawn(async move {
                let Ok(mut s) = tls.accept(tcp).await else {
                    return;
                };
                let mut buf = [0; 2048];
                let _ = s.read(&mut buf).await;
                let _ = s.write_all(OK).await;
                let _ = s.shutdown().await;
            });
        }
    });
    let net = Net {
        accounts: Some(hosts(&[format!("localhost:{port}")])),
        ..Net::default()
    };
    let settings = net.network.clone();
    let f = Fetcher::new(ID, &words(&["{account}"]), &net);
    let url = format!("https://localhost:{port}/");
    let (code, _) = f.fetch(&get(&url)).await.unwrap_err();
    assert_eq!(code, FAILED);
    settings.write().insecure = true;
    assert_eq!(f.fetch(&get(&url)).await.unwrap()["body"], "by");
}

// ---- Bodies and limits ---------------------------------------------------------------------------------------------

fn fake_routes(port: u16) -> Net {
    Net {
        routes: HashMap::from([(
            "api.exemple.dev".to_string(),
            format!("http://127.0.0.1:{port}"),
        )]),
        ..Net::default()
    }
}

fn routed(port: u16) -> Fetcher {
    Fetcher::new(ID, &words(&["api.exemple.dev"]), &fake_routes(port))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_binary_body_goes_and_comes_in_base64_and_a_text_stays_text() {
    let port = server(|_, body, s| answer(s, &body));
    let f = routed(port);
    let url = "https://api.exemple.dev/echo";
    let r = f
        .fetch(&json!({ "url": url, "method": "POST", "bodyBase64": "/wAB" }))
        .await
        .unwrap();
    assert_eq!(r["bodyBase64"], "/wAB");
    assert!(r.get("body").is_none(), "{r}");
    let r = f
        .fetch(&json!({ "url": url, "method": "POST", "bodyBase64": "aOk=" }))
        .await
        .unwrap();
    assert!(r.get("body").is_none(), "{r}");
    let r = f
        .fetch(&json!({ "url": url, "method": "POST", "body": "héllo" }))
        .await
        .unwrap();
    assert_eq!(r["body"], "héllo");
    assert!(r.get("bodyBase64").is_none(), "{r}");
    for bad in [
        json!({ "url": url, "body": "a", "bodyBase64": "YQ==" }),
        json!({ "url": url, "bodyBase64": "%%" }),
    ] {
        let (code, _) = f.fetch(&bad).await.unwrap_err();
        assert_eq!(code, crate::plugins::runtime::INVALID_PARAMS, "{bad}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_limits_are_those_of_the_manifest_within_those_of_the_host() {
    let port = server(|first, _, s| {
        if first.contains("/slow") {
            std::thread::sleep(Duration::from_millis(2500));
        }
        answer(s, &vec![b'a'; 2 << 20]);
    });
    let url = "https://api.exemple.dev/big";
    let plain = routed(port);
    let (code, why) = plain.fetch(&get(url)).await.unwrap_err();
    assert_eq!(code, FAILED);
    assert!(why.ends_with("an answer over 1 MiB"), "{why}");
    let more = routed(port).limited(Declared {
        fetch_bytes: Some(3 << 20),
        fetch_seconds: None,
        ..Declared::default()
    });
    assert_eq!(more.fetch(&get(url)).await.unwrap()["status"], 200);
    let quick = routed(port).limited(Declared {
        fetch_seconds: Some(1),
        fetch_bytes: Some(3 << 20),
        ..Declared::default()
    });
    let (code, why) = quick
        .fetch(&get("https://api.exemple.dev/slow"))
        .await
        .unwrap_err();
    assert_eq!(code, FAILED);
    assert!(why.contains("no answer within 1 s"), "{why}");
    let asked = routed(port).limited(Declared {
        fetch_seconds: Some(3600),
        fetch_bytes: Some(1 << 40),
        ..Declared::default()
    });
    assert_eq!(asked.seconds(), Duration::from_secs(120));
    assert_eq!(asked.bytes(), 32 << 20);
    assert_eq!(routed(port).seconds(), Duration::from_secs(10));
}

#[test]
fn the_manifest_reads_its_limits_and_refuses_what_is_no_positive_integer() {
    let with = |limits: Value| {
        let mut m = json!({
            "id": "dev.test.limits",
            "name": "Limites",
            "version": "1.0.0",
            "escouade": { "api": "1", "min": "2.1.0" },
        });
        m["limits"] = limits;
        manifest::check(&m, Hand::Third)
    };
    let read = with(json!({ "fetchSeconds": 120, "fetchBytes": 33554432 }));
    let limits = read.manifest.unwrap().limits;
    assert_eq!(limits.fetch_seconds, Some(120));
    assert_eq!(limits.fetch_bytes, Some(33_554_432));
    for bad in [json!({ "fetchSeconds": 0 }), json!({ "fetchBytes": "1" })] {
        assert!(with(bad.clone()).manifest.is_err(), "{bad}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failure_never_says_the_query_of_its_url() {
    let f = Fetcher::new(ID, &words(&["{account}"]), &{
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        drop(l);
        Net {
            accounts: Some(hosts(&[format!("127.0.0.1:{port}")])),
            ..fake_routes(0)
        }
    });
    let typed = f.describe(&get("http://x/"), "GET");
    assert_eq!(typed, "GET x");
    let net = Net {
        accounts: Some(hosts(&["127.0.0.1:1".to_string()])),
        ..Net::default()
    };
    let f = Fetcher::new(ID, &words(&["{account}"]), &net);
    let (code, why) = f
        .fetch(&get("http://127.0.0.1:1/x?token=s3cret"))
        .await
        .unwrap_err();
    assert_eq!(code, FAILED, "{why}");
    assert!(!why.contains("s3cret"), "{why}");
    let (code, why) = f
        .fetch(&get("http://elsewhere.dev/x?token=s3cret"))
        .await
        .unwrap_err();
    assert_eq!(code, REFUSED, "{why}");
    assert!(!why.contains("s3cret"), "{why}");
}

// ---- Through the module ------------------------------------------------------------------------------------------

/// The fixture's module, asked by `probe/host`; what the host tells it comes back as `probe/seen`.
struct Probe {
    served: Served,
    next: Cell<u64>,
    told: RefCell<VecDeque<Value>>,
}

impl Probe {
    fn new(fetcher: Fetcher) -> Self {
        Self {
            served: Served::start(fetcher, wasm::Limits::default(), None, Vec::new()),
            next: Cell::new(1),
            told: RefCell::default(),
        }
    }

    fn line(&self, within: Duration) -> Option<Value> {
        let l = self.served.lines.recv_timeout(within).ok()?;
        Some(serde_json::from_str(&l).expect("JSON"))
    }

    /// The host's answer to `method`: `{result}` or `{error}`, what it told meanwhile kept.
    fn ask(&self, method: &str, params: Value) -> Value {
        let id = self.next.get();
        self.next.set(id + 1);
        self.served.send(
            id,
            "probe/host",
            json!({ "method": method, "params": params }),
        );
        loop {
            let l = self.line(Duration::from_secs(20)).expect("an answer");
            if l["id"] == json!(id) {
                return l["result"].clone();
            }
            self.told.borrow_mut().push_back(l["params"].clone());
        }
    }

    /// What the host tells next: its line as it came.
    fn next_told(&self, within: Duration) -> Option<Value> {
        if let Some(t) = self.told.borrow_mut().pop_front() {
            return Some(t);
        }
        self.line(within).map(|l| l["params"].clone())
    }

    fn told(&self, method: &str) -> Value {
        let t = self.next_told(Duration::from_secs(20)).expect(method);
        assert_eq!(t["method"], method, "{t}");
        t["params"].clone()
    }
}

/// A response without a length: `serve` writes it, and it ends with the connection.
fn streaming(serve: impl Fn(String, &mut TcpStream) + Send + Sync + 'static) -> u16 {
    server(move |first, _, mut s| {
        let _ = write!(s, "HTTP/1.1 200 OK\r\nx-kind: test\r\n\r\n");
        let _ = s.flush();
        serve(first, &mut s);
    })
}

fn events() -> u16 {
    streaming(|_, s| {
        let _ = s.write_all(b"data: one\n\n");
        let _ = s.flush();
        std::thread::sleep(Duration::from_millis(700));
        let _ = s.write_all(b"data: two\n\n");
    })
}

fn silent(port: u16) -> Fetcher {
    let net = Net {
        idle: Some(Duration::from_millis(200)),
        ..fake_routes(port)
    };
    Fetcher::new(ID, &words(&["api.exemple.dev"]), &net)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_stream_declared_sse_comes_in_pieces_and_stays_however_silent() {
    let p = Probe::new(silent(events()));
    let opened = p.ask(
        "net/stream",
        json!({ "url": "https://api.exemple.dev/events", "sse": true }),
    );
    let stream = opened["result"]["stream"].clone();
    assert_eq!(opened["result"]["status"], 200, "{opened}");
    assert_eq!(opened["result"]["headers"]["x-kind"], "test");
    let mut text = String::new();
    while !text.contains("two") {
        let t = p.told("net/chunk");
        assert_eq!(t["stream"], stream);
        text.push_str(t["data"].as_str().unwrap());
    }
    assert_eq!(text, "data: one\n\ndata: two\n\n");
    let end = p.told("net/end");
    assert_eq!(end, json!({ "stream": stream }));
    let said = p.served.said.lock().unwrap().join("\n");
    assert!(said.contains("net/stream api.exemple.dev: 200"), "{said}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_response_without_a_byte_for_too_long_is_closed_and_says_why() {
    let p = Probe::new(silent(events()));
    let opened = p.ask(
        "net/stream",
        json!({ "url": "https://api.exemple.dev/events" }),
    );
    assert!(opened["result"]["stream"].is_u64(), "{opened}");
    assert_eq!(p.told("net/chunk")["data"], "data: one\n\n");
    let end = p.told("net/end");
    assert!(
        end["error"].as_str().unwrap().contains("no byte for"),
        "{end}"
    );
    assert_eq!(p.served.refused.lock().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_stream_follows_the_rules_of_a_fetch_at_its_opening() {
    let p = Probe::new(silent(events()));
    for (url, why) in [
        ("http://api.exemple.dev/", "only https"),
        ("https://other.dev/", "permissions.network"),
        ("https://127.0.0.1/", "permissions.network"),
    ] {
        let a = p.ask("net/stream", json!({ "url": url }));
        assert_eq!(a["error"]["code"], REFUSED, "{a}");
        assert!(a["error"]["message"].as_str().unwrap().contains(why), "{a}");
    }
    assert_eq!(p.served.refused.lock().unwrap().len(), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_slow_plugin_holds_back_the_stream_at_its_credit() {
    let port = streaming(|_, s| {
        for _ in 0..16 {
            if s.write_all(&vec![b'a'; 64 << 10]).is_err() {
                return;
            }
        }
    });
    let p = Probe::new(routed(port));
    let opened = p.ask(
        "net/stream",
        json!({ "url": "https://api.exemple.dev/big" }),
    );
    let stream = opened["result"]["stream"].clone();
    let upto = |want: usize| {
        let mut got = 0;
        while got < want {
            got += p.told("net/chunk")["data"].as_str().unwrap().len();
        }
        assert_eq!(got, want);
        assert!(
            p.next_told(Duration::from_millis(300)).is_none(),
            "nothing past the credit"
        );
    };
    upto(256 << 10);
    p.ask("net/credit", json!({ "stream": stream, "bytes": 100_000 }));
    upto(100_000);
    // Giving back more than it was handed buys nothing.
    p.ask("net/credit", json!({ "stream": stream, "bytes": 1 << 30 }));
    upto(256 << 10);
    p.ask("net/credit", json!({ "stream": stream, "bytes": 1 << 30 }));
    let mut got = 0;
    loop {
        let t = p.next_told(Duration::from_secs(20)).expect("the rest");
        if t["method"] == "net/end" {
            break;
        }
        let bytes = t["params"]["data"].as_str().unwrap().len();
        got += bytes;
        p.ask("net/credit", json!({ "stream": stream, "bytes": bytes }));
    }
    assert_eq!(got, (1 << 20) - (256 << 10) - 100_000 - (256 << 10));
}

#[tokio::test(flavor = "multi_thread")]
async fn eight_streams_at_most_and_a_cancelled_one_makes_room_and_says_nothing_more() {
    let port = streaming(|_, s| {
        while s.write_all(b".").is_ok() {
            std::thread::sleep(Duration::from_millis(20));
        }
    });
    let p = Probe::new(routed(port));
    let open = || p.ask("net/stream", json!({ "url": "https://api.exemple.dev/n" }));
    let first = open()["result"]["stream"].clone();
    for _ in 1..8 {
        assert!(open()["result"]["stream"].is_u64());
    }
    let ninth = open();
    assert_eq!(ninth["error"]["code"], REFUSED, "{ninth}");
    p.ask("net/cancel", json!({ "stream": first }));
    assert!(open()["result"]["stream"].is_u64());
    // A stream of nothing, a parameter that does not hold.
    p.ask("net/cancel", json!({ "stream": 12345 }));
    let bad = p.ask("net/credit", json!({ "stream": "a", "bytes": 1 }));
    assert_eq!(
        bad["error"]["code"],
        crate::plugins::runtime::INVALID_PARAMS,
        "{bad}"
    );
    let started = Instant::now();
    while started.elapsed() < Duration::from_millis(300) {
        if let Some(t) = p.next_told(Duration::from_millis(50)) {
            assert_ne!(t["method"], "net/end", "{t}");
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn net_fetch_takes_bytes_through_the_module_as_well() {
    let p = Probe::new(routed(server(|_, body, s| answer(s, &body))));
    let a = p.ask(
        "net/fetch",
        json!({ "url": "https://api.exemple.dev/", "method": "POST", "bodyBase64": "/wAB" }),
    );
    assert_eq!(a["result"]["bodyBase64"], "/wAB", "{a}");
    let said = p.served.said.lock().unwrap().join("\n");
    assert!(
        said.contains("net/fetch POST api.exemple.dev: 200"),
        "{said}"
    );
}

// ---- The WebSocket ---------------------------------------------------------------------------------------------------

/// A WebSocket server on what `accept` makes of each connection: it answers each message, `close <code>` by closing, and
/// `big` by a message over 1 MiB; it notes the path it was asked, and the `x-key` header it got.
#[allow(clippy::result_large_err)] // The shape of tungstenite's callback.
async fn ws_on<S>(ws: S, seen: Arc<Mutex<Vec<String>>>)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
    use tokio_tungstenite::tungstenite::protocol::frame::{coding::CloseCode, CloseFrame};
    let note = seen.clone();
    let hook = move |r: &Request, a: Response| {
        let key = r.headers().get("x-key").map(|v| v.to_str().unwrap_or(""));
        note.lock()
            .unwrap()
            .push(format!("{} {}", r.uri(), key.unwrap_or("-")));
        Ok(a)
    };
    let Ok(mut ws) = tokio_tungstenite::accept_hdr_async(ws, hook).await else {
        return;
    };
    while let Some(Ok(m)) = ws.next().await {
        match m {
            Message::Text(t) if t.as_str() == "big" => {
                let _ = ws.send(Message::text("a".repeat((1 << 20) + 1))).await;
            }
            Message::Text(t) if t.as_str().starts_with("close ") => {
                let code: u16 = t.as_str()[6..].parse().unwrap();
                let _ = ws
                    .send(Message::Close(Some(CloseFrame {
                        code: CloseCode::from(code),
                        reason: "said by the server".into(),
                    })))
                    .await;
            }
            Message::Text(_) | Message::Binary(_) => {
                let _ = ws.send(m).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}

/// A plain server, reached by a route of the tests: `wss` itself is played by `wss_through_a_proxy`.
async fn ws_server() -> (u16, Arc<Mutex<Vec<String>>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let told = seen.clone();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            tokio::spawn(ws_on(tcp, told.clone()));
        }
    });
    (port, seen)
}

fn socket_probe(port: u16) -> Probe {
    let net = Net {
        routes: HashMap::from([(
            "live.exemple.dev".to_string(),
            format!("ws://127.0.0.1:{port}"),
        )]),
        ..Net::default()
    };
    Probe::new(Fetcher::new(ID, &words(&["live.exemple.dev"]), &net))
}

const LIVE: &str = "wss://live.exemple.dev/stream?key=s3cret";

#[tokio::test(flavor = "multi_thread")]
async fn a_socket_carries_text_and_bytes_both_ways_and_says_how_it_ended() {
    let (port, seen) = ws_server().await;
    let p = socket_probe(port);
    let opened = p.ask(
        "ws/open",
        json!({ "url": LIVE, "headers": { "x-key": "h3ader" } }),
    );
    let socket = opened["result"]["socket"].clone();
    assert!(socket.is_u64(), "{opened}");
    assert_eq!(seen.lock().unwrap()[0], "/stream?key=s3cret h3ader");
    p.ask("ws/send", json!({ "socket": socket, "text": "héllo" }));
    assert_eq!(
        p.told("ws/message"),
        json!({ "socket": socket, "text": "héllo" })
    );
    p.ask("ws/send", json!({ "socket": socket, "dataBase64": "/wAB" }));
    assert_eq!(
        p.told("ws/message"),
        json!({ "socket": socket, "dataBase64": "/wAB" })
    );
    p.ask("ws/send", json!({ "socket": socket, "text": "close 4001" }));
    assert_eq!(
        p.told("ws/closed"),
        json!({ "socket": socket, "code": 4001, "reason": "said by the server" })
    );
    // Neither its headers nor its query go to the journal.
    let said = p.served.said.lock().unwrap().join("\n");
    assert!(said.contains("ws/open live.exemple.dev"), "{said}");
    assert!(
        !said.contains("s3cret") && !said.contains("h3ader"),
        "{said}"
    );
    // It is gone.
    let a = p.ask("ws/send", json!({ "socket": socket, "text": "a" }));
    assert_eq!(a["error"]["code"], REFUSED, "{a}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_socket_closes_when_asked_and_a_message_over_1_mib_ends_it() {
    let (port, _) = ws_server().await;
    let p = socket_probe(port);
    let open = || p.ask("ws/open", json!({ "url": LIVE }))["result"]["socket"].clone();
    let socket = open();
    assert_eq!(
        p.ask("ws/close", json!({ "socket": socket }))["result"],
        Value::Null
    );
    let closed = p.told("ws/closed");
    assert_eq!(closed["socket"], socket, "{closed}");
    let socket = open();
    p.ask("ws/send", json!({ "socket": socket, "text": "big" }));
    let closed = p.told("ws/closed");
    assert_eq!(closed["code"], 1009, "{closed}");
}

#[tokio::test(flavor = "multi_thread")]
async fn four_sockets_at_most_and_only_wss_to_a_declared_host() {
    let (port, _) = ws_server().await;
    let p = socket_probe(port);
    let open = || p.ask("ws/open", json!({ "url": LIVE }));
    let first = open()["result"]["socket"].clone();
    for _ in 1..4 {
        assert!(open()["result"]["socket"].is_u64());
    }
    let fifth = open();
    assert_eq!(fifth["error"]["code"], REFUSED, "{fifth}");
    p.ask("ws/close", json!({ "socket": first }));
    p.told("ws/closed");
    let last = open()["result"]["socket"].clone();
    assert!(last.is_u64());
    p.ask("ws/close", json!({ "socket": last }));
    p.told("ws/closed");
    for (url, why) in [
        ("ws://live.exemple.dev/", "only wss"),
        ("https://live.exemple.dev/", "only wss"),
        ("wss://other.exemple.dev/", "permissions.network"),
        ("wss://127.0.0.1/", "permissions.network"),
    ] {
        let a = p.ask("ws/open", json!({ "url": url }));
        assert_eq!(a["error"]["code"], REFUSED, "{a}");
        assert!(a["error"]["message"].as_str().unwrap().contains(why), "{a}");
    }
    let refused = p.served.refused.lock().unwrap().join("\n");
    assert!(refused.contains("ws/open: 4 sockets are open"), "{refused}");
}

/// A server that refuses the upgrade.
#[tokio::test(flavor = "multi_thread")]
async fn a_socket_that_is_not_opened_says_the_status_and_nothing_of_its_key() {
    let port = server(|_, _, mut s| {
        let _ = s.write_all(b"HTTP/1.1 401 Unauthorized\r\ncontent-length: 0\r\n\r\n");
    });
    let p = socket_probe(port);
    let a = p.ask(
        "ws/open",
        json!({ "url": LIVE, "headers": { "x-key": "h3ader" } }),
    );
    assert_eq!(a["error"]["code"], FAILED, "{a}");
    let why = a["error"]["message"].as_str().unwrap();
    assert!(why.contains("HTTP 401"), "{why}");
    let refused = p.served.refused.lock().unwrap().join("\n");
    assert!(!refused.contains("s3cret") && !refused.contains("h3ader"));
}

/// A service that quotes the key it refuses: alone, as `Bearer <key>` is sent, and across the end of what is kept.
#[tokio::test(flavor = "multi_thread")]
async fn a_refused_socket_says_what_the_service_answered_and_never_its_key() {
    const KEY: &str = "sk-0123456789abcdefghij";
    let answer = |body: String| {
        format!(
            "HTTP/1.1 401 Unauthorized\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        )
    };
    let bodies = [
        format!("Invalid API key: {KEY}"),
        format!("{}{KEY} is not a key", "x".repeat(500)),
    ];
    for body in bodies {
        let reply = answer(body);
        let port = server(move |_, _, mut s| {
            let _ = s.write_all(reply.as_bytes());
        });
        let p = socket_probe(port);
        let headers = json!({ "authorization": format!("Bearer {KEY}") });
        let a = p.ask("ws/open", json!({ "url": LIVE, "headers": headers }));
        let why = a["error"]["message"].as_str().unwrap();
        assert!(why.contains("HTTP 401"), "{why}");
        assert!(!why.contains("sk-0"), "{why}");
        let refused = p.served.refused.lock().unwrap().join("\n");
        assert!(!refused.contains("sk-0"), "{refused}");
    }
}

/// `wss` itself: the TLS of a self-signed server, by the name the user typed, through a proxy's tunnel; then straight.
#[tokio::test(flavor = "multi_thread")]
async fn wss_through_a_proxy_and_the_tls_of_the_settings() {
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let tls = tls_acceptor();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let told = seen.clone();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let (tls, told) = (tls.clone(), told.clone());
            tokio::spawn(async move {
                if let Ok(s) = tls.accept(tcp).await {
                    ws_on(s, told).await;
                }
            });
        }
    });
    // A proxy that joins the two ends of what it is asked.
    let proxy = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_port = proxy.local_addr().unwrap().port();
    let tunnels = Arc::new(Mutex::new(Vec::new()));
    let note = tunnels.clone();
    tokio::spawn(async move {
        while let Ok((mut from, _)) = proxy.accept().await {
            let note = note.clone();
            tokio::spawn(async move {
                use tokio::io::AsyncReadExt;
                let mut head = Vec::new();
                let mut byte = [0; 1];
                while !head.ends_with(b"\r\n\r\n") {
                    from.read_exact(&mut byte).await.unwrap();
                    head.push(byte[0]);
                }
                let head = String::from_utf8_lossy(&head).into_owned();
                note.lock()
                    .unwrap()
                    .push(head.lines().next().unwrap().to_string());
                let target = head.split_whitespace().nth(1).unwrap().to_string();
                let mut to =
                    tokio::net::TcpStream::connect(target.replace("localhost", "127.0.0.1"))
                        .await
                        .unwrap();
                from.write_all(b"HTTP/1.1 200 OK\r\n\r\n").await.unwrap();
                let _ = tokio::io::copy_bidirectional(&mut from, &mut to).await;
            });
        }
    });
    let net = Net {
        accounts: Some(hosts(&[format!("localhost:{port}")])),
        network: Arc::new(RwLock::new(Settings {
            proxy: format!("127.0.0.1:{proxy_port}"),
            no_proxy: String::new(),
            insecure: true,
        })),
        ..Net::default()
    };
    let settings = net.network.clone();
    let p = Probe::new(Fetcher::new(ID, &words(&["{account}"]), &net));
    let url = format!("wss://localhost:{port}/through");
    let socket = p.ask("ws/open", json!({ "url": url }))["result"]["socket"].clone();
    assert!(socket.is_u64());
    p.ask("ws/send", json!({ "socket": socket, "text": "a" }));
    assert_eq!(p.told("ws/message")["text"], "a");
    assert_eq!(
        tunnels.lock().unwrap()[0],
        format!("CONNECT localhost:{port} HTTP/1.1")
    );
    // Straight, past the proxy; the certificate checked, it is refused.
    settings.write().proxy = String::new();
    let socket = p.ask("ws/open", json!({ "url": url }))["result"]["socket"].clone();
    assert!(socket.is_u64());
    assert_eq!(tunnels.lock().unwrap().len(), 1);
    assert_eq!(seen.lock().unwrap().len(), 2);
    settings.write().insecure = false;
    let a = p.ask("ws/open", json!({ "url": url }));
    assert_eq!(a["error"]["code"], FAILED, "{a}");
    // `ws` on the host of an account stays refused.
    let a = p.ask(
        "ws/open",
        json!({ "url": format!("ws://localhost:{port}/") }),
    );
    assert_eq!(a["error"]["code"], REFUSED, "{a}");
}

// ---- local:shared ------------------------------------------------------------------------------------------------

fn ports(list: &Arc<Mutex<Vec<u16>>>) -> Option<PortsOf> {
    let list = list.clone();
    Some(Arc::new(move |_| list.lock().unwrap().clone()))
}

#[tokio::test(flavor = "multi_thread")]
async fn local_shared_reaches_the_port_the_core_read_while_its_process_lives() {
    let (port, _) = plain();
    let (other, _) = plain();
    let read = Arc::new(Mutex::new(vec![port]));
    let net = Net {
        shared: ports(&read),
        ..Net::default()
    };
    let f = Fetcher::new(ID, &words(&["local:shared"]), &net);
    let url = format!("http://127.0.0.1:{port}/session");
    assert_eq!(f.fetch(&get(&url)).await.unwrap()["body"], "ok");
    for refused in [
        format!("http://127.0.0.1:{other}/"),
        format!("http://localhost:{port}/"),
        format!("http://[::1]:{port}/"),
        format!("https://127.0.0.1:{port}/"),
        format!("wss://127.0.0.1:{port}/"),
    ] {
        let (code, why) = f.fetch(&get(&refused)).await.unwrap_err();
        assert_eq!(code, REFUSED, "{refused}: {why}");
    }
    // A stream goes the same way.
    let p = Probe::new(Fetcher::new(ID, &words(&["local:shared"]), &net));
    let a = p.ask("net/stream", json!({ "url": url }));
    assert_eq!(a["result"]["status"], 200, "{a}");
    let a = p.ask(
        "ws/open",
        json!({ "url": format!("wss://127.0.0.1:{port}/") }),
    );
    assert_eq!(a["error"]["code"], REFUSED, "{a}");
    // Its process gone, its port is out of reach.
    read.lock().unwrap().clear();
    let (code, _) = f.fetch(&get(&url)).await.unwrap_err();
    assert_eq!(code, REFUSED);
    // Without the declaration, a port read opens nothing.
    read.lock().unwrap().push(port);
    let undeclared = Fetcher::new(ID, &words(&["exemple.dev"]), &net);
    assert_eq!(undeclared.fetch(&get(&url)).await.unwrap_err().0, REFUSED);
}

#[tokio::test(flavor = "multi_thread")]
async fn local_shared_goes_around_the_proxy() {
    let (proxy, seen) = fake_proxy(OK);
    let (port, _) = plain();
    let mut net = proxied(&format!("127.0.0.1:{proxy}"), "");
    net.shared = ports(&Arc::new(Mutex::new(vec![port])));
    let f = Fetcher::new(ID, &words(&["local:shared"]), &net);
    let r = f.fetch(&get(&format!("http://127.0.0.1:{port}/"))).await;
    assert_eq!(r.unwrap()["body"], "ok");
    assert!(seen.lock().unwrap().is_empty());
}

#[test]
fn the_template_of_the_address_accepts_127_0_0_1_and_nothing_else() {
    use crate::plugins::shared::port_in;
    let t = "opencode server listening on http://{host}:{port}";
    let line = |s: &str| format!("  opencode server listening on {s}  ");
    assert_eq!(port_in(t, &line("http://127.0.0.1:4096")), Some(4096));
    for bad in [
        "http://0.0.0.0:4096",
        "http://192.168.1.2:4096",
        "http://localhost:4096",
        "http://127.0.0.1:",
        "http://127.0.0.1:99999",
        "http://127.0.0.1:0",
    ] {
        assert_eq!(port_in(t, &line(bad)), None, "{bad}");
    }
    assert_eq!(port_in("no port here {host}", "127.0.0.1"), None);
    assert_eq!(
        port_in("at {host}:{port}/ok", "at 127.0.0.1:80/ok"),
        Some(80)
    );
    assert_eq!(port_in("at {host}:{port}/ok", "at 127.0.0.1:80/no"), None);
}

/// The whole of it: a process the core starts says its port, which `ports_of` gives while it lives, to its plugin alone.
#[tokio::test(flavor = "multi_thread")]
async fn the_port_of_a_shared_process_is_read_from_its_output_and_goes_with_it() {
    use crate::engine::Program;
    use crate::plugins::shared::{self, Shared};
    let (rt, plugin) = (0xfeed_usize, "dev.test.shared");
    #[cfg(windows)]
    crate::paths::trim_test_path();
    let program = Program::Native("node".into());
    let key = shared::key_of(rt, plugin, &program, &[]);
    let args = words(&[
        "-e",
        "console.log('server up, listening on http://127.0.0.1:4242'); setTimeout(() => {}, 60000)",
    ]);
    let dir = std::env::temp_dir();
    let s = shared::acquire(&key, "/agent", false, |own| {
        Shared::start(shared::Start {
            key: own,
            name: "Test",
            program: &program,
            args: &args,
            env: &[],
            remove_env: &[],
            ready: "listening",
            listens: Some("listening on http://{host}:{port}"),
            keep: Value::Null,
            dir: &dir,
        })
    })
    .unwrap();
    s.ready("Test").await.unwrap();
    assert_eq!(shared::ports_of(rt, plugin), [4242]);
    assert!(shared::ports_of(rt, "dev.test.other").is_empty());
    assert!(shared::ports_of(rt + 1, plugin).is_empty());
    shared::release(&s);
    let started = Instant::now();
    while !shared::ports_of(rt, plugin).is_empty() {
        assert!(started.elapsed() < Duration::from_secs(20), "it lives on");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

// ---- The timers --------------------------------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn a_timer_fires_once_after_its_delay_and_never_under_10_ms() {
    let p = Probe::new(routed(1));
    let started = Instant::now();
    let a = p.ask("timer/set", json!({ "ms": 0 }));
    let timer = a["result"]["timer"].clone();
    assert!(timer.is_u64(), "{a}");
    assert_eq!(p.told("timer/fired"), json!({ "timer": timer }));
    assert!(started.elapsed() >= Duration::from_millis(9));
    let started = Instant::now();
    p.ask("timer/set", json!({ "ms": 150 }));
    p.told("timer/fired");
    assert!(started.elapsed() >= Duration::from_millis(140));
    assert!(p.next_told(Duration::from_millis(100)).is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_timer_taken_back_does_not_fire() {
    let p = Probe::new(routed(1));
    let timer = p.ask("timer/set", json!({ "ms": 100 }))["result"]["timer"].clone();
    p.ask("timer/cancel", json!({ "timer": timer }));
    assert!(p.next_told(Duration::from_millis(300)).is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn sixty_four_timers_at_most_and_one_that_fired_makes_room() {
    let p = Probe::new(routed(1));
    let far = json!({ "ms": 60_000 });
    let first = p.ask("timer/set", far.clone())["result"]["timer"].clone();
    for _ in 1..64 {
        assert!(p.ask("timer/set", far.clone())["result"]["timer"].is_u64());
    }
    let a = p.ask("timer/set", far.clone());
    assert_eq!(a["error"]["code"], REFUSED, "{a}");
    p.ask("timer/cancel", json!({ "timer": first }));
    assert!(p.ask("timer/set", json!({ "ms": 10 }))["result"]["timer"].is_u64());
    p.told("timer/fired");
    assert!(p.ask("timer/set", far.clone())["result"]["timer"].is_u64());
    for bad in [json!({}), json!({ "ms": "a" }), json!({ "ms": -1 })] {
        let a = p.ask("timer/set", bad);
        assert_eq!(
            a["error"]["code"],
            crate::plugins::runtime::INVALID_PARAMS,
            "{a}"
        );
    }
}

// ---- The bench ---------------------------------------------------------------------------------------------------

/// The bench's host is the app's: the same capabilities, `{account}` given by `--account`, the limits of the manifest.
#[tokio::test(flavor = "multi_thread")]
async fn the_bench_serves_them_too() {
    let (port, _) = plain();
    let mut m: Value = serde_json::from_str(
        &std::fs::read_to_string(fixture().join("escouade-plugin.json")).unwrap(),
    )
    .unwrap();
    m["permissions"]["network"] = json!(["{account}"]);
    m["limits"] = json!({ "fetchSeconds": 20 });
    let manifest = manifest::check(&m, Hand::Third).manifest.unwrap();
    assert_eq!(manifest.limits.fetch_seconds, Some(20));
    let module = wasm::reactor(&std::fs::read(fixture().join(wasm::MODULE_FILE)).unwrap()).unwrap();
    let session = Session::start_wasm(
        &fixture(),
        &manifest,
        module,
        HashMap::new(),
        None,
        wasm::Limits::default(),
        Reach {
            offline: false,
            accounts: vec![format!("127.0.0.1:{port}")],
        },
    )
    .unwrap();
    let mut watch = session.watch();
    let patience = Duration::from_secs(20);
    let host = |method: &str, params: Value| json!({ "method": method, "params": params });
    let url = format!("http://127.0.0.1:{port}/");
    let a = session
        .call(
            "probe/host",
            &host("net/fetch", json!({ "url": url })),
            patience,
        )
        .await;
    let a = format!("{a:?}");
    assert!(a.contains("200"), "{a}");
    let a = session
        .call(
            "probe/host",
            &host("timer/set", json!({ "ms": 10 })),
            patience,
        )
        .await;
    assert!(format!("{a:?}").contains("timer"), "{a:?}");
    let started = Instant::now();
    loop {
        let seen = tokio::time::timeout(patience, watch.recv()).await.unwrap();
        if matches!(seen, Some(Seen::Notification { ref method }) if method == "probe/seen") {
            break;
        }
        assert!(started.elapsed() < patience);
    }
    let a = session
        .call(
            "probe/host",
            &host("net/stream", json!({ "url": "https://elsewhere.dev/" })),
            patience,
        )
        .await;
    assert!(format!("{a:?}").contains("-32003"), "{a:?}");
    session.kill();
}
