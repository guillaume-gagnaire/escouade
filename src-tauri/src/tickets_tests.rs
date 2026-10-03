//! Integration tests of the board: real git repositories and the fake `claude` CLI, which plays a
//! ticket's agent as its title says: [ok] (every criterion met at once), [jamais] (none ever),
//! [sans-bilan] (no report), [lent] (a turn that lasts), [recette] (a launch recipe); by default
//! criterion n is met from loop n on.

use crate::core_tests::{git, harness, Harness};
use crate::model::*;
use crate::tickets::TicketDraft;

fn draft(title: &str, criteria: &[&str], max_loops: u32) -> TicketDraft {
    TicketDraft {
        title: title.into(),
        description: String::new(),
        criteria: criteria.iter().map(|c| c.to_string()).collect(),
        max_loops,
    }
}

impl Harness {
    fn ticket(&self, id: &str) -> Ticket {
        self.core.ticket(id).unwrap()
    }

    fn set_board(&self, project_id: &str, f: impl FnOnce(&mut BoardSettings)) {
        let mut s = self.core.project(project_id).unwrap().board;
        f(&mut s);
        self.core.board_set(project_id, s).unwrap();
    }
}

#[tokio::test]
async fn tickets_get_keys_from_the_project_that_are_never_reused() {
    let h = harness("tk-create");
    let (p, _) = h.project(false).await;
    // Kept "À faire": no agent starts in these tests.
    h.set_board(&p.id, |s| s.autopilot = false);
    let a = h
        .core
        .ticket_create(
            &p.id,
            draft(
                "Limiter les tentatives",
                &["5 essais par IP", " ", "Réponse 429"],
                8,
            ),
        )
        .await
        .unwrap();
    assert_eq!(
        (a.key.as_str(), a.column, a.max_loops),
        ("DEM-1", Column::Todo, 8)
    );
    let texts = |t: &Ticket| {
        t.criteria
            .iter()
            .map(|c| c.text.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(texts(&a), ["5 essais par IP", "Réponse 429"]);
    let b = h
        .core
        .ticket_create(&p.id, draft("  Sans critère ", &[], 4))
        .await
        .unwrap();
    assert_eq!(
        (b.key.as_str(), b.title.as_str(), b.max_loops),
        ("DEM-2", "Sans critère", 5)
    );
    assert_eq!(
        texts(&b),
        ["Implémentation conforme au ticket", "Tests verts"]
    );
    assert!(b.rank > a.rank);
    // The prefix and the target are fixed by the first ticket; a deleted number is never reused.
    let board = h.core.project(&p.id).unwrap().board;
    assert_eq!(
        (
            board.prefix.as_str(),
            board.target.as_str(),
            board.next_number
        ),
        ("DEM", "main", 3)
    );
    h.core.ticket_delete(&b.id).await.unwrap();
    h.core
        .update_project(Project {
            name: "Zèbre".into(),
            ..h.core.project(&p.id).unwrap()
        })
        .unwrap();
    let c = h
        .core
        .ticket_create(&p.id, draft("Encore", &[], 5))
        .await
        .unwrap();
    assert_eq!(c.key, "DEM-3");
    assert!(h
        .core
        .ticket_create(&p.id, draft("   ", &[], 5))
        .await
        .is_err());
    let events = h.events.lock();
    assert!(events
        .iter()
        .any(|e| e["type"] == "ticket" && e["ticket"]["key"] == "DEM-1"));
    assert!(events
        .iter()
        .any(|e| e["type"] == "ticketRemoved" && e["id"] == b.id.as_str()));
    assert!(events
        .iter()
        .any(|e| e["type"] == "project" && e["project"]["board"]["prefix"] == "DEM"));
}

#[tokio::test]
async fn a_ticket_to_do_is_edited_prioritized_and_deleted() {
    let h = harness("tk-edit");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let a = h
        .core
        .ticket_create(&p.id, draft("Un", &[], 5))
        .await
        .unwrap();
    let b = h
        .core
        .ticket_create(&p.id, draft("Deux", &[], 5))
        .await
        .unwrap();
    let edited = h
        .core
        .ticket_update(
            &a.id,
            TicketDraft {
                description: "Contexte".into(),
                ..draft("Un, précisé", &["Critère A"], 3)
            },
        )
        .unwrap();
    assert_eq!(
        (
            edited.title.as_str(),
            edited.description.as_str(),
            edited.max_loops,
            edited.key.as_str()
        ),
        ("Un, précisé", "Contexte", 3, "DEM-1")
    );
    h.core.ticket_prioritize(&b.id).unwrap();
    assert!(h.ticket(&b.id).rank < h.ticket(&a.id).rank);
    // Only a ticket "À faire" is edited.
    h.core
        .tickets
        .write()
        .iter_mut()
        .find(|t| t.id == a.id)
        .unwrap()
        .column = Column::Review;
    assert!(h.core.ticket_update(&a.id, draft("Non", &[], 5)).is_err());
    h.core.ticket_delete(&b.id).await.unwrap();
    assert!(h.core.ticket(&b.id).is_err());
}

#[tokio::test]
async fn the_board_settings_stay_within_bounds_and_keep_the_tickets_key() {
    let h = harness("tk-settings");
    let (p, r) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    h.core
        .ticket_create(&p.id, draft("Un", &[], 5))
        .await
        .unwrap();
    // The window sends what it knew before the first ticket: the prefix and the number stay.
    let saved = h
        .core
        .board_set(
            &p.id,
            BoardSettings {
                action: "pr".into(),
                strategy: "rebase".into(),
                conflict: "agent".into(),
                max_parallel: 9,
                autopilot: false,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        (saved.board.action.as_str(), saved.board.max_parallel),
        ("pr", 6)
    );
    // Valid choices are kept as they are.
    assert_eq!(
        (saved.board.strategy.as_str(), saved.board.conflict.as_str()),
        ("rebase", "agent")
    );
    assert_eq!(
        (saved.board.prefix.as_str(), saved.board.next_number),
        ("DEM", 2)
    );
    // That copy had no target either: the one the first ticket fixed stays.
    assert_eq!(saved.board.target, "main");
    // Anything else than the known choices falls back to the defaults.
    let saved = h
        .core
        .board_set(
            &p.id,
            BoardSettings {
                action: "x".into(),
                strategy: "y".into(),
                conflict: "z".into(),
                target: "  ".into(),
                autopilot: false,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        (
            saved.board.target.as_str(),
            saved.board.action.as_str(),
            saved.board.strategy.as_str(),
            saved.board.conflict.as_str()
        ),
        ("main", "merge", "squash", "ask")
    );
    assert_eq!(h.core.project(&p.id).unwrap().board, saved.board);
    // A target the window picked is kept, trimmed.
    let saved = h
        .core
        .board_set(
            &p.id,
            BoardSettings {
                target: " develop ".into(),
                autopilot: false,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(saved.board.target, "develop");
    git(&r, &["branch", "aaa"]);
    assert_eq!(h.core.git_branches(&p.id).await.unwrap(), ["main", "aaa"]);
}

#[tokio::test]
async fn closing_a_project_drops_its_tickets() {
    let h = harness("tk-close");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let t = h
        .core
        .ticket_create(&p.id, draft("Un", &[], 5))
        .await
        .unwrap();
    h.core.remove_project(&p.id).unwrap();
    assert!(h.core.ticket(&t.id).is_err());
    assert!(h
        .events
        .lock()
        .iter()
        .any(|e| e["type"] == "ticketRemoved" && e["id"] == t.id.as_str()));
}

#[tokio::test]
async fn deleting_a_ticket_in_progress_archives_its_agent_and_keeps_its_worktree() {
    let h = harness("tk-delete-doing");
    let (p, _) = h.project(true).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let worktree = a.meta.worktree.clone().expect("worktree created").path;
    let t = h
        .core
        .ticket_create(&p.id, draft("Un", &[], 5))
        .await
        .unwrap();
    h.core
        .edit_ticket(&t.id, |t| {
            t.column = Column::Doing;
            t.agent_id = Some(a.meta.id.clone());
            Ok(())
        })
        .unwrap();
    assert!(!h.agent(&a.meta.id).archived);
    h.core.ticket_delete(&t.id).await.unwrap();
    assert!(h.core.ticket(&t.id).is_err());
    assert!(h.agent(&a.meta.id).archived);
    assert!(std::path::Path::new(&worktree).is_dir());
    assert!(h
        .events
        .lock()
        .iter()
        .any(|e| e["type"] == "ticketRemoved" && e["id"] == t.id.as_str()));
    // A ticket whose agent is already gone is deleted all the same.
    let u = h
        .core
        .ticket_create(&p.id, draft("Deux", &[], 5))
        .await
        .unwrap();
    h.core
        .edit_ticket(&u.id, |t| {
            t.column = Column::Review;
            t.agent_id = Some("disparu".into());
            Ok(())
        })
        .unwrap();
    h.core.ticket_delete(&u.id).await.unwrap();
    assert!(h.core.ticket(&u.id).is_err());
    assert!(h.core.ticket_delete(&u.id).await.is_err());
}

#[tokio::test]
async fn only_a_ticket_to_do_is_started_or_moved_to_the_top() {
    let h = harness("tk-start");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let a = h
        .core
        .ticket_create(&p.id, draft("Un", &[], 5))
        .await
        .unwrap();
    let b = h
        .core
        .ticket_create(&p.id, draft("Deux", &[], 5))
        .await
        .unwrap();
    assert!(!a.forced);
    h.core.ticket_start(&a.id).unwrap();
    assert!(h.ticket(&a.id).forced);
    assert!(h.events.lock().iter().any(|e| e["type"] == "ticket"
        && e["ticket"]["id"] == a.id.as_str()
        && e["ticket"]["forced"] == true));
    // A ticket that left "À faire" is neither started again nor moved to the top.
    let rank = h.ticket(&b.id).rank;
    h.core
        .edit_ticket(&b.id, |t| {
            t.column = Column::Doing;
            Ok(())
        })
        .unwrap();
    assert!(h.core.ticket_start(&b.id).is_err());
    assert!(!h.ticket(&b.id).forced);
    assert!(h.core.ticket_prioritize(&b.id).is_err());
    assert_eq!(h.ticket(&b.id).rank, rank);
    assert!(h.core.ticket_start("inconnu").is_err());
    assert!(h.core.ticket_prioritize("inconnu").is_err());
}
