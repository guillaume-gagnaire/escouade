//! The client on its own, without Copilot: what it describes, what it writes, what it makes of the answers. The bench of
//! the harnesses plays it on the fake Copilot through its process (`src-tauri/src/engine/copilot/bench.rs`), the real
//! streams through the translation (`src-tauri/src/agent/copilot_tests.rs`).

use crate::client::Agent;
use escouade_plugin_protocol::engine::{Attach, Grant, Item, LaunchParams, StartAt};
use serde_json::{json, Value};

const CWD: &str = "C:\\wt";

fn attach() -> Attach {
    Attach {
        agent: "a1".into(),
        cwd: CWD.into(),
        model: "gpt-6-luna".into(),
        mode: "default".into(),
        lang: "fr".into(),
        ..Attach::default()
    }
}

fn params() -> LaunchParams {
    LaunchParams {
        agent: "a1".into(),
        cwd: CWD.into(),
        model: "gpt-6-luna".into(),
        mode: "default".into(),
        lang: "fr".into(),
        ..LaunchParams::default()
    }
}

fn parsed(line: &str) -> Value {
    serde_json::from_str(line).unwrap()
}

/// The lines among `items`.
fn writes(items: &[Item]) -> Vec<String> {
    items
        .iter()
        .filter_map(|i| match i {
            Item::Write { line } => Some(line.clone()),
            _ => None,
        })
        .collect()
}

/// Copilot's answer to the request `line`, by `answer` of its method (`None`: refused).
fn reply(line: &str, answer: &dyn Fn(&str) -> Option<Value>) -> String {
    let m = parsed(line);
    match answer(m["method"].as_str().unwrap()) {
        Some(result) => json!({ "id": m["id"], "result": result }).to_string(),
        None => json!({ "id": m["id"], "error": { "message": "non" } }).to_string(),
    }
}

/// `lines` answered one after the other until nothing is written any more: the items made, and the methods asked.
fn play(
    agent: &mut Agent,
    lines: Vec<String>,
    answer: &dyn Fn(&str) -> Option<Value>,
) -> (Vec<Item>, Vec<String>) {
    let mut items = Vec::new();
    let mut asked = Vec::new();
    let mut todo = lines;
    while let Some(line) = todo.pop() {
        asked.push(parsed(&line)["method"].as_str().unwrap().to_string());
        let out = agent.frames(&[reply(&line, answer)]);
        let mut more = writes(&out);
        more.reverse();
        todo.extend(more);
        items.extend(out.into_iter().filter(|i| !matches!(i, Item::Write { .. })));
    }
    (items, asked)
}

fn signed_in(method: &str) -> Option<Value> {
    Some(match method {
        "connect" => json!({ "protocolVersion": 3 }),
        "auth.getStatus" => json!({ "isAuthenticated": true, "authType": "user" }),
        "session.create" | "session.resume" | "sessions.fork" => json!({ "sessionId": "s1" }),
        "models.list" => json!({ "models": [] }),
        _ => json!({}),
    })
}

fn opened() -> Agent {
    let mut a = Agent::new(&attach());
    let lines = a.launch(&params()).writes;
    play(&mut a, lines, &signed_in);
    a
}

#[test]
fn escouades_token_goes_by_a_variable_and_its_file_by_the_arguments() {
    let mut a = Agent::new(&attach());
    let mut p = params();
    p.escouade = Some(Grant {
        url: "http://127.0.0.1:1/mcp".into(),
        token: "s3cr3t".into(),
    });
    p.escouade_file = Some("C:\\data\\mcp\\a1.json".into());
    let l = a.launch(&p);
    assert!(l.args.iter().any(|x| x == "--server"));
    assert!(l.args.iter().any(|x| x == "@C:\\data\\mcp\\a1.json"));
    assert!(!l.args.iter().any(|x| x.contains("s3cr3t")));
    assert_eq!(l.env.len(), 1);
    assert_eq!(l.env[0].name, "ESCOUADE_MCP_TOKEN");
    assert_eq!(l.env[0].value, "s3cr3t");
    assert!(l.remove_env.is_empty(), "the Principal keeps its tokens");
    assert_eq!(parsed(&l.writes[0])["method"], "connect");
}

