//! Tests of Jira as a plugin of the app (INT-7.3, ESC-215; `plugins/jira`): the tests the core
//! had for its own Jira client (`integrations/jira.rs`), word for word, now played against the plugin started as the
//! app starts it (its module, in the host, as `scripts/plugins-wasm.sh` built it) by the runtime and
//! `ProcessTicketSource`, on the fake Jira of the integrations' tests; and what a process adds: the token fetched by
//! the plugin and never sent, the plugin that is off or stopped saying so, the offer of the plugin to the registry of
//! sources, the texts of the form read in its manifest.

use crate::i18n::Lang;
use crate::integrations::fake::FakeServer;
use crate::integrations::secrets::{MemoryStore, SecretStore};
use crate::integrations::source::{self, TicketSource};
use crate::integrations::{Account, Container, Query};
use crate::model::{ExternalRef, ExternalState, Service};
use crate::paths::DataDir;
use crate::plugins::adapters::{is_down, ProcessTicketSource};
use crate::plugins::runtime::{secret_name, Limits, Runtime};
use crate::plugins::wasm::Net;
use crate::plugins::{Origin, Plugins};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

const ID: &str = "dev.escouade.jira";
const VERSION: &str = "2.2.0";
const S: Service = Service::JIRA;

struct Rig {
    server: FakeServer,
    rt: Arc<Runtime>,
    plugins: Arc<Plugins>,
    t: ProcessTicketSource,
}

fn app_dir() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    exe.parent().unwrap().parent().unwrap().to_path_buf()
}

/// Jira connected to a fake server: the account's site and e-mail as the core tells them, its token ` tok ` (as typed)
/// in the keychain's name for the plugin.
async fn jira() -> Rig {
    let server = FakeServer::start().await;
    let dir = std::env::temp_dir().join(format!("plugins-jira-{}", uuid::Uuid::new_v4().simple()));
    let data = DataDir::new(dir.join("data"));
    let plugins = Plugins::load_in(data.clone(), VERSION, app_dir());
    let store = Arc::new(MemoryStore::default());
    store
        .set(&secret_name(ID, "account:jira"), " tok ")
        .unwrap();
    let rt = Runtime::new(
        plugins.clone(),
        store,
        &data,
        VERSION,
        Limits {
            initialize: Duration::from_secs(20),
            shutdown: Duration::from_secs(2),
            ..Limits::default()
        },
    );
    rt.set_net(Net {
        routes: HashMap::from([("jira.test".to_string(), server.url.clone())]),
        dns: None,
        ..Net::default()
    });
    let account = Account {
        site: format!("{}/", server.url),
        email: "ada@atlas.dev".into(),
        ..Default::default()
    };
    let t = ProcessTicketSource::new(rt.clone(), ID, "jira", &account);
    Rig {
        server,
        rt,
        plugins,
        t,
    }
}

