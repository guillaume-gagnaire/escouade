//! A ticket's own target branch (ESC-552): its worktree starts from it, its validation merges into
//! it, the box says it; and a validation whose branch does not descend from its target stops before
//! any merge, its agent told nothing, until its target is chosen.

use crate::core_tests::{commit_change, git, harness, Harness};
use crate::inboxviews::{ViewAct, ViewSource};
use crate::model::*;
use crate::tickets::TicketDraft;
use crate::tickets_tests::{draft, reviewed};
use std::path::Path;

/// `branch` made in `repo` from where it is, with one commit of its own (`file`), the folder
/// back on the branch it was on.
fn branch_with_commit(repo: &Path, branch: &str, file: &str) {
    let back = git(repo, &["branch", "--show-current"]);
    git(repo, &["switch", "-q", "-c", branch]);
    std::fs::write(repo.join(file), format!("{branch}\n")).unwrap();
    git(repo, &["add", file]);
    git(
        repo,
        &[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-qm",
            &format!("{branch} only"),
        ],
    );
    git(repo, &["switch", "-q", &back]);
}

fn targeted(title: &str, target: &str) -> TicketDraft {
    TicketDraft {
        target: Some(target.into()),
        ..draft(title, &[], 5)
    }
}

/// How many messages the agent of the ticket was sent.
fn sent_count(h: &Harness, wt: &Path) -> usize {
    h.stdin_messages(wt).len()
}

#[tokio::test]
async fn a_ticket_of_its_own_target_starts_from_it_and_is_merged_into_it() {
    let h = harness("tk-own-target");
    let (p, r) = h.project(false).await;
    // The Kanban goes to main; the ticket, to « release », which has a file main has not.
    branch_with_commit(&r, "release", "release.txt");
    let t = h
        .core
        .ticket_create(&p.id, targeted("Fichier [ok]", "release"))
        .await
        .unwrap();
    assert_eq!(t.target, "release");
    assert_eq!(h.core.project(&p.id).unwrap().board.target, "main");
    h.wait_ticket(&t.id, "to test", |t| t.column == Column::Review)
        .await;
    let wt = h.worktree_of(&t.id);
    // Its worktree started from its target, which « Intégrer » and its delay go to.
    assert!(wt.join("release.txt").is_file());
    assert_eq!(h.agent_of(&t.id).worktree.unwrap().base_branch, "release");
    let main_before = git(&r, &["rev-parse", "main"]);
    h.core.ticket_approve(&t.id).await.unwrap();
    let t = h.ticket(&t.id);
    assert_eq!(t.column, Column::Done, "{:?}", t.blocked);
    assert_eq!(t.outcome.as_deref(), Some("⤵ Mergé dans release · squash"));
    assert_eq!(git(&r, &["show", "release:dem-1.txt"]), "Boucle 1");
    // The Kanban's target is as it was.
    assert_eq!(git(&r, &["rev-parse", "main"]), main_before);
}

#[tokio::test]
async fn a_ticket_target_is_the_boards_when_named_so_and_refused_when_the_repository_lacks_it() {
    let h = harness("tk-own-target-names");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    // The Kanban's own, named: none of its own.
    let t = h
        .core
        .ticket_create(&p.id, targeted("Rien", "main"))
        .await
        .unwrap();
    assert_eq!(t.target, "");
    let e = h
        .core
        .ticket_create(&p.id, targeted("Rien", "nulle-part"))
        .await
        .unwrap_err();
    assert_eq!(
        format!("{e:#}"),
        "La branche cible « nulle-part » n’existe pas dans le dépôt."
    );
    // A ticket that takes up its own target: refused.
    let e = h
        .core
        .ticket_create(
            &p.id,
            TicketDraft {
                branch: "main".into(),
                ..targeted("Rien", "")
            },
        )
        .await
        .unwrap_err();
    assert!(format!("{e:#}").contains("branche cible"), "{e:#}");
    // Edited to do: the target kept, changed, given back to the Kanban.
    let mut d = targeted("Rien", "release");
    d.target = None;
    let kept = h.core.ticket_update(&t.id, d).unwrap();
    assert_eq!(kept.target, "");
    let moved = h
        .core
        .ticket_update(&t.id, targeted("Rien", "release"))
        .unwrap();
    assert_eq!(moved.target, "release");
    let back = h.core.ticket_update(&t.id, targeted("Rien", "")).unwrap();
    assert_eq!(back.target, "");
}

/// The case of ESC-531: a ticket's branch started from main, its target release (the Kanban's
/// since), main having commits release has not.
async fn from_another_line(
    h: &Harness,
) -> (Project, std::path::PathBuf, Ticket, std::path::PathBuf) {
    let (p, r) = h.project(false).await;
    git(&r, &["branch", "release"]);
    commit_change(&r, "const a = 2;\n", "main only");
    let (t, wt) = reviewed(h, &p.id, "Fichier [ok]").await;
    h.set_board(&p.id, |s| s.target = "release".into());
    (p, r, t, wt)
}

