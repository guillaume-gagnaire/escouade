//! GitHub Issues against a fake server. Since INT-8.1 its client is the plugin « GitHub »'s
//! (`plugins/github/src/issues.rs`): the calls go to its process by
//! `ProcessTicketSource`, as the core makes them, and the plugin calls the fake server.

use super::fake::FakeServer;
use super::source::TicketSource;
use super::{Account, Query};
use crate::model::{ExternalRef, ExternalState, Service};
use crate::plugins::adapters::ProcessTicketSource;
use crate::plugins::builtin::GITHUB;
use serde_json::json;

/// GitHub Issues on github.com's account, as the core serves it: by the plugin « GitHub », in its
/// process, the account's token lent, its API the fake server.
async fn github() -> (FakeServer, ProcessTicketSource) {
    let server = FakeServer::start().await;
    let account = Account {
        token: "ghp_x".into(),
        user: "ada".into(),
        site: "https://github.com".into(),
        ..Default::default()
    };
    let rt = crate::plugins::builtin::test_runtime("github-issues");
    rt.lend(GITHUB, "account:github.com", &account.token);
    crate::plugins::builtin::let_reach(&rt, &[("fake.test", &server.url)]);
    let g = ProcessTicketSource::new(rt, GITHUB, "github", &account).with_api(&server.url);
    (server, g)
}

fn r() -> ExternalRef {
    ExternalRef {
        service: Service::GITHUB,
        id: "42".into(),
        key: "#42".into(),
        container: "acme/api".into(),
        ..Default::default()
    }
}

fn label(name: &str) -> ExternalState {
    ExternalState {
        id: format!("label:{name}"),
        name: name.into(),
    }
}

#[tokio::test]
async fn the_token_is_sent_and_the_login_read() {
    let (server, g) = github().await;
    server.on("GET", "/user", 200, json!({ "login": "ada" }));
    assert_eq!(
        g.me().await.unwrap(),
        ("@ada".to_string(), "ada".to_string())
    );
    let req = &server.requests()[0];
    assert_eq!(req.headers["authorization"], "Bearer ghp_x");
    assert_eq!(req.headers["user-agent"], "escouade");
}

#[tokio::test]
async fn the_states_are_open_closed_and_the_labels() {
    let (server, g) = github().await;
    server.on(
        "GET",
        "/repos/acme/api/labels",
        200,
        json!([{ "name": "in-progress" }, { "name": "to test" }]),
    );
    server.on(
        "GET",
        "/user/repos",
        200,
        json!([{ "full_name": "acme/api" }]),
    );
    assert_eq!(g.containers().await.unwrap()[0].id, "acme/api");
    let ids: Vec<String> = g
        .states("acme/api")
        .await
        .unwrap()
        .into_iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(
        ids,
        ["open", "closed", "label:in-progress", "label:to test"]
    );
}

