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
fn the_totals_a_subagent_reports_are_kept_without_a_word_as_the_window_shows_neither_tools_nor_tokens(
) {
    let mut p = PlanState::default();
    launch(
        &mut p,
        "a1",
        json!({ "description": "Tests", "prompt": "", "run_in_background": true }),
    );
    let launched = json!({ "status": "async_launched", "agentId": "bg1" });
    p.on_tool_result("a1", false, "Async agent launched", &launched, T0);
    sys(&mut p, started("bg1", "a1"));
    let progress = |tools: u64, tokens: u64| {
        json!({ "subtype": "task_progress", "task_id": "bg1", "tool_use_id": "a1",
            "description": "Tests", "usage": { "total_tokens": tokens, "tool_uses": tools, "duration_ms": 10 } })
    };
    // Each progress frame of a subagent brings new totals; none is worth sending the window the
    // whole agent for (a tool call of a subagent would send it every time).
    assert_eq!(sys(&mut p, progress(2, 500)), Change::None);
    assert_eq!((row(&p, "a1").tools, row(&p, "a1").tokens), (2, Some(500)));
    assert_eq!(sys(&mut p, progress(3, 900)), Change::None);
    assert_eq!((row(&p, "a1").tools, row(&p, "a1").tokens), (3, Some(900)));
    // The totals of an ended row only complete it, nothing to tell either.
    let end = json!({ "type": "system", "subtype": "task_notification", "task_id": "bg1",
        "tool_use_id": "a1", "status": "completed", "summary": "fini" });
    assert_eq!(p.on_system(&end, 6_000), Change::Saved);
    assert_eq!(
        p.on_tool_result("a1", false, "ok", &finished(5, 1, 1_500), T0),
        Change::None
    );
    assert_eq!(
        (row(&p, "a1").tools, row(&p, "a1").tokens),
        (5, Some(1_500))
    );
    // A workflow's phase is shown: its totals ride along with it.
    let start = json!({ "subtype": "task_started", "task_id": "w1", "tool_use_id": "tw",
        "description": "Revue", "task_type": "local_workflow", "workflow_name": "review" });
    p.on_system(&start, 2_000);
    let wf = json!({ "subtype": "task_progress", "task_id": "w1", "tool_use_id": "tw",
        "description": "Phase 1", "usage": { "total_tokens": 10, "tool_uses": 1, "duration_ms": 10 } });
    assert_eq!(sys(&mut p, wf), Change::View);
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
            "bgTasks": [],
        })
    );
    // Nothing yet: the window gets a plan with nothing in it, source null.
    assert_eq!(
        serde_json::to_value(PlanState::default()).unwrap(),
        json!({ "source": null, "tasks": [], "agents": [], "launched": 0, "workflows": [], "bgTasks": [] })
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
    assert_eq!(back.seen.len(), 0);
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
fn a_run_that_ended_leaves_every_task_of_the_files_done() {
    let mut p = PlanState::default();
    p.set_files(demo_files());
    assert_eq!(p.finish_files(), Change::Saved);
    assert!(p.tasks.iter().all(|t| t.status == TaskStatus::Done));
    assert_eq!(p.source, Some(PlanSource::Plan));
    // The plan keeps its name and title, and is the same plan: nothing to tell again.
    assert_eq!(p.title.as_deref(), Some("Démo"));
    assert_eq!(p.finish_files(), Change::None);
    assert!(p.is_finished());
    // The files, read again without their workspace (the plan alone), do not reopen what was done.
    let mut list = demo_files().unwrap();
    for task in &mut list.tasks {
        task.status = TaskStatus::Pending;
    }
    assert_eq!(p.set_files(Some(list)), Change::None);
    assert!(p.tasks.iter().all(|t| t.status == TaskStatus::Done));

    // The agent's own list is its own: it says when a task is done.
    let mut p = PlanState::default();
    listed(&mut p, &["Lire", "Deux"]);
    assert_eq!(p.finish_files(), Change::None);
    assert!(p.tasks.iter().all(|t| t.status == TaskStatus::Pending));
    // And nothing at all is nothing to finish.
    assert_eq!(PlanState::default().finish_files(), Change::None);
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
    listed(&mut p, &["Lire", "Deux"]);
    assert_eq!(p.source, Some(PlanSource::Tools));
    // The files tell the plan's name and title (the agent's second task is the plan's), not its
    // tasks: the window gets one list.
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
    // A task made: the list of the files makes room.
    let mut p = PlanState::default();
    p.set_files(demo_files());
    assert_eq!(
        tool(&mut p, "c1", "TaskCreate", json!({ "subject": "Mienne" })),
        Change::Saved
    );
    assert_eq!(p.source, Some(PlanSource::Tools));
    assert_eq!(p.tasks.len(), 1);
    assert_eq!(p.tasks[0].title, "Mienne");
    // The plan of the files has made room altogether: what names it comes back if the agent's
    // tasks are the plan's.
    assert!(p.title.is_none() && p.plan_file.is_none());
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
            "bgTasks": [],
        })
    );
    let back: PlanState = serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap();
    assert_eq!(back.source, Some(PlanSource::Plan));
    assert_eq!(task(&back, "2").steps, Some((2, 5)));
}

// ---------- fix round 1 ----------

#[test]
fn a_copy_of_a_plan_shares_the_calls_it_has_seen_and_is_equal_to_it() {
    let mut p = PlanState::default();
    for i in 0..600 {
        tool(
            &mut p,
            &format!("c{i}"),
            "TaskCreate",
            json!({ "subject": format!("T{i}") }),
        );
    }
    assert_eq!(p.seen.len(), 512);
    // The window's view copies the plan at each frame: the 512 ids are not copied with it.
    let q = p.clone();
    assert!(p.seen.same_ring(&q.seen));
    assert_eq!(p, q);
    // What was said of the calls stays true for the copy that goes on: a repeat is a repeat.
    let mut r = p.clone();
    assert_eq!(
        tool(&mut r, "c599", "TaskCreate", json!({ "subject": "T599" })),
        Change::None
    );
    // And a plan that starts again forgets them.
    r.reset();
    assert_eq!(r.seen.len(), 0);
    assert!(!r.seen.same_ring(&p.seen));
}

#[test]
fn the_files_title_an_agent_list_only_when_a_task_title_is_the_plans() {
    // Not one title the plan has: the plan of someone else in the same repository.
    let mut p = PlanState::default();
    listed(&mut p, &["Lire", "Écrire"]);
    assert_eq!(p.set_files(demo_files()), Change::None);
    assert!(p.title.is_none() && p.plan_file.is_none());

    // Titles of the plan, as the skills' todos and a model write them.
    for (title, name) in [
        ("deux", "case"),
        ("  DEUX. ", "spaces and stops"),
        ("Task 2: Deux", "the skills' « Task N: » form"),
        ("task 2 - deux", "the same, loosely"),
    ] {
        let mut p = PlanState::default();
        listed(&mut p, &["Lire", title]);
        assert_eq!(p.set_files(demo_files()), Change::Saved, "{name}");
        assert_eq!(p.title.as_deref(), Some("Démo"), "{name}");
        assert_eq!(p.source, Some(PlanSource::Tools));
        assert_eq!(task(&p, "2").title, title.trim().to_string(), "{name}");
    }

    // A title that only holds one of the plan's, when that is long enough to be one.
    let long = |plan: &str| {
        files(
            "docs/superpowers/plans/long.md",
            Some("Long"),
            vec![file_task("3", plan, TaskStatus::Pending)],
        )
    };
    let mut p = PlanState::default();
    listed(&mut p, &["Implement the hook installer for Task 3"]);
    assert_eq!(p.set_files(long("Hook installer")), Change::Saved);
    // Short ones are the same title or none: « Un » is in « Un autre ».
    let mut p = PlanState::default();
    listed(&mut p, &["Un autre"]);
    assert_eq!(p.set_files(demo_files()), Change::None);
    assert!(p.title.is_none());

    // The agent's list moves on to another plan's tasks: the name goes.
    let mut p = PlanState::default();
    listed(&mut p, &["Deux"]);
    p.set_files(demo_files());
    assert_eq!(p.title.as_deref(), Some("Démo"));
    tool(
        &mut p,
        "t1",
        "TodoWrite",
        json!({ "todos": [{ "content": "Autre chose", "status": "pending", "activeForm": "x" }] }),
    );
    assert_eq!(p.set_files(demo_files()), Change::Saved);
    assert!(p.title.is_none() && p.plan_file.is_none());
    assert_eq!(p.set_files(demo_files()), Change::None);
}

