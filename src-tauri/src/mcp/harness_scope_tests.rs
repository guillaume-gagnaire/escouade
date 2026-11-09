//! The tools and a harness off for a project (ESC-404): create_agent and start_ticket refuse it with why, and none
//! turns a plugin on for a project, which stays the user's gesture.

use super::private_tests::make_private;
use super::tools_tests::{call, external, project, put, ticket};
use crate::core_tests::harness;
use crate::model::*;
use serde_json::json;

const CLAUDE_PLUGIN: &str = crate::plugins::builtin::CLAUDE;

#[tokio::test]
async fn no_tool_starts_an_agent_on_a_harness_off_for_its_project_nor_turns_it_on() {
    let h = harness("esc404-mcp");
    let p = project(&h).await;
    let p = make_private(&h, &p);
    let t = put(&h, ticket(&p, "SEC-1", "Secret", Column::Todo, 1));
    let c = external(&h).await;
    let (text, error) = call(
        &c,
        "create_agent",
        json!({ "project": p.id, "message": "Bonjour" }),
    )
    .await;
    assert!(error, "{text}");
    assert!(text.contains("Claude Code"), "{text}");
    assert!(h.core.project_agents(&p.id).is_empty());
    let (text, error) = call(&c, "start_ticket", json!({ "ticket": t.key })).await;
    assert!(error, "{text}");
    assert!(text.contains("Claude Code"), "{text}");
    assert_eq!(h.core.ticket(&t.id).unwrap().column, Column::Todo);
    // Nothing of it turned the plugin on for the project.
    let choices = h.core.plugins.choices(CLAUDE_PLUGIN).unwrap();
    assert_eq!(choices.projects.get(&p.id), None);
    // A project that is not private, where the user turned it off: the same.
    let open = super::tools_tests::other_project(&h, "Ouvert").await;
    h.core
        .set_plugin_project(CLAUDE_PLUGIN, &open.id, false)
        .await
        .unwrap();
    let (text, error) = call(
        &c,
        "create_agent",
        json!({ "project": open.id, "message": "Bonjour" }),
    )
    .await;
    assert!(error, "{text}");
    assert!(h.core.project_agents(&open.id).is_empty());
}
