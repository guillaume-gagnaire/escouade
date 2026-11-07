//! Tests of Linear as a plugin of the app (INT-7.2, ESC-214; `plugins/linear`): the tests the core
//! had for its own Linear client (`integrations/linear.rs`), word for word, now played against the plugin started as
//! the app starts it (its module, in the host, as `scripts/plugins-wasm.sh` built it) by the runtime and
//! `ProcessTicketSource`, on the fake Linear of the integrations' tests; and what a process adds: the key fetched by
//! the plugin and never sent, the plugin that is off or stopped saying so, the offer of the plugin to the registry of
//! sources.

use crate::i18n::Lang;
use crate::integrations::fake::FakeServer;
use crate::integrations::secrets::{MemoryStore, SecretStore};
use crate::integrations::source::{self, TicketSource};
use crate::integrations::{Account, Container, HttpError, Query};
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

const ID: &str = "dev.escouade.linear";
const VERSION: &str = "2.2.0";
const S: Service = Service::LINEAR;

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

/// Linear connected to a fake server: the account's key ` lin_api_k1 ` (as typed) in the keychain's name for the plugin.
async fn linear() -> Rig {
    let server = FakeServer::start().await;
    let dir =
        std::env::temp_dir().join(format!("plugins-linear-{}", uuid::Uuid::new_v4().simple()));
    let data = DataDir::new(dir.join("data"));
    let plugins = Plugins::load_in(data.clone(), VERSION, app_dir());
    let store = Arc::new(MemoryStore::default());
    store
        .set(&secret_name(ID, "account:linear"), " lin_api_k1 ")
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
        routes: HashMap::from([("api.linear.app".to_string(), server.url.clone())]),
        dns: None,
        ..Net::default()
    });
    let t = ProcessTicketSource::new(rt.clone(), ID, "linear", &Account::default())
        .with_api(&server.url);
    Rig {
        server,
        rt,
        plugins,
        t,
    }
}

