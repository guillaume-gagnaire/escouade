//! The deletions agents ask of the user (`deleteask`, ESC-511): kept on the agent asked about
//! across a restart, gone with it or with its maker, bounded, a refusal told to the maker with its
//! next message, and what a deletion would lose read by git.

use crate::core::Core;
use crate::core_tests::{git, harness, Harness};
use crate::deleteask::*;
use crate::i18n::Lang::{En, Fr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tauri::test::{mock_app, MockRuntime};

/// Two agents in worktrees of a project, the second made by the first (`create_agent`'s mark).
async fn maker_and_made(h: &Harness) -> (String, String, PathBuf) {
    let (p, _) = h.project(true).await;
    let maker = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let made = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.agent(&made).unwrap().lock().meta.created_by = Some(maker.clone());
    let wt = PathBuf::from(h.agent(&maker).worktree.expect("a worktree").path);
    (maker, made, wt)
}

/// Sends `text` and waits for the turn to end; what the agent's process read on its input.
async fn say(h: &Harness, id: &str, cwd: &Path, text: &str) -> String {
    let turns = |h: &Harness| h.items(id).iter().filter(|i| i["kind"] == "turn").count();
    let before = turns(h);
    h.core
        .send_message(id, text.to_string(), Vec::new())
        .await
        .unwrap();
    h.wait("the turn's end", |h| {
        turns(h) > before && !h.agent(id).status.is_active()
    })
    .await;
    let last = h.stdin_messages(cwd).pop().expect("a message sent");
    last["message"]["content"].as_str().unwrap().to_string()
}

fn restarted(h: &Harness) -> (Arc<Core<MockRuntime>>, tauri::App<MockRuntime>) {
    let app = mock_app();
    let (core, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    (core, app)
}

#[test]
fn the_maker_reads_which_deletions_were_refused() {
    assert_eq!(
        refusal_line(Fr, &["aide".into(), "tests".into()]),
        "Note d'Escouade : l'utilisateur a refusé de supprimer « aide », « tests », que tu lui avais demandé de supprimer (delete_agent) : ils restent."
    );
    assert_eq!(
        refusal_line(En, &["aide".into()]),
        "Escouade's note: the user refused to delete “aide”, which you had asked them to delete (delete_agent): they stay."
    );
}

#[tokio::test]
async fn an_ask_is_kept_across_a_restart_and_goes_with_its_maker() {
    let h = harness("delete-ask-kept");
    let (maker, made, _) = maker_and_made(&h).await;
    h.core.ask_delete(&maker, &made, Some(false)).unwrap();
    let ask = h.agent(&made).delete_ask.expect("asked");
    assert_eq!(
        (ask.by.as_str(), ask.worktree),
        (maker.as_str(), Some(false))
    );
    // Asked again: the same ask, since the same time, the new choice for its worktree.
    h.core.ask_delete(&maker, &made, None).unwrap();
    let again = h.agent(&made).delete_ask.unwrap();
    assert_eq!((again.at, again.worktree), (ask.at, None));
    // The window shows it on the card: it is in the agent's view.
    let view = serde_json::to_value(h.core.agent(&made).unwrap().lock().view()).unwrap();
    assert_eq!(view["deleteAsk"]["by"], maker.as_str());

    // Saved, and read back by the app started again.
    h.core.save_now();
    let (core, _app) = restarted(&h);
    let kept = core.agent(&made).unwrap().lock().meta.delete_ask.clone();
    assert_eq!(kept.map(|a| a.by), Some(maker.clone()));

    // Its maker deleted: the ask goes.
    h.core.delete_agent(&maker, true).await.unwrap();
    assert_eq!(h.agent(&made).delete_ask, None);
}

#[tokio::test]
async fn an_ask_whose_maker_is_gone_does_not_come_back_at_the_start() {
    let h = harness("delete-ask-pruned");
    let (maker, made, _) = maker_and_made(&h).await;
    h.core.ask_delete(&maker, &made, None).unwrap();
    // Its maker no longer among the agents saved (its project closed, say).
    h.core.agents.write().remove(&maker);
    h.core.save_now();
    let (core, _app) = restarted(&h);
    assert_eq!(core.agent(&made).unwrap().lock().meta.delete_ask, None);
}

#[tokio::test]
async fn an_ask_goes_with_the_agent_asked_about() {
    let h = harness("delete-ask-target-gone");
    let (maker, made, _) = maker_and_made(&h).await;
    h.core.ask_delete(&maker, &made, None).unwrap();
    h.core.delete_agent(&made, true).await.unwrap();
    assert!(h.core.agent(&made).is_err());
    // Nothing left that counts against its maker's bound.
    let open = h
        .core
        .agents
        .read()
        .values()
        .filter(|a| a.lock().meta.delete_ask.is_some())
        .count();
    assert_eq!(open, 0);
}

#[tokio::test]
async fn an_agent_has_at_most_so_many_deletions_waiting() {
    let h = harness("delete-ask-bound");
    let (p, _) = h.project(false).await;
    let maker = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let mut made = Vec::new();
    for _ in 0..=MAX_ASKS {
        made.push(h.core.create_agent(&p.id, None).await.unwrap().meta.id);
    }
    for id in &made[..MAX_ASKS] {
        h.core.ask_delete(&maker, id, None).unwrap();
    }
    // One more: refused, with why.
    let e = h
        .core
        .ask_delete(&maker, &made[MAX_ASKS], None)
        .unwrap_err();
    assert!(e.to_string().contains("5 demandes"), "{e}");
    assert_eq!(h.agent(&made[MAX_ASKS]).delete_ask, None);
    // One asked again is not one more.
    h.core.ask_delete(&maker, &made[0], Some(true)).unwrap();
    // One answered frees a place.
    h.core.refuse_delete(&made[0]).unwrap();
    h.core.ask_delete(&maker, &made[MAX_ASKS], None).unwrap();
}

#[tokio::test]
async fn a_refusal_is_told_to_the_maker_with_its_next_message_once() {
    let h = harness("delete-ask-refused");
    let (maker, made, wt) = maker_and_made(&h).await;
    let name = h.agent(&made).name;
    h.core.ask_delete(&maker, &made, None).unwrap();
    h.core.refuse_delete(&made).unwrap();
    // The agent stays, its ask gone; its maker keeps the refusal to tell.
    assert_eq!(h.agent(&made).delete_ask, None);
    assert_eq!(h.agent(&maker).delete_refused, std::slice::from_ref(&name));
    // Nothing to refuse: nothing done.
    h.core.refuse_delete(&made).unwrap();
    assert_eq!(h.agent(&maker).delete_refused.len(), 1);

    let got = say(&h, &maker, &wt, "continue").await;
    assert!(
        got.starts_with(&format!(
            "Note d'Escouade : l'utilisateur a refusé de supprimer « {name} »"
        )),
        "{got}"
    );
    assert!(got.ends_with("\n\ncontinue"), "{got}");
    // The conversation shows the message alone.
    assert!(h
        .items(&maker)
        .iter()
        .any(|i| i["kind"] == "user" && i["text"] == "continue"));
    assert!(h.agent(&maker).delete_refused.is_empty());
    // Told once.
    assert_eq!(say(&h, &maker, &wt, "encore").await, "encore");
}

#[tokio::test]
async fn what_a_deletion_would_lose_is_read_from_the_worktree() {
    let h = harness("delete-ask-loss");
    let (_, made, _) = maker_and_made(&h).await;
    let wt = PathBuf::from(h.agent(&made).worktree.unwrap().path);
    let base = h.agent(&made).worktree.unwrap().base_branch;
    // Nothing of its own yet.
    let none = h.core.delete_loss(&made, LOSS_TIMEOUT).await.unwrap();
    assert_eq!(
        (none.known, none.dirty, none.unmerged),
        (true, 0, 0),
        "{none:?}"
    );

    // A commit of its own, then two files not committed.
    std::fs::write(wt.join("fait.txt"), "x\n").unwrap();
    git(&wt, &["add", "-A"]);
    git(&wt, &["commit", "-qm", "feat: fait"]);
    std::fs::write(wt.join("a.txt"), "a\n").unwrap();
    std::fs::write(wt.join("b.txt"), "b\n").unwrap();
    let loss = h.core.delete_loss(&made, LOSS_TIMEOUT).await.unwrap();
    assert!(loss.known);
    assert_eq!((loss.dirty, loss.unmerged), (2, 1));
    assert_eq!(loss.names, ["a.txt", "b.txt"]);
    assert_eq!(loss.base, base);

    // git too slow: unknown, at once.
    let late = h.core.delete_loss(&made, Duration::ZERO).await.unwrap();
    assert!(!late.known);
}

#[tokio::test]
async fn an_agent_without_a_worktree_of_its_own_loses_no_files() {
    let h = harness("delete-ask-loss-no-worktree");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    // The project's files changed: they are the project's, and stay.
    std::fs::write(r.join("a.txt"), "a\n").unwrap();
    let loss = h.core.delete_loss(&id, LOSS_TIMEOUT).await.unwrap();
    assert_eq!((loss.known, loss.dirty, loss.unmerged), (true, 0, 0));
}
