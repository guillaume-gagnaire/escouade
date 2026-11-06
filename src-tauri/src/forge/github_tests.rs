//! GitHub's client against a fake server fed with answers recorded from github.com (a merged pull
//! request of a public repository, `tests/fixtures/forges/github/`, see `record.mjs`), and with
//! small made-up ones for what the recording does not hold (a failing job, a refusal).
//!
//! Since INT-8.1 the client is the plugin « GitHub »'s (`plugins/github`): an
//! account's calls go to its module by `ProcessForge`, as the core
//! makes them, and the plugin calls the fake server. What only the client itself shows (its pace,
//! the answers it keeps) is read on it directly.

use super::*;
use crate::integrations::fake::FakeServer;
use serde_json::json;
use std::time::Duration;

fn fixture(name: &str) -> Value {
    let text = match name {
        "pull" => include_str!("../../../tests/fixtures/forges/github/pull.json"),
        "check-runs" => include_str!("../../../tests/fixtures/forges/github/check-runs.json"),
        "status" => include_str!("../../../tests/fixtures/forges/github/status.json"),
        "reviews" => include_str!("../../../tests/fixtures/forges/github/reviews.json"),
        "issue-comments" => {
            include_str!("../../../tests/fixtures/forges/github/issue-comments.json")
        }
        "graphql-pulls" => include_str!("../../../tests/fixtures/forges/github/graphql-pulls.json"),
        "graphql-branch" => {
            include_str!("../../../tests/fixtures/forges/github/graphql-branch.json")
        }
        "graphql-threads" => {
            include_str!("../../../tests/fixtures/forges/github/graphql-threads.json")
        }
        "deployments" => include_str!("../../../tests/fixtures/forges/github/deployments.json"),
        "deployment-statuses" => {
            include_str!("../../../tests/fixtures/forges/github/deployment-statuses.json")
        }
        _ => panic!("no fixture {name}"),
    };
    serde_json::from_str(text).unwrap()
}

const JOB_LOG: &str = include_str!("../../../tests/fixtures/forges/github/job-log.txt");

fn repo() -> RepoRef {
    RepoRef {
        namespace: "cli".into(),
        name: "cli".into(),
    }
}

/// GitHub's accounts as the core serves them: by the plugin « GitHub », in its process.
struct GithubProvider;

impl GithubProvider {
    fn client(&self, _http: reqwest::Client, creds: &Credentials) -> Box<dyn Forge> {
        crate::plugins::builtin::served_forge(crate::plugins::builtin::GITHUB, "github", creds)
    }
}

/// The client of github.com's account, its API being the fake server.
fn client(server: &FakeServer) -> Box<dyn Forge> {
    GithubProvider.client(
        reqwest::Client::new(),
        &Credentials {
            url: "https://github.com".into(),
            token: "ghp_test".into(),
            api: Some(server.url.clone()),
            ..Default::default()
        },
    )
}

fn route_pull(server: &FakeServer) {
    server.on("GET", "/repos/cli/cli/pulls/14768", 200, fixture("pull"));
    server.on(
        "GET",
        "/repos/cli/cli/commits/6ec1ba54542baca6cc5932582634104c83f36a28/check-runs",
        200,
        fixture("check-runs"),
    );
    server.on(
        "GET",
        "/repos/cli/cli/commits/6ec1ba54542baca6cc5932582634104c83f36a28/status",
        200,
        fixture("status"),
    );
}

fn requests_to(server: &FakeServer, path: &str) -> usize {
    server
        .requests()
        .iter()
        .filter(|r| r.path() == path)
        .count()
}

#[tokio::test]
async fn a_recorded_pull_request_is_read_in_one_graphql_request() {
    let server = FakeServer::start().await;
    server.on("POST", "/graphql#Pulls", 200, fixture("graphql-pulls"));
    let pr = client(&server).pull_request(&repo(), 14768).await.unwrap();
    assert_eq!(pr.number, 14768);
    assert_eq!(pr.state, PrState::Merged);
    assert_eq!(pr.url, "https://github.com/cli/cli/pull/14768");
    assert_eq!(
        (pr.head.as_str(), pr.base.as_str()),
        ("babakks/refresh-token-bump-deps", "trunk")
    );
    assert!(!pr.draft);
    assert_eq!(
        pr.mergeable, None,
        "GitHub does not say for a merged pull request"
    );
    assert_eq!(pr.approvals, 1);
    assert!(!pr.changes_requested);
    assert_eq!(pr.open_threads, 1);
    assert!(!pr.auto_merge);
    assert_eq!(pr.checks, Some(CheckState::Success));
    assert_eq!(server.requests().len(), 1, "one call for all of it");
}