#[test]
fn the_name_of_a_plan_file_is_one_visible_line_of_a_hundred_and_sixty_characters() {
    let mut p = PlanState::default();
    let odd = format!("docs/superpowers/plans/{}\u{202e}.md\n", "é".repeat(300));
    p.set_files(files(
        &odd,
        Some("Démo"),
        vec![file_task("1", "Un", TaskStatus::Pending)],
    ));
    let file = p.plan_file.clone().unwrap();
    assert_eq!(file.chars().count(), MAX_TITLE);
    assert!(file.ends_with('…') && !file.contains('\n') && !file.contains('\u{202e}'));
    // Stable: the same files again are nothing new.
    assert_eq!(
        p.set_files(files(
            &odd,
            Some("Démo"),
            vec![file_task("1", "Un", TaskStatus::Pending)]
        )),
        Change::None
    );
    let mut q = PlanState::default();
    q.set_files(files(
        "docs/superpowers/plans/a\u{200b}b\u{202e}.md",
        None,
        vec![file_task("1", "Un", TaskStatus::Pending)],
    ));
    assert_eq!(q.plan_file.as_deref(), Some("docs/superpowers/plans/ab.md"));
}

#[test]
fn an_empty_todo_list_with_subagents_is_a_plan_with_no_tasks() {
    // What the window keys on to know whether the agent has a task list (documented for it).
    let mut p = PlanState::default();
    tool(
        &mut p,
        "t1",
        "TodoWrite",
        json!({ "todos": [{ "content": "Lire", "status": "pending", "activeForm": "Lit" }] }),
    );
    launch(&mut p, "a1", json!({ "description": "Aide", "prompt": "" }));
    tool(&mut p, "t2", "TodoWrite", json!({ "todos": [] }));
    assert!(p.tasks.is_empty());
    assert_eq!(
        p.source,
        Some(PlanSource::Tools),
        "the source says who made it, not that it is not empty"
    );
    assert!(!p.is_empty(), "the subagent is still there");
    // Files then fill the empty list, as for an agent that never had one.
    assert_eq!(p.set_files(demo_files()), Change::Saved);
    assert_eq!(p.source, Some(PlanSource::Plan));
}

#[test]
fn a_subagent_launched_by_a_subagent_names_the_one_that_launched_it() {
    let mut p = PlanState::default();
    launch(&mut p, "a1", json!({ "description": "Chef", "prompt": "" }));
    // Its call comes from the first one's thread.
    let change = p.on_tool_use(
        Some("a1"),
        "a2",
        "Agent",
        &json!({ "description": "Aide", "prompt": "" }),
        T0,
    );
    assert_eq!(change, Change::Saved);
    assert_eq!(row(&p, "a1").parent, None);
    assert_eq!(row(&p, "a2").parent.as_deref(), Some("a1"));
    // Seen first by its task, its parent's task names the parent.
    sys(&mut p, started("bg1", "a1"));
    let nested = json!({ "type": "system", "subtype": "task_started", "task_id": "bg3",
        "tool_use_id": "a3", "description": "Petit", "task_type": "local_agent",
        "parent_task_id": "bg1" });
    assert_eq!(sys(&mut p, nested), Change::Saved);
    assert_eq!(row(&p, "a3").parent.as_deref(), Some("a1"));
    // An unknown parent task names nothing.
    let orphan = json!({ "type": "system", "subtype": "task_started", "task_id": "bg4",
        "tool_use_id": "a4", "description": "Seul", "task_type": "local_agent",
        "parent_task_id": "nobody" });
    sys(&mut p, orphan);
    assert_eq!(row(&p, "a4").parent, None);
}

#[test]
fn the_window_is_told_the_task_of_a_subagent_which_stops_it_and_not_before_it_is_known() {
    let mut p = PlanState::default();
    launch(
        &mut p,
        "a1",
        json!({ "description": "Tests", "prompt": "" }),
    );
    let sent = serde_json::to_value(&p).unwrap();
    assert!(sent["agents"][0].get("taskId").is_none());
    sys(&mut p, started("bg1", "a1"));
    let sent = serde_json::to_value(&p).unwrap();
    assert_eq!(sent["agents"][0]["taskId"], "bg1");
    // Kept with the plan: read back, it is still there.
    let back: PlanState = serde_json::from_value(sent).unwrap();
    assert_eq!(back.agents[0].task_id.as_deref(), Some("bg1"));
}

#[test]
fn a_subagent_stopped_by_stop_task_ends_stopped_as_a_real_claude_code_tells_it() {
    // What Claude Code 2.1.300 sent for `stop_task` on a running subagent (measured, docs/PROTOCOL.md):
    // the task killed, then its notification « stopped », with no usage.
    let mut p = PlanState::default();
    launch(
        &mut p,
        "toolu_1",
        json!({ "description": "Slow sleeper", "subagent_type": "general-purpose", "model": "haiku", "prompt": "…" }),
    );
    sys(
        &mut p,
        json!({ "type": "system", "subtype": "task_started", "task_id": "a3c0",
        "tool_use_id": "toolu_1", "description": "Slow sleeper", "subagent_type": "general-purpose",
        "is_backgrounded": true, "spawn_depth": 1, "task_type": "local_agent", "prompt": "…" }),
    );
    let progress = json!({ "type": "system", "subtype": "task_progress", "task_id": "a3c0",
        "tool_use_id": "toolu_1", "description": "Running sleep",
        "usage": { "total_tokens": 27_068, "tool_uses": 3, "duration_ms": 6_406 }, "last_tool_name": "Bash" });
    sys(&mut p, progress);
    let killed = json!({ "type": "system", "subtype": "task_updated", "task_id": "a3c0",
        "patch": { "status": "killed", "end_time": 1_792_064_308_799_i64 } });
    assert_eq!(p.on_system(&killed, 9_000), Change::Saved);
    let stopped = json!({ "type": "system", "subtype": "task_notification", "task_id": "a3c0",
        "tool_use_id": "toolu_1", "status": "stopped", "output_file": "x.output", "summary": "Slow sleeper" });
    assert_eq!(p.on_system(&stopped, 9_001), Change::None);
    let a = row(&p, "toolu_1");
    assert_eq!(a.status, RunStatus::Stopped);
    assert_eq!(
        (a.tools, a.tokens, a.ended_at),
        (3, Some(27_068), Some(9_000))
    );
}

// ---------- tasks in the background: commands, monitors, remote agents, teammates (ESC-14) ----------

/// The call of a command the main thread runs in the background.
fn bash(p: &mut PlanState, id: &str, command: &str, description: &str) -> Change {
    tool(
        p,
        id,
        "Bash",
        json!({ "command": command, "description": description, "run_in_background": true }),
    )
}

fn bg<'a>(p: &'a PlanState, id: &str) -> &'a BgTask {
    p.bg_tasks
        .iter()
        .find(|t| t.id == id)
        .expect("a background task")
}

fn bg_ids(p: &PlanState) -> Vec<&str> {
    p.bg_tasks.iter().map(|t| t.id.as_str()).collect()
}

/// The `task_started` of a command (`local_bash`); `background` is `is_backgrounded` when it is said.
fn bg_started(task: &str, call: &str, description: &str, background: Option<bool>) -> Value {
    let mut f = json!({ "type": "system", "subtype": "task_started", "task_id": task,
        "run_id": format!("r-{task}"), "tool_use_id": call, "description": description,
        "task_type": "local_bash" });
    if let Some(b) = background {
        f["is_backgrounded"] = json!(b);
    }
    f
}

