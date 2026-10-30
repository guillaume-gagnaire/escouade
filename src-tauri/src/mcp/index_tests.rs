//! The code index through the tools (ESC-253, ESC-298), called by a real MCP client (rmcp's) on a
//! test core: six tools answering as tokensave does (Markdown for the context, a line per symbol
//! for the search, the body as text, JSON for the calls), none of them telling the agent what it saved
//! (ESC-307), the count kept for the statistics per agent and per project, an index that is off said
//! clearly with how to turn it on, and nothing of a private project for the agents of the others.

use super::private_tests::make_private;
use super::tools_tests::{
    as_agent, call, external, other_project, project, put_agent, read, refused, Client,
};
use crate::core_tests::{harness, Harness};
use crate::model::Project;
use rmcp::model::CallToolRequestParams;
use serde_json::{json, Value};

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

/// The blocks of an answer that is no error: its text, then what follows it (nothing: no line of
/// savings is sent, ESC-307). Every answer is checked for the words of one.
async fn blocks(c: &Client, tool: &'static str, args: Value) -> (String, String) {
    let mut params = CallToolRequestParams::new(tool);
    if let Value::Object(args) = args {
        params = params.with_arguments(args);
    }
    let answer = serde_json::to_value(c.call_tool(params).await.unwrap()).unwrap();
    assert!(answer["isError"] != true, "{tool}: {answer}");
    let text = |i: usize| {
        answer["content"][i]["text"]
            .as_str()
            .unwrap_or("")
            .to_string()
    };
    let blocks = answer["content"].as_array().map_or(0, Vec::len);
    assert_eq!(blocks, 1, "{tool}: one block, no line of savings: {answer}");
    for word in [
        "index_metrics",
        "Économie",
        "économie",
        "Saving",
        "saved:",
        "before=",
    ] {
        assert!(!text(0).contains(word), "{tool}: {word} in {}", text(0));
    }
    (text(0), text(1))
}

