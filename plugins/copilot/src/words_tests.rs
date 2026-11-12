//! What the core tested of Copilot CLI's words before they left it (ESC-359), on the plugin's own.

use crate::words::*;
use serde_json::{json, Value};
use std::collections::HashMap;

#[test]
fn a_calls_cost_is_its_nano_aiu_else_its_details_else_its_models_prices() {
    // A call as the real runs told it (`assistant.usage`): its details make its total, to the nano-AIU.
    let d = json!({ "model": "gpt-6-luna", "copilotUsage": { "totalNanoAiu": 3_250_000, "tokenDetails": [
        { "tokenCount": 1000, "batchSize": 1000, "costPerBatch": 2_500_000 },
        { "tokenCount": 300, "batchSize": 1000, "costPerBatch": 2_500_000 },
    ] } });
    assert_eq!(call_nano(&d, &HashMap::new()), 3_250_000);
    let mut untold = d.clone();
    untold["copilotUsage"]
        .as_object_mut()
        .unwrap()
        .remove("totalNanoAiu");
    assert_eq!(call_nano(&untold, &HashMap::new()), 3_250_000);
    // Neither: the prices of its model, from `models.list`.
    let listed = json!({ "models": [
        { "id": "gpt-6-luna", "billing": { "multiplier": 1, "tokenPrices": {
            "inputPrice": 10, "cacheReadPrice": 1, "cacheWritePrice": 12.5,
            "outputPrice": 50, "batchSize": 1_000_000 } } },
        { "id": "gpt-5-mini", "billing": { "multiplier": 0.33 } },
    ] });
    let prices = prices(&listed);
    assert_eq!(prices.len(), 1);
    let usage = json!({ "model": "gpt-6-luna", "inputTokens": 12 + 40_145 + 13_657,
        "cacheReadTokens": 40_145, "cacheWriteTokens": 13_657, "outputTokens": 325 });
    assert_eq!(call_nano(&usage, &prices), 227_227_500);
    // An unknown model: nothing to count.
    assert_eq!(
        call_nano(&json!({ "model": "x", "inputTokens": 5 }), &prices),
        0
    );
}

#[test]
fn a_forms_answers_are_typed_as_its_schema_says() {
    let schema = json!({ "type": "object", "properties": {
        "prenom": { "type": "string", "title": "Prénom" },
        "age": { "type": "number", "title": "Âge" },
        "n": { "type": "integer" },
        "ok": { "type": "boolean" },
        "vide": { "type": "string" },
    } });
    let input = json!({
        "questions": [
            { "id": "prenom", "question": "Prénom" }, { "id": "age", "question": "Âge" },
            { "id": "n", "question": "n" }, { "id": "ok", "question": "ok" },
            { "id": "vide", "question": "vide" },
        ],
        "answers": { "Prénom": "Camille", "Âge": "30,5", "n": "4", "ok": "Oui", "vide": "" },
    });
    assert_eq!(
        form_content(&schema, &input),
        json!({ "prenom": "Camille", "age": 30.5, "n": 4, "ok": true })
    );
}

