//! What only the client itself shows, its process and its module hiding it: its pace, what it waits, the limits it keeps. The
//! rest of its tests play its module through the app: `src-tauri/src/forge/github_tests.rs`.

use super::*;
use crate::fake::FakeServer;
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
fn repo() -> RepoRef {
    RepoRef {
        namespace: "cli".into(),
        name: "cli".into(),
    }
}
fn requests_to(server: &FakeServer, path: &str) -> usize {
    server
        .requests()
        .iter()
        .filter(|r| r.path() == path)
        .count()
}
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
#[tokio::test]
async fn the_client_slows_down_near_the_limit_and_says_how_much_is_left() {
    let server = FakeServer::start().await;
    let reset = (now_secs() + 3).to_string();
    server.on_with(
        "GET",
        "/user",
        200,
        json!({"login": "ada"}),
        &[
            ("x-ratelimit-limit", "5000"),
            ("x-ratelimit-remaining", "2"),
            ("x-ratelimit-reset", &reset),
            ("x-ratelimit-resource", "core"),
        ],
    );
    let github =
        Github::new(reqwest::Client::new(), server.url.clone(), "t".into()).with_pace(Pace {
            slow_below: 0.1,
            max_wait: Duration::from_millis(300),
        });
    assert!(github.rate_limit().is_none());
    github.me().await.unwrap();
    assert_eq!(
        github.waited(),
        Duration::ZERO,
        "the first call knows nothing yet"
    );
    assert_eq!(
        github.rate_limit().map(|l| (l.limit, l.remaining)),
        Some((5000, 2))
    );

    let before = std::time::Instant::now();
    github.me().await.unwrap();
    assert!(
        before.elapsed() >= Duration::from_millis(250),
        "slowed down: {:?}",
        before.elapsed()
    );
    assert!(github.waited() >= Duration::from_millis(250));
}
#[tokio::test]
async fn far_from_the_limit_the_client_does_not_wait() {
    let server = FakeServer::start().await;
    let reset = (now_secs() + 3600).to_string();
    server.on_with(
        "GET",
        "/user",
        200,
        json!({"login": "ada"}),
        &[
            ("x-ratelimit-limit", "5000"),
            ("x-ratelimit-remaining", "4900"),
            ("x-ratelimit-reset", &reset),
        ],
    );
    let github = Github::new(reqwest::Client::new(), server.url.clone(), "t".into());
    for _ in 0..3 {
        github.me().await.unwrap();
    }
    assert_eq!(github.waited(), Duration::ZERO);
}
#[tokio::test]
async fn a_secondary_limit_is_waited_out_then_asked_once_more() {
    let server = FakeServer::start().await;
    // Answers every call, and GitHub refuses the first one for its secondary limit.
    server.on("GET", "/user", 200, json!({"login": "ada"}));
    server.on_once(
        "GET",
        "/user",
        429,
        json!({"message": "You have exceeded a secondary rate limit."}),
        &[("retry-after", "30")],
    );
    let github =
        Github::new(reqwest::Client::new(), server.url.clone(), "t".into()).with_pace(Pace {
            slow_below: 0.1,
            max_wait: Duration::from_millis(300),
        });
    let before = std::time::Instant::now();
    assert_eq!(github.me().await.unwrap().user, "ada");
    let elapsed = before.elapsed();
    // Thirty seconds asked, at most `max_wait` waited.
    assert!(elapsed >= Duration::from_millis(250), "waited: {elapsed:?}");
    assert!(elapsed < Duration::from_secs(10), "waited: {elapsed:?}");
    assert!(github.waited() >= Duration::from_millis(250));
    assert_eq!(requests_to(&server, "/user"), 2, "asked once more, no more");
}
#[tokio::test]
async fn a_secondary_limit_that_holds_is_said_after_one_more_try() {
    let server = FakeServer::start().await;
    // No `retry-after` here: GitHub's message is what names the secondary limit.
    server.on(
        "GET",
        "/user",
        403,
        json!({"message": "You have exceeded a secondary rate limit. Please wait a few minutes."}),
    );
    let github =
        Github::new(reqwest::Client::new(), server.url.clone(), "t".into()).with_pace(Pace {
            slow_below: 0.1,
            max_wait: Duration::from_millis(100),
        });
    let e = github.me().await.expect_err("refused");
    assert!(format!("{e:#}").contains("403"), "{e:#}");
    assert_eq!(
        requests_to(&server, "/user"),
        2,
        "one retry, then it is said"
    );
}
#[tokio::test]
async fn another_refusal_is_not_asked_again() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        "/user",
        403,
        json!({"message": "Resource not accessible by integration"}),
    );
    let github = Github::new(reqwest::Client::new(), server.url.clone(), "t".into());
    assert!(github.me().await.is_err());
    assert_eq!(requests_to(&server, "/user"), 1);
    assert_eq!(github.waited(), Duration::ZERO);
}
#[tokio::test]
async fn graphql_and_rest_have_their_own_limits() {
    let server = FakeServer::start().await;
    let reset = (now_secs() + 3600).to_string();
    // REST nearly spent, GraphQL fresh: a GraphQL call must not wait for REST's window.
    server.on_with(
        "GET",
        "/user",
        200,
        json!({"login": "ada"}),
        &[
            ("x-ratelimit-limit", "5000"),
            ("x-ratelimit-remaining", "1"),
            ("x-ratelimit-reset", &reset),
            ("x-ratelimit-resource", "core"),
        ],
    );
    server.on_with(
        "POST",
        "/graphql#Pulls",
        200,
        fixture("graphql-pulls"),
        &[
            ("x-ratelimit-limit", "5000"),
            ("x-ratelimit-remaining", "4990"),
            ("x-ratelimit-reset", &reset),
            ("x-ratelimit-resource", "graphql"),
        ],
    );
    let github =
        Github::new(reqwest::Client::new(), server.url.clone(), "t".into()).with_pace(Pace {
            slow_below: 0.1,
            max_wait: Duration::from_millis(200),
        });
    github.me().await.unwrap();
    github.pull_requests(&repo(), &[14768]).await.unwrap();
    github.pull_requests(&repo(), &[14768]).await.unwrap();
    assert_eq!(github.waited(), Duration::ZERO);
    // The most spent of them is what it reports.
    assert_eq!(github.rate_limit().map(|l| l.remaining), Some(1));
}
// ---- Instances ----

