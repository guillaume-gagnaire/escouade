//! « Changer de compte à » (ESC-148), run by the core on the fake CLI: an agent whose account
//! passed the threshold goes on, at the end of its turn, on another account, with its session.

use crate::core_tests::{agent_on, config_dirs, extra_account, harness, second_account, Harness};
use crate::model::*;
use crate::tickets_tests::draft;
use crate::usage::Reading;
use serde_json::json;
use std::path::{Path, PathBuf};

/// Pro then Équipe, both Claude Code accounts with a folder of their own (Principal switched off:
/// it has none in tests), « Changer de compte à » at 95 %.
fn pro_and_team(h: &Harness) -> (Account, Account) {
    let pro = second_account(h);
    let team = extra_account(h, "equipe", "Équipe");
    let order = ["pro", "equipe", "principal"].map(String::from);
    h.core.reorder_claude_accounts(&order).unwrap();
    h.set_settings(|s| {
        s.switch_at = 95;
        for a in s.accounts.iter_mut().filter(|a| a.id == "principal") {
            a.active = false;
        }
    });
    for a in [&pro, &team] {
        std::fs::create_dir_all(&a.config_dir).unwrap();
    }
    (pro, team)
}

/// The fake CLI of `account` tells its 5-hour window used `pct` percent (`fake-pct`), or its
/// weekly one (`fake-week-pct`), until a little later.
fn uses(account: &Account, file: &str, pct: u32) {
    std::fs::write(Path::new(&account.config_dir).join(file), pct.to_string()).unwrap();
}

/// `account` read as used `pct` percent of its 5-hour window, until in an hour.
fn read_as(h: &Harness, account: &str, pct: f64) {
    let window = RateWindow {
        pct,
        resets_at: Some(now_ms() + 3_600_000),
    };
    h.core
        .record_usage(account, Reading::claude((Some(window), None)));
}

/// What the agent's conversation says of itself (its notices), in order.
fn notices(h: &Harness, id: &str) -> Vec<String> {
    h.items(id)
        .iter()
        .filter(|i| i["kind"] == "notice")
        .filter_map(|i| i["text"].as_str().map(str::to_string))
        .collect()
}

fn cwd(h: &Harness, id: &str) -> PathBuf {
    PathBuf::from(h.agent(id).cwd)
}

/// How many turns of the agent are over.
fn turns(h: &Harness, id: &str) -> usize {
    h.items(id).iter().filter(|i| i["kind"] == "turn").count()
}

/// Sends `text`, and waits for the turn it starts to be over.
async fn turn_over(h: &Harness, id: &str, text: &str) {
    let before = turns(h, id);
    h.core
        .send_message(id, text.to_string(), vec![])
        .await
        .unwrap();
    h.wait("turn over", |h| {
        turns(h, id) > before && !h.agent(id).status.is_active()
    })
    .await;
}

#[tokio::test]
async fn past_the_threshold_an_agent_finishes_its_turn_then_goes_on_on_another_account() {
    let h = harness("switch-at-95");
    let (p, _) = h.project(true).await;
    let (pro, team) = pro_and_team(&h);
    uses(&pro, "fake-pct", 96);
    let id = agent_on(&h, &p, "pro").await.id;
    turn_over(&h, &id, "Bonjour").await;
    // Its turn ran on Pro whole; the reading that came with its end sends it to Équipe, and its
    // process starts again there, on its session.
    h.wait("on Équipe", |h| h.agent(&id).account == "equipe")
        .await;
    h.wait("started again on Équipe", |h| {
        config_dirs(h, &cwd(h, &id)).len() == 2
    })
    .await;
    assert_eq!(
        config_dirs(&h, &cwd(&h, &id)),
        [json!(pro.config_dir), json!(team.config_dir)]
    );
    let session = h.agent(&id).session_id.unwrap();
    let resumed = &h.launch_log(&cwd(&h, &id))[1]["argv"];
    assert!(resumed
        .as_array()
        .unwrap()
        .contains(&json!(format!("--resume={session}"))));
    // Said in its conversation, by a notification, and in the window.
    let said = "L’agent passe sur le compte Équipe : Pro est à 96 % de sa fenêtre de 5 h.";
    assert_eq!(notices(&h, &id), [said]);
    // A separator in the conversation, as a handover's (2.0 design, choice 24).
    let levels: Vec<_> = h
        .items(&id)
        .iter()
        .filter(|i| i["kind"] == "notice")
        .map(|i| i["level"].clone())
        .collect();
    assert_eq!(levels, [json!("switch")]);
    assert!(
        h.alerts().iter().any(|a| a.ends_with(said)),
        "{:?}",
        h.alerts()
    );
    let toast = h
        .events
        .lock()
        .iter()
        .filter(|e| e["type"] == "toast")
        .filter_map(|e| e["text"].as_str().map(str::to_string))
        .find(|t| t.contains("passe sur Équipe : Pro à 96 % de sa fenêtre de 5 h."));
    assert!(toast.is_some());
    // Its next turn runs there, and it stays there.
    turn_over(&h, &id, "Encore").await;
    let m = h.agent(&id);
    assert_eq!((m.account.as_str(), m.moved_from), ("equipe", None));
    assert_eq!(config_dirs(&h, &cwd(&h, &id)).len(), 2);
    assert_eq!(notices(&h, &id).len(), 1);
}

