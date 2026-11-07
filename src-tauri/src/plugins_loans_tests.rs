//! What the core lends a plugin ends with what it was lent for (ESC-360): the try of a connection, won or lost, and the
//! removal of the account. `{account}` is judged as a plugin's `net/fetch` is, on the network the core gave the runtime.

use crate::core_tests::{harness, Harness};
use crate::forge::connect::ForgeForm;
use crate::forge::host_of;
use crate::integrations::fake::FakeServer;
use crate::integrations::Account;
use crate::model::Service;
use crate::plugins::builtin::{GITHUB, GITLAB, JIRA};
use crate::plugins::wasm::{Fetcher, REFUSED};
use serde_json::{json, Value};

/// Whether the plugin may reach `url` by `{account}`.
async fn reaches(h: &Harness, plugin: &str, url: &str) -> bool {
    let f = Fetcher::new(
        plugin,
        &["{account}".to_string()],
        &h.core.plugin_runtime.net(),
    );
    match f.fetch(&json!({ "url": format!("{url}/") })).await {
        Ok(_) => true,
        Err((code, why)) => {
            assert_eq!(code, REFUSED, "{why}");
            false
        }
    }
}

fn lent(h: &Harness, plugin: &str, host: &str) -> Option<String> {
    h.core
        .plugin_runtime
        .lent_secret(plugin, &format!("account:{host}"))
}

fn jira(site: &str, token: &str) -> Account {
    Account {
        site: site.into(),
        email: "ada@atlas.dev".into(),
        token: token.into(),
        ..Default::default()
    }
}

fn gitlab(url: &str, token: &str) -> ForgeForm {
    ForgeForm {
        kind: "gitlab".into(),
        url: url.into(),
        token: token.into(),
        ..Default::default()
    }
}

/// A Jira that says who the token is, or refuses it.
fn jira_answers(server: &FakeServer, status: u16) {
    server.on(
        "GET",
        "/rest/api/3/myself",
        status,
        json!({ "accountId": "acc-1", "emailAddress": "ada@atlas.dev" }),
    );
}

