//! Tests of the plan's reducer (`plan.rs`), on the calls and frames of the forms docs/PROTOCOL.md
//! lists (« Avancée : ce que le flux dit »), written by hand.

use crate::plan::*;
use serde_json::{json, Value};

const T0: i64 = 1_000;

fn tool(p: &mut PlanState, id: &str, name: &str, input: Value) -> Change {
    p.on_tool_use(None, id, name, &input, T0)
}

/// A tool call of the subagent whose call is `parent`.
fn child(p: &mut PlanState, parent: &str, id: &str, name: &str) -> Change {
    p.on_tool_use(Some(parent), id, name, &json!({}), T0)
}

fn result(p: &mut PlanState, id: &str, text: &str) -> Change {
    p.on_tool_result(id, false, text, &Value::Null, T0)
}

fn sys(p: &mut PlanState, f: Value) -> Change {
    p.on_system(&f, T0)
}

/// Tasks "1" to "n" as the task tools make them: the calls, then their results.
fn listed(p: &mut PlanState, titles: &[&str]) {
    for (i, title) in titles.iter().enumerate() {
        tool(
            p,
            &format!("c{i}"),
            "TaskCreate",
            json!({ "subject": title }),
        );
    }
    for (i, title) in titles.iter().enumerate() {
        result(
            p,
            &format!("c{i}"),
            &format!("Task #{} created successfully: {title}", i + 1),
        );
    }
}

fn update(p: &mut PlanState, call: &str, input: Value) -> Change {
    tool(p, call, "TaskUpdate", input)
}

fn ids(p: &PlanState) -> Vec<&str> {
    p.tasks.iter().map(|t| t.id.as_str()).collect()
}

fn task<'a>(p: &'a PlanState, id: &str) -> &'a PlanTask {
    p.tasks.iter().find(|t| t.id == id).expect("a task")
}

fn row<'a>(p: &'a PlanState, id: &str) -> &'a SubAgent {
    p.agents.iter().find(|a| a.id == id).expect("a row")
}

fn launch(p: &mut PlanState, id: &str, input: Value) -> Change {
    tool(p, id, "Agent", input)
}

/// The result of a subagent in the foreground: Claude Code's `tool_use_result`.
fn finished(tools: u64, ms: u64, tokens: u64) -> Value {
    json!({ "status": "completed", "agentId": "x", "content": [], "totalToolUseCount": tools,
            "totalDurationMs": ms, "totalTokens": tokens, "usage": {} })
}

fn started(task_id: &str, tool_use_id: &str) -> Value {
    json!({ "type": "system", "subtype": "task_started", "task_id": task_id,
            "tool_use_id": tool_use_id, "description": "Chercher", "task_type": "local_agent" })
}

fn notified(task_id: &str, tool_use_id: &str, status: &str) -> Value {
    json!({ "type": "system", "subtype": "task_notification", "task_id": task_id,
            "tool_use_id": tool_use_id, "status": status, "summary": "fini" })
}

fn updated(task_id: &str, patch: Value) -> Value {
    json!({ "type": "system", "subtype": "task_updated", "task_id": task_id, "patch": patch })
}

// ---------- the task list ----------

#[test]
fn a_todo_list_replaces_the_one_before() {
    let mut p = PlanState::default();
    let change = tool(
        &mut p,
        "t1",
        "TodoWrite",
        json!({ "todos": [
            { "content": "Lire le code", "status": "completed", "activeForm": "Lit le code" },
            { "content": "Écrire le test", "status": "in_progress", "activeForm": "Écrit le test" },
            { "content": "Lancer la suite", "status": "pending", "activeForm": "Lance la suite" },
        ]}),
    );
    assert_eq!(change, Change::Saved);
    assert_eq!(p.source, Some(PlanSource::Tools));
    assert_eq!(ids(&p), ["1", "2", "3"]);
    assert_eq!(p.tasks[0].title, "Lire le code");
    assert_eq!(p.tasks[0].status, TaskStatus::Done);
    assert_eq!(p.tasks[1].status, TaskStatus::InProgress);
    assert_eq!(p.tasks[1].active.as_deref(), Some("Écrit le test"));
    assert_eq!(p.tasks[2].status, TaskStatus::Pending);

    // The next call is the whole list again: what it says is the list.
    let next =
        json!({ "todos": [{ "content": "Autre", "status": "pending", "activeForm": "Autre" }] });
    assert_eq!(tool(&mut p, "t2", "TodoWrite", next.clone()), Change::Saved);
    assert_eq!(ids(&p), ["1"]);
    assert_eq!(p.tasks[0].title, "Autre");
    // The same list again changes nothing.
    assert_eq!(tool(&mut p, "t3", "TodoWrite", next), Change::None);
}

#[test]
fn task_creates_of_one_message_are_listed_at_once_and_get_their_id_from_their_result() {
    let mut p = PlanState::default();
    for (i, subject) in ["Lire", "Écrire", "Tester"].iter().enumerate() {
        let change = tool(
            &mut p,
            &format!("c{i}"),
            "TaskCreate",
            json!({ "subject": subject, "description": "…", "activeForm": format!("{subject} en cours") }),
        );
        assert_eq!(change, Change::Saved);
    }
    // Pending, with their words, before any id is known.
    assert_eq!(p.tasks.len(), 3);
    assert!(p.tasks.iter().all(|t| t.status == TaskStatus::Pending));
    assert_eq!(p.tasks[1].title, "Écrire");
    assert_eq!(p.tasks[1].active.as_deref(), Some("Écrire en cours"));
    assert_eq!(p.source, Some(PlanSource::Tools));
    // The results come in another order: each id goes to the task of its own call.
    assert_eq!(
        result(&mut p, "c2", "Task #3 created successfully: Tester"),
        Change::Saved
    );
    result(&mut p, "c0", "Task #1 created successfully: Lire");
    result(&mut p, "c1", "Task #12 created successfully: Écrire");
    assert_eq!(ids(&p), ["1", "12", "3"]);
    assert_eq!(p.tasks[2].title, "Tester");
}

#[test]
fn a_task_create_that_failed_is_not_a_task() {
    let mut p = PlanState::default();
    tool(&mut p, "c0", "TaskCreate", json!({ "subject": "Lire" }));
    tool(&mut p, "c1", "TaskCreate", json!({ "subject": "Écrire" }));
    let change = p.on_tool_result("c0", true, "InputValidationError", &Value::Null, T0);
    assert_eq!(change, Change::Saved);
    assert_eq!(p.tasks.len(), 1);
    assert_eq!(p.tasks[0].title, "Écrire");
}

#[test]
fn a_result_that_tells_no_id_leaves_the_task_listed() {
    let mut p = PlanState::default();
    tool(&mut p, "c0", "TaskCreate", json!({ "subject": "Lire" }));
    result(&mut p, "c0", "Created.");
    assert_eq!(p.tasks.len(), 1);
    assert_eq!(p.tasks[0].title, "Lire");
}

