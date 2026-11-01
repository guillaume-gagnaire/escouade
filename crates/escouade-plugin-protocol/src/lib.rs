//! The protocol Escouade speaks to its plugins (docs/PLUGINS.md §3, schemas/plugin-protocol-1.json): JSON-RPC 2.0,
//! one message a line, on the plugin's input and output. This crate holds what both sides share: the version, the
//! names of the methods every plugin answers, the messages of the handshake, and `serve`, the loop a plugin written
//! in Rust runs (`escouade-plugins`, the program of the plugins the app gives, INT-4).

pub mod engine;
pub mod forge;
pub mod tickets;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

/// The major version of the protocol, which `escouade.api` of a manifest and `protocolVersion` of the handshake name.
pub const PROTOCOL: u64 = 1;

/// The methods Escouade calls on every plugin.
pub mod method {
    /// Comes first: the version of the protocol, who speaks, what is on offer.
    pub const INITIALIZE: &str = "initialize";
    /// Comes last: the plugin answers, then exits.
    pub const SHUTDOWN: &str = "shutdown";
}

/// The codes of a refusal (JSON-RPC's own).
pub mod code {
    pub const PARSE: i64 = -32700;
    pub const METHOD_NOT_FOUND: i64 = -32601;
    /// A refusal of the service that asking again would not change (a ticket's state no transition leads to).
    pub const REFUSED: i64 = -32010;

    /// The code with which a plugin says the HTTP error `status` of its service: `-(32000 + status)` (401 is
    /// -32401, « reconnect the account »).
    pub fn http(status: u16) -> i64 {
        -(32_000 + i64::from(status))
    }

    /// The HTTP status `code` says, when it is one of `http`'s.
    pub fn http_status(code: i64) -> Option<u16> {
        u16::try_from(-code - 32_000)
            .ok()
            .filter(|s| (400..600).contains(s))
    }
}

/// What Escouade says first (`initialize`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hello {
    pub protocol_version: u64,
    pub escouade: Version,
    pub plugin: Named,
    #[serde(default)]
    pub capabilities: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Version {
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Named {
    pub id: String,
    pub version: String,
}

impl Hello {
    /// The handshake Escouade `version` sends to the plugin `id` at `plugin_version`.
    pub fn new(version: &str, id: &str, plugin_version: &str, capabilities: Value) -> Self {
        Self {
            protocol_version: PROTOCOL,
            escouade: Version {
                version: version.into(),
            },
            plugin: Named {
                id: id.into(),
                version: plugin_version.into(),
            },
            capabilities,
        }
    }
}

/// What the plugin answers to `initialize`: the version of the protocol it speaks (Escouade refuses any other) and
/// what it can do.
pub fn initialized(capabilities: Value) -> Value {
    json!({ "protocolVersion": PROTOCOL, "capabilities": capabilities })
}

/// A request or a notification, read from a line.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Request {
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// A refusal: its code and its message.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    pub code: i64,
    pub message: String,
}

impl Refusal {
    pub fn not_found(method: &str) -> Self {
        Self {
            code: code::METHOD_NOT_FOUND,
            message: format!("no method {method}"),
        }
    }
}

/// The answer to a request: its result, or its refusal.
pub fn answer(id: &Value, result: Result<Value, Refusal>) -> String {
    match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err(e) => {
            json!({ "jsonrpc": "2.0", "id": id, "error": { "code": e.code, "message": e.message } })
        }
    }
    .to_string()
}

/// The loop of a plugin: reads a line at a time from `input`, answers the requests on `output`, calls `handler` for
/// each method but the two of the handshake it answers itself (`initialize` with `capabilities`, `shutdown`, after
/// which it returns). A line that is no request is answered with a parse error. Returns when `input` ends or after
/// `shutdown`.
pub fn serve<R: BufRead, W: Write>(
    input: R,
    mut output: W,
    capabilities: Value,
    mut handler: impl FnMut(&str, Value) -> Result<Value, Refusal>,
) -> std::io::Result<()> {
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(request) = serde_json::from_str::<Request>(&line) else {
            let refusal = Refusal {
                code: code::PARSE,
                message: "not a JSON-RPC request".into(),
            };
            writeln!(output, "{}", answer(&Value::Null, Err(refusal)))?;
            output.flush()?;
            continue;
        };
        let result = match request.method.as_str() {
            method::INITIALIZE => Ok(initialized(capabilities.clone())),
            method::SHUTDOWN => Ok(Value::Null),
            other => handler(other, request.params),
        };
        // A notification has no id: nothing is answered.
        if let Some(id) = &request.id {
            writeln!(output, "{}", answer(id, result))?;
            output.flush()?;
        }
        if request.method == method::SHUTDOWN {
            return Ok(());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(lines: &str) -> Vec<Value> {
        let mut out = Vec::new();
        serve(lines.as_bytes(), &mut out, json!({}), |m, p| match m {
            "echo" => Ok(p),
            other => Err(Refusal::not_found(other)),
        })
        .unwrap();
        String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    #[test]
    fn the_handshake_speaks_the_version_of_the_protocol() {
        let out = run(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#);
        assert_eq!(out[0]["result"]["protocolVersion"], PROTOCOL);
        assert_eq!(out[0]["id"], 1);
    }

    #[test]
    fn it_answers_shutdown_then_reads_no_more() {
        let out = run(concat!(
            r#"{"jsonrpc":"2.0","id":1,"method":"shutdown"}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":2,"method":"echo","params":{"a":1}}"#
        ));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["result"], Value::Null);
    }

    #[test]
    fn a_method_unknown_is_refused_and_a_notification_is_not_answered() {
        let out = run(concat!(
            r#"{"jsonrpc":"2.0","id":1,"method":"nope"}"#,
            "\n",
            r#"{"jsonrpc":"2.0","method":"echo","params":{}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":3,"method":"echo","params":{"a":1}}"#
        ));
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["error"]["code"], code::METHOD_NOT_FOUND);
        assert_eq!(out[1]["result"]["a"], 1);
    }

    #[test]
    fn garbage_is_refused_with_a_parse_error() {
        let out = run("not json");
        assert_eq!(out[0]["error"]["code"], code::PARSE);
    }

    #[test]
    fn the_hello_is_written_as_the_protocol_says() {
        let hello = Hello::new("2.2.0", "dev.escouade.minimal", "2.2.0", json!({}));
        let v = serde_json::to_value(hello).unwrap();
        assert_eq!(v["protocolVersion"], 1);
        assert_eq!(v["escouade"]["version"], "2.2.0");
        assert_eq!(v["plugin"]["id"], "dev.escouade.minimal");
    }
}