#[tokio::test]
async fn a_next_page_on_another_host_is_not_read() {
    let server = FakeServer::start().await;
    let other = FakeServer::start().await;
    let next = format!("<{}/items?page=2>; rel=\"next\"", other.url);
    server.on_with("GET", "/items", 200, json!([1]), &[("Link", &next)]);
    other.on("GET", "/items?page=2", 200, json!([2]));
    let github = Github::new(reqwest::Client::new(), server.url.clone(), "t".into());
    assert_eq!(github.list("/items").await.unwrap(), [json!(1)]);
    assert!(other.requests().is_empty(), "the token went there");
}

#[tokio::test]
async fn a_next_page_of_the_api_is_read() {
    let server = FakeServer::start().await;
    let next = format!("<{}/items?page=2>; rel=\"next\"", server.url);
    server.on_with("GET", "/items", 200, json!([1]), &[("Link", &next)]);
    server.on("GET", "/items?page=2", 200, json!([2]));
    let github = Github::new(reqwest::Client::new(), server.url.clone(), "t".into());
    assert_eq!(github.list("/items").await.unwrap(), [json!(1), json!(2)]);
}

#[test]
fn graphql_sits_beside_rest_on_every_kind_of_instance() {
    assert_eq!(
        graphql_url("https://api.github.com"),
        "https://api.github.com/graphql"
    );
    assert_eq!(
        graphql_url("https://api.acme.ghe.com"),
        "https://api.acme.ghe.com/graphql"
    );
    assert_eq!(
        graphql_url("https://git.acme.dev/api/v3"),
        "https://git.acme.dev/api/graphql"
    );
}

