//! The news of the pull requests (ESC-68): what two readings tell, in both languages; and on the
//! real core against a fake GitHub, that each is told once, by its own choice, without one more
//! call to the host.

use super::*;
use crate::agent::NotifyKind::*;
use crate::agentprs_tests::{github_agent, github_project};
use crate::core_tests::{harness, Harness};
use crate::i18n::Lang::{En, Fr};
use crate::integrations::fake::FakeServer;
use crate::model::{Column, NotifyFor};
use crate::tickets_tests::reviewed;
use serde_json::{json, Value};

fn facts(red: bool, changes: bool, approved: bool, conflict: bool) -> Facts {
    Facts {
        red,
        changes,
        approved,
        conflict,
        merged: false,
    }
}

#[test]
fn only_what_was_not_so_at_the_last_reading_is_news() {
    let quiet = Facts::default();
    assert!(news(&quiet, &quiet).is_empty());
    assert_eq!(
        news(&quiet, &facts(true, true, false, true)),
        vec![PrConflict, PrCiFailed, PrChanges]
    );
    assert_eq!(
        news(&quiet, &facts(false, false, true, false)),
        [PrApproved]
    );
    assert!(
        news(
            &facts(true, false, false, false),
            &facts(true, false, false, false)
        )
        .is_empty(),
        "already told"
    );
    // Green then red again: told again.
    assert_eq!(
        news(&quiet, &facts(true, false, false, false)),
        [PrCiFailed]
    );
    // Merged: that alone, once.
    let merged = Facts {
        merged: true,
        ..Facts::default()
    };
    assert_eq!(news(&facts(true, true, false, true), &merged), [PrMerged]);
    assert!(news(&merged, &merged).is_empty());
}

#[test]
fn a_closed_pull_request_says_nothing_of_its_ci_or_its_reviews() {
    let pr = PullRequest {
        state: PrState::Closed,
        checks: Some(CheckState::Failure),
        changes_requested: true,
        approvals: 2,
        mergeable: Some(false),
        ..Default::default()
    };
    assert_eq!(Facts::of_pr(&pr), Facts::default());
}

#[test]
fn an_approval_with_changes_asked_is_not_an_approval() {
    let pr = PullRequest {
        approvals: 1,
        changes_requested: true,
        ..Default::default()
    };
    assert_eq!(Facts::of_pr(&pr), facts(false, true, false, false));
}

#[test]
fn the_texts_in_both_languages_say_the_hosts_word_and_no_title() {
    let cases = [
        (PrCiFailed, "PR #12 : CI en échec", "PR #12: CI failed"),
        (
            PrChanges,
            "PR #12 : changements demandés",
            "PR #12: changes requested",
        ),
        (PrApproved, "PR #12 : approuvée", "PR #12: approved"),
        (PrMerged, "PR #12 : mergée", "PR #12: merged"),
        (
            PrConflict,
            "PR #12 : conflit avec sa cible",
            "PR #12: conflicts with its target",
        ),
        (
            PrReview,
            "PR #12 : relecture demandée",
            "PR #12: review requested",
        ),
    ];
    for (kind, fr, en) in cases {
        assert_eq!(text(Fr, kind, "PR", 12, None), fr);
        assert_eq!(text(En, kind, "PR", 12, None), en);
    }
    assert_eq!(
        text(Fr, PrCiFailed, "MR", 3, Some("DEM-1")),
        "DEM-1 · MR #3 : CI en échec"
    );
    assert_eq!(
        text(En, PrMerged, "MR", 3, Some("DEM-1")),
        "DEM-1 · MR #3: merged"
    );
}

#[test]
fn a_review_is_asked_of_me_in_any_case_and_never_on_my_own() {
    let pr = |number, author: &str, reviewers: &[&str]| PullRequest {
        number,
        author: author.into(),
        reviewers: reviewers.iter().map(|r| r.to_string()).collect(),
        ..Default::default()
    };
    let prs = [
        pr(1, "bob", &["Ada"]),
        pr(2, "ada", &["ada"]),
        pr(3, "bob", &["eve"]),
    ];
    let asked: Vec<u64> = asked_of(&prs, "ada").iter().map(|p| p.number).collect();
    assert_eq!(asked, [1]);
    assert!(asked_of(&prs, "").is_empty(), "no login: nobody");
}

