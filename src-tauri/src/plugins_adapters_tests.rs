//! Tests of the in-process adapters of the ticket sources and the code hosts (INT-5, ESC-211;
//! `docs/PLUGINS.md` §4.5) against the fake plugin, which serves one source and one host: the same
//! scenarios as the integrations' own tests (a list of containers, states and tickets, a state
//! pushed, a comment, a pull request read, its checks, a 401, a 404, a refusal that lasts), and
//! those a process adds: the secret fetched by the plugin, the delay, the plugin that crashes.

use super::plugins_runtime_tests::{fake, quick, until, Fake, ID};
use crate::forge::{Credentials, Forge, PrState, RepoRef, Unsupported};
use crate::integrations::secrets::SecretStore;
use crate::integrations::source::TicketSource;
use crate::integrations::{lasting, Account, HttpError, Query};
use crate::model::{ExternalRef, ExternalState};
use crate::plugins::adapters::{is_down, ProcessForge, ProcessTicketSource};
use crate::plugins::runtime::{secret_name, Limits, PhaseName};
use serde_json::json;
use std::time::Duration;

fn source(f: &Fake) -> ProcessTicketSource {
    ProcessTicketSource::new(f.rt.clone(), ID, "fake", &Account::default())
}

fn forge(f: &Fake) -> ProcessForge {
    ProcessForge::new(f.rt.clone(), ID, "fake", &Credentials::default())
}

fn repo(name: &str) -> RepoRef {
    RepoRef {
        namespace: "acme".into(),
        name: name.into(),
    }
}

