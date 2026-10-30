//! The code index through the tools (ESC-253), called by a real MCP client (rmcp's) on a test core:
//! five tools, each answer saying what it saved, the savings counted per agent and per project, an
//! index that is off said clearly with how to turn it on, and nothing of a private project for the
//! agents of the others.

use super::private_tests::make_private;
use super::tools_tests::{
    as_agent, call, external, other_project, project, put_agent, read, refused,
};
use crate::core_tests::{harness, Harness};
use crate::model::Project;
use serde_json::json;

const SHOP: &str = "/// Places an order.\npub fn place_order(cart: u32) {\n    checkout(cart);\n}\n\npub fn checkout(cart: u32) {\n    let _ = cart;\n}\n\npub fn unrelated_helper() {}\n";

/// Files in the project's folder, and the index turned on.
async fn indexed(h: &Harness, p: &Project) {
    std::fs::write(std::path::Path::new(&p.path).join("shop.rs"), SHOP).unwrap();
    std::fs::write(
        std::path::Path::new(&p.path).join("main.rs"),
        "mod shop;\nfn main() { shop::place_order(1); }\n",
    )
    .unwrap();
    h.core.set_code_index(&p.id, Some(true)).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_five_tools_answer_from_the_index_and_say_what_they_saved() {
    let h = harness("mcp-index-tools");
    let p = project(&h).await;
    indexed(&h, &p).await;
    let c = external(&h).await;

    let found = read(
        &c,
        "index_search",
        json!({ "project": p.name, "query": "place order" }),
    )
    .await;
    assert_eq!(found["result"][0]["name"], "place_order");
    assert_eq!(found["result"][0]["path"], "shop.rs");
    // Before / after, in numbers and in words.
    let s = &found["saving"];
    assert!(s["filesTokens"].as_u64().unwrap() > 0, "{found}");
    assert!(s["answerTokens"].as_u64().unwrap() > 0);
    // shop.rs, and main.rs: the signature of `main` (its first line) names place_order.
    assert_eq!(s["files"], 2);
    assert!(found["summary"].as_str().unwrap().contains("tokens"));

    let ctx = read(
        &c,
        "index_context",
        json!({ "project": p.id, "task": "change how an order is placed" }),
    )
    .await;
    assert_eq!(ctx["result"]["entryPoints"][0]["name"], "place_order");
    assert!(ctx["saving"].is_object());
    // How the search went, in data and in words (ESC-296).
    assert!(ctx["result"]["retrieval"]["match"].is_string(), "{ctx}");
    let words = ctx["retrieval"].as_str().unwrap();
    assert!(words.contains("order("), "{words}");
    // Keywords: a notion the task does not name.
    let ctx = read(
        &c,
        "index_context",
        json!({ "project": p.id, "task": "tidy things up", "keywords": ["unrelated"] }),
    )
    .await;
    let entry = ctx["result"]["entryPoints"].to_string();
    assert!(entry.contains("unrelated_helper"), "{ctx}");
    let words = ctx["retrieval"].as_str().unwrap();
    assert!(words.contains("unrelated("), "{words}");
    assert!(words.contains("⚠"), "{words}");

    let callers = read(
        &c,
        "index_callers",
        json!({ "project": p.id, "name": "checkout" }),
    )
    .await;
    assert_eq!(
        callers["result"]["matches"][0]["callers"][0]["name"],
        "place_order"
    );

    let callees = read(
        &c,
        "index_callees",
        json!({ "project": p.id, "name": "place_order" }),
    )
    .await;
    assert_eq!(
        callees["result"]["matches"][0]["calls"][0]["name"],
        "checkout"
    );

    let impact = read(
        &c,
        "index_impact",
        json!({ "project": p.id, "target": "checkout" }),
    )
    .await;
    let files = impact["result"]["matches"][0]["files"].to_string();
    assert!(files.contains("shop.rs"), "{impact}");

    // Nothing found says so, and counts nothing read.
    let none = read(
        &c,
        "index_search",
        json!({ "project": p.id, "query": "zzzqqq" }),
    )
    .await;
    assert_eq!(none["result"], json!([]));
    assert_eq!(none["saving"]["files"], 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_may_leave_the_project_out_and_the_savings_are_counted_per_agent_and_project() {
    let h = harness("mcp-index-savings");
    let p = project(&h).await;
    indexed(&h, &p).await;
    let a = put_agent(&h, &p, "alpha", 1);
    let b = put_agent(&h, &p, "beta", 2);
    let ca = as_agent(&h, &a.id).await;
    let cb = as_agent(&h, &b.id).await;
    // The agent's own project, by default.
    let found = read(&ca, "index_search", json!({ "query": "checkout" })).await;
    assert_eq!(found["projectId"], json!(p.id));
    read(&ca, "index_callers", json!({ "name": "checkout" })).await;
    read(&cb, "index_search", json!({ "query": "place_order" })).await;

    let by_agent = h.core.stats.index_savings(true, Some(&p.id));
    let of = |id: &str| by_agent.iter().find(|s| s.agent_id == id).cloned().unwrap();
    assert_eq!(of(&a.id).calls, 2);
    assert_eq!(of(&b.id).calls, 1);
    assert!(of(&a.id).files_tokens > 0);
    let by_project = h.core.stats.index_savings(false, None);
    assert_eq!(by_project.len(), 1);
    assert_eq!(by_project[0].project_id, p.id);
    assert_eq!(by_project[0].calls, 3);
    assert_eq!(
        by_project[0].saved_tokens,
        by_agent.iter().map(|s| s.saved_tokens).sum::<u64>()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_index_that_is_off_says_so_and_how_to_turn_it_on() {
    let h = harness("mcp-index-off");
    let p = project(&h).await;
    let c = external(&h).await;
    let text = refused(
        &c,
        "index_search",
        json!({ "project": p.name, "query": "x" }),
    )
    .await;
    assert!(text.contains("coupé") || text.contains("off"), "{text}");
    assert!(
        text.contains("réglages") || text.contains("settings"),
        "{text}"
    );
    // An external client has to name the project.
    let text = refused(&c, "index_search", json!({ "query": "x" })).await;
    assert!(
        text.contains("projet") || text.contains("project"),
        "{text}"
    );
    // Nothing was read nor counted.
    assert!(h.core.stats.index_savings(false, None).is_empty());
    assert!(!h.core.data.code_index_db(&p.id).exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_is_answered_for_its_worktree_and_a_private_project_stays_hidden() {
    let h = harness("mcp-index-sight");
    let open = project(&h).await;
    let vault = other_project(&h, "vault").await;
    std::fs::write(
        std::path::Path::new(&vault.path).join("secret.rs"),
        "pub fn secret_vault_fn() {}\n",
    )
    .unwrap();
    indexed(&h, &open).await;
    h.core.set_code_index(&vault.id, Some(true)).await.unwrap();
    let vault = make_private(&h, &vault);
    let a = put_agent(&h, &open, "alpha", 1);
    let v = put_agent(&h, &vault, "keeper", 2);

    // An agent of an open project does not find the private project, by name or by id.
    let ca = as_agent(&h, &a.id).await;
    for named in [vault.name.as_str(), vault.id.as_str()] {
        let text = refused(
            &ca,
            "index_search",
            json!({ "project": named, "query": "secret" }),
        )
        .await;
        assert!(!text.contains("secret_vault_fn"), "{text}");
        assert!(!text.contains(&vault.path), "{text}");
    }
    // An agent of the private project sees its own, and no other.
    let cv = as_agent(&h, &v.id).await;
    let own = read(&cv, "index_search", json!({ "query": "secret vault" })).await;
    assert_eq!(own["result"][0]["name"], "secret_vault_fn");
    refused(
        &cv,
        "index_search",
        json!({ "project": open.name, "query": "checkout" }),
    )
    .await;
    // The user's own client sees both.
    let c = external(&h).await;
    read(
        &c,
        "index_search",
        json!({ "project": vault.name, "query": "secret" }),
    )
    .await;

    // An agent in a worktree of the project sees what it changed there.
    let wt = std::path::Path::new(&open.path).join(".claude/worktrees/wt1");
    std::fs::create_dir_all(&wt).unwrap();
    std::fs::write(wt.join("shop.rs"), "pub fn only_in_worktree() {}\n").unwrap();
    h.core.agent(&a.id).unwrap().lock().meta.cwd = wt.to_string_lossy().into_owned();
    let seen = call(&ca, "index_search", json!({ "query": "only in worktree" })).await;
    assert!(!seen.1, "{}", seen.0);
}