#[test]
fn a_task_update_goes_by_the_id_of_its_task() {
    let mut p = PlanState::default();
    listed(&mut p, &["Lire", "Écrire", "Tester"]);
    assert_eq!(
        update(
            &mut p,
            "u1",
            json!({ "taskId": "2", "status": "in_progress", "activeForm": "Écrit" })
        ),
        Change::Saved
    );
    assert_eq!(task(&p, "2").status, TaskStatus::InProgress);
    assert_eq!(task(&p, "2").active.as_deref(), Some("Écrit"));
    assert_eq!(task(&p, "1").status, TaskStatus::Pending);
    update(
        &mut p,
        "u2",
        json!({ "taskId": "2", "status": "completed", "subject": "Écrire vite" }),
    );
    assert_eq!(task(&p, "2").status, TaskStatus::Done);
    assert_eq!(task(&p, "2").title, "Écrire vite");
    // An id is a string, or a number a model wrote.
    update(
        &mut p,
        "u3",
        json!({ "taskId": 3, "status": "in_progress" }),
    );
    assert_eq!(task(&p, "3").status, TaskStatus::InProgress);
    // Said again, nothing changes.
    assert_eq!(
        update(
            &mut p,
            "u4",
            json!({ "taskId": "3", "status": "in_progress" })
        ),
        Change::None
    );
    // Several in progress at once is allowed.
    update(
        &mut p,
        "u5",
        json!({ "taskId": "1", "status": "in_progress" }),
    );
    assert_eq!(
        p.tasks
            .iter()
            .filter(|t| t.status == TaskStatus::InProgress)
            .count(),
        2
    );
}

#[test]
fn a_deleted_task_goes() {
    let mut p = PlanState::default();
    listed(&mut p, &["Lire", "Écrire"]);
    assert_eq!(
        update(&mut p, "u1", json!({ "taskId": "1", "status": "deleted" })),
        Change::Saved
    );
    assert_eq!(ids(&p), ["2"]);
    // One that is not there: nothing to delete, and no task made of it.
    assert_eq!(
        update(&mut p, "u2", json!({ "taskId": "9", "status": "deleted" })),
        Change::None
    );
    assert_eq!(ids(&p), ["2"]);
}

#[test]
fn an_update_of_an_unknown_id_makes_the_task() {
    let mut p = PlanState::default();
    listed(&mut p, &["Lire"]);
    update(
        &mut p,
        "u1",
        json!({ "taskId": "9", "status": "in_progress", "subject": "Surprise" }),
    );
    assert_eq!(ids(&p), ["1", "9"]);
    assert_eq!(task(&p, "9").title, "Surprise");
    assert_eq!(task(&p, "9").status, TaskStatus::InProgress);
    // With nothing to call it by, its id.
    update(
        &mut p,
        "u2",
        json!({ "taskId": "10", "status": "completed" }),
    );
    assert_eq!(task(&p, "10").title, "#10");
    assert_eq!(task(&p, "10").status, TaskStatus::Done);
}

#[test]
fn blocked_by_comes_from_both_ends_of_a_dependency() {
    let mut p = PlanState::default();
    listed(&mut p, &["Lire", "Écrire", "Tester"]);
    update(
        &mut p,
        "u1",
        json!({ "taskId": "3", "addBlockedBy": ["1"] }),
    );
    assert_eq!(task(&p, "3").blocked_by, ["1"]);
    update(
        &mut p,
        "u2",
        json!({ "taskId": "3", "addBlockedBy": ["1", "2"] }),
    );
    assert_eq!(task(&p, "3").blocked_by, ["1", "2"]);
    // Task 1 blocks task 2: the same dependency, said from the other end.
    update(&mut p, "u3", json!({ "taskId": "1", "addBlocks": ["2"] }));
    assert_eq!(task(&p, "2").blocked_by, ["1"]);
    assert!(task(&p, "1").blocked_by.is_empty());
}

#[test]
fn a_call_that_comes_twice_counts_once() {
    let mut p = PlanState::default();
    assert_eq!(
        tool(&mut p, "c0", "TaskCreate", json!({ "subject": "Lire" })),
        Change::Saved
    );
    assert_eq!(
        tool(&mut p, "c0", "TaskCreate", json!({ "subject": "Lire" })),
        Change::None
    );
    result(&mut p, "c0", "Task #1 created successfully: Lire");
    assert_eq!(p.tasks.len(), 1);
    // An old update that comes again does not undo the newer one.
    update(
        &mut p,
        "u1",
        json!({ "taskId": "1", "status": "in_progress" }),
    );
    update(
        &mut p,
        "u2",
        json!({ "taskId": "1", "status": "completed" }),
    );
    assert_eq!(
        update(
            &mut p,
            "u1",
            json!({ "taskId": "1", "status": "in_progress" })
        ),
        Change::None
    );
    assert_eq!(task(&p, "1").status, TaskStatus::Done);
}

#[test]
fn a_finished_list_gives_way_to_a_new_one() {
    let mut p = PlanState::default();
    listed(&mut p, &["Lire", "Écrire"]);
    update(
        &mut p,
        "u1",
        json!({ "taskId": "1", "status": "completed" }),
    );
    update(
        &mut p,
        "u2",
        json!({ "taskId": "2", "status": "completed" }),
    );
    launch(
        &mut p,
        "a1",
        json!({ "description": "Aide", "prompt": "task-2-brief.md" }),
    );
    assert_eq!(row(&p, "a1").plan_task.as_deref(), Some("2"));
    tool(&mut p, "c9", "TaskCreate", json!({ "subject": "Nouveau" }));
    assert_eq!(p.tasks.len(), 1);
    assert_eq!(p.tasks[0].title, "Nouveau");
    // What worked on a task of the old list is not the new list's.
    assert_eq!(row(&p, "a1").plan_task, None);
    // A list that is not finished takes the new task after its own.
    tool(&mut p, "c10", "TaskCreate", json!({ "subject": "Suite" }));
    assert_eq!(p.tasks.len(), 2);
}

#[test]
fn only_the_main_thread_keeps_the_task_list() {
    let mut p = PlanState::default();
    launch(&mut p, "a1", json!({ "description": "Aide", "prompt": "" }));
    let from_sub = |p: &mut PlanState, id: &str, name: &str, input: Value| {
        p.on_tool_use(Some("a1"), id, name, &input, T0)
    };
    from_sub(
        &mut p,
        "s1",
        "TaskCreate",
        json!({ "subject": "Du sous-agent" }),
    );
    from_sub(
        &mut p,
        "s2",
        "TodoWrite",
        json!({ "todos": [{ "content": "X", "status": "pending", "activeForm": "X" }] }),
    );
    from_sub(
        &mut p,
        "s3",
        "TaskUpdate",
        json!({ "taskId": "1", "status": "completed" }),
    );
    assert!(p.tasks.is_empty());
    assert_eq!(p.source, None);
}

#[test]
fn listing_and_reading_tasks_change_nothing() {
    let mut p = PlanState::default();
    listed(&mut p, &["Lire"]);
    let before = serde_json::to_value(&p).unwrap();
    assert_eq!(tool(&mut p, "l1", "TaskList", json!({})), Change::None);
    assert_eq!(
        tool(&mut p, "g1", "TaskGet", json!({ "taskId": "1" })),
        Change::None
    );
    assert_eq!(result(&mut p, "l1", "#1 [pending] Lire"), Change::None);
    assert_eq!(serde_json::to_value(&p).unwrap(), before);
}

// ---------- subagents ----------

