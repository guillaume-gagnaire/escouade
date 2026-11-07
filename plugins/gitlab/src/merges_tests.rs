//! What only the client itself shows: its pace, what it waits. The rest of its tests play its module through the app:
//! `src-tauri/src/forge/gitlab_tests.rs`.

use super::*;
use crate::fake::FakeServer;
use serde_json::json;
use std::time::Duration;

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
async fn a_429_is_waited_out() {
    let server = FakeServer::start().await;
    // Refused for a moment: what GitLab asks is waited, then the call is sent once more.
    server.on("GET", "/user", 200, json!({"username": "ada"}));
    server.on_once(
        "GET",
        "/user",
        429,
        json!({"message": "Retry later"}),
        &[("retry-after", "30")],
    );
    server.on(
        "GET",
        "/personal_access_tokens/self",
        404,
        serde_json::Value::Null,
    );
    let gitlab =
        Gitlab::new(reqwest::Client::new(), server.url.clone(), "t".into()).with_pace(Pace {
            slow_below: 0.1,
            max_wait: Duration::from_millis(300),
        });
    assert_eq!(gitlab.me().await.unwrap().user, "ada");
    assert!(gitlab.waited() >= Duration::from_millis(250));
    assert_eq!(requests_to(&server, "/user"), 2, "asked once more");
}
#[tokio::test]
async fn near_its_limit_the_client_slows_down() {
    let server = FakeServer::start().await;
    let reset = (now_secs() + 30).to_string();
    server.on_with(
        "GET",
        "/user",
        200,
        json!({"username": "ada"}),
        &[
            ("ratelimit-limit", "2000"),
            ("ratelimit-remaining", "3"),
            ("ratelimit-reset", &reset),
        ],
    );
    server.on(
        "GET",
        "/personal_access_tokens/self",
        404,
        serde_json::Value::Null,
    );
    let gitlab =
        Gitlab::new(reqwest::Client::new(), server.url.clone(), "t".into()).with_pace(Pace {
            slow_below: 0.1,
            max_wait: Duration::from_millis(300),
        });
    gitlab.me().await.unwrap();
    assert!(
        gitlab.waited() > Duration::ZERO,
        "the second call of me waited"
    );
    let before = gitlab.waited();
    gitlab.me().await.unwrap();
    assert!(gitlab.waited() >= before + Duration::from_millis(250));
}

#[test]
fn the_squash_option_says_the_strategies() {
    assert_eq!(
        merge_options_of(&json!({ "squash_option": "never" })).strategies,
        vec![MergeStrategy::Merge]
    );
    assert_eq!(
        merge_options_of(&json!({ "squash_option": "default_on" })).strategies,
        vec![MergeStrategy::Merge, MergeStrategy::Squash]
    );
}

#[tokio::test]
async fn a_next_page_on_another_host_is_not_read() {
    let server = FakeServer::start().await;
    let other = FakeServer::start().await;
    let next = format!("<{}/items?page=2>; rel=\"next\"", other.url);
    server.on_with("GET", "/items", 200, json!([1]), &[("Link", &next)]);
    other.on("GET", "/items?page=2", 200, json!([2]));
    let gitlab = Gitlab::new(reqwest::Client::new(), server.url.clone(), "t".into());
    assert_eq!(gitlab.list("/items").await.unwrap(), [json!(1)]);
    assert!(other.requests().is_empty(), "the token went there");
}

#[tokio::test]
async fn a_next_page_of_the_api_is_read() {
    let server = FakeServer::start().await;
    let next = format!("<{}/items?page=2>; rel=\"next\"", server.url);
    server.on_with("GET", "/items", 200, json!([1]), &[("Link", &next)]);
    server.on("GET", "/items?page=2", 200, json!([2]));
    let gitlab = Gitlab::new(reqwest::Client::new(), server.url.clone(), "t".into());
    assert_eq!(gitlab.list("/items").await.unwrap(), [json!(1), json!(2)]);
}
