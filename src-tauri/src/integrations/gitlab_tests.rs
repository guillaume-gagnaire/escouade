//! GitLab Issues against the fake server, its answers as the REST API v4's documentation gives
//! them: gitlab.com and a self-hosted instance side by side.

use super::*;
use crate::integrations::fake::FakeServer;
use crate::integrations::lasting;

/// gitlab.com and git.acme.dev, both on the fake server (their API under `/com` and `/acme`).
async fn gitlab() -> (FakeServer, Gitlab) {
    let server = FakeServer::start().await;
    let inst = |host: &str, at: &str, token: &str, user: &str| Instance {
        host: host.into(),
        base: format!("{}{at}/api/v4", server.url),
        token: token.into(),
        user: user.into(),
        label: format!("@{user}"),
    };
    let g = Gitlab::new(
        reqwest::Client::new(),
        vec![
            inst("gitlab.com", "/com", "glpat-com", "ada"),
            inst("git.acme.dev:8443", "/acme", "glpat-acme", "ada.l"),
        ],
    );
    (server, g)
}

const P: &str = "/com/api/v4/projects/acme%2Fback%2Fapi";

fn r() -> ExternalRef {
    ExternalRef {
        service: S,
        id: "42".into(),
        key: "#42".into(),
        container: "gitlab.com/acme/back/api".into(),
        ..Default::default()
    }
}

fn label(name: &str) -> ExternalState {
    ExternalState {
        id: format!("label:{name}"),
        name: name.into(),
    }
}

fn an_issue(iid: u64, title: &str) -> Value {
    json!({
        "id": 9000 + iid, "iid": iid, "project_id": 7, "title": title, "state": "opened",
        "description": "Le paiement plante.\n\n## Critères d'acceptation\n- [ ] Plus de crash\n- [ ] Un test e2e",
        "labels": ["bug", "backend"],
        "author": { "username": "grace" },
        "assignees": [{ "username": "ada" }],
        "assignee": { "username": "ada" },
        "issue_type": "issue",
        "web_url": format!("https://gitlab.com/acme/back/api/-/issues/{iid}")
    })
}

#[test]
fn the_states_and_the_filter_of_gitlab_read_in_english() {
    use crate::i18n::Lang::En;
    assert_eq!(
        fixed_states(En).map(|s| s.name),
        ["Open".to_string(), "Closed".to_string()]
    );
    assert_eq!(mine_filter(En).label, "Assigned to me");
    assert_eq!(unassigned(En), "Unassigned");
    assert_eq!(by(En, "grace"), "by @grace");
    assert_eq!(kind_of(En, "task"), ("Task".to_string(), "task"));
}

#[tokio::test]
async fn the_token_goes_to_its_own_instance() {
    let (server, g) = gitlab().await;
    server.on("GET", "/com/api/v4/user", 200, json!({ "username": "ada" }));
    assert_eq!(
        g.me().await.unwrap(),
        ("@ada".to_string(), "ada".to_string())
    );
    let req = &server.requests()[0];
    assert_eq!(req.headers["private-token"], "glpat-com");
    assert!(!req.headers.contains_key("authorization"));
    // The self-hosted instance's project: its own token, its own API.
    server.on(
        "GET",
        "/acme/api/v4/projects/infra%2Fdeploy/labels",
        200,
        json!([]),
    );
    g.states("git.acme.dev:8443/infra/deploy").await.unwrap();
    let req = server.requests().into_iter().last().unwrap();
    assert_eq!(req.headers["private-token"], "glpat-acme");
    // An instance no account reaches says so.
    let e = g.states("gitlab.other.org/x/y").await.unwrap_err();
    assert!(e.to_string().contains("gitlab.other.org"), "{e}");
}

