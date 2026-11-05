//! The plugin on the lines Escouade sends, in memory. `escouade-plugin test .` plays the real module.

use super::Template;
use escouade_plugin_kit::serve::serve_on;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};

/// What the plugin writes for `messages`, one message a line.
fn play(messages: &[Value]) -> Vec<Value> {
    let input: String = messages.iter().map(|m| format!("{m}\n")).collect();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let (output, written) = tokio::io::duplex(64 * 1024);
        serve_on(Arc::new(Template), input.as_bytes(), output)
            .await
            .unwrap();
        let mut lines = BufReader::new(written).lines();
        let mut read = Vec::new();
        while let Some(line) = lines.next_line().await.unwrap() {
            read.push(serde_json::from_str(&line).unwrap());
        }
        read
    })
}

fn call(method: &str, params: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params })
}

fn decision(command: &str) -> Value {
    let request = json!({ "request": { "kind": "command", "input": { "command": command } } });
    play(&[call("permissions/decide", request)])[0]["result"].clone()
}

#[test]
fn it_speaks_version_1_of_the_protocol() {
    let hello = play(&[call("initialize", json!({ "protocolVersion": 1 }))]);
    assert_eq!(hello[0]["result"]["protocolVersion"], 1);
}

#[test]
fn a_ticket_moved_to_review_is_said_in_the_journal() {
    let fact = json!({
        "jsonrpc": "2.0",
        "method": "events/fact",
        "params": {
            "name": "ticket.column",
            "project": { "name": "Banc" },
            "ticket": { "key": "APP-7" },
            "data": { "from": "doing", "to": "review" },
        },
    });
    let lines = play(&[fact]);
    assert_eq!(lines[0]["method"], "log");
    assert_eq!(
        lines[0]["params"]["message"],
        "APP-7 is ready to test (Banc)"
    );
}

#[test]
fn a_force_push_is_refused_the_rest_left_to_the_user() {
    assert_eq!(decision("git push --force origin main")["decision"], "deny");
    assert_eq!(decision("git push -f")["decision"], "deny");
    assert_eq!(decision("npm test")["decision"], "ask");
}

#[test]
fn an_unknown_method_is_refused() {
    let answer = play(&[call("bench/unknown", Value::Null)]);
    assert_eq!(answer[0]["error"]["code"], -32601);
}