#[tokio::test]
async fn every_followed_pull_request_of_a_repository_costs_one_request() {
    let server = FakeServer::start().await;
    let one = |n: u64, state: &str, mergeable: &str, review: &str, rollup: &str| {
        json!({
            "number": n, "url": format!("https://github.com/cli/cli/pull/{n}"), "title": format!("PR {n}"),
            "state": state, "isDraft": n == 2, "headRefName": format!("b{n}"), "baseRefName": "trunk",
            "mergeable": mergeable,
            "autoMergeRequest": if n == 3 { json!({"enabledAt": "2026-10-22T08:00:00Z"}) } else { Value::Null },
            "latestOpinionatedReviews": {"nodes": [{"state": review}]},
            "reviewThreads": {"nodes": [{"isResolved": true}, {"isResolved": false}]},
            "commits": {"nodes": [{"commit": {"statusCheckRollup": {"state": rollup}}}]},
        })
    };
    server.on(
        "POST",
        "/graphql#Pulls",
        200,
        json!({"data": {"repository": {
            "p1": one(1, "OPEN", "CONFLICTING", "CHANGES_REQUESTED", "FAILURE"),
            "p2": one(2, "OPEN", "MERGEABLE", "APPROVED", "PENDING"),
            "p3": one(3, "CLOSED", "MERGEABLE", "APPROVED", "SUCCESS"),
            // Gone since: GraphQL answers null (and an error), the others still come.
            "p9": null,
        }}, "errors": [{"type": "NOT_FOUND", "message": "Could not resolve to a PullRequest"}]}),
    );
    let prs = client(&server)
        .pull_requests(&repo(), &[1, 2, 3, 9])
        .await
        .unwrap();
    assert_eq!(prs.iter().map(|p| p.number).collect::<Vec<_>>(), [1, 2, 3]);
    assert_eq!(prs[0].mergeable, Some(false), "a conflict");
    assert!(prs[0].changes_requested);
    assert_eq!(prs[0].checks, Some(CheckState::Failure));
    assert!(prs[1].draft);
    assert_eq!(prs[1].checks, Some(CheckState::Pending));
    assert_eq!(prs[1].open_threads, 1);
    assert!(prs[2].auto_merge);
    assert_eq!(prs[2].state, PrState::Closed);

    let sent = server.requests();
    assert_eq!(sent.len(), 1, "four pull requests, one request");
    let query = sent[0].json()["query"].as_str().unwrap().to_string();
    for n in [1, 2, 3, 9] {
        assert!(
            query.contains(&format!("p{n}: pullRequest(number: {n})")),
            "{query}"
        );
    }
    assert_eq!(sent[0].headers["authorization"], "Bearer ghp_test");
    assert_eq!(
        sent[0].json()["variables"],
        json!({"owner": "cli", "name": "cli"})
    );
}

#[tokio::test]
async fn a_branchs_pull_requests_are_found_by_its_name() {
    let server = FakeServer::start().await;
    server.on(
        "POST",
        "/graphql#PullsOfBranch",
        200,
        fixture("graphql-branch"),
    );
    let prs = client(&server)
        .pull_requests_of_branch(&repo(), "babakks/refresh-token-bump-deps")
        .await
        .unwrap();
    assert_eq!(prs.len(), 1);
    assert_eq!(prs[0].number, 14768);
    assert_eq!(
        server.requests()[0].json()["variables"]["branch"],
        "babakks/refresh-token-bump-deps"
    );
}

#[tokio::test]
async fn a_pull_request_that_does_not_exist_is_an_error() {
    let server = FakeServer::start().await;
    server.on(
        "POST",
        "/graphql#Pulls",
        200,
        json!({"data": {"repository": {"p5": null}}, "errors": [{"message": "Could not resolve"}]}),
    );
    assert!(client(&server).pull_request(&repo(), 5).await.is_err());
    server.on(
        "POST",
        "/graphql#Pulls",
        200,
        json!({"data": null, "errors": [{"message": "Could not resolve to a Repository"}]}),
    );
    let e = client(&server).pull_request(&repo(), 5).await.unwrap_err();
    assert!(
        format!("{e:#}").contains("Could not resolve to a Repository"),
        "{e:#}"
    );
}

#[tokio::test]
async fn a_pull_request_opens_as_a_draft_with_its_reviewers_labels_and_issue() {
    let server = FakeServer::start().await;
    server.on(
        "POST",
        "/repos/cli/cli/pulls",
        201,
        json!({"number": 77, "html_url": "https://github.com/cli/cli/pull/77", "title": "Add x",
               "state": "open", "draft": true, "head": {"ref": "feat/x"}, "base": {"ref": "trunk"},
               "mergeable": null, "merged_at": null}),
    );
    server.on(
        "POST",
        "/repos/cli/cli/pulls/77/requested_reviewers",
        201,
        json!({}),
    );
    server.on("POST", "/repos/cli/cli/issues/77/labels", 200, json!([]));
    let pr = client(&server)
        .create_pr(
            &repo(),
            &PullRequestDraft {
                title: "Add x".into(),
                body: "It adds x.".into(),
                head: "feat/x".into(),
                base: "trunk".into(),
                draft: true,
                reviewers: vec!["ada".into(), "grace".into()],
                labels: vec!["agent".into()],
                closes: Some(12),
            },
        )
        .await
        .unwrap();
    assert_eq!((pr.number, pr.draft, pr.state), (77, true, PrState::Open));
    assert_eq!(pr.url, "https://github.com/cli/cli/pull/77");
    assert_eq!(pr.mergeable, None);
    let writes = server.writes();
    assert_eq!(writes.len(), 3);
    let made = writes[0].json();
    assert_eq!(made["title"], "Add x");
    assert_eq!(made["head"], "feat/x");
    assert_eq!(made["base"], "trunk");
    assert_eq!(made["draft"], true);
    assert_eq!(made["body"], "It adds x.\n\nCloses #12");
    assert_eq!(writes[1].json(), json!({"reviewers": ["ada", "grace"]}));
    assert_eq!(writes[2].json(), json!({"labels": ["agent"]}));
}