#[tokio::test]
async fn the_weekly_window_past_the_threshold_moves_the_agent_too() {
    let h = harness("switch-at-week");
    let (p, _) = h.project(true).await;
    let (pro, _) = pro_and_team(&h);
    uses(&pro, "fake-week-pct", 97);
    let id = agent_on(&h, &p, "pro").await.id;
    turn_over(&h, &id, "Bonjour").await;
    h.wait("on Équipe", |h| h.agent(&id).account == "equipe")
        .await;
    assert_eq!(
        notices(&h, &id),
        ["L’agent passe sur le compte Équipe : Pro est à 97 % de sa fenêtre hebdomadaire."]
    );
}

#[tokio::test]
async fn at_the_limit_setting_nothing_moves_before_the_limit_as_before() {
    let h = harness("switch-at-limit");
    let (p, _) = h.project(true).await;
    let (pro, _) = pro_and_team(&h);
    h.set_settings(|s| s.switch_at = Settings::default().switch_at);
    uses(&pro, "fake-pct", 99);
    let id = agent_on(&h, &p, "pro").await.id;
    turn_over(&h, &id, "Bonjour").await;
    h.wait_quotas_read().await;
    assert!(!h.core.switch_if_over(&id).await);
    turn_over(&h, &id, "Encore").await;
    assert_eq!(h.agent(&id).account, "pro");
    assert_eq!(config_dirs(&h, &cwd(&h, &id)).len(), 1);
    assert!(notices(&h, &id).is_empty());
}

#[tokio::test]
async fn an_agent_waiting_for_an_answer_moves_only_once_its_turn_is_over() {
    let h = harness("switch-at-question");
    let (p, _) = h.project(true).await;
    let (pro, team) = pro_and_team(&h);
    let id = agent_on(&h, &p, "pro").await.id;
    h.core
        .send_message(&id, "Une question".into(), vec![])
        .await
        .unwrap();
    h.wait("the question", |h| {
        h.agent(&id).status == AgentStatus::Waiting
    })
    .await;
    // Pro passes the threshold while the question waits: its process holds it, nothing moves.
    uses(&pro, "fake-pct", 96);
    read_as(&h, "pro", 96.0);
    assert!(!h.core.switch_if_over(&id).await);
    assert_eq!(h.agent(&id).account, "pro");
    h.core
        .answer_question(
            &id,
            "req_question",
            json!({"Quelle base de données ?": "SQLite"}),
        )
        .unwrap();
    h.wait("on Équipe once the turn is over", |h| {
        h.agent(&id).account == "equipe"
    })
    .await;
    h.wait("started again on Équipe", |h| {
        config_dirs(h, &cwd(h, &id)).len() == 2
    })
    .await;
    assert_eq!(
        config_dirs(&h, &cwd(&h, &id)),
        [json!(pro.config_dir), json!(team.config_dir)]
    );
    // After the end of the turn that had the question.
    let items = h.items(&id);
    let turn = items.iter().position(|i| i["kind"] == "turn").unwrap();
    let notice = items.iter().position(|i| i["kind"] == "notice").unwrap();
    assert!(turn < notice, "{items:?}");
}

