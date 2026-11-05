//! Tests of Trello as a plugin of the app (INT-7.1, ESC-213; `plugins/trello`): the tests the core
//! had for its own Trello client (`integrations/trello.rs`), word for word, now played against the plugin started as
//! the app starts it (its module, in the host, as `scripts/plugins-wasm.sh` built it) by the runtime and
//! `ProcessTicketSource`, on the fake Trello of the integrations' tests; and what a process adds: the key and the token
//! fetched by the plugin and never sent, the plugin that is off or stopped saying so, the offer of the plugin to the
//! registry of sources.

use crate::i18n::Lang;
use crate::integrations::fake::FakeServer;
use crate::integrations::secrets::{MemoryStore, SecretStore};
use crate::integrations::source::{self, TicketSource};
use crate::integrations::{Account, Container, IssuePage, Query};
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

const ID: &str = "dev.escouade.trello";
const VERSION: &str = "2.2.0";
const S: Service = Service::TRELLO;

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

/// Trello connected to a fake server: the account's key `k1` and token `t 1` in the keychain's name for the plugin,
/// who is "me".
async fn trello() -> Rig {
    let server = FakeServer::start().await;
    let dir =
        std::env::temp_dir().join(format!("plugins-trello-{}", uuid::Uuid::new_v4().simple()));
    let data = DataDir::new(dir.join("data"));
    let plugins = Plugins::load_in(data.clone(), VERSION, app_dir());
    let store = Arc::new(MemoryStore::default());
    store
        .set(
            &secret_name(ID, "account:trello"),
            r#"{"key":"k1","token":"t 1"}"#,
        )
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
        user: "me".into(),
        ..Default::default()
    };
    rt.set_net(Net {
        routes: HashMap::from([("api.trello.com".to_string(), server.url.clone())]),
        dns: None,
        ..Net::default()
    });
    let t = ProcessTicketSource::new(rt.clone(), ID, "trello", &account).with_api(&server.url);
    Rig {
        server,
        rt,
        plugins,
        t,
    }
}