#[tokio::test]
async fn a_refused_reviewer_does_not_take_the_pull_request_back() {
    let server = FakeServer::start().await;
    server.on(
        "POST",
        "/repos/cli/cli/pulls",
        201,
        json!({"number": 78, "html_url": "u", "title": "t", "state": "open", "draft": false,
               "head": {"ref": "h"}, "base": {"ref": "b"}}),
    );
    server.on(
        "POST",
        "/repos/cli/cli/pulls/78/requested_reviewers",
        422,
        json!({"message": "Review cannot be requested from pull request author."}),
    );
    let pr = client(&server)
        .create_pr(
            &repo(),
            &PullRequestDraft {
                title: "t".into(),
                head: "h".into(),
                base: "b".into(),
                reviewers: vec!["me".into()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(pr.number, 78);
    // No « Closes » line without an issue, and an existing one is not repeated.
    assert_eq!(server.writes()[0].json()["body"], "");
}

#[tokio::test]
async fn the_closing_line_is_not_written_twice() {
    let server = FakeServer::start().await;
    server.on(
        "POST",
        "/repos/cli/cli/pulls",
        201,
        json!({"number": 1, "head": {"ref": "h"}, "base": {"ref": "b"}}),
    );
    client(&server)
        .create_pr(
            &repo(),
            &PullRequestDraft {
                body: "Closes #5".into(),
                closes: Some(5),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(server.writes()[0].json()["body"], "Closes #5");
}

#[tokio::test]
async fn the_checks_of_a_recorded_pull_request_come_with_their_jobs() {
    let server = FakeServer::start().await;
    route_pull(&server);
    let checks = client(&server).checks(&repo(), 14768).await.unwrap();
    assert_eq!(checks.len(), 11);
    assert!(checks.iter().all(|c| c.state == CheckState::Success));
    let lint = checks.iter().find(|c| c.name == "lint").unwrap();
    assert!(lint
        .url
        .as_deref()
        .unwrap()
        .starts_with("https://github.com/cli/cli/actions"));
    assert!(lint.job.is_some(), "an Actions job has a log");
    let codeql = checks.iter().find(|c| c.name == "CodeQL").unwrap();
    assert_eq!(codeql.job, None, "another app keeps its log to itself");
}

#[tokio::test]
async fn checks_and_statuses_are_read_whatever_their_outcome() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        "/repos/o/r/pulls/3",
        200,
        json!({"head": {"sha": "abc"}}),
    );
    let run = |name: &str, id: u64, status: &str, conclusion: Value, slug: &str| {
        json!({"name": name, "id": id, "status": status, "conclusion": conclusion,
               "html_url": format!("https://github.com/o/r/runs/{id}"), "app": {"slug": slug}})
    };
    server.on(
        "GET",
        "/repos/o/r/commits/abc/check-runs",
        200,
        json!({"total_count": 6, "check_runs": [
            run("build", 1, "completed", json!("failure"), "github-actions"),
            run("test", 2, "in_progress", Value::Null, "github-actions"),
            run("lint", 3, "completed", json!("skipped"), "github-actions"),
            run("docs", 4, "completed", json!("neutral"), "github-actions"),
            run("slow", 5, "completed", json!("timed_out"), "github-actions"),
            run("sonar", 6, "completed", json!("cancelled"), "sonarcloud"),
        ]}),
    );
    server.on(
        "GET",
        "/repos/o/r/commits/abc/status",
        200,
        json!({"state": "failure", "statuses": [
            {"context": "ci/jenkins", "state": "failure", "target_url": "https://ci.acme.dev/1"},
            {"context": "ci/circle", "state": "pending", "target_url": null},
            {"context": "ci/travis", "state": "success", "target_url": null},
        ]}),
    );
    let repo = RepoRef {
        namespace: "o".into(),
        name: "r".into(),
    };
    let checks = client(&server).checks(&repo, 3).await.unwrap();
    let state = |n: &str| checks.iter().find(|c| c.name == n).unwrap().state;
    assert_eq!(state("build"), CheckState::Failure);
    assert_eq!(state("test"), CheckState::Pending);
    assert_eq!(state("lint"), CheckState::Skipped);
    assert_eq!(state("docs"), CheckState::Success);
    assert_eq!(state("slow"), CheckState::Failure);
    assert_eq!(state("sonar"), CheckState::Failure);
    assert_eq!(state("ci/jenkins"), CheckState::Failure);
    assert_eq!(state("ci/circle"), CheckState::Pending);
    assert_eq!(state("ci/travis"), CheckState::Success);
    let jenkins = checks.iter().find(|c| c.name == "ci/jenkins").unwrap();
    assert_eq!(jenkins.url.as_deref(), Some("https://ci.acme.dev/1"));
    assert_eq!(jenkins.job, None, "a status has an address and no log");
}

#[tokio::test]
async fn the_check_runs_of_a_commit_are_read_on_every_page() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        "/repos/o/r/pulls/3",
        200,
        json!({"head": {"sha": "abc"}}),
    );
    let run = |name: &str, id: u64| {
        json!({"name": name, "id": id, "status": "completed", "conclusion": "success",
               "html_url": format!("https://github.com/o/r/runs/{id}"), "app": {"slug": "github-actions"}})
    };
    // The answer is an object whose `check_runs` is the list; its `Link` leads to the next page.
    let next = format!(
        "<{}/repos/o/r/commits/abc/check-runs?per_page=100&page=2>; rel=\"next\"",
        server.url
    );
    server.on_with(
        "GET",
        "/repos/o/r/commits/abc/check-runs?per_page=100",
        200,
        json!({"total_count": 3, "check_runs": [run("build", 1), run("test", 2)]}),
        &[("Link", &next)],
    );
    server.on(
        "GET",
        "/repos/o/r/commits/abc/check-runs?per_page=100&page=2",
        200,
        json!({"total_count": 3, "check_runs": [run("lint", 3)]}),
    );
    server.on(
        "GET",
        "/repos/o/r/commits/abc/status",
        200,
        json!({"statuses": []}),
    );
    let repo = RepoRef {
        namespace: "o".into(),
        name: "r".into(),
    };
    let checks = client(&server).checks(&repo, 3).await.unwrap();
    assert_eq!(
        checks.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
        ["build", "test", "lint"],
        "the second page is read too"
    );
    assert_eq!(requests_to(&server, "/repos/o/r/commits/abc/check-runs"), 2);
}

#[tokio::test]
async fn a_jobs_log_is_read_as_the_text_it_is() {
    let server = FakeServer::start().await;
    server.on_text(
        "GET",
        "/repos/cli/cli/actions/jobs/119211823435/logs",
        200,
        JOB_LOG,
        &[],
    );
    let log = client(&server)
        .job_log(&repo(), "119211823435")
        .await
        .unwrap();
    assert_eq!(log, JOB_LOG);
    assert!(log.contains("Current runner version"));
    // A job gone from GitHub (logs expire) is an error that says so.
    let e = client(&server).job_log(&repo(), "1").await.unwrap_err();
    assert!(format!("{e:#}").contains("404"), "{e:#}");
}

/// What `limits.fetchBytes` of its manifest lets through (8 MiB) fits the memory of a module; beyond it, it says so.
#[tokio::test]
async fn a_long_log_is_read_whole_and_a_longer_one_is_refused() {
    let server = FakeServer::start().await;
    let path = "/repos/cli/cli/actions/jobs/1/logs";
    let log = "2026-11-05T10:00:00Z une ligne du journal, \"citée\"
"
    .repeat(158_000);
    assert!(log.len() > 7 << 20 && log.len() < 8 << 20);
    server.on_text("GET", path, 200, &log, &[]);
    let c = client(&server);
    assert_eq!(c.job_log(&repo(), "1").await.unwrap(), log);
    server.on_text("GET", path, 200, &"x".repeat(9 << 20), &[]);
    let e = c.job_log(&repo(), "1").await.unwrap_err();
    assert!(format!("{e:#}").contains("an answer over"), "{e:#}");
}

#[tokio::test]
async fn a_jobs_log_redirected_to_another_host_is_read_without_the_token() {
    let api = FakeServer::start().await;
    // Where GitHub keeps the text: another host, which must not get the token, and that its manifest names.
    let storage = FakeServer::start().await;
    storage.on_text("GET", "/logs/119211823435.txt", 200, JOB_LOG, &[]);
    let blob = "productionresultssa0.blob.core.windows.net";
    api.on_with(
        "GET",
        "/repos/cli/cli/actions/jobs/119211823435/logs",
        302,
        Value::Null,
        &[("Location", &format!("https://{blob}/logs/119211823435.txt"))],
    );
    let client = crate::plugins::builtin::served_forge_on(
        crate::plugins::builtin::GITHUB,
        "github",
        &Credentials {
            url: "https://github.com".into(),
            token: "ghp_test".into(),
            api: Some(api.url.clone()),
            ..Default::default()
        },
        &[(blob, &storage.url)],
    );
    let log = client.job_log(&repo(), "119211823435").await.unwrap();
    assert_eq!(log, JOB_LOG, "the text the redirection leads to");
    let asked = api.requests();
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].headers["authorization"], "Bearer ghp_test");
    let followed = storage.requests();
    assert_eq!(followed.len(), 1, "the redirection is followed");
    assert!(
        !followed[0].headers.contains_key("authorization"),
        "the token stays on GitHub's host: {:?}",
        followed[0].headers
    );
}

