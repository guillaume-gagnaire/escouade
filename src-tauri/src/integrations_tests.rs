//! Integration tests of the external ticket systems on the core: accounts checked and saved
//! apart, a project's source listed and imported, and an imported ticket's external one kept in
//! step while the fake `claude` CLI works the ticket for real (see `tickets_tests`). The services
//! are a fake HTTP server (`integrations::fake`).

use crate::core::Core;
use crate::core_tests::{git, harness, Harness};
use crate::integrations::fake::{FakeServer, Request};
use crate::integrations::{Account, Bases, ExternalIssue, Query};
use crate::model::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tauri::test::mock_app;

fn state(id: &str, name: &str) -> ExternalState {
    ExternalState {
        id: id.into(),
        name: name.into(),
    }
}

impl Harness {
    fn tk(&self, id: &str) -> Ticket {
        self.core.ticket(id).unwrap()
    }

    /// The services are `server`.
    fn serve(&self, server: &FakeServer) {
        *self.core.bases.write() = Bases {
            trello: server.url.clone(),
            github: server.url.clone(),
        };
    }

    fn link(&self, project_id: &str, link: SourceLink, comments: &[Column]) {
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

    /// No sync of an external ticket is queued or running.
    async fn wait_synced(&self) {
        self.wait("syncs over", |h| {
            h.core.syncs_queued.load(Ordering::SeqCst) == 0
        })
        .await
    }

    fn set_autopilot(&self, project_id: &str, on: bool) {
        let mut s = self.core.project(project_id).unwrap().board;
        s.autopilot = on;
        self.core.board_set(project_id, s).unwrap();
    }

    fn integration_settings(&self, f: impl FnOnce(&mut IntegrationSettings)) {
        let mut s = self.core.settings.read().clone();
        f(&mut s.integrations);
        self.core.save_settings(s).unwrap();
    }
}

/// The writes sent to `server` whose path ends with `end`.
fn writes_to(server: &FakeServer, end: &str) -> Vec<Request> {
    server
        .writes()
        .into_iter()
        .filter(|r| r.path().ends_with(end))
        .collect()
}

fn jira_routes(server: &FakeServer) {
    server.on(
        "GET",
        "/rest/api/3/myself",
        200,
        json!({ "accountId": "acc-1", "emailAddress": "ada@atlas.dev" }),
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
                "description": "Paginer GET /users.\n\nAcceptance criteria:\n- Paramètre limit\n- Limite max 100"
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
        json!({ "transitions": [
            { "id": "21", "to": { "id": "3", "name": "In Progress" } },
            { "id": "31", "to": { "id": "10", "name": "In Review" } }
        ]}),
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

#[tokio::test]
async fn a_jira_issue_is_imported_then_moved_and_commented_as_its_ticket_goes() {
    let h = harness("ig-jira");
    let server = FakeServer::start().await;
    jira_routes(&server);
    let view = h
        .core
        .integration_connect(
            Service::Jira,
            Account {
                site: server.url.clone(),
                email: "ada@atlas.dev".into(),
                token: "tok".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(view.connected && view.label.starts_with("ada@atlas.dev · "));
    let (p, r) = h.project(false).await;
    let mut states = BTreeMap::new();
    states.insert(Column::Doing, state("3", "In Progress"));
    states.insert(Column::Review, state("3", "In Progress"));
    states.insert(Column::Done, state("10", "In Review"));
    h.link(
        &p.id,
        SourceLink {
            service: Service::Jira,
            container: "ATL".into(),
            name: "ATL — Atlas".into(),
            states,
        },
        &[Column::Review, Column::Done],
    );
    // Listed, then imported: the scheduler takes it at once (autopilot).
    let page = h
        .core
        .integration_issues(&p.id, Service::Jira, Query::default())
        .await
        .unwrap();
    assert!(!page.issues[0].imported);
    let made = h
        .core
        .integration_import(&p.id, page.issues.clone(), 5)
        .await
        .unwrap();
    let t = &made[0];
    assert_eq!(
        (t.key.as_str(), t.title.as_str()),
        ("DEM-1", "Paginer les utilisateurs")
    );
    let criteria: Vec<&str> = t.criteria.iter().map(|c| c.text.as_str()).collect();
    assert_eq!(criteria, ["Paramètre limit", "Limite max 100"]);
    assert!(t
        .description
        .ends_with(&format!("Ticket Jira ATL-7 : {}/browse/ATL-7", server.url)));
    let ext = t.external.clone().unwrap();
    assert_eq!((ext.id.as_str(), ext.container.as_str()), ("ATL-7", "ATL"));
    // Imported once: listed as such, and not again.
    let again = h
        .core
        .integration_import(&p.id, page.issues, 5)
        .await
        .unwrap();
    assert!(again.is_empty());
    let listed = h
        .core
        .integration_issues(&p.id, Service::Jira, Query::default())
        .await
        .unwrap();
    assert!(listed.issues[0].imported);
    // Criterion n is met from loop n on: two loops, then "À tester".
    h.wait("to test", |h| h.tk(&t.id).column == Column::Review)
        .await;
    h.wait_synced().await;
    let moves: Vec<Value> = writes_to(&server, "/transitions")
        .iter()
        .map(|r| r.json())
        .collect();
    assert_eq!(
        moves,
        [
            json!({ "transition": { "id": "21" } }),
            json!({ "transition": { "id": "21" } })
        ]
    );
    // "En cours" is not commented here; "À tester" is, with its criteria.
    let comments = writes_to(&server, "/comment");
    assert_eq!(comments.len(), 1);
    let said = comments[0].json()["body"].to_string();
    assert!(
        said.contains("Escouade : DEM-1 est prêt à tester."),
        "{said}"
    );
    assert!(said.contains("✓ Limite max 100"), "{said}");
    h.core.ticket_approve(&t.id).await.unwrap();
    assert_eq!(h.tk(&t.id).column, Column::Done);
    h.wait_synced().await;
    let moves = writes_to(&server, "/transitions");
    assert_eq!(moves.len(), 3);
    assert_eq!(moves[2].json(), json!({ "transition": { "id": "31" } }));
    let comments = writes_to(&server, "/comment");
    assert_eq!(comments.len(), 2);
    assert!(comments[1].json()["body"]
        .to_string()
        .contains("Escouade : DEM-1 est terminé — ⤵ Mergé dans main · squash"));
    assert_eq!(h.tk(&t.id).external.unwrap().error, None);
    assert_eq!(git(&r, &["rev-list", "--count", "HEAD"]), "2");
}

#[tokio::test]
async fn a_failed_sync_is_noted_on_the_ticket_until_one_succeeds_and_loops_can_be_told() {
    let h = harness("ig-trello");
    let server = FakeServer::start().await;
    h.serve(&server);
    server.on(
        "GET",
        "/members/me",
        200,
        json!({ "id": "m1", "username": "ada" }),
    );
    h.core
        .integration_connect(
            Service::Trello,
            Account {
                key: "k".into(),
                token: "t".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    h.integration_settings(|s| s.loop_comments = true);
    let (p, _) = h.project(false).await;
    h.set_autopilot(&p.id, false);
    let mut states = BTreeMap::new();
    states.insert(Column::Doing, state("l2", "En cours"));
    h.link(
        &p.id,
        SourceLink {
            service: Service::Trello,
            container: "b1".into(),
            name: "Atlas".into(),
            states,
        },
        &[],
    );
    let card = ExternalIssue {
        service: Service::Trello,
        id: "c1".into(),
        key: "#151".into(),
        title: "Exporter le journal".into(),
        url: "https://trello.com/c/abc".into(),
        criteria: vec!["un".into(), "deux".into()],
        container: "b1".into(),
        ..Default::default()
    };
    let t = h
        .core
        .integration_import(&p.id, vec![card], 5)
        .await
        .unwrap()
        .remove(0);
    // The card can neither move nor be commented: the ticket says why, and goes on all the same.
    server.on(
        "PUT",
        "/cards/c1",
        401,
        Value::String("invalid token".into()),
    );
    server.on(
        "POST",
        "/cards/c1/actions/comments",
        401,
        Value::String("invalid token".into()),
    );
    h.core.ticket_start(&t.id).unwrap();
    h.wait("to test", |h| h.tk(&t.id).column == Column::Review)
        .await;
    h.wait_synced().await;
    assert_eq!(
        h.tk(&t.id).external.unwrap().error.as_deref(),
        Some("Trello refuse ces identifiants (401) — invalid token")
    );
    // Its second loop was to be told.
    let comments = writes_to(&server, "/actions/comments");
    assert_eq!(comments.len(), 1);
    assert_eq!(
        comments[0].json()["text"],
        "Escouade : DEM-1, boucle 2/5 — 1/2 critères atteints.\n\n✓ un — vérifié\n○ deux — reste le critère 2"
    );
    // Sent back, the card moves this time: the error goes.
    server.on("PUT", "/cards/c1", 200, json!({}));
    server.on("POST", "/cards/c1/actions/comments", 200, json!({}));
    h.core.ticket_reject(&t.id, "encore").await.unwrap();
    h.wait_synced().await;
    h.wait("error gone", |h| {
        h.tk(&t.id).external.unwrap().error.is_none()
    })
    .await;
    let moved = writes_to(&server, "/cards/c1");
    assert_eq!(moved.last().unwrap().query("idList").as_deref(), Some("l2"));
}

#[tokio::test]
async fn nothing_is_said_of_a_ticket_made_by_hand_nor_with_the_sync_turned_off() {
    let h = harness("ig-quiet");
    let server = FakeServer::start().await;
    h.serve(&server);
    server.on("GET", "/user", 200, json!({ "login": "ada" }));
    server.on(
        "GET",
        "/repos/acme/api/issues/9",
        200,
        json!({ "state": "open", "labels": [] }),
    );
    server.on("POST", "/repos/acme/api/issues/9/labels", 200, json!([]));
    h.core
        .integration_connect(
            Service::Github,
            Account {
                token: "ghp".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    h.integration_settings(|s| s.sync_states = false);
    let (p, _) = h.project(false).await;
    let mut states = BTreeMap::new();
    states.insert(Column::Review, state("label:to test", "to test"));
    h.link(
        &p.id,
        SourceLink {
            service: Service::Github,
            container: "acme/api".into(),
            name: "acme/api".into(),
            states,
        },
        &[],
    );
    let issue = ExternalIssue {
        service: Service::Github,
        id: "9".into(),
        key: "#9".into(),
        title: "Doc [ok]".into(),
        container: "acme/api".into(),
        ..Default::default()
    };
    let mine = h
        .core
        .ticket_create(
            &p.id,
            crate::tickets::TicketDraft {
                title: "À la main [ok]".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let t = h
        .core
        .integration_import(&p.id, vec![issue], 5)
        .await
        .unwrap()
        .remove(0);
    h.wait("both to test", |h| {
        h.tk(&t.id).column == Column::Review && h.tk(&mine.id).column == Column::Review
    })
    .await;
    h.wait_synced().await;
    assert!(server.writes().is_empty(), "{:?}", server.writes());
    // Turned on, the next move is told.
    h.integration_settings(|s| s.sync_states = true);
    h.core.ticket_reject(&t.id, "encore").await.unwrap();
    h.wait("to test again", |h| h.tk(&t.id).column == Column::Review)
        .await;
    h.wait_synced().await;
    let added = writes_to(&server, "/labels");
    assert_eq!(added.len(), 1);
    assert_eq!(added[0].json(), json!({ "labels": ["to test"] }));
}

#[tokio::test]
async fn accounts_are_checked_saved_apart_and_forgotten() {
    let h = harness("ig-accounts");
    let server = FakeServer::start().await;
    h.serve(&server);
    // Missing fields are said before any call.
    let err = |e: anyhow::Error| e.to_string();
    let jira = |site: &str, email: &str, token: &str| Account {
        site: site.into(),
        email: email.into(),
        token: token.into(),
        ..Default::default()
    };
    assert_eq!(
        err(h
            .core
            .integration_connect(Service::Jira, jira("", "a@b", "t"))
            .await
            .unwrap_err()),
        "Indique l'adresse du site."
    );
    // The token would travel in clear: only this machine may be reached over http.
    assert_eq!(
        err(h
            .core
            .integration_connect(Service::Jira, jira("http://atlas.example.com", "a@b", "t"))
            .await
            .unwrap_err()),
        "Le site Jira doit être en https (http seulement sur cette machine)."
    );
    assert_eq!(
        err(h
            .core
            .integration_connect(Service::Jira, jira(&server.url, "", "t"))
            .await
            .unwrap_err()),
        "Indique l'e-mail."
    );
    assert_eq!(
        err(h
            .core
            .integration_connect(
                Service::Trello,
                Account {
                    token: "t".into(),
                    ..Default::default()
                }
            )
            .await
            .unwrap_err()),
        "Indique la clé d'API."
    );
    // Refused credentials are not kept.
    server.on("GET", "/rest/api/3/myself", 401, Value::Null);
    assert_eq!(
        err(h
            .core
            .integration_connect(Service::Jira, jira(&server.url, "a@b", "bad"))
            .await
            .unwrap_err()),
        "Jira refuse ces identifiants (401)"
    );
    assert!(!h.core.integration_accounts()[0].connected);
    // GitHub without a token takes the CLI's.
    *h.core.gh_on_path.write() = Some(fake_gh());
    server.on("GET", "/user", 200, json!({ "login": "ada" }));
    let view = h
        .core
        .integration_connect(Service::Github, Account::default())
        .await
        .unwrap();
    assert_eq!(view.label, "@ada · via gh");
    let sent = server
        .requests()
        .into_iter()
        .find(|r| r.path() == "/user")
        .unwrap();
    assert_eq!(sent.headers["authorization"], "Bearer gho_fake");
    // Saved apart from the settings and the state, and found again.
    let file = h.core.data.integrations_file();
    let saved = std::fs::read_to_string(&file).unwrap();
    assert!(saved.contains("\"login\"") || saved.contains("ada"));
    h.core.save_now();
    let state = std::fs::read_to_string(h.core.data.state_file()).unwrap();
    let settings = std::fs::read_to_string(h.core.data.settings_file()).unwrap();
    assert!(!state.contains("@ada") && !settings.contains("@ada"));
    let app = mock_app();
    let (reloaded, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    assert!(reloaded.integration_accounts()[2].connected);
    let views = h.core.integration_disconnect(Service::Github).unwrap();
    assert!(!views[2].connected);
    assert!(!std::fs::read_to_string(&file).unwrap().contains("ada"));
}

#[tokio::test]
async fn a_site_behind_a_self_signed_certificate_is_reached_once_tls_verification_is_off() {
    let h = harness("ig-insecure-tls");
    // As a corporate proxy that decrypts the traffic presents it: a certificate no system trusts.
    let server = FakeServer::start_tls().await;
    jira_routes(&server);
    let account = || Account {
        site: server.url.clone(),
        email: "ada@atlas.dev".into(),
        token: "t".into(),
        ..Default::default()
    };
    let refused = h
        .core
        .integration_connect(Service::Jira, account())
        .await
        .unwrap_err()
        .to_string();
    assert!(
        refused.starts_with("Jira injoignable : ") && refused.contains("UnknownIssuer"),
        "{refused}"
    );
    assert!(server.requests().is_empty());
    h.core.settings.write().insecure_tls = true;
    let view = h
        .core
        .integration_connect(Service::Jira, account())
        .await
        .unwrap();
    assert!(view.connected, "{view:?}");
    assert_eq!(server.requests()[0].path(), "/rest/api/3/myself");
}

fn fake_gh() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
        .join(if cfg!(windows) {
            "fake-gh.cmd"
        } else {
            "fake-gh"
        })
}

#[tokio::test]
async fn the_projects_own_repository_comes_first_and_the_states_come_with_their_defaults() {
    let h = harness("ig-repos");
    let server = FakeServer::start().await;
    h.serve(&server);
    server.on("GET", "/user", 200, json!({ "login": "ada" }));
    server.on(
        "GET",
        "/user/repos",
        200,
        json!([{ "full_name": "ada/blog" }, { "full_name": "acme/demo" }]),
    );
    server.on(
        "GET",
        "/repos/acme/demo/labels",
        200,
        json!([{ "name": "in progress" }, { "name": "QA" }]),
    );
    h.core
        .integration_connect(
            Service::Github,
            Account {
                token: "ghp".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let (p, r) = h.project(false).await;
    git(
        &r,
        &["remote", "add", "origin", "git@github.com:acme/demo.git"],
    );
    let list = h
        .core
        .integration_containers(Service::Github, Some(&p.id))
        .await
        .unwrap();
    let names: Vec<&str> = list.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["acme/demo (dépôt du projet)", "ada/blog"]);
    assert_eq!(list[0].id, "acme/demo");
    let v = h
        .core
        .integration_states(Service::Github, "acme/demo")
        .await
        .unwrap();
    assert_eq!(v.states.len(), 4);
    assert_eq!(v.defaults[&Column::Doing].id, "label:in progress");
    assert_eq!(v.defaults[&Column::Done].id, "label:QA");
    // No account: said so.
    assert_eq!(
        h.core
            .integration_containers(Service::Trello, None)
            .await
            .unwrap_err()
            .to_string(),
        "Aucun compte Trello connecté"
    );
}

#[tokio::test]
async fn the_automatic_import_brings_the_labelled_tickets_once() {
    let h = harness("ig-auto");
    let server = FakeServer::start().await;
    h.serve(&server);
    server.on("GET", "/user", 200, json!({ "login": "ada" }));
    server.on("GET", "/repos/acme/api/labels", 200, json!([]));
    server.on(
        "GET",
        "/search/issues",
        200,
        json!({ "items": [
            { "number": 4, "title": "Mode sombre", "body": "- [ ] Bascule persistée", "labels": [{ "name": "claude-ready" }] },
            { "number": 5, "title": "Footer", "body": "", "labels": [{ "name": "claude-ready" }] }
        ]}),
    );
    h.core
        .integration_connect(
            Service::Github,
            Account {
                token: "ghp".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let (p, _) = h.project(false).await;
    h.set_autopilot(&p.id, false);
    h.link(
        &p.id,
        SourceLink {
            service: Service::Github,
            container: "acme/api".into(),
            name: "acme/api".into(),
            states: BTreeMap::new(),
        },
        &[],
    );
    // Off: nothing comes.
    h.core.auto_import().await;
    assert!(h.core.tickets.read().is_empty());
    h.integration_settings(|s| {
        s.auto_import = true;
        s.import_label = "claude-ready".into();
    });
    h.core.auto_import().await;
    let tickets = h.core.tickets.read().clone();
    assert_eq!(tickets.len(), 2);
    assert_eq!(tickets[0].criteria[0].text, "Bascule persistée");
    assert_eq!(
        tickets[1].criteria[0].text,
        "Implémentation conforme au ticket"
    );
    let search = server
        .requests()
        .into_iter()
        .find(|r| r.path() == "/search/issues")
        .unwrap();
    assert!(search
        .query("q")
        .unwrap()
        .contains("label:\"claude-ready\""));
    assert!(
        h.events
            .lock()
            .iter()
            .any(|e| e["type"] == "toast"
                && e["text"] == "2 tickets importés depuis GitHub dans demo")
    );
    h.core.auto_import().await;
    assert_eq!(h.core.tickets.read().len(), 2);
}

#[tokio::test]
async fn a_ticket_deleted_once_imported_by_label_is_not_brought_back() {
    let h = harness("ig-auto-gone");
    let server = FakeServer::start().await;
    h.serve(&server);
    server.on("GET", "/user", 200, json!({ "login": "ada" }));
    server.on("GET", "/repos/acme/api/labels", 200, json!([]));
    server.on(
        "GET",
        "/search/issues",
        200,
        json!({ "items": [{ "number": 4, "title": "Mode sombre", "body": "", "labels": [{ "name": "claude-ready" }] }] }),
    );
    h.core
        .integration_connect(
            Service::Github,
            Account {
                token: "ghp".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let (p, _) = h.project(false).await;
    h.set_autopilot(&p.id, false);
    h.link(
        &p.id,
        SourceLink {
            service: Service::Github,
            container: "acme/api".into(),
            name: "acme/api".into(),
            states: BTreeMap::new(),
        },
        &[],
    );
    h.integration_settings(|s| s.auto_import = true);
    h.core.auto_import().await;
    let id = h.core.tickets.read()[0].id.clone();
    h.core.ticket_delete(&id).await.unwrap();
    // Not again by itself (its label stays on GitHub)…
    h.core.auto_import().await;
    assert!(h.core.tickets.read().is_empty());
    // …nor once the window saved the project's links (it does not know what was imported).
    h.link(
        &p.id,
        SourceLink {
            service: Service::Github,
            container: "acme/api".into(),
            name: "acme/api".into(),
            states: BTreeMap::new(),
        },
        &[],
    );
    h.core.auto_import().await;
    assert!(h.core.tickets.read().is_empty());
    // By hand, it comes back.
    let issue = ExternalIssue {
        service: Service::Github,
        id: "4".into(),
        key: "#4".into(),
        title: "Mode sombre".into(),
        container: "acme/api".into(),
        ..Default::default()
    };
    let made = h
        .core
        .integration_import(&p.id, vec![issue], 5)
        .await
        .unwrap();
    assert_eq!(made.len(), 1);
}

#[tokio::test]
async fn syncs_reach_the_service_in_the_order_of_the_changes_and_the_last_one_says_if_it_failed() {
    let h = harness("ig-order");
    let server = FakeServer::start().await;
    h.serve(&server);
    server.on(
        "GET",
        "/members/me",
        200,
        json!({ "id": "m1", "username": "ada" }),
    );
    h.core
        .integration_connect(
            Service::Trello,
            Account {
                key: "k".into(),
                token: "t".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let (p, _) = h.project(false).await;
    h.set_autopilot(&p.id, false);
    let mut states = BTreeMap::new();
    states.insert(Column::Doing, state("l2", "En cours"));
    states.insert(Column::Review, state("l3", "À tester"));
    h.link(
        &p.id,
        SourceLink {
            service: Service::Trello,
            container: "b1".into(),
            name: "Atlas".into(),
            states,
        },
        &[],
    );
    let card = ExternalIssue {
        service: Service::Trello,
        id: "c1".into(),
        key: "#1".into(),
        title: "Carte".into(),
        container: "b1".into(),
        ..Default::default()
    };
    let t = h
        .core
        .integration_import(&p.id, vec![card], 5)
        .await
        .unwrap()
        .remove(0);
    // « En cours » cannot move the card, « À tester » can.
    server.on("PUT", "/cards/c1?idList=l2&key=k&token=t", 500, Value::Null);
    server.on("PUT", "/cards/c1?idList=l3&key=k&token=t", 200, json!({}));
    let at = |column: Column| Ticket {
        column,
        ..t.clone()
    };
    for _ in 0..10 {
        h.core.sync_external(
            at(Column::Doing),
            crate::integrations::sync::Change::Column(Column::Doing),
        );
        h.core.sync_external(
            at(Column::Review),
            crate::integrations::sync::Change::Column(Column::Review),
        );
    }
    h.wait_synced().await;
    let lists: Vec<String> = writes_to(&server, "/cards/c1")
        .iter()
        .map(|r| r.query("idList").unwrap())
        .collect();
    assert_eq!(lists, ["l2", "l3"].repeat(10));
    // Each failure was noted, then cleared by the next success: none is left.
    assert_eq!(h.tk(&t.id).external.unwrap().error, None);
    h.core.sync_external(
        at(Column::Doing),
        crate::integrations::sync::Change::Column(Column::Doing),
    );
    h.wait_synced().await;
    assert_eq!(
        h.tk(&t.id).external.unwrap().error.as_deref(),
        Some("Trello : erreur 500")
    );
}
