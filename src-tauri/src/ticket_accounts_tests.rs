//! Several accounts per ticket source (ESC-397): two Jira accounts side by side, each project served by its own, one
//! disconnected alone (its projects' sources paused), and the accounts of 2.1.5 and 2.2.0-beta.4 read as the first ones.
//! The services are fake HTTP servers (`integrations::fake`), Jira the plugin `dev.escouade.jira` built for the tests.

use crate::core_tests::{harness, Harness};
use crate::integrations::fake::FakeServer;
use crate::integrations::secrets::{self, SecretStore};
use crate::integrations::{Account, AccountView, Query};
use crate::model::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::atomic::Ordering;

/// The Basic authorization of a Jira account.
fn basic(email: &str, token: &str) -> String {
    use base64::Engine;
    let b = base64::engine::general_purpose::STANDARD.encode(format!("{email}:{token}"));
    format!("Basic {b}")
}

/// The Jira account `email` / `token` on the site `server`.
fn jira_at(server: &FakeServer, email: &str, token: &str) -> Account {
    Account {
        site: server.url.clone(),
        email: email.into(),
        token: token.into(),
        ..Default::default()
    }
}

/// The Jira source `container` of the account `account` (empty: the first one), « En cours » → 3.
fn jira_link(container: &str, account: &str) -> SourceLink {
    let mut states = BTreeMap::new();
    states.insert(
        Column::Doing,
        ExternalState {
            id: "3".into(),
            name: "In Progress".into(),
        },
    );
    SourceLink {
        service: Service::JIRA,
        container: container.into(),
        name: container.into(),
        states,
        account: account.into(),
    }
}

/// A Jira site: who the account is, one issue ATL-7, its transitions and comments.
fn jira_routes(server: &FakeServer, email: &str) {
    server.on(
        "GET",
        "/rest/api/3/myself",
        200,
        json!({ "accountId": format!("acc-{email}"), "emailAddress": email }),
    );
    server.on(
        "GET",
        "/rest/api/3/project/search",
        200,
        json!({ "isLast": true, "values": [{ "key": "ATL", "name": "Atlas" }] }),
    );
    server.on(
        "POST",
        "/rest/api/3/search/jql",
        200,
        json!({ "issues": [{
            "key": "ATL-7",
            "fields": {
                "summary": "Paginer les utilisateurs",
                "issuetype": { "name": "Story" },
                "status": { "name": "To Do" },
                "description": "Paginer GET /users."
            }
        }]}),
    );
    server.on(
        "GET",
        "/rest/api/3/issue/ATL-7",
        200,
        json!({ "fields": { "status": { "id": "1", "name": "To Do" } } }),
    );
    server.on(
        "GET",
        "/rest/api/3/issue/ATL-7/transitions",
        200,
        json!({ "transitions": [{ "id": "21", "to": { "id": "3", "name": "In Progress" } }] }),
    );
    server.on(
        "POST",
        "/rest/api/3/issue/ATL-7/transitions",
        204,
        Value::Null,
    );
    server.on(
        "POST",
        "/rest/api/3/issue/ATL-7/comment",
        201,
        json!({ "id": "c" }),
    );
}

/// The authorizations of the requests `server` received on paths ending with `end`.
fn auths(server: &FakeServer, end: &str) -> Vec<String> {
    server
        .requests()
        .iter()
        .filter(|r| r.path().ends_with(end))
        .map(|r| r.headers.get("authorization").cloned().unwrap_or_default())
        .collect()
}

/// The writes `server` received on paths ending with `end`.
fn writes(server: &FakeServer, end: &str) -> usize {
    server
        .writes()
        .iter()
        .filter(|r| r.path().ends_with(end))
        .count()
}

impl Harness {
    fn put_link(&self, project_id: &str, link: SourceLink, comments: &[Column]) {
        let p = self.core.project(project_id).unwrap();
        self.core
            .update_project(Project {
                integrations: ProjectIntegrations {
                    links: vec![link],
                    comments: comments.to_vec(),
                    ..Default::default()
                },
                ..p
            })
            .unwrap();
    }

    fn no_autopilot(&self, project_id: &str) {
        let mut s = self.core.project(project_id).unwrap().board;
        s.autopilot = false;
        self.core.board_set(project_id, s).unwrap();
    }

    fn column(&self, id: &str, column: Column) {
        self.core
            .edit_ticket(id, |t| {
                t.column = column;
                Ok(())
            })
            .unwrap();
    }