#[test]
fn the_limit_is_near_under_a_tenth_until_it_starts_over() {
    let l = |remaining, reset| {
        Some(RateLimit {
            limit: 5000,
            remaining,
            reset,
        })
    };
    assert!(limit_near(l(499, 2000), 1000));
    assert!(!limit_near(l(500, 2000), 1000));
    assert!(!limit_near(l(0, 900), 1000), "started over since");
    assert!(!limit_near(None, 1000));
}

#[test]
fn the_first_reading_of_a_list_tells_nothing() {
    let list = |numbers: &[u64]| ProjectPrs {
        word: "PR".into(),
        me: "ada".into(),
        prs: numbers
            .iter()
            .map(|n| PullRequest {
                number: *n,
                reviewers: vec!["ada".into()],
                ..Default::default()
            })
            .collect(),
    };
    let mut l = PrList::default();
    assert!(l.read_well(1, &list(&[1])).is_empty());
    assert_eq!(l.read_well(2, &list(&[1, 2])), [2]);
    assert!(l.read_well(3, &list(&[1, 2])).is_empty());
    // Given, then asked again: told again.
    assert!(l.read_well(4, &list(&[2])).is_empty());
    assert_eq!(l.read_well(5, &list(&[1, 2])), [1]);
}

// ---------- On the real core, against a fake GitHub ----------

fn node(number: u64, rollup: &str, review: Option<&str>, mergeable: &str, state: &str) -> Value {
    let reviews = match review {
        Some(s) => json!([{ "state": s }]),
        None => json!([]),
    };
    json!({
        "number": number, "url": format!("https://github.com/acme/demo/pull/{number}"),
        "title": "feat", "state": state, "isDraft": false,
        "headRefName": "ticket/dem-1", "baseRefName": "main", "mergeable": mergeable,
        "isCrossRepository": false, "autoMergeRequest": null,
        "latestOpinionatedReviews": { "nodes": reviews },
        "reviewThreads": { "nodes": [] },
        "commits": { "nodes": [{ "commit": { "statusCheckRollup": { "state": rollup } } }] },
    })
}

fn answer(server: &FakeServer, node: Value) {
    let number = node["number"].as_u64().unwrap();
    server.on(
        "POST",
        "/graphql#Pulls",
        200,
        json!({ "data": { "repository": { format!("p{number}"): node } } }),
    );
}

/// A ticket « En revue » with its pull request #12 on the fake GitHub.
async fn ticket_in_review(h: &Harness, server: &FakeServer) -> (String, String) {
    let (p, _, _) = github_project(h, server, false).await;
    h.set_board(&p.id, |s| s.action = "pr".into());
    server.on("GET", "/repos/acme/demo", 200, json!({}));
    let (t, _) = reviewed(h, &p.id, "Fichier [ok]").await;
    h.core
        .edit_ticket(&t.id, |x| {
            x.column = Column::Pr;
            x.pr = Some(TicketPr {
                number: 12,
                url: "https://github.com/acme/demo/pull/12".into(),
                kind: "github".into(),
                host: "github.com".into(),
                namespace: "acme".into(),
                name: "demo".into(),
                target: "main".into(),
                branch: "ticket/dem-1".into(),
                word: "PR".into(),
                ..Default::default()
            });
            Ok(())
        })
        .unwrap();
    h.core.alerts.lock().clear();
    (p.id, t.id)
}

