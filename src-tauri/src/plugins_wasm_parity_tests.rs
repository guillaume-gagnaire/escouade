//! The parity of the modules (ESC-330, ESC-331, ESC-334, ESC-344) through the host of the app: played by `wasm::Instance`
//! and `wasm::serve`, Linear, Trello, Jira and ClickUp answer as their native plugins did, which their port took out of
//! `escouade-plugins` (the harnesses: `plugins_harness_parity_tests.rs`): `tests/fixtures/plugins/parity/` holds
//! what each answered and asked its service, on the same fake, at d3c0024c. `ESCOUADE_PARITY_RECORD=1` and
//! `ESCOUADE_NATIVE_PLUGINS=<a program of then>` write them again.
//!
//! Their modules come from `sh scripts/plugins-wasm.sh`, beside `cargo build --bins`. Without them, each test says so and
//! passes; with `ESCOUADE_PARITY=1` it fails.

use crate::integrations::fake::FakeServer;
use crate::plugins::wasm::{self, Fetcher, Frame, Instance, Net, Wiring};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

pub(crate) fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// `target`, as the tests run in `target/debug/deps`, wherever it is.
fn target() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    exe.ancestors().nth(3).unwrap().to_path_buf()
}

/// A module of `scripts/plugins-wasm.sh`, compiled by the app's engine.
pub(crate) fn module(name: &str) -> Option<wasmtime::Module> {
    let path = target()
        .join("plugins-wasm")
        .join(name)
        .join(wasm::MODULE_FILE);
    match std::fs::read(&path) {
        Ok(bytes) => Some(wasm::reactor(&bytes).expect("a reactor")),
        Err(e) => {
            assert!(
                std::env::var_os("ESCOUADE_PARITY").is_none(),
                "{}: {e}",
                path.display()
            );
            eprintln!("skipped: {}: {e}", path.display());
            None
        }
    }
}

/// How the core speaks to a plugin, module or process: lines.
pub(crate) trait Plugin {
    fn send(&mut self, line: &str);
    fn recv(&mut self, within: Duration) -> Option<String>;
}

/// A plugin in the host of the app, wired as `spawn_wasm` wires it.
pub(crate) struct Hosted {
    tx: UnboundedSender<Frame>,
    lines: Receiver<String>,
}

impl Hosted {
    pub(crate) fn start(module: wasmtime::Module, fetcher: Fetcher) -> Self {
        Self::run(module, fetcher, wasm::Limits::default(), None)
    }

    /// `start`, under `limits`.
    pub(crate) fn within(module: wasmtime::Module, fetcher: Fetcher, limits: wasm::Limits) -> Self {
        Self::run(module, fetcher, limits, None)
    }

    /// With what `escouade.call` serves.
    fn start_with(module: wasmtime::Module, fetcher: Fetcher, calls: Option<wasm::Calls>) -> Self {
        Self::run(module, fetcher, wasm::Limits::default(), calls)
    }

    fn run(
        module: wasmtime::Module,
        fetcher: Fetcher,
        limits: wasm::Limits,
        calls: Option<wasm::Calls>,
    ) -> Self {
        let (tx, rx) = unbounded_channel::<Frame>();
        let back = tx.downgrade();
        let (out, lines) = channel();
        std::thread::spawn(move || {
            let instance = Instance::start_with(&module, &BTreeMap::new(), limits, calls)
                .expect("an instance");
            let fault = wasm::serve(
                instance,
                fetcher,
                Wiring {
                    rx,
                    back,
                    line: Box::new(move |l| {
                        let _ = out.send(l.to_string());
                    }),
                    stderr: Box::new(|_| {}),
                    refused: Arc::new(|_| {}),
                    said: Arc::new(|_| {}),
                    capabilities: None,
                },
            );
            assert_eq!(fault, None);
        });
        Self { tx, lines }
    }
}

impl Plugin for Hosted {
    fn send(&mut self, line: &str) {
        let _ = self.tx.send((format!("{line}\n").into_bytes(), None));
    }
    fn recv(&mut self, within: Duration) -> Option<String> {
        self.lines.recv_timeout(within).ok()
    }
}

/// A native plugin: `escouade-plugins <name>`, its lines on its pipes.
pub(crate) struct Native {
    child: Child,
    input: ChildStdin,
    lines: Receiver<String>,
}

impl Native {
    pub(crate) fn start(name: &str) -> Self {
        let exe = std::env::current_exe().unwrap();
        let dir = exe.parent().unwrap().parent().unwrap();
        let program = std::env::var_os("ESCOUADE_NATIVE_PLUGINS")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dir.join(format!("escouade-plugins{}", std::env::consts::EXE_SUFFIX))
            });
        let mut child = Command::new(&program)
            .env("ESCOUADE_VERSION", "2.2.0")
            .arg(name)
            .env("ESCOUADE_VERSION", "2.2.0")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("{}: {e}: cargo build --bins", program.display()));
        let input = child.stdin.take().unwrap();
        let output = child.stdout.take().unwrap();
        let (tx, lines) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    return;
                }
            }
        });
        Self {
            child,
            input,
            lines,
        }
    }
}

impl Plugin for Native {
    fn send(&mut self, line: &str) {
        let _ = self.input.write_all(format!("{line}\n").as_bytes());
        let _ = self.input.flush();
    }
    fn recv(&mut self, within: Duration) -> Option<String> {
        self.lines.recv_timeout(within).ok()
    }
}

impl Drop for Native {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A call and its answer, the requests it makes meanwhile served as the core serves them (`secrets/get`).
pub(crate) fn call(p: &mut dyn Plugin, id: u64, method: &str, params: Value) -> Value {
    call_with(p, id, method, params, &json!("lin_api_fake"))
}

/// `call`, `secrets/get` answering `secret`.
pub(crate) fn call_with(
    p: &mut dyn Plugin,
    id: u64,
    method: &str,
    params: Value,
    secret: &Value,
) -> Value {
    p.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string());
    loop {
        let line = p.recv(Duration::from_secs(20)).expect("an answer");
        let v: Value = serde_json::from_str(&line).expect("JSON");
        if v["method"] == "secrets/get" {
            p.send(&json!({ "jsonrpc": "2.0", "id": v["id"], "result": secret }).to_string());
        } else if v["id"] == json!(id) && v.get("method").is_none() {
            return v;
        }
    }
}

pub(crate) fn hello() -> Value {
    json!({
        "protocolVersion": 1,
        "escouade": { "version": "2.2.0" },
        "plugin": { "id": "dev.escouade.test", "version": "1.0.0" },
        "capabilities": {},
    })
}

/// The error of a call of Linear when the account has no key.
fn without_a_key(p: &mut dyn Plugin) -> String {
    p.send(
        &json!({ "jsonrpc": "2.0", "id": 2, "method": "tickets/states", "params": linear_params(None) })
            .to_string(),
    );
    loop {
        let line = p.recv(Duration::from_secs(20)).expect("an answer");
        let v: Value = serde_json::from_str(&line).expect("JSON");
        if v["method"] == "secrets/get" {
            p.send(&json!({ "jsonrpc": "2.0", "id": v["id"], "result": null }).to_string());
        } else if v["id"] == json!(2) {
            return v["error"]["message"]
                .as_str()
                .unwrap_or_default()
                .to_string();
        }
    }
}

