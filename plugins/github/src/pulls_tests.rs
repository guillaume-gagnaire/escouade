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
