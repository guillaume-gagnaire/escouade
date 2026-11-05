//! The parity of the modules (ESC-330, ESC-331, ESC-334) through the host of the app: Linear and Codex compiled to
//! `wasm32-wasip1` from their crates, played by `wasm::Instance` and `wasm::serve`, answer as their native plugin, the
//! `escouade-plugins` the build made beside the tests.
//!
//! Their modules come from `sh scripts/plugins-wasm.sh`, beside `cargo build --bins`. Without them, each test says so and
//! passes; with `ESCOUADE_PARITY=1` it fails.

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
    let path = target().join("plugins-wasm").join(format!("{name}.wasm"));
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
        let program = dir.join(format!("escouade-plugins{}", std::env::consts::EXE_SUFFIX));
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
    p.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string());
    loop {
        let line = p.recv(Duration::from_secs(20)).expect("an answer");
        let v: Value = serde_json::from_str(&line).expect("JSON");
        if v["method"] == "secrets/get" {
            p.send(
                &json!({ "jsonrpc": "2.0", "id": v["id"], "result": "lin_api_fake" }).to_string(),
            );
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
        let mut native = Native::start("linear");
        assert!(call(&mut hosted, 1, "initialize", greeting.clone())["result"].is_object());
        assert!(call(&mut native, 1, "initialize", greeting)["result"].is_object());
        let said = without_a_key(&mut hosted);
        assert!(said.contains(word), "{lang}: {said}");
        assert_eq!(said, without_a_key(&mut native));
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
fn linear_answers_as_its_native_plugin() {
    let Some(linear) = module("linear") else {
        return;
    };
    let base = fake_linear(50);
    let mut hosted = Hosted::start(linear, linear_fetcher(&base));
    let mut native = Native::start("linear");
    assert!(call(&mut hosted, 1, "initialize", hello())["result"].is_object());
    assert!(call(&mut native, 1, "initialize", hello())["result"].is_object());
    for method in ["account/me", "tickets/issues", "tickets/states"] {
        let a = call(&mut hosted, 2, method, linear_params(None));
        let b = call(&mut native, 2, method, linear_params(Some(&base)));
        assert!(a["error"].is_null(), "{method}: {a}");
        assert!(b["error"].is_null(), "{method}: {b}");
        assert_eq!(a["result"], b["result"], "{method}");
    }
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