#[tokio::test]
async fn the_projects_of_every_instance_keep_their_groups() {
    let (server, g) = gitlab().await;
    server.on(
        "GET",
        "/com/api/v4/projects",
        200,
        json!([{ "id": 7, "path_with_namespace": "acme/back/api" }, { "id": 8, "path_with_namespace": "ada/notes" }]),
    );
    server.on(
        "GET",
        "/acme/api/v4/projects",
        200,
        json!([{ "id": 3, "path_with_namespace": "infra/deploy" }]),
    );
    let list = g.containers().await.unwrap();
    let ids: Vec<(&str, &str, Option<&str>)> = list
        .iter()
        .map(|c| (c.id.as_str(), c.name.as_str(), c.group.as_deref()))
        .collect();
    assert_eq!(
        ids,
        [
            (
                "gitlab.com/acme/back/api",
                "acme/back/api",
                Some("gitlab.com")
            ),
            ("gitlab.com/ada/notes", "ada/notes", Some("gitlab.com")),
            (
                "git.acme.dev:8443/infra/deploy",
                "infra/deploy",
                Some("git.acme.dev:8443")
            ),
        ]
    );
    let asked = &server.requests()[0];
    assert_eq!(asked.query("membership").as_deref(), Some("true"));
    assert_eq!(asked.query("archived").as_deref(), Some("false"));
}

#[tokio::test]
async fn the_states_are_opened_closed_the_board_lists_then_the_labels() {
    let (server, g) = gitlab().await;
    server.on(
        "GET",
        &format!("{P}/labels"),
        200,
        json!([{ "name": "bug" }, { "name": "Doing" }, { "name": "To review" }]),
    );
    server.on(
        "GET",
        &format!("{P}/boards"),
        200,
        json!([{ "id": 1, "name": "Development", "lists": [
            { "id": 12, "position": 1, "label": { "name": "To review" } },
            { "id": 11, "position": 0, "label": { "name": "Doing" } },
            { "id": 13, "position": 2, "assignee": { "username": "ada" } }
        ] }]),
    );
    let ids: Vec<String> = g
        .states("gitlab.com/acme/back/api")
        .await
        .unwrap()
        .into_iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(
        ids,
        [
            "opened",
            "closed",
            "label:Doing",
            "label:To review",
            "label:bug"
        ]
    );
    // Without boards to read (an older instance, no right): the labels alone.
    server.on(
        "GET",
        &format!("{P}/boards"),
        403,
        json!({ "message": "403 Forbidden" }),
    );
    let n = g.states("gitlab.com/acme/back/api").await.unwrap().len();
    assert_eq!(n, 5);
}

#[tokio::test]
async fn the_issues_are_listed_with_their_labels_author_link_and_criteria() {
    let (server, g) = gitlab().await;
    server.on(
        "GET",
        &format!("{P}/labels"),
        200,
        json!([{ "name": "bug" }]),
    );
    let mut doc = an_issue(44, "Doc");
    doc["labels"] = json!([]);
    doc["assignees"] = json!([]);
    doc["assignee"] = Value::Null;
    doc["description"] = Value::Null;
    doc["issue_type"] = json!("incident");
    server.on(
        "GET",
        &format!("{P}/issues"),
        200,
        json!([an_issue(42, "Crash au paiement"), doc]),
    );
    let page = g
        .issues("gitlab.com/acme/back/api", &Query::default())
        .await
        .unwrap();
    let i = &page.issues[0];
    assert_eq!(page.issues.len(), 2);
    assert_eq!(
        (
            i.id.as_str(),
            i.key.as_str(),
            i.kind.as_str(),
            i.kind_code.as_str()
        ),
        ("42", "#42", "Issue", "issue")
    );
    assert_eq!(i.service, Service::GITLAB);
    assert_eq!(i.title, "Crash au paiement");
    assert_eq!(i.meta, ["bug", "backend", "@ada", "par @grace"]);
    assert_eq!(i.url, "https://gitlab.com/acme/back/api/-/issues/42");
    assert_eq!(i.criteria, ["Plus de crash", "Un test e2e"]);
    assert!(i.description.starts_with("Le paiement plante."));
    assert_eq!(i.container, "gitlab.com/acme/back/api");
    let d = &page.issues[1];
    assert_eq!((d.kind.as_str(), d.kind_code.as_str()), ("Incident", "bug"));
    assert_eq!(d.meta, ["Non assignée", "par @grace"]);
    let asked = &server.requests()[0];
    assert_eq!(asked.query("state").as_deref(), Some("opened"));
    assert_eq!(asked.query("page").as_deref(), Some("1"));
    let ids: Vec<&str> = page.filters.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["mine", "label:bug"]);
    // A label's chip keeps those that carry it.
    let q = Query {
        filters: vec!["label:bug".into()],
        ..Default::default()
    };
    let kept = g.issues("gitlab.com/acme/back/api", &q).await.unwrap();
    assert_eq!(kept.issues.len(), 1);
}