#[test]
fn refused_escouades_server_is_turned_off_and_an_own_account_loses_the_tokens() {
    let mut a = Agent::new(&attach());
    let mut p = params();
    p.home = Some("C:\\accounts\\work".into());
    let l = a.launch(&p);
    assert!(l
        .args
        .windows(2)
        .any(|w| w == ["--disable-mcp-server", "escouade"]));
    assert!(l.env.is_empty());
    assert!(l.remove_env.iter().any(|v| v == "GH_TOKEN"));
    let (items, asked) = play(&mut a, l.writes, &signed_in);
    assert!(
        asked.iter().any(|m| m == "session.mcp.disable"),
        "{asked:?}"
    );
    assert!(items.iter().any(|i| matches!(i, Item::Ready { .. })));
}

#[test]
fn a_session_opens_on_its_modes_then_lists_the_models_and_the_agent_is_ready() {
    let mut a = Agent::new(&attach());
    let lines = a.launch(&params()).writes;
    let (items, asked) = play(&mut a, lines, &signed_in);
    assert_eq!(
        asked,
        [
            "connect",
            "auth.getStatus",
            "session.create",
            "session.permissions.setMode",
            "session.mode.set",
            "session.mcp.disable",
            "models.list",
            "session.model.list",
        ]
    );
    assert_eq!(
        items[0],
        Item::Session {
            session: "s1".into()
        }
    );
    let Some(Item::Ready { models }) = items.last() else {
        panic!("{items:?}");
    };
    assert_eq!(models[0]["id"], "gpt-6-luna");
    assert_eq!(models[0]["default"], true);
}

#[test]
fn a_message_sent_before_the_session_is_open_waits_for_it() {
    let mut a = Agent::new(&attach());
    let connect = a.launch(&params()).writes;
    assert!(a.send("bonjour", None).is_empty());
    let mut lines = connect;
    let mut sent = None;
    while let Some(line) = lines.pop() {
        for l in writes(&a.frames(&[reply(&line, &signed_in)])) {
            if parsed(&l)["method"] == "session.send" {
                sent = Some(parsed(&l));
            } else {
                lines.push(l);
            }
        }
    }
    let sent = sent.expect("the message goes once the agent is ready");
    assert_eq!(sent["params"]["prompt"], "bonjour");
    assert_eq!(sent["params"]["mode"], "enqueue");
    assert_eq!(sent["params"]["sessionId"], "s1");
}

#[test]
fn a_message_during_a_turn_goes_in_at_once_and_its_files_go_as_the_core_made_them() {
    let mut a = opened();
    let content = json!({ "prompt": "regarde", "attachments": [{ "type": "blob" }] });
    let first = parsed(&a.send("", Some(&content)).remove(0));
    assert_eq!(first["params"]["mode"], "enqueue");
    assert_eq!(first["params"]["attachments"][0]["type"], "blob");
    let second = parsed(&a.send("et ça", None).remove(0));
    assert_eq!(second["params"]["mode"], "immediate");
}

#[test]
fn another_protocol_or_no_sign_in_stops_it_and_says_why() {
    let old = |m: &str| match m {
        "connect" => Some(json!({ "protocolVersion": 2 })),
        m => signed_in(m),
    };
    let mut a = Agent::new(&attach());
    let lines = a.launch(&params()).writes;
    let (items, _) = play(&mut a, lines, &old);
    assert!(matches!(&items[..], [Item::Stop { reason }] if reason.contains("protocole 2")));

    let out = |m: &str| match m {
        "auth.getStatus" => Some(json!({ "isAuthenticated": false })),
        m => signed_in(m),
    };
    let mut a = Agent::new(&attach());
    let lines = a.launch(&params()).writes;
    let (items, _) = play(&mut a, lines, &out);
    assert!(matches!(&items[..], [Item::Stop { reason }] if reason.contains("copilot login")));

    // An account of its own on the GitHub CLI's sign-in is not signed in.
    let gh = |m: &str| match m {
        "auth.getStatus" => Some(json!({ "isAuthenticated": true, "authType": "gh-cli" })),
        m => signed_in(m),
    };
    let mut a = Agent::new(&attach());
    let mut p = params();
    p.home = Some("C:\\accounts\\work".into());
    let lines = a.launch(&p).writes;
    let (items, _) = play(&mut a, lines, &gh);
    assert!(matches!(&items[..], [Item::Stop { .. }]));
}