/// `background_tasks_changed`: the tasks alive, each `(task id, task type, description)`.
fn level(tasks: &[(&str, &str, &str)]) -> Value {
    let tasks: Vec<Value> = tasks
        .iter()
        .map(|(id, kind, d)| json!({ "task_id": id, "run_id": format!("r-{id}"), "task_type": kind, "description": d }))
        .collect();
    json!({ "type": "system", "subtype": "background_tasks_changed", "tasks": tasks })
}

fn bg_notified(task: &str, call: &str, status: Option<&str>, summary: &str, file: &str) -> Value {
    let mut f = json!({ "type": "system", "subtype": "task_notification", "task_id": task,
        "tool_use_id": call, "output_file": file, "summary": summary });
    if let Some(s) = status {
        f["status"] = json!(s);
    }
    f
}

/// The text of the result of a command sent to the background (measured on Claude Code 2.1.301).
fn launched_text(task: &str) -> String {
    format!("Command running in background with ID: {task}. Output is being written to: /tmp/claude/s1/tasks/{task}.output. You will be notified when it completes. To check interim output, use Read.")
}

fn background_id(task: &str) -> Value {
    json!({ "stdout": "", "stderr": "", "interrupted": false, "backgroundTaskId": task })
}

#[test]
fn a_command_in_the_background_runs_from_its_level_to_its_notification() {
    // The frames of a real Claude Code 2.1.301 (docs/PROTOCOL.md), in their order.
    let mut p = PlanState::default();
    let command = "for i in 1 2 3; do echo tick $i; sleep 2; done";
    // Its call says the command; nothing of it is shown yet.
    assert_eq!(bash(&mut p, "c1", command, "Ticker"), Change::None);
    assert!(p.is_empty());
    // The level comes first: the row opens with what it says.
    assert_eq!(
        sys(&mut p, level(&[("b1", "local_bash", "Ticker")])),
        Change::Saved
    );
    let t = bg(&p, "b1");
    assert_eq!(
        (t.kind, t.title.as_str(), t.status, t.command.as_deref()),
        (BgKind::Command, "Ticker", RunStatus::Running, None)
    );
    // Then its task: the command comes from the call, and the row is not opened twice.
    let start = bg_started("b1", "c1", "Ticker", Some(true));
    assert_eq!(p.on_system(&start, 2_000), Change::Saved);
    assert_eq!(p.bg_tasks.len(), 1);
    let t = bg(&p, "b1");
    assert_eq!(t.command.as_deref(), Some(command));
    assert_eq!(t.call.as_deref(), Some("c1"));
    assert_eq!((t.started_at, t.parent.as_deref()), (T0, None));
    assert_eq!(p.on_system(&start, 3_000), Change::None);
    // Its result tells where the output is written.
    let tur = background_id("b1");
    assert_eq!(
        p.on_tool_result("c1", false, &launched_text("b1"), &tur, T0),
        Change::Saved
    );
    assert_eq!(
        bg(&p, "b1").output_file.as_deref(),
        Some("/tmp/claude/s1/tasks/b1.output")
    );
    assert_eq!(p.bg_tasks.len(), 1);
    // Neither a subagent nor a workflow.
    assert!(p.agents.is_empty() && p.workflows.is_empty() && p.launched == 0);
    // Its end: the task, then its notification with its words and its file.
    let end = updated("b1", json!({ "status": "completed", "end_time": 5 }));
    assert_eq!(p.on_system(&end, 9_000), Change::Saved);
    let t = bg(&p, "b1");
    assert_eq!(
        (t.status, t.ended_at, t.summary.as_deref()),
        (RunStatus::Done, Some(9_000), None)
    );
    let words = "Background command \"Ticker\" completed (exit code 0)";
    let note = bg_notified(
        "b1",
        "c1",
        Some("completed"),
        words,
        "/tmp/claude/s1/tasks/b1.output",
    );
    assert_eq!(sys(&mut p, note.clone()), Change::Saved);
    let t = bg(&p, "b1");
    assert_eq!((t.status, t.ended_at), (RunStatus::Done, Some(9_000)));
    assert_eq!(t.summary.as_deref(), Some(words));
    // The level that follows has not got it: nothing changes. The same notification again neither.
    assert_eq!(sys(&mut p, level(&[])), Change::None);
    assert_eq!(sys(&mut p, note), Change::None);
    assert_eq!(p.bg_tasks.len(), 1);
}

#[test]
fn a_command_ends_as_its_task_or_its_notification_says() {
    for (patch, note, becomes) in [
        ("completed", "completed", RunStatus::Done),
        ("failed", "failed", RunStatus::Failed),
        ("killed", "stopped", RunStatus::Stopped),
    ] {
        // The task first, then the notification.
        let mut p = PlanState::default();
        sys(&mut p, bg_started("b1", "c1", "Job", Some(true)));
        assert_eq!(
            sys(&mut p, updated("b1", json!({ "status": patch }))),
            Change::Saved
        );
        assert_eq!(bg(&p, "b1").status, becomes, "{patch}");
        sys(
            &mut p,
            bg_notified("b1", "c1", Some(note), "Job", "/x/b1.output"),
        );
        assert_eq!(bg(&p, "b1").status, becomes, "{note}");
        assert_eq!(bg(&p, "b1").output_file.as_deref(), Some("/x/b1.output"));
        // The notification alone ends it too (its file, when it has none, is not kept).
        let mut q = PlanState::default();
        sys(&mut q, bg_started("b1", "c1", "Job", Some(true)));
        assert_eq!(
            sys(&mut q, bg_notified("b1", "c1", Some(note), "Job", "")),
            Change::Saved
        );
        let t = bg(&q, "b1");
        assert_eq!(
            (t.status, t.ended_at, t.output_file.as_deref()),
            (becomes, Some(T0), None)
        );
        assert_eq!(t.summary.as_deref(), Some("Job"));
    }
    // `killed` in a notification stops it too.
    let mut p = PlanState::default();
    sys(&mut p, bg_started("b1", "c1", "Job", Some(true)));
    sys(&mut p, bg_notified("b1", "c1", Some("killed"), "Job", ""));
    assert_eq!(bg(&p, "b1").status, RunStatus::Stopped);
    // A row that ended never opens again: not by a word, not by its task said again.
    sys(&mut p, updated("b1", json!({ "status": "failed" })));
    sys(
        &mut p,
        bg_notified("b1", "c1", Some("completed"), "Job", ""),
    );
    assert_eq!(bg(&p, "b1").status, RunStatus::Stopped);
    assert_eq!(
        sys(&mut p, bg_started("b1", "c1", "Job", Some(true))),
        Change::None
    );
    assert_eq!(bg(&p, "b1").status, RunStatus::Stopped);
    assert_eq!(
        p.on_tool_result("c1", false, "ok", &background_id("b1"), T0),
        Change::None
    );
    assert_eq!(bg(&p, "b1").status, RunStatus::Stopped);
}

#[test]
fn a_monitor_is_told_from_a_command_by_the_tool_that_called_it() {
    let mut p = PlanState::default();
    tool(
        &mut p,
        "m1",
        "Monitor",
        json!({ "command": "tail -f app.log", "description": "Watch log", "timeout_ms": 60000 }),
    );
    bash(&mut p, "c1", "sleep 5", "Sleeper");
    // Both are `local_bash` tasks, and the level cannot tell them apart.
    sys(
        &mut p,
        level(&[
            ("w1", "local_bash", "Watch log"),
            ("b1", "local_bash", "Sleeper"),
        ]),
    );
    assert_eq!(bg(&p, "w1").kind, BgKind::Command);
    sys(&mut p, bg_started("w1", "m1", "Watch log", Some(true)));
    sys(&mut p, bg_started("b1", "c1", "Sleeper", Some(true)));
    assert_eq!(bg(&p, "w1").kind, BgKind::Monitor);
    assert_eq!(bg(&p, "w1").command.as_deref(), Some("tail -f app.log"));
    assert_eq!(bg(&p, "b1").kind, BgKind::Command);
    // The Monitor of an MCP server is a monitor without any call.
    let mcp = json!({ "subtype": "task_started", "task_id": "mm", "tool_use_id": "tm",
        "description": "Feed", "task_type": "monitor_mcp" });
    assert_eq!(sys(&mut p, mcp), Change::Saved);
    assert_eq!(bg(&p, "mm").kind, BgKind::Monitor);
    assert_eq!(bg(&p, "mm").status, RunStatus::Running);
}

