//! `escouade-server` (ESC-514, `docs/REMOTE.md`, « Le mode serveur »): its command line, the pairing of a device and its
//! refusals (expired, given again, wrong, too many tries), the secrets in a file readable by its owner alone, the lock of
//! the data folder, the clean stop, and a log that says neither a code nor a token. ESC-515: the options as variables (a
//! container's), the status the server writes for `health` and `update`, the signed archive of an update and how it is
//! put in place, the name a TLS proxy serves the API under (`--allow-host`), the log of a new server.

use crate::core_tests::repo;
use crate::headless::{with_file_lock, Headless};
use crate::headless_tests::data;
use crate::integrations::secrets::SecretStore;
use crate::paths::DataDir;
use crate::remoteapi::devices::Devices;
use crate::remoteapi::pairing::{self, Refusal, LIFETIME_MS};
use crate::remoteapi::proto::{self, ToClient, ToServer};
use crate::remoteapi::server::{Limits, RemoteApi};
use crate::server::{
    self, secrets::FileStore, status, update, Args, Command, Endpoint, DEFAULT_PORT,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

type Ws = tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn args(line: &str) -> Vec<String> {
    line.split_whitespace().map(String::from).collect()
}

// ---------- the command line ----------

#[test]
fn the_command_line_reads_its_commands_and_options_and_help_is_never_a_command_run() {
    let run = server::parse_with(&[], |_| None).unwrap();
    assert_eq!(
        run,
        Args {
            command: Command::Run,
            data_dir: None,
            port: DEFAULT_PORT,
            listen: None,
            hosts: Vec::new(),
            proxies: Vec::new(),
            debug: false,
            update: update::Asked::default(),
            endpoint: None,
        }
    );
    // The phone's tunnel (ESC-567).
    use crate::server::PhoneCommand;
    for (line, want) in [
        ("phone", PhoneCommand::Status),
        ("phone status", PhoneCommand::Status),
        ("phone on", PhoneCommand::On(None)),
        ("phone on 51900", PhoneCommand::On(Some(51900))),
        ("phone off", PhoneCommand::Off),
        ("phone add Pixel", PhoneCommand::Add("Pixel".into())),
        ("phone remove abc", PhoneCommand::Remove("abc".into())),
    ] {
        assert_eq!(
            server::parse(&args(line)).unwrap().command,
            Command::Phone(want),
            "{line}"
        );
    }
    let a = server::parse(&args("phone add Pixel --endpoint vps.example.net")).unwrap();
    assert_eq!(a.endpoint.as_deref(), Some("vps.example.net"));
    assert!(server::parse(&args("phone add")).is_err());
    assert!(server::parse(&args("phone on nope")).is_err());
    let a = server::parse(&args("run --data-dir /srv/escouade --port 4213 --debug")).unwrap();
    assert_eq!(a.command, Command::Run);
    assert_eq!(
        a.data_dir.as_deref(),
        Some(std::path::Path::new("/srv/escouade"))
    );
    assert_eq!((a.port, a.debug), (4213, true));
    assert_eq!(server::parse(&args("--port=4214")).unwrap().port, 4214);
    assert_eq!(server::parse(&args("pair")).unwrap().command, Command::Pair);
    assert_eq!(
        server::parse(&args("devices")).unwrap().command,
        Command::Devices
    );
    assert_eq!(
        server::parse(&args("devices list")).unwrap().command,
        Command::Devices
    );
    assert_eq!(
        server::parse(&args("devices revoke abc --data-dir d"))
            .unwrap()
            .command,
        Command::Revoke("abc".into())
    );
    // `--help` anywhere is the help: `pair --help` never makes a code.
    for line in ["--help", "pair --help", "devices revoke x -h", "help"] {
        assert_eq!(
            server::parse(&args(line)).unwrap().command,
            Command::Help,
            "{line}"
        );
    }
    assert_eq!(
        server::parse(&args("-V")).unwrap().command,
        Command::Version
    );
    assert_eq!(
        server::parse(&args("--listen 0.0.0.0")).unwrap().listen,
        Some("0.0.0.0".parse().unwrap())
    );
    for bad in [
        "--port 70000",
        "--port",
        "--listen localhost",
        "--frobnicate",
        "devices revoke",
        "serve",
    ] {
        assert!(server::parse(&args(bad)).is_err(), "{bad}");
    }
    assert!(server::help().contains("escouade-server pair"));
}

// ---------- the pairing code ----------

fn data_dir(name: &str) -> DataDir {
    let data = DataDir::new(crate::paths::test_dir(name).join("data"));
    data.ensure().unwrap();
    data
}

#[test]
fn a_pairing_code_is_good_once_its_file_holds_its_hash_alone_and_is_its_owner_s() {
    let data = data_dir("pair-once");
    let now = 1_700_000_000_000;
    let code = pairing::create(&data, now).unwrap();
    // `ABCD-EFGH`: eight letters of the alphabet without 0, O, 1 nor I.
    assert_eq!(code.text.len(), 9, "{}", code.text);
    assert_eq!(&code.text[4..5], "-");
    assert!(code
        .text
        .chars()
        .all(|c| c == '-' || "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".contains(c)));
    assert_eq!(code.expires_at, now + LIFETIME_MS);
    let file = data.root().join(pairing::FILE);
    let kept = std::fs::read_to_string(&file).unwrap();
    assert!(
        !kept.contains(&code.text) && !kept.contains(&code.text.replace('-', "")),
        "{kept}"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&file).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    assert_eq!(pairing::waiting_until(&data, now), Some(now + LIFETIME_MS));
    // As the user types it: in small letters, without the dash.
    let typed = code.text.replace('-', " ").to_lowercase();
    assert_eq!(pairing::redeem(&data, &typed, now + 1), Ok(()));
    // Given again: no code waits any more.
    assert_eq!(
        pairing::redeem(&data, &code.text, now + 2),
        Err(Refusal::NoCode)
    );
    assert!(!file.exists());
    assert_eq!(pairing::waiting_until(&data, now), None);
}

#[test]
fn a_pairing_code_expires_and_a_new_one_takes_the_place_of_the_one_before() {
    let data = data_dir("pair-expired");
    let now = 1_700_000_000_000;
    let code = pairing::create(&data, now).unwrap();
    assert_eq!(
        pairing::redeem(&data, &code.text, now + LIFETIME_MS),
        Err(Refusal::Expired)
    );
    // Expired, it is gone: given again, none waits.
    assert_eq!(
        pairing::redeem(&data, &code.text, now + 1),
        Err(Refusal::NoCode)
    );
    let first = pairing::create(&data, now).unwrap();
    let second = pairing::create(&data, now).unwrap();
    if first.text != second.text {
        assert_eq!(
            pairing::redeem(&data, &first.text, now + 1),
            Err(Refusal::Wrong)
        );
    }
    assert_eq!(pairing::redeem(&data, &second.text, now + 1), Ok(()));
    // Cancelled by hand.
    let third = pairing::create(&data, now).unwrap();
    pairing::cancel(&data).unwrap();
    assert_eq!(
        pairing::redeem(&data, &third.text, now + 1),
        Err(Refusal::NoCode)
    );
}

/// ESC-654 (point 1 of the audit of OshOEz's server, SR1-04): a wrong code never ends the one that waits, however many
/// come; the bound of the tries is the server's, by source (`remoteapi::server::Limits`).
#[test]
fn wrong_pairing_codes_leave_the_code_waiting() {
    let data = data_dir("pair-wrong-waits");
    let now = 1_700_000_000_000;
    let code = pairing::create(&data, now).unwrap();
    let wrong = if code.text.starts_with('A') {
        "BBBB-BBBB"
    } else {
        "AAAA-AAAA"
    };
    for bad in [wrong; 12]
        .into_iter()
        .chain(["", "ABC", &"A".repeat(10_000)])
    {
        assert_eq!(pairing::redeem(&data, bad, now), Err(Refusal::Wrong));
    }
    assert!(pairing::waiting_until(&data, now).is_some());
    // Typed in lower case, with spaces: the same code.
    let typed = code.text.to_ascii_lowercase().replace('-', " ");
    assert_eq!(pairing::redeem(&data, &typed, now), Ok(()));
    assert_eq!(
        pairing::redeem(&data, &code.text, now),
        Err(Refusal::NoCode)
    );
    // Each refusal says what to do, in words, and the client its reason.
    for (r, reason) in [
        (Refusal::NoCode, "none"),
        (Refusal::Expired, "expired"),
        (Refusal::Wrong, "wrong"),
        (Refusal::Wait, "wait"),
    ] {
        assert_eq!(r.code(), reason);
        assert!(
            r.text().contains("pair") || r.text().contains("code"),
            "{}",
            r.text()
        );
    }
}

// ---------- the secrets in a file ----------

#[test]
fn the_secrets_of_a_server_are_in_a_file_readable_by_its_owner_alone_and_read_again_when_another_process_writes(
) {
    let data = data_dir("server-secrets");
    let store = FileStore::open(&data).unwrap();
    assert_eq!(store.get("cursor").unwrap(), None);
    store.set("cursor", "key-1").unwrap();
    store
        .set("plugin:dev.escouade.slack/token", "xoxb-2")
        .unwrap();
    assert_eq!(store.get("cursor").unwrap().as_deref(), Some("key-1"));
    let file = data.root().join(server::secrets::FILE);
    assert!(file.is_file());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    // Another process of the same folder (`escouade-server devices revoke`) changes it: this one reads it again.
    let other = FileStore::open(&data).unwrap();
    other.delete("cursor").unwrap();
    other.set("voice:mistral", "m-3").unwrap();
    assert_eq!(store.get("cursor").unwrap(), None);
    assert_eq!(store.get("voice:mistral").unwrap().as_deref(), Some("m-3"));
    assert_eq!(
        store
            .get("plugin:dev.escouade.slack/token")
            .unwrap()
            .as_deref(),
        Some("xoxb-2")
    );
    // What `keep_secret` checks of a store: it gives back what it was given.
    crate::integrations::secrets::keep_secret(&store, "github", "ghp-4").unwrap();
    // A file made readable by others (a backup put back) is its owner's alone again.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
        FileStore::open(&data).unwrap();
        assert_eq!(
            std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    // An unreadable file is said at the opening, without its content.
    std::fs::write(&file, b"{ \"cursor\": \"secret-5\"").unwrap();
    let e = FileStore::open(&data)
        .err()
        .expect("an unreadable file is refused");
    assert!(!format!("{e:#}").contains("secret-5"), "{e:#}");
}

#[test]
fn a_file_lock_lets_one_writer_in_at_a_time() {
    let data = data_dir("server-file-lock");
    let file = data.root().join("shared.json");
    let order = Arc::new(Mutex::new(Vec::new()));
    let threads: Vec<_> = (0..4)
        .map(|n| {
            let (file, order) = (file.clone(), order.clone());
            std::thread::spawn(move || {
                with_file_lock(&file, || {
                    order.lock().unwrap().push(format!("in {n}"));
                    std::thread::sleep(Duration::from_millis(30));
                    order.lock().unwrap().push(format!("out {n}"));
                    Ok(())
                })
                .unwrap()
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
    let order = order.lock().unwrap();
    // Never two inside: each « in » is followed by its own « out ».
    for pair in order.chunks(2) {
        assert_eq!(pair[0].replace("in", "out"), pair[1], "{order:?}");
    }
}

// ---------- the server, over its WebSocket ----------

/// What the log was told, in this process (nextest runs each test in its own).
fn captured() -> &'static Mutex<Vec<String>> {
    static LINES: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    LINES.get_or_init(|| Mutex::new(Vec::new()))
}

struct Capture;

impl log::Log for Capture {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }
    fn log(&self, r: &log::Record) {
        captured()
            .lock()
            .unwrap()
            .push(format!("{} {}", r.target(), r.args()));
    }
    fn flush(&self) {}
}

/// The log captured from now on; false when another logger was set before in this process (`cargo test` runs the
/// tests in one: the check is then nextest's alone).
fn capture_log() -> bool {
    static INSTALLED: OnceLock<bool> = OnceLock::new();
    *INSTALLED.get_or_init(|| {
        let ok = log::set_boxed_logger(Box::new(Capture)).is_ok();
        if ok {
            log::set_max_level(log::LevelFilter::Debug);
        }
        ok
    })
}

async fn open(addr: SocketAddr) -> Ws {
    let tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
    let url = format!("ws://{addr}{}", proto::PATH);
    tokio_tungstenite::client_async(url, tcp).await.unwrap().0
}

async fn send(ws: &mut Ws, m: &ToServer) {
    ws.send(Message::text(serde_json::to_string(m).unwrap()))
        .await
        .unwrap();
}

/// The next message of the protocol; None once closed.
async fn next(ws: &mut Ws) -> Option<ToClient> {
    loop {
        match tokio::time::timeout(Duration::from_secs(60), ws.next()).await {
            Ok(Some(Ok(Message::Text(t)))) => return serde_json::from_str(&t).ok(),
            Ok(Some(Ok(Message::Close(_)))) | Ok(None) | Ok(Some(Err(_))) => return None,
            Ok(Some(Ok(_))) => continue,
            Err(_) => panic!("nothing came in 60 s"),
        }
    }
}

/// Pairs with `code`, named `name`: the server's answer, the connection then closed.
async fn pair(addr: SocketAddr, code: &str, name: &str) -> ToClient {
    let mut ws = open(addr).await;
    send(
        &mut ws,
        &ToServer::Pair {
            code: code.into(),
            name: name.into(),
        },
    )
    .await;
    let answer = next(&mut ws).await.expect("an answer to the pairing");
    assert!(next(&mut ws).await.is_none(), "closed after the pairing");
    answer
}

/// Connected with `token`: its answer to the handshake.
async fn hello(addr: SocketAddr, token: &str) -> (Ws, ToClient) {
    let mut ws = open(addr).await;
    send(
        &mut ws,
        &ToServer::Hello {
            version: VERSION.into(),
            token: token.into(),
            session: None,
        },
    )
    .await;
    let first = next(&mut ws).await.expect("an answer to the handshake");
    (ws, first)
}

async fn call(ws: &mut Ws, id: u64, cmd: &str, args: Value) -> ToClient {
    let Value::Object(args) = args else { panic!() };
    send(
        ws,
        &ToServer::Call {
            id,
            cmd: cmd.into(),
            args,
            body: None,
            headers: Default::default(),
        },
    )
    .await;
    loop {
        match next(ws).await.expect("an answer") {
            reply @ ToClient::Reply { id: got, .. } if got == id => return reply,
            _ => continue,
        }
    }
}

/// `escouade-server` on `data`, on a free port of 127.0.0.1: its API, and what stops it.
async fn started(
    data: DataDir,
) -> (
    Arc<RemoteApi>,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<anyhow::Result<()>>,
) {
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        server::serve(
            data,
            Endpoint::port(0),
            Limits::default(),
            move |api| {
                let _ = ready_tx.send(api.clone());
            },
            async move {
                let _ = stop_rx.await;
                "SIGTERM"
            },
        )
        .await
    });
    let api = ready_rx.await.expect("the server is ready");
    (api, stop_tx, task)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_device_pairs_by_its_code_connects_with_its_token_and_the_log_says_neither() {
    let capturing = capture_log();
    // Under nextest (the CI's), each test has its process: the log is this test's, checked whatever.
    if std::env::var_os("NEXTEST_RUN_ID").is_some() {
        assert!(capturing, "the log of this process is captured");
    }
    let (dir, data) = data("server-pair");
    let root = data.root().to_path_buf();
    let (api, stop, task) = started(data.clone()).await;
    let addr = api.addr();
    assert!(addr.ip().is_loopback(), "{addr}");
    // No code asked for yet: refused, as a failed handshake.
    let answer = pair(addr, "ABCD-EFGH", "Portable").await;
    assert!(
        matches!(&answer, ToClient::Refused { code, .. } if code == "pairing"),
        "{answer:?}"
    );
    // `escouade-server pair` beside the server (another process: its file).
    let code = pairing::create(&DataDir::new(root.clone()), crate::model::now_ms()).unwrap();
    let wrong = if code.text.starts_with('A') {
        "BBBB-BBBB"
    } else {
        "AAAA-AAAA"
    };
    let answer = pair(addr, wrong, "Portable").await;
    let ToClient::Refused {
        code: c, reason, ..
    } = &answer
    else {
        panic!("{answer:?}")
    };
    assert_eq!(c, "pairing");
    assert_eq!(reason.as_deref(), Some("wrong"));
    let answer = pair(addr, &code.text, "Portable du bureau").await;
    let ToClient::Paired { device, token } = answer else {
        panic!("{answer:?}")
    };
    assert_eq!(device.name, "Portable du bureau");
    assert!(token.len() >= 32, "{token}");
    // Given again: refused.
    let again = pair(addr, &code.text, "Autre").await;
    assert!(matches!(&again, ToClient::Refused { code, .. } if code == "pairing"));
    // The token opens the API; the server says its secrets are in a file.
    let (mut ws, welcome) = hello(addr, &token).await;
    let ToClient::Welcome {
        device: name,
        secrets_in_file,
        ..
    } = &welcome
    else {
        panic!("{welcome:?}")
    };
    assert_eq!(name, "Portable du bureau");
    assert!(*secrets_in_file);
    // A command, through the API.
    let r = repo(&dir);
    let created = call(
        &mut ws,
        1,
        "create_project",
        json!({ "path": r.to_string_lossy(), "name": "demo", "color": "oklch(0.72 0.12 48)", "worktreePerAgent": false }),
    )
    .await;
    assert!(
        matches!(&created, ToClient::Reply { ok: Some(_), .. }),
        "{created:?}"
    );
    // The token is in the server's file of secrets, readable by its owner alone; the list of devices holds none.
    let secrets = std::fs::read_to_string(root.join(server::secrets::FILE)).unwrap();
    assert!(secrets.contains(&token));
    let list = std::fs::read_to_string(root.join(crate::remoteapi::devices::FILE)).unwrap();
    assert!(!list.contains(&token), "{list}");
    // `escouade-server devices revoke` beside it: the connection closes within seconds, the token no longer opens.
    let beside = Devices::load(
        &DataDir::new(root.clone()),
        Arc::new(FileStore::open(&DataDir::new(root.clone())).unwrap()),
    );
    assert_eq!(beside.list().len(), 1);
    assert!(beside.revoke(&device.id).unwrap());
    let closed = tokio::time::timeout(Duration::from_secs(20), async {
        while next(&mut ws).await.is_some() {}
    })
    .await;
    assert!(closed.is_ok(), "the revoked device's connection is closed");
    let (_, refused) = hello(addr, &token).await;
    assert!(
        matches!(&refused, ToClient::Refused { code, .. } if code == "token"),
        "{refused:?}"
    );
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    // Neither the code nor the token in the log.
    if capturing {
        let lines = captured().lock().unwrap().join("\n");
        assert!(lines.contains("paired"), "{lines}");
        for secret in [
            code.text.as_str(),
            &code.text.replace('-', ""),
            token.as_str(),
        ] {
            assert!(!lines.contains(secret), "the log says {secret}: {lines}");
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_server_stops_cleanly_its_state_written_and_its_folder_let_go_and_refuses_a_folder_held(
) {
    let (dir, data) = data("server-stop");
    let root = data.root().to_path_buf();
    let (api, stop, task) = started(data.clone()).await;
    // A second server on the same folder: refused, and says why.
    let second = server::serve(
        DataDir::new(root.clone()),
        Endpoint::port(0),
        Limits::default(),
        |_| {},
        async { "never" },
    )
    .await
    .expect_err("the folder is held");
    let said = format!("{second:#}");
    assert!(
        said.contains("un seul à la fois") || said.contains("one at a time"),
        "{said}"
    );
    // A project made through the API, then the stop the system asks for.
    let code = pairing::create(&DataDir::new(root.clone()), crate::model::now_ms()).unwrap();
    let ToClient::Paired { token, .. } = pair(api.addr(), &code.text, "Téléphone").await else {
        panic!()
    };
    let (mut ws, _) = hello(api.addr(), &token).await;
    let r = repo(&dir);
    let created = call(
        &mut ws,
        1,
        "create_project",
        json!({ "path": r.to_string_lossy(), "name": "gardé", "color": "oklch(0.72 0.12 48)", "worktreePerAgent": false }),
    )
    .await;
    assert!(
        matches!(&created, ToClient::Reply { ok: Some(_), .. }),
        "{created:?}"
    );
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    // Closed: its clients too, the state on the disk, the folder free again.
    assert!(next(&mut ws).await.is_none());
    let state = std::fs::read_to_string(DataDir::new(root.clone()).state_file()).unwrap();
    assert!(state.contains("gardé"), "{state}");
    let again = Headless::start(DataDir::new(root)).expect("the folder is free");
    assert!(again.core.projects.read().iter().any(|p| p.name == "gardé"));
    again.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn listen_serves_on_the_address_asked_for_in_so_many_words() {
    let (_dir, data) = data("server-listen");
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        server::serve(
            data,
            Endpoint {
                listen: Some("127.0.0.1".parse().unwrap()),
                ..Endpoint::default()
            },
            Limits::default(),
            move |api| {
                let _ = ready_tx.send(api.addr());
            },
            async move {
                let _ = stop_rx.await;
                "SIGTERM"
            },
        )
        .await
    });
    let addr = ready_rx.await.expect("the server is ready");
    assert!(addr.ip().is_loopback() && addr.port() != 0, "{addr}");
    // It answers there: a handshake without a token is refused by the protocol, not by the network.
    let (_, refused) = hello(addr, "").await;
    assert!(
        matches!(&refused, ToClient::Refused { code, .. } if code == "token"),
        "{refused:?}"
    );
    stop_tx.send(()).unwrap();
    task.await.unwrap().unwrap();
}

// ---------- ESC-515: the options as variables, the status, the signed update, the proxy's name ----------

#[test]
fn the_variables_of_a_container_stand_for_the_options_and_an_option_given_wins() {
    let env = |name: &str| -> Option<String> {
        match name {
            "ESCOUADE_SERVER_DATA" => Some("/data/escouade".into()),
            "ESCOUADE_SERVER_PORT" => Some("4300".into()),
            "ESCOUADE_SERVER_LISTEN" => Some("0.0.0.0".into()),
            "ESCOUADE_SERVER_ALLOW_HOSTS" => {
                Some(" Escouade.Example.org , other.example.org,".into())
            }
            _ => None,
        }
    };
    let a = server::parse_with(&args("pair"), env).unwrap();
    assert_eq!(a.command, Command::Pair);
    assert_eq!(
        a.data_dir.as_deref(),
        Some(std::path::Path::new("/data/escouade"))
    );
    assert_eq!(a.port, 4300);
    assert_eq!(a.listen, Some("0.0.0.0".parse().unwrap()));
    assert_eq!(a.hosts, ["escouade.example.org", "other.example.org"]);
    // An option given wins over its variable.
    let b = server::parse_with(
        &args("run --data-dir /srv/x --port 4301 --listen 127.0.0.1"),
        env,
    )
    .unwrap();
    assert_eq!(b.data_dir.as_deref(), Some(std::path::Path::new("/srv/x")));
    assert_eq!(
        (b.port, b.listen),
        (4301, Some("127.0.0.1".parse().unwrap()))
    );
    // An empty variable is none; a wrong one says which.
    assert_eq!(
        server::parse_with(&[], |n| (n == "ESCOUADE_SERVER_PORT").then(String::new))
            .unwrap()
            .port,
        DEFAULT_PORT
    );
    assert!(
        server::parse_with(&[], |n| (n == "ESCOUADE_SERVER_PORT").then(|| "x".into())).is_err()
    );
    assert!(
        server::parse_with(&[], |n| (n == "ESCOUADE_SERVER_ALLOW_HOSTS")
            .then(|| "a b".into()))
        .is_err()
    );
}

#[test]
fn health_update_and_allow_host_are_read_and_the_update_s_options_go_with_it_alone() {
    let none = |_: &str| None;
    assert_eq!(
        server::parse_with(&args("health"), none).unwrap().command,
        Command::Health
    );
    let u = server::parse_with(&args("update --to v2.3.1 --now"), none).unwrap();
    assert_eq!(u.command, Command::Update);
    assert_eq!(u.update.to.as_deref(), Some("v2.3.1"));
    assert!(u.update.now && !u.update.beta && !u.update.check);
    let c = server::parse_with(&args("update --beta --check"), none).unwrap();
    assert!(c.update.beta && c.update.check);
    assert_eq!(
        server::parse_with(
            &args("--allow-host Escouade.Example.org --allow-host b.example.org"),
            none
        )
        .unwrap()
        .hosts,
        ["escouade.example.org", "b.example.org"]
    );
    // The proxies whose X-Forwarded-For is read (SR1-04): by option and by variable.
    let proxies: Vec<std::net::IpAddr> =
        vec!["172.18.0.5".parse().unwrap(), "::1".parse().unwrap()];
    assert_eq!(
        server::parse_with(&args("--trust-proxy 172.18.0.5 --trust-proxy ::1"), none)
            .unwrap()
            .proxies,
        proxies
    );
    assert_eq!(
        server::parse_with(&[], |n| (n == "ESCOUADE_SERVER_TRUST_PROXIES")
            .then(|| "172.18.0.5, ::1".into()))
        .unwrap()
        .proxies,
        proxies
    );
    assert!(
        server::parse_with(&[], |n| (n == "ESCOUADE_SERVER_TRUST_PROXIES")
            .then(|| "traefik".into()))
        .is_err()
    );
    for bad in [
        "--trust-proxy",
        "--trust-proxy traefik",
        "run --now",
        "pair --to 2.3.0",
        "update --to latest",
        "update --to 2.3",
        "update --to 2.3.0/../x",
        "--allow-host",
        "--allow-host a/b",
    ] {
        assert!(server::parse_with(&args(bad), none).is_err(), "{bad}");
    }
    // `update --help` is the help: nothing downloaded.
    assert_eq!(
        server::parse_with(&args("update --help"), none)
            .unwrap()
            .command,
        Command::Help
    );
    let help = server::help();
    for word in ["health", "update", "--allow-host", "--trust-proxy"] {
        assert!(help.contains(word), "{word}: {help}");
    }
}

#[test]
fn versions_and_the_addresses_of_the_release_s_files() {
    for good in ["2.3.0", "10.0.12", "2.3.0-beta.4"] {
        assert!(update::is_version(good), "{good}");
    }
    for bad in ["2.3", "x.y.z", "2.3.0/x", "2.3.0-beta/..", "", "2.3.0.1"] {
        assert!(!update::is_version(bad), "{bad}");
    }
    assert_eq!(
        update::archive_url(update::RELEASES, "2.3.1", "arm64"),
        "https://github.com/guillaume-gagnaire/escouade/releases/download/v2.3.1/escouade-server-2.3.1-linux-arm64.tar.gz"
    );
    assert_eq!(
        update::channel_url("http://127.0.0.1:1/r/", false),
        "http://127.0.0.1:1/r/latest/download/latest.json"
    );
    assert!(update::channel_url(update::RELEASES, true).ends_with("/latest/download/beta.json"));
    assert!(
        ["x64", "arm64"].contains(&update::arch())
            || !cfg!(any(target_arch = "x86_64", target_arch = "aarch64"))
    );
}

#[test]
fn the_server_is_checked_with_the_key_of_the_app_s_updater() {
    let conf: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    assert_eq!(
        conf["plugins"]["updater"]["pubkey"].as_str(),
        Some(update::UPDATER_PUBLIC_KEY),
        "the server's key is the updater's"
    );
    // install.sh holds the same.
    let install = include_str!("../../deploy/install.sh");
    assert!(
        install.contains(update::UPDATER_PUBLIC_KEY),
        "install.sh checks with the updater's key"
    );
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/server-update")
            .join(name),
    )
    .unwrap()
}

#[test]
fn an_archive_passes_with_its_signature_alone_and_a_byte_changed_or_another_key_is_refused() {
    let archive = fixture("escouade-server-9.9.9-linux-x64.tar.gz");
    let sig = String::from_utf8(fixture("escouade-server-9.9.9-linux-x64.tar.gz.sig")).unwrap();
    let key = String::from_utf8(fixture("test-key.pub")).unwrap();
    update::verify(&archive, &sig, &key).expect("signed by the test key");
    let mut changed = archive.clone();
    let last = changed.len() - 1;
    changed[last] ^= 1;
    assert!(update::verify(&changed, &sig, &key).is_err());
    // The updater's key did not sign it.
    assert!(update::verify(&archive, &sig, update::UPDATER_PUBLIC_KEY).is_err());
    assert!(update::verify(&archive, "not base64 !", &key).is_err());
}

#[test]
fn an_update_puts_each_file_in_place_of_its_own_and_keeps_the_old_one() {
    let dir = crate::paths::test_dir("server-update-install");
    std::fs::write(dir.join("escouade-server"), "old program").unwrap();
    std::fs::create_dir_all(dir.join("seed")).unwrap();
    std::fs::write(dir.join("seed/old"), "old seed").unwrap();
    std::fs::write(dir.join("other"), "not the archive's").unwrap();
    let names = update::install(&fixture("escouade-server-9.9.9-linux-x64.tar.gz"), &dir).unwrap();
    assert_eq!(names, ["escouade-server", "seed"]);
    let program = std::fs::read_to_string(dir.join("escouade-server")).unwrap();
    assert!(program.contains("9.9.9"), "{program}");
    assert!(dir.join("seed/README").is_file() && !dir.join("seed/old").exists());
    assert_eq!(
        std::fs::read_to_string(dir.join("escouade-server.old")).unwrap(),
        "old program"
    );
    assert!(dir.join("seed.old/old").is_file());
    assert_eq!(
        std::fs::read_to_string(dir.join("other")).unwrap(),
        "not the archive's"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(dir.join("escouade-server"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(
            mode & 0o111,
            0o111,
            "the program is still executable: {mode:o}"
        );
    }
    // Once more: the old one before it goes.
    update::install(&fixture("escouade-server-9.9.9-linux-x64.tar.gz"), &dir).unwrap();
    assert!(std::fs::read_to_string(dir.join("escouade-server.old"))
        .unwrap()
        .contains("9.9.9"));
    // No staging folder is left.
    let left: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with(".escouade-server-update")
        })
        .collect();
    assert!(left.is_empty());
}

/// A tar.gz of `files` (path, content), the paths written as given (even one that leaves the folder).
fn targz(files: &[(&str, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let gz = flate2::write::GzEncoder::new(&mut out, flate2::Compression::fast());
        let mut b = tar::Builder::new(gz);
        for (path, content) in files {
            let mut h = tar::Header::new_old();
            let name = &mut h.as_old_mut().name;
            name[..path.len()].copy_from_slice(path.as_bytes());
            h.set_size(content.len() as u64);
            h.set_mode(0o755);
            h.set_entry_type(tar::EntryType::Regular);
            h.set_cksum();
            b.append(&h, content.as_bytes()).unwrap();
        }
        b.into_inner().unwrap().finish().unwrap();
    }
    out
}

#[test]
fn an_archive_without_the_program_or_with_a_path_out_of_its_folder_changes_nothing() {
    let dir = crate::paths::test_dir("server-update-refused");
    std::fs::write(dir.join("escouade-server"), "old program").unwrap();
    assert!(update::install(&targz(&[("README", "no program")]), &dir).is_err());
    assert!(update::install(
        &targz(&[("escouade-server", "x"), ("../escaped", "x")]),
        &dir
    )
    .is_err());
    assert!(!dir.parent().unwrap().join("escaped").exists());
    assert!(update::install(b"not an archive", &dir).is_err());
    assert_eq!(
        std::fs::read_to_string(dir.join("escouade-server")).unwrap(),
        "old program"
    );
    assert!(!dir.join("escouade-server.old").exists());
    // At the top of the archive too, not only in one folder.
    update::install(&targz(&[("escouade-server", "new program")]), &dir).unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.join("escouade-server")).unwrap(),
        "new program"
    );
}

fn status_of(pid: u32, busy: Option<&str>, at: i64) -> status::Status {
    status::Status {
        pid,
        version: VERSION.into(),
        api: "127.0.0.1:46480".into(),
        started_at: 0,
        at,
        busy: busy.map(String::from),
        turns: 0,
        supervisor: Some(status::Supervisor::Systemd),
    }
}

#[test]
fn health_needs_a_fresh_status_and_a_live_process_and_update_waits_for_the_work() {
    let alive = |pid: u32| pid == 7;
    assert!(status::healthy(&status_of(7, None, 1_000), 5_000, alive).is_ok());
    assert!(status::healthy(&status_of(8, None, 1_000), 5_000, alive).is_err());
    assert!(status::healthy(
        &status_of(7, None, 1_000),
        1_000 + status::STALE_MS + 1,
        alive
    )
    .is_err());
    assert_eq!(update::wait_of(None, alive), update::Wait::NotRunning);
    assert_eq!(
        update::wait_of(Some(&status_of(8, None, 0)), alive),
        update::Wait::NotRunning
    );
    assert_eq!(
        update::wait_of(Some(&status_of(7, None, 0)), alive),
        update::Wait::Now
    );
    assert_eq!(
        update::wait_of(Some(&status_of(7, Some("turn"), 0)), alive),
        update::Wait::Busy("turn".into())
    );
    assert!(!status::busy_text("turn").is_empty());
    // What runs the server, from its variables.
    assert_eq!(status::supervisor_of(|_| None), None);
    assert_eq!(
        status::supervisor_of(|n| (n == "INVOCATION_ID").then(|| "abc".into())),
        Some(status::Supervisor::Systemd)
    );
    assert_eq!(
        status::supervisor_of(
            |n| matches!(n, "INVOCATION_ID" | "ESCOUADE_SERVER_IMAGE").then(|| "1".into())
        ),
        Some(status::Supervisor::Docker)
    );
    // The file, written whole and read back.
    let data = data_dir("server-status-file");
    assert!(status::read(&data).is_none());
    status::write(&data, &status_of(7, Some("question"), 3)).unwrap();
    assert_eq!(status::read(&data), Some(status_of(7, Some("question"), 3)));
    status::remove(&data);
    assert!(status::read(&data).is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_running_server_writes_its_status_says_how_to_pair_and_removes_it_at_its_stop() {
    let capturing = capture_log();
    let (_dir, data) = data("server-status");
    let (api, stop, task) = started(data.clone()).await;
    let mut seen = None;
    for _ in 0..100 {
        if let Some(s) = status::read(&data) {
            seen = Some(s);
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let s = seen.expect("the status is written");
    assert_eq!(s.pid, std::process::id());
    assert_eq!(s.version, VERSION);
    assert_eq!(s.busy, None, "at rest");
    assert_eq!(s.api, format!("127.0.0.1:{}", api.addr().port()));
    assert!(status::healthy(&s, crate::model::now_ms(), status::alive).is_ok() || !cfg!(unix));
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    assert!(status::read(&data).is_none(), "removed at the stop");
    if capturing && std::env::var_os("NEXTEST_RUN_ID").is_some() {
        let lines = captured().lock().unwrap().join("\n");
        assert!(
            lines.contains("escouade-server pair"),
            "the log says how to pair: {lines}"
        );
    }
    assert!(server::first_start_hint(Some(status::Supervisor::Docker))
        .contains("docker exec <container> escouade-server pair"));
    assert!(!server::first_start_hint(None).contains("docker"));
}

/// A WebSocket opened with `host` in its `Host`.
async fn open_as(addr: SocketAddr, host: &str) -> Result<Ws, String> {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
    let mut req = format!("ws://{addr}{}", proto::PATH)
        .into_client_request()
        .unwrap();
    req.headers_mut().insert("host", host.parse().unwrap());
    tokio_tungstenite::client_async(req, tcp)
        .await
        .map(|(ws, _)| ws)
        .map_err(|e| e.to_string())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_name_a_tls_proxy_serves_the_api_under_is_taken_when_asked_for_and_any_other_still_refused(
) {
    let (_dir, data) = data("server-allow-host");
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        server::serve(
            data,
            Endpoint {
                hosts: vec!["escouade.example.org".into()],
                ..Endpoint::default()
            },
            Limits::default(),
            move |api| {
                let _ = ready_tx.send(api.addr());
            },
            async move {
                let _ = stop_rx.await;
                "SIGTERM"
            },
        )
        .await
    });
    let addr = ready_rx.await.unwrap();
    for host in [
        "escouade.example.org",
        "Escouade.Example.org:443",
        "127.0.0.1:9",
    ] {
        let mut ws = open_as(addr, host)
            .await
            .unwrap_or_else(|e| panic!("{host}: {e}"));
        send(
            &mut ws,
            &ToServer::Hello {
                version: VERSION.into(),
                token: String::new(),
                session: None,
            },
        )
        .await;
        let answer = next(&mut ws).await.expect("an answer");
        assert!(
            matches!(&answer, ToClient::Refused { code, .. } if code == "token"),
            "{host}: {answer:?}"
        );
    }
    for host in ["evil.example.org", "escouade.example.org.evil.org", "[::2]"] {
        let refused = open_as(addr, host).await;
        assert!(
            refused.is_err(),
            "{host} is refused before the WebSocket opens"
        );
    }
    stop_tx.send(()).unwrap();
    task.await.unwrap().unwrap();
}

#[test]
fn a_server_older_than_its_client_says_how_to_put_it_at_the_client_s_version() {
    let by_command = proto::server_update("2.3.1", false);
    assert_eq!(by_command.how, "command");
    assert_eq!(by_command.command, "escouade-server update --to 2.3.1");
    let by_image = proto::server_update("2.3.1", true);
    assert_eq!(by_image.how, "image");
    assert!(
        by_image
            .command
            .contains("ghcr.io/guillaume-gagnaire/escouade/escouade-server:2.3.1"),
        "{}",
        by_image.command
    );
    // What a client sends is never more than a version in it.
    assert_eq!(
        proto::server_update("2.3.1; rm -rf ~", false).command,
        "escouade-server update --to 2.3.1rm-rf"
    );
}

// ---------- SR1-07, SR1-13 (ESC-640): no downgrade, the signature bound to its archive, a private folder and log ----------

fn asked(line: &str) -> update::Asked {
    server::parse_with(&args(&format!("update {line}")), |_| None)
        .unwrap()
        .update
}

#[test]
fn update_never_takes_an_older_version_unless_it_is_named_with_downgrade() {
    // The published version older than the program's: refused, whatever latest.json says.
    let e = update::target_of("2.3.0", &asked(""), Some("2.2.10"), None).unwrap_err();
    assert!(e.to_string().contains("--downgrade"), "{e}");
    // `--to` an older one: refused without `--downgrade`, taken with it.
    assert!(update::target_of("2.3.0", &asked("--to 2.2.10"), None, None).is_err());
    assert_eq!(
        update::target_of("2.3.0", &asked("--to v2.2.10 --downgrade"), None, None).unwrap(),
        "2.2.10"
    );
    // The same version or a newer one: taken.
    assert_eq!(
        update::target_of("2.3.0", &asked(""), Some("v2.3.0"), None).unwrap(),
        "2.3.0"
    );
    assert_eq!(
        update::target_of("2.3.0", &asked("--to 2.3.1"), None, None).unwrap(),
        "2.3.1"
    );
    // A beta is older than its stable version: a server on 2.3.0 never goes to 2.3.0-beta.4.
    assert!(update::target_of("2.3.0", &asked("--to 2.3.0-beta.4"), None, None).is_err());
    // A server on a beta stays on it until the stable version that follows it.
    assert!(update::target_of("2.3.0-beta.4", &asked(""), Some("2.2.10"), None).is_err());
    assert_eq!(
        update::target_of("2.3.0-beta.4", &asked(""), Some("2.3.0"), None).unwrap(),
        "2.3.0"
    );
    // `--downgrade` alone (without the version it names) is refused, and goes with `update` alone.
    for bad in [
        "update --downgrade",
        "update --beta --downgrade",
        "--downgrade",
    ] {
        assert!(server::parse_with(&args(bad), |_| None).is_err(), "{bad}");
    }
    // Not a version: refused.
    assert!(update::target_of("2.3.0", &asked(""), Some("latest"), None).is_err());
}

#[test]
fn update_with_beta_takes_the_higher_of_the_two_channels() {
    let beta = asked("--beta");
    // beta.json ahead: the beta.
    assert_eq!(
        update::target_of("2.2.10", &beta, Some("2.2.10"), Some("2.3.0-beta.2")).unwrap(),
        "2.3.0-beta.2"
    );
    // beta.json behind latest.json (a stable version came out after the last beta): the stable one, never the old beta.
    assert_eq!(
        update::target_of("2.2.10", &beta, Some("2.3.0"), Some("2.3.0-beta.2")).unwrap(),
        "2.3.0"
    );
    assert_eq!(
        update::target_of("2.3.0", &beta, Some("2.3.0"), Some("2.2.0-beta.9")).unwrap(),
        "2.3.0"
    );
    // No beta.json: latest.json's.
    assert_eq!(
        update::target_of("2.2.10", &beta, Some("2.3.0"), None).unwrap(),
        "2.3.0"
    );
    // Without --beta, beta.json is not read.
    assert_eq!(
        update::target_of("2.2.10", &asked(""), Some("2.2.10"), Some("2.3.0-beta.2")).unwrap(),
        "2.2.10"
    );
}

#[test]
fn a_signature_that_does_not_name_its_archive_is_refused() {
    let archive = fixture("escouade-server-9.9.9-linux-x64.tar.gz");
    let sig = String::from_utf8(fixture("escouade-server-9.9.9-linux-x64.tar.gz.sig")).unwrap();
    let key = String::from_utf8(fixture("test-key.pub")).unwrap();
    // Its trusted comment (signed) says `file:escouade-server-9.9.9-linux-x64.tar.gz`.
    let comment = update::verify(&archive, &sig, &key).unwrap();
    assert!(
        comment.contains("file:escouade-server-9.9.9-linux-x64.tar.gz"),
        "{comment}"
    );
    update::verify_archive(&archive, &sig, &key, &update::archive_name("9.9.9", "x64")).unwrap();
    // The same signed archive taken for another version, another architecture, or a name it only starts with.
    for other in [
        update::archive_name("9.9.10", "x64"),
        update::archive_name("1.0.0", "x64"),
        update::archive_name("9.9.9", "arm64"),
        "escouade-server-9.9.9-linux-x64.tar".to_string(),
    ] {
        let e = update::verify_archive(&archive, &sig, &key, &other).unwrap_err();
        assert!(e.to_string().contains(&other), "{other}: {e}");
    }
}

/// A fake release on 127.0.0.1 (a port the system picks): each path of `files` answered with its bytes, any other 404.
fn fake_releases(files: Vec<(String, Vec<u8>)>) -> String {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/r", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut request = Vec::new();
            let mut buf = [0u8; 1024];
            while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => request.extend_from_slice(&buf[..n]),
                }
            }
            let text = String::from_utf8_lossy(&request).to_string();
            let path = text
                .lines()
                .next()
                .and_then(|l| l.split(' ').nth(1))
                .unwrap_or("")
                .to_string();
            let (status, body) = match files.iter().find(|(p, _)| format!("/r/{p}") == path) {
                Some((_, b)) => ("200 OK", b.clone()),
                None => ("404 Not Found", Vec::new()),
            };
            let _ = stream.write_all(
                format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .as_bytes(),
            );
            let _ = stream.write_all(&body);
        }
    });
    base
}

fn channel(version: &str) -> Vec<u8> {
    json!({ "version": version, "platforms": {} })
        .to_string()
        .into_bytes()
}

#[test]
fn update_against_a_fake_release_refuses_an_older_version_and_a_renamed_archive() {
    let archive = fixture("escouade-server-9.9.9-linux-x64.tar.gz");
    let sig = fixture("escouade-server-9.9.9-linux-x64.tar.gz.sig");
    let key = String::from_utf8(fixture("test-key.pub")).unwrap();
    let name = |v: &str| format!("download/v{v}/{}", update::archive_name(v, "x64"));
    let base = fake_releases(vec![
        ("latest/download/latest.json".into(), channel("1.0.0")),
        ("latest/download/beta.json".into(), channel("9.9.9")),
        (name("9.9.9"), archive.clone()),
        (format!("{}.sig", name("9.9.9")), sig.clone()),
        // The archive of 9.9.9 and its signature, served as 9.9.10's.
        (name("9.9.10"), archive.clone()),
        (format!("{}.sig", name("9.9.10")), sig.clone()),
    ]);
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        // latest.json says 1.0.0, older than the program: refused.
        let e = update::read_target(&client, &base, "2.3.0", &asked("--check"))
            .await
            .unwrap_err();
        assert!(e.to_string().contains("1.0.0"), "{e}");
        // With --beta, the higher of 1.0.0 and 9.9.9.
        assert_eq!(
            update::read_target(&client, &base, "2.3.0", &asked("--beta"))
                .await
                .unwrap(),
            "9.9.9"
        );
        // --to an older version, with --downgrade: nothing read, taken.
        assert_eq!(
            update::read_target(&client, &base, "2.3.0", &asked("--to 1.0.0 --downgrade"))
                .await
                .unwrap(),
            "1.0.0"
        );
        // The archive of 9.9.9 passes; the same, served as 9.9.10, is refused though its signature is good.
        let bytes = update::download(&client, &base, "9.9.9", "x64", &key)
            .await
            .unwrap();
        assert_eq!(bytes, archive);
        let e = update::download(&client, &base, "9.9.10", "x64", &key)
            .await
            .unwrap_err();
        assert!(
            format!("{e:#}").contains("escouade-server-9.9.10-linux-x64.tar.gz"),
            "{e:#}"
        );
        // A version the release does not hold: an error, nothing answered.
        assert!(update::download(&client, &base, "9.9.11", "x64", &key)
            .await
            .is_err());
    });
}