#[tokio::test(flavor = "multi_thread")]
async fn the_six_tools_answer_as_tokensave_and_say_nothing_of_savings() {
    let h = harness("mcp-index-tools");
    let p = project(&h).await;
    indexed(&h, &p).await;
    let c = external(&h).await;

    // The search: a line per symbol, the ids when asked.
    let (found, after_text) = blocks(
        &c,
        "index_search",
        json!({ "project": p.name, "query": "place order" }),
    )
    .await;
    assert!(found.starts_with("count: "), "{found}");
    assert!(
        found.contains("shop.rs:2-4: place_order (function)"),
        "{found}"
    );
    assert!(!found.contains("[id:"), "{found}");
    assert_eq!(after_text, "");
    let (with_ids, _) = blocks(
        &c,
        "index_search",
        json!({ "project": p.name, "query": "checkout", "ids": true }),
    )
    .await;
    let id: i64 = with_ids
        .split("[id: ")
        .nth(1)
        .and_then(|s| s.split(']').next())
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| panic!("{with_ids}"));

    // A literal search: a text inside a body, file:line.
    let (lit, _) = blocks(
        &c,
        "index_search",
        json!({ "project": p.id, "query": "let _ = cart", "literal": true, "ids": true }),
    )
    .await;
    assert!(lit.contains("shop.rs:7: let _ = cart;"), "{lit}");
    assert!(lit.contains(&format!("[in checkout, id: {id}]")), "{lit}");

    // The context, in Markdown, its code and its retrieval.
    let (ctx, _) = blocks(
        &c,
        "index_context",
        json!({ "project": p.id, "task": "change how an order is placed" }),
    )
    .await;
    assert!(ctx.starts_with("## Code Context\n"), "{ctx}");
    assert!(
        ctx.contains("- **place_order** (function) - shop.rs:2"),
        "{ctx}"
    );
    assert!(
        ctx.contains("#### place_order (shop.rs:2)\n```rust\n"),
        "{ctx}"
    );
    assert!(ctx.contains("### Retrieval\n"), "{ctx}");
    assert!(ctx.contains("order("), "{ctx}");
    assert!(ctx.contains("seen_node_ids: ["), "{ctx}");
    // Keywords: a notion the task does not name; what was seen is not given again.
    let (ctx, _) = blocks(
        &c,
        "index_context",
        json!({ "project": p.id, "task": "tidy things up", "keywords": ["unrelated"] }),
    )
    .await;
    assert!(ctx.contains("**unrelated_helper**"), "{ctx}");
    assert!(ctx.contains("unrelated("), "{ctx}");
    assert!(ctx.contains("⚠"), "{ctx}");
    let seen: Vec<i64> =
        serde_json::from_str(ctx.rsplit("seen_node_ids: ").next().unwrap().trim()).unwrap();
    let (again, _) = blocks(
        &c,
        "index_context",
        json!({
            "project": p.id,
            "task": "tidy things up",
            "keywords": ["unrelated"],
            "exclude_node_ids": seen
        }),
    )
    .await;
    assert!(!again.contains("**unrelated_helper**"), "{again}");
    // The path filters, and no code when it is not wanted.
    let (none, _) = blocks(
        &c,
        "index_context",
        json!({
            "project": p.id,
            "task": "place order",
            "path_exclude": ["shop"],
            "include_code": false
        }),
    )
    .await;
    assert!(!none.contains("**place_order**"), "{none}");
    assert!(!none.contains("```"), "{none}");

    // The body, by name or by id.
    let (body, _) = blocks(
        &c,
        "index_body",
        json!({ "project": p.id, "symbol": "checkout" }),
    )
    .await;
    assert!(
        body.starts_with(
            "match_count: 1\n\nfile: shop.rs:6-8: checkout (function)\npub fn checkout"
        ),
        "{body}"
    );
    let (by_id, _) = blocks(&c, "index_body", json!({ "project": p.id, "node_id": id })).await;
    assert_eq!(by_id, body);

    let callers = read(
        &c,
        "index_callers",
        json!({ "project": p.id, "name": "checkout" }),
    )
    .await;
    assert_eq!(callers["matches"][0]["callers"][0]["name"], "place_order");

    let callees = read(
        &c,
        "index_callees",
        json!({ "project": p.id, "name": "place_order" }),
    )
    .await;
    assert_eq!(callees["matches"][0]["calls"][0]["name"], "checkout");

    let impact = read(
        &c,
        "index_impact",
        json!({ "project": p.id, "target": "checkout" }),
    )
    .await;
    let files = impact["matches"][0]["files"].to_string();
    assert!(files.contains("shop.rs"), "{impact}");

    // Nothing found: no file touched.
    let (none, line) = blocks(
        &c,
        "index_search",
        json!({ "project": p.id, "query": "zzzqqq" }),
    )
    .await;
    assert_eq!(none, "count: 0\n\n");
    assert_eq!(line, "");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_paths_of_queryignore_are_left_out_of_the_search_and_the_context() {
    let h = harness("mcp-index-queryignore");
    let p = project(&h).await;
    indexed(&h, &p).await;
    let dir = std::path::Path::new(&p.path).join(".escouade");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("queryignore"), "# the shop is noise\nshop.rs\n").unwrap();
    let c = external(&h).await;
    let (found, _) = blocks(
        &c,
        "index_search",
        json!({ "project": p.id, "query": "place order" }),
    )
    .await;
    assert!(!found.contains("shop.rs"), "{found}");
    let (ctx, _) = blocks(
        &c,
        "index_context",
        json!({ "project": p.id, "task": "place order" }),
    )
    .await;
    assert!(!ctx.contains("**place_order**"), "{ctx}");
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
    let (found, _) = blocks(&ca, "index_search", json!({ "query": "checkout" })).await;
    assert!(found.contains("shop.rs"), "{found}");
    read(&ca, "index_callers", json!({ "name": "checkout" })).await;
    blocks(&cb, "index_search", json!({ "query": "place_order" })).await;

    let by_agent = h.core.stats.index_savings(true, Some(&p.id));
    let of = |id: &str| by_agent.iter().find(|s| s.agent_id == id).cloned().unwrap();
    assert_eq!(of(&a.id).calls, 2);
    assert_eq!(of(&b.id).calls, 1);
    assert!(of(&a.id).files_tokens > 0);
    // A small answer on small files, the schemas paid by its session: a debt, nothing credited.
    assert_eq!(of(&b.id).saved_tokens, 0);
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
async fn the_descriptions_stay_the_same_and_ask_for_the_words_of_the_code() {
    let h = harness("mcp-index-descriptions");
    let c = external(&h).await;
    let list = |tools: Vec<rmcp::model::Tool>| -> Vec<(String, String)> {
        tools
            .into_iter()
            .filter(|t| t.name.starts_with("index_"))
            .map(|t| {
                (
                    t.name.to_string(),
                    t.description.unwrap_or_default().to_string(),
                )
            })
            .collect()
    };
    let first = list(c.list_all_tools().await.unwrap());
    let p = project(&h).await;
    indexed(&h, &p).await;
    blocks(
        &c,
        "index_search",
        json!({ "project": p.id, "query": "checkout" }),
    )
    .await;
    let second = list(c.list_all_tools().await.unwrap());
    assert_eq!(first, second, "a description changed between two listings");
    assert_eq!(first.len(), 6);
    let of = |name: &str| first.iter().find(|(n, _)| n == name).unwrap().1.clone();
    assert!(of("index_search").contains("words of the code"));
    assert!(of("index_search").contains("literal"));
    assert!(of("index_context").contains("words of the code"));
    assert!(of("index_context").contains("`keywords`"));
    assert!(of("index_context").contains("index_body"));
    assert!(of("index_context").contains("exclude_node_ids"));
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
    let (own, _) = blocks(&cv, "index_search", json!({ "query": "secret vault" })).await;
    assert!(own.contains("secret_vault_fn"), "{own}");
    refused(
        &cv,
        "index_search",
        json!({ "project": open.name, "query": "checkout" }),
    )
    .await;
    // The user's own client sees both.
    let c = external(&h).await;
    blocks(
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
    assert!(seen.0.contains("only_in_worktree"), "{}", seen.0);
}