pub(crate) fn board(server: &FakeServer) {
    server.on(
        "GET",
        "/boards/b1/lists",
        200,
        json!([{ "id": "l1", "name": "Prêt pour Claude" }, { "id": "l2", "name": "En cours" }, { "id": "l3", "name": "Recette" }]),
    );
    server.on(
        "GET",
        "/boards/b1/cards/open",
        200,
        json!([
            { "id": "c1", "idShort": 151, "name": "Exporter le journal", "desc": "Export CSV.", "idList": "l1",
              "shortUrl": "https://trello.com/c/abc", "labels": [{ "name": "claude-ready" }], "idMembers": ["me"] },
            { "id": "c2", "idShort": 156, "name": "Email de bienvenue", "desc": "## Critères\n- Template HTML", "idList": "l2",
              "shortUrl": "https://trello.com/c/def", "labels": [], "idMembers": [] }
        ]),
    );
    server.on(
        "GET",
        "/boards/b1/checklists",
        200,
        json!([
            { "id": "k1", "idCard": "c1", "name": "Tâches", "checkItems": [{ "name": "Préparer" }] },
            { "id": "k2", "idCard": "c1", "name": "Critères d'acceptation", "checkItems": [{ "name": "Filtre par date" }, { "name": "Colonnes IP" }] }
        ]),
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_key_and_token_go_with_every_call() {
    let Rig { server, t, rt, .. } = trello().await;
    server.on(
        "GET",
        "/members/me",
        200,
        json!({ "id": "me", "username": "ada" }),
    );
    assert_eq!(
        t.me().await.unwrap(),
        ("@ada".to_string(), "me".to_string())
    );
    let req = &server.requests()[0];
    assert_eq!(req.query("key").as_deref(), Some("k1"));
    assert_eq!(req.query("token").as_deref(), Some("t 1"));
    assert_eq!(req.query("fields").as_deref(), Some("username,fullName"));
    server.on(
        "GET",
        "/members/me",
        401,
        Value::String("invalid token".into()),
    );
    assert_eq!(
        t.me().await.unwrap_err().to_string(),
        "Trello refuse ces identifiants (401) — invalid token"
    );
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_boards_and_their_lists_are_listed() {
    let Rig { server, t, rt, .. } = trello().await;
    board(&server);
    server.on(
        "GET",
        "/members/me/boards",
        200,
        json!([{ "id": "b1", "name": "Atlas" }]),
    );
    assert_eq!(
        t.containers().await.unwrap(),
        [Container {
            id: "b1".into(),
            name: "Atlas".into(),
            group: None,
            own: false,
        }]
    );
    let lists: Vec<String> = t
        .states("b1")
        .await
        .unwrap()
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(lists, ["Prêt pour Claude", "En cours", "Recette"]);
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_cards_come_with_their_list_and_criteria_and_are_filtered() {
    let Rig { server, t, rt, .. } = trello().await;
    board(&server);
    let page = t.issues("b1", &Query::default()).await.unwrap();
    let c1 = &page.issues[0];
    assert_eq!(
        (c1.key.as_str(), c1.kind.as_str(), c1.id.as_str()),
        ("#151", "Carte", "c1")
    );
    assert_eq!(c1.kind_code, "card");
    assert_eq!(c1.meta, ["Prêt pour Claude", "claude-ready"]);
    assert_eq!(c1.criteria, ["Filtre par date", "Colonnes IP"]);
    assert_eq!(c1.url, "https://trello.com/c/abc");
    assert_eq!(c1.service, S);
    // Without a checklist: its description's.
    assert_eq!(page.issues[1].criteria, ["Template HTML"]);
    let ids: Vec<&str> = page.filters.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["mine", "list:l1", "list:l2", "list:l3"]);
    assert_eq!(page.filters[0].label, "Mes cartes");
    // A board's open cards come all at once: no next page.
    assert_eq!((&page.next, page.total), (&None, None));
    let keys = |p: IssuePage| p.issues.into_iter().map(|i| i.key).collect::<Vec<_>>();
    let q = |text: &str, filters: &[&str], label: Option<&str>| Query {
        text: text.into(),
        filters: filters.iter().map(|f| f.to_string()).collect(),
        label: label.map(str::to_string),
        ..Default::default()
    };
    assert_eq!(
        keys(t.issues("b1", &q("bienvenue", &[], None)).await.unwrap()),
        ["#156"]
    );
    assert_eq!(
        keys(t.issues("b1", &q("156", &[], None)).await.unwrap()),
        ["#156"]
    );
    assert_eq!(
        keys(t.issues("b1", &q("", &["mine"], None)).await.unwrap()),
        ["#151"]
    );
    assert_eq!(
        keys(
            t.issues("b1", &q("", &["list:l2", "list:l3"], None))
                .await
                .unwrap()
        ),
        ["#156"]
    );
    assert_eq!(
        keys(
            t.issues("b1", &q("", &[], Some("Claude-Ready")))
                .await
                .unwrap()
        ),
        ["#151"]
    );
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_card_moves_to_its_list_and_is_commented() {
    let Rig { server, t, rt, .. } = trello().await;
    server.on("PUT", "/cards/c1", 200, json!({}));
    server.on("POST", "/cards/c1/actions/comments", 200, json!({}));
    let r = ExternalRef {
        service: S,
        id: "c1".into(),
        ..Default::default()
    };
    t.set_state(
        &r,
        &ExternalState {
            id: "l3".into(),
            name: "Recette".into(),
        },
        &[],
    )
    .await
    .unwrap();
    t.comment(&r, "Escouade : prêt & fini").await.unwrap();
    let w = server.writes();
    assert_eq!(
        (w[0].method.as_str(), w[0].query("idList").as_deref()),
        ("PUT", Some("l3"))
    );
    // In the body: a long comment would not fit in an address.
    assert_eq!(w[1].json(), json!({ "text": "Escouade : prêt & fini" }));
    assert_eq!(w[1].query("text"), None);
    assert_eq!(w[1].query("token").as_deref(), Some("t 1"));
    rt.shutdown();
}

/// The texts are the interface's: the plugin is told the language when it starts (`ESCOUADE_LANG`, as the core tells
/// the plugins it gives), and says its card, its filter and its errors in it.
#[tokio::test(flavor = "multi_thread")]
async fn the_plugin_says_its_texts_in_the_language_it_is_started_in() {
    let Rig { server, rt, t, .. } = trello().await;
    rt.set_builtin_env(vec![("ESCOUADE_LANG".into(), "en".into())]);
    board(&server);
    let page = t.issues("b1", &Query::default()).await.unwrap();
    assert_eq!(page.issues[0].kind, "Card");
    assert_eq!(page.filters[0].label, "My cards");
    server.on("GET", "/members/me", 404, Value::String("gone".into()));
    assert_eq!(
        t.me().await.unwrap_err().to_string(),
        "Trello: not found (404) — gone"
    );
    rt.shutdown();
}

/// Neither the key nor the token is in what Escouade sends: the plugin asks for them, under its own name.
#[tokio::test(flavor = "multi_thread")]
async fn the_key_and_token_are_asked_by_the_plugin_and_never_sent() {
    let Rig { server, t, rt, .. } = trello().await;
    server.on(
        "GET",
        "/members/me",
        200,
        json!({ "id": "me", "username": "ada" }),
    );
    t.me().await.unwrap();
    let journal = format!("{:?}", rt.journal(ID));
    assert!(
        !journal.contains("k1") && !journal.contains("t 1"),
        "{journal}"
    );
    assert!(journal.contains("account:trello"), "{journal}");
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
    } = trello().await;
    server.on(
        "GET",
        "/members/me",
        200,
        json!({ "id": "me", "username": "ada" }),
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
    assert_eq!(t.me().await.unwrap().0, "@ada", "on again, it answers");
    rt.shutdown();
}

#[test]
fn trello_is_a_plugin_of_the_app_that_the_registry_of_sources_names() {
    let kind = source::registry().get(S).expect("the core knows its name");
    assert_eq!(kind.plugin(), Some(ID));
    assert_eq!((kind.label(), kind.short()), ("Trello", "Trello"));
    let plugins = Plugins::load_in(
        DataDir::new(
            std::env::temp_dir()
                .join(format!("plugins-trello-{}", uuid::Uuid::new_v4().simple()))
                .join("data"),
        ),
        VERSION,
        app_dir(),
    );
    let v = plugins.get(ID, Lang::En).expect("the registry gives it");
    assert_eq!(v.origin, Origin::Builtin);
    let m = v.manifest.unwrap();
    assert_eq!(m.run.unwrap().wasm.as_deref(), Some("Cargo.toml"));
    assert_eq!(m.permissions.network, ["api.trello.com"]);
    assert_eq!(m.permissions.secrets, ["account:trello"]);
}

/// The texts of the form's missing fields are the plugin's (its manifest), the token first.
#[test]
fn what_the_form_lacks_is_said_by_the_manifest_of_the_plugin() {
    use crate::plugins::builtin::missing;
    let given = |key: &'static str, token: &'static str| {
        move |field: &str| match field {
            "key" => key.to_string(),
            "token" => token.to_string(),
            _ => String::new(),
        }
    };
    assert_eq!(
        missing(ID, "trello", given("", ""), Lang::En).as_deref(),
        Some("Give the token.")
    );
    assert_eq!(
        missing(ID, "trello", given("", "t"), Lang::Fr).as_deref(),
        Some("Indique la clé d'API.")
    );
    assert_eq!(missing(ID, "trello", given("k", "t"), Lang::Fr), None);
    assert_eq!(missing(ID, "jira", given("", ""), Lang::Fr), None);
}
