//! The gate against a fake connector in memory (RP-1): an allowed host goes through its connector, a host of a private
//! network out of the list is refused, a public host goes straight out (or through the space's proxy), a connector cut
//! refuses at once and closes its flows, an unknown or missing token is refused; in HTTP and in SOCKS5, and through
//! `escouade-server via`. Nothing here leaves the loopback.

use escouade_gate::connectors::Opening;
use escouade_gate::*;
use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

/// A connector in memory: each flow is a pipe whose other end echoes what it reads, until the connector is cut.
struct Fake {
    known: Mutex<Known>,
    opened: Mutex<Vec<Ask>>,
    refuse: Mutex<Option<String>>,
    cut: watch::Sender<bool>,
}

impl Fake {
    fn new(ceiling: &[&str]) -> Arc<Self> {
        Arc::new(Self {
            known: Mutex::new(Known {
                id: "acme".into(),
                name: "Acme".into(),
                reach: Reach::Up,
                ceiling: ceiling.iter().map(|p| Pattern::parse(p).unwrap()).collect(),
            }),
            opened: Mutex::default(),
            refuse: Mutex::default(),
            cut: watch::channel(false).0,
        })
    }

    fn cut(&self) {
        self.known.lock().reach = Reach::Down {
            since_ms: chrono::Utc::now().timestamp_millis() as u64,
        };
        self.cut.send_replace(true);
    }

    fn opened(&self) -> Vec<Ask> {
        self.opened.lock().clone()
    }
}

/// The fake, as the gate holds it.
struct Plugged(Arc<Fake>);

impl Connectors for Plugged {
    fn known(&self) -> Vec<Known> {
        vec![self.0.known.lock().clone()]
    }

    fn open(&self, id: &str, ask: Ask) -> Opening {
        assert_eq!(id, "acme");
        let me = self.0.clone();
        Box::pin(async move {
            if let Reach::Down { since_ms } = me.known.lock().reach {
                return Err(Failed::Down { since_ms });
            }
            if let Some(why) = me.refuse.lock().clone() {
                return Err(Failed::Refused(why));
            }
            me.opened.lock().push(ask);
            let (near, mut far) = tokio::io::duplex(64 * 1024);
            let mut cut = me.cut.subscribe();
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                loop {
                    tokio::select! {
                        n = far.read(&mut buf) => match n {
                            Ok(0) | Err(_) => break,
                            Ok(n) => if far.write_all(&buf[..n]).await.is_err() { break },
                        },
                        _ = cut.changed() => break,
                    }
                }
            });
            Ok(Box::new(near) as Flow)
        })
    }
}

/// A host of the Internet on the loopback: it echoes, after a word that says it is itself.
async fn public_host() -> SocketAddr {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut s, _)) = l.accept().await {
            tokio::spawn(async move {
                let _ = s.write_all(b"public:").await;
                let mut buf = [0u8; 4096];
                while let Ok(n) = s.read(&mut buf).await {
                    if n == 0 || s.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
            });
        }
    });
    addr
}

const DEMO: &str = "p1";

fn holder() -> Holder {
    Holder {
        who: "agent-1".into(),
        project: DEMO.into(),
        project_name: "Démo".into(),
    }
}

fn lists(hosts: &[(&str, u16)]) -> Lists {
    let mut projects = BTreeMap::new();
    projects.insert(
        DEMO.to_string(),
        ProjectList {
            hosts: hosts
                .iter()
                .map(|(h, p)| Allowed {
                    host: h.to_string(),
                    port: *p,
                    connector: "acme".into(),
                })
                .collect(),
        },
    );
    Lists { projects }
}

const CEILING: &[&str] = &[
    "gitlab.acme.internal:22",
    "*.npm.acme.internal:443",
    "secret.acme.internal:443",
];

async fn gate_with(fake: &Arc<Fake>, lists: Lists, outside: Outside) -> Gate {
    Gate::start(Config {
        lang: Lang::Fr,
        connectors: Arc::new(Plugged(fake.clone())),
        lists,
        outside,
        port: 0,
    })
    .await
    .unwrap()
}