#[tokio::test]
async fn failed_jobs_run_again_once_per_run() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        "/repos/o/r/pulls/3",
        200,
        json!({"head": {"sha": "abc"}}),
    );
    let run = |name: &str, id: u64, conclusion: &str| {
        json!({"name": name, "id": id, "status": "completed", "conclusion": conclusion,
               "html_url": "u", "app": {"slug": "github-actions"}})
    };
    server.on(
        "GET",
        "/repos/o/r/commits/abc/check-runs",
        200,
        json!({"check_runs": [run("a", 11, "failure"), run("b", 12, "failure"),
                              run("c", 13, "success"), run("d", 14, "failure")]}),
    );
    server.on(
        "GET",
        "/repos/o/r/commits/abc/status",
        200,
        json!({"statuses": []}),
    );
    // a and b belong to one run, d to another; c passed and is not asked about.
    server.on(
        "GET",
        "/repos/o/r/actions/jobs/11",
        200,
        json!({"run_id": 900}),
    );
    server.on(
        "GET",
        "/repos/o/r/actions/jobs/12",
        200,
        json!({"run_id": 900}),
    );
    server.on(
        "GET",
        "/repos/o/r/actions/jobs/14",
        200,
        json!({"run_id": 901}),
    );
    server.on(
        "POST",
        "/repos/o/r/actions/runs/900/rerun-failed-jobs",
        201,
        Value::Null,
    );
    server.on(
        "POST",
        "/repos/o/r/actions/runs/901/rerun-failed-jobs",
        201,
        Value::Null,
    );
    let repo = RepoRef {
        namespace: "o".into(),
        name: "r".into(),
    };
    let started = client(&server).rerun_failed(&repo, 3).await.unwrap();
    assert_eq!(started, 2);
    let writes: Vec<String> = server
        .writes()
        .iter()
        .map(|w| w.path().to_string())
        .collect();
    assert_eq!(
        writes,
        [
            "/repos/o/r/actions/runs/900/rerun-failed-jobs",
            "/repos/o/r/actions/runs/901/rerun-failed-jobs"
        ]
    );
    assert_eq!(requests_to(&server, "/repos/o/r/actions/jobs/13"), 0);
}

#[tokio::test]
async fn nothing_runs_again_when_nothing_failed() {
    let server = FakeServer::start().await;
    route_pull(&server);
    assert_eq!(
        client(&server).rerun_failed(&repo(), 14768).await.unwrap(),
        0
    );
    assert!(server.writes().is_empty());
}

#[tokio::test]
async fn a_pull_request_merges_with_the_strategy_asked() {
    let server = FakeServer::start().await;
    server.on(
        "PUT",
        "/repos/cli/cli/pulls/9/merge",
        200,
        json!({"merged": true}),
    );
    for (strategy, method) in [
        (MergeStrategy::Squash, "squash"),
        (MergeStrategy::Merge, "merge"),
        (MergeStrategy::Rebase, "rebase"),
    ] {
        client(&server).merge(&repo(), 9, strategy).await.unwrap();
        assert_eq!(
            server.writes().last().unwrap().json(),
            json!({"merge_method": method})
        );
    }
    server.on(
        "PUT",
        "/repos/cli/cli/pulls/10/merge",
        405,
        json!({"message": "Pull Request is not mergeable"}),
    );
    let e = client(&server)
        .merge(&repo(), 10, MergeStrategy::Merge)
        .await
        .unwrap_err();
    assert!(format!("{e:#}").contains("not mergeable"), "{e:#}");
}