    async fn synced(&self) {
        self.wait("syncs over", |h| {
            h.core.syncs_queued.load(Ordering::SeqCst) == 0
        })
        .await
    }

    fn jira_views(&self) -> Vec<AccountView> {
        self.core
            .integration_accounts()
            .into_iter()
            .filter(|v| v.service == Service::JIRA)
            .collect()
    }

    /// Two Jira accounts (ada on `a`, bob on `b`), and two projects without autopilot: the first linked to ATL of the
    /// first account, the second to ATL of the second one, commenting « À tester ».
    async fn two_jiras(&self, a: &FakeServer, b: &FakeServer) -> (Project, Project, String) {
        jira_routes(a, "ada@atlas.dev");
        jira_routes(b, "bob@mobile.dev");
        self.core
            .integration_connect(Service::JIRA, jira_at(a, "ada@atlas.dev", "tok-a"))
            .await
            .unwrap();
        let other = self
            .core
            .integration_connect(Service::JIRA, jira_at(b, "bob@mobile.dev", "tok-b"))
            .await
            .unwrap()
            .id;
        let (p1, _) = self.project(false).await;
        // The second one in a repository of its own.
        let r2 = crate::core_tests::repo(&self.dir.join("second"));
        let p2 = self
            .core
            .create_project(
                &r2.to_string_lossy(),
                "mobile",
                "oklch(0.7 0.1 200)",
                false,
                None,
                true,
            )
            .await
            .unwrap();
        for p in [&p1, &p2] {
            self.no_autopilot(&p.id);
        }
        self.put_link(&p1.id, jira_link("ATL", ""), &[Column::Review]);
        self.put_link(&p2.id, jira_link("ATL", &other), &[Column::Review]);
        (p1, p2, other)
    }
}

#[tokio::test]
async fn two_jira_accounts_connect_side_by_side_each_disconnects_alone_and_the_same_one_is_put_right(
) {
    let h = harness("ta-two");
    let (a, b) = (FakeServer::start().await, FakeServer::start().await);
    jira_routes(&a, "ada@atlas.dev");
    jira_routes(&b, "ada@atlas.dev");
    let first = h
        .core
        .integration_connect(Service::JIRA, jira_at(&a, "ada@atlas.dev", "tok-a"))
        .await
        .unwrap();
    // The same person on another site: another account.
    let second = h
        .core
        .integration_connect(Service::JIRA, jira_at(&b, "ada@atlas.dev", "tok-b"))
        .await
        .unwrap();
    // The first is the one of before (its own field, its former entry); the second comes beside it.
    assert_eq!(first.id, "jira");
    assert!(
        second.id.starts_with("jira-") && second.connected,
        "{second:?}"
    );
    assert_eq!(h.jira_views().len(), 2);
    // The same site and e-mail again: put right (its new token), not added.
    let again = h
        .core
        .integration_connect(Service::JIRA, jira_at(&b, "ADA@atlas.dev", "tok-b2"))
        .await
        .unwrap();
    assert_eq!(again.id, second.id);
    assert_eq!(h.jira_views().len(), 2);
    let keychain = secrets::memory_of(&h.core.data);
    let second_entry = format!("plugin:dev.escouade.jira:account:{}", second.id);
    assert!(keychain.entry(&second_entry).unwrap().contains("tok-b2"));
    assert!(keychain.entry("jira").unwrap().contains("tok-a"));
    // On disk: the first in integrations.json (what 2.1 reads), the second apart, neither with its token.
    let main = std::fs::read_to_string(h.core.data.integrations_file()).unwrap();
    let more = std::fs::read_to_string(h.core.data.ticket_accounts_file()).unwrap();
    assert!(main.contains(&a.url) && !main.contains(&b.url), "{main}");
    assert!(more.contains(&b.url) && more.contains(&second.id), "{more}");
    assert!(!main.contains("tok-") && !more.contains("tok-"));
    // Read back at the next start, with their secrets.
    let loaded = secrets::load_accounts(&h.core.data, &*keychain, false);
    assert_eq!(loaded.list(Service::JIRA).len(), 2);
    assert_eq!(
        loaded.account(Service::JIRA, &second.id).unwrap().token,
        "tok-b2"
    );
    // The second goes alone: the first keeps its account and its token.
    let views = h
        .core
        .integration_disconnect(Service::JIRA, &second.id)
        .unwrap();
    let left: Vec<_> = views
        .iter()
        .filter(|v| v.service == Service::JIRA)
        .collect();
    assert_eq!((left.len(), left[0].id.as_str()), (1, "jira"));
    assert_eq!(keychain.entry(&second_entry), None);
    assert!(keychain.entry("jira").unwrap().contains("tok-a"));
    // A new account after it never takes its id back.
    let third = h
        .core
        .integration_connect(Service::JIRA, jira_at(&b, "ada@atlas.dev", "tok-b3"))
        .await
        .unwrap();
    assert!(third.id.starts_with("jira-") && third.id != second.id);
    // Then the first: the other stays.
    h.core
        .integration_disconnect(Service::JIRA, "jira")
        .unwrap();
    assert_eq!(keychain.entry("jira"), None);
    let views = h.jira_views();
    assert_eq!((views.len(), views[0].id.as_str()), (1, third.id.as_str()));
}

