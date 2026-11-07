//! Tests of ClickUp as a plugin of the app (INT-7.4, ESC-216; `plugins/clickup`): the tests the
//! core had for its own ClickUp client (`integrations/clickup.rs`), word for word, now played against the plugin
//! started as the app starts it (its module, in the host, as `scripts/plugins-wasm.sh` built it) by the runtime
//! and `ProcessTicketSource`, on the fake ClickUp of the integrations' tests; and what a process adds: the token
//! fetched by the plugin and never sent, the plugin that is off or stopped saying so, the offer of the plugin to the
//! registry of sources, the text of the form read in its manifest. The pure parts of the client (the ids a text
//! names, the defaults by type, the wait) are tested in the plugin's own crate.

use crate::i18n::Lang;
use crate::integrations::fake::FakeServer;
use crate::integrations::lasting;
use crate::integrations::secrets::{MemoryStore, SecretStore};
use crate::integrations::source::{self, TicketSource};
use crate::integrations::{Account, HttpError, Query};
use crate::model::{Column, ExternalRef, ExternalState, Service};
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

const ID: &str = "dev.escouade.clickup";
const VERSION: &str = "2.2.0";
const S: Service = Service::CLICKUP;

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

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// ClickUp connected to a fake server: the account's user as the core tells it, its token ` pk_42_k1 ` (as typed) in
/// the keychain's name for the plugin, the API where the core says it is.
async fn clickup() -> Rig {
    let server = FakeServer::start().await;
    let dir =
        std::env::temp_dir().join(format!("plugins-clickup-{}", uuid::Uuid::new_v4().simple()));
    let data = DataDir::new(dir.join("data"));
    let plugins = Plugins::load_in(data.clone(), VERSION, app_dir());
    let store = Arc::new(MemoryStore::default());
    store
        .set(&secret_name(ID, "account:clickup"), " pk_42_k1 ")
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
    let account = Account {
        user: "183".into(),
        ..Default::default()
    };
    rt.set_net(Net {
        routes: HashMap::from([("api.clickup.com".to_string(), server.url.clone())]),
        dns: None,
        ..Net::default()
    });
    let t = ProcessTicketSource::new(rt.clone(), ID, "clickup", &account).with_api(&server.url);
    Rig {
        server,
        rt,
        plugins,
        t,
    }
}

/// The list l1 « Sprint 12 » and its statuses, out of order as ClickUp may give them.
pub(crate) fn list(server: &FakeServer) {
    server.on(
        "GET",
        "/list/l1",
        200,
        json!({ "id": "l1", "name": "Sprint 12", "statuses": [
            { "id": "p1_c", "status": "complete", "orderindex": 4, "type": "closed" },
            { "id": "p1_r", "status": "in review", "orderindex": 2, "type": "custom" },
            { "id": "p1_o", "status": "to do", "orderindex": 0, "type": "open" },
            { "id": "p1_p", "status": "in progress", "orderindex": 1, "type": "custom" },
            { "id": "p1_d", "status": "shipped", "orderindex": 3, "type": "done" }
        ] }),
    );
}

pub(crate) fn task(id: &str, custom: Option<&str>, name: &str, list: &str) -> Value {
    json!({
        "id": id, "custom_id": custom, "name": name,
        "markdown_description": "Exporter le journal.\n\n![capture](https://t.clickup.com/a.png)",
        "description": "Exporter le journal.",
        "status": { "status": "to do", "type": "open" },
        "priority": { "priority": "high" },
        "assignees": [{ "id": 183, "username": "Ada Lovelace" }],
        "tags": [{ "name": "claude-ready" }],
        "checklists": [
            { "name": "Checklist", "items": [{ "name": "Relire", "orderindex": 0 }] },
            { "name": "Critères d'acceptation", "items": [
                { "name": "Le fichier s'ouvre dans Excel", "orderindex": 1, "resolved": false },
                { "name": "Un bouton Exporter", "orderindex": 0, "resolved": false }
            ] }
        ],
        "url": format!("https://app.clickup.com/t/{id}"),
        "list": { "id": list }
    })
}