#[tokio::test]
async fn a_branch_of_another_line_stops_its_validation_before_any_merge_and_tells_its_agent_nothing(
) {
    let h = harness("tk-other-line");
    let (_, r, t, wt) = from_another_line(&h).await;
    let release = git(&r, &["rev-parse", "release"]);
    let head = git(&wt, &["rev-parse", "HEAD"]);
    let told = sent_count(&h, &wt);
    h.core.ticket_approve(&t.id).await.unwrap();
    let x = h.ticket(&t.id);
    assert_eq!(
        (x.column, x.step.as_deref(), x.conflict),
        (Column::Review, None, false)
    );
    assert_eq!(x.other_line.as_deref(), Some("main"));
    assert_eq!(
        x.blocked.as_deref(),
        Some("« ticket/dem-1 » ne part pas de sa cible release : elle porte 1 commit de main que release n’a pas. Rien n’est fusionné : choisis la cible de ce ticket.")
    );
    // Nothing merged anywhere: the target as it was, the branch as it was, nothing in the
    // worktree, and its agent not told anything (nor archived).
    assert_eq!(git(&r, &["rev-parse", "release"]), release);
    assert_eq!(git(&wt, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(&wt, &["diff", "--name-only", "--diff-filter=U"]), "");
    assert_eq!(sent_count(&h, &wt), told);
    assert!(!h.agent(x.agent_id.as_deref().unwrap()).archived);
    // « L'agent résout » would merge the target into the branch: refused the same way.
    h.core
        .edit_ticket(&t.id, |x| {
            x.conflict = true;
            Ok(())
        })
        .unwrap();
    assert!(h.core.ticket_resolve_conflict(&t.id).await.is_err());
    assert_eq!(git(&wt, &["rev-parse", "HEAD"]), head);
    assert_eq!(sent_count(&h, &wt), told);
    h.core.ticket_dismiss(&t.id).unwrap();
    h.core.ticket_approve(&t.id).await.unwrap();
    assert_eq!(h.ticket(&t.id).other_line.as_deref(), Some("main"));
    // « Choisir la cible de ce ticket »: main. The block goes, its worktree goes to main, and the
    // validation merges there.
    let chosen = h.core.ticket_set_target(&t.id, "main").await.unwrap();
    assert_eq!(
        (chosen.target.as_str(), chosen.blocked, chosen.other_line),
        ("main", None, None)
    );
    assert_eq!(h.agent_of(&t.id).worktree.unwrap().base_branch, "main");
    h.core.ticket_approve(&t.id).await.unwrap();
    let x = h.ticket(&t.id);
    assert_eq!(x.column, Column::Done, "{:?}", x.blocked);
    assert_eq!(x.outcome.as_deref(), Some("⤵ Mergé dans main · squash"));
    assert_eq!(git(&r, &["rev-parse", "release"]), release);
}

#[tokio::test]
async fn an_ordinary_ticket_is_validated_as_before_beside_branches_of_their_own() {
    let h = harness("tk-other-line-none");
    let (p, r) = h.project(false).await;
    // Another line with commits of its own, and one main is ahead of: neither is the ticket's.
    branch_with_commit(&r, "release", "release.txt");
    git(&r, &["branch", "old"]);
    commit_change(&r, "const a = 2;\n", "main moves on");
    let (t, wt) = reviewed(&h, &p.id, "Fichier [commite]").await;
    // Its agent committed; another ticket's branch made from it does not count either.
    git(
        &r,
        &["branch", "ticket/dem-99", &git(&wt, &["rev-parse", "HEAD"])],
    );
    h.core.ticket_approve(&t.id).await.unwrap();
    let x = h.ticket(&t.id);
    assert_eq!(x.column, Column::Done, "{:?}", x.blocked);
    assert_eq!(x.other_line, None);
}

#[tokio::test]
async fn the_box_says_a_tickets_own_target_and_chooses_it_after_a_stop() {
    let h = harness("inbox-own-target");
    h.core.settings.write().mcp_enabled = true;
    let (_, _, t, _) = from_another_line(&h).await;
    // To approve, from the box: the same guard.
    let views = h.core.inbox_views(None).await.views;
    let view = views
        .iter()
        .find(|v| v.source == ViewSource::TicketReview)
        .unwrap();
    assert_eq!(view.target, None, "it goes to the Kanban's");
    h.core
        .inbox_view_act(&view.key, ViewAct::Approve)
        .await
        .unwrap();
    let views = h.core.inbox_views(None).await.views;
    let blocked = views
        .iter()
        .find(|v| v.source == ViewSource::TicketBlocked)
        .expect("stopped for another line");
    assert_eq!(blocked.other_line.as_deref(), Some("main"));
    // « Viser main » in the box: its own target, said on its line from now on.
    h.core
        .inbox_view_act(
            &blocked.key,
            ViewAct::Retarget {
                target: "main".into(),
            },
        )
        .await
        .unwrap();
    let views = h.core.inbox_views(None).await.views;
    let view = views
        .iter()
        .find(|v| v.source == ViewSource::TicketReview)
        .unwrap();
    assert_eq!(view.target.as_deref(), Some("main"));
    h.core
        .inbox_view_act(&view.key, ViewAct::Approve)
        .await
        .unwrap();
    assert_eq!(h.ticket(&t.id).column, Column::Done);
}

#[test]
fn the_retarget_gesture_comes_as_the_window_sends_it() {
    let a: ViewAct =
        serde_json::from_value(serde_json::json!({"act": "retarget", "target": "main"})).unwrap();
    assert_eq!(
        a,
        ViewAct::Retarget {
            target: "main".into()
        }
    );
}
