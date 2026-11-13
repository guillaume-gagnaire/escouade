//! Azure Boards against the fake server, its answers as the REST API 7.1's documentation gives
//! them: two organizations, and the processes Agile, Scrum and Basic.
//!
//! Since INT-8.5 the client is the plugin « Azure DevOps »'s (`plugins/azure-devops`): each
//! organization's account is a `ProcessTicketSource` to its module, as the core
//! makes them, and `ByOrganization` sends a call to the organization its container names; the real trial uses the
//! plugin's client in process.

use super::*;
use crate::integrations::fake::FakeServer;
use crate::integrations::lasting;
use crate::integrations::source::TicketSource;
use crate::plugins::adapters::{ByOrganization, ProcessTicketSource};
use crate::plugins::builtin::{test_runtime, AZURE_DEVOPS};
use serde_json::{json, Value};

const S: Service = Service::AZURE;

/// The organizations acme (on dev.azure.com) and globex (a Server's collection), both on the fake server (dev.azure.com's
/// API under `/azure`, as `ESCOUADE_AZURE_DEVOPS_API` puts it), each with the token lent as the core lends it.
async fn boards() -> (FakeServer, ByOrganization) {
    let server = FakeServer::start().await;
    let rt = test_runtime("azure-boards");
    crate::plugins::builtin::let_reach(&rt, &[("fake.test", &server.url)]);
    let source = |site: &str, token: &str| -> Box<dyn TicketSource> {
        rt.lend(
            AZURE_DEVOPS,
            &format!("account:{}", crate::forge::host_of(site)),
            token,
        );
        let account = Account {
            site: site.into(),
            ..Account::default()
        };
        Box::new(
            ProcessTicketSource::new(rt.clone(), AZURE_DEVOPS, "azure", &account)
                .with_api(&format!("{}/azure", server.url)),
        )
    };
    let b = ByOrganization::new(vec![
        (
            "acme".into(),
            source("https://dev.azure.com/acme", "pat-acme"),
        ),
        (
            "globex".into(),
            source("https://tfs.example.com/tfs/globex", "pat-globex"),
        ),
    ]);
    (server, b)
}

/// The project « Web Shop » of acme.
const P: &str = "/azure/acme/Web%20Shop/_apis";
const C: &str = "acme/Web Shop";

fn state(name: &str, category: &str) -> Value {
    json!({ "name": name, "color": "007acc", "category": category })
}

/// A process's work item types, as `GET …/wit/workitemtypes` lists them.
fn agile() -> Value {
    let states = || {
        json!([
            state("New", "Proposed"),
            state("Active", "InProgress"),
            state("Resolved", "Resolved"),
            state("Closed", "Completed"),
            state("Removed", "Removed")
        ])
    };
    json!({ "count": 4, "value": [
        { "name": "User Story", "referenceName": "Microsoft.VSTS.WorkItemTypes.UserStory", "isDisabled": false, "states": states() },
        { "name": "Bug", "referenceName": "Microsoft.VSTS.WorkItemTypes.Bug", "isDisabled": false, "states": [
            state("New", "Proposed"), state("Active", "InProgress"), state("Resolved", "Resolved"), state("Closed", "Completed")
        ] },
        { "name": "Task", "referenceName": "Microsoft.VSTS.WorkItemTypes.Task", "isDisabled": false, "states": [
            state("New", "Proposed"), state("Active", "InProgress"), state("Closed", "Completed"), state("Removed", "Removed")
        ] },
        { "name": "Test Case", "referenceName": "Microsoft.VSTS.WorkItemTypes.TestCase", "isDisabled": false, "states": [
            state("Design", "Proposed"), state("Ready", "InProgress"), state("Closed", "Completed")
        ] }
    ] })
}

fn scrum() -> Value {
    json!({ "count": 3, "value": [
        { "name": "Product Backlog Item", "isDisabled": false, "states": [
            state("New", "Proposed"), state("Approved", "Proposed"), state("Committed", "InProgress"),
            state("Done", "Completed"), state("Removed", "Removed")
        ] },
        { "name": "Task", "isDisabled": false, "states": [
            state("To Do", "Proposed"), state("In Progress", "InProgress"), state("Done", "Completed"),
            state("Removed", "Removed")
        ] },
        { "name": "Impediment", "isDisabled": true, "states": [
            state("Open", "Proposed"), state("Closed", "Completed")
        ] }
    ] })
}

