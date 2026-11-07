//! The plugin on its lines, without a module: the host and the server are this file. What goes through a real
//! `opencode serve` (or its fake) is in the tests of the app, `src-tauri/src/opencode_tests.rs`.

use super::*;
use escouade_plugin_protocol::engine::method;

const BASE: &str = "http://127.0.0.1:4096";

#[derive(Default)]
struct Fake {
    plugin: Reactor,
    next: u64,
}

impl Fake {
    fn turn(&self, lines: Vec<String>) -> Vec<Value> {
        let out = self.plugin.turn(lines);
        out.iter()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    /// A call of the core; what the plugin wrote.
    fn call(&mut self, method: &str, params: Value) -> Vec<Value> {
        self.next += 1;
        let id = 1000 + self.next;
        let line = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        self.turn(vec![line.to_string()])
    }

    fn attach(&mut self) -> Vec<Value> {
        self.call(
            method::ATTACH,
            json!({
                "agent": "a1",
                "cwd": r"C:\wt",
                "model": "ollama/qwen",
                "mode": "default",
                "lang": "fr",
                "shared": {
                    "key": "k",
                    "line": format!("opencode server listening on {BASE}"),
                    "keep": { "password": "pw" },
                },
            }),
        )
    }

    /// The agent attached and its stream read, by the id `stream`.
    fn attached(&mut self, stream: u64) {
        let out = self.attach();
        let open = find(&out, "net/stream");
        let done = self.answer(open, stream_opened(stream, 200));
        assert!(find_all(&done, "net/fetch").is_empty(), "{done:?}");
    }

    /// The agent's session opened, new: `ses_1`.
    fn opened(&mut self, stream: u64) {
        self.attached(stream);
        let mut out = self.call(
            method::OPEN,
            json!({ "agent": "a1", "start": { "kind": "new" } }),
        );
        assert_eq!(fetched(&out), ("POST".into(), "/session".into()));
        out = self.answer(&out[0], fetch(200, json!({ "id": "ses_1" })));
        assert_eq!(fetched(&out), ("GET".into(), "/mcp".into()));
        out = self.answer(&out[0], fetch(200, json!({})));
        assert_eq!(
            fetched(&out[1..]),
            ("GET".into(), "/config/providers".into())
        );
        out = self.answer(
            &out[1],
            fetch(200, json!({ "providers": [], "default": {} })),
        );
        assert_eq!(fetched(&out), ("GET".into(), "/config".into()));
        out = self.answer(&out[0], fetch(200, json!({})));
        assert!(out.iter().any(|m| m["id"].as_u64() > Some(1000)), "{out:?}");
    }

    /// The answer to the call `asked` of the plugin.
    fn answer(&self, asked: &Value, result: Value) -> Vec<Value> {
        let line = json!({ "jsonrpc": "2.0", "id": asked["id"], "result": result });
        self.turn(vec![line.to_string()])
    }

    fn refuse(&self, asked: &Value, message: &str) -> Vec<Value> {
        let error = json!({ "code": -32000, "message": message });
        let line = json!({ "jsonrpc": "2.0", "id": asked["id"], "error": error });
        self.turn(vec![line.to_string()])
    }

    fn says(&self, method: &str, params: Value) -> Vec<Value> {
        let line = json!({ "jsonrpc": "2.0", "method": method, "params": params });
        self.turn(vec![line.to_string()])
    }

    /// `events` on the stream, in one piece.
    fn events(&self, stream: u64, events: &[Value]) -> Vec<Value> {
        let data: String = events.iter().map(|e| format!("data: {e}\n\n")).collect();
        self.says("net/chunk", json!({ "stream": stream, "data": data }))
    }
}

fn stream_opened(stream: u64, status: u16) -> Value {
    json!({ "stream": stream, "status": status, "headers": {} })
}

fn fetch(status: u16, body: Value) -> Value {
    json!({ "status": status, "headers": {}, "body": body.to_string() })
}

fn find_all<'a>(out: &'a [Value], method: &str) -> Vec<&'a Value> {
    out.iter().filter(|m| m["method"] == method).collect()
}

fn find<'a>(out: &'a [Value], method: &str) -> &'a Value {
    let found = find_all(out, method);
    found
        .first()
        .unwrap_or_else(|| panic!("no {method} in {out:?}"))
}

