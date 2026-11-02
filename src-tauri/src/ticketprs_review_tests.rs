//! The review's comments and the conflict of a ticket's pull request (ESC-61): counted on its card,
//! given to its agent on a click only, answered under when the board says so and never resolved;
//! its branch brought up to its target by a merge (or a rebase), against a fake GitHub served by
//! its plugin.

use super::*;
use crate::agentprs_tests::github_project;
use crate::core_tests::{git, harness, Harness};
use crate::i18n::Lang::{En, Fr};
use crate::integrations::fake::FakeServer;
use crate::model::Ticket;
use crate::tickets_tests::reviewed;
use serde_json::{json, Value};

fn comment(id: &str, author: &str, created: u64) -> ReviewComment {
    ReviewComment {
        id: id.into(),
        author: author.into(),
        body: format!("commentaire {id}"),
        path: Some("src/app.ts".into()),
        line: Some(4),
        resolved: false,
        created,
        excerpt: None,
    }
}

fn review(author: &str, state: ReviewState, body: &str, created: u64) -> Review {
    Review {
        author: author.into(),
        state,
        body: body.into(),
        created,
    }
}

fn open_pr(threads: u32, changes: bool) -> PullRequest {
    PullRequest {
        number: 12,
        state: PrState::Open,
        open_threads: threads,
        changes_requested: changes,
        ..Default::default()
    }
}

fn since(t: i64) -> TicketPr {
    TicketPr {
        number: 12,
        word: "PR".into(),
        state: PrState::Open,
        feedback_since: Some(t * 1000),
        ..Default::default()
    }
}

#[test]
fn only_the_comments_not_resolved_since_the_last_push_and_not_handed_are_feedback() {
    let mut tp = since(1_000);
    let comments = vec![
        comment("old", "grace", 900),
        // Before its push, to the second: not feedback; of its push's second: feedback.
        comment("before", "grace", 999),
        comment("same", "grace", 1_000),
        comment("new", "grace", 1_100),
        ReviewComment {
            resolved: true,
            ..comment("done", "grace", 1_100)
        },
        comment("bot", "codecov[bot]", 1_100),
        // The host gives no date: it counts until it is handed.
        comment("undated", "grace", 0),
        // Escouade's own answer under a thread, by the account itself.
        ReviewComment {
            body: "Corrigé dans abc1234".into(),
            ..comment("mine", "ada", 1_200)
        },
        // The account's own words otherwise are feedback.
        ReviewComment {
            body: "Pense au README".into(),
            ..comment("note", "ada", 1_200)
        },
    ];
    let reviews = vec![
        review("grace", ReviewState::ChangesRequested, "Renomme-la.", 1_100),
        review("grace", ReviewState::Commented, "", 1_100),
        review("bob", ReviewState::Approved, "Bien.", 1_100),
        review("eve", ReviewState::ChangesRequested, "Avant.", 900),
    ];
    let (rs, cs) = new_feedback(&tp, "ada", &reviews, &comments);
    assert_eq!(
        cs.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
        vec!["same", "new", "undated", "note"]
    );
    assert_eq!(
        rs.iter().map(|r| r.body.as_str()).collect::<Vec<_>>(),
        vec!["Renomme-la."]
    );
    // Handed (or in the fix under way): never again.
    tp.handed = vec!["new".into(), review_key(&reviews[0])];
    tp.fix_ids = vec!["undated".into()];
    let (rs, cs) = new_feedback(&tp, "ada", &reviews, &comments);
    assert!(rs.is_empty());
    assert_eq!(
        cs.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
        vec!["same", "note"]
    );
}