#[cfg(unix)]
#[test]
fn the_server_s_data_folder_is_0700_and_its_log_0600_whatever_the_umask() {
    use std::os::unix::fs::PermissionsExt;
    let mode = |p: &std::path::Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    let data_dir = |p: &std::path::Path| {
        let parsed = server::parse_with(
            &["--data-dir".to_string(), p.to_string_lossy().to_string()],
            |_| None,
        )
        .unwrap();
        server::data_of(&parsed).unwrap()
    };
    let dir = crate::paths::test_dir("server-private");
    // A folder an earlier start left in 0755, and a log and an older one in 0644.
    let root = dir.join("data");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755)).unwrap();
    for name in [server::logs::FILE, "server.1.log"] {
        std::fs::write(root.join(name), "old line\n").unwrap();
        std::fs::set_permissions(root.join(name), std::fs::Permissions::from_mode(0o644)).unwrap();
    }
    let data = data_dir(&root);
    assert_eq!(mode(data.root()), 0o700);
    let _logger = server::logs::Logger::new(data.root(), 40, false, false);
    assert_eq!(mode(&root.join(server::logs::FILE)), 0o600);
    assert_eq!(mode(&root.join("server.1.log")), 0o600);
    // A new folder, and the log made after a turn: the same, under the usual mask 0022.
    // SAFETY: the process's mask, set back right after; the other tests' files do not lean on it.
    let before = unsafe { libc::umask(0o022) };
    let fresh = dir.join("fresh");
    let data = data_dir(&fresh);
    let logger = server::logs::Logger::new(data.root(), 40, false, false);
    logger.write(log::Level::Info, "a line long enough to turn the file");
    logger.write(log::Level::Info, "and another, in a new file");
    unsafe { libc::umask(before) };
    assert_eq!(mode(&fresh), 0o700);
    assert_eq!(mode(&fresh.join(server::logs::FILE)), 0o600);
    assert_eq!(mode(&fresh.join("server.1.log")), 0o600);
}
