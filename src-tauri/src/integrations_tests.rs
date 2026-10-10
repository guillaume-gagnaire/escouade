//! Integration tests of the external ticket systems on the core: accounts checked and saved
//! apart, a project's source listed and imported, and an imported ticket's external one kept in
//! step while the fake `claude` CLI works the ticket for real (see `tickets_tests`). The services
//! are a fake HTTP server (`integrations::fake`).

use crate::core::Core;
use crate::core_tests::{git, harness, Harness};
use crate::integrations::fake::{FakeServer, Request};
use crate::integrations::secrets::{self, SecretStore};
use crate::integrations::{self, Account, Bases, ExternalIssue, Query};
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

    /// Trello connected to `server`, and a project without its autopilot whose board b1 is
    /// linked (« En cours » → l2, « À tester » → l3), commenting `comments`.
    async fn trello_board(&self, server: &FakeServer, comments: &[Column]) -> Project {
        self.serve(server);
        server.on(
            "GET",
            "/members/me",
            200,
            json!({ "id": "m1", "username": "ada" }),
        );
        self.core
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
        let (p, _) = self.project(false).await;
        self.set_autopilot(&p.id, false);
        let mut states = BTreeMap::new();
        states.insert(Column::Doing, state("l2", "En cours"));
        states.insert(Column::Review, state("l3", "À tester"));
        self.link(
            &p.id,
            SourceLink {
                service: Service::Trello,
                container: "b1".into(),
                name: "Atlas".into(),
                states,
            },
            comments,
        );
        p
    }

    /// The card `id` of the board b1 imported into the project.
    async fn import_card(&self, project_id: &str, id: &str) -> Ticket {
        let card = ExternalIssue {
            service: Service::Trello,
            id: id.into(),
            key: format!("#{id}"),
            title: format!("Carte {id}"),
            container: "b1".into(),
            ..Default::default()
        };
        self.core
            .integration_import(project_id, vec![card], 5)
            .await
            .unwrap()
            .remove(0)
    }

    /// The ticket comes into `column`, as the board moves it: its external one hears of it.
    fn move_to(&self, id: &str, column: Column) {
        self.core
            .edit_ticket(id, |t| {
                t.column = column;
                Ok(())
            })
            .unwrap();
    }

    fn sync_error(&self, id: &str) -> Option<String> {
        self.tk(id).external.unwrap().error
    }

    /// The app's timer looks at the failed syncs `ms` after now.
    async fn retry_after(&self, ms: i64) {
        self.core.clock_ahead.store(ms, Ordering::SeqCst);
        self.core.retry_due_syncs();
        self.wait_synced().await;
    }

    /// When the ticket's next sync is tried again by itself, after now (ms), if it is.
    fn next_retry(&self, id: &str) -> Option<i64> {
        let at = self.core.pending_syncs.lock().head(id)?.retry_at?;
        Some(at - crate::model::now_ms())
    }
}