/// A module has no environment: it writes in the language `initialize` says, as its native plugin does.
#[test]
fn linear_writes_in_the_language_of_initialize() {
    let Some(linear) = module("linear") else {
        return;
    };
    for (lang, word) in [
        ("en", "Connect the account"),
        ("fr", "Reconnecte le compte"),
    ] {
        let mut greeting = hello();
        greeting["lang"] = json!(lang);
        let net = Net::default();
        let fetcher = Fetcher::new("dev.escouade.linear", &[], &net);
        let mut hosted = Hosted::start(linear.clone(), fetcher);
        assert!(call(&mut hosted, 1, "initialize", greeting)["result"].is_object());
        let said = without_a_key(&mut hosted);
        assert!(said.contains(word), "{lang}: {said}");
    }
}

/// Its ids are told apart from those of the instance before it: `std::process::id()` traps in a module, WASI's random
/// numbers stand in for it.
#[test]
fn two_instances_of_codex_have_ids_of_their_own() {
    let Some(codex) = module("codex") else {
        return;
    };
    let launch = || {
        let net = Net::default();
        let fetcher = Fetcher::new("dev.escouade.codex", &[], &net);
        let mut hosted = Hosted::start(codex.clone(), fetcher);
        let attach = json!({ "agent": "a", "cwd": "wt", "model": "gpt-6-luna", "mode": "default", "lang": "fr" });
        let mut launch = attach.clone();
        launch["start"] = json!({ "kind": "new" });
        call(&mut hosted, 1, "initialize", hello());
        call(&mut hosted, 2, "agent/attach", attach);
        let v = call(&mut hosted, 3, "engine/launch", launch);
        assert!(v["error"].is_null(), "{v}");
        v["result"].to_string()
    };
    assert_ne!(launch(), launch());
}

/// A fake Linear, as the prototype's: GraphQL on a socket of its own, keep-alive, one thread a connection. `slow<ms>` in
/// a request's body holds its answer back that long.
fn fake_linear(issues: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || {
                let _ = stream.set_nodelay(true);
                let mut writer = stream.try_clone().unwrap();
                let mut reader = BufReader::new(stream);
                loop {
                    let mut length = 0usize;
                    loop {
                        let mut line = String::new();
                        if reader.read_line(&mut line).unwrap_or(0) == 0 {
                            return;
                        }
                        if line == "\r\n" {
                            break;
                        }
                        if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                            length = v.trim().parse().unwrap_or(0);
                        }
                    }
                    let mut body = vec![0u8; length];
                    if reader.read_exact(&mut body).is_err() {
                        return;
                    }
                    let body = String::from_utf8_lossy(&body);
                    if let Some(rest) = body.split("slow").nth(1) {
                        let ms: String = rest.chars().take_while(char::is_ascii_digit).collect();
                        std::thread::sleep(Duration::from_millis(ms.parse().unwrap_or(0)));
                    }
                    let answer = linear_answer(&body, issues);
                    let head = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                        answer.len()
                    );
                    if writer.write_all(head.as_bytes()).is_err()
                        || writer.write_all(answer.as_bytes()).is_err()
                    {
                        return;
                    }
                }
            });
        }
    });
    base
}

fn linear_answer(body: &str, issues: usize) -> String {
    if body.contains("query Me") {
        return json!({ "data": { "viewer": {
            "id": "u1", "name": "Ada", "displayName": "ada", "email": "ada@example.dev",
        } } })
        .to_string();
    }
    if body.contains("states { nodes") {
        let kinds = [
            "triage",
            "backlog",
            "unstarted",
            "started",
            "completed",
            "canceled",
        ];
        let states: Vec<Value> = kinds
            .iter()
            .enumerate()
            .map(|(i, t)| {
                json!({ "id": format!("s{i}"), "name": format!("State {i}"), "type": t, "position": i })
            })
            .collect();
        return json!({ "data": { "team": { "states": { "nodes": states } } } }).to_string();
    }
    let nodes: Vec<Value> = (0..issues)
        .map(|i| {
            json!({
                "id": format!("id-{i}"), "identifier": format!("ENG-{i}"),
                "title": format!("Une issue de démonstration, la numéro {i}"),
                "description": "- [ ] Un critère\n- [ ] Un autre critère\n\nUn long texte. ".repeat(4),
                "url": format!("https://linear.app/acme/issue/ENG-{i}"), "priorityLabel": "High",
                "assignee": { "displayName": "ada", "name": "Ada" },
                "state": { "id": "s3", "name": "State 3" },
                "labels": { "nodes": [{ "name": "bug" }, { "name": "backend" }] },
            })
        })
        .collect();
    json!({ "data": { "issues": {
        "nodes": nodes, "pageInfo": { "hasNextPage": false, "endCursor": null },
    } } })
    .to_string()
}

/// What Linear is granted: its host, served by the fake.
fn linear_fetcher(base: &str) -> Fetcher {
    let net = Net {
        routes: HashMap::from([("api.linear.app".to_string(), base.to_string())]),
        dns: None,
        ..Net::default()
    };
    Fetcher::new("dev.escouade.linear", &["linear.app".to_string()], &net)
}

/// A call of the source Linear, as `ProcessTicketSource` writes it. The fake is reached by `account.api` natively, by
/// the route of `api.linear.app` in a module.
fn linear_params(native_base: Option<&str>) -> Value {
    json!({
        "point": "ticketSource",
        "source": "linear",
        "account": { "api": native_base },
        "container": "team-1",
        "query": { "text": "", "filters": [], "label": null, "page": null },
    })
}

#[test]
fn a_slow_call_of_linear_holds_back_neither_the_next_nor_the_thread() {
    let Some(linear) = module("linear") else {
        return;
    };
    let base = fake_linear(1);
    let mut hosted = Hosted::start(linear, linear_fetcher(&base));
    call(&mut hosted, 1, "initialize", hello());
    // The first is held 400 ms by the fake (`slow400` in its body, a search text), the second is not.
    let mut slow = linear_params(None);
    slow["query"]["text"] = json!("slow400");
    let write = |id: u64, params: &Value| {
        json!({ "jsonrpc": "2.0", "id": id, "method": "tickets/issues", "params": params })
            .to_string()
    };
    let started = Instant::now();
    hosted.send(&write(10, &slow));
    hosted.send(&write(11, &linear_params(None)));
    let mut order = Vec::new();
    while order.len() < 2 {
        let line = hosted.recv(Duration::from_secs(5)).expect("an answer");
        let v: Value = serde_json::from_str(&line).unwrap();
        if v["method"] == "secrets/get" {
            hosted.send(&json!({ "jsonrpc": "2.0", "id": v["id"], "result": "k" }).to_string());
        } else if v.get("method").is_none() {
            order.push((v["id"].as_u64().unwrap(), started.elapsed()));
        }
    }
    // The quick one first is what says it was not held back: no time of its own is asked, a slow runner takes more
    // than 400 ms for it (the CI's macOS, 2026-11-05: 515 ms).
    assert_eq!(order[0].0, 11, "the quick one first: {order:?}");
    assert!(order[1].1 >= Duration::from_millis(400), "{order:?}");
}

#[test]
fn a_host_outside_the_manifest_is_refused_to_linear() {
    let Some(linear) = module("linear") else {
        return;
    };
    let base = fake_linear(1);
    let net = Net {
        routes: HashMap::from([("api.linear.app".to_string(), base)]),
        dns: None,
        ..Net::default()
    };
    let fetcher = Fetcher::new("dev.escouade.linear", &[], &net);
    let mut hosted = Hosted::start(linear, fetcher);
    call(&mut hosted, 1, "initialize", hello());
    let a = call(&mut hosted, 2, "tickets/issues", linear_params(None));
    let said = a["error"]["message"].as_str().unwrap_or_default();
    assert!(said.contains("not in permissions.network"), "{a}");
}