#[test]
fn a_free_account_lists_no_model_and_the_menu_keeps_its_own_with_their_price() {
    // As `session.model.list` answered on a Free account (the real run `first-turn.jsonl`).
    let of_session = json!({ "list": [], "modelPriceCategories": [
        { "id": "gpt-6-luna", "priceCategory": "low" }, { "id": "gpt-5-mini", "priceCategory": "low" },
    ] });
    let models = catalog(Some(&json!({ "models": [] })), Some(&of_session));
    let ids: Vec<&str> = models.iter().map(|m| m["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["gpt-6-luna", "gpt-5-mini"]);
    assert_eq!(models[0]["price"], "low");
    assert!(models[0]["default"] == true && models[1]["default"] == false);
    assert!(models.iter().all(|m| m["id"] != "auto"));
}

#[test]
fn a_listed_model_says_its_multiplier_efforts_and_images() {
    let listed = json!({ "models": [
        { "id": "claude-sonnet-4.5", "name": "Claude Sonnet 4.5",
          "capabilities": { "supports": { "vision": true } },
          "billing": { "multiplier": 3 },
          "supportedReasoningEfforts": ["low", "medium", "high"], "defaultReasoningEffort": "medium" },
        { "id": "gpt-5-mini", "name": "GPT-5 mini", "billing": { "multiplier": 0.33 } },
        { "id": "o9", "name": "O9", "policy": { "state": "disabled" } },
    ] });
    let models = catalog(Some(&listed), None);
    assert_eq!(models.len(), 2, "the disabled one left out");
    let sonnet = &models[0];
    assert_eq!(sonnet["multiplier"], "3");
    assert_eq!(sonnet["efforts"], json!(["low", "medium", "high"]));
    assert_eq!(sonnet["defaultEffort"], "medium");
    assert!(sonnet["images"] == true && sonnet["default"] == true);
    assert_eq!(models[1]["multiplier"], "0.33");
}

#[test]
fn a_permission_that_names_a_path_of_the_fence_is_refused_by_escouade() {
    let fence = vec![r"C:\Code\Secret".to_string()];
    // Read, written, in a command, among the paths a command may touch.
    for r in [
        json!({ "kind": "read", "path": "C:/code/secret/a.txt" }),
        json!({ "kind": "write", "fileName": r"C:\Code\Secret\b.rs" }),
        json!({ "kind": "shell", "fullCommandText": r"Get-Content C:\Code\Secret\a.txt" }),
        json!({ "kind": "shell", "fullCommandText": "x", "possiblePaths": [r"C:\Code\Secret"] }),
    ] {
        assert_eq!(
            fenced(&r, &fence),
            cfg!(windows) || r["kind"] != "read",
            "{r}"
        );
    }
    assert!(!fenced(
        &json!({ "kind": "read", "path": r"C:\Code\Other\a.txt" }),
        &fence
    ));
    assert!(!fenced(
        &json!({ "kind": "url", "url": "https://x" }),
        &fence
    ));
    // No fence, nothing refused.
    assert!(!fenced(
        &json!({ "kind": "read", "path": r"C:\Code\Secret\a.txt" }),
        &[]
    ));
    assert_ne!(fence_refusal("fr"), fence_refusal("en"));
}

#[test]
fn for_this_session_remembers_each_kind_and_never_writes_a_rule_of_copilots() {
    let of = |r: Value| session_approval(&json!({ "permissionRequest": r }));
    let shell = of(json!({ "kind": "shell", "commands": [{ "identifier": "npm test" }] }));
    assert_eq!(shell["kind"], "approve-for-session");
    assert_eq!(shell["approval"]["commandIdentifiers"], json!(["npm test"]));
    assert_eq!(of(json!({ "kind": "write" }))["approval"]["kind"], "write");
    assert_eq!(of(json!({ "kind": "read" }))["approval"]["kind"], "read");
    let mcp = of(json!({ "kind": "mcp", "serverName": "essai", "toolName": "ping" }));
    assert_eq!(mcp["approval"]["serverName"], "essai");
    assert_eq!(
        of(json!({ "kind": "url", "url": "https://example.com/a" }))["domain"],
        "example.com"
    );
    for kind in ["shell", "write", "read", "mcp", "url", "memory"] {
        let a = of(json!({ "kind": kind, "url": "https://x.y" }));
        let s = a.to_string();
        assert!(
            !s.contains("approve-permanently") && !s.contains("approve-for-location"),
            "{s}"
        );
    }
}

#[test]
fn the_months_credits_come_from_chat_and_a_reading_without_its_end_ends_next_month() {
    let at = |d: &str| {
        chrono::DateTime::parse_from_rfc3339(d)
            .unwrap()
            .timestamp_millis()
    };
    let now = at("2026-10-18T10:00:00Z");
    // A call's snapshot (`assistant.usage`): its own end.
    let call = json!({ "chat": { "entitlementRequests": 200, "usedRequests": 1,
        "remainingPercentage": 99.3, "resetDate": "2026-11-01T00:00:00Z" } });
    let m = credits(&call, now).unwrap();
    assert_eq!(m.pct, 0.7);
    assert_eq!(m.resets_at, (at("2026-11-01T00:00:00Z")));
    // `account.getQuota` says the time of its reading: the first of next month instead.
    let read = json!({ "chat": { "entitlementRequests": 200, "remainingPercentage": 80,
        "resetDate": "2026-10-18T09:59:59Z" } });
    let m = credits(&read, now).unwrap();
    assert_eq!(m.pct, 20.0);
    assert_eq!(m.resets_at, (at("2026-11-01T00:00:00Z")));
    // `session.quota.get`: in epoch ms; December goes to January.
    let dec = at("2026-12-30T10:00:00Z");
    let m = credits(
        &json!({ "chat": { "entitlementRequests": 200, "remainingPercentage": 0 } }),
        dec,
    )
    .unwrap();
    assert_eq!(m.pct, 100.0);
    assert_eq!(m.resets_at, (at("2027-01-01T00:00:00Z")));
    // A plan without a limit, or nothing told: no meter. Premium requests when chat has none.
    let unlimited = json!({ "chat": { "isUnlimitedEntitlement": true,
        "entitlementRequests": -1, "remainingPercentage": 100 } });
    assert_eq!(credits(&unlimited, now), None);
    assert_eq!(credits(&json!({}), now), None);
    let premium = json!({
        "chat": { "entitlementRequests": 0, "remainingPercentage": 0 },
        "premium_interactions": { "entitlementRequests": 300, "remainingPercentage": 75 },
    });
    assert_eq!(credits(&premium, now).unwrap().pct, 25.0);
}

#[test]
fn the_server_has_no_builtin_mcp_nor_temporary_folder_and_hides_the_tokens() {
    let a = server_args();
    for flag in [
        "--server",
        "--stdio",
        "--no-auto-update",
        "--disable-builtin-mcps",
        "--disallow-temp-dir",
    ] {
        assert!(a.iter().any(|x| x == flag), "{flag}");
    }
    assert!(a
        .iter()
        .any(|x| x.starts_with("--secret-env-vars=") && x.contains("GH_TOKEN")));
    assert!(!a.iter().any(|x| x == "--experimental"), "no assisted mode");
}
