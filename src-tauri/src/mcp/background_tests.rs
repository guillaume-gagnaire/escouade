//! The tools of the subagents and background tasks (ESC-107), called by a real MCP client (rmcp's)
//! on a test core: plans put on agents as the stream would leave them, and the fake `claude` for
//! what has to reach a process (a task stopped, its output, a message to a subagent).

use super::private_tests::make_private;
use super::tools_tests::{
    as_agent, call, external, last_entry, other_project, project, put_agent, read, refused, Client,
};
use crate::core_tests::{harness, Harness};
use crate::model::*;
use crate::plan::{BgKind, BgTask, PlanState, RunStatus, SubAgent, WorkflowRun};
use serde_json::{json, Value};

/// `plan` as the plan of the agent `id`.
fn with_plan(h: &Harness, id: &str, plan: PlanState) {
    h.core.agent(id).unwrap().lock().meta.plan = Some(plan);
}

/// A subagent of the plan, launched by the call `call`.
fn subagent(call: &str, title: &str, status: RunStatus) -> SubAgent {
    SubAgent {
        id: call.into(),
        title: title.into(),
        status,
        started_at: 1_000,
        ..Default::default()
    }
}

/// A task of Claude Code in the background, of the kind `kind`.
fn task(id: &str, kind: BgKind, status: RunStatus) -> BgTask {
    BgTask {
        id: id.into(),
        kind,
        title: format!("Tâche {id}"),
        status,
        started_at: 1_000,
        ..Default::default()
    }
}

/// A workflow of the plan.
fn workflow(id: &str, status: RunStatus) -> WorkflowRun {
    WorkflowRun {
        id: id.into(),
        title: format!("Flux {id}"),
        status,
        started_at: 1_000,
        ..Default::default()
    }
}

/// The row of the agent `name` in an answer that lists agents.
fn row<'a>(rows: &'a Value, name: &str) -> &'a Value {
    rows.as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == name)
        .unwrap_or_else(|| panic!("{name} in {rows}"))
}

/// How many subagents and background tasks an answer says run.
fn counts(v: &Value) -> (Value, Value) {
    (
        v["subagentsRunning"].clone(),
        v["backgroundTasksRunning"].clone(),
    )
}

async fn summary(c: &Client, agent: &str) -> Value {
    read(c, "get_agent_summary", json!({ "agent": agent })).await
}

// ---------- the counts of list_agents and get_agent_summary ----------

