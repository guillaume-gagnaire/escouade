//! Integration tests of the board: real git repositories and the fake `claude` CLI, which plays a
//! ticket's agent as its title says: [ok] (every criterion met at once), [jamais] (none ever),
//! [sans-bilan] (no report), [lent] (a turn that lasts), [recette] (a launch recipe); by default
//! criterion n is met from loop n on.

use crate::board::TurnEnd;
use crate::core::AgentOptions;
use crate::core_tests::{git, harness, Harness};
use crate::model::*;
use crate::tickets::TicketDraft;
use std::path::{Path, PathBuf};
use std::time::Duration;

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

    async fn wait_ticket(&self, id: &str, what: &str, pred: impl Fn(&Ticket) -> bool) {
        self.wait(what, |h| pred(&h.ticket(id))).await
    }

    fn agent_of(&self, ticket_id: &str) -> AgentMeta {
        self.agent(self.ticket(ticket_id).agent_id.as_deref().unwrap())
    }

    fn worktree_of(&self, ticket_id: &str) -> PathBuf {
        PathBuf::from(self.agent_of(ticket_id).worktree.unwrap().path)
    }

    fn alerts(&self) -> Vec<String> {
        self.core.alerts.lock().clone()
    }

    fn last_sent(&self, dir: &Path) -> String {
        self.stdin_messages(dir).last().unwrap()["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    }

    /// The ticket's agent has hit the usage limit and waits for its automatic resume, and the board
    /// has read that turn (the ticket's 4th event: created, started, given its agent, that turn),
    /// as it has long before the user does anything about it.
    async fn wait_resume_planned(&self, ticket_id: &str) {
        self.wait("resume planned and its turn read", |h| {
            let events = h
                .events
                .lock()
                .iter()
                .filter(|e| e["type"] == "ticket" && e["ticket"]["id"] == ticket_id)
                .count();
            events >= 4
                && h.ticket(ticket_id)
                    .agent_id
                    .is_some_and(|id| h.agent(&id).resume_at.is_some())
        })
        .await
    }

    /// Claude Code can no longer be found: no agent process starts.
    fn lose_claude(&self) {
        self.core.settings.write().claude_path =
            self.dir.join("absent.cmd").to_string_lossy().to_string();
    }
}

/// What the app answers when Claude Code cannot be found.
const NO_CLAUDE: &str =
    "Erreur : Claude Code introuvable. Installe-le ou indique son chemin dans les réglages.";

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
    h.set_board(&p.id, |s| {
        s.autopilot = false;
        s.max_parallel = 1;
    });
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
    // B holds the only place: A, launched by hand, waits for it.
    let rank = h.ticket(&b.id).rank;
    h.core
        .edit_ticket(&b.id, |t| {
            t.column = Column::Doing;
            Ok(())
        })
        .unwrap();
    assert!(!a.forced);
    h.core.ticket_start(&a.id).unwrap();
    h.core.schedule_now().await;
    assert_eq!(
        (h.ticket(&a.id).forced, h.ticket(&a.id).column),
        (true, Column::Todo)
    );
    assert!(h.events.lock().iter().any(|e| e["type"] == "ticket"
        && e["ticket"]["id"] == a.id.as_str()
        && e["ticket"]["forced"] == true));
    // A ticket that left "À faire" is neither started again nor moved to the top.
    assert!(h.core.ticket_start(&b.id).is_err());
    assert!(!h.ticket(&b.id).forced);
    assert!(h.core.ticket_prioritize(&b.id).is_err());
    assert_eq!(h.ticket(&b.id).rank, rank);
    assert!(h.core.ticket_start("inconnu").is_err());
    assert!(h.core.ticket_prioritize("inconnu").is_err());
}

// ---------- the scheduler and the loop of a ticket ----------

