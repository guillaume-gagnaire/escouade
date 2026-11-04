//! What the client does of Gemini CLI's ways: its questions, its modes, its plan, its replay, its limit.

use super::*;
use crate::Agent;
use escouade_plugin_protocol::engine::{Attach, HarnessDecl, Item, LaunchParams};

const SESSION: &str = "s1";

fn update(u: Value) -> String {
    json!({ "jsonrpc": "2.0", "method": "session/update", "params": { "sessionId": SESSION, "update": u } })
        .to_string()
}

fn chunk(text: &str) -> String {
    update(
        json!({ "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": text } }),
    )
}

/// The id of the request `method` among the lines the client wrote.
fn id_of(lines: &[String], method: &str) -> Option<u64> {
    lines.iter().find_map(|l| {
        let m: Value = serde_json::from_str(l).ok()?;
        (m["method"] == method).then(|| m["id"].as_u64()).flatten()
    })
}

fn written(items: &[Item]) -> Vec<String> {
    items
        .iter()
        .filter_map(|i| match i {
            Item::Write { line } => Some(line.clone()),
            _ => None,
        })
        .collect()
}

fn decl() -> HarnessDecl {
    serde_json::from_value(json!({
        "id": "gemini", "name": "Gemini CLI", "speaker": "Gemini",
        "acp": { "args": ["--acp"], "fs": false, "fenceOnPermissions": true, "questionsInText": true,
                 "modes": { "default": "default", "plan": "plan", "acceptEdits": "autoEdit", "bypassPermissions": "yolo" },
                 "readOnlyMode": "default", "extensions": ["gemini"] },
    }))
    .unwrap()
}

/// An agent launched as `params` says, its session open.
fn open_with(mode: &str, append_prompt: Option<&str>) -> Agent {
    let attach = Attach {
        agent: "gemini-abc".into(),
        cwd: "C:\\repo".into(),
        mode: mode.into(),
        ..Attach::default()
    };
    let mut a = Agent::new(&attach, &decl(), "0");
    let launch = a
        .launch(&LaunchParams {
            cwd: "C:\\repo".into(),
            mode: mode.into(),
            lang: "fr".into(),
            append_prompt: append_prompt.map(str::to_string),
            ..LaunchParams::default()
        })
        .unwrap();
    let hello = id_of(&launch.writes, "initialize").unwrap();
    let items = a.frames(&[
        json!({ "jsonrpc": "2.0", "id": hello, "result": { "protocolVersion": 1 } }).to_string(),
    ]);
    let new = id_of(&written(&items), "session/new").unwrap();
    a.frames(&[json!({ "jsonrpc": "2.0", "id": new, "result": {
        "sessionId": SESSION, "modes": { "currentModeId": "default" },
    } })
    .to_string()]);
    a
}

fn opened(mode: &str) -> Agent {
    open_with(mode, None)
}

fn prompt_end(id: u64, usage: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "result": { "stopReason": "end_turn", "usage": usage } })
        .to_string()
}

fn turn_of(items: &[Item]) -> Option<&Item> {
    items.iter().find(|i| matches!(i, Item::Turn { .. }))
}

#[test]
fn a_question_is_kept_out_of_the_thread_even_cut_in_pieces_and_its_turn_waits_for_its_answers() {
    let block = r#"<escouade_question>{"questions": [{"question": "Rouge ou bleu ?", "options": ["rouge", "bleu"], "multiSelect": false}]}</escouade_question>"#;
    let text = format!("Il me faut ton choix.\n{block}");
    for size in [1, 3, 7, 12, 200] {
        let mut a = opened("default");
        let prompt = a.send("bonjour");
        let id = id_of(&prompt, "session/prompt").unwrap();
        let chars: Vec<char> = text.chars().collect();
        let mut shown = String::new();
        for piece in chars.chunks(size) {
            for i in a.frames(&[chunk(&piece.iter().collect::<String>())]) {
                if let Item::Delta { text, .. } = i {
                    shown.push_str(&text);
                }
            }
        }
        assert_eq!(shown, "Il me faut ton choix.\n", "pieces of {size}");
        let ended = a.frames(&[prompt_end(
            id,
            json!({ "inputTokens": 10, "cachedReadTokens": 4, "outputTokens": 2, "thoughtTokens": 1 }),
        )]);
        assert!(turn_of(&ended).is_none(), "the turn waits for its answers");
        let request = ended
            .iter()
            .find_map(|i| match i {
                Item::Question { request, .. } => Some(request.clone()),
                _ => None,
            })
            .expect("its card");
        // A message waits for the answers.
        assert!(a.send("et aussi").is_empty());
        let answered = a
            .answer_question(&request, &json!({ "Rouge ou bleu ?": "rouge" }))
            .unwrap();
        assert!(answered[0].contains("Rouge ou bleu ? → rouge"));
        // Its tokens add up, the turn over.
        let id = id_of(&answered, "session/prompt").unwrap();
        let ended = a.frames(&[prompt_end(
            id,
            json!({ "inputTokens": 1, "outputTokens": 1 }),
        )]);
        match turn_of(&ended) {
            Some(Item::Turn { usage, .. }) => {
                assert_eq!((usage.input, usage.cache, usage.output), (7, 4, 4));
            }
            other => panic!("no turn: {other:?}"),
        }
        // The message that waited goes.
        assert!(id_of(&written(&ended), "session/prompt").is_some());
    }
}