#[tokio::test]
async fn the_agents_say_how_many_subagents_and_background_tasks_run_whatever_their_own_status() {
    let h = harness("mcp-bg-counts");
    let p = project(&h).await;
    let finished = put_agent(&h, &p, "fini", 1);
    let working = put_agent(&h, &p, "sous-agent", 2);
    let flowing = put_agent(&h, &p, "flux", 3);
    let both = put_agent(&h, &p, "les-deux", 4);
    let over = put_agent(&h, &p, "termine", 5);
    let bare = put_agent(&h, &p, "nu", 6);
    // An agent « Terminé » whose command goes on in the background.
    h.core.agent(&finished.id).unwrap().lock().meta.status = AgentStatus::Done;
    with_plan(
        &h,
        &finished.id,
        PlanState {
            bg_tasks: vec![task("b1", BgKind::Command, RunStatus::Running)],
            ..Default::default()
        },
    );
    // A subagent that runs, in the foreground: not a background task.
    with_plan(
        &h,
        &working.id,
        PlanState {
            agents: vec![subagent("c1", "Chercher", RunStatus::Running)],
            ..Default::default()
        },
    );
    // A workflow counts with the tasks.
    with_plan(
        &h,
        &flowing.id,
        PlanState {
            workflows: vec![workflow("w1", RunStatus::Running)],
            ..Default::default()
        },
    );
    // Each count on its own, a monitor and a teammate among the tasks; what is over counts for none.
    with_plan(
        &h,
        &both.id,
        PlanState {
            agents: vec![
                subagent("c1", "Un", RunStatus::Running),
                SubAgent {
                    background: true,
                    ..subagent("c2", "Deux", RunStatus::Running)
                },
                subagent("c3", "Trois", RunStatus::Done),
            ],
            bg_tasks: vec![
                task("b1", BgKind::Monitor, RunStatus::Running),
                task("b2", BgKind::Teammate, RunStatus::Running),
                task("b3", BgKind::Command, RunStatus::Stopped),
            ],
            workflows: vec![
                workflow("w1", RunStatus::Running),
                workflow("w2", RunStatus::Failed),
            ],
            ..Default::default()
        },
    );
    // Everything over: nothing runs.
    with_plan(
        &h,
        &over.id,
        PlanState {
            agents: vec![
                subagent("c1", "Fini", RunStatus::Done),
                subagent("c2", "Arrêté", RunStatus::Stopped),
            ],
            bg_tasks: vec![task("b1", BgKind::Command, RunStatus::Interrupted)],
            workflows: vec![workflow("w1", RunStatus::Done)],
            ..Default::default()
        },
    );
    let c = external(&h).await;
    let listed = read(&c, "list_agents", json!({})).await;
    let expected = [
        ("fini", 0, 1),
        ("sous-agent", 1, 0),
        ("flux", 0, 1),
        ("les-deux", 2, 3),
        ("termine", 0, 0),
        // No plan at all.
        ("nu", 0, 0),
    ];
    for (name, subagents, background) in expected {
        assert_eq!(
            counts(row(&listed, name)),
            (json!(subagents), json!(background)),
            "list_agents: {name}"
        );
        assert_eq!(
            counts(&summary(&c, name).await),
            (json!(subagents), json!(background)),
            "get_agent_summary: {name}"
        );
    }
    // What the rows already said stays: the counts are beside it.
    let one = summary(&c, "fini").await;
    assert_eq!(
        (&one["id"], &one["status"]),
        (&json!(finished.id), &json!("done"))
    );
    assert_eq!(bare.name, "nu");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

// ---------- list_background_work ----------

#[tokio::test]
async fn the_work_of_an_agent_is_listed_with_the_fields_of_each_kind_and_never_a_path() {
    let h = harness("mcp-bg-list-shape");
    let p = project(&h).await;
    let a = put_agent(&h, &p, "chef", 1);
    h.core.agent(&a.id).unwrap().lock().meta.status = AgentStatus::Done;
    let secret = "C:\\Users\\moi\\AppData\\Local\\Temp\\claude\\tasks\\b1.output";
    with_plan(
        &h,
        &a.id,
        PlanState {
            agents: vec![
                // Every field a subagent has.
                SubAgent {
                    kind: Some("Explore".into()),
                    model: Some("haiku".into()),
                    background: true,
                    tools: 3,
                    tokens: Some(1_800),
                    doing: Some("Édite src/a.ts".into()),
                    plan_task: Some("2".into()),
                    parent: Some("toolu_0".into()),
                    task_id: Some("sub-task-1".into()),
                    started_at: 2_000,
                    ..subagent("toolu_1", "Chercher la cause", RunStatus::Running)
                },
                // Only what it must have: a subagent in the foreground, which just began.
                subagent("toolu_2", "Relire", RunStatus::Running),
                // One that ended.
                SubAgent {
                    ended_at: Some(4_000),
                    tools: 7,
                    tokens: Some(4_200),
                    ..subagent("toolu_3", "Écrire", RunStatus::Done)
                },
            ],
            bg_tasks: vec![
                BgTask {
                    title: "Ticker".into(),
                    command: Some("for i in 1 2; do echo $i; done".into()),
                    started_at: 1_500,
                    call: Some("toolu_9".into()),
                    parent: Some("sub-task-1".into()),
                    output_file: Some(secret.into()),
                    ..task("b1", BgKind::Command, RunStatus::Running)
                },
                BgTask {
                    title: "Watch loop output".into(),
                    started_at: 1_600,
                    ended_at: Some(3_000),
                    summary: Some("Monitor \"Watch loop output\" stream ended".into()),
                    output_file: Some(secret.replace("b1", "m1")),
                    ..task("m1", BgKind::Monitor, RunStatus::Done)
                },
                BgTask {
                    title: "Agent distant".into(),
                    started_at: 1_650,
                    ..task("r1", BgKind::Remote, RunStatus::Running)
                },
                BgTask {
                    title: "Coéquipier".into(),
                    started_at: 1_660,
                    ..task("t1", BgKind::Teammate, RunStatus::Running)
                },
            ],
            workflows: vec![WorkflowRun {
                name: Some("revue-finale".into()),
                title: "Revue finale".into(),
                now: Some("Phase 2 : relecture".into()),
                tools: 4,
                tokens: 900,
                started_at: 1_700,
                ..workflow("wf1", RunStatus::Running)
            }],
            ..Default::default()
        },
    );
    let c = external(&h).await;
    let listed = read(&c, "list_background_work", json!({ "agent": "chef" })).await;
    assert_eq!(
        (
            &listed["id"],
            &listed["name"],
            &listed["status"],
            &listed["subagentsRunning"],
            &listed["backgroundTasksRunning"]
        ),
        (
            &json!(a.id),
            &json!("chef"),
            &json!("done"),
            &json!(2),
            &json!(4)
        )
    );
    assert_eq!(
        listed["subagents"],
        json!([
            {
                "id": "toolu_1", "type": "subagent", "description": "Chercher la cause",
                "subagentType": "Explore", "model": "haiku", "status": "running",
                "background": true, "startedAt": 2000, "tools": 3, "tokens": 1800,
                "doing": "Édite src/a.ts", "taskId": "sub-task-1", "parent": "toolu_0",
            },
            {
                "id": "toolu_2", "type": "subagent", "description": "Relire",
                "status": "running", "background": false, "startedAt": 1000, "tools": 0,
            },
            {
                "id": "toolu_3", "type": "subagent", "description": "Écrire",
                "status": "done", "background": false, "startedAt": 1000, "endedAt": 4000,
                "tools": 7, "tokens": 4200,
            },
        ])
    );
    assert_eq!(
        listed["tasks"],
        json!([
            {
                "id": "b1", "type": "command", "description": "Ticker",
                "command": "for i in 1 2; do echo $i; done", "status": "running",
                "startedAt": 1500, "parent": "sub-task-1",
            },
            {
                "id": "r1", "type": "remote", "description": "Agent distant",
                "status": "running", "startedAt": 1650,
            },
            {
                "id": "t1", "type": "teammate", "description": "Coéquipier",
                "status": "running", "startedAt": 1660,
            },
            {
                "id": "wf1", "type": "workflow", "description": "Revue finale",
                "name": "revue-finale", "status": "running", "startedAt": 1700,
                "now": "Phase 2 : relecture", "tools": 4, "tokens": 900,
            },
            {
                "id": "m1", "type": "monitor", "description": "Watch loop output",
                "status": "done", "startedAt": 1600, "endedAt": 3000,
                "summary": "Monitor \"Watch loop output\" stream ended",
            },
        ])
    );
    // The disk is not the model's to wander: no path, under any key.
    let (text, _) = call(&c, "list_background_work", json!({ "agent": a.id })).await;
    assert!(
        !text.contains("outputFile")
            && !text.contains("output_file")
            && !text.contains("tasks\\\\b1"),
        "{text}"
    );
    assert!(
        !text.contains("AppData") && !text.contains("toolu_9"),
        "{text}"
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

/// The ids of the entries of `list` (`subagents` or `tasks`), in order.
fn ids(listed: &Value, list: &str) -> Vec<String> {
    listed[list]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn the_work_is_listed_in_the_order_of_the_window_and_an_id_only_once() {
    let h = harness("mcp-bg-list-order");
    let p = project(&h).await;
    let a = put_agent(&h, &p, "chef", 1);
    let ended = |at: Option<i64>, row: SubAgent| SubAgent {
        ended_at: at,
        ..row
    };
    let bg = |id: &str, status: RunStatus, started: i64, ended: Option<i64>| BgTask {
        started_at: started,
        ended_at: ended,
        ..task(id, BgKind::Command, status)
    };
    with_plan(
        &h,
        &a.id,
        PlanState {
            // Running first, in the plan's order, then those that ended, the latest first (one
            // that never said when comes last).
            agents: vec![
                ended(Some(5_000), subagent("sa", "A", RunStatus::Done)),
                subagent("sb", "B", RunStatus::Running),
                ended(Some(9_000), subagent("sc", "C", RunStatus::Failed)),
                subagent("sd", "D", RunStatus::Running),
                ended(None, subagent("se", "E", RunStatus::Interrupted)),
                ended(Some(7_000), subagent("sf", "F", RunStatus::Stopped)),
            ],
            // Running first, the oldest launched first (a tie keeps the order given), then those
            // that ended, the latest first; a task that never said when it ended counts from its
            // start.
            bg_tasks: vec![
                bg("t300", RunStatus::Running, 300, None),
                bg("tdone", RunStatus::Done, 10, Some(500)),
                bg("t100", RunStatus::Running, 100, None),
                bg("tfail", RunStatus::Failed, 20, Some(800)),
                bg("tstopped", RunStatus::Stopped, 600, None),
                bg("ttie", RunStatus::Running, 300, None),
            ],
            // A workflow takes its place among them; one that has the id of a task is not listed
            // again, and never in the task's place.
            workflows: vec![
                WorkflowRun {
                    started_at: 200,
                    ..workflow("w200", RunStatus::Running)
                },
                WorkflowRun {
                    started_at: 1,
                    ended_at: Some(9_999),
                    ..workflow("tdone", RunStatus::Done)
                },
            ],
            ..Default::default()
        },
    );
    let c = external(&h).await;
    let listed = read(&c, "list_background_work", json!({ "agent": a.id })).await;
    assert_eq!(
        ids(&listed, "subagents"),
        ["sb", "sd", "sc", "sf", "sa", "se"]
    );
    assert_eq!(
        ids(&listed, "tasks"),
        ["t100", "w200", "t300", "ttie", "tfail", "tstopped", "tdone"]
    );
    let shadowed = listed["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == "tdone")
        .unwrap();
    assert_eq!(
        (&shadowed["type"], &shadowed["description"]),
        (&json!("command"), &json!("Tâche tdone"))
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn any_agent_can_be_listed_by_anyone_in_the_claude_code_view_too_and_the_call_is_logged() {
    let h = harness("mcp-bg-list-access");
    let p = project(&h).await;
    let other = other_project(&h, "autre").await;
    let mine = put_agent(&h, &p, "chef", 1);
    let calling = put_agent(&h, &p, "appelant", 2);
    let neighbour = put_agent(&h, &other, "voisin", 3);
    let twin = put_agent(&h, &other, "chef", 4);
    for (agent, id) in [
        (&mine, "b-mine"),
        (&neighbour, "b-neighbour"),
        (&twin, "b-twin"),
    ] {
        with_plan(
            &h,
            &agent.id,
            PlanState {
                bg_tasks: vec![task(id, BgKind::Command, RunStatus::Running)],
                ..Default::default()
            },
        );
    }
    // In the Claude Code view the plan is the same.
    h.core.agent(&neighbour.id).unwrap().lock().meta.terminal = true;
    let c = external(&h).await;
    let listed = read(&c, "list_background_work", json!({ "agent": "voisin" })).await;
    assert_eq!(ids(&listed, "tasks"), ["b-neighbour"]);
    // An agent of Escouade lists its own and those of another project (the tools that read are open).
    let from = as_agent(&h, &calling.id).await;
    for (agent, expected) in [
        (&neighbour.id, "b-neighbour"),
        (&mine.id, "b-mine"),
        (&calling.id, ""),
    ] {
        let listed = read(&from, "list_background_work", json!({ "agent": agent })).await;
        assert_eq!(
            ids(&listed, "tasks"),
            if expected.is_empty() {
                vec![]
            } else {
                vec![expected]
            },
            "{agent}"
        );
    }
    // Two agents of the same name: the project tells which.
    let listed = read(
        &c,
        "list_background_work",
        json!({ "agent": "chef", "project": "autre" }),
    )
    .await;
    assert_eq!(
        (listed["id"].as_str(), ids(&listed, "tasks")),
        (Some(twin.id.as_str()), vec!["b-twin".to_string()])
    );
    let (_, ambiguous) = call(&c, "list_background_work", json!({ "agent": "chef" })).await;
    assert!(ambiguous);
    // An agent that does not exist is told as get_agent_summary tells it.
    let unknown = refused(&c, "list_background_work", json!({ "agent": "zzz" })).await;
    assert!(unknown.contains("Aucun agent « zzz »"), "{unknown}");
    assert_eq!(
        unknown,
        refused(&c, "get_agent_summary", json!({ "agent": "zzz" })).await
    );
    let logged = last_entry(&h);
    assert_eq!(
        (logged.1.as_str(), logged.3),
        ("get_agent_summary", "error")
    );
    let _ = read(&c, "list_background_work", json!({ "agent": "voisin" })).await;
    assert_eq!(
        last_entry(&h),
        (
            "Claude (hors Escouade)".into(),
            "list_background_work".into(),
            "agent: voisin".into(),
            "ok"
        )
    );
    from.cancel().await.unwrap();
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

/// A command line that holds a secret, as `curl -H "Authorization: …"` does.
const COMMAND_WITH_A_SECRET: &str =
    "curl -H 'Authorization: Bearer SECRET-TOKEN-107' https://example.test/api";

#[tokio::test]
async fn an_agent_of_escouade_is_given_the_work_it_sees_and_one_the_core_no_longer_has_only_what_runs(
) {
    let h = harness("mcp-bg-list-outline");
    let p = project(&h).await;
    let other = other_project(&h, "autre").await;
    let target = put_agent(&h, &other, "cible", 1);
    let mate = put_agent(&h, &other, "collegue", 2);
    let far = put_agent(&h, &p, "voisin", 3);
    h.core.agent(&target.id).unwrap().lock().meta.status = AgentStatus::Done;
    with_plan(
        &h,
        &target.id,
        PlanState {
            agents: vec![
                SubAgent {
                    kind: Some("Explore".into()),
                    model: Some("haiku".into()),
                    background: true,
                    tools: 3,
                    tokens: Some(1_800),
                    doing: Some("Édite src/a.ts".into()),
                    parent: Some("toolu_0".into()),
                    task_id: Some("sub-task-1".into()),
                    started_at: 2_000,
                    ..subagent("toolu_1", "Chercher la cause", RunStatus::Running)
                },
                SubAgent {
                    ended_at: Some(4_000),
                    tools: 7,
                    tokens: Some(4_200),
                    ..subagent("toolu_3", "Écrire le rapport", RunStatus::Done)
                },
            ],
            bg_tasks: vec![
                BgTask {
                    title: "Appeler l’API".into(),
                    command: Some(COMMAND_WITH_A_SECRET.into()),
                    started_at: 1_500,
                    summary: Some("Réponse 200 avec le jeton".into()),
                    parent: Some("sub-task-1".into()),
                    ..task("b1", BgKind::Command, RunStatus::Running)
                },
                BgTask {
                    title: "Surveiller le journal".into(),
                    started_at: 1_600,
                    ended_at: Some(3_000),
                    summary: Some("Surveillance finie".into()),
                    ..task("m1", BgKind::Monitor, RunStatus::Done)
                },
                BgTask {
                    title: "Agent distant".into(),
                    started_at: 1_650,
                    ..task("r1", BgKind::Remote, RunStatus::Running)
                },
            ],
            workflows: vec![WorkflowRun {
                name: Some("revue-finale".into()),
                title: "Revue finale".into(),
                now: Some("Phase 2 : relecture".into()),
                tools: 4,
                tokens: 900,
                started_at: 1_700,
                ..workflow("wf1", RunStatus::Running)
            }],
            ..Default::default()
        },
    );
    let ask = json!({ "agent": "cible" });
    // Claude outside Escouade is given all of it.
    let outside = external(&h).await;
    let full = read(&outside, "list_background_work", ask.clone()).await;
    let b1 = full["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == "b1")
        .unwrap();
    assert_eq!(b1["command"], COMMAND_WITH_A_SECRET);
    assert_eq!(full["subagents"][0]["doing"], "Édite src/a.ts");
    // So is an agent of the project of that agent, its own work included, and one of another
    // project that is not private.
    for caller in [&mate.id, &target.id, &far.id] {
        let c = as_agent(&h, caller).await;
        assert_eq!(
            read(&c, "list_background_work", ask.clone()).await,
            full,
            "{caller}"
        );
        c.cancel().await.unwrap();
    }

    // An agent of a private project does not find it, as an agent that does not exist.
    let vault = make_private(&h, &other_project(&h, "coffre").await);
    let inside = put_agent(&h, &vault, "enferme", 4);
    let c = as_agent(&h, &inside.id).await;
    let (text, error) = call(&c, "list_background_work", ask.clone()).await;
    assert!(error && text.starts_with("Aucun agent « cible »"), "{text}");
    assert!(!text.contains("autre"), "{text}");
    c.cancel().await.unwrap();

    // An agent the core no longer has is given which work there is, and how it stands.
    let from = as_agent(&h, &far.id).await;
    h.core.agents.write().remove(&far.id);
    let (text, error) = call(&from, "list_background_work", ask.clone()).await;
    assert!(!error, "{text}");
    let outline: Value = serde_json::from_str(&text).unwrap();
    // The agent and its counts stay, as the lists that follow.
    let mut keys: Vec<&str> = outline
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "backgroundTasksRunning",
            "id",
            "name",
            "status",
            "subagents",
            "subagentsRunning",
            "tasks"
        ]
    );
    for key in [
        "id",
        "name",
        "status",
        "subagentsRunning",
        "backgroundTasksRunning",
    ] {
        assert_eq!(outline[key], full[key], "{key}");
    }
    assert_eq!(
        (
            &outline["subagentsRunning"],
            &outline["backgroundTasksRunning"]
        ),
        (&json!(1), &json!(3))
    );
    // Each row has an id, a type, a status, when it started and when it ended: those keys, no more,
    // and in the order the others see.
    assert_eq!(
        outline["subagents"],
        json!([
            { "id": "toolu_1", "type": "subagent", "status": "running", "startedAt": 2000 },
            { "id": "toolu_3", "type": "subagent", "status": "done", "startedAt": 1000, "endedAt": 4000 },
        ])
    );
    assert_eq!(
        outline["tasks"],
        json!([
            { "id": "b1", "type": "command", "status": "running", "startedAt": 1500 },
            { "id": "r1", "type": "remote", "status": "running", "startedAt": 1650 },
            { "id": "wf1", "type": "workflow", "status": "running", "startedAt": 1700 },
            { "id": "m1", "type": "monitor", "status": "done", "startedAt": 1600, "endedAt": 3000 },
        ])
    );
    assert_eq!(ids(&outline, "tasks"), ids(&full, "tasks"));
    assert_eq!(ids(&outline, "subagents"), ids(&full, "subagents"));
    // None of what could hold a secret is anywhere in the answer.
    for hidden in [
        COMMAND_WITH_A_SECRET,
        "SECRET-TOKEN-107",
        "Authorization",
        "Appeler",
        "Réponse 200",
        "Surveillance finie",
        "Chercher la cause",
        "Édite",
        "haiku",
        "Explore",
        "sub-task-1",
        "toolu_0",
        "revue-finale",
        "Phase 2",
    ] {
        assert!(!text.contains(hidden), "{hidden} in {text}");
    }
    // The call is logged as any other.
    assert_eq!(
        last_entry(&h),
        (
            far.id.clone(),
            "list_background_work".into(),
            "agent: cible".into(),
            "ok"
        )
    );
    // And it acts nowhere: not even on what it may still find.
    let why = refused(
        &from,
        "get_background_output",
        json!({ "agent": "cible", "id": "b1" }),
    )
    .await;
    assert_eq!(
        why,
        format!(
            "L’agent {} n’est plus dans Escouade : il n’agit plus.",
            far.id
        )
    );
    assert_eq!(last_entry(&h).3, "refused");
    from.cancel().await.unwrap();
    outside.cancel().await.unwrap();
    h.core.mcp.stop();
}

// ---------- get_background_output ----------

/// A task of the plan whose output is the file `<id>.output` of a folder named `tasks`, as Claude
/// Code writes it, which holds `content`.
fn with_output(h: &Harness, id: &str, kind: BgKind, status: RunStatus, content: &[u8]) -> BgTask {
    let dir = h.dir.join("tasks");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(format!("{id}.output"));
    std::fs::write(&file, content).unwrap();
    BgTask {
        output_file: Some(file.to_string_lossy().into()),
        ..task(id, kind, status)
    }
}

/// What `get_background_output` answers for the task or subagent `id` of `agent`, `bytes` asked or
/// not.
async fn output_of(c: &Client, agent: &str, id: &str, bytes: Option<u64>) -> Value {
    let mut args = json!({ "agent": agent, "id": id });
    if let Some(bytes) = bytes {
        args["bytes"] = json!(bytes);
    }
    read(c, "get_background_output", args).await
}

/// The size, in bytes, of the output an answer gives, and whether it says it is cut.
fn shape(v: &Value) -> (usize, bool) {
    (
        v["output"].as_str().unwrap().len(),
        v["truncated"].as_bool().unwrap(),
    )
}

#[tokio::test]
async fn the_output_of_a_command_that_runs_is_asked_to_its_process() {
    let h = harness("mcp-bg-output-process");
    let (p, _) = h.project(false).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    h.core
        .send_message(&a.id, "commande de fond 60s".into(), vec![])
        .await
        .unwrap();
    let row = h.bg_task(&a.id, "Ticker", true).await;
    assert_eq!(
        (row.kind, row.status),
        (BgKind::Command, RunStatus::Running)
    );
    h.wait("some ticks", |_| {
        std::fs::read_to_string(row.output_file.as_deref().unwrap())
            .is_ok_and(|s| s.contains("tick 3"))
    })
    .await;
    let c = external(&h).await;
    // Named after its first message, by the fake claude: waited for, the name is the one it keeps.
    h.wait("its name", |h| h.agent(&a.id).named).await;
    let name = h.agent(&a.id).name;
    let v = output_of(&c, &name, &row.id, None).await;
    assert_eq!(
        (
            &v["agent"],
            &v["id"],
            &v["type"],
            &v["status"],
            &v["source"]
        ),
        (
            &json!({ "id": a.id, "name": name }),
            &json!(row.id),
            &json!("command"),
            &json!("running"),
            &json!("process")
        )
    );
    let text = v["output"].as_str().unwrap();
    assert!(text.starts_with("tick 1\n"), "{text:?}");
    assert_eq!(v["totalBytes"], text.len());
    assert_eq!(v["truncated"], false);
    // Cut to the end of what the process gave when fewer bytes are asked for.
    let v = output_of(&c, &a.id, &row.id, Some(10)).await;
    let (size, cut) = shape(&v);
    assert!((6..=10).contains(&size) && cut, "{v}");
    assert_eq!(v["source"], "process");
    assert!(v["totalBytes"].as_u64().unwrap() > 10);
    // What the call asked is in the log, and not the output.
    let logged = last_entry(&h);
    assert_eq!(
        (logged.1.as_str(), logged.2, logged.3),
        (
            "get_background_output",
            format!("agent: {} · id: {} · bytes: 10", a.id, row.id),
            "ok"
        )
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_output_of_a_task_is_the_end_of_its_file_cut_to_the_bytes_asked_at_a_character() {
    let h = harness("mcp-bg-output-bounds");
    let p = project(&h).await;
    let a = put_agent(&h, &p, "chef", 1);
    // 20 000 bytes while it runs, 300 KiB once over: the first and the last word tell which end.
    let running = format!("START{}END", "y".repeat(20_000 - 8));
    let over = format!("FIRST{}LAST", "y".repeat(300 * 1024 - 9));
    with_plan(
        &h,
        &a.id,
        PlanState {
            bg_tasks: vec![
                with_output(
                    &h,
                    "run",
                    BgKind::Command,
                    RunStatus::Running,
                    running.as_bytes(),
                ),
                with_output(
                    &h,
                    "over",
                    BgKind::Command,
                    RunStatus::Failed,
                    over.as_bytes(),
                ),
                with_output(&h, "small", BgKind::Monitor, RunStatus::Done, b"abcdef"),
                with_output(
                    &h,
                    "multi",
                    BgKind::Command,
                    RunStatus::Done,
                    "xé€é€é€".as_bytes(),
                ),
            ],
            ..Default::default()
        },
    );
    let c = external(&h).await;
    // By default 8 KiB, the end, whether it runs or is over: the window's own bound for the first,
    // the answer's for the second (up to 64 KiB for whom asks).
    let v = output_of(&c, &a.id, "run", None).await;
    assert_eq!(shape(&v), (8_192, true));
    assert_eq!(
        (&v["source"], &v["totalBytes"]),
        (&json!("file"), &json!(20_000))
    );
    assert!(v["output"].as_str().unwrap().ends_with("yEND"));
    let v = output_of(&c, &a.id, "over", None).await;
    assert_eq!(shape(&v), (8_192, true));
    assert_eq!(
        (&v["source"], &v["status"]),
        (&json!("file"), &json!("failed"))
    );
    assert_eq!(v["totalBytes"], 300 * 1024);
    let text = v["output"].as_str().unwrap();
    assert!(text.ends_with("yLAST") && !text.contains("FIRST"));
    // Fewer, as many as asked, the end every time.
    let v = output_of(&c, &a.id, "run", Some(100)).await;
    assert_eq!(shape(&v), (100, true));
    assert!(v["output"].as_str().unwrap().ends_with("yEND"));
    // A task that runs gives no more than its window's 8 KiB, whatever is asked.
    assert_eq!(
        shape(&output_of(&c, &a.id, "run", Some(100_000)).await),
        (8_192, true)
    );
    // One that is over gives up to 64 KiB (the window shows up to 256 KiB of it, which an answer
    // to a client could not carry), and no more than that however much is asked.
    let v = output_of(&c, &a.id, "over", Some(64 * 1024)).await;
    assert_eq!(shape(&v), (64 * 1024, true));
    assert!(v["output"].as_str().unwrap().ends_with("yLAST"));
    for asked in [64 * 1024 + 1, 256 * 1024, 10_000_000, u64::MAX] {
        let v = output_of(&c, &a.id, "over", Some(asked)).await;
        assert_eq!(shape(&v), (64 * 1024, true), "{asked}");
        assert!(!v["output"].as_str().unwrap().contains("FIRST"));
    }
    // At least one byte.
    let v = output_of(&c, &a.id, "over", Some(0)).await;
    assert_eq!(
        (v["output"].as_str(), v["truncated"].as_bool()),
        (Some("T"), Some(true))
    );
    // A short output is whole as long as it fits in what is asked, cut when it does not.
    for (bytes, expected, cut) in [
        (None, "abcdef", false),
        (Some(6), "abcdef", false),
        (Some(600), "abcdef", false),
        (Some(3), "def", true),
        (Some(1), "f", true),
    ] {
        let v = output_of(&c, &a.id, "small", bytes).await;
        assert_eq!(
            (
                v["output"].as_str(),
                v["truncated"].as_bool(),
                v["totalBytes"].as_u64()
            ),
            (Some(expected), Some(cut), Some(6)),
            "{bytes:?}"
        );
    }
    // Never in the middle of a character: « xé€é€é€ » is 16 bytes (é takes two, € three).
    for (bytes, expected, cut) in [
        (Some(16), "xé€é€é€", false),
        (Some(15), "é€é€é€", true),
        (Some(14), "€é€é€", true),
        (Some(5), "é€", true),
        (Some(4), "€", true),
        (Some(3), "€", true),
        (Some(2), "", true),
    ] {
        let v = output_of(&c, &a.id, "multi", bytes).await;
        assert_eq!(
            (
                v["output"].as_str(),
                v["truncated"].as_bool(),
                v["totalBytes"].as_u64()
            ),
            (Some(expected), Some(cut), Some(16)),
            "{bytes:?}"
        );
    }
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_output_of_a_finished_task_is_given_up_to_64_kib_and_8_kib_when_not_asked() {
    let h = harness("mcp-bg-output-cap");
    let p = project(&h).await;
    let a = put_agent(&h, &p, "chef", 1);
    // 100 KiB: more than an answer to a client carries (Claude Code refuses one of more than about
    // 25,000 tokens), less than the 256 KiB the window shows of a task that is over.
    let size = 100 * 1024;
    let over = format!("FIRST{}LAST", "y".repeat(size - 9));
    with_plan(
        &h,
        &a.id,
        PlanState {
            bg_tasks: vec![with_output(
                &h,
                "over",
                BgKind::Command,
                RunStatus::Done,
                over.as_bytes(),
            )],
            ..Default::default()
        },
    );
    let c = external(&h).await;
    // Not asked: 8 KiB, the end.
    let v = output_of(&c, &a.id, "over", None).await;
    assert_eq!(shape(&v), (8_192, true));
    assert_eq!(v["totalBytes"], size);
    assert!(v["output"].as_str().unwrap().ends_with("yLAST"));
    // Asked for more than 64 KiB, whatever the number: 64 KiB, the end, and it says it is cut.
    for asked in [65_537, 100 * 1024, 262_144, 10_000_000, u64::MAX] {
        let v = output_of(&c, &a.id, "over", Some(asked)).await;
        assert_eq!(shape(&v), (65_536, true), "{asked}");
        assert_eq!(v["totalBytes"], size, "{asked}");
        let text = v["output"].as_str().unwrap();
        assert!(
            text.ends_with("yLAST") && !text.contains("FIRST"),
            "{asked}"
        );
    }
    // 64 KiB is the most that is given, and what is asked below it is given as asked.
    for asked in [65_536, 65_535, 20_000, 1] {
        let v = output_of(&c, &a.id, "over", Some(asked)).await;
        assert_eq!(shape(&v), (asked as usize, true), "{asked}");
    }
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

/// What the log says of a call whose arguments the schema refuses.
const DO_NOT_FIT: &str =
    "Appel refusé par le serveur : les arguments ne correspondent pas au schéma de l’outil";

#[tokio::test]
async fn a_bytes_that_is_no_whole_number_is_refused_by_the_schema_and_never_echoed() {
    let h = harness("mcp-bg-output-bytes-unreadable");
    let p = project(&h).await;
    let a = put_agent(&h, &p, "chef", 1);
    with_plan(
        &h,
        &a.id,
        PlanState {
            bg_tasks: vec![with_output(
                &h,
                "b1",
                BgKind::Command,
                RunStatus::Done,
                b"sortie",
            )],
            ..Default::default()
        },
    );
    let c = external(&h).await;
    for (what, bytes, value) in [
        ("a text", json!("abc"), "abc"),
        (
            "a text that stands out",
            json!("octets-secrets-107"),
            "octets-secrets-107",
        ),
        ("a negative number", json!(-1), "-1"),
        ("a fraction", json!(1.5), "1.5"),
    ] {
        let (text, error) = call(
            &c,
            "get_background_output",
            json!({ "agent": a.id, "id": "b1", "bytes": bytes }),
        )
        .await;
        assert!(error, "{what}: {text}");
        // Short, nothing quoted from the request, and still something the model can act on.
        assert!(
            !text.contains(value) && !text.contains(['"', '`']) && text.chars().count() < 300,
            "{what}: {text}"
        );
        assert!(
            text.contains("schéma") && !text.contains("sortie"),
            "{what}: {text}"
        );
        let e = h.core.mcp.activity.entries().pop().unwrap();
        assert_eq!(
            (e.tool.as_str(), e.outcome, e.summary.as_str()),
            ("get_background_output", "error", ""),
            "{what}"
        );
        assert_eq!(e.message.as_deref(), Some(DO_NOT_FIT), "{what}");
        // The log as a whole, and what the window was told of it.
        let log = serde_json::to_string(&h.core.mcp.activity.entries()).unwrap();
        assert!(!log.contains(value), "{what}: {log}");
        let told: Vec<_> = h
            .events
            .lock()
            .iter()
            .filter(|e| e["type"] == "mcpActivity")
            .cloned()
            .collect();
        assert!(!told.is_empty());
        assert!(
            !serde_json::to_string(&told).unwrap().contains(value),
            "{what}"
        );
    }
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_file_that_is_not_the_tasks_own_is_not_read_and_a_task_without_output_says_so() {
    let h = harness("mcp-bg-output-hostile");
    let p = project(&h).await;
    let a = put_agent(&h, &p, "chef", 1);
    let dir = h.dir.join("tasks");
    std::fs::create_dir_all(dir.join("b4.output")).unwrap();
    std::fs::write(dir.join("other.output"), "never").unwrap();
    std::fs::write(h.dir.join("b3.output"), "never").unwrap();
    let told = |id: &str, file: Option<String>| BgTask {
        output_file: file,
        ..task(id, BgKind::Command, RunStatus::Done)
    };
    with_plan(
        &h,
        &a.id,
        PlanState {
            bg_tasks: vec![
                // A share of another machine: not even looked for.
                told(
                    "b1",
                    Some(r"\\nonexistent.invalid\share\tasks\b1.output".into()),
                ),
                told(
                    "b1b",
                    Some("//nonexistent.invalid/share/tasks/b1b.output".into()),
                ),
                // A file of another name, not in a folder named `tasks`, a folder, no file told.
                told(
                    "b2",
                    Some(dir.join("other.output").to_string_lossy().into()),
                ),
                told("b3", Some(h.dir.join("b3.output").to_string_lossy().into())),
                told("b4", Some(dir.join("b4.output").to_string_lossy().into())),
                told("b5", None),
                // A remote agent and a teammate have no output to read, a file or not.
                with_output(&h, "r1", BgKind::Remote, RunStatus::Running, b"never"),
                with_output(&h, "t1", BgKind::Teammate, RunStatus::Running, b"never"),
            ],
            workflows: vec![workflow("w1", RunStatus::Running)],
            ..Default::default()
        },
    );
    let c = external(&h).await;
    for (id, kind) in [
        ("b1", "command"),
        ("b1b", "command"),
        ("b2", "command"),
        ("b3", "command"),
        ("b4", "command"),
        ("b5", "command"),
        ("r1", "remote"),
        ("t1", "teammate"),
        ("w1", "workflow"),
    ] {
        let v = output_of(&c, &a.id, id, Some(1000)).await;
        assert_eq!(
            (
                &v["type"],
                &v["source"],
                &v["output"],
                &v["totalBytes"],
                &v["truncated"]
            ),
            (
                &json!(kind),
                &json!("none"),
                &json!(""),
                &json!(0),
                &json!(false)
            ),
            "{id}"
        );
    }
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

// ---------- stop_background_work ----------

/// An agent of `p` named `name` for good (the fake claude does not rename it after its first
/// message), with a process of its own once it is sent something.
async fn named(h: &Harness, p: &Project, name: &str) -> AgentMeta {
    h.core
        .create_agent_with(
            &p.id,
            crate::core::AgentOptions {
                name: Some(name.into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
}

/// The agent `id` is told to run a command in the background (the fake claude's « Ticker »), and
/// it does.
async fn run_command(h: &Harness, id: &str) -> BgTask {
    h.core
        .send_message(id, "commande de fond 60s".into(), vec![])
        .await
        .unwrap();
    h.bg_task(id, "Ticker", true).await
}

/// The agent `id` is told to launch a subagent in the background (the fake claude's « Surveiller
/// le serveur », task `fakelong-agent`), which asks for a permission, is let go on, and runs.
async fn run_subagent(h: &Harness, id: &str) -> SubAgent {
    h.core
        .send_message(id, "sous-agent long".into(), vec![])
        .await
        .unwrap();
    h.wait("the subagent's request", |h| {
        h.agent(id).status == AgentStatus::Waiting
    })
    .await;
    h.core
        .answer_permission(id, "req_subperm", "allow", None)
        .unwrap();
    h.agent(id).plan.unwrap().agents[0].clone()
}

/// The `stop_task` requests the fake claude started in `cwd` got.
fn stops(h: &Harness, cwd: &std::path::Path) -> Vec<Value> {
    h.fake_log(cwd, "control")
        .into_iter()
        .filter(|c| c["subtype"] == "stop_task")
        .collect()
}

#[tokio::test]
async fn a_command_in_the_background_is_stopped_as_the_window_stops_it_and_the_call_is_logged() {
    let h = harness("mcp-bg-stop-command");
    let (p, r) = h.project(false).await;
    let a = named(&h, &p, "chef").await;
    let row = run_command(&h, &a.id).await;
    let c = external(&h).await;
    let v = read(
        &c,
        "stop_background_work",
        json!({ "agent": "chef", "id": row.id }),
    )
    .await;
    assert_eq!(
        v,
        json!({ "agent": { "id": a.id, "name": "chef" }, "id": row.id, "type": "command", "stopped": true })
    );
    // Claude Code was asked to stop its task, once, and its end is the frames' to tell.
    let sent = stops(&h, &r);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0]["task_id"], row.id.as_str());
    h.bg_task_status(&a.id, &row.id, RunStatus::Stopped).await;
    let listed = read(&c, "list_background_work", json!({ "agent": "chef" })).await;
    assert_eq!(
        (
            listed["tasks"][0]["status"].as_str(),
            listed["backgroundTasksRunning"].as_u64()
        ),
        (Some("stopped"), Some(0))
    );
    // The call is in the log: its tool, the agent, the task and how it ended. Nothing else.
    let stopped = h
        .core
        .mcp
        .activity
        .entries()
        .into_iter()
        .find(|e| e.tool == "stop_background_work")
        .unwrap();
    assert_eq!(
        (
            stopped.caller.as_str(),
            stopped.summary.as_str(),
            stopped.outcome,
            stopped.message
        ),
        (
            "Claude (hors Escouade)",
            // As it was asked.
            format!("agent: chef · id: {}", row.id).as_str(),
            "ok",
            None
        )
    );
    // Over, it is not stopped again: the window's own refusal, and nothing more is sent.
    let again = refused(
        &c,
        "stop_background_work",
        json!({ "agent": "chef", "id": row.id }),
    )
    .await;
    assert_eq!(again, "Cette tâche de fond ne tourne plus.");
    assert_eq!(last_entry(&h).3, "refused");
    assert_eq!(stops(&h, &r).len(), 1);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_subagent_in_the_background_is_stopped_by_its_call_or_by_its_task() {
    for by_task in [false, true] {
        let h = harness(&format!("mcp-bg-stop-subagent-{by_task}"));
        let (p, r) = h.project(false).await;
        let a = named(&h, &p, "chef").await;
        let row = run_subagent(&h, &a.id).await;
        assert_eq!(row.task_id.as_deref(), Some("fakelong-agent"));
        let c = external(&h).await;
        let id = if by_task {
            "fakelong-agent"
        } else {
            row.id.as_str()
        };
        let v = read(
            &c,
            "stop_background_work",
            json!({ "agent": a.id, "id": id }),
        )
        .await;
        // Named as the list names it, whichever way it was asked for.
        assert_eq!(
            v,
            json!({ "agent": { "id": a.id, "name": "chef" }, "id": row.id, "type": "subagent", "stopped": true })
        );
        let sent = stops(&h, &r);
        assert_eq!(sent.len(), 1, "{by_task}");
        assert_eq!(sent[0]["task_id"], "fakelong-agent");
        h.wait("the subagent stopped", |h| {
            h.agent(&a.id)
                .plan
                .is_some_and(|p| p.agents[0].status == RunStatus::Stopped)
        })
        .await;
        let listed = read(&c, "list_background_work", json!({ "agent": "chef" })).await;
        assert_eq!(
            (
                listed["subagents"][0]["status"].as_str(),
                listed["subagentsRunning"].as_u64()
            ),
            (Some("stopped"), Some(0))
        );
        // It does not run any more: refused, and nothing more is sent.
        let again = refused(
            &c,
            "stop_background_work",
            json!({ "agent": a.id, "id": id }),
        )
        .await;
        assert_eq!(again, "Ce sous-agent ne tourne plus.");
        assert_eq!(stops(&h, &r).len(), 1);
        c.cancel().await.unwrap();
        h.core.mcp.stop();
    }
}

#[tokio::test]
async fn work_that_cannot_be_stopped_is_refused_in_the_words_of_the_window() {
    let h = harness("mcp-bg-stop-refused");
    let p = project(&h).await;
    let a = put_agent(&h, &p, "chef", 1);
    let sub = |call: &str, status: RunStatus, background: bool, task: Option<&str>| SubAgent {
        background,
        task_id: task.map(str::to_string),
        ..subagent(call, call, status)
    };
    with_plan(
        &h,
        &a.id,
        PlanState {
            agents: vec![
                sub("s-done", RunStatus::Done, true, Some("t-done")),
                sub("s-fg", RunStatus::Running, false, Some("t-fg")),
                sub("s-notask", RunStatus::Running, true, None),
                sub("s-noproc", RunStatus::Running, true, Some("t-noproc")),
            ],
            bg_tasks: vec![
                task("b-done", BgKind::Command, RunStatus::Stopped),
                task("b-noproc", BgKind::Command, RunStatus::Running),
            ],
            workflows: vec![workflow("w-done", RunStatus::Done)],
            ..Default::default()
        },
    );
    let c = external(&h).await;
    let stop = |id: &'static str| {
        let (c, agent) = (&c, a.id.clone());
        async move {
            refused(
                c,
                "stop_background_work",
                json!({ "agent": agent, "id": id }),
            )
            .await
        }
    };
    let sub_over = "Ce sous-agent ne tourne plus.";
    let task_over = "Cette tâche de fond ne tourne plus.";
    let no_process = "Le processus de l’agent ne tourne pas.";
    for (id, said) in [
        ("s-done", sub_over),
        ("t-done", sub_over),
        (
            "s-notask",
            "Claude Code n’a pas encore donné la tâche de ce sous-agent : il ne peut pas être arrêté seul.",
        ),
        ("s-noproc", no_process),
        ("t-noproc", no_process),
        ("b-done", task_over),
        ("w-done", task_over),
        ("b-noproc", no_process),
    ] {
        assert_eq!(stop(id).await, said, "{id}");
        let logged = last_entry(&h);
        assert_eq!(
            (logged.1.as_str(), logged.2, logged.3),
            (
                "stop_background_work",
                format!("agent: {} · id: {id}", a.id),
                "refused"
            ),
            "{id}"
        );
    }
    // In the foreground the agent waits for it: its own « Arrêter » stops it with its turn.
    assert!(stop("s-fg")
        .await
        .starts_with("Ce sous-agent travaille au premier plan"));
    // In the Claude Code view, the interface stops them.
    h.core.agent(&a.id).unwrap().lock().tui = Some(crate::agent::TuiRun::quiet(false));
    assert_eq!(
        stop("s-noproc").await,
        "En vue Claude Code, un sous-agent s’arrête depuis son interface."
    );
    assert_eq!(
        stop("b-noproc").await,
        "En vue Claude Code, une tâche de fond s’arrête depuis son interface."
    );
    h.core.agent(&a.id).unwrap().lock().tui = None;
    // Nothing of that name: something asked that does not fit, not a guard.
    assert_eq!(
        stop("zzz").await,
        "L’agent chef n’a ni sous-agent ni tâche de fond « zzz » : list_background_work les liste."
    );
    assert_eq!(last_entry(&h).3, "error");
    // An agent that is archived runs nothing any more.
    h.core.agent(&a.id).unwrap().lock().meta.archived = true;
    let archived = stop("b-noproc").await;
    assert!(archived.contains("L’agent chef est archivé"), "{archived}");
    assert_eq!(last_entry(&h).3, "refused");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agent_of_escouade_stops_the_work_of_the_agents_it_sees_and_never_its_own() {
    let h = harness("mcp-bg-stop-callers");
    let (p, r) = h.project(false).await;
    let other = other_project(&h, "autre").await;
    let worker = named(&h, &p, "travailleur").await;
    let row = run_command(&h, &worker.id).await;
    let mate = put_agent(&h, &p, "collegue", 2);
    let neighbour = put_agent(&h, &other, "voisin", 3);
    let ask = json!({ "agent": worker.id, "id": row.id });

    // Its own: an agent does not take its work from itself.
    let own = as_agent(&h, &worker.id).await;
    assert_eq!(
        refused(&own, "stop_background_work", ask.clone()).await,
        "Un agent ne peut pas arrêter ses propres tâches de fond."
    );
    let logged = last_entry(&h);
    assert_eq!(
        (logged.0.as_str(), logged.1.as_str(), logged.3),
        ("travailleur", "stop_background_work", "refused")
    );
    // An agent of a private project does not find it, and nothing is sent.
    let vault = make_private(&h, &other_project(&h, "coffre").await);
    let inside = put_agent(&h, &vault, "enferme", 4);
    let shut = as_agent(&h, &inside.id).await;
    let said = refused(&shut, "stop_background_work", ask.clone()).await;
    assert!(said.starts_with("Aucun agent «"), "{said}");
    assert!(
        !said.contains("travailleur") && !said.contains("demo"),
        "{said}"
    );
    assert_eq!(last_entry(&h).3, "error");
    shut.cancel().await.unwrap();
    let far = as_agent(&h, &neighbour.id).await;
    // The Claude Code view, where the interface does it: refused, and nothing is sent either.
    let near = as_agent(&h, &mate.id).await;
    h.core.agent(&worker.id).unwrap().lock().tui = Some(crate::agent::TuiRun::quiet(false));
    assert_eq!(
        refused(&near, "stop_background_work", ask.clone()).await,
        "En vue Claude Code, une tâche de fond s’arrête depuis son interface."
    );
    h.core.agent(&worker.id).unwrap().lock().tui = None;
    assert!(stops(&h, &r).is_empty(), "nothing was asked of Claude Code");
    // An agent of another project that is not private stops it, as one of its own would.
    let v = read(&far, "stop_background_work", ask).await;
    assert_eq!(
        (&v["stopped"], &v["type"], &v["id"]),
        (&json!(true), &json!("command"), &json!(row.id))
    );
    assert_eq!(stops(&h, &r).len(), 1);
    h.bg_task_status(&worker.id, &row.id, RunStatus::Stopped)
        .await;
    let logged = last_entry(&h);
    assert_eq!(
        (logged.0.as_str(), logged.1.as_str(), logged.3),
        ("voisin", "stop_background_work", "ok")
    );
    for client in [own, far, near] {
        client.cancel().await.unwrap();
    }
    h.core.mcp.stop();
}

#[tokio::test]
async fn what_claude_code_itself_refuses_is_a_failure_and_a_workflow_is_stopped_as_a_task() {
    let h = harness("mcp-bg-stop-failure");
    let (p, r) = h.project(false).await;
    let a = named(&h, &p, "chef").await;
    // A process that lives, and does not know the workflow of the plan.
    h.turn(&a.id, "bonjour").await;
    with_plan(
        &h,
        &a.id,
        PlanState {
            workflows: vec![workflow("wf1", RunStatus::Running)],
            ..Default::default()
        },
    );
    let c = external(&h).await;
    let said = refused(
        &c,
        "stop_background_work",
        json!({ "agent": "chef", "id": "wf1" }),
    )
    .await;
    assert!(
        said.contains("Claude Code n’a pas arrêté la tâche")
            && said.contains("No task found with ID: wf1"),
        "{said}"
    );
    // The request went, and Claude Code said no: the call failed, it was not refused by a guard.
    assert_eq!(stops(&h, &r).len(), 1);
    let logged = last_entry(&h);
    assert_eq!(
        (logged.1.as_str(), logged.3),
        ("stop_background_work", "error")
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

// ---------- message_subagent ----------

/// What a message must never leave in the log: its words.
const SECRET: &str = "MARQUEUR-DU-MESSAGE-107";

/// The activity log, and what the window was told of it, hold nothing of `SECRET`.
fn no_trace(h: &Harness) {
    let log = serde_json::to_string(&h.core.mcp.activity.entries()).unwrap();
    assert!(!log.contains(SECRET), "{log}");
    let told: Vec<_> = h
        .events
        .lock()
        .iter()
        .filter(|e| e["type"] == "mcpActivity")
        .cloned()
        .collect();
    assert!(!told.is_empty());
    assert!(!serde_json::to_string(&told).unwrap().contains(SECRET));
}

/// The messages the fake claude started in `cwd` got from Escouade that ask it to pass something
/// on to a subagent (`SendMessage`), their text.
fn passed_on(h: &Harness, cwd: &std::path::Path) -> Vec<String> {
    h.stdin_messages(cwd)
        .into_iter()
        .filter_map(|m| m["message"]["content"].as_str().map(str::to_string))
        .filter(|c| c.contains("SendMessage"))
        .collect()
}

/// What `message_subagent` turns `text` for `subagent` of `agent` down with, and how the activity
/// log says the call ended ("refused" for a guard, "error" for something asked that does not fit).
async fn turned_down(
    h: &Harness,
    c: &Client,
    agent: &str,
    subagent: &str,
    text: String,
) -> (String, &'static str) {
    let said = refused(
        c,
        "message_subagent",
        json!({ "agent": agent, "subagent": subagent, "text": text }),
    )
    .await;
    (said, last_entry(h).3)
}

#[tokio::test]
async fn a_message_to_a_subagent_reaches_it_through_its_agent_under_the_name_of_who_wrote_it() {
    let h = harness("mcp-bg-message");
    let (p, r) = h.project(false).await;
    let a = named(&h, &p, "chef").await;
    let row = run_subagent(&h, &a.id).await;
    let c = external(&h).await;
    let text = format!("  Regarde le port 3001. {SECRET}  ");
    // The agent is at work, its subagent going on in its turn: the message waits for the end of it.
    assert!(h.agent(&a.id).status.is_active());
    let v = read(
        &c,
        "message_subagent",
        json!({ "agent": "chef", "subagent": row.id, "text": text }),
    )
    .await;
    assert_eq!(
        v,
        json!({ "agent": { "id": a.id, "name": "chef" }, "subagent": row.id, "queued": true })
    );
    h.wait("the message", |h| !passed_on(h, &r).is_empty())
        .await;
    let sent = passed_on(&h, &r).pop().unwrap();
    // The agent is asked to pass on the words of who wrote them, under his name: never the user's.
    assert!(
        sent.starts_with(
            "Transmets ce message au sous-agent « Surveiller le serveur » (agentId fakelong-agent)"
        ),
        "{sent}"
    );
    assert!(
        sent.ends_with(&format!(
            "\n\nMessage de Claude (hors Escouade) : Regarde le port 3001. {SECRET}"
        )),
        "{sent}"
    );
    // And it does, with SendMessage, to the task of the subagent.
    h.wait("the agent to pass it on", |h| {
        h.items(&a.id).iter().any(|i| {
            i["kind"] == "tool"
                && i["name"] == "SendMessage"
                && i["input"]["to"] == "fakelong-agent"
                && i["input"]["message"]
                    == format!("Message de Claude (hors Escouade) : Regarde le port 3001. {SECRET}")
        })
    })
    .await;
    h.wait("the turn to end", |h| !h.agent(&a.id).status.is_active())
        .await;

    // Named by its task, the subagent is the same, and the answer names it by its call. The agent
    // is at rest now: the message is not kept waiting.
    let v = read(
        &c,
        "message_subagent",
        json!({ "agent": a.id, "subagent": "fakelong-agent", "text": format!("Après {SECRET}") }),
    )
    .await;
    assert_eq!(
        (&v["subagent"], &v["queued"]),
        (&json!(row.id), &json!(false))
    );
    h.wait("the second message", |h| passed_on(h, &r).len() == 2)
        .await;
    assert!(passed_on(&h, &r)[1].ends_with(&format!(
        "Message de Claude (hors Escouade) : Après {SECRET}"
    )));

    // The log says who asked what to whom, never the words.
    let logged = h
        .core
        .mcp
        .activity
        .entries()
        .into_iter()
        .filter(|e| e.tool == "message_subagent")
        .collect::<Vec<_>>();
    assert_eq!(logged.len(), 2);
    assert_eq!(
        (
            logged[0].caller.as_str(),
            logged[0].summary.as_str(),
            logged[0].outcome,
            &logged[0].message
        ),
        (
            "Claude (hors Escouade)",
            format!("agent: chef · subagent: {}", row.id).as_str(),
            "ok",
            &None
        )
    );
    assert_eq!(
        logged[1].summary,
        format!("agent: {} · subagent: fakelong-agent", a.id)
    );
    no_trace(&h);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_message_that_cannot_go_to_a_subagent_is_refused_in_the_words_of_the_window_and_nothing_is_sent(
) {
    let h = harness("mcp-bg-message-refused");
    let p = project(&h).await;
    let a = put_agent(&h, &p, "chef", 1);
    let sub = |call: &str, background: bool, task: Option<&str>| SubAgent {
        background,
        task_id: task.map(str::to_string),
        ..subagent(call, call, RunStatus::Running)
    };
    with_plan(
        &h,
        &a.id,
        PlanState {
            agents: vec![
                sub("s-ok", true, Some("t-ok")),
                sub("s-fg", false, Some("t-fg")),
                sub("s-notask", true, None),
            ],
            bg_tasks: vec![task("b1", BgKind::Command, RunStatus::Running)],
            workflows: vec![workflow("w1", RunStatus::Running)],
            ..Default::default()
        },
    );
    let c = external(&h).await;
    let write = |subagent: &'static str, text: String| turned_down(&h, &c, &a.id, subagent, text);
    // The guards of the window, in its words.
    let (said, how) = write("s-fg", format!("Au premier plan {SECRET}")).await;
    assert!(
        said.starts_with("Ce sous-agent travaille au premier plan"),
        "{said}"
    );
    assert_eq!(how, "refused");
    let (said, how) = write("s-notask", format!("Sans tâche {SECRET}")).await;
    assert_eq!(
        said,
        "Claude Code n’a pas donné la tâche de ce sous-agent : on ne peut pas lui écrire."
    );
    assert_eq!(how, "refused");
    // Something asked that does not fit: a blank text, a task that is no subagent, nothing of that
    // name.
    let (said, how) = write("s-ok", "  \n ".into()).await;
    assert_eq!(
        (said.as_str(), how),
        ("Écris le message à envoyer.", "error")
    );
    for (id, kind) in [("b1", "command"), ("w1", "workflow")] {
        let (said, how) = write(id, format!("À une tâche {SECRET}")).await;
        assert_eq!(
            said,
            format!("« {id} » n’est pas un sous-agent mais une tâche de fond ({kind}) : seuls les sous-agents reçoivent un message."),
            "{id}"
        );
        assert_eq!(how, "error", "{id}");
    }
    let (said, how) = write("zzz", format!("À personne {SECRET}")).await;
    assert_eq!(
        said,
        "L’agent chef n’a ni sous-agent ni tâche de fond « zzz » : list_background_work les liste."
    );
    assert_eq!(how, "error");
    // The Claude Code view, where it is written from the interface.
    h.core.agent(&a.id).unwrap().lock().meta.terminal = true;
    let (said, how) = write("s-ok", format!("En vue Claude Code {SECRET}")).await;
    assert_eq!(
        said,
        "En vue Claude Code, on écrit à un sous-agent depuis l’interface de Claude Code."
    );
    assert_eq!(how, "refused");
    h.core.agent(&a.id).unwrap().lock().meta.terminal = false;
    // An archived agent takes nothing, nor do its subagents.
    h.core.agent(&a.id).unwrap().lock().meta.archived = true;
    let (said, how) = write("s-ok", format!("Archivé {SECRET}")).await;
    assert!(said.contains("L’agent chef est archivé"), "{said}");
    assert_eq!(how, "refused");
    // Nothing reached the agent, whose process was never started.
    assert_eq!(h.agent(&a.id).prompts, 0);
    no_trace(&h);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

/// The status of the first subagent of the agent `id`'s plan is `status`.
fn first_subagent_stands(h: &Harness, id: &str, status: RunStatus) {
    h.core
        .agent(id)
        .unwrap()
        .lock()
        .meta
        .plan
        .as_mut()
        .unwrap()
        .agents[0]
        .status = status;
}

#[tokio::test]
async fn a_message_to_a_subagent_that_no_longer_runs_is_refused_whatever_its_end_and_nothing_is_sent(
) {
    let h = harness("mcp-bg-message-over");
    let (p, r) = h.project(false).await;
    let a = named(&h, &p, "chef").await;
    let row = run_subagent(&h, &a.id).await;
    assert!(row.background && row.task_id.is_some());
    let c = external(&h).await;
    let prompts = h.agent(&a.id).prompts;
    for status in [
        RunStatus::Done,
        RunStatus::Stopped,
        RunStatus::Failed,
        RunStatus::Interrupted,
    ] {
        first_subagent_stands(&h, &a.id, status);
        // By its call and by its task: the process is there to be written to, only its end is why.
        for id in [row.id.as_str(), "fakelong-agent"] {
            let (said, how) = turned_down(&h, &c, &a.id, id, format!("Trop tard {SECRET}")).await;
            assert_eq!(said, "Ce sous-agent ne tourne plus.", "{status:?} {id}");
            assert_eq!(how, "refused", "{status:?} {id}");
        }
    }
    assert_eq!(h.agent(&a.id).prompts, prompts, "no turn was spent");
    assert!(passed_on(&h, &r).is_empty(), "nothing was passed on");
    no_trace(&h);
    // Going on, the same subagent takes the message.
    first_subagent_stands(&h, &a.id, RunStatus::Running);
    read(
        &c,
        "message_subagent",
        json!({ "agent": a.id, "subagent": row.id, "text": format!("Ça tourne {SECRET}") }),
    )
    .await;
    h.wait("the message", |h| passed_on(h, &r).len() == 1).await;
    no_trace(&h);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_refusals_a_wait_would_not_change_come_before_the_one_of_the_held_back_account() {
    let h = harness("mcp-bg-message-order");
    let (p, r) = h.project(false).await;
    let a = named(&h, &p, "chef").await;
    let row = run_subagent(&h, &a.id).await;
    // Beside the one that runs in the background, with its task: one in the foreground, one with
    // no task told, one that ended.
    {
        let handle = h.core.agent(&a.id).unwrap();
        let mut rt = handle.lock();
        let plan = rt.meta.plan.as_mut().unwrap();
        let sub = |call: &str, status: RunStatus, background: bool, task: Option<&str>| SubAgent {
            background,
            task_id: task.map(str::to_string),
            ..subagent(call, call, status)
        };
        plan.agents.extend([
            sub("s-fg", RunStatus::Running, false, Some("t-fg")),
            sub("s-notask", RunStatus::Running, true, None),
            sub("s-done", RunStatus::Done, true, Some("t-done")),
        ]);
        plan.bg_tasks
            .push(task("b1", BgKind::Command, RunStatus::Running));
    }
    let c = external(&h).await;
    h.core.pause_after_limit(crate::accounts::PRINCIPAL);
    let until = h.core.autopilot_pause().unwrap().until;
    let prompts = h.agent(&a.id).prompts;
    let held = |said: &str| said.contains("pause") && said.contains(&crate::board::clock(until));
    let write = |subagent: &str, text: String| {
        let (subagent, agent) = (subagent.to_string(), a.id.clone());
        let (h, c) = (&h, &c);
        async move { turned_down(h, c, &agent, &subagent, text).await }
    };

    // What waiting would not change is told, in the words of the window, and not the quota.
    for (subagent, expected) in [
        (
            "s-fg",
            "Ce sous-agent travaille au premier plan : l’agent ne lui transmettrait le message qu’une fois fini.",
        ),
        (
            "s-notask",
            "Claude Code n’a pas donné la tâche de ce sous-agent : on ne peut pas lui écrire.",
        ),
        ("s-done", "Ce sous-agent ne tourne plus."),
    ] {
        let (said, how) = write(subagent, format!("Plus tard {SECRET}")).await;
        assert_eq!((said.as_str(), how), (expected, "refused"), "{subagent}");
    }
    h.core.agent(&a.id).unwrap().lock().meta.terminal = true;
    let (said, how) = write("fakelong-agent", format!("Vue {SECRET}")).await;
    assert_eq!(
        said,
        "En vue Claude Code, on écrit à un sous-agent depuis l’interface de Claude Code."
    );
    assert_eq!(how, "refused");
    h.core.agent(&a.id).unwrap().lock().meta.terminal = false;

    // What the call asks for comes before even those: a blank or a too long text, nothing of that
    // name, a task that is no subagent, an archived agent.
    let (said, how) = write(&row.id, "  ".into()).await;
    assert_eq!(
        (said.as_str(), how),
        ("Écris le message à envoyer.", "error")
    );
    let (said, how) = write(&row.id, "x".repeat(32_001)).await;
    assert_eq!(
        (said.as_str(), how),
        ("Le message fait 32001 caractères : 32000 au plus.", "error")
    );
    let (said, _) = write("zzz", format!("À personne {SECRET}")).await;
    assert!(
        said.contains("n’a ni sous-agent ni tâche de fond"),
        "{said}"
    );
    let (said, _) = write("b1", format!("À une tâche {SECRET}")).await;
    assert!(said.contains("n’est pas un sous-agent"), "{said}");
    h.core.agent(&a.id).unwrap().lock().meta.archived = true;
    let (said, how) = write(&row.id, format!("Archivé {SECRET}")).await;
    assert!(said.contains("L’agent chef est archivé"), "{said}");
    assert_eq!(how, "refused");
    h.core.agent(&a.id).unwrap().lock().meta.archived = false;

    // A subagent that can be written to waits for the quota, and the answer says until when.
    let (said, how) = write(&row.id, format!("Pause {SECRET}")).await;
    assert!(held(&said), "{said}");
    assert_eq!(how, "refused");
    let (said, _) = write("fakelong-agent", format!("Pause {SECRET}")).await;
    assert!(held(&said), "{said}");
    // And not one of the others was the quota's to say.
    for subagent in ["s-fg", "s-notask", "s-done"] {
        let (said, _) = write(subagent, format!("Encore {SECRET}")).await;
        assert!(!held(&said), "{subagent}: {said}");
    }
    assert_eq!(h.agent(&a.id).prompts, prompts);
    assert!(passed_on(&h, &r).is_empty());
    no_trace(&h);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_message_to_a_subagent_is_32000_characters_at_most_counted_as_characters_and_never_given_back(
) {
    let h = harness("mcp-bg-message-bound");
    let (p, r) = h.project(false).await;
    let a = named(&h, &p, "chef").await;
    let row = run_subagent(&h, &a.id).await;
    let c = external(&h).await;
    // The words end with the marker the log must never hold.
    let words = |n: usize| format!("{}{SECRET}", "é".repeat(n - SECRET.chars().count()));

    // 32,000 characters of two bytes each go through, the whitespace around them no part of them.
    read(
        &c,
        "message_subagent",
        json!({ "agent": a.id, "subagent": row.id, "text": format!("\n  {}  ", words(32_000)) }),
    )
    .await;
    h.wait("the message", |h| passed_on(h, &r).len() == 1).await;
    assert!(
        passed_on(&h, &r)[0].ends_with(&format!(
            "Message de Claude (hors Escouade) : {}",
            words(32_000)
        )),
        "the words went whole"
    );
    // One more is refused, told in characters, and the words are given back nowhere.
    let prompts = h.agent(&a.id).prompts;
    let (said, how) = turned_down(&h, &c, &a.id, &row.id, words(32_001)).await;
    assert_eq!(said, "Le message fait 32001 caractères : 32000 au plus.");
    assert_eq!(how, "error");
    let (said, _) = turned_down(&h, &c, &a.id, &row.id, words(300_000)).await;
    assert_eq!(said, "Le message fait 300000 caractères : 32000 au plus.");
    assert_eq!(h.agent(&a.id).prompts, prompts);
    assert_eq!(passed_on(&h, &r).len(), 1, "nothing more was passed on");
    let logged = last_entry(&h);
    assert_eq!(
        (logged.1.as_str(), logged.2, logged.3),
        (
            "message_subagent",
            format!("agent: {} · subagent: {}", a.id, row.id),
            "error"
        )
    );
    no_trace(&h);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agent_writes_to_the_subagents_it_sees_and_never_to_its_own() {
    let h = harness("mcp-bg-message-callers");
    let (p, r) = h.project(false).await;
    let other = other_project(&h, "autre").await;
    let vault = make_private(&h, &other_project(&h, "coffre").await);
    let worker = named(&h, &p, "travailleur").await;
    let row = run_subagent(&h, &worker.id).await;
    let inside = put_agent(&h, &vault, "enferme", 2);
    let neighbour = put_agent(&h, &other, "voisin", 3);
    let ask =
        json!({ "agent": worker.id, "subagent": row.id, "text": format!("Bonjour {SECRET}") });

    // Its own subagents: an agent does not write to them (its own words are its own to pass on).
    let own = as_agent(&h, &worker.id).await;
    assert_eq!(
        refused(&own, "message_subagent", ask.clone()).await,
        "Un agent n’écrit pas à ses propres sous-agents."
    );
    let logged = last_entry(&h);
    assert_eq!(
        (logged.0.as_str(), logged.1.as_str(), logged.3),
        ("travailleur", "message_subagent", "refused")
    );
    // An agent of a private project does not find it: nothing is passed on.
    let shut = as_agent(&h, &inside.id).await;
    let said = refused(&shut, "message_subagent", ask.clone()).await;
    assert!(said.starts_with("Aucun agent «"), "{said}");
    assert!(
        !said.contains("travailleur") && !said.contains("demo"),
        "{said}"
    );
    assert_eq!(last_entry(&h).3, "error");
    assert!(passed_on(&h, &r).is_empty());
    // An agent of another project that is not private: its words reach the subagent under its
    // name, as those of an agent of its own project would.
    let far = as_agent(&h, &neighbour.id).await;
    let v = read(&far, "message_subagent", ask).await;
    assert_eq!(v["subagent"], row.id.as_str());
    h.wait("the message", |h| !passed_on(h, &r).is_empty())
        .await;
    assert!(passed_on(&h, &r)[0].ends_with(&format!("\n\nMessage de voisin : Bonjour {SECRET}")));
    let logged = last_entry(&h);
    assert_eq!(
        (logged.0.as_str(), logged.1.as_str(), logged.3),
        ("voisin", "message_subagent", "ok")
    );
    no_trace(&h);
    for client in [own, far, shut] {
        client.cancel().await.unwrap();
    }
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_message_to_a_subagent_costs_its_agent_a_turn_and_waits_while_its_account_is_held_back() {
    let h = harness("mcp-bg-message-quota");
    let (p, r) = h.project(false).await;
    let a = named(&h, &p, "chef").await;
    let row = run_subagent(&h, &a.id).await;
    let c = external(&h).await;
    let ask = json!({ "agent": "chef", "subagent": row.id, "text": format!("Pause {SECRET}") });
    // Past the usage limit: no turn is spent, and the answer says until when.
    h.core.pause_after_limit(crate::accounts::PRINCIPAL);
    let until = h.core.autopilot_pause().unwrap().until;
    let prompts = h.agent(&a.id).prompts;
    let said = refused(&c, "message_subagent", ask.clone()).await;
    assert!(
        said.contains("pause") && said.contains(&crate::board::clock(until)),
        "{said}"
    );
    assert_eq!(last_entry(&h).3, "refused");
    assert_eq!(h.agent(&a.id).prompts, prompts);
    assert!(passed_on(&h, &r).is_empty());
    // Free again: it goes.
    h.core.autopilot_resume();
    read(&c, "message_subagent", ask).await;
    h.wait("the message", |h| !passed_on(h, &r).is_empty())
        .await;
    no_trace(&h);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

/// The call of a subagent in the conversation of the agent `id`, as the stream leaves it.
fn call_of(h: &Harness, id: &str, call: &str, status: &str, report: Option<&str>, result: &str) {
    let mut item = json!({
        "kind": "tool", "id": call, "name": "Agent", "status": status,
        "input": { "description": "Chercher", "prompt": "Cherche." }, "ts": 1,
        "result": { "isError": false, "text": result },
    });
    if let Some(report) = report {
        item["report"] = json!(report);
    }
    h.core
        .agent(id)
        .unwrap()
        .lock()
        .conv
        .apply(&ConvOp::Append { item });
}

#[tokio::test]
async fn the_output_of_a_subagent_is_its_report_and_nothing_before_it_has_handed_one_back() {
    let h = harness("mcp-bg-output-report");
    let p = project(&h).await;
    let a = put_agent(&h, &p, "chef", 1);
    let launched = "Async agent launched successfully.\nagentId: bg1";
    let sub = |call: &str, status: RunStatus, task: Option<&str>| SubAgent {
        background: true,
        task_id: task.map(str::to_string),
        ..subagent(call, "Chercher", status)
    };
    with_plan(
        &h,
        &a.id,
        PlanState {
            agents: vec![
                sub("c-done", RunStatus::Done, Some("task-done")),
                sub("c-running", RunStatus::Running, Some("task-running")),
                sub("c-silent", RunStatus::Done, None),
                sub("c-fg", RunStatus::Done, None),
            ],
            ..Default::default()
        },
    );
    let long = "é".repeat(8_000);
    call_of(&h, &a.id, "c-done", "ok", Some(&long), launched);
    call_of(&h, &a.id, "c-running", "running", None, "");
    call_of(&h, &a.id, "c-silent", "ok", None, launched);
    call_of(&h, &a.id, "c-fg", "ok", None, "Le résultat au premier plan");
    let c = external(&h).await;
    // Its report, whole as the core keeps it, whatever is asked: `bytes` is for outputs.
    for (id, bytes) in [
        ("c-done", None),
        ("task-done", Some(10)),
        ("c-done", Some(1)),
    ] {
        let v = output_of(&c, &a.id, id, bytes).await;
        assert_eq!(
            (&v["id"], &v["type"], &v["status"], &v["source"]),
            (
                &json!("c-done"),
                &json!("subagent"),
                &json!("done"),
                &json!("report")
            ),
            "{id} {bytes:?}"
        );
        assert_eq!(v["output"], long.as_str());
        assert_eq!(
            (&v["totalBytes"], &v["truncated"]),
            (&json!(16_000), &json!(false))
        );
        assert_eq!(v["agent"], json!({ "id": a.id, "name": "chef" }));
    }
    // The result of a subagent that worked in the foreground is its report too.
    let v = output_of(&c, &a.id, "c-fg", None).await;
    assert_eq!(
        (&v["source"], &v["output"]),
        (&json!("report"), &json!("Le résultat au premier plan"))
    );
    assert_eq!(v["totalBytes"], "Le résultat au premier plan".len());
    // Nothing handed back: it still runs, or the answer to its launch is all there is.
    for (id, status) in [
        ("c-running", "running"),
        ("task-running", "running"),
        ("c-silent", "done"),
    ] {
        let v = output_of(&c, &a.id, id, None).await;
        assert_eq!(
            (
                &v["status"],
                &v["source"],
                &v["output"],
                &v["totalBytes"],
                &v["truncated"]
            ),
            (
                &json!(status),
                &json!("none"),
                &json!(""),
                &json!(0),
                &json!(false)
            ),
            "{id}"
        );
    }
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn in_the_claude_code_view_the_output_of_a_subagent_is_the_handback_of_its_transcript() {
    let h = harness("mcp-bg-output-report-tui");
    let p = project(&h).await;
    let a = put_agent(&h, &p, "chef", 1);
    let root = h.dir.join("claude-view");
    let subs = root.join("s9").join("subagents");
    std::fs::create_dir_all(&subs).unwrap();
    std::fs::write(root.join("s9.jsonl"), "").unwrap();
    let entry = |message: &str| {
        json!({
            "isSidechain": true, "type": "assistant", "uuid": "u1",
            "timestamp": "2026-10-16T10:00:00.000Z",
            "message": { "role": "assistant", "content": [
                { "type": "tool_use", "id": "hb1", "name": "SubagentHandback", "input": { "message": message } },
            ] },
        })
        .to_string()
    };
    std::fs::write(
        subs.join("agent-t1.jsonl"),
        entry("Le rapport du transcript") + "\n",
    )
    .unwrap();
    {
        let handle = h.core.agent(&a.id).unwrap();
        let mut rt = handle.lock();
        rt.meta.terminal = true;
        rt.meta.transcript_read = Some((root.join("s9.jsonl").to_string_lossy().into(), 0));
    }
    with_plan(
        &h,
        &a.id,
        PlanState {
            agents: vec![
                SubAgent {
                    task_id: Some("t1".into()),
                    ..subagent("c1", "Chercher", RunStatus::Done)
                },
                SubAgent {
                    task_id: Some("t2".into()),
                    ..subagent("c2", "Autre", RunStatus::Running)
                },
            ],
            ..Default::default()
        },
    );
    call_of(
        &h,
        &a.id,
        "c1",
        "ok",
        None,
        "Async agent launched successfully.\nagentId: t1",
    );
    let c = external(&h).await;
    let v = output_of(&c, &a.id, "c1", None).await;
    assert_eq!(
        (&v["source"], &v["output"], &v["totalBytes"]),
        (
            &json!("report"),
            &json!("Le rapport du transcript"),
            &json!("Le rapport du transcript".len())
        )
    );
    // Its files say nothing for the other one yet.
    let v = output_of(&c, &a.id, "c2", None).await;
    assert_eq!((&v["source"], &v["output"]), (&json!("none"), &json!("")));
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn reading_the_output_of_a_subagent_in_the_claude_code_view_leaves_its_row_as_it_is() {
    // What Claude Code wrote of a real subagent the user stopped in its interface: its files say
    // it ended, which the window's view of it acts on and a read through the server does not.
    const STOPPED: &str = include_str!("../../../tests/fixtures/real-sidechain.jsonl");
    let h = harness("mcp-bg-output-report-stopped");
    let p = project(&h).await;
    let a = put_agent(&h, &p, "chef", 1);
    let root = h.dir.join("claude-view");
    let subs = root.join("s9").join("subagents");
    std::fs::create_dir_all(&subs).unwrap();
    std::fs::write(root.join("s9.jsonl"), "").unwrap();
    let file = subs.join("agent-a3c004cb08b483eb9.jsonl");
    std::fs::write(&file, STOPPED).unwrap();
    {
        let handle = h.core.agent(&a.id).unwrap();
        let mut rt = handle.lock();
        rt.meta.terminal = true;
        rt.meta.transcript_read = Some((root.join("s9.jsonl").to_string_lossy().into(), 0));
    }
    with_plan(
        &h,
        &a.id,
        PlanState {
            agents: vec![SubAgent {
                background: true,
                task_id: Some("a3c004cb08b483eb9".into()),
                ..subagent("c1", "Lent", RunStatus::Running)
            }],
            ..Default::default()
        },
    );
    call_of(
        &h,
        &a.id,
        "c1",
        "ok",
        None,
        "Async agent launched successfully.\nagentId: a3c004cb08b483eb9",
    );
    let told = || {
        h.events
            .lock()
            .iter()
            .filter(|e| e["type"] == "agent" && e["agent"]["id"] == a.id.as_str())
            .count()
    };
    let c = external(&h).await;
    let told_before = told();
    let still_running = |listed: &Value| {
        (
            listed["subagents"][0]["status"].clone(),
            listed["subagents"][0].get("endedAt").cloned(),
            listed["subagentsRunning"].clone(),
        )
    };
    // It handed nothing back and its files say it was stopped: nothing to read, and it goes on
    // running as far as the list says.
    let v = output_of(&c, &a.id, "c1", None).await;
    assert_eq!(
        (&v["status"], &v["source"], &v["output"]),
        (&json!("running"), &json!("none"), &json!(""))
    );
    let listed = read(&c, "list_background_work", json!({ "agent": "chef" })).await;
    assert_eq!(still_running(&listed), (json!("running"), None, json!(1)));
    // A hand-back that comes after is its report, and reading it does not end the row either.
    let handback = json!({
        "isSidechain": true, "type": "assistant", "uuid": "u9",
        "timestamp": "2026-10-16T10:00:00.000Z",
        "message": { "role": "assistant", "content": [
            { "type": "tool_use", "id": "hb9", "name": "SubagentHandback", "input": { "message": "Rapport tardif" } },
        ] },
    });
    std::fs::write(&file, format!("{STOPPED}\n{handback}\n")).unwrap();
    for _ in 0..2 {
        let v = output_of(&c, &a.id, "c1", None).await;
        assert_eq!(
            (&v["source"], &v["output"], &v["status"]),
            (
                &json!("report"),
                &json!("Rapport tardif"),
                &json!("running")
            )
        );
    }
    let listed = read(&c, "list_background_work", json!({ "agent": "chef" })).await;
    assert_eq!(still_running(&listed), (json!("running"), None, json!(1)));
    // The window was told nothing, and the call keeps no trace of an end.
    assert_eq!(told(), told_before);
    let item = h
        .items(&a.id)
        .into_iter()
        .find(|i| i["id"] == "c1")
        .unwrap();
    assert!(item["run"].is_null(), "{item}");
    assert_eq!(h.agent(&a.id).plan.unwrap().agents[0].ended_at, None);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_unknown_id_is_told_and_an_agent_of_escouade_reads_the_tasks_of_the_projects_it_sees() {
    let h = harness("mcp-bg-output-access");
    let p = project(&h).await;
    let other = other_project(&h, "autre").await;
    let calling = put_agent(&h, &p, "appelant", 1);
    let mate = put_agent(&h, &p, "collegue", 2);
    let neighbour = put_agent(&h, &other, "voisin", 3);
    for agent in [&calling, &mate, &neighbour] {
        with_plan(
            &h,
            &agent.id,
            PlanState {
                bg_tasks: vec![
                    with_output(
                        &h,
                        &format!("b-{}", agent.name),
                        BgKind::Command,
                        RunStatus::Done,
                        format!("sortie de {}", agent.name).as_bytes(),
                    ),
                    // A row that has no id (a frame that never said it) is nobody's to be named
                    // by a blank.
                    task("", BgKind::Command, RunStatus::Done),
                ],
                ..Default::default()
            },
        );
    }
    let c = external(&h).await;
    // An id the agent does not have, or a task of another agent: told, with where to look.
    for id in ["zzz", "b-voisin", "  "] {
        let said = refused(
            &c,
            "get_background_output",
            json!({ "agent": "collegue", "id": id }),
        )
        .await;
        assert_eq!(
            said,
            format!(
                "L’agent collegue n’a ni sous-agent ni tâche de fond « {} » : list_background_work les liste.",
                id.trim()
            )
        );
    }
    assert_eq!(last_entry(&h).3, "error");
    // Claude outside Escouade reads any project's.
    let v = output_of(&c, "voisin", "b-voisin", None).await;
    assert_eq!(v["output"], "sortie de voisin");
    // An agent of Escouade: its own tasks, those of its project's agents, and those of another
    // project that is not private.
    let from = as_agent(&h, &calling.id).await;
    assert_eq!(
        output_of(&from, "appelant", "b-appelant", None).await["output"],
        "sortie de appelant"
    );
    assert_eq!(
        output_of(&from, "collegue", "b-collegue", None).await["output"],
        "sortie de collegue"
    );
    assert_eq!(
        output_of(&from, "voisin", "b-voisin", None).await["output"],
        "sortie de voisin"
    );
    assert_eq!(
        last_entry(&h),
        (
            "appelant".into(),
            "get_background_output".into(),
            "agent: voisin · id: b-voisin".into(),
            "ok"
        )
    );
    // Not those of a private project, which it does not find.
    make_private(&h, &other);
    let said = refused(
        &from,
        "get_background_output",
        json!({ "agent": "voisin", "id": "b-voisin" }),
    )
    .await;
    assert!(said.starts_with("Aucun agent « voisin »"), "{said}");
    assert!(
        !said.contains("sortie de voisin") && !said.contains("autre"),
        "{said}"
    );
    assert_eq!(last_entry(&h).3, "error");
    from.cancel().await.unwrap();
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

// ---------- a real client ----------

#[tokio::test]
async fn a_real_client_lists_the_four_tools_with_their_schemas_and_reads_the_work_of_an_agent() {
    let h = harness("mcp-bg-client");
    let p = project(&h).await;
    let a = put_agent(&h, &p, "chef", 1);
    with_plan(
        &h,
        &a.id,
        PlanState {
            agents: vec![subagent("toolu_1", "Chercher", RunStatus::Running)],
            bg_tasks: vec![task("b1", BgKind::Command, RunStatus::Running)],
            ..Default::default()
        },
    );
    let c = external(&h).await;
    let tools = c.list_all_tools().await.unwrap();
    let spec = |name: &str| {
        let tool = tools.iter().find(|t| t.name == name).unwrap();
        serde_json::to_value(tool).unwrap()
    };
    let schema = |name: &str| spec(name)["inputSchema"].clone();
    // Two read, two act; the one that stops is destructive, none reaches beyond Escouade.
    for (name, read_only, destructive) in [
        ("list_background_work", true, None),
        ("get_background_output", true, None),
        ("stop_background_work", false, Some(true)),
        ("message_subagent", false, Some(false)),
    ] {
        let spec = spec(name);
        assert_eq!(spec["annotations"]["readOnlyHint"], read_only, "{name}");
        assert_eq!(spec["annotations"]["openWorldHint"], false, "{name}");
        if let Some(destructive) = destructive {
            assert_eq!(
                spec["annotations"]["destructiveHint"], destructive,
                "{name}"
            );
        }
        // What the model reads is English, on one line, and says something.
        let said = spec["description"].as_str().unwrap();
        assert!(said.len() > 200 && !said.contains('\n'), "{name}: {said}");
        assert_eq!(crate::i18n::check::french_in(said), None, "{name}: {said}");
    }
    // The arguments of each, what they take, which ones are needed.
    for (name, arguments, needed) in [
        (
            "list_background_work",
            &["agent", "project"][..],
            &["agent"][..],
        ),
        (
            "get_background_output",
            &["agent", "bytes", "id", "project"],
            &["agent", "id"],
        ),
        (
            "stop_background_work",
            &["agent", "id", "project"],
            &["agent", "id"],
        ),
        (
            "message_subagent",
            &["agent", "project", "subagent", "text"],
            &["agent", "subagent", "text"],
        ),
    ] {
        let schema = schema(name);
        let mut given: Vec<&str> = schema["properties"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        given.sort();
        assert_eq!(given, arguments, "{name}");
        let mut required: Vec<String> = serde_json::from_value(schema["required"].clone()).unwrap();
        required.sort();
        assert_eq!(required, needed, "{name}");
        for (arg, spec) in schema["properties"].as_object().unwrap() {
            let said = spec["description"].as_str().unwrap_or_default();
            assert!(
                said.len() > 10 && !said.contains('\n'),
                "{name}.{arg}: {spec}"
            );
            assert_eq!(crate::i18n::check::french_in(said), None, "{name}.{arg}");
        }
    }
    // `bytes` is a whole number; the others are texts.
    let bytes = schema("get_background_output")["properties"]["bytes"].to_string();
    assert!(
        bytes.contains("integer") && !bytes.contains("string"),
        "{bytes}"
    );
    // The bounds are said where they are asked for: what an answer carries, what a message takes.
    let argument = |name: &str, arg: &str| {
        schema(name)["properties"][arg]["description"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let tool = |name: &str| spec(name)["description"].as_str().unwrap().to_string();
    for said in [
        argument("get_background_output", "bytes"),
        tool("get_background_output"),
    ] {
        assert!(
            said.contains("65536") && said.contains("8192") && !said.contains("262144"),
            "{said}"
        );
    }
    for (name, said) in [
        ("message_subagent", argument("message_subagent", "text")),
        ("message_subagent", tool("message_subagent")),
        ("send_message", argument("send_message", "text")),
        ("send_message", tool("send_message")),
    ] {
        assert!(said.contains("32,000 characters"), "{name}: {said}");
    }
    // Which projects an agent sees is said.
    let said = tool("list_background_work");
    assert!(
        said.contains("every project that is not private")
            && said.contains("seen by its own agents alone"),
        "{said}"
    );
    for (name, arg) in [
        ("stop_background_work", "id"),
        ("message_subagent", "subagent"),
        ("message_subagent", "text"),
    ] {
        let text = schema(name)["properties"][arg]["type"].to_string();
        assert!(text.contains("string"), "{name}.{arg}: {text}");
    }
    // The counts are said where they are given.
    for name in ["list_agents", "get_agent_summary"] {
        let said = spec(name)["description"].as_str().unwrap().to_string();
        assert!(
            said.contains("subagentsRunning") && said.contains("backgroundTasksRunning"),
            "{name}: {said}"
        );
    }
    // And a call goes through the wire.
    let listed = read(&c, "list_background_work", json!({ "agent": "chef" })).await;
    assert_eq!(counts(&listed), (json!(1), json!(1)), "{listed}");
    assert_eq!(ids(&listed, "subagents"), ["toolu_1"]);
    assert_eq!(ids(&listed, "tasks"), ["b1"]);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}
