//! « Projet privé » (ESC-142): what Escouade passes itself from a project to another, out of the
//! MCP server (whose side is `mcp::private_tests`). A ticket comes after tickets of its own project
//! alone, a copy of an agent stays in its original's project, and what Escouade writes to the
//! agents of a ticket (their instructions, their messages, their conversation) names no other
//! project, private or not.

use crate::core_tests::{harness, repo, Harness};
use crate::model::*;
use crate::tickets::TicketDraft;
use crate::tickets_tests::draft;
use serde_json::Value;
use std::path::Path;

/// A project « coffre-fort » next to `demo`, made private as « Projet privé » saves it.
async fn vault(h: &Harness) -> Project {
    let r = repo(&h.dir.join("coffre"));
    let p = h
        .core
        .create_project(
            &r.to_string_lossy(),
            "coffre-fort",
            "oklch(0.7 0.1 200)",
            false,
            None,
            true,
        )
        .await
        .unwrap();
    let found = {
        let mut projects = h.core.projects.write();
        let found = projects.iter_mut().find(|x| x.id == p.id).unwrap();
        found.private = true;
        found.clone()
    };
    // Its harness turned on for it by the user (ESC-404).
    crate::harness_scope_tests::turn_on_every_harness(h, &found.id);
    found
}

fn after(title: &str, ids: &[&str]) -> TicketDraft {
    TicketDraft {
        after: ids.iter().map(|id| id.to_string()).collect(),
        ..draft(title, &[], 5)
    }
}

#[tokio::test]
async fn a_ticket_never_comes_after_a_ticket_of_a_private_project_nor_the_other_way() {
    let h = harness("private-after");
    let (open, _) = h.project(false).await;
    let vault = vault(&h).await;
    for p in [&open, &vault] {
        h.set_board(&p.id, |s| s.autopilot = false);
    }
    let secret = h
        .core
        .ticket_create(&vault.id, draft("Secret", &[], 5))
        .await
        .unwrap();
    let shared = h
        .core
        .ticket_create(&open.id, draft("Ouvert", &[], 5))
        .await
        .unwrap();
    // Made or edited, a ticket of each keeps none of the other's: the window offers its own
    // project's alone, and the core leaves out what another one sends.
    let made = h
        .core
        .ticket_create(&open.id, after("Après", &[&secret.id, &shared.id]))
        .await
        .unwrap();
    assert_eq!(made.after, [shared.id.as_str()]);
    let edited = h
        .core
        .ticket_update(&shared.id, after("Ouvert", &[&secret.id]))
        .unwrap();
    assert!(edited.after.is_empty());
    let edited = h
        .core
        .ticket_update(&secret.id, after("Secret", &[&shared.id, &made.id]))
        .unwrap();
    assert!(edited.after.is_empty());
    let made = h
        .core
        .ticket_create(&vault.id, after("Secret 2", &[&shared.id]))
        .await
        .unwrap();
    assert!(made.after.is_empty());
    // Nor is one put before the other's in its column: the reason is said.
    let e = h
        .core
        .ticket_move(&shared.id, crate::board::MoveTo::Before(&secret.id))
        .unwrap_err();
    assert_eq!(
        e.to_string(),
        format!("{} est dans un autre projet.", secret.key)
    );
}

#[tokio::test]
async fn a_copy_of_an_agent_stays_in_the_project_of_its_original_private_or_not() {
    let h = harness("private-copy");
    let (open, _) = h.project(false).await;
    let vault = vault(&h).await;
    for p in [&open, &vault] {
        let original = h.core.create_agent(&p.id, None).await.unwrap().meta;
        let copy = h.core.duplicate_agent(&original.id).await.unwrap().meta;
        assert_eq!(copy.project_id, p.id);
        assert_eq!(h.agent(&copy.id).project_id, p.id);
    }
    // Each project has its original and its copy, nothing of the other.
    for p in [&open, &vault] {
        let ids: Vec<String> = h
            .core
            .project_agents(&p.id)
            .iter()
            .map(|a| a.lock().meta.project_id.clone())
            .collect();
        assert_eq!(ids, [p.id.clone(), p.id.clone()]);
    }
}

/// Everything the fake CLI of the agent working in `dir` was given (its arguments, its folder,
/// the messages written to it), with its conversation, as one text.
fn told(h: &Harness, agent: &str, dir: &Path) -> String {
    let conv: Vec<Value> = h.core.agent(agent).unwrap().lock().conv.items();
    let all = [
        h.launch_log(dir),
        h.stdin_messages(dir),
        h.fake_log(dir, "control"),
        conv,
    ];
    serde_json::to_string(&all).unwrap()
}

#[tokio::test]
async fn what_escouade_writes_to_a_tickets_agent_names_no_other_project() {
    let h = harness("private-texts");
    let (open, _) = h.project(false).await;
    let vault = vault(&h).await;
    let mut tickets = Vec::new();
    for p in [&open, &vault] {
        let t = h
            .core
            .ticket_create(
                &p.id,
                draft(
                    "Ajouter le fichier",
                    &["Le fichier existe", "Il dit sa boucle"],
                    5,
                ),
            )
            .await
            .unwrap();
        tickets.push(t);
    }
    for t in &tickets {
        h.wait_ticket(&t.id, "ticket to test", |t| t.column == Column::Review)
            .await;
    }
    // What names a project: its name, its id, its folder, its tickets' keys.
    let names = |p: &Project, t: &Ticket| {
        vec![
            p.name.clone(),
            p.id.clone(),
            p.path.clone(),
            p.path.replace('\\', "\\\\"),
            format!("{}-", t.key.split('-').next().unwrap()),
        ]
    };
    let projects = [&open, &vault];
    for (i, t) in tickets.iter().enumerate() {
        let a = h.agent_of(&t.id);
        let dir = a.worktree.clone().unwrap().path;
        let text = told(&h, &a.id, Path::new(&dir));
        // Given its ticket, with its protocol and its loops: the test reads what was sent.
        assert!(
            text.contains(&format!("Ticket {} : Ajouter le fichier", t.key)),
            "{text}"
        );
        assert!(text.contains("Boucle 2/5"), "{text}");
        let (other, other_ticket) = (projects[1 - i], &tickets[1 - i]);
        for name in names(other, other_ticket) {
            assert!(
                !text.contains(&name),
                "the agent of {} was told “{name}” of {}: {text}",
                projects[i].name,
                other.name
            );
        }
    }
}
