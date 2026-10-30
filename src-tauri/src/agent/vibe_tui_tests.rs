use super::*;

/// The segments of the journal a real interface wrote (Mistral Vibe 3.0.3, ESC-268): a turn with a
/// command of its list, a turn whose command asked its permission (« Allow once » in the
/// interface), a turn stopped by Escape.
const TOOL: &str = include_str!("../../../tests/fixtures/vibe/tui-journal-tool.jsonl");
const PERMISSION: &str = include_str!("../../../tests/fixtures/vibe/tui-journal-permission.jsonl");
const INTERRUPTED: &str =
    include_str!("../../../tests/fixtures/vibe/tui-journal-interrupted.jsonl");

const SESSION: &str = "9a9345e7-3db5-2873-705d-f50226790eba";

fn agent() -> AgentRt {
    let dir = std::env::temp_dir().join(format!("escouade-vibe-tui-{}", new_id()));
    let meta = AgentMeta {
        id: "a1".into(),
        engine: EngineId::VIBE,
        cwd: "C:\\x\\repo".into(),
        session_id: Some(SESSION.into()),
        ..Default::default()
    };
    let mut a = AgentRt::new(meta, &dir);
    a.start_tui();
    a
}

fn lines(text: &str) -> Vec<Value> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

/// Each record of `text` read, the ends of turns and the alerts it gave.
fn read(a: &mut AgentRt, text: &str) -> (Vec<TurnEnd>, Vec<String>) {
    let mut fx = Effects::default();
    let (mut ends, mut alerts) = (Vec::new(), Vec::new());
    for e in lines(text) {
        a.vibe_log(&e, &mut fx);
        ends.extend(fx.turn_end.take());
        if let Some(n) = fx.notify.take() {
            alerts.push(n.body);
        }
    }
    (ends, alerts)
}

fn of_kind(a: &mut AgentRt, kind: &str) -> Vec<Value> {
    a.conv
        .items()
        .into_iter()
        .filter(|i| i["kind"] == kind)
        .collect()
}

