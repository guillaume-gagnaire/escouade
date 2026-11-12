//! What the core tested of OpenCode's catalog before it left it (ESC-359), on the plugin's own.

use crate::rules::*;
use serde_json::{json, Value};

#[test]
fn the_models_come_from_the_connected_providers_with_their_variants() {
    // As OpenCode 1.19.3 answered `/config/providers` (the core's recording `models.jsonl`, its local models).
    let answer = json!({ "body": {
        "providers": [{ "id": "ollama", "name": "Ollama (local)", "models": {
            "gpt-oss:20b-32k": { "name": "gpt-oss 20b (32k)", "variants": { "high": {}, "low": {}, "medium": {} },
                "limit": { "context": 32768 } },
            "qwen3-coder:30b-32k": { "name": "qwen3-coder 30b (32k)", "capabilities": { "input": { "image": false } } },
            "old": { "name": "old", "status": "deprecated" },
        } }],
        "default": { "ollama": "qwen3-coder:30b-32k" },
    } });
    let c = catalog(&answer["body"], &Value::Null);
    let gpt = c
        .models
        .iter()
        .find(|m| m.id == "ollama/gpt-oss:20b-32k")
        .unwrap();
    assert_eq!(gpt.label, "gpt-oss 20b (32k)");
    assert_eq!(gpt.efforts, ["low", "medium", "high"]);
    assert_eq!(gpt.provider.as_deref(), Some("Ollama (local)"));
    let qwen = c
        .models
        .iter()
        .find(|m| m.id == "ollama/qwen3-coder:30b-32k")
        .unwrap();
    assert!(qwen.efforts.is_empty() && !qwen.images);
    assert_eq!(c.models.iter().filter(|m| m.default).count(), 1);
    // The user's own default model, when it is one of them.
    let c = catalog(
        &answer["body"],
        &json!({ "model": "ollama/gpt-oss:20b-32k" }),
    );
    assert!(c.models.iter().find(|m| m.default).unwrap().id == "ollama/gpt-oss:20b-32k");
    assert!(catalog(&json!({ "providers": [] }), &Value::Null)
        .models
        .is_empty());
}

#[test]
fn a_deprecated_model_is_left_out_and_a_context_window_kept() {
    let c = catalog(
        &json!({ "providers": [{ "id": "p", "models": {
            "a": { "limit": { "context": 1000 } }, "b": { "status": "deprecated" },
        } }] }),
        &Value::Null,
    );
    assert_eq!(c.models.len(), 1);
    assert_eq!(c.contexts.get("p/a"), Some(&1000));
}
