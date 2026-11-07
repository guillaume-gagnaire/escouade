//! What only the client itself shows: its pace, the budget it keeps, what it waits. The rest of its tests play its module
//! through the app: `src-tauri/src/forge/bitbucket_tests.rs`.

use super::*;
use crate::fake::FakeServer;
use serde_json::json;
use std::time::Duration;

const PR: &str = "/repositories/acme/demo/pullrequests";

fn fixture(name: &str) -> Value {
    let text = match name {
        "user" => include_str!("../../../tests/fixtures/forges/bitbucket/user.json"),
        "pullrequest" => include_str!("../../../tests/fixtures/forges/bitbucket/pullrequest.json"),
        "open" => include_str!("../../../tests/fixtures/forges/bitbucket/pullrequests-open.json"),
        "comments" => include_str!("../../../tests/fixtures/forges/bitbucket/comments.json"),
        "statuses" => include_str!("../../../tests/fixtures/forges/bitbucket/statuses.json"),
        "steps" => include_str!("../../../tests/fixtures/forges/bitbucket/steps.json"),
        "environments" => {
            include_str!("../../../tests/fixtures/forges/bitbucket/environments.json")
        }
        "deployments" => include_str!("../../../tests/fixtures/forges/bitbucket/deployments.json"),
        "branch" => include_str!("../../../tests/fixtures/forges/bitbucket/branch.json"),
        "error" => include_str!("../../../tests/fixtures/forges/bitbucket/error.json"),
        _ => panic!("no fixture {name}"),
    };
    serde_json::from_str(text).unwrap()
}
fn repo() -> RepoRef {
    RepoRef {
        namespace: "acme".into(),
        name: "demo".into(),
    }
}
/// The client itself, for what its process hides.
fn direct(server: &FakeServer) -> Bitbucket {
    Bitbucket::new(
        reqwest::Client::new(),
        server.url.clone(),
        "ada@atlas.dev".into(),
        "ATATT_test".into(),
    )
}
#[tokio::test]
async fn a_merge_that_goes_on_in_the_background_is_waited_for() {
    let server = FakeServer::start().await;
    let task = format!("{}{PR}/9/merge/task-status/t1", server.url);
    server.on_with(
        "POST",
        &format!("{PR}/9/merge"),
        202,
        Value::Null,
        &[("location", &task)],
    );
    server.on(
        "GET",
        &format!("{PR}/9/merge/task-status/t1"),
        200,
        json!({"task_status": "SUCCESS"}),
    );
    server.on_once(
        "GET",
        &format!("{PR}/9/merge/task-status/t1"),
        200,
        json!({"task_status": "PENDING"}),
        &[],
    );
    direct(&server)
        .with_poll(Duration::from_millis(5))
        .merge(&repo(), 9, MergeStrategy::Merge)
        .await
        .unwrap();
    let looks = server
        .requests()
        .iter()
        .filter(|r| r.method == "GET")
        .count();
    assert_eq!(looks, 2, "pending, then done");

    // A merge that failed in the background is an error.
    server.on(
        "GET",
        &format!("{PR}/9/merge/task-status/t1"),
        200,
        json!({"task_status": "ERROR"}),
    );
    let e = direct(&server)
        .with_poll(Duration::from_millis(5))
        .merge(&repo(), 9, MergeStrategy::Merge)
        .await
        .unwrap_err();
    assert!(format!("{e:#}").contains("ERROR"), "{e:#}");
}
// ---- The hour's limit ----