#[tokio::test]
async fn the_issues_are_listed_or_searched_without_the_pull_requests() {
    let (server, g) = github().await;
    server.on(
        "GET",
        "/repos/acme/api/labels",
        200,
        json!([{ "name": "bug" }]),
    );
    let issues = json!([
        { "number": 42, "title": "Crash au paiement", "body": "- [ ] Plus de crash\n- [ ] Test e2e",
          "html_url": "https://github.com/acme/api/issues/42", "labels": [{ "name": "bug" }], "assignee": { "login": "ada" } },
        { "number": 43, "title": "Une PR", "pull_request": {}, "labels": [] },
        { "number": 44, "title": "Doc", "body": null, "labels": [], "assignee": null }
    ]);
    server.on("GET", "/repos/acme/api/issues", 200, issues.clone());
    let page = g.issues("acme/api", &Query::default()).await.unwrap();
    let i = &page.issues[0];
    assert_eq!(page.issues.len(), 2);
    assert_eq!(
        (i.id.as_str(), i.key.as_str(), i.kind.as_str()),
        ("42", "#42", "Issue")
    );
    assert_eq!(i.kind_code, "issue");
    assert_eq!(i.meta, ["bug", "@ada"]);
    assert_eq!(i.criteria, ["Plus de crash", "Test e2e"]);
    assert_eq!(page.issues[1].meta, ["Non assignée"]);
    assert_eq!(server.requests()[0].query("state").as_deref(), Some("open"));
    let ids: Vec<&str> = page.filters.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["mine", "label:bug"]);
    // A label's chip keeps those that carry it.
    let q = Query {
        filters: vec!["label:bug".into()],
        ..Default::default()
    };
    assert_eq!(g.issues("acme/api", &q).await.unwrap().issues.len(), 1);
    server.on("GET", "/search/issues", 200, json!({ "items": issues }));
    let q = Query {
        text: "#crash".into(),
        filters: vec!["mine".into()],
        label: Some("claude-ready".into()),
        ..Default::default()
    };
    assert_eq!(g.issues("acme/api", &q).await.unwrap().issues.len(), 2);
    let search = server
        .requests()
        .into_iter()
        .find(|r| r.path() == "/search/issues")
        .unwrap();
    assert_eq!(
        search.query("q").as_deref(),
        Some("repo:acme/api is:issue is:open assignee:@me label:\"claude-ready\" crash")
    );
    // A number is the issue itself, when it is an open one.
    server.on(
        "GET",
        "/repos/acme/api/issues/44",
        200,
        json!({ "number": 44, "title": "Doc", "state": "open", "labels": [] }),
    );
    let q = Query {
        text: "#44".into(),
        ..Default::default()
    };
    let found = g.issues("acme/api", &q).await.unwrap().issues;
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].key, "#44");
    server.on(
        "GET",
        "/repos/acme/api/issues/44",
        200,
        json!({ "number": 44, "title": "Doc", "state": "closed", "labels": [] }),
    );
    assert!(g.issues("acme/api", &q).await.unwrap().issues.is_empty());
    server.on(
        "GET",
        "/repos/acme/api/issues/45",
        404,
        json!({ "message": "Not Found" }),
    );
    let q = Query {
        text: "45".into(),
        ..Default::default()
    };
    assert!(g.issues("acme/api", &q).await.unwrap().issues.is_empty());
}

#[tokio::test]
async fn the_issues_come_a_page_at_a_time_and_a_search_says_how_many_there_are() {
    let (server, g) = github().await;
    server.on("GET", "/repos/acme/api/labels", 200, json!([]));
    let issue = |n: u64| json!({ "number": n, "title": format!("Issue {n}"), "labels": [] });
    let more = format!(
        "<{0}/repositories/1/issues?page=2>; rel=\"next\", <{0}/repositories/1/issues?page=4>; rel=\"last\"",
        server.url
    );
    server.on_with(
        "GET",
        "/repos/acme/api/issues",
        200,
        json!([issue(1), issue(2)]),
        &[("Link", &more)],
    );
    let first = g.issues("acme/api", &Query::default()).await.unwrap();
    assert_eq!((first.next.as_deref(), first.total), (Some("2"), None));
    assert_eq!(server.requests()[0].query("page"), None);
    let end = format!(
        "<{0}/repositories/1/issues?page=1>; rel=\"prev\", <{0}/repositories/1/issues?page=1>; rel=\"first\"",
        server.url
    );
    server.on_with(
        "GET",
        "/repos/acme/api/issues",
        200,
        json!([issue(3)]),
        &[("Link", &end)],
    );
    let q = Query {
        page: Some("2".into()),
        ..Default::default()
    };
    let last = g.issues("acme/api", &q).await.unwrap();
    let listed = |path: &str| {
        server
            .requests()
            .into_iter()
            .rfind(|r| r.path() == path)
            .unwrap()
    };
    assert_eq!(
        listed("/repos/acme/api/issues").query("page").as_deref(),
        Some("2")
    );
    assert_eq!(last.issues[0].key, "#3");
    assert_eq!(last.next, None);
    server.on_with(
        "GET",
        "/search/issues",
        200,
        json!({ "total_count": 312, "items": [issue(4)] }),
        &[("Link", &more)],
    );
    let q = Query {
        text: "crash".into(),
        ..Default::default()
    };
    let found = g.issues("acme/api", &q).await.unwrap();
    assert_eq!((found.next.as_deref(), found.total), (Some("2"), Some(312)));
    let q = Query {
        page: Some("3".into()),
        ..q
    };
    g.issues("acme/api", &q).await.unwrap();
    let search = listed("/search/issues");
    assert_eq!(
        (
            search.query("per_page").as_deref(),
            search.query("page").as_deref()
        ),
        (Some("50"), Some("3"))
    );
    // A label's chip keeps some of them here: the search's count is no longer theirs.
    let q = Query {
        filters: vec!["label:bug".into()],
        ..q
    };
    let chipped = g.issues("acme/api", &q).await.unwrap();
    assert_eq!((chipped.next.as_deref(), chipped.total), (Some("4"), None));
}