#[test]
fn a_text_that_only_looks_like_a_question_is_shown_whole_and_a_block_that_is_none_as_written() {
    let mut a = opened("default");
    let id = id_of(&a.send("x"), "session/prompt").unwrap();
    let mut said = a.frames(&[chunk("a <esc"), chunk("ape> b")]);
    said.extend(a.frames(&[prompt_end(id, Value::Null)]));
    let texts = |items: &[Item]| -> String {
        items
            .iter()
            .filter_map(|i| match i {
                Item::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    };
    assert_eq!(texts(&said), "a <escape> b");
    let id = id_of(&a.send("x"), "session/prompt").unwrap();
    a.frames(&[chunk("<escouade_question>pas du json</escouade_question>")]);
    let ended = a.frames(&[prompt_end(id, Value::Null)]);
    assert!(texts(&ended).contains("pas du json"), "{ended:?}");
    assert!(turn_of(&ended).is_some());
}

#[test]
fn stopping_a_turn_held_for_its_question_ends_it() {
    let mut a = opened("default");
    let id = id_of(&a.send("x"), "session/prompt").unwrap();
    a.frames(&[chunk(
        r#"<escouade_question>{"questions": [{"question": "Oui ?", "options": ["oui"]}]}</escouade_question>"#,
    )]);
    a.frames(&[prompt_end(id, Value::Null)]);
    let lines = a.interrupt();
    let id = id_of(&lines, "session/set_mode").expect("what closes the held turn");
    let ended = a.frames(&[json!({ "jsonrpc": "2.0", "id": id, "result": {} }).to_string()]);
    assert!(matches!(
        turn_of(&ended),
        Some(Item::Turn {
            interrupted: true,
            ..
        })
    ));
}

#[test]
fn its_mode_is_told_as_a_text_and_a_plan_goes_on_in_the_mode_chosen_once_it_left_its_mode_plan() {
    let mut a = opened("plan");
    let id = id_of(&a.send("un plan"), "session/prompt").unwrap();
    let said = a.frames(&[
        chunk("[MODE_UPDATE] plan"),
        json!({ "jsonrpc": "2.0", "id": 77, "method": "session/request_permission", "params": {
            "sessionId": SESSION,
            "toolCall": { "toolCallId": "exit_plan_mode__1", "title": "Requesting plan approval for: C:\\nulle-part\\plan.md" },
            "options": [
                { "kind": "allow_once", "optionId": "proceed_once" },
                { "kind": "allow_always", "optionId": "proceed_always" },
                { "kind": "reject_once", "optionId": "cancel" },
            ],
        } })
        .to_string(),
    ]);
    let (request, plan) = said
        .iter()
        .find_map(|i| match i {
            Item::Permission { request, input, .. } => Some((request.clone(), input.clone())),
            _ => None,
        })
        .expect("its card");
    // The file gone: its name.
    assert_eq!(plan.unwrap()["plan"], "C:\\nulle-part\\plan.md");
    assert!(!said.iter().any(|i| matches!(i, Item::Delta { .. })));
    // Approved for all: `proceed_once`, with its writes accepted.
    let lines = a
        .answer_permission(&request, true, true, None, None)
        .unwrap();
    assert!(lines[0].contains("proceed_once"), "{lines:?}");
    assert!(id_of(&lines, "session/set_mode").is_none(), "not yet");
    let left = a.frames(&[chunk("[MODE_UPDATE] default")]);
    assert!(left
        .iter()
        .any(|i| matches!(i, Item::Mode { mode } if mode == "acceptEdits")));
    let set = written(&left);
    assert!(set.iter().any(|l| l.contains("autoEdit")), "{set:?}");
    // A change of its own afterwards is told, as the core's client told it.
    let told = a.frames(&[chunk("[MODE_UPDATE] plan")]);
    assert!(told
        .iter()
        .any(|i| matches!(i, Item::Mode { mode } if mode == "plan")));
    a.frames(&[prompt_end(id, Value::Null)]);
}

#[test]
fn its_replay_counts_only_while_a_prompt_runs_and_its_limit_is_read_from_its_words() {
    let mut a = opened("default");
    assert!(a.frames(&[chunk("l'historique rejoué")]).is_empty());
    let id = id_of(&a.send("x"), "session/prompt").unwrap();
    let ended = a.frames(&[json!({ "jsonrpc": "2.0", "id": id, "error": {
        "code": 429, "message": "Please retry in 30.5s",
    } })
    .to_string()]);
    match turn_of(&ended) {
        Some(Item::Turn { limited, error, .. }) => {
            assert!(matches!(limited, Some(Some(_))), "{limited:?}");
            assert!(error
                .as_deref()
                .unwrap()
                .contains("Limite d’usage de Gemini"));
        }
        other => panic!("no turn: {other:?}"),
    }
}

#[test]
fn its_instructions_stand_beside_the_rules_of_its_questions() {
    let mut a = open_with("default", Some("Fais le ticket."));
    let line: Value = serde_json::from_str(&a.send("x")[0]).unwrap();
    let first = line["params"]["prompt"][0]["text"].as_str().unwrap();
    assert!(
        first.starts_with("<instructions>\nFais le ticket.\n</instructions>\n\n<escouade>"),
        "{first}"
    );
}

#[test]
fn a_message_waiting_for_a_loaded_session_never_takes_its_replay_for_its_turn() {
    let attach = Attach {
        agent: "gemini-abc".into(),
        cwd: "C:\\repo".into(),
        mode: "default".into(),
        session: Some(SESSION.into()),
        ..Attach::default()
    };
    let mut a = Agent::new(&attach, &decl(), "0");
    let launch = a
        .launch(&LaunchParams {
            cwd: "C:\\repo".into(),
            mode: "default".into(),
            lang: "fr".into(),
            append_prompt: Some("Fais le ticket.".into()),
            start: escouade_plugin_protocol::engine::StartAt::Resume {
                session: SESSION.into(),
            },
            ..LaunchParams::default()
        })
        .unwrap();
    // The message comes before the session is open, as after a restart of the app.
    assert!(a.send("tu te souviens ?").is_empty());
    let hello = id_of(&launch.writes, "initialize").unwrap();
    let items = a.frames(&[
        json!({ "jsonrpc": "2.0", "id": hello, "result": { "protocolVersion": 1 } }).to_string(),
    ]);
    let load = id_of(&written(&items), "session/load").expect("asked whatever its capabilities");
    let block = r#"<escouade_question>{"questions": [{"question": "Oui ?", "options": ["oui"]}]}</escouade_question>"#;
    // Its answer, then the rest of its history in the same batch.
    let opened = a.frames(&[
        json!({ "jsonrpc": "2.0", "id": load, "result": { "modes": { "currentModeId": "default" } } })
            .to_string(),
        chunk(block),
    ]);
    let lines = written(&opened);
    assert!(id_of(&lines, "session/prompt").is_none(), "{lines:?}");
    let settled = id_of(&lines, "session/set_mode").expect("what tells its replay is behind");
    let more = a.frames(&[
        chunk(block),
        json!({ "jsonrpc": "2.0", "id": settled, "result": {} }).to_string(),
    ]);
    assert!(
        !more.iter().any(|i| matches!(
            i,
            Item::Delta { .. } | Item::Text { .. } | Item::Question { .. }
        )),
        "{more:?}"
    );
    let lines = written(&more);
    let prompt: Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!(prompt["method"], "session/prompt");
    // Its session has its instructions already.
    assert_eq!(prompt["params"]["prompt"].as_array().unwrap().len(), 1);
}
