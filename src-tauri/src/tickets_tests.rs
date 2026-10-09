//! Integration tests of the board: real git repositories and the fake `claude` CLI, which plays a
//! ticket's agent as its title says: [ok] (every criterion met at once), [jamais] (none ever),
//! [sans-bilan] (no report), [lent] (a turn that lasts), [recette] (a launch recipe), [question]
//! (a question first), [fin-d-abord] (an interrupted turn's end read before the interrupt's
//! answer), [commite] (its work committed, every file of its folder included), [tenace] (a process
//! that lasts 5 s once its input is closed), [message-libre] (Haiku's commit message out of form);
//! by default criterion n is met from loop n on. Pushes go to a local bare repository, and pull
//! requests to the fake `gh` (tests/fixtures/fake-gh.cmd), or to none.

use crate::board::TurnEnd;
use crate::core::{AgentOptions, Core};
use crate::core_tests::{commit_change, git, harness, ignore, wt_step, Harness};
use crate::model::*;
use crate::tickets::{error_reason, TicketDraft};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tauri::test::mock_app;

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

    fn last_sent(&self, dir: &Path) -> String {
        self.stdin_messages(dir).last().unwrap()["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    }

    /// The ticket's agent has hit the usage limit and waits for its automatic resume. Whenever the
    /// board reads that turn, before or after what the test does next, the outcome is the same
    /// (see `a_turn_stopped_by_the_usage_limit_leaves_a_blocked_ticket_as_it_is`).
    async fn wait_resume_planned(&self, ticket_id: &str) {
        self.wait("resume planned", |h| {
            h.ticket(ticket_id)
                .agent_id
                .is_some_and(|id| h.agent(&id).resume_at.is_some())
        })
        .await
    }

    /// No scheduling pass is queued or running.
    async fn wait_board_idle(&self) {
        self.wait("no scheduling pass queued", |h| {
            h.core.passes_queued.load(Ordering::SeqCst) == 0
        })
        .await
    }

    /// The app's settings changed by `f`, saved as the window does.
    fn set_settings(&self, f: impl FnOnce(&mut Settings)) {
        let mut s = self.core.settings.read().clone();
        f(&mut s);
        self.core.save_settings(s).unwrap();
    }

    /// The autopilot's pauses the window was told of, in order (null: over).
    fn pauses(&self) -> Vec<Value> {
        self.events
            .lock()
            .iter()
            .filter(|e| e["type"] == "autopilotPause")
            .map(|e| e["pause"].clone())
            .collect()
    }

    /// The autopilot's clock `ms` ahead of the real one, and its timer's look at its pause then.
    fn pause_tick_after(&self, ms: i64) {
        self.core.clock_ahead.store(ms, Ordering::SeqCst);
        self.core.pause_tick();
    }

    /// Claude Code can no longer be found: no agent process starts.
    fn lose_claude(&self) {
        self.core.settings.write().claude_path =
            self.dir.join("absent.cmd").to_string_lossy().to_string();
    }

    /// Waits until the fake CLI in `dir` read a message starting with `start` (it logs what it
    /// reads: right after a send, it may not have yet).
    async fn wait_sent(&self, dir: &Path, start: &str) {
        self.wait("message read by the agent", |h| {
            h.stdin_messages(dir).iter().any(|m| {
                m["message"]["content"]
                    .as_str()
                    .unwrap_or_default()
                    .starts_with(start)
            })
        })
        .await
    }

    /// For a while (what it takes a late turn end to be read), no alert is raised.
    async fn stays_quiet(&self) {
        for _ in 0..50 {
            assert!(self.alerts().is_empty(), "{:?}", self.alerts());
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        self.wait_board_idle().await;
        assert!(self.alerts().is_empty(), "{:?}", self.alerts());
    }

    /// A plain agent of the project (no worktree, no protocol: the fake CLI only echoes) that
    /// already took a message.
    async fn agent_that_worked(&self, project_id: &str) -> String {
        let a = self
            .core
            .create_agent_with(project_id, AgentOptions::default())
            .await
            .unwrap()
            .meta
            .id;
        self.core
            .send_message(&a, "Bonjour".into(), vec![])
            .await
            .unwrap();
        self.wait("its turn over", |h| {
            let m = h.agent(&a);
            m.prompts == 1 && m.status == AgentStatus::Done
        })
        .await;
        a
    }
}

/// Waits until the ticket's agent is running a turn.
async fn wait_running(h: &Harness, ticket_id: &str) -> String {
    h.wait("turn running", |h| {
        h.ticket(ticket_id)
            .agent_id
            .is_some_and(|id| h.agent(&id).status == AgentStatus::Running)
    })
    .await;
    h.ticket(ticket_id).agent_id.unwrap()
}

/// The texts of the user messages in the agent's conversation.
fn user_texts(core: &Core<tauri::test::MockRuntime>, agent_id: &str) -> Vec<String> {
    core.agent(agent_id)
        .unwrap()
        .lock()
        .conv
        .items()
        .iter()
        .filter(|i| i["kind"] == "user")
        .map(|i| i["text"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// A hook of `repo` that fails every checkout, a new worktree's included, on several lines.
fn fail_checkouts(repo: &Path) {
    let hook = repo.join(".git").join("hooks").join("post-checkout");
    std::fs::write(
        &hook,
        "#!/bin/sh\necho 'ligne un' >&2\necho 'ligne deux' >&2\nexit 1\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
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
    ignore(&r, ".env");
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
    assert_eq!((t.iteration, t.partial, t.loops), (2, false, 2));
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
    // None from the agent's own end of turn (titled with the project and the agent).
    assert!(
        !alerts.iter().any(|x| x.starts_with("demo · ")),
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
        !alerts.iter().any(|x| x.starts_with("demo · ")),
        "{alerts:?}"
    );
}

#[tokio::test]
async fn a_ticket_whose_agent_cannot_start_is_blocked_and_the_next_one_gets_its_place() {
    let h = harness("tk-start-fail");
    let (p, r) = h.project(false).await;
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
    // No worktree can be made: git says why on several lines.
    fail_checkouts(&r);
    // The passes asked for so far had nothing to start; from here, a single pass.
    h.wait_board_idle().await;
    for id in [&a.id, &b.id] {
        h.core
            .edit_ticket(id, |t| {
                t.forced = true;
                Ok(())
            })
            .unwrap();
    }
    h.core.schedule_now().await;
    // That pass had one place, for A, which could not start: blocked with the first line only.
    let ta = h.ticket(&a.id);
    let reason = ta.blocked.clone().unwrap();
    assert_eq!(ta.column, Column::Doing);
    assert!(
        reason.starts_with("Erreur : worktree du ticket non créé: ")
            && !reason.contains('\n')
            && !reason.contains("ligne"),
        "{reason:?}"
    );
    // Blocked, it holds no place: the next one is tried at once.
    h.wait_ticket(&b.id, "the next one tried", |t| t.blocked.is_some())
        .await;
    h.wait("notified", |h| {
        h.alerts()
            .iter()
            .any(|x| x.ends_with(&format!("DEM-1 bloqué : {reason}")))
    })
    .await;
}

#[test]
fn a_block_reason_is_one_line_the_one_where_git_says_why_when_there_is_one() {
    // git's `fatal:` line, after the context the app gave.
    let e = anyhow::anyhow!("Preparing worktree (new branch 'ticket/dem-1')\nfatal: refusé")
        .context("worktree du ticket non créé");
    assert_eq!(
        error_reason(&e),
        "Erreur : worktree du ticket non créé: fatal: refusé"
    );
    assert_eq!(
        error_reason(&anyhow::anyhow!("\n  \nfatal: seul")),
        "Erreur : fatal: seul"
    );
    // Or its `error:` line, under every context.
    let e = anyhow::anyhow!(
        "Switched to a new branch\nerror: pathspec 'x' did not match\nhint: try again"
    )
    .context("checkout")
    .context("préparation");
    assert_eq!(
        error_reason(&e),
        "Erreur : préparation: checkout: error: pathspec 'x' did not match"
    );
    // Else its first line.
    let e = anyhow::anyhow!("ligne un\nligne deux").context("étape");
    assert_eq!(error_reason(&e), "Erreur : étape: ligne un");
}

#[tokio::test]
async fn no_ticket_starts_while_claude_code_cannot_be_found() {
    let h = harness("tk-no-claude");
    let (p, r) = h.project(false).await;
    let claude = h.core.settings.read().claude_path.clone();
    h.lose_claude();
    let t = h
        .core
        .ticket_create(&p.id, draft("Patienter [ok]", &[], 5))
        .await
        .unwrap();
    h.core.schedule_now().await;
    h.wait_board_idle().await;
    // Nothing made, nothing blocked: the ticket waits for Claude Code.
    let waiting = h.ticket(&t.id);
    assert_eq!(
        (waiting.column, waiting.agent_id, waiting.blocked),
        (Column::Todo, None, None)
    );
    assert_eq!(git(&r, &["branch", "--list", "ticket/*"]), "");
    assert!(!r.join(".claude").join("worktrees").exists());
    assert!(h.alerts().is_empty(), "{:?}", h.alerts());
    // Its path set right in the settings: the ticket starts.
    let fixed = Settings {
        claude_path: claude,
        ..h.core.settings.read().clone()
    };
    h.core.save_settings(fixed).unwrap();
    h.wait_ticket(&t.id, "started once found", |t| t.column == Column::Review)
        .await;
}

#[tokio::test]
async fn a_ticket_agents_question_still_calls_for_the_user() {
    let h = harness("tk-question");
    let (p, _) = h.project(false).await;
    let t = h
        .core
        .ticket_create(&p.id, draft("Choisir [question] [ok]", &[], 5))
        .await
        .unwrap();
    h.wait("question notified", |h| {
        h.alerts()
            .iter()
            .any(|x| x.ends_with(" | Quelle base de données ?"))
    })
    .await;
    let id = h.ticket(&t.id).agent_id.unwrap();
    h.core
        .answer_question(
            &id,
            "req_question",
            json!({ "Quelle base de données ?": "SQLite" }),
        )
        .unwrap();
    h.wait_ticket(&t.id, "to test", |t| t.column == Column::Review)
        .await;
    h.wait("ready notified", |h| {
        h.alerts()
            .iter()
            .any(|x| x.ends_with("DEM-1 prêt à tester"))
    })
    .await;
    // The question was for the user; the end of the turn is the board's to tell.
    let alerts = h.alerts();
    let from_the_agent: Vec<_> = alerts.iter().filter(|x| x.starts_with("demo · ")).collect();
    assert!(
        matches!(
            from_the_agent[..],
            [x] if x.starts_with("demo · dem-1-") && x.ends_with(" | Quelle base de données ?")
        ),
        "{alerts:?}"
    );
}

#[tokio::test]
async fn a_tickets_notifications_follow_the_tickets_choice_alone() {
    let h = harness("tk-notify-choice");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let t = h
        .core
        .ticket_create(&p.id, draft("Choix [ok]", &[], 5))
        .await
        .unwrap();
    h.core.settings.write().notify_for.tickets = false;
    h.core.notify_ticket(&t.id, true);
    h.core.notify_ticket(&t.id, false);
    assert!(h.alerts().is_empty(), "{:?}", h.alerts());
    // Every other choice off, this one on: the ticket is told.
    h.core.settings.write().notify_for = NotifyFor {
        questions: false,
        done: false,
        errors: false,
        tickets: true,
    };
    h.core.notify_ticket(&t.id, true);
    assert_eq!(h.alerts(), ["demo | DEM-1 prêt à tester"]);
}

#[tokio::test]
async fn a_turn_stopped_by_the_usage_limit_leaves_a_blocked_ticket_as_it_is() {
    let h = harness("tk-limit-blocked");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let t = h
        .core
        .ticket_create(&p.id, draft("Bloqué", &["Un"], 5))
        .await
        .unwrap();
    let a = h
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
    // Its resume was lost (cancelled, turned off, failed) and that blocked it, before the board
    // read the turn the limit stopped: that late reading changes nothing.
    h.core
        .edit_ticket(&t.id, |t| {
            t.column = Column::Doing;
            t.iteration = 1;
            t.agent_id = Some(a.clone());
            t.blocked = Some(NO_CLAUDE.into());
            Ok(())
        })
        .unwrap();
    h.core.agent(&a).unwrap().lock().meta.resume_at = Some(now_ms() + 3_600_000);
    h.core.turn_ended(&a, TurnEnd::Limited).await;
    assert_eq!(h.ticket(&t.id).blocked.as_deref(), Some(NO_CLAUDE));
    h.core.agent(&a).unwrap().lock().meta.resume_at = None;
    h.core.turn_ended(&a, TurnEnd::Limited).await;
    assert_eq!(h.ticket(&t.id).blocked.as_deref(), Some(NO_CLAUDE));
    assert!(h.alerts().is_empty(), "{:?}", h.alerts());
    // Not blocked, with no resume to wait for, it is.
    h.core
        .edit_ticket(&t.id, |t| {
            t.blocked = None;
            Ok(())
        })
        .unwrap();
    h.core.turn_ended(&a, TurnEnd::Limited).await;
    assert_eq!(
        h.ticket(&t.id).blocked.as_deref(),
        Some("Erreur : limite d'usage atteinte")
    );
    assert_eq!(
        h.alerts(),
        ["demo | DEM-1 bloqué : Erreur : limite d'usage atteinte"]
    );
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
    h.wait("notified", |h| {
        h.alerts()
            .iter()
            .any(|x| x.ends_with("DEM-1 bloqué : Erreur : limite d'usage atteinte"))
    })
    .await;
    // The next one would meet the limit too: none starts in its place before the quota resets
    // (the fake CLI's 5-hour window, used up, ends in an hour).
    h.wait_board_idle().await;
    assert_eq!(h.ticket(&b.id).column, Column::Todo);
    assert_eq!(h.pauses().last().unwrap()["reason"], "fiveHour");
    h.pause_tick_after(3_600_000 + 1_000);
    h.wait_ticket(&b.id, "started after the quota", |t| {
        t.column != Column::Todo
    })
    .await;
    assert_eq!(h.pauses().last(), Some(&Value::Null));
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
    h.set_settings(|s| s.auto_resume = false);
    assert_eq!(
        h.ticket(&a.id).blocked.as_deref(),
        Some("Erreur : limite d'usage atteinte")
    );
    h.wait("notified", |h| {
        h.alerts()
            .iter()
            .any(|x| x.ends_with("DEM-1 bloqué : Erreur : limite d'usage atteinte"))
    })
    .await;
    // Nothing starts in its place: the next one would meet the limit too.
    h.wait_board_idle().await;
    assert_eq!(h.ticket(&b.id).column, Column::Todo);
    assert!(h.pauses().last().unwrap().is_object());
    // Until "Reprendre maintenant", which lifts the pause after the limit and the window's.
    h.core.autopilot_resume();
    h.wait_ticket(&b.id, "started on « Reprendre maintenant »", |t| {
        t.column != Column::Todo
    })
    .await;
    assert_eq!(h.pauses().last(), Some(&Value::Null));
}

#[tokio::test]
async fn a_usage_limit_with_no_resume_blocks_its_ticket_and_pauses_the_autopilot_for_30_minutes() {
    let h = harness("tk-quota-pause");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.max_parallel = 1);
    h.set_settings(|s| s.auto_resume = false);
    let a = h
        .core
        .ticket_create(&p.id, draft("Atteindre la limite", &[], 5))
        .await
        .unwrap();
    let b = h
        .core
        .ticket_create(&p.id, draft("Ensuite [ok]", &[], 5))
        .await
        .unwrap();
    h.wait_ticket(&a.id, "blocked by the limit", |t| t.blocked.is_some())
        .await;
    assert_eq!(
        h.ticket(&a.id).blocked.as_deref(),
        Some("Erreur : limite d'usage atteinte")
    );
    // As with an API key: no quota window is known, only the pause after the limit holds.
    {
        let mut u = h.core.usage.lock();
        u.five_hour = None;
        u.seven_day = None;
    }
    h.core.pause_tick();
    h.wait_board_idle().await;
    assert_eq!(h.ticket(&b.id).column, Column::Todo);
    let pause = h.pauses().last().cloned().unwrap();
    assert_eq!(
        (&pause["reason"], &pause["pct"]),
        (&json!("limit"), &Value::Null)
    );
    let left = pause["until"].as_i64().unwrap() - now_ms();
    assert!((29 * 60_000..=30 * 60_000).contains(&left), "{left}");
    // Not before its 30 minutes are over.
    h.pause_tick_after(29 * 60_000);
    h.wait_board_idle().await;
    assert_eq!(h.ticket(&b.id).column, Column::Todo);
    h.pause_tick_after(30 * 60_000 + 1_000);
    h.wait_ticket(&b.id, "started after the pause", |t| {
        t.column != Column::Todo
    })
    .await;
    assert_eq!(h.pauses().last(), Some(&Value::Null));
}

#[tokio::test]
async fn nothing_starts_while_a_window_is_used_past_the_threshold_until_it_reads_under_it() {
    let h = harness("tk-quota-window");
    let (p, _) = h.project(false).await;
    // A process to read the quotas from: the fake CLI's are well under any threshold.
    h.agent_that_worked(&p.id).await;
    h.set_settings(|s| s.quota_pause = 95);
    let end = now_ms() + 3_600_000;
    h.core.usage.lock().seven_day = Some(RateWindow {
        pct: 96.0,
        resets_at: Some(end),
    });
    let t = h
        .core
        .ticket_create(&p.id, draft("Patienter [ok]", &[], 5))
        .await
        .unwrap();
    h.wait_board_idle().await;
    assert_eq!(h.ticket(&t.id).column, Column::Todo);
    assert_eq!(
        h.pauses(),
        [json!({ "reason": "week", "pct": 96.0, "until": end })]
    );
    // Launched by hand ("Lancer", the autopilot off), it waits too.
    h.set_board(&p.id, |s| s.autopilot = false);
    h.core.ticket_start(&t.id).unwrap();
    h.wait_board_idle().await;
    assert_eq!(h.ticket(&t.id).column, Column::Todo);
    // Read under it, the tickets go by themselves.
    h.core.refresh_usage().await;
    h.wait_ticket(&t.id, "started under the threshold", |t| {
        t.column != Column::Todo
    })
    .await;
    assert_eq!(h.pauses().last(), Some(&Value::Null));
}

#[tokio::test]
async fn a_window_past_the_threshold_holds_nothing_back_once_it_ends() {
    let h = harness("tk-quota-window-end");
    let (p, _) = h.project(false).await;
    // Ended already: it holds nothing back.
    h.core.usage.lock().five_hour = Some(RateWindow {
        pct: 100.0,
        resets_at: Some(now_ms() - 1_000),
    });
    let a = h
        .core
        .ticket_create(&p.id, draft("Tout de suite [ok]", &[], 5))
        .await
        .unwrap();
    h.wait_ticket(&a.id, "to test", |t| t.column == Column::Review)
        .await;
    // Under the threshold (the default 100 %): nothing holds either.
    h.core.usage.lock().five_hour = Some(RateWindow {
        pct: 96.0,
        resets_at: Some(now_ms() + 60_000),
    });
    let b = h
        .core
        .ticket_create(&p.id, draft("Aussi [ok]", &[], 5))
        .await
        .unwrap();
    h.wait_ticket(&b.id, "to test", |t| t.column == Column::Review)
        .await;
    assert!(h.pauses().is_empty(), "{:?}", h.pauses());
    // Used up until a minute from now: the next one waits for its end (every place free), then
    // goes by itself.
    h.core.usage.lock().five_hour = Some(RateWindow {
        pct: 100.0,
        resets_at: Some(now_ms() + 60_000),
    });
    let c = h
        .core
        .ticket_create(&p.id, draft("Ensuite [ok]", &[], 5))
        .await
        .unwrap();
    h.wait_board_idle().await;
    assert_eq!(h.ticket(&c.id).column, Column::Todo);
    assert_eq!(h.pauses().last().unwrap()["reason"], "fiveHour");
    h.pause_tick_after(61_000);
    h.wait_ticket(&c.id, "started once the window ended", |t| {
        t.column != Column::Todo
    })
    .await;
    assert_eq!(h.pauses().last(), Some(&Value::Null));
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
    let agent = h.agent_of(&t.id);
    let recipe = agent.recipe.unwrap();
    // On the ports its protocol reserved for it.
    let web = agent.port_base.unwrap() + 1;
    assert_eq!(
        (
            recipe.processes[0].name.as_str(),
            recipe.processes[0].url.as_str(),
            recipe.open.as_str()
        ),
        (
            "web",
            format!("http://localhost:{web}").as_str(),
            format!("http://localhost:{web}/fonction").as_str()
        )
    );
}

// ---------- recovery at startup and manual management ----------

#[tokio::test]
async fn a_ticket_whose_turn_the_app_cut_asks_its_agent_to_go_on_at_the_next_start() {
    let h = harness("tk-restart");
    let (p, _) = h.project(false).await;
    let t = h
        .core
        .ticket_create(&p.id, draft("Long travail [lent]", &[], 5))
        .await
        .unwrap();
    let aid = wait_running(&h, &t.id).await;
    // Another ticket was being validated when the app stopped.
    h.core.tickets.write().push(Ticket {
        id: "t2".into(),
        project_id: p.id.clone(),
        key: "DEM-9".into(),
        column: Column::Review,
        step: Some("Merge…".into()),
        conflict: true,
        ..Default::default()
    });
    h.core.shutdown();
    let app = mock_app();
    let (re, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    let v = re.ticket("t2").unwrap();
    assert_eq!(
        (v.column, v.step.clone(), v.blocked.as_deref(), v.conflict),
        (Column::Review, None, Some("Validation interrompue"), false)
    );
    assert_eq!(*re.cut_turns.lock(), std::slice::from_ref(&aid));
    re.recover_tickets();
    let asked = |re: &Core<tauri::test::MockRuntime>| {
        user_texts(re, &aid).iter().any(|text| {
            text.starts_with("L'app a redémarré pendant ton travail sur DEM-1 : reprends")
        })
    };
    for _ in 0..750 {
        if asked(&re) {
            // Taken once: a second call finds nothing cut.
            assert!(re.cut_turns.lock().is_empty());
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the agent was not asked to go on");
}

#[tokio::test]
async fn a_turn_the_apps_stop_cuts_is_no_error_of_the_agent_and_is_saved_running() {
    let h = harness("tk-quit-cut");
    let (p, _) = h.project(false).await;
    let t = h
        .core
        .ticket_create(&p.id, draft("Long travail [lent]", &[], 5))
        .await
        .unwrap();
    let aid = wait_running(&h, &t.id).await;
    let proc = h.core.agent(&aid).unwrap().lock().proc.clone().unwrap();
    h.core.shutdown();
    h.wait("its process gone", |_| !proc.is_alive()).await;
    // Its exit is handled right after: given the time, it does not turn the cut turn into an error.
    for _ in 0..25 {
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(h.agent(&aid).status, AgentStatus::Running);
    }
    assert!(
        !h.items(&aid).iter().any(|i| i["kind"] == "notice"),
        "{:?}",
        h.items(&aid)
    );
    // Saved again after that exit, it is still a cut turn.
    h.core.save_now();
    let app = mock_app();
    let (re, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    assert_eq!(*re.cut_turns.lock(), [aid]);
}

#[tokio::test]
async fn an_interrupted_ticket_is_blocked_until_taken_up_again() {
    let h = harness("tk-interrupt");
    let (p, _) = h.project(false).await;
    let t = h
        .core
        .ticket_create(&p.id, draft("Long travail [lent]", &[], 5))
        .await
        .unwrap();
    let aid = wait_running(&h, &t.id).await;
    h.core.interrupt(&aid).await.unwrap();
    h.wait_ticket(&t.id, "blocked", |t| t.blocked.is_some())
        .await;
    assert_eq!(h.ticket(&t.id).blocked.as_deref(), Some("Interrompu"));
    h.core.ticket_resume(&t.id).await.unwrap();
    assert_eq!(h.ticket(&t.id).blocked, None);
    h.wait_sent(
        &h.worktree_of(&t.id),
        "Reprends le ticket DEM-1 là où tu en étais, puis termine par le bilan.",
    )
    .await;
    // Taken up, it is no longer blocked: a second click is refused, as for a ticket not under way.
    assert!(h.core.ticket_resume(&t.id).await.is_err());
    h.core
        .edit_ticket(&t.id, |t| {
            t.column = Column::Review;
            t.blocked = Some("Validation interrompue".into());
            Ok(())
        })
        .unwrap();
    assert!(h.core.ticket_resume(&t.id).await.is_err());
    assert!(h.core.ticket_resume("inconnu").await.is_err());
}

#[tokio::test]
async fn resuming_a_ticket_whose_agent_never_got_its_first_message_sends_it() {
    let h = harness("tk-resume-first");
    let (p, r) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let t = h
        .core
        .ticket_create(
            &p.id,
            TicketDraft {
                description: "Contexte : l'API publique.".into(),
                ..draft("Limiter", &["5 essais par IP"], 5)
            },
        )
        .await
        .unwrap();
    // Its start failed once the agent was made, before its first message.
    let a = h
        .core
        .create_agent_with(&p.id, AgentOptions::default())
        .await
        .unwrap()
        .meta
        .id;
    h.core
        .edit_ticket(&t.id, |t| {
            t.column = Column::Doing;
            t.iteration = 1;
            t.agent_id = Some(a.clone());
            t.blocked = Some(NO_CLAUDE.into());
            Ok(())
        })
        .unwrap();
    h.core.ticket_resume(&t.id).await.unwrap();
    h.wait_sent(&r, "Ticket DEM-1 : Limiter").await;
    let first = h.last_sent(&r);
    assert!(
        first.contains("Contexte : l'API publique.")
            && first.contains("1. 5 essais par IP")
            && first.contains("Boucle 1/5"),
        "{first}"
    );
}

#[tokio::test]
async fn resuming_a_ticket_whose_agent_is_gone_starts_it_again() {
    let h = harness("tk-resume-gone");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let t = h
        .core
        .ticket_create(&p.id, draft("Repartir [ok]", &[], 5))
        .await
        .unwrap();
    h.core
        .edit_ticket(&t.id, |t| {
            t.column = Column::Doing;
            t.iteration = 2;
            t.agent_id = Some("disparu".into());
            t.blocked = Some("Interrompu".into());
            Ok(())
        })
        .unwrap();
    h.core.ticket_resume(&t.id).await.unwrap();
    // Even with the autopilot off: "Reprendre" asked for it.
    h.wait_ticket(&t.id, "started again to test", |t| {
        t.column == Column::Review
    })
    .await;
    assert_ne!(h.ticket(&t.id).agent_id.as_deref(), Some("disparu"));
}

#[tokio::test]
async fn archiving_or_deleting_the_agent_of_a_ticket_sends_it_back_to_do() {
    let h = harness("tk-archive");
    let (p, _) = h.project(false).await;
    let a = h
        .core
        .ticket_create(&p.id, draft("Premier [lent]", &[], 5))
        .await
        .unwrap();
    let aid = wait_running(&h, &a.id).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    h.core.archive_agent(&aid, true).await.unwrap();
    let t = h.ticket(&a.id);
    assert_eq!(
        (t.column, t.agent_id.clone(), t.iteration),
        (Column::Todo, None, 0)
    );
    assert_eq!(h.agent(&aid).port_base, None);
    // The same from "À tester", for a deleted agent.
    let b = h
        .core
        .ticket_create(&p.id, draft("Second [ok]", &[], 5))
        .await
        .unwrap();
    h.core.ticket_start(&b.id).unwrap();
    h.wait_ticket(&b.id, "to test", |t| t.column == Column::Review)
        .await;
    let bid = h.ticket(&b.id).agent_id.unwrap();
    h.core.delete_agent(&bid, true).await.unwrap();
    let t = h.ticket(&b.id);
    assert_eq!(
        (t.column, t.agent_id.clone(), t.review_at),
        (Column::Todo, None, None)
    );
    assert!(t.criteria.iter().all(|c| !c.ok));
}

#[tokio::test]
async fn archiving_a_ticket_agent_mid_turn_neither_blocks_its_ticket_nor_alerts() {
    let h = harness("tk-archive-quiet");
    let (p, _) = h.project(false).await;
    // The stopped turn's end is read before the archive goes on.
    let t = h
        .core
        .ticket_create(&p.id, draft("Long [lent] [fin-d-abord]", &[], 5))
        .await
        .unwrap();
    let aid = wait_running(&h, &t.id).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    h.core.archive_agent(&aid, true).await.unwrap();
    // Its turn ended as a stop, not as an error of the agent.
    let turns: Vec<_> = h
        .items(&aid)
        .into_iter()
        .filter(|i| i["kind"] == "turn")
        .collect();
    assert!(
        turns.len() == 1 && turns[0]["interrupted"] == true && turns[0]["isError"] == false,
        "{turns:?}"
    );
    h.stays_quiet().await;
    let t = h.ticket(&t.id);
    assert_eq!((t.column, t.blocked), (Column::Todo, None));
}

#[tokio::test]
async fn deleting_a_ticket_under_way_archives_its_agent_and_keeps_its_worktree() {
    let h = harness("tk-delete-running");
    let (p, _) = h.project(false).await;
    let t = h
        .core
        .ticket_create(&p.id, draft("Long [lent]", &[], 5))
        .await
        .unwrap();
    let aid = wait_running(&h, &t.id).await;
    let wt = h.worktree_of(&t.id);
    h.core.ticket_delete(&t.id).await.unwrap();
    assert!(h.agent(&aid).archived);
    assert!(wt.is_dir());
}

#[tokio::test]
async fn a_ticket_stopped_mid_start_goes_back_to_do_at_the_next_start() {
    let h = harness("tk-recover-start");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let mut ids = Vec::new();
    for title in ["Sans agent", "Jamais lancé", "Agent disparu", "Bloqué"] {
        let t = h
            .core
            .ticket_create(&p.id, draft(title, &["Un"], 5))
            .await
            .unwrap();
        ids.push(t.id);
    }
    let mut made = Vec::new();
    for ticket in &ids[..2] {
        let a = h
            .core
            .create_agent_with(
                &p.id,
                AgentOptions {
                    ticket_id: Some(ticket.clone()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        made.push(a.meta.id);
    }
    // The first one made, but the app stopped before its ticket knew it; the second known to its
    // ticket, but stopped before its first message.
    let (orphan, never) = (made[0].clone(), made[1].clone());
    let agents = [None, Some(never.clone()), Some("disparu".to_string()), None];
    for (id, agent) in ids.iter().zip(agents) {
        h.core
            .edit_ticket(id, |t| {
                t.column = Column::Doing;
                t.iteration = 1;
                t.started_at = Some(1);
                t.criteria[0].ok = true;
                t.agent_id = agent;
                Ok(())
            })
            .unwrap();
    }
    // A blocked one shows why, and waits for "Reprendre".
    h.core
        .edit_ticket(&ids[3], |t| {
            t.blocked = Some(NO_CLAUDE.into());
            Ok(())
        })
        .unwrap();
    {
        // No scheduling pass runs meanwhile: what recovery left is seen before anything starts.
        let _pass = h.core.board_lock.lock().await;
        h.core.recover_tickets();
        for id in &ids[..3] {
            let t = h.ticket(id);
            assert_eq!(
                (
                    t.column,
                    t.agent_id.clone(),
                    t.iteration,
                    t.started_at,
                    t.criteria[0].ok,
                    t.forced
                ),
                (Column::Todo, None, 0, None, false, true),
                "{}",
                t.title
            );
        }
        let blocked = h.ticket(&ids[3]);
        assert_eq!(
            (blocked.column, blocked.blocked.as_deref()),
            (Column::Doing, Some(NO_CLAUDE))
        );
    }
    h.wait("the agents made for them archived", |h| {
        h.agent(&orphan).archived && h.agent(&never).archived
    })
    .await;
    // They were under way: they start again, even with the autopilot off.
    for id in &ids[..3] {
        h.wait_ticket(id, "started again to test", |t| t.column == Column::Review)
            .await;
        let agent = h.ticket(id).agent_id;
        assert!(
            ![Some(orphan.clone()), Some(never.clone())].contains(&agent),
            "{agent:?}"
        );
    }
}

#[tokio::test]
async fn a_ticket_under_way_whose_agent_sits_idle_is_asked_to_go_on_at_the_next_start() {
    let h = harness("tk-recover-idle");
    let (p, r) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let t = h
        .core
        .ticket_create(&p.id, draft("Coincé", &["Un"], 5))
        .await
        .unwrap();
    // Its turn ended, but the app stopped before reading it: nothing would move it again.
    let a = h.agent_that_worked(&p.id).await;
    h.core
        .edit_ticket(&t.id, |t| {
            t.column = Column::Doing;
            t.iteration = 1;
            t.agent_id = Some(a.clone());
            Ok(())
        })
        .unwrap();
    assert!(h.core.cut_turns.lock().is_empty());
    h.core.recover_tickets();
    h.wait_sent(
        &r,
        "L'app a redémarré pendant ton travail sur DEM-1 : reprends là où tu en étais",
    )
    .await;
    // One waiting for its quota is left to its automatic resume.
    let u = h
        .core
        .ticket_create(&p.id, draft("Quota", &["Un"], 5))
        .await
        .unwrap();
    let b = h.agent_that_worked(&p.id).await;
    h.core.agent(&b).unwrap().lock().meta.resume_at = Some(now_ms() + 3_600_000);
    h.core
        .edit_ticket(&u.id, |t| {
            t.column = Column::Doing;
            t.iteration = 1;
            t.agent_id = Some(b.clone());
            Ok(())
        })
        .unwrap();
    let restarts = |h: &Harness| {
        user_texts(&h.core, &a)
            .iter()
            .filter(|x| x.starts_with("L'app a redémarré"))
            .count()
    };
    let before = restarts(&h);
    h.core.recover_tickets();
    // Asked along with the other one, it would have been by now.
    h.wait("the idle one asked again", |h| restarts(h) > before)
        .await;
    h.wait_board_idle().await;
    assert_eq!(user_texts(&h.core, &b), ["Bonjour"]);
    assert!(h.agent(&b).resume_at.is_some());
}

/// The issues of the project's board the window was told, in order (None: cleared).
fn board_issues(h: &Harness, project_id: &str) -> Vec<Option<String>> {
    h.events
        .lock()
        .iter()
        .filter(|e| e["type"] == "boardIssue" && e["projectId"] == project_id)
        .map(|e| e["issue"].as_str().map(str::to_string))
        .collect()
}

#[tokio::test]
async fn no_ticket_starts_while_the_target_branch_is_missing() {
    let h = harness("tk-no-target");
    let (p, r) = h.project(false).await;
    h.set_board(&p.id, |s| s.target = "disparue".into());
    let t = h
        .core
        .ticket_create(&p.id, draft("Patienter [ok]", &[], 5))
        .await
        .unwrap();
    h.core.schedule_now().await;
    h.wait_board_idle().await;
    // Nothing made, nothing blocked: the ticket waits for its target.
    let waiting = h.ticket(&t.id);
    assert_eq!(
        (waiting.column, waiting.agent_id, waiting.blocked),
        (Column::Todo, None, None)
    );
    assert_eq!(git(&r, &["branch", "--list", "ticket/*"]), "");
    assert!(h.alerts().is_empty(), "{:?}", h.alerts());
    // The window is told why, once.
    let missing = "Branche cible disparue introuvable — aucun ticket ne démarre";
    assert_eq!(board_issues(&h, &p.id), [Some(missing.to_string())]);
    assert_eq!(
        h.core.board_issues.lock().get(&p.id).map(String::as_str),
        Some(missing)
    );
    // Another target chosen in the settings: the ticket starts.
    h.set_board(&p.id, |s| s.target = "main".into());
    h.wait_ticket(&t.id, "started once its target exists", |t| {
        t.column == Column::Review
    })
    .await;
    assert!(!h.core.board_issues.lock().contains_key(&p.id));
    assert_eq!(board_issues(&h, &p.id), [Some(missing.to_string()), None]);
}

#[tokio::test]
async fn a_target_branch_made_again_starts_the_queue_at_the_next_git_refresh() {
    let h = harness("tk-target-back");
    let (p, r) = h.project(false).await;
    h.set_board(&p.id, |s| s.target = "release".into());
    let t = h
        .core
        .ticket_create(&p.id, draft("Patienter [ok]", &[], 5))
        .await
        .unwrap();
    h.wait_board_idle().await;
    assert_eq!(h.ticket(&t.id).column, Column::Todo);
    assert!(h.core.board_issues.lock().contains_key(&p.id));
    // The branch made again (no board event): the project's git refresh looks again.
    git(&r, &["branch", "release"]);
    h.core.compute_git(&p.id).await;
    h.wait_ticket(&t.id, "started from its target", |t| {
        t.column == Column::Review
    })
    .await;
    assert!(!h.core.board_issues.lock().contains_key(&p.id));
    assert_eq!(
        h.agent_of(&t.id).worktree.unwrap().base_branch.as_str(),
        "release"
    );
}

#[tokio::test]
async fn a_board_with_nothing_left_to_start_still_drops_its_issue_once_its_target_is_back() {
    let h = harness("tk-target-idle");
    let (p, r) = h.project(false).await;
    h.set_board(&p.id, |s| s.target = "disparue".into());
    let t = h
        .core
        .ticket_create(&p.id, draft("Patienter", &[], 5))
        .await
        .unwrap();
    h.wait_board_idle().await;
    assert!(h.core.board_issues.lock().contains_key(&p.id));
    // Nothing waits any more; the branch comes back.
    h.core.ticket_delete(&t.id).await.unwrap();
    h.wait_board_idle().await;
    git(&r, &["branch", "disparue"]);
    h.core.compute_git(&p.id).await;
    h.wait("the issue gone", |h| {
        !h.core.board_issues.lock().contains_key(&p.id)
    })
    .await;
    assert_eq!(board_issues(&h, &p.id).last(), Some(&None));
}

#[tokio::test]
async fn a_board_on_a_new_repository_waits_for_its_first_commit_then_starts() {
    let h = harness("tk-unborn");
    let dir = h.dir.join("neuf");
    std::fs::create_dir_all(&dir).unwrap();
    // A folder that is not a repository yet: the project makes one, with no commit.
    let p = h
        .core
        .create_project(
            &dir.to_string_lossy(),
            "demo",
            "oklch(0.72 0.12 48)",
            false,
            None,
        )
        .await
        .unwrap();
    let branch = git(&dir, &["symbolic-ref", "--short", "HEAD"]);
    let t = h
        .core
        .ticket_create(&p.id, draft("Premier [ok]", &[], 5))
        .await
        .unwrap();
    // The branch the project is on, though it has no commit yet.
    assert_eq!(h.core.project(&p.id).unwrap().board.target, branch);
    h.wait_board_idle().await;
    let waiting = h.ticket(&t.id);
    assert_eq!((waiting.column, waiting.agent_id), (Column::Todo, None));
    let unborn = format!("{branch} n'a encore aucun commit — aucun ticket ne démarre");
    assert_eq!(board_issues(&h, &p.id), [Some(unborn.clone())]);
    // The first commit, seen by the project's git refresh: the queue starts.
    git(&dir, &["config", "user.email", "t@t"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join("README.md"), "Neuf\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-qm", "init"]);
    h.core.compute_git(&p.id).await;
    h.wait_ticket(&t.id, "started after the first commit", |t| {
        t.column == Column::Review
    })
    .await;
    assert_eq!(board_issues(&h, &p.id), [Some(unborn), None]);
}

#[tokio::test]
async fn a_first_ticket_on_a_detached_head_asks_for_a_target() {
    let h = harness("tk-detached");
    let (p, r) = h.project(false).await;
    git(&r, &["checkout", "-q", "--detach"]);
    let e = h
        .core
        .ticket_create(&p.id, draft("Un", &[], 5))
        .await
        .unwrap_err();
    assert_eq!(
        format!("{e:#}"),
        "Le projet n'est sur aucune branche : choisis la branche cible dans les réglages du Kanban."
    );
    // Nothing made, no number taken.
    assert!(h.core.tickets.read().is_empty());
    let board = h.core.project(&p.id).unwrap().board;
    assert_eq!((board.target.as_str(), board.next_number), ("", 1));
    // The settings still list the branches; once one is chosen, the ticket is made.
    assert_eq!(h.core.git_branches(&p.id).await.unwrap(), ["main"]);
    h.set_board(&p.id, |s| {
        s.target = "main".into();
        s.autopilot = false;
    });
    let t = h
        .core
        .ticket_create(&p.id, draft("Un", &[], 5))
        .await
        .unwrap();
    assert_eq!(t.key, "DEM-1");
    assert_eq!(h.core.project(&p.id).unwrap().board.target, "main");
}

#[tokio::test]
async fn a_board_saved_without_a_target_takes_the_projects_branch_for_good() {
    let h = harness("tk-target-fixed");
    let (p, r) = h.project(false).await;
    // Saved by an earlier version, its first ticket made on a branch-less HEAD.
    h.core.tickets.write().push(Ticket {
        id: "t1".into(),
        project_id: p.id.clone(),
        key: "DEM-1".into(),
        title: "Ancien [ok]".into(),
        criteria: vec![Criterion {
            text: "Un".into(),
            ..Default::default()
        }],
        max_loops: 5,
        ..Default::default()
    });
    h.core.schedule_now().await;
    h.wait_ticket("t1", "started", |t| t.column == Column::Review)
        .await;
    // Fixed once: checking out another branch no longer moves it.
    assert_eq!(h.core.project(&p.id).unwrap().board.target, "main");
    git(&r, &["checkout", "-qb", "ailleurs"]);
    assert_eq!(h.core.project(&p.id).unwrap().board.target, "main");
    assert!(h
        .events
        .lock()
        .iter()
        .any(|e| e["type"] == "project" && e["project"]["board"]["target"] == "main"));
}

#[tokio::test]
async fn resuming_a_ticket_whose_agent_waits_for_its_quota_is_refused() {
    use chrono::TimeZone;
    let h = harness("tk-resume-quota");
    let (p, r) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let t = h
        .core
        .ticket_create(&p.id, draft("Quota", &["Un"], 5))
        .await
        .unwrap();
    let a = h.agent_that_worked(&p.id).await;
    let at = chrono::Local
        .with_ymd_and_hms(2026, 10, 3, 15, 7, 0)
        .unwrap()
        .timestamp_millis();
    h.core.agent(&a).unwrap().lock().meta.resume_at = Some(at);
    h.core
        .edit_ticket(&t.id, |t| {
            t.column = Column::Doing;
            t.iteration = 1;
            t.agent_id = Some(a.clone());
            t.blocked = Some("Interrompu".into());
            Ok(())
        })
        .unwrap();
    // A message now would meet the limit again: its automatic resume takes it up.
    let e = h.core.ticket_resume(&t.id).await.unwrap_err();
    assert_eq!(format!("{e:#}"), "En attente du quota — reprise à 15:07");
    assert_eq!(h.ticket(&t.id).blocked.as_deref(), Some("Interrompu"));
    assert_eq!(user_texts(&h.core, &a), ["Bonjour"]);
    assert_eq!(h.agent(&a).resume_at, Some(at));
    assert_eq!(h.stdin_messages(&r).len(), 1);
}

// ---------- validation ----------

/// A ticket of `title` that reached "À tester", and its worktree ([ok]: in one loop).
async fn reviewed(h: &Harness, project_id: &str, title: &str) -> (Ticket, PathBuf) {
    let t = h
        .core
        .ticket_create(project_id, draft(title, &[], 5))
        .await
        .unwrap();
    h.wait_ticket(&t.id, "to test", |t| t.column == Column::Review)
        .await;
    (h.ticket(&t.id), h.worktree_of(&t.id))
}

/// The ticket's branch still exists in `repo`.
fn has_branch(repo: &Path, branch: &str) -> bool {
    std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "rev-parse",
            "--verify",
            "-q",
            &format!("refs/heads/{branch}"),
        ])
        .status()
        .unwrap()
        .success()
}

#[tokio::test]
async fn validating_squashes_the_ticket_into_the_projects_branch_with_a_generated_message() {
    let h = harness("tk-squash");
    let (p, r) = h.project(false).await;
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    assert_eq!(t.column, Column::Done, "{:?}", t.blocked);
    assert_eq!(t.outcome.as_deref(), Some("⤵ Mergé dans main · squash"));
    assert!(t.done_at.is_some() && t.cost > 0.0);
    assert_eq!(t.step, None);
    assert_eq!(
        std::fs::read_to_string(r.join("dem-1.txt")).unwrap(),
        "Boucle 1\n"
    );
    assert_eq!(
        git(&r, &["log", "-1", "--format=%s"]),
        "feat: travail du faux claude [DEM-1]"
    );
    assert_eq!(git(&r, &["rev-list", "--count", "HEAD"]), "2");
    // Cleaned up: its agent archived, its worktree and its branch gone.
    assert!(h.agent(t.agent_id.as_deref().unwrap()).archived);
    assert!(!wt.exists());
    assert!(!has_branch(&r, "ticket/dem-1"));
    assert!(
        h.alerts().iter().all(|a| !a.contains("bloqué")),
        "{:?}",
        h.alerts()
    );
}

#[tokio::test]
async fn a_merged_tickets_worktree_goes_even_while_its_agent_takes_its_time_to_stop() {
    let h = harness("tk-cleanup-linger");
    let (p, r) = h.project(false).await;
    // Its process lasts a while once its input is closed, and something still holds it (a
    // request in flight): the archive alone leaves it running, and on Windows holding the folder.
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok] [tenace]").await;
    let aid = t.agent_id.clone().unwrap();
    let held = h.core.agent(&aid).unwrap().lock().proc.clone().unwrap();
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    assert_eq!(t.column, Column::Done, "{:?}", t.blocked);
    assert!(!wt.exists(), "{}", wt.display());
    assert!(!has_branch(&r, "ticket/dem-1"));
    // Stopped for good, with all it started.
    assert!(!held.is_alive());
}

#[tokio::test]
async fn a_merged_tickets_worktree_that_cannot_be_removed_is_said_kept() {
    let h = harness("tk-cleanup-kept");
    let (p, r) = h.project(false).await;
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    // Locked: git keeps it registered, and its branch with it.
    git(&r, &["worktree", "lock", &wt.to_string_lossy()]);
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    // Merged all the same: done, and the card says what is left.
    assert_eq!(
        (t.column, t.outcome.as_deref()),
        (
            Column::Done,
            Some("⤵ Mergé dans main · squash · worktree gardé")
        ),
        "{:?}",
        t.blocked
    );
    assert!(has_branch(&r, "ticket/dem-1"));
    assert!(h.events.lock().iter().any(|e| e["type"] == "ticket"
        && e["ticket"]["outcome"] == "⤵ Mergé dans main · squash · worktree gardé"));
}

#[tokio::test]
async fn validating_merges_into_a_branch_checked_out_nowhere_through_a_temporary_worktree() {
    let h = harness("tk-merge-temp");
    let (p, r) = h.project(false).await;
    git(&r, &["branch", "release"]);
    h.set_board(&p.id, |s| {
        s.target = "release".into();
        s.strategy = "merge".into();
        s.cleanup = false;
    });
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    // Left behind, registered, by a validation the app's stop cut: it holds the target.
    let stale = r.join(".claude").join("worktrees").join(".merge-dem-1");
    git(
        &r,
        &["worktree", "add", "-q", &stale.to_string_lossy(), "release"],
    );
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    assert_eq!(
        t.outcome.as_deref(),
        Some("⤵ Mergé dans release · merge commit"),
        "{:?}",
        t.blocked
    );
    assert_eq!(git(&r, &["show", "release:dem-1.txt"]), "Boucle 1");
    assert_eq!(
        git(&r, &["rev-list", "--count", "--merges", "release"]),
        "1"
    );
    assert!(!r.join("dem-1.txt").exists() && !stale.exists());
    assert!(!git(&r, &["worktree", "list"]).contains(".merge-dem-1"));
    // "Supprimer le worktree" off: the worktree and the branch stay.
    assert!(wt.is_dir() && has_branch(&r, "ticket/dem-1"));
}

#[tokio::test]
async fn validating_by_rebase_keeps_the_history_straight() {
    let h = harness("tk-rebase");
    let (p, r) = h.project(false).await;
    h.set_board(&p.id, |s| s.strategy = "rebase".into());
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    std::fs::write(r.join("autre.txt"), "main avance\n").unwrap();
    git(&r, &["add", "autre.txt"]);
    git(&r, &["commit", "-qm", "main avance"]);
    h.core.ticket_approve(&t.id).await.unwrap();
    assert_eq!(
        h.ticket(&t.id).outcome.as_deref(),
        Some("⤵ Mergé dans main · rebase"),
        "{:?}",
        h.ticket(&t.id).blocked
    );
    assert_eq!(git(&r, &["rev-list", "--count", "--merges", "main"]), "0");
    assert_eq!(git(&r, &["rev-list", "--count", "main"]), "3");
    assert!(r.join("dem-1.txt").exists() && r.join("autre.txt").exists());
}

#[tokio::test]
async fn a_merge_is_refused_on_a_project_folder_with_uncommitted_changes() {
    let h = harness("tk-dirty");
    let (p, r) = h.project(false).await;
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    std::fs::write(r.join("src").join("app.ts"), "const a = 99; // en cours\n").unwrap();
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    assert_eq!((t.column, t.step.clone()), (Column::Review, None));
    assert_eq!(
        t.blocked.as_deref(),
        Some("Le dossier du projet a des modifications non commitées sur main")
    );
    assert!(h.alerts().iter().any(|x| x.ends_with(
        "DEM-1 bloqué : Le dossier du projet a des modifications non commitées sur main"
    )));
    assert_eq!(
        std::fs::read_to_string(r.join("src").join("app.ts")).unwrap(),
        "const a = 99; // en cours\n"
    );
    // "Réessayer" once the folder is clean: the validation goes through.
    git(&r, &["checkout", "--", "src/app.ts"]);
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    assert_eq!((t.column, t.blocked.as_deref()), (Column::Done, None));
    assert_eq!(
        git(&r, &["log", "-1", "--format=%s"]),
        "feat: travail du faux claude [DEM-1]"
    );
}

#[tokio::test]
async fn a_merge_is_refused_into_a_branch_an_agent_works_on() {
    let h = harness("tk-target-busy");
    let (p, _) = h.project(true).await;
    let other = h.core.create_agent(&p.id, None).await.unwrap();
    let branch = other.meta.worktree.clone().unwrap().branch;
    h.set_board(&p.id, |s| s.target = branch.clone());
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    assert_eq!(
        h.ticket(&t.id).blocked,
        Some(format!(
            "{branch} est extraite dans le worktree de {}",
            other.meta.name
        ))
    );
}

#[tokio::test]
async fn failing_tests_send_the_ticket_back_to_its_agent_with_their_last_lines() {
    let h = harness("tk-tests-fail");
    let (p, _) = h.project(false).await;
    let cmd = "node -e \"console.log('1 test en échec'); process.exit(3)\"";
    h.set_board(&p.id, |s| {
        s.tests_first = true;
        s.test_command = cmd.into();
    });
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    h.wait_sent(&wt, &format!("Les tests (`{cmd}`) échouent :"))
        .await;
    assert!(h.last_sent(&wt).contains("1 test en échec"));
    assert_ne!(h.ticket(&t.id).column, Column::Done);
    // Its agent answers with its report: back to test, from loop 1, its second loop all the same.
    h.wait_ticket(&t.id, "to test again", |t| {
        t.column == Column::Review && t.iteration == 1 && t.loops == 2
    })
    .await;
}

#[tokio::test]
async fn passing_tests_let_the_validation_go_on_with_the_ports_exported() {
    let h = harness("tk-tests-pass");
    let (p, r) = h.project(false).await;
    h.set_board(&p.id, |s| {
        s.tests_first = true;
        s.test_command = "node -e \"process.exit(process.env.ESCOUADE_PORT_BASE ? 0 : 4)\"".into();
        s.conventional = false;
    });
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    assert_eq!(
        h.ticket(&t.id).column,
        Column::Done,
        "{:?}",
        h.ticket(&t.id).blocked
    );
    // Without "Message de commit généré": the key and the title.
    assert_eq!(git(&r, &["log", "-1", "--format=%s"]), "DEM-1 Fichier [ok]");
}

#[tokio::test]
async fn a_haiku_answer_out_of_form_gives_the_titles_message() {
    let h = harness("tk-commit-fallback");
    let (p, r) = h.project(false).await;
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok] [message-libre]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    assert_eq!(
        git(&r, &["log", "-1", "--format=%s"]),
        "feat: Fichier [ok] [message-libre] [DEM-1]",
        "{:?}",
        h.ticket(&t.id).blocked
    );
}

#[tokio::test]
async fn the_env_files_copied_into_the_worktree_never_reach_a_commit() {
    let h = harness("tk-env");
    let (p, r) = h.project(false).await;
    ignore(&r, ".env");
    std::fs::write(r.join(".env"), "SECRET=1\n").unwrap();
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    assert!(wt.join(".env").exists());
    h.core.ticket_approve(&t.id).await.unwrap();
    assert_eq!(
        h.ticket(&t.id).column,
        Column::Done,
        "{:?}",
        h.ticket(&t.id).blocked
    );
    assert_eq!(
        git(&r, &["ls-tree", "-r", "--name-only", "HEAD"]),
        "dem-1.txt\nsrc/app.ts"
    );
}

#[tokio::test]
async fn an_env_file_the_agent_made_is_committed_but_a_copied_one_it_staged_is_not() {
    let h = harness("tk-env-own");
    let (p, r) = h.project(false).await;
    ignore(&r, ".env");
    std::fs::write(r.join(".env"), "SECRET=1\n").unwrap();
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    // The agent's own example file, and the copied one it forced into the index (not committed).
    std::fs::write(wt.join(".env.example"), "SECRET=\n").unwrap();
    git(&wt, &["add", "-f", ".env"]);
    h.core.ticket_approve(&t.id).await.unwrap();
    assert_eq!(
        h.ticket(&t.id).column,
        Column::Done,
        "{:?}",
        h.ticket(&t.id).blocked
    );
    assert_eq!(
        git(&r, &["ls-tree", "-r", "--name-only", "HEAD"]),
        ".env.example\ndem-1.txt\nsrc/app.ts"
    );
    assert_eq!(
        std::fs::read_to_string(r.join(".env")).unwrap(),
        "SECRET=1\n"
    );
}

#[tokio::test]
async fn a_copied_file_the_agent_committed_stops_the_merge() {
    let h = harness("tk-env-committed");
    let (p, r) = h.project(false).await;
    ignore(&r, ".env");
    std::fs::write(r.join(".env"), "SECRET=1\n").unwrap();
    // Merged through a temporary worktree, where nothing else would stop the file.
    git(&r, &["branch", "release"]);
    h.set_board(&p.id, |s| s.target = "release".into());
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok] [commite]").await;
    // The agent committed its work, the copied .env with it.
    assert!(git(&wt, &["ls-tree", "-r", "--name-only", "HEAD"]).contains(".env"));
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    assert_eq!((t.column, t.step.clone()), (Column::Review, None));
    assert_eq!(
        t.blocked.as_deref(),
        Some(".env copié du projet est commité dans la branche")
    );
    assert!(h
        .alerts()
        .iter()
        .any(|x| x.ends_with("DEM-1 bloqué : .env copié du projet est commité dans la branche")));
    // Nothing reached the target.
    assert_eq!(git(&r, &["rev-list", "--count", "release"]), "1");
    assert_eq!(
        git(&r, &["ls-tree", "-r", "--name-only", "release"]),
        "src/app.ts"
    );
}

#[tokio::test]
async fn a_worktree_left_with_unresolved_conflicts_is_not_committed() {
    let h = harness("tk-unmerged");
    let (p, r) = h.project(false).await;
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    // A merge of main the agent left half done.
    std::fs::write(wt.join("src").join("app.ts"), "const a = 2;\n").unwrap();
    git(&wt, &["commit", "-qam", "agent"]);
    commit_change(&r, "const a = 3;\n", "main change");
    let merge = std::process::Command::new("git")
        .arg("-C")
        .arg(&wt)
        .args(["merge", "main"])
        .output()
        .unwrap();
    assert!(!merge.status.success());
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    assert_eq!((t.column, t.step.clone()), (Column::Review, None));
    assert_eq!(
        t.blocked.as_deref(),
        Some("Conflits non résolus dans le worktree sur : src/app.ts")
    );
    // Nothing committed with the markers, nothing merged.
    assert_eq!(git(&wt, &["log", "-1", "--format=%s"]), "agent");
    assert_eq!(
        git(&wt, &["diff", "--name-only", "--diff-filter=U"]),
        "src/app.ts"
    );
    assert_eq!(git(&r, &["rev-list", "--count", "main"]), "2");
}

#[tokio::test]
async fn a_ticket_with_nothing_to_merge_is_done_without_a_change() {
    let h = harness("tk-nothing");
    let (p, r) = h.project(false).await;
    let (t, wt) = reviewed(&h, &p.id, "Rien à changer [ok] [rien]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    assert_eq!(
        (t.column, t.outcome.as_deref(), t.blocked.as_deref()),
        (Column::Done, Some("∅ Aucune modification"), None)
    );
    assert!(t.done_at.is_some() && t.step.is_none());
    assert_eq!(git(&r, &["rev-list", "--count", "main"]), "1");
    // Cleaned up as after a merge: its agent archived, its worktree and its empty branch gone.
    assert!(h.agent(t.agent_id.as_deref().unwrap()).archived);
    assert!(!wt.exists());
    assert!(!has_branch(&r, "ticket/dem-1"));
    // A change made and then taken back is no change either.
    let (u, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    std::fs::remove_file(wt.join("dem-2.txt")).unwrap();
    h.set_board(&p.id, |s| s.cleanup = false);
    h.core.ticket_approve(&u.id).await.unwrap();
    let u = h.ticket(&u.id);
    assert_eq!(
        (u.column, u.outcome.as_deref()),
        (Column::Done, Some("∅ Aucune modification"))
    );
    // Without the cleanup, its worktree stays.
    assert!(wt.is_dir() && has_branch(&r, "ticket/dem-2"));
}

#[tokio::test]
async fn a_second_validation_while_one_runs_is_refused() {
    let h = harness("tk-double");
    let (p, r) = h.project(false).await;
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    let (a, b) = tokio::join!(h.core.ticket_approve(&t.id), h.core.ticket_approve(&t.id));
    assert!(a.is_ok() != b.is_ok(), "{a:?} {b:?}");
    assert_eq!(h.ticket(&t.id).column, Column::Done);
    assert_eq!(git(&r, &["rev-list", "--count", "HEAD"]), "2");
    // Done: no third one either.
    assert!(h.core.ticket_approve(&t.id).await.is_err());
}

#[tokio::test]
async fn a_ticket_is_not_validated_while_its_agent_works() {
    let h = harness("tk-approve-busy");
    let (p, r) = h.project(false).await;
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    let aid = t.agent_id.clone().unwrap();
    h.core
        .send_message(&aid, "Encore un détail [lent]".into(), vec![])
        .await
        .unwrap();
    h.wait("its turn running", |h| {
        h.agent(&aid).status == AgentStatus::Running
    })
    .await;
    let e = h.core.ticket_approve(&t.id).await.unwrap_err();
    assert_eq!(
        format!("{e:#}"),
        "L'agent de ce ticket travaille encore : attends la fin de son tour pour valider."
    );
    let still = h.ticket(&t.id);
    assert_eq!(
        (still.column, still.step, still.blocked),
        (Column::Review, None, None)
    );
    assert_eq!(git(&r, &["rev-list", "--count", "HEAD"]), "1");
    // Its turn over, the validation goes.
    h.core.interrupt(&aid).await.unwrap();
    h.wait("its turn over", |h| !h.agent(&aid).status.is_active())
        .await;
    h.core.ticket_approve(&t.id).await.unwrap();
    assert_eq!(
        h.ticket(&t.id).column,
        Column::Done,
        "{:?}",
        h.ticket(&t.id).blocked
    );
}

#[tokio::test]
async fn an_agent_that_cannot_be_archived_leaves_its_ticket_done() {
    let h = harness("tk-archive-fails");
    let (p, r) = h.project(false).await;
    // The tests wait for the go of this test; meanwhile the agent goes from under the validation.
    let go = h.dir.join("go");
    let cmd = format!(
        "node -e \"const t = setInterval(() => {{ if (require('fs').existsSync('{}')) clearInterval(t); }}, 20)\"",
        go.to_string_lossy().replace('\\', "/")
    );
    h.set_board(&p.id, |s| {
        s.tests_first = true;
        s.test_command = cmd;
    });
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    let aid = t.agent_id.clone().unwrap();
    let meanwhile = async {
        h.wait_ticket(&t.id, "tests running", |t| {
            t.step.as_deref() == Some("Tests…")
        })
        .await;
        // Deleted (its process killed) without its ticket knowing: archiving it fails.
        let gone = h.core.agents.write().remove(&aid).unwrap();
        if let Some(p) = gone.lock().proc.take() {
            p.kill();
        }
        std::fs::write(&go, "").unwrap();
    };
    let (approved, ()) = tokio::join!(h.core.ticket_approve(&t.id), meanwhile);
    approved.unwrap();
    // Merged: the ticket is done all the same, not blocked by what came after.
    let t = h.ticket(&t.id);
    assert_eq!(
        (t.column, t.step.as_deref(), t.blocked.as_deref()),
        (Column::Done, None, None)
    );
    assert_eq!(t.outcome.as_deref(), Some("⤵ Mergé dans main · squash"));
    assert_eq!(git(&r, &["rev-list", "--count", "HEAD"]), "2");
    assert_eq!(h.alerts(), ["demo | DEM-1 prêt à tester"]);
    // Nor does it keep the merged worktree and branch.
    assert!(!wt.exists() && !has_branch(&r, "ticket/dem-1"));
}

#[tokio::test]
async fn archiving_its_agent_during_the_validation_stops_it_before_anything_is_merged() {
    let h = harness("tk-approve-archived");
    let (p, r) = h.project(false).await;
    let go = h.dir.join("go");
    let cmd = format!(
        "node -e \"const t = setInterval(() => {{ if (require('fs').existsSync('{}')) clearInterval(t); }}, 20)\"",
        go.to_string_lossy().replace('\\', "/")
    );
    h.set_board(&p.id, |s| {
        s.tests_first = true;
        s.test_command = cmd;
    });
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let aid = t.agent_id.clone().unwrap();
    let meanwhile = async {
        h.wait_ticket(&t.id, "tests running", |t| {
            t.step.as_deref() == Some("Tests…")
        })
        .await;
        h.core.archive_agent(&aid, true).await.unwrap();
        std::fs::write(&go, "").unwrap();
    };
    let (approved, ()) = tokio::join!(h.core.ticket_approve(&t.id), meanwhile);
    approved.unwrap();
    // Back to do with its agent gone, as an archive does; nothing went on after it.
    let t = h.ticket(&t.id);
    assert_eq!(
        (
            t.column,
            t.step.as_deref(),
            t.blocked.as_deref(),
            t.agent_id
        ),
        (Column::Todo, None, None, None)
    );
    assert_eq!(git(&r, &["rev-list", "--count", "main"]), "1");
    assert_eq!(git(&wt, &["status", "--porcelain"]), "?? dem-1.txt");
    assert_eq!(h.alerts(), ["demo | DEM-1 prêt à tester"]);
}

#[tokio::test]
async fn validating_left_as_is_keeps_the_agent_and_its_changes() {
    let h = harness("tk-keep");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.action = "keep".into());
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    assert_eq!(
        (t.column, t.outcome.as_deref()),
        (Column::Done, Some("◇ Laissé dans le worktree"))
    );
    assert!(!h.agent(t.agent_id.as_deref().unwrap()).archived);
    assert_eq!(git(&wt, &["status", "--porcelain"]), "?? dem-1.txt");
}

/// A test command that waits for the file `go`, then ends with `code`.
fn gated_tests(go: &Path, code: u8) -> String {
    format!(
        "node -e \"const t = setInterval(() => {{ if (require('fs').existsSync('{}')) {{ clearInterval(t); process.exitCode = {code}; }} }}, 20)\"",
        go.to_string_lossy().replace('\\', "/")
    )
}

#[tokio::test]
async fn failing_tests_of_a_ticket_whose_agent_was_archived_meanwhile_leave_it_alone() {
    let h = harness("tk-tests-fail-archived");
    let (p, _) = h.project(false).await;
    let go = h.dir.join("go");
    h.set_board(&p.id, |s| {
        s.tests_first = true;
        s.test_command = gated_tests(&go, 1);
    });
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    let a = t.agent_id.clone().unwrap();
    let launches = h.launches(&wt).len();
    let meanwhile = async {
        h.wait_ticket(&t.id, "tests running", |t| {
            t.step.as_deref() == Some("Tests…")
        })
        .await;
        // Archived: the ticket goes back to do, and starts again with another agent.
        h.core.archive_agent(&a, true).await.unwrap();
        h.wait_ticket(&t.id, "to test with another agent", |x| {
            x.column == Column::Review && x.agent_id.as_deref().is_some_and(|b| b != a)
        })
        .await;
        std::fs::write(&go, "").unwrap();
    };
    let (approved, ()) = tokio::join!(h.core.ticket_approve(&t.id), meanwhile);
    approved.unwrap();
    // The ticket is the new agent's, as it was.
    let after = h.ticket(&t.id);
    assert_ne!(after.agent_id.as_deref(), Some(a.as_str()));
    assert_eq!(
        (
            after.column,
            after.iteration,
            after.step.as_deref(),
            after.blocked.as_deref()
        ),
        (Column::Review, 1, None, None)
    );
    // The archived agent was neither told about the tests nor started again.
    assert!(h.agent(&a).archived);
    assert_eq!(h.launches(&wt).len(), launches);
    assert!(
        !h.stdin_messages(&wt).iter().any(|m| m["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .starts_with("Les tests"))
    );
}

#[tokio::test]
async fn a_copied_file_committed_then_removed_still_stops_the_merge() {
    let h = harness("tk-env-history");
    let (p, r) = h.project(false).await;
    ignore(&r, ".env");
    std::fs::write(r.join(".env"), "SECRET=1\n").unwrap();
    h.set_board(&p.id, |s| s.strategy = "merge".into());
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok] [commite] [retire-env]").await;
    // Gone from the branch's files, still in its history (which a merge commit would bring).
    assert_eq!(
        git(&wt, &["diff", "--name-only", "main...HEAD"]),
        "dem-1.txt"
    );
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    // Told apart from one still in the branch: what is to fix is its history.
    assert_eq!(
        (t.column, t.blocked.as_deref()),
        (
            Column::Review,
            Some(".env copié du projet est dans l'historique de la branche")
        )
    );
    assert_eq!(git(&r, &["rev-list", "--count", "main"]), "1");
}

#[tokio::test]
async fn a_copied_file_a_merge_commit_of_the_agent_brought_stops_the_merge() {
    let h = harness("tk-env-merge");
    let (p, r) = h.project(false).await;
    ignore(&r, ".env");
    std::fs::write(r.join(".env"), "SECRET=1\n").unwrap();
    git(&r, &["branch", "release"]);
    h.set_board(&p.id, |s| s.target = "release".into());
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    // The target moved on; the agent merged it into its branch and, resolving with
    // `git add -A` and forcing what git ignores, committed the copied .env with the merge.
    commit_change(&r, "const a = 3;\n", "main change");
    git(&r, &["branch", "-f", "release", "main"]);
    git(&wt, &["merge", "-q", "--no-ff", "--no-commit", "release"]);
    git(&wt, &["add", "-A"]);
    git(&wt, &["add", "-f", ".env"]);
    git(&wt, &["commit", "-qm", "merge release"]);
    assert!(git(&wt, &["ls-tree", "-r", "--name-only", "HEAD"]).contains(".env"));
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    assert_eq!(
        (t.column, t.blocked.as_deref()),
        (
            Column::Review,
            Some(".env copié du projet est commité dans la branche")
        )
    );
    assert_eq!(
        git(&r, &["ls-tree", "-r", "--name-only", "release"]),
        "src/app.ts"
    );
}

#[tokio::test]
async fn the_merge_lock_of_a_repository_is_one_however_its_path_is_spelled() {
    use crate::tickets::merge_lock_key;
    let h = harness("tk-lock-key");
    let (_, r) = h.project(false).await;
    let a = r.to_string_lossy().to_string();
    let b = if cfg!(windows) {
        a.replace('\\', "/").to_uppercase()
    } else {
        format!("{a}/")
    };
    assert_eq!(merge_lock_key(&a), merge_lock_key(&b));
    assert!(std::sync::Arc::ptr_eq(
        &h.core.merge_lock(&a),
        &h.core.merge_lock(&b)
    ));
    assert_ne!(
        merge_lock_key(&a),
        merge_lock_key(&h.dir.join("data").to_string_lossy())
    );
    // A folder that is not there: compared as it is written, case and separators aside.
    assert_eq!(
        merge_lock_key("C:\\Nulle\\Part\\"),
        merge_lock_key("c:/nulle/part")
    );
}

#[tokio::test]
async fn archiving_its_agent_while_its_merge_waits_merges_nothing() {
    let h = harness("tk-lock-archive");
    let (p, r) = h.project(false).await;
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let aid = t.agent_id.clone().unwrap();
    // Another validation merges into this repository: this one waits for it.
    let repo = h.core.toplevel(&p.path).await.unwrap();
    let lock = h.core.merge_lock(&repo);
    let held = lock.lock().await;
    let meanwhile = async {
        h.wait_ticket(&t.id, "at the merge", |t| {
            t.step.as_deref() == Some("Merge…")
        })
        .await;
        h.core.archive_agent(&aid, true).await.unwrap();
        drop(held);
    };
    let (approved, ()) = tokio::join!(h.core.ticket_approve(&t.id), meanwhile);
    approved.unwrap();
    let t = h.ticket(&t.id);
    assert_eq!(
        (t.column, t.step.as_deref(), t.blocked.as_deref()),
        (Column::Todo, None, None)
    );
    assert_eq!(git(&r, &["rev-list", "--count", "main"]), "1");
}

#[tokio::test]
async fn a_merge_leftover_of_another_ticket_does_not_hold_the_target() {
    let h = harness("tk-merge-leftover");
    let (p, r) = h.project(false).await;
    git(&r, &["branch", "release"]);
    h.set_board(&p.id, |s| s.target = "release".into());
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    // Left, registered, by the cut validation of another ticket.
    let stale = r.join(".claude").join("worktrees").join(".merge-dem-9");
    git(
        &r,
        &["worktree", "add", "-q", &stale.to_string_lossy(), "release"],
    );
    h.core.ticket_approve(&t.id).await.unwrap();
    assert_eq!(
        h.ticket(&t.id).outcome.as_deref(),
        Some("⤵ Mergé dans release · squash"),
        "{:?}",
        h.ticket(&t.id).blocked
    );
    assert_eq!(git(&r, &["show", "release:dem-1.txt"]), "Boucle 1");
    assert!(!stale.exists());
    assert!(!git(&r, &["worktree", "list"]).contains(".merge-"));
}

#[tokio::test]
async fn a_worktree_no_longer_on_its_branch_is_not_committed() {
    let h = harness("tk-detached");
    let (p, r) = h.project(false).await;
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    git(&wt, &["checkout", "-q", "--detach"]);
    let tip = git(&r, &["rev-parse", "ticket/dem-1"]);
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    assert_eq!(
        (t.column, t.blocked.as_deref()),
        (
            Column::Review,
            Some("Le worktree n'est plus sur la branche ticket/dem-1")
        )
    );
    assert_eq!(git(&r, &["rev-parse", "ticket/dem-1"]), tip);
    assert_eq!(git(&wt, &["status", "--porcelain"]), "?? dem-1.txt");
    assert_eq!(git(&r, &["rev-list", "--count", "main"]), "1");
}

#[tokio::test]
async fn validations_into_the_same_folder_merge_one_at_a_time() {
    let h = harness("tk-merge-lock");
    let (p, r) = h.project(false).await;
    let (a, _) = reviewed(&h, &p.id, "Un [ok]").await;
    let (b, _) = reviewed(&h, &p.id, "Deux [ok]").await;
    let (c, _) = reviewed(&h, &p.id, "Trois [ok]").await;
    // A merge into this repository under way: the validation waits for it.
    let repo = h.core.toplevel(&p.path).await.unwrap();
    let lock = h.core.merge_lock(&repo);
    let held = lock.lock().await;
    let waits = async {
        h.wait_ticket(&a.id, "at the merge", |t| {
            t.step.as_deref() == Some("Merge…")
        })
        .await;
        for _ in 0..25 {
            tokio::time::sleep(Duration::from_millis(20)).await;
            assert_eq!(h.ticket(&a.id).step.as_deref(), Some("Merge…"));
        }
        assert_eq!(git(&r, &["rev-list", "--count", "main"]), "1");
        drop(held);
    };
    let (approved, ()) = tokio::join!(h.core.ticket_approve(&a.id), waits);
    approved.unwrap();
    assert_eq!(h.ticket(&a.id).column, Column::Done);
    // Two at once: one after the other, both merged.
    let (x, y) = tokio::join!(h.core.ticket_approve(&b.id), h.core.ticket_approve(&c.id));
    x.unwrap();
    y.unwrap();
    for id in [&b.id, &c.id] {
        assert_eq!(
            h.ticket(id).column,
            Column::Done,
            "{:?}",
            h.ticket(id).blocked
        );
    }
    assert_eq!(git(&r, &["rev-list", "--count", "main"]), "4");
    assert_eq!(git(&r, &["status", "--porcelain"]), "");
    assert!(["dem-1.txt", "dem-2.txt", "dem-3.txt"]
        .iter()
        .all(|f| r.join(f).exists()));
}

#[tokio::test]
async fn a_haiku_that_never_reads_its_question_does_not_hold_up_the_validation() {
    let h = harness("tk-haiku-stalled");
    // More than a pipe holds: writing it waits for a reader that never comes.
    let prompt = "x".repeat(5 * 1024 * 1024);
    let started = std::time::Instant::now();
    let answer = tokio::time::timeout(
        Duration::from_secs(20),
        h.core
            .one_shot_within("[sourd]", &prompt, Duration::from_secs(2)),
    )
    .await
    .expect("the question to Haiku was never given up");
    assert!(answer.is_err());
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[tokio::test]
async fn a_message_sent_during_the_tests_stops_the_validation_before_its_commit() {
    let h = harness("tk-busy-after-tests");
    let (p, r) = h.project(false).await;
    let go = h.dir.join("go");
    h.set_board(&p.id, |s| {
        s.tests_first = true;
        s.test_command = gated_tests(&go, 0);
    });
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    let aid = t.agent_id.clone().unwrap();
    let tip = git(&r, &["rev-parse", "ticket/dem-1"]);
    let meanwhile = async {
        h.wait_ticket(&t.id, "tests running", |t| {
            t.step.as_deref() == Some("Tests…")
        })
        .await;
        h.core
            .send_message(&aid, "Encore un détail [lent]".into(), vec![])
            .await
            .unwrap();
        h.wait("its turn running", |h| {
            h.agent(&aid).status == AgentStatus::Running
        })
        .await;
        std::fs::write(&go, "").unwrap();
    };
    let (approved, ()) = tokio::join!(h.core.ticket_approve(&t.id), meanwhile);
    approved.unwrap();
    let blocked = h.ticket(&t.id);
    assert_eq!(
        (
            blocked.column,
            blocked.step.as_deref(),
            blocked.blocked.as_deref()
        ),
        (
            Column::Review,
            None,
            Some(
                "L'agent de ce ticket travaille encore : attends la fin de son tour pour valider."
            )
        )
    );
    // Nothing of its work in progress committed.
    assert_eq!(git(&r, &["rev-parse", "ticket/dem-1"]), tip);
    assert_eq!(git(&wt, &["status", "--porcelain"]), "?? dem-1.txt");
    // Its turn over, "Réessayer" goes through.
    h.core.interrupt(&aid).await.unwrap();
    h.wait("its turn over", |h| !h.agent(&aid).status.is_active())
        .await;
    h.core.ticket_approve(&t.id).await.unwrap();
    assert_eq!(
        h.ticket(&t.id).column,
        Column::Done,
        "{:?}",
        h.ticket(&t.id).blocked
    );
}

#[tokio::test]
async fn a_rebase_conflict_leaves_the_tickets_branch_as_it_was() {
    let h = harness("tk-rebase-conflict");
    let (p, r) = h.project(false).await;
    h.set_board(&p.id, |s| s.strategy = "rebase".into());
    // Its agent committed all its work, a change of src/app.ts included.
    let (t, wt) = reviewed(&h, &p.id, "Conflit [ok] [commite]").await;
    std::fs::write(wt.join("src").join("app.ts"), "const a = 2;\n").unwrap();
    git(&wt, &["commit", "-qam", "agent"]);
    commit_change(&r, "const a = 3;\n", "main change");
    let tip = git(&r, &["rev-parse", "ticket/dem-1"]);
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    assert_eq!(t.column, Column::Review);
    assert!(
        t.blocked
            .as_deref()
            .is_some_and(|b| b.starts_with("Conflit avec main")),
        "{:?}",
        t.blocked
    );
    // The rebase undone: the branch, its worktree and the target as they were.
    assert_eq!(git(&r, &["rev-parse", "ticket/dem-1"]), tip);
    assert_eq!(
        git(&wt, &["rev-parse", "--abbrev-ref", "HEAD"]),
        "ticket/dem-1"
    );
    assert_eq!(git(&wt, &["status", "--porcelain"]), "");
    assert_eq!(git(&r, &["rev-list", "--count", "main"]), "2");
}

#[tokio::test]
async fn a_merge_conflict_in_the_project_folder_leaves_the_users_untracked_files() {
    let h = harness("tk-merge-conflict");
    let (p, r) = h.project(false).await;
    h.set_board(&p.id, |s| s.strategy = "merge".into());
    let (t, wt) = reviewed(&h, &p.id, "Conflit [ok]").await;
    std::fs::write(wt.join("src").join("app.ts"), "const a = 2;\n").unwrap();
    commit_change(&r, "const a = 3;\n", "main change");
    // The user's own files, never in git.
    std::fs::write(r.join("notes.txt"), "à garder\n").unwrap();
    std::fs::write(r.join("src").join("brouillon.ts"), "// à garder\n").unwrap();
    let head = git(&r, &["rev-parse", "HEAD"]);
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    assert_eq!(t.column, Column::Review);
    assert!(
        t.blocked
            .as_deref()
            .is_some_and(|b| b.starts_with("Conflit avec main")),
        "{:?}",
        t.blocked
    );
    assert_eq!(git(&r, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        git(&r, &["status", "--porcelain"]),
        "?? notes.txt\n?? src/brouillon.ts"
    );
    assert_eq!(
        std::fs::read_to_string(r.join("notes.txt")).unwrap(),
        "à garder\n"
    );
    assert_eq!(
        std::fs::read_to_string(r.join("src").join("app.ts")).unwrap(),
        "const a = 3;\n"
    );
}

// ---------- pull requests, pushes, conflicts, rejection ----------

/// The project's repository gets a remote that reads as GitHub's but is a local bare repository
/// (a git `insteadOf` rule): pushes work, without network.
fn github_remote(h: &Harness, r: &Path) -> PathBuf {
    let bare = h.dir.join("remote.git");
    git(
        &h.dir,
        &[
            "init",
            "-q",
            "--bare",
            "-b",
            "main",
            &bare.to_string_lossy(),
        ],
    );
    let url = "https://github.com/acme/demo.git";
    git(r, &["remote", "add", "origin", url]);
    let rule = format!(
        "url.{}.insteadOf",
        bare.to_string_lossy().replace('\\', "/")
    );
    git(r, &["config", &rule, url]);
    git(r, &["push", "-qu", "origin", "main"]);
    bare
}

fn fake_gh() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
        .join(if cfg!(windows) {
            "fake-gh.cmd"
        } else {
            "fake-gh"
        })
}

/// The calls of the fake `gh` run in `cwd` (an earlier run's log of the same folder is gone with
/// `test_dir`).
fn gh_calls(cwd: &Path) -> Vec<Value> {
    let key: String = cwd
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    std::fs::read_to_string(std::env::temp_dir().join(format!("fake-gh-{key}.jsonl")))
        .unwrap_or_default()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

/// The branches of a (bare) repository.
fn branches_of(repo: &Path) -> String {
    git(
        repo,
        &["for-each-ref", "--format=%(refname:short)", "refs/heads"],
    )
}

#[tokio::test]
async fn validating_by_push_publishes_the_tickets_branch_and_archives_its_agent() {
    let h = harness("tk-push");
    let (p, r) = h.project(false).await;
    let bare = github_remote(&h, &r);
    h.set_board(&p.id, |s| s.action = "push".into());
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    assert_eq!(
        t.outcome.as_deref(),
        Some("⇡ Poussé sur ticket/dem-1"),
        "{:?}",
        t.blocked
    );
    assert_eq!(t.column, Column::Done);
    assert_eq!(git(&bare, &["show", "ticket/dem-1:dem-1.txt"]), "Boucle 1");
    // Committed with the generated message, and tracking its remote branch.
    assert_eq!(
        git(&bare, &["log", "-1", "--format=%s", "ticket/dem-1"]),
        "feat: travail du faux claude [DEM-1]"
    );
    assert_eq!(
        git(
            &r,
            &["rev-parse", "--abbrev-ref", "ticket/dem-1@{upstream}"]
        ),
        "origin/ticket/dem-1"
    );
    assert!(h.agent(t.agent_id.as_deref().unwrap()).archived);
    // Its worktree goes, its branch (pushed) stays; the target is untouched.
    assert!(!wt.exists() && has_branch(&r, "ticket/dem-1"));
    assert_eq!(git(&r, &["rev-list", "--count", "main"]), "1");
    // Without the cleanup, the worktree stays too.
    h.set_board(&p.id, |s| s.cleanup = false);
    let (u, wt) = reviewed(&h, &p.id, "Autre fichier [ok]").await;
    h.core.ticket_approve(&u.id).await.unwrap();
    assert_eq!(h.ticket(&u.id).column, Column::Done);
    assert!(wt.is_dir() && has_branch(&r, "ticket/dem-2"));
}

#[tokio::test]
async fn without_gh_a_github_pull_request_is_finished_in_the_browser() {
    let h = harness("tk-pr-compare");
    let (p, r) = h.project(false).await;
    let bare = github_remote(&h, &r);
    *h.core.gh.write() = None;
    h.set_board(&p.id, |s| s.action = "pr".into());
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    assert_eq!(
        t.outcome.as_deref(),
        Some("⇡ ticket/dem-1 poussée · PR à finaliser"),
        "{:?}",
        t.blocked
    );
    let url = t.outcome_url.unwrap();
    assert!(
        url.starts_with("https://github.com/acme/demo/compare/main...ticket/dem-1?expand=1&title=feat%3A%20travail%20du%20faux%20claude%20%5BDEM-1%5D&body="),
        "{url}"
    );
    assert!(h
        .events
        .lock()
        .iter()
        .any(|e| e["type"] == "openUrl" && e["url"] == url.as_str()));
    assert_eq!(git(&bare, &["show", "ticket/dem-1:dem-1.txt"]), "Boucle 1");
    assert!(h.agent(t.agent_id.as_deref().unwrap()).archived);
    // Its worktree goes, its branch (pushed, proposed) stays.
    assert!(!wt.exists() && has_branch(&r, "ticket/dem-1"));
}

#[tokio::test]
async fn with_gh_a_pull_request_is_opened_with_the_criteria() {
    let h = harness("tk-pr-gh");
    let (p, r) = h.project(false).await;
    github_remote(&h, &r);
    *h.core.gh.write() = Some(fake_gh());
    h.set_board(&p.id, |s| {
        s.action = "pr".into();
        s.draft = true;
    });
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    assert_eq!(
        t.outcome.as_deref(),
        Some("⇡ PR #12 → main"),
        "{:?}",
        t.blocked
    );
    assert_eq!(
        t.outcome_url.as_deref(),
        Some("https://github.com/acme/demo/pull/12")
    );
    let call = gh_calls(&wt).pop().expect("gh called in the worktree");
    let argv: Vec<&str> = call["argv"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap())
        .collect();
    assert_eq!(
        argv,
        [
            "pr",
            "create",
            "--repo",
            "acme/demo",
            "--base",
            "main",
            "--head",
            "ticket/dem-1",
            "--title",
            "feat: travail du faux claude [DEM-1]",
            "--body-file",
            "-",
            "--draft"
        ]
    );
    // The description through the standard input: what was done, then the criteria.
    assert_eq!(
        call["body"],
        "Ce qui a été fait :\n- Fichier dem-1.txt écrit\n- Boucle 1 faite\n\nCritères :\n- [x] Implémentation conforme au ticket\n- [x] Tests verts"
    );
    // The page is not opened as well.
    assert!(!h.events.lock().iter().any(|e| e["type"] == "openUrl"));
}

#[tokio::test]
async fn a_gh_installed_while_the_app_runs_opens_the_pull_request() {
    let h = harness("tk-pr-gh-later");
    let (p, r) = h.project(false).await;
    github_remote(&h, &r);
    // Not there when the app started; on the PATH by the time of the validation.
    *h.core.gh.write() = None;
    *h.core.gh_on_path.write() = Some(fake_gh());
    h.set_board(&p.id, |s| s.action = "pr".into());
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    assert_eq!(
        h.ticket(&t.id).outcome.as_deref(),
        Some("⇡ PR #12 → main"),
        "{:?}",
        h.ticket(&t.id).blocked
    );
    assert_eq!(h.core.gh.read().clone(), Some(fake_gh()));
}

#[tokio::test]
async fn a_pull_request_to_another_host_than_github_is_left_to_open_there() {
    let h = harness("tk-pr-other");
    let (p, r) = h.project(false).await;
    let bare = h.dir.join("remote.git");
    git(
        &h.dir,
        &[
            "init",
            "-q",
            "--bare",
            "-b",
            "main",
            &bare.to_string_lossy(),
        ],
    );
    git(&r, &["remote", "add", "origin", &bare.to_string_lossy()]);
    *h.core.gh.write() = Some(fake_gh());
    h.set_board(&p.id, |s| s.action = "pr".into());
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    assert_eq!(
        t.outcome.as_deref(),
        Some("⇡ ticket/dem-1 poussée · PR à ouvrir sur ton hébergeur"),
        "{:?}",
        t.blocked
    );
    assert_eq!(t.outcome_url, None);
    assert_eq!(git(&bare, &["show", "ticket/dem-1:dem-1.txt"]), "Boucle 1");
    assert!(gh_calls(&wt).is_empty());
}

#[tokio::test]
async fn a_push_without_remote_leaves_the_ticket_to_test_with_why() {
    let h = harness("tk-push-none");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.action = "push".into());
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    assert_eq!(
        (t.column, t.step.as_deref(), t.blocked.as_deref()),
        (
            Column::Review,
            None,
            Some("Aucun dépôt distant n'est configuré pour ce dépôt.")
        )
    );
    assert!(!h.agent(t.agent_id.as_deref().unwrap()).archived);
}

#[tokio::test]
async fn a_copied_file_the_agent_committed_is_neither_pushed_nor_proposed() {
    for action in ["push", "pr"] {
        let h = harness(&format!("tk-env-{action}"));
        let (p, r) = h.project(false).await;
        ignore(&r, ".env");
        std::fs::write(r.join(".env"), "SECRET=1\n").unwrap();
        let bare = github_remote(&h, &r);
        *h.core.gh.write() = Some(fake_gh());
        h.set_board(&p.id, |s| s.action = action.into());
        let (t, wt) = reviewed(&h, &p.id, "Fichier [ok] [commite]").await;
        h.core.ticket_approve(&t.id).await.unwrap();
        let t = h.ticket(&t.id);
        assert_eq!(
            (t.column, t.blocked.as_deref()),
            (
                Column::Review,
                Some(".env copié du projet est commité dans la branche")
            ),
            "{action}"
        );
        assert_eq!(branches_of(&bare), "main", "{action}");
        assert!(gh_calls(&wt).is_empty(), "{action}");
    }
}

/// A ticket to test whose change of src/app.ts conflicts with one committed on main meanwhile.
async fn conflicting(h: &Harness, project_id: &str, r: &Path) -> (Ticket, PathBuf) {
    let (t, wt) = reviewed(h, project_id, "Conflit [ok]").await;
    std::fs::write(wt.join("src").join("app.ts"), "const a = 2;\n").unwrap();
    commit_change(r, "const a = 3;\n", "main change");
    (t, wt)
}

#[tokio::test]
async fn a_conflict_asks_what_to_do_and_leaves_the_project_clean() {
    let h = harness("tk-conflict-ask");
    let (p, r) = h.project(false).await;
    let (t, wt) = conflicting(&h, &p.id, &r).await;
    h.core.ticket_approve(&t.id).await.unwrap();
    let c = h.ticket(&t.id);
    assert_eq!(
        (c.column, c.conflict, c.blocked.as_deref(), c.step),
        (Column::Review, true, Some("Conflit avec main"), None)
    );
    assert!(h
        .alerts()
        .iter()
        .any(|a| a.ends_with("DEM-1 bloqué : Conflit avec main")));
    assert_eq!(git(&r, &["status", "--porcelain"]), "");
    assert_eq!(
        std::fs::read_to_string(r.join("src").join("app.ts")).unwrap(),
        "const a = 3;\n"
    );
    // Nothing merged into the worktree either: the agent is not asked anything.
    assert_eq!(git(&wt, &["status", "--porcelain"]), "");
    assert!(!h.agent(c.agent_id.as_deref().unwrap()).archived);
    // "Annuler": the block goes, the ticket stays to test.
    h.core.ticket_dismiss(&t.id).unwrap();
    let c = h.ticket(&t.id);
    assert_eq!(
        (c.column, c.conflict, c.blocked),
        (Column::Review, false, None)
    );
}

#[tokio::test]
async fn a_block_is_dismissed_or_handed_to_the_agent_only_on_a_ticket_to_test() {
    let h = harness("tk-dismiss-guard");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let t = h
        .core
        .ticket_create(&p.id, draft("Rien", &[], 5))
        .await
        .unwrap();
    for column in [Column::Todo, Column::Doing, Column::Done] {
        h.core
            .edit_ticket(&t.id, |x| {
                x.column = column;
                x.blocked = Some("Erreur : panne".into());
                Ok(())
            })
            .unwrap();
        let e = h.core.ticket_dismiss(&t.id).unwrap_err();
        assert_eq!(
            format!("{e:#}"),
            "Ce ticket n'est pas à tester.",
            "{column:?}"
        );
        let e = h.core.ticket_resolve_conflict(&t.id).await.unwrap_err();
        assert_eq!(
            format!("{e:#}"),
            "Ce ticket n'est pas à tester.",
            "{column:?}"
        );
        let x = h.ticket(&t.id);
        assert_eq!(
            (x.column, x.blocked.as_deref(), x.step),
            (column, Some("Erreur : panne"), None),
            "{column:?}"
        );
    }
}

#[tokio::test]
async fn when_the_agent_resolves_it_gets_the_target_merged_into_its_branch_with_the_conflicts() {
    let h = harness("tk-conflict-agent");
    let (p, r) = h.project(false).await;
    h.set_board(&p.id, |s| s.conflict = "agent".into());
    let (t, wt) = conflicting(&h, &p.id, &r).await;
    let agent = t.agent_id.clone();
    h.core.ticket_approve(&t.id).await.unwrap();
    h.wait_sent(
        &wt,
        "Le merge de main dans ta branche a des conflits sur : src/app.ts. Résous-les, commite le merge, revérifie les critères, puis termine par le bilan.",
    )
    .await;
    assert!(git(&wt, &["diff", "--name-only", "--diff-filter=U"]).contains("src/app.ts"));
    let c = h.ticket(&t.id);
    assert_ne!(c.column, Column::Done);
    // Loop 1 again, its second loop all the same.
    assert_eq!(
        (c.agent_id, c.iteration, c.loops, c.conflict),
        (agent, 1, 2, false)
    );
    // The project's folder was left as it was.
    assert_eq!(git(&r, &["status", "--porcelain"]), "");
    assert_eq!(git(&r, &["rev-list", "--count", "main"]), "2");
    // Its agent answers with its report: to test again.
    h.wait_ticket(&t.id, "to test again", |t| t.column == Column::Review)
        .await;
}

#[tokio::test]
async fn a_cancelled_conflict_can_still_be_handed_to_the_agent_from_the_card() {
    let h = harness("tk-conflict-later");
    let (p, r) = h.project(false).await;
    h.set_board(&p.id, |s| s.conflict = "abort".into());
    let (t, wt) = conflicting(&h, &p.id, &r).await;
    h.core.ticket_approve(&t.id).await.unwrap();
    let c = h.ticket(&t.id);
    assert_eq!(
        (c.column, c.conflict, c.blocked.as_deref()),
        (
            Column::Review,
            false,
            Some("Conflit avec main sur : src/app.ts")
        )
    );
    h.core.ticket_resolve_conflict(&t.id).await.unwrap();
    h.wait_sent(
        &wt,
        "Le merge de main dans ta branche a des conflits sur : src/app.ts.",
    )
    .await;
    let c = h.ticket(&t.id);
    assert_eq!((c.blocked, c.step, c.iteration), (None, None, 1));
}

#[tokio::test]
async fn a_conflict_is_not_handed_to_the_agent_while_a_validation_runs() {
    let h = harness("tk-conflict-busy");
    let (p, r) = h.project(false).await;
    let (t, wt) = conflicting(&h, &p.id, &r).await;
    h.core.ticket_approve(&t.id).await.unwrap();
    assert!(h.ticket(&t.id).conflict);
    // "Réessayer" runs meanwhile (its tests, its commit…).
    h.core
        .edit_ticket(&t.id, |x| {
            x.step = Some("Tests…".into());
            Ok(())
        })
        .unwrap();
    let e = h.core.ticket_resolve_conflict(&t.id).await.unwrap_err();
    assert_eq!(
        format!("{e:#}"),
        "La validation de ce ticket est déjà en cours."
    );
    // Nothing merged into its worktree, nothing sent.
    assert_eq!(git(&wt, &["status", "--porcelain"]), "");
    assert!(
        !h.stdin_messages(&wt).iter().any(|m| m["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .starts_with("Le merge"))
    );
    let c = h.ticket(&t.id);
    assert_eq!(
        (c.column, c.step.as_deref()),
        (Column::Review, Some("Tests…"))
    );
}

#[tokio::test]
async fn a_conflict_is_not_handed_to_an_agent_at_work() {
    let h = harness("tk-conflict-agent-busy");
    let (p, r) = h.project(false).await;
    h.set_board(&p.id, |s| s.conflict = "abort".into());
    let (t, wt) = conflicting(&h, &p.id, &r).await;
    h.core.ticket_approve(&t.id).await.unwrap();
    let aid = t.agent_id.clone().unwrap();
    h.core
        .send_message(&aid, "Encore un détail [lent]".into(), vec![])
        .await
        .unwrap();
    h.wait("its turn running", |h| {
        h.agent(&aid).status == AgentStatus::Running
    })
    .await;
    let e = h.core.ticket_resolve_conflict(&t.id).await.unwrap_err();
    assert_eq!(
        format!("{e:#}"),
        "L'agent de ce ticket travaille encore : attends la fin de son tour pour valider."
    );
    // Its files left alone; the ticket as it was.
    assert!(git(&wt, &["diff", "--name-only", "--diff-filter=U"]).is_empty());
    let c = h.ticket(&t.id);
    assert_eq!(
        (c.column, c.step, c.blocked.as_deref()),
        (
            Column::Review,
            None,
            Some("Conflit avec main sur : src/app.ts")
        )
    );
    h.core.interrupt(&aid).await.unwrap();
}

#[tokio::test]
async fn a_merge_into_the_worktree_that_fails_without_conflicts_is_reported_not_sent() {
    let h = harness("tk-conflict-merge-fails");
    let (p, r) = h.project(false).await;
    h.set_board(&p.id, |s| s.conflict = "abort".into());
    let (t, wt) = conflicting(&h, &p.id, &r).await;
    h.core.ticket_approve(&t.id).await.unwrap();
    // A change left in the worktree on the file the merge would bring: git refuses to start it.
    std::fs::write(wt.join("src").join("app.ts"), "const a = 9;\n").unwrap();
    let e = h.core.ticket_resolve_conflict(&t.id).await.unwrap_err();
    assert!(format!("{e:#}").contains("src/app.ts"), "{e:#}");
    let c = h.ticket(&t.id);
    assert_eq!(
        (c.column, c.step, c.blocked.as_deref()),
        (
            Column::Review,
            None,
            Some("Conflit avec main sur : src/app.ts")
        )
    );
    assert!(git(&wt, &["diff", "--name-only", "--diff-filter=U"]).is_empty());
    assert_eq!(
        std::fs::read_to_string(wt.join("src").join("app.ts")).unwrap(),
        "const a = 9;\n"
    );
    // The agent is told nothing, "sans conflit" least of all.
    assert!(
        !h.stdin_messages(&wt).iter().any(|m| m["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .contains("main dans ta branche"))
    );
}

#[tokio::test]
async fn a_rejected_ticket_goes_back_to_the_same_agent_with_the_comment() {
    let h = harness("tk-reject");
    let (p, _) = h.project(false).await;
    let (t, wt) = reviewed(&h, &p.id, "Renvoi [ok]").await;
    let agent = h.ticket(&t.id).agent_id;
    assert!(h.core.ticket_reject(&t.id, "  ").await.is_err());
    assert_eq!(h.ticket(&t.id).column, Column::Review);
    // Not while it is being validated.
    let validation = |step: Option<&str>| {
        let step = step.map(str::to_string);
        h.core
            .edit_ticket(&t.id, |x| {
                x.step = step;
                Ok(())
            })
            .unwrap()
    };
    validation(Some("Tests…"));
    let e = h.core.ticket_reject(&t.id, "trop tôt").await.unwrap_err();
    assert_eq!(
        format!("{e:#}"),
        "La validation de ce ticket est déjà en cours."
    );
    assert_eq!(h.ticket(&t.id).column, Column::Review);
    validation(None);
    h.core
        .ticket_reject(&t.id, "le bouton est mal placé")
        .await
        .unwrap();
    h.wait_sent(
        &wt,
        "Retour de test sur DEM-1 : le bouton est mal placé. Corrige, revérifie tous les critères, puis termine par le bilan.",
    )
    .await;
    h.wait_ticket(&t.id, "to test again", |t| t.column == Column::Review)
        .await;
    let back = h.ticket(&t.id);
    // Loop 1 again, its second loop all the same.
    assert_eq!((back.agent_id, back.iteration, back.loops), (agent, 1, 2));
    // Only a ticket to test is sent back.
    h.core.ticket_approve(&t.id).await.unwrap();
    let e = h.core.ticket_reject(&t.id, "encore").await.unwrap_err();
    assert_eq!(format!("{e:#}"), "Ce ticket n'est pas à tester.");
    let done = h.ticket(&t.id);
    assert_eq!(
        (done.column, done.blocked, done.loops),
        (Column::Done, None, 2)
    );
}

/// The agent read a message starting with `start`.
fn was_sent(h: &Harness, dir: &Path, start: &str) -> bool {
    h.stdin_messages(dir).iter().any(|m| {
        m["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .starts_with(start)
    })
}

#[tokio::test]
async fn a_ticket_is_not_sent_back_while_its_agent_works() {
    let h = harness("tk-reject-busy");
    let (p, _) = h.project(false).await;
    let (t, wt) = reviewed(&h, &p.id, "Renvoi [ok]").await;
    let aid = t.agent_id.clone().unwrap();
    // A chat with its agent under way: its end would be read as the end of the rework.
    h.core
        .send_message(&aid, "Encore un détail [lent]".into(), vec![])
        .await
        .unwrap();
    h.wait("its turn running", |h| {
        h.agent(&aid).status == AgentStatus::Running
    })
    .await;
    let e = h
        .core
        .ticket_reject(&t.id, "le bouton est mal placé")
        .await
        .unwrap_err();
    assert_eq!(
        format!("{e:#}"),
        "L'agent de ce ticket travaille encore : attends la fin de son tour pour le renvoyer."
    );
    let still = h.ticket(&t.id);
    assert_eq!(
        (still.column, still.blocked, still.iteration),
        (Column::Review, None, t.iteration)
    );
    assert!(!was_sent(&h, &wt, "Retour de test"));
    h.core.interrupt(&aid).await.unwrap();
}

#[tokio::test]
async fn a_ticket_that_leaves_while_its_target_is_merged_into_its_worktree_leaves_no_merge_behind()
{
    let h = harness("tk-conflict-left");
    let (p, r) = h.project(false).await;
    h.set_board(&p.id, |s| s.conflict = "abort".into());
    let (t, wt) = conflicting(&h, &p.id, &r).await;
    h.core.ticket_approve(&t.id).await.unwrap();
    h.set_board(&p.id, |s| s.autopilot = false);
    // src/app.ts merges through a driver that says it started, waits for the go of this test,
    // then leaves a conflict.
    let (started, go) = (h.dir.join("started"), h.dir.join("go"));
    let slash = |p: &Path| p.to_string_lossy().replace('\\', "/");
    let driver = format!(
        "node -e \"const fs = require('fs'); fs.writeFileSync('{}', ''); const t = setInterval(() => {{ if (fs.existsSync('{}')) {{ clearInterval(t); process.exitCode = 1; }} }}, 20)\"",
        slash(&started),
        slash(&go)
    );
    git(&r, &["config", "merge.attente.driver", &driver]);
    std::fs::write(wt.join(".gitattributes"), "src/app.ts merge=attente\n").unwrap();
    let aid = t.agent_id.clone().unwrap();
    let meanwhile = async {
        h.wait("the merge under way", |_| started.exists()).await;
        // Its agent archived meanwhile: the ticket goes back to do.
        h.core.archive_agent(&aid, true).await.unwrap();
        std::fs::write(&go, "").unwrap();
    };
    let (resolved, ()) = tokio::join!(h.core.ticket_resolve_conflict(&t.id), meanwhile);
    assert_eq!(
        format!("{:#}", resolved.unwrap_err()),
        "Ce ticket a changé pendant sa validation."
    );
    assert_eq!(h.ticket(&t.id).column, Column::Todo);
    // Its worktree as it was: no merge left half done, its conflicts with it.
    assert_eq!(git(&wt, &["status", "--porcelain"]), "?? .gitattributes");
    assert_eq!(
        std::fs::read_to_string(wt.join("src").join("app.ts")).unwrap(),
        "const a = 2;\n"
    );
    assert!(!was_sent(&h, &wt, "Le merge de main"));
}

#[tokio::test]
async fn a_ticket_with_nothing_beyond_its_target_is_done_neither_pushed_nor_proposed() {
    for action in ["push", "pr"] {
        let h = harness(&format!("tk-nothing-{action}"));
        let (p, r) = h.project(false).await;
        let bare = github_remote(&h, &r);
        *h.core.gh.write() = Some(fake_gh());
        h.set_board(&p.id, |s| s.action = action.into());
        let (t, wt) = reviewed(&h, &p.id, "Rien à changer [ok] [rien]").await;
        h.core.ticket_approve(&t.id).await.unwrap();
        let t = h.ticket(&t.id);
        assert_eq!(
            (t.column, t.outcome.as_deref(), t.blocked.as_deref()),
            (Column::Done, Some("∅ Aucune modification"), None),
            "{action}"
        );
        assert_eq!(branches_of(&bare), "main", "{action}");
        assert!(gh_calls(&wt).is_empty(), "{action}");
        assert!(h.agent(t.agent_id.as_deref().unwrap()).archived, "{action}");
        // Nothing worth keeping on its branch: it goes with the worktree.
        assert!(!wt.exists() && !has_branch(&r, "ticket/dem-1"), "{action}");
    }
}

#[tokio::test]
async fn when_the_agent_resolves_a_rebase_conflict_it_is_asked_to_rebase_itself() {
    let h = harness("tk-conflict-agent-rebase");
    let (p, r) = h.project(false).await;
    h.set_board(&p.id, |s| {
        s.strategy = "rebase".into();
        s.conflict = "agent".into();
    });
    let (t, wt) = conflicting(&h, &p.id, &r).await;
    h.core.ticket_approve(&t.id).await.unwrap();
    h.wait_sent(
        &wt,
        "Rebase ta branche sur main et résous ses conflits, puis revérifie les critères et termine par le bilan.",
    )
    .await;
    // The app ran nothing in the worktree: no merge, no rebase, the branch as it was.
    assert_eq!(git(&wt, &["status", "--porcelain"]), "");
    assert_eq!(
        git(&wt, &["rev-parse", "--abbrev-ref", "HEAD"]),
        "ticket/dem-1"
    );
    assert_eq!(git(&r, &["rev-list", "--count", "ticket/dem-1..main"]), "1");
    assert!(!was_sent(&h, &wt, "Le merge de main"));
    let c = h.ticket(&t.id);
    assert_eq!((c.iteration, c.conflict, c.step), (1, false, None));
    assert_ne!(c.column, Column::Done);
}

#[tokio::test]
async fn a_target_that_no_longer_conflicts_is_merged_into_the_branch_and_the_agent_told_so() {
    let h = harness("tk-conflict-gone");
    let (p, r) = h.project(false).await;
    h.set_board(&p.id, |s| s.conflict = "abort".into());
    let (t, wt) = conflicting(&h, &p.id, &r).await;
    h.core.ticket_approve(&t.id).await.unwrap();
    assert!(h.ticket(&t.id).blocked.is_some());
    // The target comes round to the ticket's change meanwhile.
    commit_change(&r, "const a = 2;\n", "main agrees");
    let seen = h.events.lock().len();
    h.core.ticket_resolve_conflict(&t.id).await.unwrap();
    h.wait_sent(
        &wt,
        "J'ai mergé main dans ta branche, sans conflit. Revérifie les critères, puis termine par le bilan.",
    )
    .await;
    // Merged into its branch, by a merge commit; nothing left to resolve.
    assert_eq!(git(&r, &["rev-list", "--count", "ticket/dem-1..main"]), "0");
    assert_eq!(
        git(
            &r,
            &["rev-list", "--count", "--merges", "main..ticket/dem-1"]
        ),
        "1"
    );
    assert_eq!(git(&wt, &["status", "--porcelain"]), "");
    // Back "En cours", loop 1, unblocked (its report then brings it to test again).
    let doing = h.events.lock()[seen..].iter().any(|e| {
        e["type"] == "ticket"
            && e["ticket"]["id"] == t.id.as_str()
            && e["ticket"]["column"] == "doing"
            && e["ticket"]["iteration"] == 1
            && e["ticket"]["blocked"].is_null()
    });
    assert!(doing);
}

// ---------- test launches ----------

#[tokio::test]
async fn preparing_a_launch_reserves_ports_and_keeps_the_recipe_the_agent_answers() {
    let h = harness("tk-prepare");
    let (p, _) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    let b = h.core.create_agent(&p.id, None).await.unwrap().meta;
    h.core.agent_prepare_launch(&a.id).await.unwrap();
    h.core.agent_prepare_launch(&b.id).await.unwrap();
    let (pa, pb) = (
        h.agent(&a.id).port_base.unwrap(),
        h.agent(&b.id).port_base.unwrap(),
    );
    assert!(pa >= 4100 && pa % 10 == 0 && pb != pa);
    let dir = PathBuf::from(&a.cwd);
    h.wait_sent(
        &dir,
        &format!(
            "Prépare le lancement de test de ce worktree. Ports réservés : {pa} à {}",
            pa + 9
        ),
    )
    .await;
    // Its answer is kept as its recipe; a step runs in the worktree with the ports.
    h.wait("recipe", |h| h.agent(&a.id).recipe.is_some()).await;
    let spec = h.core.test_run_spec(&a.id, "run", 0).unwrap();
    assert_eq!(spec.command, "node serveur.js");
    assert_eq!(
        Path::new(&spec.cwd).canonicalize().unwrap(),
        dir.canonicalize().unwrap()
    );
    assert!(spec
        .env
        .contains(&("ESCOUADE_PORT_BASE".to_string(), pa.to_string())));
    assert!(spec
        .env
        .contains(&("ESCOUADE_PORT_END".to_string(), (pa + 9).to_string())));
    // Asked again, it keeps its block.
    h.core.agent_prepare_launch(&a.id).await.unwrap();
    assert_eq!(h.agent(&a.id).port_base, Some(pa));
    // An agent without a worktree has no test launch.
    h.core
        .update_project(Project {
            worktree_per_agent: false,
            ..h.core.project(&p.id).unwrap()
        })
        .unwrap();
    let plain = h.core.create_agent(&p.id, None).await.unwrap().meta;
    assert!(h.core.agent_prepare_launch(&plain.id).await.is_err());
    assert_eq!(h.agent(&plain.id).port_base, None);
}

#[tokio::test]
async fn a_block_reserved_for_a_launch_stays_with_the_agent_when_the_request_cannot_go() {
    let h = harness("tk-prepare-unsent");
    let (p, _) = h.project(true).await;
    // No Claude Code: the agent's process never starts, its message never goes.
    h.lose_claude();
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    assert!(h.core.agent_prepare_launch(&a.id).await.is_err());
    let base = h.agent(&a.id).port_base.unwrap();
    // The window knows it all the same, and asking again gives the same block.
    assert!(h.events.lock().iter().any(|e| {
        e["type"] == "agent" && e["agent"]["id"] == a.id.as_str() && e["agent"]["portBase"] == base
    }));
    assert!(h.core.agent_prepare_launch(&a.id).await.is_err());
    assert_eq!(h.agent(&a.id).port_base, Some(base));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn launch_preparations_at_once_get_blocks_of_their_own() {
    let h = harness("tk-prepare-race");
    let (p, _) = h.project(true).await;
    let mut ids = Vec::new();
    for _ in 0..4 {
        ids.push(h.core.create_agent(&p.id, None).await.unwrap().meta.id);
    }
    // All set off at the same instant, each on a thread of its own.
    let start = Arc::new(tokio::sync::Barrier::new(ids.len()));
    let runs: Vec<_> = ids
        .iter()
        .map(|id| {
            let (core, id, start) = (h.core.clone(), id.clone(), start.clone());
            tokio::spawn(async move {
                start.wait().await;
                core.agent_prepare_launch(&id).await
            })
        })
        .collect();
    for r in runs {
        r.await.unwrap().unwrap();
    }
    let mut bases: Vec<u16> = ids
        .iter()
        .map(|id| h.agent(id).port_base.unwrap())
        .collect();
    bases.sort();
    bases.dedup();
    assert_eq!(bases.len(), ids.len(), "{bases:?}");
}

#[tokio::test]
async fn a_launch_preparation_never_takes_a_block_reserved_for_an_agent_being_made() {
    let h = harness("tk-prepare-reserved");
    let (p, _) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    // A ticket's start holds this block while its agent is made.
    let held = h.core.reserve_ports().unwrap();
    h.core.agent_prepare_launch(&a.id).await.unwrap();
    let base = h.agent(&a.id).port_base.unwrap();
    assert_ne!(base, held);
    h.core.unreserve_ports(Some(held));
    // The agent holds its block: no reservation of it is left behind (it would stay taken once
    // the agent lets it go).
    assert_eq!(*h.core.ports_reserved.lock(), Vec::<u16>::new());
}

/// A terminal of `agent` that lasts (node waiting for a minute), as a test launch starts one;
/// false when no shell can run it.
fn test_terminal(h: &Harness, agent: &AgentMeta, term_id: &str) -> bool {
    let settings = h.core.settings.read().clone();
    let Some(shell) = crate::pty::detect_shells(&settings).into_iter().next() else {
        return false;
    };
    let info = crate::pty::TermInfo {
        id: term_id.into(),
        project_id: agent.project_id.clone(),
        name: "web".into(),
        shell: shell.id.clone(),
    };
    h.core
        .pty
        .spawn_command(
            info,
            &shell,
            "",
            &agent.cwd,
            (80, 24),
            vec![],
            1,
            "node -e \"setTimeout(() => {}, 60000)\"",
            |_| {},
            |_| {},
        )
        .unwrap();
    true
}

/// A test launch of `agent` that lasts, tracked as its own; false when no shell can run it.
fn test_run(h: &Harness, agent: &AgentMeta, term_id: &str) -> bool {
    if !test_terminal(h, agent, term_id) {
        return false;
    }
    h.core.track_test_run(&agent.id, term_id).unwrap();
    true
}

#[tokio::test]
async fn an_agents_test_launches_stop_when_it_is_archived() {
    let h = harness("tk-launch-stop");
    let (p, _) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    if !test_run(&h, &a, "tk-run-1") {
        return;
    }
    assert!(term_running(&h, "tk-run-1"));
    h.core.archive_agent(&a.id, true).await.unwrap();
    assert!(!term_running(&h, "tk-run-1"));
}

#[tokio::test]
async fn no_test_launch_starts_while_its_ticket_is_being_validated() {
    let h = harness("tk-launch-validating");
    let (p, _) = h.project(false).await;
    let go = h.dir.join("go");
    // Left as it is, its tests held until `go`: its agent stays, and the validation waits.
    h.set_board(&p.id, |s| {
        s.action = "keep".into();
        s.tests_first = true;
        s.test_command = gated_tests(&go, 0);
    });
    let (t, _) = reviewed(&h, &p.id, "Page [ok] [recette]").await;
    let agent = h.agent_of(&t.id);
    h.wait("recipe", |h| h.agent(&agent.id).recipe.is_some())
        .await;
    assert!(h.core.test_run_spec(&agent.id, "run", 0).is_ok());
    let meanwhile = async {
        h.wait_ticket(&t.id, "tests running", |t| {
            t.step.as_deref() == Some("Tests…")
        })
        .await;
        // Its servers would take the ports its tests use.
        let e = h
            .core
            .test_run_spec(&agent.id, "run", 0)
            .unwrap_err()
            .to_string();
        assert_eq!(
            e,
            "Validation en cours : le lancement de test attendra sa fin."
        );
        // One whose step was read just before is stopped as soon as it is tracked.
        if test_terminal(&h, &agent, "tk-run-during") {
            let e = h
                .core
                .track_test_run(&agent.id, "tk-run-during")
                .unwrap_err()
                .to_string();
            assert_eq!(
                e,
                "Validation en cours : le lancement de test attendra sa fin."
            );
            assert!(!term_running(&h, "tk-run-during"));
            assert!(!tracks_runs_of(&h, &agent.id));
        }
        std::fs::write(&go, "").unwrap();
    };
    let (approved, ()) = tokio::join!(h.core.ticket_approve(&t.id), meanwhile);
    approved.unwrap();
    assert_eq!(h.ticket(&t.id).column, Column::Done);
    // Validated and left as it is: its agent runs its steps again.
    assert!(h.core.test_run_spec(&agent.id, "run", 0).is_ok());
}

#[tokio::test]
async fn an_agent_without_a_worktree_runs_no_step_of_a_recipe() {
    let h = harness("tk-launch-no-worktree");
    let (p, _) = h.project(false).await;
    let plain = h.core.create_agent(&p.id, None).await.unwrap().meta;
    // A recipe it gave all the same, in a block of its answer.
    h.core.agent(&plain.id).unwrap().lock().meta.recipe = Some(TestRecipe {
        processes: vec![RecipeProcess {
            name: "web".into(),
            command: "node serveur.js".into(),
            ..Default::default()
        }],
        ..Default::default()
    });
    let e = h
        .core
        .test_run_spec(&plain.id, "run", 0)
        .unwrap_err()
        .to_string();
    assert_eq!(e, "Seul un agent à worktree a un lancement de test.");
}

fn term_running(h: &Harness, term_id: &str) -> bool {
    h.core.pty.list().iter().any(|t| t.id == term_id)
}

fn tracks_runs_of(h: &Harness, agent_id: &str) -> bool {
    h.core.test_runs.lock().contains_key(agent_id)
}

#[tokio::test]
async fn an_agents_test_launches_stop_when_it_is_deleted() {
    let h = harness("tk-launch-delete");
    let (p, _) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    if !test_run(&h, &a, "tk-run-del") {
        return;
    }
    assert!(term_running(&h, "tk-run-del"));
    h.core.delete_agent(&a.id, false).await.unwrap();
    assert!(!term_running(&h, "tk-run-del"));
    assert!(!tracks_runs_of(&h, &a.id));
}

#[tokio::test]
async fn a_tickets_test_launches_stop_when_it_is_validated() {
    let h = harness("tk-launch-validate");
    let (p, _) = h.project(false).await;
    // Left as it is: its agent stays, so only the validation can stop its launches.
    h.set_board(&p.id, |s| s.action = "keep".into());
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    let agent = h.agent_of(&t.id);
    if !test_run(&h, &agent, "tk-run-val") {
        return;
    }
    h.core.ticket_approve(&t.id).await.unwrap();
    assert_eq!(h.ticket(&t.id).column, Column::Done);
    assert!(!h.agent(&agent.id).archived);
    assert!(!term_running(&h, "tk-run-val"));
    assert!(!tracks_runs_of(&h, &agent.id));
}

#[tokio::test]
async fn closing_a_project_forgets_its_agents_test_launches() {
    let h = harness("tk-launch-close");
    let (p, _) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    if !test_run(&h, &a, "tk-run-close") {
        return;
    }
    h.core.remove_project(&p.id).unwrap();
    assert!(!term_running(&h, "tk-run-close"));
    assert!(!tracks_runs_of(&h, &a.id));
}

#[tokio::test]
async fn an_archived_or_deleted_agent_has_no_test_launch() {
    let h = harness("tk-launch-archived");
    let (p, _) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    h.core.agent_prepare_launch(&a.id).await.unwrap();
    h.wait("recipe", |h| h.agent(&a.id).recipe.is_some()).await;
    h.wait("turn over", |h| !h.agent(&a.id).status.is_active())
        .await;
    h.core.archive_agent(&a.id, true).await.unwrap();
    // Its block went with the archive; it gets none back, and none of its steps runs.
    assert!(h.core.agent_prepare_launch(&a.id).await.is_err());
    assert_eq!(h.agent(&a.id).port_base, None);
    assert!(h.core.test_run_spec(&a.id, "run", 0).is_err());
    // A launch started as it was archived is stopped as soon as it is tracked.
    assert!(h.core.track_test_run(&a.id, "tk-late-1").is_err());
    assert!(!tracks_runs_of(&h, &a.id));
    // Likewise once it is deleted.
    let b = h.core.create_agent(&p.id, None).await.unwrap().meta;
    h.core.delete_agent(&b.id, false).await.unwrap();
    assert!(h.core.track_test_run(&b.id, "tk-late-2").is_err());
    assert!(!tracks_runs_of(&h, &b.id));
}

// ---------- worktree setup and teardown, isola ----------

#[tokio::test]
async fn a_tickets_first_message_waits_for_its_worktree_setup_and_tells_its_failure() {
    let h = harness("tk-setup");
    let (p, _) = h.project(false).await;
    h.set_worktree_steps(
        &p.id,
        vec![
            wt_step(
                r#"node -e "require('fs').writeFileSync('prepared.txt', process.env.ESCOUADE_PORT_BASE || '')""#,
                "",
            ),
            wt_step(
                r#"node -e "setTimeout(() => { console.log('dépendances absentes'); process.exit(1) }, 1000)""#,
                "",
            ),
        ],
        vec![],
    );
    let t = h
        .core
        .ticket_create(&p.id, draft("Fichier [ok]", &[], 5))
        .await
        .unwrap();
    h.wait_ticket(&t.id, "to test", |t| t.column == Column::Review)
        .await;
    let wt = h.worktree_of(&t.id);
    // Set up with its reserved ports.
    let base = std::fs::read_to_string(wt.join("prepared.txt")).unwrap();
    assert!(base.parse::<u16>().is_ok_and(|b| b >= 4100), "{base}");
    // Its first message went after the setup, and says what failed.
    let first = h.stdin_messages(&wt)[0]["message"]["content"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(first.starts_with("Ticket DEM-1 : Fichier [ok]"), "{first}");
    assert!(
        first.contains("La préparation du worktree a échoué sur `node -e")
            && first.contains("dépendances absentes")
            && first
                .ends_with("Fais le nécessaire pour pouvoir travailler et tester, puis continue."),
        "{first}"
    );
}

#[tokio::test]
async fn a_tickets_agent_archived_or_deleted_during_its_setup_is_never_sent_the_ticket() {
    for how in ["archive", "delete"] {
        let h = harness(&format!("tk-setup-gone-{how}"));
        let (p, _) = h.project(false).await;
        h.set_worktree_steps(
            &p.id,
            vec![wt_step(r#"node -e "setTimeout(() => {}, 2500)""#, "")],
            vec![],
        );
        let t = h
            .core
            .ticket_create(&p.id, draft("Fichier [ok]", &[], 5))
            .await
            .unwrap();
        h.wait("its agent setting its worktree up", |h| {
            h.ticket(&t.id)
                .agent_id
                .is_some_and(|a| h.view(&a).setup.is_some())
        })
        .await;
        let first = h.ticket(&t.id).agent_id.unwrap();
        let wt = h.worktree_of(&t.id);
        match how {
            "archive" => h.core.archive_agent(&first, true).await.unwrap(),
            _ => {
                h.core.delete_agent(&first, false).await.unwrap();
            }
        }
        // Back to do, then started again with another agent, which it is the ticket of.
        h.wait_ticket(&t.id, "to test with another agent", |t| {
            t.column == Column::Review && t.agent_id.as_deref() != Some(first.as_str())
        })
        .await;
        let t = h.ticket(&t.id);
        assert_eq!(t.blocked, None, "{how}");
        // The first agent never took the ticket, and never will.
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(h.stdin_messages(&wt).is_empty(), "{how}");
        if how == "archive" {
            assert_eq!(h.view(&first).setup, None);
            assert!(!h.alive(&first));
        }
        assert!(
            h.alerts().iter().all(|a| !a.contains("bloqué")),
            "{how}: {:?}",
            h.alerts()
        );
    }
}

#[tokio::test]
async fn a_merged_tickets_worktree_is_torn_down_before_it_goes() {
    let h = harness("tk-teardown");
    let (p, r) = h.project(false).await;
    h.set_worktree_steps(
        &p.id,
        vec![],
        vec![wt_step(
            r#"node -e "const fs = require('fs'); fs.writeFileSync(require('path').join(process.env.ESCOUADE_PROJECT_DIR, '..', 'down.txt'), process.env.ESCOUADE_BRANCH + ' ' + fs.existsSync('dem-1.txt'))""#,
            "",
        )],
    );
    let (t, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    assert_eq!(h.ticket(&t.id).column, Column::Done);
    assert_eq!(
        std::fs::read_to_string(h.dir.join("down.txt")).unwrap(),
        "ticket/dem-1 true"
    );
    assert!(!wt.exists());
    // Kept (no cleanup), no teardown.
    std::fs::remove_file(h.dir.join("down.txt")).unwrap();
    h.set_board(&p.id, |s| s.cleanup = false);
    let (u, wt) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&u.id).await.unwrap();
    assert_eq!(h.ticket(&u.id).column, Column::Done);
    assert!(wt.is_dir() && !h.dir.join("down.txt").exists());
    let _ = r;
}

#[tokio::test]
async fn an_isola_projects_ticket_reserves_no_port_and_is_told_of_isola() {
    crate::isola::tests::use_fake();
    let h = harness("tk-isola");
    let (p, r) = h.project(false).await;
    std::fs::write(r.join(".isola.toml"), "setup = \"npm ci\"\n").unwrap();
    git(&r, &["add", "-A"]);
    git(&r, &["commit", "-qm", "isola"]);
    let t = h
        .core
        .ticket_create(&p.id, draft("Fichier [ok]", &[], 5))
        .await
        .unwrap();
    h.wait_ticket(&t.id, "to test", |t| t.column == Column::Review)
        .await;
    let a = h.agent_of(&t.id);
    assert_eq!(a.port_base, None);
    let argv = h.launches(&h.worktree_of(&t.id)).pop().unwrap();
    let i = argv
        .iter()
        .position(|x| x == "--append-system-prompt")
        .unwrap();
    assert!(
        argv[i + 1].contains("isola up") && !argv[i + 1].contains("Ports réservés"),
        "{}",
        argv[i + 1]
    );
    // Its launch is isola's: nothing to prepare.
    assert!(h.core.agent_prepare_launch(&a.id).await.is_err());
}