/// The method and the path of the first thing asked of the server.
fn fetched(out: &[Value]) -> (String, String) {
    let p = &find(out, "net/fetch")["params"];
    let url = p["url"].as_str().unwrap();
    let path = url.strip_prefix(BASE).unwrap().split('?').next().unwrap();
    (p["method"].as_str().unwrap().to_string(), path.to_string())
}

fn event(kind: &str, session: &str) -> Value {
    json!({
        "directory": r"C:\wt",
        "payload": { "type": kind, "properties": { "sessionID": session } },
    })
}

fn busy(session: &str) -> Value {
    json!({ "payload": {
        "type": "session.status",
        "properties": { "sessionID": session, "status": { "type": "busy" } },
    } })
}

/// What the plugin told the core of its agent's conversation.
fn items(out: &[Value]) -> Vec<&Value> {
    find_all(out, method::ITEMS)
        .iter()
        .flat_map(|m| m["params"]["items"].as_array().unwrap())
        .collect()
}

#[test]
fn the_handshake_and_what_is_no_call() {
    let mut f = Fake::default();
    let out = f.call("initialize", json!({ "protocolVersion": 1 }));
    assert_eq!(out[0]["result"]["protocolVersion"], 1);
    assert_eq!(out[0]["result"]["capabilities"]["harnesses"][0], "opencode");
    let out = f.turn(vec!["not json".to_string(), String::new()]);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0]["error"]["code"], code::PARSE);
    let out = f.call("nothing/here", json!({}));
    assert_eq!(out[0]["error"]["code"], code::METHOD_NOT_FOUND);
    let out = f.call("shutdown", json!({}));
    assert!(out[0]["result"].is_null(), "{out:?}");
}

#[test]
fn its_events_are_read_on_the_port_the_core_read_behind_its_password() {
    let mut f = Fake::default();
    let out = f.attach();
    assert_eq!(out.len(), 2, "the stream, the answer: {out:?}");
    let p = &find(&out, "net/stream")["params"];
    assert_eq!(p["url"], format!("{BASE}/global/event"));
    assert_eq!(p["method"], "GET");
    assert_eq!(p["sse"], true, "no idle limit on it");
    assert_eq!(p["headers"]["Accept"], "text/event-stream");
    assert_eq!(p["headers"]["Authorization"], "Basic b3BlbmNvZGU6cHc=");
}