fn basic() -> Value {
    json!({ "count": 3, "value": [
        { "name": "Epic", "isDisabled": false, "states": [
            state("To Do", "Proposed"), state("Doing", "InProgress"), state("Done", "Completed")
        ] },
        { "name": "Issue", "isDisabled": false, "states": [
            state("To Do", "Proposed"), state("Doing", "InProgress"), state("Done", "Completed")
        ] },
        { "name": "Task", "isDisabled": false, "states": [
            state("To Do", "Proposed"), state("Doing", "InProgress"), state("Done", "Completed")
        ] }
    ] })
}

fn r(id: &str) -> ExternalRef {
    ExternalRef {
        service: S,
        id: id.into(),
        key: format!("#{id}"),
        container: C.into(),
        ..Default::default()
    }
}

fn named(name: &str) -> ExternalState {
    ExternalState {
        id: name.into(),
        name: name.into(),
    }
}

/// A work item as `GET …/wit/workitems?ids=` gives it.
fn item(id: u64, kind: &str, title: &str) -> Value {
    json!({
        "id": id, "rev": 3,
        "fields": {
            "System.Id": id,
            "System.TeamProject": "Web Shop",
            "System.WorkItemType": kind,
            "System.State": "Active",
            "System.Title": title,
            "System.Tags": "paiement; urgent",
            "System.AssignedTo": { "displayName": "Ada Lovelace", "uniqueName": "ada@acme.dev", "id": "u-ada" },
            "System.CreatedBy": { "displayName": "Grace Hopper", "uniqueName": "grace@acme.dev" },
            "System.Description": "<div>Le paiement&nbsp;plante.</div><div><br></div><div><b>Critères d'acceptation</b></div><ul><li>Description seule</li></ul>",
            "Microsoft.VSTS.Common.AcceptanceCriteria": "<ul><li>Plus de crash</li><li>Un test e2e</li></ul>"
        },
        "url": format!("https://dev.azure.com/acme/_apis/wit/workItems/{id}")
    })
}

#[tokio::test]
async fn the_projects_of_each_organization_page_by_page() {
    let (server, b) = boards().await;
    server.on(
        "GET",
        "/azure/acme/_apis/projects",
        200,
        json!({ "count": 1, "value": [{ "id": "p3", "name": "Infra", "state": "wellFormed" }] }),
    );
    server.on_once(
        "GET",
        "/azure/acme/_apis/projects",
        200,
        json!({ "count": 2, "value": [
            { "id": "p1", "name": "Web Shop", "state": "wellFormed", "visibility": "private" },
            { "id": "p2", "name": "Mobile", "state": "wellFormed", "visibility": "private" }
        ] }),
        &[("x-ms-continuationtoken", "page 2")],
    );
    server.on(
        "GET",
        "/azure/globex/_apis/projects",
        200,
        json!({ "count": 1, "value": [{ "id": "g1", "name": "Ops", "state": "wellFormed" }] }),
    );
    let list = b.containers().await.unwrap();
    let ids: Vec<(&str, &str, Option<&str>)> = list
        .iter()
        .map(|c| (c.id.as_str(), c.name.as_str(), c.group.as_deref()))
        .collect();
    assert_eq!(
        ids,
        [
            ("acme/Web Shop", "Web Shop", Some("acme")),
            ("acme/Mobile", "Mobile", Some("acme")),
            ("acme/Infra", "Infra", Some("acme")),
            ("globex/Ops", "Ops", Some("globex")),
        ]
    );
    let reqs = server.requests();
    assert_eq!(reqs[0].query("api-version").as_deref(), Some("7.1"));
    assert_eq!(reqs[0].query("continuationToken"), None);
    assert_eq!(
        reqs[1].query("continuationToken").as_deref(),
        Some("page 2")
    );
    // Each organization's token, in Basic (`:token`).
    use base64::Engine;
    let basic = |t: &str| {
        format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(format!(":{t}"))
        )
    };
    assert_eq!(reqs[0].headers["authorization"], basic("pat-acme"));
    assert_eq!(reqs[2].headers["authorization"], basic("pat-globex"));
}