#[test]
fn a_subagent_in_the_foreground_runs_then_ends_with_its_totals() {
    let mut p = PlanState::default();
    let change = p.on_tool_use(
        None,
        "a1",
        "Agent",
        &json!({ "description": "Chercher le bug", "prompt": "SECRET prompt", "subagent_type": "Explore",
                 "model": "haiku" }),
        5_000,
    );
    assert_eq!(change, Change::Saved);
    assert_eq!(p.launched, 1);
    let a = row(&p, "a1");
    assert_eq!(a.title, "Chercher le bug");
    assert_eq!(a.kind.as_deref(), Some("Explore"));
    assert_eq!(a.model.as_deref(), Some("haiku"));
    assert_eq!(a.status, RunStatus::Running);
    assert!(!a.background);
    assert_eq!((a.tools, a.started_at, a.ended_at), (0, 5_000, None));

    // What its own calls tell: the count and the step.
    // Counted without a word: what the subagent does is what is worth telling.
    assert_eq!(child(&mut p, "a1", "k1", "Read"), Change::None);
    assert_eq!(
        p.on_child_activity("a1", Some("Lit src/a.ts")),
        Change::View
    );
    assert_eq!(row(&p, "a1").tools, 1);
    assert_eq!(row(&p, "a1").doing.as_deref(), Some("Lit src/a.ts"));
    // The same step again is not news.
    assert_eq!(
        p.on_child_activity("a1", Some("Lit src/a.ts")),
        Change::None
    );
    child(&mut p, "a1", "k2", "Edit");
    p.on_child_activity("a1", Some("Modifie src/a.ts"));
    assert_eq!(row(&p, "a1").tools, 2);
    assert_eq!(row(&p, "a1").doing.as_deref(), Some("Modifie src/a.ts"));
    // A tool without words leaves the step as it was.
    child(&mut p, "a1", "k3", "TodoWrite");
    assert_eq!(p.on_child_activity("a1", None), Change::None);
    assert_eq!(row(&p, "a1").doing.as_deref(), Some("Modifie src/a.ts"));
    // The same call twice is one tool.
    assert_eq!(child(&mut p, "a1", "k3", "TodoWrite"), Change::None);
    assert_eq!(row(&p, "a1").tools, 3);

    let change = p.on_tool_result("a1", false, "Trouvé", &finished(7, 2_500, 4_200), 9_000);
    assert_eq!(change, Change::Saved);
    let a = row(&p, "a1");
    assert_eq!(a.status, RunStatus::Done);
    assert_eq!((a.tools, a.tokens), (7, Some(4_200)));
    assert_eq!(a.ended_at, Some(7_500));
    assert_eq!(a.doing, None);
    // Once ended, its steps no longer show.
    assert_eq!(
        p.on_child_activity("a1", Some("Lit src/b.ts")),
        Change::None
    );
    assert_eq!(row(&p, "a1").doing, None);
}

#[test]
fn a_subagent_whose_result_is_an_error_failed() {
    let mut p = PlanState::default();
    launch(
        &mut p,
        "a1",
        json!({ "description": "Chercher", "prompt": "" }),
    );
    p.on_tool_result("a1", true, "boom", &Value::Null, 4_000);
    let a = row(&p, "a1");
    assert_eq!(a.status, RunStatus::Failed);
    assert_eq!(a.ended_at, Some(4_000));
}

#[test]
fn task_and_agent_are_the_same_tool() {
    let mut p = PlanState::default();
    tool(&mut p, "a1", "Task", json!({ "description": "Un" }));
    tool(&mut p, "a2", "Agent", json!({ "description": "Deux" }));
    assert_eq!(p.agents.len(), 2);
    assert_eq!(p.launched, 2);
    // A workflow is not a subagent: its task says what it is.
    assert_eq!(
        tool(&mut p, "w1", "Workflow", json!({ "name": "review" })),
        Change::None
    );
    assert_eq!(p.launched, 2);
}

#[test]
fn a_subagent_is_called_by_what_the_call_says_of_it() {
    let mut p = PlanState::default();
    launch(&mut p, "a1", json!({ "name": "qa-mobile", "prompt": "" }));
    launch(
        &mut p,
        "a2",
        json!({ "subagent_type": "Explore", "prompt": "" }),
    );
    launch(&mut p, "a3", json!({ "prompt": "" }));
    assert_eq!(row(&p, "a1").title, "qa-mobile");
    assert_eq!(row(&p, "a2").title, "Explore");
    assert_eq!(row(&p, "a3").title, "Agent");
    assert_eq!(row(&p, "a3").model, None);
    assert_eq!(row(&p, "a3").kind, None);
}

#[test]
fn a_subagent_in_the_background_runs_until_its_notification() {
    let mut p = PlanState::default();
    launch(
        &mut p,
        "a1",
        json!({ "description": "Tests", "prompt": "", "run_in_background": true }),
    );
    assert!(row(&p, "a1").background);
    // The result of the call only says it started.
    let launched = json!({ "status": "async_launched", "agentId": "bg1" });
    p.on_tool_result(
        "a1",
        false,
        "Async agent launched successfully.\nagentId: bg1",
        &launched,
        T0,
    );
    assert_eq!(row(&p, "a1").status, RunStatus::Running);
    assert_eq!(sys(&mut p, started("bg1", "a1")), Change::None);
    let end = json!({ "type": "system", "subtype": "task_notification", "task_id": "bg1",
        "tool_use_id": "a1", "status": "completed", "summary": "fini",
        "usage": { "total_tokens": 900, "tool_uses": 3, "duration_ms": 4_000 } });
    assert_eq!(p.on_system(&end, 6_000), Change::Saved);
    let a = row(&p, "a1");
    assert_eq!(a.status, RunStatus::Done);
    assert_eq!((a.tools, a.tokens, a.ended_at), (3, Some(900), Some(6_000)));
}

#[test]
fn a_launch_result_in_words_alone_is_a_background_launch_too() {
    let mut p = PlanState::default();
    launch(
        &mut p,
        "a1",
        json!({ "description": "Tests", "prompt": "" }),
    );
    assert!(!row(&p, "a1").background);
    let change = result(
        &mut p,
        "a1",
        "Async agent launched successfully.\nagentId: bg1 (internal ID)",
    );
    assert_eq!(change, Change::Saved);
    assert!(row(&p, "a1").background);
    assert_eq!(row(&p, "a1").status, RunStatus::Running);
}

#[test]
fn the_notification_status_is_the_rows() {
    for (said, becomes) in [
        ("completed", RunStatus::Done),
        ("failed", RunStatus::Failed),
        ("stopped", RunStatus::Stopped),
    ] {
        let mut p = PlanState::default();
        launch(
            &mut p,
            "a1",
            json!({ "description": "Tests", "prompt": "", "run_in_background": true }),
        );
        // Found by its tool call alone, as when no `task_started` came.
        assert_eq!(sys(&mut p, notified("bg1", "a1", said)), Change::Saved);
        assert_eq!(row(&p, "a1").status, becomes, "{said}");
    }
}

#[test]
fn a_task_update_ends_the_row_it_names() {
    for (said, becomes) in [
        ("completed", RunStatus::Done),
        ("failed", RunStatus::Failed),
        ("killed", RunStatus::Stopped),
    ] {
        let mut p = PlanState::default();
        launch(
            &mut p,
            "a1",
            json!({ "description": "Tests", "prompt": "", "run_in_background": true }),
        );
        sys(&mut p, started("bg1", "a1"));
        assert_eq!(
            sys(&mut p, updated("bg1", json!({ "status": "running" }))),
            Change::None
        );
        assert_eq!(
            sys(&mut p, updated("bg1", json!({ "status": "paused" }))),
            Change::None
        );
        assert_eq!(row(&p, "a1").status, RunStatus::Running);
        assert_eq!(
            sys(&mut p, updated("bg1", json!({ "status": said }))),
            Change::Saved
        );
        assert_eq!(row(&p, "a1").status, becomes, "{said}");
    }
}

#[test]
fn a_subagent_sent_to_the_background_later_is_one() {
    let mut p = PlanState::default();
    launch(
        &mut p,
        "a1",
        json!({ "description": "Tests", "prompt": "" }),
    );
    sys(&mut p, started("t1", "a1"));
    assert!(!row(&p, "a1").background);
    sys(&mut p, updated("t1", json!({ "is_backgrounded": true })));
    assert!(row(&p, "a1").background);
    // It then survives the end of the turn.
    p.end_turn(T0);
    assert_eq!(row(&p, "a1").status, RunStatus::Running);
}

