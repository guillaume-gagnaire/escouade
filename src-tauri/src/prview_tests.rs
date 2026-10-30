//! The « PR et CI » tab (ESC-305): the kinds of a host's errors, and the reads of the agent's
//! pull request (its checks with their times, the log of a failed one, its reviews, comments and
//! deployments), « Relancer les échecs », the reason without one and the project's open pull
//! requests, on the real core against a fake GitHub.

use crate::agentprs_tests::{github_agent, github_project};
use crate::core_tests::harness;
use crate::forge::{CheckState, PrState, PullRequest, RateLimit};
use crate::integrations::fake::FakeServer;
use crate::integrations::HttpError;
use crate::prview::*;
use serde_json::json;

fn http(status: u16, message: &str) -> anyhow::Error {
    HttpError {
        status,
        message: message.into(),
    }
    .into()
}

#[test]
fn a_hosts_error_is_told_by_its_kind() {
    let spent = Some(RateLimit {
        limit: 5000,
        remaining: 0,
        reset: 1_900_000_000,
    });
    let left = Some(RateLimit {
        limit: 5000,
        remaining: 4000,
        reset: 1_900_000_000,
    });
    assert_eq!(
        failure(&http(401, "Bad credentials"), None).kind,
        FailureKind::Refused
    );
    let f = failure(&http(403, "Forbidden"), spent);
    assert_eq!(
        (f.kind, f.reset),
        (FailureKind::RateLimit, Some(1_900_000_000))
    );
    // Its message says so, the client had not learned it yet.
    let f = failure(&http(429, "API rate limit exceeded for user"), None);
    assert_eq!((f.kind, f.reset), (FailureKind::RateLimit, None));
    // A 403 with calls left is a right missing, not the limit.
    assert_eq!(
        failure(&http(403, "Resource not accessible"), left).kind,
        FailureKind::Other
    );
    assert_eq!(
        failure(&http(502, "Bad gateway"), None).kind,
        FailureKind::Unreachable
    );
    assert_eq!(
        failure(&anyhow::anyhow!("GitHub injoignable : timeout"), None).kind,
        FailureKind::Unreachable
    );
    let f = failure(&http(404, "Not Found"), None);
    assert_eq!(
        (f.kind, f.message.as_str()),
        (FailureKind::Other, "Not Found")
    );
}

fn open_pr(number: u64, head: &str) -> PullRequest {
    PullRequest {
        number,
        url: format!("https://github.com/acme/demo/pull/{number}"),
        state: PrState::Open,
        head: head.into(),
        base: "main".into(),
        ..Default::default()
    }
}