#[tokio::test]
async fn an_agent_with_a_task_in_the_background_moves_at_the_first_turn_without_one() {
    let h = harness("switch-at-background");
    let (p, _) = h.project(true).await;
    let (pro, _) = pro_and_team(&h);
    uses(&pro, "fake-pct", 96);
    let id = agent_on(&h, &p, "pro").await.id;
    // Its first turn leaves a command running in its process: no move then. Once the command is
    // over, Claude Code starts a turn by itself, at whose end it moves.
    h.core
        .send_message(&id, "commande de fond 2s".into(), vec![])
        .await
        .unwrap();
    h.wait("on Équipe", |h| h.agent(&id).account == "equipe")
        .await;
    let items = h.items(&id);
    let done = items
        .iter()
        .position(|i| i.to_string().contains("La tâche de fond est finie"))
        .expect("the turn after the command");
    let notice = items.iter().position(|i| i["kind"] == "notice").unwrap();
    assert!(done < notice, "{items:?}");
    assert_eq!(turns(&h, &id), 2);
}

#[tokio::test]
async fn with_no_account_under_the_threshold_the_agent_stays_and_is_told_why_once() {
    let h = harness("switch-at-none");
    let (p, _) = h.project(true).await;
    let (pro, _) = pro_and_team(&h);
    // Pro in extra usage, its window full; Équipe past the threshold too.
    std::fs::write(Path::new(&pro.config_dir).join("fake-overage"), "").unwrap();
    read_as(&h, "equipe", 97.0);
    let id = agent_on(&h, &p, "pro").await.id;
    turn_over(&h, &id, "Bonjour").await;
    let said = "Pro est à 100 % de sa fenêtre de 5 h, mais aucun autre compte Claude Code ne peut prendre l’agent (sous le seuil, connecté, servant ce projet) : il y reste. Ce compte est en usage supplémentaire, facturé à part.";
    h.wait("told why it stays", |h| notices(h, &id) == [said])
        .await;
    turn_over(&h, &id, "Encore").await;
    h.wait_quotas_read().await;
    assert!(!h.core.switch_if_over(&id).await);
    assert_eq!(h.agent(&id).account, "pro");
    assert_eq!(notices(&h, &id), [said], "told once");
    assert_eq!(config_dirs(&h, &cwd(&h, &id)).len(), 1);
}

#[tokio::test]
async fn a_private_project_never_goes_on_its_own_to_an_account_it_shares() {
    let h = harness("switch-at-private");
    let (p, _) = h.project(true).await;
    let (pro, _) = pro_and_team(&h);
    let mut project = h.core.project(&p.id).unwrap();
    project.private = true;
    project.shared_on_limit = false;
    h.core.update_project(project).unwrap();
    uses(&pro, "fake-pct", 96);
    let id = agent_on(&h, &p, "pro").await.id;
    turn_over(&h, &id, "Bonjour").await;
    let said = "Pro est à 96 % de sa fenêtre de 5 h, mais ce projet privé ne passe pas tout seul sur un compte qu’il partage : l’agent y reste.";
    h.wait("told why it stays", |h| notices(h, &id) == [said])
        .await;
    assert_eq!(h.agent(&id).account, "pro");
    // Équipe reserved to it alone: it goes there.
    h.set_settings(|s| {
        for a in s.accounts.iter_mut().filter(|a| a.id == "equipe") {
            a.projects = vec![p.id.clone()];
        }
    });
    turn_over(&h, &id, "Encore").await;
    h.wait("on Équipe", |h| h.agent(&id).account == "equipe")
        .await;
}