#[tokio::test]
async fn auto_merge_is_enabled_through_graphql() {
    let server = FakeServer::start().await;
    server.on(
        "POST",
        "/graphql#PullRequestId",
        200,
        json!({"data": {"repository": {"pullRequest": {"id": "PR_kwDOabc"}}}}),
    );
    server.on(
        "POST",
        "/graphql#EnableAutoMerge",
        200,
        json!({"data": {"enablePullRequestAutoMerge": {"pullRequest": {
            "number": 9, "autoMergeRequest": {"enabledAt": "2026-10-22T08:00:00Z"}}}}}),
    );
    client(&server)
        .auto_merge(&repo(), 9, MergeStrategy::Squash)
        .await
        .unwrap();
    let sent = server.requests();
    assert_eq!(sent.len(), 2);
    let m = sent[1].json();
    assert!(m["query"]
        .as_str()
        .unwrap()
        .contains("enablePullRequestAutoMerge"));
    assert_eq!(
        m["variables"],
        json!({"id": "PR_kwDOabc", "method": "SQUASH"})
    );
}

#[tokio::test]
async fn auto_merge_says_when_the_repository_does_not_allow_it() {
    let server = FakeServer::start().await;
    server.on(
        "POST",
        "/graphql#PullRequestId",
        200,
        json!({"data": {"repository": {"pullRequest": {"id": "PR_1"}}}}),
    );
    server.on(
        "POST",
        "/graphql#EnableAutoMerge",
        200,
        json!({"data": null, "errors": [{"message": "Pull request Auto merge is not allowed for this repository"}]}),
    );
    let e = client(&server)
        .auto_merge(&repo(), 9, MergeStrategy::Merge)
        .await
        .unwrap_err();
    assert!(
        format!("{e:#}").contains("not allowed for this repository"),
        "{e:#}"
    );
}

#[tokio::test]
async fn reviews_keep_each_reviewers_last_word() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        "/repos/cli/cli/pulls/14768/reviews",
        200,
        fixture("reviews"),
    );
    let reviews = client(&server).reviews(&repo(), 14768).await.unwrap();
    assert_eq!(reviews.len(), 2);
    assert_eq!(reviews[0].author, "copilot-pull-request-reviewer[bot]");
    assert_eq!(reviews[0].state, ReviewState::Commented);
    assert_eq!(reviews[1].author, "williammartin");
    assert_eq!(reviews[1].state, ReviewState::Approved);
}

#[tokio::test]
async fn a_comment_does_not_take_back_a_request_for_changes_but_a_new_approval_does() {
    let server = FakeServer::start().await;
    let r = |who: &str, state: &str| json!({"user": {"login": who}, "state": state, "body": ""});
    server.on(
        "GET",
        "/repos/o/r/pulls/1/reviews",
        200,
        json!([
            r("ada", "CHANGES_REQUESTED"),
            r("ada", "COMMENTED"),
            r("bob", "PENDING"),
            r("bob", "APPROVED"),
            r("bob", "DISMISSED"),
            r("cy", "CHANGES_REQUESTED"),
            r("cy", "APPROVED")
        ]),
    );
    let repo = RepoRef {
        namespace: "o".into(),
        name: "r".into(),
    };
    let reviews = client(&server).reviews(&repo, 1).await.unwrap();
    let state = |who: &str| reviews.iter().find(|x| x.author == who).unwrap().state;
    assert_eq!(state("ada"), ReviewState::ChangesRequested);
    assert_eq!(state("bob"), ReviewState::Approved);
    assert_eq!(state("cy"), ReviewState::Approved);
}

#[tokio::test]
async fn line_comments_come_with_their_thread_and_the_conversation_beside() {
    let server = FakeServer::start().await;
    server.on("POST", "/graphql#Threads", 200, fixture("graphql-threads"));
    server.on(
        "GET",
        "/repos/cli/cli/issues/14768/comments",
        200,
        json!([{"id": 5, "user": {"login": "ada"}, "body": "Thanks!"}]),
    );
    let comments = client(&server).comments(&repo(), 14768).await.unwrap();
    assert_eq!(comments.len(), 2);
    assert_eq!(
        comments[0].path.as_deref(),
        Some("pkg/cmd/status/status.go")
    );
    assert_eq!(comments[0].line, Some(679));
    assert!(!comments[0].resolved);
    assert_eq!(comments[0].id, "4335329893");
    assert_eq!(comments[0].author, "copilot-pull-request-reviewer");
    assert_eq!(comments[1].path, None);
    assert_eq!(comments[1].body, "Thanks!");
}

#[tokio::test]
async fn a_validated_review_is_published_with_its_line_comments() {
    let server = FakeServer::start().await;
    server.on(
        "POST",
        "/repos/cli/cli/pulls/9/reviews",
        200,
        json!({"id": 1}),
    );
    client(&server)
        .review(
            &repo(),
            9,
            &ReviewDraft {
                state: ReviewState::ChangesRequested,
                body: "A few things.".into(),
                lines: vec![ReviewLine {
                    path: "src/a.rs".into(),
                    line: 12,
                    body: "Why?".into(),
                }],
            },
        )
        .await
        .unwrap();
    assert_eq!(
        server.writes()[0].json(),
        json!({"event": "REQUEST_CHANGES", "body": "A few things.",
               "comments": [{"path": "src/a.rs", "line": 12, "side": "RIGHT", "body": "Why?"}]})
    );
    client(&server)
        .review(
            &repo(),
            9,
            &ReviewDraft {
                state: ReviewState::Approved,
                body: String::new(),
                lines: vec![],
            },
        )
        .await
        .unwrap();
    assert_eq!(server.writes()[1].json()["event"], "APPROVE");
}

