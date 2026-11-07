//! What holds through the service, as the core calls it, for each harness on this client: a restart of the plugin, a write
//! in `.git` (ESC-362).

use super::*;
use escouade_plugin_protocol::engine::Item;

const AGENT: &str = "agent-abc";

/// A harness on this client, with its extension.
fn decl(extension: Option<&str>, fs: bool) -> HarnessDecl {
    let extensions: Vec<&str> = extension.into_iter().collect();
    serde_json::from_value(json!({
        "id": "h", "name": "H",
        "acp": {
            "args": ["--acp"], "fs": fs, "fenceOnPermissions": !fs, "readOnlyMode": "ask",
            "modes": { "default": "default", "acceptEdits": "accept-edits", "bypassPermissions": "auto-approve" },
            "extensions": extensions,
        },
    }))
    .unwrap()
}

/// Each harness on this client, as its manifest has it: the plugin ACP's, Cursor, Gemini and Vibe.
fn harnesses() -> Vec<HarnessDecl> {
    vec![
        decl(None, true),
        decl(Some("cursor"), false),
        decl(Some("gemini"), false),
        decl(Some("vibe"), false),
    ]
}

/// A service, as a plugin of a harness runs it.
type Service = Box<dyn FnMut(&str, Value) -> Result<Value, Refusal>>;

fn service(d: &HarnessDecl) -> Service {
    Box::new(handler(Some(d.clone())))
}

fn call(s: &mut Service, m: &str, params: impl serde::Serialize) -> Value {
    s(m, serde_json::to_value(params).unwrap()).unwrap_or_else(|e| panic!("{m}: {}", e.message))
}

/// What the agent wrote: the lines the client answers.
fn told(s: &mut Service, frame: Value) -> Vec<Value> {
    let frames = json!({ "agent": AGENT, "frames": [frame.to_string()] });
    let items: Vec<Item> =
        serde_json::from_value(call(s, method::FRAMES, frames)["items"].clone()).unwrap();
    items
        .iter()
        .filter_map(|i| match i {
            Item::Write { line } => serde_json::from_str(line).ok(),
            _ => None,
        })
        .collect()
}