#[tokio::test]
async fn the_search_mine_and_the_import_label_are_asked_of_the_instance() {
    let (server, g) = gitlab().await;
    server.on("GET", &format!("{P}/labels"), 200, json!([]));
    server.on(
        "GET",
        &format!("{P}/issues"),
        200,
        json!([an_issue(42, "Crash")]),
    );
    let q = Query {
        text: "crash paiement".into(),
        filters: vec!["mine".into()],
        label: Some("claude-ready".into()),
        ..Default::default()
    };
    g.issues("gitlab.com/acme/back/api", &q).await.unwrap();
    let asked = server
        .requests()
        .into_iter()
        .find(|r| r.path().ends_with("/issues"))
        .unwrap();
    assert_eq!(asked.query("search").as_deref(), Some("crash paiement"));
    assert_eq!(asked.query("assignee_username").as_deref(), Some("ada"));
    assert_eq!(asked.query("labels").as_deref(), Some("claude-ready"));
    // A number is the issue itself, when it is an opened one.
    server.on("GET", &format!("{P}/issues/44"), 200, an_issue(44, "Doc"));
    let q = Query {
        text: "#44".into(),
        ..Default::default()
    };
    let found = g
        .issues("gitlab.com/acme/back/api", &q)
        .await
        .unwrap()
        .issues;
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].key, "#44");
    let mut closed = an_issue(44, "Doc");
    closed["state"] = json!("closed");
    server.on("GET", &format!("{P}/issues/44"), 200, closed);
    assert!(g
        .issues("gitlab.com/acme/back/api", &q)
        .await
        .unwrap()
        .issues
        .is_empty());
    server.on(
        "GET",
        &format!("{P}/issues/45"),
        404,
        json!({ "message": "404 Not found" }),
    );
    let q = Query {
        text: "45".into(),
        ..Default::default()
    };
    assert!(g
        .issues("gitlab.com/acme/back/api", &q)
        .await
        .unwrap()
        .issues
        .is_empty());
}

#[tokio::test]
async fn the_issues_come_a_page_at_a_time_with_their_total() {
    let (server, g) = gitlab().await;
    server.on("GET", &format!("{P}/labels"), 200, json!([]));
    server.on_with(
        "GET",
        &format!("{P}/issues"),
        200,
        json!([an_issue(1, "Un"), an_issue(2, "Deux")]),
        &[("X-Next-Page", "2"), ("X-Total", "3"), ("X-Page", "1")],
    );
    let first = g
        .issues("gitlab.com/acme/back/api", &Query::default())
        .await
        .unwrap();
    assert_eq!((first.next.as_deref(), first.total), (Some("2"), Some(3)));
    server.on_with(
        "GET",
        &format!("{P}/issues"),
        200,
        json!([an_issue(3, "Trois")]),
        &[("X-Next-Page", ""), ("X-Total", "3"), ("X-Page", "2")],
    );
    let q = Query {
        page: first.next.clone(),
        ..Default::default()
    };
    let last = g.issues("gitlab.com/acme/back/api", &q).await.unwrap();
    let asked = server
        .requests()
        .into_iter()
        .rfind(|r| r.path().ends_with("/issues"))
        .unwrap();
    assert_eq!(asked.query("page").as_deref(), Some("2"));
    assert_eq!(asked.query("per_page").as_deref(), Some("50"));
    assert_eq!(last.issues[0].key, "#3");
    assert_eq!((last.next, last.total), (None, Some(3)));
    // A label's chip keeps some of them here: the count is no longer theirs.
    let q = Query {
        filters: vec!["label:bug".into()],
        ..Default::default()
    };
    assert_eq!(
        g.issues("gitlab.com/acme/back/api", &q)
            .await
            .unwrap()
            .total,
        None
    );
}

#[tokio::test]
async fn a_label_state_replaces_the_other_mapped_labels_in_one_call() {
    let (server, g) = gitlab().await;
    server.on(
        "GET",
        &format!("{P}/issues/42"),
        200,
        json!({ "iid": 42, "state": "opened", "labels": ["Doing", "bug"] }),
    );
    server.on("PUT", &format!("{P}/issues/42"), 200, json!({}));
    let mapped = [label("Doing"), label("To review")];
    g.set_state(&r(), &label("To review"), &mapped)
        .await
        .unwrap();
    let w = server.writes();
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].method, "PUT");
    assert_eq!(
        w[0].json(),
        json!({ "remove_labels": "Doing", "add_labels": "To review" })
    );
    // Already carrying it: nothing to do.
    g.set_state(&r(), &label("Doing"), &mapped).await.unwrap();
    assert_eq!(server.writes().len(), 1);
}

