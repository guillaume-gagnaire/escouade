//! `escouade-server` (ESC-514, `docs/REMOTE.md`, « Le mode serveur »): its command line, the pairing of a device and its
//! refusals (expired, given again, wrong, too many tries), the secrets in a file readable by its owner alone, the lock of
//! the data folder, the clean stop, and a log that says neither a code nor a token.

use crate::core_tests::repo;
use crate::headless::{with_file_lock, Headless};
use crate::headless_tests::data;
use crate::integrations::secrets::SecretStore;
use crate::paths::DataDir;
use crate::remoteapi::devices::Devices;
use crate::remoteapi::pairing::{self, Refusal, LIFETIME_MS, MAX_TRIES};
use crate::remoteapi::proto::{self, ToClient, ToServer};
use crate::remoteapi::server::{Limits, RemoteApi};
use crate::server::{self, secrets::FileStore, Args, Command, DEFAULT_PORT};
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
    let run = server::parse(&[]).unwrap();
    assert_eq!(
        run,
        Args {
            command: Command::Run,
            data_dir: None,
            port: DEFAULT_PORT,
            listen: None,
            debug: false,
        }
    );
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
            Err(Refusal::Wrong {
                left: MAX_TRIES - 1
            })
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

#[test]
fn wrong_pairing_codes_cost_a_try_each_and_the_last_one_ends_the_code() {
    let data = data_dir("pair-tries");
    let now = 1_700_000_000_000;
    let code = pairing::create(&data, now).unwrap();
    let wrong = if code.text.starts_with('A') {
        "BBBB-BBBB"
    } else {
        "AAAA-AAAA"
    };
    for left in (1..MAX_TRIES).rev() {
        assert_eq!(
            pairing::redeem(&data, wrong, now),
            Err(Refusal::Wrong { left })
        );
    }
    // Too short, too long, empty: wrong as well, never hashed whole.
    let code2 = pairing::create(&data, now).unwrap();
    for bad in ["", "ABC", &"A".repeat(10_000)] {
        assert!(matches!(
            pairing::redeem(&data, bad, now),
            Err(Refusal::Wrong { .. })
        ));
    }
    assert_eq!(pairing::redeem(&data, &code2.text, now), Ok(()));
    // The fifth wrong try ends the code: the right one comes too late.
    let code = pairing::create(&data, now).unwrap();
    let wrong = if code.text.starts_with('A') {
        "BBBB-BBBB"
    } else {
        "AAAA-AAAA"
    };
    for _ in 1..MAX_TRIES {
        assert!(matches!(
            pairing::redeem(&data, wrong, now),
            Err(Refusal::Wrong { .. })
        ));
    }
    assert_eq!(
        pairing::redeem(&data, wrong, now),
        Err(Refusal::TooManyTries)
    );
    assert_eq!(
        pairing::redeem(&data, &code.text, now),
        Err(Refusal::NoCode)
    );
    // Each refusal says what to do, in words.
    for r in [
        Refusal::NoCode,
        Refusal::Expired,
        Refusal::Wrong { left: 2 },
        Refusal::TooManyTries,
    ] {
        assert!(
            r.text().contains("pair") || r.text().contains('2'),
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
            None,
            0,
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
    let ToClient::Refused { code: c, error, .. } = &answer else {
        panic!("{answer:?}")
    };
    assert_eq!(c, "pairing");
    assert!(error.contains(&(MAX_TRIES - 1).to_string()), "{error}");
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
        None,
        0,
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
            Some("127.0.0.1".parse().unwrap()),
            0,
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
