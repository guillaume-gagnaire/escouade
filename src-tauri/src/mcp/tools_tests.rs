//! The tools that read, called by a real MCP client (rmcp's) on a test core: real git
//! repositories, the fake `claude`, tickets put on the boards as they are (their autopilot off).

use super::requests_tests::{asks, bash, live};
use super::tests::{client, names, started, EXPOSED, READING};
use crate::agent::AgentRt;
use crate::core::RESUME_MARGIN_MS;
use crate::core_tests::{git, harness, Harness};
use crate::model::*;
use crate::usage::Reading;
use base64::Engine;
use rmcp::model::CallToolRequestParams;
use rmcp::service::RunningService;
use rmcp::RoleClient;
use serde_json::{json, Value};
use std::future::Future;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(super) type Client = RunningService<RoleClient, ()>;

/// The project of the harness (its repository on `main`), its autopilot off: a ticket put on its
/// board stays where it is put.
pub(super) async fn project(h: &Harness) -> Project {
    let (p, _) = h.project(false).await;
    still(h, &p)
}

/// Another project, named `name`, in a folder of its own (made a repository).
pub(super) async fn other_project(h: &Harness, name: &str) -> Project {
    let dir = h.dir.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    let p = h
        .core
        .create_project(
            &dir.to_string_lossy(),
            name,
            "oklch(0.7 0.1 200)",
            false,
            None,
            true,
        )
        .await
        .unwrap();
    still(h, &p)
}

pub(super) fn still(h: &Harness, p: &Project) -> Project {
    h.core
        .board_set(
            &p.id,
            BoardSettings {
                autopilot: false,
                ..p.board.clone()
            },
        )
        .unwrap()
}

/// A ticket of `p`, in `column`, `at` its creation (and its start, test and end, as its column
/// has them): its rank in « À faire » too.
pub(super) fn ticket(p: &Project, key: &str, title: &str, column: Column, at: i64) -> Ticket {
    Ticket {
        id: format!("t-{}-{}", p.id, key.to_lowercase()),
        project_id: p.id.clone(),
        key: key.into(),
        title: title.into(),
        column,
        max_loops: 5,
        rank: at,
        created_at: at,
        started_at: (column != Column::Todo).then_some(at),
        review_at: matches!(column, Column::Review | Column::Done).then_some(at),
        done_at: (column == Column::Done).then_some(at),
        ..Default::default()
    }
}

/// `t` on its project's board.
pub(super) fn put(h: &Harness, t: Ticket) -> Ticket {
    h.core.tickets.write().push(t.clone());
    t
}

/// An agent of `p` named `name`, made as the app keeps one (no process).
pub(super) fn put_agent(h: &Harness, p: &Project, name: &str, at: i64) -> AgentMeta {
    let meta = AgentMeta {
        id: format!("a-{}-{name}", p.id),
        project_id: p.id.clone(),
        name: name.into(),
        model: "sonnet".into(),
        cwd: p.path.clone(),
        created_at: at,
        account: crate::accounts::PRINCIPAL.into(),
        ..Default::default()
    };
    let rt = AgentRt::new(meta.clone(), &h.core.data.conversations());
    h.core
        .agents
        .write()
        .insert(meta.id.clone(), Arc::new(parking_lot::Mutex::new(rt)));
    meta
}

/// The agent `agent_id` works on the ticket `ticket_id`.
pub(super) fn link(h: &Harness, agent_id: &str, ticket_id: &str) {
    h.core.agent(agent_id).unwrap().lock().meta.ticket_id = Some(ticket_id.into());
    for t in h.core.tickets.write().iter_mut() {
        if t.id == ticket_id {
            t.agent_id = Some(agent_id.into());
        }
    }
}

/// Claude outside Escouade, connected to the harness's server.
pub(super) async fn external(h: &Harness) -> Client {
    let (port, token) = started(h);
    client(port, &token).await.unwrap()
}

/// The agent `agent_id`, connected with a token of its own.
pub(super) async fn as_agent(h: &Harness, agent_id: &str) -> Client {
    let (port, _) = started(h);
    client(port, &h.core.mcp.register_agent(agent_id))
        .await
        .unwrap()
}

/// What `tool` answers to `args`: its text, and whether it is an error the model reads.
pub(super) async fn call(c: &Client, tool: &'static str, args: Value) -> (String, bool) {
    let mut params = CallToolRequestParams::new(tool);
    if let Value::Object(args) = args {
        params = params.with_arguments(args);
    }
    let answer = serde_json::to_value(c.call_tool(params).await.unwrap()).unwrap();
    let text = answer["content"][0]["text"].as_str().unwrap().to_string();
    (text, answer["isError"] == true)
}

/// What `tool` answers to `args`, a JSON that is no error.
pub(super) async fn read(c: &Client, tool: &'static str, args: Value) -> Value {
    let (text, error) = call(c, tool, args).await;
    assert!(!error, "{tool}: {text}");
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{tool}: {e}: {text}"))
}

/// The error `tool` answers to `args`.
pub(super) async fn refused(c: &Client, tool: &'static str, args: Value) -> String {
    let (text, error) = call(c, tool, args).await;
    assert!(error, "{tool} answered: {text}");
    text
}

/// The activity log's last entry: (caller, tool, summary, outcome).
pub(super) fn last_entry(h: &Harness) -> (String, String, String, &'static str) {
    let e = h.core.mcp.activity.entries().pop().unwrap();
    (e.caller, e.tool, e.summary, e.outcome)
}

pub(super) fn keys(rows: &Value) -> Vec<String> {
    rows.as_array()
        .unwrap()
        .iter()
        .map(|r| r["key"].as_str().unwrap().to_string())
        .collect()
}

/// A junction (a link to a folder that needs no privilege) at `link` to `target`. It goes when it
/// is dropped, and only it: `remove_dir_all` through it would empty the target.
#[cfg(windows)]
pub(super) struct Junction(std::path::PathBuf);

#[cfg(windows)]
pub(super) fn junction(link: &std::path::Path, target: &std::path::Path) -> Junction {
    let made = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .unwrap();
    assert!(
        made.status.success(),
        "mklink: {}{}",
        String::from_utf8_lossy(&made.stdout),
        String::from_utf8_lossy(&made.stderr)
    );
    Junction(link.to_path_buf())
}

#[cfg(windows)]
impl Drop for Junction {
    fn drop(&mut self) {
        // `RemoveDirectory` on a junction takes the link, not what it leads to.
        let _ = std::fs::remove_dir(&self.0);
    }
}