#[tokio::test]
async fn closed_closes_and_opened_or_a_label_opens_again() {
    let (server, g) = gitlab().await;
    server.on("PUT", &format!("{P}/issues/42"), 200, json!({}));
    server.on(
        "GET",
        &format!("{P}/issues/42"),
        200,
        json!({ "iid": 42, "state": "opened", "labels": [] }),
    );
    let closed = ExternalState {
        id: "closed".into(),
        name: "Fermée".into(),
    };
    g.set_state(&r(), &closed, &[]).await.unwrap();
    assert_eq!(server.writes()[0].json(), json!({ "state_event": "close" }));
    server.on(
        "GET",
        &format!("{P}/issues/42"),
        200,
        json!({ "iid": 42, "state": "closed", "labels": [] }),
    );
    g.set_state(&r(), &closed, &[]).await.unwrap();
    assert_eq!(server.writes().len(), 1);
    g.set_state(&r(), &label("wip"), &[label("wip")])
        .await
        .unwrap();
    assert_eq!(
        server.writes()[1].json(),
        json!({ "state_event": "reopen", "add_labels": "wip" })
    );
    let opened = ExternalState {
        id: "opened".into(),
        name: "Ouverte".into(),
    };
    g.set_state(&r(), &opened, &[]).await.unwrap();
    assert_eq!(
        server.writes()[2].json(),
        json!({ "state_event": "reopen" })
    );
}

#[tokio::test]
async fn a_comment_is_a_note_in_markdown() {
    let (server, g) = gitlab().await;
    server.on(
        "POST",
        &format!("{P}/issues/42/notes"),
        201,
        json!({ "id": 1 }),
    );
    g.comment(&r(), "Escouade : **prêt**").await.unwrap();
    let w = &server.writes()[0];
    assert_eq!(w.json(), json!({ "body": "Escouade : **prêt**" }));
    assert_eq!(w.headers["private-token"], "glpat-com");
}

#[tokio::test]
async fn a_refusal_of_the_instance_says_so_and_lasts_or_not() {
    let (server, g) = gitlab().await;
    server.on(
        "GET",
        &format!("{P}/issues/42"),
        401,
        json!({ "message": "401 Unauthorized" }),
    );
    let e = g.set_state(&r(), &label("wip"), &[]).await.unwrap_err();
    assert_eq!(
        e.to_string(),
        "GitLab refuse ces identifiants (401) — 401 Unauthorized"
    );
    assert!(!lasting(&e));
    server.on(
        "GET",
        &format!("{P}/issues/42"),
        404,
        json!({ "message": "404 Issue Not Found" }),
    );
    let e = g.set_state(&r(), &label("wip"), &[]).await.unwrap_err();
    assert!(lasting(&e), "{e}");
    server.on("POST", &format!("{P}/issues/42/notes"), 429, json!({}));
    let e = g.comment(&r(), "x").await.unwrap_err();
    assert!(e.to_string().contains("429"), "{e}");
    assert!(!lasting(&e));
}