/// What the client answers of a permission: its option, or None for a card.
fn permission(s: &mut Service, id: u64, kind: &str, path: &str) -> Option<String> {
    let lines = told(
        s,
        json!({
            "jsonrpc": "2.0", "id": id, "method": "session/request_permission",
            "params": {
                "sessionId": "s1",
                "options": [
                    { "kind": "allow_once", "optionId": "allow" },
                    { "kind": "reject_once", "optionId": "reject" },
                ],
                "toolCall": {
                    "toolCallId": format!("c{id}"), "title": "Écrire", "kind": kind,
                    "locations": [{ "path": path }],
                    "content": [{ "type": "diff", "path": path, "oldText": "a", "newText": "b" }],
                },
            },
        }),
    );
    lines.iter().find(|l| l["id"] == id).map(|l| {
        l["result"]["outcome"]["optionId"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    })
}

fn root() -> String {
    let dir = std::env::temp_dir().join(format!("esc362-acp-{}", std::process::id()));
    let _ = std::fs::create_dir_all(dir.join("wt"));
    let _ = std::fs::create_dir_all(dir.join("prive"));
    std::fs::write(dir.join("prive").join("secret.txt"), "secret").unwrap();
    std::fs::write(dir.join("wt").join("a.txt"), "a").unwrap();
    dir.to_string_lossy().into_owned()
}

fn attached(cwd: &str, fence: &[String], read_only: bool, mode: &str) -> Attach {
    Attach {
        agent: AGENT.into(),
        cwd: cwd.into(),
        mode: mode.into(),
        lang: "fr".into(),
        session: Some("s1".into()),
        fence: fence.to_vec(),
        read_only,
        ..Attach::default()
    }
}

/// An agent launched with `fence` and `read_only`, its session open.
fn launched(d: &HarnessDecl, cwd: &str, fence: &[String], read_only: bool, mode: &str) -> Service {
    let mut s = service(d);
    let params = LaunchParams {
        agent: AGENT.into(),
        cwd: cwd.into(),
        mode: mode.into(),
        lang: "fr".into(),
        fence: fence.to_vec(),
        read_only,
        ..LaunchParams::default()
    };
    call(&mut s, method::LAUNCH, params);
    // The session it runs, as after a restart: what follows names it.
    s
}

/// What the client answers of a file read through it: Ok, or the refusal.
fn file(s: &mut Service, method: &str, id: u64, path: &str) -> Result<(), String> {
    let lines = told(
        s,
        json!({
            "jsonrpc": "2.0", "id": id, "method": method,
            "params": { "sessionId": "s1", "path": path, "content": "x" },
        }),
    );
    let line = lines.iter().find(|l| l["id"] == id).expect("an answer");
    match line["error"]["message"].as_str() {
        Some(e) => Err(e.to_string()),
        None => Ok(()),
    }
}

#[test]
fn the_fence_holds_before_and_after_a_restart_of_the_plugin_on_each_harness() {
    let dir = root();
    let cwd = format!("{dir}/wt");
    let fence = vec![format!("{dir}/prive")];
    let secret = format!("{dir}/prive/secret.txt");
    for d in harnesses() {
        let core = attached(&cwd, &fence, false, "default");
        let mut before = launched(&d, &cwd, &fence, false, "default");
        let got = permission(&mut before, 1, "read", &secret);
        assert_eq!(got.as_deref(), Some("reject"), "{:?}", d.acp);
        drop(before);
        let mut after = service(&d);
        call(&mut after, method::ATTACH, &core);
        let got = permission(&mut after, 1, "read", &secret);
        assert_eq!(got.as_deref(), Some("reject"), "{:?}", d.acp);
    }
}

#[test]
fn the_files_through_the_client_hold_before_and_after_a_restart_of_the_plugin() {
    let dir = root();
    let cwd = format!("{dir}/wt");
    let fence = vec![format!("{dir}/prive")];
    let secret = format!("{dir}/prive/secret.txt");
    let d = decl(None, true);
    let core = attached(&cwd, &fence, false, "default");
    let mut before = launched(&d, &cwd, &fence, false, "default");
    assert!(file(&mut before, "fs/read_text_file", 1, &secret).is_err());
    drop(before);
    let mut after = service(&d);
    call(&mut after, method::ATTACH, &core);
    assert!(file(&mut after, "fs/read_text_file", 1, &secret).is_err());
    assert!(file(&mut after, "fs/write_text_file", 2, &secret).is_err());
    assert!(file(&mut after, "fs/read_text_file", 3, &format!("{cwd}/a.txt")).is_ok());
}

#[test]
fn read_only_holds_before_and_after_a_restart_of_the_plugin_on_each_harness() {
    let dir = root();
    let cwd = format!("{dir}/wt");
    let target = format!("{cwd}/a.txt");
    for d in harnesses() {
        let core = attached(&cwd, &[], true, "acceptEdits");
        let mut before = launched(&d, &cwd, &[], true, "acceptEdits");
        let got = permission(&mut before, 1, "edit", &target);
        assert_eq!(got.as_deref(), Some("reject"), "{:?}", d.acp);
        drop(before);
        let mut after = service(&d);
        call(&mut after, method::ATTACH, &core);
        let got = permission(&mut after, 1, "edit", &target);
        assert_eq!(got.as_deref(), Some("reject"), "{:?}", d.acp);
    }
    // A write through the client.
    let d = decl(None, true);
    let mut after = service(&d);
    call(
        &mut after,
        method::ATTACH,
        attached(&cwd, &[], true, "acceptEdits"),
    );
    assert!(file(&mut after, "fs/write_text_file", 2, &target).is_err());
}

#[test]
fn what_the_core_gives_again_wins_over_what_the_agent_kept() {
    let dir = root();
    let cwd = format!("{dir}/wt");
    let fence = vec![format!("{dir}/prive")];
    let secret = format!("{dir}/prive/secret.txt");
    let d = decl(None, true);
    let mut s = launched(&d, &cwd, &fence, true, "acceptEdits");
    assert!(file(&mut s, "fs/read_text_file", 1, &secret).is_err());
    // Taken off: `fence_keys` too, which the files are judged by.
    call(
        &mut s,
        method::ATTACH,
        attached(&cwd, &[], false, "acceptEdits"),
    );
    assert!(file(&mut s, "fs/read_text_file", 2, &secret).is_ok());
    let target = format!("{cwd}/a.txt");
    assert_eq!(
        permission(&mut s, 3, "edit", &target).as_deref(),
        Some("allow")
    );
    // Set on an agent that kept none.
    call(
        &mut s,
        method::ATTACH,
        attached(&cwd, &fence, true, "acceptEdits"),
    );
    assert!(file(&mut s, "fs/read_text_file", 4, &secret).is_err());
    assert_eq!(
        permission(&mut s, 5, "edit", &target).as_deref(),
        Some("reject")
    );
}

#[test]
fn vibes_agent_keeps_its_fence_after_a_restart() {
    let d = decl(Some("vibe"), false);
    let core = attached("/w", &["/w/prive".to_string()], false, "default");
    let mut s = started_again(&d, &core);
    let bypass = json!({ "agent": AGENT, "value": "bypassPermissions" });
    let written = call(&mut s, method::SET_MODE, &bypass).to_string();
    assert!(written.contains("session/set_mode"), "{written}");
    assert!(!written.contains("auto-approve"), "{written}");
}

#[test]
fn vibes_agent_follows_a_fence_given_again() {
    // `auto-approve` passes over a denylist: fenced, Vibe stays in `ask` (`vibe::agent_of`).
    let d = decl(Some("vibe"), false);
    let mut s = service(&d);
    let launch = LaunchParams {
        agent: AGENT.into(),
        cwd: "/w".into(),
        mode: "default".into(),
        lang: "fr".into(),
        ..LaunchParams::default()
    };
    let mut todo: Vec<String> =
        serde_json::from_value(call(&mut s, method::LAUNCH, launch)["writes"].clone()).unwrap();
    while let Some(line) = todo.pop() {
        let m: Value = serde_json::from_str(&line).unwrap();
        if m["id"].is_null() {
            continue;
        }
        let result = json!({ "protocolVersion": crate::PROTOCOL, "sessionId": "s1" });
        let reply = json!({ "jsonrpc": "2.0", "id": m["id"], "result": result });
        todo.extend(told(&mut s, reply).iter().map(Value::to_string));
    }
    let bypass = json!({ "agent": AGENT, "value": "bypassPermissions" });
    let free = call(&mut s, method::SET_MODE, &bypass).to_string();
    assert!(free.contains("auto-approve"), "{free}");
    let core = attached("/w", &["/w/prive".to_string()], false, "default");
    call(&mut s, method::ATTACH, &core);
    let fenced = call(&mut s, method::SET_MODE, &bypass).to_string();
    assert!(!fenced.contains("auto-approve"), "{fenced}");
}

/// A plugin started anew, its agent's harness still running, as the core gives it again.
fn started_again(d: &HarnessDecl, core: &Attach) -> Service {
    let mut s = service(d);
    call(&mut s, method::ATTACH, core);
    s
}

/// Each harness that holds its fence on its permissions, with no `fs`: nothing else stands behind them.
fn fenced_on_permissions() -> Vec<HarnessDecl> {
    vec![
        decl(Some("cursor"), false),
        decl(Some("gemini"), false),
        decl(Some("vibe"), false),
    ]
}

/// The agent's next message, as its harness is told it.
fn message(s: &mut Service) -> Vec<Value> {
    let sent = call(
        s,
        method::SEND_MESSAGE,
        json!({ "agent": AGENT, "uuid": "u1", "text": "suite" }),
    );
    sent["writes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| serde_json::from_str(l.as_str().unwrap()).unwrap())
        .collect()
}

/// The items of what its harness wrote.
fn items(s: &mut Service, frame: Value) -> Vec<Item> {
    let frames = json!({ "agent": AGENT, "frames": [frame.to_string()] });
    serde_json::from_value(call(s, method::FRAMES, frames)["items"].clone()).unwrap()
}

#[test]
fn a_restarted_agent_goes_on_with_its_session_and_its_fence_holds_through_a_whole_turn() {
    let dir = root();
    let cwd = format!("{dir}/wt");
    let fence = vec![format!("{dir}/prive")];
    let secret = format!("{dir}/prive/secret.txt");
    for d in fenced_on_permissions() {
        let core = attached(&cwd, &fence, false, "default");
        let mut s = started_again(&d, &core);
        // Its message goes to the session it had, at once.
        let sent = message(&mut s);
        assert_eq!(sent.len(), 1, "waits for a session: {:?}", d.acp);
        assert_eq!(sent[0]["method"], "session/prompt");
        assert_eq!(sent[0]["params"]["sessionId"], "s1");
        // What it says of it is read.
        let chunk = json!({ "jsonrpc": "2.0", "method": "session/update", "params": {
            "sessionId": "s1",
            "update": { "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": "ok" } },
        } });
        let said = items(&mut s, chunk);
        assert!(
            said.iter()
                .any(|i| matches!(i, Item::Delta { .. } | Item::Text { .. })),
            "{said:?}"
        );
        // Its fence refuses in the turn, without a card.
        let got = permission(&mut s, 7, "read", &secret);
        assert_eq!(got.as_deref(), Some("reject"), "{:?}", d.acp);
        // And its turn ends.
        let end = json!({ "jsonrpc": "2.0", "id": sent[0]["id"], "result": { "stopReason": "end_turn" } });
        let ended = items(&mut s, end);
        assert!(
            ended.iter().any(|i| matches!(i, Item::Turn { .. })),
            "{ended:?}"
        );
        assert_eq!(message(&mut s).len(), 1, "the next one goes at once");
    }
}

#[test]
fn a_turn_that_ran_when_the_plugin_restarted_ends_by_the_answer_it_cannot_know() {
    for d in harnesses() {
        let core = Attach {
            running: true,
            ..attached("/w", &[], false, "default")
        };
        let mut s = started_again(&d, &core);
        assert!(message(&mut s).is_empty(), "a turn runs: it waits");
        // Another old answer is no end.
        let other = json!({ "jsonrpc": "2.0", "id": 3, "result": {} });
        assert!(items(&mut s, other).is_empty());
        let end = json!({ "jsonrpc": "2.0", "id": 4, "result": { "stopReason": "end_turn" } });
        let ended = items(&mut s, end);
        assert!(
            ended.iter().any(|i| matches!(i, Item::Turn { .. })),
            "{ended:?}"
        );
        let sent = items_written(&ended);
        assert_eq!(sent[0]["method"], "session/prompt", "what waited goes");
        assert_eq!(sent[0]["params"]["sessionId"], "s1");
    }
}

fn items_written(items: &[Item]) -> Vec<Value> {
    items
        .iter()
        .filter_map(|i| match i {
            Item::Write { line } => serde_json::from_str(line).ok(),
            _ => None,
        })
        .collect()
}

#[test]
fn a_permission_that_waited_is_answered_by_its_id_after_a_restart() {
    let d = decl(Some("cursor"), false);
    let core = Attach {
        pending: vec!["acp-abc-1-7".into(), "acp-abc-2-plan".into()],
        ..attached("/w", &[], false, "default")
    };
    let mut s = started_again(&d, &core);
    let answer = json!({ "agent": AGENT, "request": "acp-abc-1-7", "allow": true });
    let lines = call(&mut s, method::ANSWER_PERMISSION, answer);
    let line: Value = serde_json::from_str(lines["writes"][0].as_str().unwrap()).unwrap();
    assert_eq!(line["id"], 7);
    assert_eq!(line["result"]["outcome"]["optionId"], "allow-once");
    // Held by the client, not by its harness.
    let held = json!({ "agent": AGENT, "request": "acp-abc-2-plan", "allow": true });
    assert!(s(method::ANSWER_PERMISSION, held).is_err());
}

#[test]
fn its_requests_are_told_from_those_of_the_plugin_before() {
    let d = decl(Some("cursor"), false);
    let mut s = started_again(&d, &attached("/w", &[], false, "default"));
    let sent = message(&mut s);
    assert!(sent[0]["id"].as_u64().unwrap() > 1 << 20, "{:?}", sent[0]);
}

#[test]
fn a_launch_opens_its_own_session_after_an_attach_that_named_one() {
    let d = decl(None, true);
    let mut s = started_again(&d, &attached("/w", &[], false, "default"));
    let launch = LaunchParams {
        agent: AGENT.into(),
        cwd: "/w".into(),
        mode: "default".into(),
        lang: "fr".into(),
        start: escouade_plugin_protocol::engine::StartAt::Resume {
            session: "s1".into(),
        },
        ..LaunchParams::default()
    };
    let lines = call(&mut s, method::LAUNCH, launch);
    let first: Value = serde_json::from_str(lines["writes"][0].as_str().unwrap()).unwrap();
    assert_eq!(first["method"], "initialize");
    // Its process does not know it yet: a message waits for it.
    assert!(message(&mut s).is_empty());
}

#[test]
fn a_write_in_git_has_a_card_on_each_harness() {
    for d in harnesses() {
        let mut s = launched(&d, "/w/wt", &[], false, "acceptEdits");
        for (id, path) in [
            "/w/wt/.git/hooks/pre-commit",
            "/w/wt/.git/config",
            "/w/wt/.GIT/config",
            "/w/wt/.git",
            "/w/wt/sous/.git/config",
        ]
        .iter()
        .enumerate()
        {
            assert_eq!(
                permission(&mut s, id as u64 + 1, "edit", path),
                None,
                "{path}"
            );
        }
        let got = permission(&mut s, 20, "edit", "/w/wt/.gitignore");
        assert_eq!(got.as_deref(), Some("allow"), "{:?}", d.acp);
    }
}