#[tokio::test]
async fn a_ticket_gets_an_agent_on_its_own_branch_and_loops_until_its_criteria_are_met() {
    let h = harness("tk-loop");
    // Tickets get a worktree even when the project's agents do not.
    let (p, r) = h.project(false).await;
    std::fs::write(r.join(".env"), "SECRET=1\n").unwrap();
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
    h.wait_ticket(&t.id, "ticket to test", |t| t.column == Column::Review)
        .await;
    let t = h.ticket(&t.id);
    assert_eq!((t.iteration, t.partial), (2, false));
    assert!(t.criteria.iter().all(|c| c.ok && c.note == "vérifié"));
    // The agent's last account of what is in place is kept on the ticket.
    assert_eq!(t.progress, ["Fichier dem-1.txt écrit", "Boucle 2 faite"]);
    let a = h.agent_of(&t.id);
    assert_eq!(a.name, "dem-1-ajouter-le-fichier");
    assert_eq!(a.ticket_id.as_deref(), Some(t.id.as_str()));
    let base = a.port_base.unwrap();
    assert!(base >= 4100 && base % 10 == 0);
    let wt = a.worktree.clone().unwrap();
    assert_eq!(
        (wt.branch.as_str(), wt.base_branch.as_str()),
        ("ticket/dem-1", "main")
    );
    let dir = Path::new(&wt.path);
    assert_eq!(
        std::fs::read_to_string(dir.join("dem-1.txt")).unwrap(),
        "Boucle 2\n"
    );
    // The project's files to copy are in its worktree.
    assert_eq!(
        std::fs::read_to_string(dir.join(".env")).unwrap(),
        "SECRET=1\n"
    );
    // The protocol rides along, on one line, with the ticket's criteria and its ports.
    let argv = h.launches(dir).pop().unwrap();
    let i = argv
        .iter()
        .position(|x| x == "--append-system-prompt")
        .unwrap();
    assert!(
        argv[i + 1].contains("le ticket DEM-1")
            && argv[i + 1].contains("Critères d'acceptation (2)")
            && argv[i + 1].contains(&format!("Ports réservés à ce worktree : {base} à")),
        "{}",
        argv[i + 1]
    );
    // The first message, then the next loop with what was missing.
    let sent: Vec<String> = h
        .stdin_messages(dir)
        .iter()
        .map(|m| {
            m["message"]["content"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    assert!(
        sent[0].starts_with("Ticket DEM-1 : Ajouter le fichier"),
        "{sent:?}"
    );
    assert!(
        sent[1].starts_with("Boucle 2/5. Critères non atteints : 2 (reste le critère 2)"),
        "{sent:?}"
    );
    // Notified once when ready; never per turn.
    h.wait("notified", |h| {
        h.alerts()
            .iter()
            .any(|x| x.ends_with("DEM-1 prêt à tester"))
    })
    .await;
    let alerts = h.alerts();
    assert_eq!(
        alerts
            .iter()
            .filter(|x| x.ends_with("DEM-1 prêt à tester"))
            .count(),
        1,
        "{alerts:?}"
    );
    assert!(
        !alerts.iter().any(|x| x.contains("Tâche terminée")),
        "{alerts:?}"
    );
    // Titled with the project's name.
    assert!(alerts.contains(&"demo | DEM-1 prêt à tester".to_string()));
}

#[tokio::test]
async fn a_ticket_out_of_loops_goes_to_test_as_a_partial_goal() {
    let h = harness("tk-partial");
    let (p, _) = h.project(false).await;
    let t = h
        .core
        .ticket_create(&p.id, draft("Jamais fini [jamais]", &["Impossible"], 3))
        .await
        .unwrap();
    h.wait_ticket(&t.id, "partial", |t| t.column == Column::Review)
        .await;
    let t = h.ticket(&t.id);
    assert_eq!((t.iteration, t.partial), (3, true));
    assert_eq!(t.criteria[0].note, "reste le critère 1");
    assert_eq!(t.progress, ["Fichier dem-1.txt écrit", "Boucle 3 faite"]);
}

#[tokio::test]
async fn a_ticket_without_report_is_reminded_once_then_blocked_and_gives_its_place_up() {
    let h = harness("tk-no-report");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.max_parallel = 1);
    let a = h
        .core
        .ticket_create(&p.id, draft("Muet [sans-bilan]", &[], 5))
        .await
        .unwrap();
    let b = h
        .core
        .ticket_create(&p.id, draft("Suivant [ok]", &[], 5))
        .await
        .unwrap();
    h.wait_ticket(&a.id, "blocked", |t| t.blocked.is_some())
        .await;
    let ta = h.ticket(&a.id);
    assert_eq!(
        (ta.column, ta.blocked.as_deref(), ta.iteration),
        (Column::Doing, Some("Bilan des critères manquant"), 1)
    );
    assert_eq!(
        h.last_sent(&h.worktree_of(&a.id)),
        "Termine par le bilan des critères (bloc escouade)."
    );
    // Blocked, it holds no place: the next ticket starts.
    h.wait_ticket(&b.id, "next ticket to test", |t| t.column == Column::Review)
        .await;
    h.wait("notified", |h| {
        h.alerts()
            .iter()
            .any(|x| x.ends_with("DEM-1 bloqué : Bilan des critères manquant"))
    })
    .await;
}

#[tokio::test]
async fn without_the_autopilot_a_ticket_waits_until_launched_by_hand() {
    let h = harness("tk-manual");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let t = h
        .core
        .ticket_create(&p.id, draft("À la main [ok]", &[], 5))
        .await
        .unwrap();
    h.core.schedule_now().await;
    assert_eq!(h.ticket(&t.id).column, Column::Todo);
    h.core.ticket_start(&t.id).unwrap();
    h.wait_ticket(&t.id, "to test", |t| t.column == Column::Review)
        .await;
    assert!(!h.ticket(&t.id).forced);
}

#[tokio::test]
async fn a_ticket_whose_agent_dies_is_blocked_and_frees_its_place() {
    let h = harness("tk-crash");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.max_parallel = 1);
    let a = h
        .core
        .ticket_create(&p.id, draft("Faire crash le faux claude", &[], 5))
        .await
        .unwrap();
    let b = h
        .core
        .ticket_create(&p.id, draft("Ensuite [ok]", &[], 5))
        .await
        .unwrap();
    h.wait_ticket(&a.id, "blocked", |t| t.blocked.is_some())
        .await;
    assert_eq!(
        h.ticket(&a.id).blocked.as_deref(),
        Some("Erreur : Claude Code s'est arrêté (code 3)")
    );
    h.wait_ticket(&b.id, "the next one", |t| t.column != Column::Todo)
        .await;
    // The board tells, not the agent's own end of turn.
    h.wait("notified", |h| {
        h.alerts()
            .iter()
            .any(|x| x.ends_with("DEM-1 bloqué : Erreur : Claude Code s'est arrêté (code 3)"))
    })
    .await;
    let alerts = h.alerts();
    assert!(
        !alerts.iter().any(|x| x.contains("l'agent s'est arrêté")),
        "{alerts:?}"
    );
}

#[tokio::test]
async fn a_ticket_whose_agent_cannot_start_is_blocked_and_the_next_one_gets_its_place() {
    let h = harness("tk-start-fail");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| {
        s.autopilot = false;
        s.max_parallel = 1;
    });
    let a = h
        .core
        .ticket_create(&p.id, draft("Un [ok]", &[], 5))
        .await
        .unwrap();
    let b = h
        .core
        .ticket_create(&p.id, draft("Deux [ok]", &[], 5))
        .await
        .unwrap();
    h.lose_claude();
    // The passes asked for so far had nothing to start; from here, a single pass.
    tokio::time::sleep(Duration::from_millis(300)).await;
    for id in [&a.id, &b.id] {
        h.core
            .edit_ticket(id, |t| {
                t.forced = true;
                Ok(())
            })
            .unwrap();
    }
    h.core.schedule_now().await;
    // That pass had one place, for A, which could not start.
    let ta = h.ticket(&a.id);
    assert_eq!(
        (ta.column, ta.blocked.as_deref()),
        (Column::Doing, Some(NO_CLAUDE))
    );
    // Blocked, it holds no place: the next one is tried at once.
    h.wait_ticket(&b.id, "the next one tried", |t| t.blocked.is_some())
        .await;
    h.wait("notified", |h| {
        h.alerts()
            .iter()
            .any(|x| x.ends_with(&format!("DEM-1 bloqué : {NO_CLAUDE}")))
    })
    .await;
}