#[test]
fn the_notification_and_the_result_of_a_foreground_subagent_come_in_any_order() {
    // The notification first: it ended the row, the result does not open it again.
    let mut p = PlanState::default();
    launch(
        &mut p,
        "a1",
        json!({ "description": "Tests", "prompt": "" }),
    );
    sys(&mut p, started("t1", "a1"));
    p.on_system(&notified("t1", "a1", "failed"), 2_000);
    p.on_tool_result("a1", false, "ok", &finished(4, 100, 50), 3_000);
    let a = row(&p, "a1");
    assert_eq!(a.status, RunStatus::Failed);
    assert_eq!(a.ended_at, Some(2_000));
    // The totals still complete it.
    assert_eq!((a.tools, a.tokens), (4, Some(50)));

    // The result first: a later notification does not change how it ended.
    let mut p = PlanState::default();
    launch(
        &mut p,
        "a1",
        json!({ "description": "Tests", "prompt": "" }),
    );
    sys(&mut p, started("t1", "a1"));
    p.on_tool_result("a1", false, "ok", &finished(4, 100, 50), 3_000);
    assert_eq!(
        p.on_system(&notified("t1", "a1", "failed"), 4_000),
        Change::None
    );
    assert_eq!(row(&p, "a1").status, RunStatus::Done);
    assert_eq!(row(&p, "a1").ended_at, Some(1_100));
}

#[test]
fn a_subagent_of_a_subagent_has_its_own_row() {
    let mut p = PlanState::default();
    launch(&mut p, "a1", json!({ "description": "Chef", "prompt": "" }));
    let change = p.on_tool_use(
        Some("a1"),
        "a2",
        "Agent",
        &json!({ "description": "Aide", "prompt": "" }),
        T0,
    );
    assert_eq!(change, Change::Saved);
    assert_eq!(p.launched, 2);
    // It is a tool the first used.
    assert_eq!(row(&p, "a1").tools, 1);
    child(&mut p, "a2", "k1", "Read");
    p.on_child_activity("a2", Some("Lit"));
    assert_eq!(row(&p, "a2").tools, 1);
    assert_eq!(row(&p, "a2").doing.as_deref(), Some("Lit"));
    assert_eq!(row(&p, "a1").tools, 1);
    assert_eq!(row(&p, "a1").doing, None);
}

#[test]
fn a_subagent_works_on_the_task_its_prompt_names() {
    let mut p = PlanState::default();
    listed(&mut p, &["Lire", "Écrire", "Tester"]);
    update(
        &mut p,
        "u1",
        json!({ "taskId": "1", "status": "in_progress" }),
    );
    update(
        &mut p,
        "u2",
        json!({ "taskId": "2", "status": "in_progress" }),
    );
    let link = |p: &mut PlanState, id: &str, prompt: &str, description: &str| {
        launch(
            p,
            id,
            json!({ "description": description, "prompt": prompt }),
        );
        row(p, id).plan_task.clone()
    };
    let brief = "Read C:\\repo\\.superpowers\\sdd\\plan\\task-3-brief.md and do it";
    assert_eq!(
        link(&mut p, "a1", brief, "Implémenter").as_deref(),
        Some("3")
    );
    let report = "Review the work: /repo/.superpowers/sdd/plan/task-2-report.md";
    assert_eq!(link(&mut p, "a2", report, "Relire").as_deref(), Some("2"));
    // The file's name comes before the words.
    assert_eq!(link(&mut p, "a3", brief, "Task 1").as_deref(), Some("3"));
    // The description, when the prompt names nothing.
    assert_eq!(
        link(&mut p, "a4", "Fais-le", "Implement Task 3: Tester").as_deref(),
        Some("3")
    );
    assert_eq!(link(&mut p, "a5", "", "Task #2").as_deref(), Some("2"));
    // An id the list does not have names nothing; two in progress: no single one to take.
    assert_eq!(
        link(&mut p, "a6", "task-9-brief.md", "Task 7").as_deref(),
        None
    );
}

#[test]
fn without_a_name_a_subagent_takes_the_one_task_in_progress() {
    let mut p = PlanState::default();
    listed(&mut p, &["Lire", "Écrire"]);
    // None in progress yet.
    launch(
        &mut p,
        "a1",
        json!({ "description": "Aide", "prompt": "Fais-le" }),
    );
    assert_eq!(row(&p, "a1").plan_task, None);
    update(
        &mut p,
        "u1",
        json!({ "taskId": "2", "status": "in_progress" }),
    );
    launch(
        &mut p,
        "a2",
        json!({ "description": "Aide", "prompt": "Fais-le" }),
    );
    assert_eq!(row(&p, "a2").plan_task.as_deref(), Some("2"));
    update(
        &mut p,
        "u2",
        json!({ "taskId": "1", "status": "in_progress" }),
    );
    launch(
        &mut p,
        "a3",
        json!({ "description": "Aide", "prompt": "Fais-le" }),
    );
    assert_eq!(row(&p, "a3").plan_task, None);
    // The link is made when it starts, not redone.
    assert_eq!(row(&p, "a2").plan_task.as_deref(), Some("2"));
}

#[test]
fn a_task_id_may_be_a_word() {
    let mut p = PlanState::default();
    update(
        &mut p,
        "u1",
        json!({ "taskId": "L1", "subject": "Langues" }),
    );
    update(
        &mut p,
        "u2",
        json!({ "taskId": "K4", "subject": "Comptes" }),
    );
    launch(
        &mut p,
        "a1",
        json!({ "description": "Aide", "prompt": "Brief: task-K4-brief.md" }),
    );
    assert_eq!(row(&p, "a1").plan_task.as_deref(), Some("K4"));
    launch(
        &mut p,
        "a2",
        json!({ "description": "Aide", "prompt": "Do Task L1: the languages" }),
    );
    assert_eq!(row(&p, "a2").plan_task.as_deref(), Some("L1"));
}

#[test]
fn a_deleted_task_is_no_longer_worked_on() {
    let mut p = PlanState::default();
    listed(&mut p, &["Lire", "Écrire"]);
    launch(
        &mut p,
        "a1",
        json!({ "description": "Aide", "prompt": "task-2-brief.md" }),
    );
    update(&mut p, "u1", json!({ "taskId": "2", "status": "deleted" }));
    assert_eq!(row(&p, "a1").plan_task, None);
}

#[test]
fn tasks_that_are_not_agents_are_not_counted() {
    let mut p = PlanState::default();
    let bash = json!({ "subtype": "task_started", "task_id": "b1", "tool_use_id": "tb",
                       "description": "npm test", "task_type": "local_bash" });
    assert_eq!(sys(&mut p, bash), Change::None);
    for flag in ["ambient", "skip_transcript"] {
        let mut f = started("x1", "tx");
        f[flag] = json!(true);
        assert_eq!(sys(&mut p, f), Change::None, "{flag}");
    }
    let no_type = json!({ "subtype": "task_started", "task_id": "n1", "tool_use_id": "tn", "description": "?" });
    assert_eq!(sys(&mut p, no_type), Change::None);
    assert!(p.is_empty());
    assert_eq!(p.launched, 0);
    // Nor are their notifications.
    assert_eq!(sys(&mut p, notified("b1", "tb", "completed")), Change::None);
}

