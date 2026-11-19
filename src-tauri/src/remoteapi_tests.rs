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
            session: None,
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
            body: None,
            headers: Default::default(),
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

/// The bytes of a binary message of the channel `channel` (a terminal's output, numbered or not).
fn channel_bytes(m: &Message, channel: u32) -> Option<Vec<u8>> {
    match m {
        Message::Binary(b) => match proto::read_frame(b)? {
            (Frame::Channel, id, bytes) if id == u64::from(channel) => Some(bytes.to_vec()),
            (Frame::Term, id, rest) if id == u64::from(channel) => {
                proto::read_term(rest).map(|(_, bytes)| bytes.to_vec())
            }
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
    assert_eq!(class("attach_chunk"), Some(Class::Moves { served: true }));
    assert_eq!(class("fs_write_bytes"), Some(Class::Served));
    assert_eq!(class("no_such_command"), None);
    // `docs/REMOTE.md` lists them all, in the same classes.
    let doc =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/REMOTE.md"))
            .unwrap();
    let errors = classes_as_docs_says(&doc);
    assert!(errors.is_empty(), "docs/REMOTE.md: {errors:#?}");
}

/// The title of a class's part in `docs/REMOTE.md`, « Le classement des commandes ».
fn class_title(class: Class) -> &'static str {
    match class {
        Class::Served => "Servies telles quelles",
        Class::Desk => "Du poste seulement",
        Class::Moves { served: true } => "Changent de machine, servies",
        Class::Moves { served: false } => "Changent de machine, refusées",
    }
}

/// What « Le classement des commandes » of `doc` says amiss of `list.rs`: one part per class (`#### <its title>`), one
/// bullet per command (`` - `name` ``, maybe with why), in alphabetical order, so that two tickets that each add a
/// command touch two lines. A command missing, one too many, in the wrong class or out of its place: each said.
fn classes_as_docs_says(doc: &str) -> Vec<String> {
    let mut errors = Vec::new();
    let Some(start) = doc.find("### Le classement des commandes") else {
        return vec!["no section « Le classement des commandes »".into()];
    };
    let section = &doc[start..];
    let end = ["\n## ", "\n### "]
        .iter()
        .filter_map(|h| section[4..].find(h))
        .min()
        .map_or(section.len(), |i| i + 4);
    let parts: Vec<(&str, Vec<&str>)> = section[..end]
        .split("\n#### ")
        .skip(1)
        .map(|p| {
            let (title, body) = p.split_once('\n').unwrap_or((p, ""));
            let names = body
                .lines()
                .filter_map(|l| l.strip_prefix("- `"))
                .filter_map(|l| l.split('`').next())
                .collect();
            (title.trim(), names)
        })
        .collect();
    let titles: Vec<&str> = parts.iter().map(|(t, _)| *t).collect();
    let wanted = [
        class_title(Class::Served),
        class_title(Class::Desk),
        class_title(Class::Moves { served: true }),
        class_title(Class::Moves { served: false }),
    ];
    if titles != wanted {
        errors.push(format!("the parts are {titles:?}, not {wanted:?}"));
    }
    for (part, names) in &parts {
        for pair in names.windows(2) {
            if pair[0] >= pair[1] {
                errors.push(format!(
                    "« {part} »: `{}` before `{}`: not in alphabetical order, or twice",
                    pair[0], pair[1]
                ));
            }
        }
        for name in names {
            match remoteapi::find(name) {
                None => errors.push(format!(
                    "« {part} »: `{name}` is no command of remoteapi/list.rs"
                )),
                Some(c) if class_title(c.class) != *part => errors.push(format!(
                    "`{name}`: in « {part} », of « {} » in remoteapi/list.rs",
                    class_title(c.class)
                )),
                Some(_) => {}
            }
        }
    }
    for c in list::COMMANDS {
        if !parts.iter().any(|(_, names)| names.contains(&c.name)) {
            errors.push(format!(
                "`{}`: missing from « {} »",
                c.name,
                class_title(c.class)
            ));
        }
    }
    errors
}

/// The check of « Le classement des commandes » fails on each way it can be wrong.
#[test]
fn the_classes_of_docs_remote_fail_when_a_command_is_missing_extra_misplaced_or_out_of_order() {
    let doc =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/REMOTE.md"))
            .unwrap();
    assert!(classes_as_docs_says(&doc).is_empty());
    let says = |doc: String, what: &str| {
        let errors = classes_as_docs_says(&doc);
        assert!(
            errors.iter().any(|e| e.contains(what)),
            "wanted « {what} » in {errors:#?}"
        );
    };
    // Missing.
    says(
        doc.replace("- `create_agent`\n", ""),
        "`create_agent`: missing",
    );
    // One too many.
    says(
        doc.replace(
            "- `create_agent`\n",
            "- `create_agent`\n- `create_agentz`\n",
        ),
        "`create_agentz` is no command",
    );
    // In the wrong class: a command of the desk among the served ones.
    let misplaced = doc.replace("- `data_dir`\n", "- `data_dir`\n- `deep_links_take`\n");
    says(
        misplaced,
        "`deep_links_take`: in « Servies telles quelles »",
    );
    // Out of its place.
    says(
        doc.replace("- `create_agent`\n", "")
            .replace("- `data_dir`\n", "- `data_dir`\n- `create_agent`\n"),
        "before `create_agent`",
    );
}

// ---------- the WebSocket, against the core without a window ----------

#[tokio::test(flavor = "multi_thread")]
async fn the_handshake_passes_with_a_device_token_and_the_server_s_version_and_a_command_answers() {
    let (dir, h, api, token) = served("remote-handshake").await;
    let (mut c, welcome) = Client::hello(&api, VERSION, &token).await;
    let ToClient::Welcome { session, desk, .. } = &welcome else {
        panic!("{welcome:?}")
    };
    assert_eq!(session.len(), 32, "{session}");
    // The desktop's commands, for a client that has a desktop to do them.
    assert!(desk.iter().any(|c| c == "open_url") && !desk.iter().any(|c| c == "term_spawn"));
    assert_eq!(
        welcome,
        ToClient::Welcome {
            version: VERSION.into(),
            client: "c1".into(),
            device: "Téléphone".into(),
            session: session.clone(),
            resumed: false,
            desk: desk.clone(),
            // A core whose host keeps no file of secrets (the tests' memory store): the keychain's, as the desktop.
            secrets_in_file: false,
            // The machine it runs on (ESC-537).
            machine: Some(crate::remoteapi::this_machine()),
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
            server_update,
            ..
        } => {
            assert_eq!(code, codes::VERSION);
            assert_eq!(server_version.as_deref(), Some(VERSION));
            // The client is newer: « Mettre à jour le serveur », and how (ESC-515).
            assert_eq!(update.as_deref(), Some("server"));
            let how = server_update.as_ref().expect("how to update the server");
            assert_eq!(how.command, "escouade-server update --to 999.0.0");
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
        matches!(&refused, ToClient::Refused { update: Some(u), server_update: None, .. } if u == "client"),
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

// ---------- fondation 2b (ESC-495): the resume, the bounded queue, the bounds, the raw bodies ----------

/// A core without a window whose remote API keeps to `limits`.
async fn served_within(
    name: &str,
    limits: crate::remoteapi::server::Limits,
) -> (PathBuf, Headless, Arc<RemoteApi>, String) {
    let (dir, data) = data(name);
    let h = Headless::start(data).unwrap();
    let api = h.serve_remote_with(0, limits).await.unwrap();
    let (_, token) = api.devices().add("Téléphone").unwrap();
    (dir, h, api, token)
}

impl Client {
    /// Connected with `token`, giving back `session` (a session of before a cut): its `Welcome`.
    async fn hello_in(api: &RemoteApi, token: &str, session: Option<&str>) -> (Self, ToClient) {
        let ws = open(api.addr(), proto::PATH, None, None).await.unwrap();
        let mut c = Client {
            ws,
            next: 1_000,
            seen: Vec::new(),
        };
        c.send(&ToServer::Hello {
            version: VERSION.into(),
            token: token.into(),
            session: session.map(str::to_string),
        })
        .await;
        let first = c.recv().await.expect("an answer to the handshake");
        (c, parse(&first).expect("a message of the protocol"))
    }

    /// The command `cmd` called with `args` and a raw body, sent before it in parts, with `headers`.
    async fn call_body(
        &mut self,
        cmd: &str,
        args: Value,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> ToClient {
        let id = self.next;
        self.next += 1;
        for part in body.chunks(proto::MAX_MESSAGE - proto::FRAME_HEAD - 1) {
            self.ws
                .send(Message::binary(proto::frame(Frame::Body, id, part)))
                .await
                .unwrap();
        }
        let Value::Object(args) = args else {
            panic!("args are an object")
        };
        let headers = headers
            .iter()
            .map(|(k, v)| (k.to_string(), Value::String(v.to_string())))
            .collect();
        self.send(&ToServer::Call {
            id,
            cmd: cmd.into(),
            args,
            body: Some(body.len() as u64),
            headers,
        })
        .await;
        self.reply_to(id, cmd).await
    }

    async fn reply_to(&mut self, id: u64, cmd: &str) -> ToClient {
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

    /// The answer in bytes to `cmd`, its parts put together, and how many frames it came in.
    async fn call_bytes(&mut self, cmd: &str, args: Value) -> (Vec<u8>, usize) {
        let id = self.next;
        self.next += 1;
        let Value::Object(args) = args else {
            panic!("args are an object")
        };
        self.send(&ToServer::Call {
            id,
            cmd: cmd.into(),
            args,
            body: None,
            headers: Default::default(),
        })
        .await;
        let mut bytes = Vec::new();
        let mut frames = 0;
        loop {
            let m = self
                .recv()
                .await
                .unwrap_or_else(|| panic!("closed before the answer to {cmd}"));
            if let Message::Binary(b) = &m {
                match proto::read_frame(b) {
                    Some((Frame::ReplyPart, got, part)) if got == id => {
                        assert!(b.len() <= proto::MAX_MESSAGE);
                        bytes.extend_from_slice(part);
                        frames += 1;
                        continue;
                    }
                    Some((Frame::Reply, got, part)) if got == id => {
                        assert!(b.len() <= proto::MAX_MESSAGE);
                        bytes.extend_from_slice(part);
                        return (bytes, frames + 1);
                    }
                    _ => {}
                }
            }
            if let Some(ToClient::Reply { id: got, .. }) = parse(&m) {
                assert_ne!(got, id, "{cmd} answered in JSON: {m:?}");
            }
            self.seen.push(m);
        }
    }
}

/// A terminal as the window's transport shows it (`src/lib/transport.ts`): the end of the last numbered byte, what
/// it shows, whether a resume's replay is under way.
#[derive(Default)]
struct Screen {
    last: Option<u64>,
    shown: Vec<u8>,
    resuming: bool,
    gaps: u32,
    lost: u64,
}

impl Screen {
    /// `m`, if it is of the channel `channel`. Its gap to resume, when it is one.
    fn take(&mut self, m: &Message, channel: u32) -> Option<bool> {
        match m {
            Message::Binary(b) => match proto::read_frame(b)? {
                (Frame::Term, id, rest) if id == u64::from(channel) => {
                    let (end, data) = proto::read_term(rest)?;
                    let start = end - data.len() as u64;
                    let last = *self.last.get_or_insert(start);
                    if end <= last {
                        return Some(false);
                    }
                    if start > last {
                        // A hole: a resume makes up for it.
                        let ask = !self.resuming;
                        self.resuming = true;
                        self.gaps += u32::from(ask);
                        return Some(ask);
                    }
                    self.shown
                        .extend_from_slice(&data[(last - start) as usize..]);
                    self.last = Some(end);
                    Some(false)
                }
                (Frame::Channel, id, data) if id == u64::from(channel) => {
                    self.shown.extend_from_slice(data);
                    Some(false)
                }
                _ => None,
            },
            m => match parse(m)? {
                ToClient::Mark { channel: c, end } if c == channel => {
                    self.last = Some(end);
                    Some(false)
                }
                ToClient::Lost { channel: c, bytes } if c == channel => {
                    self.lost += bytes;
                    self.last = self.last.map(|l| l + bytes);
                    Some(false)
                }
                ToClient::Resumed { channel: c, .. } if c == channel => {
                    self.resuming = false;
                    Some(false)
                }
                ToClient::Gap { channel: c } if c == channel => {
                    let ask = !self.resuming;
                    self.resuming = true;
                    Some(ask)
                }
                _ => None,
            },
        }
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.shown).into_owned()
    }
}

/// What `c` receives of the terminal of `channel`, into `screen`, until `done` says so (90 s at most); a hole is
/// resumed at once.
async fn pump(c: &mut Client, screen: &mut Screen, channel: u32, done: impl Fn(&Screen) -> bool) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let mut kept = std::mem::take(&mut c.seen).into_iter();
    while !done(screen) {
        let m = match kept.next() {
            Some(m) => m,
            None => {
                let left = deadline.saturating_duration_since(tokio::time::Instant::now());
                match tokio::time::timeout(left, c.ws.next()).await {
                    Ok(Some(Ok(m))) => m,
                    other => panic!("{other:?} waiting for the terminal: {}", screen.text()),
                }
            }
        };
        if screen.take(&m, channel) == Some(true) {
            c.send(&ToServer::Resume {
                channels: vec![proto::Resume {
                    channel,
                    after: screen.last,
                }],
            })
            .await;
        }
    }
    c.seen.extend(kept);
}

/// The lines `L0001.` to `L<n>.` a terminal is to print, a few milliseconds apart, in its shell `shell`.
fn numbered_lines(shell: &str, n: u32) -> String {
    match shell {
        "pwsh" | "powershell" => {
            format!("1..{n} | % {{ 'L{{0:d4}}.' -f $_; Start-Sleep -Milliseconds 4 }}\r")
        }
        "fish" => format!("for i in (seq 1 {n}); printf 'L%04d.\\n' $i; sleep 0.004; end\r"),
        _ => format!("for i in $(seq 1 {n}); do printf 'L%04d.\\n' $i; sleep 0.004; done\r"),
    }
}

/// Each line `L0001.` to `L<n>.` once, in order, in `text`.
fn each_line_once(text: &str, n: u32) -> std::result::Result<(), String> {
    let mut at = 0;
    for i in 1..=n {
        let line = format!("L{i:04}.");
        let count = text.matches(&line).count();
        if count != 1 {
            return Err(format!("{line} {count} times"));
        }
        let place = text.find(&line).unwrap();
        if place < at {
            return Err(format!("{line} out of order"));
        }
        at = place;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_terminal_cut_while_it_writes_resumes_with_no_byte_missing_or_twice() {
    let (dir, h, api, token) = served("remote-resume").await;
    let (mut c, welcome) = Client::hello_in(&api, &token, None).await;
    let ToClient::Welcome { session, .. } = welcome else {
        panic!("{welcome:?}")
    };
    let initial = c.ok("subscribe", json!({"channel": "__CHANNEL__:1"})).await;
    let shell = initial["shells"][0]["id"].as_str().unwrap().to_string();
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
    let mut screen = Screen::default();
    pump(&mut c, &mut screen, 7, |s| !s.shown.is_empty()).await;
    if screen.shown.windows(4).any(|w| w == b"\x1b[6n") {
        c.ok("term_write", json!({"id": tid, "data": "\u{1b}[1;1R"}))
            .await;
    }
    const N: u32 = 600;
    c.ok(
        "term_write",
        json!({"id": tid, "data": numbered_lines(&shell, N)}),
    )
    .await;
    // Some lines shown, then the connection cut while the terminal writes on.
    pump(&mut c, &mut screen, 7, |s| s.text().contains("L0040.")).await;
    drop(c);
    tokio::time::sleep(Duration::from_millis(600)).await;
    // Back in its session: it resumes the terminal from the end of the last bytes it received.
    let (mut c, welcome) = Client::hello_in(&api, &token, Some(&session)).await;
    assert!(
        matches!(&welcome, ToClient::Welcome { session: s, resumed: true, .. } if *s == session),
        "{welcome:?}"
    );
    c.send(&ToServer::Resume {
        channels: vec![proto::Resume {
            channel: 7,
            after: screen.last,
        }],
    })
    .await;
    screen.resuming = true;
    let last_line = format!("L{N:04}.");
    pump(&mut c, &mut screen, 7, |s| {
        !s.resuming && s.text().contains(&last_line)
    })
    .await;
    let text = screen.text();
    each_line_once(&text, N).unwrap_or_else(|e| panic!("{e}: {text}"));
    assert_eq!(screen.lost, 0);
    c.ok("term_kill", json!({"id": tid})).await;
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_session_is_its_device_s_alone_and_a_channel_it_does_not_hold_is_gone() {
    let (_dir, h, api, token) = served("remote-session-device").await;
    let (c, welcome) = Client::hello_in(&api, &token, None).await;
    let ToClient::Welcome { session, .. } = welcome else {
        panic!("{welcome:?}")
    };
    drop(c);
    // Another device that gives this session's id gets a session of its own.
    let (_, other) = api.devices().add("Portable").unwrap();
    let (mut o, welcome) = Client::hello_in(&api, &other, Some(&session)).await;
    assert!(
        matches!(&welcome, ToClient::Welcome { session: s, resumed: false, .. } if *s != session),
        "{welcome:?}"
    );
    // And resumes nothing: the channel is not one of its own.
    o.send(&ToServer::Resume {
        channels: vec![proto::Resume {
            channel: 7,
            after: Some(0),
        }],
    })
    .await;
    let m = o
        .until("the resume's answer", |m| {
            matches!(parse(m), Some(ToClient::Resumed { .. }))
        })
        .await;
    assert_eq!(
        parse(&m),
        Some(ToClient::Resumed {
            channel: 7,
            ended: false,
            gone: true
        })
    );
    // A session id too long, or made up: a new session.
    let long = "x".repeat(proto::MAX_SESSION + 1);
    let (_c, welcome) = Client::hello_in(&api, &token, Some(&long)).await;
    assert!(matches!(welcome, ToClient::Welcome { resumed: false, .. }));
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_client_that_reads_nothing_is_closed_and_the_core_keeps_little_for_it() {
    let limits = crate::remoteapi::server::Limits {
        queue: crate::remoteapi::outbox::Bounds {
            messages: 64,
            bytes: 256 << 10,
            hard_bytes: 4 << 20,
            stall: Duration::from_secs(2),
        },
        ..Default::default()
    };
    let (_dir, h, api, token) = served_within("remote-muted", limits).await;
    let mut c = Client::connect(&api, &token).await;
    c.ok("subscribe", json!({"channel": "__CHANNEL__:1"})).await;
    assert_eq!(api.clients(), 1);
    // The client reads nothing from now on, while the core pushes 64 MB of events at it.
    let text = "x".repeat(4 << 10);
    for _ in 0..(16 << 10) {
        h.core
            .hub
            .emit(crate::model::UiEvent::Toast { text: text.clone() });
    }
    // Closed once it read nothing for 2 s.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    while api.clients() > 0 {
        assert!(tokio::time::Instant::now() < deadline, "still connected");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let counts = api.counts();
    assert_eq!(
        counts
            .closed_slow
            .load(std::sync::atomic::Ordering::Relaxed),
        1
    );
    // What waited for it never went past the soft bound by more than a message: the events melted into a re-read.
    let peak = counts
        .peak_queued
        .load(std::sync::atomic::Ordering::Relaxed);
    assert!(peak <= (256 << 10) + (8 << 10), "{peak} bytes waited");
    drop(c);
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_client_that_lets_answers_pile_up_unread_is_closed_past_the_hard_bound() {
    let limits = crate::remoteapi::server::Limits {
        queue: crate::remoteapi::outbox::Bounds {
            messages: 64,
            bytes: 256 << 10,
            hard_bytes: 3 << 20,
            stall: Duration::from_secs(60),
        },
        ..Default::default()
    };
    let (dir, h, api, token) = served_within("remote-hard", limits).await;
    let mut c = Client::connect(&api, &token).await;
    let p = project(&mut c, &dir).await;
    let r = PathBuf::from(p["path"].as_str().unwrap());
    std::fs::write(r.join("big.bin"), vec![7u8; 2 << 20]).unwrap();
    // Asked again and again, never read: past 3 MB waiting, it is closed.
    for id in 0..64u64 {
        c.send(&ToServer::Call {
            id: 50_000 + id,
            cmd: "fs_read_bytes".into(),
            args: json!({"projectId": p["id"], "path": "big.bin"})
                .as_object()
                .unwrap()
                .clone(),
            body: None,
            headers: Default::default(),
        })
        .await;
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    while api.clients() > 0 {
        assert!(tokio::time::Instant::now() < deadline, "still connected");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let peak = api
        .counts()
        .peak_queued
        .load(std::sync::atomic::Ordering::Relaxed);
    assert!(peak <= 3 << 20, "{peak} bytes waited");
    drop(c);
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_connections_of_a_device_and_in_all_are_bounded() {
    let limits = crate::remoteapi::server::Limits {
        per_device: 2,
        total: 3,
        ..Default::default()
    };
    let (_dir, h, api, token) = served_within("remote-bounds", limits).await;
    let _a = Client::connect(&api, &token).await;
    let _b = Client::connect(&api, &token).await;
    let (mut third, refused) = Client::hello(&api, VERSION, &token).await;
    assert_eq!(code_of(&refused), Some(codes::TOO_MANY), "{refused:?}");
    assert!(third.closed().await);
    // Another device has room, until all are taken.
    let (_, other) = api.devices().add("Portable").unwrap();
    let _c = Client::connect(&api, &other).await;
    let (_d, refused) = Client::hello(&api, VERSION, &other).await;
    assert_eq!(code_of(&refused), Some(codes::TOO_MANY), "{refused:?}");
    assert_eq!(api.clients(), 3);
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn failed_handshakes_turn_the_next_connections_away_for_a_while() {
    let limits = crate::remoteapi::server::Limits {
        failures_per_minute: 3,
        ..Default::default()
    };
    let (_dir, h, api, token) = served_within("remote-failures", limits).await;
    for _ in 0..3 {
        let (mut c, refused) = Client::hello(&api, VERSION, "not-a-token").await;
        assert_eq!(code_of(&refused), Some(codes::TOKEN));
        assert!(c.closed().await);
    }
    // Now even the right token finds the door shut: closed as soon as accepted, before the WebSocket.
    let opened = open(api.addr(), proto::PATH, None, None).await;
    assert!(opened.is_err(), "the WebSocket opened");
    assert!(
        api.counts()
            .turned_away
            .load(std::sync::atomic::Ordering::Relaxed)
            >= 1
    );
    let _ = token;
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn too_many_calls_waiting_are_refused_as_busy() {
    let limits = crate::remoteapi::server::Limits {
        in_flight: 0,
        ..Default::default()
    };
    let (_dir, h, api, token) = served_within("remote-busy", limits).await;
    let mut c = Client::connect(&api, &token).await;
    let reply = c.call("closed_projects", json!({})).await;
    assert_eq!(code_of(&reply), Some(codes::BUSY), "{reply:?}");
    h.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_raw_body_sent_in_parts_writes_a_file_and_a_large_answer_comes_in_parts() {
    let (dir, h, api, token) = served("remote-body").await;
    let mut c = Client::connect(&api, &token).await;
    let p = project(&mut c, &dir).await;
    // 20 MB: past one message, in two parts each way.
    let bytes: Vec<u8> = (0..(20u32 << 20)).map(|i| (i % 253) as u8).collect();
    let pid = p["id"].as_str().unwrap().to_string();
    let reply = c
        .call_body(
            "fs_write_bytes",
            json!({}),
            &[
                ("x-project", &pid),
                ("x-agent", ""),
                ("x-path", "big.bin"),
                ("x-expect", "absent"),
            ],
            &bytes,
        )
        .await;
    assert!(
        matches!(&reply, ToClient::Reply { ok: Some(stamp), error: None, .. } if stamp["size"] == bytes.len()),
        "{reply:?}"
    );
    assert_eq!(
        std::fs::read(PathBuf::from(p["path"].as_str().unwrap()).join("big.bin")).unwrap(),
        bytes
    );
    let (back, frames) = c
        .call_bytes(
            "fs_read_bytes",
            json!({"projectId": pid, "path": "big.bin"}),
        )
        .await;
    assert_eq!(frames, 2);
    assert!(back == bytes, "the bytes read back differ");
    // A body whose length is not the one its call says: refused, nothing written.
    let id = c.next;
    c.next += 1;
    c.ws.send(Message::binary(proto::frame(Frame::Body, id, b"abc")))
        .await
        .unwrap();
    c.send(&ToServer::Call {
        id,
        cmd: "fs_write_bytes".into(),
        args: Default::default(),
        body: Some(4),
        headers: Default::default(),
    })
    .await;
    let reply = c.reply_to(id, "fs_write_bytes").await;
    assert_eq!(code_of(&reply), Some(codes::TOO_LARGE), "{reply:?}");
    // A file joined, in raw chunks.
    let handle = c
        .ok(
            "attach_open",
            json!({"owner": "draft", "name": "note.txt", "size": 5}),
        )
        .await;
    let handle = handle.as_str().unwrap().to_string();
    let reply = c
        .call_body(
            "attach_chunk",
            json!({}),
            &[("x-handle", &handle)],
            b"hello",
        )
        .await;
    assert!(
        matches!(reply, ToClient::Reply { error: None, .. }),
        "{reply:?}"
    );
    let file = c.ok("attach_finish", json!({"handle": handle})).await;
    let path = file["path"].as_str().expect("its path");
    assert_eq!(std::fs::read(path).unwrap(), b"hello");
    h.stop();
}

// ---------- the box « À toi » at a distance (ESC-509, ESC-520) ----------

/// A remote client — the phone, whose first screen the box is — is told the box changed, lists its requests, reads one
/// whole and its file in raw bytes (binary frames, never base64), and answers with a file sent in raw chunks; opening a
/// file is the core's machine's, refused for now (`not-served-yet`).
#[tokio::test(flavor = "multi_thread")]
async fn a_remote_client_gets_the_requests_of_the_box_their_files_in_bytes_and_answers_them() {
    use crate::asks::{AskAuthor, AskDraft, AskKind, Incoming, Source};
    let (dir, h, api, token) = served("remote-inbox").await;
    h.core.settings.write().mcp_enabled = true;
    let mut c = Client::connect(&api, &token).await;
    c.ok("subscribe", json!({"channel": "__CHANNEL__:1"})).await;
    let p = project(&mut c, &dir).await;
    let pid = p["id"].as_str().unwrap().to_string();
    // A PDF of 5 MB: more than one part of `inbox_file`, one message of the API.
    let mut pdf = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    pdf.extend((0..(5u32 << 20)).map(|i| (i % 251) as u8));
    let ask = h
        .core
        .ask_deposit(
            AskAuthor {
                agent: None,
                name: "Claude · client MCP".into(),
                chef: false,
            },
            &pid,
            None,
            AskDraft {
                kind: AskKind::Confirm,
                title: "Je publie la note de version ?".into(),
                wake: true,
                ..Default::default()
            },
            vec![Incoming {
                name: "note.pdf".into(),
                declared: None,
                option: None,
                source: Source::Bytes(pdf.clone()),
            }],
        )
        .unwrap();
    c.until("the box changed", |m| {
        channel_data(m, 1).is_some_and(|d| d["type"] == "inbox" && d["projectId"] == pid.as_str())
    })
    .await;
    let view = c.ok("inbox_list", json!({})).await;
    assert_eq!(view["enabled"], true);
    assert_eq!(view["open"], 1);
    assert_eq!(view["asks"][0]["id"], ask.id.as_str());
    let whole = c.ok("inbox_get", json!({"id": ask.id})).await;
    assert_eq!(whole["title"], "Je publie la note de version ?");
    let file = whole["files"][0]["file"].as_str().unwrap().to_string();
    let (bytes, frames) = c
        .call_bytes("inbox_file_bytes", json!({"id": ask.id, "file": file}))
        .await;
    assert_eq!(frames, 1);
    assert!(bytes == pdf, "the bytes read back differ");
    // Its answer, with a file of the user's sent in raw chunks.
    let handle = c
        .ok(
            "attach_open",
            json!({"owner": "draft", "name": "relu.txt", "size": 5}),
        )
        .await;
    let handle = handle.as_str().unwrap().to_string();
    let reply = c
        .call_body(
            "attach_chunk",
            json!({}),
            &[("x-handle", &handle)],
            b"relu.",
        )
        .await;
    assert!(
        matches!(reply, ToClient::Reply { error: None, .. }),
        "{reply:?}"
    );
    let joined = c.ok("attach_finish", json!({"handle": handle})).await;
    let answered = c
        .ok(
            "inbox_answer",
            json!({"id": ask.id, "answer": {"yes": true}, "files": [joined]}),
        )
        .await;
    assert_eq!(answered["state"], "answered");
    assert_eq!(c.ok("inbox_list", json!({})).await["open"], 0);
    // Opened on the core's machine: not at a distance yet.
    let id = c.next;
    c.next += 1;
    c.send(&ToServer::Call {
        id,
        cmd: "inbox_open_file".into(),
        args: json!({"id": ask.id, "file": file, "reveal": false})
            .as_object()
            .unwrap()
            .clone(),
        body: None,
        headers: Default::default(),
    })
    .await;
    let reply = c.reply_to(id, "inbox_open_file").await;
    assert_eq!(code_of(&reply), Some(codes::NOT_SERVED_YET), "{reply:?}");
    h.stop();
}

// ---------- the bounded queue and the sessions, alone ----------

#[test]
fn the_events_of_a_slow_client_melt_into_a_re_read_and_a_terminal_s_dropped_output_is_said() {
    use crate::remoteapi::outbox::{Bounds, Lane, Outbox};
    let out = Outbox::new(Bounds {
        messages: 2,
        bytes: 1 << 20,
        hard_bytes: 2 << 20,
        stall: Duration::from_secs(60),
    });
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let text = |m: &Message| match m {
        Message::Text(t) => t.to_string(),
        Message::Binary(b) => format!("bin {}", b.len()),
        _ => String::new(),
    };
    out.push(Message::text("answer"), Lane::Control);
    for i in 0..10 {
        out.push(Message::text(format!("state {i}")), Lane::State);
    }
    out.push(Message::binary(vec![0; 10]), Lane::Term(7));
    // The answer is never dropped; the events melted into one re-read; the terminal's output dropped, and said.
    let mut sent = Vec::new();
    while out.waiting() > 0 {
        sent.push(text(&rt.block_on(out.next()).unwrap()));
    }
    assert_eq!(
        sent,
        ["answer", r#"{"t":"resync"}"#, r#"{"t":"gap","channel":7}"#]
    );
    assert!(out.dropped() >= 11, "{}", out.dropped());
    // Room again: the state is queued as it comes.
    out.push(Message::text("state 11"), Lane::State);
    assert_eq!(text(&rt.block_on(out.next()).unwrap()), "state 11");
    // Nothing read while something waits: stuck once its time is past; past the hard bound at once.
    out.push(Message::text("waiting"), Lane::Control);
    assert_eq!(out.stuck(std::time::Instant::now()), None);
    assert!(out
        .stuck(std::time::Instant::now() + Duration::from_secs(61))
        .is_some());
    out.push(Message::binary(vec![0; 3 << 20]), Lane::Control);
    assert_eq!(
        out.stuck(std::time::Instant::now()),
        Some("too much left unread")
    );
    assert!(rt.block_on(out.next()).is_none());
}

#[test]
fn a_session_keeps_its_terminals_within_its_bounds_and_is_forgotten_in_time() {
    use crate::remoteapi::sessions::{self, Sessions};
    use crate::termlog::{Replay, TermLog};
    let s = Sessions::default();
    let now = std::time::Instant::now();
    let (id, again) = s.open("phone", None, now);
    assert!(!again);
    for channel in 0..(sessions::MAX_CHANNELS as u32 + 10) {
        let log: Arc<dyn Replay> = TermLog::with_size(16, Box::new(|_: Vec<u8>| {}));
        s.bind(&id, channel, log);
    }
    assert_eq!(s.channels(&id), sessions::MAX_CHANNELS);
    assert!(s.find(&id, "phone", 0).is_none(), "the oldest let go");
    assert!(s.find(&id, "phone", 265).is_some());
    assert!(s.find(&id, "laptop", 265).is_none(), "another device's");
    s.leave(&id, now);
    // Taken again in time; a new one past it.
    assert_eq!(s.open("phone", Some(&id), now), (id.clone(), true));
    s.leave(&id, now);
    let later = now + sessions::KEPT + Duration::from_secs(1);
    let (fresh, again) = s.open("phone", Some(&id), later);
    assert!(!again && fresh != id);
    // A device's sessions bounded: the oldest without a connection let go.
    for _ in 0..(sessions::MAX_PER_DEVICE * 2) {
        let (x, _) = s.open("tablet", None, later);
        s.leave(&x, later);
    }
    assert!(s.len() <= sessions::MAX_PER_DEVICE + 1, "{}", s.len());
    s.forget_device("phone");
    assert!(s.find(&fresh, "phone", 0).is_none());
}