#[test]
fn an_event_of_a_monitor_replayed_without_a_status_does_not_end_it() {
    let mut p = PlanState::default();
    tool(
        &mut p,
        "m1",
        "Monitor",
        json!({ "command": "tail -f x", "description": "Watch" }),
    );
    sys(&mut p, bg_started("w1", "m1", "Watch", Some(true)));
    let event = json!({ "type": "system", "subtype": "task_notification", "task_id": "w1",
        "tool_use_id": "m1", "summary": "Monitor event: \"watch 1\"", "output_file": "/x/w1.output" });
    assert_eq!(sys(&mut p, event.clone()), Change::None);
    for status in ["", "running", "event"] {
        let mut f = event.clone();
        f["status"] = json!(status);
        assert_eq!(sys(&mut p, f), Change::None, "{status:?}");
    }
    let t = bg(&p, "w1");
    assert_eq!(
        (t.status, t.summary.as_deref(), t.output_file.as_deref()),
        (RunStatus::Running, None, None)
    );
    // Its end does.
    let end = bg_notified(
        "w1",
        "m1",
        Some("completed"),
        "Monitor \"Watch\" stream ended",
        "/x/w1.output",
    );
    assert_eq!(sys(&mut p, end), Change::Saved);
    let t = bg(&p, "w1");
    assert_eq!(t.status, RunStatus::Done);
    assert_eq!(t.summary.as_deref(), Some("Monitor \"Watch\" stream ended"));
}

#[test]
fn a_command_in_the_foreground_is_listed_once_it_goes_to_the_background() {
    let mut p = PlanState::default();
    let command = "for i in 1 2; do echo fg $i; sleep 2; done";
    tool(
        &mut p,
        "c1",
        "Bash",
        json!({ "command": command, "description": "Foreground loop" }),
    );
    // In the foreground, its task is not listed.
    let start = bg_started("b2", "c1", "Foreground loop", Some(false));
    assert_eq!(p.on_system(&start, 5_000), Change::None);
    assert!(p.is_empty());
    // Sent to the background: the row opens, from when it started.
    let flip = updated("b2", json!({ "is_backgrounded": true }));
    assert_eq!(sys(&mut p, flip.clone()), Change::Saved);
    let t = bg(&p, "b2");
    assert_eq!(
        (t.kind, t.title.as_str(), t.status, t.started_at),
        (
            BgKind::Command,
            "Foreground loop",
            RunStatus::Running,
            5_000
        )
    );
    assert_eq!(
        (t.command.as_deref(), t.call.as_deref()),
        (Some(command), Some("c1"))
    );
    // The result that says so completes it: one row.
    let text = "Command was manually backgrounded by user with ID: b2. Output is being written to: /t/tasks/b2.output. If it is still running after 30m, it will be killed.";
    let tur = json!({ "backgroundTaskId": "b2", "backgroundedByUser": true });
    assert_eq!(p.on_tool_result("c1", false, text, &tur, T0), Change::Saved);
    assert_eq!(p.bg_tasks.len(), 1);
    assert_eq!(
        bg(&p, "b2").output_file.as_deref(),
        Some("/t/tasks/b2.output")
    );
    // The update said again changes nothing, and the end is its notification's.
    assert_eq!(sys(&mut p, flip), Change::None);
    sys(
        &mut p,
        bg_notified(
            "b2",
            "c1",
            Some("completed"),
            "Background command \"Foreground loop\" completed (exit code 0)",
            "/t/tasks/b2.output",
        ),
    );
    assert_eq!(bg(&p, "b2").status, RunStatus::Done);
}

#[test]
fn the_level_that_lists_a_command_sent_to_the_background_opens_its_row_from_when_it_started() {
    // The order of a real Claude Code: the task in the foreground, the level that has it, then the update.
    let mut p = PlanState::default();
    tool(
        &mut p,
        "c1",
        "Bash",
        json!({ "command": "sleep 9", "description": "Loop" }),
    );
    p.on_system(&bg_started("b2", "c1", "Loop", Some(false)), 5_000);
    assert!(p.is_empty());
    assert_eq!(
        sys(&mut p, level(&[("b2", "local_bash", "Loop")])),
        Change::Saved
    );
    let t = bg(&p, "b2");
    assert_eq!(
        (t.started_at, t.command.as_deref(), t.call.as_deref()),
        (5_000, Some("sleep 9"), Some("c1"))
    );
    assert_eq!(
        sys(&mut p, updated("b2", json!({ "is_backgrounded": true }))),
        Change::None
    );
    assert_eq!(p.bg_tasks.len(), 1);
}

#[test]
fn a_command_that_ends_in_the_foreground_is_never_listed() {
    let ends = [
        updated("b2", json!({ "status": "completed" })),
        updated("b2", json!({ "status": "failed" })),
        updated("b2", json!({ "status": "killed" })),
        bg_notified("b2", "c1", Some("completed"), "Loop", ""),
    ];
    for end in ends {
        let mut p = PlanState::default();
        sys(&mut p, bg_started("b2", "c1", "Loop", Some(false)));
        assert_eq!(sys(&mut p, end), Change::None);
        assert!(p.is_empty());
        // It is over: sent to the background after that, it lists nothing.
        assert_eq!(
            sys(&mut p, updated("b2", json!({ "is_backgrounded": true }))),
            Change::None
        );
        assert!(p.is_empty());
    }
    // `local_bash` without any word on it is in the foreground.
    let mut p = PlanState::default();
    assert_eq!(
        sys(&mut p, bg_started("b3", "c3", "Loop", None)),
        Change::None
    );
    assert!(p.is_empty());
}

#[test]
fn a_command_of_a_subagent_is_listed_only_when_it_runs_in_the_background() {
    let mut p = PlanState::default();
    let mut fg = bg_started("s1", "k1", "Sleep 20", Some(false));
    fg["owned_by_subagent"] = json!(true);
    fg["parent_task_id"] = json!("sa1");
    assert_eq!(sys(&mut p, fg), Change::None);
    assert!(p.is_empty());
    let mut back = bg_started("s2", "k2", "Serve", Some(true));
    back["owned_by_subagent"] = json!(true);
    back["parent_task_id"] = json!("sa1");
    assert_eq!(sys(&mut p, back), Change::Saved);
    assert_eq!(bg(&p, "s2").parent.as_deref(), Some("sa1"));
    assert_eq!(p.bg_tasks.len(), 1);
    assert!(p.agents.is_empty() && p.launched == 0);
}

#[test]
fn remote_agents_teammates_and_the_monitors_of_mcp_are_background_tasks_too() {
    for (task_type, kind) in [
        ("remote_agent", BgKind::Remote),
        ("in_process_teammate", BgKind::Teammate),
        ("monitor_mcp", BgKind::Monitor),
    ] {
        let mut p = PlanState::default();
        let f = json!({ "subtype": "task_started", "task_id": "t1", "tool_use_id": "tt",
            "description": "Revue distante", "task_type": task_type });
        // Without a word on the background, they are in it.
        assert_eq!(p.on_system(&f, 7), Change::Saved, "{task_type}");
        let t = bg(&p, "t1");
        assert_eq!(
            (t.kind, t.title.as_str(), t.status, t.started_at),
            (kind, "Revue distante", RunStatus::Running, 7)
        );
        assert_eq!(t.command, None);
        // They end with their notification.
        sys(
            &mut p,
            bg_notified("t1", "tt", Some("completed"), "Done", ""),
        );
        assert_eq!(bg(&p, "t1").status, RunStatus::Done);
        // Said to be in the foreground, they are not listed.
        let mut q = PlanState::default();
        let mut g = f.clone();
        g["is_backgrounded"] = json!(false);
        assert_eq!(sys(&mut q, g), Change::None, "{task_type}");
        assert!(q.is_empty());
    }
}

