use super::*;

/// The transcripts a real interface wrote (`agy` 1.4.9, ESC-272): a command allowed in the
/// interface then the conversation resumed for a question about it; a question of its own
/// answered, then a turn stopped by Escape.
const COMMAND: &str =
    include_str!("../../../tests/fixtures/agy/tui-transcript-command-resumed.jsonl");
const QUESTION: &str =
    include_str!("../../../tests/fixtures/agy/tui-transcript-question-interrupted.jsonl");
/// What its hooks read on their input, the event added (`escouade-hook <event>`).
const HOOKS: &str = include_str!("../../../tests/fixtures/agy/tui-hook-payloads.jsonl");

fn agent(session: Option<&str>) -> AgentRt {
    let dir = std::env::temp_dir().join(format!("escouade-agy-tui-{}", new_id()));
    let meta = AgentMeta {
        id: "a1".into(),
        engine: EngineId::AGY,
        cwd: "C:\\tmp\\esc272\\w1".into(),
        session_id: session.map(str::to_string),
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

/// Each step of `text` read, the ends of turns and the alerts it gave.
fn read(a: &mut AgentRt, text: &str) -> (Vec<TurnEnd>, Vec<String>) {
    let mut fx = Effects::default();
    let (mut ends, mut alerts) = (Vec::new(), Vec::new());
    for e in lines(text) {
        a.agy_log(&e, &mut fx);
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

fn texts(a: &mut AgentRt) -> Vec<String> {
    of_kind(a, "text")
        .iter()
        .map(|t| t["text"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn a_real_turn_shows_the_message_its_command_and_its_reply_and_the_resumed_one_goes_on() {
    let mut a = agent(Some("e117fa75-c9ef-4892-a1af-e702b8bcc9d8"));
    let (ends, _) = read(&mut a, COMMAND);
    let users: Vec<String> = of_kind(&mut a, "user")
        .iter()
        .map(|u| u["text"].as_str().unwrap().to_string())
        .collect();
    // Out of its `<USER_REQUEST>` and metadata.
    assert_eq!(
        users,
        [
            "Lance la commande echo hi8 avec run_command, puis reponds ok.",
            "Quelle commande as-tu lancee ? Reponds en trois mots."
        ]
    );
    assert!(of_kind(&mut a, "user")
        .iter()
        .all(|u| u["origin"] == "terminal"));
    let tools = of_kind(&mut a, "tool");
    assert_eq!(tools.len(), 1, "{tools:?}");
    assert_eq!(tools[0]["name"], "Bash");
    assert_eq!(tools[0]["input"]["command"], "echo hi8");
    assert_eq!(tools[0]["status"], "ok", "{:?}", tools[0]);
    // The system's message of the resumed conversation is not shown.
    assert_eq!(texts(&mut a), ["ok", "echo hi8"]);
    assert!(
        matches!(&ends[..], [TurnEnd::Finished(t), TurnEnd::Finished(u)] if t == "ok" && u == "echo hi8"),
        "{ends:?}"
    );
    assert_eq!(a.meta.status, AgentStatus::Done);
}

#[test]
fn its_question_waits_for_the_user_and_a_turn_stopped_by_escape_ends_with_what_it_wrote() {
    let mut a = agent(Some("434725b7-c90f-46a3-8a2b-d41956239965"));
    let all = lines(QUESTION);
    let mut fx = Effects::default();
    // The message and the call of its question: it waits for the answer in its interface.
    for e in &all[..2] {
        a.agy_log(e, &mut fx);
    }
    assert_eq!(a.meta.status, AgentStatus::Waiting);
    assert!(
        fx.notify
            .as_ref()
            .is_some_and(|n| n.body.contains("Bleu ou rouge")),
        "{:?}",
        fx.notify.as_ref().map(|n| &n.body)
    );
    // Answered.
    a.agy_log(&all[2], &mut fx);
    assert_eq!(a.meta.status, AgentStatus::Running);
    let rest: String = all[3..].iter().map(|v| v.to_string() + "\n").collect();
    let (ends, _) = read(&mut a, &rest);
    assert_eq!(ends.len(), 2, "{ends:?}");
    let t = texts(&mut a);
    assert_eq!(t[0], "Ta réponse est : **Bleu** (l'option recommandée).");
    // Escape: the verses written so far, the end of its turn.
    assert!(t[1].starts_with("Vaste miroir"), "{t:?}");
    assert_eq!(a.meta.status, AgentStatus::Done);
}

#[test]
fn a_step_that_failed_shows_its_tool_in_error() {
    let mut a = agent(Some("ff1f5598-da9e-4bb6-b9dc-bdd1f9622b73"));
    let text = [
        json!({"step_index": 0, "source": "USER_EXPLICIT", "type": "USER_INPUT", "status": "DONE", "content": "<USER_REQUEST>\nLance echo\n</USER_REQUEST>"}),
        json!({"step_index": 1, "source": "MODEL", "type": "PLANNER_RESPONSE", "status": "DONE", "tool_calls": [{"name": "run_command", "args": {"CommandLine": "echo hi7"}}]}),
        json!({"step_index": 2, "source": "MODEL", "type": "GENERIC", "status": "ERROR", "error": "JSON hook failed"}),
    ]
    .iter()
    .map(|v| v.to_string() + "\n")
    .collect::<String>();
    read(&mut a, &text);
    let tools = of_kind(&mut a, "tool");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["status"], "error", "{:?}", tools[0]);
    assert_eq!(a.meta.status, AgentStatus::Running);
}

#[test]
fn its_first_hook_gives_a_new_conversation_and_its_transcript_and_stop_says_the_turn_may_be_over() {
    let hooks = lines(HOOKS);
    let with_event = |h: &Value| {
        let mut h = h.clone();
        h[crate::tui::EVENT_KEY] = h["event"].clone();
        h
    };
    let start = hooks.iter().find(|h| h["event"] == "SessionStart").unwrap();
    // Its interface started with no conversation: the one it opened is the agent's.
    let mut a = agent(None);
    let mut fx = Effects::default();
    let out = a.agy_hook(&with_event(start), &mut fx);
    assert_eq!(
        a.meta.session_id.as_deref(),
        Some("434725b7-c90f-46a3-8a2b-d41956239965")
    );
    assert!(out
        .transcript
        .as_deref()
        .is_some_and(|t| t.ends_with("transcript_full.jsonl")));
    assert!(fx.save);
    // The same conversation: nothing more.
    let pre = hooks.iter().find(|h| h["event"] == "PreToolUse").unwrap();
    let out = a.agy_hook(&with_event(pre), &mut fx);
    assert!(out.transcript.is_none());
    assert!(a.meta.status.is_active());
    let stop = hooks.iter().find(|h| h["event"] == "Stop").unwrap();
    let out = a.agy_hook(&with_event(stop), &mut fx);
    assert!(out.stopped);
    assert!(!a.interrupted);
    // Another conversation than the agent's (`/clear`): said, and read from its start.
    let mut b = agent(Some("e117fa75-c9ef-4892-a1af-e702b8bcc9d8"));
    let mut fx = Effects::default();
    let out = b.agy_hook(&with_event(start), &mut fx);
    assert!(out.transcript.is_some());
    assert!(of_kind(&mut b, "notice").iter().any(|n| n["text"]
        .as_str()
        .unwrap_or_default()
        .contains("Antigravity")));
}

/// A file written from a real interface (`write_file`, allowed there): its diff, done.
const WRITE: &str = include_str!("../../../tests/fixtures/agy/tui-transcript-write.jsonl");

#[test]
fn a_file_written_in_the_interface_shows_its_diff_and_is_done() {
    let mut a = agent(Some("87315736-6142-44ce-a2c6-dd0da72bd345"));
    let (ends, _) = read(&mut a, WRITE);
    let tools = of_kind(&mut a, "tool");
    assert_eq!(tools.len(), 1, "{tools:?}");
    let t = &tools[0];
    assert_eq!(t["status"], "ok", "{t:?}");
    assert!(t.to_string().contains("vu du terminal"), "{t:?}");
    assert!(!t.to_string().contains("rien écrit"), "{t:?}");
    assert_eq!(ends.len(), 1);
}