/// A source of the first hand, on a fake service: what it is told and what the fake answers.
struct Source {
    name: &'static str,
    id: &'static str,
    /// The host of its manifest, which the fake stands in for.
    host: &'static str,
    network: &'static [&'static str],
    container: &'static str,
    item: &'static str,
    state: (&'static str, &'static str),
    routes: fn(&FakeServer),
}

const SOURCES: [Source; 4] = [
    Source {
        name: "linear",
        id: "dev.escouade.linear",
        host: "api.linear.app",
        network: &["api.linear.app"],
        container: "team-1",
        item: "i1",
        state: ("s-review", "In Review"),
        routes: linear_routes,
    },
    Source {
        name: "trello",
        id: "dev.escouade.trello",
        host: "api.trello.com",
        network: &["api.trello.com"],
        container: "b1",
        item: "c1",
        state: ("l2", "En cours"),
        routes: trello_routes,
    },
    Source {
        name: "jira",
        id: "dev.escouade.jira",
        host: "jira.test",
        network: &["*.atlassian.net", "{account}"],
        container: "ATL",
        item: "ATL-1",
        state: ("3", "In Progress"),
        routes: jira_routes,
    },
    Source {
        name: "clickup",
        id: "dev.escouade.clickup",
        host: "api.clickup.com",
        network: &["api.clickup.com"],
        container: "l1",
        item: "t1",
        state: ("in progress", "in progress"),
        routes: clickup_routes,
    },
];

fn linear_routes(s: &FakeServer) {
    s.on(
        "POST",
        "/graphql#Me",
        200,
        json!({ "data": { "viewer": { "id": "u1", "name": "Ada", "email": "ada@example.com" } } }),
    );
    s.on(
        "POST",
        "/graphql#Teams",
        200,
        json!({ "data": { "teams": { "nodes": [{ "id": "team-1", "key": "ENG", "name": "Engineering" }] } } }),
    );
    s.on(
        "POST",
        "/graphql#States",
        200,
        json!({ "data": { "team": { "states": { "nodes": [
            { "id": "s-done", "name": "Done", "type": "completed", "position": 0 },
            { "id": "s-review", "name": "In Review", "type": "started", "position": 2 },
            { "id": "s-todo", "name": "Todo", "type": "unstarted", "position": 1 },
        ] } } } }),
    );
    s.on(
        "POST",
        "/graphql#Issues",
        200,
        linear_answer_json("query Issues", 3),
    );
    s.on(
        "POST",
        "/graphql#SetState",
        200,
        json!({ "data": { "issueUpdate": { "success": true } } }),
    );
    s.on(
        "POST",
        "/graphql#Comment",
        200,
        json!({ "data": { "commentCreate": { "success": false } } }),
    );
}

fn linear_answer_json(body: &str, issues: usize) -> Value {
    serde_json::from_str(&linear_answer(body, issues)).unwrap()
}

fn trello_routes(s: &FakeServer) {
    s.on(
        "GET",
        "/members/me",
        200,
        json!({ "id": "me", "username": "ada" }),
    );
    s.on(
        "GET",
        "/members/me/boards",
        200,
        json!([{ "id": "b1", "name": "Atlas" }]),
    );
    crate::plugins_trello_tests::board(s);
    s.on("PUT", "/cards/c1", 200, json!({ "id": "c1" }));
    s.on(
        "POST",
        "/cards/c1/actions/comments",
        401,
        json!({ "message": "invalid token" }),
    );
}

fn jira_routes(s: &FakeServer) {
    s.on(
        "GET",
        "/rest/api/3/myself",
        200,
        json!({ "accountId": "acc-1", "emailAddress": "ada@atlas.dev", "displayName": "Ada" }),
    );
    s.on(
        "GET",
        "/rest/api/3/project/search",
        200,
        json!({ "isLast": true, "values": [{ "key": "ATL", "name": "Atlas" }] }),
    );
    s.on(
        "GET",
        "/rest/api/3/project/ATL/statuses",
        200,
        json!([{ "name": "Story", "statuses": [{ "id": "1", "name": "To Do" }, { "id": "3", "name": "In Progress" }] }]),
    );
    s.on(
        "POST",
        "/rest/api/3/search/jql",
        200,
        json!({ "issues": [{ "id": "10001", "key": "ATL-1", "fields": {
            "summary": "Rafraîchir le token", "issuetype": { "name": "Task" }, "priority": { "name": "High" },
            "assignee": null, "status": { "name": "To Do" }, "labels": ["claude-ready"],
            "description": { "type": "doc", "version": 1, "content": [
                { "type": "paragraph", "content": [{ "type": "text", "text": "Acceptance criteria:" }] },
                { "type": "bulletList", "content": [{ "type": "listItem", "content": [
                    { "type": "paragraph", "content": [{ "type": "text", "text": "Tests unitaires" }] }
                ] }] }
            ] },
        } }] }),
    );
    s.on(
        "POST",
        "/rest/api/3/search/approximate-count",
        200,
        json!({ "count": 1 }),
    );
    s.on(
        "GET",
        "/rest/api/3/issue/ATL-1",
        200,
        json!({ "fields": { "status": { "id": "1", "name": "To Do" } } }),
    );
    s.on(
        "GET",
        "/rest/api/3/issue/ATL-1/transitions",
        200,
        json!({ "transitions": [{ "id": "31", "name": "Start", "to": { "id": "3", "name": "In Progress" } }] }),
    );
    s.on(
        "POST",
        "/rest/api/3/issue/ATL-1/transitions",
        204,
        Value::Null,
    );
    s.on(
        "POST",
        "/rest/api/3/issue/ATL-1/comment",
        201,
        json!({ "id": "1" }),
    );
}

fn clickup_routes(s: &FakeServer) {
    s.on(
        "GET",
        "/user",
        200,
        json!({ "user": { "id": 183, "username": "Ada Lovelace", "email": "ada@atlas.dev" } }),
    );
    s.on(
        "GET",
        "/team",
        200,
        json!({ "teams": [{ "id": "w1", "name": "Atlas" }] }),
    );
    s.on(
        "GET",
        "/team/w1/space",
        200,
        json!({ "spaces": [{ "id": "s1", "name": "Produit" }] }),
    );
    s.on("GET", "/space/s1/folder", 200, json!({ "folders": [] }));
    s.on(
        "GET",
        "/space/s1/list",
        200,
        json!({ "lists": [{ "id": "l1", "name": "Sprint 12" }] }),
    );
    crate::plugins_clickup_tests::list(s);
    s.on(
        "GET",
        "/list/l1/task",
        200,
        json!({ "tasks": [crate::plugins_clickup_tests::task("t1", Some("ATL-1"), "Exporter le journal", "l1")], "last_page": true }),
    );
    s.on("PUT", "/task/t1", 200, json!({ "id": "t1" }));
    s.on_with(
        "POST",
        "/task/t1/comment",
        429,
        json!({ "err": "Rate limit reached" }),
        &[("X-RateLimit-Reset", "1")],
    );
}