#[test]
fn the_tasks_claude_code_runs_on_its_own_account_or_of_a_kind_nobody_knows_are_not_listed() {
    let mut p = PlanState::default();
    for (key, value) in [("ambient", json!(true)), ("skip_transcript", json!(true))] {
        let mut f = bg_started("x1", "tx", "Quiet", Some(true));
        f[key] = value;
        assert_eq!(sys(&mut p, f), Change::None, "{key}");
    }
    for task_type in ["dream", "local_whatever"] {
        let mut f = bg_started("x2", "tx", "Odd", Some(true));
        f["task_type"] = json!(task_type);
        assert_eq!(sys(&mut p, f), Change::None, "{task_type}");
    }
    let mut no_id = bg_started("x3", "tx", "No id", Some(true));
    no_id["task_id"] = json!("");
    assert_eq!(sys(&mut p, no_id), Change::None);
    assert!(p.is_empty());
}

#[test]
fn the_level_opens_the_background_tasks_it_lists_and_no_other() {
    let mut p = PlanState::default();
    let f = json!({ "type": "system", "subtype": "background_tasks_changed", "tasks": [
        { "task_id": "b1", "run_id": "r1", "task_type": "local_bash", "description": "Ticker" },
        { "task_id": "sa", "run_id": "r2", "task_type": "local_agent", "subagent_type": "Explore", "description": "Helper" },
        { "task_id": "wf", "task_type": "local_workflow", "description": "Flow" },
        { "task_id": "am", "task_type": "local_bash", "description": "Ambient", "ambient": true },
        { "task_id": "dr", "task_type": "dream", "description": "Dream" },
        { "task_id": "rm", "task_type": "remote_agent", "description": "Remote job" },
        { "task_id": "tm", "task_type": "in_process_teammate", "description": "Mate", "parent_task_id": "lead" },
        { "task_id": "mm", "task_type": "monitor_mcp", "description": "Feed", "awaited": true },
        { "task_type": "local_bash", "description": "No id" },
    ]});
    assert_eq!(sys(&mut p, f.clone()), Change::Saved);
    assert_eq!(bg_ids(&p), ["b1", "rm", "tm", "mm"]);
    let kinds: Vec<_> = p.bg_tasks.iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        [
            BgKind::Command,
            BgKind::Remote,
            BgKind::Teammate,
            BgKind::Monitor
        ]
    );
    assert_eq!(bg(&p, "tm").parent.as_deref(), Some("lead"));
    assert_eq!(bg(&p, "rm").title, "Remote job");
    assert!(p
        .bg_tasks
        .iter()
        .all(|t| t.status == RunStatus::Running && t.command.is_none()));
    // Not the subagents nor the workflows, whose rows come from elsewhere.
    assert!(p.agents.is_empty() && p.workflows.is_empty() && p.launched == 0);
    // The same level again says nothing new.
    assert_eq!(sys(&mut p, f), Change::None);
    // A level with no task, or none at all, on a plan with no row: nothing.
    assert_eq!(sys(&mut PlanState::default(), level(&[])), Change::None);
    assert_eq!(
        sys(
            &mut PlanState::default(),
            json!({ "subtype": "background_tasks_changed" })
        ),
        Change::None
    );
}

#[test]
fn a_task_the_level_no_longer_has_is_over_whatever_a_lost_notification_said() {
    let mut p = PlanState::default();
    sys(
        &mut p,
        level(&[("b1", "local_bash", "A"), ("b2", "local_bash", "B")]),
    );
    assert_eq!(
        p.on_system(&level(&[("b2", "local_bash", "B")]), 9_000),
        Change::Saved
    );
    let t = bg(&p, "b1");
    assert_eq!(
        (t.status, t.ended_at, t.summary.as_deref()),
        (RunStatus::Done, Some(9_000), None)
    );
    assert_eq!(bg(&p, "b2").status, RunStatus::Running);
    // What ended is not ended again, and the level that has it back does not open it again.
    assert_eq!(p.on_system(&level(&[]), 10_000), Change::Saved);
    assert_eq!(bg(&p, "b1").ended_at, Some(9_000));
    assert_eq!(bg(&p, "b2").ended_at, Some(10_000));
    assert_eq!(
        sys(&mut p, level(&[("b1", "local_bash", "A")])),
        Change::None
    );
    assert_eq!(bg(&p, "b1").status, RunStatus::Done);
    // The subagents and the workflows have rows of their own, which a level does not end.
    launch(
        &mut p,
        "a1",
        json!({ "description": "Aide", "prompt": "", "run_in_background": true }),
    );
    sys(
        &mut p,
        json!({ "subtype": "task_started", "task_id": "w1", "tool_use_id": "tw",
        "description": "Flow", "task_type": "local_workflow" }),
    );
    sys(&mut p, level(&[]));
    assert_eq!(row(&p, "a1").status, RunStatus::Running);
    assert_eq!(p.workflows[0].status, RunStatus::Running);
}

#[test]
fn the_title_of_a_task_comes_from_its_task_then_its_call_then_its_command() {
    let mut p = PlanState::default();
    bash(&mut p, "c1", "echo one\necho two", "Call words");
    // The task's own description first.
    sys(&mut p, bg_started("b1", "c1", "Task words", Some(true)));
    assert_eq!(bg(&p, "b1").title, "Task words");
    // Then the call's.
    bash(&mut p, "c2", "echo hi", "Call words");
    sys(&mut p, bg_started("b2", "c2", "", Some(true)));
    assert_eq!(bg(&p, "b2").title, "Call words");
    // Then the first line of the command; else the task's id.
    tool(
        &mut p,
        "c3",
        "Bash",
        json!({ "command": "  \n echo   three\nmore" }),
    );
    sys(&mut p, bg_started("b3", "c3", "", Some(true)));
    assert_eq!(bg(&p, "b3").title, "echo three");
    sys(&mut p, bg_started("b4", "nobody", "", Some(true)));
    assert_eq!(bg(&p, "b4").title, "b4");
    // A long description is cut as every title is; an update changes it.
    sys(&mut p, bg_started("b5", "c5", &"x".repeat(500), Some(true)));
    assert_eq!(bg(&p, "b5").title.chars().count(), 160);
    assert_eq!(
        sys(&mut p, updated("b5", json!({ "description": "Renamed" }))),
        Change::Saved
    );
    assert_eq!(bg(&p, "b5").title, "Renamed");
    assert_eq!(
        sys(&mut p, updated("b5", json!({ "description": "Renamed" }))),
        Change::None
    );
}

#[test]
fn the_command_is_kept_as_the_tool_received_it_up_to_2000_characters() {
    let mut p = PlanState::default();
    let command = format!("echo \"é\\n\"  \t{}", "x".repeat(10));
    tool(
        &mut p,
        "c1",
        "Bash",
        json!({ "command": command, "description": "Raw" }),
    );
    sys(&mut p, bg_started("b1", "c1", "Raw", Some(true)));
    assert_eq!(bg(&p, "b1").command.as_deref(), Some(command.as_str()));
    // 2000 characters whole; beyond, cut with an ellipsis that counts in the 2000.
    for (len, cut) in [(2_000, false), (2_001, true), (6_000, true)] {
        let long = "é".repeat(len);
        let call = format!("l{len}");
        tool(
            &mut p,
            &call,
            "PowerShell",
            json!({ "command": long, "description": "Long" }),
        );
        sys(
            &mut p,
            bg_started(&format!("t{len}"), &call, "Long", Some(true)),
        );
        let kept = bg(&p, &format!("t{len}")).command.clone().unwrap();
        assert_eq!(kept.chars().count(), if cut { 2_000 } else { len }, "{len}");
        assert_eq!(kept.ends_with('…'), cut, "{len}");
    }
    // No command, or an empty one, is none.
    tool(
        &mut p,
        "e1",
        "Bash",
        json!({ "command": "", "description": "Empty" }),
    );
    sys(&mut p, bg_started("b9", "e1", "Empty", Some(true)));
    assert_eq!(bg(&p, "b9").command, None);
}