#[test]
fn a_session_that_cannot_be_resumed_gives_a_new_one_and_says_so() {
    let gone = |m: &str| match m {
        "session.resume" => None,
        "session.create" => Some(json!({ "sessionId": "s2" })),
        m => signed_in(m),
    };
    let mut a = Agent::new(&attach());
    let mut p = params();
    p.start = StartAt::Resume {
        session: "s1".into(),
    };
    let lines = a.launch(&p).writes;
    let (items, _) = play(&mut a, lines, &gone);
    assert!(matches!(&items[0], Item::Notice { text, .. } if text.contains("nouvelle session")));
    assert_eq!(
        items[1],
        Item::Session {
            session: "s2".into()
        }
    );
}

#[test]
fn a_read_in_its_folder_is_accepted_and_a_path_of_the_fence_refused_without_a_card() {
    let mut a = Agent::new(&attach());
    let mut p = params();
    p.fence = vec!["C:\\prive".into()];
    let lines = a.launch(&p).writes;
    play(&mut a, lines, &signed_in);
    let asks = |rid: &str, path: &str| {
        json!({ "method": "session.event", "params": { "sessionId": "s1", "event": {
            "id": rid, "type": "permission.requested",
            "data": { "requestId": rid, "permissionRequest": { "kind": "read", "path": path } },
        } } })
        .to_string()
    };
    let out = a.frames(&[asks("r1", "C:\\wt\\a.txt")]);
    let answer = parsed(&writes(&out)[0]);
    assert_eq!(answer["params"]["result"]["kind"], "approve-once");
    assert_eq!(writes(&out).len(), out.len(), "no card: {out:?}");
    let out = a.frames(&[asks("r2", "C:\\prive\\a.txt")]);
    let answer = parsed(&writes(&out)[0]);
    assert_eq!(answer["params"]["result"]["kind"], "reject");
    assert_eq!(writes(&out).len(), out.len(), "no card: {out:?}");
    // Out of its folder: a card.
    let out = a.frames(&[asks("r3", "C:\\ailleurs\\a.txt")]);
    assert!(matches!(&out[0], Item::PermissionCard { request, .. } if request == "r3"));
}

#[test]
fn the_quota_and_a_compaction_are_told_when_copilot_answers() {
    let mut a = opened();
    let line = a.read_usage().remove(0);
    let out = a.frames(&[reply(&line, &|_| {
        Some(json!({ "snapshots": { "chat": { "entitlementRequests": 200, "remainingPercentage": 80 } } }))
    })]);
    let Item::Usage { usage } = &out[0] else {
        panic!("{out:?}");
    };
    assert_eq!(usage["meters"][0]["pct"], 20.0);
    let line = a.compact("garde les décisions").remove(0);
    assert_eq!(
        parsed(&line)["params"]["customInstructions"],
        "garde les décisions"
    );
    let out = a.frames(&[reply(&line, &|_| {
        Some(json!({ "success": false, "error": "trop court" }))
    })]);
    assert_eq!(
        out,
        [Item::Compacted {
            error: Some("trop court".into())
        }]
    );
}

#[test]
fn a_plugin_started_again_goes_on_with_the_agents_session() {
    // No launch: `agent/attach` alone gives it again, its session known.
    let mut again = attach();
    again.session = Some("s1".into());
    let mut a = Agent::new(&again);
    let sent = parsed(&a.send("bonjour", None).remove(0));
    assert_eq!(sent["method"], "session.send");
    assert_eq!(sent["params"]["sessionId"], "s1");
    // The turn it did not see begin ends all the same.
    let idle = json!({ "method": "session.event", "params": { "sessionId": "s1", "event": {
        "id": "e1", "type": "session.idle", "data": {},
    } } });
    let out = a.frames(&[idle.to_string()]);
    assert!(
        out.iter().any(|i| matches!(i, Item::Turn { .. })),
        "{out:?}"
    );
}

#[test]
fn its_manifest_declares_copilot_with_its_framing_and_its_close() {
    let v: Value = serde_json::from_str(crate::MANIFEST).unwrap();
    let h = &v["contributes"]["harnesses"][0];
    assert_eq!(h["id"], "copilot");
    assert_eq!(h["framing"], "contentLength");
    assert_eq!(h["opens"], true);
    let close: Value = serde_json::from_str(h["closeLine"].as_str().unwrap()).unwrap();
    assert_eq!(close["method"], "runtime.shutdown");
    assert_eq!(v["run"]["wasm"], "Cargo.toml");
}