#[tokio::test]
async fn a_comment_and_an_answer_under_a_thread_are_published() {
    let server = FakeServer::start().await;
    server.on("POST", "/repos/cli/cli/issues/9/comments", 201, json!({}));
    server.on(
        "POST",
        "/repos/cli/cli/pulls/9/comments/4335329893/replies",
        201,
        json!({}),
    );
    client(&server)
        .comment(&repo(), 9, "Fixed in abc1234")
        .await
        .unwrap();
    client(&server)
        .reply(&repo(), 9, "4335329893", "Fixed in abc1234")
        .await
        .unwrap();
    let writes = server.writes();
    assert_eq!(writes[0].json(), json!({"body": "Fixed in abc1234"}));
    assert_eq!(
        writes[1].path(),
        "/repos/cli/cli/pulls/9/comments/4335329893/replies"
    );
}

#[tokio::test]
async fn deployments_come_with_their_last_status() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        "/repos/cli/cli/deployments?ref=trunk&per_page=10",
        200,
        fixture("deployments"),
    );
    for id in [7229954119u64, 7229755238] {
        server.on(
            "GET",
            &format!("/repos/cli/cli/deployments/{id}/statuses"),
            200,
            fixture("deployment-statuses"),
        );
    }
    let deployments = client(&server).deployments(&repo(), "trunk").await.unwrap();
    assert_eq!(deployments.len(), 2);
    assert_eq!(deployments[0].environment, "production");
    assert_eq!(deployments[0].state, "success");
    assert_eq!(
        deployments[0].url.as_deref(),
        Some("https://github.com/cli/cli/actions/runs/39624591320/job/119311177697")
    );
}

#[tokio::test]
async fn a_deployment_nobody_reported_on_is_pending() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        "/repos/o/r/deployments?ref=feat%2Fx&per_page=10",
        200,
        json!([{"id": 4, "environment": "staging"}]),
    );
    server.on("GET", "/repos/o/r/deployments/4/statuses", 200, json!([]));
    let repo = RepoRef {
        namespace: "o".into(),
        name: "r".into(),
    };
    let d = client(&server).deployments(&repo, "feat/x").await.unwrap();
    assert_eq!(d.len(), 1);
    assert_eq!(
        (
            d[0].environment.as_str(),
            d[0].state.as_str(),
            d[0].url.clone()
        ),
        ("staging", "pending", None)
    );
}

// ---- Sparing the quota ----

#[tokio::test]
async fn a_conditional_read_that_changed_nothing_is_served_from_what_was_kept() {
    let server = FakeServer::start().await;
    server.on_with(
        "GET",
        "/repos/cli/cli/pulls/14768",
        200,
        fixture("pull"),
        &[("ETag", "\"v1\"")],
    );
    server.on(
        "GET",
        "/repos/cli/cli/commits/6ec1ba54542baca6cc5932582634104c83f36a28/check-runs",
        200,
        fixture("check-runs"),
    );
    server.on(
        "GET",
        "/repos/cli/cli/commits/6ec1ba54542baca6cc5932582634104c83f36a28/status",
        200,
        fixture("status"),
    );
    let github = client(&server);
    let first = github.checks(&repo(), 14768).await.unwrap();

    // GitHub now answers 304 to a read that carries the tag: nothing changed.
    server.on_with(
        "GET",
        "/repos/cli/cli/pulls/14768",
        304,
        Value::Null,
        &[("ETag", "\"v1\"")],
    );
    let second = github.checks(&repo(), 14768).await.unwrap();
    assert_eq!(first, second, "the answer kept stands in for the 304");

    let pulls: Vec<_> = server
        .requests()
        .into_iter()
        .filter(|r| r.path() == "/repos/cli/cli/pulls/14768")
        .collect();
    assert_eq!(pulls.len(), 2);
    assert!(
        !pulls[0].headers.contains_key("if-none-match"),
        "never asked before"
    );
    assert_eq!(pulls[1].headers["if-none-match"], "\"v1\"");

    // And when it did change, the new answer replaces the old one.
    let mut moved = fixture("pull");
    moved["head"]["sha"] = json!("fffffff");
    server.on_with(
        "GET",
        "/repos/cli/cli/pulls/14768",
        200,
        moved,
        &[("ETag", "\"v2\"")],
    );
    server.on(
        "GET",
        "/repos/cli/cli/commits/fffffff/check-runs",
        200,
        json!({"check_runs": []}),
    );
    server.on(
        "GET",
        "/repos/cli/cli/commits/fffffff/status",
        200,
        json!({"statuses": []}),
    );
    assert!(github.checks(&repo(), 14768).await.unwrap().is_empty());
}

#[tokio::test]
async fn the_kept_answers_are_bounded_and_the_oldest_leaves_first() {
    let server = FakeServer::start().await;
    let github = client(&server);
    let repo = repo();
    // One address more than the cache keeps, each answered with a tag of its own.
    // What the client keeps (`pulls::CACHE_MAX`).
    let newest = 501;
    for n in 1..=newest {
        server.on_with(
            "GET",
            &format!("/repos/cli/cli/pulls/{n}/reviews?per_page=100"),
            200,
            json!([]),
            &[("ETag", &format!("\"{n}\""))],
        );
        github.reviews(&repo, n).await.unwrap();
    }
    // The newest and the one before it are still kept: their reads are conditional.
    for n in [newest, 2] {
        github.reviews(&repo, n).await.unwrap();
        let sent = server
            .requests()
            .last()
            .unwrap()
            .headers
            .get("if-none-match")
            .cloned();
        assert_eq!(sent, Some(format!("\"{n}\"")), "address {n} is kept");
    }
    // The oldest was let go: its read asks without the condition.
    github.reviews(&repo, 1).await.unwrap();
    let sent = server
        .requests()
        .last()
        .unwrap()
        .headers
        .get("if-none-match")
        .cloned();
    assert_eq!(sent, None, "the oldest answer was let go");
}