#[tokio::test]
async fn the_agents_pull_request_is_read_whole_and_only_rerun_writes() {
    let h = harness("prview-detail");
    let server = FakeServer::start().await;
    let (id, _, branch) = github_agent(&h, &server).await;
    h.core.agent(&id).unwrap().lock().meta.prs = vec![open_pr(7, &branch)];
    server.on(
        "GET",
        "/repos/acme/demo/pulls/7",
        200,
        json!({ "number": 7, "head": { "sha": "abc" } }),
    );
    server.on(
        "GET",
        "/repos/acme/demo/commits/abc/check-runs",
        200,
        json!({ "check_runs": [
            { "id": 77, "name": "test", "status": "completed", "conclusion": "failure",
              "started_at": "2026-10-30T10:00:00Z", "completed_at": "2026-10-30T10:02:30Z",
              "html_url": "https://github.com/acme/demo/actions/runs/7/job/77",
              "app": { "slug": "github-actions" } },
            { "id": 78, "name": "lint", "status": "completed", "conclusion": "cancelled",
              "started_at": "2026-10-30T10:00:00Z", "completed_at": "2026-10-30T10:00:10Z",
              "app": { "slug": "github-actions" } },
            { "id": 79, "name": "build", "status": "in_progress",
              "started_at": "2026-10-30T10:01:00Z", "app": { "slug": "github-actions" } },
            { "id": 80, "name": "e2e", "status": "queued", "app": { "slug": "github-actions" } },
        ] }),
    );
    server.on(
        "GET",
        "/repos/acme/demo/commits/abc/status",
        200,
        json!({ "statuses": [{ "context": "vercel", "state": "success", "target_url": "https://vercel.com/x" }] }),
    );
    server.on(
        "GET",
        "/repos/acme/demo/pulls/7/reviews",
        200,
        json!([
            { "user": { "login": "bob" }, "state": "CHANGES_REQUESTED", "body": "Ajoute un test." },
            { "user": { "login": "eve" }, "state": "APPROVED", "body": "" },
        ]),
    );
    server.on(
        "POST",
        "/graphql#Threads",
        200,
        json!({ "data": { "repository": { "pullRequest": { "reviewThreads": { "nodes": [
            { "isResolved": false, "path": "src/app.ts", "line": 12,
              "comments": { "nodes": [{ "databaseId": 1, "author": { "login": "bob" }, "body": "Ce nom est peu clair." }] } },
        ] } } } } }),
    );
    server.on(
        "GET",
        "/repos/acme/demo/issues/7/comments",
        200,
        json!([{ "id": 5, "user": { "login": "eve" }, "body": "Bien vu." }]),
    );
    server.on(
        "GET",
        "/repos/acme/demo/deployments",
        200,
        json!([{ "id": 3, "environment": "Preview" }]),
    );
    server.on(
        "GET",
        "/repos/acme/demo/deployments/3/statuses",
        200,
        json!([{ "state": "success", "environment_url": "https://demo-git-login.vercel.app" }]),
    );

    let read = h.core.agent_pr_detail(&id, 7, "").await;
    assert_eq!(read.failure, None);
    let d = read.data.unwrap();
    let names: Vec<_> = d.checks.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["test", "lint", "build", "e2e", "vercel"]);
    let test = &d.checks[0];
    assert_eq!(test.state, CheckState::Failure);
    assert_eq!(test.job.as_deref(), Some("77"));
    assert_eq!(
        test.finished.unwrap() - test.started.unwrap(),
        150,
        "2 min 30"
    );
    assert!(d.checks[1].cancelled);
    assert_eq!(d.checks[1].state, CheckState::Failure);
    // Running: started, not finished; waiting: neither.
    assert_eq!(
        (
            d.checks[2].state,
            d.checks[2].started.is_some(),
            d.checks[2].finished
        ),
        (CheckState::Pending, true, None)
    );
    assert_eq!((d.checks[3].started, d.checks[3].finished), (None, None));
    assert_eq!(d.reviews.len(), 2);
    assert_eq!(d.comments.len(), 2);
    let line = d.comments.iter().find(|c| c.path.is_some()).unwrap();
    assert_eq!(
        (line.path.as_deref(), line.line, line.author.as_str()),
        (Some("src/app.ts"), Some(12), "bob")
    );
    assert_eq!(d.deployments.len(), 1);
    assert_eq!(
        d.deployments[0].url.as_deref(),
        Some("https://demo-git-login.vercel.app")
    );
    assert!(d.can.rerun && d.can.job_logs && d.can.deployments);
    // Nothing written: the reads only (GraphQL's queries are POSTs).
    let posted: Vec<String> = server
        .writes()
        .iter()
        .filter(|r| r.path() != "/graphql")
        .map(|r| r.target.clone())
        .collect();
    assert!(posted.is_empty(), "{posted:?}");

    // The log of the failed one: its end.
    let long = format!("{}\nFAIL src/app.test.ts", "ligne\n".repeat(10_000));
    server.on_text(
        "GET",
        "/repos/acme/demo/actions/jobs/77/logs",
        200,
        &long,
        &[],
    );
    let log = h.core.agent_pr_log(&id, 7, "", "77").await.data.unwrap();
    assert!(log.starts_with("…\n"), "cut");
    assert!(log.ends_with("FAIL src/app.test.ts"));
    assert!(log.len() <= LOG_VIEW + 4);

    // « Relancer les échecs »: the failed jobs' runs, once each.
    server.on(
        "GET",
        "/repos/acme/demo/actions/jobs/77",
        200,
        json!({ "run_id": 7 }),
    );
    server.on(
        "GET",
        "/repos/acme/demo/actions/jobs/78",
        200,
        json!({ "run_id": 7 }),
    );
    server.on(
        "POST",
        "/repos/acme/demo/actions/runs/7/rerun-failed-jobs",
        201,
        json!({}),
    );
    assert_eq!(h.core.agent_pr_rerun(&id, 7, "").await.unwrap(), 1);
    let posted: Vec<String> = server
        .writes()
        .iter()
        .filter(|r| r.path() != "/graphql")
        .map(|r| r.path().to_string())
        .collect();
    assert_eq!(
        posted,
        ["/repos/acme/demo/actions/runs/7/rerun-failed-jobs"]
    );
}

#[tokio::test]
async fn a_refused_token_and_the_limit_are_told_apart() {
    let h = harness("prview-errors");
    let server = FakeServer::start().await;
    let (id, _, branch) = github_agent(&h, &server).await;
    h.core.agent(&id).unwrap().lock().meta.prs = vec![open_pr(7, &branch)];
    server.on(
        "GET",
        "/repos/acme/demo/pulls/7",
        401,
        json!({ "message": "Bad credentials" }),
    );
    let f = h.core.agent_pr_detail(&id, 7, "").await.failure.unwrap();
    assert_eq!(f.kind, FailureKind::Refused);
    server.on_with(
        "GET",
        "/repos/acme/demo/pulls/7",
        403,
        json!({ "message": "API rate limit exceeded" }),
        &[
            ("x-ratelimit-limit", "5000"),
            ("x-ratelimit-remaining", "0"),
            ("x-ratelimit-reset", "4102444800"),
        ],
    );
    let f = h.core.agent_pr_detail(&id, 7, "").await.failure.unwrap();
    assert_eq!(
        (f.kind, f.reset),
        (FailureKind::RateLimit, Some(4_102_444_800))
    );
}

