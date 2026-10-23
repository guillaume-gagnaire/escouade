//! A turn stopped by a sign-in Claude Code could not refresh (ESC-162), run by the core on the
//! fake CLI: its process starts again and goes on, or the agent waits for a new sign-in. The
//! sign-ins are the tests' own, in their folders (never the user's).

use crate::core_tests::{agent_on, config_dirs, harness, second_account, Harness};
use crate::model::*;
use crate::tickets_tests::draft;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const LOST: &str = "Failed to authenticate: OAuth session expired and could not be refreshed";
const REFRESHED: &str = "Session du compte rafraîchie : l’agent reprend.";

/// A sign-in in date, `tag` telling one from another (`usage::Stored::stamp`).
fn signed_in(account: &Account, tag: &str) {
    let creds = json!({ "claudeAiOauth": {
        "accessToken": format!("tok-{tag}"),
        "refreshToken": format!("ref-{tag}"),
        "expiresAt": 4_102_444_800_000i64,
    }});
    std::fs::write(
        Path::new(&account.config_dir).join(".credentials.json"),
        creds.to_string(),
    )
    .unwrap();
}

/// The fake CLI of `account` fails its turns as a sign-in not refreshed: `turns` of them, every
/// one when empty (`fake-oauth-lost`).
fn lost_for(account: &Account, turns: &str) {
    std::fs::write(
        Path::new(&account.config_dir).join("fake-oauth-lost"),
        turns,
    )
    .unwrap();
}

/// Pro, a Claude Code account with a folder of its own, first in the order.
fn pro(h: &Harness) -> Account {
    let pro = second_account(h);
    h.core
        .reorder_claude_accounts(&["pro".to_string(), "principal".to_string()])
        .unwrap();
    std::fs::create_dir_all(&pro.config_dir).unwrap();
    pro
}