#[test]
fn a_repository_that_says_nothing_has_every_strategy_and_no_auto_merge() {
    let untold = merge_options_of(&json!({ "name": "cli" }));
    assert_eq!(untold.strategies.len(), 3);
    assert!(!untold.auto_merge);
}

// A pull request read whole (ESC-452).

#[test]
fn a_description_says_its_text_its_commits_and_when_it_moved() {
    let d = description_of(&json!({
        "body": "Ajoute la connexion par lien.",
        "updated_at": "2026-11-14T10:00:00Z",
        "head": { "sha": "abc" }, "base": { "sha": "def" },
        "labels": [{ "name": "auth" }, { "name": "ux" }],
        "additions": 12, "deletions": 3, "changed_files": 2, "commits": 4,
    }));
    assert_eq!(d.body, "Ajoute la connexion par lien.");
    assert_eq!((d.head_sha.as_str(), d.base_sha.as_str()), ("abc", "def"));
    assert_eq!(d.updated, 1_794_650_400);
    assert_eq!(d.labels, ["auth", "ux"]);
    assert_eq!(
        (d.additions, d.deletions, d.changed_files, d.commits),
        (12, 3, 2, 4)
    );
    // A pull request without a text: GitHub's null.
    assert_eq!(description_of(&json!({ "body": null })).body, "");
}

#[test]
fn a_commit_is_its_authors_login_else_the_name_git_keeps() {
    let c = commit_of(&json!({
        "sha": "a1", "html_url": "https://github.com/cli/cli/commit/a1",
        "author": { "login": "ada" },
        "commit": { "message": "feat: lien\n\nLe corps.", "author": { "name": "Ada L.", "date": "2026-11-14T09:00:00Z" } },
    }));
    assert_eq!((c.sha.as_str(), c.author.as_str()), ("a1", "ada"));
    assert_eq!(c.message, "feat: lien\n\nLe corps.");
    assert_eq!(c.created, 1_794_646_800);
    let unknown = commit_of(
        &json!({ "sha": "b2", "author": null, "commit": { "message": "x", "author": { "name": "Bob" } } }),
    );
    assert_eq!(unknown.author, "Bob");
}

#[test]
fn a_files_status_is_one_of_five() {
    let f = file_of(
        &json!({ "filename": "src/b.rs", "previous_filename": "src/a.rs", "status": "renamed", "additions": 1, "deletions": 0 }),
    );
    assert_eq!(f.status, FileStatus::Renamed);
    assert_eq!(f.previous_path.as_deref(), Some("src/a.rs"));
    for (said, status) in [
        ("added", FileStatus::Added),
        ("removed", FileStatus::Removed),
        ("copied", FileStatus::Copied),
        ("modified", FileStatus::Modified),
        ("changed", FileStatus::Modified),
    ] {
        assert_eq!(
            file_of(&json!({ "filename": "x", "status": said })).status,
            status,
            "{said}"
        );
    }
}