#[tokio::test]
async fn the_readings_of_a_tickets_pull_request_tell_each_news_once() {
    let h = harness("prnews-ticket");
    let server = FakeServer::start().await;
    let (p, t) = ticket_in_review(&h, &server).await;
    let key = h.ticket(&t).key;
    answer(&server, node(12, "PENDING", None, "MERGEABLE", "OPEN"));
    h.core.ticket_prs_refresh(Some(&p)).await.unwrap();
    assert!(h.alerts().is_empty(), "{:?}", h.alerts());

    // Red, and a conflict: both told, by the project's name, then not again.
    answer(&server, node(12, "FAILURE", None, "CONFLICTING", "OPEN"));
    h.core.ticket_prs_refresh(Some(&p)).await.unwrap();
    h.core.ticket_prs_refresh(Some(&p)).await.unwrap();
    assert_eq!(
        h.alerts(),
        [
            format!("demo | {key} · PR #12 : conflit avec sa cible"),
            format!("demo | {key} · PR #12 : CI en échec"),
        ]
    );

    // Changes asked, then approved.
    h.core.alerts.lock().clear();
    let changes = node(
        12,
        "SUCCESS",
        Some("CHANGES_REQUESTED"),
        "MERGEABLE",
        "OPEN",
    );
    answer(&server, changes);
    server.on("GET", "/repos/acme/demo/pulls/12/reviews", 200, json!([]));
    server.on(
        "POST",
        "/graphql#Threads",
        200,
        json!({ "data": { "repository": { "pullRequest": { "reviewThreads": { "nodes": [] } } } } }),
    );
    server.on("GET", "/repos/acme/demo/issues/12/comments", 200, json!([]));
    h.core.ticket_prs_refresh(Some(&p)).await.unwrap();
    answer(
        &server,
        node(12, "SUCCESS", Some("APPROVED"), "MERGEABLE", "OPEN"),
    );
    h.core.ticket_prs_refresh(Some(&p)).await.unwrap();
    assert_eq!(
        h.alerts(),
        [
            format!("demo | {key} · PR #12 : changements demandés"),
            format!("demo | {key} · PR #12 : approuvée"),
        ]
    );

    // Merged: told once, the ticket done.
    h.core.alerts.lock().clear();
    answer(
        &server,
        node(12, "SUCCESS", Some("APPROVED"), "MERGEABLE", "MERGED"),
    );
    h.core.ticket_prs_refresh(Some(&p)).await.unwrap();
    assert_eq!(h.ticket(&t).column, Column::Done);
    assert_eq!(h.alerts(), [format!("demo | {key} · PR #12 : mergée")]);
}

#[tokio::test]
async fn each_news_is_switched_off_by_its_own_choice_and_says_nothing_then() {
    let h = harness("prnews-off");
    let server = FakeServer::start().await;
    let (p, t) = ticket_in_review(&h, &server).await;
    let key = h.ticket(&t).key;
    h.core.settings.write().notify_for = NotifyFor {
        pr_ci_failed: false,
        ..NotifyFor::default()
    };
    answer(&server, node(12, "FAILURE", None, "CONFLICTING", "OPEN"));
    h.core.ticket_prs_refresh(Some(&p)).await.unwrap();
    assert_eq!(
        h.alerts(),
        [format!("demo | {key} · PR #12 : conflit avec sa cible")]
    );
}

#[tokio::test]
async fn a_notification_asks_the_host_nothing_more_than_the_reading_did() {
    let h = harness("prnews-calls");
    let server = FakeServer::start().await;
    let (p, _) = ticket_in_review(&h, &server).await;
    answer(&server, node(12, "PENDING", None, "MERGEABLE", "OPEN"));
    h.core.ticket_prs_refresh(Some(&p)).await.unwrap();
    server.clear_requests();
    h.core.ticket_prs_refresh(Some(&p)).await.unwrap();
    let quiet = server.requests().len();
    server.clear_requests();
    answer(
        &server,
        node(12, "PENDING", Some("APPROVED"), "CONFLICTING", "OPEN"),
    );
    h.core.ticket_prs_refresh(Some(&p)).await.unwrap();
    assert_eq!(h.alerts().len(), 2, "{:?}", h.alerts());
    assert_eq!(server.requests().len(), quiet);
}

#[tokio::test]
async fn an_agents_pull_requests_are_told_by_its_own_name_unless_a_ticket_holds_them() {
    let h = harness("prnews-agent");
    let server = FakeServer::start().await;
    let (id, _, branch) = github_agent(&h, &server).await;
    let name = h.agent(&id).name;
    let of = |rollup: &str| {
        let mut n = node(7, rollup, None, "MERGEABLE", "OPEN");
        n["headRefName"] = json!(branch);
        json!({ "data": { "repository": { "pullRequests": { "nodes": [n] } } } })
    };
    server.on("POST", "/graphql#PullsOfBranch", 200, of("SUCCESS"));
    h.core.agent_prs_refresh(&id, true).await.unwrap();
    assert!(h.alerts().is_empty(), "first seen: no news");
    server.on("POST", "/graphql#PullsOfBranch", 200, of("FAILURE"));
    h.core.agent_prs_refresh(&id, true).await.unwrap();
    assert_eq!(h.alerts(), [format!("demo · {name} | PR #7 : CI en échec")]);

    // A ticket holds it: its follow tells, not this reading.
    h.core.alerts.lock().clear();
    server.on("POST", "/graphql#PullsOfBranch", 200, of("SUCCESS"));
    h.core.agent_prs_refresh(&id, true).await.unwrap();
    let p = h.agent(&id).project_id;
    h.core.tickets.write().push(crate::model::Ticket {
        project_id: p,
        pr: Some(TicketPr {
            number: 7,
            ..Default::default()
        }),
        ..Default::default()
    });
    server.on("POST", "/graphql#PullsOfBranch", 200, of("FAILURE"));
    h.core.agent_prs_refresh(&id, true).await.unwrap();
    assert!(h.alerts().is_empty(), "{:?}", h.alerts());
}

