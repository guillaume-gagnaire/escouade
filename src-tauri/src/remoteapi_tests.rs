//! The remote API of the core (ESC-478, `docs/REMOTE.md`, « La fondation commune »): the single list of the commands,
//! then a WebSocket client of test against the core without a window (`Headless`) and the fake `claude`.

use crate::core_tests::repo;
use crate::headless::Headless;
use crate::headless_tests::{data, wait};
use crate::remoteapi::proto::{self, Frame, ToClient, ToServer};
use crate::remoteapi::server::RemoteApi;
use crate::remoteapi::{self, codes, list, Class};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

type Ws = tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>;

/// The version this client says it is: the app's, as the server's.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A core without a window on a data folder of its own (the fake `claude`), its remote API served on a free port of
/// 127.0.0.1, and the token of a device « Téléphone ».
async fn served(name: &str) -> (PathBuf, Headless, Arc<RemoteApi>, String) {
    let (dir, data) = data(name);
    let h = Headless::start(data).unwrap();
    let api = h.serve_remote(0).await.unwrap();
    let (_, token) = api.devices().add("Téléphone").unwrap();
    (dir, h, api, token)
}

/// The WebSocket opened on `addr` (its path, its `Host` and its `Origin` as given), before any message.
async fn open(
    addr: SocketAddr,
    path: &str,
    host: Option<&str>,
    origin: Option<&str>,
) -> Result<Ws, String> {
    let mut req = format!("ws://{addr}{path}").into_client_request().unwrap();
    if let Some(h) = host {
        req.headers_mut().insert("host", h.parse().unwrap());
    }
    if let Some(o) = origin {
        req.headers_mut().insert("origin", o.parse().unwrap());
    }
    let tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
    tokio_tungstenite::client_async(req, tcp)
        .await
        .map(|(ws, _)| ws)
        .map_err(|e| match e {
            tokio_tungstenite::tungstenite::Error::Http(resp) => {
                format!("HTTP {}", resp.status().as_u16())
            }
            other => other.to_string(),
        })
}

/// A client of the remote API, as the tests drive it.
struct Client {
    ws: Ws,
    next: u64,
    /// What came and was not taken yet.
    seen: Vec<Message>,
}

impl Client {
    /// Connected to `api` with `token`, saying `version`: the server's first answer.
    async fn hello(api: &RemoteApi, version: &str, token: &str) -> (Self, ToClient) {
        let ws = open(api.addr(), proto::PATH, None, None).await.unwrap();
        let mut c = Client {
            ws,
            next: 1,
            seen: Vec::new(),
        };
        c.send(&ToServer::Hello {
            version: version.into(),
            token: token.into(),
        })
        .await;
        let first = c.recv().await.expect("an answer to the handshake");
        (c, parse(&first).expect("a message of the protocol"))
    }

    /// Connected, its handshake passed.
    async fn connect(api: &RemoteApi, token: &str) -> Self {
        let (c, welcome) = Self::hello(api, VERSION, token).await;
        assert!(matches!(welcome, ToClient::Welcome { .. }), "{welcome:?}");
        c
    }

    async fn send(&mut self, message: &ToServer) {
        self.ws
            .send(Message::text(serde_json::to_string(message).unwrap()))
            .await
            .unwrap();
    }

    /// The next message, 60 s at most; None once the connection is closed.
    async fn recv(&mut self) -> Option<Message> {
        match tokio::time::timeout(Duration::from_secs(60), self.ws.next()).await {
            Ok(Some(Ok(Message::Close(_)))) | Ok(None) | Ok(Some(Err(_))) => None,
            Ok(Some(Ok(m))) => Some(m),
            Err(_) => panic!("nothing came in 60 s"),
        }
    }

    /// Whether the server closed the connection (what came before it skipped).
    async fn closed(&mut self) -> bool {
        loop {
            match tokio::time::timeout(Duration::from_secs(30), self.ws.next()).await {
                Ok(Some(Ok(Message::Close(_)))) | Ok(None) | Ok(Some(Err(_))) => return true,
                Ok(Some(Ok(_))) => continue,
                Err(_) => return false,
            }
        }
    }

