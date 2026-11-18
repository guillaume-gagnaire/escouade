//! ESC-491: what the client does for any harness an ACP manifest describes, with no extension: a session it does not
//! take back (said, its instructions given again with what was done), the arguments a manifest resumes one with, the
//! turns it begins by itself, the effort by its option of reasoning, a model whose value is made of parts, what its
//! context holds.

use super::*;

const AGENT: &str = "agent-491";

fn decl(acp: Value) -> HarnessDecl {
    let mut acp = acp;
    acp["args"] = json!(["--mode", "acp"]);
    serde_json::from_value(json!({ "id": "h", "name": "Harness", "acp": acp })).unwrap()
}

fn attach() -> Attach {
    Attach {
        agent: AGENT.into(),
        cwd: "/w".into(),
        lang: "fr".into(),
        ..Attach::default()
    }
}

fn params(start: StartAt) -> LaunchParams {
    LaunchParams {
        agent: AGENT.into(),
        cwd: "/w".into(),
        mode: "bypassPermissions".into(),
        effort: "high".into(),
        lang: "fr".into(),
        append_prompt: Some("Le protocole du ticket ESC-1.".into()),
        start,
        ..LaunchParams::default()
    }
}

fn resume(session: &str) -> StartAt {
    StartAt::Resume {
        session: session.into(),
    }
}

/// The lines among `items`, as JSON.
fn lines(items: &[Item]) -> Vec<Value> {
    items
        .iter()
        .filter_map(|i| match i {
            Item::Write { line } => serde_json::from_str(line).ok(),
            _ => None,
        })
        .collect()
}

fn heard(a: &mut Agent, msg: Value) -> Vec<Item> {
    a.frames(&[msg.to_string()])
}

/// The agent launched and answered `initialize` with `caps`: what it does next.
fn initialized(d: &HarnessDecl, start: StartAt, caps: Value) -> (Agent, Vec<Item>) {
    let mut a = Agent::new(&attach(), d, "2.2.3");
    a.launch(&LaunchParams {
        recap: Some("# Reprise\n\nLe test passe.".into()),
        ..params(start)
    })
    .unwrap();
    let items = heard(
        &mut a,
        json!({ "jsonrpc": "2.0", "id": 1, "result": { "protocolVersion": 1, "agentCapabilities": caps } }),
    );
    (a, items)
}