#[test]
fn a_task_started_without_a_call_opens_a_row_that_the_call_then_completes() {
    let mut p = PlanState::default();
    listed(&mut p, &["Lire"]);
    update(
        &mut p,
        "u1",
        json!({ "taskId": "1", "status": "in_progress" }),
    );
    let mut f = started("t1", "a1");
    f["subagent_type"] = json!("Explore");
    f["is_backgrounded"] = json!(true);
    f["prompt"] = json!("SECRET prompt");
    assert_eq!(sys(&mut p, f), Change::Saved);
    assert_eq!(p.launched, 1);
    let a = row(&p, "a1");
    assert_eq!(
        (a.title.as_str(), a.kind.as_deref()),
        ("Chercher", Some("Explore"))
    );
    assert!(a.background);
    assert_eq!(a.plan_task.as_deref(), Some("1"));
    // Its call comes after: it adds what it knows, and it is not a second subagent.
    let change = launch(
        &mut p,
        "a1",
        json!({ "description": "Chercher", "prompt": "", "model": "sonnet" }),
    );
    assert_eq!(change, Change::Saved);
    assert_eq!(p.launched, 1);
    assert_eq!(p.agents.len(), 1);
    assert_eq!(row(&p, "a1").model.as_deref(), Some("sonnet"));
    assert!(!serde_json::to_string(&p).unwrap().contains("SECRET"));
}

#[test]
fn what_nothing_opened_is_ignored() {
    let mut p = PlanState::default();
    assert_eq!(
        sys(&mut p, notified("zzz", "tz", "completed")),
        Change::None
    );
    assert_eq!(
        sys(&mut p, updated("zzz", json!({ "status": "completed" }))),
        Change::None
    );
    let progress = json!({ "subtype": "task_progress", "task_id": "zzz", "tool_use_id": "tz",
        "description": "Phase : agent", "usage": { "total_tokens": 1, "tool_uses": 1, "duration_ms": 1 } });
    assert_eq!(sys(&mut p, progress), Change::None);
    assert_eq!(p.on_child_activity("zzz", Some("Lit")), Change::None);
    assert_eq!(child(&mut p, "zzz", "k1", "Read"), Change::None);
    assert!(p.is_empty());
}

#[test]
fn the_count_of_launched_subagents_never_goes_down() {
    let mut p = PlanState::default();
    for i in 0..3 {
        let id = format!("a{i}");
        launch(&mut p, &id, json!({ "description": "Aide", "prompt": "" }));
        p.on_tool_result(&id, false, "ok", &finished(1, 1, 1), T0);
        assert_eq!(p.launched, i + 1);
    }
    assert_eq!(p.agents.len(), 3);
    assert_eq!(p.launched, 3);
}

// ---------- workflows ----------

#[test]
fn a_workflow_runs_from_its_task_to_its_notification() {
    let mut p = PlanState::default();
    let start = json!({ "subtype": "task_started", "task_id": "w1", "tool_use_id": "tw",
        "description": "Revue du dépôt", "task_type": "local_workflow", "workflow_name": "review" });
    assert_eq!(p.on_system(&start, 2_000), Change::Saved);
    let w = &p.workflows[0];
    assert_eq!(
        (w.id.as_str(), w.name.as_deref(), w.title.as_str()),
        ("w1", Some("review"), "Revue du dépôt")
    );
    assert_eq!(
        (w.status, w.tools, w.tokens, w.started_at),
        (RunStatus::Running, 0, 0, 2_000)
    );
    // Not a subagent.
    assert!(p.agents.is_empty());
    assert_eq!(p.launched, 0);
    // Said again, still one.
    assert_eq!(p.on_system(&start, 2_000), Change::None);
    assert_eq!(p.workflows.len(), 1);

    let progress = |desc: &str, tokens: u64, tools: u64| {
        json!({ "subtype": "task_progress", "task_id": "w1", "tool_use_id": "tw", "description": desc,
            "usage": { "total_tokens": tokens, "tool_uses": tools, "duration_ms": 10 }, "last_tool_name": "lecteur A" })
    };
    assert_eq!(
        sys(&mut p, progress("Phase 1 : lecteur A", 1_200, 4)),
        Change::View
    );
    let w = &p.workflows[0];
    assert_eq!(
        (w.now.as_deref(), w.tools, w.tokens),
        (Some("Phase 1 : lecteur A"), 4, 1_200)
    );
    assert_eq!(
        sys(&mut p, progress("Phase 2 : lecteur B", 3_000, 9)),
        Change::View
    );
    let w = &p.workflows[0];
    assert_eq!(
        (w.now.as_deref(), w.tools, w.tokens),
        (Some("Phase 2 : lecteur B"), 9, 3_000)
    );
    assert_eq!(
        sys(&mut p, progress("Phase 2 : lecteur B", 3_000, 9)),
        Change::None
    );

    let end = json!({ "subtype": "task_notification", "task_id": "w1", "tool_use_id": "tw",
        "status": "completed", "summary": "fini" });
    assert_eq!(p.on_system(&end, 9_000), Change::Saved);
    let w = &p.workflows[0];
    assert_eq!(
        (w.status, w.ended_at, w.now.as_deref()),
        (RunStatus::Done, Some(9_000), None)
    );
    // Once ended, a late progress changes nothing.
    assert_eq!(
        sys(&mut p, progress("Phase 3 : lecteur C", 5_000, 12)),
        Change::None
    );
    assert_eq!(p.workflows[0].tokens, 3_000);
}

#[test]
fn a_workflow_may_fail_or_be_stopped() {
    for (said, becomes) in [
        ("failed", RunStatus::Failed),
        ("stopped", RunStatus::Stopped),
    ] {
        let mut p = PlanState::default();
        sys(
            &mut p,
            json!({ "subtype": "task_started", "task_id": "w1", "tool_use_id": "tw",
            "description": "Revue", "task_type": "local_workflow" }),
        );
        assert_eq!(p.workflows[0].name, None);
        sys(&mut p, notified("w1", "tw", said));
        assert_eq!(p.workflows[0].status, becomes);
    }
}

// ---------- reset, interruption ----------

#[test]
fn a_reset_forgets_everything() {
    let mut p = PlanState::default();
    assert_eq!(p.reset(), Change::None);
    listed(&mut p, &["Lire"]);
    launch(&mut p, "a1", json!({ "description": "Aide", "prompt": "" }));
    sys(
        &mut p,
        json!({ "subtype": "task_started", "task_id": "w1", "tool_use_id": "tw",
        "description": "Revue", "task_type": "local_workflow" }),
    );
    assert!(!p.is_empty());
    assert_eq!(p.reset(), Change::Saved);
    assert!(p.is_empty());
    assert_eq!((p.launched, p.source), (0, None));
    assert_eq!(
        serde_json::to_value(&p).unwrap(),
        serde_json::to_value(PlanState::default()).unwrap()
    );
    // A call seen before is news again (a new conversation reuses nothing, but ids are not forever).
    assert_eq!(
        tool(&mut p, "c0", "TaskCreate", json!({ "subject": "Lire" })),
        Change::Saved
    );
}

#[test]
fn a_plan_is_finished_when_nothing_is_left_to_do_or_to_wait_for() {
    let mut p = PlanState::default();
    // Not a plan at all.
    assert!(p.is_finished());
    listed(&mut p, &["Lire", "Écrire"]);
    assert!(!p.is_finished());
    update(
        &mut p,
        "u1",
        json!({ "taskId": "1", "status": "completed" }),
    );
    assert!(!p.is_finished());
    update(
        &mut p,
        "u2",
        json!({ "taskId": "2", "status": "in_progress" }),
    );
    assert!(!p.is_finished());
    update(
        &mut p,
        "u3",
        json!({ "taskId": "2", "status": "completed" }),
    );
    assert!(p.is_finished());
    // A subagent still running is something to wait for.
    launch(
        &mut p,
        "a1",
        json!({ "description": "Aide", "prompt": "", "run_in_background": true }),
    );
    assert!(!p.is_finished());
    sys(&mut p, notified("bg", "a1", "completed"));
    assert!(p.is_finished());
    sys(
        &mut p,
        json!({ "subtype": "task_started", "task_id": "w1", "tool_use_id": "tw",
        "description": "Revue", "task_type": "local_workflow" }),
    );
    assert!(!p.is_finished());
}