    /// The command `cmd` called with `args`: its answer (the messages that came meanwhile kept).
    async fn call(&mut self, cmd: &str, args: Value) -> ToClient {
        let id = self.next;
        self.next += 1;
        let Value::Object(args) = args else {
            panic!("args are an object")
        };
        self.send(&ToServer::Call {
            id,
            cmd: cmd.into(),
            args,
        })
        .await;
        loop {
            let m = self
                .recv()
                .await
                .unwrap_or_else(|| panic!("closed before the answer to {cmd}"));
            if let Some(reply @ ToClient::Reply { id: got, .. }) = parse(&m) {
                if got == id {
                    return reply;
                }
            }
            self.seen.push(m);
        }
    }

    /// Its value, the call gone well.
    async fn ok(&mut self, cmd: &str, args: Value) -> Value {
        match self.call(cmd, args).await {
            ToClient::Reply {
                ok, error: None, ..
            } => ok.unwrap_or(Value::Null),
            other => panic!("{cmd}: {other:?}"),
        }
    }

    /// The first message, kept or to come, that `pred` takes.
    async fn until(&mut self, what: &str, pred: impl Fn(&Message) -> bool) -> Message {
        if let Some(i) = self.seen.iter().position(&pred) {
            return self.seen.remove(i);
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
        loop {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            match tokio::time::timeout(left, self.ws.next()).await {
                Ok(Some(Ok(m))) if pred(&m) => return m,
                Ok(Some(Ok(m))) => self.seen.push(m),
                Ok(other) => panic!("closed waiting for {what}: {other:?}"),
                Err(_) => panic!("timed out waiting for {what}"),
            }
        }
    }
}

fn parse(m: &Message) -> Option<ToClient> {
    match m {
        Message::Text(t) => serde_json::from_str(t).ok(),
        _ => None,
    }
}

/// The data of a message of the channel `channel`, in JSON.
fn channel_data(m: &Message, channel: u32) -> Option<Value> {
    match parse(m)? {
        ToClient::Channel { channel: c, data } if c == channel => Some(data),
        _ => None,
    }
}

/// The bytes of a binary message of the channel `channel`.
fn channel_bytes(m: &Message, channel: u32) -> Option<Vec<u8>> {
    match m {
        Message::Binary(b) => match proto::read_frame(b)? {
            (Frame::Channel, id, bytes) if id == u64::from(channel) => Some(bytes.to_vec()),
            _ => None,
        },
        _ => None,
    }
}

/// A project of a new repository, made through the API.
async fn project(c: &mut Client, dir: &Path) -> Value {
    let r = repo(dir);
    c.ok(
        "create_project",
        json!({
            "path": r.to_string_lossy(),
            "name": "demo",
            "color": "oklch(0.72 0.12 48)",
            "worktreePerAgent": false,
        }),
    )
    .await
}

fn code_of(reply: &ToClient) -> Option<&str> {
    match reply {
        ToClient::Reply { code, .. } => code.as_deref(),
        ToClient::Refused { code, .. } => Some(code),
        _ => None,
    }
}

/// The functions of `src` written as commands (`#[tauri::command]`, `#[escouade_command::command]`), by name.
fn commands_written() -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut stack = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            // The tests' own commands (`windowthread_tests`) are not the app's.
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if path.extension().is_none_or(|e| e != "rs") || name.ends_with("tests.rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            let mut lines = text.lines();
            while let Some(line) = lines.next() {
                let line = line.trim();
                if !(line.starts_with("#[tauri::command")
                    || line.starts_with("#[escouade_command::command"))
                {
                    continue;
                }
                // Its function, past its other attributes and its comments.
                for next in lines.by_ref() {
                    let next = next.trim();
                    if let Some(i) = next.find("fn ") {
                        if next.starts_with("pub")
                            || next.starts_with("fn")
                            || next.starts_with("async")
                        {
                            let name: String = next[i + 3..]
                                .chars()
                                .take_while(|c| c.is_alphanumeric() || *c == '_')
                                .collect();
                            found.insert(name);
                            break;
                        }
                    }
                }
            }
        }
    }
    found
}