async fn gate(fake: &Arc<Fake>) -> Gate {
    gate_with(
        fake,
        lists(&[("gitlab.acme.internal", 22), ("other.acme.internal", 22)]),
        Outside::default(),
    )
    .await
}

fn basic(token: &str) -> String {
    use base64::Engine as _;
    format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!("{token}:"))
    )
}

/// A `CONNECT` to the gate: its status, its body, and the stream.
async fn connect(gate: &Gate, token: Option<&str>, target: &str) -> (u16, String, TcpStream) {
    let mut s = TcpStream::connect(gate.addr()).await.unwrap();
    let mut ask = format!("CONNECT {target} HTTP/1.1\r\nHost: {target}\r\n");
    if let Some(t) = token {
        ask.push_str(&format!("Proxy-Authorization: {}\r\n", basic(t)));
    }
    ask.push_str("\r\n");
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

async fn echo(s: &mut TcpStream, word: &[u8]) -> Vec<u8> {
    s.write_all(word).await.unwrap();
    let mut got = vec![0u8; word.len()];
    tokio::time::timeout(Duration::from_secs(5), s.read_exact(&mut got))
        .await
        .unwrap()
        .unwrap();
    got
}

/// The journal once it has `n` entries.
async fn journal(gate: &Gate, n: usize) -> Vec<Entry> {
    for _ in 0..200 {
        let j = gate.journal();
        if j.len() >= n {
            return j;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the journal has {:?}", gate.journal());
}

#[tokio::test]
async fn an_allowed_host_goes_through_its_connector_and_is_journaled_without_its_content() {
    let fake = Fake::new(CEILING);
    let gate = gate(&fake).await;
    let token = gate.issue(holder());
    let (status, _, mut s) = connect(&gate, Some(&token), "GitLab.Acme.Internal:22").await;
    assert_eq!(status, 200);
    assert_eq!(echo(&mut s, b"SSH-2.0-test\r\n").await, b"SSH-2.0-test\r\n");
    assert_eq!(
        fake.opened(),
        vec![Ask {
            host: "gitlab.acme.internal".into(),
            port: 22,
            agent: "agent-1".into(),
            project: DEMO.into(),
        }]
    );
    drop(s);
    let j = journal(&gate, 1).await;
    let e = &j[0];
    assert!(e.allowed);
    assert_eq!(e.route, Route::Connector);
    assert_eq!(e.connector.as_deref(), Some("acme"));
    assert_eq!((e.host.as_str(), e.port), ("gitlab.acme.internal", 22));
    assert_eq!((e.agent.as_str(), e.project.as_str()), ("agent-1", DEMO));
    assert_eq!((e.up, e.down), (14, 14));
    let shown = serde_json::to_string(&j).unwrap();
    assert!(!shown.contains(&token), "{shown}");
    assert!(!shown.contains("SSH-2.0"), "{shown}");
}

#[tokio::test]
async fn a_host_of_the_private_network_out_of_the_list_is_refused_with_its_reason() {
    let fake = Fake::new(CEILING);
    let gate = gate(&fake).await;
    let token = gate.issue(holder());
    let (status, body, _) = connect(&gate, Some(&token), "secret.acme.internal:443").await;
    assert_eq!(status, 403);
    assert_eq!(
        body.trim(),
        "secret.acme.internal:443 n’est pas dans la liste du projet Démo"
    );
    // A name under a pattern of the ceiling, and the right host on another port: as private.
    let (status, _, _) = connect(&gate, Some(&token), "a.npm.acme.internal:443").await;
    assert_eq!(status, 403);
    let (status, body, _) = connect(&gate, Some(&token), "gitlab.acme.internal:2222").await;
    assert_eq!(status, 403, "{body}");
    assert!(fake.opened().is_empty());
    let j = journal(&gate, 3).await;
    assert!(j
        .iter()
        .all(|e| !e.allowed && e.refusal == Some("not-in-list")));
}

#[tokio::test]
async fn a_host_out_of_the_connector_s_ceiling_or_refused_by_it_is_refused() {
    let fake = Fake::new(CEILING);
    let gate = gate(&fake).await;
    let token = gate.issue(holder());
    let (status, body, _) = connect(&gate, Some(&token), "other.acme.internal:22").await;
    assert_eq!(status, 403);
    assert_eq!(
        body.trim(),
        "other.acme.internal:22 n’est pas permis par le connecteur d’Acme"
    );
    *fake.refuse.lock() = Some("adresse de lien local".into());
    let (status, body, _) = connect(&gate, Some(&token), "gitlab.acme.internal:22").await;
    assert_eq!(status, 403);
    assert_eq!(
        body.trim(),
        "gitlab.acme.internal:22 n’est pas permis par le connecteur d’Acme (adresse de lien local)"
    );
}

#[tokio::test]
async fn a_public_host_goes_straight_out() {
    let fake = Fake::new(CEILING);
    let gate = gate(&fake).await;
    let token = gate.issue(holder());
    let public = public_host().await;
    let (status, _, mut s) =
        connect(&gate, Some(&token), &format!("localhost:{}", public.port())).await;
    assert_eq!(status, 200);
    // Its greeting, then its echo.
    assert_eq!(echo(&mut s, b"npm-i").await, b"publi");
    assert_eq!(echo(&mut s, b"").await, b"");
    let mut rest = [0u8; 7];
    s.read_exact(&mut rest).await.unwrap();
    assert_eq!(&rest, b"c:npm-i");
    drop(s);
    assert!(fake.opened().is_empty());
    let e = &journal(&gate, 1).await[0];
    assert!(e.allowed);
    assert_eq!(e.route, Route::Direct);
}

/// The space's proxy, on the loopback: it takes a `CONNECT` with its own authorization, and says what it saw.
async fn space_proxy() -> (SocketAddr, Arc<Mutex<Vec<String>>>) {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    let seen: Arc<Mutex<Vec<String>>> = Arc::default();
    let heads = seen.clone();
    tokio::spawn(async move {
        while let Ok((mut s, _)) = l.accept().await {
            let heads = heads.clone();
            tokio::spawn(async move {
                let (head, _) = escouade_gate::pipe::read_head(&mut s).await.unwrap();
                let head = String::from_utf8(head).unwrap();
                heads.lock().push(head.clone());
                let target = head.split(' ').nth(1).unwrap().to_string();
                let mut out = TcpStream::connect(target).await.unwrap();
                s.write_all(b"HTTP/1.1 200 OK\r\n\r\n").await.unwrap();
                let _ = tokio::io::copy_bidirectional(&mut s, &mut out).await;
            });
        }
    });
    (addr, seen)
}

#[tokio::test]
async fn a_public_host_goes_through_the_space_s_proxy_behind_the_gate_but_its_no_proxy() {
    let fake = Fake::new(CEILING);
    let (proxy, seen) = space_proxy().await;
    let outside = Outside::of_proxy(
        &format!("http://bob:secret@127.0.0.1:{}", proxy.port()),
        "localhost",
    );
    let gate = gate_with(&fake, lists(&[("gitlab.acme.internal", 22)]), outside).await;
    let token = gate.issue(holder());
    let public = public_host().await;
    let (status, _, mut s) =
        connect(&gate, Some(&token), &format!("127.0.0.1:{}", public.port())).await;
    assert_eq!(status, 200);
    assert_eq!(echo(&mut s, b"public:").await, b"public:");
    drop(s);
    let heads = seen.lock().clone();
    assert_eq!(heads.len(), 1);
    assert!(heads[0].starts_with(&format!("CONNECT 127.0.0.1:{} HTTP/1.1", public.port())));
    assert!(
        heads[0].contains("Proxy-Authorization: Basic Ym9iOnNlY3JldA=="),
        "{}",
        heads[0]
    );
    assert!(!heads[0].contains(&token));
    // Its NO_PROXY goes straight.
    let (status, _, _) =
        connect(&gate, Some(&token), &format!("localhost:{}", public.port())).await;
    assert_eq!(status, 200);
    assert_eq!(seen.lock().len(), 1);
    let j = journal(&gate, 1).await;
    assert_eq!(j[0].route, Route::Proxy);
}

#[tokio::test]
async fn a_connector_cut_closes_its_flows_and_refuses_at_once() {
    let fake = Fake::new(CEILING);
    let gate = gate(&fake).await;
    let token = gate.issue(holder());
    let (status, _, mut s) = connect(&gate, Some(&token), "gitlab.acme.internal:22").await;
    assert_eq!(status, 200);
    assert_eq!(echo(&mut s, b"x").await, b"x");
    fake.cut();
    // The flow open is closed, with no resuming.
    let mut rest = Vec::new();
    tokio::time::timeout(Duration::from_secs(2), s.read_to_end(&mut rest))
        .await
        .expect("the flow is closed at once")
        .unwrap();
    assert!(rest.is_empty());
    let started = Instant::now();
    let (status, body, _) = connect(&gate, Some(&token), "gitlab.acme.internal:22").await;
    assert!(started.elapsed() < Duration::from_secs(1));
    assert_eq!(status, 403);
    let at = chrono::Local::now().format("%H:%M").to_string();
    assert!(
        body.trim() == format!("le réseau privé d’Acme est injoignable depuis {at}"),
        "{body}"
    );
    assert_eq!(fake.opened().len(), 1);
    let j = journal(&gate, 2).await;
    assert!(j.iter().any(|e| e.refusal == Some("connector-down")));
}

#[tokio::test]
async fn an_unknown_or_missing_token_is_refused() {
    let fake = Fake::new(CEILING);
    let gate = gate(&fake).await;
    let (status, body, _) = connect(&gate, None, "gitlab.acme.internal:22").await;
    assert_eq!(status, 407);
    assert!(body.contains("jeton"), "{body}");
    let (status, body, _) =
        connect(&gate, Some("0123456789abcdef"), "gitlab.acme.internal:22").await;
    assert_eq!(status, 407);
    assert!(body.contains("ne connaît pas ce jeton"), "{body}");
    let token = gate.issue(holder());
    gate.revoke(&token);
    let (status, _, _) = connect(&gate, Some(&token), "gitlab.acme.internal:22").await;
    assert_eq!(status, 407);
    let other = gate.issue(holder());
    gate.revoke_who("agent-1");
    assert!(gate.holder(&other).is_none());
    assert!(fake.opened().is_empty());
    let j = journal(&gate, 3).await;
    assert_eq!(j[0].refusal, Some("no-token"));
    assert_eq!(j[1].refusal, Some("unknown-token"));
}

/// A SOCKS5 greeting with `token` as the user's name (or no method of names), then a `CONNECT` to `addr`: the reply
/// code and the stream.
async fn socks(
    gate: &Gate,
    token: Option<&str>,
    addr: &[u8],
    port: u16,
) -> (Option<u8>, TcpStream) {
    let mut s = TcpStream::connect(gate.addr()).await.unwrap();
    match token {
        None => {
            s.write_all(&[5, 1, 0]).await.unwrap();
            let mut r = [0u8; 2];
            s.read_exact(&mut r).await.unwrap();
            assert_eq!(r, [5, 0xff]);
            return (None, s);
        }
        Some(t) => {
            s.write_all(&[5, 2, 0, 2]).await.unwrap();
            let mut r = [0u8; 2];
            s.read_exact(&mut r).await.unwrap();
            assert_eq!(r, [5, 2]);
            let mut auth = vec![1, t.len() as u8];
            auth.extend_from_slice(t.as_bytes());
            auth.extend_from_slice(&[0]);
            s.write_all(&auth).await.unwrap();
            let mut r = [0u8; 2];
            s.read_exact(&mut r).await.unwrap();
            if r != [1, 0] {
                return (Some(0xff), s);
            }
        }
    }
    let mut ask = vec![5, 1, 0];
    ask.extend_from_slice(addr);
    ask.extend_from_slice(&port.to_be_bytes());
    s.write_all(&ask).await.unwrap();
    let mut r = [0u8; 10];
    s.read_exact(&mut r).await.unwrap();
    (Some(r[1]), s)
}

fn name(n: &str) -> Vec<u8> {
    let mut v = vec![3, n.len() as u8];
    v.extend_from_slice(n.as_bytes());
    v
}

#[tokio::test]
async fn socks5_by_name_goes_through_the_connector_and_an_address_is_never_taken_for_a_private_name(
) {
    let fake = Fake::new(CEILING);
    let public = public_host().await;
    // The project's list holds the name on the port of a host of the loopback.
    let gate = gate_with(
        &fake,
        lists(&[
            ("gitlab.acme.internal", 22),
            ("gitlab.acme.internal", public.port()),
        ]),
        Outside::default(),
    )
    .await;
    let token = gate.issue(holder());
    let (code, mut s) = socks(&gate, Some(&token), &name("gitlab.acme.internal"), 22).await;
    assert_eq!(code, Some(0));
    assert_eq!(echo(&mut s, b"git").await, b"git");
    drop(s);
    assert_eq!(fake.opened().len(), 1);
    // The agent resolved the name itself (`socks5`, not `socks5h`): its address is no name of the list.
    let (code, mut s) = socks(&gate, Some(&token), &[1, 127, 0, 0, 1], public.port()).await;
    assert_eq!(code, Some(0));
    assert_eq!(echo(&mut s, b"public:").await, b"public:");
    drop(s);
    assert_eq!(
        fake.opened().len(),
        1,
        "an address never goes to the connector"
    );
    // Out of the list, refused: « connexion non permise ».
    let (code, _) = socks(&gate, Some(&token), &name("secret.acme.internal"), 443).await;
    assert_eq!(code, Some(2));
    // No token, an unknown one.
    let (code, _) = socks(&gate, None, &[], 0).await;
    assert_eq!(code, None);
    let (code, _) = socks(&gate, Some("nope"), &[], 0).await;
    assert_eq!(code, Some(0xff));
    fake.cut();
    let (code, _) = socks(&gate, Some(&token), &name("gitlab.acme.internal"), 22).await;
    assert_eq!(code, Some(2));
    let j = journal(&gate, 6).await;
    let routes: Vec<_> = j.iter().map(|e| (e.by, e.route, e.refusal)).collect();
    assert!(
        routes.contains(&("socks", Route::Connector, None)),
        "{routes:?}"
    );
    assert!(
        routes.contains(&("socks", Route::Direct, None)),
        "{routes:?}"
    );
    assert!(routes.contains(&("socks", Route::Refused, Some("not-in-list"))));
    assert!(routes.contains(&("socks", Route::Refused, Some("connector-down"))));
}

#[tokio::test]
async fn a_plain_http_request_goes_on_in_origin_form_without_the_token() {
    let fake = Fake::new(CEILING);
    let gate = gate(&fake).await;
    let token = gate.issue(holder());
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        let (mut s, _) = l.accept().await.unwrap();
        let (head, _) = escouade_gate::pipe::read_head(&mut s).await.unwrap();
        s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
            .await
            .unwrap();
        String::from_utf8(head).unwrap()
    });
    let mut s = TcpStream::connect(gate.addr()).await.unwrap();
    let ask = format!(
        "GET http://localhost:{port}/registry/pkg?v=1 HTTP/1.1\r\nHost: localhost:{port}\r\nProxy-Authorization: {}\r\n\r\n",
        basic(&token)
    );
    s.write_all(ask.as_bytes()).await.unwrap();
    let mut answer = Vec::new();
    s.read_to_end(&mut answer).await.unwrap();
    assert!(String::from_utf8(answer).unwrap().ends_with("ok"));
    let head = server.await.unwrap();
    assert!(
        head.starts_with("GET /registry/pkg?v=1 HTTP/1.1\r\n"),
        "{head}"
    );
    assert!(!head.contains(&token));
    assert!(!head.to_ascii_lowercase().contains("proxy-authorization"));
    // A private host in plain HTTP is refused as well.
    let mut s = TcpStream::connect(gate.addr()).await.unwrap();
    let ask = format!(
        "GET http://secret.acme.internal:443/ HTTP/1.1\r\nProxy-Authorization: {}\r\n\r\n",
        basic(&token)
    );
    s.write_all(ask.as_bytes()).await.unwrap();
    let mut answer = Vec::new();
    s.read_to_end(&mut answer).await.unwrap();
    let answer = String::from_utf8(answer).unwrap();
    assert!(answer.starts_with("HTTP/1.1 403 "), "{answer}");
    assert!(answer.contains("n’est pas dans la liste du projet Démo"));
}

#[tokio::test(flavor = "multi_thread")]
async fn escouade_server_via_goes_through_the_gate_and_says_its_refusal() {
    let fake = Fake::new(CEILING);
    let gate = gate(&fake).await;
    let token = gate.issue(holder());
    let env: Vec<(String, String)> = gate.env(&token, "");
    let var = move |k: &str| env.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
    let var2 = var.clone();
    let out = tokio::task::spawn_blocking(move || {
        let s = via::connect("gitlab.acme.internal", 22, &var).unwrap();
        let mut out = Vec::new();
        via::pump(
            s,
            std::io::Cursor::new(b"SSH-2.0-OpenSSH\r\n".to_vec()),
            &mut out,
        )
        .unwrap();
        out
    })
    .await
    .unwrap();
    assert_eq!(out, b"SSH-2.0-OpenSSH\r\n");
    assert_eq!(fake.opened().len(), 1);
    let refused =
        tokio::task::spawn_blocking(move || via::connect("secret.acme.internal", 22, &var2).err())
            .await
            .unwrap();
    assert_eq!(
        refused.as_deref(),
        Some("secret.acme.internal:22 n’est pas dans la liste du projet Démo")
    );
    // No gate in its environment: straight out.
    let public = public_host().await;
    let direct = tokio::task::spawn_blocking(move || {
        let s = via::connect("127.0.0.1", public.port(), &|_| None).unwrap();
        let mut out = Vec::new();
        via::pump(s, std::io::Cursor::new(Vec::new()), &mut out).unwrap();
        out
    });
    let out = tokio::time::timeout(Duration::from_secs(5), direct)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(out, b"public:");
}

#[tokio::test]
async fn the_variables_name_the_gate_with_the_token() {
    let fake = Fake::new(CEILING);
    let gate = gate(&fake).await;
    let token = gate.issue(holder());
    let ssh = env::ssh_command(None, "/opt/escouade/escouade-server");
    let vars = gate.env(&token, &ssh);
    let port = gate.addr().port();
    let get = |k: &str| {
        vars.iter()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.clone())
            .unwrap()
    };
    assert_eq!(
        get("HTTPS_PROXY"),
        format!("http://{token}@127.0.0.1:{port}")
    );
    assert_eq!(
        get("http_proxy"),
        format!("http://{token}@127.0.0.1:{port}")
    );
    assert_eq!(
        get("ALL_PROXY"),
        format!("socks5h://{token}@127.0.0.1:{port}")
    );
    assert_eq!(get("NO_PROXY"), "127.0.0.1,localhost");
    assert_eq!(
        get("GIT_SSH_COMMAND"),
        "ssh -o ProxyCommand=\"/opt/escouade/escouade-server via %h %p\""
    );
    assert_eq!(token.len(), 64);
    assert_ne!(token, gate.issue(holder()));
}

