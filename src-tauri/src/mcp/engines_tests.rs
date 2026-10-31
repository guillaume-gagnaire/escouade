//! The harness of the agents, as `list_agents` and `get_agent_summary` tell it (ESC-27).

use super::tools_tests::{external, project, put_agent, read};
use crate::core_tests::harness;
use crate::model::*;
use serde_json::json;

#[tokio::test]
async fn the_agents_say_the_harness_they_run_on() {
    let h = harness("mcp-engines");
    let p = project(&h).await;
    // One saved before 1.8 (Claude Code, the default), and one on each other harness.
    let mut agents = vec![(put_agent(&h, &p, "claude", 1), "claude")];
    for (at, (engine, id)) in [
        (EngineId::CODEX, "codex"),
        (EngineId::COPILOT, "copilot"),
        (EngineId::CURSOR, "cursor"),
        (EngineId::OPENCODE, "opencode"),
    ]
    .into_iter()
    .enumerate()
    {
        let meta = put_agent(&h, &p, id, at as i64 + 2);
        h.core.agent(&meta.id).unwrap().lock().meta.engine = engine;
        agents.push((meta, id));
    }
    let c = external(&h).await;
    let all = read(&c, "list_agents", json!({})).await;
    let listed: Vec<(String, String)> = all
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            (
                a["name"].as_str().unwrap().to_string(),
                a["engine"].as_str().unwrap_or("absent").to_string(),
            )
        })
        .collect();
    let expected: Vec<(String, String)> = agents
        .iter()
        .map(|(m, id)| (m.name.clone(), id.to_string()))
        .collect();
    assert_eq!(listed, expected);
    for (meta, id) in &agents {
        let summary = read(&c, "get_agent_summary", json!({ "agent": meta.id })).await;
        assert_eq!(summary["engine"], json!(id), "{}", meta.name);
    }
}