#[test]
fn the_ipc_and_the_remote_api_serve_the_same_commands_each_written_once() {
    let ipc: BTreeSet<String> = list::IPC.iter().map(|s| s.to_string()).collect();
    let api: BTreeSet<String> = list::COMMANDS.iter().map(|c| c.name.to_string()).collect();
    assert_eq!(
        ipc.len(),
        list::IPC.len(),
        "a command twice in the IPC's list"
    );
    assert_eq!(
        api.len(),
        list::COMMANDS.len(),
        "a command twice in the API's table"
    );
    let missing_from_api: Vec<_> = ipc.difference(&api).collect();
    let missing_from_ipc: Vec<_> = api.difference(&ipc).collect();
    assert!(
        missing_from_api.is_empty(),
        "served by the IPC, not by the remote API: {missing_from_api:?}"
    );
    assert!(
        missing_from_ipc.is_empty(),
        "served by the remote API, not by the IPC: {missing_from_ipc:?}"
    );
    // And every command written in the crate is in the list (else neither serves it).
    let written = commands_written();
    let unlisted: Vec<_> = written.difference(&ipc).collect();
    let unwritten: Vec<_> = ipc.difference(&written).collect();
    assert!(
        unlisted.is_empty(),
        "written but in no class of remoteapi/list.rs: {unlisted:?}"
    );
    assert!(
        unwritten.is_empty(),
        "in the list but written nowhere: {unwritten:?}"
    );
    assert!(written.len() > 300, "{}", written.len());
}

#[test]
fn each_command_has_its_class_as_docs_remote_says() {
    let class = |name: &str| remoteapi::find(name).map(|c| c.class);
    assert_eq!(class("create_agent"), Some(Class::Served));
    assert_eq!(class("term_spawn"), Some(Class::Served));
    assert_eq!(class("subscribe"), Some(Class::Served));
    // The window, the dialogs, the microphone, the global shortcut, the updater: the desktop's.
    for desk in [
        "open_url",
        "save_image",
        "voice_start",
        "overseer_shortcut_set",
        "update_check",
        "quit_app",
    ] {
        assert_eq!(class(desk), Some(Class::Desk), "{desk}");
    }
    assert_eq!(class("create_project"), Some(Class::Moves { served: true }));
    assert_eq!(
        class("clipboard_image"),
        Some(Class::Moves { served: false })
    );
    assert_eq!(class("attach_chunk"), Some(Class::Moves { served: false }));
    assert_eq!(class("no_such_command"), None);
    // `docs/REMOTE.md` lists them all, in the same classes.
    let doc =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/REMOTE.md"))
            .unwrap();
    // Its section « Le classement des commandes », one part per class, each command in one of them.
    let start = doc
        .find("### Le classement des commandes")
        .expect("the section of the classes");
    let section = &doc[start..];
    let end = [
        "
## ", "
### ",
    ]
    .iter()
    .filter_map(|h| section[4..].find(h))
    .min()
    .map_or(section.len(), |i| i + 4);
    let section = &section[..end];
    let parts: Vec<(&str, &str)> = section
        .split(
            "
#### ",
        )
        .skip(1)
        .map(|p| p.split_once('\n').unwrap_or((p, "")))
        .collect();
    for c in list::COMMANDS {
        let wanted = match c.class {
            Class::Served => "Servies",
            Class::Desk => "Du poste seulement",
            Class::Moves { served: true } => "Changent de machine, servies",
            Class::Moves { served: false } => "Changent de machine, refusées",
        };
        let named = format!("`{}`", c.name);
        let found: Vec<&str> = parts
            .iter()
            .filter(|(_, body)| body.contains(&named))
            .map(|(title, _)| *title)
            .collect();
        assert!(
            found.len() == 1 && found[0].starts_with(wanted),
            "{}: wanted in « {wanted} », found in {found:?}",
            c.name
        );
    }
}

// ---------- the WebSocket, against the core without a window ----------