#[tokio::test]
async fn a_ticket_stopped_by_the_usage_limit_waits_and_nothing_starts_until_the_quota_resets() {
    let h = harness("tk-quota");
    let (p, _) = h.project(false).await;
    let a = h
        .core
        .ticket_create(&p.id, draft("Atteindre la limite", &[], 5))
        .await
        .unwrap();
    h.wait_resume_planned(&a.id).await;
    let ta = h.ticket(&a.id);
    assert_eq!((ta.column, ta.blocked.clone()), (Column::Doing, None));
    // A place is free, but an agent waits for its quota: nothing starts.
    let b = h
        .core
        .ticket_create(&p.id, draft("Patienter [ok]", &[], 5))
        .await
        .unwrap();
    h.core.schedule_now().await;
    assert_eq!(h.ticket(&b.id).column, Column::Todo);
    // Its resume cancelled, the ticket would wait forever: blocked, it gives its place up.
    h.set_board(&p.id, |s| s.max_parallel = 1);
    h.core
        .cancel_resume(ta.agent_id.as_deref().unwrap())
        .unwrap();
    assert_eq!(
        h.ticket(&a.id).blocked.as_deref(),
        Some("Erreur : limite d'usage atteinte")
    );
    h.wait_ticket(&b.id, "started after the quota", |t| {
        t.column != Column::Todo
    })
    .await;
    h.wait("notified", |h| {
        h.alerts()
            .iter()
            .any(|x| x.ends_with("DEM-1 bloqué : Erreur : limite d'usage atteinte"))
    })
    .await;
}

