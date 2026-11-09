use super::*;
use crate::heat::{broken, FIVE_MINUTES, ONE_HOUR};
use serde_json::json;

fn rt(engine: EngineId, model: &str) -> AgentRt {
    AgentRt::new(
        AgentMeta {
            id: new_id(),
            cwd: "C:/p".into(),
            engine,
            model: model.into(),
            ..Default::default()
        },
        &std::env::temp_dir().join(format!("ccm-heat-tests-{}", std::process::id())),
    )
}

fn frames(a: &mut AgentRt, name: &str) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures")
        .join(name);
    let mut fx = Effects::default();
    for line in std::fs::read_to_string(path).unwrap().lines() {
        if let Ok(f) = serde_json::from_str::<Value>(line) {
            a.handle_frame(&f, &mut fx);
        }
    }
}

fn message(a: &mut AgentRt, id: &str, parent: Option<&str>, usage: Value) {
    a.handle_frame(
        &json!({"type":"stream_event","parent_tool_use_id":parent,
            "event":{"type":"message_start","message":{"id":id,"model":"claude-opus-5-5","usage":usage}}}),
        &mut Effects::default(),
    );
}

fn turn(a: &mut AgentRt, input: u64, cache: u64) {
    a.apply_items(
        &[json!({ "op": "turn", "usage": { "input": input, "cache": cache, "output": 10 } })],
        &mut Effects::default(),
    );
}

#[test]
fn claude_code_s_recorded_turn_warms_its_cache_for_an_hour() {
    let mut a = rt(EngineId::CLAUDE, "sonnet");
    let before = now_ms();
    frames(&mut a, "claude/long-turn.jsonl");
    let h = a.view().meta.heat.expect("warmed");
    assert_eq!(h.ttl, Some(ONE_HOUR));
    assert!(h.at >= before);
    assert_eq!(h.broken, None);
    assert_eq!(h.model, "claude-sonnet-5-5");
    assert!(h.read.unwrap() > 0 && h.written.unwrap() > 0);
    assert!(h.saved.unwrap() > 0.0 && h.cold.unwrap() > h.warm.unwrap());
    assert_eq!(h.context, a.context_tokens);
}

#[test]
fn a_subagent_s_request_does_not_warm_the_agent_s_cache() {
    let mut a = rt(EngineId::CLAUDE, "opus");
    message(
        &mut a,
        "s1",
        Some("tu1"),
        json!({"input_tokens":5,"cache_read_input_tokens":900}),
    );
    assert_eq!(a.view().meta.heat, None);
    message(
        &mut a,
        "m1",
        None,
        json!({"input_tokens":5,"cache_read_input_tokens":900}),
    );
    assert_eq!(a.view().meta.heat.unwrap().read, Some(900));
}

#[test]
fn a_turn_adds_up_its_requests_and_the_next_starts_again() {
    let mut a = rt(EngineId::CLAUDE, "opus");
    let split = |w5: u64| json!({"ephemeral_5m_input_tokens": w5, "ephemeral_1h_input_tokens": 0});
    message(
        &mut a,
        "m1",
        None,
        json!({"input_tokens":5,"cache_read_input_tokens":900,
        "cache_creation_input_tokens":100,"cache_creation":split(100)}),
    );
    message(
        &mut a,
        "m2",
        None,
        json!({"input_tokens":3,"cache_read_input_tokens":1000,
        "cache_creation_input_tokens":50,"cache_creation":split(50)}),
    );
    let h = a.view().meta.heat.unwrap();
    assert_eq!(
        (h.read, h.written, h.fresh, h.ttl),
        (Some(1900), Some(150), 8, Some(FIVE_MINUTES))
    );
    a.heat_close();
    message(
        &mut a,
        "m3",
        None,
        json!({"input_tokens":1,"cache_read_input_tokens":1050}),
    );
    let h = a.view().meta.heat.unwrap();
    // Read only: the life it had.
    assert_eq!(
        (h.read, h.written, h.ttl),
        (Some(1050), Some(0), Some(FIVE_MINUTES))
    );
    assert!(h.saved_today > h.saved.unwrap());
}

#[test]
fn a_compaction_of_claude_code_breaks_the_cache_and_the_next_request_warms_it() {
    let mut a = rt(EngineId::CLAUDE, "opus");
    message(
        &mut a,
        "m1",
        None,
        json!({"input_tokens":5,"cache_read_input_tokens":900}),
    );
    a.handle_frame(
        &json!({ "type": "system", "subtype": "compact_boundary", "uuid": "b1" }),
        &mut Effects::default(),
    );
    assert_eq!(
        a.view().meta.heat.unwrap().broken.as_deref(),
        Some(broken::COMPACTION)
    );
    message(
        &mut a,
        "m2",
        None,
        json!({"input_tokens":5,"cache_creation_input_tokens":3000}),
    );
    assert_eq!(a.view().meta.heat.unwrap().broken, None);
}

#[test]
fn a_model_changed_shows_the_cache_cold() {
    let mut a = rt(EngineId::CLAUDE, "opus");
    message(
        &mut a,
        "m1",
        None,
        json!({"input_tokens":5,"cache_read_input_tokens":900}),
    );
    a.meta.model = "sonnet".into();
    assert_eq!(
        a.view().meta.heat.unwrap().broken.as_deref(),
        Some(broken::MODEL)
    );
    // What is saved keeps the state as it was told: the view alone says it cold.
    assert_eq!(a.meta.heat.as_ref().unwrap().broken, None);
}

#[test]
fn codex_s_turn_warms_its_cache_for_five_minutes_from_what_it_read() {
    let mut a = rt(EngineId::CODEX, "gpt-6-luna");
    turn(&mut a, 4000, 129_000);
    let h = a.view().meta.heat.unwrap();
    assert_eq!(
        (h.ttl, h.read, h.written, h.fresh),
        (Some(FIVE_MINUTES), Some(129_000), Some(0), 4000)
    );
    assert!(h.saved.unwrap() > 0.0);
}

#[test]
fn a_turn_that_cached_nothing_does_not_warm_and_an_unknown_life_is_none() {
    let mut a = rt(EngineId::CODEX, "gpt-6-luna");
    turn(&mut a, 4000, 0);
    assert_eq!(a.view().meta.heat, None);
    // Gemini CLI tells what it read (`cachedReadTokens`), not how long it lives: « — °C ».
    let mut g = rt(EngineId::GEMINI, "gemini-2.5-pro");
    turn(&mut g, 4000, 9600);
    let h = g.view().meta.heat.unwrap();
    assert_eq!((h.ttl, h.cached), (None, 9600));
}

#[test]
fn copilot_on_claude_tells_its_cache_in_one_figure() {
    let mut a = rt(EngineId::COPILOT, "claude-sonnet-4.5");
    turn(&mut a, 400, 13_324);
    let h = a.view().meta.heat.unwrap();
    assert_eq!(
        (h.ttl, h.read, h.written, h.cached, h.saved),
        (Some(FIVE_MINUTES), None, None, 13_324, None)
    );
}

#[test]
fn a_compaction_asked_of_its_harness_leaves_the_cache_cold_after_the_turn_that_ends_it() {
    let mut a = rt(EngineId::CODEX, "gpt-6-luna");
    turn(&mut a, 4000, 129_000);
    a.heat_compacts(&mut Effects::default());
    turn(&mut a, 2000, 120_000);
    assert_eq!(
        a.view().meta.heat.unwrap().broken.as_deref(),
        Some(broken::COMPACTION)
    );
    turn(&mut a, 2000, 3000);
    assert_eq!(a.view().meta.heat.unwrap().broken, None);
}