fn ext(container: &str) -> ExternalRef {
    ExternalRef {
        id: "1".into(),
        key: "FAKE-1".into(),
        container: container.into(),
        ..ExternalRef::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_source_lists_its_containers_states_and_tickets_like_the_others() {
    let f = fake("src", &[], quick());
    let s = source(&f);
    assert_eq!(
        s.me().await.unwrap(),
        ("ada@fake.test".to_string(), "ada".to_string())
    );
    let containers = s.containers().await.unwrap();
    assert_eq!(containers.len(), 2);
    assert!(containers[0].own);
    assert_eq!(containers[1].group.as_deref(), Some("Équipe"));

    let view = s.states_view("demo").await.unwrap();
    assert_eq!(view.states.len(), 2);
    assert_eq!(
        view.defaults,
        crate::integrations::text::default_states(&view.states),
        "guessed by their names"
    );

    let page = s.issues("demo", &Query::default()).await.unwrap();
    assert_eq!(page.issues.len(), 2);
    assert_eq!(page.filters[0].id, "mine");
    let q = Query {
        text: "second".into(),
        ..Query::default()
    };
    assert_eq!(s.issues("demo", &q).await.unwrap().issues.len(), 1);

    let done = ExternalState {
        id: "done".into(),
        name: "Fait".into(),
    };
    s.set_state(&ext("demo"), &done, std::slice::from_ref(&done))
        .await
        .unwrap();
    s.comment(&ext("demo"), "Fini").await.unwrap();

    let calls = f.rt.call(ID, "test/calls", json!({})).await.unwrap();
    let last = calls.as_array().unwrap().last().unwrap();
    assert_eq!(last["method"], "tickets/comment");
    assert_eq!(last["params"]["text"], "Fini");
    assert_eq!(last["params"]["ref"]["key"], "FAKE-1");
    let account = last["params"]["account"].to_string();
    assert!(!account.contains("token"), "{account}");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_plugin_gets_its_token_itself_and_a_refused_one_is_a_401() {
    let f = fake("token", &["--token-from", "account:fake"], quick());
    let s = source(&f);
    let e = s.containers().await.unwrap_err();
    // No secret kept: refused.
    assert_eq!(e.downcast_ref::<HttpError>().unwrap().status, 401);
    assert!(crate::forge::refused(&e));

    f.store
        .set(&secret_name(ID, "account:fake"), "bad")
        .unwrap();
    assert!(source(&f).containers().await.is_err());
    f.store
        .set(&secret_name(ID, "account:fake"), "good")
        .unwrap();
    assert_eq!(source(&f).containers().await.unwrap().len(), 2);
    // Never in a call, and not in the journal.
    let journal = format!("{:?}", f.rt.journal(ID));
    assert!(!journal.contains("good"), "{journal}");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_errors_of_the_service_are_typed_as_the_core_knows_them() {
    let f = fake("typed", &[], quick());
    let s = source(&f);
    let e = s.states("gone").await.unwrap_err();
    assert_eq!(e.downcast_ref::<HttpError>().unwrap().status, 404);
    assert!(lasting(&e), "a 404 is not tried again");

    let e = s.states("refused").await.unwrap_err();
    assert!(e.downcast_ref::<crate::integrations::Refused>().is_some());
    assert!(lasting(&e));
    assert!(!is_down(&e));

    // The plugin still runs after those.
    assert_eq!(f.rt.status(ID).phase, PhaseName::Running);
    assert!(s.containers().await.is_ok());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plugin_that_crashes_or_is_too_slow_is_down_and_the_sync_waits() {
    let limits = Limits {
        restarts: 0,
        ..quick()
    };
    let f = fake("crash", &[], limits);
    let s = source(&f);
    assert!(s.containers().await.is_ok());
    let e = s.states("crash").await.unwrap_err();
    assert!(is_down(&e), "{e:#}");
    assert!(!lasting(&e), "the sync tries again, with its waits");
    until("stopped", || f.rt.status(ID).phase == PhaseName::Stopped).await;
    // Stopped: every call says so at once, as a plugin that does not answer.
    let e = s.containers().await.unwrap_err();
    assert!(is_down(&e), "{e:#}");
    assert!(format!("{e}").contains(ID) || format!("{e}").contains("stopped"));
}

#[test]
fn the_codes_of_a_plugin_say_the_status_of_its_service() {
    assert_eq!(crate::plugins::adapters::http_code(401), -32401);
    assert_eq!(crate::plugins::adapters::REFUSED, -32010);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_call_past_its_delay_is_down_not_lost() {
    let f = fake("slow", &[], quick());
    let line = f.rt.clone();
    // The adapters take the delay of their method; this one is held by the fake for ever, so
    // the same wait is played with a short one.
    let r = line
        .call_within(
            ID,
            "tickets/states",
            json!({ "container": "slow" }),
            Duration::from_millis(300),
        )
        .await;
    assert_eq!(r, Err(crate::plugins::runtime::CallError::Timeout));
    assert_eq!(
        crate::plugins::runtime::timeout_of("tickets/issues"),
        Duration::from_secs(60)
    );
    assert_eq!(
        crate::plugins::runtime::timeout_of("account/me"),
        Duration::from_secs(60)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_host_reads_and_opens_pull_requests_and_reports_its_budget() {
    let f = fake("forge", &[], quick());
    let h = forge(&f);
    let me = h.me().await.unwrap();
    assert_eq!((me.user.as_str(), me.label.as_str()), ("ada", "@ada"));
    assert_eq!(me.scopes, Some(vec!["repo".to_string()]));
    assert!(h.rate_limit().is_none());

    let pr = h.pull_request(&repo("demo"), 7).await.unwrap();
    assert_eq!(pr.number, 7);
    assert_eq!(pr.state, PrState::Open);
    assert_eq!(h.rate_limit().unwrap().remaining, 4990);

    let of_branch = h
        .pull_requests_of_branch(&repo("demo"), "feat/x")
        .await
        .unwrap();
    assert_eq!(of_branch.len(), 1);
    assert!(h
        .pull_requests_of_branch(&repo("demo"), "none")
        .await
        .unwrap()
        .is_empty());

    let checks = h.checks(&repo("demo"), 7).await.unwrap();
    assert_eq!(checks[0].name, "ci");

    let draft = crate::forge::PullRequestDraft {
        title: "Neuve".into(),
        head: "feat/y".into(),
        base: "main".into(),
        ..Default::default()
    };
    let made = h.create_pr(&repo("demo"), &draft).await.unwrap();
    assert_eq!((made.number, made.head.as_str()), (8, "feat/y"));
    h.comment(&repo("demo"), 7, "ok").await.unwrap();
    assert_eq!(h.parent(&repo("demo")).await.unwrap(), None);
}

#[tokio::test(flavor = "multi_thread")]
async fn what_the_host_cannot_do_is_unsupported_and_its_errors_are_typed() {
    let f = fake("forge-err", &[], quick());
    let h = forge(&f);
    let e = h
        .merge(&repo("demo"), 7, crate::forge::MergeStrategy::Squash)
        .await
        .unwrap_err();
    assert!(e.downcast_ref::<Unsupported>().is_some(), "{e:#}");

    let e = h.pull_request(&repo("gone"), 1).await.unwrap_err();
    assert_eq!(e.downcast_ref::<HttpError>().unwrap().status, 404);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_host_that_crashes_leaves_the_pull_requests_unknown_not_lost() {
    let limits = Limits {
        restarts: 0,
        ..quick()
    };
    let f = fake("forge-crash", &[], limits);
    let h = forge(&f);
    assert!(h.pull_request(&repo("demo"), 7).await.is_ok());
    let e = h.pull_requests_of_branch(&repo("crash"), "x").await;
    let e = e.unwrap_err();
    // The caller (`agentprs`) keeps what it read last on an error: down is that error, typed.
    assert!(is_down(&e), "{e:#}");
    assert_eq!(h.rate_limit().unwrap().limit, 5000, "the last budget stays");
}