#[tokio::test]
async fn the_states_are_those_of_the_process_by_category() {
    let (server, b) = boards().await;
    let url = format!("{P}/wit/workitemtypes");
    let names = |v: &StatesView| v.states.iter().map(|s| s.name.clone()).collect::<Vec<_>>();
    let default = |v: &StatesView, c: Column| v.defaults.get(&c).map(|s| s.name.clone());

    // Agile: the removed and the test cases' are left out.
    server.on("GET", &url, 200, agile());
    let v = b.states_view(C).await.unwrap();
    assert_eq!(names(&v), ["New", "Active", "Resolved", "Closed"]);
    assert_eq!(default(&v, Column::Doing).as_deref(), Some("Active"));
    assert_eq!(default(&v, Column::Review).as_deref(), Some("Active"));
    assert_eq!(default(&v, Column::Done).as_deref(), Some("Resolved"));
    assert_eq!(default(&v, Column::Todo), None);
    assert_eq!(b.states(C).await.unwrap()[1], named("Active"));

    // Scrum: the backlog item's and the task's, a disabled type's left out; « In Progress » is
    // named like the column, and nothing is resolved before done.
    server.on("GET", &url, 200, scrum());
    let v = b.states_view(C).await.unwrap();
    assert_eq!(
        names(&v),
        [
            "New",
            "Approved",
            "To Do",
            "Committed",
            "In Progress",
            "Done"
        ]
    );
    assert_eq!(default(&v, Column::Doing).as_deref(), Some("In Progress"));
    assert_eq!(default(&v, Column::Done), None);

    // Basic.
    server.on("GET", &url, 200, basic());
    let v = b.states_view(C).await.unwrap();
    assert_eq!(names(&v), ["To Do", "Doing", "Done"]);
    assert_eq!(default(&v, Column::Doing).as_deref(), Some("Doing"));
}