fn notices(items: &[Item]) -> Vec<String> {
    items
        .iter()
        .filter_map(|i| match i {
            Item::Notice { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// The session `id` opened as the answer to the request `n`, with its `result`.
fn opened(a: &mut Agent, n: u64, id: &str, result: Value) -> Vec<Item> {
    let mut result = result;
    result["sessionId"] = json!(id);
    heard(a, json!({ "jsonrpc": "2.0", "id": n, "result": result }))
}

/// The prompt a message makes, as its blocks' texts.
fn prompt_texts(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|l| l["method"] == "session/prompt")
        .flat_map(|l| {
            l["params"]["prompt"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .filter_map(|b| b["text"].as_str().map(str::to_string))
        .collect()
}

#[test]
fn a_harness_that_loads_no_session_starts_a_new_one_said_with_its_instructions_and_what_was_done() {
    let d = decl(json!({}));
    let (mut a, items) = initialized(&d, resume("s-old"), json!({ "loadSession": false }));
    let said = notices(&items);
    assert_eq!(said.len(), 1, "{items:?}");
    assert!(
        said[0]
            .starts_with("Harness ne reprend pas sa session : l’agent repart d’une session neuve.")
            && said[0].contains("Ses consignes lui sont redonnées")
            && said[0].contains("un résumé de ce qu’il a fait"),
        "{said:?}"
    );
    let asked = lines(&items);
    assert_eq!(asked[0]["method"], "session/new", "{asked:?}");
    opened(&mut a, 2, "s-new", json!({}));
    let texts = prompt_texts(&a.send("Continue."));
    assert_eq!(texts.len(), 3, "{texts:?}");
    assert!(texts[0].starts_with("<instructions>") && texts[0].contains("ESC-1"));
    assert!(texts[1].starts_with("# Reprise"));
    assert_eq!(texts[2], "Continue.");
    // Once: the next message goes alone.
    let n = a.next;
    heard(
        &mut a,
        json!({ "jsonrpc": "2.0", "id": n, "result": { "stopReason": "end_turn" } }),
    );
    assert_eq!(
        prompt_texts(&a.send("Encore.")),
        vec!["Encore.".to_string()]
    );
}

#[test]
fn a_session_it_cannot_find_gives_a_new_one_with_its_instructions_again() {
    let d = decl(json!({}));
    let (mut a, items) = initialized(&d, resume("s-old"), json!({ "loadSession": true }));
    assert!(notices(&items).is_empty());
    assert_eq!(lines(&items)[0]["method"], "session/load");
    let items = heard(
        &mut a,
        json!({ "jsonrpc": "2.0", "id": 2, "error": { "code": -32002, "message": "no such session" } }),
    );
    let said = notices(&items);
    assert!(
        said[0].starts_with("La session Harness n’a pas pu être reprise")
            && said[0].contains("Ses consignes lui sont redonnées"),
        "{said:?}"
    );
    assert_eq!(lines(&items)[0]["method"], "session/new");
    opened(&mut a, 3, "s-new", json!({}));
    let texts = prompt_texts(&a.send("Continue."));
    assert!(
        texts[0].contains("ESC-1") && texts[1].starts_with("# Reprise"),
        "{texts:?}"
    );
}

#[test]
fn a_session_loaded_keeps_what_it_had_and_is_given_nothing_again() {
    let d = decl(json!({}));
    let (mut a, items) = initialized(&d, resume("s-old"), json!({ "loadSession": true }));
    assert_eq!(lines(&items)[0]["params"]["sessionId"], "s-old");
    let items = opened(&mut a, 2, "s-old", json!({}));
    assert!(notices(&items).is_empty());
    assert_eq!(
        prompt_texts(&a.send("Continue.")),
        vec!["Continue.".to_string()]
    );
}

#[test]
fn a_new_agent_has_its_instructions_but_no_summary() {
    let d = decl(json!({}));
    let (mut a, items) = initialized(&d, StartAt::New, json!({ "loadSession": false }));
    assert!(
        notices(&items).is_empty(),
        "nothing to say of a first launch"
    );
    opened(&mut a, 2, "s1", json!({}));
    let texts = prompt_texts(&a.send("Bonjour."));
    assert_eq!(texts.len(), 2, "{texts:?}");
    assert!(texts[0].contains("ESC-1"));
}

#[test]
fn the_manifest_resumes_a_session_by_its_arguments_and_the_id_it_keeps_it_under() {
    let d = decl(json!({
        "resume": { "args": ["--resume", "{session}"], "session": "/_meta/h/saved/sessionId" },
    }));
    let mut a = Agent::new(&attach(), &d, "2.2.3");
    // A first launch: no argument of more.
    let launch = a.launch(&params(StartAt::New)).unwrap();
    assert_eq!(launch.args, vec!["--mode", "acp"]);
    heard(
        &mut a,
        json!({ "jsonrpc": "2.0", "id": 1, "result": { "protocolVersion": 1, "agentCapabilities": { "loadSession": true } } }),
    );
    let items = opened(
        &mut a,
        2,
        "live-1",
        json!({ "_meta": { "h": { "saved": { "sessionId": "kept-1" } } } }),
    );
    // The core keeps the id it is saved under; the client speaks to the live one.
    assert!(
        items
            .iter()
            .any(|i| matches!(i, Item::Session { session } if session == "kept-1")),
        "{items:?}"
    );
    let sent: Vec<Value> = a
        .send("Bonjour.")
        .iter()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(sent[0]["params"]["sessionId"], "live-1");
    let items = heard(
        &mut a,
        json!({ "jsonrpc": "2.0", "method": "session/update", "params": { "sessionId": "live-1",
            "update": { "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": "Salut" } } } }),
    );
    assert!(
        items
            .iter()
            .any(|i| matches!(i, Item::Start { session: Some(s) } if s == "kept-1")),
        "{items:?}"
    );
    // Its resume: the arguments with that id, then `session/load` of it.
    let mut b = Agent::new(&attach(), &d, "2.2.3");
    let launch = b.launch(&params(resume("kept-1"))).unwrap();
    assert_eq!(launch.args, vec!["--mode", "acp", "--resume", "kept-1"]);
    let items = heard(
        &mut b,
        json!({ "jsonrpc": "2.0", "id": 1, "result": { "protocolVersion": 1, "agentCapabilities": { "loadSession": true } } }),
    );
    assert_eq!(lines(&items)[0]["method"], "session/load");
    assert_eq!(lines(&items)[0]["params"]["sessionId"], "kept-1");
    // An id that could be another argument is never put on its command line.
    for bad in ["--exec=x", "a b", "a;b", ""] {
        let mut c = Agent::new(&attach(), &d, "2.2.3");
        assert_eq!(
            c.launch(&params(resume(bad))).unwrap().args,
            vec!["--mode", "acp"],
            "{bad}"
        );
    }
}

/// Its turn of its own: begun by what it says with no prompt under way, ended by the marks of its manifest.
fn update(text: &str, meta: Value) -> Value {
    json!({ "jsonrpc": "2.0", "method": "session/update", "params": { "sessionId": "s1",
        "update": { "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": text }, "_meta": meta } } })
}

fn info(meta: Value) -> Value {
    json!({ "jsonrpc": "2.0", "method": "session/update", "params": { "sessionId": "s1",
        "update": { "sessionUpdate": "session_info_update", "_meta": meta } } })
}

fn idle(d: &HarnessDecl) -> Agent {
    let (mut a, _) = initialized(d, StartAt::New, json!({}));
    opened(&mut a, 2, "s1", json!({}));
    let n = a.next;
    let _ = a.send("Bonjour.");
    heard(
        &mut a,
        json!({ "jsonrpc": "2.0", "id": n + 1, "result": { "stopReason": "end_turn" } }),
    );
    a
}

fn turns(items: &[Item]) -> Vec<(bool, Option<String>)> {
    items
        .iter()
        .filter_map(|i| match i {
            Item::Turn {
                interrupted, error, ..
            } => Some((*interrupted, error.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn a_turn_the_harness_begins_by_itself_is_a_turn_ended_by_its_marks() {
    let d = decl(json!({ "turnEnd": [
        { "/_meta/h/phase": "terminalQuiescence" },
        { "/_meta/h/phase": "responseBoundary", "/_meta/h/terminalQuiescenceExpected": false },
    ] }));
    let mut a = idle(&d);
    let items = heard(&mut a, update("Réveillé.", json!({})));
    assert!(
        matches!(items.first(), Some(Item::Start { session: Some(s) }) if s == "s1"),
        "the agent at work: {items:?}"
    );
    assert!(turns(&items).is_empty());
    // A mark that is not all of one entry ends nothing.
    let items = heard(
        &mut a,
        info(json!({ "h": { "phase": "responseBoundary", "terminalQuiescenceExpected": true } })),
    );
    assert!(turns(&items).is_empty(), "{items:?}");
    let items = heard(
        &mut a,
        info(json!({ "h": { "phase": "terminalQuiescence" } })),
    );
    assert_eq!(turns(&items), vec![(false, None)], "{items:?}");
    let text = items.iter().find_map(|i| match i {
        Item::Text { text, .. } => Some(text.clone()),
        _ => None,
    });
    assert_eq!(
        text.as_deref(),
        Some("Réveillé."),
        "its text closed in its turn"
    );
    // The next message is a turn of its own, its text alone.
    let sent = a.send("Et maintenant ?");
    assert_eq!(prompt_texts(&sent), vec!["Et maintenant ?".to_string()]);
    assert!(a.take_items().is_empty());
}

#[test]
fn its_turn_whose_mark_does_not_come_ends_with_the_next_message_or_a_stop() {
    let d = decl(json!({ "turnEnd": [{ "/_meta/h/phase": "terminalQuiescence" }] }));
    let mut a = idle(&d);
    heard(&mut a, update("Réveillé.", json!({})));
    let sent = a.send("Et maintenant ?");
    assert_eq!(
        prompt_texts(&sent),
        vec!["Et maintenant ?".to_string()],
        "sent at once, not held"
    );
    let items = a.take_items();
    assert_eq!(turns(&items), vec![(false, None)], "{items:?}");
    assert!(items
        .iter()
        .any(|i| matches!(i, Item::Text { text, .. } if text == "Réveillé.")));
    // The prompt's turn is told again as it begins.
    let items = heard(&mut a, update("Voici.", json!({})));
    assert!(
        matches!(items.first(), Some(Item::Start { .. })),
        "{items:?}"
    );
    // A stop ends a turn of its own at once, interrupted.
    let n = a.next;
    heard(
        &mut a,
        json!({ "jsonrpc": "2.0", "id": n, "result": { "stopReason": "end_turn" } }),
    );
    heard(&mut a, update("Encore réveillé.", json!({})));
    let lines = a.interrupt();
    assert!(lines.iter().any(|l| l.contains("session/cancel")));
    assert_eq!(turns(&a.take_items()), vec![(true, None)]);
}

/// A manifest that says nothing of such turns: what comes after the answer to the prompt is as it was before ESC-491 (the
/// streams of Goose, Cursor and the others, played against their recordings).
#[test]
fn without_marks_nothing_is_a_turn_of_its_own() {
    let mut a = idle(&decl(json!({})));
    let items = heard(&mut a, update("Après coup.", json!({})));
    assert!(
        !items.iter().any(|i| matches!(i, Item::Start { .. })),
        "{items:?}"
    );
    assert!(!a.own);
    let _ = a.send("Et maintenant ?");
    assert!(a.take_items().is_empty());
}

#[test]
fn what_comes_while_a_prompt_runs_is_no_turn_of_its_own() {
    let d = decl(json!({ "turnEnd": [{ "/_meta/h/phase": "terminalQuiescence" }] }));
    let (mut a, _) = initialized(&d, StartAt::New, json!({}));
    opened(&mut a, 2, "s1", json!({}));
    let _ = a.send("Bonjour.");
    let items = heard(&mut a, update("Je travaille.", json!({})));
    // Its mark before the answer to the prompt: the prompt's turn goes on.
    let more = heard(
        &mut a,
        info(json!({ "h": { "phase": "terminalQuiescence" } })),
    );
    assert!(turns(&items).is_empty() && turns(&more).is_empty());
    assert!(a.take_items().is_empty());
}

fn options(current_model: &str, thought: Option<&str>) -> Value {
    let mut o = vec![json!({
        "id": "model", "category": "model", "currentValue": format!("[\"ollama\",\"{current_model}\"]"),
        "options": [
            { "value": "[\"ollama\",\"qwen3-coder\"]" },
            { "value": "[\"ollama\",\"qwen3-think\"]" },
        ],
    })];
    if let Some(level) = thought {
        o.push(json!({
            "id": "thought_level", "category": "thought_level", "currentValue": level,
            "options": [{ "value": "off" }, { "value": "low" }, { "value": "medium" }, { "value": "high" }],
        }));
    }
    Value::Array(o)
}

fn settings(items: &[Item]) -> Vec<(String, Value)> {
    lines(items)
        .iter()
        .filter(|l| l["method"] == "session/set_config_option")
        .map(|l| {
            (
                l["params"]["configId"].as_str().unwrap().to_string(),
                l["params"]["value"].clone(),
            )
        })
        .collect()
}

#[test]
fn the_effort_goes_by_its_option_of_reasoning_when_the_session_offers_it() {
    let d = decl(
        json!({ "modelOption": "model", "efforts": { "low": "low", "medium": "medium", "high": "high" } }),
    );
    // A model that reasons: the option is there, set to the effort.
    let (mut a, _) = initialized(&d, StartAt::New, json!({}));
    let items = opened(
        &mut a,
        2,
        "s1",
        json!({ "configOptions": options("qwen3-think", Some("medium")) }),
    );
    assert_eq!(
        settings(&items),
        vec![("thought_level".into(), json!("high"))]
    );
    // Already on it: nothing.
    let (mut b, _) = initialized(&d, StartAt::New, json!({}));
    let items = opened(
        &mut b,
        2,
        "s1",
        json!({ "configOptions": options("qwen3-think", Some("high")) }),
    );
    assert!(settings(&items).is_empty());
    // A model that does not reason has no option: nothing, until the model changes to one that does.
    let (mut c, _) = initialized(&d, StartAt::New, json!({}));
    let items = opened(
        &mut c,
        2,
        "s1",
        json!({ "configOptions": options("qwen3-coder", None) }),
    );
    assert!(settings(&items).is_empty());
    let model = c.set_model("qwen3-think");
    let line: Value = serde_json::from_str(&model[0]).unwrap();
    assert_eq!(
        line["params"]["value"], "[\"ollama\",\"qwen3-think\"]",
        "the value made of parts"
    );
    let n = line["id"].as_u64().unwrap();
    let items = heard(
        &mut c,
        json!({ "jsonrpc": "2.0", "id": n, "result": { "configOptions": options("qwen3-think", Some("medium")) } }),
    );
    assert_eq!(
        settings(&items),
        vec![("thought_level".into(), json!("high"))]
    );
    // An effort the table leaves out sets none.
    assert!(c.set_effort("max").is_empty());
}

#[test]
fn without_a_table_the_effort_changes_nothing() {
    let d = decl(json!({ "modelOption": "model" }));
    let (mut a, _) = initialized(&d, StartAt::New, json!({}));
    let items = opened(
        &mut a,
        2,
        "s1",
        json!({ "configOptions": options("qwen3-think", Some("medium")) }),
    );
    assert!(settings(&items).is_empty());
}

#[test]
fn a_model_value_made_of_parts_is_found_by_its_last_part_or_all_of_them() {
    let d = decl(json!({ "modelOption": "model" }));
    for model in [
        "qwen3-think",
        "ollama/qwen3-think",
        "[\"ollama\",\"qwen3-think\"]",
    ] {
        let (mut a, _) = initialized(&d, StartAt::New, json!({}));
        a.model = model.into();
        let items = opened(
            &mut a,
            2,
            "s1",
            json!({ "configOptions": options("qwen3-coder", None) }),
        );
        assert_eq!(
            settings(&items),
            vec![("model".into(), json!("[\"ollama\",\"qwen3-think\"]"))],
            "{model}"
        );
    }
    assert!(!composed("[\"ollama\",\"qwen3-think\"]", "think"));
    assert!(!composed("qwen3-think", "qwen3-think"), "not made of parts");
}

#[test]
fn what_its_context_holds_is_read_where_its_manifest_says_never_guessed() {
    let d = decl(
        json!({ "usage": { "context": "/_meta/h/usage/tokens", "window": "/_meta/h/usage/window" } }),
    );
    let mut a = idle(&d);
    let ctx = |items: &[Item]| {
        items
            .iter()
            .filter_map(|i| match i {
                Item::Context { tokens, window } => Some((*tokens, *window)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let items = heard(
        &mut a,
        info(json!({ "h": { "usage": { "tokens": 1289, "window": 32768, "cost": 0 } } })),
    );
    assert_eq!(ctx(&items), vec![(Some(1289), Some(32768))]);
    assert!(turns(&items).is_empty() && !items.iter().any(|i| matches!(i, Item::Start { .. })));
    // The same again: nothing; what it does not say: nothing.
    let items = heard(
        &mut a,
        info(json!({ "h": { "usage": { "tokens": 1289, "window": 32768 } } })),
    );
    assert!(ctx(&items).is_empty());
    assert!(ctx(&heard(&mut a, info(json!({ "h": {} })))).is_empty());
    // Without the manifest's word, ACP's `usage_update`.
    let mut b = idle(&decl(json!({})));
    let items = heard(
        &mut b,
        json!({ "jsonrpc": "2.0", "method": "session/update", "params": { "sessionId": "s1",
            "update": { "sessionUpdate": "usage_update", "used": 500, "size": 8000 } } }),
    );
    assert_eq!(ctx(&items), vec![(Some(500), Some(8000))]);
    assert!(ctx(&heard(
        &mut b,
        info(json!({ "h": { "usage": { "tokens": 3 } } }))
    ))
    .is_empty());
}

/// A recorded stream (`tests/fixtures/<harness>/<case>.jsonl`): what the agent wrote, by its order.
fn recorded(text: &str) -> Vec<Value> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .collect()
}

fn from_agent(entry: &Value) -> bool {
    matches!(entry["dir"].as_str(), Some("s2c" | "in"))
}

/// The harness of a plugin's manifest.
fn of_manifest(text: &str) -> HarnessDecl {
    let v: Value = serde_json::from_str(text).unwrap();
    serde_json::from_value(v["contributes"]["harnesses"][0].clone()).unwrap()
}

/// Prime Agent 0.10.3, recorded (ESC-485, config-options.jsonl): a model that reasons, chosen by its value made of
/// parts, brings the option `thought_level`, which its manifest's table sets to the agent's effort, once; the option
/// `thinking` the study tried does not exist and is never asked.
#[test]
fn the_recorded_stream_of_prime_agent_sets_its_model_by_its_parts_and_its_effort_by_thought_level()
{
    let d = of_manifest(include_str!(
        "../../../plugins/prime-agent/escouade-plugin.json"
    ));
    let flux = recorded(include_str!(
        "../../../tests/fixtures/prime-agent/config-options.jsonl"
    ));
    let theirs: Vec<&Value> = flux.iter().filter(|e| from_agent(e)).collect();
    let ours: Vec<&Value> = flux.iter().filter(|e| !from_agent(e)).collect();
    let mut a = Agent::new(&attach(), &d, "2.2.3");
    a.launch(&LaunchParams {
        append_prompt: None,
        ..params(StartAt::New)
    })
    .unwrap();
    // `initialize`, then `session/new` (a model that does not reason: no option of effort, nothing set).
    let items = a.frames(&[theirs[0]["msg"].to_string()]);
    assert_eq!(lines(&items)[0]["method"], "session/new");
    let items = a.frames(&[theirs[1]["msg"].to_string()]);
    assert!(settings(&items).is_empty(), "{items:?}");
    // The model that reasons, by its last part: the value the study sent.
    let sent: Vec<Value> = a
        .set_model("qwen3-think")
        .iter()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(sent.len(), 1);
    assert_eq!(
        sent[0]["params"]["value"],
        ours[2]["msg"]["params"]["value"]
    );
    assert_eq!(sent[0]["id"], 3);
    // Its options told twice (the update, then the answer): `thought_level` set once, to the effort's value.
    let mut asked = Vec::new();
    for e in &theirs[2..4] {
        asked.extend(settings(&a.frames(&[e["msg"].to_string()])));
    }
    assert_eq!(asked, vec![("thought_level".to_string(), json!("high"))]);
    assert_eq!(
        ours[4]["msg"]["params"]["configId"], "thought_level",
        "the one the study found"
    );
    // Its answer (the option on `high`): nothing more.
    let mut answer = theirs[6]["msg"].clone();
    answer["id"] = json!(4);
    let mut more = settings(&a.frames(&[theirs[5]["msg"].to_string()]));
    more.extend(settings(&a.frames(&[answer.to_string()])));
    assert!(more.is_empty(), "{more:?}");
    assert!(notices(&a.frames(&[])).is_empty());
}

/// Unharnessed 0.9.8, recorded (ESC-484, full-turn.jsonl): its `session_info_update`s tell what its context holds and its
/// window, read by its manifest's pointers; its turns of its own (`turnStart: wake`) that say nothing begin no turn.
#[test]
fn the_recorded_stream_of_unharnessed_tells_its_context_and_its_silent_wakes_begin_no_turn() {
    let d = of_manifest(include_str!(
        "../../../plugins/unharnessed/escouade-plugin.json"
    ));
    let flux = recorded(include_str!(
        "../../../tests/fixtures/unharnessed/full-turn.jsonl"
    ));
    let mut a = Agent::new(&attach(), &d, "2.2.3");
    a.launch(&LaunchParams {
        append_prompt: None,
        ..params(StartAt::New)
    })
    .unwrap();
    let mut items = Vec::new();
    for e in &flux {
        if from_agent(e) {
            items.extend(a.frames(&[e["msg"].to_string()]));
        } else {
            a.recorded(&e["msg"]);
        }
    }
    let ctx: Vec<(Option<u64>, Option<u64>)> = items
        .iter()
        .filter_map(|i| match i {
            Item::Context { tokens, window } => Some((*tokens, *window)),
            _ => None,
        })
        .collect();
    // Its window first, as its session opens: what it holds is not told yet, nor guessed.
    assert_eq!(ctx[0], (None, Some(32768)), "{ctx:?}");
    assert_eq!(ctx[1], (Some(1289), Some(32768)), "{ctx:?}");
    assert_eq!(ctx.last(), Some(&(Some(2222), Some(32768))), "{ctx:?}");
    // Its two prompts, two turns; its wakes said nothing: no turn of their own.
    assert_eq!(turns(&items).len(), 2, "{:?}", turns(&items));
    assert!(!a.own);
}