#[test]
fn what_ran_in_a_process_that_is_gone_is_interrupted() {
    let mut p = PlanState::default();
    listed(&mut p, &["Lire", "Écrire"]);
    update(
        &mut p,
        "u1",
        json!({ "taskId": "1", "status": "in_progress" }),
    );
    launch(&mut p, "a1", json!({ "description": "Un", "prompt": "" }));
    launch(
        &mut p,
        "a2",
        json!({ "description": "Deux", "prompt": "", "run_in_background": true }),
    );
    launch(
        &mut p,
        "a3",
        json!({ "description": "Trois", "prompt": "" }),
    );
    p.on_tool_result("a3", false, "ok", &finished(1, 1, 1), 2_000);
    child(&mut p, "a1", "k1", "Read");
    p.on_child_activity("a1", Some("Lit"));
    sys(
        &mut p,
        json!({ "subtype": "task_started", "task_id": "w1", "tool_use_id": "tw",
        "description": "Revue", "task_type": "local_workflow" }),
    );

    assert_eq!(p.close_running(8_000), Change::Saved);
    for id in ["a1", "a2"] {
        let a = row(&p, id);
        assert_eq!(
            (a.status, a.doing.clone(), a.ended_at),
            (RunStatus::Interrupted, None, Some(8_000)),
            "{id}"
        );
    }
    assert_eq!(p.workflows[0].status, RunStatus::Interrupted);
    // What had ended keeps how it ended; the tasks keep theirs, the agent picks them up again.
    assert_eq!(row(&p, "a3").status, RunStatus::Done);
    assert_eq!(task(&p, "1").status, TaskStatus::InProgress);
    assert_eq!(p.close_running(9_000), Change::None);
}

#[test]
fn the_end_of_a_turn_interrupts_a_subagent_in_the_foreground_only() {
    let mut p = PlanState::default();
    launch(
        &mut p,
        "a1",
        json!({ "description": "Premier plan", "prompt": "" }),
    );
    launch(
        &mut p,
        "a2",
        json!({ "description": "Fond", "prompt": "", "run_in_background": true }),
    );
    sys(
        &mut p,
        json!({ "subtype": "task_started", "task_id": "w1", "tool_use_id": "tw",
        "description": "Revue", "task_type": "local_workflow" }),
    );
    assert_eq!(p.end_turn(7_000), Change::Saved);
    assert_eq!(row(&p, "a1").status, RunStatus::Interrupted);
    assert_eq!(row(&p, "a1").ended_at, Some(7_000));
    assert_eq!(row(&p, "a2").status, RunStatus::Running);
    assert_eq!(p.workflows[0].status, RunStatus::Running);
    assert_eq!(p.end_turn(8_000), Change::None);
}

// ---------- bounds ----------

#[test]
fn a_plan_lists_a_hundred_tasks_at_most() {
    let mut p = PlanState::default();
    for i in 0..150 {
        tool(
            &mut p,
            &format!("c{i}"),
            "TaskCreate",
            json!({ "subject": format!("Tâche {i}") }),
        );
    }
    assert_eq!(p.tasks.len(), MAX_TASKS);
    assert_eq!(p.tasks[99].title, "Tâche 99");
    let todos: Vec<Value> = (0..150)
        .map(|i| json!({ "content": format!("T{i}"), "status": "pending", "activeForm": "x" }))
        .collect();
    let mut q = PlanState::default();
    tool(&mut q, "t1", "TodoWrite", json!({ "todos": todos }));
    assert_eq!(q.tasks.len(), MAX_TASKS);
    // Updates of tasks beyond it make none.
    update(&mut q, "u1", json!({ "taskId": "500", "subject": "Trop" }));
    assert_eq!(q.tasks.len(), MAX_TASKS);
}

#[test]
fn the_rows_of_subagents_keep_those_running_and_the_latest_ended() {
    let mut p = PlanState::default();
    for i in 0..25 {
        let id = format!("a{i}");
        launch(
            &mut p,
            &id,
            json!({ "description": format!("Aide {i}"), "prompt": "" }),
        );
        p.on_tool_result(&id, false, "ok", &finished(1, 1, 1), T0);
    }
    for i in 25..40 {
        launch(
            &mut p,
            &format!("a{i}"),
            json!({ "description": format!("Aide {i}"), "prompt": "" }),
        );
    }
    assert_eq!(p.launched, 40);
    assert_eq!(p.agents.len(), MAX_AGENTS);
    let kept: Vec<&str> = p.agents.iter().map(|a| a.id.as_str()).collect();
    let expected: Vec<String> = (10..40).map(|i| format!("a{i}")).collect();
    assert_eq!(
        kept,
        expected.iter().map(String::as_str).collect::<Vec<_>>()
    );
    assert_eq!(
        p.agents
            .iter()
            .filter(|a| a.status == RunStatus::Running)
            .count(),
        15
    );

    // All running: the oldest go.
    let mut q = PlanState::default();
    for i in 0..40 {
        launch(
            &mut q,
            &format!("a{i}"),
            json!({ "description": "Aide", "prompt": "" }),
        );
    }
    assert_eq!((q.agents.len(), q.launched), (30, 40));
    assert_eq!(q.agents[0].id, "a10");
}

#[test]
fn a_plan_keeps_ten_workflow_runs() {
    let mut p = PlanState::default();
    for i in 0..12 {
        let id = format!("w{i}");
        sys(
            &mut p,
            json!({ "subtype": "task_started", "task_id": id, "tool_use_id": id,
            "description": "Revue", "task_type": "local_workflow" }),
        );
        if i < 11 {
            sys(&mut p, notified(&id, &id, "completed"));
        }
    }
    assert_eq!(p.workflows.len(), MAX_WORKFLOWS);
    // The one still running stays, with the latest ended.
    assert_eq!(p.workflows.last().unwrap().id, "w11");
    assert_eq!(p.workflows.last().unwrap().status, RunStatus::Running);
    assert_eq!(p.workflows[0].id, "w2");
}

#[test]
fn titles_are_one_visible_line_of_a_hundred_and_sixty_characters_at_most() {
    let mut p = PlanState::default();
    tool(
        &mut p,
        "c0",
        "TaskCreate",
        json!({ "subject": "x".repeat(500), "activeForm": "y".repeat(500) }),
    );
    let t = &p.tasks[0];
    assert_eq!(t.title.chars().count(), MAX_TITLE);
    assert!(t.title.ends_with('…'));
    assert_eq!(t.active.as_ref().unwrap().chars().count(), MAX_TITLE);
    // No direction mark, no zero-width character, one line.
    tool(
        &mut p,
        "c1",
        "TaskCreate",
        json!({ "subject": "Lire\u{202e}le\u{200b} code\n\n  vite" }),
    );
    assert_eq!(p.tasks[1].title, "Lirele code vite");
    launch(
        &mut p,
        "a1",
        json!({ "description": "d".repeat(500), "model": "so\u{202e}nnet", "subagent_type": "Ex\u{200b}plore", "prompt": "" }),
    );
    let a = row(&p, "a1");
    assert_eq!(a.title.chars().count(), MAX_TITLE);
    assert_eq!(a.model.as_deref(), Some("sonnet"));
    assert_eq!(a.kind.as_deref(), Some("Explore"));
    p.on_child_activity("a1", Some(&format!("Lit {}", "z".repeat(500))));
    assert_eq!(
        row(&p, "a1").doing.as_ref().unwrap().chars().count(),
        MAX_TITLE
    );
}

