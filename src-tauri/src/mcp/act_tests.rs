//! The tools that act, called by a real MCP client (rmcp's) on a test core: real git
//! repositories and the fake `claude`. Each does what the window does, under the guardrails of the
//! window and the autopilot; every call, refused ones included, is in the activity log.

use super::tests::{ACTING, AGENTS_ONLY, EXPOSED};
use super::tools_tests::{
    as_agent, external, keys, last_entry, other_project, project, put, put_agent, read, refused,
    ticket, Client,
};
use crate::board::clock;
use crate::core_tests::{harness, Harness};
use crate::model::*;
use crate::tickets::TicketDraft;
use serde_json::{json, Value};

/// A ticket made as the window makes one (at the end of « À faire »).
async fn made(h: &Harness, p: &Project, title: &str) -> Ticket {
    h.core
        .ticket_create(
            &p.id,
            TicketDraft {
                title: title.into(),
                ..Default::default()
            },
        )
        .await
        .unwrap()
}

/// `made`, coming after `after`.
async fn made_after(h: &Harness, p: &Project, title: &str, after: &[&Ticket]) -> Ticket {
    h.core
        .ticket_create(
            &p.id,
            TicketDraft {
                title: title.into(),
                after: after.iter().map(|t| t.id.clone()).collect(),
                ..Default::default()
            },
        )
        .await
        .unwrap()
}

fn stored(h: &Harness, id: &str) -> Ticket {
    h.core.ticket(id).unwrap()
}

/// The keys of the tickets « À faire » of `p`, in the order the board shows them.
async fn todo(c: &Client, p: &Project) -> Vec<String> {
    keys(
        &read(
            c,
            "list_tickets",
            json!({ "project": p.id, "column": "todo" }),
        )
        .await,
    )
}

/// The activity log's last entry, whole.
fn entry(h: &Harness) -> crate::mcp::activity::ActivityEntry {
    h.core.mcp.activity.entries().pop().unwrap()
}

/// How many `ticket` events the window was sent for ticket `id`.
fn ticket_events(h: &Harness, id: &str) -> usize {
    h.events
        .lock()
        .iter()
        .filter(|e| e["type"] == "ticket" && e["ticket"]["id"] == id)
        .count()
}

/// An agent of `p` with a running turn (no process): it counts as working.
fn working_agent(h: &Harness, p: &Project, name: &str, at: i64) -> AgentMeta {
    let meta = put_agent(h, p, name, at);
    h.core.agent(&meta.id).unwrap().lock().meta.status = AgentStatus::Running;
    meta
}

/// Waits until the agent's turn is over.
async fn turn_over(h: &Harness, id: &str) {
    h.wait("the agent's turn to end", |h| {
        let m = h.agent(id);
        !m.status.is_active() && m.prompts > 0
    })
    .await;
}

