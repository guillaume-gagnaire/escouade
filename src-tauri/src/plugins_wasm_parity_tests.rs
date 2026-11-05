//! The parity of the modules (ESC-330, ESC-331, ESC-334, ESC-344) through the host of the app: played by `wasm::Instance`
//! and `wasm::serve`, Codex answers as its native plugin, the `escouade-plugins` the build made beside the tests, and
//! Linear, Trello, Jira and ClickUp as theirs did, which their port took out of it: `tests/fixtures/plugins/parity/` holds
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

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// `target`, as the tests run in `target/debug/deps`, wherever it is.
fn target() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    exe.ancestors().nth(3).unwrap().to_path_buf()
}

/// A module of `scripts/plugins-wasm.sh`, compiled by the app's engine.
fn module(name: &str) -> Option<wasmtime::Module> {
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
trait Plugin {
    fn send(&mut self, line: &str);
    fn recv(&mut self, within: Duration) -> Option<String>;
}

/// A plugin in the host of the app, wired as `spawn_wasm` wires it.
struct Hosted {
    tx: UnboundedSender<Frame>,
    lines: Receiver<String>,
}

impl Hosted {
    fn start(module: wasmtime::Module, fetcher: Fetcher) -> Self {
        let (tx, rx) = unbounded_channel::<Frame>();
        let back = tx.downgrade();
        let (out, lines) = channel();
        std::thread::spawn(move || {
            let limits = wasm::Limits::default();
            let instance = Instance::start(&module, &BTreeMap::new(), limits).expect("an instance");
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
struct Native {
    child: Child,
    input: ChildStdin,
    lines: Receiver<String>,
}

impl Native {
    fn start(name: &str) -> Self {
        let exe = std::env::current_exe().unwrap();
        let dir = exe.parent().unwrap().parent().unwrap();
        let program = std::env::var_os("ESCOUADE_NATIVE_PLUGINS")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dir.join(format!("escouade-plugins{}", std::env::consts::EXE_SUFFIX))
            });
        let mut child = Command::new(&program)
            .arg(name)
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
fn call(p: &mut dyn Plugin, id: u64, method: &str, params: Value) -> Value {
    call_with(p, id, method, params, &json!("lin_api_fake"))
}

/// `call`, `secrets/get` answering `secret`.
fn call_with(p: &mut dyn Plugin, id: u64, method: &str, params: Value, secret: &Value) -> Value {
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

fn hello() -> Value {
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
    assert_eq!(order[0].0, 11, "the quick one first: {order:?}");
    assert!(order[0].1 < Duration::from_millis(390), "{order:?}");
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
const HEADERS_OF_THE_CLIENT: [&str; 5] = [
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

/// The server-to-client lines of a recording, as the harness wrote them.
fn said(file: &str) -> Vec<String> {
    let path = root().join("tests/fixtures/codex").join(file);
    std::fs::read_to_string(path)
        .expect("a recording")
        .lines()
        .filter_map(|l| {
            let v: Value = serde_json::from_str(l).ok()?;
            (v["dir"] == "s2c").then(|| v["msg"].to_string())
        })
        .collect()
}

/// The ids of a plugin's own requests start at a number of its own: what is compared is the rest.
fn without_ids(v: Value) -> Value {
    match v {
        Value::Object(o) => o
            .into_iter()
            .map(|(k, v)| {
                let kept = if k == "line" {
                    Value::Null
                } else {
                    without_ids(v)
                };
                (k, kept)
            })
            .collect(),
        Value::Array(a) => a.into_iter().map(without_ids).collect(),
        other => other,
    }
}

#[test]
fn codex_translates_a_whole_turn_as_its_native_plugin() {
    let Some(codex) = module("codex") else {
        return;
    };
    let net = Net::default();
    let mut hosted = Hosted::start(codex, Fetcher::new("dev.escouade.codex", &[], &net));
    let mut native = Native::start("codex");
    let attach = json!({ "agent": "a", "cwd": "C:\\wt", "model": "gpt-6-luna", "mode": "default", "lang": "fr" });
    let mut launch = attach.clone();
    launch["start"] = json!({ "kind": "new" });
    for p in [&mut hosted as &mut dyn Plugin, &mut native] {
        assert!(call(p, 1, "initialize", hello())["result"].is_object());
        assert!(call(p, 2, "agent/attach", attach.clone())["error"].is_null());
        assert!(call(p, 3, "engine/launch", launch.clone())["error"].is_null());
    }
    let (mut n, mut items) = (0, 0);
    for file in ["full-turn.jsonl", "plan-mode.jsonl"] {
        for (i, line) in said(file).iter().enumerate() {
            let p = json!({ "agent": "a", "frames": [line] });
            let id = 10 + i as u64;
            let a = call(&mut hosted, id, "agent/frames", p.clone());
            let b = call(&mut native, id, "agent/frames", p);
            assert!(a["error"].is_null(), "{a}");
            assert_eq!(
                without_ids(a["result"].clone()),
                without_ids(b["result"].clone()),
                "{file}, line {i}: {line}"
            );
            n += 1;
            items += a["result"]["items"].as_array().map_or(0, Vec::len);
        }
    }
    assert!(n > 300 && items > 200, "{n} lines, {items} items");
}