#[tokio::test]
async fn turning_the_automatic_resume_off_blocks_the_ticket_that_waited_for_it() {
    let h = harness("tk-quota-off");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.max_parallel = 1);
    let a = h
        .core
        .ticket_create(&p.id, draft("Atteindre la limite", &[], 5))
        .await
        .unwrap();
    h.wait_resume_planned(&a.id).await;
    let b = h
        .core
        .ticket_create(&p.id, draft("Ensuite [ok]", &[], 5))
        .await
        .unwrap();
    let off = Settings {
        auto_resume: false,
        ..h.core.settings.read().clone()
    };
    h.core.save_settings(off).unwrap();
    assert_eq!(
        h.ticket(&a.id).blocked.as_deref(),
        Some("Erreur : limite d'usage atteinte")
    );
    h.wait_ticket(&b.id, "started in its place", |t| t.column != Column::Todo)
        .await;
    h.wait("notified", |h| {
        h.alerts()
            .iter()
            .any(|x| x.ends_with("DEM-1 bloqué : Erreur : limite d'usage atteinte"))
    })
    .await;
}

#[tokio::test]
async fn a_ticket_whose_automatic_resume_cannot_be_sent_is_blocked() {
    let h = harness("tk-quota-fail");
    let (p, _) = h.project(false).await;
    let a = h
        .core
        .ticket_create(&p.id, draft("Atteindre la limite", &[], 5))
        .await
        .unwrap();
    h.wait_resume_planned(&a.id).await;
    let id = h.ticket(&a.id).agent_id.unwrap();
    // Its process gone and Claude Code nowhere: "continue" cannot be sent.
    if let Some(proc) = h.core.agent(&id).unwrap().lock().detach() {
        proc.kill();
    }
    h.lose_claude();
    h.core.agent(&id).unwrap().lock().meta.resume_at = Some(now_ms() - 1_000);
    h.core.resume_due().await;
    assert_eq!(h.ticket(&a.id).blocked.as_deref(), Some(NO_CLAUDE));
    assert!(h
        .alerts()
        .iter()
        .any(|x| x.ends_with(&format!("DEM-1 bloqué : {NO_CLAUDE}"))));
}