#[test]
fn threads_come_with_their_lines_their_answers_and_their_state() {
    let data = json!({ "repository": { "pullRequest": { "reviewThreads": { "nodes": [
        { "id": "PRRT_1", "isResolved": false, "isOutdated": false, "path": "src/app.ts", "line": 12, "startLine": 10,
          "originalLine": 12, "diffSide": "RIGHT",
          "comments": { "nodes": [
            { "databaseId": 1, "author": { "login": "bob" }, "body": "Ce nom est peu clair.", "createdAt": "2026-11-14T09:00:00Z",
              "lastEditedAt": null, "diffHunk": "@@ -1 +1 @@\n+export const x = 1;" },
            { "databaseId": 2, "author": { "login": "ada" }, "body": "Renommé.", "createdAt": "2026-11-14T10:00:00Z",
              "lastEditedAt": "2026-11-14T10:05:00Z", "diffHunk": "" },
          ] } },
        { "id": "PRRT_2", "isResolved": true, "isOutdated": true, "path": "src/a.ts", "line": null, "originalLine": 3,
          "diffSide": "LEFT", "comments": { "nodes": [{ "databaseId": 3, "author": null, "body": "Réglé." }] } },
        { "id": "PRRT_3", "isResolved": false, "isOutdated": true, "path": "src/a.ts", "line": null, "originalLine": 5,
          "comments": { "nodes": [] } },
    ] } } } });
    let t = threads_of(&data);
    assert_eq!(t.len(), 3);
    assert_eq!(t[0].id, "PRRT_1");
    assert_eq!(
        (t[0].path.as_deref(), t[0].line, t[0].start_line),
        (Some("src/app.ts"), Some(12), Some(10))
    );
    assert_eq!(t[0].state, ThreadState::Open);
    assert_eq!(t[0].side, DiffSide::Right);
    assert_eq!(t[0].comments.len(), 2, "its answer under it");
    assert_eq!(t[0].comments[1].author, "ada");
    assert_ne!(t[0].comments[1].updated, 0, "edited");
    assert_eq!(t[0].comments[0].updated, 0);
    assert_eq!(
        t[0].excerpt.as_deref(),
        Some("@@ -1 +1 @@\n+export const x = 1;")
    );
    // Resolved says it first, outdated next.
    assert_eq!(
        (t[1].state, t[1].side, t[1].line, t[1].original_line),
        (ThreadState::Resolved, DiffSide::Left, None, Some(3))
    );
    assert_eq!(t[1].comments[0].author, "?", "a user since deleted");
    assert_eq!(t[2].state, ThreadState::Outdated);
    assert!(threads_of(&json!({ "repository": { "pullRequest": null } })).is_empty());
}

#[test]
fn a_comment_of_the_conversation_is_a_thread_without_a_path() {
    let t = conversation_of(&json!({
        "id": 9, "user": { "login": "bob" }, "body": "Merci !",
        "created_at": "2026-11-14T09:00:00Z", "updated_at": "2026-11-14T09:00:00Z",
    }));
    assert!(!t.on_lines());
    assert_eq!(t.state, ThreadState::Open);
    assert_eq!(
        (t.comments[0].id.as_str(), t.comments[0].author.as_str()),
        ("9", "bob")
    );
    assert_eq!(t.comments[0].updated, 0, "never edited");
}

#[tokio::test]
async fn the_four_reads_ask_github_where_they_should() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        "/repos/cli/cli/pulls/7",
        200,
        json!({ "body": "Texte", "head": { "sha": "abc" } }),
    );
    server.on(
        "GET",
        "/repos/cli/cli/pulls/7/commits",
        200,
        json!([{ "sha": "a1", "author": { "login": "ada" }, "commit": { "message": "feat: x" } }]),
    );
    server.on(
        "GET",
        "/repos/cli/cli/pulls/7/files",
        200,
        json!([{ "filename": "a.ts", "status": "added", "additions": 2, "deletions": 0 }]),
    );
    server.on(
        "POST",
        "/graphql#PrThreads",
        200,
        json!({ "data": { "repository": { "pullRequest": { "reviewThreads": { "nodes": [
            { "id": "T", "isResolved": false, "isOutdated": false, "path": "a.ts", "line": 1, "comments": { "nodes": [] } },
        ] } } } } }),
    );
    server.on(
        "GET",
        "/repos/cli/cli/issues/7/comments",
        200,
        json!([{ "id": 4, "user": { "login": "bob" }, "body": "LGTM" }]),
    );
    let github = Github::new(reqwest::Client::new(), server.url.clone(), "t".into());
    assert_eq!(
        github.description(&repo(), 7).await.unwrap().head_sha,
        "abc"
    );
    assert_eq!(github.commits(&repo(), 7).await.unwrap()[0].sha, "a1");
    assert_eq!(
        github.files(&repo(), 7).await.unwrap()[0].status,
        FileStatus::Added
    );
    let threads = github.threads(&repo(), 7).await.unwrap();
    assert_eq!(
        threads.len(),
        2,
        "the thread on its line, then the conversation"
    );
    assert!(threads[0].on_lines() && !threads[1].on_lines());
    let asked: Vec<String> = server.requests().iter().map(|r| r.target.clone()).collect();
    assert!(
        asked
            .iter()
            .any(|t| t == "/repos/cli/cli/pulls/7/commits?per_page=100"),
        "{asked:?}"
    );
    assert!(
        asked
            .iter()
            .any(|t| t == "/repos/cli/cli/pulls/7/files?per_page=100"),
        "{asked:?}"
    );
    // Nothing written: a read is a read.
    assert!(server.writes().iter().all(|r| r.path() == "/graphql"));
}

