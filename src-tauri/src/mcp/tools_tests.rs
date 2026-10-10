//! The tools that read, called by a real MCP client (rmcp's) on a test core: real git
//! repositories, the fake `claude`, tickets put on the boards as they are (their autopilot off).

use super::tests::{client, names, started, EXPOSED};
use crate::agent::AgentRt;
use crate::core_tests::{harness, Harness};
use crate::model::*;
use rmcp::model::CallToolRequestParams;
use rmcp::service::RunningService;
use rmcp::RoleClient;
use serde_json::{json, Value};
use std::sync::Arc;

type Client = RunningService<RoleClient, ()>;

/// The project of the harness (its repository on `main`), its autopilot off: a ticket put on its
/// board stays where it is put.
async fn project(h: &Harness) -> Project {
    let (p, _) = h.project(false).await;
    still(h, &p)
}

/// Another project, named `name`, in a folder of its own (made a repository).
async fn other_project(h: &Harness, name: &str) -> Project {
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
        )
        .await
        .unwrap();
    still(h, &p)
}

fn still(h: &Harness, p: &Project) -> Project {
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
fn ticket(p: &Project, key: &str, title: &str, column: Column, at: i64) -> Ticket {
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
fn put(h: &Harness, t: Ticket) -> Ticket {
    h.core.tickets.write().push(t.clone());
    t
}

/// An agent of `p` named `name`, made as the app keeps one (no process).
fn put_agent(h: &Harness, p: &Project, name: &str, at: i64) -> AgentMeta {
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
fn link(h: &Harness, agent_id: &str, ticket_id: &str) {
    h.core.agent(agent_id).unwrap().lock().meta.ticket_id = Some(ticket_id.into());
    for t in h.core.tickets.write().iter_mut() {
        if t.id == ticket_id {
            t.agent_id = Some(agent_id.into());
        }
    }
}

/// Claude outside Escouade, connected to the harness's server.
async fn external(h: &Harness) -> Client {
    let (port, token) = started(h);
    client(port, &token).await.unwrap()
}

/// The agent `agent_id`, connected with a token of its own.
async fn as_agent(h: &Harness, agent_id: &str) -> Client {
    let (port, _) = started(h);
    client(port, &h.core.mcp.register_agent(agent_id))
        .await
        .unwrap()
}

/// What `tool` answers to `args`: its text, and whether it is an error the model reads.
async fn call(c: &Client, tool: &'static str, args: Value) -> (String, bool) {
    let mut params = CallToolRequestParams::new(tool);
    if let Value::Object(args) = args {
        params = params.with_arguments(args);
    }
    let answer = serde_json::to_value(c.call_tool(params).await.unwrap()).unwrap();
    let text = answer["content"][0]["text"].as_str().unwrap().to_string();
    (text, answer["isError"] == true)
}

/// What `tool` answers to `args`, a JSON that is no error.
async fn read(c: &Client, tool: &'static str, args: Value) -> Value {
    let (text, error) = call(c, tool, args).await;
    assert!(!error, "{tool}: {text}");
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{tool}: {e}: {text}"))
}

/// The error `tool` answers to `args`.
async fn refused(c: &Client, tool: &'static str, args: Value) -> String {
    let (text, error) = call(c, tool, args).await;
    assert!(error, "{tool} answered: {text}");
    text
}

/// The activity log's last entry: (caller, tool, summary, outcome).
fn last_entry(h: &Harness) -> (String, String, String, &'static str) {
    let e = h.core.mcp.activity.entries().pop().unwrap();
    (e.caller, e.tool, e.summary, e.outcome)
}

fn keys(rows: &Value) -> Vec<String> {
    rows.as_array()
        .unwrap()
        .iter()
        .map(|r| r["key"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn the_reading_tools_say_they_only_read_and_what_they_take() {
    let h = harness("mcp-read-tools");
    let c = external(&h).await;
    let tools = c.list_all_tools().await.unwrap();
    let mut listed = names(&tools);
    listed.retain(|n| n != "whoami");
    assert_eq!(listed, EXPOSED);
    for t in tools.iter().filter(|t| t.name != "whoami") {
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
    for name in ["list_projects", "list_agents", "get_ticket", "get_usage"] {
        assert!(
            required(name).as_array().is_none_or(|r| r.is_empty()),
            "{name}"
        );
    }
    // Every argument says what it takes, in English, and a column is one of the four.
    for name in [
        "list_agents",
        "list_tickets",
        "get_ticket",
        "get_agent_summary",
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
            "status": "idle",
            "waiting": false,
            "ticket": "DEM-1",
            "model": "sonnet",
            "account": "Principal",
            "cost": 0.0,
        })
    );
    assert_eq!(
        (&all[1]["status"], &all[1]["waiting"], &all[1]["ticket"]),
        (&json!("waiting"), &json!(true), &json!(null))
    );
    assert!(all[1]["cost"].is_number(), "{}", all[1]);
    assert_eq!(all[2]["project"], "autre");
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
    // A column that is none of the four: an error the model reads, in the log too.
    let said = refused(
        &c,
        "list_tickets",
        json!({ "project": "demo", "column": "later" }),
    )
    .await;
    assert!(said.contains("later"), "{said}");
    let e = h.core.mcp.activity.entries().pop().unwrap();
    assert_eq!((e.tool.as_str(), e.outcome), ("list_tickets", "error"));
    assert!(
        e.message
            .as_deref()
            .is_some_and(|m| m.starts_with("Appel refusé par le serveur : ") && m.contains("later")),
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
                service: Service::Jira,
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
        "external": {
            "service": "jira",
            "key": "ATL-12",
            "url": "https://example.atlassian.net/browse/ATL-12",
            "error": "401 Unauthorized",
        },
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
    // Not imported: no external ticket.
    let plain = read(&c, "get_ticket", json!({ "ticket": "DEM-1" })).await;
    assert_eq!(plain["external"], json!(null));
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
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agents_summary_gives_its_last_message_cut_cleanly_and_the_files_it_edited() {
    let h = harness("mcp-agent-summary");
    let p = project(&h).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    h.turn(&a.id, "edit").await;
    let c = external(&h).await;
    let summary = read(&c, "get_agent_summary", json!({ "agent": a.name })).await;
    assert_eq!(
        summary,
        json!({
            "id": a.id,
            "name": a.name,
            "status": "done",
            "ticket": null,
            "lastMessage": "Fichier modifié.",
            "touchedFiles": ["src/app.ts"],
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
    // A reply of 2,400 characters and more: cut between two words, 2,000 characters at most.
    let long = ["mot"; 600].join(" ");
    h.turn(&a.id, &long).await;
    let summary = read(&c, "get_agent_summary", json!({ "agent": a.id })).await;
    let last = summary["lastMessage"].as_str().unwrap();
    assert!(last.chars().count() <= 2000, "{}", last.chars().count());
    assert!(last.chars().count() > 1900, "{}", last.chars().count());
    assert!(last.starts_with("Bonjour, tu as dit : mot mot"), "{last}");
    assert!(last.ends_with("mot…"), "{last}");
    // Never the whole conversation: neither the message sent nor anything else.
    let text = serde_json::to_string(&summary).unwrap();
    assert!(text.chars().count() < 2300, "{}", text.chars().count());
    assert_eq!(
        summary.as_object().unwrap().keys().collect::<Vec<_>>(),
        [
            "id",
            "lastMessage",
            "name",
            "status",
            "ticket",
            "touchedFiles"
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
    let soon = now_ms() + 3_600_000;
    {
        let mut u = h.core.usage.lock();
        u.five_hour = Some(RateWindow {
            pct: 42.0,
            resets_at: Some(soon),
        });
        u.seven_day = Some(RateWindow {
            pct: 10.5,
            resets_at: Some(soon + 1),
        });
    }
    let c = external(&h).await;
    assert_eq!(
        read(&c, "get_usage", json!({})).await,
        json!({
            "accounts": [
                {
                    "name": "Principal",
                    "current": true,
                    "fiveHour": { "pct": 42.0, "resetsAt": soon },
                    "sevenDay": { "pct": 10.5, "resetsAt": soon + 1 },
                },
                { "name": "Pro", "current": false, "fiveHour": null, "sevenDay": null },
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
    // The 5-hour window used up: the autopilot waits for its end.
    h.core.usage.lock().five_hour = Some(RateWindow {
        pct: 100.0,
        resets_at: Some(soon),
    });
    let until = h.core.autopilot_pause().unwrap().until;
    let usage = read(&c, "get_usage", json!({})).await;
    assert_eq!(
        usage["autopilotPause"],
        json!({ "reason": "fiveHour", "until": until })
    );
    // Principal switched off: new agents go to Pro.
    h.core.settings.write().accounts[0].active = false;
    let usage = read(&c, "get_usage", json!({})).await;
    assert_eq!(
        (
            &usage["accounts"][0]["current"],
            &usage["accounts"][1]["current"]
        ),
        (&json!(false), &json!(true))
    );
    assert_eq!(usage["accounts"][1]["fiveHour"]["pct"], 100.0);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}