#[test]
fn the_calls_of_commands_are_remembered_64_at_most_whoever_makes_them() {
    let mut p = PlanState::default();
    // Commands and monitors, of the main thread or a subagent's; no other tool.
    assert_eq!(
        p.on_tool_use(Some("a1"), "k1", "Bash", &json!({ "command": "ls" }), T0),
        Change::None
    );
    tool(&mut p, "k2", "PowerShell", json!({ "command": "dir" }));
    tool(&mut p, "k3", "Monitor", json!({ "command": "tail" }));
    tool(&mut p, "k4", "Read", json!({ "file_path": "a" }));
    assert_eq!(p.memory.calls_len(), 3);
    // A call that comes twice is one.
    tool(&mut p, "k2", "PowerShell", json!({ "command": "dir" }));
    assert_eq!(p.memory.calls_len(), 3);
    assert!(p.is_empty());
    // The first of 70 is forgotten, the last is not.
    for i in 0..70 {
        bash(&mut p, &format!("c{i}"), &format!("echo {i}"), "Job");
    }
    assert_eq!(p.memory.calls_len(), 64);
    sys(&mut p, bg_started("b0", "c0", "Job", Some(true)));
    sys(&mut p, bg_started("b69", "c69", "Job", Some(true)));
    assert_eq!(bg(&p, "b0").command, None);
    assert_eq!(bg(&p, "b69").command.as_deref(), Some("echo 69"));
    // A copy of the plan shares what it remembers and is equal to it; a plan that starts again does not.
    let q = p.clone();
    assert!(p.memory.same_ring(&q.memory));
    assert_eq!(p, q);
    let mut r = p.clone();
    r.reset();
    assert_eq!(r.memory.calls_len(), 0);
    assert!(!r.memory.same_ring(&p.memory));
}

#[test]
fn in_the_claude_code_view_the_result_of_the_call_opens_the_task_and_the_replayed_notification_ends_it(
) {
    let mut p = PlanState::default();
    bash(&mut p, "c1", "npm run dev", "Dev server");
    let file = r"C:\Users\Jean Dupont\AppData\Local\Temp\claude\proj\sess\tasks\b9.output";
    let text = format!("Command running in background with ID: b9. Output is being written to: {file}. You will be notified when it completes.");
    assert_eq!(
        p.on_tool_result("c1", false, &text, &background_id("b9"), 4_000),
        Change::Saved
    );
    let t = bg(&p, "b9");
    assert_eq!(
        (
            t.kind,
            t.title.as_str(),
            t.command.as_deref(),
            t.call.as_deref()
        ),
        (
            BgKind::Command,
            "Dev server",
            Some("npm run dev"),
            Some("c1")
        )
    );
    assert_eq!(
        (t.status, t.started_at, t.output_file.as_deref()),
        (RunStatus::Running, 4_000, Some(file))
    );
    // The same result again opens no second row.
    assert_eq!(
        p.on_tool_result("c1", false, &text, &background_id("b9"), 4_100),
        Change::None
    );
    assert_eq!(p.bg_tasks.len(), 1);
    // A monitor says its task as `taskId`.
    tool(
        &mut p,
        "m1",
        "Monitor",
        json!({ "command": "tail -f x", "description": "Watch", "timeout_ms": 1000 }),
    );
    let tur = json!({ "taskId": "w1", "timeoutMs": 1000, "persistent": false });
    let started = "Monitor started (task w1, expires in 1m unless the source ends first).";
    assert_eq!(
        p.on_tool_result("m1", false, started, &tur, 5_000),
        Change::Saved
    );
    let t = bg(&p, "w1");
    assert_eq!(
        (t.kind, t.title.as_str(), t.command.as_deref()),
        (BgKind::Monitor, "Watch", Some("tail -f x"))
    );
    assert_eq!((t.started_at, t.output_file.as_deref()), (5_000, None));
    // `taskId` of any other call is no task; nor is a result with no id, or one that failed.
    bash(&mut p, "c2", "ls", "List");
    assert_eq!(
        p.on_tool_result("c2", false, "ok", &json!({ "taskId": "zzz" }), T0),
        Change::None
    );
    assert_eq!(
        p.on_tool_result("c2", false, "ok", &Value::Null, T0),
        Change::None
    );
    assert_eq!(
        p.on_tool_result("c2", true, "boom", &background_id("e1"), T0),
        Change::None
    );
    assert_eq!(p.bg_tasks.len(), 2);
    // The notification Claude Code replays to Claude ends the row (here, a command stopped).
    let note = bg_notified("b9", "c1", Some("killed"), "Dev server", file);
    assert_eq!(p.on_system(&note, 6_000), Change::Saved);
    let t = bg(&p, "b9");
    assert_eq!((t.status, t.ended_at), (RunStatus::Stopped, Some(6_000)));
}

#[test]
fn the_output_file_of_a_result_is_kept_only_when_it_is_named_after_its_task() {
    let launched = |path: &str| {
        format!("Command running in background with ID: b1. Output is being written to: {path}. You will be notified when it completes.")
    };
    for (text, kept) in [
        (launched("/t/tasks/b1.output"), Some("/t/tasks/b1.output")),
        (
            r"Output is being written to: C:\t u\tasks\b1.output".to_string(),
            Some(r"C:\t u\tasks\b1.output"),
        ),
        (
            "Output is being written to: /t/tasks/b1.output".to_string(),
            Some("/t/tasks/b1.output"),
        ),
        (launched("/t/tasks/other.output"), None),
        (launched("/t/tasks/b1.output.txt"), None),
        (launched("/t/tasks/xb1.output"), None),
        (launched("/t/tasks/b1.output/../../etc/b1x.output"), None),
        (launched("/t/tasks/b1.output/sub"), None),
        (
            "Output is being written to: \n/t/tasks/b1.output".to_string(),
            None,
        ),
        ("it was written to /t/tasks/b1.output".to_string(), None),
        (launched(""), None),
    ] {
        let mut p = PlanState::default();
        bash(&mut p, "c1", "ls", "List");
        p.on_tool_result("c1", false, &text, &background_id("b1"), T0);
        assert_eq!(bg(&p, "b1").output_file.as_deref(), kept, "{text:?}");
    }
    // A text that names a file and no task opens nothing.
    let mut p = PlanState::default();
    assert_eq!(
        p.on_tool_result(
            "c1",
            false,
            &launched("/t/tasks/b1.output"),
            &Value::Null,
            T0
        ),
        Change::None
    );
    assert!(p.is_empty());
    // The file of a row that has one is not replaced by a result.
    sys(&mut p, bg_started("b1", "c1", "Job", Some(true)));
    sys(&mut p, bg_notified("b1", "c1", None, "", ""));
    p.on_tool_result(
        "c1",
        false,
        &launched("/t/tasks/b1.output"),
        &background_id("b1"),
        T0,
    );
    assert_eq!(
        bg(&p, "b1").output_file.as_deref(),
        Some("/t/tasks/b1.output")
    );
    p.on_tool_result(
        "c1",
        false,
        &launched("/u/tasks/b1.output"),
        &background_id("b1"),
        T0,
    );
    assert_eq!(
        bg(&p, "b1").output_file.as_deref(),
        Some("/t/tasks/b1.output")
    );
}