#[test]
fn a_reading_counts_the_feedback_and_says_when_there_is_more() {
    let mut tp = since(1_000);
    let comments = vec![comment("1", "grace", 1_100)];
    let grew = feedback_read(&mut tp, &open_pr(1, false), Some((&[], &comments)), "ada");
    assert!(grew);
    assert_eq!(tp.feedback, 1);
    // The same again: nothing new to tell.
    assert!(!feedback_read(
        &mut tp,
        &open_pr(1, false),
        Some((&[], &comments)),
        "ada"
    ));
    // Not read this time (the host failed): the count stays.
    assert!(!feedback_read(&mut tp, &open_pr(1, false), None, "ada"));
    assert_eq!(tp.feedback, 1);
    // Every thread resolved and no change asked: none.
    feedback_read(&mut tp, &open_pr(0, false), None, "ada");
    assert_eq!(tp.feedback, 0);
    assert!(feedback_worth_reading(&open_pr(0, true)));
    assert!(!feedback_worth_reading(&open_pr(0, false)));
    assert!(!feedback_worth_reading(&PullRequest {
        state: PrState::Merged,
        ..open_pr(2, true)
    }));
}

fn on_card(tp: TicketPr) -> Ticket {
    Ticket {
        id: "t1".into(),
        column: Column::Pr,
        agent_id: Some("a1".into()),
        pr: Some(tp),
        ..Default::default()
    }
}

#[test]
fn the_feedback_and_the_conflict_go_to_the_agent_on_a_click_only() {
    let mut t = on_card(TicketPr {
        feedback: 2,
        ..since(1)
    });
    assert!(reviews_fixable(&t));
    // The autopilot never gives it: only a failed CI.
    let s = BoardSettings {
        autopilot: true,
        ..Default::default()
    };
    assert!(to_fix(std::slice::from_ref(&t), "", &s).is_empty());
    assert!(!updatable(&t));
    t.pr.as_mut().unwrap().mergeable = Some(false);
    assert!(updatable(&t));
    assert!(to_fix(std::slice::from_ref(&t), "", &s).is_empty());
    // A fix under way: neither.
    fix_begins_as(&mut t, PrFix::Reviews, None, 5);
    assert!(!reviews_fixable(&t) && !updatable(&t));
    assert_eq!(busy(std::slice::from_ref(&t), ""), 1, "it holds a place");
    assert_eq!(update_kind(&BoardSettings::default()), PrFix::Merge);
    assert_eq!(
        update_kind(&BoardSettings {
            pr_update: "rebase".into(),
            ..Default::default()
        }),
        PrFix::Rebase
    );
}

#[test]
fn a_reviews_fix_pushed_hands_its_comments_and_counts_from_its_push() {
    let mut t = on_card(TicketPr {
        feedback: 2,
        ..since(1)
    });
    fix_begins_as(&mut t, PrFix::Reviews, None, 5);
    {
        let p = t.pr.as_mut().unwrap();
        p.fix_ids = vec!["1".into(), "2".into()];
        p.fix_threads = vec!["1".into()];
    }
    // Stopped: not handed, still to address.
    fix_failed(&mut t, "Interrompu".into());
    let p = t.pr.clone().unwrap();
    assert_eq!(p.feedback, 2);
    assert!(p.handed.is_empty() && p.fix_ids.is_empty());
    assert!(!p.awaiting_ci, "a review's fix holds no CI back");
    // Pushed: handed, and only what comes after counts.
    fix_begins_as(&mut t, PrFix::Reviews, None, 6);
    t.pr.as_mut().unwrap().fix_ids = vec!["1".into(), "2".into()];
    fix_done_at(&mut t, 7_000);
    let p = t.pr.clone().unwrap();
    assert_eq!(p.handed, vec!["1".to_string(), "2".to_string()]);
    assert_eq!((p.feedback, p.feedback_since), (0, Some(7_000)));
    assert_eq!(p.fix_kind, PrFix::Reviews);
    // A branch brought up to its target: the host says again whether it conflicts.
    t.pr.as_mut().unwrap().mergeable = Some(false);
    fix_begins_as(&mut t, PrFix::Merge, None, 8);
    fix_done_at(&mut t, 9_000);
    assert_eq!(t.pr.as_ref().unwrap().mergeable, None);
}