#[test]
fn decide_reads_the_list_then_the_private_networks_then_outside() {
    let known = vec![Known {
        id: "acme".into(),
        name: "Acme".into(),
        reach: Reach::Up,
        ceiling: vec![Pattern::parse("gitlab.acme.internal:22").unwrap()],
    }];
    let l = lists(&[("gitlab.acme.internal", 22)]);
    let t = |s: &str| Target::parse(s).unwrap();
    let h = holder();
    assert_eq!(
        decide(&l, &known, &h, &t("gitlab.acme.internal:22")),
        Decision::Connector {
            id: "acme".into(),
            name: "Acme".into()
        }
    );
    assert_eq!(
        decide(&l, &known, &h, &t("github.com:22")),
        Decision::Outside
    );
    // Another project: no list, nothing of the private network.
    let other = Holder {
        project: "p2".into(),
        project_name: "Perso".into(),
        ..holder()
    };
    assert!(matches!(
        decide(&l, &known, &other, &t("gitlab.acme.internal:22")),
        Decision::Refused(Reason::NotInList { .. })
    ));
    // Its connector gone: refused, said.
    assert!(matches!(
        decide(&l, &[], &h, &t("gitlab.acme.internal:22")),
        Decision::Refused(Reason::UnknownConnector { .. })
    ));
}