#[tokio::test]
async fn long_lists_are_followed_page_after_page() {
    let server = FakeServer::start().await;
    let next = format!(
        "<{}/repos/o/r/pulls/1/reviews?per_page=100&page=2>; rel=\"next\"",
        server.url
    );
    let r = |who: &str| json!({"user": {"login": who}, "state": "APPROVED", "body": ""});
    server.on_with(
        "GET",
        "/repos/o/r/pulls/1/reviews?per_page=100",
        200,
        json!([r("ada")]),
        &[("Link", &next)],
    );
    server.on(
        "GET",
        "/repos/o/r/pulls/1/reviews?per_page=100&page=2",
        200,
        json!([r("bob")]),
    );
    let repo = RepoRef {
        namespace: "o".into(),
        name: "r".into(),
    };
    let reviews = client(&server).reviews(&repo, 1).await.unwrap();
    assert_eq!(
        reviews
            .iter()
            .map(|r| r.author.as_str())
            .collect::<Vec<_>>(),
        ["ada", "bob"]
    );
}

fn limit(limit: u32, remaining: u32, reset: u64) -> RateLimit {
    RateLimit {
        limit,
        remaining,
        reset,
    }
}

#[test]
fn the_pace_is_free_while_the_limit_is_far_then_spreads_what_remains() {
    let pace = Pace {
        slow_below: 0.1,
        max_wait: Duration::from_secs(30),
    };
    // 600 of 5000 left: far from the limit.
    assert_eq!(pace.delay(&limit(5000, 600, 1_100), 1_000), Duration::ZERO);
    // 50 left over the 100 s that remain: a call every ~2 s.
    let d = pace.delay(&limit(5000, 49, 1_100), 1_000);
    assert_eq!(d, Duration::from_secs(2));
    // Fewer left, longer waits: 4 left over 100 s.
    assert!(pace.delay(&limit(5000, 4, 1_100), 1_000) > d);
    // None left: wait for the window to start over (at most `max_wait`).
    assert_eq!(
        pace.delay(&limit(5000, 0, 1_010), 1_000),
        Duration::from_secs(11)
    );
    assert_eq!(
        pace.delay(&limit(5000, 0, 9_000), 1_000),
        Duration::from_secs(30)
    );
    // The window is over: nothing to wait for.
    assert_eq!(pace.delay(&limit(5000, 3, 900), 1_000), Duration::ZERO);
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[tokio::test]
async fn an_enterprise_server_is_reached_under_its_api_v3_and_api_graphql() {
    let server = FakeServer::start().await;
    server.on("GET", "/api/v3/user", 200, json!({"login": "ada"}));
    server.on("POST", "/api/graphql#Pulls", 200, fixture("graphql-pulls"));
    server.on(
        "PUT",
        "/api/v3/repos/cli/cli/pulls/14768/merge",
        200,
        json!({"merged": true}),
    );
    // The account's address is the instance's, no API address given.
    let github = GithubProvider.client(
        reqwest::Client::new(),
        &Credentials {
            url: server.url.clone(),
            token: "t".into(),
            ..Default::default()
        },
    );
    assert_eq!(github.me().await.unwrap().user, "ada");
    assert_eq!(
        github.pull_request(&repo(), 14768).await.unwrap().number,
        14768
    );
    github
        .merge(&repo(), 14768, MergeStrategy::Merge)
        .await
        .unwrap();
    assert_eq!(server.requests().len(), 3);
}

#[tokio::test]
async fn an_expired_token_is_a_refusal_the_account_can_tell() {
    let server = FakeServer::start().await;
    server.on(
        "POST",
        "/graphql#Pulls",
        401,
        json!({"message": "Bad credentials"}),
    );
    let e = client(&server).pull_request(&repo(), 1).await.unwrap_err();
    assert!(refused(&e), "{e:#}");
}

// ---- The real github.com, read only ----

/// Reads a merged pull request of a public repository on github.com with the GitHub CLI's token:
/// `ESCOUADE_TEST_REAL_GITHUB=1 cargo test --lib real_github_reads -- --ignored`. Nothing is
/// written.
#[tokio::test]
#[ignore = "talks to github.com"]
async fn real_github_reads() {
    if std::env::var("ESCOUADE_TEST_REAL_GITHUB").is_err() {
        return;
    }
    let token = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
        .unwrap();
    let token = String::from_utf8(token.stdout).unwrap();
    // github.com's account, served by the plugin as the core serves it.
    let github = GithubProvider.client(
        reqwest::Client::new(),
        &Credentials {
            url: "https://github.com".into(),
            token,
            api: Some("https://api.github.com".into()),
            ..Default::default()
        },
    );
    let repo = repo();
    let me = github.me().await.unwrap();
    println!("me: {me:?}");
    let prs = github
        .pull_requests(&repo, &[14768, 14770, 1])
        .await
        .unwrap();
    println!("prs: {prs:#?}");
    assert!(prs
        .iter()
        .any(|p| p.number == 14768 && p.state == PrState::Merged));
    let by_branch = github
        .pull_requests_of_branch(&repo, "babakks/refresh-token-bump-deps")
        .await
        .unwrap();
    assert_eq!(by_branch[0].number, 14768);
    let checks = github.checks(&repo, 14768).await.unwrap();
    println!("checks: {}", checks.len());
    assert!(!checks.is_empty());
    let job = checks.iter().find_map(|c| c.job.clone()).unwrap();
    let log = github.job_log(&repo, &job).await.unwrap();
    assert!(
        log.contains("runner version"),
        "{}",
        &log[..200.min(log.len())]
    );
    println!("reviews: {:?}", github.reviews(&repo, 14768).await.unwrap());
    println!(
        "comments: {}",
        github.comments(&repo, 14768).await.unwrap().len()
    );
    println!(
        "deployments: {:?}",
        github.deployments(&repo, "trunk").await.unwrap().first()
    );
    // Asked twice: the second read is conditional, a 304 for GitHub.
    github.checks(&repo, 14768).await.unwrap();
    println!("rate limit: {:?}", github.rate_limit());
}

#[tokio::test]
async fn the_environments_are_read_down_to_their_newest_success() {
    let server = FakeServer::start().await;
    let dep = |id: u64, env: &str, sha: &str| {
        json!({
            "id": id, "environment": env, "sha": sha, "ref": "main",
            "created_at": "2026-10-30T10:00:00Z", "creator": {"login": "ada"}
        })
    };
    let mut preview = dep(5, "Preview", "eeee");
    preview["transient_environment"] = json!(true);
    server.on(
        "GET",
        "/repos/o/r/deployments?per_page=30",
        200,
        json!([
            preview,
            dep(4, "production", "dddd"),
            dep(3, "staging", "cccc"),
            dep(2, "production", "bbbb"),
            dep(1, "production", "aaaa"),
        ]),
    );
    let status = |state: &str, env_url: &str| {
        json!([{
            "state": state, "environment_url": env_url,
            "log_url": "https://github.com/o/r/actions/runs/1/job/2"
        }])
    };
    server.on(
        "GET",
        "/repos/o/r/deployments/4/statuses",
        200,
        status("failure", ""),
    );
    server.on(
        "GET",
        "/repos/o/r/deployments/3/statuses",
        200,
        status("success", "https://staging.example.com"),
    );
    server.on(
        "GET",
        "/repos/o/r/deployments/2/statuses",
        200,
        status("success", "https://example.com"),
    );
    let repo = RepoRef {
        namespace: "o".into(),
        name: "r".into(),
    };
    let d = client(&server).environments(&repo).await.unwrap();
    assert_eq!(
        d.iter()
            .map(|d| (d.environment.as_str(), d.state.as_str(), d.sha.as_str()))
            .collect::<Vec<_>>(),
        [
            ("production", "failure", "dddd"),
            ("staging", "success", "cccc"),
            ("production", "success", "bbbb"),
        ],
        "a preview left out, and nothing read past production's newest success"
    );
    assert_eq!(d[1].url.as_deref(), Some("https://staging.example.com"));
    assert_eq!(d[0].url, None);
    assert_eq!(
        d[0].log.as_deref(),
        Some("https://github.com/o/r/actions/runs/1/job/2")
    );
    assert_eq!(d[0].author, "ada");
    assert!(d[0].created > 0);
    assert!(server
        .requests()
        .iter()
        .all(|r| !r.path().starts_with("/repos/o/r/deployments/1/")));
}

/// A branch's checks (the target's, one pushed without a pull request, ESC-63): those of its last
/// commit, read by its name; its failed jobs run again once per run.
#[tokio::test]
async fn a_branchs_checks_are_read_by_its_name_and_its_failed_jobs_run_again() {
    let server = FakeServer::start().await;
    let base = "/repos/o/r/commits/feat%2Fx";
    server.on(
        "GET",
        &format!("{base}/check-runs"),
        200,
        json!({ "total_count": 2, "check_runs": [
            { "id": 7, "name": "test", "status": "completed", "conclusion": "failure",
              "app": { "slug": "github-actions" } },
            { "id": 8, "name": "lint", "status": "completed", "conclusion": "success",
              "app": { "slug": "github-actions" } },
        ] }),
    );
    server.on(
        "GET",
        &format!("{base}/status"),
        200,
        json!({ "statuses": [
            { "context": "ci/jenkins", "state": "failure", "target_url": "https://ci.example.com/1" },
        ] }),
    );
    let repo = RepoRef {
        namespace: "o".into(),
        name: "r".into(),
    };
    let checks = client(&server)
        .branch_checks(&repo, "feat/x")
        .await
        .unwrap();
    assert_eq!(
        checks
            .iter()
            .map(|c| (c.name.as_str(), c.state, c.job.as_deref()))
            .collect::<Vec<_>>(),
        [
            ("test", CheckState::Failure, Some("7")),
            ("lint", CheckState::Success, Some("8")),
            ("ci/jenkins", CheckState::Failure, None),
        ]
    );
    server.on(
        "GET",
        "/repos/o/r/actions/jobs/7",
        200,
        json!({ "id": 7, "run_id": 70 }),
    );
    server.on(
        "POST",
        "/repos/o/r/actions/runs/70/rerun-failed-jobs",
        201,
        json!({}),
    );
    assert_eq!(
        client(&server)
            .rerun_failed_of_branch(&repo, "feat/x")
            .await
            .unwrap(),
        1
    );
    assert_eq!(server.writes().len(), 1);
}

/// How a pull request may merge (ESC-62): the repository's merge methods and « Allow auto-merge »;
/// not told (no right to push), the three.
#[tokio::test]
async fn the_repositorys_settings_say_how_a_pull_request_may_merge() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        "/repos/cli/cli",
        200,
        json!({ "allow_merge_commit": false, "allow_squash_merge": true, "allow_rebase_merge": true, "allow_auto_merge": true }),
    );
    let o = client(&server).merge_options(&repo(), 7).await.unwrap();
    assert_eq!(
        o.strategies,
        vec![MergeStrategy::Squash, MergeStrategy::Rebase]
    );
    assert!(o.auto_merge);
}