/// The writes sent to `server` as « PUT /cards/c1 l3 » (with the list a card goes to).
fn trace(server: &FakeServer) -> Vec<String> {
    server
        .writes()
        .iter()
        .map(|r| {
            let list = r.query("idList").map(|l| format!(" {l}"));
            format!("{} {}{}", r.method, r.path(), list.unwrap_or_default())
        })
        .collect()
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
    // Its second loop was to be told, after the move (tried again with it): the summary waited
    // behind it until the ticket left « En cours », which took the move away, then was tried at
    // once.
    assert_eq!(
        trace(&server),
        [
            "PUT /cards/c1 l2",
            "PUT /cards/c1 l2",
            "POST /cards/c1/actions/comments"
        ]
    );
    // Sent back, the card moves this time: the summary goes first, then the move, and the error
    // goes.
    server.on("PUT", "/cards/c1", 200, json!({}));
    server.on("POST", "/cards/c1/actions/comments", 200, json!({}));
    h.core.ticket_reject(&t.id, "encore").await.unwrap();
    h.wait_synced().await;
    h.wait("error gone", |h| {
        h.tk(&t.id).external.unwrap().error.is_none()
    })
    .await;
    let comments = writes_to(&server, "/actions/comments");
    assert_eq!(
        comments[1].json()["text"],
        "Escouade : DEM-1, boucle 2/5 — 1/2 critères atteints.\n\n✓ un — vérifié\n○ deux — reste le critère 2"
    );
    assert_eq!(
        trace(&server)[3..5],
        ["POST /cards/c1/actions/comments", "PUT /cards/c1 l2"]
    );
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
async fn an_accounts_secrets_go_to_the_keychain_never_to_the_file_and_leave_with_it() {
    let h = harness("ig-keychain");
    let server = FakeServer::start().await;
    h.serve(&server);
    let keychain = secrets::memory_of(&h.core.data);
    server.on(
        "GET",
        "/members/me",
        200,
        json!({ "id": "m1", "username": "ada" }),
    );
    let view = h
        .core
        .integration_connect(
            Service::Trello,
            Account {
                key: "trello-key".into(),
                token: "trello-secret".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(view.connected && !view.in_file, "{view:?}");
    assert_eq!(
        keychain.entry("trello").as_deref(),
        Some(r#"{"key":"trello-key","token":"trello-secret"}"#)
    );
    let file = h.core.data.integrations_file();
    let saved = std::fs::read_to_string(&file).unwrap();
    assert!(saved.contains("@ada"), "{saved}");
    assert!(
        !saved.contains("trello-key") && !saved.contains("trello-secret"),
        "{saved}"
    );
    // The calls carry them.
    let sent = &server.requests()[0];
    assert_eq!(
        (sent.query("key"), sent.query("token")),
        (Some("trello-key".into()), Some("trello-secret".into()))
    );
    // The next start finds them in the keychain.
    let app = mock_app();
    let (again, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    let trello = again.accounts.read().trello.clone().unwrap();
    assert_eq!(
        (trello.key.as_str(), trello.token.as_str()),
        ("trello-key", "trello-secret")
    );
    // GitHub through gh keeps none: the token of a former account goes.
    keychain
        .set("github", r#"{"key":"","token":"ghp-former"}"#)
        .unwrap();
    *h.core.gh_on_path.write() = Some(fake_gh());
    server.on("GET", "/user", 200, json!({ "login": "ada" }));
    h.core
        .integration_connect(Service::Github, Account::default())
        .await
        .unwrap();
    assert_eq!(keychain.entry("github"), None);
    // Disconnected, its secrets go.
    h.core.integration_disconnect(Service::Trello).unwrap();
    assert_eq!(keychain.entry("trello"), None);
}

#[tokio::test]
async fn a_token_the_keychain_refuses_stays_in_the_file_and_the_window_is_told() {
    let h = harness("ig-keychain-refused");
    let server = FakeServer::start().await;
    jira_routes(&server);
    let keychain = secrets::memory_of(&h.core.data);
    keychain.refuse.store(true, Ordering::SeqCst);
    let view = h
        .core
        .integration_connect(
            Service::Jira,
            Account {
                site: server.url.clone(),
                email: "ada@atlas.dev".into(),
                token: "jira-secret".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(view.connected && view.in_file, "{view:?}");
    let file = h.core.data.integrations_file();
    assert!(std::fs::read_to_string(&file)
        .unwrap()
        .contains("jira-secret"));
    let views = serde_json::to_string(&h.core.integration_accounts()).unwrap();
    assert!(views.contains(r#""inFile":true"#), "{views}");
    assert!(!views.contains("jira-secret"), "{views}");
    // Disconnected, it leaves the file; the keychain never had it: nothing to say.
    let views = h.core.integration_disconnect(Service::Jira).unwrap();
    assert!(!views[0].connected && !views[0].in_file);
    assert!(!std::fs::read_to_string(&file)
        .unwrap()
        .contains("jira-secret"));
    let events = h.events.lock().clone();
    assert!(!events.iter().any(|e| e["type"] == "toast"), "{events:?}");
}

#[tokio::test]
async fn a_keychain_unreadable_at_a_start_loses_nothing_and_no_call_goes_without_its_token() {
    let h = harness("ig-keychain-unread");
    let server = FakeServer::start().await;
    h.serve(&server);
    server.on(
        "GET",
        "/members/me",
        200,
        json!({ "id": "m1", "username": "ada" }),
    );
    server.on("GET", "/user", 200, json!({ "login": "work" }));
    h.core
        .integration_connect(
            Service::Trello,
            Account {
                key: "trello-key".into(),
                token: "trello-secret".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    h.core
        .integration_connect(
            Service::Github,
            Account {
                token: "ghp-work".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let file = h.core.data.integrations_file();
    let before = std::fs::read_to_string(&file).unwrap();
    let keychain = secrets::memory_of(&h.core.data);
    let entries = || (keychain.entry("trello"), keychain.entry("github"));
    let kept = entries();
    // The next start finds the keychain out of reach.
    keychain.refuse.store(true, Ordering::SeqCst);
    let app = mock_app();
    let (again, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    *again.bases.write() = h.core.bases.read().clone();
    // The CLI would answer for whoever it is logged in as: never asked in the account's place.
    *again.gh_on_path.write() = Some(fake_gh());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), before);
    assert_eq!(entries(), kept);
    let views = again.integration_accounts();
    assert!(
        views[1].connected && views[1].unread && views[2].connected && views[2].unread,
        "{views:?}"
    );
    let calls = server.requests().len();
    for service in [Service::Trello, Service::Github] {
        let e = again
            .integration_containers(service, None)
            .await
            .unwrap_err();
        assert_eq!(
            e.to_string(),
            format!(
                "Trousseau du système illisible : relance Escouade ou reconnecte le compte {}.",
                service.label()
            )
        );
    }
    assert_eq!(server.requests().len(), calls);
    // An account connected meanwhile leaves their entries alone.
    jira_routes(&server);
    let view = again
        .integration_connect(
            Service::Jira,
            Account {
                site: server.url.clone(),
                email: "ada@atlas.dev".into(),
                token: "jira-secret".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(view.in_file && !view.unread, "{view:?}");
    assert_eq!(entries(), kept);
    // Back at the start after: everything is there.
    keychain.refuse.store(false, Ordering::SeqCst);
    let app = mock_app();
    let (third, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    let a = third.accounts.read().clone();
    assert_eq!(a.trello.unwrap().token, "trello-secret");
    assert_eq!(a.github.unwrap().token, "ghp-work");
    assert_eq!(a.jira.unwrap().token, "jira-secret");
    let views = third.integration_accounts();
    assert!(views.iter().all(|v| !v.unread && !v.in_file), "{views:?}");
}

#[tokio::test]
async fn a_secret_the_keychain_cannot_forget_is_said_and_forgotten_at_the_next_start() {
    let h = harness("ig-keychain-forget");
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
                key: "trello-key".into(),
                token: "trello-secret".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    *h.core.gh_on_path.write() = Some(fake_gh());
    server.on("GET", "/user", 200, json!({ "login": "ada" }));
    h.core
        .integration_connect(Service::Github, Account::default())
        .await
        .unwrap();
    let keychain = secrets::memory_of(&h.core.data);
    keychain.refuse.store(true, Ordering::SeqCst);
    let views = h.core.integration_disconnect(Service::Trello).unwrap();
    assert!(!views[1].connected);
    // GitHub through gh has no token of its own to remove: nothing to say.
    h.core.integration_disconnect(Service::Github).unwrap();
    let toasts: Vec<Value> = h
        .events
        .lock()
        .iter()
        .filter(|e| e["type"] == "toast")
        .cloned()
        .collect();
    assert_eq!(
        toasts,
        [
            json!({ "type": "toast", "text": "Le jeton Trello n'a pas pu être retiré du trousseau du système : Escouade réessaiera au prochain démarrage, ou retire-le à la main (son nom contient « escouade »)." })
        ]
    );
    assert!(keychain.entry("trello").is_some());
    // Kept in the file for the next start: the services only, never a secret.
    let file = h.core.data.integrations_file();
    let saved = std::fs::read_to_string(&file).unwrap();
    let listed: integrations::Accounts = serde_json::from_str(&saved).unwrap();
    assert_eq!(listed.to_forget, [Service::Trello, Service::Github]);
    assert!(!saved.contains("trello-secret"), "{saved}");
    // GitHub connected again meanwhile, with a token of its own: its entry is its own now.
    keychain.refuse.store(false, Ordering::SeqCst);
    h.core
        .integration_connect(
            Service::Github,
            Account {
                token: "ghp-new".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let listed: integrations::Accounts =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(listed.to_forget, [Service::Trello]);
    // The next start forgets Trello's only.
    let app = mock_app();
    let (_again, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    assert_eq!(keychain.entry("trello"), None);
    assert_eq!(
        keychain.entry("github").as_deref(),
        Some(r#"{"key":"","token":"ghp-new"}"#)
    );
    let listed: integrations::Accounts =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert!(listed.to_forget.is_empty());
}

#[tokio::test]
async fn a_broken_file_then_another_account_never_costs_the_others_their_tokens() {
    let h = harness("ig-keychain-broken-file");
    let server = FakeServer::start().await;
    h.serve(&server);
    server.on(
        "GET",
        "/members/me",
        200,
        json!({ "id": "m1", "username": "ada" }),
    );
    jira_routes(&server);
    h.core
        .integration_connect(
            Service::Trello,
            Account {
                key: "trello-key".into(),
                token: "trello-secret".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    h.core
        .integration_connect(
            Service::Jira,
            Account {
                site: server.url.clone(),
                email: "ada@atlas.dev".into(),
                token: "jira-secret".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let keychain = secrets::memory_of(&h.core.data);
    let entries = || (keychain.entry("trello"), keychain.entry("jira"));
    let kept = entries();
    assert!(kept.0.is_some() && kept.1.is_some());
    // The file breaks; the next start knows no account, and GitHub is connected.
    let file = h.core.data.integrations_file();
    std::fs::write(&file, r#"{ "jira": "#).unwrap();
    let app = mock_app();
    let (again, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    *again.bases.write() = h.core.bases.read().clone();
    *again.gh_on_path.write() = Some(fake_gh());
    server.on("GET", "/user", 200, json!({ "login": "ada" }));
    again
        .integration_connect(Service::Github, Account::default())
        .await
        .unwrap();
    let listed: integrations::Accounts =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert!(listed.jira.is_none() && listed.trello.is_none());
    // The start after lists GitHub only: the others' tokens stay where they are.
    let app = mock_app();
    let (_third, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    assert_eq!(entries(), kept);
}

#[tokio::test]
async fn the_token_a_switch_to_the_cli_could_not_delete_goes_at_the_next_start() {
    let h = harness("ig-keychain-switch-gh");
    let server = FakeServer::start().await;
    h.serve(&server);
    server.on("GET", "/user", 200, json!({ "login": "ada" }));
    h.core
        .integration_connect(
            Service::Github,
            Account {
                token: "ghp-former".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let keychain = secrets::memory_of(&h.core.data);
    keychain.refuse.store(true, Ordering::SeqCst);
    *h.core.gh_on_path.write() = Some(fake_gh());
    let view = h
        .core
        .integration_connect(Service::Github, Account::default())
        .await
        .unwrap();
    assert_eq!(view.label, "@ada · via gh");
    assert!(keychain.entry("github").is_some());
    let file = h.core.data.integrations_file();
    let listed: integrations::Accounts =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(listed.to_forget, [Service::Github]);
    keychain.refuse.store(false, Ordering::SeqCst);
    let app = mock_app();
    let (again, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    assert_eq!(keychain.entry("github"), None);
    let github = again.accounts.read().github.clone().unwrap();
    assert_eq!((github.token.as_str(), github.unread), ("", false));
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

#[tokio::test]
async fn a_failed_sync_is_tried_again_a_minute_later_and_its_success_clears_the_warning() {
    let h = harness("ig-retry");
    let server = FakeServer::start().await;
    let p = h.trello_board(&server, &[]).await;
    let t = h.import_card(&p.id, "c1").await;
    server.on("PUT", "/cards/c1", 500, Value::Null);
    h.move_to(&t.id, Column::Doing);
    h.wait_synced().await;
    assert_eq!(h.sync_error(&t.id).as_deref(), Some("Trello : erreur 500"));
    // Kept on disk, for a restart.
    let file = h.core.data.sync_queue_file();
    let kept = std::fs::read_to_string(&file).unwrap();
    assert!(kept.contains("\"l2\"") && !kept.contains("\"t\""), "{kept}");
    // Not before its minute.
    h.retry_after(30_000).await;
    assert_eq!(trace(&server), ["PUT /cards/c1 l2"]);
    server.on("PUT", "/cards/c1", 200, json!({}));
    h.retry_after(60_000).await;
    assert_eq!(trace(&server), ["PUT /cards/c1 l2", "PUT /cards/c1 l2"]);
    assert_eq!(h.sync_error(&t.id), None);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "[]");
    // Through: never sent again.
    h.retry_after(3_600_000).await;
    assert_eq!(trace(&server).len(), 2);
}

#[tokio::test]
async fn resync_tries_again_at_once_and_says_how_it_went() {
    let h = harness("ig-resync");
    let server = FakeServer::start().await;
    let p = h.trello_board(&server, &[]).await;
    let t = h.import_card(&p.id, "c1").await;
    server.on(
        "PUT",
        "/cards/c1",
        401,
        Value::String("invalid token".into()),
    );
    h.move_to(&t.id, Column::Doing);
    h.wait_synced().await;
    assert_eq!(
        h.core
            .integration_resync(&t.id)
            .await
            .unwrap_err()
            .to_string(),
        "Trello refuse ces identifiants (401) — invalid token"
    );
    assert_eq!(trace(&server).len(), 2);
    assert!(h.sync_error(&t.id).is_some());
    server.on("PUT", "/cards/c1", 200, json!({}));
    h.core.integration_resync(&t.id).await.unwrap();
    assert_eq!(trace(&server).len(), 3);
    assert_eq!(h.sync_error(&t.id), None);
    assert!(h.core.pending_syncs.lock().ops.is_empty());
}

#[tokio::test]
async fn the_syncs_left_waiting_are_tried_again_when_the_app_starts() {
    let h = harness("ig-retry-start");
    let server = FakeServer::start().await;
    let p = h.trello_board(&server, &[Column::Doing]).await;
    let t = h.import_card(&p.id, "c1").await;
    server.on("PUT", "/cards/c1", 500, Value::Null);
    h.move_to(&t.id, Column::Doing);
    h.wait_synced().await;
    // The move failed: its comment waits behind it.
    assert_eq!(trace(&server), ["PUT /cards/c1 l2"]);
    h.core.save_now();
    // The app stops, the service comes back, the app starts again.
    server.on("PUT", "/cards/c1", 200, json!({}));
    server.on("POST", "/cards/c1/actions/comments", 200, json!({}));
    let app = mock_app();
    let (again, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    *again.bases.write() = h.core.bases.read().clone();
    assert_eq!(again.pending_syncs.lock().ops.len(), 2);
    assert!(again
        .ticket(&t.id)
        .unwrap()
        .external
        .unwrap()
        .error
        .is_some());
    again.retry_all_syncs();
    for _ in 0..750 {
        if again.syncs_queued.load(Ordering::SeqCst) == 0 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert_eq!(
        trace(&server),
        [
            "PUT /cards/c1 l2",
            "PUT /cards/c1 l2",
            "POST /cards/c1/actions/comments"
        ]
    );
    assert!(writes_to(&server, "/actions/comments")[0].json()["text"]
        .as_str()
        .unwrap()
        .starts_with("Escouade : DEM-1 est pris par un agent"));
    assert_eq!(again.ticket(&t.id).unwrap().external.unwrap().error, None);
    assert!(again.pending_syncs.lock().ops.is_empty());
}

#[tokio::test]
async fn a_kind_of_sync_a_later_version_left_waiting_does_not_cost_the_others_their_place() {
    let h = harness("ig-queue-later-kind");
    h.core.save_now();
    // Written by a later version (then rolled back): a kind of operation this one does not know.
    let file = h.core.data.sync_queue_file();
    let text = r#"[
  { "ticketId": "t1", "external": { "service": "trello", "id": "c1" }, "op": { "kind": "label", "name": "urgent" } },
  { "ticketId": "t1", "external": { "service": "trello", "id": "c1" }, "op": { "kind": "comment", "text": "pris" } }
]"#;
    std::fs::write(&file, text).unwrap();
    let app = mock_app();
    let (again, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    let ops = again.pending_syncs.lock().ops.clone();
    assert_eq!(ops.len(), 1, "{ops:?}");
    assert_eq!(
        ops[0].op,
        crate::integrations::sync::SyncOp::Comment {
            text: "pris".into()
        }
    );
    assert!(!file.with_extension("broken.json").exists());
}

#[tokio::test]
async fn a_failed_transition_gives_way_to_the_next_one() {
    let h = harness("ig-retry-stale");
    let server = FakeServer::start().await;
    let p = h.trello_board(&server, &[]).await;
    let t = h.import_card(&p.id, "c1").await;
    server.on("PUT", "/cards/c1?idList=l2&key=k&token=t", 500, Value::Null);
    server.on("PUT", "/cards/c1?idList=l3&key=k&token=t", 200, json!({}));
    h.move_to(&t.id, Column::Doing);
    h.wait_synced().await;
    assert!(h.sync_error(&t.id).is_some());
    h.move_to(&t.id, Column::Review);
    h.wait_synced().await;
    assert_eq!(h.sync_error(&t.id), None);
    // « En cours » would only undo « À tester »: never tried again, even once it could be.
    server.on("PUT", "/cards/c1?idList=l2&key=k&token=t", 200, json!({}));
    h.retry_after(2 * 3_600_000).await;
    assert_eq!(trace(&server), ["PUT /cards/c1 l2", "PUT /cards/c1 l3"]);
    assert!(h.core.pending_syncs.lock().ops.is_empty());
}

#[tokio::test]
async fn a_failed_transition_goes_when_its_ticket_comes_into_a_column_without_a_state() {
    let h = harness("ig-retry-unmapped");
    let server = FakeServer::start().await;
    let p = h.trello_board(&server, &[]).await;
    let t = h.import_card(&p.id, "c1").await;
    server.on("PUT", "/cards/c1", 500, Value::Null);
    h.move_to(&t.id, Column::Doing);
    h.wait_synced().await;
    assert_eq!(h.sync_error(&t.id).as_deref(), Some("Trello : erreur 500"));
    // Its agent archived, the ticket is back in « À faire », which gives the card no list: the
    // move to « En cours » would put the card where the ticket no longer is.
    h.move_to(&t.id, Column::Todo);
    h.wait_synced().await;
    assert!(h.core.pending_syncs.lock().ops.is_empty());
    assert_eq!(h.sync_error(&t.id), None);
    let file = h.core.data.sync_queue_file();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "[]");
    // Trello back: the card stays where it is, by itself, at a start or by « Resynchroniser ».
    server.on("PUT", "/cards/c1", 200, json!({}));
    h.retry_after(2 * 3_600_000).await;
    h.core.retry_all_syncs();
    h.wait_synced().await;
    h.core.integration_resync(&t.id).await.unwrap();
    assert_eq!(trace(&server), ["PUT /cards/c1 l2"]);

    // Likewise for a move set aside (refused for good), kept for « Resynchroniser ».
    server.on(
        "PUT",
        "/cards/c1",
        404,
        Value::String("card not found".into()),
    );
    h.move_to(&t.id, Column::Doing);
    h.wait_synced().await;
    assert!(h.core.pending_syncs.lock().ops[0].set_aside);
    h.move_to(&t.id, Column::Todo);
    h.wait_synced().await;
    assert!(h.core.pending_syncs.lock().ops.is_empty());
    assert_eq!(h.sync_error(&t.id), None);
    server.on("PUT", "/cards/c1", 200, json!({}));
    h.core.integration_resync(&t.id).await.unwrap();
    assert_eq!(trace(&server), ["PUT /cards/c1 l2", "PUT /cards/c1 l2"]);
}

#[tokio::test]
async fn a_tickets_syncs_keep_their_order_behind_one_that_failed_and_the_others_go_on() {
    let h = harness("ig-retry-order");
    let server = FakeServer::start().await;
    let p = h.trello_board(&server, &[Column::Review]).await;
    let t = h.import_card(&p.id, "c1").await;
    let other = h.import_card(&p.id, "c2").await;
    server.on("PUT", "/cards/c1", 200, json!({}));
    server.on("PUT", "/cards/c2", 200, json!({}));
    server.on("POST", "/cards/c1/actions/comments", 500, Value::Null);
    h.move_to(&t.id, Column::Review);
    h.wait_synced().await;
    // Sent back: its move waits behind the comment, tried again at once with it; the other
    // ticket's goes on.
    h.move_to(&t.id, Column::Doing);
    h.move_to(&other.id, Column::Doing);
    h.wait_synced().await;
    assert_eq!(
        trace(&server),
        [
            "PUT /cards/c1 l3",
            "POST /cards/c1/actions/comments",
            "POST /cards/c1/actions/comments",
            "PUT /cards/c2 l2"
        ]
    );
    assert_eq!(h.sync_error(&t.id).as_deref(), Some("Trello : erreur 500"));
    assert_eq!(h.sync_error(&other.id), None);
    // Its second failure: five minutes until the next try.
    let next = h.next_retry(&t.id).unwrap();
    assert!((4 * 60_000..=5 * 60_000).contains(&next), "{next}");
    server.on("POST", "/cards/c1/actions/comments", 200, json!({}));
    h.retry_after(next + 1_000).await;
    assert_eq!(
        trace(&server)[4..],
        ["POST /cards/c1/actions/comments", "PUT /cards/c1 l2"]
    );
    let said = writes_to(&server, "/actions/comments")[2].json();
    assert!(said["text"]
        .as_str()
        .unwrap()
        .starts_with("Escouade : DEM-1 est prêt à tester."));
    assert_eq!(h.sync_error(&t.id), None);
}

#[tokio::test]
async fn a_failed_sync_is_set_aside_a_day_later_and_resync_tries_it_again() {
    let h = harness("ig-retry-day");
    let server = FakeServer::start().await;
    let p = h.trello_board(&server, &[]).await;
    let t = h.import_card(&p.id, "c1").await;
    server.on("PUT", "/cards/c1", 500, Value::Null);
    h.move_to(&t.id, Column::Doing);
    h.wait_synced().await;
    // Each try when its time comes, as the clock goes: 1, 5, 15 minutes, then every hour.
    let mut at = Vec::new();
    while let Some(next) = h.next_retry(&t.id) {
        let ahead = next + 10;
        h.retry_after(ahead).await;
        at.push((ahead as f64 / 60_000.0).round() as i64);
    }
    assert_eq!(at[..5], [1, 6, 21, 81, 141]);
    assert_eq!(at.last(), Some(&1401));
    assert_eq!(trace(&server).len(), 27);
    // Set aside: the warning stays, nothing is tried by itself any more, not even at a start.
    let aside = h.core.pending_syncs.lock().ops.clone();
    assert_eq!(aside.len(), 1);
    assert!(aside[0].set_aside && aside[0].retry_at.is_none());
    assert_eq!(h.sync_error(&t.id).as_deref(), Some("Trello : erreur 500"));
    h.retry_after(48 * 3_600_000).await;
    h.core.retry_all_syncs();
    h.wait_synced().await;
    assert_eq!(trace(&server).len(), 27);
    // Kept on disk with the others.
    let kept = std::fs::read_to_string(h.core.data.sync_queue_file()).unwrap();
    assert!(kept.contains("\"setAside\": true"), "{kept}");
    // « Resynchroniser » tries it again: through, the warning goes with it.
    server.on("PUT", "/cards/c1", 200, json!({}));
    h.core.integration_resync(&t.id).await.unwrap();
    assert_eq!(trace(&server).len(), 28);
    assert_eq!(trace(&server)[27], "PUT /cards/c1 l2");
    assert_eq!(h.sync_error(&t.id), None);
    assert!(h.core.pending_syncs.lock().ops.is_empty());
}

#[tokio::test]
async fn a_refused_comment_is_set_aside_at_once_and_does_not_hold_the_next_move_back() {
    let h = harness("ig-refused");
    let server = FakeServer::start().await;
    let p = h.trello_board(&server, &[Column::Review]).await;
    let t = h.import_card(&p.id, "c1").await;
    server.on("PUT", "/cards/c1", 200, json!({}));
    server.on(
        "POST",
        "/cards/c1/actions/comments",
        404,
        Value::String("model not found".into()),
    );
    h.move_to(&t.id, Column::Review);
    h.wait_synced().await;
    // Asking again would not change a 404: the next move goes at once.
    h.move_to(&t.id, Column::Doing);
    h.wait_synced().await;
    assert_eq!(
        trace(&server),
        [
            "PUT /cards/c1 l3",
            "POST /cards/c1/actions/comments",
            "PUT /cards/c1 l2"
        ]
    );
    assert_eq!(
        h.sync_error(&t.id).as_deref(),
        Some("Trello : introuvable (404) — model not found")
    );
    // Never tried by itself: neither later nor at a start.
    h.retry_after(48 * 3_600_000).await;
    h.core.retry_all_syncs();
    h.wait_synced().await;
    assert_eq!(trace(&server).len(), 3);
    // Still refused, « Resynchroniser » says why and the warning stays.
    assert_eq!(
        h.core
            .integration_resync(&t.id)
            .await
            .unwrap_err()
            .to_string(),
        "Trello : introuvable (404) — model not found"
    );
    assert!(h.sync_error(&t.id).is_some());
    // Through at last: the warning goes with it.
    server.on("POST", "/cards/c1/actions/comments", 200, json!({}));
    h.core.integration_resync(&t.id).await.unwrap();
    assert_eq!(trace(&server)[4], "POST /cards/c1/actions/comments");
    assert_eq!(h.sync_error(&t.id), None);
    assert!(h.core.pending_syncs.lock().ops.is_empty());
}

#[tokio::test]
async fn a_warning_left_without_its_operation_is_resynced_from_the_tickets_column() {
    let h = harness("ig-resync-column");
    let server = FakeServer::start().await;
    let p = h.trello_board(&server, &[]).await;
    let t = h.import_card(&p.id, "c1").await;
    server.on("PUT", "/cards/c1", 200, json!({}));
    h.move_to(&t.id, Column::Doing);
    h.wait_synced().await;
    // As an earlier version left it: the warning, and nothing kept to try again.
    let warn = |h: &Harness| {
        h.core
            .edit_ticket(&t.id, |x| {
                x.external.as_mut().unwrap().error = Some("Trello : erreur 500".into());
                Ok(())
            })
            .unwrap()
    };
    warn(&h);
    h.core.integration_resync(&t.id).await.unwrap();
    assert_eq!(trace(&server), ["PUT /cards/c1 l2", "PUT /cards/c1 l2"]);
    assert_eq!(h.sync_error(&t.id), None);
    // « À faire » gives the card no list: nothing to send, the warning goes.
    h.move_to(&t.id, Column::Todo);
    h.wait_synced().await;
    warn(&h);
    h.core.integration_resync(&t.id).await.unwrap();
    assert_eq!(trace(&server).len(), 2);
    assert_eq!(h.sync_error(&t.id), None);
}
