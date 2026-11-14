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