#[test]
fn the_texts_of_the_feedback_and_the_conflict_in_both_languages() {
    assert_eq!(reply_text(Fr, "abc1234def"), "Corrigé dans abc1234");
    assert_eq!(reply_text(En, "abc1234def"), "Fixed in abc1234");
    assert_eq!(
        feedback_alert(Fr, "DEM-1", "PR", 12, 1),
        "DEM-1 : un retour de relecture sur la PR #12"
    );
    assert_eq!(
        feedback_alert(En, "DEM-1", "MR", 3, 2),
        "DEM-1: 2 review comments on MR #3"
    );
    let m = merge_conflict_message(Fr, "PR", 12, "main", &["a.txt".into()]);
    assert!(
        m.contains("conflits sur a.txt") && m.contains("Ne pousse pas"),
        "{m}"
    );
    let m = rebase_conflict_message(En, "MR", 3, "origin/main");
    assert!(m.contains("git rebase origin/main") && m.contains("--force-with-lease"));
    assert_eq!(
        nothing_done(Fr, PrFix::Reviews),
        "L'agent n'a rien commité : les retours restent à traiter"
    );
    assert_eq!(
        nothing_done(En, PrFix::Merge),
        "The branch didn’t move: the conflict remains"
    );
    let hunk: String = (1..=20).map(|i| format!("+ligne {i}\n")).collect();
    let tail = excerpt_tail(&hunk);
    assert_eq!(tail.lines().count(), 12);
    assert!(tail.ends_with("+ligne 20"));
}

// ---------- On the real core, against a fake GitHub ----------

/// PR #12 as the fake GitHub tells it: `threads` open, `mergeable` or in conflict.
fn answer_pr(server: &FakeServer, threads: u32, mergeable: bool) {
    let nodes: Vec<Value> = (0..threads).map(|_| json!({"isResolved": false})).collect();
    server.on(
        "POST",
        "/graphql#Pulls",
        200,
        json!({ "data": { "repository": { "p12": {
            "number": 12, "url": "https://github.com/acme/demo/pull/12",
            "title": "feat", "state": "OPEN", "isDraft": false,
            "headRefName": "ticket/dem-1", "baseRefName": "main",
            "mergeable": if mergeable { "MERGEABLE" } else { "CONFLICTING" },
            "isCrossRepository": false, "autoMergeRequest": null,
            "latestOpinionatedReviews": {"nodes": [{"state": "CHANGES_REQUESTED"}]},
            "reviewThreads": {"nodes": nodes},
            "commits": {"nodes": [{"commit": {"statusCheckRollup": null}}]},
        } } } }),
    );
}

/// Its review: grace asks for changes, and her threads (`(id, line, resolved, createdAt)`).
fn answer_review(server: &FakeServer, threads: &[(u64, u32, bool, &str)]) {
    server.on(
        "GET",
        "/repos/acme/demo/pulls/12/reviews",
        200,
        json!([{ "user": {"login": "grace"}, "state": "CHANGES_REQUESTED", "body": "",
                 "submitted_at": "2099-01-01T00:00:00Z" }]),
    );
    let nodes: Vec<Value> = threads
        .iter()
        .map(|(id, line, resolved, at)| {
            json!({ "isResolved": resolved, "path": "dem-1.txt", "line": line,
                "comments": {"nodes": [{ "databaseId": id, "author": {"login": "grace"},
                    "body": format!("Remarque {id} de grace"), "createdAt": at,
                    "diffHunk": format!("@@ -0,0 +1 @@\n+ligne {line}") }]} })
        })
        .collect();
    server.on(
        "POST",
        "/graphql#Threads",
        200,
        json!({ "data": { "repository": { "pullRequest": { "reviewThreads": { "nodes": nodes } } } } }),
    );
    server.on("GET", "/repos/acme/demo/issues/12/comments", 200, json!([]));
}

