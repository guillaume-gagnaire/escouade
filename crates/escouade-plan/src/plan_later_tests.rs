//! Tests of what an agent leaves to happen after its turn and outside its own tasks (ESC-138):
//! the wake-ups of its session (`CronCreate`, `ScheduleWakeup`) and the agents of Escouade it
//! drives through the MCP server. The forms are those of a real Claude Code 2.1.306.

use crate::plan::*;
use serde_json::{json, Value};

const T0: i64 = 1_000_000;

/// A call, then its result: `text` the tool result's text, `tur` its structured result.
fn call(p: &mut PlanState, id: &str, name: &str, input: Value, text: &str, tur: Value) -> Change {
    p.on_tool_use(None, id, name, &input, T0);
    p.on_tool_result(id, false, text, &tur, T0)
}

/// `CronCreate`'s result as Claude Code 2.1.306 gives it.
fn cron_result(id: &str, at: i64, recurring: bool) -> Value {
    json!({ "id": id, "humanSchedule": "17 8 22 10 *", "recurring": recurring, "durable": false,
        "nextFireAt": at })
}

#[test]
fn a_cron_of_the_session_is_a_wake_up_until_it_is_deleted() {
    let mut p = PlanState::default();
    let at = T0 + 60_000;
    let change = call(
        &mut p,
        "c1",
        "CronCreate",
        json!({ "cron": "17 8 22 10 *", "prompt": "Vérifie les checks de la PR 148", "recurring": false }),
        "Scheduled one-shot task 9efa53bd (17 8 22 10 *). Next run: Oct 22 8:17am.",
        cron_result("9efa53bd", at, false),
    );
    assert_eq!(change, Change::Saved);
    assert_eq!(
        p.wakes,
        [Wake {
            id: "9efa53bd".into(),
            kind: WakeKind::Cron,
            at,
            recurring: false,
            durable: false,
            schedule: Some("17 8 22 10 *".into()),
            prompt: Some("Vérifie les checks de la PR 148".into()),
        }]
    );
    assert!(!p.is_empty());
    assert_eq!(p.pending_wakes(T0).count(), 1);
    // A one-shot wake-up whose time passed is over.
    assert_eq!(p.pending_wakes(at + 1).count(), 0);

    // Deleted by its id: gone. A refused deletion, or another id, changes nothing.
    p.on_tool_use(None, "d0", "CronDelete", &json!({ "id": "9efa53bd" }), T0);
    assert_eq!(
        p.on_tool_result("d0", true, "no such job", &Value::Null, T0),
        Change::None
    );
    assert_eq!(
        call(
            &mut p,
            "d1",
            "CronDelete",
            json!({ "id": "other" }),
            "",
            Value::Null
        ),
        Change::None
    );
    assert_eq!(p.wakes.len(), 1);
    assert_eq!(
        call(
            &mut p,
            "d2",
            "CronDelete",
            json!({ "id": "9efa53bd" }),
            "Cancelled job 9efa53bd.",
            json!({ "id": "9efa53bd" })
        ),
        Change::Saved
    );
    assert!(p.wakes.is_empty());
}

#[test]
fn a_recurring_cron_waits_after_its_first_time() {
    let mut p = PlanState::default();
    call(
        &mut p,
        "c1",
        "CronCreate",
        json!({ "cron": "*/5 * * * *", "prompt": "/babysit-prs", "recurring": true }),
        "",
        cron_result("r1", T0 + 1_000, true),
    );
    assert_eq!(p.pending_wakes(T0 + 3_600_000).count(), 1);
}

#[test]
fn the_wake_up_of_a_dynamic_loop_is_one_and_ends_with_its_stop() {
    let mut p = PlanState::default();
    let first = T0 + 1_200_000;
    let wake = |delay: i64| json!({ "delaySeconds": delay, "reason": "watching CI run", "prompt": "/loop check CI", "noop": true });
    call(
        &mut p,
        "w1",
        "ScheduleWakeup",
        wake(1200),
        "",
        json!({ "scheduledFor": first, "clampedDelaySeconds": 1200, "wasClamped": false }),
    );
    assert_eq!(p.wakes.len(), 1);
    assert_eq!(p.wakes[0].kind, WakeKind::Loop);
    assert_eq!(p.wakes[0].at, first);
    assert_eq!(p.wakes[0].prompt.as_deref(), Some("watching CI run"));
    // The next one replaces it: a loop has one wake-up pending.
    call(
        &mut p,
        "w2",
        "ScheduleWakeup",
        wake(60),
        "",
        json!({ "scheduledFor": T0 + 60_000, "clampedDelaySeconds": 60, "wasClamped": false }),
    );
    assert_eq!(p.wakes.len(), 1);
    assert_eq!(p.wakes[0].at, T0 + 60_000);
    // None armed (`scheduledFor` 0): nothing new.
    call(
        &mut p,
        "w3",
        "ScheduleWakeup",
        wake(60),
        "",
        json!({ "scheduledFor": 0, "clampedDelaySeconds": 0, "wasClamped": false }),
    );
    assert_eq!(p.wakes.len(), 1);
    // The loop stopped: no wake-up any more.
    call(
        &mut p,
        "w4",
        "ScheduleWakeup",
        json!({ "stop": true }),
        "",
        json!({ "scheduledFor": 0, "clampedDelaySeconds": 0, "wasClamped": false, "stopped": true, "cancelledWakeups": 1 }),
    );
    assert!(p.wakes.is_empty());
}