#[tokio::test]
async fn an_agent_does_not_go_back_to_the_account_it_left_before_that_one_resets() {
    let h = harness("switch-at-no-back");
    let (p, _) = h.project(true).await;
    let (pro, team) = pro_and_team(&h);
    uses(&pro, "fake-pct", 96);
    let id = agent_on(&h, &p, "pro").await.id;
    turn_over(&h, &id, "Bonjour").await;
    h.wait("on Équipe", |h| h.agent(&id).account == "equipe")
        .await;
    // Équipe passes it too while Pro still has: it stays on Équipe.
    uses(&team, "fake-pct", 97);
    turn_over(&h, &id, "Encore").await;
    h.wait("told it stays", |h| notices(h, &id).len() == 2)
        .await;
    assert_eq!(h.agent(&id).account, "equipe");
    assert!(notices(&h, &id)[1].starts_with("Équipe est à 97 %"));
    // Pro is reset: a choice again, at the next end of a turn.
    std::fs::remove_file(Path::new(&pro.config_dir).join("fake-pct")).unwrap();
    h.wait_quotas_read().await;
    read_as(&h, "pro", 5.0);
    turn_over(&h, &id, "Encore une fois").await;
    h.wait("back on Pro", |h| h.agent(&id).account == "pro")
        .await;
    assert_eq!(
        notices(&h, &id)[2],
        "L’agent passe sur le compte Pro : Équipe est à 97 % de sa fenêtre de 5 h."
    );
}

#[tokio::test]
async fn a_tickets_agent_past_the_threshold_goes_on_on_another_account_after_its_turn() {
    let h = harness("switch-at-ticket");
    let (p, _) = h.project(false).await;
    let (pro, team) = pro_and_team(&h);
    uses(&pro, "fake-pct", 96);
    let t = h
        .core
        .ticket_create(&p.id, draft("Un [ok]", &[], 5))
        .await
        .unwrap();
    h.wait_ticket(&t.id, "to test", |t| t.column == Column::Review)
        .await;
    h.wait("on Équipe", |h| h.agent_of(&t.id).account == "equipe")
        .await;
    let cwd = h.worktree_of(&t.id);
    h.wait("started again on Équipe", |h| {
        config_dirs(h, &cwd).len() == 2
    })
    .await;
    assert_eq!(
        config_dirs(&h, &cwd),
        [json!(pro.config_dir), json!(team.config_dir)]
    );
    assert_eq!(h.ticket(&t.id).blocked, None);
}

#[tokio::test]
async fn new_agents_go_to_another_account_past_the_threshold() {
    let h = harness("switch-at-new");
    let (p, _) = h.project(false).await;
    pro_and_team(&h);
    read_as(&h, "pro", 96.0);
    assert_eq!(
        h.core.account_for_new(&p.id, EngineId::CLAUDE).unwrap(),
        "equipe"
    );
    // At the limit, as before: Pro takes them until « Pause au-delà du quota ».
    h.set_settings(|s| s.switch_at = 100);
    assert_eq!(
        h.core.account_for_new(&p.id, EngineId::CLAUDE).unwrap(),
        "pro"
    );
}

#[tokio::test]
async fn a_ticket_set_on_its_account_stays_there_past_the_threshold_and_is_told_why() {
    // ESC-153: « Compte » set on Pro, its agent never leaves it on its own.
    let h = harness("switch-at-pinned");
    let (p, _) = h.project(false).await;
    let (pro, _) = pro_and_team(&h);
    uses(&pro, "fake-pct", 96);
    let t = h
        .core
        .ticket_create(
            &p.id,
            crate::tickets::TicketDraft {
                account: Some("pro".into()),
                ..draft("Un [ok]", &[], 5)
            },
        )
        .await
        .unwrap();
    h.wait_ticket(&t.id, "to test", |t| t.column == Column::Review)
        .await;
    let id = h.ticket(&t.id).agent_id.unwrap();
    let said = "Pro est à 96 % de sa fenêtre de 5 h, mais le ticket de l’agent est fixé sur ce compte : il y reste.";
    h.wait("told why it stays", |h| notices(h, &id) == [said])
        .await;
    assert!(!h.core.switch_if_over(&id).await);
    assert_eq!(h.agent(&id).account, "pro");
    assert_eq!(config_dirs(&h, &h.worktree_of(&t.id)).len(), 1);
}