/// ESC-165, by hand: GitLab Issues against a real instance, with the user's token. Never runs
/// without them:
/// - `ESCOUADE_TEST_GITLAB_TOKEN`: a personal token with the `api` right (never printed);
/// - `ESCOUADE_TEST_GITLAB_PROJECT`: the path of a project of theirs where an issue may be opened
///   and deleted (`ada/escouade-essai`);
/// - `ESCOUADE_TEST_GITLAB_URL`: the instance, gitlab.com by default.
///
/// It opens an issue (two criteria, the label `escouade-essai`), finds it as the import does (by
/// its label, by its number), gives it the label `escouade-doing` as a column would, writes a
/// note, closes it, then deletes it (left closed when the token may not delete it).
#[tokio::test]
#[ignore = "the real GitLab: run by hand with a token"]
async fn real_gitlab_issues() {
    let token = std::env::var("ESCOUADE_TEST_GITLAB_TOKEN").expect("ESCOUADE_TEST_GITLAB_TOKEN");
    let project =
        std::env::var("ESCOUADE_TEST_GITLAB_PROJECT").expect("ESCOUADE_TEST_GITLAB_PROJECT");
    let url = std::env::var("ESCOUADE_TEST_GITLAB_URL")
        .unwrap_or_else(|_| "https://gitlab.com".into())
        .trim_end_matches('/')
        .to_string();
    let host = crate::forge::host_of(&url);
    let base = format!("{url}/api/v4");
    let http = reqwest::Client::new();
    let raw = |method: reqwest::Method, path: &str| {
        http.request(method, format!("{base}{path}"))
            .header("PRIVATE-TOKEN", &token)
    };
    let me: Value = raw(reqwest::Method::GET, "/user")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let user = me["username"]
        .as_str()
        .expect("the token's user")
        .to_string();
    let g = Gitlab::new(
        reqwest::Client::new(),
        vec![Instance {
            host: host.clone(),
            base: base.clone(),
            token: token.clone(),
            user: user.clone(),
            label: format!("@{user}"),
        }],
    );
    let started = std::time::Instant::now();
    assert_eq!(g.me().await.unwrap().1, user);
    let container = format!("{host}/{project}");
    let projects = g.containers().await.unwrap();
    eprintln!("{} projects; {container} among them", projects.len());
    assert!(
        projects.iter().any(|c| c.id == container),
        "{container} is not among the projects the account is a member of"
    );
    let pid = encode(&project);
    // The label a column gives (made if the project lacks it), and the issue.
    let _ = raw(reqwest::Method::POST, &format!("/projects/{pid}/labels"))
        .json(&json!({ "name": "escouade-doing", "color": "#6699cc" }))
        .send()
        .await;
    let made: Value = raw(reqwest::Method::POST, &format!("/projects/{pid}/issues"))
        .json(&json!({
            "title": "Escouade, essai réel d'ESC-165",
            "description": "Ouverte par l'essai réel d'Escouade, supprimée à la fin.\n\n## Critères d'acceptation\n- [ ] Importée avec ses critères\n- [ ] Étiquetée puis fermée",
            "labels": "escouade-essai",
            "assignee_ids": [me["id"]]
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let iid = made["iid"].as_u64().expect("the issue opened");
    let states = g.states(&container).await.unwrap();
    assert!(states.iter().any(|s| s.id == "label:escouade-doing"));
    let by_label = g
        .issues(
            &container,
            &Query {
                label: Some("escouade-essai".into()),
                filters: vec!["mine".into()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let found = by_label
        .issues
        .iter()
        .find(|i| i.id == iid.to_string())
        .expect("found by its label, assigned to me");
    assert_eq!(
        found.criteria,
        ["Importée avec ses critères", "Étiquetée puis fermée"]
    );
    assert!(found.url.starts_with(&url), "{}", found.url);
    let by_number = g
        .issues(
            &container,
            &Query {
                text: format!("#{iid}"),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(by_number.issues.len(), 1);
    let r = ExternalRef {
        service: S,
        id: iid.to_string(),
        key: format!("#{iid}"),
        container: container.clone(),
        ..Default::default()
    };
    let doing = label("escouade-doing");
    g.set_state(&r, &doing, std::slice::from_ref(&doing))
        .await
        .unwrap();
    g.comment(&r, "Escouade : essai réel d'ESC-165, une note.")
        .await
        .unwrap();
    let closed = ExternalState {
        id: "closed".into(),
        name: "Fermée".into(),
    };
    g.set_state(&r, &closed, &[]).await.unwrap();
    let now: Value = raw(
        reqwest::Method::GET,
        &format!("/projects/{pid}/issues/{iid}"),
    )
    .send()
    .await
    .unwrap()
    .json()
    .await
    .unwrap();
    eprintln!(
        "issue #{iid}: {} {:?}, {} notes, in {} ms",
        now["state"],
        now["labels"],
        now["user_notes_count"],
        started.elapsed().as_millis()
    );
    assert_eq!(now["state"], "closed");
    assert!(label_names(&now).contains(&"escouade-doing".to_string()));
    assert_eq!(now["user_notes_count"], 1);
    let gone = raw(
        reqwest::Method::DELETE,
        &format!("/projects/{pid}/issues/{iid}"),
    )
    .send()
    .await
    .unwrap()
    .status();
    eprintln!("issue #{iid} deleted: {gone}");
}