#[tokio::test]
async fn a_line_comment_a_resolution_and_a_reopening_ask_github_where_they_should() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        "/repos/cli/cli/pulls/7",
        200,
        json!({ "head": { "sha": "tip" } }),
    );
    server.on(
        "POST",
        "/repos/cli/cli/pulls/7/comments",
        201,
        json!({ "id": 9 }),
    );
    server.on(
        "POST",
        "/graphql#ResolveThread",
        200,
        json!({ "data": { "resolveReviewThread": { "thread": { "id": "T", "isResolved": true } } } }),
    );
    server.on(
        "POST",
        "/graphql#UnresolveThread",
        200,
        json!({ "data": { "unresolveReviewThread": { "thread": { "id": "T", "isResolved": false } } } }),
    );
    let github = Github::new(reqwest::Client::new(), server.url.clone(), "t".into());
    // A range of lines removed, on the tip read.
    let range = LineComment {
        path: "a.ts".into(),
        line: 12,
        start_line: Some(10),
        side: DiffSide::Left,
        commit: "abc".into(),
        body: "Pourquoi ?".into(),
    };
    github.line_comment(&repo(), 7, &range).await.unwrap();
    // One line, on the tip now.
    let one = LineComment {
        path: "a.ts".into(),
        line: 3,
        body: "Ici".into(),
        ..Default::default()
    };
    github.line_comment(&repo(), 7, &one).await.unwrap();
    let posted: Vec<Value> = server
        .writes()
        .iter()
        .filter(|r| r.path() == "/repos/cli/cli/pulls/7/comments")
        .map(|r| r.json())
        .collect();
    assert_eq!(posted[0]["commit_id"], "abc");
    assert_eq!(
        (
            &posted[0]["line"],
            &posted[0]["start_line"],
            &posted[0]["side"],
            &posted[0]["start_side"]
        ),
        (&json!(12), &json!(10), &json!("LEFT"), &json!("LEFT"))
    );
    assert_eq!(
        posted[1]["commit_id"], "tip",
        "the tip now, asked of GitHub"
    );
    assert_eq!(posted[1]["side"], "RIGHT");
    assert!(posted[1].get("start_line").is_none());

    github.resolve(&repo(), 7, "T").await.unwrap();
    github.unresolve(&repo(), 7, "T").await.unwrap();
    let mutations: Vec<Value> = server
        .writes()
        .iter()
        .filter(|r| r.path() == "/graphql")
        .map(|r| r.json())
        .collect();
    assert_eq!(mutations.len(), 2);
    assert_eq!(mutations[0]["variables"]["id"], "T");
    assert!(mutations[0]["query"]
        .as_str()
        .unwrap()
        .contains("resolveReviewThread"));
    assert!(mutations[1]["query"]
        .as_str()
        .unwrap()
        .contains("unresolveReviewThread"));
}

#[tokio::test]
async fn a_thread_deleted_since_it_was_read_is_a_404() {
    let server = FakeServer::start().await;
    server.on(
        "POST",
        "/graphql#ResolveThread",
        200,
        json!({ "data": { "resolveReviewThread": null }, "errors": [{ "type": "NOT_FOUND", "message": "Could not resolve to a node with the global id of 'T'" }] }),
    );
    let github = Github::new(reqwest::Client::new(), server.url.clone(), "t".into());
    let e = github.resolve(&repo(), 7, "T").await.unwrap_err();
    assert_eq!(
        e.downcast_ref::<escouade_plugin_kit::errors::HttpError>()
            .map(|h| h.status),
        Some(404),
        "{e:#}"
    );
}

