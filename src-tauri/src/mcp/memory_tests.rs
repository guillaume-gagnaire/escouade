//! The tools of the shared memory (ESC-255), called by a real MCP client (rmcp's) on a test core: one memory in the
//! project's folder for every agent, whatever its harness or its worktree; the project left out by an agent, given by
//! Claude outside Escouade; a name refused, a memory forgotten. The private projects are in `private_tests`.

use super::tools_tests::{as_agent, call, external, last_entry, project, put_agent, read, refused};
use crate::core_tests::harness;
use crate::model::*;
use serde_json::json;
use std::path::Path;

#[tokio::test(flavor = "multi_thread")]
async fn the_agents_of_every_harness_share_one_memory_in_the_projects_folder() {
    let h = harness("mcp-memory-shared");
    let p = project(&h).await;
    // Claude Code in a worktree of its own, Codex in the project's folder.
    let worktree = h.dir.join("wt-claude");
    std::fs::create_dir_all(&worktree).unwrap();
    let claude = put_agent(&h, &p, "Claude", 1);
    h.core.agent(&claude.id).unwrap().lock().meta.cwd = worktree.to_string_lossy().into();
    let codex = put_agent(&h, &p, "Codex", 2);
    h.core.agent(&codex.id).unwrap().lock().meta.engine = EngineId::CODEX;

    let c = as_agent(&h, &claude.id).await;
    let written = read(
        &c,
        "write_memory",
        json!({
            "name": "E2E après build",
            "description": "Lancer les e2e après un build de debug",
            "body": "`npx tauri build --debug --no-bundle` d'abord : sinon l'app testée est l'ancienne."
        }),
    )
    .await;
    assert_eq!(written["memory"]["name"], "e2e-apres-build");
    assert_eq!(written["memory"]["kind"], "feedback");
    assert_eq!(written["new"], true);
    let file = Path::new(&p.path).join(".escouade/memory/e2e-apres-build.md");
    assert!(file.is_file(), "in the project's folder");
    assert!(
        !worktree.join(".escouade").exists(),
        "not in the agent's worktree"
    );
    assert!(Path::new(&p.path)
        .join(".escouade/memory/MEMORY.md")
        .is_file());
    let (who, tool, summary, outcome) = last_entry(&h);
    assert_eq!((tool.as_str(), outcome), ("write_memory", "ok"), "{who}");
    assert_eq!(summary, "name: E2E après build");

    // Codex finds it by the words, and reads it whole.
    let x = as_agent(&h, &codex.id).await;
    let found = read(&x, "search_memory", json!({ "query": "build debug" })).await;
    assert_eq!(found["by"], "words");
    assert_eq!(found["memories"][0]["name"], "e2e-apres-build");
    assert_eq!(
        found["memories"][0]["description"],
        "Lancer les e2e après un build de debug"
    );
    let one = read(&x, "read_memory", json!({ "name": "e2e-apres-build" })).await;
    assert!(one["memory"]["body"]
        .as_str()
        .unwrap()
        .contains("--no-bundle"));

    // Claude outside Escouade names the project.
    let e = external(&h).await;
    let err = refused(&e, "search_memory", json!({ "query": "e2e" })).await;
    assert!(err.contains("projet") || err.contains("project"), "{err}");
    let found = read(
        &e,
        "search_memory",
        json!({ "project": p.name, "query": "e2e" }),
    )
    .await;
    assert_eq!(found["memories"].as_array().unwrap().len(), 1);

    // Rewritten, then forgotten by Codex.
    let again = read(
        &x,
        "write_memory",
        json!({ "name": "e2e-apres-build", "description": "Autre", "type": "project", "body": "Autre" }),
    )
    .await;
    assert_eq!(
        (again["new"].as_bool(), again["memory"]["kind"].as_str()),
        (Some(false), Some("project"))
    );
    read(&x, "forget_memory", json!({ "name": "e2e-apres-build" })).await;
    assert!(!file.exists());
    let err = refused(&x, "read_memory", json!({ "name": "e2e-apres-build" })).await;
    assert!(err.contains("e2e-apres-build"), "{err}");
    refused(&x, "forget_memory", json!({ "name": "e2e-apres-build" })).await;
    for client in [c, x, e] {
        client.cancel().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_memory_without_a_description_or_a_name_is_refused() {
    let h = harness("mcp-memory-refused");
    let p = project(&h).await;
    let a = put_agent(&h, &p, "Alice", 1);
    let c = as_agent(&h, &a.id).await;
    refused(
        &c,
        "write_memory",
        json!({ "name": "x", "description": " ", "body": "b" }),
    )
    .await;
    refused(
        &c,
        "write_memory",
        json!({ "name": "--", "description": "d", "body": "b" }),
    )
    .await;
    let (text, error) = call(
        &c,
        "write_memory",
        json!({ "name": "x", "description": "d", "type": "secret", "body": "b" }),
    )
    .await;
    assert!(error, "an unknown type: {text}");
    assert!(!Path::new(&p.path).join(".escouade/memory").exists());
    c.cancel().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_agent_is_told_of_the_memory() {
    let h = harness("mcp-memory-told");
    let p = project(&h).await;
    let made = h
        .core
        .create_agent_with(&p.id, crate::core::AgentOptions::default())
        .await
        .unwrap();
    let told = h
        .core
        .agent(&made.meta.id)
        .unwrap()
        .lock()
        .meta
        .append_prompt
        .clone();
    let told = told.unwrap_or_default();
    for tool in ["search_memory", "write_memory"] {
        assert!(told.contains(tool), "{told}");
    }
}