fn notices(h: &Harness, id: &str) -> Vec<(String, String)> {
    h.items(id)
        .iter()
        .filter(|i| i["kind"] == "notice")
        .map(|i| {
            (
                i["level"].as_str().unwrap_or_default().to_string(),
                i["text"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect()
}

fn turns(h: &Harness, id: &str) -> Vec<Value> {
    h.items(id)
        .into_iter()
        .filter(|i| i["kind"] == "turn")
        .collect()
}

fn cwd(h: &Harness, id: &str) -> PathBuf {
    PathBuf::from(h.agent(id).cwd)
}

/// Whether the conversation has an answer of the fake CLI to "continue".
fn went_on(h: &Harness, id: &str) -> bool {
    h.items(id).iter().any(|i| {
        i["kind"] == "text"
            && i["text"]
                .as_str()
                .is_some_and(|t| t.contains("tu as dit : continue"))
    })
}

#[tokio::test]
async fn a_good_sign_in_on_disk_starts_the_process_again_and_the_agent_goes_on() {
    let h = harness("signin-refreshed");
    let (p, _) = h.project(true).await;
    let pro = pro(&h);
    signed_in(&pro, "a");
    lost_for(&pro, "1");
    let id = agent_on(&h, &p, "pro").await.id;
    h.core
        .send_message(&id, "Bonjour".into(), vec![])
        .await
        .unwrap();
    h.wait("gone on", |h| {
        went_on(h, &id) && !h.agent(&id).status.is_active()
    })
    .await;
    // Its process started again on its session, on the same account.
    let dirs = config_dirs(&h, &cwd(&h, &id));
    assert_eq!(dirs, [json!(pro.config_dir), json!(pro.config_dir)]);
    let session = h.agent(&id).session_id.unwrap();
    let resumed = &h.launch_log(&cwd(&h, &id))[1]["argv"];
    assert!(resumed
        .as_array()
        .unwrap()
        .contains(&json!(format!("--resume={session}"))));
    // Said in one neutral line: no error in red, no notification of an error.
    assert_eq!(
        notices(&h, &id),
        [("info".to_string(), REFRESHED.to_string())]
    );
    let t = turns(&h, &id);
    assert_eq!(
        (t[0]["signIn"].clone(), t[0]["error"].clone()),
        (json!(true), json!(LOST))
    );
    assert_eq!(t[1]["isError"], json!(false));
    assert!(
        !h.alerts().iter().any(|a| a.contains("OAuth")),
        "{:?}",
        h.alerts()
    );
    let m = h.agent(&id);
    assert_eq!((m.status, m.sign_in), (AgentStatus::Done, None));
}

#[tokio::test]
async fn without_a_sign_in_the_agent_waits_for_one_then_goes_on_by_itself() {
    let h = harness("signin-missing");
    let (p, _) = h.project(true).await;
    let pro = pro(&h);
    // Emptied, as Claude Code leaves it when its refresh token is refused.
    std::fs::write(
        Path::new(&pro.config_dir).join(".credentials.json"),
        r#"{"claudeAiOauth":{"accessToken":"","refreshToken":"","expiresAt":0}}"#,
    )
    .unwrap();
    lost_for(&pro, "1");
    let id = agent_on(&h, &p, "pro").await.id;
    h.core
        .send_message(&id, "Bonjour".into(), vec![])
        .await
        .unwrap();
    h.wait("waits for a sign-in", |h| h.agent(&id).sign_in.is_some())
        .await;
    assert_eq!(
        h.agent(&id).sign_in,
        Some(SignInWait {
            account: "pro".into(),
            stamp: None
        })
    );
    // Not started again: nothing would come of it.
    assert_eq!(config_dirs(&h, &cwd(&h, &id)).len(), 1);
    let said = "Le compte Pro doit se reconnecter : l’agent reprendra seul après.";
    h.wait("told", |h| h.alerts().iter().any(|a| a.ends_with(said)))
        .await;
    // Still not signed in: it waits.
    h.core.sign_in_due().await;
    assert!(h.agent(&id).sign_in.is_some());
    assert!(!went_on(&h, &id));
    // Signed in again: it goes on by itself, on its session.
    signed_in(&pro, "b");
    h.core.sign_in_due().await;
    h.wait("gone on", |h| {
        went_on(h, &id) && !h.agent(&id).status.is_active()
    })
    .await;
    assert_eq!(h.agent(&id).sign_in, None);
    assert_eq!(
        notices(&h, &id),
        [(
            "info".to_string(),
            "Compte Pro reconnecté : l’agent reprend.".to_string()
        )]
    );
    assert_eq!(config_dirs(&h, &cwd(&h, &id)).len(), 2);
}

#[tokio::test]
async fn two_restarts_at_most_then_it_waits_for_another_sign_in() {
    let h = harness("signin-retries");
    let (p, _) = h.project(true).await;
    let pro = pro(&h);
    signed_in(&pro, "a");
    // Every turn fails so, whatever the process.
    lost_for(&pro, "");
    let id = agent_on(&h, &p, "pro").await.id;
    h.core
        .send_message(&id, "Bonjour".into(), vec![])
        .await
        .unwrap();
    h.wait("waits for a sign-in", |h| h.agent(&id).sign_in.is_some())
        .await;
    // The first process and two restarts.
    assert_eq!(config_dirs(&h, &cwd(&h, &id)).len(), 3);
    assert_eq!(
        notices(&h, &id),
        vec![("info".to_string(), REFRESHED.to_string()); 2]
    );
    let wait = h.agent(&id).sign_in.unwrap();
    assert!(wait.stamp.is_some());
    // The same sign-in on disk: it waits still.
    std::fs::remove_file(Path::new(&pro.config_dir).join("fake-oauth-lost")).unwrap();
    h.core.sign_in_due().await;
    assert_eq!(h.agent(&id).sign_in, Some(wait));
    // Another one: it goes on.
    signed_in(&pro, "b");
    h.core.sign_in_due().await;
    h.wait("gone on", |h| {
        went_on(h, &id) && !h.agent(&id).status.is_active()
    })
    .await;
    assert_eq!(h.agent(&id).sign_in, None);
}

#[tokio::test]
async fn a_message_of_the_user_takes_over_from_the_wait() {
    let h = harness("signin-taken-over");
    let (p, _) = h.project(true).await;
    let pro = pro(&h);
    lost_for(&pro, "1");
    let id = agent_on(&h, &p, "pro").await.id;
    h.core
        .send_message(&id, "Bonjour".into(), vec![])
        .await
        .unwrap();
    h.wait("waits for a sign-in", |h| h.agent(&id).sign_in.is_some())
        .await;
    h.core
        .send_message(&id, "Encore".into(), vec![])
        .await
        .unwrap();
    assert_eq!(h.agent(&id).sign_in, None);
}

#[tokio::test]
async fn a_tickets_agent_goes_on_and_its_ticket_is_never_blocked() {
    let h = harness("signin-ticket");
    let (p, _) = h.project(false).await;
    let pro = pro(&h);
    signed_in(&pro, "a");
    lost_for(&pro, "1");
    let t = h
        .core
        .ticket_create(&p.id, draft("Un [ok]", &[], 5))
        .await
        .unwrap();
    h.wait_ticket(&t.id, "to test", |t| t.column == Column::Review)
        .await;
    let ticket = h.ticket(&t.id);
    assert_eq!(ticket.blocked, None);
    let agent = h.agent_of(&t.id);
    assert_eq!(agent.account, "pro");
    assert!(notices(&h, &agent.id).contains(&("info".to_string(), REFRESHED.to_string())));
}

/// What each launch of the fake CLI of `account` found of its sign-in as it started
/// (`fake-refresh.jsonl`): whether it was out of date.
fn refresh_log(account: &Account) -> Vec<bool> {
    refresh_lines(account)
        .iter()
        .map(|l| l["expired"] == json!(true))
        .collect()
}

fn refresh_lines(account: &Account) -> Vec<Value> {
    std::fs::read_to_string(Path::new(&account.config_dir).join("fake-refresh.jsonl"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect()
}

#[tokio::test]
async fn the_launches_of_an_account_out_of_date_wait_for_the_first_ones_refresh() {
    let h = harness("signin-serial");
    let (p, _) = h.project(true).await;
    let pro = pro(&h);
    // Out of date: the first process refreshes it, 8 s after it started (longer than
    // the launches of the three agents take, one after the other: a second or two each).
    let creds = json!({ "claudeAiOauth": {
        "accessToken": "tok-old", "refreshToken": "ref-old", "expiresAt": 1000,
    }});
    std::fs::write(
        Path::new(&pro.config_dir).join(".credentials.json"),
        creds.to_string(),
    )
    .unwrap();
    std::fs::write(Path::new(&pro.config_dir).join("fake-refresh"), "8000").unwrap();
    // Three agents started at once.
    let (a, b, c) = tokio::join!(
        agent_on(&h, &p, "pro"),
        agent_on(&h, &p, "pro"),
        agent_on(&h, &p, "pro")
    );
    for id in [&a.id, &b.id, &c.id] {
        h.core.warm(id);
    }
    // Each started (the fake CLI tells it first thing).
    let agents = |l: &[Value]| {
        l.iter()
            .filter(|e| {
                let argv = e["argv"].as_array().cloned().unwrap_or_default();
                !argv.contains(&json!("-p")) && !argv.contains(&json!("--no-session-persistence"))
            })
            .count()
    };
    h.wait("three processes", |_| agents(&refresh_lines(&pro)) >= 3)
        .await;
    // One alone found it out of date; the others started once it was refreshed.
    let log = refresh_log(&pro);
    assert!(log.len() >= 3, "{log:?}");
    assert_eq!(log.iter().filter(|e| **e).count(), 1, "{log:?}");
    assert!(log[0], "{log:?}");
}
