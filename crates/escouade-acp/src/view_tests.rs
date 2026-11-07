//! The calls of « Vue Gemini »'s hooks in ACP's form (ESC-275), the view on its own.

use super::*;
use serde_json::Value;

#[test]
fn its_tools_are_put_in_acps_form_with_their_diff() {
    let c = acp_call(
        "write_file",
        &json!({ "file_path": "/r/a.txt", "content": "bonjour\n" }),
        &Value::Null,
    );
    assert_eq!(c["kind"], "edit");
    assert_eq!(c["content"][0]["newText"], "bonjour\n");
    assert!(c["content"][0]["oldText"].is_null());
    let c = acp_call(
        "replace",
        &json!({ "file_path": "/r/a.txt", "old_string": "a", "new_string": "b" }),
        &Value::Null,
    );
    assert_eq!(
        (
            c["content"][0]["oldText"].as_str(),
            c["content"][0]["newText"].as_str()
        ),
        (Some("a"), Some("b"))
    );
    let c = acp_call(
        "run_shell_command",
        &json!({ "command": "npm test" }),
        &Value::Null,
    );
    assert_eq!(
        (c["kind"].as_str(), c["rawInput"]["command"].as_str()),
        (Some("execute"), Some("npm test"))
    );
    let c = acp_call(
        "mcp_escouade_get_board",
        &json!({}),
        &json!({ "server_name": "escouade", "tool_name": "get_board" }),
    );
    assert_eq!(c["rawInput"]["providerIdentifier"], "escouade");
    assert_eq!(c["rawInput"]["toolName"], "get_board");
    let c = acp_call("save_memory", &json!({ "fact": "x" }), &Value::Null);
    assert_eq!(c["rawInput"]["_toolName"], "save_memory");
}

#[test]
fn what_a_tool_answered_is_its_text_or_its_error() {
    assert_eq!(
        response_text(&json!({ "llmContent": "ok", "returnDisplay": "ok", "error": null })),
        ("ok".to_string(), false)
    );
    assert_eq!(
        response_text(&json!({ "llmContent": [{ "text": "a" }, { "text": "b" }] })),
        ("a\nb".to_string(), false)
    );
    assert_eq!(
        response_text(&json!({ "llmContent": "", "error": { "message": "denied" } })),
        ("denied".to_string(), true)
    );
}