/// What a source answers each call, and what it asked its service: compared whole, the fake's address taken out.
fn transcript(plugin: &mut dyn Plugin, s: &Source, server: &FakeServer) -> Value {
    (s.routes)(server);
    let account = match s.name {
        "jira" => {
            json!({ "site": format!("{}/", server.url), "email": "ada@atlas.dev", "label": "", "user": "" })
        }
        _ => json!({ "api": server.url, "site": "", "email": "", "label": "", "user": "me" }),
    };
    let secret = match s.name {
        "trello" => json!(r#"{"key":"k1","token":"t 1"}"#),
        "jira" => json!(" tok "),
        "linear" => json!(" lin_api_k1 "),
        _ => json!("pk_1"),
    };
    let params = |extra: Value| {
        let mut p = json!({ "point": "ticketSource", "source": s.name, "account": account });
        p.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        p
    };
    let reference = json!({ "service": s.name, "id": s.item, "key": s.item, "url": "" });
    let state = json!({ "id": s.state.0, "name": s.state.1 });
    let query = |text: &str, label: Value| json!({ "text": text, "filters": [], "label": label, "page": null });
    let calls = [
        ("account/me", params(json!({}))),
        ("tickets/containers", params(json!({}))),
        (
            "tickets/states",
            params(json!({ "container": s.container })),
        ),
        (
            "tickets/issues",
            params(json!({ "container": s.container, "query": query("", Value::Null) })),
        ),
        (
            "tickets/issues",
            params(
                json!({ "container": s.container, "query": query("token", json!("claude-ready")) }),
            ),
        ),
        (
            "tickets/issues",
            params(json!({ "container": "nowhere", "query": query("", Value::Null) })),
        ),
        (
            "tickets/setState",
            params(json!({ "ref": reference, "state": state, "mapped": [state] })),
        ),
        (
            "tickets/comment",
            params(json!({ "ref": reference, "text": "Fait, voir la PR.\n\n— Escouade" })),
        ),
        ("tickets/nothing", params(json!({}))),
    ];
    let mut greeting = hello();
    greeting["lang"] = json!("fr");
    assert!(call_with(plugin, 1, "initialize", greeting, &secret)["result"].is_object());
    let mut played = Vec::new();
    let mut seen = 0;
    for (i, (method, p)) in calls.into_iter().enumerate() {
        let v = call_with(plugin, 2 + i as u64, method, p, &secret);
        let requests = server.requests();
        let asked: Vec<Value> = requests[seen..]
            .iter()
            .map(|r| {
                let mut headers: Vec<(&String, &String)> = r
                    .headers
                    .iter()
                    .filter(|(k, _)| !HEADERS_OF_THE_CLIENT.contains(&k.as_str()))
                    .collect();
                headers.sort();
                json!({ "method": r.method, "target": r.target, "headers": headers, "body": r.body })
            })
            .collect();
        seen = requests.len();
        played.push(json!({
            "method": method, "result": v["result"], "error": v["error"], "asked": asked,
        }));
    }
    let bare = server.url.trim_start_matches("http://");
    let text = serde_json::to_string(&played)
        .unwrap()
        .replace(&server.url, "{base}")
        .replace(bare, "{host}");
    serde_json::from_str(&text).unwrap()
}

/// What each HTTP client adds on its own: not the plugin's words.
pub(crate) const HEADERS_OF_THE_CLIENT: [&str; 5] = [
    "host",
    "user-agent",
    "accept-encoding",
    "connection",
    "content-length",
];

/// The recording of a source, its native plugin's answers.
fn recording(name: &str) -> PathBuf {
    root()
        .join("tests/fixtures/plugins/parity")
        .join(format!("{name}.json"))
}

async fn parity_of(s: &Source) {
    let Some(module) = module(s.name) else {
        return;
    };
    let server = FakeServer::start().await;
    let net = Net {
        routes: HashMap::from([(s.host.to_string(), server.url.clone())]),
        dns: None,
        ..Net::default()
    };
    let network: Vec<String> = s.network.iter().map(ToString::to_string).collect();
    let mut hosted = Hosted::start(module, Fetcher::new(s.id, &network, &net));
    let played = tokio::task::block_in_place(|| transcript(&mut hosted, s, &server));
    if std::env::var_os("ESCOUADE_PARITY_RECORD").is_some() {
        let server = FakeServer::start().await;
        let mut native = Native::start(s.name);
        let was = tokio::task::block_in_place(|| transcript(&mut native, s, &server));
        let path = recording(s.name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, serde_json::to_string_pretty(&was).unwrap()).unwrap();
    }
    let was: Value = serde_json::from_str(&std::fs::read_to_string(recording(s.name)).unwrap())
        .expect("a recording");
    for (a, b) in played
        .as_array()
        .unwrap()
        .iter()
        .zip(was.as_array().unwrap())
    {
        assert_eq!(a, b, "{}: {}", s.name, a["method"]);
    }
    assert_eq!(played, was, "{}", s.name);
}

#[tokio::test(flavor = "multi_thread")]
async fn linear_answers_as_its_native_plugin() {
    parity_of(&SOURCES[0]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn trello_answers_as_its_native_plugin() {
    parity_of(&SOURCES[1]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn jira_answers_as_its_native_plugin() {
    parity_of(&SOURCES[2]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn clickup_answers_as_its_native_plugin() {
    parity_of(&SOURCES[3]).await;
}

/// The 50th and 99th percentiles of `rounds` calls of `tickets/issues`, in milliseconds.
fn latency(p: &mut dyn Plugin, base: &str, rounds: usize) -> (f64, f64) {
    assert!(call(p, 1, "initialize", hello())["result"].is_object());
    let mut times = Vec::with_capacity(rounds);
    for i in 0..rounds + 50 {
        let started = Instant::now();
        let v = call(p, 2 + i as u64, "tickets/issues", linear_params(Some(base)));
        assert!(v["error"].is_null(), "{v}");
        // The first ones warm it up, unmeasured.
        if i >= 50 {
            times.push(started.elapsed().as_secs_f64() * 1000.0);
        }
    }
    times.sort_by(f64::total_cmp);
    let at = |q: f64| times[((times.len() - 1) as f64 * q).round() as usize];
    (at(0.5), at(0.99))
}

/// INT-0 on a source (`npm run bench:int0`): a call of Linear through the host, one fetch and one secret, on a fake that
/// answers at once. With `ESCOUADE_NATIVE_PLUGINS`, a program of before its port, the same through its process: the
/// threshold is 5 ms more at the 99th percentile. Written to `bench-results/int0-linear-<ESCOUADE_BENCH_LABEL>.json`.
#[test]
#[ignore = "a measure, run by `npm run bench:int0`"]
fn int0_linear_bench() {
    let linear = module("linear").expect("sh scripts/plugins-wasm.sh");
    let rounds = 2000;
    let base = fake_linear(50);
    let mut hosted = Hosted::start(linear, linear_fetcher(&base));
    let (p50, p99) = latency(&mut hosted, &base, rounds);
    println!("linear in the host: p50 {p50:.2} ms, p99 {p99:.2} ms");
    let mut report = json!({ "rounds": rounds, "module": { "p50": p50, "p99": p99 } });
    if std::env::var_os("ESCOUADE_NATIVE_PLUGINS").is_some() {
        let mut native = Native::start("linear");
        let (n50, n99) = latency(&mut native, &base, rounds);
        println!("linear native: p50 {n50:.2} ms, p99 {n99:.2} ms");
        report["native"] = json!({ "p50": n50, "p99": n99 });
        assert!(p99 <= n99 + 5.0, "{p99} ms against {n99} ms");
    }
    let label = std::env::var("ESCOUADE_BENCH_LABEL").unwrap_or_else(|_| "run".into());
    let out = root().join("bench-results");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(
        out.join(format!("int0-linear-{label}.json")),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}

// ---- OpenCode (ESC-349) ---------------------------------------------------------------------------------------------

/// A `opencode serve` of the tests (tests/fixtures/fake-opencode.mjs), its requests in a file of its own.
struct FakeOpencode {
    child: Child,
    port: u16,
    line: String,
    log: PathBuf,
    dir: PathBuf,
    /// What the file held at the last look.
    seen: usize,
}

impl FakeOpencode {
    fn start(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "esc-opencode-parity-{name}-{}-{:?}",
            std::process::id(),
            Instant::now()
        ));
        let dir = PathBuf::from(dir.to_string_lossy().replace(['{', '}', ' ', ':', '.'], ""));
        std::fs::create_dir_all(dir.join("wt/src")).unwrap();
        std::fs::write(dir.join("wt/src/app.ts"), "const a = 1;\n").unwrap();
        let log = dir.join("requests.jsonl");
        let mut child = Command::new("node")
            .arg(root().join("tests/fixtures/fake-opencode.mjs"))
            .args(["serve", "--hostname", "127.0.0.1", "--port", "0"])
            .env("OPENCODE_SERVER_PASSWORD", "pw")
            .env(
                "OPENCODE_CONFIG_CONTENT",
                r#"{"model":"ollama/qwen3-coder:30b-32k"}"#,
            )
            .env("FAKE_OPENCODE_HOME", dir.join("home"))
            .env("FAKE_OPENCODE_LOG", &log)
            .env("FAKE_OPENCODE_STEP_MS", "5")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("node");
        let output = child.stdout.take().unwrap();
        let mut lines = BufReader::new(output).lines();
        let line = lines
            .find_map(|l| l.ok().filter(|l| l.contains("listening on")))
            .expect("the line of the server");
        let port = line.rsplit(':').next().unwrap().trim().parse().unwrap();
        Self {
            child,
            port,
            line,
            log,
            dir,
            seen: 0,
        }
    }

    fn wt(&self) -> String {
        self.dir.join("wt").to_string_lossy().into_owned()
    }

    /// A request of the test itself, outside what is compared.
    fn ask(&self, target: &str) -> String {
        let mut c = std::net::TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        // Base64 of `opencode:pw`.
        let head = format!(
            "POST {target} HTTP/1.1\r\nHost: x\r\nAuthorization: Basic b3BlbmNvZGU6cHc=\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
        );
        c.write_all(head.as_bytes()).unwrap();
        let mut said = String::new();
        let _ = c.read_to_string(&mut said);
        said.rsplit("\r\n\r\n").next().unwrap_or("").to_string()
    }

    /// Who reads the event stream.
    fn readers(&self) -> u64 {
        self.ask("/fake/readers").trim().parse().unwrap_or(0)
    }

    fn wait_readers(&self, n: u64) {
        let until = Instant::now() + Duration::from_secs(10);
        while self.readers() != n {
            assert!(Instant::now() < until, "nobody reads the stream again");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// What was asked of it since the last look.
    fn asked(&mut self) -> Vec<Value> {
        let text = std::fs::read_to_string(&self.log).unwrap_or_default();
        let new = text[self.seen..].to_string();
        self.seen = text.len();
        let mut asked: Vec<Value> = new
            .lines()
            .filter_map(|l| serde_json::from_str::<Value>(l).ok())
            .filter_map(|v| v.get("req").cloned())
            .filter(|r| r["path"] != "/global/event")
            .collect();
        // Sent side by side, in no fixed order.
        asked.sort_by_key(|r| format!("{} {} {}", r["path"], r["method"], r["body"]));
        asked
    }
}

impl Drop for FakeOpencode {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A plugin of OpenCode and what it said so far, its answers apart.
struct Speaker<'a> {
    plugin: &'a mut dyn Plugin,
    next: u64,
    items: Vec<Value>,
    answers: BTreeMap<u64, Value>,
}

impl Speaker<'_> {
    fn read(&mut self, within: Duration) -> bool {
        let Some(line) = self.plugin.recv(within) else {
            return false;
        };
        let v: Value = serde_json::from_str(&line).unwrap();
        if v["method"] == "agent/items" {
            self.items
                .extend(v["params"]["items"].as_array().unwrap().iter().cloned());
        } else if let Some(id) = v["id"].as_u64() {
            self.answers.insert(id, v);
        }
        true
    }

    fn send(&mut self, method: &str, params: Value) -> u64 {
        self.next += 1;
        let line = json!({ "jsonrpc": "2.0", "id": self.next, "method": method, "params": params });
        self.plugin.send(&line.to_string());
        self.next
    }

    /// The answer to the call `id`.
    fn answer(&mut self, id: u64) -> Value {
        let until = Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(v) = self.answers.remove(&id) {
                return v;
            }
            assert!(Instant::now() < until, "no answer to {id}");
            self.read(Duration::from_millis(200));
        }
    }

    fn call(&mut self, method: &str, params: Value) -> Value {
        let id = self.send(method, params);
        self.answer(id)
    }

    /// Reads until the item `op` came since `from`.
    fn until(&mut self, op: &str, from: usize) -> Value {
        let until = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(i) = self.items[from..].iter().find(|i| i["op"] == op) {
                return i.clone();
            }
            assert!(
                Instant::now() < until,
                "no {op} in {:?}, after {} items: {:?}",
                &self.items[from..],
                from,
                self.items.iter().rev().take(4).collect::<Vec<_>>()
            );
            self.read(Duration::from_millis(200));
        }
    }

    /// Reads until nothing more comes.
    fn settle(&mut self) {
        while self.read(Duration::from_millis(250)) {}
    }
}

/// `text` with what changes from one run to the next as the same words: ids by their first appearance, the address.
fn same_words(text: &str, ids: &mut HashMap<String, String>, fake: &FakeOpencode) -> String {
    // Its folder as it is, in JSON, and as a path of the fake's file; the same on every system.
    let wt = fake.wt();
    let text = text
        .replace(&fake.port.to_string(), "{port}")
        .replace(&wt, "{wt}")
        .replace(&wt.replace('\\', "\\\\"), "{wt}")
        .replace(&wt.replace('\\', "/").replace(':', ""), "{wt}")
        .replace("\\\\", "/")
        // Under Windows the fake keeps its folder inside itself (`path.join` of a path without its `:`).
        .replace("{wt}/{wt}", "{wt}");
    let mut out = String::new();
    let mut word = String::new();
    let mut flush = |word: &mut String, out: &mut String| {
        let id = word.split_once('_').is_some_and(|(kind, rest)| {
            kind.len() >= 3 && rest.len() >= 12 && rest.bytes().all(|b| b.is_ascii_hexdigit())
        });
        if id {
            let n = ids.len();
            out.push_str(ids.entry(word.clone()).or_insert(format!("id{n}")));
        } else {
            out.push_str(word);
        }
        word.clear();
    };
    for c in text.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
            out.push(c);
        }
    }
    flush(&mut word, &mut out);
    out
}

/// A step: what the plugin told of the conversation, what it answered, and what it asked the server, its order kept.
fn step(
    s: &mut Speaker,
    name: &str,
    answer: Value,
    fake: &mut FakeOpencode,
    from: &mut usize,
    played: &mut Vec<Value>,
) {
    s.settle();
    let items = s.items[*from..].to_vec();
    *from = s.items.len();
    played.push(json!({
        "step": name,
        "answer": { "result": answer["result"], "error": answer["error"] },
        "items": items,
        "asked": fake.asked(),
    }));
}

/// The fields that hold a time or a duration of the run.
fn without_times(v: Value) -> Value {
    match v {
        Value::Object(o) => o
            .into_iter()
            .filter(|(k, _)| !matches!(k.as_str(), "took" | "time" | "duration" | "at"))
            .map(|(k, v)| (k, without_times(v)))
            .collect(),
        Value::Array(a) => a.into_iter().map(without_times).collect(),
        other => other,
    }
}

/// The turns of an agent of OpenCode, on the fake server: what the plugin says and asks, a cut of its stream, and the
/// plugin started again in a turn.
fn opencode_transcript(plugin: &mut dyn Plugin, fake: &mut FakeOpencode) -> Value {
    let (agent, wt) = ("a1", fake.wt());
    let attach = json!({
        "agent": agent,
        "cwd": wt,
        "model": "ollama/qwen3-coder:30b-32k",
        "mode": "default",
        "lang": "fr",
        "shared": { "key": "k", "line": fake.line, "keep": { "password": "pw" } },
    });
    let mut played = Vec::new();
    let mut from = 0;
    let mut s = Speaker {
        plugin,
        next: 0,
        items: Vec::new(),
        answers: BTreeMap::new(),
    };
    let s = &mut s;
    let mut greeting = hello();
    greeting["lang"] = json!("fr");
    assert!(s.call("initialize", greeting)["result"].is_object());
    assert!(s.call("agent/attach", attach.clone())["error"].is_null());
    let a = s.call(
        "agent/open",
        json!({ "agent": agent, "start": { "kind": "new" } }),
    );
    assert!(a["error"].is_null(), "{a}");
    step(s, "open", a, fake, &mut from, &mut played);

    let say = |s: &mut Speaker, text: &str| {
        let params = json!({ "agent": agent, "text": text, "uuid": text });
        s.call("agent/sendMessage", params)
    };
    for text in [
        "bonjour",
        "raisonnement",
        "étapes",
        "sous-agent",
        "deux appels",
        "fichier",
    ] {
        let a = say(s, text);
        s.until("turn", from);
        step(s, text, a, fake, &mut from, &mut played);
    }

    let a = say(s, "commande");
    let card = s.until("permission", from);
    assert!(card["request"].is_string(), "{card}");
    let yes = s.call(
        "agent/answerPermission",
        json!({ "agent": agent, "request": card["request"], "allow": true }),
    );
    s.until("turn", from);
    step(s, "commande", json!([a, yes]), fake, &mut from, &mut played);

    let a = say(s, "commande");
    let card = s.until("permission", from);
    let no = s.call(
        "agent/answerPermission",
        json!({ "agent": agent, "request": card["request"], "allow": false, "message": "pas ça" }),
    );
    s.until("turn", from);
    step(
        s,
        "commande refusée",
        json!([a, no]),
        fake,
        &mut from,
        &mut played,
    );

    let a = say(s, "question");
    let card = s.until("question", from);
    let first = &card["questions"][0];
    let answers = json!({ first["question"].as_str().unwrap_or(""): first["options"][0]["label"] });
    let yes = s.call(
        "agent/answerQuestion",
        json!({
            "agent": agent, "request": card["request"], "call": card["call"],
            "answers": answers, "questions": card["questions"],
        }),
    );
    s.until("turn", from);
    step(s, "question", json!([a, yes]), fake, &mut from, &mut played);

    // A message of a turn waits for its end.
    let first = say(s, "lent");
    s.until("text", from);
    let held = say(s, "gardé pour après");
    let stop = s.call("agent/interrupt", json!({ "agent": agent }));
    let end = s.items.iter().rposition(|i| i["op"] == "turn").unwrap();
    s.until("turn", end + 1);
    step(
        s,
        "interruption",
        json!([first, held, stop]),
        fake,
        &mut from,
        &mut played,
    );

    // The plan, its mode set, approved.
    let mode = s.call("agent/setMode", json!({ "agent": agent, "value": "plan" }));
    let a = say(s, "propose un plan");
    let card = s.until("permission", from);
    let yes = s.call(
        "agent/answerPermission",
        json!({ "agent": agent, "request": card["request"], "allow": true, "resume": "default" }),
    );
    s.until("turn", from);
    step(
        s,
        "plan",
        json!([mode, a, yes]),
        fake,
        &mut from,
        &mut played,
    );

    let a = s.call("agent/compact", json!({ "agent": agent }));
    s.until("turn", from);
    step(s, "compaction", a, fake, &mut from, &mut played);

    let a = s.call(
        "agent/rename",
        json!({ "agent": agent, "value": "Nouveau titre" }),
    );
    step(s, "renommage", a, fake, &mut from, &mut played);
    let request = json!({ "method": "GET", "path": "/nothing/here" });
    let a = s.call(
        "agent/control",
        json!({ "agent": agent, "request": request, "withinMs": 5000 }),
    );
    step(s, "contrôle refusé", a, fake, &mut from, &mut played);

    // The stream cut between two turns: read again, the next turn goes as any other.
    fake.wait_readers(1);
    fake.ask("/fake/cut");
    fake.wait_readers(1);
    let a = say(s, "bonjour après la coupure");
    s.until("turn", from);
    step(
        s,
        "coupure entre deux tours",
        a,
        fake,
        &mut from,
        &mut played,
    );

    // Cut in a turn: what was lost is lost, the rest comes, and the turn can be stopped.
    let a = say(s, "lent");
    s.until("text", from);
    s.settle();
    fake.ask("/fake/cut");
    fake.wait_readers(1);
    let stop = s.call("agent/interrupt", json!({ "agent": agent }));
    s.until("turn", from);
    step(
        s,
        "coupure dans un tour",
        json!([a, stop]),
        fake,
        &mut from,
        &mut played,
    );

    // The session as a copy, then resumed.
    let session = s.items.iter().find_map(|i| {
        (i["op"] == "session")
            .then(|| i["session"].clone())
            .filter(Value::is_string)
    });
    let session = session.expect("its session");
    let d = s.call("agent/detach", json!({ "agent": agent }));
    assert!(d["error"].is_null(), "{d}");
    let mut other = attach.clone();
    other["agent"] = json!("a2");
    assert!(s.call("agent/attach", other)["error"].is_null());
    let a = s.call(
        "agent/open",
        json!({ "agent": "a2", "start": { "kind": "fork", "of": session } }),
    );
    step(s, "copie", a, fake, &mut from, &mut played);
    let mut other = attach.clone();
    other["agent"] = json!("a3");
    assert!(s.call("agent/attach", other)["error"].is_null());
    let a = s.call(
        "agent/open",
        json!({ "agent": "a3", "start": { "kind": "resume", "session": session } }),
    );
    step(s, "reprise", a, fake, &mut from, &mut played);
    let a = s.call("agent/letGo", json!({ "agent": "a3" }));
    step(s, "lâché", a, fake, &mut from, &mut played);
    let a = s.call(
        "agent/open",
        json!({ "agent": "a4", "start": { "kind": "new" } }),
    );
    step(s, "agent inconnu", a, fake, &mut from, &mut played);

    let mut ids = HashMap::new();
    let text = serde_json::to_string(&without_times(Value::Array(played))).unwrap();
    serde_json::from_str(&same_words(&text, &mut ids, fake)).unwrap()
}

/// `played` and `was` side by side, a step at a time: the first that differs says so.
fn same_steps(played: &Value, was: &Value) {
    for (a, b) in played
        .as_array()
        .unwrap()
        .iter()
        .zip(was.as_array().unwrap())
    {
        assert_eq!(
            serde_json::to_string_pretty(a).unwrap(),
            serde_json::to_string_pretty(b).unwrap(),
            "OpenCode, the step {}",
            a["step"]
        );
    }
    assert_eq!(played, was, "OpenCode: not the same steps");
}

/// The module on a port the core read; only that one.
fn opencode_hosted(module: wasmtime::Module, port: u16) -> Hosted {
    let net = Net {
        shared: Some(Arc::new(move |_| vec![port])),
        ..Net::default()
    };
    let network = ["local:shared".to_string()];
    let manifest = root().join("plugins/opencode/escouade-plugin.json");
    let manifest: Value = serde_json::from_slice(&std::fs::read(manifest).unwrap()).unwrap();
    let limits = crate::plugins::manifest::Limits {
        fetch_seconds: manifest["limits"]["fetchSeconds"].as_u64(),
        fetch_bytes: manifest["limits"]["fetchBytes"].as_u64(),
        ..Default::default()
    };
    Hosted::start(
        module,
        Fetcher::new("dev.escouade.opencode", &network, &net).limited(limits),
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn opencode_says_and_asks_as_its_native_plugin() {
    let Some(module) = module("opencode") else {
        return;
    };
    let mut fake = FakeOpencode::start("module");
    let mut hosted = opencode_hosted(module, fake.port);
    let played = tokio::task::block_in_place(|| opencode_transcript(&mut hosted, &mut fake));
    let path = recording("opencode");
    if std::env::var_os("ESCOUADE_PARITY_RECORD").is_some() {
        let mut fake = FakeOpencode::start("native");
        let mut native = Native::start("opencode");
        let was = tokio::task::block_in_place(|| opencode_transcript(&mut native, &mut fake));
        std::fs::write(&path, serde_json::to_string_pretty(&was).unwrap()).unwrap();
    }
    let was: Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).expect("a recording");
    same_steps(&played, &was);
}

/// A plugin started again in a turn, as the core gives it its agent: it hears its session again, and a turn that ended
/// meanwhile is closed.
#[tokio::test(flavor = "multi_thread")]
async fn opencode_started_again_goes_on_as_its_native_plugin() {
    let Some(module) = module("opencode") else {
        return;
    };
    let run = |plugin: &mut dyn Plugin,
               fake: &mut FakeOpencode,
               restart: &mut dyn FnMut() -> Box<dyn Plugin>| {
        let attach = json!({
            "agent": "a1",
            "cwd": fake.wt(),
            "model": "ollama/qwen3-coder:30b-32k",
            "mode": "default",
            "lang": "fr",
            "shared": { "key": "k", "line": fake.line, "keep": { "password": "pw" } },
        });
        let mut s = Speaker {
            plugin,
            next: 0,
            items: Vec::new(),
            answers: BTreeMap::new(),
        };
        s.call("initialize", hello());
        s.call("agent/attach", attach.clone());
        s.call(
            "agent/open",
            json!({ "agent": "a1", "start": { "kind": "new" } }),
        );
        let session = s.items.iter().find_map(|i| {
            (i["op"] == "session")
                .then(|| i["session"].clone())
                .filter(Value::is_string)
        });
        let params = json!({ "agent": "a1", "text": "bonjour", "uuid": "1" });
        s.call("agent/sendMessage", params);
        s.until("turn", 0);
        s.settle();
        // Its turn ended while the plugin was down, a turn the core still thinks runs.
        let mut again = restart();
        let mut s = Speaker {
            plugin: &mut *again,
            next: 0,
            items: Vec::new(),
            answers: BTreeMap::new(),
        };
        let mut attach = attach;
        attach["session"] = session.expect("its session");
        attach["running"] = json!(true);
        s.call("initialize", hello());
        s.call("agent/attach", attach);
        s.until("turn", 0);
        s.settle();
        let mut ids = HashMap::new();
        let text = serde_json::to_string(&without_times(json!(s.items))).unwrap();
        serde_json::from_str::<Value>(&same_words(&text, &mut ids, fake)).unwrap()
    };
    let mut fake = FakeOpencode::start("again-module");
    let port = fake.port;
    let mut first = opencode_hosted(module.clone(), port);
    let played = tokio::task::block_in_place(|| {
        run(&mut first, &mut fake, &mut || {
            Box::new(opencode_hosted(module.clone(), port))
        })
    });
    assert!(
        played.as_array().unwrap().iter().any(|i| i["op"] == "turn"),
        "{played}"
    );
    let name = "opencode-again";
    if std::env::var_os("ESCOUADE_PARITY_RECORD").is_some() {
        let mut fake = FakeOpencode::start("again-native");
        let mut native = Native::start("opencode");
        let was = tokio::task::block_in_place(|| {
            run(&mut native, &mut fake, &mut || {
                Box::new(Native::start("opencode"))
            })
        });
        std::fs::write(recording(name), serde_json::to_string_pretty(&was).unwrap()).unwrap();
    }
    let was: Value = serde_json::from_str(&std::fs::read_to_string(recording(name)).unwrap())
        .expect("a recording");
    assert_eq!(played, was);
}

// ---- INT-0 on the stream of OpenCode ---------------------------------------------------------------------------------

/// A server of the bench: what `agent/open` needs, then the stream it is told to write, without a delay of its own.
fn stream_server() -> (u16, Receiver<std::net::TcpStream>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, streams) = channel();
    std::thread::spawn(move || {
        for mut c in listener.incoming().map_while(Result::ok) {
            let tx = tx.clone();
            std::thread::spawn(move || loop {
                let mut head = Vec::new();
                let mut byte = [0u8; 1];
                while !head.ends_with(b"\r\n\r\n") {
                    if !matches!(c.read(&mut byte), Ok(1)) {
                        return;
                    }
                    head.push(byte[0]);
                }
                let text = String::from_utf8_lossy(&head).to_lowercase();
                let length = text
                    .lines()
                    .find_map(|l| l.strip_prefix("content-length: "))
                    .and_then(|l| l.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                let mut body = vec![0u8; length];
                if c.read_exact(&mut body).is_err() {
                    return;
                }
                let target = text.split_whitespace().nth(1).unwrap_or("").to_string();
                if target.starts_with("/global/event") {
                    let head = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: identity\r\nconnection: close\r\n\r\n";
                    c.write_all(head.as_bytes()).unwrap();
                    let _ = c.set_nodelay(true);
                    let _ = tx.send(c);
                    return;
                }
                let big;
                let json = if target.starts_with("/session/ses_big/message") {
                    big = big_session();
                    big.as_str()
                } else if target.starts_with("/session") {
                    r#"{"id":"ses_bench"}"#
                } else if target.starts_with("/config/providers") {
                    r#"{"providers":[],"default":{}}"#
                } else {
                    "{}"
                };
                let answer = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{json}",
                    json.len()
                );
                if c.write_all(answer.as_bytes()).is_err() {
                    return;
                }
            });
        }
    });
    (port, streams)
}

/// The messages of a long session: close to all a request of the plugin may read (`limits.fetchBytes`).
fn big_session() -> String {
    let words = "x".repeat(1 << 20);
    let messages: Vec<Value> = (0..15)
        .map(|n| json!({ "info": { "id": format!("m{n}") }, "parts": [{ "text": words }] }))
        .collect();
    Value::Array(messages).to_string()
}

/// A delta of a text: one event, one item.
fn delta_event(n: usize) -> String {
    let event = json!({ "payload": { "type": "message.part.delta", "properties": {
        "sessionID": "ses_bench", "messageID": "msg_b", "partID": format!("prt_{n}"),
        "field": "text", "delta": "Une ligne de texte comme un modèle en écrit, mot à mot. ",
    } } });
    format!("data: {event}\n\n")
}

/// The events of a real turn, as the stream wrote them, on the session of the bench.
fn recorded_events() -> Vec<String> {
    let path = root().join("tests/fixtures/opencode/full-turn.jsonl");
    let text = std::fs::read_to_string(path).expect("a recording");
    let session = "ses_eb4a5d013ffe3bev1AKFcCo7Cs";
    text.lines()
        .filter_map(|l| {
            let v: Value = serde_json::from_str(l).ok()?;
            let event = v["msg"].get("event").filter(|_| v["dir"] == "s2c")?;
            let payload = json!({ "payload": event }).to_string();
            Some(format!(
                "data: {}\n\n",
                payload.replace(session, "ses_bench")
            ))
        })
        .collect()
}

/// What the plugin takes of the stream of OpenCode: the time from an event written on its socket to the item of it on
/// the plugin's output, in turn (`rounds` events, each waited for), then how many lines a second it reads when they come
/// at once, a delta each and the events of a real turn.
fn stream_measures(
    plugin: &mut dyn Plugin,
    port: u16,
    streams: &Receiver<std::net::TcpStream>,
    rounds: usize,
) -> Value {
    let attach = json!({
        "agent": "a1",
        "cwd": "C:\\wt",
        "model": "ollama/m",
        "lang": "fr",
        "shared": {
            "key": "k",
            "line": format!("opencode server listening on http://127.0.0.1:{port}"),
            "keep": { "password": "pw" },
        },
    });
    let mut s = Speaker {
        plugin,
        next: 0,
        items: Vec::new(),
        answers: BTreeMap::new(),
    };
    s.call("initialize", hello());
    s.call("agent/attach", attach);
    let a = s.call(
        "agent/open",
        json!({ "agent": "a1", "start": { "kind": "new" } }),
    );
    assert!(a["error"].is_null(), "{a}");
    s.settle();
    let mut c = streams
        .recv_timeout(Duration::from_secs(5))
        .expect("its stream");

    let mut times = Vec::with_capacity(rounds);
    for n in 0..rounds {
        let event = delta_event(n);
        let at = Instant::now();
        c.write_all(event.as_bytes()).unwrap();
        let line = s.plugin.recv(Duration::from_secs(5)).expect("its item");
        times.push(at.elapsed().as_secs_f64() * 1000.0);
        // Its own item, not one of an event before it.
        assert!(line.contains(&format!("\"id\":\"prt_{n}\"")), "{n}: {line}");
    }
    times.sort_by(f64::total_cmp);
    let at = |q: f64| times[((times.len() as f64 * q) as usize).min(times.len() - 1)];

    let sent = 20_000;
    let burst: String = (0..sent).map(delta_event).collect();
    let writer = {
        let mut c = c.try_clone().unwrap();
        std::thread::spawn(move || c.write_all(burst.as_bytes()).unwrap())
    };
    let start = Instant::now();
    let mut got = 0;
    while got < sent {
        let line = s.plugin.recv(Duration::from_secs(30)).expect("its items");
        got += usize::from(line.contains("\"op\":\"delta\""));
    }
    let deltas = sent as f64 / start.elapsed().as_secs_f64();
    writer.join().unwrap();

    // A real turn, 20 times, its end marked by a delta.
    let events = recorded_events();
    let turn: String = events.concat();
    let lines = events.len() * 20;
    let mut all = turn.repeat(20);
    all.push_str(&delta_event(sent + 1));
    let writer = {
        let mut c = c.try_clone().unwrap();
        std::thread::spawn(move || c.write_all(all.as_bytes()).unwrap())
    };
    let target = format!("\"id\":\"prt_{}\"", sent + 1);
    let start = Instant::now();
    loop {
        let line = s.plugin.recv(Duration::from_secs(60)).expect("its items");
        if line.contains(&target) {
            break;
        }
    }
    let real = lines as f64 / start.elapsed().as_secs_f64();
    writer.join().unwrap();
    json!({
        "p50": at(0.5), "p99": at(0.99), "deltas_per_second": deltas,
        "real_lines_per_second": real, "real_lines": lines,
    })
}

/// A copy reads every message of the session: they fit in the memory of an instance (64 Mio) up to what the manifest lets
/// a request read.
#[test]
fn a_long_session_is_copied_within_the_memory_of_the_module() {
    let Some(module) = module("opencode") else {
        return;
    };
    let (port, _streams) = stream_server();
    let mut hosted = opencode_hosted(module, port);
    let mut s = Speaker {
        plugin: &mut hosted,
        next: 0,
        items: Vec::new(),
        answers: BTreeMap::new(),
    };
    s.call("initialize", hello());
    let attach = json!({
        "agent": "a1",
        "cwd": "C:\\wt",
        "model": "ollama/m",
        "shared": {
            "key": "k",
            "line": format!("opencode server listening on http://127.0.0.1:{port}"),
            "keep": { "password": "pw" },
        },
    });
    s.call("agent/attach", attach);
    let start = json!({ "kind": "fork", "of": "ses_big", "at": "m3" });
    let a = s.call("agent/open", json!({ "agent": "a1", "start": start }));
    assert!(a["error"].is_null(), "{a}");
}

/// INT-0 on `net/stream` (ESC-349): `npm run bench:int0 -- --opencode-only`, a release. With `ESCOUADE_NATIVE_PLUGINS`, a
/// program of before the port, it compares the two and holds the thresholds: 5 ms more at the 99th percentile, 2 200
/// lines a second.
#[test]
#[ignore = "a measure: npm run bench:int0"]
fn int0_opencode_stream_bench() {
    let module = module("opencode").expect("sh scripts/plugins-wasm.sh opencode");
    let rounds = 2000;
    let (port, streams) = stream_server();
    let mut hosted = opencode_hosted(module, port);
    let wasm = stream_measures(&mut hosted, port, &streams, rounds);
    println!("module: {wasm}");
    assert!(
        wasm["real_lines_per_second"].as_f64().unwrap() >= 2200.0,
        "{wasm}"
    );
    assert!(
        wasm["deltas_per_second"].as_f64().unwrap() >= 2200.0,
        "{wasm}"
    );
    if std::env::var_os("ESCOUADE_NATIVE_PLUGINS").is_none() {
        return;
    }
    let (port, streams) = stream_server();
    let mut native = Native::start("opencode");
    let was = stream_measures(&mut native, port, &streams, rounds);
    println!("native: {was}");
    let more = wasm["p99"].as_f64().unwrap() - was["p99"].as_f64().unwrap();
    assert!(more <= 5.0, "{more} ms more at the 99th percentile");
}

#[path = "plugins_wasm_agy_tests.rs"]
mod agy;