/// A GitLab that says who the token is, or refuses it.
fn gitlab_answers(server: &FakeServer, status: u16) {
    let body = |b: Value| if status == 200 { b } else { Value::Null };
    server.on(
        "GET",
        "/api/v4/user",
        status,
        body(json!({ "username": "ada" })),
    );
    server.on(
        "GET",
        "/api/v4/personal_access_tokens/self",
        status,
        body(json!({ "scopes": ["api"] })),
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_try_of_a_site_leaves_its_host_closed() {
    let h = harness("loan-try-site");
    let server = FakeServer::start().await;
    jira_answers(&server, 401);
    let e = h
        .core
        .integration_connect(Service::JIRA, jira(&server.url, "bad"))
        .await
        .unwrap_err();
    // The plugin reached the site for its try: it is the site that refused.
    assert_eq!(e.to_string(), "Jira refuse ces identifiants (401)");
    assert!(!reaches(&h, JIRA, &server.url).await);
    assert_eq!(lent(&h, JIRA, "jira"), None);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_try_of_a_code_host_leaves_neither_its_host_nor_its_token() {
    let h = harness("loan-try-forge");
    let server = FakeServer::start().await;
    gitlab_answers(&server, 401);
    assert!(h
        .core
        .forge_connect(gitlab(&server.url, "bad"))
        .await
        .is_err());
    assert_eq!(server.requests()[0].path(), "/api/v4/user");
    assert!(!reaches(&h, GITLAB, &server.url).await);
    assert_eq!(lent(&h, GITLAB, &host_of(&server.url)), None);
}

#[tokio::test(flavor = "multi_thread")]
async fn tries_at_the_same_time_reach_their_hosts_and_leave_none_open() {
    let h = harness("loan-two-tries");
    let (one, other) = (FakeServer::start().await, FakeServer::start().await);
    gitlab_answers(&one, 401);
    gitlab_answers(&other, 401);
    let (a, b) = tokio::join!(
        h.core.forge_connect(gitlab(&one.url, "bad")),
        h.core.forge_connect(gitlab(&other.url, "worse")),
    );
    for e in [a.unwrap_err(), b.unwrap_err()] {
        assert!(e.to_string().contains("401"), "{e}");
    }
    assert!(!reaches(&h, GITLAB, &one.url).await);
    assert!(!reaches(&h, GITLAB, &other.url).await);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_saved_account_keeps_its_host_with_no_loan_until_it_goes() {
    let h = harness("loan-saved-forge");
    let server = FakeServer::start().await;
    gitlab_answers(&server, 200);
    h.core
        .forge_connect(gitlab(&server.url, "glpat-good"))
        .await
        .unwrap();
    let host = host_of(&server.url);
    assert!(h.core.plugin_runtime.lent_hosts(GITLAB).is_empty());
    assert!(reaches(&h, GITLAB, &server.url).await);
    assert_eq!(lent(&h, GITLAB, &host).as_deref(), Some("glpat-good"));

    h.core.forge_disconnect(&host).await.unwrap();
    assert!(!reaches(&h, GITLAB, &server.url).await);
    assert_eq!(lent(&h, GITLAB, &host), None);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_removed_site_closes_its_host_and_gives_its_token_back() {
    let h = harness("loan-saved-site");
    let server = FakeServer::start().await;
    jira_answers(&server, 200);
    h.core
        .integration_connect(Service::JIRA, jira(&server.url, "good"))
        .await
        .unwrap();
    assert!(reaches(&h, JIRA, &server.url).await);
    assert_eq!(lent(&h, JIRA, "jira").as_deref(), Some("good"));

    h.core.integration_disconnect(Service::JIRA).unwrap();
    assert!(!reaches(&h, JIRA, &server.url).await);
    assert_eq!(lent(&h, JIRA, "jira"), None);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_com_gives_its_token_back_with_its_account() {
    let h = harness("loan-github");
    let server = FakeServer::start().await;
    h.core.bases.write().github = server.url.clone();
    server.on_with(
        "GET",
        "/user",
        200,
        json!({ "login": "ada" }),
        &[("X-OAuth-Scopes", "repo, read:org")],
    );
    h.core
        .forge_connect(ForgeForm {
            kind: "github".into(),
            token: "ghp".into(),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(lent(&h, GITHUB, "github.com").as_deref(), Some("ghp"));
    h.core.forge_disconnect("github.com").await.unwrap();
    assert_eq!(lent(&h, GITHUB, "github.com"), None);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_try_on_a_running_account_leaves_it_its_token() {
    let h = harness("loan-running");
    let server = FakeServer::start().await;
    gitlab_answers(&server, 200);
    h.core
        .forge_connect(gitlab(&server.url, "glpat-good"))
        .await
        .unwrap();
    let host = host_of(&server.url);
    h.core.forge_client(&host).await.unwrap();

    gitlab_answers(&server, 401);
    assert!(h
        .core
        .forge_connect(gitlab(&server.url, "typo"))
        .await
        .is_err());
    assert_eq!(lent(&h, GITLAB, &host).as_deref(), Some("glpat-good"));
    assert!(reaches(&h, GITLAB, &server.url).await);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_won_try_lends_the_new_token_of_a_running_account() {
    let h = harness("loan-reconnect");
    let server = FakeServer::start().await;
    gitlab_answers(&server, 200);
    for token in ["glpat-old", "glpat-new"] {
        h.core
            .forge_connect(gitlab(&server.url, token))
            .await
            .unwrap();
    }
    assert_eq!(
        lent(&h, GITLAB, &host_of(&server.url)).as_deref(),
        Some("glpat-new")
    );
}

#[tokio::test]
async fn a_try_that_ends_takes_back_what_it_lent_and_nothing_else() {
    let h = harness("loan-unit");
    let rt = &h.core.plugin_runtime;
    let key = "account:git.acme.dev";
    rt.lend(GITLAB, key, "running");
    let (one, two) = (rt.trial(), rt.trial());
    one.lend(GITLAB, key, "first", "git.acme.dev");
    two.lend(GITLAB, key, "second", "git.acme.dev");
    assert_eq!(rt.lent_secret(GITLAB, key).as_deref(), Some("second"));
    // Each is a plugin's own.
    assert_eq!(rt.lent_secret(GITHUB, key), None);
    assert!(rt.lent_hosts(GITHUB).is_empty());
    drop(two);
    assert_eq!(rt.lent_secret(GITLAB, key).as_deref(), Some("first"));
    assert_eq!(rt.lent_hosts(GITLAB), ["git.acme.dev"]);
    drop(one);
    assert_eq!(rt.lent_secret(GITLAB, key).as_deref(), Some("running"));
    assert!(rt.lent_hosts(GITLAB).is_empty());
}

#[tokio::test]
async fn a_token_read_for_an_account_that_went_is_not_lent() {
    let h = harness("loan-gone");
    h.core.lend_account(GITLAB, "git.acme.dev", "late");
    assert_eq!(lent(&h, GITLAB, "git.acme.dev"), None);
}

#[tokio::test(flavor = "multi_thread")]
async fn removing_an_account_leaves_the_loan_of_another_one() {
    let h = harness("loan-two-accounts");
    let (one, other) = (FakeServer::start().await, FakeServer::start().await);
    gitlab_answers(&one, 200);
    gitlab_answers(&other, 200);
    for (server, token) in [(&one, "glpat-one"), (&other, "glpat-other")] {
        h.core
            .forge_connect(gitlab(&server.url, token))
            .await
            .unwrap();
    }
    h.core.forge_disconnect(&host_of(&one.url)).await.unwrap();
    let kept = host_of(&other.url);
    assert_eq!(lent(&h, GITLAB, &kept).as_deref(), Some("glpat-other"));
    assert!(reaches(&h, GITLAB, &other.url).await);
}