#[test]
fn no_prompt_is_kept() {
    let mut p = PlanState::default();
    launch(
        &mut p,
        "a1",
        json!({ "description": "Aide", "prompt": "SECRET-PROMPT task-1-brief.md" }),
    );
    sys(&mut p, {
        let mut f = started("t1", "a1");
        f["prompt"] = json!("SECRET-PROMPT");
        f
    });
    let json = serde_json::to_string(&p).unwrap();
    assert!(!json.contains("SECRET"), "{json}");
}

// ---------- the shape the window gets ----------

#[test]
fn the_plan_reaches_the_window_in_its_own_shape() {
    let mut p = PlanState::default();
    listed(&mut p, &["Lire", "Écrire"]);
    update(
        &mut p,
        "u1",
        json!({ "taskId": "1", "status": "in_progress", "activeForm": "Lit" }),
    );
    update(
        &mut p,
        "u2",
        json!({ "taskId": "2", "addBlockedBy": ["1"] }),
    );
    p.on_tool_use(
        None,
        "a1",
        "Agent",
        &json!({ "description": "Aide", "prompt": "p", "subagent_type": "Explore", "model": "haiku",
                 "run_in_background": true }),
        50,
    );
    p.on_system(
        &json!({ "subtype": "task_started", "task_id": "w1", "tool_use_id": "tw",
            "description": "Revue", "task_type": "local_workflow", "workflow_name": "review" }),
        60,
    );
    assert_eq!(
        serde_json::to_value(&p).unwrap(),
        json!({
            "source": "tools",
            "tasks": [
                { "id": "1", "title": "Lire", "status": "inProgress", "active": "Lit" },
                { "id": "2", "title": "Écrire", "status": "pending", "blockedBy": ["1"] },
            ],
            "agents": [{ "id": "a1", "title": "Aide", "kind": "Explore", "model": "haiku",
                "status": "running", "background": true, "tools": 0, "startedAt": 50, "planTask": "1" }],
            "launched": 1,
            "workflows": [{ "id": "w1", "name": "review", "title": "Revue", "status": "running",
                "tools": 0, "tokens": 0, "startedAt": 60 }],
        })
    );
    // Nothing yet: the window gets a plan with nothing in it, source null.
    assert_eq!(
        serde_json::to_value(PlanState::default()).unwrap(),
        json!({ "source": null, "tasks": [], "agents": [], "launched": 0, "workflows": [] })
    );
}

#[test]
fn a_saved_plan_is_read_back() {
    let mut p = PlanState::default();
    listed(&mut p, &["Lire"]);
    launch(&mut p, "a1", json!({ "description": "Aide", "prompt": "" }));
    child(&mut p, "a1", "k1", "Read");
    p.on_child_activity("a1", Some("Lit"));
    let saved = serde_json::to_value(&p).unwrap();
    let back: PlanState = serde_json::from_value(saved.clone()).unwrap();
    assert_eq!(serde_json::to_value(&back).unwrap(), saved);
    // A call seen before the save is not remembered: the process is another one.
    assert!(back.seen.is_empty());
    // Written by an older version, or by hand: what is missing is empty.
    let old: PlanState =
        serde_json::from_value(json!({ "tasks": [{ "id": "1", "title": "A" }] })).unwrap();
    assert_eq!(old.tasks[0].status, TaskStatus::Pending);
    assert!(old.agents.is_empty());
    let none: PlanState = serde_json::from_value(json!({})).unwrap();
    assert!(none.is_empty());
}

// ---------- the plan of a file ----------

use crate::planfiles::FileList;

fn file_task(id: &str, title: &str, status: TaskStatus) -> PlanTask {
    PlanTask {
        id: id.into(),
        title: title.into(),
        status,
        ..PlanTask::default()
    }
}

fn files(file: &str, title: Option<&str>, tasks: Vec<PlanTask>) -> Option<FileList> {
    Some(FileList {
        plan_file: file.into(),
        title: title.map(String::from),
        tasks,
    })
}

fn demo_files() -> Option<FileList> {
    files(
        "docs/superpowers/plans/demo.md",
        Some("Démo"),
        vec![
            file_task("1", "Un", TaskStatus::Done),
            file_task("2", "Deux", TaskStatus::InProgress),
            file_task("3", "Trois", TaskStatus::Pending),
        ],
    )
}

#[test]
fn the_tasks_of_a_plan_file_fill_a_plan_that_has_none() {
    let mut p = PlanState::default();
    assert_eq!(p.set_files(demo_files()), Change::Saved);
    assert_eq!(p.source, Some(PlanSource::Plan));
    assert_eq!(
        p.plan_file.as_deref(),
        Some("docs/superpowers/plans/demo.md")
    );
    assert_eq!(p.title.as_deref(), Some("Démo"));
    assert_eq!(ids(&p), ["1", "2", "3"]);
    assert_eq!(task(&p, "2").status, TaskStatus::InProgress);
    assert!(!p.is_empty());
    // The same files again: nothing to tell.
    assert_eq!(p.set_files(demo_files()), Change::None);
    // A change of status or of steps is told.
    let mut list = demo_files().unwrap();
    list.tasks[2].status = TaskStatus::InProgress;
    assert_eq!(p.set_files(Some(list.clone())), Change::Saved);
    list.tasks[2].steps = Some((1, 4));
    assert_eq!(p.set_files(Some(list)), Change::Saved);
    assert_eq!(task(&p, "3").steps, Some((1, 4)));
}

#[test]
fn nothing_read_changes_nothing() {
    let mut p = PlanState::default();
    assert_eq!(p.set_files(None), Change::None);
    assert!(p.is_empty());
    p.set_files(demo_files());
    // A read that failed, or found nothing: what was known stays.
    assert_eq!(p.set_files(None), Change::None);
    assert_eq!(ids(&p), ["1", "2", "3"]);
    assert_eq!(p.source, Some(PlanSource::Plan));
}

#[test]
fn the_list_of_the_agent_comes_before_the_one_of_the_files() {
    let mut p = PlanState::default();
    listed(&mut p, &["Lire", "Écrire"]);
    assert_eq!(p.source, Some(PlanSource::Tools));
    // The files tell the plan's name and title, not its tasks: the window gets one list.
    assert_eq!(p.set_files(demo_files()), Change::Saved);
    assert_eq!(p.source, Some(PlanSource::Tools));
    assert_eq!(ids(&p), ["1", "2"]);
    assert_eq!(task(&p, "1").title, "Lire");
    assert_eq!(task(&p, "1").status, TaskStatus::Pending);
    assert_eq!(
        p.plan_file.as_deref(),
        Some("docs/superpowers/plans/demo.md")
    );
    assert_eq!(p.title.as_deref(), Some("Démo"));
    assert_eq!(p.set_files(demo_files()), Change::None);
}

#[test]
fn an_agent_whose_list_is_empty_gets_the_one_of_the_files() {
    let mut p = PlanState::default();
    // A subagent was launched, no task was listed.
    launch(&mut p, "a1", json!({ "description": "Aide", "prompt": "" }));
    assert!(p.tasks.is_empty());
    assert_eq!(p.set_files(demo_files()), Change::Saved);
    assert_eq!(p.source, Some(PlanSource::Plan));
    assert_eq!(ids(&p), ["1", "2", "3"]);
    assert_eq!(p.agents.len(), 1, "the subagents are the stream's");
    assert_eq!(p.launched, 1);
}