#[test]
fn a_line_comment_on_one_line_says_no_first_line() {
    let c = LineComment {
        path: "a".into(),
        line: 4,
        start_line: Some(4),
        body: "x".into(),
        ..Default::default()
    };
    let b = line_comment_body(&c, "s");
    assert!(b.get("start_line").is_none(), "{b}");
    assert_eq!(b["side"], "RIGHT");
}

#[tokio::test]
async fn the_control_of_a_pull_request_reads_its_state_its_labels_and_its_lag() {
    let server = FakeServer::start().await;
    server.on(
        "POST",
        "/graphql#PrControl",
        200,
        json!({ "data": { "repository": { "deleteBranchOnMerge": true, "pullRequest": {
            "id": "PR_7", "mergeStateStatus": "BLOCKED", "reviewDecision": "REVIEW_REQUIRED",
            "baseRefOid": "b1", "headRefOid": "h1",
            "labels": { "nodes": [{ "name": "bug" }, { "name": "<b>x</b>" }] },
            "autoMergeRequest": { "mergeMethod": "SQUASH" } } } } }),
    );
    server.on(
        "GET",
        "/repos/cli/cli/compare/b1...h1",
        200,
        json!({ "behind_by": 3, "ahead_by": 1 }),
    );
    let github = Github::new(reqwest::Client::new(), server.url.clone(), "t".into());
    let c = github.control(&repo(), 7).await.unwrap();
    assert_eq!(c.merge_state, MergeState::Blocked);
    assert_eq!(c.behind, Some(true), "the comparison says it");
    assert_eq!(c.review_decision, Some(ReviewDecision::ReviewRequired));
    assert_eq!(c.labels, vec!["bug".to_string(), "<b>x</b>".to_string()]);
    assert_eq!(c.auto_merge, Some(MergeStrategy::Squash));
    assert!(c.deletes_branch);
    assert_eq!(c.id, "PR_7");
    assert!(server.writes().iter().all(|r| r.path() == "/graphql"));
}

#[tokio::test]
async fn the_candidates_are_the_assignable_people_and_the_labels() {
    let server = FakeServer::start().await;
    server.on(
        "POST",
        "/graphql#PrCandidates",
        200,
        json!({ "data": { "repository": {
            "assignableUsers": { "nodes": [{ "login": "marta" }, { "login": "bob" }] },
            "labels": { "nodes": [{ "name": "bug", "color": "d73a4a", "description": "Ça casse" }, { "name": "doc", "color": null }] } } } }),
    );
    let github = Github::new(reqwest::Client::new(), server.url.clone(), "t".into());
    let k = github.candidates(&repo()).await.unwrap();
    assert_eq!(k.reviewers, vec!["marta".to_string(), "bob".to_string()]);
    assert_eq!(k.labels[0].color, "d73a4a");
    assert_eq!(k.labels[1].color, "");
}

