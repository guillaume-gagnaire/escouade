//! The tools that act, called by a real MCP client (rmcp's) on a test core: real git
//! repositories and the fake `claude`. Each does what the window does, under the guardrails of the
//! window and the autopilot; every call, refused ones included, is in the activity log.

use super::act::{board_change, board_changes};
use super::private_tests::make_private;
use super::tests::{ACTING, AGENTS_ONLY, EXPOSED};
use super::tools::{SetBoardArgs, ToolError};
use super::tools_tests::{
    as_agent, external, keys, last_entry, link, other_project, project, put, put_agent, read,
    refused, still, ticket, Client,
};
use crate::accounts::PRINCIPAL;
use crate::board::clock;
use crate::core::RESUME_MARGIN_MS;
use crate::core_tests::{agent_on, git, harness, repo, Harness};
use crate::model::*;
use crate::tickets::TicketDraft;
use crate::tickets_tests::{conflicting, draft, gated_tests, reviewed, wait_running};
use crate::usage::Reading;
use base64::Engine;
use serde_json::{json, Value};
use std::time::{Duration, Instant};

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
        // Only an approval (it may push and open a pull request) and the sharing of the docs (it
        // pushes) may reach beyond Escouade.
        assert_eq!(
            v["annotations"]["openWorldHint"],
            matches!(*name, "approve_ticket" | "share_docs"),
            "{name}: {v}"
        );
        // And of the board's flow, only an approval and a deletion cannot be taken back.
        let destructive = match *name {
            "approve_ticket" | "delete_ticket" | "stop_background_work" => Some(true),
            "reject_ticket" | "resume_ticket" | "resolve_conflict" | "dismiss_block"
            | "message_subagent" => Some(false),
            _ => None,
        };
        if let Some(destructive) = destructive {
            assert_eq!(
                v["annotations"]["destructiveHint"], destructive,
                "{name}: {v}"
            );
        }
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
    assert_eq!(required("set_board"), ["project"]);
    assert_eq!(required("create_agent"), ["message", "project"]);
    assert_eq!(required("send_message"), ["agent", "text"]);
    assert_eq!(required("stop_agent"), ["agent"]);
    assert_eq!(required("stop_background_work"), ["agent", "id"]);
    assert_eq!(required("message_subagent"), ["agent", "subagent", "text"]);
    assert_eq!(required("report_progress"), ["line"]);
    assert_eq!(required("split_ticket"), ["tickets"]);
    assert_eq!(required("reject_ticket"), ["comment", "ticket"]);
    for name in [
        "approve_ticket",
        "resume_ticket",
        "resolve_conflict",
        "dismiss_block",
        "delete_ticket",
    ] {
        assert_eq!(required(name), ["ticket"], "{name}");
    }
    let position = schema("move_ticket")["properties"]["position"].to_string();
    for p in ["top", "bottom", "before"] {
        assert!(position.contains(p), "{position}");
    }
    // The Kanban's settings, named as `get_board` answers them (it can be given back as it is),
    // the choices of the three that have a few written in the schema.
    let board = schema("set_board");
    let mut names: Vec<&str> = board["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "action",
            "autopilot",
            "cleanup",
            "conflict",
            "conventional",
            "draft",
            "effort",
            "maxParallel",
            "mode",
            "model",
            "project",
            "strategy",
            "target",
            "testsFirst"
        ]
    );
    for (arg, choices) in [
        ("action", &["merge", "pr", "push", "keep"][..]),
        ("strategy", &["merge", "squash", "rebase"]),
        ("conflict", &["ask", "agent", "abort"]),
    ] {
        let spec = board["properties"][arg].to_string();
        for choice in choices {
            assert!(spec.contains(choice), "{arg}: {spec}");
        }
    }
    // What a refusal comes of is said before the call: the mode that is the user's to give, the
    // test command it does not set.
    let (mode, tests) = (
        board["properties"]["mode"]["description"].as_str().unwrap(),
        board["properties"]["testsFirst"]["description"]
            .as_str()
            .unwrap(),
    );
    assert!(mode.contains("bypassPermissions"), "{mode}");
    assert!(tests.contains("test command"), "{tests}");
    // And the tool says it is for the next tickets' agents, not the ones already at work.
    let described = tools.iter().find(|t| t.name == "set_board").unwrap();
    let said = serde_json::to_value(described).unwrap()["description"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        said.contains("bypassPermissions") && said.contains("already"),
        "{said}"
    );
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
            "description": "Une page.\nAvec un titre.\n\nCréé par Claude (hors Escouade) via Escouade",
            "criteria": [
                { "text": "Le titre s’affiche", "ok": false, "note": "" },
                { "text": "Le lien marche", "ok": false, "note": "" },
                { "text": "Le test passe", "ok": false, "note": "" },
            ],
            "loops": { "iteration": 0, "max": 5 },
            "column": "todo",
            "after": [],
            "progress": [],
            "images": [],
            "files": [],
            // Its account, model and effort left to their default (ESC-153).
            "account": "auto",
            "model": "auto",
            "effort": "auto",
            "external": null,
            "agent": null,
            "branch": null,
            "worktree": null,
            "blocked": null,
            "conflict": false,
            "step": null,
            "outcome": null,
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
    // No description of its own: the origin line is all it has.
    assert_eq!(
        second["description"],
        "Créé par Claude (hors Escouade) via Escouade"
    );
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
async fn a_ticket_lists_its_images() {
    use crate::tickets::DraftImage;
    use base64::Engine;
    let h = harness("mcp-act-ticket-images");
    let p = project(&h).await;
    let image = |name: &str| DraftImage {
        name: name.into(),
        media_type: "image/png".into(),
        data: base64::engine::general_purpose::STANDARD.encode(b"\x89PNG\r\n\x1a\n-capture"),
        ..Default::default()
    };
    let t = h
        .core
        .ticket_create(
            &p.id,
            TicketDraft {
                title: "Avec des images".into(),
                images: Some(vec![image("a.png"), image("b.png")]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let plain = made(&h, &p, "Sans image").await;
    let c = external(&h).await;
    let got = read(&c, "get_ticket", json!({ "ticket": t.key })).await;
    let names: Vec<&str> = got["images"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["a.png", "b.png"], "{got}");
    // The tools that change a ticket answer with it as it is, images kept.
    let updated = read(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "title": "Avec deux images" }),
    )
    .await;
    assert_eq!(updated["images"], got["images"], "{updated}");
    let none = read(&c, "get_ticket", json!({ "ticket": plain.key })).await;
    assert_eq!(none["images"], json!([]), "{none}");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_lists_its_files_and_the_tools_that_change_it_keep_them() {
    use crate::tickets::DraftFile;
    let h = harness("mcp-act-ticket-files");
    let p = project(&h).await;
    let joined = |name: &str| {
        let copy = crate::attachfiles_tests::copy(&h, "draft", name, b"zip");
        DraftFile {
            name: copy.name,
            path: copy.path,
            ..Default::default()
        }
    };
    let t = h
        .core
        .ticket_create(
            &p.id,
            TicketDraft {
                title: "Avec des fichiers".into(),
                files: Some(vec![joined("a.zip"), joined("b.xlsx"), joined("c.pdf")]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let plain = made(&h, &p, "Sans fichier").await;
    let c = external(&h).await;
    let got = read(&c, "get_ticket", json!({ "ticket": t.key })).await;
    let files: Vec<Value> = t
        .files
        .iter()
        .enumerate()
        .map(|(index, f)| json!({ "index": index, "file": f.file, "name": f.name, "size": 3 }))
        .collect();
    assert_eq!(
        (&got["files"], &got["images"]),
        (&json!(files), &json!([])),
        "{got}"
    );
    // The tools that change a ticket answer with it as it is, files kept.
    let updated = read(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "title": "Avec trois fichiers" }),
    )
    .await;
    assert_eq!(updated["files"], json!(files), "{updated}");
    assert_eq!(h.core.ticket(&t.id).unwrap().files.len(), 3);
    let none = read(&c, "get_ticket", json!({ "ticket": plain.key })).await;
    assert_eq!(none["files"], json!([]), "{none}");
    // Made through the tools without files, a ticket has none.
    let made_by_tool = read(
        &c,
        "create_ticket",
        json!({ "project": p.id, "title": "Par l’outil" }),
    )
    .await;
    assert_eq!(made_by_tool["files"], json!([]), "{made_by_tool}");
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
    assert_eq!(
        v["description"],
        "Plus de détails\n\nModifié par Claude (hors Escouade) via Escouade"
    );
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

    // The branch the window gave the ticket (« Reprendre une branche existante… ») is not the
    // tools' to change: editing another field keeps it.
    h.core
        .edit_ticket(&b.id, |t| {
            t.branch = "feat/login".into();
            Ok(())
        })
        .unwrap();
    read(
        &c,
        "update_ticket",
        json!({ "ticket": "DEM-2", "title": "Deuxième, encore revu" }),
    )
    .await;
    assert_eq!(stored(&h, &b.id).branch, "feat/login");

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
async fn pause_past_the_quota_holds_back_the_autopilot_alone_never_a_ticket_started_by_hand() {
    let h = harness("mcp-act-start-past-threshold");
    let p = project(&h).await;
    let a = made(&h, &p, "Malgré le seuil [ok]").await;
    // The weekly window past « Pause au-delà du quota »: the autopilot is paused. Set before the
    // client connects: saving the settings starts the server again.
    h.set_settings(|s| s.quota_pause = 95);
    let end = h.core.pause_now() + 3_600_000;
    h.core.record_usage(
        PRINCIPAL,
        Reading::claude((
            None,
            Some(RateWindow {
                pct: 97.0,
                resets_at: Some(end),
            }),
        )),
    );
    assert!(h.core.autopilot_pause().is_some());
    assert!(h.core.project_hold(&p.id, EngineId::Claude).is_some());
    // Started by hand, it goes (the user, 2026-10-28): the scheduler gives it an agent.
    assert!(h.core.project_limit(&p.id, EngineId::Claude).is_none());
    let c = external(&h).await;
    let v = read(&c, "start_ticket", json!({ "ticket": a.key })).await;
    assert_eq!(v["key"], a.key.as_str());
    h.wait("the ticket to start past the threshold", |h| {
        stored(h, &a.id).column != Column::Todo
    })
    .await;
    // A window used up holds it back, by hand too.
    let b = made(&h, &p, "À la limite").await;
    h.core.record_usage(
        PRINCIPAL,
        Reading::claude((
            None,
            Some(RateWindow {
                pct: 100.0,
                resets_at: Some(end),
            }),
        )),
    );
    let limit = refused(&c, "start_ticket", json!({ "ticket": b.key })).await;
    assert!(limit.contains("100 %"), "{limit}");
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
    h.core.pause_after_limit(PRINCIPAL);
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
    h.core.record_usage(
        PRINCIPAL,
        Reading::claude((
            Some(RateWindow {
                pct: 100.0,
                resets_at: Some(end),
            }),
            None,
        )),
    );
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
    // Made as the window makes one, but the sidebar's selection is the user's. Its first message
    // names it, in the background: its name is waited for, then read as it is.
    h.wait("its name", |h| h.agent(&id).named).await;
    let meta = h.agent(&id);
    assert_eq!(
        (meta.project_id.as_str(), meta.name.as_str()),
        (p.id.as_str(), "bonjour-l-fake")
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
        ..Default::default()
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
        json!({ "project": p.id, "message": "x", "model": "inconnu" }),
    )
    .await;
    assert!(
        unknown.contains("inconnu") && unknown.contains("sonnet"),
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

    h.core.pause_after_limit(PRINCIPAL);
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
    // Its first message has it named (by the fake claude, in the background): the name it keeps, before it is listed.
    h.wait("its name", |h| h.agent(&a.id).named).await;
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
    h.core.pause_after_limit(PRINCIPAL);
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
        unknown.contains("Aucun agent « zzz »") && unknown.contains(&h.agent(&a.id).name),
        "{unknown}"
    );
    assert_eq!(h.agent(&a.id).prompts, prompts);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_message_is_32000_characters_at_most_counted_as_characters_and_never_given_back() {
    let h = harness("mcp-act-send-message-bound");
    let (p, _) = h.project(false).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    let c = external(&h).await;
    // The end of the words, far past what the log keeps of a call: it is where a leak shows.
    let end = "FIN-DU-MESSAGE-107";
    let words = |n: usize| format!("{}{end}", "é".repeat(n - end.chars().count()));

    // 32,000 characters of two bytes each go through, the whitespace around them no part of them.
    let v = read(
        &c,
        "send_message",
        json!({ "agent": a.id, "text": format!("  {}\n ", words(32_000)) }),
    )
    .await;
    assert_eq!(v["id"], a.id.as_str());
    turn_over(&h, &a.id).await;
    let sent = format!("Message de Claude (hors Escouade) : {}", words(32_000));
    assert!(h
        .items(&a.id)
        .iter()
        .any(|i| i["kind"] == "user" && i["text"] == sent.as_str()));
    let prompts = h.agent(&a.id).prompts;
    let user_items = || {
        h.items(&a.id)
            .iter()
            .filter(|i| i["kind"] == "user")
            .count()
    };
    let said_by_user = user_items();

    // One more is told in characters, and the words do not come back anywhere.
    let said = refused(
        &c,
        "send_message",
        json!({ "agent": a.id, "text": words(32_001) }),
    )
    .await;
    assert_eq!(said, "Le message fait 32001 caractères : 32000 au plus.");
    let logged = entry(&h);
    assert_eq!(
        (
            logged.tool.as_str(),
            logged.outcome,
            logged.message.as_deref()
        ),
        ("send_message", "error", Some(said.as_str()))
    );
    // The log keeps the start of the words of a call, as it does for any: not their end.
    assert!(logged.summary.chars().count() < 400, "{}", logged.summary);
    let log = serde_json::to_string(&h.core.mcp.activity.entries()).unwrap();
    let told = h
        .events
        .lock()
        .iter()
        .filter(|e| e["type"] == "mcpActivity")
        .map(|e| e.to_string())
        .collect::<String>();
    assert!(!said.contains(end) && !log.contains(end) && !told.contains(end));
    // Nothing was sent: no turn, and the conversation has not got it.
    assert_eq!(h.agent(&a.id).prompts, prompts);
    assert_eq!(user_items(), said_by_user);
    // A blank text is still told as it was, and an enormous one as well (counted, not echoed).
    let blank = refused(&c, "send_message", json!({ "agent": a.id, "text": "  " })).await;
    assert_eq!(blank, "Écris le message à envoyer.");
    let huge = refused(
        &c,
        "send_message",
        json!({ "agent": a.id, "text": words(200_000) }),
    )
    .await;
    assert_eq!(huge, "Le message fait 200000 caractères : 32000 au plus.");
    assert_eq!(h.agent(&a.id).prompts, prompts);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_running_turn_is_interrupted_by_another_and_never_by_the_agent_itself() {
    let h = harness("mcp-act-stop-agent");
    let (p, _) = h.project(false).await;
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
        (
            Column::Todo,
            "Étape 1",
            "D’abord\n\nCréé par gros via Escouade"
        )
    );
    // Each one says who split the ticket, with a description or without.
    let second_step = h
        .core
        .tickets
        .read()
        .iter()
        .find(|t| t.key == "DEM-3")
        .cloned()
        .unwrap();
    assert_eq!(second_step.description, "Créé par gros via Escouade");
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

#[test]
fn a_ticket_made_through_the_server_ends_with_who_made_it_in_the_language_of_the_interface() {
    use super::act::{made_by, with_origin};
    use crate::i18n::Lang::{En, Fr};
    assert_eq!(made_by(Fr, "chef"), "Créé par chef via Escouade");
    assert_eq!(
        made_by(En, "Claude (outside Escouade)"),
        "Created by Claude (outside Escouade) through Escouade"
    );
    // The origin is one line, whatever its name holds.
    assert_eq!(
        made_by(En, "agent\n1  b"),
        "Created by agent 1 b through Escouade"
    );
    // After the description, a blank line between; alone when there is no description.
    assert_eq!(with_origin("", "L"), "L");
    assert_eq!(with_origin("D\nE", "L"), "D\nE\n\nL");
}

#[tokio::test]
async fn a_ticket_made_by_an_agent_names_it_and_the_line_is_no_part_of_the_descriptions_limit() {
    let h = harness("mcp-act-origin-line");
    let p = project(&h).await;
    let a = put_agent(&h, &p, "chef", 1);
    let c = as_agent(&h, &a.id).await;
    // The limit is on what the caller wrote: 10,000 characters still make a ticket.
    let long = "a".repeat(10_000);
    let made = read(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "Long", "description": long }),
    )
    .await;
    // (Compared as a boolean: a failure would print 10,000 characters.)
    assert!(
        made["description"] == format!("{long}\n\nCréé par chef via Escouade"),
        "the description does not end with the origin line"
    );
    let too_long = refused(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "Trop", "description": "a".repeat(10_001) }),
    )
    .await;
    assert!(
        too_long.contains("10000") || too_long.contains("10 000"),
        "{too_long}"
    );
    // Editing the description: the caller's new words, then who edited it (the limit is on the
    // words alone).
    let edited = read(
        &c,
        "update_ticket",
        json!({ "ticket": made["key"], "description": "Autre" }),
    )
    .await;
    assert_eq!(
        edited["description"],
        "Autre\n\nModifié par chef via Escouade"
    );
    let at_limit = read(
        &c,
        "update_ticket",
        json!({ "ticket": made["key"], "description": "b".repeat(10_000) }),
    )
    .await;
    assert!(
        at_limit["description"]
            == format!("{}\n\nModifié par chef via Escouade", "b".repeat(10_000)),
        "the edited description does not end with the origin line"
    );
    let too_long = refused(
        &c,
        "update_ticket",
        json!({ "ticket": made["key"], "description": "b".repeat(10_001) }),
    )
    .await;
    assert!(
        too_long.contains("10000") || too_long.contains("10 000"),
        "{too_long}"
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[test]
fn an_edit_line_replaces_the_origin_line_of_the_description_in_either_language() {
    use super::act::{changed_by, without_origin};
    use crate::i18n::Lang::{En, Fr};
    assert_eq!(changed_by(Fr, "chef"), "Modifié par chef via Escouade");
    assert_eq!(
        changed_by(En, "Claude (outside Escouade)"),
        "Modified by Claude (outside Escouade) through Escouade"
    );
    assert_eq!(
        changed_by(En, "agent\n1  b"),
        "Modified by agent 1 b through Escouade"
    );
    // The line, and the blank line before it, go: whoever made or edited it, in either language.
    assert_eq!(without_origin("D\nE\n\nCréé par chef via Escouade"), "D\nE");
    assert_eq!(without_origin("D\n\nModifié par chef via Escouade"), "D");
    assert_eq!(without_origin("D\n\nCreated by a b through Escouade"), "D");
    assert_eq!(without_origin("D\n\nModified by a through Escouade\n"), "D");
    assert_eq!(without_origin("Créé par chef via Escouade"), "");
    // Only the line the server adds: a sentence of the user's that looks like it, or a line not
    // set apart by a blank line, stays.
    assert_eq!(
        without_origin("D\nCréé par chef via Escouade"),
        "D\nCréé par chef via Escouade"
    );
    assert_eq!(
        without_origin("D\n\nCréé par chef via Escouade, puis revu"),
        "D\n\nCréé par chef via Escouade, puis revu"
    );
    assert_eq!(without_origin("Du texte"), "Du texte");
    assert_eq!(without_origin(""), "");
}

#[tokio::test]
async fn a_ticket_edited_through_the_server_says_who_edited_it_without_stacking_the_lines() {
    let h = harness("mcp-act-edit-origin");
    let p = project(&h).await;
    let a = put_agent(&h, &p, "chef", 1);
    let c = as_agent(&h, &a.id).await;
    let first = read(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "Page", "description": "Une page." }),
    )
    .await;
    assert_eq!(
        first["description"],
        "Une page.\n\nCréé par chef via Escouade"
    );
    // A new title alone: the description is kept, the « Créé par » line becomes the edit's.
    let titled = read(
        &c,
        "update_ticket",
        json!({ "ticket": first["key"], "title": "Page d’accueil" }),
    )
    .await;
    assert_eq!(
        titled["description"],
        "Une page.\n\nModifié par chef via Escouade"
    );
    // New criteria alone, then another title: the same line, never two.
    let criteria = read(
        &c,
        "update_ticket",
        json!({ "ticket": first["key"], "criteria": ["Un seul"] }),
    )
    .await;
    assert_eq!(
        criteria["description"],
        "Une page.\n\nModifié par chef via Escouade"
    );
    // The edit by someone else is theirs: Claude outside Escouade takes the line over.
    let outside = external(&h).await;
    let again = read(
        &outside,
        "update_ticket",
        json!({ "ticket": first["key"], "title": "Encore" }),
    )
    .await;
    assert_eq!(
        again["description"],
        "Une page.\n\nModifié par Claude (hors Escouade) via Escouade"
    );
    // A description given back as `get_ticket` answered it, its line included: the line is
    // replaced, not kept under the new one.
    let echoed = read(
        &c,
        "update_ticket",
        json!({
            "ticket": first["key"],
            "description": "Une page revue.\n\nModifié par Claude (hors Escouade) via Escouade"
        }),
    )
    .await;
    assert_eq!(
        echoed["description"],
        "Une page revue.\n\nModifié par chef via Escouade"
    );
    // Only the dependencies: no edit of the words, so the line stays as it was.
    let other = read(
        &outside,
        "create_ticket",
        json!({ "project": "demo", "title": "Autre" }),
    )
    .await;
    let after = read(
        &outside,
        "update_ticket",
        json!({ "ticket": other["key"], "after": [first["key"]] }),
    )
    .await;
    assert_eq!(
        after["description"],
        "Créé par Claude (hors Escouade) via Escouade"
    );
    // A ticket the window made has no line yet: an edit through the server adds one.
    let mine = made(&h, &p, "De la fenêtre").await;
    let edited = read(
        &outside,
        "update_ticket",
        json!({ "ticket": mine.key, "title": "Revu" }),
    )
    .await;
    assert_eq!(
        edited["description"],
        "Modifié par Claude (hors Escouade) via Escouade"
    );
    // What is stored is what was answered.
    assert_eq!(
        stored(&h, &mine.id).description,
        "Modifié par Claude (hors Escouade) via Escouade"
    );
    outside.cancel().await.unwrap();
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agent_acts_on_another_project_that_is_not_private_as_on_its_own() {
    // Cédric's case (Discord): the agent of his back end makes, edits and starts a ticket of his
    // front end, another project of Escouade, and writes to its agent.
    let h = harness("mcp-act-other-project");
    let p = project(&h).await;
    // A repository of its own, which a ticket started there can branch from.
    let other = h
        .core
        .create_project(
            &repo(&h.dir.join("autre")).to_string_lossy(),
            "autre",
            "oklch(0.7 0.1 200)",
            false,
            None,
            true,
        )
        .await
        .unwrap();
    let other = still(&h, &other);
    let mine = put_agent(&h, &p, "agent-root", 1);
    let theirs = h
        .core
        .create_agent_with(
            &other.id,
            crate::core::AgentOptions {
                name: Some("leur-agent".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta;
    let t_other = made(&h, &other, "Chez l’autre").await;
    let c = as_agent(&h, &mine.id).await;
    let logged_ok = |h: &Harness, tool: &str| {
        let logged = entry(h);
        assert_eq!(
            (logged.caller.as_str(), logged.tool.as_str(), logged.outcome),
            ("agent-root", tool, "ok")
        );
    };

    // It makes a ticket there.
    let v = read(
        &c,
        "create_ticket",
        json!({ "project": "autre", "title": "Pour le front" }),
    )
    .await;
    logged_ok(&h, "create_ticket");
    let created = stored(&h, v["id"].as_str().unwrap());
    assert_eq!(
        (created.project_id.as_str(), created.title.as_str()),
        (other.id.as_str(), "Pour le front")
    );
    assert!(created.description.contains("agent-root"));
    // Edits it, and puts it first.
    read(
        &c,
        "update_ticket",
        json!({ "ticket": created.key, "title": "Pour le front, revu" }),
    )
    .await;
    logged_ok(&h, "update_ticket");
    assert_eq!(stored(&h, &created.id).title, "Pour le front, revu");
    read(
        &c,
        "move_ticket",
        json!({ "ticket": created.key, "position": "top" }),
    )
    .await;
    assert_eq!(
        todo(&c, &other).await,
        [created.key.as_str(), t_other.key.as_str()]
    );
    // Starts it: the scheduler gives it an agent of that project.
    read(&c, "start_ticket", json!({ "ticket": created.key })).await;
    logged_ok(&h, "start_ticket");
    h.wait("the ticket to start", |h| {
        stored(h, &created.id).column != Column::Todo
    })
    .await;
    assert!(h.events.lock().iter().any(|e| e["type"] == "ticket"
        && e["ticket"]["id"] == created.id.as_str()
        && e["ticket"]["forced"] == true));
    h.wait("its agent to be made", |h| {
        stored(h, &created.id).agent_id.is_some()
    })
    .await;
    let runner = stored(&h, &created.id).agent_id.unwrap();
    assert_eq!(h.agent(&runner).project_id, other.id);
    // Writes to its agent, under its own name.
    read(
        &c,
        "send_message",
        json!({ "agent": "leur-agent", "project": "autre", "text": "Le contrat d’API change" }),
    )
    .await;
    logged_ok(&h, "send_message");
    turn_over(&h, &theirs.id).await;
    assert!(h
        .items(&theirs.id)
        .iter()
        .any(|i| i["kind"] == "user"
            && i["text"] == "Message de agent-root : Le contrat d’API change"));
    // Interrupts it (it is at rest: nothing to interrupt) and makes an agent there.
    let v = read(&c, "stop_agent", json!({ "agent": theirs.id })).await;
    assert_eq!(v["interrupted"], false);
    let v = read(
        &c,
        "create_agent",
        json!({ "project": other.id, "message": "Regarde le front" }),
    )
    .await;
    let made_there = v["id"].as_str().unwrap().to_string();
    assert_eq!(h.agent(&made_there).project_id, other.id);
    turn_over(&h, &made_there).await;
    h.wait("the started ticket's turn to end", |h| {
        stored(h, &created.id).column == Column::Review
    })
    .await;
    c.cancel().await.unwrap();

    // A project made private is no longer found, and nothing changes in it.
    let other = make_private(&h, &other);
    let c = as_agent(&h, &mine.id).await;
    let (tickets, agents) = (h.core.tickets.read().len(), h.core.agents.read().len());
    let prompts = h.agent(&theirs.id).prompts;
    for (tool, args) in [
        (
            "create_ticket",
            json!({ "project": other.id, "title": "x" }),
        ),
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
        assert!(why.starts_with("Aucun "), "{tool}: {why}");
        assert!(
            !why.contains("autre") && !why.contains("leur-agent"),
            "{tool}: {why}"
        );
        assert_eq!(entry(&h).outcome, "error", "{tool}");
    }
    assert_eq!(
        (h.core.tickets.read().len(), h.core.agents.read().len()),
        (tickets, agents)
    );
    assert_eq!(stored(&h, &t_other.id).title, "Chez l’autre");
    assert!(!stored(&h, &t_other.id).forced);
    assert_eq!(h.agent(&theirs.id).prompts, prompts);
    // An agent of its own project whose ticket is there does not split it from another project.
    let stray = put_agent(&h, &p, "egare", 3);
    h.core.agent(&stray.id).unwrap().lock().meta.ticket_id = Some(t_other.id.clone());
    let from_stray = as_agent(&h, &stray.id).await;
    let why = refused(
        &from_stray,
        "split_ticket",
        json!({ "tickets": [{ "title": "Un" }] }),
    )
    .await;
    assert!(why.contains("egare") && !why.contains("autre"), "{why}");
    assert_eq!(h.core.tickets.read().len(), tickets);
    from_stray.cancel().await.unwrap();
    c.cancel().await.unwrap();

    // Claude outside Escouade is the user's: it acts on any project, private ones too.
    let outside = external(&h).await;
    read(
        &outside,
        "update_ticket",
        json!({ "ticket": t_other.key, "title": "Revu de l’extérieur" }),
    )
    .await;
    assert_eq!(stored(&h, &t_other.id).title, "Revu de l’extérieur");
    outside.cancel().await.unwrap();
    h.core.mcp.stop();
}

// ---------- the accounts: held per account, not for all ----------

/// A second account, Pro, with a folder of its own (the fake `claude` is launched with it). Not
/// through `save_settings`: it looks again at whether the server is wanted, as saving a project's
/// settings (`prefer`) does: so the server is wanted.
fn with_pro(h: &Harness) {
    let dir = h.dir.join("claude-pro");
    std::fs::create_dir_all(&dir).unwrap();
    let mut s = h.core.settings.write();
    s.mcp_enabled = true;
    s.accounts.push(Account {
        id: "pro".into(),
        name: "Pro".into(),
        config_dir: dir.to_string_lossy().to_string(),
        ..Default::default()
    });
}

/// The account's 5-hour window used `pct` percent, until `end`.
fn window_used(h: &Harness, account: &str, pct: f64, end: i64) {
    h.core.record_usage(
        account,
        Reading::claude((
            Some(RateWindow {
                pct,
                resets_at: Some(end),
            }),
            None,
        )),
    );
}

/// The account's 5-hour window used up, until `end`: it holds back what spends its quota.
fn used_up(h: &Harness, account: &str, end: i64) {
    window_used(h, account, 100.0, end);
}

/// The project asks for `account` (empty: any).
fn prefer(h: &Harness, p: &Project, account: &str) {
    let mut project = h.core.project(&p.id).unwrap();
    project.account = account.into();
    h.core.update_project(project).unwrap();
}

#[tokio::test]
async fn a_ticket_is_started_while_an_account_is_free_for_its_project_and_refused_when_none_is() {
    let h = harness("mcp-act-start-accounts");
    let p = project(&h).await;
    with_pro(&h);
    let a = made(&h, &p, "Un [ok]").await;
    let b = made(&h, &p, "Deux [ok]").await;
    let c = external(&h).await;
    let end = h.core.pause_now() + 3_600_000;

    // Principal is past the threshold, Pro is not: the ticket starts, and goes to Pro.
    used_up(&h, "principal", end);
    assert!(h.core.autopilot_pause().is_none());
    let v = read(&c, "start_ticket", json!({ "ticket": a.key })).await;
    assert_eq!(v["key"], "DEM-1");
    h.wait("its agent to be made", |h| {
        stored(h, &a.id).agent_id.is_some()
    })
    .await;
    let agent = stored(&h, &a.id).agent_id.unwrap();
    assert_eq!(h.agent(&agent).account, "pro");
    h.wait("its turn to end", |h| {
        stored(h, &a.id).column == Column::Review
    })
    .await;

    // A project that prefers Principal has no other account to go to: refused, until Principal's
    // window ends.
    prefer(&h, &p, "principal");
    let waits = refused(&c, "start_ticket", json!({ "ticket": b.key })).await;
    assert!(
        waits.contains("pause")
            && waits.contains("5 h")
            && waits.contains("100 %")
            && waits.contains(&clock(end + RESUME_MARGIN_MS)),
        "{waits}"
    );
    assert!(!stored(&h, &b.id).forced);
    assert_eq!(entry(&h).outcome, "refused");
    // The one that prefers Pro does not wait.
    prefer(&h, &p, "pro");
    assert!(h.core.project_hold(&p.id, EngineId::Claude).is_none());
    prefer(&h, &p, "");

    // Pro past the threshold too, sooner: no account left, and the answer says when the first is
    // free again.
    used_up(&h, "pro", end - 600_000);
    let none = refused(&c, "start_ticket", json!({ "ticket": b.key })).await;
    assert!(
        none.contains("pause")
            && none.contains(&clock(end - 600_000 + RESUME_MARGIN_MS))
            && !none.contains(&clock(end + RESUME_MARGIN_MS)),
        "{none}"
    );
    assert!(!stored(&h, &b.id).forced);
    let logged = entry(&h);
    assert_eq!(
        (
            logged.tool.as_str(),
            logged.outcome,
            logged.message.as_deref()
        ),
        ("start_ticket", "refused", Some(none.as_str()))
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agent_is_created_while_an_account_is_free_for_its_project_and_refused_when_none_is() {
    let h = harness("mcp-act-create-agent-accounts");
    let (p, _) = h.project(false).await;
    with_pro(&h);
    let c = external(&h).await;
    let end = h.core.pause_now() + 3_600_000;

    // Principal is past the threshold, Pro is not: the agent is made, on Pro.
    used_up(&h, "principal", end);
    let v = read(
        &c,
        "create_agent",
        json!({ "project": "demo", "message": "Sur Pro" }),
    )
    .await;
    let id = v["id"].as_str().unwrap().to_string();
    assert_eq!(h.agent(&id).account, "pro");
    turn_over(&h, &id).await;

    // A project that prefers Principal has no other account to go to.
    prefer(&h, &p, "principal");
    let count = h.core.agents.read().len();
    let waits = refused(
        &c,
        "create_agent",
        json!({ "project": "demo", "message": "Sur Principal" }),
    )
    .await;
    assert!(
        waits.contains("pause")
            && waits.contains("5 h")
            && waits.contains(&clock(end + RESUME_MARGIN_MS)),
        "{waits}"
    );
    assert_eq!(h.core.agents.read().len(), count);
    assert_eq!(entry(&h).outcome, "refused");
    prefer(&h, &p, "");

    // Pro past the threshold too, sooner: no account left.
    used_up(&h, "pro", end - 600_000);
    let none = refused(
        &c,
        "create_agent",
        json!({ "project": "demo", "message": "Nulle part" }),
    )
    .await;
    assert!(
        none.contains("pause") && none.contains(&clock(end - 600_000 + RESUME_MARGIN_MS)),
        "{none}"
    );
    assert_eq!(h.core.agents.read().len(), count);

    // An agent that waits for Pro's quota holds Pro back only: once Principal is free again, a new
    // agent goes to it.
    window_used(&h, "principal", 10.0, end);
    h.core.agent(&id).unwrap().lock().meta.resume_at = Some(h.core.pause_now() + 7_200_000);
    let v = read(
        &c,
        "create_agent",
        json!({ "project": "demo", "message": "Sur Principal, libre" }),
    )
    .await;
    let next = v["id"].as_str().unwrap().to_string();
    assert_eq!(h.agent(&next).account, "principal");
    turn_over(&h, &next).await;
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_message_is_refused_when_the_account_of_the_agent_it_goes_to_is_held_and_only_then() {
    let h = harness("mcp-act-send-message-accounts");
    let (p, _) = h.project(false).await;
    with_pro(&h);
    let main = agent_on(&h, &p, "principal").await;
    let pro = agent_on(&h, &p, "pro").await;
    let c = external(&h).await;
    let end = h.core.pause_now() + 3_600_000;

    // Principal is past the threshold: its agent takes no message, Pro's does.
    used_up(&h, "principal", end);
    let prompts = h.agent(&main.id).prompts;
    let held = refused(
        &c,
        "send_message",
        json!({ "agent": main.id, "text": "Sur Principal" }),
    )
    .await;
    assert!(
        held.contains("pause")
            && held.contains("5 h")
            && held.contains(&clock(end + RESUME_MARGIN_MS))
            && held.contains("« Principal »")
            && held.contains(&main.name),
        "{held}"
    );
    assert_eq!(h.agent(&main.id).prompts, prompts);
    assert_eq!(entry(&h).outcome, "refused");
    let v = read(
        &c,
        "send_message",
        json!({ "agent": pro.id, "text": "Sur Pro" }),
    )
    .await;
    assert_eq!(v["id"], pro.id.as_str());
    turn_over(&h, &pro.id).await;

    // A usage limit met on Pro: its agent waits, Principal's goes on once it is free again.
    window_used(&h, "principal", 10.0, end);
    h.core.pause_after_limit("pro");
    let limited = refused(
        &c,
        "send_message",
        json!({ "agent": pro.id, "text": "Encore" }),
    )
    .await;
    assert!(
        limited.contains("limite d’usage") && limited.contains("« Pro »"),
        "{limited}"
    );
    read(
        &c,
        "send_message",
        json!({ "agent": main.id, "text": "Sur Principal, libre" }),
    )
    .await;
    turn_over(&h, &main.id).await;
    h.core.autopilot_resume();

    // An agent that waits for its quota holds its own account back, not the other.
    let resume = h.core.pause_now() + 7_200_000;
    h.core.agent(&pro.id).unwrap().lock().meta.resume_at = Some(resume);
    let waiting = refused(
        &c,
        "send_message",
        json!({ "agent": pro.id, "text": "Attends" }),
    )
    .await;
    assert!(
        waiting.contains("quota") && waiting.contains(&clock(resume)),
        "{waiting}"
    );
    read(
        &c,
        "send_message",
        json!({ "agent": main.id, "text": "Toujours libre" }),
    )
    .await;
    turn_over(&h, &main.id).await;
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

// ---------- the board's flow: approve, send back, resume, resolve, dismiss, delete ----------

/// A ticket of `p` in `column` with an agent of its own, as the app keeps them (no process): the
/// ticket and the agent name each other.
fn staffed(h: &Harness, p: &Project, key: &str, column: Column, at: i64) -> (Ticket, AgentMeta) {
    let t = put(h, ticket(p, key, key, column, at));
    let a = put_agent(h, p, &format!("agent-{}", key.to_lowercase()), at);
    link(h, &a.id, &t.id);
    (stored(h, &t.id), a)
}

/// `t` with its approval running (at its `step`).
fn approving(h: &Harness, t: &Ticket, step: &str) -> Ticket {
    h.core
        .edit_ticket(&t.id, |x| {
            x.step = Some(step.into());
            Ok(())
        })
        .unwrap();
    stored(h, &t.id)
}

/// What an action answered as the ticket's state before it: the one mark of its `since`, the
/// cursor `wait_ticket` takes.
fn before_of(answer: &Value) -> Value {
    let since = answer["since"].as_str().expect("an answer with a since");
    let marks: Value = serde_json::from_slice(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(since)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(marks.as_array().map(Vec::len), Some(1), "{marks}");
    marks[0].clone()
}

/// What `wait_ticket` tells of `key` from the `since` an action answered: at once, well before the
/// 20 seconds the wait is given.
async fn told_since(c: &Client, key: &str, since: &Value) -> Value {
    let began = Instant::now();
    let v = read(
        c,
        "wait_ticket",
        json!({ "tickets": [key], "timeout": 20, "since": since }),
    )
    .await;
    assert!(began.elapsed() < Duration::from_secs(10), "{v}");
    v
}

/// What `tool` turns `args` down with, as the activity log says it (`outcome`: « refused » for a
/// guard, « error » for something asked that does not fit), and that `left` is as it was.
async fn turned_down(
    h: &Harness,
    c: &Client,
    tool: &'static str,
    args: Value,
    outcome: &str,
    left: &Ticket,
) -> String {
    let why = refused(c, tool, args).await;
    let logged = entry(h);
    assert_eq!(
        (
            logged.tool.as_str(),
            logged.outcome,
            logged.message.as_deref()
        ),
        (tool, outcome, Some(why.as_str()))
    );
    assert_eq!(&stored(h, &left.id), left, "{tool}: {why}");
    why
}

const NOT_IN_REVIEW: &str = "Ce ticket n'est pas à tester.";
const APPROVING: &str = "La validation de ce ticket est déjà en cours.";
const NOT_BLOCKED: &str = "Ce ticket n’est pas bloqué.";
/// What the core says of a ticket that is not blocked when it is resumed: not the words of `dismiss_block`.
const RESUME_NOT_BLOCKED: &str = "Ce ticket n'est pas bloqué.";
const OWN_TICKET: &str = "Un agent n’agit pas sur son propre ticket avec cet outil : c’est à l’utilisateur ou à un autre client de le faire.";

#[tokio::test]
async fn the_tools_of_the_flow_refuse_an_agent_its_own_ticket_and_act_on_the_projects_that_are_not_private(
) {
    let h = harness("mcp-act-flow-agents");
    let p = project(&h).await;
    let other = other_project(&h, "autre").await;
    let vault = other_project(&h, "coffre").await;
    let vault = make_private(&h, &vault);
    let (own, agent) = staffed(&h, &p, "DEM-1", Column::Review, 1);
    let waiting = put(&h, ticket(&p, "DEM-9", "En attente", Column::Todo, 9));
    // Its record names the ticket though the ticket names another agent: it is the same ticket.
    let relieved = put_agent(&h, &p, "releve", 2);
    h.core.agent(&relieved.id).unwrap().lock().meta.ticket_id = Some(own.id.clone());
    let elsewhere = put_agent(&h, &other, "ailleurs", 3);
    let vaulted = put_agent(&h, &vault, "enferme", 4);
    let args = |tool: &str| match tool {
        "reject_ticket" => json!({ "ticket": own.key, "comment": "Pas bon" }),
        _ => json!({ "ticket": own.key }),
    };
    let on_waiting = |tool: &str| match tool {
        "reject_ticket" => json!({ "ticket": waiting.key, "comment": "Pas bon" }),
        _ => json!({ "ticket": waiting.key }),
    };
    for (tool, said) in [
        (
            "approve_ticket",
            "Un agent ne valide pas son propre ticket : il fusionnerait son travail sans relecture.",
        ),
        (
            "reject_ticket",
            "Un agent ne renvoie pas son propre ticket.",
        ),
        ("resume_ticket", OWN_TICKET),
        ("resolve_conflict", OWN_TICKET),
        ("dismiss_block", OWN_TICKET),
        ("delete_ticket", OWN_TICKET),
    ] {
        // Whatever state the ticket is in, the agent that works on it is refused first.
        for (who, caller) in [("its agent", &agent), ("the one its record names", &relieved)] {
            let c = as_agent(&h, &caller.id).await;
            let why = turned_down(&h, &c, tool, args(tool), "refused", &own).await;
            assert_eq!(why, said, "{tool}, {who}");
            assert_eq!(entry(&h).caller, caller.name, "{tool}, {who}");
            c.cancel().await.unwrap();
        }
        // An agent of a private project acts on no other: the ticket is not found, as one that
        // does not exist, and the refusal names no project.
        let c = as_agent(&h, &vaulted.id).await;
        let why = turned_down(&h, &c, tool, args(tool), "error", &own).await;
        assert!(why.starts_with("Aucun ticket « DEM-1 »"), "{tool}: {why}");
        assert!(!why.contains("coffre") && !why.contains("demo"), "{tool}: {why}");
        c.cancel().await.unwrap();
        // An agent of another project that is not private acts here as Claude outside Escouade
        // does: on a ticket to do, the window's own refusal (but for `delete_ticket`, below).
        if tool != "delete_ticket" {
            let c = external(&h).await;
            let said = refused(&c, tool, on_waiting(tool)).await;
            c.cancel().await.unwrap();
            let c = as_agent(&h, &elsewhere.id).await;
            let why = turned_down(&h, &c, tool, on_waiting(tool), "refused", &waiting).await;
            assert_eq!(why, said, "{tool}");
            c.cancel().await.unwrap();
        }
        // Claude outside Escouade acts on any (what it does is checked by each tool's own test):
        // an unknown ticket is told so, for every tool.
        let c = external(&h).await;
        let none = refused(&c, tool, json!({ "ticket": "ZZZ-9", "comment": "x" })).await;
        assert!(none.contains("Aucun ticket « ZZZ-9 »"), "{tool}: {none}");
        assert_eq!(entry(&h).outcome, "error", "{tool}");
        c.cancel().await.unwrap();
    }
    assert_eq!(stored(&h, &own.id), own);

    // And it acts there: it dismisses the block of a ticket and deletes another.
    let c = as_agent(&h, &elsewhere.id).await;
    h.core
        .edit_ticket(&own.id, |x| {
            x.blocked = Some("Bloqué".into());
            Ok(())
        })
        .unwrap();
    let v = read(&c, "dismiss_block", json!({ "ticket": own.key })).await;
    assert_eq!(v["key"], "DEM-1");
    assert_eq!(stored(&h, &own.id).blocked, None);
    read(&c, "delete_ticket", json!({ "ticket": waiting.key })).await;
    assert!(h.core.ticket(&waiting.id).is_err());
    assert_eq!(
        (entry(&h).caller.as_str(), entry(&h).outcome),
        ("ailleurs", "ok")
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_is_not_approved_where_the_window_does_not_offer_it() {
    let h = harness("mcp-act-approve-guards");
    let p = project(&h).await;
    let c = external(&h).await;
    let ask = |t: &Ticket| json!({ "ticket": t.key });
    // Not in « À tester ».
    for (i, column) in [Column::Todo, Column::Doing, Column::Done]
        .into_iter()
        .enumerate()
    {
        let t = put(
            &h,
            ticket(&p, &format!("DEM-{}", i + 1), "Ailleurs", column, 1),
        );
        let why = turned_down(&h, &c, "approve_ticket", ask(&t), "refused", &t).await;
        assert_eq!(why, NOT_IN_REVIEW, "{column:?}");
    }
    // Its agent works: its files would be committed mid-write.
    let (busy, agent) = staffed(&h, &p, "DEM-4", Column::Review, 4);
    h.core.agent(&agent.id).unwrap().lock().meta.status = AgentStatus::Running;
    let why = turned_down(&h, &c, "approve_ticket", ask(&busy), "refused", &busy).await;
    assert_eq!(
        why,
        "L'agent de ce ticket travaille encore : attends la fin de son tour pour valider."
    );
    // Its approval already runs.
    let (running, _) = staffed(&h, &p, "DEM-5", Column::Review, 5);
    let running = approving(&h, &running, "Tests…");
    let why = turned_down(&h, &c, "approve_ticket", ask(&running), "refused", &running).await;
    assert_eq!(why, APPROVING);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_is_approved_in_the_background_merged_and_its_merge_commit_kept() {
    let h = harness("mcp-act-approve");
    let (p, r) = h.project(false).await;
    // Held at its tests, to see the approval under way.
    let go = h.dir.join("go");
    h.set_board(&p.id, |s| {
        s.tests_first = true;
        s.test_command = gated_tests(&go, 0);
    });
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    let agent = t.agent_id.clone().unwrap();
    let c = external(&h).await;
    let seen = ticket_events(&h, &t.id);
    let v = read(&c, "approve_ticket", json!({ "ticket": "dem-1" })).await;
    // It answers while the approval runs: the ticket as `get_ticket` gives it, not yet merged.
    assert_eq!(
        (v["key"].as_str(), v["column"].as_str()),
        (Some("DEM-1"), Some("review"))
    );
    let logged = entry(&h);
    assert_eq!(
        (
            logged.caller.as_str(),
            logged.tool.as_str(),
            logged.summary.as_str(),
            logged.outcome
        ),
        (
            "Claude (hors Escouade)",
            "approve_ticket",
            "ticket: dem-1",
            "ok"
        )
    );
    h.wait_ticket(&t.id, "its tests to run", |t| {
        t.step.as_deref() == Some("Tests…")
    })
    .await;
    // The window was told of it, as of an approval made there.
    assert!(ticket_events(&h, &t.id) > seen);
    // A second approval meanwhile is refused as the window's.
    let again = refused(&c, "approve_ticket", json!({ "ticket": t.key })).await;
    assert_eq!(again, APPROVING);
    assert_eq!(entry(&h).outcome, "refused");
    // The tests pass: the rest goes by itself, as for the window's approval.
    std::fs::write(&go, "").unwrap();
    h.wait_ticket(&t.id, "the ticket to be done", |t| t.column == Column::Done)
        .await;
    let done = stored(&h, &t.id);
    assert_eq!(done.outcome.as_deref(), Some("⤵ Mergé dans main · squash"));
    assert_eq!(
        (done.step.as_deref(), done.blocked.as_deref()),
        (None, None)
    );
    // Its merge commit is kept: the target's head.
    let head = git(&r, &["rev-parse", "refs/heads/main"]);
    assert_eq!(done.outcome_commit.as_deref(), Some(head.as_str()));
    // A client reads it with `get_ticket`, with the outcome's text.
    let read_back = read(&c, "get_ticket", json!({ "ticket": t.key })).await;
    assert_eq!(
        read_back["outcome"],
        json!({ "text": "⤵ Mergé dans main · squash", "url": null, "commit": head })
    );
    assert_eq!(git(&r, &["rev-list", "--count", "HEAD"]), "2");
    assert_eq!(
        std::fs::read_to_string(r.join("dem-1.txt")).unwrap(),
        "Boucle 1\n"
    );
    // Cleaned up: its agent archived, its worktree gone. The ticket is done before its cleanup ends
    // (`Core::approve_run`), which goes on in the background: waited for, not looked at once.
    h.wait("its agent archived and its worktree removed", |h| {
        h.agent(&agent).archived && !wt.exists()
    })
    .await;
    assert!(h.events.lock().iter().any(|e| e["type"] == "ticket"
        && e["ticket"]["id"] == t.id.as_str()
        && e["ticket"]["column"] == "done"));
    // Done: nothing more to approve.
    let late = refused(&c, "approve_ticket", json!({ "ticket": t.key })).await;
    assert_eq!(late, NOT_IN_REVIEW);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_approval_made_through_the_server_is_counted_in_the_usage_statistics_as_the_windows_is()
{
    let h = harness("mcp-act-approve-counted");
    let (p, _) = h.project(false).await;
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    // Counted from now on: saying yes counts the start.
    h.core.set_telemetry(true).unwrap();
    let counted = || -> Vec<Value> {
        let events = h.core.telemetry.pending();
        events.into_iter().map(|e| e["eventName"].clone()).collect()
    };
    assert_eq!(counted(), [json!("app_started")]);
    let c = external(&h).await;
    // Refused, as the window's would be: nothing happened, nothing is counted.
    h.core
        .edit_ticket(&t.id, |x| {
            x.column = Column::Doing;
            Ok(())
        })
        .unwrap();
    refused(&c, "approve_ticket", json!({ "ticket": t.key })).await;
    assert_eq!(counted(), [json!("app_started")]);
    h.core
        .edit_ticket(&t.id, |x| {
            x.column = Column::Review;
            Ok(())
        })
        .unwrap();
    read(&c, "approve_ticket", json!({ "ticket": t.key })).await;
    assert_eq!(counted(), [json!("app_started"), json!("ticket_approved")]);
    h.wait_ticket(&t.id, "the ticket to be done", |t| t.column == Column::Done)
        .await;
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_whose_branch_was_merged_already_is_closed_without_merging_again() {
    let h = harness("mcp-act-approve-nothing");
    let (p, r) = h.project(false).await;
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok] [commite]").await;
    // Merged by hand: squashed on the target, then its branch taken back to the target.
    git(&r, &["merge", "--squash", "ticket/dem-1"]);
    git(&r, &["commit", "-qm", "À la main"]);
    git(&wt, &["reset", "--keep", "main"]);
    let head = git(&r, &["rev-parse", "refs/heads/main"]);
    let commits = git(&r, &["rev-list", "--count", "main"]);
    let c = external(&h).await;
    read(&c, "approve_ticket", json!({ "ticket": t.key })).await;
    h.wait_ticket(&t.id, "the ticket to be done", |t| t.column == Column::Done)
        .await;
    let done = stored(&h, &t.id);
    assert_eq!(done.outcome.as_deref(), Some("∅ Aucune modification"));
    // Nothing merged: the target is as it was, and no merge commit is kept.
    assert_eq!(git(&r, &["rev-parse", "refs/heads/main"]), head);
    assert_eq!(git(&r, &["rev-list", "--count", "main"]), commits);
    assert_eq!(done.outcome_commit, None);
    // Its agent is archived once the ticket is done, by the approval that goes on in the background.
    let agent = done.agent_id.clone().unwrap();
    h.wait("its agent archived", |h| h.agent(&agent).archived)
        .await;
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_fast_approval_is_told_by_the_since_its_answer_gave_when_the_wait_comes_after_its_end() {
    let h = harness("mcp-act-approve-since");
    let (p, r) = h.project(false).await;
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok] [commite]").await;
    // Nothing to merge: the approval ends at once, before a wait could take its baseline.
    git(&r, &["merge", "--squash", "ticket/dem-1"]);
    git(&r, &["commit", "-qm", "À la main"]);
    git(&wt, &["reset", "--keep", "main"]);
    let c = external(&h).await;
    let v = read(&c, "approve_ticket", json!({ "ticket": t.key })).await;
    // The ticket as it was before the approval: in review, its agent at rest.
    let before = before_of(&v);
    assert_eq!(
        (&before["id"], &before["column"], &before["resting"]),
        (&json!(t.id), &json!("review"), &json!(true))
    );
    h.wait_ticket(&t.id, "the ticket to be done", |t| t.column == Column::Done)
        .await;
    // A wait that begins now would see a ticket done and nothing more: with the answer's since it
    // is told at once that the ticket left « À tester ».
    let told = told_since(&c, &t.key, &v["since"]).await;
    assert_eq!(told["changed"], json!(true));
    assert!(
        told["changes"][0]["what"]
            .as_array()
            .unwrap()
            .contains(&json!("column")),
        "{told}"
    );
    assert_eq!(told["changes"][0]["before"]["column"], json!("review"));
    assert_eq!(told["tickets"][0]["column"], json!("done"));
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_approval_that_fails_blocks_the_ticket_in_to_review_and_the_server_has_answered() {
    let h = harness("mcp-act-approve-fails");
    let (p, r) = h.project(false).await;
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    // The project's folder has changes of its own on the target: the merge is refused.
    std::fs::write(r.join("src").join("app.ts"), "const a = 99; // en cours\n").unwrap();
    let c = external(&h).await;
    read(&c, "approve_ticket", json!({ "ticket": t.key })).await;
    assert_eq!(entry(&h).outcome, "ok");
    h.wait_ticket(&t.id, "the ticket to be blocked", |t| t.blocked.is_some())
        .await;
    let blocked = stored(&h, &t.id);
    assert_eq!(
        (
            blocked.column,
            blocked.step.as_deref(),
            blocked.blocked.as_deref()
        ),
        (
            Column::Review,
            None,
            Some("Le dossier du projet a des modifications non commitées sur main")
        )
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_is_sent_back_with_the_comment_followed_by_who_sent_it_back() {
    let h = harness("mcp-act-reject");
    let (p, _) = h.project(false).await;
    let (t, wt) = reviewed(&h, &p.id, "Renvoi [ok]").await;
    let agent = t.agent_id.clone().unwrap();
    let c = external(&h).await;
    let v = read(
        &c,
        "reject_ticket",
        json!({ "ticket": "dem-1", "comment": "  Le bouton est mal placé.  " }),
    )
    .await;
    // Back in progress, with the same agent.
    assert_eq!(
        (v["key"].as_str(), v["column"].as_str()),
        (Some("DEM-1"), Some("doing"))
    );
    // Its since is the ticket as it was: in review, its agent at rest.
    let before = before_of(&v);
    assert_eq!(
        (&before["column"], &before["resting"]),
        (&json!("review"), &json!(true))
    );
    let logged = entry(&h);
    assert_eq!(
        (
            logged.caller.as_str(),
            logged.tool.as_str(),
            logged.summary.as_str(),
            logged.outcome
        ),
        (
            "Claude (hors Escouade)",
            "reject_ticket",
            "ticket: dem-1 · comment: Le bouton est mal placé.",
            "ok"
        )
    );
    // The agent reads the comment as test feedback, never as the user's words: it says who sent it.
    h.wait_sent(
        &wt,
        "Retour de test sur DEM-1 : Le bouton est mal placé (renvoyé par Claude (hors Escouade) via Escouade). Corrige, revérifie tous les critères, puis termine par le bilan.",
    )
    .await;
    h.wait_ticket(&t.id, "to test again", |t| t.column == Column::Review)
        .await;
    let back = stored(&h, &t.id);
    // Loop 1 again, its second loop all the same.
    assert_eq!(
        (back.agent_id.as_deref(), back.iteration, back.loops),
        (Some(agent.as_str()), 1, 2)
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_sent_back_by_an_agent_names_it_in_the_language_of_the_texts_for_claude() {
    let h = harness("mcp-act-reject-en");
    let (p, _) = h.project(false).await;
    h.set_settings(|s| s.claude_language = "en".into());
    let (t, wt) = reviewed(&h, &p.id, "Rework [ok]").await;
    let reviewer = put_agent(&h, &p, "relecteur", 5);
    let c = as_agent(&h, &reviewer.id).await;
    read(
        &c,
        "reject_ticket",
        json!({ "ticket": t.key, "comment": "the button is misplaced" }),
    )
    .await;
    h.wait_sent(
        &wt,
        "Test feedback on DEM-1: the button is misplaced (sent back by relecteur through Escouade). Fix it, check every criterion again, then end with the report.",
    )
    .await;
    h.wait_ticket(&t.id, "to test again", |t| t.column == Column::Review)
        .await;
    assert_eq!(entry(&h).caller, "relecteur");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_is_not_sent_back_without_a_comment_or_where_the_window_does_not_offer_it() {
    let h = harness("mcp-act-reject-guards");
    let p = project(&h).await;
    let c = external(&h).await;
    let ask = |t: &Ticket| json!({ "ticket": t.key, "comment": "Pas bon" });
    let (review, agent) = staffed(&h, &p, "DEM-1", Column::Review, 1);
    // A comment is needed, a blank one is none: something asked that does not fit.
    for blank in ["", "   \n "] {
        let why = turned_down(
            &h,
            &c,
            "reject_ticket",
            json!({ "ticket": review.key, "comment": blank }),
            "error",
            &review,
        )
        .await;
        assert_eq!(why, "Dis ce qui ne va pas.");
    }
    // Not in « À tester ».
    for (i, column) in [Column::Todo, Column::Doing, Column::Done]
        .into_iter()
        .enumerate()
    {
        let t = put(
            &h,
            ticket(&p, &format!("DEM-{}", i + 2), "Ailleurs", column, 1),
        );
        let why = turned_down(&h, &c, "reject_ticket", ask(&t), "refused", &t).await;
        assert_eq!(why, NOT_IN_REVIEW, "{column:?}");
    }
    // Without an agent to send it to.
    let orphan = put(&h, ticket(&p, "DEM-5", "Orphelin", Column::Review, 5));
    let why = turned_down(&h, &c, "reject_ticket", ask(&orphan), "refused", &orphan).await;
    assert_eq!(why, "Ce ticket n'a pas d'agent.");
    // Its agent works: the end of that turn would be read as the end of the rework.
    let (busy, worker) = staffed(&h, &p, "DEM-6", Column::Review, 6);
    h.core.agent(&worker.id).unwrap().lock().meta.status = AgentStatus::Running;
    let why = turned_down(&h, &c, "reject_ticket", ask(&busy), "refused", &busy).await;
    assert_eq!(
        why,
        "L'agent de ce ticket travaille encore : attends la fin de son tour pour le renvoyer."
    );
    // Its approval runs.
    let (running, _) = staffed(&h, &p, "DEM-7", Column::Review, 7);
    let running = approving(&h, &running, "Merge…");
    let why = turned_down(&h, &c, "reject_ticket", ask(&running), "refused", &running).await;
    assert_eq!(why, APPROVING);
    // Nothing was sent to any agent.
    assert_eq!(h.agent(&agent.id).prompts, 0);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_is_not_sent_back_while_its_agents_account_is_held_back_and_only_then() {
    let h = harness("mcp-act-reject-accounts");
    let p = project(&h).await;
    let c = external(&h).await;
    let end = h.core.pause_now() + 3_600_000;
    used_up(&h, PRINCIPAL, end);
    let ask = |t: &Ticket| json!({ "ticket": t.key, "comment": "Pas bon" });
    // Where the window would go ahead, the message would spend the held account's quota.
    let (review, agent) = staffed(&h, &p, "DEM-1", Column::Review, 1);
    let held = turned_down(&h, &c, "reject_ticket", ask(&review), "refused", &review).await;
    assert!(
        held.contains("pause")
            && held.contains("5 h")
            && held.contains(&clock(end + RESUME_MARGIN_MS)),
        "{held}"
    );
    assert_eq!(h.agent(&agent.id).prompts, 0);
    // Where it would not, the window's own refusal is the one told.
    let (running, _) = staffed(&h, &p, "DEM-2", Column::Review, 2);
    let running = approving(&h, &running, "Tests…");
    let why = turned_down(&h, &c, "reject_ticket", ask(&running), "refused", &running).await;
    assert_eq!(why, APPROVING);
    let (doing, _) = staffed(&h, &p, "DEM-3", Column::Doing, 3);
    let why = turned_down(&h, &c, "reject_ticket", ask(&doing), "refused", &doing).await;
    assert_eq!(why, NOT_IN_REVIEW);
    // Its agent works, on the held account all the same: the window says it first.
    let (busy, worker) = staffed(&h, &p, "DEM-4", Column::Review, 4);
    h.core.agent(&worker.id).unwrap().lock().meta.status = AgentStatus::Running;
    let why = turned_down(&h, &c, "reject_ticket", ask(&busy), "refused", &busy).await;
    assert_eq!(
        why,
        "L'agent de ce ticket travaille encore : attends la fin de son tour pour le renvoyer."
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

/// Claude Code nowhere to be found, as the app says it (with its repair, ESC-133).
fn no_claude() -> String {
    crate::engine::missing_line(crate::model::EngineId::Claude, crate::i18n::Lang::Fr)
}

/// The block of a ticket whose agent got no message: Claude Code is nowhere to be found.
fn no_claude_block() -> String {
    format!("Erreur : {}", no_claude())
}

/// What a tool says when its message failed after the ticket moved.
fn unsent(key: &str) -> String {
    format!("{key} a été pris en compte, mais son agent n’a pas reçu le message : {} Le ticket est bloqué avec cette raison.", no_claude())
}

#[tokio::test]
async fn a_message_that_fails_after_the_ticket_moved_says_so_and_the_ticket_is_blocked_with_the_reason(
) {
    let h = harness("mcp-act-unsent");
    let p = project(&h).await;
    let c = external(&h).await;
    h.lose_claude();
    // Sent back: in progress again, blocked, and the call is not told as a refusal.
    let (review, _) = staffed(&h, &p, "DEM-1", Column::Review, 1);
    let said = refused(
        &c,
        "reject_ticket",
        json!({ "ticket": review.key, "comment": "Pas bon" }),
    )
    .await;
    assert_eq!(said, unsent("DEM-1"));
    let logged = entry(&h);
    assert_eq!(
        (
            logged.tool.as_str(),
            logged.outcome,
            logged.message.as_deref()
        ),
        ("reject_ticket", "error", Some(said.as_str()))
    );
    let moved = stored(&h, &review.id);
    assert_eq!(
        (moved.column, moved.blocked.as_deref()),
        (Column::Doing, Some(no_claude_block().as_str()))
    );
    // Resumed: no longer blocked for what it was, blocked for this.
    let (blocked, _) = staffed(&h, &p, "DEM-2", Column::Doing, 2);
    h.core
        .edit_ticket(&blocked.id, |x| {
            x.blocked = Some("Interrompu".into());
            Ok(())
        })
        .unwrap();
    let said = refused(&c, "resume_ticket", json!({ "ticket": blocked.key })).await;
    assert_eq!(said, unsent("DEM-2"));
    assert_eq!(entry(&h).outcome, "error");
    assert_eq!(
        stored(&h, &blocked.id).blocked.as_deref(),
        Some(no_claude_block().as_str())
    );
    // Blocked for the very same reason as before: nothing moved, the failure is a refusal.
    let (same, _) = staffed(&h, &p, "DEM-3", Column::Doing, 3);
    h.core
        .edit_ticket(&same.id, |x| {
            x.blocked = Some(no_claude_block());
            Ok(())
        })
        .unwrap();
    let same = stored(&h, &same.id);
    let why = turned_down(
        &h,
        &c,
        "resume_ticket",
        json!({ "ticket": same.key }),
        "refused",
        &same,
    )
    .await;
    assert_eq!(why, no_claude());
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_conflict_handed_over_whose_message_fails_says_so_and_the_ticket_is_blocked() {
    let h = harness("mcp-act-resolve-unsent");
    let (p, r) = h.project(false).await;
    let (t, _) = conflicting(&h, &p.id, &r).await;
    h.core.ticket_approve(&t.id).await.unwrap();
    // Its process gone and Claude Code nowhere: the conflicts cannot be told to the agent.
    let agent = t.agent_id.clone().unwrap();
    if let Some(proc) = h.core.agent(&agent).unwrap().lock().detach() {
        proc.kill();
    }
    h.lose_claude();
    let c = external(&h).await;
    let said = refused(&c, "resolve_conflict", json!({ "ticket": "dem-1" })).await;
    assert_eq!(said, unsent("DEM-1"));
    assert_eq!(entry(&h).outcome, "error");
    let moved = stored(&h, &t.id);
    assert_eq!(
        (moved.column, moved.conflict, moved.blocked.as_deref()),
        (Column::Doing, false, Some(no_claude_block().as_str()))
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_blocked_ticket_is_taken_up_by_its_agent() {
    let h = harness("mcp-act-resume");
    let (p, _) = h.project(false).await;
    let t = h
        .core
        .ticket_create(&p.id, draft("Long travail [lent]", &[], 5))
        .await
        .unwrap();
    let agent = wait_running(&h, &t.id).await;
    h.core.interrupt(&agent).await.unwrap();
    h.wait_ticket(&t.id, "the ticket to be blocked", |t| t.blocked.is_some())
        .await;
    assert_eq!(stored(&h, &t.id).blocked.as_deref(), Some("Interrompu"));
    let c = external(&h).await;
    let v = read(&c, "resume_ticket", json!({ "ticket": "dem-1" })).await;
    let logged = entry(&h);
    // Its since is the ticket as it was: in progress, blocked.
    let before = before_of(&v);
    assert_eq!(before["column"], json!("doing"));
    assert!(before["blocked"].is_number(), "{before}");
    // Taken up, it is no longer blocked: a second call is refused. Made at once: once its agent has
    // ended its turn, the ticket is no longer in progress, and the refusal would be another.
    let again = refused(&c, "resume_ticket", json!({ "ticket": "DEM-1" })).await;
    assert_eq!(again, RESUME_NOT_BLOCKED);
    assert_eq!(
        (v["key"].as_str(), v["column"].as_str()),
        (Some("DEM-1"), Some("doing"))
    );
    assert_eq!(stored(&h, &t.id).blocked, None);
    assert_eq!(
        (
            logged.tool.as_str(),
            logged.summary.as_str(),
            logged.outcome
        ),
        ("resume_ticket", "ticket: dem-1", "ok")
    );
    // Its agent goes on where it was.
    h.wait_sent(
        &h.worktree_of(&t.id),
        "Reprends le ticket DEM-1 là où tu en étais, puis termine par le bilan.",
    )
    .await;
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_blocked_ticket_whose_agent_is_gone_starts_again_even_with_the_autopilot_off() {
    let h = harness("mcp-act-resume-gone");
    let p = project(&h).await;
    let t = made(&h, &p, "Repartir [ok]").await;
    h.core
        .edit_ticket(&t.id, |t| {
            t.column = Column::Doing;
            t.iteration = 2;
            t.agent_id = Some("disparu".into());
            t.blocked = Some("Interrompu".into());
            Ok(())
        })
        .unwrap();
    let c = external(&h).await;
    read(&c, "resume_ticket", json!({ "ticket": t.key })).await;
    assert_eq!(entry(&h).outcome, "ok");
    h.wait_ticket(&t.id, "started again to test", |t| {
        t.column == Column::Review
    })
    .await;
    assert_ne!(stored(&h, &t.id).agent_id.as_deref(), Some("disparu"));
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_is_not_resumed_where_the_window_does_not_offer_it() {
    use chrono::TimeZone;
    let h = harness("mcp-act-resume-guards");
    let p = project(&h).await;
    let c = external(&h).await;
    let ask = |t: &Ticket| json!({ "ticket": t.key });
    // Not in progress.
    for (i, column) in [Column::Todo, Column::Review, Column::Done]
        .into_iter()
        .enumerate()
    {
        let t = put(
            &h,
            Ticket {
                blocked: Some("Erreur".into()),
                ..ticket(&p, &format!("DEM-{}", i + 1), "Ailleurs", column, 1)
            },
        );
        let why = turned_down(&h, &c, "resume_ticket", ask(&t), "refused", &t).await;
        assert_eq!(why, "Ce ticket n'est pas en cours.", "{column:?}");
    }
    // Not blocked: taken up already.
    let (moving, _) = staffed(&h, &p, "DEM-4", Column::Doing, 4);
    let why = turned_down(&h, &c, "resume_ticket", ask(&moving), "refused", &moving).await;
    assert_eq!(why, RESUME_NOT_BLOCKED);
    // Its agent waits for its quota: a message now would meet the limit again.
    let (waiting, agent) = staffed(&h, &p, "DEM-5", Column::Doing, 5);
    let at = chrono::Local
        .with_ymd_and_hms(2026, 10, 3, 15, 7, 0)
        .unwrap()
        .timestamp_millis();
    h.core.agent(&agent.id).unwrap().lock().meta.resume_at = Some(at);
    h.core
        .edit_ticket(&waiting.id, |x| {
            x.blocked = Some("Interrompu".into());
            Ok(())
        })
        .unwrap();
    let waiting = stored(&h, &waiting.id);
    let why = turned_down(&h, &c, "resume_ticket", ask(&waiting), "refused", &waiting).await;
    assert_eq!(why, "En attente du quota — reprise à 15:07");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_blocked_ticket_is_not_resumed_while_the_account_that_would_take_it_is_held_back() {
    let h = harness("mcp-act-resume-accounts");
    let p = project(&h).await;
    let c = external(&h).await;
    let end = h.core.pause_now() + 3_600_000;
    used_up(&h, PRINCIPAL, end);
    let ask = |t: &Ticket| json!({ "ticket": t.key });
    let held_for = |said: &str| {
        said.contains("pause")
            && said.contains("5 h")
            && said.contains(&clock(end + RESUME_MARGIN_MS))
    };
    // Its agent is there: the message would spend its account's quota.
    let (blocked, agent) = staffed(&h, &p, "DEM-1", Column::Doing, 1);
    h.core
        .edit_ticket(&blocked.id, |x| {
            x.blocked = Some("Interrompu".into());
            Ok(())
        })
        .unwrap();
    let blocked = stored(&h, &blocked.id);
    let why = turned_down(&h, &c, "resume_ticket", ask(&blocked), "refused", &blocked).await;
    assert!(held_for(&why), "{why}");
    assert_eq!(h.agent(&agent.id).prompts, 0);
    // It is gone: a new agent would start on a held account.
    let gone = put(&h, ticket(&p, "DEM-2", "Sans agent", Column::Doing, 2));
    h.core
        .edit_ticket(&gone.id, |x| {
            x.agent_id = Some("disparu".into());
            x.blocked = Some("Interrompu".into());
            Ok(())
        })
        .unwrap();
    let gone = stored(&h, &gone.id);
    let why = turned_down(&h, &c, "resume_ticket", ask(&gone), "refused", &gone).await;
    assert!(held_for(&why), "{why}");
    // Not blocked: the window's own refusal is the one told, held or not.
    let (moving, _) = staffed(&h, &p, "DEM-3", Column::Doing, 3);
    let why = turned_down(&h, &c, "resume_ticket", ask(&moving), "refused", &moving).await;
    assert_eq!(why, RESUME_NOT_BLOCKED);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_conflict_is_handed_to_the_tickets_agent_which_gets_the_target_merged_into_its_branch() {
    let h = harness("mcp-act-resolve");
    let (p, r) = h.project(false).await;
    let (t, wt) = conflicting(&h, &p.id, &r).await;
    let agent = t.agent_id.clone();
    // The approval stops on the conflict and asks what to do.
    h.core.ticket_approve(&t.id).await.unwrap();
    let stopped = stored(&h, &t.id);
    assert_eq!(
        (stopped.column, stopped.conflict, stopped.blocked.as_deref()),
        (Column::Review, true, Some("Conflit avec main"))
    );
    let c = external(&h).await;
    let v = read(&c, "resolve_conflict", json!({ "ticket": "dem-1" })).await;
    assert_eq!(
        (v["key"].as_str(), v["column"].as_str()),
        (Some("DEM-1"), Some("doing"))
    );
    // Its since is the ticket as it was: in review, stopped on the conflict.
    let before = before_of(&v);
    assert_eq!(
        (&before["column"], &before["conflict"]),
        (&json!("review"), &json!(true))
    );
    let logged = entry(&h);
    assert_eq!(
        (
            logged.tool.as_str(),
            logged.summary.as_str(),
            logged.outcome
        ),
        ("resolve_conflict", "ticket: dem-1", "ok")
    );
    // The target is merged into its branch, the conflicts left in the worktree for the agent.
    h.wait_sent(
        &wt,
        "Le merge de main dans ta branche a des conflits sur : src/app.ts.",
    )
    .await;
    assert!(git(&wt, &["diff", "--name-only", "--diff-filter=U"]).contains("src/app.ts"));
    let handed = stored(&h, &t.id);
    assert_eq!(
        (
            handed.agent_id,
            handed.iteration,
            handed.conflict,
            handed.blocked,
            handed.step
        ),
        (agent, 1, false, None, None)
    );
    // The project's folder was left as it was.
    assert_eq!(git(&r, &["status", "--porcelain"]), "");
    // Its agent answers with its report: to test again.
    h.wait_ticket(&t.id, "to test again", |t| t.column == Column::Review)
        .await;
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_conflict_is_not_handed_over_where_the_window_does_not_offer_it() {
    let h = harness("mcp-act-resolve-guards");
    let p = project(&h).await;
    let c = external(&h).await;
    let ask = |t: &Ticket| json!({ "ticket": t.key });
    // Not in « À tester », a conflict or not.
    for (i, column) in [Column::Todo, Column::Doing, Column::Done]
        .into_iter()
        .enumerate()
    {
        let t = put(
            &h,
            Ticket {
                conflict: true,
                blocked: Some("Conflit avec main".into()),
                ..ticket(&p, &format!("DEM-{}", i + 1), "Ailleurs", column, 1)
            },
        );
        let why = turned_down(&h, &c, "resolve_conflict", ask(&t), "refused", &t).await;
        assert_eq!(why, NOT_IN_REVIEW, "{column:?}");
    }
    // Its approval runs.
    let (running, _) = staffed(&h, &p, "DEM-4", Column::Review, 4);
    h.core
        .edit_ticket(&running.id, |x| {
            x.conflict = true;
            Ok(())
        })
        .unwrap();
    let running = approving(&h, &stored(&h, &running.id), "Merge…");
    let why = turned_down(
        &h,
        &c,
        "resolve_conflict",
        ask(&running),
        "refused",
        &running,
    )
    .await;
    assert_eq!(why, APPROVING);
    // Blocked by something else than a conflict (a step that failed), or not blocked at all.
    for (key, blocked) in [("DEM-5", Some("Tests en échec")), ("DEM-6", None)] {
        let (t, _) = staffed(&h, &p, key, Column::Review, 5);
        h.core
            .edit_ticket(&t.id, |x| {
                x.blocked = blocked.map(str::to_string);
                Ok(())
            })
            .unwrap();
        let t = stored(&h, &t.id);
        let why = turned_down(&h, &c, "resolve_conflict", ask(&t), "refused", &t).await;
        assert_eq!(why, "Ce ticket n’a pas de conflit à résoudre.", "{key}");
    }
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_conflict_is_not_handed_over_while_its_agents_account_is_held_back_and_only_then() {
    let h = harness("mcp-act-resolve-accounts");
    let p = project(&h).await;
    let c = external(&h).await;
    let end = h.core.pause_now() + 3_600_000;
    used_up(&h, PRINCIPAL, end);
    let ask = |t: &Ticket| json!({ "ticket": t.key });
    let stopped_on_a_conflict = |key: &str, at: i64| {
        let (t, agent) = staffed(&h, &p, key, Column::Review, at);
        h.core
            .edit_ticket(&t.id, |x| {
                x.conflict = true;
                x.blocked = Some("Conflit avec main".into());
                Ok(())
            })
            .unwrap();
        (stored(&h, &t.id), agent)
    };
    // The message to its agent would spend the held account's quota.
    let (stopped, agent) = stopped_on_a_conflict("DEM-1", 1);
    let why = turned_down(
        &h,
        &c,
        "resolve_conflict",
        ask(&stopped),
        "refused",
        &stopped,
    )
    .await;
    assert!(
        why.contains("pause")
            && why.contains("5 h")
            && why.contains(&clock(end + RESUME_MARGIN_MS)),
        "{why}"
    );
    assert_eq!(h.agent(&agent.id).prompts, 0);
    // Where the window would not go ahead, its own refusal is the one told.
    let (running, _) = stopped_on_a_conflict("DEM-2", 2);
    let running = approving(&h, &running, "Merge…");
    let why = turned_down(
        &h,
        &c,
        "resolve_conflict",
        ask(&running),
        "refused",
        &running,
    )
    .await;
    assert_eq!(why, APPROVING);
    // Its agent works, on the held account all the same: the window says it first.
    let (busy, worker) = stopped_on_a_conflict("DEM-3", 3);
    {
        let worker = h.core.agent(&worker.id).unwrap();
        let mut rt = worker.lock();
        rt.meta.status = AgentStatus::Running;
        rt.meta.worktree = Some(Worktree {
            repos: Vec::new(),
            path: h.dir.join("dem-3").to_string_lossy().into(),
            branch: "ticket/dem-3".into(),
            base_branch: "main".into(),
            existing: false,
            named_branch: false,
        });
    }
    let why = turned_down(&h, &c, "resolve_conflict", ask(&busy), "refused", &busy).await;
    assert_eq!(
        why,
        "L'agent de ce ticket travaille encore : attends la fin de son tour pour valider."
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_block_of_a_ticket_to_review_is_dismissed_and_the_ticket_stays_to_review() {
    let h = harness("mcp-act-dismiss");
    let p = project(&h).await;
    let c = external(&h).await;
    // A conflict, and a step that failed.
    for (key, blocked, conflict) in [
        ("DEM-1", "Conflit avec main", true),
        (
            "DEM-2",
            "Le dossier du projet a des modifications non commitées sur main",
            false,
        ),
    ] {
        let (t, _) = staffed(&h, &p, key, Column::Review, 1);
        h.core
            .edit_ticket(&t.id, |x| {
                x.conflict = conflict;
                x.blocked = Some(blocked.into());
                Ok(())
            })
            .unwrap();
        let before = stored(&h, &t.id);
        let seen = ticket_events(&h, &t.id);
        let v = read(&c, "dismiss_block", json!({ "ticket": key.to_lowercase() })).await;
        assert_eq!(
            (v["key"].as_str(), v["column"].as_str()),
            (Some(key), Some("review"))
        );
        let logged = entry(&h);
        assert_eq!(
            (logged.tool.as_str(), logged.outcome),
            ("dismiss_block", "ok")
        );
        assert_eq!(logged.summary, format!("ticket: {}", key.to_lowercase()));
        // The block goes, nothing else changes; the window is told.
        assert_eq!(
            stored(&h, &t.id),
            Ticket {
                blocked: None,
                conflict: false,
                ..before
            },
            "{key}"
        );
        assert!(ticket_events(&h, &t.id) > seen, "{key}");
        // The answer's since is the ticket as it was, blocked: a wait from it is told at once that
        // the block went (and the conflict).
        assert_eq!(before_of(&v)["conflict"], json!(conflict), "{key}");
        let told = told_since(&c, key, &v["since"]).await;
        let what = if conflict {
            json!(["blocked", "conflict"])
        } else {
            json!(["blocked"])
        };
        assert_eq!(told["changes"][0]["what"], what, "{key}");
        assert_eq!(
            told["changes"][0]["before"]["blocked"],
            json!(true),
            "{key}"
        );
    }
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_block_is_not_dismissed_where_the_window_does_not_offer_it() {
    let h = harness("mcp-act-dismiss-guards");
    let p = project(&h).await;
    let c = external(&h).await;
    let ask = |t: &Ticket| json!({ "ticket": t.key });
    // Not in « À tester »: the block of a ticket under way is the autopilot's, and an agent's.
    for (i, column) in [Column::Todo, Column::Doing, Column::Done]
        .into_iter()
        .enumerate()
    {
        let t = put(
            &h,
            Ticket {
                blocked: Some("Interrompu".into()),
                ..ticket(&p, &format!("DEM-{}", i + 1), "Ailleurs", column, 1)
            },
        );
        let why = turned_down(&h, &c, "dismiss_block", ask(&t), "refused", &t).await;
        assert_eq!(why, NOT_IN_REVIEW, "{column:?}");
    }
    // Its approval runs: the block it may make is not yet there to dismiss.
    let (running, _) = staffed(&h, &p, "DEM-4", Column::Review, 4);
    let running = approving(&h, &running, "Tests…");
    let why = turned_down(&h, &c, "dismiss_block", ask(&running), "refused", &running).await;
    assert_eq!(why, APPROVING);
    // Nothing blocks it.
    let (free, _) = staffed(&h, &p, "DEM-5", Column::Review, 5);
    let why = turned_down(&h, &c, "dismiss_block", ask(&free), "refused", &free).await;
    assert_eq!(why, NOT_BLOCKED);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_is_deleted_whatever_its_column_and_the_ones_after_it_no_longer_wait_for_it() {
    let h = harness("mcp-act-delete");
    let p = project(&h).await;
    let first = made(&h, &p, "Premier").await;
    let second = made_after(&h, &p, "Deuxième", &[&first]).await;
    let c = external(&h).await;
    let seen = ticket_events(&h, &second.id);
    let v = read(&c, "delete_ticket", json!({ "ticket": "dem-1" })).await;
    assert_eq!(
        v,
        json!({ "key": "DEM-1", "deleted": true, "agentArchived": false })
    );
    let logged = entry(&h);
    assert_eq!(
        (
            logged.tool.as_str(),
            logged.summary.as_str(),
            logged.outcome
        ),
        ("delete_ticket", "ticket: dem-1", "ok")
    );
    // Gone for good, the window told, and the ticket that came after it no longer waits for it.
    assert!(h.core.ticket(&first.id).is_err());
    assert!(h.events.lock().iter().any(|e| e["type"] == "ticketRemoved"
        && e["id"] == first.id.as_str()
        && e["projectId"] == p.id.as_str()));
    assert!(stored(&h, &second.id).after.is_empty());
    assert!(ticket_events(&h, &second.id) > seen);
    let again = refused(&c, "delete_ticket", json!({ "ticket": first.id })).await;
    assert!(again.contains("Aucun ticket"), "{again}");

    // One to review: its agent is archived (its worktree kept), and the answer says so.
    let (review, agent) = staffed(&h, &p, "DEM-7", Column::Review, 7);
    let v = read(&c, "delete_ticket", json!({ "ticket": review.key })).await;
    assert_eq!(
        v,
        json!({ "key": "DEM-7", "deleted": true, "agentArchived": true })
    );
    assert!(h.core.ticket(&review.id).is_err());
    assert!(h.agent(&agent.id).archived);
    // One in progress, its agent at work: as the window deletes it.
    let (doing, worker) = staffed(&h, &p, "DEM-8", Column::Doing, 8);
    h.core.agent(&worker.id).unwrap().lock().meta.status = AgentStatus::Running;
    let v = read(&c, "delete_ticket", json!({ "ticket": doing.key })).await;
    assert_eq!(v["agentArchived"], true);
    assert!(h.agent(&worker.id).archived);
    // One done leaves its agent as it was: it is no longer the ticket's to archive.
    let (done, finished) = staffed(&h, &p, "DEM-9", Column::Done, 9);
    let v = read(&c, "delete_ticket", json!({ "ticket": done.key })).await;
    assert_eq!(v["agentArchived"], false);
    assert!(!h.agent(&finished.id).archived);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_is_not_deleted_while_its_approval_runs() {
    let h = harness("mcp-act-delete-approving");
    let p = project(&h).await;
    let (own, agent) = staffed(&h, &p, "DEM-1", Column::Review, 1);
    let own = approving(&h, &own, "Merge…");
    let c = external(&h).await;
    let why = turned_down(
        &h,
        &c,
        "delete_ticket",
        json!({ "ticket": own.key }),
        "refused",
        &own,
    )
    .await;
    assert_eq!(
        why,
        "La validation de ce ticket est en cours : attends sa fin pour le supprimer."
    );
    // An agent of the ticket is refused first: the ticket is not its to act on.
    let a = as_agent(&h, &agent.id).await;
    let why = turned_down(
        &h,
        &a,
        "delete_ticket",
        json!({ "ticket": own.key }),
        "refused",
        &own,
    )
    .await;
    assert_eq!(why, OWN_TICKET);
    // The approval over, it is deleted as any.
    h.core
        .edit_ticket(&own.id, |x| {
            x.step = None;
            Ok(())
        })
        .unwrap();
    let v = read(&c, "delete_ticket", json!({ "ticket": own.key })).await;
    assert_eq!(v["deleted"], json!(true));
    assert!(h.core.ticket(&own.id).is_err());
    a.cancel().await.unwrap();
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

// ---------- the Kanban's settings, and the model of a ticket ----------

/// The board of `p`, as the core keeps it.
fn board_of(h: &Harness, p: &Project) -> BoardSettings {
    h.core.project(&p.id).unwrap().board
}

/// What the tools do not set, written as the core has it (the key's prefix and number, the test
/// command, the user's own choice of mode).
fn edit_board(h: &Harness, p: &Project, f: impl FnOnce(&mut BoardSettings)) {
    let mut projects = h.core.projects.write();
    f(&mut projects.iter_mut().find(|q| q.id == p.id).unwrap().board);
}

/// How many `project` events the window was sent for `p`.
fn project_events(h: &Harness, p: &Project) -> usize {
    h.events
        .lock()
        .iter()
        .filter(|e| e["type"] == "project" && e["project"]["id"] == p.id.as_str())
        .count()
}

/// The models the window offers: a model is checked against them.
fn known_models(h: &Harness) {
    *h.core.models.write() = ["sonnet", "opus", "haiku"]
        .iter()
        .map(|m| ModelInfo {
            value: m.to_string(),
            resolved_model: format!("claude-{m}-9"),
            ..Default::default()
        })
        .collect();
}

#[tokio::test]
async fn a_boards_settings_change_only_the_ones_given_and_the_core_and_the_window_follow() {
    let h = harness("mcp-act-set-board");
    let p = project(&h).await;
    git(&h.dir.join("repo"), &["branch", "develop"]);
    known_models(&h);
    edit_board(&h, &p, |b| {
        b.prefix = "DEM".into();
        b.next_number = 7;
        b.target = "main".into();
        b.test_command = "npm test".into();
    });
    let c = external(&h).await;
    let before = board_of(&h, &p);
    let seen = project_events(&h, &p);

    let v = read(
        &c,
        "set_board",
        json!({
            "project": "demo", "action": "pr", "strategy": "squash", "draft": true,
            "model": "opus", "effort": "max",
        }),
    )
    .await;
    // The others are what they were: the prefix and the number, the target, the test command.
    let after = BoardSettings {
        action: "pr".into(),
        draft: true,
        model: "opus".into(),
        effort: "max".into(),
        ..before.clone()
    };
    assert_eq!(board_of(&h, &p), after);
    // The answer is `get_board`'s, after the change.
    assert_eq!(
        v,
        json!({
            "project": "demo",
            "projectId": p.id,
            "action": "pr",
            "target": "main",
            "strategy": "squash",
            "draft": true,
            "cleanup": true,
            "conventional": true,
            "conflict": "ask",
            "maxParallel": 2,
            "autopilot": false,
            "model": "opus",
            "effort": "max",
            "mode": "",
            "defaults": { "model": "sonnet", "effort": "medium", "mode": "auto" },
            "testsFirst": false,
            "testCommand": "npm test",
            "prefix": "DEM",
        })
    );
    let logged = entry(&h);
    assert_eq!(read(&c, "get_board", json!({ "project": "demo" })).await, v);
    // The window is told as it is for the settings it changes itself.
    assert_eq!(project_events(&h, &p), seen + 1);
    let told = h
        .events
        .lock()
        .iter()
        .rev()
        .find(|e| e["type"] == "project" && e["project"]["id"] == p.id.as_str())
        .cloned()
        .unwrap();
    assert_eq!(told["project"]["board"]["action"], "pr");
    assert_eq!(told["project"]["board"]["draft"], true);
    // The call is in the activity log with what it was given, and what it changed, each setting
    // from its value to its new one: not the strategy, given as it was.
    assert_eq!(
        (
            logged.caller.as_str(),
            logged.tool.as_str(),
            logged.outcome,
            logged.summary.as_str(),
            logged.message.as_deref()
        ),
        (
            "Claude (hors Escouade)",
            "set_board",
            "ok",
            "project: demo · action: pr · strategy: squash · draft: true · model: opus · effort: max",
            Some("action: merge → pr · draft: false → true · model: (défaut) → opus · effort: (défaut) → max")
        )
    );

    // Every setting at once, the target and the choices as the window has them; an empty model
    // and effort are the app's.
    let v = read(
        &c,
        "set_board",
        json!({
            "project": p.id, "action": "keep", "target": " develop ", "strategy": "rebase",
            "draft": false, "cleanup": false, "conventional": false, "conflict": "agent",
            "maxParallel": 6, "autopilot": true, "model": "", "effort": "", "mode": "plan",
            "testsFirst": true,
        }),
    )
    .await;
    let everything = BoardSettings {
        action: "keep".into(),
        target: "develop".into(),
        strategy: "rebase".into(),
        draft: false,
        cleanup: false,
        conventional: false,
        conflict: "agent".into(),
        max_parallel: 6,
        autopilot: true,
        model: String::new(),
        effort: String::new(),
        mode: "plan".into(),
        tests_first: true,
        ..after
    };
    assert_eq!(board_of(&h, &p), everything);
    assert_eq!(
        (
            &v["target"],
            &v["maxParallel"],
            &v["mode"],
            &v["testsFirst"]
        ),
        (&json!("develop"), &json!(6), &json!("plan"), &json!(true))
    );
    let logged = entry(&h);
    // Thirteen settings, each said from its value to its new one: more than a line's 300 characters.
    let said = "action: pr → keep · target: main → develop · strategy: squash → rebase · draft: true → false · cleanup: true → false · conventional: true → false · conflict: ask → agent · maxParallel: 2 → 6 · autopilot: false → true · model: opus → (défaut) · effort: max → (défaut) · mode: (défaut) → plan · testsFirst: false → true";
    assert!(said.chars().count() > 300);
    assert_eq!(logged.message.as_deref(), Some(said));
    let summary = logged.summary;
    for given in [
        "action: keep",
        "target:  develop ",
        "strategy: rebase",
        "draft: false",
        "cleanup: false",
        "conventional: false",
        "conflict: agent",
        "maxParallel: 6",
        "autopilot: true",
        "model: ",
        "effort: ",
        "mode: plan",
        "testsFirst: true",
    ] {
        assert!(summary.contains(given), "{given} not in {summary}");
    }

    // What `get_board` answers can be given back as it is: nothing changes, and the log says so.
    let got = read(&c, "get_board", json!({ "project": "demo" })).await;
    assert_eq!(read(&c, "set_board", got.clone()).await, got);
    assert_eq!(board_of(&h, &p), everything);
    let logged = entry(&h);
    assert_eq!(
        (logged.outcome, logged.message.as_deref()),
        (
            "ok",
            Some("Aucun changement : le Kanban était déjà réglé ainsi.")
        )
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_setting_that_does_not_fit_or_is_the_users_to_give_is_refused_with_the_reason_and_nothing_changes(
) {
    let h = harness("mcp-act-set-board-refused");
    let p = project(&h).await;
    git(&h.dir.join("repo"), &["branch", "develop"]);
    known_models(&h);
    edit_board(&h, &p, |b| {
        b.prefix = "DEM".into();
        b.next_number = 4;
        b.target = "main".into();
    });
    let c = external(&h).await;
    let before = board_of(&h, &p);
    let seen = project_events(&h, &p);

    // (What is asked, how the log says it ended, what the answer says.)
    let cases: Vec<(Value, &str, Vec<&str>)> = vec![
        // Nothing to change.
        (
            json!({ "project": "demo" }),
            "error",
            vec!["au moins un réglage"],
        ),
        // A model the app does not offer, or no model's name: the other settings given with it
        // wait too.
        (
            json!({ "project": "demo", "draft": true, "model": "gpt" }),
            "error",
            vec!["gpt", "sonnet", "opus"],
        ),
        (
            json!({ "project": "demo", "model": "--permission-mode" }),
            "error",
            vec!["--permission-mode"],
        ),
        (
            json!({ "project": "demo", "draft": true, "effort": "ultra" }),
            "error",
            vec!["ultra", "low, medium, high, xhigh, max"],
        ),
        (
            json!({ "project": "demo", "mode": "yolo" }),
            "error",
            vec!["yolo", "auto, default, plan, acceptEdits"],
        ),
        // No permission request at all is the user's decision, taken in the window.
        (
            json!({ "project": "demo", "mode": "bypassPermissions" }),
            "refused",
            vec!["bypassPermissions", "Réglages › Kanban"],
        ),
        (
            json!({ "project": "demo", "target": "nope" }),
            "error",
            vec!["nope", "main", "develop"],
        ),
        (
            json!({ "project": "demo", "target": "origin/main" }),
            "error",
            vec!["origin/main"],
        ),
        (
            json!({ "project": "demo", "target": "  " }),
            "error",
            vec!["vide"],
        ),
        (
            json!({ "project": "demo", "maxParallel": 0 }),
            "error",
            vec!["1", "6"],
        ),
        (
            json!({ "project": "demo", "maxParallel": 7 }),
            "error",
            vec!["1", "6"],
        ),
        // The server runs no command: the test command is the user's, set in the window.
        (
            json!({ "project": "demo", "testsFirst": true }),
            "refused",
            vec!["commande de test", "Réglages › Kanban"],
        ),
    ];
    for (args, outcome, says) in cases {
        let said = refused(&c, "set_board", args.clone()).await;
        for part in says {
            assert!(said.contains(part), "{args}: {said}");
        }
        let logged = entry(&h);
        assert_eq!(
            (logged.tool.as_str(), logged.outcome),
            ("set_board", outcome),
            "{args}"
        );
        assert_eq!(logged.message.as_deref(), Some(said.as_str()), "{args}");
        assert_eq!(board_of(&h, &p), before, "{args}");
    }
    assert_eq!(project_events(&h, &p), seen);
    // A project that is none.
    let none = refused(&c, "set_board", json!({ "project": "nope", "draft": true })).await;
    assert!(none.contains("Aucun projet « nope »"), "{none}");

    // The boundaries are in: one agent in parallel, six; the app's mode by an empty one; no tests
    // first, with no command, is no change to refuse.
    let v = read(
        &c,
        "set_board",
        json!({ "project": "demo", "maxParallel": 1, "testsFirst": false, "mode": "" }),
    )
    .await;
    assert_eq!((&v["maxParallel"], &v["mode"]), (&json!(1), &json!("")));
    read(
        &c,
        "set_board",
        json!({ "project": "demo", "maxParallel": 6 }),
    )
    .await;
    assert_eq!(board_of(&h, &p).max_parallel, 6);
    for effort in ["low", "medium", "high", "xhigh", "max"] {
        let v = read(
            &c,
            "set_board",
            json!({ "project": "demo", "effort": effort }),
        )
        .await;
        assert_eq!(v["effort"], effort);
    }
    for mode in ["auto", "default", "plan", "acceptEdits"] {
        let v = read(&c, "set_board", json!({ "project": "demo", "mode": mode })).await;
        assert_eq!(v["mode"], mode);
    }
    // A model by the name the window offers, in any case, or by the name it stands for.
    let v = read(
        &c,
        "set_board",
        json!({ "project": "demo", "model": " OPUS " }),
    )
    .await;
    assert_eq!(v["model"], "opus");
    let v = read(
        &c,
        "set_board",
        json!({ "project": "demo", "model": "claude-sonnet-9" }),
    )
    .await;
    assert_eq!(v["model"], "claude-sonnet-9");
    let v = read(&c, "set_board", json!({ "project": "demo", "model": "" })).await;
    assert_eq!(v["model"], "");

    // Bypassing the permissions is not given here, but a Kanban the user set to it keeps it, and
    // can be taken off it.
    edit_board(&h, &p, |b| b.mode = "bypassPermissions".into());
    let v = read(
        &c,
        "set_board",
        json!({ "project": "demo", "mode": "bypassPermissions", "draft": true }),
    )
    .await;
    assert_eq!(
        (&v["mode"], &v["draft"]),
        (&json!("bypassPermissions"), &json!(true))
    );
    let v = read(
        &c,
        "set_board",
        json!({ "project": "demo", "mode": "plan" }),
    )
    .await;
    assert_eq!(v["mode"], "plan");
    // Once off it, it is the user's again.
    let again = refused(
        &c,
        "set_board",
        json!({ "project": "demo", "mode": "bypassPermissions" }),
    )
    .await;
    assert!(again.contains("Réglages › Kanban"), "{again}");
    assert_eq!(board_of(&h, &p).mode, "plan");

    // The command is the user's; with one, tests first is a setting like the others, and the
    // command stays what it is.
    edit_board(&h, &p, |b| b.test_command = "npm test".into());
    let v = read(
        &c,
        "set_board",
        json!({ "project": "demo", "testsFirst": true }),
    )
    .await;
    assert_eq!(
        (&v["testsFirst"], &v["testCommand"]),
        (&json!(true), &json!("npm test"))
    );
    let kept = board_of(&h, &p);
    assert_eq!(
        (kept.prefix.as_str(), kept.next_number, kept.target.as_str()),
        ("DEM", 4, "main")
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

/// What `get_board` answers for `project`, given back to `set_board` as it is: accepted, answered
/// the same, the Kanban as it was. Returns what `get_board` said.
async fn board_taken_back(h: &Harness, c: &Client, p: &Project, project: &str) -> Value {
    let before = board_of(h, p);
    let got = read(c, "get_board", json!({ "project": project })).await;
    let said = read(c, "set_board", got.clone()).await;
    assert_eq!(said, got);
    assert_eq!(board_of(h, p), before, "{got}");
    got
}

#[tokio::test]
async fn a_kanban_with_no_target_takes_back_the_branch_get_board_shows_and_leaves_none_on_a_detached_head(
) {
    let h = harness("mcp-act-back-target");
    let p = project(&h).await;
    let repo = h.dir.join("repo");
    let c = external(&h).await;

    // No target yet: the branch the project is on is shown, and kept off the Kanban.
    let got = board_taken_back(&h, &c, &p, "demo").await;
    assert_eq!(got["target"], "main");
    assert_eq!(board_of(&h, &p).target, "");
    // The same asked by hand, around with spaces.
    let v = read(
        &c,
        "set_board",
        json!({ "project": "demo", "target": " main " }),
    )
    .await;
    assert_eq!(v["target"], "main");
    assert_eq!(board_of(&h, &p).target, "");

    // A detached HEAD: no branch to show, the target is empty, and giving it back is no refusal.
    git(&repo, &["checkout", "-q", "--detach"]);
    let got = board_taken_back(&h, &c, &p, "demo").await;
    assert_eq!(got["target"], "");
    let v = read(&c, "set_board", json!({ "project": "demo", "target": "" })).await;
    assert_eq!(v["target"], "");
    assert_eq!(board_of(&h, &p).target, "");
    // Any other branch is checked all the same, and a real one is set.
    let said = refused(
        &c,
        "set_board",
        json!({ "project": "demo", "target": "nope" }),
    )
    .await;
    assert!(said.contains("nope") && said.contains("main"), "{said}");
    assert_eq!(board_of(&h, &p).target, "");
    let v = read(
        &c,
        "set_board",
        json!({ "project": "demo", "target": "main" }),
    )
    .await;
    assert_eq!(v["target"], "main");
    assert_eq!(board_of(&h, &p).target, "main");
    assert_eq!(
        entry(&h).message.as_deref(),
        Some("target: (aucune) → main")
    );
    // Once the Kanban has one, an empty target is a refusal again.
    let said = refused(&c, "set_board", json!({ "project": "demo", "target": "" })).await;
    assert!(said.contains("vide"), "{said}");

    // A repository without a commit: its branch is shown, though git lists none.
    let fresh = other_project(&h, "vide").await;
    let got = board_taken_back(&h, &c, &fresh, "vide").await;
    assert!(!got["target"].as_str().unwrap().is_empty(), "{got}");
    assert_eq!(board_of(&h, &fresh).target, "");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_kanban_stored_with_tests_first_and_no_test_command_is_read_off_and_taken_back_as_it_is()
{
    let h = harness("mcp-act-back-tests-first");
    let p = project(&h).await;
    edit_board(&h, &p, |b| {
        b.target = "main".into();
        b.tests_first = true;
        b.test_command = "  ".into();
    });
    let c = external(&h).await;

    // The window shows it off, the validation ignores it: so does get_board, and the Kanban is
    // left as it is when that is given back.
    let got = board_taken_back(&h, &c, &p, "demo").await;
    assert_eq!(got["testsFirst"], false);
    assert!(board_of(&h, &p).tests_first);
    // Asked on: it is on already, nothing changes (the window shows it off all the same).
    let v = read(
        &c,
        "set_board",
        json!({ "project": "demo", "testsFirst": true }),
    )
    .await;
    assert_eq!(v["testsFirst"], false);
    assert!(board_of(&h, &p).tests_first);
    // Asked off, as get_board shows it: it stays as it is too.
    let v = read(
        &c,
        "set_board",
        json!({ "project": "demo", "testsFirst": false, "draft": true }),
    )
    .await;
    assert_eq!(
        (&v["testsFirst"], &v["draft"]),
        (&json!(false), &json!(true))
    );
    assert!(board_of(&h, &p).tests_first);

    // Off for good it cannot be turned on with no command: the user's to give, in the window.
    edit_board(&h, &p, |b| b.tests_first = false);
    let before = board_of(&h, &p);
    let said = refused(
        &c,
        "set_board",
        json!({ "project": "demo", "testsFirst": true }),
    )
    .await;
    assert!(said.contains("commande de test"), "{said}");
    assert_eq!(board_of(&h, &p), before);

    // With a command it is what get_board says, and what is given back changes nothing.
    edit_board(&h, &p, |b| {
        b.tests_first = true;
        b.test_command = "npm test".into();
    });
    let got = board_taken_back(&h, &c, &p, "demo").await;
    assert_eq!(got["testsFirst"], true);
    let v = read(
        &c,
        "set_board",
        json!({ "project": "demo", "testsFirst": false }),
    )
    .await;
    assert_eq!(v["testsFirst"], false);
    assert!(!board_of(&h, &p).tests_first);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_target_branch_deleted_since_is_taken_back_as_get_board_shows_it_and_no_other_missing_one_is_taken(
) {
    let h = harness("mcp-act-back-deleted-target");
    let p = project(&h).await;
    git(&h.dir.join("repo"), &["branch", "develop"]);
    edit_board(&h, &p, |b| b.target = "gone".into());
    let c = external(&h).await;

    // It is not a local branch any more, but it is the Kanban's: shown, and given back as it is.
    let got = board_taken_back(&h, &c, &p, "demo").await;
    assert_eq!(got["target"], "gone");
    let v = read(
        &c,
        "set_board",
        json!({ "project": "demo", "target": "gone", "draft": true }),
    )
    .await;
    assert_eq!((&v["target"], &v["draft"]), (&json!("gone"), &json!(true)));
    assert_eq!(board_of(&h, &p).target, "gone");
    // Another one that is missing stays a refusal, the Kanban's own is not touched.
    let before = board_of(&h, &p);
    let said = refused(
        &c,
        "set_board",
        json!({ "project": "demo", "target": "nope" }),
    )
    .await;
    assert!(
        said.contains("nope") && said.contains("main") && said.contains("develop"),
        "{said}"
    );
    assert_eq!(board_of(&h, &p), before);
    // A branch that exists replaces it.
    let v = read(
        &c,
        "set_board",
        json!({ "project": "demo", "target": "develop" }),
    )
    .await;
    assert_eq!(v["target"], "develop");
    assert_eq!(board_of(&h, &p).target, "develop");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agent_sets_the_board_of_the_projects_that_are_not_private_and_claude_outside_escouade_any(
) {
    let h = harness("mcp-act-set-board-own");
    let p = project(&h).await;
    let other = other_project(&h, "autre").await;
    let vault = other_project(&h, "coffre").await;
    let vault = make_private(&h, &vault);
    let mine = put_agent(&h, &p, "mon-agent", 1);
    let c = as_agent(&h, &mine.id).await;

    // Its own, and another project that is not private.
    let v = read(&c, "set_board", json!({ "project": "demo", "draft": true })).await;
    assert_eq!(v["draft"], true);
    assert!(board_of(&h, &p).draft);
    assert_eq!(entry(&h).caller, "mon-agent");
    assert_eq!(
        read(&c, "get_board", json!({ "project": "autre" })).await["draft"],
        false
    );
    let v = read(
        &c,
        "set_board",
        json!({ "project": "autre", "draft": true }),
    )
    .await;
    assert_eq!(v["projectId"], other.id.as_str());
    assert!(board_of(&h, &other).draft);
    let logged = entry(&h);
    assert_eq!(
        (logged.caller.as_str(), logged.tool.as_str(), logged.outcome),
        ("mon-agent", "set_board", "ok")
    );
    // A private project is not found, and its Kanban stays as it was.
    let vault_before = board_of(&h, &vault);
    let why = refused(
        &c,
        "set_board",
        json!({ "project": vault.id, "draft": true }),
    )
    .await;
    assert!(why.starts_with("Aucun projet «"), "{why}");
    assert!(!why.contains("coffre"), "{why}");
    assert_eq!(board_of(&h, &vault), vault_before);
    c.cancel().await.unwrap();
    // Nor does its own agent set another project's.
    let inside = put_agent(&h, &vault, "enferme", 2);
    let c = as_agent(&h, &inside.id).await;
    let p_before = board_of(&h, &p);
    let why = refused(
        &c,
        "set_board",
        json!({ "project": "demo", "maxParallel": 5 }),
    )
    .await;
    assert!(why.starts_with("Aucun projet « demo »"), "{why}");
    assert_eq!(board_of(&h, &p), p_before);
    c.cancel().await.unwrap();

    // Claude outside Escouade is the user's: it sets any project's.
    let outside = external(&h).await;
    let v = read(
        &outside,
        "set_board",
        json!({ "project": "autre", "maxParallel": 3, "action": "push" }),
    )
    .await;
    assert_eq!(
        (&v["project"], &v["projectId"], &v["maxParallel"]),
        (&json!("autre"), &json!(other.id), &json!(3))
    );
    let theirs = board_of(&h, &other);
    assert_eq!((theirs.action.as_str(), theirs.max_parallel), ("push", 3));
    // Not the other's.
    assert_eq!(board_of(&h, &p).max_parallel, 2);
    outside.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_call_that_changes_nothing_says_so_even_when_it_gives_the_values_the_kanban_has() {
    let h = harness("mcp-act-set-board-same");
    let p = project(&h).await;
    edit_board(&h, &p, |b| b.target = "main".into());
    let c = external(&h).await;
    let before = board_of(&h, &p);
    let v = read(
        &c,
        "set_board",
        json!({
            "project": "demo", "action": "merge", "target": "main", "draft": false,
            "maxParallel": 2, "model": "", "mode": "",
        }),
    )
    .await;
    assert_eq!(v["action"], "merge");
    assert_eq!(board_of(&h, &p), before);
    let logged = entry(&h);
    assert_eq!(
        (logged.outcome, logged.message.as_deref()),
        (
            "ok",
            Some("Aucun changement : le Kanban était déjà réglé ainsi.")
        )
    );
    assert!(
        logged.summary.contains("draft: false"),
        "{}",
        logged.summary
    );
    // One that changes something says that alone, whatever else it gave.
    read(
        &c,
        "set_board",
        json!({ "project": "demo", "action": "merge", "draft": true, "maxParallel": 2 }),
    )
    .await;
    assert_eq!(entry(&h).message.as_deref(), Some("draft: false → true"));
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[test]
fn what_a_set_board_changed_is_said_in_the_language_of_the_interface_each_setting_from_its_value_to_its_new_one(
) {
    use crate::i18n::check::french_in;
    use crate::i18n::Lang::{En, Fr};
    let before = BoardSettings {
        model: "opus".into(),
        test_command: "npm test".into(),
        ..Default::default()
    };
    let after = BoardSettings {
        target: "main".into(),
        model: String::new(),
        effort: "high".into(),
        mode: "plan".into(),
        tests_first: true,
        ..before.clone()
    };
    let fr = board_changes(Fr, &before, &after);
    let en = board_changes(En, &before, &after);
    assert_eq!(
        fr,
        "target: (aucune) → main · model: opus → (défaut) · effort: (défaut) → high · mode: (défaut) → plan · testsFirst: false → true"
    );
    assert_eq!(
        en,
        "target: (none) → main · model: opus → (default) · effort: (default) → high · mode: (default) → plan · testsFirst: false → true"
    );
    assert_eq!(french_in(&en), None);
    // Tests first with no command is off, as the window and get_board show it: nothing to say.
    let stored = BoardSettings {
        tests_first: true,
        ..Default::default()
    };
    for lang in [Fr, En] {
        let none = board_changes(lang, &stored, &BoardSettings::default());
        assert_eq!(none, board_changes(lang, &stored, &stored));
    }
    assert_eq!(
        board_changes(Fr, &stored, &stored),
        "Aucun changement : le Kanban était déjà réglé ainsi."
    );
    assert_eq!(
        board_changes(En, &stored, &stored),
        "No change: the Kanban was already set so."
    );
    // The key's prefix and number are not settings of the tool: never said.
    let numbered = BoardSettings {
        prefix: "DEM".into(),
        next_number: 9,
        ..Default::default()
    };
    assert_eq!(
        board_changes(En, &BoardSettings::default(), &numbered),
        "No change: the Kanban was already set so."
    );
}

#[test]
fn the_refusals_of_set_board_are_written_in_the_language_of_the_interface() {
    use crate::i18n::check::french_in;
    use crate::i18n::Lang::{En, Fr};
    // A Kanban with a target: an empty one is then a refusal.
    let board = BoardSettings {
        target: "main".into(),
        ..Default::default()
    };
    let branches = ["main".to_string(), "develop".to_string()];
    let said = |lang, args: Value| {
        let a: SetBoardArgs = serde_json::from_value(args).unwrap();
        match board_change(lang, &board, &branches, "main", &a, None) {
            Err(ToolError::Refused(m)) => ("refused", m),
            Err(ToolError::Failed(m)) => ("error", m),
            Ok(_) => panic!("accepted"),
        }
    };
    let cases = [
        (json!({ "project": "p" }), "error"),
        (json!({ "project": "p", "effort": "ultra" }), "error"),
        (json!({ "project": "p", "mode": "yolo" }), "error"),
        (
            json!({ "project": "p", "mode": "bypassPermissions" }),
            "refused",
        ),
        (json!({ "project": "p", "target": "nope" }), "error"),
        (json!({ "project": "p", "target": " " }), "error"),
        (json!({ "project": "p", "maxParallel": 9 }), "error"),
        (json!({ "project": "p", "testsFirst": true }), "refused"),
    ];
    for (args, kind) in cases {
        let (fr_kind, fr) = said(Fr, args.clone());
        let (en_kind, en) = said(En, args.clone());
        assert_eq!((fr_kind, en_kind), (kind, kind), "{args}");
        assert_ne!(fr, en, "{args}");
        assert_eq!(french_in(&en), None, "{args}: {en}");
    }
    assert_eq!(
        said(En, json!({ "project": "p", "mode": "bypassPermissions" })).1,
        "Running agents with no permission request at all is the user’s decision: set the mode bypassPermissions in the window (Settings › Kanban)."
    );
    assert_eq!(
        said(En, json!({ "project": "p", "effort": "ultra" })).1,
        "Unknown effort “ultra”. Efforts: low, medium, high, xhigh, max."
    );
    assert_eq!(
        said(En, json!({ "project": "p", "target": "nope" })).1,
        "“nope” is not a local branch of the project. Branches: main, develop."
    );
    assert_eq!(
        said(En, json!({ "project": "p", "testsFirst": true })).1,
        "The Kanban has no test command, and the server runs none: set it in the window (Settings › Kanban), then turn on “Re-run the tests first”."
    );
    // The most branches listed, for a repository that has hundreds.
    let many: Vec<String> = (0..60).map(|i| format!("b{i}")).collect();
    let a: SetBoardArgs =
        serde_json::from_value(json!({ "project": "p", "target": "nope" })).unwrap();
    let Err(ToolError::Failed(listed)) = board_change(En, &board, &many, "b0", &a, None) else {
        panic!("accepted")
    };
    assert!(
        listed.contains("b19") && !listed.contains("b20"),
        "{listed}"
    );
    assert!(listed.contains("60"), "{listed}");
}

#[tokio::test]
async fn a_ticket_is_launched_with_a_model_and_an_effort_of_its_own_and_the_answer_says_what_its_agent_goes_with(
) {
    let h = harness("mcp-act-start-model");
    let p = project(&h).await;
    known_models(&h);
    let a = made(&h, &p, "Architecture [ok]").await;
    let b = made(&h, &p, "Petit [ok]").await;
    let third = made(&h, &p, "Moyen [ok]").await;
    let c = external(&h).await;

    // Refused before anything is written, or started: the ticket is as it was.
    for (args, says) in [
        (
            json!({ "ticket": a.key, "model": "gpt" }),
            vec!["gpt", "sonnet", "opus"],
        ),
        (
            json!({ "ticket": a.key, "model": "opus", "effort": "ultra" }),
            vec!["ultra", "low, medium, high, xhigh, max"],
        ),
        (json!({ "ticket": a.key, "effort": "huge" }), vec!["huge"]),
    ] {
        let said = refused(&c, "start_ticket", args.clone()).await;
        for part in says {
            assert!(said.contains(part), "{args}: {said}");
        }
        assert_eq!(entry(&h).outcome, "error", "{args}");
        let t = stored(&h, &a.id);
        assert_eq!(
            (t.column, t.forced, t.model.as_str(), t.effort.as_str()),
            (Column::Todo, false, "", ""),
            "{args}"
        );
    }

    // Its own, for this ticket alone: written on it, and answered.
    let v = read(
        &c,
        "start_ticket",
        json!({ "ticket": "dem-1", "model": " opus ", "effort": "max" }),
    )
    .await;
    assert_eq!(
        v,
        json!({ "key": "DEM-1", "starting": true, "model": "opus", "effort": "max" })
    );
    let t = stored(&h, &a.id);
    assert_eq!((t.model.as_str(), t.effort.as_str()), ("opus", "max"));
    let logged = entry(&h);
    assert_eq!(
        (logged.tool.as_str(), logged.outcome),
        ("start_ticket", "ok")
    );
    assert_eq!(
        logged.summary,
        "ticket: dem-1 · model:  opus  · effort: max"
    );

    // Without them nothing is written, and the answer says what its agent takes: the app's.
    let v = read(&c, "start_ticket", json!({ "ticket": b.key })).await;
    assert_eq!(
        v,
        json!({ "key": "DEM-2", "starting": true, "model": "sonnet", "effort": "medium" })
    );
    let t = stored(&h, &b.id);
    assert_eq!((t.model.as_str(), t.effort.as_str()), ("", ""));
    assert_eq!(entry(&h).summary, "ticket: DEM-2");

    // Each goes with what it was told (the agent of a ticket is made when its turn comes).
    for id in [&a.id, &b.id] {
        h.wait("its agent to be made", |h| stored(h, id).agent_id.is_some())
            .await;
    }
    let agent = |id: &str| h.agent(&stored(&h, id).agent_id.unwrap());
    let (first, second) = (agent(&a.id), agent(&b.id));
    assert_eq!(
        (first.model.as_str(), first.effort.as_str()),
        ("opus", "max")
    );
    assert_eq!(
        (second.model.as_str(), second.effort.as_str()),
        ("sonnet", "medium")
    );

    // The Kanban's comes before the app's, and a ticket's own before the Kanban's, each of the
    // two apart.
    h.core
        .board_set(
            &p.id,
            BoardSettings {
                model: "opus".into(),
                effort: "high".into(),
                max_parallel: 6,
                ..board_of(&h, &p)
            },
        )
        .unwrap();
    let v = read(
        &c,
        "start_ticket",
        json!({ "ticket": third.key, "model": "sonnet" }),
    )
    .await;
    assert_eq!(
        v,
        json!({ "key": "DEM-3", "starting": true, "model": "sonnet", "effort": "high" })
    );
    let t = stored(&h, &third.id);
    assert_eq!((t.model.as_str(), t.effort.as_str()), ("sonnet", ""));

    h.wait("its agent to be made", |h| {
        stored(h, &third.id).agent_id.is_some()
    })
    .await;
    let last = agent(&third.id);
    assert_eq!(
        (last.model.as_str(), last.effort.as_str()),
        ("sonnet", "high")
    );
    for id in [&a.id, &b.id, &third.id] {
        h.wait("its turn to end", |h| {
            stored(h, id).column == Column::Review
        })
        .await;
    }
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_answer_of_start_ticket_has_no_effort_when_the_model_of_its_agent_takes_none() {
    let h = harness("mcp-act-start-haiku");
    let p = project(&h).await;
    known_models(&h);
    let a = made(&h, &p, "Petit [ok]").await;
    let b = made(&h, &p, "Autre [ok]").await;
    let c = external(&h).await;

    // Haiku takes no effort: given or not, it is not the agent's, and the answer leaves it out.
    let v = read(
        &c,
        "start_ticket",
        json!({ "ticket": a.key, "model": "haiku", "effort": "high" }),
    )
    .await;
    assert_eq!(
        v,
        json!({ "key": "DEM-1", "starting": true, "model": "haiku" })
    );
    // The same for a Kanban on haiku, with a ticket that says nothing of its own.
    h.core
        .board_set(
            &p.id,
            BoardSettings {
                model: "haiku".into(),
                effort: "low".into(),
                ..board_of(&h, &p)
            },
        )
        .unwrap();
    let v = read(&c, "start_ticket", json!({ "ticket": b.key })).await;
    assert_eq!(
        v,
        json!({ "key": "DEM-2", "starting": true, "model": "haiku" })
    );
    for id in [&a.id, &b.id] {
        h.wait("its turn to end", |h| {
            stored(h, id).column == Column::Review
        })
        .await;
    }
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

// ---------- the account, the model and the effort of a ticket (ESC-153) ----------

#[tokio::test]
async fn a_tickets_account_model_and_effort_are_set_read_and_given_at_its_start() {
    let h = harness("mcp-act-ticket-account");
    let p = project(&h).await;
    with_pro(&h);
    let c = external(&h).await;
    // Made on Pro, named by its name in any case, on opus at its highest effort.
    let v = read(
        &c,
        "create_ticket",
        json!({ "project": p.id, "title": "Sur Pro [ok]", "account": "pro", "model": "opus", "effort": "MAX" }),
    )
    .await;
    assert_eq!(
        (&v["account"], &v["accountId"], &v["model"], &v["effort"]),
        (&json!("Pro"), &json!("pro"), &json!("opus"), &json!("max"))
    );
    let t = stored(&h, v["id"].as_str().unwrap());
    assert_eq!(
        (t.account.as_str(), t.model.as_str(), t.effort.as_str()),
        ("pro", "opus", "max")
    );
    // One left to « Auto » says so; the list says only what is not.
    let auto = read(
        &c,
        "create_ticket",
        json!({ "project": p.id, "title": "Auto [ok]" }),
    )
    .await;
    assert_eq!(
        (
            &auto["account"],
            auto.get("accountId"),
            &auto["model"],
            &auto["effort"]
        ),
        (&json!("auto"), None, &json!("auto"), &json!("auto"))
    );
    let rows = read(&c, "list_tickets", json!({ "project": p.id })).await;
    assert_eq!(
        (&rows[0]["account"], &rows[0]["model"], &rows[0]["effort"]),
        (&json!("Pro"), &json!("opus"), &json!("max"))
    );
    assert_eq!(
        (
            rows[1].get("account"),
            rows[1].get("model"),
            rows[1].get("effort")
        ),
        (None, None, None)
    );
    // Back to « Auto » by an update, the account alone; then the effort alone.
    let v = read(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "account": "AUTO" }),
    )
    .await;
    assert_eq!(
        (&v["account"], &v["model"], &v["effort"]),
        (&json!("auto"), &json!("opus"), &json!("max"))
    );
    let v = read(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "effort": "auto", "account": "Pro" }),
    )
    .await;
    assert_eq!(
        (&v["account"], &v["effort"]),
        (&json!("Pro"), &json!("auto"))
    );
    let got = read(&c, "get_ticket", json!({ "ticket": t.key })).await;
    assert_eq!(
        (&got["account"], &got["model"], &got["effort"]),
        (&json!("Pro"), &json!("opus"), &json!("auto"))
    );
    // An effort that is none, an account reserved to another project: refused, with why.
    let e = refused(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "effort": "énorme" }),
    )
    .await;
    assert!(e.starts_with("Effort « énorme » inconnu."), "{e}");
    let q = other_project(&h, "autre").await;
    h.set_settings(|s| {
        for a in s.accounts.iter_mut().filter(|a| a.id == "pro") {
            a.projects = vec![q.id.clone()];
        }
    });
    let e = refused(
        &c,
        "create_ticket",
        json!({ "project": p.id, "title": "Sur Pro [ok]", "account": "pro" }),
    )
    .await;
    assert_eq!(e, "Le compte « Pro » est réservé à d’autres projets.");
    let e = refused(
        &c,
        "start_ticket",
        json!({ "ticket": auto["key"], "account": "pro" }),
    )
    .await;
    assert_eq!(e, "Le compte « Pro » est réservé à d’autres projets.");
    h.set_settings(|s| {
        for a in s.accounts.iter_mut().filter(|a| a.id == "pro") {
            a.projects.clear();
        }
    });
    // Started on Pro by start_ticket: it is the ticket's, and its agent goes there.
    read(
        &c,
        "start_ticket",
        json!({ "ticket": auto["key"], "account": "Pro" }),
    )
    .await;
    let id = auto["id"].as_str().unwrap();
    assert_eq!(stored(&h, id).account, "pro");
    h.wait("its agent to be made", |h| stored(h, id).agent_id.is_some())
        .await;
    assert_eq!(h.agent(&stored(&h, id).agent_id.unwrap()).account, "pro");
    h.wait("its turn to end", |h| {
        stored(h, id).column == Column::Review
    })
    .await;
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

/// The shared Kanban (ESC-190, choice 130): the ticket tools say the sharing, `update_ticket` shares, and an agent never
/// starts nor deletes a ticket another one of the team shared.
#[tokio::test]
async fn a_shared_ticket_is_said_by_the_tools_and_another_ones_is_never_started_by_them() {
    let h = harness("mcp-act-shared");
    {
        let mut s = h.core.settings.write();
        s.user_name = "Bob Durand".into();
        s.user_id = "5d0e77b1".into();
    }
    let p = project(&h).await;
    let mine = made(&h, &p, "Le mien").await;
    let hers = made(&h, &p, "Le sien").await;
    let c = external(&h).await;
    // Shared by `update_ticket` alone: its file written, said by `get_ticket`.
    let v = read(
        &c,
        "update_ticket",
        json!({ "ticket": mine.key, "shared": true }),
    )
    .await;
    assert_eq!(v["shared"]["by"], "Bob Durand");
    assert_eq!(v["shared"]["byMe"], true);
    let file = v["shared"]["file"].as_str().unwrap().to_string();
    assert!(file.starts_with(".escouade/tickets/"), "{file}");
    assert!(std::path::Path::new(&p.path).join(&file).is_file());
    // Another one's ticket, as a pull brings it.
    {
        let mut tickets = h.core.tickets.write();
        let t = tickets.iter_mut().find(|t| t.id == hers.id).unwrap();
        t.shared = Some(Shared {
            id: "h8m2v6kd".into(),
            key: t.key.clone(),
            author: Person {
                name: "Alice Martin".into(),
                id: "a91f03c2".into(),
            },
            mine: false,
            ..Default::default()
        });
    }
    let v = read(&c, "get_ticket", json!({ "ticket": hers.key })).await;
    assert_eq!(v["shared"]["by"], "Alice Martin");
    assert_eq!(v["shared"]["byMe"], false);
    let start = refused(&c, "start_ticket", json!({ "ticket": hers.key })).await;
    assert!(start.contains("partagé par Alice Martin"), "{start}");
    assert!(start.contains("à la main"), "{start}");
    let delete = refused(&c, "delete_ticket", json!({ "ticket": hers.key })).await;
    assert!(delete.contains("partagé par Alice Martin"), "{delete}");
    assert_eq!(stored(&h, &hers.id).column, Column::Todo);
    // The list says it of both.
    let rows = read(&c, "list_tickets", json!({ "project": p.name })).await;
    let shared: Vec<bool> = rows
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r.get("shared").is_some())
        .collect();
    assert_eq!(shared, [true, true]);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

/// A Codex account on the fake `codex`, its models read (ESC-186), and the project `p` on Codex.
async fn on_codex(h: &Harness, p: &Project) {
    let home = h.dir.join("codex-work");
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "codex-work".into(),
            name: "Codex".into(),
            engine: EngineId::Codex,
            config_dir: home.to_string_lossy().into(),
            claude_path: crate::engine::fake_codex(),
            active: true,
            ..Default::default()
        })
    });
    h.core.read_account_models("codex-work").await;
    h.core
        .projects
        .write()
        .iter_mut()
        .find(|x| x.id == p.id)
        .unwrap()
        .engine = Some(EngineId::Codex);
}

#[tokio::test]
async fn the_model_and_the_effort_are_checked_against_the_harness_of_the_project_that_carries_them()
{
    let h = harness("mcp-act-codex-models");
    let p = project(&h).await;
    // Claude Code's models are known too: they must not be what a Codex project is held to.
    known_models(&h);
    on_codex(&h, &p).await;
    let c = external(&h).await;

    // list_projects tells the harness of the project and what its tools take for it.
    let listed = read(&c, "list_projects", json!({})).await;
    let row = &listed[0];
    assert_eq!(row["engine"], "codex");
    let models: Vec<&str> = row["models"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m.as_str().unwrap())
        .collect();
    assert!(
        models.contains(&"gpt-6-luna") && !models.contains(&"sonnet"),
        "{row}"
    );
    assert!(
        row["efforts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e == "max"),
        "{row}"
    );

    // A model of Codex, by its name as the menu shows it: the agent goes on it, not on the default.
    let v = read(
        &c,
        "create_agent",
        json!({ "project": p.id, "message": "Bonjour", "model": "GPT-5.6-Terra" }),
    )
    .await;
    let id = v["id"].as_str().unwrap();
    let agent = h.agent(id);
    assert_eq!(
        (agent.engine, agent.model.as_str()),
        (EngineId::Codex, "gpt-5.6-terra")
    );
    turn_over(&h, id).await;

    // A model of Claude is refused, with the models of Codex.
    let said = refused(
        &c,
        "create_agent",
        json!({ "project": p.id, "message": "x", "model": "sonnet" }),
    )
    .await;
    for part in [
        "sonnet",
        "Codex",
        "compte Codex",
        "gpt-6-luna",
        "gpt-5.6-terra",
    ] {
        assert!(said.contains(part), "{said}");
    }
    assert!(!said.contains("opus"), "{said}");

    // The Kanban: a model of Codex goes in, its effort is one of that model's.
    let said = refused(&c, "set_board", json!({ "project": p.id, "model": "opus" })).await;
    assert!(
        said.contains("Codex") && said.contains("gpt-6-luna"),
        "{said}"
    );
    let said = refused(
        &c,
        "set_board",
        json!({ "project": p.id, "model": "gpt-5.6-terra", "effort": "max" }),
    )
    .await;
    assert!(
        said.contains("Codex") && said.contains("low, medium, high"),
        "{said}"
    );
    assert!(!said.contains("xhigh"), "{said}");
    let v = read(
        &c,
        "set_board",
        json!({ "project": p.id, "model": "gpt-6-luna", "effort": "max" }),
    )
    .await;
    assert_eq!(
        (v["model"].as_str(), v["effort"].as_str()),
        (Some("gpt-6-luna"), Some("max"))
    );
    let board = h.core.project(&p.id).unwrap().board;
    assert_eq!(
        (board.model.as_str(), board.effort.as_str()),
        ("gpt-6-luna", "max")
    );

    // A ticket: its model and its effort too, when it is made and when it is changed.
    let said = refused(
        &c,
        "create_ticket",
        json!({ "project": p.id, "title": "Un", "model": "haiku" }),
    )
    .await;
    assert!(
        said.contains("Codex") && said.contains("gpt-6-luna"),
        "{said}"
    );
    let v = read(
        &c,
        "create_ticket",
        json!({ "project": p.id, "title": "Deux [ok]", "model": "gpt-6-luna", "effort": "xhigh" }),
    )
    .await;
    let key = v["key"].as_str().unwrap().to_string();
    let said = refused(
        &c,
        "update_ticket",
        json!({ "ticket": key, "model": "gpt-5.6-terra", "effort": "xhigh" }),
    )
    .await;
    assert!(
        said.contains("Codex") && said.contains("low, medium, high"),
        "{said}"
    );
    let v = read(
        &c,
        "update_ticket",
        json!({ "ticket": key, "model": "GPT-5.6-TERRA", "effort": "high" }),
    )
    .await;
    assert_eq!(
        (v["model"].as_str(), v["effort"].as_str()),
        (Some("gpt-5.6-terra"), Some("high"))
    );

    // Started: nothing is written on a model that does not fit, and the agent runs on the one asked.
    let said = refused(
        &c,
        "start_ticket",
        json!({ "ticket": key, "model": "opus" }),
    )
    .await;
    assert!(said.contains("Codex"), "{said}");
    let t = h
        .core
        .tickets
        .read()
        .iter()
        .find(|t| t.key == key)
        .cloned()
        .unwrap();
    assert_eq!(t.column, Column::Todo);
    let v = read(
        &c,
        "start_ticket",
        json!({ "ticket": key, "model": "gpt-6-luna", "effort": "max" }),
    )
    .await;
    assert_eq!(v["model"], "gpt-6-luna");
    let agent_id = wait_running_key(&h, &key).await;
    assert_eq!(h.agent(&agent_id).model, "gpt-6-luna");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

/// The id of the agent of the ticket `key`, once it has one.
async fn wait_running_key(h: &Harness, key: &str) -> String {
    let id = h
        .core
        .tickets
        .read()
        .iter()
        .find(|t| t.key == key)
        .unwrap()
        .id
        .clone();
    h.wait("the ticket's agent", |h| {
        h.core
            .tickets
            .read()
            .iter()
            .any(|t| t.id == id && t.agent_id.is_some())
    })
    .await;
    h.ticket(&id).agent_id.unwrap()
}

#[tokio::test]
async fn a_ticket_set_on_codex_in_a_claude_project_takes_the_models_of_codex_and_a_claude_project_stays_as_it_was(
) {
    let h = harness("mcp-act-ticket-harness");
    let p = project(&h).await;
    known_models(&h);
    on_codex(&h, &p).await;
    // Back on Claude Code (the default), the ticket alone on Codex (ESC-153).
    h.core
        .projects
        .write()
        .iter_mut()
        .find(|x| x.id == p.id)
        .unwrap()
        .engine = None;
    let mut t = ticket(&p, "DEM-9", "Codex [ok]", Column::Todo, 5);
    t.engine = Some(EngineId::Codex);
    let t = put(&h, t);
    let c = external(&h).await;

    let said = refused(
        &c,
        "start_ticket",
        json!({ "ticket": t.key, "model": "sonnet" }),
    )
    .await;
    assert!(
        said.contains("Codex") && said.contains("gpt-6-luna"),
        "{said}"
    );
    let v = read(
        &c,
        "start_ticket",
        json!({ "ticket": t.key, "model": "gpt-6-luna" }),
    )
    .await;
    assert_eq!(v["model"], "gpt-6-luna");

    // The project, on Claude Code, is held to Claude Code's models as before.
    let said = refused(
        &c,
        "set_board",
        json!({ "project": p.id, "model": "gpt-6-luna" }),
    )
    .await;
    assert!(said.contains("sonnet") && !said.contains("Codex"), "{said}");
    let said = refused(
        &c,
        "set_board",
        json!({ "project": p.id, "effort": "ultra" }),
    )
    .await;
    assert!(said.contains("low, medium, high, xhigh, max"), "{said}");
    read(
        &c,
        "set_board",
        json!({ "project": p.id, "model": "opus", "effort": "max" }),
    )
    .await;
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn several_tickets_started_one_after_the_other_all_start() {
    let h = harness("mcp-act-start-in-a-row");
    let p = project(&h).await;
    let a = made(&h, &p, "Un [ok]").await;
    let b = made(&h, &p, "Deux [ok]").await;
    let d = made(&h, &p, "Trois [ok]").await;
    // Its worktree's setup takes a while: the next start comes while the first is under way.
    let slow = if cfg!(windows) {
        "Start-Sleep -Seconds 2"
    } else {
        "sleep 2"
    };
    h.set_worktree_steps(&p.id, vec![crate::core_tests::wt_step(slow, "")], vec![]);
    let c = external(&h).await;
    for t in [&a, &b, &d] {
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        let v = read(&c, "start_ticket", json!({ "ticket": t.key })).await;
        assert_eq!(v["starting"], true);
    }
    for t in [&a, &b, &d] {
        h.wait("each ticket to start", |h| {
            stored(h, &t.id).column != Column::Todo
        })
        .await;
    }
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_the_board_does_not_start_after_the_tool_checked_says_why_and_stays_to_do() {
    let h = harness("mcp-act-start-met-limit");
    let p = project(&h).await;
    let a = made(&h, &p, "Un [ok]").await;
    let c = external(&h).await;
    let end = h.core.pause_now() + 3_600_000;
    // A pass of the board is under way (a launch in progress): the one this call asks for waits.
    let gate = h.core.board_lock.lock().await;
    let call = refused(&c, "start_ticket", json!({ "ticket": a.key }));
    // The usage limit is met after the tool's checks, before the pass comes to the ticket.
    let driver = async {
        while !stored(&h, &a.id).forced {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        window_used(&h, "principal", 100.0, end);
        drop(gate);
    };
    let (said, ()) = tokio::join!(call, driver);
    assert!(said.contains("100 %"), "{said}");
    let t = stored(&h, &a.id);
    assert_eq!(t.column, Column::Todo);
    assert!(!t.forced);
    assert_eq!(entry(&h).outcome, "refused");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}