#[tokio::test]
async fn the_client_counts_the_hours_calls_and_says_how_many_are_left() {
    let server = FakeServer::start().await;
    server.on("GET", "/user", 200, fixture("user"));
    let bb = direct(&server);
    assert!(bb.rate_limit().is_none(), "nothing is known before a call");
    for _ in 0..3 {
        bb.me().await.unwrap();
    }
    let rl = bb.rate_limit().unwrap();
    assert_eq!((rl.limit, rl.remaining), (1000, 997));
    assert!(rl.reset > 0);
    assert_eq!(
        bb.waited(),
        Duration::ZERO,
        "far from the limit nothing waits"
    );
}
#[tokio::test]
async fn the_client_believes_the_limit_bitbucket_tells_and_slows_down_near_it() {
    let server = FakeServer::start().await;
    server.on_with(
        "GET",
        "/user",
        200,
        fixture("user"),
        &[
            ("x-ratelimit-limit", "50"),
            ("x-ratelimit-resource", "repository"),
        ],
    );
    let bb = direct(&server);
    bb.me().await.unwrap();
    assert_eq!(bb.rate_limit().map(|l| l.limit), Some(50));

    // « Near the limit »: a fifth is left at most, calls are spread over what remains of the hour: the wait is
    // the longest allowed.
    let server = FakeServer::start().await;
    server.on_with(
        "GET",
        "/user",
        200,
        fixture("user"),
        &[
            ("x-ratelimit-limit", "1000"),
            ("x-ratelimit-nearlimit", "true"),
        ],
    );
    let bb = direct(&server).with_pace(Pace {
        slow_below: 0.5,
        max_wait: Duration::from_millis(200),
    });
    bb.me().await.unwrap();
    assert!(bb.rate_limit().unwrap().remaining <= 200);
    let before = std::time::Instant::now();
    bb.me().await.unwrap();
    assert!(
        before.elapsed() >= Duration::from_millis(150),
        "{:?}",
        before.elapsed()
    );
    assert!(bb.waited() >= Duration::from_millis(150));
}
#[tokio::test]
async fn a_small_budget_slows_the_client_as_it_runs_out() {
    let server = FakeServer::start().await;
    server.on("GET", "/user", 200, fixture("user"));
    let bb = direct(&server).with_limit(10).with_pace(Pace {
        slow_below: 0.3,
        max_wait: Duration::from_millis(100),
    });
    for _ in 0..7 {
        bb.me().await.unwrap();
    }
    assert_eq!(
        bb.waited(),
        Duration::ZERO,
        "3 left of 10 is not below a third yet"
    );
    bb.me().await.unwrap();
    bb.me().await.unwrap();
    assert!(
        bb.waited() >= Duration::from_millis(100),
        "{:?}",
        bb.waited()
    );
    assert_eq!(bb.rate_limit().map(|l| l.remaining), Some(1));
}
#[tokio::test]
async fn a_refusal_for_the_limit_is_waited_out_then_asked_once_more() {
    let server = FakeServer::start().await;
    server.on("GET", "/user", 200, fixture("user"));
    server.on_once(
        "GET",
        "/user",
        429,
        json!({"type": "error", "error": {"message": "Rate limit exceeded"}}),
        &[("retry-after", "1")],
    );
    let bb = direct(&server).with_pace(Pace {
        slow_below: 0.1,
        max_wait: Duration::from_millis(80),
    });
    let id = bb.me().await.unwrap();
    assert_eq!(id.label, "@ada");
    assert_eq!(server.requests().len(), 2);
    assert!(
        bb.waited() >= Duration::from_millis(80),
        "{:?}",
        bb.waited()
    );
}
#[tokio::test]
async fn a_refusal_that_holds_is_said_and_the_hour_taken_as_spent() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        "/user",
        429,
        json!({"type": "error", "error": {"message": "Rate limit exceeded"}}),
    );
    let bb = direct(&server).with_pace(Pace {
        slow_below: 0.1,
        max_wait: Duration::from_millis(20),
    });
    let e = bb.me().await.unwrap_err();
    assert!(format!("{e:#}").contains("429"), "{e:#}");
    assert_eq!(server.requests().len(), 2, "one more try, no more");
    assert_eq!(bb.rate_limit().map(|l| l.remaining), Some(0));
}

#[test]
fn a_branch_that_says_nothing_has_every_strategy() {
    assert_eq!(merge_options_of(&json!({})).strategies.len(), 4);
}
