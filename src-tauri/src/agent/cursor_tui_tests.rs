use super::*;

/// The transcript of a real interface's turn, stopped by Ctrl+C (ESC-24).
const TUI: &str = include_str!("../../../tests/fixtures/cursor/transcript-tui.jsonl");
/// The transcript of a real one-shot (`agent -p`), its tools written (ESC-24).
const PRINT: &str = include_str!("../../../tests/fixtures/cursor/transcript-print.jsonl");

fn agent() -> AgentRt {
    let dir = std::env::temp_dir().join(format!("escouade-cursor-tui-{}", new_id()));
    let meta = AgentMeta {
        id: "a1".into(),
        engine: EngineId::CURSOR,
        cwd: "C:\\Users\\user\\AppData\\Local\\Temp\\esc24\\wt-agent1".into(),
        session_id: Some("6753be15-7845-4e8d-abf9-3c3ed0985561".into()),
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

fn read(a: &mut AgentRt, text: &str) -> Vec<TurnEnd> {
    let mut fx = Effects::default();
    let mut ends = Vec::new();
    for e in lines(text) {
        a.cursor_log(&e, &mut fx);
        ends.extend(fx.turn_end.take());
    }
    ends
}

fn of_kind(a: &mut AgentRt, kind: &str) -> Vec<Value> {
    a.conv
        .items()
        .into_iter()
        .filter(|i| i["kind"] == kind)
        .collect()
}

#[test]
fn the_users_text_is_taken_out_of_what_the_interface_wraps_it_in() {
    assert_eq!(
        typed_text("<timestamp>Sunday, Oct 25, 2026, 11:29 AM (UTC+1)</timestamp>\n<user_query>\nRéponds seulement : OK VUE\n</user_query>"),
        "Réponds seulement : OK VUE"
    );
    // A command typed (ESC-199, tried for real): the interface puts the command's file first, then the query.
    assert_eq!(
        typed_text(
            "<cursor_commands>

--- Cursor Command: etude ---
Write it.
--- End Command ---
</cursor_commands>
<timestamp>Tuesday, Oct 27, 2026, 12:29 AM (UTC+1)</timestamp>
<user_query>
/etude Fais ça.
</user_query>"
        ),
        "/etude Fais ça."
    );
    assert_eq!(typed_text("  plain  "), "plain");
    assert_eq!(
        typed_text("<user_query>\nDeux\nlignes\n</user_query>"),
        "Deux\nlignes"
    );
}

#[test]
fn a_real_turn_stopped_by_ctrl_c_ends_as_interrupted_with_what_it_said() {
    let mut a = agent();
    let ends = read(&mut a, TUI);
    let users: Vec<Value> = of_kind(&mut a, "user");
    assert_eq!(users.len(), 1);
    assert_eq!(users[0]["text"], "Réponds seulement : OK VUE");
    assert_eq!(users[0]["origin"], "terminal");
    let texts: Vec<String> = of_kind(&mut a, "text")
        .iter()
        .map(|t| t["text"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(texts, ["OK VUE\nBANANE", "OK CURSOR\nBANANE"]);
    assert!(of_kind(&mut a, "text")
        .iter()
        .all(|t| t["streaming"] == false));
    assert_eq!(ends, [TurnEnd::Interrupted]);
    let turns = of_kind(&mut a, "turn");
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0]["interrupted"], true);
    assert_eq!(a.meta.prompts, 1);
    assert!(!a.meta.status.is_active());
}

#[test]
fn a_turn_with_tools_shows_them_done_under_claude_codes_names_and_ends_finished() {
    let mut a = agent();
    let ends = read(&mut a, PRINT);
    let tools = of_kind(&mut a, "tool");
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Glob", "Read", "Read"]);
    assert_eq!(tools[0]["input"]["pattern"], "**/README*");
    assert_eq!(
        tools[2]["input"]["file_path"],
        "C:\\Users\\user\\AppData\\Local\\Temp\\esc24\\wt-agent1\\README.md"
    );
    // Their results are never written: done, nothing to show.
    assert!(tools.iter().all(|t| t["status"] == "ok"));
    assert!(matches!(&ends[..], [TurnEnd::Finished(t)] if t.contains("Essai")));
    assert_eq!(a.meta.status, AgentStatus::Done);
}

#[test]
fn a_turn_ended_in_error_says_why_and_a_turn_end_out_of_a_turn_is_nothing() {
    let mut a = agent();
    let mut fx = Effects::default();
    // Nothing runs: an end alone tells nothing.
    a.cursor_log(
        &json!({ "type": "turn_ended", "status": "success" }),
        &mut fx,
    );
    assert!(fx.turn_end.take().is_none());
    let user = json!({ "role": "user", "message": { "content": [{ "type": "text", "text": "<user_query>\nVas-y\n</user_query>" }] } });
    a.cursor_log(&user, &mut fx);
    assert_eq!(a.meta.status, AgentStatus::Running);
    a.cursor_log(
        &json!({ "type": "turn_ended", "status": "error", "error": "Network down" }),
        &mut fx,
    );
    assert_eq!(
        fx.turn_end.take(),
        Some(TurnEnd::Error("Network down".into()))
    );
    assert_eq!(a.meta.status, AgentStatus::Error);
}

#[test]
fn the_interface_gone_mid_turn_ends_it_as_interrupted() {
    let mut a = agent();
    let mut fx = Effects::default();
    let user =
        json!({ "role": "user", "message": { "content": [{ "type": "text", "text": "lent" }] } });
    a.cursor_log(&user, &mut fx);
    a.stop_tui(&mut fx);
    assert_eq!(fx.turn_end.take(), Some(TurnEnd::Interrupted));
    assert!(!a.meta.status.is_active());
}

#[test]
fn the_tools_of_its_interface_take_claude_codes_names() {
    let (n, i) = tui_tool(
        "Shell",
        &json!({ "command": "npm test", "description": "Run" }),
    );
    assert_eq!((n.as_str(), &i["command"]), ("Bash", &json!("npm test")));
    let (n, i) = tui_tool("StrReplace", &json!({ "path": "a.rs", "old_string": "x" }));
    assert_eq!((n.as_str(), &i["file_path"]), ("Edit", &json!("a.rs")));
    let (n, i) = tui_tool("Grep", &json!({ "pattern": "fn", "path": "src" }));
    assert_eq!(
        (n.as_str(), &i["pattern"], &i["path"]),
        ("Grep", &json!("fn"), &json!("src"))
    );
    let (n, i) = tui_tool("GetDynamicTools", &json!({ "namespace": "cursor" }));
    assert_eq!(
        (n.as_str(), &i["namespace"]),
        ("GetDynamicTools", &json!("cursor"))
    );
}
