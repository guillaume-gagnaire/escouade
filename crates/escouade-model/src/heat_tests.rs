use super::*;
use serde_json::json;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// The usages of `tests/fixtures/claude/long-turn.jsonl`: Claude Code writes its cache for 1 hour.
fn fixture_usages(name: &str) -> Vec<Value> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    let text = std::fs::read_to_string(path).unwrap();
    text.lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter_map(|f| {
            let u = &f["event"]["message"]["usage"];
            let u = if u.is_object() {
                u
            } else {
                &f["message"]["usage"]
            };
            (u.is_object() && u["cache_creation"].is_object()).then(|| u.clone())
        })
        .collect()
}

#[test]
fn claude_code_s_cache_lives_an_hour_when_it_writes_for_an_hour_as_its_fixture_does() {
    let usages = fixture_usages("claude/long-turn.jsonl");
    assert!(!usages.is_empty());
    for u in &usages {
        assert_eq!(claude_ttl(u, None), ONE_HOUR, "{u}");
    }
}

#[test]
fn claude_code_s_cache_lives_five_minutes_when_written_for_five_or_not_told_apart() {
    let five = json!({ "cache_creation_input_tokens": 40,
        "cache_creation": { "ephemeral_5m_input_tokens": 40, "ephemeral_1h_input_tokens": 0 } });
    assert_eq!(claude_ttl(&five, Some(ONE_HOUR)), FIVE_MINUTES);
    // An older CLI, its writes not told apart: the API's default.
    let old = json!({ "cache_creation_input_tokens": 40 });
    assert_eq!(claude_ttl(&old, None), FIVE_MINUTES);
    // A request that only read keeps the life it had, 5 minutes when it had none.
    let read = json!({ "cache_read_input_tokens": 900,
        "cache_creation": { "ephemeral_5m_input_tokens": 0, "ephemeral_1h_input_tokens": 0 } });
    assert_eq!(claude_ttl(&read, Some(ONE_HOUR)), ONE_HOUR);
    assert_eq!(claude_ttl(&read, None), FIVE_MINUTES);
}

#[test]
fn the_vendor_of_a_model_gives_the_life_of_its_cache_or_none() {
    for m in [
        "claude-opus-5-5",
        "anthropic/claude-sonnet-4-5",
        "claude-sonnet-4.5",
        "gpt-6-luna",
        "openai/gpt-5",
        "o4-mini",
        "gpt-5-codex",
    ] {
        assert_eq!(vendor_ttl(m), Some(FIVE_MINUTES), "{m}");
    }
    for m in [
        "gemini-2.5-pro",
        "devstral-medium",
        "auto",
        "",
        "ollama/qwen3",
        "gemini-3-flash",
    ] {
        assert_eq!(vendor_ttl(m), None, "{m}");
    }
}

#[test]
fn a_request_of_claude_code_tells_what_it_read_wrote_and_saved() {
    let u = json!({"input_tokens":10,"cache_read_input_tokens":100_000,"cache_creation_input_tokens":2000,
        "cache_creation":{"ephemeral_5m_input_tokens":0,"ephemeral_1h_input_tokens":2000},"output_tokens":1});
    let s = claude_served("claude-opus-5-5", &u);
    assert_eq!(
        (s.read, s.written, s.cached, s.fresh),
        (Some(100_000), Some(2000), 102_000, 10)
    );
    // 100 k read at 0.20 $ instead of 4 $ the million.
    assert!(close(s.saved.unwrap(), 0.1 * (4.0 - 0.2)));
}

#[test]
fn a_turn_told_by_its_tokens_tells_read_apart_only_at_openai() {
    // Codex (`cachedInputTokens`, its writes always 0 in its fixtures).
    let codex = turn_served("gpt-6-luna", 4000, 129_000);
    assert_eq!(
        (codex.read, codex.written, codex.cached),
        (Some(129_000), Some(0), 129_000)
    );
    // Copilot or OpenCode on Claude: read and written in one figure, nothing saved invented.
    let copilot = turn_served("claude-sonnet-4.5", 4000, 129_000);
    assert_eq!(
        (copilot.read, copilot.written, copilot.saved),
        (None, None, None)
    );
    assert_eq!(copilot.cached, 129_000);
}

#[test]
fn the_next_message_costs_cents_warm_and_tens_of_cents_cold() {
    // As the doc says it: Opus 5.5, 100 k of context.
    let (warm, cold) = next_message("claude-opus-5-5", 100_000, Some(FIVE_MINUTES));
    assert!(close(warm.unwrap(), 0.02));
    assert!(close(cold.unwrap(), 0.5));
    let (_, cold_1h) = next_message("claude-opus-5-5", 100_000, Some(ONE_HOUR));
    assert!(close(cold_1h.unwrap(), 0.8));
    assert_eq!(next_message("gemini-2.5-pro", 100_000, None), (None, None));
}

fn meta() -> AgentMeta {
    AgentMeta {
        model: "opus".into(),
        account: "main".into(),
        append_prompt: Some("Protocole".into()),
        ..Default::default()
    }
}

fn warm(m: &AgentMeta) -> CacheHeat {
    let mut w = Warming::open(None);
    w.model = "claude-opus-5-5".into();
    w.ttl = Some(FIVE_MINUTES);
    w.add(claude_served(
        "claude-opus-5-5",
        &json!({"input_tokens":10,"cache_read_input_tokens":1000}),
    ));
    CacheHeat::warmed(m, &w, 1010, 1_000)
}

#[test]
fn the_cache_breaks_when_its_model_its_account_or_its_instructions_change() {
    let m = meta();
    let h = warm(&m);
    assert_eq!(h.seen(&m).broken, None);
    let changed = |f: fn(&mut AgentMeta)| {
        let mut other = m.clone();
        f(&mut other);
        h.seen(&other).broken
    };
    assert_eq!(
        changed(|m| m.model = "sonnet".into()).as_deref(),
        Some(broken::MODEL)
    );
    assert_eq!(
        changed(|m| m.account = "other".into()).as_deref(),
        Some(broken::ACCOUNT)
    );
    assert_eq!(
        changed(|m| m.append_prompt = Some("Autre".into())).as_deref(),
        Some(broken::PROMPT)
    );
    // Its mode or its effort is no part of the prefix.
    assert_eq!(changed(|m| m.effort = "high".into()), None);
}

#[test]
fn what_the_cache_saved_adds_up_over_the_day_and_starts_again_the_next() {
    let m = meta();
    let first = warm(&m);
    let saved = first.saved.unwrap();
    assert!(close(first.saved_today, saved));
    // The same day: added.
    let mut w = Warming::open(Some(&first));
    w.model = "claude-opus-5-5".into();
    w.add(claude_served(
        "claude-opus-5-5",
        &json!({"cache_read_input_tokens":1000}),
    ));
    let second = CacheHeat::warmed(&m, &w, 1000, first.at + 60_000);
    assert!(close(second.saved_today, 2.0 * saved));
    // Another day: only its own.
    let third = CacheHeat::warmed(&m, &w, 1000, first.at + 2 * 86_400_000);
    assert!(close(third.saved_today, saved));
}

#[test]
fn it_is_kept_as_the_agent_is_saved() {
    let mut m = meta();
    m.heat = Some(warm(&m));
    let back: AgentMeta = serde_json::from_value(serde_json::to_value(&m).unwrap()).unwrap();
    assert_eq!(back.heat, m.heat);
    // An agent saved before it: none.
    let old: AgentMeta = serde_json::from_value(json!({ "id": "a1" })).unwrap();
    assert_eq!(old.heat, None);
}