fn r(key: &str) -> ExternalRef {
    ExternalRef {
        service: S,
        id: key.into(),
        key: key.into(),
        ..Default::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_account_is_checked_with_its_email_and_token() {
    let Rig { server, t, rt, .. } = jira().await;
    server.on(
        "GET",
        "/rest/api/3/myself",
        200,
        json!({ "accountId": "acc-1", "emailAddress": "ada@atlas.dev", "displayName": "Ada" }),
    );
    let (label, user) = t.me().await.unwrap();
    let host = server.url.trim_start_matches("http://").to_string();
    assert_eq!(label, format!("ada@atlas.dev · {host}"));
    assert_eq!(user, "acc-1");
    let req = &server.requests()[0];
    // base64("ada@atlas.dev:tok")
    assert_eq!(
        req.headers["authorization"],
        "Basic YWRhQGF0bGFzLmRldjp0b2s="
    );
    server.on("GET", "/rest/api/3/myself", 401, Value::Null);
    assert_eq!(
        t.me().await.unwrap_err().to_string(),
        "Jira refuse ces identifiants (401)"
    );
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_projects_and_their_statuses_are_listed() {
    let Rig { server, t, rt, .. } = jira().await;
    server.on(
        "GET",
        "/rest/api/3/project/search",
        200,
        json!({ "isLast": true, "values": [{ "key": "ATL", "name": "Atlas" }, { "key": "MOB", "name": "Mobile" }] }),
    );
    let c = t.containers().await.unwrap();
    assert_eq!(
        c,
        [
            Container {
                id: "ATL".into(),
                name: "ATL — Atlas".into(),
                group: None,
                own: false,
            },
            Container {
                id: "MOB".into(),
                name: "MOB — Mobile".into(),
                group: None,
                own: false,
            }
        ]
    );
    server.on(
        "GET",
        "/rest/api/3/project/ATL/statuses",
        200,
        json!([
            { "name": "Story", "statuses": [{ "id": "1", "name": "To Do" }, { "id": "3", "name": "In Progress" }] },
            { "name": "Bug", "statuses": [{ "id": "3", "name": "In Progress" }, { "id": "10", "name": "In Review" }] }
        ]),
    );
    let names: Vec<String> = t
        .states("ATL")
        .await
        .unwrap()
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(names, ["To Do", "In Progress", "In Review"]);
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_search_builds_its_jql_and_reads_the_issues() {
    let Rig { server, t, rt, .. } = jira().await;
    server.on(
        "POST",
        "/rest/api/3/search/jql",
        200,
        json!({ "issues": [{
            "id": "10001", "key": "ATL-1287",
            "fields": {
                "summary": "Rafraîchir le token",
                "issuetype": { "name": "Task" },
                "priority": { "name": "High" },
                "assignee": null,
                "status": { "name": "To Do" },
                "description": { "type": "doc", "version": 1, "content": [
                    { "type": "paragraph", "content": [{ "type": "text", "text": "Acceptance criteria:" }] },
                    { "type": "bulletList", "content": [
                        { "type": "listItem", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Refresh avant expiration" }] }] },
                        { "type": "listItem", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Tests unitaires" }] }] }
                    ]}
                ]}
            }
        }]}),
    );
    let q = Query {
        text: "token \"refresh\"+".into(),
        filters: vec!["mine".into(), "sprint".into(), "todo".into()],
        label: Some("claude-ready".into()),
        ..Default::default()
    };
    let page = t.issues("ATL", &q).await.unwrap();
    let sent = server.requests()[0].json();
    assert_eq!(
        sent["jql"],
        "project = \"ATL\" AND statusCategory != Done AND assignee = currentUser() AND sprint in openSprints() \
         AND statusCategory = \"To Do\" AND labels = \"claude-ready\" AND text ~ \"token refresh\" ORDER BY updated DESC"
    );
    assert!(sent["fields"]
        .as_array()
        .unwrap()
        .contains(&json!("description")));
    let i = &page.issues[0];
    // Known by its id, which stays when it moves to another project; shown by its key.
    assert_eq!(
        (i.id.as_str(), i.key.as_str(), i.kind.as_str()),
        ("10001", "ATL-1287", "Tâche")
    );
    assert_eq!(i.kind_code, "task");
    assert_eq!(i.meta, ["Haute", "Non assigné", "To Do"]);
    assert_eq!(i.url, format!("{}/browse/ATL-1287", server.url));
    assert_eq!(i.criteria, ["Refresh avant expiration", "Tests unitaires"]);
    assert!(i
        .description
        .starts_with("Acceptance criteria:\n\n- Refresh"));
    assert_eq!(i.container, "ATL");
    let filters: Vec<&str> = page.filters.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(filters, ["mine", "sprint", "todo"]);
    // A key is looked for as such.
    t.issues(
        "ATL",
        &Query {
            text: "atl-12".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        server.requests()[1].json()["jql"],
        "project = \"ATL\" AND statusCategory != Done AND key = \"ATL-12\" ORDER BY updated DESC"
    );
    server.on(
        "POST",
        "/rest/api/3/search/jql",
        400,
        json!({ "errorMessages": ["Field 'sprint' does not exist."] }),
    );
    assert_eq!(
        t.issues("ATL", &q).await.unwrap_err().to_string(),
        "Jira : erreur 400 — Field 'sprint' does not exist."
    );
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_search_goes_on_from_its_next_page_and_says_about_how_many_match() {
    let Rig { server, t, rt, .. } = jira().await;
    let page = |keys: &[&str], next: Option<&str>| {
        let issues: Vec<Value> = keys
            .iter()
            .map(|k| json!({ "id": k, "key": k, "fields": { "summary": k } }))
            .collect();
        match next {
            Some(token) => json!({ "issues": issues, "nextPageToken": token, "isLast": false }),
            None => json!({ "issues": issues, "isLast": true }),
        }
    };
    server.on(
        "POST",
        "/rest/api/3/search/jql",
        200,
        page(&["ATL-1", "ATL-2"], Some("p2")),
    );
    server.on(
        "POST",
        "/rest/api/3/search/approximate-count",
        200,
        json!({ "count": 312 }),
    );
    let first = t.issues("ATL", &Query::default()).await.unwrap();
    assert_eq!(
        (first.next.as_deref(), first.total),
        (Some("p2"), Some(312))
    );
    let sent = server.requests();
    assert_eq!(sent[0].json()["maxResults"], 50);
    assert!(sent[0].json().get("nextPageToken").is_none());
    // Counted on the same issues, without their order.
    assert_eq!(sent[1].path(), "/rest/api/3/search/approximate-count");
    assert_eq!(
        sent[1].json()["jql"],
        "project = \"ATL\" AND statusCategory != Done"
    );
    server.on(
        "POST",
        "/rest/api/3/search/jql",
        200,
        page(&["ATL-3"], None),
    );
    let q = Query {
        page: Some("p2".into()),
        ..Default::default()
    };
    let last = t.issues("ATL", &q).await.unwrap();
    assert_eq!(server.requests()[2].json()["nextPageToken"], "p2");
    assert_eq!(last.issues[0].key, "ATL-3");
    // The last page: nothing more, nothing to count.
    assert_eq!((last.next, last.total), (None, None));
    assert_eq!(server.requests().len(), 3);
    // A count refused only leaves the total unknown.
    server.on(
        "POST",
        "/rest/api/3/search/jql",
        200,
        page(&["ATL-1"], Some("p2")),
    );
    server.on(
        "POST",
        "/rest/api/3/search/approximate-count",
        404,
        Value::Null,
    );
    let again = t.issues("ATL", &Query::default()).await.unwrap();
    assert_eq!((again.next.as_deref(), again.total), (Some("p2"), None));
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_state_is_reached_by_the_transition_that_leads_to_it() {
    let Rig { server, t, rt, .. } = jira().await;
    let review = ExternalState {
        id: "10".into(),
        name: "In Review".into(),
    };
    server.on(
        "GET",
        "/rest/api/3/issue/ATL-1/transitions",
        200,
        json!({ "transitions": [
            { "id": "21", "to": { "id": "3", "name": "In Progress" } },
            { "id": "31", "to": { "id": "10", "name": "In Review" } }
        ]}),
    );
    server.on(
        "POST",
        "/rest/api/3/issue/ATL-1/transitions",
        204,
        Value::Null,
    );
    // Already there: nothing moves.
    server.on(
        "GET",
        "/rest/api/3/issue/ATL-1",
        200,
        json!({ "fields": { "status": { "id": "10", "name": "In Review" } } }),
    );
    t.set_state(&r("ATL-1"), &review, &[]).await.unwrap();
    assert!(server.writes().is_empty());
    server.on(
        "GET",
        "/rest/api/3/issue/ATL-1",
        200,
        json!({ "fields": { "status": { "id": "3", "name": "In Progress" } } }),
    );
    t.set_state(&r("ATL-1"), &review, &[]).await.unwrap();
    let w = server.writes();
    assert_eq!(w[0].target, "/rest/api/3/issue/ATL-1/transitions");
    assert_eq!(w[0].json(), json!({ "transition": { "id": "31" } }));
    // By its name when its id differs (another workflow).
    let named = ExternalState {
        id: "99".into(),
        name: "in progress".into(),
    };
    server.on(
        "GET",
        "/rest/api/3/issue/ATL-1",
        200,
        json!({ "fields": { "status": { "id": "1", "name": "To Do" } } }),
    );
    t.set_state(&r("ATL-1"), &named, &[]).await.unwrap();
    assert_eq!(
        server.writes()[1].json(),
        json!({ "transition": { "id": "21" } })
    );
    // Already there by its name: nothing moves.
    server.on(
        "GET",
        "/rest/api/3/issue/ATL-1",
        200,
        json!({ "fields": { "status": { "id": "3", "name": "In Progress" } } }),
    );
    t.set_state(&r("ATL-1"), &named, &[]).await.unwrap();
    assert_eq!(server.writes().len(), 2);
    let done = ExternalState {
        id: "5".into(),
        name: "Done".into(),
    };
    let err = t.set_state(&r("ATL-1"), &done, &[]).await.unwrap_err();
    assert_eq!(
        err.to_string(),
        "Jira : aucune transition de « In Progress » vers « Done » pour ATL-1"
    );
    // Asking again would not change it.
    assert!(crate::integrations::lasting(&err));
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_comment_is_a_document() {
    let Rig { server, t, rt, .. } = jira().await;
    server.on(
        "POST",
        "/rest/api/3/issue/ATL-1/comment",
        201,
        json!({ "id": "c1" }),
    );
    t.comment(&r("ATL-1"), "Escouade : prêt\n\n✓ un")
        .await
        .unwrap();
    let body = server.writes()[0].json();
    assert_eq!(body["body"]["type"], "doc");
    assert_eq!(body["body"]["content"][1]["content"][0]["text"], "✓ un");
    rt.shutdown();
}

/// The texts are the interface's: the plugin is told the language when it starts (`ESCOUADE_LANG`, as the core tells
/// the plugins it gives), and says its filters and its errors in it.
#[tokio::test(flavor = "multi_thread")]
async fn the_plugin_says_its_texts_in_the_language_it_is_started_in() {
    let Rig { server, rt, t, .. } = jira().await;
    rt.set_builtin_env(vec![("ESCOUADE_LANG".into(), "en".into())]);
    server.on(
        "POST",
        "/rest/api/3/search/jql",
        200,
        json!({ "issues": [], "isLast": true }),
    );
    let page = t.issues("ATL", &Query::default()).await.unwrap();
    assert_eq!(page.filters[0].label, "Assigned to me");
    server.on("GET", "/rest/api/3/myself", 401, Value::Null);
    assert_eq!(
        t.me().await.unwrap_err().to_string(),
        "Jira refuses these credentials (401)"
    );
    rt.shutdown();
}

/// The token is not in what Escouade sends: the plugin asks for it, under its own name (`account:jira`, whatever the
/// site is).
#[tokio::test(flavor = "multi_thread")]
async fn the_token_is_asked_by_the_plugin_and_never_sent() {
    let Rig { server, t, rt, .. } = jira().await;
    server.on(
        "GET",
        "/rest/api/3/myself",
        200,
        json!({ "accountId": "acc-1", "displayName": "Ada" }),
    );
    t.me().await.unwrap();
    let journal = format!("{:?}", rt.journal(ID));
    assert!(!journal.contains("tok"), "{journal}");
    assert!(journal.contains("account:jira"), "{journal}");
    rt.shutdown();
}

/// Turned off for the app, or stopped, the plugin does not answer and the call says so: the sync tries again later.
#[tokio::test(flavor = "multi_thread")]
async fn a_plugin_that_is_off_or_stopped_is_said_so() {
    let Rig {
        server,
        t,
        rt,
        plugins,
    } = jira().await;
    server.on(
        "GET",
        "/rest/api/3/myself",
        200,
        json!({ "accountId": "acc-1", "displayName": "Ada" }),
    );
    t.me().await.unwrap();
    // Stopped (by the user, in its page): it says so.
    rt.stop(ID);
    let err = t.me().await.unwrap_err();
    assert!(is_down(&err), "{err}");
    assert!(err.to_string().contains(ID), "{err}");
    assert!(err.to_string().contains("stopped"), "{err}");
    // Turned off for the app: it does not start, and says so.
    rt.forget(ID);
    plugins.choose(ID, |c| c.enabled = false).unwrap();
    let err = t.me().await.unwrap_err();
    assert!(is_down(&err), "{err}");
    assert!(err.to_string().contains("switched off"), "{err}");
    plugins.choose(ID, |c| c.enabled = true).unwrap();
    assert!(
        t.me().await.unwrap().0.starts_with("Ada"),
        "on again, it answers"
    );
    rt.shutdown();
}

#[test]
fn jira_is_a_plugin_of_the_app_that_the_registry_of_sources_names() {
    let kind = source::registry().get(S).expect("the core knows its name");
    assert_eq!(kind.plugin(), Some(ID));
    assert_eq!((kind.label(), kind.short()), ("Jira", "Jira"));
    let plugins = Plugins::load_in(
        DataDir::new(
            std::env::temp_dir()
                .join(format!("plugins-jira-{}", uuid::Uuid::new_v4().simple()))
                .join("data"),
        ),
        VERSION,
        app_dir(),
    );
    let v = plugins.get(ID, Lang::En).expect("the registry gives it");
    assert_eq!(v.origin, Origin::Builtin);
    let m = v.manifest.unwrap();
    assert_eq!(m.run.unwrap().wasm.as_deref(), Some("Cargo.toml"));
    assert_eq!(m.permissions.network, ["*.atlassian.net", "{account}"]);
    assert_eq!(m.permissions.secrets, ["account:jira"]);
}

/// The texts of the form's missing fields are the plugin's (its manifest), in the order of the form: the site, the
/// e-mail, the token.
#[test]
fn what_the_form_lacks_is_said_by_the_manifest_of_the_plugin() {
    use crate::plugins::builtin::missing;
    let given = |site: &'static str, email: &'static str, token: &'static str| {
        move |field: &str| match field {
            "site" => site.to_string(),
            "email" => email.to_string(),
            "token" => token.to_string(),
            _ => String::new(),
        }
    };
    let said = |lang, f| missing(ID, "jira", f, lang);
    assert_eq!(
        said(Lang::Fr, given("", "", "")).as_deref(),
        Some("Indique l'adresse du site.")
    );
    assert_eq!(
        said(Lang::En, given("", "", "")).as_deref(),
        Some("Give the site’s address.")
    );
    assert_eq!(
        said(Lang::Fr, given("https://x", "", "t")).as_deref(),
        Some("Indique l'e-mail.")
    );
    assert_eq!(
        said(Lang::En, given("https://x", "a@b.c", "")).as_deref(),
        Some("Give the token.")
    );
    assert_eq!(said(Lang::Fr, given("https://x", "a@b.c", "t")), None);
}