#[tokio::test]
async fn the_reading_tools_say_they_only_read_and_what_they_take() {
    let h = harness("mcp-read-tools");
    let c = external(&h).await;
    let tools = c.list_all_tools().await.unwrap();
    let mut listed = names(&tools);
    listed.retain(|n| n != "whoami");
    assert_eq!(listed, EXPOSED);
    for t in tools.iter().filter(|t| READING.contains(&&*t.name)) {
        let v = serde_json::to_value(t).unwrap();
        assert!(
            v["description"].as_str().unwrap().len() > 40,
            "{}: {v}",
            t.name
        );
        assert_eq!(v["annotations"]["readOnlyHint"], true, "{}: {v}", t.name);
        assert_eq!(v["annotations"]["openWorldHint"], false, "{}: {v}", t.name);
    }
    let schema = |name: &str| {
        let t = tools.iter().find(|t| t.name == name).unwrap();
        serde_json::to_value(&t.input_schema).unwrap()
    };
    let required = |name: &str| schema(name)["required"].clone();
    assert_eq!(required("list_tickets"), json!(["project"]));
    assert_eq!(required("get_agent_summary"), json!(["agent"]));
    assert_eq!(required("wait_ticket"), json!(["tickets"]));
    assert_eq!(required("get_board"), json!(["project"]));
    assert_eq!(required("get_workflow"), json!(["project"]));
    assert_eq!(required("list_background_work"), json!(["agent"]));
    assert_eq!(required("get_background_output"), json!(["agent", "id"]));
    for name in [
        "list_projects",
        "list_agents",
        "list_requests",
        "get_ticket",
        "get_ticket_image",
        "get_usage",
    ] {
        assert!(
            required(name).as_array().is_none_or(|r| r.is_empty()),
            "{name}"
        );
    }
    // Every argument says what it takes, in English, and a column is one of the four.
    for name in [
        "list_agents",
        "list_requests",
        "list_tickets",
        "get_ticket",
        "get_ticket_image",
        "get_agent_summary",
        "wait_ticket",
        "get_board",
        "get_workflow",
        "list_background_work",
        "get_background_output",
    ] {
        let s = schema(name);
        for (arg, spec) in s["properties"].as_object().unwrap() {
            assert!(
                spec["description"]
                    .as_str()
                    .is_some_and(|d| d.len() > 10 && !d.contains('\n')),
                "{name}.{arg}: {spec}"
            );
        }
    }
    let column = schema("list_tickets")["properties"]["column"].to_string();
    for c in ["todo", "doing", "review", "done"] {
        assert!(column.contains(c), "{column}");
    }
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_projects_are_listed_with_their_branch_agents_and_tickets_by_column() {
    let h = harness("mcp-list-projects");
    let p = project(&h).await;
    let other = other_project(&h, "autre").await;
    put_agent(&h, &p, "un", 1);
    let gone = put_agent(&h, &p, "deux", 2);
    h.core.agent(&gone.id).unwrap().lock().meta.archived = true;
    for (key, column) in [
        ("DEM-1", Column::Todo),
        ("DEM-2", Column::Todo),
        ("DEM-3", Column::Doing),
        ("DEM-4", Column::Done),
    ] {
        put(&h, ticket(&p, key, key, column, 1));
    }
    let c = external(&h).await;
    let projects = read(&c, "list_projects", json!({})).await;
    assert_eq!(
        projects[0],
        json!({
            "id": p.id,
            "name": "demo",
            "path": p.path,
            "branch": "main",
            // The archived one left out.
            "agents": 1,
            "tickets": { "todo": 2, "doing": 1, "review": 0, "done": 1 },
            "remoteControl": false,
            "remoteControlAll": false,
            // Its harness, and what the tools take for it (ESC-248): no model told yet.
            "engine": "claude",
            "models": [],
            "efforts": ["low", "medium", "high", "xhigh", "max"],
        })
    );
    assert_eq!(projects.as_array().unwrap().len(), 2);
    assert_eq!(
        (
            &projects[1]["id"],
            &projects[1]["agents"],
            &projects[1]["tickets"]
        ),
        (
            &json!(other.id),
            &json!(0),
            &json!({ "todo": 0, "doing": 0, "review": 0, "done": 0 })
        )
    );
    // The call is in the activity log.
    assert_eq!(
        last_entry(&h),
        (
            "Claude (hors Escouade)".into(),
            "list_projects".into(),
            String::new(),
            "ok"
        )
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_agents_are_listed_with_what_waits_their_ticket_model_account_and_cost() {
    let h = harness("mcp-list-agents");
    let p = project(&h).await;
    let other = other_project(&h, "autre").await;
    let t = put(
        &h,
        ticket(&p, "DEM-1", "Ajouter le fichier", Column::Doing, 1),
    );
    let worker = put_agent(&h, &p, "dem-1-ajouter", 1);
    link(&h, &worker.id, &t.id);
    // One asks for a permission: it waits for the user.
    let asking = h.core.create_agent(&p.id, None).await.unwrap().meta;
    h.core
        .send_message(&asking.id, "permission".into(), vec![])
        .await
        .unwrap();
    h.wait("a permission asked", |h| {
        h.view(&asking.id).pending.len() == 1
    })
    .await;
    let archived = put_agent(&h, &p, "vieux", 3);
    h.core.agent(&archived.id).unwrap().lock().meta.archived = true;
    let elsewhere = put_agent(&h, &other, "ailleurs", i64::MAX);
    let c = external(&h).await;
    let all = read(&c, "list_agents", json!({})).await;
    let ids: Vec<&str> = all
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["id"].as_str().unwrap())
        .collect();
    // Oldest first, the archived one left out.
    assert_eq!(ids, [worker.id.as_str(), &asking.id, &elsewhere.id]);
    assert_eq!(
        all[0],
        json!({
            "id": worker.id,
            "name": "dem-1-ajouter",
            "project": "demo",
            "projectId": p.id,
            "status": "idle",
            "waiting": false,
            "requests": 0,
            "ticket": "DEM-1",
            "engine": "claude",
            "model": "sonnet",
            "account": "Principal",
            "cost": 0.0,
            "subagentsRunning": 0,
            "backgroundTasksRunning": 0,
            "remoteControl": false,
            "remoteSource": null,
            "remoteState": null,
            "interfaceRemote": null,
            "workflow": null,
        })
    );
    assert_eq!(
        (
            &all[1]["status"],
            &all[1]["waiting"],
            &all[1]["requests"],
            &all[1]["ticket"]
        ),
        (&json!("waiting"), &json!(true), &json!(1), &json!(null))
    );
    assert!(all[1]["cost"].is_number(), "{}", all[1]);
    assert_eq!(
        (&all[2]["project"], &all[2]["projectId"]),
        (&json!("autre"), &json!(other.id))
    );
    // One project: by its name in any case, or by its id.
    let demo = read(&c, "list_agents", json!({ "project": "DEMO" })).await;
    assert_eq!(demo.as_array().unwrap().len(), 2);
    assert_eq!(
        last_entry(&h),
        (
            "Claude (hors Escouade)".into(),
            "list_agents".into(),
            "project: DEMO".into(),
            "ok"
        )
    );
    let autre = read(&c, "list_agents", json!({ "project": other.id })).await;
    assert_eq!(autre[0]["id"], json!(elsewhere.id));
    assert_eq!(autre.as_array().unwrap().len(), 1);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_tickets_of_a_project_are_listed_in_the_kanban_order() {
    let h = harness("mcp-list-tickets");
    let p = project(&h).await;
    let worker = put_agent(&h, &p, "dem-3-c", 1);
    // « À faire » by priority, « En cours » and « À tester » by arrival, « Terminé » newest
    // first.
    let a = put(
        &h,
        Ticket {
            rank: 1,
            ..ticket(&p, "DEM-1", "A", Column::Todo, 2)
        },
    );
    put(
        &h,
        Ticket {
            rank: 2,
            after: vec![a.id.clone()],
            ..ticket(&p, "DEM-2", "B", Column::Todo, 1)
        },
    );
    let c3 = put(&h, ticket(&p, "DEM-3", "C", Column::Doing, 5));
    link(&h, &worker.id, &c3.id);
    put(
        &h,
        Ticket {
            blocked: Some("Conflit de merge".into()),
            ..ticket(&p, "DEM-4", "D", Column::Doing, 3)
        },
    );
    put(&h, ticket(&p, "DEM-5", "E", Column::Review, 4));
    put(&h, ticket(&p, "DEM-6", "F", Column::Done, 1));
    put(&h, ticket(&p, "DEM-7", "G", Column::Done, 9));
    // Another project's are not among them.
    let other = other_project(&h, "autre").await;
    put(&h, ticket(&other, "AUT-1", "X", Column::Todo, 0));
    let c = external(&h).await;
    let all = read(&c, "list_tickets", json!({ "project": "demo" })).await;
    assert_eq!(
        keys(&all),
        ["DEM-1", "DEM-2", "DEM-4", "DEM-3", "DEM-5", "DEM-7", "DEM-6"]
    );
    assert_eq!(
        all[1],
        json!({
            "id": format!("t-{}-dem-2", p.id),
            "key": "DEM-2",
            "title": "B",
            "column": "todo",
            "after": ["DEM-1"],
            "agent": null,
            "blocked": null,
        })
    );
    let doing = read(
        &c,
        "list_tickets",
        json!({ "project": p.id, "column": "doing" }),
    )
    .await;
    assert_eq!(keys(&doing), ["DEM-4", "DEM-3"]);
    assert_eq!(doing[0]["blocked"], "Conflit de merge");
    assert_eq!(doing[1]["agent"], "dem-3-c");
    assert_eq!(
        last_entry(&h),
        (
            "Claude (hors Escouade)".into(),
            "list_tickets".into(),
            format!("project: {} · column: doing", p.id),
            "ok"
        )
    );
    // A column that is none of the four: an error the model reads, in the log too, neither of which
    // repeats the value it gave.
    let said = refused(
        &c,
        "list_tickets",
        json!({ "project": "demo", "column": "later" }),
    )
    .await;
    assert!(!said.contains("later") && said.contains("schéma"), "{said}");
    let e = h.core.mcp.activity.entries().pop().unwrap();
    assert_eq!((e.tool.as_str(), e.outcome), ("list_tickets", "error"));
    assert_eq!(
        e.message.as_deref(),
        Some(
            "Appel refusé par le serveur : les arguments ne correspondent pas au schéma de l’outil"
        ),
        "{e:?}"
    );
    // Logged once only.
    let logged = |h: &Harness| h.core.mcp.activity.entries().len();
    let before = logged(&h);
    read(&c, "list_tickets", json!({ "project": "demo" })).await;
    assert_eq!(logged(&h), before + 1);
    // A tool the server does not have: a protocol error, logged.
    assert!(c
        .call_tool(CallToolRequestParams::new("nothing_here"))
        .await
        .is_err());
    let (_, tool, _, outcome) = last_entry(&h);
    assert_eq!((tool.as_str(), outcome), ("nothing_here", "error"));
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_is_read_whole_by_its_key_in_any_case_or_its_id() {
    let h = harness("mcp-get-ticket");
    let p = project(&h).await;
    let first = put(&h, ticket(&p, "DEM-1", "Le socle", Column::Done, 1));
    let t = put(
        &h,
        Ticket {
            description: "Écrire le fichier.\nPuis le relire.".into(),
            criteria: vec![
                Criterion {
                    text: "Le fichier existe".into(),
                    ok: true,
                    note: "vérifié".into(),
                },
                Criterion {
                    text: "Il est relu".into(),
                    ok: false,
                    note: String::new(),
                },
            ],
            iteration: 2,
            max_loops: 5,
            after: vec![first.id.clone()],
            progress: vec!["Fichier écrit".into()],
            external: Some(ExternalRef {
                service: Service::JIRA,
                id: "10042".into(),
                key: "ATL-12".into(),
                container: "ATL".into(),
                url: "https://example.atlassian.net/browse/ATL-12".into(),
                error: Some("401 Unauthorized".into()),
            }),
            ..ticket(&p, "DEM-2", "Écrire le fichier", Column::Doing, 2)
        },
    );
    let c = external(&h).await;
    let expected = json!({
        "id": t.id,
        "key": "DEM-2",
        "title": "Écrire le fichier",
        "description": "Écrire le fichier.\nPuis le relire.",
        "criteria": [
            { "text": "Le fichier existe", "ok": true, "note": "vérifié" },
            { "text": "Il est relu", "ok": false, "note": "" },
        ],
        "loops": { "iteration": 2, "max": 5 },
        "column": "doing",
        "after": ["DEM-1"],
        "progress": ["Fichier écrit"],
        "images": [],
        "files": [],
        "account": "auto",
        "model": "auto",
        "effort": "auto",
        "external": {
            "service": "jira",
            "key": "ATL-12",
            "url": "https://example.atlassian.net/browse/ATL-12",
            "error": "401 Unauthorized",
        },
        // Nothing of the flow yet: no agent, no branch, nothing blocked, running or done.
        "agent": null,
        "branch": null,
        "worktree": null,
        "blocked": null,
        "conflict": false,
        "step": null,
        "outcome": null,
    });
    assert_eq!(
        read(&c, "get_ticket", json!({ "ticket": "dem-2" })).await,
        expected
    );
    assert_eq!(
        read(&c, "get_ticket", json!({ "ticket": t.id })).await,
        expected
    );
    assert_eq!(
        last_entry(&h),
        (
            "Claude (hors Escouade)".into(),
            "get_ticket".into(),
            format!("ticket: {}", t.id),
            "ok"
        )
    );
    // Not imported: no external ticket. No account, model or effort of its own either: « auto » (ESC-153).
    let plain = read(&c, "get_ticket", json!({ "ticket": "DEM-1" })).await;
    assert_eq!(plain["external"], json!(null));
    assert_eq!(
        (&plain["account"], &plain["model"], &plain["effort"]),
        (&json!("auto"), &json!("auto"), &json!("auto"))
    );
    assert!(!plain.as_object().unwrap().contains_key("accountId"));
    // Launched with a model and an effort of its own (`start_ticket`): read back, its agent's alone.
    put(
        &h,
        Ticket {
            model: "opus".into(),
            effort: "max".into(),
            ..ticket(&p, "DEM-3", "Architecture", Column::Doing, 3)
        },
    );
    let own = read(&c, "get_ticket", json!({ "ticket": "dem-3" })).await;
    assert_eq!(
        (&own["model"], &own["effort"]),
        (&json!("opus"), &json!("max"))
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_tells_its_agent_branch_block_step_and_outcome_to_follow_the_flow() {
    let h = harness("mcp-get-ticket-flow");
    let p = project(&h).await;
    async fn get(c: &Client, key: &str) -> Value {
        read(c, "get_ticket", json!({ "ticket": key })).await
    }
    // To review on a merge conflict, its agent at rest in its worktree.
    let conflict = put(
        &h,
        Ticket {
            blocked: Some("Conflit de merge".into()),
            conflict: true,
            ..ticket(&p, "DEM-1", "Le conflit", Column::Review, 1)
        },
    );
    let rest = put_agent(&h, &p, "dem-1-le-conflit", 1);
    h.core.agent(&rest.id).unwrap().lock().meta.worktree = Some(Worktree {
        repos: Vec::new(),
        path: "/travail/dem-1".into(),
        branch: "ticket/dem-1".into(),
        base_branch: "main".into(),
        existing: false,
        named_branch: false,
    });
    link(&h, &rest.id, &conflict.id);
    // In progress, its agent asking for two permissions: how many wait is told.
    let working = put(&h, ticket(&p, "DEM-2", "En cours", Column::Doing, 2));
    let asking = live(&h, &p, "dem-2-en-cours").await;
    link(&h, &asking.id, &working.id);
    asks(&h, &asking.id, "r1", bash("ls"));
    asks(&h, &asking.id, "r2", bash("pwd"));
    // Its approval running, on an existing branch it takes up, without an agent.
    put(
        &h,
        Ticket {
            step: Some("Tests…".into()),
            branch: "feat/existante".into(),
            ..ticket(&p, "DEM-3", "La validation", Column::Review, 3)
        },
    );
    // Done, merged: what it made, and its agent gone.
    put(
        &h,
        Ticket {
            agent_id: Some("a-parti".into()),
            outcome: Some("⤵ Mergé dans main · squash".into()),
            outcome_url: Some("https://example.com/pull/7".into()),
            outcome_commit: Some("0123abc".into()),
            ..ticket(&p, "DEM-4", "Fini", Column::Done, 4)
        },
    );
    // Done with nothing to link to.
    put(
        &h,
        Ticket {
            outcome: Some("∅ Aucune modification".into()),
            ..ticket(&p, "DEM-5", "Déjà là", Column::Done, 5)
        },
    );
    let c = external(&h).await;
    let v = get(&c, "DEM-1").await;
    assert_eq!(
        v["agent"],
        json!({ "id": rest.id, "name": "dem-1-le-conflit", "status": "idle", "requests": 0 })
    );
    assert_eq!(
        (&v["branch"], &v["worktree"]),
        (&json!("ticket/dem-1"), &json!("/travail/dem-1"))
    );
    assert_eq!(
        (&v["blocked"], &v["conflict"], &v["step"], &v["outcome"]),
        (
            &json!("Conflit de merge"),
            &json!(true),
            &json!(null),
            &json!(null)
        )
    );
    let v = get(&c, "DEM-2").await;
    assert_eq!(v["agent"]["id"], json!(asking.id));
    assert_eq!(v["agent"]["requests"], json!(2));
    assert_eq!(
        (&v["blocked"], &v["conflict"]),
        (&json!(null), &json!(false))
    );
    // The same count as `list_agents` gives.
    let listed = read(&c, "list_agents", json!({})).await;
    let row = listed
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == json!(asking.id))
        .unwrap();
    assert_eq!(row["requests"], v["agent"]["requests"]);
    assert_eq!(row["status"], v["agent"]["status"]);
    let v = get(&c, "DEM-3").await;
    assert_eq!(
        (&v["agent"], &v["branch"], &v["worktree"], &v["step"]),
        (
            &json!(null),
            &json!("feat/existante"),
            &json!(null),
            &json!("Tests…")
        )
    );
    // An agent that is gone: its id is all that is left.
    let v = get(&c, "DEM-4").await;
    assert_eq!(
        v["agent"],
        json!({ "id": "a-parti", "name": null, "status": null, "requests": 0 })
    );
    assert_eq!(
        v["outcome"],
        json!({
            "text": "⤵ Mergé dans main · squash",
            "url": "https://example.com/pull/7",
            "commit": "0123abc",
        })
    );
    let v = get(&c, "DEM-5").await;
    assert_eq!(
        v["outcome"],
        json!({ "text": "∅ Aucune modification", "url": null, "commit": null })
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn without_a_ticket_named_an_agent_reads_its_own_and_claude_outside_is_refused() {
    let h = harness("mcp-own-ticket");
    let p = project(&h).await;
    let t = put(&h, ticket(&p, "DEM-1", "Le sien", Column::Doing, 1));
    let worker = put_agent(&h, &p, "dem-1-le-sien", 1);
    link(&h, &worker.id, &t.id);
    let idle = put_agent(&h, &p, "sans-ticket", 2);
    let agent = as_agent(&h, &worker.id).await;
    let own = read(&agent, "get_ticket", json!({})).await;
    assert_eq!((&own["id"], &own["key"]), (&json!(t.id), &json!("DEM-1")));
    assert_eq!(
        last_entry(&h),
        (
            "dem-1-le-sien".into(),
            "get_ticket".into(),
            String::new(),
            "ok"
        )
    );
    agent.cancel().await.unwrap();
    // An agent without a ticket: told so.
    let lost = as_agent(&h, &idle.id).await;
    let said = refused(&lost, "get_ticket", json!({})).await;
    assert_eq!(
        said,
        "L’agent sans-ticket n’a pas de ticket : donne la clé ou l’id de celui à lire."
    );
    assert_eq!(last_entry(&h).3, "error");
    lost.cancel().await.unwrap();
    // Claude outside Escouade has no ticket of its own.
    let c = external(&h).await;
    let said = refused(&c, "get_ticket", json!({})).await;
    assert_eq!(
        said,
        "Donne la clé ou l’id du ticket : seul un agent d’Escouade lit le sien sans le nommer."
    );
    assert_eq!(
        last_entry(&h),
        (
            "Claude (hors Escouade)".into(),
            "get_ticket".into(),
            String::new(),
            "refused"
        )
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_ambiguous_or_unknown_name_is_an_error_that_lists_the_choices() {
    let h = harness("mcp-resolution");
    let p = project(&h).await;
    let twin = other_project(&h, "demo").await;
    let c = external(&h).await;
    // Two projects named « demo »: its id is asked for.
    let said = refused(&c, "list_tickets", json!({ "project": "Demo" })).await;
    assert_eq!(
        said,
        format!(
            "Plusieurs projets s’appellent « Demo » : demo (id {}), demo (id {}). Donne son id.",
            p.id, twin.id
        )
    );
    assert_eq!(
        last_entry(&h),
        (
            "Claude (hors Escouade)".into(),
            "list_tickets".into(),
            "project: Demo".into(),
            "error"
        )
    );
    assert!(read(&c, "list_tickets", json!({ "project": twin.id }))
        .await
        .as_array()
        .unwrap()
        .is_empty());
    let said = refused(&c, "list_agents", json!({ "project": "nulle-part" })).await;
    assert_eq!(
        said,
        format!(
            "Aucun projet « nulle-part ». Projets : demo (id {}), demo (id {}).",
            p.id, twin.id
        )
    );
    // Two agents named « fix », one per project: its id or its project.
    let fix = put_agent(&h, &p, "fix", 1);
    let other_fix = put_agent(&h, &twin, "fix", 2);
    let said = refused(&c, "get_agent_summary", json!({ "agent": "FIX" })).await;
    assert_eq!(
        said,
        format!(
            "Plusieurs agents s’appellent « FIX » : fix (projet demo, id {}), fix (projet demo, id {}). Donne son id ou son projet.",
            fix.id, other_fix.id
        )
    );
    let one = read(
        &c,
        "get_agent_summary",
        json!({ "agent": "fix", "project": twin.id }),
    )
    .await;
    assert_eq!(one["id"], json!(other_fix.id));
    let by_id = read(&c, "get_agent_summary", json!({ "agent": fix.id })).await;
    assert_eq!(by_id["id"], json!(fix.id));
    // Unknown: five choices at most, those that look like it first.
    for (i, name) in ["alpha", "beta", "gamma", "delta", "epsilon", "fixture"]
        .iter()
        .enumerate()
    {
        put_agent(&h, &p, name, 10 + i as i64);
    }
    let said = refused(
        &c,
        "get_agent_summary",
        json!({ "agent": "fi", "project": p.id }),
    )
    .await;
    assert_eq!(
        said,
        format!(
            "Aucun agent « fi » dans le projet demo. Agents : fix (projet demo, id {}), fixture (projet demo, id {}), alpha (projet demo, id {}), beta (projet demo, id {}), gamma (projet demo, id {}), …",
            fix.id,
            format_args!("a-{}-fixture", p.id),
            format_args!("a-{}-alpha", p.id),
            format_args!("a-{}-beta", p.id),
            format_args!("a-{}-gamma", p.id),
        )
    );
    // The same key in two projects: its id.
    let mine = put(&h, ticket(&p, "DEM-1", "Le mien", Column::Todo, 1));
    let theirs = put(&h, ticket(&twin, "DEM-1", "Le leur", Column::Todo, 2));
    let said = refused(&c, "get_ticket", json!({ "ticket": "DEM-1" })).await;
    assert_eq!(
        said,
        format!(
            "Plusieurs tickets ont la clé « DEM-1 » : DEM-1 « Le mien » (projet demo, id {}), DEM-1 « Le leur » (projet demo, id {}). Donne son id.",
            mine.id, theirs.id
        )
    );
    let said = refused(&c, "get_ticket", json!({ "ticket": "DEM-9" })).await;
    assert!(
        said.starts_with("Aucun ticket « DEM-9 ». Tickets : DEM-1 « Le "),
        "{said}"
    );
    assert_eq!(
        read(&c, "get_ticket", json!({ "ticket": theirs.id })).await["title"],
        "Le leur"
    );
    // A project whose agents are all archived: told so, with them as the choices.
    let quiet = other_project(&h, "calme").await;
    let old = put_agent(&h, &quiet, "ancien", 1);
    h.core.agent(&old.id).unwrap().lock().meta.archived = true;
    let said = refused(
        &c,
        "get_agent_summary",
        json!({ "agent": "nouveau", "project": "calme" }),
    )
    .await;
    assert_eq!(
        said,
        format!(
            "Aucun agent « nouveau » dans le projet calme : ses agents sont tous archivés. Agents archivés : ancien (projet calme, id {}).",
            old.id
        )
    );
    // An archived one is found by its name all the same.
    let found = read(
        &c,
        "get_agent_summary",
        json!({ "agent": "ancien", "project": "calme" }),
    )
    .await;
    assert_eq!(found["id"], json!(old.id));
    // A project without any.
    let empty = other_project(&h, "vide").await;
    let said = refused(
        &c,
        "get_agent_summary",
        json!({ "agent": "personne", "project": empty.id }),
    )
    .await;
    assert_eq!(
        said,
        "Aucun agent « personne » dans le projet vide : il n’en a aucun."
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agents_summary_gives_its_last_message_cut_cleanly_and_the_files_it_edited() {
    let h = harness("mcp-agent-summary");
    let p = project(&h).await;
    let a = h
        .core
        .create_agent_with(
            &p.id,
            crate::core::AgentOptions {
                // Named: never renamed after its first message, as the test names it.
                name: Some("agent-1".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta;
    h.turn(&a.id, "edit").await;
    let c = external(&h).await;
    let summary = read(&c, "get_agent_summary", json!({ "agent": a.name })).await;
    assert_eq!(
        summary,
        json!({
            "id": a.id,
            "name": a.name,
            "status": "done",
            "engine": "claude",
            "ticket": null,
            "lastMessage": "Fichier modifié.",
            "touchedFiles": ["src/app.ts"],
            "requests": 0,
            "waitingFor": [],
            "subagentsRunning": 0,
            "backgroundTasksRunning": 0,
            "remoteControl": false,
            "remoteSource": null,
            "remoteState": null,
            "interfaceRemote": null,
            "workflow": null,
        })
    );
    assert_eq!(
        last_entry(&h),
        (
            "Claude (hors Escouade)".into(),
            "get_agent_summary".into(),
            format!("agent: {}", a.name),
            "ok"
        )
    );
    // A reply of 2,400 characters and more: its start and its end (what the agent concluded or
    // asks), each cut between two words, « … » between them, 2,000 characters at most.
    let long = format!("début {} fin, on fusionne ?", ["mot"; 600].join(" "));
    h.turn(&a.id, &long).await;
    let summary = read(&c, "get_agent_summary", json!({ "agent": a.id })).await;
    let last = summary["lastMessage"].as_str().unwrap();
    assert!(last.chars().count() <= 2000, "{}", last.chars().count());
    assert!(last.chars().count() > 1900, "{}", last.chars().count());
    assert!(last.starts_with("Bonjour, tu as dit : début mot"), "{last}");
    assert!(last.ends_with("mot mot fin, on fusionne ?"), "{last}");
    let (head, tail) = last.split_once(" … ").unwrap();
    assert!(head.ends_with(" mot") && tail.starts_with("mot "), "{last}");
    assert!(
        (400..=500).contains(&head.chars().count()),
        "{}",
        head.chars().count()
    );
    assert!(!tail.contains('…'), "{last}");
    // Never the whole conversation: neither the message sent nor anything else.
    let text = serde_json::to_string(&summary).unwrap();
    assert!(text.chars().count() < 2400, "{}", text.chars().count());
    assert_eq!(
        summary.as_object().unwrap().keys().collect::<Vec<_>>(),
        [
            "backgroundTasksRunning",
            "engine",
            "id",
            "interfaceRemote",
            "lastMessage",
            "name",
            "remoteControl",
            "remoteSource",
            "remoteState",
            "requests",
            "status",
            "subagentsRunning",
            "ticket",
            "touchedFiles",
            "waitingFor",
            "workflow"
        ]
    );
    // The 100 latest files it edited.
    h.core.agent(&a.id).unwrap().lock().meta.touched_files =
        (0..150).map(|i| format!("src/f{i}.ts")).collect();
    let summary = read(&c, "get_agent_summary", json!({ "agent": a.id })).await;
    let files = summary["touchedFiles"].as_array().unwrap();
    assert_eq!(files.len(), 100);
    assert_eq!(
        (&files[0], &files[99]),
        (&json!("src/f50.ts"), &json!("src/f149.ts"))
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_usage_gives_each_account_its_windows_and_the_autopilots_pause() {
    let h = harness("mcp-usage");
    {
        let mut s = h.core.settings.write();
        s.accounts.push(Account {
            id: "pro".into(),
            name: "Pro".into(),
            config_dir: h.dir.join("pro").to_string_lossy().to_string(),
            ..Default::default()
        });
    }
    // Each account's windows, as read from its own Claude Code.
    let soon = now_ms() + 3_600_000;
    let window = |pct: f64, at: i64| {
        Some(RateWindow {
            pct,
            resets_at: Some(at),
        })
    };
    h.core.record_usage(
        crate::accounts::PRINCIPAL,
        Reading::claude((window(42.0, soon), window(10.5, soon + 1))),
    );
    h.core
        .record_usage("pro", Reading::claude((window(7.0, soon + 2), None)));
    let c = external(&h).await;
    assert_eq!(
        read(&c, "get_usage", json!({})).await,
        json!({
            "accounts": [
                {
                    "name": "Principal",
                    "engine": "claude",
                    "current": true,
                    "meters": [
                        { "minutes": 300, "pct": 42.0, "resetsAt": soon },
                        { "minutes": 10080, "pct": 10.5, "resetsAt": soon + 1 },
                    ],
                    "fiveHour": { "pct": 42.0, "resetsAt": soon },
                    "sevenDay": { "pct": 10.5, "resetsAt": soon + 1 },
                },
                {
                    "name": "Pro",
                    "engine": "claude",
                    "current": false,
                    "meters": [{ "minutes": 300, "pct": 7.0, "resetsAt": soon + 2 }],
                    "fiveHour": { "pct": 7.0, "resetsAt": soon + 2 },
                    "sevenDay": null,
                },
            ],
            "autopilotPause": null,
        })
    );
    assert_eq!(
        last_entry(&h),
        (
            "Claude (hors Escouade)".into(),
            "get_usage".into(),
            String::new(),
            "ok"
        )
    );
    let currents = |usage: &Value| {
        (
            usage["accounts"][0]["current"].clone(),
            usage["accounts"][1]["current"].clone(),
        )
    };
    // Principal's 5-hour window used up: new agents go to Pro, under the threshold, and the
    // autopilot goes on with it.
    h.core.record_usage(
        crate::accounts::PRINCIPAL,
        Reading::claude((window(100.0, soon), None)),
    );
    let usage = read(&c, "get_usage", json!({})).await;
    assert_eq!(currents(&usage), (json!(false), json!(true)));
    assert_eq!(usage["accounts"][0]["fiveHour"]["pct"], 100.0);
    assert_eq!(usage["accounts"][0]["sevenDay"]["pct"], 10.5);
    assert_eq!(usage["autopilotPause"], Value::Null);
    // Pro's too, and sooner: no account is left under the threshold, the first active one is
    // the current again, and the autopilot waits for the nearest reset, Pro's.
    h.core.record_usage(
        "pro",
        Reading::claude((window(100.0, soon - 600_000), None)),
    );
    let usage = read(&c, "get_usage", json!({})).await;
    assert_eq!(currents(&usage), (json!(true), json!(false)));
    assert_eq!(
        usage["autopilotPause"],
        json!({ "reason": "window", "minutes": 300, "until": soon - 600_000 + RESUME_MARGIN_MS })
    );
    // Pro switched off: only Principal, whose window holds the autopilot back until its end.
    // (Not saved: that would take the server down, which no project asks for.)
    h.core.settings.write().accounts[1].active = false;
    h.core.update_usage(|_| {});
    let until = h.core.autopilot_pause().unwrap().until;
    assert_eq!(until, soon + RESUME_MARGIN_MS);
    let usage = read(&c, "get_usage", json!({})).await;
    assert_eq!(currents(&usage), (json!(true), json!(false)));
    assert_eq!(
        usage["autopilotPause"],
        json!({ "reason": "window", "minutes": 300, "until": until })
    );
    assert_eq!(usage["accounts"][1]["fiveHour"]["pct"], 100.0);
    // Pro back on and Principal switched off: only Pro is left to go to.
    h.core.settings.write().accounts[1].active = true;
    h.core.settings.write().accounts[0].active = false;
    h.core.update_usage(|_| {});
    let usage = read(&c, "get_usage", json!({})).await;
    assert_eq!(currents(&usage), (json!(false), json!(true)));
    // Pro on a Max plan without a 5-hour window (ESC-312): its meters in the server's order, Fable's
    // week with its scope, and no 5-hour window left of the reading before.
    let body = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/claude-usage/max-no-five-hour.json"),
    )
    .unwrap();
    h.core.record_usage(
        "pro",
        crate::usage::reading_of(&serde_json::from_str(&body).unwrap()),
    );
    let usage = read(&c, "get_usage", json!({})).await;
    let pro = &usage["accounts"][1];
    assert_eq!(
        pro["meters"],
        json!([
            { "minutes": 10080, "pct": 47.0, "resetsAt": 1_793_829_600_147i64 },
            { "minutes": 10080, "pct": 0.0, "resetsAt": 1_793_829_600_000i64, "model": "Fable" },
        ])
    );
    assert_eq!(pro["fiveHour"], Value::Null);
    assert_eq!(pro["sevenDay"]["pct"], 47.0);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

// ---------- wait_ticket ----------

/// How far into a wait the change it waits for is made: the wait has most likely begun.
const CHANGE_AFTER: Duration = Duration::from_millis(300);

/// How far into a wait a ticket is deleted: the wait names it, so it must have found it first
/// (a ticket deleted before is an unknown one to the call).
const DELETE_AFTER: Duration = Duration::from_secs(1);

/// The timeout of a wait that must be answered by a change: far beyond the time the answer takes,
/// even on a loaded machine.
const LONG: u64 = 20;

/// What `wait_ticket` answers to `args`, and how long it took.
async fn waited(c: &Client, args: Value) -> (Value, Duration) {
    let began = Instant::now();
    let answer = read(c, "wait_ticket", args).await;
    (answer, began.elapsed())
}

/// The `since` of `tickets` (keys) as they are now, from a first short wait. A wait that begins
/// with it compares the tickets with it, whether it has looked at them yet when the change is made
/// or not: what it answers does not depend on timing.
async fn baseline(c: &Client, tickets: Value) -> Value {
    let (v, _) = waited(c, json!({ "tickets": tickets, "timeout": 1 })).await;
    v["since"].clone()
}

/// `waited`, with `change` made `after` into it (`args` hold the `since` to compare with).
async fn waited_after(
    c: &Client,
    args: Value,
    after: Duration,
    change: impl Future<Output = ()>,
) -> (Value, Duration) {
    let change = async {
        tokio::time::sleep(after).await;
        change.await;
    };
    tokio::join!(waited(c, args), change).0
}

/// `waited_after` `CHANGE_AFTER`.
async fn waited_during(
    c: &Client,
    args: Value,
    change: impl Future<Output = ()>,
) -> (Value, Duration) {
    waited_after(c, args, CHANGE_AFTER, change).await
}

/// A change of the ticket, as the core makes one.
fn edit(h: &Harness, id: &str, f: impl FnOnce(&mut Ticket)) {
    h.core
        .edit_ticket(id, |t| {
            f(t);
            Ok(())
        })
        .unwrap();
}

/// Answered by the change, well before the `LONG` seconds the wait was given.
fn answered_by_the_change(took: Duration) {
    assert!(took < Duration::from_secs(10), "{took:?}");
}

/// What `before` says of a ticket as it was: in `column`, nothing else going on.
fn calm(column: &str) -> Value {
    json!({
        "column": column,
        "blocked": false,
        "conflict": false,
        "waiting": false,
        "resting": false,
    })
}

/// What `tickets` says of a ticket that is in `column`, nothing else going on.
fn still_in(key: &str, id: &str, column: &str) -> Value {
    json!({
        "key": key,
        "id": id,
        "column": column,
        "blocked": null,
        "conflict": false,
        "step": null,
        "waiting": false,
        "resting": false,
        "gone": false,
    })
}

#[tokio::test]
async fn a_change_during_the_wait_is_answered_at_once_with_what_changed_and_how_it_was() {
    let h = harness("mcp-wait-change");
    let p = project(&h).await;
    let one = put(&h, ticket(&p, "DEM-1", "Un", Column::Doing, 1));
    let two = put(&h, ticket(&p, "DEM-2", "Deux", Column::Doing, 2));
    let c = external(&h).await;
    let since = baseline(&c, json!(["dem-1", "DEM-2"])).await;
    let args = json!({ "tickets": ["dem-1", "DEM-2"], "timeout": LONG, "since": since });
    // The first one is blocked.
    let (v, took) = waited_during(&c, args, async {
        edit(&h, &one.id, |t| t.blocked = Some("Tests en échec".into()));
    })
    .await;
    answered_by_the_change(took);
    assert_eq!(v["changed"], json!(true));
    assert_eq!(
        v["changes"],
        json!([{ "key": "DEM-1", "what": ["blocked"], "before": calm("doing") }])
    );
    // Every ticket asked is given as it is now, in the order asked.
    let mut blocked = still_in("DEM-1", &one.id, "doing");
    blocked["blocked"] = json!("Tests en échec");
    assert_eq!(
        v["tickets"],
        json!([blocked, still_in("DEM-2", &two.id, "doing")])
    );
    assert!(v["waited"].is_u64(), "{v}");
    assert_eq!(
        last_entry(&h),
        (
            "Claude (hors Escouade)".into(),
            "wait_ticket".into(),
            "tickets: dem-1, DEM-2 · timeout: 20".into(),
            "ok"
        )
    );
    // The second one goes to review: its column.
    let args = json!({ "tickets": ["dem-1", "DEM-2"], "timeout": LONG, "since": v["since"] });
    let (v, took) = waited_during(&c, args, async {
        edit(&h, &two.id, |t| t.column = Column::Review);
    })
    .await;
    answered_by_the_change(took);
    assert_eq!(
        v["changes"],
        json!([{ "key": "DEM-2", "what": ["column"], "before": calm("doing") }])
    );
    // A merge conflict stops the first one: its column, its block and the conflict.
    let args = json!({ "tickets": ["dem-1", "DEM-2"], "timeout": LONG, "since": v["since"] });
    let (v, took) = waited_during(&c, args, async {
        edit(&h, &one.id, |t| {
            t.column = Column::Review;
            t.blocked = Some("Conflit de merge".into());
            t.conflict = true;
        });
    })
    .await;
    answered_by_the_change(took);
    assert_eq!(
        v["changes"],
        json!([{
            "key": "DEM-1",
            "what": ["column", "blocked", "conflict"],
            "before": {
                "column": "doing", "blocked": true, "conflict": false,
                "waiting": false, "resting": false,
            },
        }])
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn without_a_change_it_answers_after_its_timeout_that_nothing_changed_and_gives_since() {
    let h = harness("mcp-wait-timeout");
    let p = project(&h).await;
    let one = put(&h, ticket(&p, "DEM-1", "Un", Column::Doing, 1));
    let two = put(&h, ticket(&p, "DEM-2", "Deux", Column::Todo, 2));
    let c = external(&h).await;
    // A ticket named twice is watched once, and a blank since is none.
    let (v, took) = waited(
        &c,
        json!({ "tickets": ["DEM-1", "dem-1", two.id], "timeout": 1, "since": " " }),
    )
    .await;
    assert!(
        took >= Duration::from_secs(1) && took < Duration::from_secs(10),
        "{took:?}"
    );
    assert_eq!((&v["changed"], &v["changes"]), (&json!(false), &json!([])));
    assert_eq!(v["waited"], json!(1));
    assert_eq!(
        v["tickets"],
        json!([
            still_in("DEM-1", &one.id, "doing"),
            still_in("DEM-2", &two.id, "todo"),
        ])
    );
    // The cursor: base64url without padding, a list of what is watched of each ticket.
    let since = v["since"].as_str().unwrap();
    assert!(
        since
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "{since}"
    );
    let cursor: Value = serde_json::from_slice(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(since)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        cursor,
        json!([
            {
                "id": one.id, "column": "doing", "blocked": null,
                "conflict": false, "waiting": false, "resting": false,
            },
            {
                "id": two.id, "column": "todo", "blocked": null,
                "conflict": false, "waiting": false, "resting": false,
            },
        ])
    );
    assert_eq!(
        last_entry(&h),
        (
            "Claude (hors Escouade)".into(),
            "wait_ticket".into(),
            format!("tickets: DEM-1, dem-1, {} · timeout: 1", two.id),
            "ok"
        )
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_change_made_between_two_calls_is_answered_at_once_by_the_since_of_the_first() {
    let h = harness("mcp-wait-since");
    let p = project(&h).await;
    let t = put(&h, ticket(&p, "DEM-1", "Un", Column::Doing, 1));
    let c = external(&h).await;
    // A first answer, for its cursor.
    let since = baseline(&c, json!(["DEM-1"])).await;
    let (first, _) = waited_during(
        &c,
        json!({ "tickets": ["DEM-1"], "timeout": LONG, "since": since }),
        async {
            edit(&h, &t.id, |t| t.blocked = Some("Bloqué".into()));
        },
    )
    .await;
    assert_eq!(first["changes"][0]["what"], json!(["blocked"]));
    // It goes to review, its block gone, before the next call: which it would miss without it.
    edit(&h, &t.id, |t| {
        t.column = Column::Review;
        t.blocked = None;
    });
    let (second, took) = waited(
        &c,
        json!({ "tickets": ["DEM-1"], "timeout": LONG, "since": first["since"] }),
    )
    .await;
    answered_by_the_change(took);
    assert_eq!(
        second["changes"],
        json!([{
            "key": "DEM-1",
            "what": ["column", "blocked"],
            "before": {
                "column": "doing", "blocked": true, "conflict": false,
                "waiting": false, "resting": false,
            },
        }])
    );
    // What it said is not told again, and a ticket its cursor does not hold is as it is at the
    // call: nothing changed.
    put(&h, ticket(&p, "DEM-2", "Deux", Column::Doing, 2));
    let (third, took) = waited(
        &c,
        json!({ "tickets": ["DEM-1", "DEM-2"], "timeout": 1, "since": second["since"] }),
    )
    .await;
    assert!(took >= Duration::from_secs(1), "{took:?}");
    assert_eq!(third["changed"], json!(false));
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agent_that_comes_to_rest_in_to_review_makes_its_ticket_resting() {
    let h = harness("mcp-wait-resting");
    let p = project(&h).await;
    let review = put(&h, ticket(&p, "DEM-1", "À relire", Column::Review, 1));
    let worker = put_agent(&h, &p, "dem-1-a-relire", 1);
    link(&h, &worker.id, &review.id);
    let status = |s: AgentStatus| h.core.agent(&worker.id).unwrap().lock().meta.status = s;
    status(AgentStatus::Running);
    // Idle, but its ticket is in progress: it is not ready to approve.
    let doing = put(&h, ticket(&p, "DEM-2", "En cours", Column::Doing, 2));
    let idle = put_agent(&h, &p, "dem-2-en-cours", 2);
    link(&h, &idle.id, &doing.id);
    let c = external(&h).await;
    let since = baseline(&c, json!(["DEM-1", "DEM-2"])).await;
    let (v, took) = waited_during(
        &c,
        json!({ "tickets": ["DEM-1", "DEM-2"], "timeout": LONG, "since": since }),
        async { status(AgentStatus::Idle) },
    )
    .await;
    answered_by_the_change(took);
    assert_eq!(
        v["changes"],
        json!([{ "key": "DEM-1", "what": ["resting"], "before": calm("review") }])
    );
    let resting: Vec<bool> = v["tickets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["resting"].as_bool().unwrap())
        .collect();
    assert_eq!(resting, [true, false]);
    // At work again (sent back, a conflict handed over): no longer resting.
    let (v, took) = waited_during(
        &c,
        json!({ "tickets": ["DEM-1"], "timeout": LONG, "since": v["since"] }),
        async { status(AgentStatus::Running) },
    )
    .await;
    answered_by_the_change(took);
    assert_eq!(v["changes"][0]["what"], json!(["resting"]));
    assert_eq!(v["tickets"][0]["resting"], json!(false));
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_request_that_waits_for_an_answer_makes_its_ticket_waiting_until_it_is_answered() {
    let h = harness("mcp-wait-waiting");
    let p = project(&h).await;
    let t = put(&h, ticket(&p, "DEM-1", "Demande", Column::Doing, 1));
    let agent = live(&h, &p, "dem-1-demande").await;
    link(&h, &agent.id, &t.id);
    let c = external(&h).await;
    let since = baseline(&c, json!(["DEM-1"])).await;
    let (v, took) = waited_during(
        &c,
        json!({ "tickets": ["DEM-1"], "timeout": LONG, "since": since }),
        async { asks(&h, &agent.id, "r1", bash("ls")) },
    )
    .await;
    answered_by_the_change(took);
    assert_eq!(
        v["changes"],
        json!([{ "key": "DEM-1", "what": ["waiting"], "before": calm("doing") }])
    );
    assert_eq!(v["tickets"][0]["waiting"], json!(true));
    // The user answers it in the window: it no longer waits.
    let (v, took) = waited_during(
        &c,
        json!({ "tickets": ["DEM-1"], "timeout": LONG, "since": v["since"] }),
        async {
            h.core
                .answer_permission(&agent.id, "r1", "allow", None)
                .unwrap()
        },
    )
    .await;
    answered_by_the_change(took);
    assert_eq!(v["changes"][0]["what"], json!(["waiting"]));
    assert_eq!(v["tickets"][0]["waiting"], json!(false));
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_deleted_during_the_wait_is_gone_and_left_out_of_since() {
    let h = harness("mcp-wait-gone");
    let p = project(&h).await;
    let gone = put(&h, ticket(&p, "DEM-1", "Un", Column::Todo, 1));
    let kept = put(&h, ticket(&p, "DEM-2", "Deux", Column::Todo, 2));
    let c = external(&h).await;
    let since = baseline(&c, json!(["DEM-1", "DEM-2"])).await;
    let (v, took) = waited_after(
        &c,
        json!({ "tickets": ["DEM-1", "DEM-2"], "timeout": LONG, "since": since }),
        DELETE_AFTER,
        async { h.core.ticket_delete(&gone.id).await.unwrap() },
    )
    .await;
    answered_by_the_change(took);
    assert_eq!(
        v["changes"],
        json!([{ "key": "DEM-1", "what": ["gone"], "before": calm("todo") }])
    );
    // It is still named, as it was last seen.
    let mut last = still_in("DEM-1", &gone.id, "todo");
    last["gone"] = json!(true);
    assert_eq!(
        v["tickets"],
        json!([last, still_in("DEM-2", &kept.id, "todo")])
    );
    let cursor: Value = serde_json::from_slice(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(v["since"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(cursor.as_array().unwrap().len(), 1);
    assert_eq!(cursor[0]["id"], json!(kept.id));
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_wait_stops_when_its_client_drops_the_connection_and_the_log_says_it_was_cancelled() {
    use tokio::io::AsyncWriteExt;
    let h = harness("mcp-wait-cancelled");
    let p = project(&h).await;
    put(&h, ticket(&p, "DEM-1", "Un", Column::Doing, 1));
    let (port, token) = started(&h);
    // A raw request, to be able to leave: a long wait on a ticket that does not change.
    let body = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": { "name": "wait_ticket", "arguments": { "tickets": ["DEM-1"], "timeout": 60 } },
    })
    .to_string();
    let request = format!(
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nMCP-Protocol-Version: 2025-11-25\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let mut stream = tokio::net::TcpStream::connect((Ipv4Addr::LOCALHOST, port))
        .await
        .unwrap();
    stream.write_all(request.as_bytes()).await.unwrap();
    tokio::time::sleep(CHANGE_AFTER).await;
    assert!(
        h.core.mcp.activity.entries().is_empty(),
        "answered before the client left"
    );
    // The client gives up: the wait stops and is logged, long before its minute is over.
    drop(stream);
    let began = Instant::now();
    let entry = loop {
        if let Some(e) = h.core.mcp.activity.entries().pop() {
            break e;
        }
        assert!(
            began.elapsed() < Duration::from_secs(10),
            "the wait goes on though its client left"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    assert_eq!(
        (entry.tool.as_str(), entry.outcome, entry.message.as_deref()),
        (
            "wait_ticket",
            "error",
            Some("Attente annulée par le client.")
        )
    );
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_wait_asked_for_nothing_too_much_an_unknown_ticket_or_a_since_that_is_none_fails() {
    let h = harness("mcp-wait-failures");
    let p = project(&h).await;
    put(&h, ticket(&p, "DEM-1", "Un", Column::Doing, 1));
    let c = external(&h).await;
    let failed = |said: String| {
        let e = h.core.mcp.activity.entries().pop().unwrap();
        assert_eq!((e.tool.as_str(), e.outcome), ("wait_ticket", "error"));
        assert_eq!(e.message.as_deref(), Some(said.as_str()));
        said
    };
    let said = refused(&c, "wait_ticket", json!({ "tickets": [] })).await;
    assert_eq!(
        failed(said),
        "Donne au moins un ticket à attendre : sa clé ou son id."
    );
    // Twenty at most.
    let many: Vec<String> = (0..21).map(|_| "DEM-1".into()).collect();
    let said = refused(&c, "wait_ticket", json!({ "tickets": many })).await;
    assert_eq!(failed(said), "21 tickets : 20 au plus.");
    let said = refused(&c, "wait_ticket", json!({ "tickets": ["DEM-1", "DEM-9"] })).await;
    assert!(
        said.starts_with("Aucun ticket « DEM-9 ». Tickets : DEM-1 « Un »"),
        "{said}"
    );
    failed(said);
    let said = refused(&c, "wait_ticket", json!({ "tickets": [" "] })).await;
    assert_eq!(failed(said), "Donne un ticket : sa clé ou son id.");
    // A cursor is the one an answer gave, as it was given: not any text, nor any JSON.
    let not_json = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("pas du json");
    let not_marks = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("{}");
    for since in ["!?", &not_json, &not_marks] {
        let said = refused(
            &c,
            "wait_ticket",
            json!({ "tickets": ["DEM-1"], "since": since }),
        )
        .await;
        assert_eq!(
            failed(said),
            "since n’est pas un curseur rendu par wait_ticket : redonne-le tel qu’il t’a été donné, ou laisse-le de côté."
        );
    }
    // A timeout that is no number is the router's refusal, which does not repeat what was given.
    let said = refused(
        &c,
        "wait_ticket",
        json!({ "tickets": ["DEM-1"], "timeout": "long" }),
    )
    .await;
    assert!(
        said.contains("invalid type") && said.contains("schéma") && !said.contains("long"),
        "{said}"
    );
    let e = h.core.mcp.activity.entries().pop().unwrap();
    assert_eq!((e.tool.as_str(), e.outcome), ("wait_ticket", "error"));
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_boards_settings_are_read_whole_with_its_accounts_defaults_and_the_branch_it_merges_into()
{
    let h = harness("mcp-get-board");
    let (p, r) = h.project(false).await;
    let p = still(&h, &p);
    let c = external(&h).await;
    // A Kanban that has no target yet merges into the branch the project is on, and says so.
    git(&r, &["checkout", "-qb", "develop"]);
    let expected = json!({
        "project": "demo",
        "projectId": p.id,
        "action": "merge",
        "target": "develop",
        "strategy": "squash",
        "draft": false,
        "cleanup": true,
        "conventional": true,
        "conflict": "ask",
        "maxParallel": 2,
        "autopilot": false,
        "model": "",
        "effort": "",
        "mode": "",
        "defaults": { "model": "sonnet", "effort": "medium", "mode": "auto" },
        "testsFirst": false,
        "testCommand": "",
        "prefix": "",
    });
    assert_eq!(
        read(&c, "get_board", json!({ "project": "DEMO" })).await,
        expected
    );
    // It reads, it does not fix the target: that is the first ticket's.
    assert_eq!(h.core.project(&p.id).unwrap().board.target, "");
    assert_eq!(
        last_entry(&h),
        (
            "Claude (hors Escouade)".into(),
            "get_board".into(),
            "project: DEMO".into(),
            "ok"
        )
    );

    // Every setting as the Kanban has it, the defaults as the account of its new agents has them
    // (ESC-128).
    {
        let mut s = h.core.settings.write();
        let a = &mut s.accounts[0];
        a.default_model = "opus".into();
        a.default_effort = "high".into();
        a.default_mode = "plan".into();
    }
    h.core
        .board_set(
            &p.id,
            BoardSettings {
                action: "pr".into(),
                target: "main".into(),
                strategy: "rebase".into(),
                draft: true,
                cleanup: false,
                conventional: false,
                conflict: "agent".into(),
                max_parallel: 4,
                autopilot: false,
                model: "haiku".into(),
                effort: "low".into(),
                mode: "acceptEdits".into(),
                tests_first: true,
                test_command: "npm test".into(),
                ..Default::default()
            },
        )
        .unwrap();
    let board = read(&c, "get_board", json!({ "project": p.id })).await;
    assert_eq!(
        board,
        json!({
            "project": "demo",
            "projectId": p.id,
            "action": "pr",
            // The one the Kanban has, not the branch the project is on.
            "target": "main",
            "strategy": "rebase",
            "draft": true,
            "cleanup": false,
            "conventional": false,
            "conflict": "agent",
            "maxParallel": 4,
            "autopilot": false,
            "model": "haiku",
            "effort": "low",
            "mode": "acceptEdits",
            "defaults": { "model": "opus", "effort": "high", "mode": "plan" },
            "testsFirst": true,
            "testCommand": "npm test",
            "prefix": "",
        })
    );

    // Tests first with no test command to run is off, as the window shows it and the validation
    // takes it: so does it say.
    let stored = h.core.project(&p.id).unwrap().board;
    h.core
        .board_set(
            &p.id,
            BoardSettings {
                tests_first: true,
                test_command: "  ".into(),
                ..stored.clone()
            },
        )
        .unwrap();
    let off = read(&c, "get_board", json!({ "project": "demo" })).await;
    assert_eq!(
        (&off["testsFirst"], &off["testCommand"]),
        (&json!(false), &json!("  "))
    );
    assert!(h.core.project(&p.id).unwrap().board.tests_first);
    h.core.board_set(&p.id, stored).unwrap();

    // The tickets' key prefix is the first ticket's.
    let made = h
        .core
        .ticket_create(
            &p.id,
            crate::tickets::TicketDraft {
                title: "Un".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(made.key, "DEM-1");
    assert_eq!(
        read(&c, "get_board", json!({ "project": "demo" })).await["prefix"],
        "DEM"
    );

    // An agent of another project reads it too: the tools that read are open.
    let other = other_project(&h, "autre").await;
    let theirs = put_agent(&h, &other, "leur-agent", 1);
    let agent = as_agent(&h, &theirs.id).await;
    assert_eq!(
        read(&agent, "get_board", json!({ "project": "demo" })).await["action"],
        "pr"
    );
    assert_eq!(last_entry(&h).0, "leur-agent");
    agent.cancel().await.unwrap();

    // A project that is none: an error that says so.
    let said = refused(&c, "get_board", json!({ "project": "nope" })).await;
    assert!(said.contains("Aucun projet « nope »"), "{said}");
    assert_eq!(last_entry(&h).3, "error");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}