#[test]
fn a_task_done_is_never_pending_again() {
    let mut p = PlanState::default();
    p.set_files(demo_files());
    // The ledger lost its lines (cut, rewritten): the plan says the tasks are to do again.
    let again = files(
        "docs/superpowers/plans/demo.md",
        Some("Démo"),
        vec![
            file_task("1", "Un", TaskStatus::Pending),
            file_task("2", "Deux", TaskStatus::Pending),
            file_task("3", "Trois", TaskStatus::Pending),
        ],
    );
    assert_eq!(p.set_files(again), Change::Saved);
    assert_eq!(task(&p, "1").status, TaskStatus::Done);
    // What was only in progress is not held to it.
    assert_eq!(task(&p, "2").status, TaskStatus::Pending);
    // The ledger itself can say a task is at work again (a fix round after its complete).
    let mut list = demo_files().unwrap();
    list.tasks[0].status = TaskStatus::InProgress;
    assert_eq!(p.set_files(Some(list)), Change::Saved);
    assert_eq!(task(&p, "1").status, TaskStatus::InProgress);
}

#[test]
fn another_plan_starts_afresh() {
    let mut p = PlanState::default();
    p.set_files(demo_files());
    let other = files(
        "docs/superpowers/plans/other.md",
        Some("Autre"),
        vec![
            file_task("1", "Un autre", TaskStatus::Pending),
            file_task("9", "Neuf", TaskStatus::Pending),
        ],
    );
    assert_eq!(p.set_files(other), Change::Saved);
    // Task 1 of this plan is not the first plan's task 1: it is not done.
    assert_eq!(task(&p, "1").status, TaskStatus::Pending);
    assert_eq!(ids(&p), ["1", "9"]);
    assert_eq!(p.title.as_deref(), Some("Autre"));
    assert_eq!(
        p.plan_file.as_deref(),
        Some("docs/superpowers/plans/other.md")
    );
}

#[test]
fn a_plan_without_tasks_is_no_plan() {
    let mut p = PlanState::default();
    let empty = files("docs/superpowers/plans/empty.md", Some("Rien"), vec![]);
    assert_eq!(p.set_files(empty.clone()), Change::None);
    assert!(p.is_empty() && p.title.is_none() && p.plan_file.is_none());
    assert_eq!(p.source, None);
    // The plan was edited and lost its tasks: what was read of it goes.
    p.set_files(demo_files());
    assert_eq!(p.set_files(empty.clone()), Change::Saved);
    assert!(p.tasks.is_empty() && p.title.is_none() && p.plan_file.is_none());
    assert_eq!(p.source, None);
    // The agent's own list does not go for it.
    let mut q = PlanState::default();
    listed(&mut q, &["Lire"]);
    q.set_files(empty);
    assert_eq!(ids(&q), ["1"]);
    assert_eq!(q.source, Some(PlanSource::Tools));
}

#[test]
fn a_subagent_is_linked_to_a_task_of_the_files_by_its_brief() {
    let mut p = PlanState::default();
    p.set_files(files(
        "docs/superpowers/plans/big.md",
        None,
        vec![
            file_task("L1", "Langues", TaskStatus::Done),
            file_task("K4", "Comptes", TaskStatus::InProgress),
            file_task("K5", "Barre", TaskStatus::Pending),
        ],
    ));
    launch(
        &mut p,
        "a1",
        json!({ "description": "Implémente K5", "prompt": "Read .superpowers/sdd/big/task-K5-brief.md" }),
    );
    assert_eq!(row(&p, "a1").plan_task.as_deref(), Some("K5"));
    // Without a name, the one task in progress.
    launch(
        &mut p,
        "a2",
        json!({ "description": "Aide", "prompt": "x" }),
    );
    assert_eq!(row(&p, "a2").plan_task.as_deref(), Some("K4"));
    // The plan changes: a link to a task it no longer has goes.
    p.set_files(files(
        "docs/superpowers/plans/big.md",
        None,
        vec![file_task("L1", "Langues", TaskStatus::Done)],
    ));
    assert_eq!(row(&p, "a1").plan_task, None);
    assert_eq!(row(&p, "a2").plan_task, None);
}

#[test]
fn the_task_tools_take_the_list_over_from_the_files() {
    // A task made: the list of the files makes room, the plan keeps its name and title.
    let mut p = PlanState::default();
    p.set_files(demo_files());
    assert_eq!(
        tool(&mut p, "c1", "TaskCreate", json!({ "subject": "Mienne" })),
        Change::Saved
    );
    assert_eq!(p.source, Some(PlanSource::Tools));
    assert_eq!(p.tasks.len(), 1);
    assert_eq!(p.tasks[0].title, "Mienne");
    assert_eq!(p.title.as_deref(), Some("Démo"));
    // The files go on being read, and no longer change the list.
    assert_eq!(p.set_files(demo_files()), Change::None);
    assert_eq!(p.tasks.len(), 1);

    // A todo list does too.
    let mut p = PlanState::default();
    p.set_files(demo_files());
    tool(
        &mut p,
        "t1",
        "TodoWrite",
        json!({ "todos": [{ "content": "Lire", "status": "pending", "activeForm": "Lit" }] }),
    );
    assert_eq!(p.source, Some(PlanSource::Tools));
    assert_eq!(ids(&p), ["1"]);
    assert_eq!(task(&p, "1").title, "Lire");

    // An update of a task it does not know makes it, as ever: the files' tasks are not its own.
    let mut p = PlanState::default();
    p.set_files(demo_files());
    update(
        &mut p,
        "u1",
        json!({ "taskId": "2", "status": "completed" }),
    );
    assert_eq!(p.source, Some(PlanSource::Tools));
    assert_eq!(ids(&p), ["2"]);
    assert_eq!(task(&p, "2").title, "#2");
    assert_eq!(task(&p, "2").status, TaskStatus::Done);
}

#[test]
fn what_the_task_tools_do_nothing_with_leaves_the_files_alone() {
    let mut p = PlanState::default();
    p.set_files(demo_files());
    // An empty todo list, a deletion of a task it does not have, a read.
    assert_eq!(
        tool(&mut p, "t1", "TodoWrite", json!({ "todos": [] })),
        Change::None
    );
    assert_eq!(
        update(&mut p, "u1", json!({ "taskId": "2", "status": "deleted" })),
        Change::None
    );
    tool(&mut p, "g1", "TaskList", json!({}));
    assert_eq!(p.source, Some(PlanSource::Plan));
    assert_eq!(ids(&p), ["1", "2", "3"]);
}

#[test]
fn a_new_plan_forgets_the_files_too() {
    let mut p = PlanState::default();
    p.set_files(demo_files());
    assert_eq!(p.reset(), Change::Saved);
    assert!(p.is_empty() && p.title.is_none() && p.plan_file.is_none() && p.source.is_none());
}

#[test]
fn a_plan_of_the_files_goes_to_the_window_with_its_steps() {
    let mut p = PlanState::default();
    let mut list = demo_files().unwrap();
    list.tasks[1].steps = Some((2, 5));
    p.set_files(Some(list));
    assert_eq!(
        serde_json::to_value(&p).unwrap(),
        json!({
            "source": "plan",
            "planFile": "docs/superpowers/plans/demo.md",
            "title": "Démo",
            "tasks": [
                { "id": "1", "title": "Un", "status": "done" },
                { "id": "2", "title": "Deux", "status": "inProgress", "steps": [2, 5] },
                { "id": "3", "title": "Trois", "status": "pending" },
            ],
            "agents": [],
            "launched": 0,
            "workflows": [],
        })
    );
    let back: PlanState = serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap();
    assert_eq!(back.source, Some(PlanSource::Plan));
    assert_eq!(task(&back, "2").steps, Some((2, 5)));
}
