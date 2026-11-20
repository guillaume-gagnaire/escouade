//! The MCP tools of the features switched off (F-1, ESC-550), called by a real MCP client on a test core: absent from
//! the list, refused with the reason when called all the same (a client that read the list before), back with their
//! data once the feature is on again. One test per feature that has tools; the editor, the voice, the cost and the
//! statistics have none (`features_tests`).

use super::tests::names;
use super::tools_tests::Client;
use super::tools_tests::{call, external, project, put, read, refused, ticket};
use crate::core_tests::{harness, Harness};
use crate::features::{Feature, TOOLS};
use crate::model::Column;
use serde_json::json;

/// The switch of `f`, as Réglages › Fonctionnalités sets it. Written in place: a save would stop the test's MCP
/// server (shared memory, `mcp-tests-save-settings-stops-server`).
fn switch(h: &Harness, f: Feature, on: bool) {
    h.core
        .settings
        .write()
        .features
        .get_or_insert_with(Default::default)
        .set(f, on);
}

fn tools_of(f: Feature) -> &'static [&'static str] {
    TOOLS.iter().find(|(x, _)| *x == f).unwrap().1
}

/// The tools of `f`: none listed while it is off, every one again once it is on.
async fn listed(c: &Client, f: Feature) -> usize {
    let all = names(&c.list_all_tools().await.unwrap());
    tools_of(f)
        .iter()
        .filter(|t| all.iter().any(|n| n == *t))
        .count()
}

/// `f` off: its tools leave the list and each is refused with the reason, whatever its arguments.
async fn gone(h: &Harness, c: &Client, f: Feature) {
    switch(h, f, false);
    assert_eq!(listed(c, f).await, 0, "{}", f.id());
    for tool in tools_of(f) {
        let tool: &'static str = tool;
        let why = refused(c, tool, json!({})).await;
        assert!(
            why.contains(f.name(true)) && why.contains("Réglages › Fonctionnalités"),
            "{tool}: {why}"
        );
    }
    // The essential stays.
    let all = names(&c.list_all_tools().await.unwrap());
    assert!(all.iter().any(|n| n == "list_agents"));
    switch(h, f, true);
    assert_eq!(listed(c, f).await, tools_of(f).len(), "{}", f.id());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_kanban_off_its_tools_go_and_its_tickets_come_back_with_it() {
    let h = harness("mcp-feature-kanban");
    let p = project(&h).await;
    put(&h, ticket(&p, "P-1", "Garder ce ticket", Column::Todo, 1));
    let c = external(&h).await;
    gone(&h, &c, Feature::Kanban).await;
    let list = read(&c, "list_tickets", json!({ "project": p.id })).await;
    assert!(list.to_string().contains("Garder ce ticket"), "{list}");
    // The refusal is in the activity log, as refused.
    switch(&h, Feature::Kanban, false);
    let _ = call(&c, "get_board", json!({ "project": p.id })).await;
    let last = h.core.mcp.activity.entries().pop().unwrap();
    assert_eq!((last.tool.as_str(), last.outcome), ("get_board", "refused"));
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_docs_off_their_tools_go_and_their_pages_come_back_with_them() {
    let h = harness("mcp-feature-docs");
    let p = project(&h).await;
    let c = external(&h).await;
    read(
        &c,
        "write_spec_page",
        json!({ "project": p.id, "path": "general/garde", "body": "Elle reste." }),
    )
    .await;
    gone(&h, &c, Feature::Docs).await;
    let page = read(
        &c,
        "read_spec_page",
        json!({ "project": p.id, "path": "general/garde" }),
    )
    .await;
    assert!(page["body"].as_str().unwrap().contains("Elle reste."));
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_workflows_off_their_tool_goes() {
    let h = harness("mcp-feature-workflows");
    let p = project(&h).await;
    let c = external(&h).await;
    gone(&h, &c, Feature::Workflows).await;
    read(&c, "get_workflow", json!({ "project": p.id })).await;
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_scheduled_tasks_off_their_tools_go_with_them_and_with_the_kanban() {
    let h = harness("mcp-feature-scheduled");
    let p = project(&h).await;
    let c = external(&h).await;
    read(
        &c,
        "create_scheduled_task",
        json!({ "project": p.id, "title": "Revue", "description": "Relis.", "rhythm": "daily", "time": "07:30" }),
    )
    .await;
    gone(&h, &c, Feature::Scheduled).await;
    // They go with the Kanban too, their runs being tickets: said so.
    switch(&h, Feature::Kanban, false);
    assert_eq!(listed(&c, Feature::Scheduled).await, 0);
    let why = refused(&c, "list_scheduled_tasks", json!({ "project": p.id })).await;
    assert!(why.contains("Kanban"), "{why}");
    switch(&h, Feature::Kanban, true);
    let list = read(&c, "list_scheduled_tasks", json!({ "project": p.id })).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_overseer_off_its_tools_go() {
    let h = harness("mcp-feature-overseer");
    let c = external(&h).await;
    gone(&h, &c, Feature::Overseer).await;
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_feature_imposed_off_says_who_imposes_it() {
    let h = harness("mcp-feature-locked");
    let p = project(&h).await;
    let c = external(&h).await;
    // As `set_managed` brings the settings within the policy, without saving them (which would stop the server).
    *h.core.locks.write() = crate::locks::Locks::compute(
        crate::locks::KEYS,
        None,
        &crate::locks::Managed {
            values: [(
                "features.docs".to_string(),
                (json!(false), "the machine's policy".to_string()),
            )]
            .into(),
            warnings: Vec::new(),
        },
    );
    switch(&h, Feature::Docs, false);
    let why = refused(&c, "get_spec_tree", json!({ "project": p.id })).await;
    assert!(why.contains("par la stratégie de ce poste"), "{why}");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}