#[tokio::test]
async fn the_acting_tools_say_what_they_take_and_that_they_change_things() {
    let h = harness("mcp-act-schemas");
    let c = external(&h).await;
    let tools = c.list_all_tools().await.unwrap();
    let schema = |name: &str| {
        let t = tools.iter().find(|t| t.name == name).unwrap();
        serde_json::to_value(&t.input_schema).unwrap()
    };
    for name in ACTING.iter().chain(AGENTS_ONLY) {
        let t = tools.iter().find(|t| t.name == *name).unwrap();
        let v = serde_json::to_value(t).unwrap();
        assert!(v["description"].as_str().unwrap().len() > 60, "{name}: {v}");
        // They change something: the client must not take them for reads.
        assert_eq!(v["annotations"]["readOnlyHint"], false, "{name}: {v}");
        assert_eq!(v["annotations"]["openWorldHint"], false, "{name}: {v}");
        for (arg, spec) in schema(name)["properties"].as_object().unwrap() {
            assert!(
                spec["description"]
                    .as_str()
                    .is_some_and(|d| d.len() > 10 && !d.contains('\n')),
                "{name}.{arg}: {spec}"
            );
        }
    }
    let required = |name: &str| {
        let mut r: Vec<String> = serde_json::from_value(schema(name)["required"].clone()).unwrap();
        r.sort();
        r
    };
    assert_eq!(required("create_ticket"), ["project", "title"]);
    assert_eq!(required("update_ticket"), ["ticket"]);
    assert_eq!(required("move_ticket"), ["position", "ticket"]);
    assert_eq!(required("start_ticket"), ["ticket"]);
    assert_eq!(required("create_agent"), ["message", "project"]);
    assert_eq!(required("send_message"), ["agent", "text"]);
    assert_eq!(required("stop_agent"), ["agent"]);
    assert_eq!(required("report_progress"), ["line"]);
    assert_eq!(required("split_ticket"), ["tickets"]);
    let position = schema("move_ticket")["properties"]["position"].to_string();
    for p in ["top", "bottom", "before"] {
        assert!(position.contains(p), "{position}");
    }
    assert!(
        schema("report_progress")["properties"]["line"]["description"]
            .as_str()
            .unwrap()
            .contains("120")
    );
    // Exactly the set the server is meant to offer.
    let mut listed: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();
    listed.retain(|n| n != "whoami");
    listed.sort();
    assert_eq!(listed, EXPOSED);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_is_created_in_to_do_with_its_criteria_and_what_it_comes_after() {
    let h = harness("mcp-act-create-ticket");
    let p = project(&h).await;
    let c = external(&h).await;
    let first = read(
        &c,
        "create_ticket",
        json!({
            "project": "DEMO",
            "title": "  Ajouter la page d’accueil  ",
            "description": "Une page.\nAvec un titre.",
            "criteria": ["Le titre s’affiche", "  ", "Le lien marche\nLe test passe"],
        }),
    )
    .await;
    // The answer is the ticket as `get_ticket` gives it.
    let id = first["id"].as_str().unwrap().to_string();
    assert_eq!(
        first,
        json!({
            "id": id,
            "key": "DEM-1",
            "title": "Ajouter la page d’accueil",
            "description": "Une page.\nAvec un titre.",
            "criteria": [
                { "text": "Le titre s’affiche", "ok": false, "note": "" },
                { "text": "Le lien marche", "ok": false, "note": "" },
                { "text": "Le test passe", "ok": false, "note": "" },
            ],
            "loops": { "iteration": 0, "max": 5 },
            "column": "todo",
            "after": [],
            "progress": [],
            "external": null,
        })
    );
    // Made on the board, the window told as it is for a ticket made there.
    let t = stored(&h, &id);
    assert_eq!(
        (t.column, t.project_id.as_str()),
        (Column::Todo, p.id.as_str())
    );
    assert!(ticket_events(&h, &id) >= 1);
    assert!(h.events.lock().iter().any(|e| e["type"] == "project"));
    let (caller, tool, summary, outcome) = last_entry(&h);
    assert_eq!(
        (caller.as_str(), tool.as_str(), outcome),
        ("Claude (hors Escouade)", "create_ticket", "ok")
    );
    assert!(summary.contains("project: DEMO") && summary.contains("Ajouter la page"));
    assert_eq!(
        read(&c, "get_ticket", json!({ "ticket": "dem-1" })).await,
        first
    );

    // Without criteria the default two; after another ticket, named by its key in any case.
    let second = read(
        &c,
        "create_ticket",
        json!({ "project": p.id, "title": "La suite", "after": ["dem-1"] }),
    )
    .await;
    assert_eq!(second["key"], "DEM-2");
    assert_eq!(second["after"], json!(["DEM-1"]));
    assert_eq!(
        second["criteria"][0]["text"],
        "Implémentation conforme au ticket"
    );
    assert_eq!(
        stored(&h, second["id"].as_str().unwrap()).after,
        vec![id.clone()]
    );

    // Refused or failed, and told why; nothing is made.
    let before = h.core.tickets.read().len();
    let no_project = refused(
        &c,
        "create_ticket",
        json!({ "project": "nope", "title": "x" }),
    )
    .await;
    assert!(no_project.contains("Aucun projet « nope »"), "{no_project}");
    assert_eq!(entry(&h).outcome, "error");
    let blank = refused(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "   " }),
    )
    .await;
    assert!(blank.contains("titre"), "{blank}");
    let no_after = refused(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "x", "after": ["ZZZ-9"] }),
    )
    .await;
    assert!(
        no_after.contains("Aucun ticket « ZZZ-9 » dans le projet demo")
            && no_after.contains("DEM-1"),
        "{no_after}"
    );
    // A ticket of another project is no ticket to come after.
    let other = other_project(&h, "autre").await;
    let theirs = put(&h, ticket(&other, "AUT-1", "Ailleurs", Column::Todo, 1));
    let elsewhere = refused(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "x", "after": [theirs.key] }),
    )
    .await;
    assert!(elsewhere.contains("dans le projet demo"), "{elsewhere}");
    let too_long = refused(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "t".repeat(201) }),
    )
    .await;
    assert!(too_long.contains("200"), "{too_long}");
    assert_eq!(h.core.tickets.read().len(), before + 1);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_to_do_is_edited_a_field_at_a_time_and_never_into_a_loop() {
    let h = harness("mcp-act-update-ticket");
    let p = project(&h).await;
    let a = made(&h, &p, "Premier").await;
    let b = made_after(&h, &p, "Deuxième", &[&a]).await;
    let c = external(&h).await;

    // Only what is given changes.
    let v = read(
        &c,
        "update_ticket",
        json!({ "ticket": "dem-2", "title": "  Deuxième, revu " }),
    )
    .await;
    assert_eq!(v["title"], "Deuxième, revu");
    assert_eq!(v["after"], json!(["DEM-1"]));
    assert_eq!(v["criteria"][1]["text"], "Tests verts");
    let v = read(
        &c,
        "update_ticket",
        json!({ "ticket": b.id, "description": "Plus de détails", "criteria": ["Un seul critère"] }),
    )
    .await;
    assert_eq!(v["description"], "Plus de détails");
    assert_eq!(v["title"], "Deuxième, revu");
    assert_eq!(
        v["criteria"],
        json!([{ "text": "Un seul critère", "ok": false, "note": "" }])
    );
    let seen = ticket_events(&h, &b.id);
    // An empty list takes its dependencies away, a list sets them.
    let v = read(
        &c,
        "update_ticket",
        json!({ "ticket": "DEM-2", "after": [] }),
    )
    .await;
    assert_eq!(v["after"], json!([]));
    assert!(stored(&h, &b.id).after.is_empty());
    let v = read(
        &c,
        "update_ticket",
        json!({ "ticket": "DEM-2", "after": ["DEM-1"] }),
    )
    .await;
    assert_eq!(v["after"], json!(["DEM-1"]));
    assert!(ticket_events(&h, &b.id) >= seen + 2);
    assert_eq!(entry(&h).tool, "update_ticket");

    // DEM-1 after DEM-2, which waits for DEM-1: refused with the board's own words.
    let loop_ = refused(
        &c,
        "update_ticket",
        json!({ "ticket": "DEM-1", "after": ["DEM-2"] }),
    )
    .await;
    assert_eq!(loop_, "DEM-2 attend déjà DEM-1 (directement ou non).");
    assert!(stored(&h, &a.id).after.is_empty());
    let logged = entry(&h);
    assert_eq!(
        (logged.outcome, logged.message.as_deref()),
        ("refused", Some(loop_.as_str()))
    );

    // Only a ticket « À faire » is edited.
    let doing = put(&h, ticket(&p, "DEM-9", "En route", Column::Doing, 5));
    let late = refused(
        &c,
        "update_ticket",
        json!({ "ticket": doing.key, "title": "Autre" }),
    )
    .await;
    assert_eq!(late, "Seul un ticket « À faire » se modifie.");
    assert_eq!(entry(&h).outcome, "refused");
    assert_eq!(stored(&h, &doing.id).title, "En route");

    // Nothing to change, a blank title, a ticket that is not there.
    let none = refused(&c, "update_ticket", json!({ "ticket": "DEM-1" })).await;
    assert!(none.contains("Rien à modifier"), "{none}");
    let blank = refused(
        &c,
        "update_ticket",
        json!({ "ticket": "DEM-1", "title": " " }),
    )
    .await;
    assert!(blank.contains("titre"), "{blank}");
    assert_eq!(stored(&h, &a.id).title, "Premier");
    let unknown = refused(
        &c,
        "update_ticket",
        json!({ "ticket": "ZZZ-1", "title": "x" }),
    )
    .await;
    assert!(unknown.contains("Aucun ticket « ZZZ-1 »"), "{unknown}");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_to_do_goes_to_the_top_the_bottom_or_before_another_and_nothing_else_moves() {
    let h = harness("mcp-act-move-ticket");
    let p = project(&h).await;
    for title in ["Un", "Deux", "Trois", "Quatre"] {
        made(&h, &p, title).await;
    }
    let c = external(&h).await;
    let order = || todo(&c, &p);
    assert_eq!(order().await, ["DEM-1", "DEM-2", "DEM-3", "DEM-4"]);

    let v = read(
        &c,
        "move_ticket",
        json!({ "ticket": "dem-4", "position": "top" }),
    )
    .await;
    assert_eq!(v["todo"], json!(["DEM-4", "DEM-1", "DEM-2", "DEM-3"]));
    assert_eq!(entry(&h).tool, "move_ticket");
    assert_eq!(order().await, ["DEM-4", "DEM-1", "DEM-2", "DEM-3"]);
    let t4 = h
        .core
        .tickets
        .read()
        .iter()
        .find(|t| t.key == "DEM-4")
        .cloned()
        .unwrap();
    assert!(ticket_events(&h, &t4.id) >= 2);

    read(
        &c,
        "move_ticket",
        json!({ "ticket": "DEM-4", "position": "bottom" }),
    )
    .await;
    assert_eq!(order().await, ["DEM-1", "DEM-2", "DEM-3", "DEM-4"]);
    read(
        &c,
        "move_ticket",
        json!({ "ticket": "DEM-3", "position": "before", "before": "dem-1" }),
    )
    .await;
    assert_eq!(order().await, ["DEM-3", "DEM-1", "DEM-2", "DEM-4"]);
    // Between two tickets whose ranks leave no room: the others keep their order.
    read(
        &c,
        "move_ticket",
        json!({ "ticket": "DEM-4", "position": "before", "before": "DEM-2" }),
    )
    .await;
    assert_eq!(order().await, ["DEM-3", "DEM-1", "DEM-4", "DEM-2"]);
    // Where it is already: nothing changes.
    let seen: usize = h
        .core
        .tickets
        .read()
        .iter()
        .map(|t| ticket_events(&h, &t.id))
        .sum();
    read(
        &c,
        "move_ticket",
        json!({ "ticket": "DEM-1", "position": "before", "before": "DEM-4" }),
    )
    .await;
    assert_eq!(order().await, ["DEM-3", "DEM-1", "DEM-4", "DEM-2"]);
    let now: usize = h
        .core
        .tickets
        .read()
        .iter()
        .map(|t| ticket_events(&h, &t.id))
        .sum();
    assert_eq!(seen, now);

    // A ticket that left « À faire » does not move, and nothing is put before one.
    let doing = put(&h, ticket(&p, "DEM-9", "En route", Column::Doing, 5));
    let rank = doing.rank;
    let not_todo = refused(
        &c,
        "move_ticket",
        json!({ "ticket": "DEM-9", "position": "top" }),
    )
    .await;
    assert_eq!(not_todo, "Seul un ticket « À faire » se déplace.");
    assert_eq!(entry(&h).outcome, "refused");
    assert_eq!(stored(&h, &doing.id).rank, rank);
    let before_it = refused(
        &c,
        "move_ticket",
        json!({ "ticket": "DEM-1", "position": "before", "before": "DEM-9" }),
    )
    .await;
    assert!(
        before_it.contains("DEM-9") && before_it.contains("« À faire »"),
        "{before_it}"
    );
    let itself = refused(
        &c,
        "move_ticket",
        json!({ "ticket": "DEM-1", "position": "before", "before": "DEM-1" }),
    )
    .await;
    assert!(itself.contains("avant lui-même"), "{itself}");
    let missing = refused(
        &c,
        "move_ticket",
        json!({ "ticket": "DEM-1", "position": "before" }),
    )
    .await;
    assert!(missing.contains("before"), "{missing}");
    let extra = refused(
        &c,
        "move_ticket",
        json!({ "ticket": "DEM-1", "position": "top", "before": "DEM-2" }),
    )
    .await;
    assert!(extra.contains("before"), "{extra}");
    let other = other_project(&h, "autre").await;
    put(&h, ticket(&other, "AUT-1", "Ailleurs", Column::Todo, 1));
    let foreign = refused(
        &c,
        "move_ticket",
        json!({ "ticket": "DEM-1", "position": "before", "before": "AUT-1" }),
    )
    .await;
    assert!(foreign.contains("dans le projet demo"), "{foreign}");
    assert_eq!(order().await, ["DEM-3", "DEM-1", "DEM-4", "DEM-2"]);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_is_started_by_hand_and_the_scheduler_takes_it() {
    let h = harness("mcp-act-start-ticket");
    let p = project(&h).await;
    let a = made(&h, &p, "Un ticket [ok]").await;
    let c = external(&h).await;
    assert!(!stored(&h, &a.id).forced);
    let v = read(&c, "start_ticket", json!({ "ticket": "dem-1" })).await;
    assert_eq!(v["key"], "DEM-1");
    let logged = entry(&h);
    assert_eq!(
        (
            logged.tool.as_str(),
            logged.outcome,
            logged.summary.as_str()
        ),
        ("start_ticket", "ok", "ticket: dem-1")
    );
    // The scheduler gives it an agent, as it does for the window's « Lancer ».
    h.wait("the ticket to start", |h| {
        stored(h, &a.id).column != Column::Todo
    })
    .await;
    assert!(h.events.lock().iter().any(|e| e["type"] == "ticket"
        && e["ticket"]["id"] == a.id.as_str()
        && e["ticket"]["forced"] == true));
    h.wait("its agent to be made", |h| {
        stored(h, &a.id).agent_id.is_some()
    })
    .await;
    // Started already: refused.
    let again = refused(&c, "start_ticket", json!({ "ticket": a.key })).await;
    assert_eq!(again, "Ce ticket est déjà parti.");
    assert_eq!(entry(&h).outcome, "refused");
    h.wait("its turn to end", |h| {
        stored(h, &a.id).column == Column::Review
    })
    .await;
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_is_not_started_while_the_autopilot_is_paused_the_board_is_full_or_it_waits() {
    let h = harness("mcp-act-start-guards");
    let p = project(&h).await;
    let a = made(&h, &p, "Premier").await;
    let b = made_after(&h, &p, "Deuxième", &[&a]).await;
    let c = external(&h).await;

    // Paused after a usage limit: the message says until when, and the ticket is not marked.
    h.core.pause_after_limit();
    let until = h.core.autopilot_pause().unwrap().until;
    let paused = refused(&c, "start_ticket", json!({ "ticket": a.key })).await;
    assert!(
        paused.contains("pause")
            && paused.contains("limite d’usage")
            && paused.contains(&clock(until)),
        "{paused}"
    );
    assert!(!stored(&h, &a.id).forced);
    let logged = entry(&h);
    assert_eq!(
        (
            logged.tool.as_str(),
            logged.outcome,
            logged.message.as_deref()
        ),
        ("start_ticket", "refused", Some(paused.as_str()))
    );
    h.core.autopilot_resume();
    assert!(h.core.autopilot_pause().is_none());

    // A window of the quota over the threshold: the percentage and the end.
    let end = h.core.pause_now() + 3_600_000;
    h.core.usage.lock().five_hour = Some(RateWindow {
        pct: 100.0,
        resets_at: Some(end),
    });
    // It holds until the window's end and the margin a resume waits after it.
    let until = h.core.autopilot_pause().unwrap().until;
    assert!(until > end);
    let over = refused(&c, "start_ticket", json!({ "ticket": a.key })).await;
    assert!(
        over.contains("5 h") && over.contains("100 %") && over.contains(&clock(until)),
        "{over}"
    );
    *h.core.usage.lock() = UsageSnapshot::default();
    assert!(h.core.autopilot_pause().is_none());

    // An agent waits for its quota: nothing starts until it resumes.
    let waiting = put_agent(&h, &p, "attend", 1);
    let resume = h.core.pause_now() + 7_200_000;
    h.core.agent(&waiting.id).unwrap().lock().meta.resume_at = Some(resume);
    let quota = refused(&c, "start_ticket", json!({ "ticket": a.key })).await;
    assert!(
        quota.contains("quota") && quota.contains(&clock(resume)),
        "{quota}"
    );
    h.core.agent(&waiting.id).unwrap().lock().meta.resume_at = None;

    // The board already runs its most tickets in parallel (2 by default): blocked ones do not count.
    let d1 = put(&h, ticket(&p, "DEM-7", "En route", Column::Doing, 10));
    put(&h, ticket(&p, "DEM-8", "Aussi", Column::Doing, 11));
    let full = refused(&c, "start_ticket", json!({ "ticket": a.key })).await;
    assert!(
        full.contains("demo") && full.contains("(2,") && full.contains("« En parallèle »"),
        "{full}"
    );
    assert!(!stored(&h, &a.id).forced);
    h.core
        .edit_ticket(&d1.id, |t| {
            t.blocked = Some("bloqué".into());
            Ok(())
        })
        .unwrap();
    // One place is free again: the ticket that waits for DEM-1 is still refused, and which.
    let waits = refused(&c, "start_ticket", json!({ "ticket": b.key })).await;
    assert!(
        waits.contains("DEM-2") && waits.contains("DEM-1") && waits.contains("update_ticket"),
        "{waits}"
    );
    assert!(!stored(&h, &b.id).forced);
    assert_eq!(entry(&h).outcome, "refused");

    // A board that starts nothing says why.
    h.core
        .board_issues
        .lock()
        .insert(p.id.clone(), "Branche cible main introuvable".into());
    let issue = refused(&c, "start_ticket", json!({ "ticket": a.key })).await;
    assert!(issue.contains("Branche cible main introuvable"), "{issue}");
    assert!(!stored(&h, &a.id).forced);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agent_is_created_in_a_project_with_its_first_message_and_the_user_s_selection_stays() {
    let h = harness("mcp-act-create-agent");
    let (p, _) = h.project(false).await;
    // Saving a project's settings looks again at whether the server is wanted.
    h.core.settings.write().mcp_enabled = true;
    let c = external(&h).await;
    let v = read(
        &c,
        "create_agent",
        json!({ "project": "demo", "message": "Bonjour l’équipe" }),
    )
    .await;
    let id = v["id"].as_str().unwrap().to_string();
    assert_eq!(v["name"], "agent-1");
    assert_eq!(v.as_object().unwrap().len(), 2);
    // Made as the window makes one, but the sidebar's selection is the user's.
    let meta = h.agent(&id);
    assert_eq!(
        (meta.project_id.as_str(), meta.name.as_str()),
        (p.id.as_str(), "agent-1")
    );
    assert!(meta.worktree.is_none());
    assert!(!h.core.ui.read().selected_agent.contains_key(&p.id));
    assert!(h
        .events
        .lock()
        .iter()
        .any(|e| e["type"] == "agent" && e["agent"]["id"] == id.as_str()));
    // Its first message is sent: Claude answers it.
    turn_over(&h, &id).await;
    assert!(h.items(&id).iter().any(|i| i["text"]
        == "Bonjour, tu as dit : Message de Claude (hors Escouade) : Bonjour l’équipe"));
    // The conversation shows who wrote it: it is not the user's own message.
    assert!(h.items(&id).iter().any(|i| i["kind"] == "user"
        && i["text"] == "Message de Claude (hors Escouade) : Bonjour l’équipe"));
    let logged = entry(&h);
    assert_eq!(
        (logged.tool.as_str(), logged.outcome),
        ("create_agent", "ok")
    );
    assert!(
        logged.summary.contains("project: demo") && logged.summary.contains("Bonjour l’équipe")
    );

    // A worktree of its own, or the project's folder, whatever the project does by default.
    let v = read(
        &c,
        "create_agent",
        json!({ "project": p.id, "message": "Dans un worktree", "worktree": true }),
    )
    .await;
    let wt = h.agent(v["id"].as_str().unwrap()).worktree;
    assert!(wt.is_some_and(|w| std::path::Path::new(&w.path).is_dir()));
    turn_over(&h, v["id"].as_str().unwrap()).await;
    h.core
        .update_project(Project {
            worktree_per_agent: true,
            ..h.core.project(&p.id).unwrap()
        })
        .unwrap();
    let v = read(
        &c,
        "create_agent",
        json!({ "project": p.id, "message": "Par défaut" }),
    )
    .await;
    assert!(h.agent(v["id"].as_str().unwrap()).worktree.is_some());
    turn_over(&h, v["id"].as_str().unwrap()).await;
    let v = read(
        &c,
        "create_agent",
        json!({ "project": p.id, "message": "Sans", "worktree": false }),
    )
    .await;
    assert!(h.agent(v["id"].as_str().unwrap()).worktree.is_none());
    turn_over(&h, v["id"].as_str().unwrap()).await;

    // A model, as the window offers them; a name that is none is refused before anything is made.
    *h.core.models.write() = vec![ModelInfo {
        value: "sonnet".into(),
        resolved_model: "claude-sonnet-9".into(),
    }];
    let count = h.core.agents.read().len();
    let v = read(
        &c,
        "create_agent",
        json!({ "project": p.id, "message": "Un modèle", "model": "sonnet" }),
    )
    .await;
    assert_eq!(h.agent(v["id"].as_str().unwrap()).model, "sonnet");
    turn_over(&h, v["id"].as_str().unwrap()).await;
    let unknown = refused(
        &c,
        "create_agent",
        json!({ "project": p.id, "message": "x", "model": "opus" }),
    )
    .await;
    assert!(
        unknown.contains("opus") && unknown.contains("sonnet"),
        "{unknown}"
    );
    let odd = refused(
        &c,
        "create_agent",
        json!({ "project": p.id, "message": "x", "model": "--permission-mode" }),
    )
    .await;
    assert!(odd.contains("--permission-mode"), "{odd}");
    let blank = refused(
        &c,
        "create_agent",
        json!({ "project": p.id, "message": "  " }),
    )
    .await;
    assert!(blank.contains("message"), "{blank}");
    let none = refused(
        &c,
        "create_agent",
        json!({ "project": "nope", "message": "x" }),
    )
    .await;
    assert!(none.contains("Aucun projet « nope »"), "{none}");
    assert_eq!(h.core.agents.read().len(), count + 1);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn no_agent_is_created_while_the_autopilot_is_paused_or_the_project_s_agents_are_all_at_work()
{
    let h = harness("mcp-act-create-agent-guards");
    let (p, _) = h.project(false).await;
    let c = external(&h).await;

    h.core.pause_after_limit();
    let until = h.core.autopilot_pause().unwrap().until;
    let paused = refused(
        &c,
        "create_agent",
        json!({ "project": "demo", "message": "x" }),
    )
    .await;
    assert!(
        paused.contains("pause") && paused.contains(&clock(until)),
        "{paused}"
    );
    assert!(h.core.agents.read().is_empty());
    let logged = entry(&h);
    assert_eq!(
        (
            logged.tool.as_str(),
            logged.outcome,
            logged.message.as_deref()
        ),
        ("create_agent", "refused", Some(paused.as_str()))
    );
    h.core.autopilot_resume();

    // Two agents work (the project's maximum is 2); one that is done, idle or archived does not count.
    working_agent(&h, &p, "un", 1);
    let other = working_agent(&h, &p, "deux", 2);
    put_agent(&h, &p, "oisif", 3);
    let gone = working_agent(&h, &p, "archivé", 4);
    h.core.agent(&gone.id).unwrap().lock().meta.archived = true;
    let full = refused(
        &c,
        "create_agent",
        json!({ "project": "demo", "message": "x" }),
    )
    .await;
    assert!(
        full.contains("demo") && full.contains("(2,") && full.contains("« En parallèle »"),
        "{full}"
    );
    assert_eq!(h.core.agents.read().len(), 4);
    assert_eq!(entry(&h).outcome, "refused");
    // One finishes: a place is free.
    h.core.agent(&other.id).unwrap().lock().meta.status = AgentStatus::Done;
    let v = read(
        &c,
        "create_agent",
        json!({ "project": "demo", "message": "Place libre" }),
    )
    .await;
    turn_over(&h, v["id"].as_str().unwrap()).await;
    // Another project has its own count.
    let elsewhere = other_project(&h, "autre").await;
    let v = read(
        &c,
        "create_agent",
        json!({ "project": elsewhere.id, "message": "Ailleurs" }),
    )
    .await;
    turn_over(&h, v["id"].as_str().unwrap()).await;
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_message_reaches_an_agent_unless_the_autopilot_is_paused_it_is_oneself_or_archived() {
    let h = harness("mcp-act-send-message");
    let (p, _) = h.project(false).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    let b = h.core.create_agent(&p.id, None).await.unwrap().meta;
    let c = external(&h).await;

    let v = read(
        &c,
        "send_message",
        json!({ "agent": a.name, "text": "Salut" }),
    )
    .await;
    assert_eq!(
        (v["id"].as_str(), v["name"].as_str()),
        (Some(a.id.as_str()), Some("agent-1"))
    );
    turn_over(&h, &a.id).await;
    // The agent is told who writes, and the conversation shows it.
    assert!(h
        .items(&a.id)
        .iter()
        .any(|i| i["text"] == "Bonjour, tu as dit : Message de Claude (hors Escouade) : Salut"));
    assert!(h
        .items(&a.id)
        .iter()
        .any(|i| i["kind"] == "user" && i["text"] == "Message de Claude (hors Escouade) : Salut"));
    let logged = entry(&h);
    assert_eq!(
        (logged.tool.as_str(), logged.outcome),
        ("send_message", "ok")
    );
    assert!(logged.summary.contains("agent: agent-1") && logged.summary.contains("text: Salut"));

    // Another agent may write to it; to itself it may not.
    let from_b = as_agent(&h, &b.id).await;
    read(
        &from_b,
        "send_message",
        json!({ "agent": a.id, "text": "De la part de B" }),
    )
    .await;
    turn_over(&h, &a.id).await;
    assert!(h
        .items(&a.id)
        .iter()
        .any(|i| i["text"] == "Bonjour, tu as dit : Message de agent-2 : De la part de B"));
    assert!(h
        .items(&a.id)
        .iter()
        .any(|i| i["kind"] == "user" && i["text"] == "Message de agent-2 : De la part de B"));
    let prompts = h.agent(&b.id).prompts;
    let itself = refused(
        &from_b,
        "send_message",
        json!({ "agent": "AGENT-2", "text": "Moi" }),
    )
    .await;
    assert!(
        itself.contains("lui-même") || itself.contains("toi-même"),
        "{itself}"
    );
    assert_eq!(h.agent(&b.id).prompts, prompts);
    let logged = entry(&h);
    assert_eq!(
        (logged.caller.as_str(), logged.tool.as_str(), logged.outcome),
        ("agent-2", "send_message", "refused")
    );
    from_b.cancel().await.unwrap();

    // Paused: no message goes out (it would spend quota), and the answer says until when.
    h.core.pause_after_limit();
    let until = h.core.autopilot_pause().unwrap().until;
    let prompts = h.agent(&a.id).prompts;
    let paused = refused(
        &c,
        "send_message",
        json!({ "agent": a.id, "text": "Pause" }),
    )
    .await;
    assert!(
        paused.contains("pause") && paused.contains(&clock(until)),
        "{paused}"
    );
    assert_eq!(h.agent(&a.id).prompts, prompts);
    h.core.autopilot_resume();

    // An archived agent takes no message; a blank text, an unknown agent: told, nothing sent.
    let b_prompts = h.agent(&b.id).prompts;
    h.core.agent(&b.id).unwrap().lock().meta.archived = true;
    let archived = refused(
        &c,
        "send_message",
        json!({ "agent": b.id, "text": "Encore" }),
    )
    .await;
    assert!(archived.contains("archivé"), "{archived}");
    assert_eq!(h.agent(&b.id).prompts, b_prompts);
    let blank = refused(&c, "send_message", json!({ "agent": a.id, "text": " " })).await;
    assert!(blank.contains("message"), "{blank}");
    let unknown = refused(&c, "send_message", json!({ "agent": "zzz", "text": "x" })).await;
    assert!(
        unknown.contains("Aucun agent « zzz »") && unknown.contains("agent-1"),
        "{unknown}"
    );
    assert_eq!(h.agent(&a.id).prompts, prompts);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_running_turn_is_interrupted_by_another_and_never_by_the_agent_itself() {
    let h = harness("mcp-act-stop-agent");
    let (p, _) = h.project(false).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    let b = h.core.create_agent(&p.id, None).await.unwrap().meta;
    let c = external(&h).await;
    // Not at work: nothing to stop, and it says so.
    let v = read(&c, "stop_agent", json!({ "agent": "agent-1" })).await;
    assert_eq!(
        (v["id"].as_str(), v["interrupted"].as_bool()),
        (Some(a.id.as_str()), Some(false))
    );

    h.core
        .send_message(&a.id, "slow".into(), vec![])
        .await
        .unwrap();
    h.wait("its turn to run", |h| h.agent(&a.id).status.is_active())
        .await;
    // Not itself.
    let from_a = as_agent(&h, &a.id).await;
    let itself = refused(&from_a, "stop_agent", json!({ "agent": a.name })).await;
    assert!(itself.contains("propre tour"), "{itself}");
    assert!(h.agent(&a.id).status.is_active());
    let logged = entry(&h);
    assert_eq!(
        (logged.caller.as_str(), logged.tool.as_str(), logged.outcome),
        ("agent-1", "stop_agent", "refused")
    );
    from_a.cancel().await.unwrap();
    // Another agent may.
    let from_b = as_agent(&h, &b.id).await;
    let v = read(&from_b, "stop_agent", json!({ "agent": a.id })).await;
    assert_eq!(
        (v["name"].as_str(), v["interrupted"].as_bool()),
        (Some("agent-1"), Some(true))
    );
    h.wait("the turn to stop", |h| !h.agent(&a.id).status.is_active())
        .await;
    assert_eq!(entry(&h).outcome, "ok");
    let unknown = refused(&from_b, "stop_agent", json!({ "agent": "zzz" })).await;
    assert!(unknown.contains("Aucun agent « zzz »"), "{unknown}");
    from_b.cancel().await.unwrap();
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agent_reports_what_it_does_on_one_line_and_a_new_report_replaces_it() {
    let h = harness("mcp-act-report-progress");
    let (p, _) = h.project(false).await;
    let a = put_agent(&h, &p, "rapporteur", 1);
    let c = as_agent(&h, &a.id).await;
    let line = || h.agent(&a.id).progress_line;
    assert_eq!(line(), None);
    let v = read(
        &c,
        "report_progress",
        json!({ "line": "  Écrit les tests du parseur  " }),
    )
    .await;
    assert_eq!(v, json!({ "line": "Écrit les tests du parseur" }));
    assert_eq!(line().as_deref(), Some("Écrit les tests du parseur"));
    // The window is told (a card shows it).
    assert!(h.events.lock().iter().any(|e| e["type"] == "agent"
        && e["agent"]["id"] == a.id.as_str()
        && e["agent"]["progressLine"] == "Écrit les tests du parseur"));
    let logged = entry(&h);
    assert_eq!(
        (
            logged.caller.as_str(),
            logged.tool.as_str(),
            logged.outcome,
            logged.summary.as_str()
        ),
        (
            "rapporteur",
            "report_progress",
            "ok",
            "line: Écrit les tests du parseur"
        )
    );
    // One line: breaks and runs of spaces become single spaces.
    read(
        &c,
        "report_progress",
        json!({ "line": "Lit\nle code\r\n  du   parseur" }),
    )
    .await;
    assert_eq!(line().as_deref(), Some("Lit le code du parseur"));
    // 120 characters (not bytes) are the most.
    let longest = "é".repeat(120);
    read(&c, "report_progress", json!({ "line": longest })).await;
    assert_eq!(line().as_deref(), Some(longest.as_str()));
    let too_long = refused(&c, "report_progress", json!({ "line": "é".repeat(121) })).await;
    assert!(
        too_long.contains("121") && too_long.contains("120"),
        "{too_long}"
    );
    assert_eq!(line().as_deref(), Some(longest.as_str()));
    assert_eq!(entry(&h).outcome, "error");
    // Nothing hidden is kept: a direction override would reverse how the line reads on the cards,
    // a zero-width mark hide part of it.
    let sneaky = "Lit le code\u{202E}reverse\u{200B}d\u{FEFF} \u{2066}x\u{2069}\u{0007} é日本😀";
    let v = read(&c, "report_progress", json!({ "line": sneaky })).await;
    assert_eq!(v, json!({ "line": "Lit le codereversed x é日本😀" }));
    assert_eq!(line().as_deref(), Some("Lit le codereversed x é日本😀"));
    // Only a line that is nothing but such marks is empty.
    let v = read(&c, "report_progress", json!({ "line": "\u{202E}\u{200B}" })).await;
    assert_eq!(v, json!({ "line": null }));
    assert_eq!(line(), None);
    // Nothing on the line takes the old one off the cards.
    let v = read(&c, "report_progress", json!({ "line": "  " })).await;
    assert_eq!(v, json!({ "line": null }));
    assert_eq!(line(), None);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_status_line_is_kept_with_the_agent_and_an_agent_saved_before_it_has_none() {
    let h = harness("mcp-act-progress-saved");
    let (p, _) = h.project(false).await;
    let a = put_agent(&h, &p, "gardé", 1);
    let c = as_agent(&h, &a.id).await;
    read(&c, "report_progress", json!({ "line": "Relit le diff" })).await;
    c.cancel().await.unwrap();
    h.core.mcp.stop();
    h.core.save_now();
    let data = crate::paths::DataDir::new(h.dir.join("data"));
    let app = tauri::test::mock_app();
    let (reloaded, _rx) = crate::core::Core::load(app.handle().clone(), data);
    assert_eq!(
        reloaded
            .agent(&a.id)
            .unwrap()
            .lock()
            .meta
            .progress_line
            .as_deref(),
        Some("Relit le diff")
    );
    // A state.json of 1.6 has no such field.
    let old =
        r#"{ "projects": [], "agents": [{ "id": "a1", "name": "vieux", "projectId": "p1" }] }"#;
    let state: PersistedState = serde_json::from_str(old).unwrap();
    assert_eq!(state.agents[0].progress_line, None);
    let back = serde_json::to_value(&state.agents[0]).unwrap();
    assert_eq!(back["progressLine"], Value::Null);
}

#[tokio::test]
async fn the_tools_for_agents_are_refused_to_claude_outside_escouade_and_logged() {
    let h = harness("mcp-act-reserved");
    let (p, _) = h.project(false).await;
    let a = put_agent(&h, &p, "agent", 1);
    let t = put(&h, ticket(&p, "DEM-1", "Le sien", Column::Doing, 1));
    let c = external(&h).await;
    for (tool, args) in [
        ("report_progress", json!({ "line": "Je travaille" })),
        ("split_ticket", json!({ "tickets": [{ "title": "Un" }] })),
    ] {
        let why = refused(&c, tool, args).await;
        assert!(
            why.contains("réservé aux agents d’Escouade") && why.contains("Claude hors Escouade"),
            "{tool}: {why}"
        );
        let logged = entry(&h);
        assert_eq!(
            (logged.caller.as_str(), logged.tool.as_str(), logged.outcome),
            ("Claude (hors Escouade)", tool, "refused")
        );
        assert_eq!(logged.message.as_deref(), Some(why.as_str()));
    }
    assert_eq!(h.agent(&a.id).progress_line, None);
    assert_eq!(h.core.tickets.read().len(), 1);
    assert_eq!(stored(&h, &t.id).title, "Le sien");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_is_split_into_tickets_after_it_in_a_chain_or_all_after_it_alone() {
    let h = harness("mcp-act-split-ticket");
    let p = project(&h).await;
    let own = made(&h, &p, "Gros ticket").await;
    h.core
        .edit_ticket(&own.id, |t| {
            t.column = Column::Doing;
            Ok(())
        })
        .unwrap();
    let a = put_agent(&h, &p, "gros", 1);
    h.core.agent(&a.id).unwrap().lock().meta.ticket_id = Some(own.id.clone());
    let c = as_agent(&h, &a.id).await;

    // Chained (by default): the first after the agent's ticket, each after the one before.
    let keys_made = read(
        &c,
        "split_ticket",
        json!({ "tickets": [
            { "title": "Étape 1", "description": "D’abord", "criteria": ["Un critère"] },
            { "title": "Étape 2" },
            { "title": "Étape 3" },
        ] }),
    )
    .await;
    assert_eq!(keys_made, json!(["DEM-2", "DEM-3", "DEM-4"]));
    let after = |key: &str| -> Vec<String> {
        let t = h
            .core
            .tickets
            .read()
            .iter()
            .find(|t| t.key == key)
            .cloned()
            .unwrap();
        t.after.iter().map(|id| stored(&h, id).key).collect()
    };
    assert_eq!(after("DEM-2"), ["DEM-1"]);
    assert_eq!(after("DEM-3"), ["DEM-2"]);
    assert_eq!(after("DEM-4"), ["DEM-3"]);
    let first = h
        .core
        .tickets
        .read()
        .iter()
        .find(|t| t.key == "DEM-2")
        .cloned()
        .unwrap();
    assert_eq!(
        (
            first.column,
            first.title.as_str(),
            first.description.as_str()
        ),
        (Column::Todo, "Étape 1", "D’abord")
    );
    assert_eq!(first.criteria.len(), 1);
    assert!(ticket_events(&h, &first.id) >= 1);
    // The agent's own ticket is left as it was.
    assert_eq!(stored(&h, &own.id).column, Column::Doing);
    let logged = entry(&h);
    assert_eq!(
        (logged.caller.as_str(), logged.tool.as_str(), logged.outcome),
        ("gros", "split_ticket", "ok")
    );
    assert!(logged.summary.contains("Étape 1"));

    // In parallel: all after the agent's ticket alone.
    let parallel = read(
        &c,
        "split_ticket",
        json!({ "tickets": [{ "title": "Voie A" }, { "title": "Voie B" }], "chain": false }),
    )
    .await;
    assert_eq!(parallel, json!(["DEM-5", "DEM-6"]));
    assert_eq!(after("DEM-5"), ["DEM-1"]);
    assert_eq!(after("DEM-6"), ["DEM-1"]);

    // Nothing is made when one of them is no ticket, or none is given, or too many.
    let count = h.core.tickets.read().len();
    let blank = refused(
        &c,
        "split_ticket",
        json!({ "tickets": [{ "title": "Bon" }, { "title": " " }] }),
    )
    .await;
    assert!(blank.contains("2") && blank.contains("titre"), "{blank}");
    let none = refused(&c, "split_ticket", json!({ "tickets": [] })).await;
    assert!(none.contains("au moins"), "{none}");
    let many: Vec<Value> = (0..11)
        .map(|i| json!({ "title": format!("T{i}") }))
        .collect();
    let flood = refused(&c, "split_ticket", json!({ "tickets": many })).await;
    assert!(flood.contains("10"), "{flood}");
    assert_eq!(h.core.tickets.read().len(), count);
    c.cancel().await.unwrap();

    // An agent without a ticket has nothing to split.
    let free = put_agent(&h, &p, "libre", 2);
    let c = as_agent(&h, &free.id).await;
    let nothing = refused(
        &c,
        "split_ticket",
        json!({ "tickets": [{ "title": "Un" }] }),
    )
    .await;
    assert!(
        nothing.contains("libre") && nothing.contains("pas de ticket"),
        "{nothing}"
    );
    assert_eq!(entry(&h).outcome, "refused");
    assert_eq!(h.core.tickets.read().len(), count);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_first_message_of_an_agent_created_by_an_agent_says_who_wrote_it_and_the_name_ignores_that(
) {
    let h = harness("mcp-act-origin");
    let (p, _) = h.project(false).await;
    let boss = put_agent(&h, &p, "chef", 1);
    let c = as_agent(&h, &boss.id).await;
    let v = read(
        &c,
        "create_agent",
        json!({ "project": "demo", "message": "Write the docs" }),
    )
    .await;
    let id = v["id"].as_str().unwrap().to_string();
    turn_over(&h, &id).await;
    assert!(h
        .items(&id)
        .iter()
        .any(|i| i["kind"] == "user" && i["text"] == "Message de chef : Write the docs"));
    assert!(h
        .items(&id)
        .iter()
        .any(|i| i["text"] == "Bonjour, tu as dit : Message de chef : Write the docs"));
    // Named after the task: the fake Claude names it after the first two words it is given, and
    // « Message de » is no part of the task.
    h.wait("its name", |h| h.agent(&id).named).await;
    assert_eq!(h.agent(&id).name, "write-the-fake");
    // A message to an existing agent from outside: the same, and a slash command is no command.
    let v = read(
        &c,
        "send_message",
        json!({ "agent": "write-the-fake", "text": "/compact" }),
    )
    .await;
    assert_eq!(v["id"], id.as_str());
    h.wait("the message", |h| {
        h.items(&id)
            .iter()
            .any(|i| i["kind"] == "user" && i["text"] == "Message de chef : /compact")
    })
    .await;
    turn_over(&h, &id).await;
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[test]
fn a_message_from_the_server_is_headed_by_its_author_in_the_language_of_the_interface() {
    use crate::core::from_origin;
    use crate::i18n::Lang::{En, Fr};
    assert_eq!(
        from_origin(Fr, "Claude (hors Escouade)", "Salut"),
        "Message de Claude (hors Escouade) : Salut"
    );
    assert_eq!(
        from_origin(En, "Claude (outside Escouade)", "Hi"),
        "Message from Claude (outside Escouade): Hi"
    );
    // The author is one line, whatever its name holds; the text is kept as it is.
    assert_eq!(
        from_origin(En, "agent\n1  b", "a\n\nb"),
        "Message from agent 1 b: a\n\nb"
    );
}

#[tokio::test]
async fn an_agent_acts_on_its_own_project_alone_and_reads_the_others() {
    let h = harness("mcp-act-own-project");
    let p = project(&h).await;
    let other = other_project(&h, "autre").await;
    let mine = put_agent(&h, &p, "mon-agent", 1);
    let theirs = put_agent(&h, &other, "leur-agent", 2);
    let t_mine = made(&h, &p, "Chez moi").await;
    let t_other = made(&h, &other, "Chez l’autre").await;
    let c = as_agent(&h, &mine.id).await;
    let (tickets, agents) = (h.core.tickets.read().len(), h.core.agents.read().len());

    for (tool, args) in [
        ("create_ticket", json!({ "project": "autre", "title": "x" })),
        (
            "update_ticket",
            json!({ "ticket": t_other.key, "title": "x" }),
        ),
        (
            "move_ticket",
            json!({ "ticket": t_other.key, "position": "top" }),
        ),
        ("start_ticket", json!({ "ticket": t_other.key })),
        (
            "create_agent",
            json!({ "project": other.id, "message": "x" }),
        ),
        ("send_message", json!({ "agent": theirs.id, "text": "x" })),
        ("stop_agent", json!({ "agent": theirs.id })),
    ] {
        let why = refused(&c, tool, args).await;
        assert!(
            why.contains("mon-agent")
                && why.contains("propre projet (demo)")
                && why.contains("le projet autre"),
            "{tool}: {why}"
        );
        let logged = entry(&h);
        assert_eq!(
            (logged.caller.as_str(), logged.tool.as_str(), logged.outcome),
            ("mon-agent", tool, "refused")
        );
    }
    // A ticket of another project is no ticket of its own to split: its agent's ticket is
    // another project's by mistake.
    let stray = put_agent(&h, &p, "egare", 3);
    h.core.agent(&stray.id).unwrap().lock().meta.ticket_id = Some(t_other.id.clone());
    let from_stray = as_agent(&h, &stray.id).await;
    let why = refused(
        &from_stray,
        "split_ticket",
        json!({ "tickets": [{ "title": "Un" }] }),
    )
    .await;
    assert!(why.contains("propre projet"), "{why}");
    from_stray.cancel().await.unwrap();
    // Nothing happened over there.
    assert_eq!(
        (h.core.tickets.read().len(), h.core.agents.read().len()),
        (tickets, agents + 1)
    );
    assert_eq!(stored(&h, &t_other.id).title, "Chez l’autre");
    assert!(!stored(&h, &t_other.id).forced);
    assert_eq!(h.agent(&theirs.id).prompts, 0);

    // It reads them all the same.
    let listed = read(&c, "list_tickets", json!({ "project": "autre" })).await;
    assert_eq!(keys(&listed), [t_other.key.as_str()]);
    let summary = read(&c, "get_agent_summary", json!({ "agent": theirs.id })).await;
    assert_eq!(summary["name"], "leur-agent");
    assert_eq!(
        read(&c, "get_ticket", json!({ "ticket": t_other.key })).await["title"],
        "Chez l’autre"
    );

    // And acts on its own.
    let made_here = read(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "Pour moi" }),
    )
    .await;
    assert_eq!(made_here["column"], "todo");
    read(
        &c,
        "update_ticket",
        json!({ "ticket": t_mine.key, "title": "Chez moi, revu" }),
    )
    .await;
    assert_eq!(stored(&h, &t_mine.id).title, "Chez moi, revu");
    c.cancel().await.unwrap();

    // Claude outside Escouade is the user's: it acts on any project.
    let outside = external(&h).await;
    read(
        &outside,
        "update_ticket",
        json!({ "ticket": t_other.key, "title": "Revu de l’extérieur" }),
    )
    .await;
    assert_eq!(stored(&h, &t_other.id).title, "Revu de l’extérieur");
    read(
        &outside,
        "create_ticket",
        json!({ "project": "autre", "title": "Aussi" }),
    )
    .await;
    outside.cancel().await.unwrap();
    h.core.mcp.stop();
}