#[test]
fn a_real_turn_shows_the_message_its_command_and_its_reply_then_ends() {
    let mut a = agent();
    let (ends, alerts) = read(&mut a, TOOL);
    let users = of_kind(&mut a, "user");
    assert_eq!(users.len(), 1);
    assert_eq!(
        users[0]["text"],
        "Use your bash tool to run: echo kiwi. Then reply with the single word: done"
    );
    assert_eq!(users[0]["origin"], "terminal");
    let tools = of_kind(&mut a, "tool");
    assert_eq!(tools.len(), 1, "{tools:?}");
    assert_eq!(tools[0]["name"], "Bash");
    assert_eq!(tools[0]["input"]["command"], "echo kiwi");
    assert_eq!(tools[0]["status"], "ok");
    let texts: Vec<String> = of_kind(&mut a, "text")
        .iter()
        .map(|t| t["text"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(texts, ["done"]);
    assert!(
        matches!(&ends[..], [TurnEnd::Finished(t)] if t == "done"),
        "{ends:?}"
    );
    // Its end alone is told: no permission nor question waited.
    assert_eq!(alerts, ["done"]);
    assert_eq!(a.meta.status, AgentStatus::Done);
    assert_eq!(a.meta.prompts, 1);
    // Its reasoning and its hooks' notices are not shown.
    assert_eq!(a.conv.items().len(), 4, "{:#?}", a.conv.items());
}

#[test]
fn a_permission_asked_in_the_interface_makes_the_agent_wait_until_it_is_answered() {
    let mut a = agent();
    let records = lines(PERMISSION);
    let mut fx = Effects::default();
    let mut waited = None;
    for e in &records {
        a.vibe_log(e, &mut fx);
        if e["type"] == "callback_registered" {
            assert_eq!(a.meta.status, AgentStatus::Waiting);
            waited = fx.notify.take().map(|n| n.body);
        }
        if e["type"] == "callback_resolved" {
            assert_eq!(a.meta.status, AgentStatus::Running);
        }
    }
    let body = waited.expect("the card says it waits");
    assert!(body.contains("node --version"), "{body}");
    assert_eq!(a.meta.status, AgentStatus::Done);
    let tools = of_kind(&mut a, "tool");
    assert_eq!(tools[0]["input"]["command"], "node --version");
    assert_eq!(tools[0]["status"], "ok");
    assert!(of_kind(&mut a, "text")
        .iter()
        .any(|t| t["text"] == "v24.14.0"));
}

#[test]
fn a_turn_stopped_by_escape_ends_as_interrupted() {
    let mut a = agent();
    let (ends, _) = read(&mut a, INTERRUPTED);
    assert_eq!(ends, [TurnEnd::Interrupted]);
    assert_eq!(of_kind(&mut a, "user").len(), 1);
    // Its reasoning, discarded by the interruption, is not shown.
    assert!(of_kind(&mut a, "text").is_empty());
    assert!(!a.meta.status.is_active());
}

#[test]
fn the_segments_read_one_after_the_other_make_one_conversation_read_once() {
    let mut a = agent();
    read(&mut a, TOOL);
    read(&mut a, PERMISSION);
    // The same records again (a segment read twice): nothing twice.
    read(&mut a, PERMISSION);
    let (ends, _) = read(&mut a, INTERRUPTED);
    assert_eq!(ends, [TurnEnd::Interrupted]);
    assert_eq!(of_kind(&mut a, "user").len(), 3);
    assert_eq!(of_kind(&mut a, "tool").len(), 2);
    assert_eq!(of_kind(&mut a, "turn").len(), 3);
    assert_eq!(a.meta.prompts, 3);
}

#[test]
fn a_failed_turn_says_its_error_and_an_end_out_of_a_turn_is_nothing() {
    let mut a = agent();
    let mut fx = Effects::default();
    let envelope = |status: &str| {
        json!({ "type": "projection_delta", "payload": { "delta": [{ "op": "set_envelope", "state": {
            "activeCallbacks": [],
            "latestTurn": { "id": "t1", "status": status, "error": { "message": "Rate limit exceeded" } },
            "session": { "status": { "type": if status == "in_progress" { "running" } else { "idle" } } },
        } }] } })
    };
    a.vibe_log(&envelope("completed"), &mut fx);
    assert!(fx.turn_end.take().is_none());
    a.vibe_log(&envelope("in_progress"), &mut fx);
    assert_eq!(a.meta.status, AgentStatus::Running);
    a.vibe_log(&envelope("failed"), &mut fx);
    assert_eq!(
        fx.turn_end.take(),
        Some(TurnEnd::Error("Rate limit exceeded".into()))
    );
}

#[test]
fn a_question_waits_on_its_text() {
    let mut a = agent();
    let mut fx = Effects::default();
    let question = json!({ "type": "callback_registered", "payload": { "routing": {
        "type": "callback", "title": "Answer the question", "state": { "status": "open" },
        "detail": { "kind": "user_input", "request": { "questions": [{ "question": "Which color?" }] } },
    } } });
    a.vibe_log(&question, &mut fx);
    assert_eq!(a.meta.status, AgentStatus::Waiting);
    assert_eq!(fx.notify.take().unwrap().body, "Which color?");
}

#[test]
fn the_tools_of_its_journal_take_claude_codes_names() {
    let cwd = "C:\\x\\repo";
    let (n, i) = journal_tool("file_system.bash", &json!({ "command": "ls" }), cwd).unwrap();
    assert_eq!((n.as_str(), &i["command"]), ("Bash", &json!("ls")));
    let (n, i) = journal_tool("read_file", &json!({ "file_path": "a.txt" }), cwd).unwrap();
    assert_eq!(n, "Read");
    assert!(i["file_path"].as_str().unwrap().ends_with("a.txt"));
    let (n, i) = journal_tool(
        "write_file",
        &json!({ "filePath": "b.txt", "content": "x" }),
        cwd,
    )
    .unwrap();
    assert_eq!(n, "Write");
    assert!(i["file_path"].as_str().unwrap().ends_with("b.txt"));
    let (n, _) = journal_tool("mcp_escouade.get_board", &json!({}), cwd).unwrap();
    assert_eq!(n, "mcp__escouade__get_board");
    let (n, i) = journal_tool("task", &json!({ "task": "look", "agent": "explore" }), cwd).unwrap();
    assert_eq!((n.as_str(), &i["prompt"]), ("Task", &json!("look")));
    for unshown in ["ui.ask_user_question", "todo", "search_tool_functions"] {
        assert!(
            journal_tool(unshown, &json!({}), cwd).is_none(),
            "{unshown}"
        );
    }
}

#[test]
fn a_hook_of_a_new_session_makes_it_the_agents_and_reads_its_journal_from_its_start() {
    let home = crate::paths::test_dir("esc268-vibe-hook");
    let dir = crate::engine::vibe::terminal::session_dir(&home, "s-new");
    std::fs::create_dir_all(dir.join("journal")).unwrap();
    std::fs::write(dir.join("journal").join("0000000000000001.jsonl"), "").unwrap();
    std::fs::write(dir.join("journal").join("0000000000000014.jsonl"), "").unwrap();
    let mut a = agent();
    a.meta.session_id = None;
    let mut fx = Effects::default();
    let hook = json!({
        "hook_event_name": "pre_tool", "session_id": "s-new", "parent_session_id": null,
        "transcript_path": dir.to_string_lossy(), "cwd": "C:\\x\\repo", "tool_name": "bash",
    });
    let out = a.vibe_hook(&hook, &mut fx);
    assert_eq!(a.meta.session_id.as_deref(), Some("s-new"));
    assert_eq!(
        out.transcript.map(std::path::PathBuf::from),
        Some(dir.join("journal").join("0000000000000014.jsonl"))
    );
    // Its own session again, or a subagent's: nothing more.
    assert!(a.vibe_hook(&hook, &mut fx).transcript.is_none());
    let child = json!({ "session_id": "child-1", "parent_session_id": "s-new" });
    assert!(a.vibe_hook(&child, &mut fx).transcript.is_none());
    assert_eq!(a.meta.session_id.as_deref(), Some("s-new"));
}