fn open_pulls(server: &FakeServer, asked: &[u64]) {
    let nodes: Vec<Value> = asked
        .iter()
        .map(|n| {
            json!({
                "number": n, "url": format!("https://github.com/acme/demo/pull/{n}"), "title": "x",
                "state": "OPEN", "isDraft": false, "headRefName": "x", "baseRefName": "main",
                "mergeable": "MERGEABLE", "author": { "login": "bob" },
                "isCrossRepository": false, "autoMergeRequest": null,
                "latestOpinionatedReviews": { "nodes": [] }, "reviewThreads": { "nodes": [] },
                "reviewRequests": { "nodes": [{ "requestedReviewer": { "login": "ada" } }] },
                "commits": { "nodes": [] },
            })
        })
        .collect();
    server.on(
        "POST",
        "/graphql#OpenPulls",
        200,
        json!({ "data": { "repository": { "pullRequests": { "nodes": nodes } } } }),
    );
}

#[tokio::test]
async fn a_review_newly_asked_of_the_user_is_told_by_the_list_and_kept_for_who_can_wait() {
    let h = harness("prnews-review");
    let server = FakeServer::start().await;
    let (p, _, _) = github_project(&h, &server, false).await;
    open_pulls(&server, &[3]);
    h.core.project_prs(&p.id).await.data.unwrap();
    assert!(h.alerts().is_empty(), "the first reading tells nothing");
    open_pulls(&server, &[3, 4]);
    h.core.project_prs(&p.id).await.data.unwrap();
    assert_eq!(h.alerts(), ["demo | PR #4 : relecture demandée"]);

    // Under a minute old, the list is given again without asking the host.
    server.clear_requests();
    let (read, at) = h.core.project_prs_recent(&p.id).await;
    assert_eq!(read.data.unwrap().prs.len(), 2);
    assert!(at.is_some());
    assert!(server.requests().is_empty(), "{:?}", server.requests());

    // The overview takes it the same way.
    let lists = h.core.overview_prs().await;
    assert_eq!(lists.len(), 1);
    assert_eq!(
        (lists[0].project_id.as_str(), lists[0].prs.len()),
        (p.id.as_str(), 2)
    );
    assert!(server.requests().is_empty());
}

#[tokio::test]
async fn near_the_hosts_limit_what_can_wait_is_not_read() {
    let h = harness("prnews-limit");
    let server = FakeServer::start().await;
    let (p, _, _) = github_project(&h, &server, false).await;
    let reset = (crate::model::now_ms() / 1000) as u64 + 600;
    let limit = RateLimit {
        limit: 5000,
        remaining: 10,
        reset,
    };
    // Nothing read yet: the reason.
    h.core
        .pr_lists
        .lock()
        .entry(p.id.clone())
        .or_default()
        .limit = Some(limit);
    let (read, _) = h.core.project_prs_recent(&p.id).await;
    let f = read.failure.unwrap();
    assert_eq!(f.kind, crate::prview::FailureKind::RateLimit);
    assert_eq!(f.reset, Some(reset));
    assert!(server.requests().iter().all(|r| r.path() != "/graphql"));
    // The window's own reading goes on, and the last list stays what can wait reads.
    open_pulls(&server, &[3]);
    h.core.project_prs(&p.id).await.data.unwrap();
    {
        let mut lists = h.core.pr_lists.lock();
        let list = lists.get_mut(&p.id).unwrap();
        list.limit = Some(limit);
        let (at, _, _) = list.read.as_mut().unwrap();
        *at -= std::time::Duration::from_secs(3600);
    }
    server.clear_requests();
    let (read, _) = h.core.project_prs_recent(&p.id).await;
    assert_eq!(read.data.unwrap().prs.len(), 1);
    assert!(server.requests().is_empty());
}