#[tokio::test]
async fn trello_linear_and_clickup_take_several_accounts_too() {
    let h = harness("ta-others");
    let server = FakeServer::start().await;
    *h.core.bases.write() = crate::integrations::Bases {
        trello: server.url.clone(),
        github: server.url.clone(),
        linear: server.url.clone(),
        clickup: server.url.clone(),
    };
    // Trello: two members, by the key and token each is.
    server.on(
        "GET",
        "/members/me",
        200,
        json!({ "id": "m1", "username": "ada" }),
    );
    let ada = h
        .core
        .integration_connect(
            Service::TRELLO,
            Account {
                key: "k".into(),
                token: "t1".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    server.on(
        "GET",
        "/members/me",
        200,
        json!({ "id": "m2", "username": "bob" }),
    );
    let bob = h
        .core
        .integration_connect(
            Service::TRELLO,
            Account {
                key: "k".into(),
                token: "t2".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_ne!(ada.id, bob.id);
    // The same member again: put right.
    server.on(
        "GET",
        "/members/me",
        200,
        json!({ "id": "m1", "username": "ada" }),
    );
    let ada2 = h
        .core
        .integration_connect(
            Service::TRELLO,
            Account {
                key: "k".into(),
                token: "t3".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(ada2.id, ada.id);
    let trello: Vec<_> = h
        .core
        .integration_accounts()
        .into_iter()
        .filter(|v| v.service == Service::TRELLO)
        .collect();
    assert_eq!(trello.len(), 2);
    // Linear and ClickUp: a second key is a second account.
    for (service, me) in [(Service::LINEAR, "/graphql"), (Service::CLICKUP, "/user")] {
        for (n, who) in [(1, "ada"), (2, "bob")] {
            if service == Service::LINEAR {
                server.on(
                    "POST",
                    me,
                    200,
                    json!({ "data": { "viewer": { "id": format!("u{n}"), "name": who, "email": format!("{who}@acme.dev") } } }),
                );
            } else {
                server.on(
                    "GET",
                    me,
                    200,
                    json!({ "user": { "id": n, "username": who } }),
                );
            }
            h.core
                .integration_connect(
                    service,
                    Account {
                        token: format!("pk_{who}"),
                        ..Default::default()
                    },
                )
                .await
                .unwrap();
        }
        let n = h
            .core
            .integration_accounts()
            .into_iter()
            .filter(|v| v.service == service && v.connected)
            .count();
        assert_eq!(n, 2, "{service}");
    }
    // Each disconnects alone.
    h.core
        .integration_disconnect(Service::TRELLO, &bob.id)
        .unwrap();
    let left: Vec<_> = h
        .core
        .integration_accounts()
        .into_iter()
        .filter(|v| v.service == Service::TRELLO)
        .collect();
    assert_eq!((left.len(), left[0].id.as_str()), (1, ada.id.as_str()));
}

#[tokio::test]
async fn two_projects_list_import_sync_and_comment_each_with_its_own_jira_account() {
    let h = harness("ta-projects");
    let (a, b) = (FakeServer::start().await, FakeServer::start().await);
    let (p1, p2, other) = h.two_jiras(&a, &b).await;
    // Each project's containers and states come from its own account.
    let listed = h
        .core
        .integration_containers(Service::JIRA, &other, Some(&p2.id))
        .await
        .unwrap();
    assert_eq!(listed[0].id, "ATL");
    assert_eq!(
        auths(&b, "/project/search"),
        [basic("bob@mobile.dev", "tok-b")]
    );
    assert!(auths(&a, "/project/search").is_empty());
    let page1 = h
        .core
        .integration_issues(&p1.id, Service::JIRA, Query::default())
        .await
        .unwrap();
    let page2 = h
        .core
        .integration_issues(&p2.id, Service::JIRA, Query::default())
        .await
        .unwrap();
    assert_eq!(auths(&a, "/search/jql"), [basic("ada@atlas.dev", "tok-a")]);
    assert_eq!(auths(&b, "/search/jql"), [basic("bob@mobile.dev", "tok-b")]);
    let t1 = h
        .core
        .integration_import(&p1.id, page1.issues, 5)
        .await
        .unwrap()
        .remove(0);
    let t2 = h
        .core
        .integration_import(&p2.id, page2.issues, 5)
        .await
        .unwrap()
        .remove(0);
    // A ticket keeps the account it was imported with, and links to its own site.
    assert_eq!(t1.external.as_ref().unwrap().account, "");
    assert_eq!(t2.external.as_ref().unwrap().account, other);
    assert_eq!(
        t2.external.as_ref().unwrap().url,
        format!("{}/browse/ATL-7", b.url)
    );
    // Moved: each transition goes to its own site, as its own account.
    h.column(&t2.id, Column::Doing);
    h.synced().await;
    assert_eq!(
        (writes(&a, "/transitions"), writes(&b, "/transitions")),
        (0, 1)
    );
    assert_eq!(
        auths(&b, "/transitions").last().unwrap(),
        &basic("bob@mobile.dev", "tok-b")
    );
    h.column(&t1.id, Column::Doing);
    h.synced().await;
    assert_eq!(
        (writes(&a, "/transitions"), writes(&b, "/transitions")),
        (1, 1)
    );
    // The first account goes: its project's source is paused, its link kept; the other one's goes on.
    h.core
        .integration_disconnect(Service::JIRA, "jira")
        .unwrap();
    let p1_now = h.core.project(&p1.id).unwrap();
    assert_eq!(p1_now.integrations.links, vec![jira_link("ATL", "")]);
    assert!(!h
        .core
        .source_served(p1_now.integrations.link(Service::JIRA).unwrap()));
    assert!(h
        .core
        .integration_issues(&p1.id, Service::JIRA, Query::default())
        .await
        .is_err());
    h.column(&t2.id, Column::Review);
    h.synced().await;
    assert_eq!((writes(&a, "/comment"), writes(&b, "/comment")), (0, 1));
    assert_eq!(
        auths(&b, "/comment").last().unwrap(),
        &basic("bob@mobile.dev", "tok-b")
    );
    assert_eq!(h.core.ticket(&t2.id).unwrap().external.unwrap().error, None);
    // The ticket of the paused source says why it is not synced.
    h.column(&t1.id, Column::Review);
    h.synced().await;
    let why = h.core.ticket(&t1.id).unwrap().external.unwrap().error;
    assert_eq!(why.as_deref(), Some("Aucun compte Jira connecté"));
}

#[tokio::test]
async fn the_automatic_import_takes_each_projects_account_and_skips_a_paused_source() {
    let h = harness("ta-auto");
    let (a, b) = (FakeServer::start().await, FakeServer::start().await);
    let (p1, p2, _) = h.two_jiras(&a, &b).await;
    let mut s = h.core.settings.read().clone();
    s.integrations.auto_import = true;
    s.integrations.import_label = "claude-ready".into();
    h.core.save_settings(s).unwrap();
    h.core
        .integration_disconnect(Service::JIRA, "jira")
        .unwrap();
    h.core.auto_import().await;
    // Only the project whose account is there imported, from its account.
    assert!(auths(&a, "/search/jql").is_empty());
    assert_eq!(auths(&b, "/search/jql"), [basic("bob@mobile.dev", "tok-b")]);
    let tickets = |p: &str| {
        h.core
            .tickets
            .read()
            .iter()
            .filter(|t| t.project_id == p)
            .count()
    };
    assert_eq!((tickets(&p1.id), tickets(&p2.id)), (0, 1));
}

/// What 2.1.5 left: the Jira account in `integrations.json` without its token, the token under the keychain's former
/// entry `jira`; what 2.2.0-beta.4 left: the same, with `vault: 2` and the token also under the plugin's name. A
/// project's link names no account, nor does a ticket imported then.
#[tokio::test]
async fn the_accounts_of_2_1_5_and_of_2_2_0_beta_4_are_the_first_ones_with_nothing_to_redo() {
    for (case, beta) in [("2.1.5", false), ("2.2.0-beta.4", true)] {
        let h = harness(&format!("ta-migrate-{beta}"));
        let server = FakeServer::start().await;
        jira_routes(&server, "ada@atlas.dev");
        let host = server.url.trim_start_matches("http://").to_string();
        let mut file = json!({
            "jira": {
                "site": server.url, "email": "ada@atlas.dev", "key": "", "token": "",
                "label": format!("ada@atlas.dev · {host}"), "user": "acc-1"
            },
            "trello": null, "github": null, "linear": null, "clickup": null,
            "forges": [], "forgetForges": [], "toForget": []
        });
        let keychain = secrets::memory_of(&h.core.data);
        let secret = r#"{"key":"","token":"tok-old"}"#;
        keychain.set("jira", secret).unwrap();
        if beta {
            file["vault"] = json!(2);
            keychain
                .set("plugin:dev.escouade.jira:account:jira", secret)
                .unwrap();
        }
        std::fs::write(
            h.core.data.integrations_file(),
            serde_json::to_vec(&file).unwrap(),
        )
        .unwrap();
        *h.core.accounts.write() = secrets::load_accounts(&h.core.data, &*keychain, false);
        let views = h.jira_views();
        assert_eq!(views.len(), 1, "{case}");
        assert!(
            views[0].connected && views[0].id == "jira",
            "{case}: {views:?}"
        );
        let (p, _) = h.project(false).await;
        h.no_autopilot(&p.id);
        // A link as it was saved then: no account.
        let saved: SourceLink = serde_json::from_value(json!({
            "service": "jira", "container": "ATL", "name": "ATL — Atlas",
            "states": { "doing": { "id": "3", "name": "In Progress" } }
        }))
        .unwrap();
        assert_eq!(saved.account, "");
        h.put_link(&p.id, saved, &[]);
        let page = h
            .core
            .integration_issues(&p.id, Service::JIRA, Query::default())
            .await
            .unwrap();
        assert_eq!(page.issues.len(), 1, "{case}");
        assert_eq!(
            auths(&server, "/search/jql"),
            [basic("ada@atlas.dev", "tok-old")],
            "{case}"
        );
        // A ticket imported before (no account) syncs with the first account.
        let t = h
            .core
            .integration_import(&p.id, page.issues, 5)
            .await
            .unwrap()
            .remove(0);
        h.core
            .edit_ticket(&t.id, |t| {
                t.external.as_mut().unwrap().account = String::new();
                Ok(())
            })
            .unwrap();
        h.column(&t.id, Column::Doing);
        h.synced().await;
        assert_eq!(writes(&server, "/transitions"), 1, "{case}");
        // Nothing written apart while there is no second account.
        assert!(!h.core.data.ticket_accounts_file().exists(), "{case}");
        // The keychain's entries are where 2.1.5 reads them.
        assert_eq!(keychain.entry("jira").as_deref(), Some(secret), "{case}");
    }
}

#[test]
fn a_file_of_a_later_version_with_an_unknown_source_loses_none_of_the_others() {
    let dir = std::env::temp_dir().join(format!("ta-more-{}", uuid::Uuid::new_v4().simple()));
    let data = crate::paths::DataDir::new(dir);
    std::fs::create_dir_all(data.integrations_file().parent().unwrap()).unwrap();
    std::fs::write(
        data.ticket_accounts_file(),
        serde_json::to_vec(&json!({ "accounts": [
            { "id": "jira-1a2b3c4d", "service": "jira", "site": "https://b.atlassian.net", "email": "a@b", "label": "a@b · b.atlassian.net", "user": "u" },
            { "id": "notion-1", "service": "notion", "label": "x" }
        ]}))
        .unwrap(),
    )
    .unwrap();
    let store = secrets::MemoryStore::default();
    store
        .set(
            "plugin:dev.escouade.jira:account:jira-1a2b3c4d",
            r#"{"key":"","token":"t"}"#,
        )
        .unwrap();
    let a = secrets::load_accounts(&data, &store, false);
    let list = a.list(Service::JIRA);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].0, "jira-1a2b3c4d");
    assert_eq!(list[0].1.token, "t");
}