fn workflow(server: &FakeServer) {
    server.on(
        "POST",
        "/graphql#States",
        200,
        json!({ "data": { "team": { "states": { "nodes": [
            { "id": "s-done", "name": "Done", "type": "completed", "position": 0 },
            { "id": "s-review", "name": "In Review", "type": "started", "position": 2 },
            { "id": "s-todo", "name": "Todo", "type": "unstarted", "position": 1 },
            { "id": "s-doing", "name": "In Progress", "type": "started", "position": 1 },
            { "id": "s-backlog", "name": "Backlog", "type": "backlog", "position": 0 },
            { "id": "s-canceled", "name": "Canceled", "type": "canceled", "position": 0 }
        ] } } } }),
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_account_is_who_the_key_belongs_to_and_the_key_goes_as_it_is() {
    let Rig { server, t, rt, .. } = linear().await;
    server.on(
        "POST",
        "/graphql#Me",
        200,
        json!({ "data": { "viewer": { "id": "u1", "name": "Ada", "email": "ada@example.com" } } }),
    );
    assert_eq!(
        t.me().await.unwrap(),
        ("ada@example.com".to_string(), "u1".to_string())
    );
    let req = &server.requests()[0];
    assert_eq!(req.headers["authorization"], "lin_api_k1");
    assert_eq!(req.json()["operationName"], "Me");
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_key_refused_reads_as_credentials_refused_and_can_be_put_right() {
    let Rig { server, t, rt, .. } = linear().await;
    server.on(
        "POST",
        "/graphql#Me",
        400,
        json!({ "errors": [{ "message": "Authentication required, not authenticated",
                             "extensions": { "code": "AUTHENTICATION_ERROR" } }] }),
    );
    let e = t.me().await.unwrap_err();
    assert_eq!(e.downcast_ref::<HttpError>().unwrap().status, 401);
    assert!(!crate::integrations::lasting(&e));
    assert!(
        e.to_string()
            .contains("Authentication required, not authenticated"),
        "{e}"
    );
    // An error of the query, answered with a 200: a request refused as it is.
    server.on(
        "POST",
        "/graphql#Teams",
        200,
        json!({ "data": null, "errors": [{ "message": "Argument Validation Error" }] }),
    );
    let e = t.containers().await.unwrap_err();
    assert!(crate::integrations::lasting(&e));
    assert!(e.to_string().contains("Argument Validation Error"), "{e}");
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_teams_are_the_containers_and_their_states_come_in_workflow_order() {
    let Rig { server, t, rt, .. } = linear().await;
    server.on(
        "POST",
        "/graphql#Teams",
        200,
        json!({ "data": { "teams": { "nodes": [{ "id": "t1", "key": "ENG", "name": "Engineering" }] } } }),
    );
    assert_eq!(
        t.containers().await.unwrap(),
        [Container {
            id: "t1".into(),
            name: "Engineering (ENG)".into(),
            group: None,
            own: false,
        }]
    );
    workflow(&server);
    let names: Vec<String> = t
        .states("t1")
        .await
        .unwrap()
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(
        names,
        [
            "Backlog",
            "Todo",
            "In Progress",
            "In Review",
            "Done",
            "Canceled"
        ]
    );
    // « En cours » and « À tester » by default, as for the other services.
    let defaults = crate::integrations::text::default_states(&t.states("t1").await.unwrap());
    use crate::model::Column;
    assert_eq!(defaults[&Column::Doing].name, "In Progress");
    assert_eq!(defaults[&Column::Done].name, "In Review");
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_issues_come_with_their_criteria_and_the_filters_go_to_linear() {
    let Rig { server, t, rt, .. } = linear().await;
    workflow(&server);
    server.on(
        "POST",
        "/graphql#Issues",
        200,
        json!({ "data": { "issues": {
            "nodes": [{
                "id": "i1", "identifier": "ENG-42", "title": "Exporter le journal",
                "description": "Export CSV.\n\n## Critères d'acceptation\n- Un bouton Exporter\n- Le fichier s'ouvre dans Excel",
                "url": "https://linear.app/atlas/issue/ENG-42", "priorityLabel": "High",
                "assignee": { "displayName": "ada" }, "state": { "id": "s-todo", "name": "Todo" },
                "labels": { "nodes": [{ "name": "claude-ready" }] }
            }],
            "pageInfo": { "hasNextPage": true, "endCursor": "c2" }
        } } }),
    );
    let q = Query {
        text: "eng-42".into(),
        filters: vec!["mine".into(), "state:s-todo".into()],
        label: Some("claude-ready".into()),
        page: Some("c1".into()),
    };
    let page = t.issues("t1", &q).await.unwrap();
    let i = &page.issues[0];
    assert_eq!((i.key.as_str(), i.kind_code.as_str()), ("ENG-42", "issue"));
    assert_eq!(i.service, S);
    assert_eq!(i.meta, ["Todo", "High", "ada", "claude-ready"]);
    assert_eq!(
        i.criteria,
        ["Un bouton Exporter", "Le fichier s'ouvre dans Excel"]
    );
    assert_eq!(i.container, "t1");
    assert_eq!(page.next.as_deref(), Some("c2"));
    // The chips: mine, then the states not done nor canceled.
    let chips: Vec<&str> = page.filters.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(
        chips,
        [
            "mine",
            "state:s-backlog",
            "state:s-todo",
            "state:s-doing",
            "state:s-review"
        ]
    );
    let sent = server
        .requests()
        .into_iter()
        .find(|r| r.json()["operationName"] == "Issues")
        .unwrap()
        .json();
    let f = &sent["variables"]["filter"];
    assert_eq!(f["team"]["id"]["eq"], "t1");
    assert_eq!(f["state"]["id"]["in"], json!(["s-todo"]));
    assert_eq!(f["state"]["type"]["nin"], json!(["completed", "canceled"]));
    assert_eq!(f["assignee"]["isMe"]["eq"], true);
    assert_eq!(f["labels"]["some"]["name"]["eqIgnoreCase"], "claude-ready");
    assert_eq!(f["number"]["eq"], 42);
    assert_eq!(sent["variables"]["after"], "c1");
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_state_and_a_comment_are_mutations_and_a_refusal_lasts() {
    let Rig { server, t, rt, .. } = linear().await;
    let r = ExternalRef {
        service: S,
        id: "i1".into(),
        key: "ENG-42".into(),
        ..Default::default()
    };
    let review = ExternalState {
        id: "s-review".into(),
        name: "In Review".into(),
    };
    server.on(
        "POST",
        "/graphql#SetState",
        200,
        json!({ "data": { "issueUpdate": { "success": true } } }),
    );
    server.on(
        "POST",
        "/graphql#Comment",
        200,
        json!({ "data": { "commentCreate": { "success": true } } }),
    );
    t.set_state(&r, &review, &[]).await.unwrap();
    t.comment(&r, "Escouade : ENG-42 est prêt à tester.")
        .await
        .unwrap();
    let writes: Vec<Value> = server.writes().iter().map(|w| w.json()).collect();
    assert_eq!(
        writes[0]["variables"],
        json!({ "id": "i1", "state": "s-review" })
    );
    assert_eq!(
        writes[1]["variables"]["body"],
        "Escouade : ENG-42 est prêt à tester."
    );
    server.on(
        "POST",
        "/graphql#SetState",
        200,
        json!({ "data": { "issueUpdate": { "success": false } } }),
    );
    let e = t.set_state(&r, &review, &[]).await.unwrap_err();
    assert!(crate::integrations::lasting(&e));
    rt.shutdown();
}

/// The texts are the interface's: the plugin is told the language when it starts (`ESCOUADE_LANG`, as the core tells
/// the plugins it gives), and says its filter and its errors in it.
#[tokio::test(flavor = "multi_thread")]
async fn the_plugin_says_its_texts_in_the_language_it_is_started_in() {
    let Rig { server, rt, t, .. } = linear().await;
    rt.set_builtin_env(vec![("ESCOUADE_LANG".into(), "en".into())]);
    workflow(&server);
    server.on(
        "POST",
        "/graphql#Issues",
        200,
        json!({ "data": { "issues": { "nodes": [], "pageInfo": { "hasNextPage": false, "endCursor": null } } } }),
    );
    let page = t.issues("t1", &Query::default()).await.unwrap();
    assert_eq!(page.filters[0].label, "My issues");
    server.on("POST", "/graphql#Me", 404, Value::String("gone".into()));
    assert_eq!(
        t.me().await.unwrap_err().to_string(),
        "Linear: not found (404) — gone"
    );
    rt.shutdown();
}

/// The key is not in what Escouade sends: the plugin asks for it, under its own name.
#[tokio::test(flavor = "multi_thread")]
async fn the_key_is_asked_by_the_plugin_and_never_sent() {
    let Rig { server, t, rt, .. } = linear().await;
    server.on(
        "POST",
        "/graphql#Me",
        200,
        json!({ "data": { "viewer": { "id": "u1", "name": "Ada" } } }),
    );
    t.me().await.unwrap();
    let journal = format!("{:?}", rt.journal(ID));
    assert!(!journal.contains("lin_api_k1"), "{journal}");
    assert!(journal.contains("account:linear"), "{journal}");
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
    } = linear().await;
    server.on(
        "POST",
        "/graphql#Me",
        200,
        json!({ "data": { "viewer": { "id": "u1", "name": "Ada" } } }),
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
    assert_eq!(t.me().await.unwrap().0, "Ada", "on again, it answers");
    rt.shutdown();
}

#[test]
fn linear_is_a_plugin_of_the_app_that_the_registry_of_sources_names() {
    let kind = source::registry().get(S).expect("the core knows its name");
    assert_eq!(kind.plugin(), Some(ID));
    assert_eq!((kind.label(), kind.short()), ("Linear", "Linear"));
    let plugins = Plugins::load_in(
        DataDir::new(
            std::env::temp_dir()
                .join(format!("plugins-linear-{}", uuid::Uuid::new_v4().simple()))
                .join("data"),
        ),
        VERSION,
        app_dir(),
    );
    let v = plugins.get(ID, Lang::En).expect("the registry gives it");
    assert_eq!(v.origin, Origin::Builtin);
    let m = v.manifest.unwrap();
    assert_eq!(m.run.unwrap().wasm.as_deref(), Some("Cargo.toml"));
    assert_eq!(m.permissions.network, ["api.linear.app"]);
    assert_eq!(m.permissions.secrets, ["account:linear"]);
}

/// The text of the form's missing key is the plugin's (its manifest).
#[test]
fn what_the_form_lacks_is_said_by_the_manifest_of_the_plugin() {
    use crate::plugins::builtin::missing;
    let given = |token: &'static str| {
        move |field: &str| match field {
            "token" => token.to_string(),
            _ => String::new(),
        }
    };
    assert_eq!(
        missing(ID, "linear", given(""), Lang::En).as_deref(),
        Some("Give the token.")
    );
    assert_eq!(
        missing(ID, "linear", given(""), Lang::Fr).as_deref(),
        Some("Indique le jeton.")
    );
    assert_eq!(missing(ID, "linear", given("k"), Lang::Fr), None);
}