#[tokio::test]
async fn the_controls_write_where_github_takes_them() {
    let server = FakeServer::start().await;
    server.on(
        "POST",
        "/graphql#PullRequestId",
        200,
        json!({ "data": { "repository": { "pullRequest": { "id": "PR_7" } } } }),
    );
    server.on(
        "POST",
        "/graphql#MarkReady",
        200,
        json!({ "data": { "markPullRequestReadyForReview": { "pullRequest": { "isDraft": false } } } }),
    );
    server.on(
        "POST",
        "/graphql#ConvertToDraft",
        200,
        json!({ "data": { "convertPullRequestToDraft": { "pullRequest": { "isDraft": true } } } }),
    );
    server.on(
        "POST",
        "/graphql#UpdateBranch",
        200,
        json!({ "data": { "updatePullRequestBranch": { "pullRequest": { "number": 7 } } } }),
    );
    server.on(
        "POST",
        "/graphql#DisableAutoMerge",
        200,
        json!({ "data": { "disablePullRequestAutoMerge": { "pullRequest": { "number": 7 } } } }),
    );
    for (method, path) in [
        ("POST", "/repos/cli/cli/pulls/7/requested_reviewers"),
        ("DELETE", "/repos/cli/cli/pulls/7/requested_reviewers"),
        ("POST", "/repos/cli/cli/issues/7/labels"),
        ("DELETE", "/repos/cli/cli/issues/7/labels/needs%20review"),
        ("POST", "/repos/cli/cli/actions/jobs/42/rerun"),
        ("DELETE", "/repos/cli/cli/git/refs/heads/ticket/esc-1"),
    ] {
        server.on(method, path, 200, json!({}));
    }
    let github = Github::new(reqwest::Client::new(), server.url.clone(), "t".into());
    let r = repo();
    github.set_draft(&r, 7, false).await.unwrap();
    github.set_draft(&r, 7, true).await.unwrap();
    github
        .request_reviewers(&r, 7, &["marta".to_string()])
        .await
        .unwrap();
    github.remove_reviewer(&r, 7, "bob").await.unwrap();
    github
        .add_labels(&r, 7, &["bug".to_string()])
        .await
        .unwrap();
    github.remove_label(&r, 7, "needs review").await.unwrap();
    github
        .update_branch(&r, 7, UpdateMethod::Rebase)
        .await
        .unwrap();
    github.rerun_check(&r, "42").await.unwrap();
    github.cancel_auto_merge(&r, 7).await.unwrap();
    github.delete_branch(&r, "ticket/esc-1").await.unwrap();
    let writes = server.writes();
    let body_of = |path: &str, method: &str| {
        writes
            .iter()
            .find(|w| w.path() == path && w.method == method)
            .map(|w| w.json())
            .unwrap_or_else(|| panic!("no {method} {path}"))
    };
    assert_eq!(
        body_of("/repos/cli/cli/pulls/7/requested_reviewers", "POST")["reviewers"],
        json!(["marta"])
    );
    assert_eq!(
        body_of("/repos/cli/cli/pulls/7/requested_reviewers", "DELETE")["reviewers"],
        json!(["bob"])
    );
    assert_eq!(
        body_of("/repos/cli/cli/issues/7/labels", "POST")["labels"],
        json!(["bug"])
    );
    let graphql: Vec<Value> = writes
        .iter()
        .filter(|w| w.path() == "/graphql")
        .map(|w| w.json())
        .filter(|b| b["operationName"] != "PullRequestId")
        .collect();
    let ops: Vec<&str> = graphql
        .iter()
        .map(|b| b["operationName"].as_str().unwrap())
        .collect();
    assert_eq!(
        ops,
        vec![
            "MarkReady",
            "ConvertToDraft",
            "UpdateBranch",
            "DisableAutoMerge"
        ]
    );
    assert_eq!(
        graphql[2]["variables"],
        json!({ "id": "PR_7", "method": "REBASE" })
    );
    for path in [
        "/repos/cli/cli/issues/7/labels/needs%20review",
        "/repos/cli/cli/actions/jobs/42/rerun",
        "/repos/cli/cli/git/refs/heads/ticket/esc-1",
    ] {
        assert_eq!(requests_to(&server, path), 1, "{path}");
    }
}

#[tokio::test]
async fn a_graphql_refusal_says_its_status() {
    let server = FakeServer::start().await;
    server.on(
        "POST",
        "/graphql#PullRequestId",
        200,
        json!({ "data": { "repository": { "pullRequest": { "id": "PR_7" } } } }),
    );
    server.on(
        "POST",
        "/graphql#UpdateBranch",
        200,
        json!({ "data": { "updatePullRequestBranch": null }, "errors": [{ "type": "FORBIDDEN", "message": "Resource not accessible by integration" }] }),
    );
    server.on(
        "POST",
        "/graphql#MarkReady",
        200,
        json!({ "data": null, "errors": [{ "message": "merge conflict between base and head" }] }),
    );
    let github = Github::new(reqwest::Client::new(), server.url.clone(), "t".into());
    let status = |e: anyhow::Error| {
        e.downcast_ref::<escouade_plugin_kit::errors::HttpError>()
            .map(|h| h.status)
    };
    let e = github
        .update_branch(&repo(), 7, UpdateMethod::Merge)
        .await
        .unwrap_err();
    assert_eq!(status(e), Some(403));
    let e = github.set_draft(&repo(), 7, false).await.unwrap_err();
    assert_eq!(status(e), Some(422));
    assert!(graphql_refusal(&json!({ "data": {} })).is_none());
}