#[test]
fn the_output_file_of_a_notification_is_never_one_of_another_machine() {
    // The file will be read: a path to a share, a verbatim path or a device would make Windows
    // reach the network, with the user's credentials.
    for (file, kept) in [
        (r"\\nonexistent.invalid\share\tasks\b1.output", None),
        ("//nonexistent.invalid/share/tasks/b1.output", None),
        (r"\/nonexistent.invalid/share/tasks/b1.output", None),
        (r"/\nonexistent.invalid\share\tasks\b1.output", None),
        (r"\\?\UNC\nonexistent.invalid\share\tasks\b1.output", None),
        (r"\\?\C:\Users\x\tasks\b1.output", None),
        (r"\\.\pipe\tasks\b1.output", None),
        // Its own machine: kept.
        (
            r"C:\Users\x\Temp\claude\s1\tasks\b1.output",
            Some(r"C:\Users\x\Temp\claude\s1\tasks\b1.output"),
        ),
        (
            "C:/Users/x/Temp/claude/s1/tasks/b1.output",
            Some("C:/Users/x/Temp/claude/s1/tasks/b1.output"),
        ),
        (
            "/tmp/claude/s1/tasks/b1.output",
            Some("/tmp/claude/s1/tasks/b1.output"),
        ),
    ] {
        let mut p = PlanState::default();
        sys(&mut p, bg_started("b1", "c1", "Job", Some(true)));
        let note = bg_notified("b1", "c1", Some("completed"), "Job done", file);
        // The task ends all the same, with its words: only the file is left out.
        assert_eq!(sys(&mut p, note), Change::Saved, "{file}");
        let t = bg(&p, "b1");
        assert_eq!(
            (t.status, t.summary.as_deref(), t.output_file.as_deref()),
            (RunStatus::Done, Some("Job done"), kept),
            "{file}"
        );
    }
    // A refused path does not replace the file a row has, and a good one comes after it.
    let mut p = PlanState::default();
    sys(&mut p, bg_started("b1", "c1", "Job", Some(true)));
    let unc = r"\\nonexistent.invalid\share\tasks\b1.output";
    sys(
        &mut p,
        bg_notified("b1", "c1", Some("completed"), "Job", unc),
    );
    assert_eq!(bg(&p, "b1").output_file, None);
    sys(
        &mut p,
        bg_notified("b1", "c1", Some("completed"), "Job", "/t/tasks/b1.output"),
    );
    assert_eq!(
        bg(&p, "b1").output_file.as_deref(),
        Some("/t/tasks/b1.output")
    );
    sys(
        &mut p,
        bg_notified("b1", "c1", Some("completed"), "Job", unc),
    );
    assert_eq!(
        bg(&p, "b1").output_file.as_deref(),
        Some("/t/tasks/b1.output")
    );
}

#[test]
fn the_output_file_a_result_tells_is_never_one_of_another_machine() {
    let launched = |path: &str| {
        format!("Command running in background with ID: b1. Output is being written to: {path}. You will be notified when it completes.")
    };
    for (path, kept) in [
        (r"\\nonexistent.invalid\share\tasks\b1.output", None),
        ("//nonexistent.invalid/share/tasks/b1.output", None),
        (r"\\?\UNC\nonexistent.invalid\share\tasks\b1.output", None),
        (r"\\?\C:\Users\x\tasks\b1.output", None),
        (r"\\.\pipe\tasks\b1.output", None),
        (
            r"C:\Users\x\Temp\claude\s1\tasks\b1.output",
            Some(r"C:\Users\x\Temp\claude\s1\tasks\b1.output"),
        ),
        (
            "/tmp/claude/s1/tasks/b1.output",
            Some("/tmp/claude/s1/tasks/b1.output"),
        ),
    ] {
        // A command that opens its row by its result, and one whose row is already open.
        for opened in [false, true] {
            let mut p = PlanState::default();
            bash(&mut p, "c1", "ls", "List");
            if opened {
                sys(&mut p, bg_started("b1", "c1", "List", Some(true)));
            }
            p.on_tool_result("c1", false, &launched(path), &background_id("b1"), T0);
            assert_eq!(
                bg(&p, "b1").output_file.as_deref(),
                kept,
                "{path} (row opened first: {opened})"
            );
        }
    }
    // The text of a monitor, no differently.
    let mut p = PlanState::default();
    tool(
        &mut p,
        "m1",
        "Monitor",
        json!({ "command": "tail -f x", "description": "Watch" }),
    );
    let text = r"Monitor started (task w1). Output is being written to: \\nonexistent.invalid\share\tasks\w1.output";
    let tur = json!({ "taskId": "w1", "timeoutMs": 1000, "persistent": false });
    p.on_tool_result("m1", false, text, &tur, T0);
    assert_eq!(bg(&p, "w1").output_file, None);
}

#[test]
fn the_ten_latest_ended_tasks_are_kept_and_all_that_run_up_to_forty() {
    let mut p = PlanState::default();
    for i in 0..3 {
        sys(
            &mut p,
            bg_started(&format!("r{i}"), "c", "Runs", Some(true)),
        );
    }
    for i in 0..13 {
        let id = format!("f{i}");
        sys(&mut p, bg_started(&id, "c", "Ends", Some(true)));
        p.on_system(&updated(&id, json!({ "status": "completed" })), 100 + i);
    }
    // The three that run first, then the ten latest ended; the oldest ended went first.
    let ids: Vec<String> = bg_ids(&p).iter().map(|s| s.to_string()).collect();
    let mut want: Vec<String> = ["r0", "r1", "r2"].iter().map(|s| s.to_string()).collect();
    want.extend((3..13).map(|i| format!("f{i}")));
    assert_eq!(ids, want);
    // The one that ended last is kept, whenever it started.
    let mut q = PlanState::default();
    for i in 0..12 {
        sys(&mut q, bg_started(&format!("t{i}"), "c", "T", Some(true)));
    }
    for i in (0..12).rev() {
        q.on_system(
            &updated(&format!("t{i}"), json!({ "status": "completed" })),
            100 + (11 - i),
        );
    }
    assert!(q.bg_tasks.iter().any(|t| t.id == "t0"));
    assert!(!q.bg_tasks.iter().any(|t| t.id == "t11"));
    assert_eq!(q.bg_tasks.len(), 10);
    // Forty running at most; the oldest goes, and an ended row goes before any that runs.
    let mut r = PlanState::default();
    for i in 0..45 {
        sys(
            &mut r,
            bg_started(&format!("b{i}"), "c", "Runs", Some(true)),
        );
    }
    assert_eq!(r.bg_tasks.len(), 40);
    assert_eq!(
        (r.bg_tasks[0].id.as_str(), r.bg_tasks[39].id.as_str()),
        ("b5", "b44")
    );
    r.on_system(&updated("b20", json!({ "status": "completed" })), 9_000);
    sys(&mut r, bg_started("b45", "c", "Runs", Some(true)));
    assert_eq!(r.bg_tasks.len(), 40);
    assert!(!r.bg_tasks.iter().any(|t| t.id == "b20"));
    assert_eq!(r.bg_tasks[0].id, "b5");
    assert!(r.bg_tasks.iter().all(|t| t.status == RunStatus::Running));
}

#[test]
fn a_process_gone_interrupts_what_ran_and_the_end_of_a_turn_forgets_only_the_foreground() {
    let mut p = PlanState::default();
    sys(&mut p, bg_started("b1", "c1", "Back", Some(true)));
    sys(&mut p, bg_started("b2", "c2", "Front", Some(false)));
    // The turn ends: the one in the foreground is gone with it, the other goes on.
    assert_eq!(p.memory.foreground_len(), 1);
    assert_eq!(p.end_turn(9_000), Change::None);
    assert_eq!(p.memory.foreground_len(), 0);
    assert_eq!(bg(&p, "b1").status, RunStatus::Running);
    assert_eq!(
        sys(&mut p, updated("b2", json!({ "is_backgrounded": true }))),
        Change::None
    );
    assert_eq!(p.bg_tasks.len(), 1);
    // The process is gone: it is interrupted, and so is nothing twice.
    sys(&mut p, bg_started("b3", "c3", "Front", Some(false)));
    sys(&mut p, bg_started("b4", "c4", "Done", Some(true)));
    sys(&mut p, updated("b4", json!({ "status": "completed" })));
    assert_eq!(p.close_running(9_500), Change::Saved);
    let t = bg(&p, "b1");
    assert_eq!(
        (t.status, t.ended_at),
        (RunStatus::Interrupted, Some(9_500))
    );
    assert_eq!(bg(&p, "b4").status, RunStatus::Done);
    assert_eq!(p.close_running(9_600), Change::None);
    assert_eq!(bg(&p, "b1").ended_at, Some(9_500));
    // And the foreground one with it.
    assert_eq!(p.memory.foreground_len(), 0);
    assert_eq!(
        sys(&mut p, updated("b3", json!({ "is_backgrounded": true }))),
        Change::None
    );
    // Starting again forgets all.
    assert_eq!(p.reset(), Change::Saved);
    assert!(p.bg_tasks.is_empty() && p.is_empty());
    assert_eq!(p.reset(), Change::None);
}