#[tokio::test]
async fn an_old_agents_turn_never_moves_its_ticket() {
    let h = harness("tk-stale-turn");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let t = h
        .core
        .ticket_create(&p.id, draft("Relancé", &["Un"], 5))
        .await
        .unwrap();
    let old = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                ticket_id: Some(t.id.clone()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    // The ticket started again with another agent.
    h.core
        .edit_ticket(&t.id, |t| {
            t.column = Column::Doing;
            t.iteration = 1;
            t.agent_id = Some("nouveau".into());
            Ok(())
        })
        .unwrap();
    let report = TurnEnd::Finished(
        "Fini.\n\n```escouade\n{\"criteres\": [{\"n\": 1, \"ok\": true}]}\n```".into(),
    );
    h.core.turn_ended(&old, report.clone()).await;
    h.core.turn_ended(&old, TurnEnd::Interrupted).await;
    let now = h.ticket(&t.id);
    assert_eq!(
        (now.column, now.iteration, now.criteria[0].ok, now.blocked),
        (Column::Doing, 1, false, None)
    );
    assert!(h.alerts().is_empty(), "{:?}", h.alerts());
    // Its own agent's turn moves it.
    h.core
        .edit_ticket(&t.id, |t| {
            t.agent_id = Some(old.clone());
            Ok(())
        })
        .unwrap();
    h.core.turn_ended(&old, report).await;
    assert_eq!(h.ticket(&t.id).column, Column::Review);
}

#[tokio::test]
async fn tickets_started_together_get_ports_of_their_own() {
    let h = harness("tk-ports");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let a = h
        .core
        .ticket_create(&p.id, draft("Un [ok]", &[], 5))
        .await
        .unwrap();
    let b = h
        .core
        .ticket_create(&p.id, draft("Deux [ok]", &[], 5))
        .await
        .unwrap();
    // Both start in the same pass.
    h.set_board(&p.id, |s| s.autopilot = true);
    h.wait("both started", |h| {
        h.ticket(&a.id).agent_id.is_some() && h.ticket(&b.id).agent_id.is_some()
    })
    .await;
    let (pa, pb) = (
        h.agent_of(&a.id).port_base.unwrap(),
        h.agent_of(&b.id).port_base.unwrap(),
    );
    assert_ne!(pa, pb);
    // A block is taken as soon as it is reserved, before any agent holds it.
    let x = h.core.reserve_ports().unwrap();
    let y = h.core.reserve_ports().unwrap();
    assert!(x != y && ![pa, pb].contains(&x) && ![pa, pb].contains(&y));
    h.core.unreserve_ports(Some(x));
    h.core.unreserve_ports(Some(y));
}

#[tokio::test]
async fn a_title_with_quotes_percents_and_ampersands_still_starts_its_agent() {
    let h = harness("tk-quoting");
    let (p, _) = h.project(false).await;
    let t = h
        .core
        .ticket_create(
            &p.id,
            draft("Gérer « % & \" » [ok]", &["Les 100 % passent"], 5),
        )
        .await
        .unwrap();
    h.wait_ticket(&t.id, "to test", |t| t.column == Column::Review)
        .await;
    let argv = h.launches(&h.worktree_of(&t.id)).pop().unwrap();
    assert!(
        argv.iter().any(|x| x.contains("« Gérer « % & \" » [ok] »")),
        "{argv:?}"
    );
}

#[tokio::test]
async fn a_ticket_agent_gives_its_launch_recipe_in_its_report() {
    let h = harness("tk-recipe");
    let (p, _) = h.project(false).await;
    let t = h
        .core
        .ticket_create(&p.id, draft("Page [ok] [recette]", &[], 5))
        .await
        .unwrap();
    h.wait_ticket(&t.id, "to test", |t| t.column == Column::Review)
        .await;
    h.wait("recipe kept", |h| h.agent_of(&t.id).recipe.is_some())
        .await;
    let recipe = h.agent_of(&t.id).recipe.unwrap();
    assert_eq!(
        (recipe.processes[0].name.as_str(), recipe.open.as_str()),
        ("web", "http://localhost:4100/fonction")
    );
}