#[test]
fn the_host_of_an_address_is_read_as_a_permission_wants_it() {
    let host = |u: &str| crate::words::host_of(u);
    assert_eq!(
        host("https://Example.COM/a?b").as_deref(),
        Some("example.com")
    );
    assert_eq!(
        host("http://moi:mdp@api.exemple.fr:8080").as_deref(),
        Some("api.exemple.fr")
    );
    assert_eq!(host("http://[::1]:3000/x").as_deref(), Some("[::1]"));
    for no in [
        "pas une adresse",
        "https://",
        "https://exämple.com/",
        "1a://x",
    ] {
        assert_eq!(host(no), None, "{no}");
    }
}

#[test]
fn the_tools_take_claude_codes_names() {
    let cwd = "C:\\p";
    let (n, i) = crate::translate::tool_of("powershell", &json!({ "command": "ls" }), None, cwd);
    assert_eq!((n.as_str(), i["command"].as_str()), ("Bash", Some("ls")));
    let (n, i) = crate::translate::tool_of("view", &json!({ "path": "a.js" }), None, cwd);
    assert_eq!(
        (n.as_str(), i["file_path"].as_str()),
        ("Read", Some("C:\\p\\a.js"))
    );
    let (n, _) = crate::translate::tool_of("rg", &json!({ "pattern": "x" }), None, cwd);
    assert_eq!(n, "Grep");
    let (n, i) = crate::translate::tool_of(
        "apply_patch",
        &json!("*** Begin Patch\n*** Add File: b.txt\n+x\n*** End Patch\n"),
        None,
        cwd,
    );
    assert_eq!(
        (n.as_str(), i["file_path"].as_str()),
        ("Write", Some("C:\\p\\b.txt"))
    );
    let (n, _) = crate::translate::tool_of("essai-ping", &json!({}), Some("essai"), cwd);
    assert_eq!(n, "mcp__essai__ping");
    let (n, _) = crate::translate::tool_of(
        "github-mcp-server/get_file_contents",
        &json!({}),
        Some("github-mcp-server"),
        cwd,
    );
    assert_eq!(n, "mcp__github-mcp-server__get_file_contents");
    let (n, _) = crate::translate::tool_of("sql", &json!({}), None, cwd);
    assert_eq!(n, "sql");
}

/// A conversation that holds nothing.
struct Nothing;

impl crate::translate::Seen for Nothing {
    fn has(&self, _: &str) -> bool {
        false
    }
    fn call(&self, _: &str) -> Option<Value> {
        None
    }
}

fn safe(r: &Value, mode: &str, discussion: bool) -> bool {
    let c = crate::translate::Ctx {
        cwd: CWD,
        mode,
        active: true,
        discussion,
        fenced: false,
        process: true,
        lang: "fr",
        resumed: None,
        seen: &Nothing,
    };
    crate::translate::Translator::safe(r, &c)
}

#[test]
fn safe_requests_go_without_a_card_only_when_a_process_answers_them() {
    let read = json!({ "kind": "read", "path": format!("{CWD}\\calc.js") });
    let outside = json!({ "kind": "read", "path": "C:\\Windows\\win.ini" });
    let write = json!({ "kind": "write", "fileName": format!("{CWD}\\a.txt") });
    assert!(safe(&read, "acceptEdits", false));
    assert!(!safe(&outside, "acceptEdits", false));
    assert!(safe(&write, "acceptEdits", false));
    assert!(
        !safe(&write, "plan", false),
        "nothing written in the mode Plan"
    );
    assert!(safe(&read, "plan", false));
    assert!(safe(&write, "default", false));
    let npm = json!({ "kind": "shell", "fullCommandText": "npm test", "commands": [{ "identifier": "npm test" }] });
    assert!(!safe(&npm, "default", false));
    // A discussion of the Docs tab (ESC-261): its reads alone, in any mode.
    let commit = json!({ "kind": "shell", "fullCommandText": "git commit -m x", "commands": [{ "identifier": "git commit" }] });
    for mode in ["default", "acceptEdits", "auto"] {
        assert!(!safe(&write, mode, true), "{mode}");
        assert!(!safe(&commit, mode, true), "{mode}");
        assert!(safe(&read, mode, true), "{mode}");
    }
}

#[path = "rules_tests.rs"]
mod rules;