#[tokio::test]
async fn a_label_state_replaces_the_other_mapped_labels() {
    let (server, g) = github().await;
    server.on(
        "GET",
        "/repos/acme/api/issues/42",
        200,
        json!({ "state": "open", "labels": [{ "name": "in-progress" }, { "name": "bug" }] }),
    );
    server.on(
        "DELETE",
        "/repos/acme/api/issues/42/labels/in-progress",
        200,
        json!([]),
    );
    server.on("POST", "/repos/acme/api/issues/42/labels", 200, json!([]));
    let mapped = [label("in-progress"), label("to test")];
    g.set_state(&r(), &label("to test"), &mapped).await.unwrap();
    let w = server.writes();
    assert_eq!(w.len(), 2);
    assert_eq!(
        (w[0].method.as_str(), w[0].path()),
        ("DELETE", "/repos/acme/api/issues/42/labels/in-progress")
    );
    assert_eq!(w[1].json(), json!({ "labels": ["to test"] }));
    // Already carrying it: nothing to do.
    g.set_state(&r(), &label("in-progress"), &mapped)
        .await
        .unwrap();
    assert_eq!(server.writes().len(), 2);
}

#[tokio::test]
async fn closed_closes_and_open_or_a_label_opens_again() {
    let (server, g) = github().await;
    server.on("PATCH", "/repos/acme/api/issues/42", 200, json!({}));
    server.on("POST", "/repos/acme/api/issues/42/labels", 200, json!([]));
    server.on(
        "GET",
        "/repos/acme/api/issues/42",
        200,
        json!({ "state": "open", "labels": [] }),
    );
    let closed = ExternalState {
        id: "closed".into(),
        name: "Fermée".into(),
    };
    g.set_state(&r(), &closed, &[]).await.unwrap();
    assert_eq!(
        server.writes()[0].json(),
        json!({ "state": "closed", "state_reason": "completed" })
    );
    server.on(
        "GET",
        "/repos/acme/api/issues/42",
        200,
        json!({ "state": "closed", "labels": [] }),
    );
    g.set_state(&r(), &closed, &[]).await.unwrap();
    assert_eq!(server.writes().len(), 1);
    g.set_state(&r(), &label("wip"), &[label("wip")])
        .await
        .unwrap();
    let w = server.writes();
    assert_eq!(w[1].json(), json!({ "state": "open" }));
    assert_eq!(w[2].json(), json!({ "labels": ["wip"] }));
}

#[tokio::test]
async fn a_comment_is_markdown() {
    let (server, g) = github().await;
    server.on(
        "POST",
        "/repos/acme/api/issues/42/comments",
        201,
        json!({ "id": 1 }),
    );
    g.comment(&r(), "Escouade : prêt").await.unwrap();
    assert_eq!(
        server.writes()[0].json(),
        json!({ "body": "Escouade : prêt" })
    );
}