fn told(h: &Harness, id: &str, what: &str) -> bool {
    h.items(id)
        .iter()
        .any(|i| i["kind"] == "user" && i["text"].as_str().is_some_and(|t| t.contains(what)))
}

fn on_origin(bare: &std::path::Path, branch: &str) -> String {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(bare)
        .args(["log", "--format=%s", branch])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn pr_opens(server: &FakeServer) {
    server.on(
        "POST",
        "/repos/acme/demo/pulls",
        201,
        json!({
            "number": 12, "html_url": "https://github.com/acme/demo/pull/12", "title": "feat",
            "state": "open", "draft": false, "head": {"ref": "ticket/dem-1"}, "base": {"ref": "main"},
        }),
    );
}

async fn fix_over(h: &Harness, id: &str) {
    h.wait_ticket(id, "the fix over", |x| {
        x.pr.as_ref().is_some_and(|p| p.fixing.is_none())
    })
    .await;
}

/// What was written to the host other than the pull request's opening and readings.
fn published(server: &FakeServer) -> Vec<String> {
    server
        .writes()
        .into_iter()
        .filter(|r| {
            let op = r.json()["operationName"].as_str().unwrap_or("").to_string();
            !(r.path() == "/graphql" && (op == "Pulls" || op == "Threads"))
                && !r.path().ends_with("/pulls")
                && !r.path().ends_with("/requested_reviewers")
                && !r.path().ends_with("/labels")
        })
        .map(|r| format!("{} {} {}", r.method, r.path(), r.body))
        .collect()
}

/// New comments are counted on the card with a notification; the autopilot leaves them for the
/// click. « L'agent corrige les retours » gives its agent those not resolved since the last push,
/// each with its file, line, excerpt, author and text; Escouade pushes, nothing is published under
/// the threads (the board's setting is off) and nothing is resolved. Given once only; with
/// « Répondre sous les fils » on, the next ones are answered under.
#[tokio::test]
async fn the_review_comments_go_to_the_agent_on_a_click_and_nothing_is_published() {
    let h = harness("tp-reviews");
    let server = FakeServer::start().await;
    let (p, _r, bare) = github_project(&h, &server, false).await;
    h.set_board(&p.id, |s| s.action = "pr".into());
    pr_opens(&server);
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    let u = h.ticket(&t.id);
    assert_eq!(u.column, Column::Pr, "{:?}", u.blocked);
    let agent_id = u.agent_id.clone().unwrap();
    let alerts = h.alerts().len();

    // Grace's review: one thread before the opening, one resolved, one new.
    answer_pr(&server, 2, true);
    answer_review(
        &server,
        &[
            (901, 1, false, "2020-01-01T00:00:00Z"),
            (902, 1, true, "2099-01-01T00:00:00Z"),
            (903, 1, false, "2099-01-01T00:00:00Z"),
        ],
    );
    h.core.ticket_prs_refresh(Some(&p.id)).await.unwrap();
    let tp = h.ticket(&t.id).pr.unwrap();
    assert_eq!(tp.feedback, 1);
    assert!(
        h.alerts()[alerts..]
            .iter()
            .any(|a| a.contains("DEM-1 : un retour de relecture sur la PR #12")),
        "{:?}",
        h.alerts()
    );
    // The autopilot (on in the harness) leaves it for the click.
    h.core.schedule_now().await;
    assert_eq!(h.ticket(&t.id).pr.unwrap().fixing, None);
    assert!(h.agent(&agent_id).archived);

    h.core.ticket_pr_fix_reviews(&t.id).await.unwrap();
    h.wait("the feedback sent", |h| {
        told(h, &agent_id, "a des retours de relecture")
    })
    .await;
    assert!(told(
        &h,
        &agent_id,
        "<comment author=\"grace\" path=\"dem-1.txt\" line=\"1\">\n<diff>\n@@ -0,0 +1 @@\n+ligne 1\n</diff>\nRemarque 903 de grace\n</comment>"
    ));
    assert!(!told(&h, &agent_id, "Remarque 901"), "older than its push");
    assert!(!told(&h, &agent_id, "Remarque 902"), "resolved");
    // A second click meanwhile is refused.
    assert!(h.core.ticket_pr_fix_reviews(&t.id).await.is_err());
    fix_over(&h, &t.id).await;
    let tp = h.ticket(&t.id).pr.unwrap();
    assert_eq!(tp.fix_error, None, "{:?}", tp.fix_error);
    assert_eq!(tp.handed, vec!["903".to_string()]);
    assert!(on_origin(&bare, "ticket/dem-1").contains("fix: retours de DEM-1"));
    h.wait("its agent archived again", |h| h.agent(&agent_id).archived)
        .await;
    assert!(
        published(&server).is_empty(),
        "nothing answered nor resolved: {:?}",
        published(&server)
    );

    // Read again: handed, not counted; a click has nothing to give.
    h.core.ticket_prs_refresh(Some(&p.id)).await.unwrap();
    assert_eq!(h.ticket(&t.id).pr.unwrap().feedback, 0);
    let e = h.core.ticket_pr_fix_reviews(&t.id).await.unwrap_err();
    assert_eq!(e.to_string(), no_feedback(Fr, "PR", 12));

    // With « Répondre sous les fils », a new one is answered under once pushed, never resolved.
    h.set_board(&p.id, |s| s.reply_threads = true);
    server.on(
        "POST",
        "/repos/acme/demo/pulls/12/comments/904/replies",
        201,
        json!({}),
    );
    answer_review(
        &server,
        &[
            (903, 1, false, "2099-01-01T00:00:00Z"),
            (904, 1, false, "2099-01-02T00:00:00Z"),
        ],
    );
    h.core.ticket_prs_refresh(Some(&p.id)).await.unwrap();
    assert_eq!(h.ticket(&t.id).pr.unwrap().feedback, 1);
    h.core.ticket_pr_fix_reviews(&t.id).await.unwrap();
    fix_over(&h, &t.id).await;
    assert_eq!(h.ticket(&t.id).pr.unwrap().fix_error, None);
    // Answered once pushed, just after the ticket is back « En revue ».
    h.wait("the answer under its thread", |_| {
        !published(&server).is_empty()
    })
    .await;
    h.wait("its agent archived again", |h| h.agent(&agent_id).archived)
        .await;
    let out = published(&server);
    assert_eq!(out.len(), 1, "{out:?}");
    assert!(
        out[0].starts_with("POST /repos/acme/demo/pulls/12/comments/904/replies")
            && out[0].contains("Corrigé dans "),
        "{out:?}"
    );
}

/// In conflict with its target: « L'agent met à jour la branche » merges the target into its
/// branch; without a conflict Escouade pushes at once; with some, its agent resolves them and
/// Escouade pushes at the end of its turn; « abort » undoes it and names the files.
#[tokio::test]
async fn a_conflict_with_the_target_is_settled_by_a_merge_of_the_target() {
    let h = harness("tp-update");
    let server = FakeServer::start().await;
    let (p, r, bare) = github_project(&h, &server, false).await;
    h.set_board(&p.id, |s| s.action = "pr".into());
    pr_opens(&server);
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    let agent_id = h.ticket(&t.id).agent_id.unwrap();
    answer_pr(&server, 0, false);
    h.core.ticket_prs_refresh(Some(&p.id)).await.unwrap();
    assert!(updatable(&h.ticket(&t.id)));
    // The autopilot never does it by itself.
    h.core.schedule_now().await;
    assert_eq!(h.ticket(&t.id).pr.unwrap().fixing, None);

    // The target moved elsewhere: merged in without a conflict, pushed at once.
    std::fs::write(r.join("autre.txt"), "autre\n").unwrap();
    git(&r, &["add", "autre.txt"]);
    git(&r, &["commit", "-qm", "feat: autre"]);
    git(&r, &["push", "-q", "origin", "main"]);
    h.core.ticket_pr_update_branch(&t.id).await.unwrap();
    fix_over(&h, &t.id).await;
    let tp = h.ticket(&t.id).pr.unwrap();
    assert_eq!(tp.fix_error, None, "{:?}", tp.fix_error);
    assert_eq!(tp.mergeable, None, "read again from the host");
    let log = on_origin(&bare, "ticket/dem-1");
    assert!(
        log.contains("feat: autre") && log.contains("Merge"),
        "{log}"
    );
    h.wait("its agent archived again", |h| h.agent(&agent_id).archived)
        .await;
    assert!(!told(&h, &agent_id, "est en conflit avec"));

    // On its own file this time, with « abort »: undone, the card names the file.
    answer_pr(&server, 0, false);
    h.core.ticket_prs_refresh(Some(&p.id)).await.unwrap();
    std::fs::write(r.join("dem-1.txt"), "autre version\n").unwrap();
    git(&r, &["add", "dem-1.txt"]);
    git(&r, &["commit", "-qm", "feat: dem-1 ailleurs"]);
    git(&r, &["push", "-q", "origin", "main"]);
    h.set_board(&p.id, |s| s.conflict = "abort".into());
    let before = on_origin(&bare, "ticket/dem-1");
    assert!(h.core.ticket_pr_update_branch(&t.id).await.is_err());
    let tp = h.ticket(&t.id).pr.unwrap();
    assert_eq!(tp.fixing, None);
    assert!(
        tp.fix_error
            .as_deref()
            .is_some_and(|e| e.contains("dem-1.txt")),
        "{:?}",
        tp.fix_error
    );
    assert_eq!(on_origin(&bare, "ticket/dem-1"), before);

    // As the board asks (« ask »): its agent resolves them, Escouade pushes.
    h.set_board(&p.id, |s| s.conflict = "ask".into());
    h.core.ticket_pr_update_branch(&t.id).await.unwrap();
    h.wait("the conflict sent", |h| {
        told(h, &agent_id, "est en conflit avec main")
    })
    .await;
    assert!(told(&h, &agent_id, "conflits sur dem-1.txt"));
    fix_over(&h, &t.id).await;
    let tp = h.ticket(&t.id).pr.unwrap();
    assert_eq!(tp.fix_error, None, "{:?}", tp.fix_error);
    assert!(on_origin(&bare, "ticket/dem-1").contains("feat: dem-1 ailleurs"));
}

/// With the board's « Rebase », the branch is rebased onto its target and pushed with
/// `--force-with-lease`: no merge commit.
#[tokio::test]
async fn with_the_rebase_setting_the_branch_is_rebased_and_pushed_with_a_lease() {
    let h = harness("tp-rebase");
    let server = FakeServer::start().await;
    let (p, r, bare) = github_project(&h, &server, false).await;
    h.set_board(&p.id, |s| {
        s.action = "pr".into();
        s.pr_update = "rebase".into();
    });
    pr_opens(&server);
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    answer_pr(&server, 0, false);
    h.core.ticket_prs_refresh(Some(&p.id)).await.unwrap();
    std::fs::write(r.join("autre.txt"), "autre\n").unwrap();
    git(&r, &["add", "autre.txt"]);
    git(&r, &["commit", "-qm", "feat: autre"]);
    git(&r, &["push", "-q", "origin", "main"]);
    h.core.ticket_pr_update_branch(&t.id).await.unwrap();
    fix_over(&h, &t.id).await;
    let tp = h.ticket(&t.id).pr.unwrap();
    assert_eq!(tp.fix_error, None, "{:?}", tp.fix_error);
    assert_eq!(tp.fix_kind, PrFix::Rebase);
    let log = on_origin(&bare, "ticket/dem-1");
    assert!(
        log.contains("feat: autre") && !log.contains("Merge"),
        "{log}"
    );
}