/// « Ta relecture » (ESC-494): one review of GitHub's for the text, the verdict and the comments on lines, each on its
/// side and its range, on the commit read.
#[tokio::test]
async fn a_review_goes_whole_with_its_verdict_its_lines_and_its_commit() {
    let server = FakeServer::start().await;
    server.on(
        "POST",
        "/repos/cli/cli/pulls/7/reviews",
        200,
        json!({ "id": 3 }),
    );
    let github = Github::new(reqwest::Client::new(), server.url.clone(), "t".into());
    let draft = |state| ReviewDraft {
        state,
        body: "Deux points.".into(),
        lines: vec![
            ReviewLine {
                path: "a.ts".into(),
                line: 12,
                start_line: Some(10),
                side: DiffSide::Left,
                body: "Pourquoi l'enlever ?".into(),
            },
            ReviewLine {
                path: "b.ts".into(),
                line: 3,
                body: "Ici".into(),
                ..Default::default()
            },
        ],
        commit: "abc".into(),
    };
    for state in [
        ReviewState::Commented,
        ReviewState::Approved,
        ReviewState::ChangesRequested,
    ] {
        github.review(&repo(), 7, &draft(state)).await.unwrap();
    }
    let sent: Vec<Value> = server
        .writes()
        .iter()
        .filter(|r| r.path() == "/repos/cli/cli/pulls/7/reviews")
        .map(|r| r.json())
        .collect();
    let events: Vec<&str> = sent.iter().map(|b| b["event"].as_str().unwrap()).collect();
    assert_eq!(events, ["COMMENT", "APPROVE", "REQUEST_CHANGES"]);
    let b = &sent[2];
    assert_eq!(b["body"], "Deux points.");
    assert_eq!(b["commit_id"], "abc");
    let c = &b["comments"];
    assert_eq!(c.as_array().unwrap().len(), 2, "in the one review: {b}");
    assert_eq!(
        (
            &c[0]["line"],
            &c[0]["start_line"],
            &c[0]["side"],
            &c[0]["start_side"]
        ),
        (&json!(12), &json!(10), &json!("LEFT"), &json!("LEFT"))
    );
    assert_eq!(
        (&c[1]["side"], c[1].get("start_line")),
        (&json!("RIGHT"), None)
    );

    // No commit said: GitHub's tip now.
    let bare = review_body(&ReviewDraft {
        commit: String::new(),
        ..draft(ReviewState::Commented)
    });
    assert!(bare.get("commit_id").is_none(), "{bare}");
}

/// A line no longer in the diff, or the user's own pull request: GitHub's 422 is said with its status and its words, for
/// the core to keep the review waiting and say why.
#[tokio::test]
async fn a_review_github_refuses_says_its_422_and_its_words() {
    for (message, errors) in [
        (
            "Unprocessable Entity",
            json!(["Line could not be resolved"]),
        ),
        (
            "Unprocessable Entity",
            json!(["Review Can not request changes on your own pull request"]),
        ),
    ] {
        let server = FakeServer::start().await;
        server.on(
            "POST",
            "/repos/cli/cli/pulls/7/reviews",
            422,
            json!({ "message": message, "errors": errors, "status": "422" }),
        );
        let github = Github::new(reqwest::Client::new(), server.url.clone(), "t".into());
        let e = github
            .review(
                &repo(),
                7,
                &ReviewDraft {
                    state: ReviewState::ChangesRequested,
                    body: "Non.".into(),
                    lines: vec![ReviewLine {
                        path: "a.ts".into(),
                        line: 99,
                        body: "x".into(),
                        ..Default::default()
                    }],
                    commit: String::new(),
                },
            )
            .await
            .unwrap_err();
        let h = e
            .downcast_ref::<escouade_plugin_kit::errors::HttpError>()
            .expect("an HttpError");
        assert_eq!(h.status, 422);
        let said = errors[0].as_str().unwrap();
        assert!(format!("{e:#}").contains(said), "{e:#}");
    }
}