#[tokio::test]
async fn the_import_queries_with_wiql_and_reads_a_page_at_a_time() {
    let (server, b) = boards().await;
    server.on("GET", &format!("{P}/wit/workitemtypes"), 200, agile());
    server.on(
        "GET",
        &format!("{P}/wit/tags"),
        200,
        json!({ "count": 2, "value": [{ "id": "t1", "name": "paiement" }, { "id": "t2", "name": "urgent" }] }),
    );
    // 60 work items match: a page of 50, then 10.
    let ids: Vec<Value> = (1..=60)
        .map(|i| json!({ "id": 1000 + i, "url": "…" }))
        .collect();
    server.on(
        "POST",
        &format!("{P}/wit/wiql"),
        200,
        json!({ "queryType": "flat", "queryResultType": "workItem", "asOf": "2026-10-27T08:00:00Z",
            "columns": [{ "referenceName": "System.Id", "name": "ID" }], "workItems": ids }),
    );
    // Azure gives them in any order: the query's is kept.
    let first: Vec<Value> = (1..=50)
        .rev()
        .map(|i| item(1000 + i, "User Story", &format!("Story {i}")))
        .collect();
    server.on(
        "GET",
        &format!("{P}/wit/workitems"),
        200,
        json!({ "count": 50, "value": first }),
    );
    let page = b
        .issues(
            C,
            &Query {
                filters: vec![
                    "mine".into(),
                    "type:Bug".into(),
                    "type:User Story".into(),
                    "tag:paiement".into(),
                ],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(page.total, Some(60));
    assert_eq!(page.next.as_deref(), Some("50"));
    assert_eq!(page.issues.len(), 50);
    let one = &page.issues[0];
    assert_eq!((one.id.as_str(), one.key.as_str()), ("1001", "#1001"));
    assert_eq!(one.title, "Story 1");
    assert_eq!(
        (one.kind.as_str(), one.kind_code.as_str()),
        ("User Story", "story")
    );
    assert_eq!(
        one.meta,
        [
            "Active",
            "paiement",
            "urgent",
            "Ada Lovelace",
            "par Grace Hopper"
        ]
    );
    assert_eq!(
        one.url,
        format!("{}/azure/acme/Web%20Shop/_workitems/edit/1001", server.url)
    );
    assert_eq!(
        one.description,
        "Le paiement plante.\n**Critères d'acceptation**\n- Description seule"
    );
    // Its Acceptance Criteria field, not its description's.
    assert_eq!(one.criteria, ["Plus de crash", "Un test e2e"]);
    assert_eq!(one.service, Service::AZURE);
    assert_eq!(one.container, C);
    let chips: Vec<&str> = page.filters.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(
        chips,
        [
            "mine",
            "type:User Story",
            "type:Bug",
            "type:Task",
            "tag:paiement",
            "tag:urgent"
        ]
    );
    assert_eq!(page.filters[0].label, "Assignés à moi");

    let wiql = server
        .requests()
        .into_iter()
        .find(|r| r.method == "POST")
        .unwrap();
    assert_eq!(wiql.query("$top").as_deref(), Some("1000"));
    assert_eq!(wiql.headers["content-type"], "application/json");
    assert_eq!(
        wiql.json()["query"],
        "SELECT [System.Id] FROM WorkItems WHERE [System.TeamProject] = @project \
         AND [System.WorkItemType] IN ('Bug', 'User Story') \
         AND [System.State] NOT IN ('Closed', 'Removed') \
         AND [System.AssignedTo] = @Me AND ([System.Tags] CONTAINS 'paiement') \
         ORDER BY [System.ChangedDate] DESC"
    );
    let read = server
        .requests()
        .into_iter()
        .find(|r| r.path().ends_with("/wit/workitems"))
        .unwrap();
    let asked: Vec<String> = (1001..=1050).map(|i| i.to_string()).collect();
    assert_eq!(read.query("ids"), Some(asked.join(",")));

    // The next page: the 10 left.
    let rest: Vec<Value> = (51..=60)
        .map(|i| item(1000 + i, "Bug", &format!("Bug {i}")))
        .collect();
    server.on(
        "GET",
        &format!("{P}/wit/workitems"),
        200,
        json!({ "count": 10, "value": rest }),
    );
    let page = b
        .issues(
            C,
            &Query {
                page: Some("50".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(page.issues.len(), 10);
    assert_eq!(page.issues[0].key, "#1051");
    assert_eq!(page.issues[0].kind_code, "bug");
    assert_eq!(page.next, None);
    let last = server
        .requests()
        .into_iter()
        .rfind(|r| r.path().ends_with("/wit/workitems"))
        .unwrap();
    assert_eq!(
        last.query("ids"),
        Some(
            (1051..=1060)
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",")
        )
    );
}

#[tokio::test]
async fn a_number_a_text_and_the_automatic_imports_label_narrow_the_query() {
    let (server, b) = boards().await;
    server.on("GET", &format!("{P}/wit/workitemtypes"), 200, basic());
    server.on(
        "POST",
        &format!("{P}/wit/wiql"),
        200,
        json!({ "workItems": [] }),
    );
    let query = |q: Query| {
        let b = &b;
        let server = &server;
        async move {
            let page = b.issues(C, &q).await.unwrap();
            assert!(page.issues.is_empty());
            assert_eq!((page.total, page.next), (Some(0), None));
            server
                .requests()
                .into_iter()
                .rfind(|r| r.method == "POST")
                .unwrap()
                .json()["query"]
                .as_str()
                .unwrap()
                .to_string()
        }
    };
    let q = query(Query {
        text: " #12 ".into(),
        ..Default::default()
    })
    .await;
    assert!(q.contains("AND [System.Id] = 12 ORDER BY"), "{q}");
    assert!(
        q.contains("[System.WorkItemType] IN ('Epic', 'Issue', 'Task')"),
        "{q}"
    );
    assert!(q.contains("[System.State] NOT IN ('Done')"), "{q}");
    let q = query(Query {
        text: "l'écran".into(),
        label: Some("claude".into()),
        ..Default::default()
    })
    .await;
    assert!(
        q.contains("([System.Tags] CONTAINS 'claude') AND [System.Title] CONTAINS 'l''écran'"),
        "{q}"
    );
    // No work item read for an empty page.
    assert!(!server
        .requests()
        .iter()
        .any(|r| r.path().ends_with("/wit/workitems")));
}

#[tokio::test]
async fn without_acceptance_criteria_they_come_from_the_description() {
    let (server, b) = boards().await;
    server.on("GET", &format!("{P}/wit/workitemtypes"), 200, agile());
    server.on(
        "POST",
        &format!("{P}/wit/wiql"),
        200,
        json!({ "workItems": [{ "id": 7 }] }),
    );
    let mut bug = item(7, "Bug", "Le panier se vide");
    let f = bug["fields"].as_object_mut().unwrap();
    f.remove("Microsoft.VSTS.Common.AcceptanceCriteria");
    f.remove("System.Description");
    f.remove("System.AssignedTo");
    f.insert(
        "Microsoft.VSTS.TCM.ReproSteps".into(),
        json!("<p>Ajouter un article</p><h3>Acceptance criteria</h3><ul><li>Le panier reste plein</li></ul>"),
    );
    // A former identity given as a text.
    f.insert(
        "System.CreatedBy".into(),
        json!("Grace Hopper <grace@acme.dev>"),
    );
    server.on(
        "GET",
        &format!("{P}/wit/workitems"),
        200,
        json!({ "value": [bug] }),
    );
    let page = b.issues(C, &Query::default()).await.unwrap();
    let one = &page.issues[0];
    assert_eq!(
        one.description,
        "Ajouter un article\n\n### Acceptance criteria\n\n- Le panier reste plein"
    );
    assert_eq!(one.criteria, ["Le panier reste plein"]);
    assert_eq!(
        one.meta,
        [
            "Active",
            "paiement",
            "urgent",
            "Non assigné",
            "par Grace Hopper"
        ]
    );
}

#[tokio::test]
async fn a_state_is_pushed_in_json_patch_and_a_comment_in_markdown() {
    let (server, b) = boards().await;
    server.on("GET", &format!("{P}/wit/workitemtypes"), 200, agile());
    let at = format!("{P}/wit/workitems/42");
    server.on(
        "GET",
        &at,
        200,
        json!({ "id": 42, "fields": { "System.State": "New", "System.WorkItemType": "User Story" } }),
    );
    server.on("PATCH", &at, 200, json!({ "id": 42, "rev": 4 }));
    b.set_state(&r("42"), &named("Active"), &[]).await.unwrap();
    let w = server.writes();
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].method, "PATCH");
    assert_eq!(w[0].headers["content-type"], "application/json-patch+json");
    assert_eq!(w[0].query("api-version").as_deref(), Some("7.1"));
    assert_eq!(
        w[0].json(),
        json!([{ "op": "add", "path": "/fields/System.State", "value": "Active" }])
    );
    let read = server
        .requests()
        .into_iter()
        .find(|r| r.path() == at)
        .unwrap();
    assert_eq!(
        read.query("fields").as_deref(),
        Some("System.State,System.WorkItemType")
    );

    // Already in it: nothing written.
    b.set_state(&r("42"), &named("New"), &[]).await.unwrap();
    assert_eq!(server.writes().len(), 1);

    server.on(
        "POST",
        &format!("{at}/comments"),
        200,
        json!({ "workItemId": 42, "commentId": 9, "version": 1, "text": "<p>Prise en charge</p>" }),
    );
    b.comment(&r("42"), "**ESC-1** pris en charge")
        .await
        .unwrap();
    let c = server.writes().pop().unwrap();
    assert_eq!(c.method, "POST");
    assert_eq!(c.query("format").as_deref(), Some("markdown"));
    assert_eq!(c.query("api-version").as_deref(), Some("7.1-preview.4"));
    assert_eq!(c.json(), json!({ "text": "**ESC-1** pris en charge" }));
}

#[tokio::test]
async fn a_state_its_type_lacks_is_its_first_of_the_same_category() {
    let (server, b) = boards().await;
    server.on("GET", &format!("{P}/wit/workitemtypes"), 200, scrum());
    let at = format!("{P}/wit/workitems/5");
    server.on(
        "GET",
        &at,
        200,
        json!({ "id": 5, "fields": { "System.State": "To Do", "System.WorkItemType": "Task" } }),
    );
    server.on("PATCH", &at, 200, json!({ "id": 5 }));
    // « Committed » is a backlog item's: a task's in progress is « In Progress ».
    b.set_state(&r("5"), &named("Committed"), &[])
        .await
        .unwrap();
    assert_eq!(server.writes()[0].json()[0]["value"], json!("In Progress"));
}

#[tokio::test]
async fn a_token_without_the_work_items_right_is_told_where_to_make_another() {
    let (server, b) = boards().await;
    server.on("GET", &format!("{P}/wit/workitemtypes"), 200, agile());
    server.on(
        "POST",
        &format!("{P}/wit/wiql"),
        401,
        json!({ "message": "TF400813: The user 'ada' is not authorized to access this resource.", "typeKey": "UnauthorizedRequestException" }),
    );
    let e = b.issues(C, &Query::default()).await.unwrap_err();
    let text = e.to_string();
    assert!(
        text.starts_with("Azure DevOps refuse ces identifiants (401) — TF400813"),
        "{text}"
    );
    assert!(
        text.contains("droit Work Items (lecture et écriture)"),
        "{text}"
    );
    assert!(
        text.contains("https://dev.azure.com/acme/_usersSettings/tokens"),
        "{text}"
    );
    // It may be put right: tried again later.
    assert!(!lasting(&e));
    assert_eq!(e.downcast_ref::<HttpError>().unwrap().status, 401);

    // A token Azure does not take at all: its sign-in page, with a 203.
    server.on_text(
        "GET",
        &format!("{P}/wit/workitemtypes"),
        203,
        "<html>Sign in</html>",
        &[],
    );
    let e = b.states(C).await.unwrap_err();
    assert!(e.to_string().contains("Work Items"), "{e}");
}

#[tokio::test]
async fn azure_refusing_a_state_or_a_work_item_gone_is_lasting() {
    let (server, b) = boards().await;
    server.on("GET", &format!("{P}/wit/workitemtypes"), 200, agile());
    let at = format!("{P}/wit/workitems/42");
    server.on(
        "GET",
        &at,
        200,
        json!({ "id": 42, "fields": { "System.State": "New", "System.WorkItemType": "Bug" } }),
    );
    server.on(
        "PATCH",
        &at,
        400,
        json!({ "message": "TF401320: Rule Error for field State. Error code: Required, InvalidEmpty." }),
    );
    let e = b
        .set_state(&r("42"), &named("Resolved"), &[])
        .await
        .unwrap_err();
    assert!(lasting(&e));
    assert!(e.to_string().contains("TF401320"), "{e}");
    server.on(
        "POST",
        &format!("{at}/comments"),
        404,
        json!({ "message": "TF401232: Work item 42 does not exist." }),
    );
    let e = b.comment(&r("42"), "x").await.unwrap_err();
    assert!(lasting(&e));
    // An organization no account is connected for.
    let e = b.states("initech/Web").await.unwrap_err().to_string();
    assert!(e.contains("initech"), "{e}");
}

#[tokio::test]
async fn the_account_is_named_by_azure() {
    let (server, b) = boards().await;
    server.on(
        "GET",
        "/azure/acme/_apis/connectionData",
        200,
        json!({ "authenticatedUser": { "id": "u-ada", "descriptor": "Microsoft.IdentityModel.Claims.ClaimsIdentity;ada", "providerDisplayName": "Ada Lovelace", "isActive": true } }),
    );
    assert_eq!(
        b.me().await.unwrap(),
        ("Ada Lovelace".to_string(), "u-ada".to_string())
    );
}