#[test]
fn the_wake_ups_outlive_a_new_plan_but_not_the_process() {
    let mut p = PlanState::default();
    call(
        &mut p,
        "c1",
        "CronCreate",
        json!({ "cron": "0 9 * * *", "prompt": "Bilan", "recurring": true }),
        "",
        json!({ "id": "a", "humanSchedule": "0 9 * * *", "recurring": true, "durable": false, "nextFireAt": T0 + 1 }),
    );
    call(
        &mut p,
        "c2",
        "CronCreate",
        json!({ "cron": "0 10 * * *", "prompt": "Bilan durable", "recurring": true, "durable": true }),
        "",
        json!({ "id": "b", "humanSchedule": "0 10 * * *", "recurring": true, "durable": true, "nextFireAt": T0 + 2 }),
    );
    // A new turn after a plan that is over resets the plan: the wake-ups are still to come.
    p.reset();
    assert_eq!(p.wakes.len(), 2);
    // The process is gone: those of the session went with it, a durable one is on disk.
    assert_eq!(p.close_running(T0), Change::Saved);
    assert_eq!(p.wakes.len(), 1);
    assert_eq!(p.wakes[0].id, "b");
}

#[test]
fn a_pending_wake_up_holds_the_process() {
    let mut p = PlanState::default();
    assert!(!p.holds_process(T0));
    call(
        &mut p,
        "w1",
        "ScheduleWakeup",
        json!({ "delaySeconds": 60, "reason": "r", "prompt": "p", "noop": false }),
        "",
        json!({ "scheduledFor": T0 + 60_000, "clampedDelaySeconds": 60, "wasClamped": false }),
    );
    assert!(p.holds_process(T0));
    // It does not count among what runs (« N en fond »), nor keeps the plan from being over.
    assert!(!p.runs_something());
    assert!(p.is_finished());
    assert!(!p.holds_process(T0 + 60_001));
}

#[test]
fn the_agents_and_tickets_it_drives_through_escouade_are_kept() {
    let mut p = PlanState::default();
    let change = call(
        &mut p,
        "m1",
        "mcp__escouade__send_message",
        json!({ "agent": "deploy-api", "text": "Vérifie deux points" }),
        r#"{"id":"a7","name":"deploy-api","queued":false}"#,
        json!([{ "type": "text", "text": "…" }]),
    );
    assert_eq!(change, Change::Saved);
    call(
        &mut p,
        "m2",
        "mcp__escouade__create_agent",
        json!({ "project": "socle", "message": "Fais #53" }),
        r#"{"id":"a9","name":"issue-53"}"#,
        Value::Null,
    );
    call(
        &mut p,
        "m3",
        "mcp__escouade__reject_ticket",
        json!({ "ticket": "ESC-148", "comment": "Change d'approche" }),
        r#"{"key":"ESC-148","column":"inProgress"}"#,
        Value::Null,
    );
    call(
        &mut p,
        "m4",
        "mcp__escouade__start_ticket",
        json!({ "ticket": "t-id-12" }),
        "Queued.",
        Value::Null,
    );
    let driven: Vec<(Option<&str>, Option<&str>)> = p
        .driven
        .iter()
        .map(|d| (d.agent.as_deref(), d.ticket.as_deref()))
        .collect();
    assert_eq!(
        driven,
        [
            (Some("a7"), None),
            (Some("a9"), None),
            (None, Some("ESC-148")),
            (None, Some("t-id-12")),
        ]
    );
    // The same agent again: once, the latest last.
    call(
        &mut p,
        "m5",
        "mcp__escouade__send_message",
        json!({ "agent": "a7", "text": "Dernier tour" }),
        r#"{"id":"a7","name":"deploy-api","queued":true}"#,
        Value::Null,
    );
    assert_eq!(p.driven.len(), 4);
    assert_eq!(p.driven.last().unwrap().agent.as_deref(), Some("a7"));
    // A refusal, a reading tool and another server's tool drive nothing.
    p.on_tool_use(
        None,
        "m6",
        "mcp__escouade__send_message",
        &json!({ "agent": "x" }),
        T0,
    );
    p.on_tool_result("m6", true, "Refused", &Value::Null, T0);
    call(
        &mut p,
        "m7",
        "mcp__escouade__get_ticket",
        json!({ "ticket": "ESC-1" }),
        r#"{"key":"ESC-1"}"#,
        Value::Null,
    );
    call(
        &mut p,
        "m8",
        "mcp__other__send_message",
        json!({ "agent": "y" }),
        r#"{"id":"y"}"#,
        Value::Null,
    );
    assert_eq!(p.driven.len(), 4);
    // Kept over a new plan and the end of the process: the agents work on in Escouade.
    p.reset();
    p.close_running(T0);
    assert_eq!(p.driven.len(), 4);
    // Read back as it was saved.
    let back: PlanState = serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap();
    assert_eq!(back.driven, p.driven);
}

#[test]
fn the_list_of_the_driven_keeps_the_latest() {
    let mut p = PlanState::default();
    for i in 0..(MAX_DRIVEN + 5) {
        call(
            &mut p,
            &format!("m{i}"),
            "mcp__escouade__send_message",
            json!({ "agent": format!("a{i}"), "text": "…" }),
            &format!(r#"{{"id":"a{i}","name":"n{i}"}}"#),
            Value::Null,
        );
    }
    assert_eq!(p.driven.len(), MAX_DRIVEN);
    assert_eq!(p.driven[0].agent.as_deref(), Some("a5"));
}

#[test]
fn a_plan_without_wake_ups_nor_driven_agents_saves_none() {
    let v = serde_json::to_value(PlanState::default()).unwrap();
    assert!(v.get("wakes").is_none() && v.get("driven").is_none(), "{v}");
}