#[test]
fn a_request_says_its_folder_and_its_json() {
    let mut f = Fake::default();
    f.attached(1);
    let out = f.call(
        method::OPEN,
        json!({ "agent": "a1", "start": { "kind": "new" } }),
    );
    let p = &find(&out, "net/fetch")["params"];
    assert_eq!(p["url"], format!("{BASE}/session?directory=C%3A%5Cwt"));
    assert_eq!(p["headers"]["Content-Type"], "application/json");
    assert_eq!(p["headers"]["Authorization"], "Basic b3BlbmNvZGU6cHc=");
    let body: Value = serde_json::from_str(p["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["title"], "Escouade");
}

#[test]
fn a_session_opens_once_the_stream_is_read() {
    let mut f = Fake::default();
    let out = f.attach();
    let open = find(&out, "net/stream").clone();
    let waiting = f.call(
        method::OPEN,
        json!({ "agent": "a1", "start": { "kind": "new" } }),
    );
    let timer = find(&waiting, "timer/set");
    assert_eq!(timer["params"]["ms"], 60_000);
    assert!(find_all(&waiting, "net/fetch").is_empty(), "{waiting:?}");
    let out = f.answer(timer, json!({ "timer": 8 }));
    assert!(out.is_empty(), "{out:?}");
    // Its stream read, in the same turn: the timer let go, the session asked.
    let out = f.answer(&open, stream_opened(1, 200));
    assert_eq!(fetched(&out), ("POST".into(), "/session".into()));
    assert_eq!(find(&out, "timer/cancel")["params"]["timer"], 8);
}

#[test]
fn no_stream_within_its_minute_refuses_the_session_in_its_language() {
    let mut f = Fake::default();
    f.attach();
    let waiting = f.call(
        method::OPEN,
        json!({ "agent": "a1", "start": { "kind": "new" } }),
    );
    let timer = find(&waiting, "timer/set");
    f.answer(timer, json!({ "timer": 8 }));
    let out = f.says("timer/fired", json!({ "timer": 8 }));
    let refused = out[0]["error"]["message"].as_str().unwrap();
    assert_eq!(refused, "Le flux d’événements d’OpenCode ne répond pas");
}

#[test]
fn a_cut_stream_is_opened_again_in_200_ms_and_goes_on() {
    let mut f = Fake::default();
    f.opened(1);
    let out = f.events(1, &[event("session.updated", "ses_1")]);
    assert!(find_all(&out, "net/stream").is_empty(), "{out:?}");

    let out = f.says("net/end", json!({ "stream": 1, "error": "cut" }));
    let timer = find(&out, "timer/set");
    assert_eq!(timer["params"]["ms"], 200);
    assert!(find_all(&out, "net/cancel").is_empty(), "it ended: {out:?}");
    assert!(f.answer(timer, json!({ "timer": 2 })).is_empty());
    let out = f.says("timer/fired", json!({ "timer": 2 }));
    let open = find(&out, "net/stream");
    assert_eq!(open["params"]["sse"], true);

    // A refused reopening waits as long, and tries again: as long as the server is said to run.
    let out = f.refuse(open, "connection refused");
    let timer = find(&out, "timer/set");
    assert_eq!(timer["params"]["ms"], 200);
    f.answer(timer, json!({ "timer": 3 }));
    let out = f.says("timer/fired", json!({ "timer": 3 }));
    let open = find(&out, "net/stream");

    // A server that answers an error too: its stream let go.
    let out = f.answer(open, stream_opened(2, 503));
    assert_eq!(find(&out, "net/cancel")["params"]["stream"], 2);
    let timer = find(&out, "timer/set");
    f.answer(timer, json!({ "timer": 4 }));
    let out = f.says("timer/fired", json!({ "timer": 4 }));
    let open = find(&out, "net/stream");

    // Back: the agent's events come again, on the new stream.
    assert!(f.answer(open, stream_opened(3, 200)).is_empty());
    let out = f.events(3, &[busy("ses_1"), event("session.idle", "ses_1")]);
    let told = items(&out);
    assert!(
        told.iter().any(|i| i["op"] == "turn"),
        "the turn's end after a cut: {out:?}"
    );
}

#[test]
fn an_event_cut_by_the_pieces_of_the_stream_is_read_whole_in_its_order() {
    let mut f = Fake::default();
    f.opened(1);
    let one = format!("data: {}\n\n", busy("ses_1"));
    let two = format!("data: {}\r\n\r\n", event("session.idle", "ses_1"));
    let all = format!("{one}{two}");
    let (a, b) = all.split_at(one.len() + 5);
    let piece = |data: &str| json!({ "stream": 1, "data": data });
    let out = f.says("net/chunk", piece(a));
    let credit = find(&out, "net/credit");
    assert_eq!(credit["params"]["stream"], 1);
    assert_eq!(credit["params"]["bytes"], a.len());
    assert!(items(&out).iter().all(|i| i["op"] != "turn"), "{out:?}");
    let out = f.says("net/chunk", piece(b));
    assert_eq!(find(&out, "net/credit")["params"]["bytes"], b.len());
    assert!(items(&out).iter().any(|i| i["op"] == "turn"), "{out:?}");
}

#[test]
fn a_message_of_a_turn_is_held_and_goes_at_its_end() {
    let mut f = Fake::default();
    f.opened(1);
    let send = |text: &str| json!({ "agent": "a1", "text": text, "uuid": text });
    let out = f.call(method::SEND_MESSAGE, send("one"));
    assert_eq!(
        fetched(&out),
        ("POST".into(), "/session/ses_1/prompt_async".into())
    );
    let asked = find(&out, "net/fetch").clone();
    assert_eq!(f.answer(&asked, fetch(204, Value::Null)).len(), 0);
    let out = f.call(method::SEND_MESSAGE, send("two"));
    assert!(find_all(&out, "net/fetch").is_empty(), "held: {out:?}");
    // An `idle` of the turn before: the turn goes on.
    let out = f.events(1, &[event("session.idle", "ses_1")]);
    assert!(find_all(&out, "net/fetch").is_empty(), "{out:?}");
    let out = f.events(1, &[busy("ses_1"), event("session.idle", "ses_1")]);
    let body = find(&out, "net/fetch")["params"]["body"].as_str().unwrap();
    let body: Value = serde_json::from_str(body).unwrap();
    assert_eq!(body["parts"][0]["text"], "two");
}

#[test]
fn a_prompt_refused_ends_its_turn_with_what_the_server_said() {
    let mut f = Fake::default();
    f.opened(1);
    let out = f.call(
        method::SEND_MESSAGE,
        json!({ "agent": "a1", "text": "one", "uuid": "u1" }),
    );
    let asked = find(&out, "net/fetch");
    let body = json!({ "data": { "message": "not this model" } });
    let out = f.answer(asked, fetch(400, body));
    let said = out[0].to_string();
    assert!(
        said.contains("POST /session/ses_1/prompt_async: 400 Bad Request not this model"),
        "{said}"
    );
}

/// The agent attached with `more` to what `attach` gives, its stream read.
fn attached_with(more: Value) -> Fake {
    let mut f = Fake::default();
    let mut params = json!({
        "agent": "a1",
        "cwd": r"C:\wt",
        "model": "ollama/qwen",
        "mode": "default",
        "lang": "fr",
        "shared": {
            "key": "k",
            "line": format!("opencode server listening on {BASE}"),
            "keep": { "password": "pw" },
        },
    });
    params
        .as_object_mut()
        .unwrap()
        .extend(more.as_object().unwrap().clone());
    let out = f.call(method::ATTACH, params);
    f.answer(find(&out, "net/stream"), stream_opened(1, 200));
    f
}

fn fenced(fence: Value) -> Fake {
    attached_with(json!({ "fence": fence }))
}

fn message() -> Value {
    json!({ "agent": "a1", "uuid": "u1", "text": "bonjour" })
}

/// Answers each request of `out` until the session is said opened, by `answer`; what the server was asked, in its
/// order.
fn until_open(f: &Fake, mut out: Vec<Value>) -> Vec<(String, String)> {
    let mut asked = Vec::new();
    for _ in 0..12 {
        let Some(request) = find_all(&out, "net/fetch").first().copied().cloned() else {
            break;
        };
        let (method, path) = fetched(std::slice::from_ref(&request));
        asked.push((method, path.clone()));
        let body = match path.as_str() {
            "/path" => json!({ "worktree": "C:/wt" }),
            "/session/ses_copy" | "/session" | "/session/old" => json!({ "id": "ses_copy" }),
            "/config/providers" => json!({ "providers": [], "default": {} }),
            _ => json!({}),
        };
        out = f.answer(&request, fetch(200, body));
    }
    asked
}

#[test]
fn a_copy_takes_its_rules_before_it_opens_and_is_refused_with_them() {
    let copy = json!({ "agent": "a1", "start": { "kind": "fork", "of": "old" } });
    // Refused: no session, the copy without rules goes, and nothing can be sent to it.
    let mut f = Fake::default();
    f.attached(1);
    let out = f.call(method::OPEN, copy.clone());
    let out = f.answer(&out[0], fetch(200, json!([])));
    assert_eq!(fetched(&out), ("POST".into(), "/session/old/fork".into()));
    let out = f.answer(&out[0], fetch(200, json!({ "id": "ses_copy" })));
    assert_eq!(fetched(&out), ("PATCH".into(), "/session/ses_copy".into()));
    let out = f.answer(&out[0], fetch(500, json!({ "message": "broken" })));
    assert_eq!(fetched(&out), ("DELETE".into(), "/session/ses_copy".into()));
    let out = f.answer(&out[0], fetch(200, json!(true)));
    let refused = out[0]["error"]["message"].as_str().unwrap();
    assert!(refused.contains("PATCH /session/ses_copy"), "{refused}");
    let sent = f.call(method::SEND_MESSAGE, message());
    assert!(sent[0]["error"]["message"]
        .as_str()
        .unwrap()
        .contains("no session"));
    assert!(find_all(&sent, "net/fetch").is_empty());

    // Taken: its rules come before anything else of it.
    let mut f = Fake::default();
    f.attached(1);
    let out = f.call(method::OPEN, copy);
    let out = f.answer(&out[0], fetch(200, json!([])));
    let out = f.answer(&out[0], fetch(200, json!({ "id": "ses_copy" })));
    assert_eq!(fetched(&out), ("PATCH".into(), "/session/ses_copy".into()));
    let body: Value = serde_json::from_str(out[0]["params"]["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["permission"][0]["action"], "ask");
    let out = f.answer(&out[0], fetch(200, json!({ "id": "ses_copy" })));
    assert_eq!(fetched(&out), ("GET".into(), "/mcp".into()));
}

#[test]
fn a_resumed_session_refused_its_rules_does_not_open_and_keeps_no_session() {
    let mut f = attached_with(json!({ "session": "old", "readOnly": true }));
    let out = f.call(
        method::OPEN,
        json!({ "agent": "a1", "start": { "kind": "resume", "session": "old" } }),
    );
    assert_eq!(fetched(&out), ("GET".into(), "/session/old".into()));
    let out = f.answer(&out[0], fetch(200, json!({ "id": "old" })));
    assert_eq!(fetched(&out), ("PATCH".into(), "/session/old".into()));
    let body: Value = serde_json::from_str(out[0]["params"]["body"].as_str().unwrap()).unwrap();
    let rules = body["permission"].as_array().unwrap();
    assert_eq!(rules[rules.len() - 1]["action"], "deny");
    let out = f.answer(&out[0], fetch(500, json!({ "message": "broken" })));
    assert!(out[0]["error"]["message"].as_str().is_some(), "{out:?}");
    let sent = f.call(method::SEND_MESSAGE, message());
    assert!(sent[0]["error"]["message"].as_str().is_some(), "{sent:?}");
}

#[test]
fn a_fenced_session_reads_its_worktree_before_its_rules_or_is_refused() {
    let f = fenced(json!([r"C:\wt\secret"]));
    let open = json!({ "agent": "a1", "start": { "kind": "new" } });
    let mut f = f;
    let out = f.call(method::OPEN, open.clone());
    assert_eq!(fetched(&out), ("GET".into(), "/path".into()));
    let out = f.answer(&out[0], fetch(200, json!({ "worktree": r"C:\wt" })));
    assert_eq!(fetched(&out), ("POST".into(), "/session".into()));
    let body: Value = serde_json::from_str(out[0]["params"]["body"].as_str().unwrap()).unwrap();
    assert!(body["permission"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["permission"] == "edit" && r["pattern"] == "secret/*" && r["action"] == "deny"));

    let mut f = fenced(json!([r"C:\wt\secret"]));
    let out = f.call(method::OPEN, open);
    let out = f.answer(&out[0], fetch(500, json!({ "message": "broken" })));
    assert!(out[0]["error"]["message"]
        .as_str()
        .unwrap()
        .contains("/path"));
    assert!(find_all(&out, "net/fetch").is_empty());
}

#[test]
fn a_mode_its_rules_refused_stays_the_one_that_was() {
    let mut f = Fake::default();
    f.opened(1);
    let change = json!({ "agent": "a1", "value": "bypassPermissions" });
    // Nothing is answered before the server is: the core would show the mode at once.
    let out = f.call(method::SET_MODE, change);
    assert_eq!(fetched(&out), ("PATCH".into(), "/session/ses_1".into()));
    let body: Value = serde_json::from_str(out[0]["params"]["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["permission"][0]["action"], "allow");
    let out = f.answer(&out[0], fetch(500, json!({ "message": "broken" })));
    assert!(out[0]["error"]["message"].as_str().is_some(), "{out:?}");
    // Its next prompt, and the rules given next, are of its old mode.
    let out = f.call(method::SET_MODE, json!({ "agent": "a1", "value": "plan" }));
    let out = f.answer(&out[0], fetch(500, json!({ "message": "broken" })));
    assert!(out[0]["error"]["message"].as_str().is_some(), "{out:?}");
    let out = f.call(method::SEND_MESSAGE, message());
    let sent = find(&out, "net/fetch");
    let body: Value = serde_json::from_str(sent["params"]["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["agent"], "build");
}

#[test]
fn a_plan_approved_starts_once_the_rules_of_its_mode_hold() {
    let mut f = Fake::default();
    f.opened(1);
    let approve = |f: &mut Fake| {
        f.call(
            method::ANSWER_PERMISSION,
            json!({ "agent": "a1", "request": "plan:m1", "allow": true, "resume": "acceptEdits" }),
        )
    };
    // Its prompt waits for its rules, and does not go without them.
    let out = approve(&mut f);
    let asked = find(&out, "net/fetch");
    assert_eq!(
        fetched(std::slice::from_ref(asked)),
        ("PATCH".into(), "/session/ses_1".into())
    );
    assert_eq!(find_all(&out, "net/fetch").len(), 1, "{out:?}");
    let out = f.answer(asked, fetch(500, json!({ "message": "broken" })));
    assert!(find_all(&out, "net/fetch").is_empty(), "{out:?}");

    let out = approve(&mut f);
    let out = f.answer(find(&out, "net/fetch"), fetch(200, json!({})));
    let asked = find(&out, "net/fetch");
    assert_eq!(
        fetched(std::slice::from_ref(asked)),
        ("POST".into(), "/session/ses_1/prompt_async".into())
    );
}

#[test]
fn a_fenced_card_offers_no_always_and_none_is_kept() {
    let mut f = fenced(json!(["C:/prive"]));
    let out = f.call(
        method::OPEN,
        json!({ "agent": "a1", "start": { "kind": "new" } }),
    );
    until_open(&f, out);
    let asked = json!({
        "id": "per_1", "sessionID": "ses_copy", "permission": "external_directory",
        "patterns": ["C:/autre/*"], "always": ["C:/*"], "metadata": {},
    });
    let out = f.events(
        1,
        &[json!({ "payload": { "type": "permission.asked", "properties": asked } })],
    );
    let card = items(&out)
        .into_iter()
        .find(|i| {
            i["kind"] == "permission" || i["type"] == "permission" || i.get("request").is_some()
        })
        .unwrap_or_else(|| panic!("{out:?}"));
    assert_eq!(card["canAlways"], false, "{card}");
    assert!(!card["description"]
        .as_str()
        .unwrap_or_default()
        .contains("Toujours"));
    let out = f.call(
        method::ANSWER_PERMISSION,
        json!({ "agent": "a1", "request": "per_1", "allow": true, "always": true }),
    );
    let reply = find(&out, "net/fetch");
    let body: Value = serde_json::from_str(reply["params"]["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["reply"], "once");
}

#[test]
fn an_unfenced_card_offers_always_as_it_did() {
    let mut f = Fake::default();
    f.opened(1);
    let asked = json!({
        "id": "per_1", "sessionID": "ses_1", "permission": "bash",
        "patterns": ["ls"], "always": ["ls *"], "metadata": {},
    });
    let out = f.events(
        1,
        &[json!({ "payload": { "type": "permission.asked", "properties": asked } })],
    );
    let card = items(&out)
        .into_iter()
        .find(|i| i.get("request").is_some())
        .unwrap_or_else(|| panic!("{out:?}"));
    assert_eq!(card["canAlways"], true, "{card}");
    let out = f.call(
        method::ANSWER_PERMISSION,
        json!({ "agent": "a1", "request": "per_1", "allow": true, "always": true }),
    );
    let reply = find(&out, "net/fetch");
    let body: Value = serde_json::from_str(reply["params"]["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["reply"], "always");
}

#[test]
fn a_session_gone_starts_a_new_one_and_a_server_that_fails_refuses_it() {
    let mut f = Fake::default();
    f.attached(1);
    let resume = json!({ "agent": "a1", "start": { "kind": "resume", "session": "old" } });
    let out = f.call(method::OPEN, resume.clone());
    assert_eq!(fetched(&out), ("GET".into(), "/session/old".into()));
    let out = f.answer(&out[0], fetch(404, json!({ "message": "gone" })));
    assert_eq!(fetched(&out), ("POST".into(), "/session".into()));

    let mut f = Fake::default();
    f.attached(1);
    let out = f.call(method::OPEN, resume);
    let out = f.answer(&out[0], fetch(500, json!({ "message": "broken" })));
    let refused = out[0]["error"]["message"].as_str().unwrap();
    assert_eq!(
        refused,
        "GET /session/old: 500 Internal Server Error broken"
    );
}

#[test]
fn a_request_the_host_refuses_says_it_and_a_compaction_past_the_limit_goes_on() {
    let mut f = Fake::default();
    f.opened(1);
    let out = f.call(method::RENAME, json!({ "agent": "a1", "value": "Nouveau" }));
    let out = f.refuse(&out[0], "127.0.0.1: refused");
    let refused = out[0]["error"]["message"].as_str().unwrap();
    assert_eq!(refused, "PATCH /session/ses_1: 127.0.0.1: refused");

    let out = f.call(method::COMPACT, json!({ "agent": "a1" }));
    let asked = find(&out, "net/fetch");
    let out = f.refuse(asked, "127.0.0.1: no answer within 60 s");
    assert!(out.is_empty(), "its end comes by `idle`: {out:?}");
}

#[test]
fn a_control_has_its_own_limit() {
    let mut f = Fake::default();
    f.opened(1);
    let out = f.call(
        method::CONTROL,
        json!({ "agent": "a1", "request": { "path": "/x" }, "withinMs": 500 }),
    );
    let timer = find(&out, "timer/set");
    assert_eq!(timer["params"]["ms"], 500);
    f.answer(timer, json!({ "timer": 9 }));
    let out = f.says("timer/fired", json!({ "timer": 9 }));
    let refused = out[0]["error"]["message"].as_str().unwrap();
    assert_eq!(refused, "/x: no answer in time");
}

#[test]
fn the_models_are_read_once_for_the_sessions_of_a_server() {
    let mut f = Fake::default();
    f.opened(1);
    let attach = json!({
        "agent": "a2",
        "cwd": r"C:\wt2",
        "model": "ollama/qwen",
        "shared": {
            "key": "k",
            "line": format!("opencode server listening on {BASE}"),
            "keep": { "password": "pw" },
        },
    });
    f.call(method::ATTACH, attach);
    let out = f.call(
        method::OPEN,
        json!({ "agent": "a2", "start": { "kind": "new" } }),
    );
    let out = f.answer(&out[0], fetch(200, json!({ "id": "ses_2" })));
    let out = f.answer(&out[0], fetch(200, json!({})));
    assert!(find_all(&out, "net/fetch").is_empty(), "{out:?}");
    assert!(out.iter().any(|m| m["id"].as_u64() > Some(1000)), "{out:?}");
}

#[test]
fn the_last_agent_gone_its_stream_is_let_go_and_read_no_more() {
    let mut f = Fake::default();
    f.opened(1);
    let out = f.call(method::DETACH, json!({ "agent": "a1" }));
    assert_eq!(find(&out, "net/cancel")["params"]["stream"], 1);
    assert!(
        find_all(&out, "timer/set").is_empty(),
        "nothing to open again: {out:?}"
    );
    let out = f.events(1, &[busy("ses_1")]);
    assert!(out.is_empty(), "{out:?}");
}

#[test]
fn a_plugin_started_again_hears_its_session_and_closes_a_turn_ended_meanwhile() {
    let mut f = Fake::default();
    let attach = json!({
        "agent": "a1",
        "cwd": r"C:\wt",
        "session": "ses_1",
        "model": "ollama/qwen",
        "running": true,
        "shared": {
            "key": "k",
            "line": format!("opencode server listening on {BASE}"),
            "keep": { "password": "pw" },
        },
    });
    let out = f.call(method::ATTACH, attach);
    let open = find(&out, "net/stream");
    let out = f.answer(open, stream_opened(1, 200));
    assert_eq!(fetched(&out), ("GET".into(), "/config/providers".into()));
    let asked = find(&out, "net/fetch");
    let out = f.answer(asked, fetch(200, json!({ "providers": [], "default": {} })));
    let out = f.answer(find(&out, "net/fetch"), fetch(200, json!({})));
    assert_eq!(fetched(&out), ("GET".into(), "/session/status".into()));
    let out = f.answer(find(&out, "net/fetch"), fetch(200, json!({})));
    assert!(items(&out).iter().any(|i| i["op"] == "turn"), "{out:?}");
}

/// A stop that comes while the plugin started again still reads its server: its turn was running, and the stop would be
/// lost (the CI's macOS, 2026-11-06).
#[test]
fn a_stop_right_after_the_plugin_started_again_reaches_the_server() {
    let mut f = Fake::default();
    let attach = json!({
        "agent": "a1",
        "cwd": r"C:\wt",
        "session": "ses_1",
        "model": "ollama/qwen",
        "running": true,
        "shared": {
            "key": "k",
            "line": format!("opencode server listening on {BASE}"),
            "keep": { "password": "pw" },
        },
    });
    f.call(method::ATTACH, attach);
    let out = f.call(
        method::INTERRUPT,
        json!({ "agent": "a1", "cancelQueued": false }),
    );
    assert_eq!(
        fetched(&out),
        ("POST".into(), "/session/ses_1/abort".into()),
        "{out:?}"
    );
}

#[test]
fn a_server_started_again_is_read_at_its_new_address_and_the_old_one_let_go() {
    let mut f = Fake::default();
    f.opened(1);
    let attach = json!({
        "agent": "a1",
        "cwd": r"C:\wt",
        "shared": {
            "key": "k",
            "line": "opencode server listening on http://127.0.0.1:5000",
            "keep": { "password": "other" },
        },
    });
    let out = f.call(method::ATTACH, attach);
    assert_eq!(find(&out, "net/cancel")["params"]["stream"], 1);
    assert_eq!(
        find(&out, "net/stream")["params"]["url"],
        "http://127.0.0.1:5000/global/event"
    );
}

#[test]
fn a_server_is_described_shared_with_its_password_never_among_its_arguments() {
    let l = launch(&LaunchParams {
        model: "ollama/qwen3-coder:30b-32k".into(),
        ..LaunchParams::default()
    });
    let password = l
        .env
        .iter()
        .find(|v| v.name == "OPENCODE_SERVER_PASSWORD")
        .map(|v| v.value.clone())
        .unwrap();
    assert_eq!(password.len(), 48);
    assert!(!l.args.iter().any(|a| a.contains(&password)));
    let shared = l.shared.unwrap();
    assert!(shared.one_agent_a_folder);
    assert_eq!(shared.keep["password"], password.as_str());
    assert!(l.fence_by_client && l.read_only);
}

#[test]
fn its_manifest_declares_the_harness_opencode() {
    let m: Value = serde_json::from_str(MANIFEST).unwrap();
    assert_eq!(m["id"], "dev.escouade.opencode");
    assert_eq!(m["run"]["wasm"], "Cargo.toml");
    assert_eq!(m["permissions"]["network"][0], "local:shared");
    let h = &m["contributes"]["harnesses"][0];
    assert_eq!(h["id"], "opencode");
    assert_eq!(h["accountVar"], "OPENCODE_CONFIG_DIR");
}

#[test]
fn the_tools_take_claude_codes_names() {
    let cwd = if cfg!(windows) { "C:\\p" } else { "/p" };
    let (n, i) = crate::translate::tool_of(
        "edit",
        &json!({ "filePath": "a.py", "oldString": "x", "newString": "y" }),
        cwd,
    );
    assert_eq!(n, "Edit");
    assert_eq!(
        i["file_path"],
        std::path::Path::new(cwd)
            .join("a.py")
            .to_string_lossy()
            .as_ref()
    );
    assert_eq!(
        crate::translate::tool_of("list", &json!({ "path": "src" }), cwd).0,
        "LS"
    );
    assert_eq!(
        crate::translate::tool_of("grep", &json!({ "pattern": "x", "include": "*.rs" }), cwd).1
            ["glob"],
        "*.rs"
    );
    assert_eq!(
        crate::translate::tool_of("escouade_get_board", &json!({}), cwd).0,
        "mcp__escouade__get_board"
    );
    let (n, i) = crate::translate::tool_of(
        "apply_patch",
        &json!({ "patchText": "*** Begin Patch\n*** Update File: src/a.ts\n@@\n-a\n+b\n*** End Patch" }),
        cwd,
    );
    assert_eq!(n, "Edit");
    assert!(i["file_path"].as_str().unwrap().ends_with("a.ts"));
    assert_eq!(
        crate::translate::tool_of("webfetch", &json!({ "url": "https://x" }), cwd).0,
        "WebFetch"
    );
    assert_eq!(
        crate::translate::tool_of("bash", &json!({ "command": "ls" }), cwd).1,
        json!({ "command": "ls" })
    );
}

#[test]
fn a_path_of_windows_or_of_posix_is_absolute_on_every_system() {
    let cwd = "/work/wt";
    for path in [
        r"C:\Users\user\wt\calc.py",
        "C:/Users/user/wt/calc.py",
        r"\\server\share\calc.py",
        "/home/user/wt/calc.py",
    ] {
        assert_eq!(crate::translate::absolute(path, cwd), path);
    }
    // A relative one is joined with the separator of the folder, whatever the system.
    assert_eq!(
        crate::translate::absolute("calc.py", cwd),
        "/work/wt/calc.py"
    );
    assert_eq!(
        crate::translate::absolute("src/app.ts", "/work/wt/"),
        "/work/wt/src/app.ts"
    );
    assert_eq!(
        crate::translate::absolute("calc.py", r"C:\Users\user\wt"),
        r"C:\Users\user\wt\calc.py"
    );
    assert_eq!(crate::translate::absolute("", cwd), "");
}