#[tokio::test]
async fn the_projects_open_pull_requests_and_why_none() {
    let h = harness("prview-project");
    let server = FakeServer::start().await;
    let (p, _, _) = github_project(&h, &server, true).await;
    server.on(
        "POST",
        "/graphql#OpenPulls",
        200,
        json!({ "data": { "repository": { "pullRequests": { "nodes": [{
            "number": 12, "url": "https://github.com/acme/demo/pull/12", "title": "Mode sombre",
            "state": "OPEN", "isDraft": false, "headRefName": "escouade/sombre", "baseRefName": "main",
            "mergeable": "CONFLICTING", "author": { "login": "ada" }, "createdAt": "2026-10-29T08:00:00Z",
            "isCrossRepository": false, "autoMergeRequest": null,
            "latestOpinionatedReviews": { "nodes": [] }, "reviewThreads": { "nodes": [] },
            "commits": { "nodes": [{ "commit": { "statusCheckRollup": { "state": "FAILURE" } } }] },
        }] } } } }),
    );
    let read = h.core.project_prs(&p.id).await;
    let d = read.data.unwrap();
    assert_eq!(d.word, "PR");
    let pr = &d.prs[0];
    assert_eq!(
        (pr.number, pr.author.as_str(), pr.mergeable, pr.checks),
        (12, "ada", Some(false), Some(CheckState::Failure))
    );
    assert_eq!(pr.created, 1_793_260_800);

    // A project without a remote: the reason, Réglages not the place.
    let h = harness("prview-project-none");
    let (q, _) = h.project(false).await;
    let f = h.core.project_prs(&q.id).await.failure.unwrap();
    assert_eq!(f.kind, FailureKind::Unavailable);
    assert_eq!(
        f.message,
        "Ce projet n'a pas de dépôt distant chez un hébergeur de code."
    );
    assert!(!f.settings);
}

#[tokio::test]
async fn without_a_pull_request_the_agent_is_told_why_it_cannot_open_one() {
    let h = harness("prview-nopr");
    let server = FakeServer::start().await;
    let (id, _, _) = github_agent(&h, &server).await;
    assert_eq!(h.core.agent_no_pr(&id).await.unwrap(), None, "it can");
    // No account for github.com: Réglages › Intégrations settles it.
    let h = harness("prview-noaccount");
    let (p, r) = h.project(true).await;
    crate::core_tests::git(
        &r,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/acme/demo.git",
        ],
    );
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let no = h.core.agent_no_pr(&id).await.unwrap().unwrap();
    assert_eq!(no.reason, "Aucun compte connecté pour github.com");
    assert!(no.settings);
    // An agent without a branch of its own.
    let h = harness("prview-plain");
    let (q, _) = h.project(false).await;
    let plain = h.core.create_agent(&q.id, None).await.unwrap().meta.id;
    let no = h.core.agent_no_pr(&plain).await.unwrap().unwrap();
    assert!(no.reason.starts_with("Cet agent n'a pas de branche à lui"));
    assert!(!no.settings);
}

#[tokio::test]
async fn a_server_error_is_read_once_more_before_it_is_told() {
    let h = harness("prview-again");
    let server = FakeServer::start().await;
    let (id, _, branch) = github_agent(&h, &server).await;
    h.core.agent(&id).unwrap().lock().meta.prs = vec![open_pr(7, &branch)];
    server.on(
        "GET",
        "/repos/acme/demo/pulls/7",
        200,
        json!({ "number": 7, "head": { "sha": "abc" } }),
    );
    server.on(
        "GET",
        "/repos/acme/demo/commits/abc/check-runs",
        200,
        json!({ "check_runs": [{ "id": 1, "name": "test", "status": "completed", "conclusion": "success" }] }),
    );
    // GitHub's check runs answer 500 now and then: once, then the answer.
    server.on_once(
        "GET",
        "/repos/acme/demo/commits/abc/check-runs",
        500,
        json!({ "message": "Server Error" }),
        &[],
    );
    server.on(
        "GET",
        "/repos/acme/demo/commits/abc/status",
        200,
        json!({ "statuses": [] }),
    );
    server.on("GET", "/repos/acme/demo/pulls/7/reviews", 200, json!([]));
    server.on(
        "POST",
        "/graphql#Threads",
        200,
        json!({ "data": { "repository": { "pullRequest": { "reviewThreads": { "nodes": [] } } } } }),
    );
    server.on("GET", "/repos/acme/demo/issues/7/comments", 200, json!([]));
    server.on("GET", "/repos/acme/demo/deployments", 200, json!([]));
    let read = h.core.agent_pr_detail(&id, 7, "").await;
    assert_eq!(read.failure, None);
    assert_eq!(read.data.unwrap().checks[0].name, "test");
    // Twice in a row: told, as the host out of reach.
    for _ in 0..2 {
        server.on_once(
            "GET",
            "/repos/acme/demo/commits/abc/check-runs",
            502,
            json!({ "message": "Bad gateway" }),
            &[],
        );
    }
    let f = h.core.agent_pr_detail(&id, 7, "").await.failure.unwrap();
    assert_eq!(f.kind, FailureKind::Unreachable);
}