#[test]
fn a_running_background_task_is_something_running_and_keeps_the_plan_unfinished() {
    let mut p = PlanState::default();
    assert!(p.is_empty() && p.is_finished() && !p.runs_something());
    sys(&mut p, bg_started("b1", "c1", "Job", Some(true)));
    assert!(!p.is_empty() && !p.is_finished() && p.runs_something());
    p.on_system(&updated("b1", json!({ "status": "completed" })), 9_000);
    // Over, it is still shown until the plan starts again.
    assert!(!p.is_empty() && p.is_finished() && !p.runs_something());
    // A command in the foreground is not something that runs on.
    let mut q = PlanState::default();
    sys(&mut q, bg_started("b2", "c2", "Job", Some(false)));
    assert!(q.is_empty() && q.is_finished() && !q.runs_something());
}

#[test]
fn what_runs_in_the_background_is_counted_by_row_subagents_workflows_and_tasks_alike() {
    let mut p = PlanState::default();
    assert_eq!(p.running_count(), 0);
    // A command and a monitor.
    sys(&mut p, bg_started("b1", "c1", "Job", Some(true)));
    sys(&mut p, bg_started("b2", "c2", "Job 2", Some(true)));
    assert_eq!(p.running_count(), 2);
    // A subagent and a workflow (rows of their own).
    p.agents.push(SubAgent::default());
    p.workflows.push(WorkflowRun::default());
    assert_eq!(p.running_count(), 4);
    assert!(p.runs_something());
    // What is over is not counted, whatever way it ended.
    p.on_system(&updated("b1", json!({ "status": "completed" })), 9_000);
    p.agents.push(SubAgent {
        status: RunStatus::Stopped,
        ..Default::default()
    });
    p.workflows.push(WorkflowRun {
        status: RunStatus::Interrupted,
        ..Default::default()
    });
    assert_eq!(p.running_count(), 3);
    p.on_system(&updated("b2", json!({ "status": "failed" })), 9_100);
    p.agents[0].status = RunStatus::Done;
    p.workflows[0].status = RunStatus::Failed;
    assert_eq!(p.running_count(), 0);
    assert!(!p.runs_something());
    // The tasks of the foreground are not in the plan.
    let mut q = PlanState::default();
    sys(&mut q, bg_started("b3", "c3", "Job", Some(false)));
    assert_eq!(q.running_count(), 0);
}

#[test]
fn the_subagents_and_the_background_tasks_that_run_are_counted_apart_as_the_window_counts_them() {
    let mut p = PlanState::default();
    assert_eq!((p.running_subagents(), p.running_background()), (0, 0));
    // A subagent, in the foreground or the background, is not a background task.
    p.agents.push(SubAgent::default());
    p.agents.push(SubAgent {
        background: true,
        ..Default::default()
    });
    assert_eq!((p.running_subagents(), p.running_background()), (2, 0));
    // A command and a monitor, then a workflow: « N en fond », whatever their kind.
    sys(&mut p, bg_started("b1", "c1", "Job", Some(true)));
    sys(&mut p, bg_started("b2", "c2", "Job 2", Some(true)));
    p.workflows.push(WorkflowRun::default());
    assert_eq!((p.running_subagents(), p.running_background()), (2, 3));
    // What is over is counted in neither, whatever way it ended.
    p.agents[0].status = RunStatus::Done;
    p.agents[1].status = RunStatus::Interrupted;
    p.on_system(&updated("b1", json!({ "status": "completed" })), 9_000);
    p.workflows[0].status = RunStatus::Failed;
    assert_eq!((p.running_subagents(), p.running_background()), (0, 1));
    // Together they are what a restart of the agent's process would kill.
    p.agents[0].status = RunStatus::Running;
    assert_eq!(
        p.running_subagents() + p.running_background(),
        p.running_count()
    );
}

#[test]
fn the_window_is_told_the_background_tasks_and_a_saved_plan_without_them_is_read_back() {
    let mut p = PlanState::default();
    tool(
        &mut p,
        "m1",
        "Monitor",
        json!({ "command": "tail -f x", "description": "Watch" }),
    );
    sys(&mut p, bg_started("w1", "m1", "Watch", Some(true)));
    let mut owned = bg_started("b1", "c1", "Ticker", Some(true));
    owned["parent_task_id"] = json!("sa1");
    bash(&mut p, "c1", "echo hi", "Ticker");
    p.on_system(&owned, 60);
    p.on_tool_result("c1", false, &launched_text("b1"), &background_id("b1"), 61);
    p.on_system(&updated("b1", json!({ "status": "failed" })), 70);
    sys(
        &mut p,
        bg_notified(
            "b1",
            "c1",
            Some("failed"),
            "Background command \"Ticker\" failed with exit code 3",
            "",
        ),
    );
    let sent = serde_json::to_value(&p).unwrap();
    assert_eq!(
        sent["bgTasks"],
        json!([
            { "id": "w1", "kind": "monitor", "title": "Watch", "command": "tail -f x", "status": "running",
              "startedAt": T0, "call": "m1" },
            { "id": "b1", "kind": "command", "title": "Ticker", "command": "echo hi", "status": "failed",
              "startedAt": 60, "endedAt": 70, "summary": "Background command \"Ticker\" failed with exit code 3",
              "call": "c1", "parent": "sa1", "outputFile": "/tmp/claude/s1/tasks/b1.output" },
        ])
    );
    // What the plan remembers of the calls and of the foreground is not part of what is said or saved.
    let keys: Vec<&String> = sent.as_object().unwrap().keys().collect();
    assert!(
        !keys
            .iter()
            .any(|k| k.contains("call") || k.contains("memory") || k.contains("fg")),
        "{keys:?}"
    );
    // The kinds in words, lowercase.
    for (kind, word) in [
        (BgKind::Command, "command"),
        (BgKind::Monitor, "monitor"),
        (BgKind::Remote, "remote"),
        (BgKind::Teammate, "teammate"),
    ] {
        assert_eq!(serde_json::to_value(kind).unwrap(), json!(word));
    }
    // Read back, as saved.
    let back: PlanState = serde_json::from_value(sent.clone()).unwrap();
    assert_eq!(serde_json::to_value(&back).unwrap(), sent);
    assert_eq!(back.memory.calls_len(), 0);
    // Written by a version that had none: none.
    let old: PlanState =
        serde_json::from_value(json!({ "tasks": [{ "id": "1", "title": "A" }] })).unwrap();
    assert!(old.bg_tasks.is_empty());
    assert_eq!(BgKind::default(), BgKind::Command);
}

#[test]
fn a_subagents_messages_tell_the_model_it_runs_on() {
    let mut p = PlanState::default();
    // Its call names none: its agent's definition chooses, or it inherits the agent's.
    launch(
        &mut p,
        "a1",
        json!({ "description": "Lire", "prompt": "Lis" }),
    );
    assert_eq!(row(&p, "a1").model, None);
    assert_eq!(p.on_child_model("a1", "claude-haiku-5-5"), Change::Saved);
    assert_eq!(row(&p, "a1").model.as_deref(), Some("claude-haiku-5-5"));
    // The same again is not news; Claude Code's own messages and an empty one tell nothing.
    assert_eq!(p.on_child_model("a1", "claude-haiku-5-5"), Change::None);
    assert_eq!(p.on_child_model("a1", "<synthetic>"), Change::None);
    assert_eq!(p.on_child_model("a1", ""), Change::None);
    assert_eq!(row(&p, "a1").model.as_deref(), Some("claude-haiku-5-5"));
    // What it runs on wins over what its call asked for.
    launch(
        &mut p,
        "a2",
        json!({ "description": "Tests", "prompt": "Lance", "model": "haiku" }),
    );
    assert_eq!(row(&p, "a2").model.as_deref(), Some("haiku"));
    p.on_child_model("a2", "claude-sonnet-5-5");
    assert_eq!(row(&p, "a2").model.as_deref(), Some("claude-sonnet-5-5"));
    // A thread that is no subagent's has no row to tell.
    assert_eq!(p.on_child_model("zzz", "claude-haiku-5-5"), Change::None);
}