fn task_ref() -> ExternalRef {
    ExternalRef {
        service: S,
        id: "86abx3".into(),
        key: "ABC-12".into(),
        container: "l1".into(),
        ..Default::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_account_is_the_user_the_key_belongs_to_and_the_key_goes_as_it_is() {
    let Rig { server, t, rt, .. } = clickup().await;
    server.on(
        "GET",
        "/user",
        200,
        json!({ "user": { "id": 183, "username": "Ada Lovelace", "email": "ada@atlas.dev" } }),
    );
    assert_eq!(
        t.me().await.unwrap(),
        ("Ada Lovelace".to_string(), "183".to_string())
    );
    assert_eq!(server.requests()[0].headers["authorization"], "pk_42_k1");
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_key_refused_reads_as_credentials_refused_and_can_be_put_right() {
    let Rig { server, t, rt, .. } = clickup().await;
    server.on(
        "GET",
        "/user",
        401,
        json!({ "err": "Token invalid", "ECODE": "OAUTH_025" }),
    );
    let e = t.me().await.unwrap_err();
    assert_eq!(e.downcast_ref::<HttpError>().unwrap().status, 401);
    assert!(!lasting(&e));
    assert_eq!(
        e.to_string(),
        "ClickUp refuse ces identifiants (401) — Token invalid"
    );
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_limit_is_waited_for_until_its_reset_then_asked_again() {
    let Rig { server, t, rt, .. } = clickup().await;
    server.on(
        "GET",
        "/user",
        200,
        json!({ "user": { "id": 183, "username": "Ada" } }),
    );
    let now = (now_ms() / 1000).to_string();
    server.on_once(
        "GET",
        "/user",
        429,
        json!({ "err": "Rate limit reached", "ECODE": "APP_002" }),
        &[("X-RateLimit-Reset", &now)],
    );
    assert_eq!(t.me().await.unwrap().0, "Ada");
    assert_eq!(server.requests().len(), 2);

    // Limited again after the wait: the limit is said, and may pass later.
    for _ in 0..2 {
        server.on_once(
            "GET",
            "/user",
            429,
            Value::Null,
            &[("X-RateLimit-Reset", &now)],
        );
    }
    let e = t.me().await.unwrap_err();
    assert_eq!(e.downcast_ref::<HttpError>().unwrap().status, 429);
    assert!(!lasting(&e));
    assert!(e.to_string().contains("limite les requêtes (429)"), "{e}");
    assert_eq!(server.requests().len(), 4);

    // A reset too far ahead is not waited for.
    let later = (now_ms() / 1000 + 3600).to_string();
    server.on_once(
        "GET",
        "/user",
        429,
        Value::Null,
        &[("X-RateLimit-Reset", &later)],
    );
    assert!(t.me().await.is_err());
    assert_eq!(server.requests().len(), 5);
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_lists_come_in_their_place_in_the_hierarchy() {
    let Rig { server, t, rt, .. } = clickup().await;
    server.on(
        "GET",
        "/team",
        200,
        json!({ "teams": [{ "id": "9001", "name": "Atlas" }] }),
    );
    server.on(
        "GET",
        "/team/9001/space",
        200,
        json!({ "spaces": [{ "id": 790, "name": "Produit" }] }),
    );
    server.on(
        "GET",
        "/space/790/folder",
        200,
        json!({ "folders": [{ "id": "457", "name": "Sprints", "lists": [
            { "id": "l1", "name": "Sprint 12" }, { "id": "l2", "name": "Sprint 13" }
        ] }] }),
    );
    server.on(
        "GET",
        "/space/790/list",
        200,
        json!({ "lists": [{ "id": "l0", "name": "Backlog" }] }),
    );
    let lists = t.containers().await.unwrap();
    let seen: Vec<(&str, &str, &str)> = lists
        .iter()
        .map(|l| {
            (
                l.id.as_str(),
                l.name.as_str(),
                l.group.as_deref().unwrap_or(""),
            )
        })
        .collect();
    assert_eq!(
        seen,
        [
            ("l0", "Backlog", "Produit"),
            ("l1", "Sprint 12", "Produit › Sprints"),
            ("l2", "Sprint 13", "Produit › Sprints"),
        ]
    );
    // Archived ones are left out.
    assert!(server
        .requests()
        .iter()
        .filter(|r| r.path() != "/team")
        .all(|r| r.query("archived").as_deref() == Some("false")));

    // Several workspaces: each list says its own.
    server.on(
        "GET",
        "/team",
        200,
        json!({ "teams": [{ "id": "9001", "name": "Atlas" }, { "id": "9002", "name": "Perso" }] }),
    );
    server.on("GET", "/team/9002/space", 200, json!({ "spaces": [] }));
    let lists = t.containers().await.unwrap();
    assert_eq!(lists[1].group.as_deref(), Some("Atlas › Produit › Sprints"));
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_statuses_are_the_lists_own_in_its_order_and_the_defaults_follow_their_names() {
    let Rig { server, t, rt, .. } = clickup().await;
    list(&server);
    let v = t.states_view("l1").await.unwrap();
    let names: Vec<&str> = v.states.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        names,
        ["to do", "in progress", "in review", "shipped", "complete"]
    );
    assert_eq!(v.defaults[&Column::Doing].name, "in progress");
    assert_eq!(v.defaults[&Column::Review].name, "in progress");
    assert_eq!(v.defaults[&Column::Done].name, "in review");
    assert!(!v.defaults.contains_key(&Column::Todo));
    // Its own view: the states alone are the same ones.
    assert_eq!(t.states("l1").await.unwrap(), v.states);
    rt.shutdown();
}

/// ClickUp has its own view of the states: statuses whose names say nothing get their defaults by type.
#[tokio::test(flavor = "multi_thread")]
async fn statuses_whose_names_say_nothing_get_their_defaults_by_type() {
    let Rig { server, t, rt, .. } = clickup().await;
    server.on(
        "GET",
        "/list/l1",
        200,
        json!({ "id": "l1", "name": "Sprint 12", "statuses": [
            { "status": "ouvert", "orderindex": 0, "type": "open" },
            { "status": "chez Ada", "orderindex": 1, "type": "custom" },
            { "status": "chez Bob", "orderindex": 2, "type": "custom" },
            { "status": "fini", "orderindex": 3, "type": "closed" }
        ] }),
    );
    let v = t.states_view("l1").await.unwrap();
    assert_eq!(v.defaults[&Column::Doing].name, "chez Ada");
    assert_eq!(v.defaults[&Column::Review].name, "chez Ada");
    assert_eq!(v.defaults[&Column::Done].name, "chez Bob");
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_tasks_come_with_their_checklist_as_criteria_and_the_filters_go_to_clickup() {
    let Rig { server, t, rt, .. } = clickup().await;
    list(&server);
    server.on(
        "GET",
        "/list/l1/task",
        200,
        json!({ "tasks": [
            task("86abx3", Some("ABC-12"), "Exporter le journal", "l1"),
            { "id": "86zz", "name": "Fini", "status": { "status": "shipped", "type": "done" }, "list": { "id": "l1" } }
        ], "last_page": false }),
    );
    let q = Query {
        filters: vec!["mine".into(), "status:in progress".into()],
        label: Some("Claude-Ready".into()),
        page: Some("1".into()),
        ..Default::default()
    };
    let page = t.issues("l1", &q).await.unwrap();
    assert_eq!(page.issues.len(), 1);
    let i = &page.issues[0];
    assert_eq!(
        (i.id.as_str(), i.key.as_str(), i.kind_code.as_str()),
        ("86abx3", "ABC-12", "task")
    );
    assert_eq!(i.service, S);
    assert_eq!(i.meta, ["to do", "high", "Ada Lovelace", "claude-ready"]);
    assert_eq!(
        i.criteria,
        ["Un bouton Exporter", "Le fichier s'ouvre dans Excel"]
    );
    assert!(i
        .description
        .contains("![capture](https://t.clickup.com/a.png)"));
    assert_eq!(i.container, "l1");
    assert_eq!(i.url, "https://app.clickup.com/t/86abx3");
    assert_eq!(page.next.as_deref(), Some("2"));
    let chips: Vec<&str> = page.filters.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(
        chips,
        [
            "mine",
            "status:to do",
            "status:in progress",
            "status:in review"
        ]
    );
    assert_eq!(page.filters[0].label, "Mes tâches");
    let sent = server
        .requests()
        .into_iter()
        .find(|r| r.path() == "/list/l1/task")
        .unwrap();
    assert_eq!(sent.query("page").as_deref(), Some("1"));
    assert_eq!(sent.query("assignees[]").as_deref(), Some("183"));
    assert_eq!(sent.query("statuses[]").as_deref(), Some("in progress"));
    assert_eq!(sent.query("tags[]").as_deref(), Some("claude-ready"));
    assert_eq!(sent.query("include_closed").as_deref(), Some("false"));
    assert_eq!(
        sent.query("include_markdown_description").as_deref(),
        Some("true")
    );
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_search_by_title_reads_the_pages_until_it_finds() {
    let Rig { server, t, rt, .. } = clickup().await;
    list(&server);
    // The last route set answers first.
    server.on_once(
        "GET",
        "/list/l1/task",
        200,
        json!({ "tasks": [task("86b", None, "Mode sombre du site", "l1")], "last_page": false }),
        &[],
    );
    server.on_once(
        "GET",
        "/list/l1/task",
        200,
        json!({ "tasks": [task("86a", None, "Exporter le journal", "l1")], "last_page": false }),
        &[],
    );
    let q = Query {
        text: "SOMBRE".into(),
        ..Default::default()
    };
    let page = t.issues("l1", &q).await.unwrap();
    let keys: Vec<&str> = page.issues.iter().map(|i| i.key.as_str()).collect();
    assert_eq!(keys, ["#86b"]);
    assert_eq!(page.next.as_deref(), Some("2"));
    let pages: Vec<String> = server
        .requests()
        .iter()
        .filter(|r| r.path() == "/list/l1/task")
        .filter_map(|r| r.query("page"))
        .collect();
    assert_eq!(pages, ["0", "1"]);
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_task_is_found_by_its_id_or_custom_id_in_the_list_only() {
    let Rig { server, t, rt, .. } = clickup().await;
    list(&server);
    server.on(
        "GET",
        "/task/86abx3",
        200,
        task("86abx3", Some("ABC-12"), "Exporter le journal", "l1"),
    );
    let by_id = |text: &str| Query {
        text: text.into(),
        ..Default::default()
    };
    let page = t.issues("l1", &by_id("#86abx3")).await.unwrap();
    assert_eq!(page.issues[0].key, "ABC-12");
    assert_eq!(page.next, None);

    server.on(
        "GET",
        "/team",
        200,
        json!({ "teams": [{ "id": 9001, "name": "Atlas" }] }),
    );
    server.on(
        "GET",
        "/task/ABC-12",
        200,
        task("86abx3", Some("ABC-12"), "Exporter le journal", "l1"),
    );
    let page = t.issues("l1", &by_id("ABC-12")).await.unwrap();
    assert_eq!(page.issues[0].id, "86abx3");
    let asked = server
        .requests()
        .into_iter()
        .find(|r| r.path() == "/task/ABC-12")
        .unwrap();
    assert_eq!(asked.query("custom_task_ids").as_deref(), Some("true"));
    assert_eq!(asked.query("team_id").as_deref(), Some("9001"));

    // In another list: not this project's.
    server.on(
        "GET",
        "/task/ABC-13",
        200,
        task("86other", Some("ABC-13"), "Ailleurs", "l9"),
    );
    server.on(
        "GET",
        "/list/l1/task",
        200,
        json!({ "tasks": [], "last_page": true }),
    );
    assert!(t
        .issues("l1", &by_id("ABC-13"))
        .await
        .unwrap()
        .issues
        .is_empty());
    // Unknown to ClickUp: searched in the names.
    server.on(
        "GET",
        "/task/zz9999",
        404,
        json!({ "err": "Task not found", "ECODE": "ITEM_013" }),
    );
    assert!(t
        .issues("l1", &by_id("zz9999"))
        .await
        .unwrap()
        .issues
        .is_empty());
    assert!(server
        .requests()
        .iter()
        .any(|r| r.path() == "/list/l1/task"));
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_list_gone_or_out_of_reach_says_so_and_lasts() {
    let Rig { server, t, rt, .. } = clickup().await;
    server.on(
        "GET",
        "/list/l1",
        404,
        json!({ "err": "List not found", "ECODE": "ITEM_013" }),
    );
    let e = t.issues("l1", &Query::default()).await.unwrap_err();
    assert!(e.to_string().contains("n'existe plus"), "{e}");
    assert!(lasting(&e));

    // Refused while the key is good: the list left the key's reach.
    server.on(
        "GET",
        "/list/l1",
        401,
        json!({ "err": "Team not authorized", "ECODE": "OAUTH_027" }),
    );
    server.on("GET", "/user", 200, json!({ "user": { "id": 183 } }));
    let e = t.states("l1").await.unwrap_err();
    assert!(e.to_string().contains("n'est plus accessible"), "{e}");
    assert!(lasting(&e));

    // Refused with the key: the key's refusal.
    server.on(
        "GET",
        "/user",
        401,
        json!({ "err": "Token invalid", "ECODE": "OAUTH_025" }),
    );
    let e = t.states("l1").await.unwrap_err();
    assert_eq!(e.downcast_ref::<HttpError>().unwrap().status, 401);
    assert!(!lasting(&e));
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_status_and_a_comment_are_written_on_the_task() {
    let Rig { server, t, rt, .. } = clickup().await;
    list(&server);
    server.on("PUT", "/task/86abx3", 200, json!({ "id": "86abx3" }));
    server.on(
        "POST",
        "/task/86abx3/comment",
        200,
        json!({ "id": 458, "hist_id": "26508", "date": 1568036964079_i64 }),
    );
    let state = ExternalState {
        id: "In Progress".into(),
        name: "In Progress".into(),
    };
    t.set_state(&task_ref(), &state, &[]).await.unwrap();
    t.comment(&task_ref(), "Escouade : ABC-12 est prêt à tester.")
        .await
        .unwrap();
    let writes: Vec<Value> = server.writes().iter().map(|w| w.json()).collect();
    // The list's own spelling.
    assert_eq!(writes[0], json!({ "status": "in progress" }));
    assert_eq!(
        writes[1],
        json!({ "comment_text": "Escouade : ABC-12 est prêt à tester.", "notify_all": false })
    );
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_status_the_list_lacks_is_said_and_not_sent() {
    let Rig { server, t, rt, .. } = clickup().await;
    list(&server);
    let gone = ExternalState {
        id: "qa".into(),
        name: "qa".into(),
    };
    let e = t.set_state(&task_ref(), &gone, &[]).await.unwrap_err();
    assert!(lasting(&e));
    assert_eq!(
        e.to_string(),
        "La liste « Sprint 12 » de ClickUp n'a pas le statut « qa » : ABC-12 reste où il est ; choisis un autre statut dans Réglages › Intégrations"
    );
    assert!(server.writes().is_empty());
    rt.shutdown();
}

/// The texts are the interface's: the plugin is told the language when it starts (`ESCOUADE_LANG`, as the core tells
/// the plugins it gives), and says its filters, its kinds and its errors in it.
#[tokio::test(flavor = "multi_thread")]
async fn the_plugin_says_its_texts_in_the_language_it_is_started_in() {
    let Rig { server, rt, t, .. } = clickup().await;
    rt.set_builtin_env(vec![("ESCOUADE_LANG".into(), "en".into())]);
    list(&server);
    server.on(
        "GET",
        "/list/l1/task",
        200,
        json!({ "tasks": [task("86abx3", None, "Export", "l1")], "last_page": true }),
    );
    let page = t.issues("l1", &Query::default()).await.unwrap();
    assert_eq!(page.filters[0].label, "My tasks");
    assert_eq!(page.issues[0].kind, "Task");
    server.on("GET", "/user", 401, Value::Null);
    assert_eq!(
        t.me().await.unwrap_err().to_string(),
        "ClickUp refuses these credentials (401)"
    );
    rt.shutdown();
}

/// The token is not in what Escouade sends: the plugin asks for it, under its own name (`account:clickup`).
#[tokio::test(flavor = "multi_thread")]
async fn the_token_is_asked_by_the_plugin_and_never_sent() {
    let Rig { server, t, rt, .. } = clickup().await;
    server.on(
        "GET",
        "/user",
        200,
        json!({ "user": { "id": 183, "username": "Ada" } }),
    );
    t.me().await.unwrap();
    let journal = format!("{:?}", rt.journal(ID));
    assert!(!journal.contains("pk_42_k1"), "{journal}");
    assert!(journal.contains("account:clickup"), "{journal}");
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
    } = clickup().await;
    server.on(
        "GET",
        "/user",
        200,
        json!({ "user": { "id": 183, "username": "Ada" } }),
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
fn clickup_is_a_plugin_of_the_app_that_the_registry_of_sources_names() {
    let kind = source::registry().get(S).expect("the core knows its name");
    assert_eq!(kind.plugin(), Some(ID));
    assert_eq!((kind.label(), kind.short()), ("ClickUp", "ClickUp"));
    let plugins = Plugins::load_in(
        DataDir::new(
            std::env::temp_dir()
                .join(format!("plugins-clickup-{}", uuid::Uuid::new_v4().simple()))
                .join("data"),
        ),
        VERSION,
        app_dir(),
    );
    let v = plugins.get(ID, Lang::En).expect("the registry gives it");
    assert_eq!(v.origin, Origin::Builtin);
    let m = v.manifest.unwrap();
    assert_eq!(m.run.unwrap().wasm.as_deref(), Some("Cargo.toml"));
    assert_eq!(m.permissions.network, ["api.clickup.com"]);
    assert_eq!(m.permissions.secrets, ["account:clickup"]);
}

/// The text of the form's missing token is the plugin's (its manifest).
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
        missing(ID, "clickup", given(""), Lang::Fr).as_deref(),
        Some("Indique le jeton.")
    );
    assert_eq!(
        missing(ID, "clickup", given(""), Lang::En).as_deref(),
        Some("Give the token.")
    );
    assert_eq!(missing(ID, "clickup", given("pk_1"), Lang::Fr), None);
}

/// The account's user is known after « Connecter »: the client made without it does not serve the calls after it.
#[tokio::test(flavor = "multi_thread")]
async fn mine_filters_on_the_user_the_account_learns_after_connecting() {
    let Rig { server, rt, .. } = clickup().await;
    list(&server);
    server.on(
        "GET",
        "/list/l1/task",
        200,
        json!({ "tasks": [], "last_page": true }),
    );
    let q = Query {
        filters: vec!["mine".into()],
        ..Default::default()
    };
    let without = Account::default();
    let t = ProcessTicketSource::new(rt.clone(), ID, "clickup", &without).with_api(&server.url);
    t.issues("l1", &q).await.unwrap();
    let known = Account {
        user: "183".into(),
        ..Default::default()
    };
    let t = ProcessTicketSource::new(rt.clone(), ID, "clickup", &known).with_api(&server.url);
    t.issues("l1", &q).await.unwrap();
    let asked: Vec<Option<String>> = server
        .requests()
        .iter()
        .filter(|r| r.path() == "/list/l1/task")
        .map(|r| r.query("assignees[]"))
        .collect();
    assert_eq!(asked, [None, Some("183".to_string())]);
    rt.shutdown();
}