#[tokio::test(flavor = "multi_thread")]
async fn the_handshake_passes_with_a_device_token_and_the_server_s_version_and_a_command_answers() {
    let (dir, h, api, token) = served("remote-handshake").await;
    let (mut c, welcome) = Client::hello(&api, VERSION, &token).await;
    assert_eq!(
        welcome,
        ToClient::Welcome {
            version: VERSION.into(),
            client: "c1".into(),
            device: "Téléphone".into(),
        }
    );
    assert_eq!(api.clients(), 1);
    // A command and its answer, as the IPC gives it.
    let p = project(&mut c, &dir).await;
    assert_eq!(p["name"], "demo");
    assert!(h.core.projects.read().iter().any(|x| x.id == p["id"]));
    let closed = c.ok("closed_projects", json!({})).await;
    assert!(closed.is_array(), "{closed}");
    // The device was seen.
    assert!(api.devices().list()[0].last_seen.is_some());
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_client_of_another_version_is_refused_and_told_which_one_to_update() {
    let (_dir, h, api, token) = served("remote-version").await;
    let (mut c, refused) = Client::hello(&api, "999.0.0", &token).await;
    match &refused {
        ToClient::Refused {
            code,
            error,
            server_version,
            update,
        } => {
            assert_eq!(code, codes::VERSION);
            assert_eq!(server_version.as_deref(), Some(VERSION));
            // The client is newer: « Mettre à jour le serveur ».
            assert_eq!(update.as_deref(), Some("server"));
            assert!(
                error.contains(VERSION) && error.contains("999.0.0"),
                "{error}"
            );
        }
        other => panic!("{other:?}"),
    }
    assert!(c.closed().await);
    let (mut c, refused) = Client::hello(&api, "0.0.1", &token).await;
    assert!(
        matches!(&refused, ToClient::Refused { update: Some(u), .. } if u == "client"),
        "{refused:?}"
    );
    assert!(c.closed().await);
    assert_eq!(api.clients(), 0);
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_token_missing_wrong_or_revoked_is_refused_and_a_revoked_device_is_put_out() {
    let (_dir, h, api, token) = served("remote-token").await;
    for presented in ["", "not-a-token", &token[..token.len() - 1]] {
        let (mut c, refused) = Client::hello(&api, VERSION, presented).await;
        assert_eq!(
            code_of(&refused),
            Some(codes::TOKEN),
            "{presented}: {refused:?}"
        );
        assert!(c.closed().await);
    }
    // A first message that is not the handshake.
    let mut ws = open(api.addr(), proto::PATH, None, None).await.unwrap();
    ws.send(Message::text(
        r#"{"t":"call","id":1,"cmd":"closed_projects"}"#,
    ))
    .await
    .unwrap();
    let first = ws.next().await.unwrap().unwrap();
    assert_eq!(code_of(&parse(&first).unwrap()), Some(codes::HANDSHAKE));

    // Revoked while connected: put out, then refused.
    let (device, second) = api.devices().add("Portable").unwrap();
    let mut c = Client::connect(&api, &second).await;
    let mut other = Client::connect(&api, &token).await;
    assert!(api.revoke(&device.id).unwrap());
    assert!(c.closed().await, "the revoked device stays connected");
    let (mut again, refused) = Client::hello(&api, VERSION, &second).await;
    assert_eq!(code_of(&refused), Some(codes::TOKEN));
    assert!(again.closed().await);
    // The other device is still there; a device's token is kept, its name in the file, never its token there.
    assert!(other.ok("closed_projects", json!({})).await.is_array());
    let file = std::fs::read_to_string(h.core.data.root().join(remoteapi::devices::FILE)).unwrap();
    assert!(
        file.contains("Téléphone") && !file.contains("Portable"),
        "{file}"
    );
    assert!(
        !file.contains(&token),
        "the token is in the keychain, not the file"
    );
    assert!(!api.revoke(&device.id).unwrap());
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_web_page_s_origin_a_host_that_is_not_the_loopback_and_another_path_are_refused_before_the_websocket(
) {
    let (_dir, h, api, token) = served("remote-origin").await;
    let addr = api.addr();
    assert_eq!(
        open(addr, proto::PATH, None, Some("https://evil.example"))
            .await
            .unwrap_err(),
        "HTTP 403"
    );
    assert_eq!(
        open(addr, proto::PATH, None, Some("http://localhost:5173"))
            .await
            .unwrap_err(),
        "HTTP 403"
    );
    assert_eq!(
        open(addr, proto::PATH, Some("evil.example"), None)
            .await
            .unwrap_err(),
        "HTTP 403"
    );
    assert_eq!(
        open(addr, "/mcp", None, None).await.unwrap_err(),
        "HTTP 404"
    );
    // Escouade's own window (fondation 2b), and a program of its own (no Origin), through a tunnel's port.
    for origin in ["http://tauri.localhost", "tauri://localhost"] {
        open(addr, proto::PATH, None, Some(origin)).await.unwrap();
    }
    open(addr, proto::PATH, Some("localhost:4999"), None)
        .await
        .unwrap();
    // A message past the bound closes the connection.
    let mut c = Client::connect(&api, &token).await;
    let big = "x".repeat(proto::MAX_MESSAGE + 1);
    let _ = c.ws.send(Message::text(big)).await;
    assert!(c.closed().await, "a message past the bound was read");
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn it_listens_on_127_0_0_1_alone() {
    let (_dir, h, api, _token) = served("remote-loopback").await;
    let addr = api.addr();
    assert_eq!(addr.ip(), std::net::Ipv4Addr::LOCALHOST);
    let port = addr.port();
    // Not on IPv6's loopback, nor on the address of the machine on its network.
    assert!(tokio::net::TcpStream::connect(("::1", port)).await.is_err());
    let outward = std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| s.connect("192.0.2.1:9").map(|()| s))
        .and_then(|s| s.local_addr())
        .map(|a| a.ip());
    match outward {
        Ok(ip) if !ip.is_loopback() && !ip.is_unspecified() => {
            let r = tokio::time::timeout(
                Duration::from_secs(5),
                tokio::net::TcpStream::connect((ip, port)),
            )
            .await;
            assert!(!matches!(r, Ok(Ok(_))), "reached on {ip}:{port}");
        }
        // No network on this machine: nothing else to try.
        _ => {}
    }
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn an_error_comes_back_as_the_ipc_gives_it_and_the_desktop_s_commands_are_refused_by_name() {
    let (_dir, h, api, token) = served("remote-errors").await;
    let mut c = Client::connect(&api, &token).await;
    // The command's own error: a text, no code.
    match c
        .call("rename_agent", json!({"id": "nope", "name": "x"}))
        .await
    {
        ToClient::Reply {
            ok: None,
            error: Some(Value::String(e)),
            code: None,
            ..
        } => assert!(!e.is_empty()),
        other => panic!("{other:?}"),
    }
    // An argument missing, as the IPC says it.
    match c.call("rename_agent", json!({"id": "nope"})).await {
        ToClient::Reply { error: Some(e), .. } => {
            assert_eq!(e, "command rename_agent missing required key name")
        }
        other => panic!("{other:?}"),
    }
    // The desktop's: refused by name, never run.
    let refused = c
        .call("open_url", json!({"url": "https://escouade.dev"}))
        .await;
    assert_eq!(code_of(&refused), Some(codes::DESK_ONLY), "{refused:?}");
    let refused = c.call("voice_start", json!({})).await;
    assert_eq!(code_of(&refused), Some(codes::DESK_ONLY));
    let refused = c.call("clipboard_image", json!({})).await;
    assert_eq!(code_of(&refused), Some(codes::NOT_SERVED_YET));
    let refused = c.call("no_such_command", json!({})).await;
    assert_eq!(code_of(&refused), Some(codes::UNKNOWN_COMMAND));
    // A message that is not a call: told, and the connection stays.
    c.ws.send(Message::text("{}")).await.unwrap();
    let told = c
        .until("the bad message", |m| {
            matches!(parse(m), Some(ToClient::Error { .. }))
        })
        .await;
    assert!(
        matches!(parse(&told), Some(ToClient::Error { code, .. }) if code == codes::BAD_MESSAGE)
    );
    assert!(c.ok("closed_projects", json!({})).await.is_array());
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_events_of_an_agent_led_to_the_end_of_a_turn_are_pushed_with_what_the_host_shows() {
    let (dir, h, api, token) = served("remote-events").await;
    let mut c = Client::connect(&api, &token).await;
    let initial = c.ok("subscribe", json!({"channel": "__CHANNEL__:1"})).await;
    assert_eq!(initial["version"], VERSION);
    let p = project(&mut c, &dir).await;
    let agent = c.ok("create_agent", json!({"projectId": p["id"]})).await;
    let id = agent["id"].as_str().unwrap().to_string();
    c.ok(
        "send_message",
        json!({"id": id, "text": "Bonjour", "attachments": []}),
    )
    .await;
    c.until("the agent done", |m| {
        channel_data(m, 1).is_some_and(|d| {
            d["type"] == "agent"
                && d["agent"]["id"] == id.as_str()
                && d["agent"]["status"] == "done"
        })
    })
    .await;
    c.until("the reply in the conversation", |m| {
        channel_data(m, 1).is_some_and(|d| {
            d["type"] == "conv" && d.to_string().contains("Bonjour, tu as dit : Bonjour")
        })
    })
    .await;
    // What the desktop would show, out of the host.
    let notify = c
        .until(
            "the notification",
            |m| matches!(parse(m), Some(ToClient::Host { event }) if event["type"] == "notify"),
        )
        .await;
    let Some(ToClient::Host { event }) = parse(&notify) else {
        unreachable!()
    };
    assert!(
        event["title"].as_str().unwrap().starts_with("demo · "),
        "{event}"
    );
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_terminal_s_output_comes_in_binary_on_its_channel() {
    let (dir, h, api, token) = served("remote-terminal").await;
    let mut c = Client::connect(&api, &token).await;
    let initial = c.ok("subscribe", json!({"channel": "__CHANNEL__:1"})).await;
    let shell = initial["shells"][0]["id"]
        .as_str()
        .expect("a shell")
        .to_string();
    let p = project(&mut c, &dir).await;
    let term = c
        .ok(
            "term_spawn",
            json!({
                "projectId": p["id"], "shell": shell, "name": "t", "cols": 100, "rows": 30,
                "output": "__CHANNEL__:7",
            }),
        )
        .await;
    let tid = term["id"].as_str().unwrap().to_string();
    // Its first bytes, in binary on its channel. A pseudo-console of Windows first asks where the cursor is (DSR), and
    // waits for the answer xterm.js gives in the window: the test gives it.
    let first = c
        .until("the terminal's first bytes", |m| {
            channel_bytes(m, 7).is_some()
        })
        .await;
    let first = channel_bytes(&first, 7).unwrap();
    assert!(!first.is_empty());
    if first.windows(4).any(|w| w == b"\x1b[6n") {
        c.ok("term_write", json!({"id": tid, "data": "\u{1b}[1;1R"}))
            .await;
    }
    c.ok(
        "term_write",
        json!({"id": tid, "data": "echo escouade-a-distance\r"}),
    )
    .await;
    // The bytes of the terminal, in binary frames of its channel, until the echo shows.
    let mut out = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    // Typed, then said back: twice.
    while String::from_utf8_lossy(&out)
        .matches("escouade-a-distance")
        .count()
        < 2
    {
        assert!(
            tokio::time::Instant::now() < deadline,
            "{}",
            String::from_utf8_lossy(&out)
        );
        let m = c
            .until("the terminal's output", |m| channel_bytes(m, 7).is_some())
            .await;
        out.extend(channel_bytes(&m, 7).unwrap());
    }
    c.ok("term_kill", json!({"id": tid})).await;
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn two_clients_have_each_their_state_of_the_interface_and_an_answer_of_one_closes_the_request_at_the_other(
) {
    let (dir, h, api, token) = served("remote-two").await;
    let (_, second) = api.devices().add("Portable").unwrap();
    let mut a = Client::connect(&api, &token).await;
    let mut b = Client::connect(&api, &second).await;
    a.ok("subscribe", json!({"channel": "__CHANNEL__:1"})).await;
    b.ok("subscribe", json!({"channel": "__CHANNEL__:1"})).await;
    assert_eq!(h.core.hub.subscribers(), 2);

    // Each its own state of the interface, the window's untouched.
    let window_before = h.core.ui.read().view.clone();
    a.ok(
        "set_ui",
        json!({"ui": {"view": "stats", "activeProject": "pa"}}),
    )
    .await;
    b.ok("set_ui", json!({"ui": {"view": "board"}})).await;
    let a_again = a.ok("subscribe", json!({"channel": "__CHANNEL__:2"})).await;
    let b_again = b.ok("subscribe", json!({"channel": "__CHANNEL__:2"})).await;
    assert_eq!(a_again["ui"]["view"], "stats");
    assert_eq!(a_again["ui"]["activeProject"], "pa");
    assert_eq!(b_again["ui"]["view"], "board");
    assert_eq!(b_again["ui"]["activeProject"], Value::Null);
    assert_eq!(h.core.ui.read().view, window_before);
    // Subscribing again took the place of the client's channel, not the other's.
    assert_eq!(h.core.hub.subscribers(), 2);

    // A permission asked: both see it; one answers, it closes at the other.
    let p = project(&mut a, &dir).await;
    let agent = a.ok("create_agent", json!({"projectId": p["id"]})).await;
    let id = agent["id"].as_str().unwrap().to_string();
    a.ok(
        "send_message",
        json!({"id": id, "text": "permission", "attachments": []}),
    )
    .await;
    let asked = |m: &Message| {
        channel_data(m, 2).is_some_and(|d| {
            d["type"] == "conv" && d["agentId"] == id.as_str() && permission_id(&d).is_some()
        })
    };
    let request = b.until("the request at the other client", asked).await;
    let request_id = permission_id(&channel_data(&request, 2).unwrap()).unwrap();
    a.until("the request at the client that answers", asked)
        .await;
    a.ok(
        "answer_permission",
        json!({"id": id, "requestId": request_id, "decision": "allow"}),
    )
    .await;
    b.until("the request closed at the other client", |m| {
        channel_data(m, 2).is_some_and(|d| {
            d["type"] == "conv"
                && d["agentId"] == id.as_str()
                && d.to_string().contains(r#""decision":""#)
        })
    })
    .await;

    // A client gone: its subscriber and its state with it.
    drop(b);
    wait("the client gone", || h.core.hub.subscribers() == 1).await;
    assert_eq!(h.core.client_ui.read().len(), 1);
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_policies_of_the_device_forbid_the_remote_api_at_its_start_and_at_each_connection() {
    let (_dir, h, api, token) = served("remote-policy").await;
    let mut c = Client::connect(&api, &token).await;
    h.core.set_managed(crate::locks::Managed {
        values: [(
            escouade_policy::list::REMOTE_API.to_string(),
            (Value::Bool(false), "test".to_string()),
        )]
        .into(),
        warnings: Vec::new(),
    });
    let (mut refused, told) = Client::hello(&api, VERSION, &token).await;
    assert_eq!(code_of(&told), Some(codes::POLICY), "{told:?}");
    assert!(refused.closed().await);
    let services = Arc::new(remoteapi::Services::new(h.core.clone()));
    let err = RemoteApi::start(services, 0).await.err().expect("refused");
    assert!(err.to_string().contains("servers.remoteApi"), "{err}");
    // The connection already open goes on: the policy is read at each new one.
    assert!(c.ok("closed_projects", json!({})).await.is_array());
    h.stop();
}

#[test]
fn the_older_side_is_the_one_to_update_and_a_binary_frame_reads_back() {
    assert_eq!(proto::to_update("2.3.0", "2.2.1"), "client");
    assert_eq!(proto::to_update("2.2.1", "2.3.0"), "server");
    assert_eq!(proto::to_update("2.3.0-beta.2", "2.3.0-beta.1"), "server");
    let frame = proto::frame(Frame::Channel, 7, b"abc");
    assert_eq!(
        proto::read_frame(&frame),
        Some((Frame::Channel, 7, &b"abc"[..]))
    );
    assert_eq!(proto::read_frame(&[1, 2]), None);
}

/// The id of the permission asked somewhere in `v` (an item of the conversation whose kind is `permission`).
fn permission_id(v: &Value) -> Option<String> {
    if v["kind"] == "permission" {
        return v["id"].as_str().map(str::to_string);
    }
    match v {
        Value::Object(m) => m.values().find_map(permission_id),
        Value::Array(a) => a.iter().find_map(permission_id),
        _ => None,
    }
}
