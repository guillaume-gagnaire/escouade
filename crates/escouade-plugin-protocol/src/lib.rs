#![doc = include_str!("../README.md")]
//!
//! ## Internals
//!
//! The protocol Escouade speaks to its plugins (docs/PLUGINS.md §3, schemas/plugin-protocol-1.json): JSON-RPC 2.0,
//! one message a line, on the plugin's input and output. This crate holds what both sides share: the version, the
//! names of the methods every plugin answers, the messages of the handshake, and `serve`, the loop a plugin written
//! in Rust runs (`escouade-plugins`, the program of the plugins the app gives, INT-4).

pub mod engine;
pub mod forge;
pub mod tickets;
pub mod turn;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::sync::{Arc, Mutex, OnceLock};

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
    /// The language of the interface (`fr`, `en`), in which the plugin writes what the user reads: what a module cannot
    /// get from its environment (docs/PLUGINS.md §8.4.8).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
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
            lang: None,
            capabilities,
        }
    }
}

/// What Escouade said of itself at `initialize`, kept by [`greeted`]: what a process reads from its environment, a module
/// has no other way to get (docs/PLUGINS.md §8.4.8).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Greeting {
    /// The language of the interface, when it was said.
    pub lang: Option<String>,
    /// The version of Escouade.
    pub version: String,
}

impl Greeting {
    fn of(params: &Value) -> Self {
        Self {
            lang: params["lang"].as_str().map(str::to_string),
            version: params["escouade"]["version"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        }
    }
}

static GREETING: OnceLock<Greeting> = OnceLock::new();

/// What Escouade said at `initialize`: none before it. For the language of a plugin of the kit, see its `i18n`.
pub fn greeting() -> Option<&'static Greeting> {
    GREETING.get()
}

/// Keeps what the params of `initialize` say of Escouade (`serve`, the kit's loop). Only the first counts: a process, or
/// an instance, is told once.
pub fn greeted(params: &Value) {
    let _ = GREETING.set(Greeting::of(params));
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
            method::INITIALIZE => {
                greeted(&request.params);
                Ok(initialized(capabilities.clone()))
            }
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

/// The output of a plugin that answers later, or says something without being asked: a line at a time, from any thread.
#[derive(Clone)]
pub struct Output(Arc<Mutex<Box<dyn Write + Send>>>);

impl Output {
    pub fn new(out: impl Write + Send + 'static) -> Self {
        Self(Arc::new(Mutex::new(Box::new(out))))
    }

    fn line(&self, text: &str) {
        let mut out = self.0.lock().unwrap_or_else(|e| e.into_inner());
        // The core is gone: its end closes this process's input, which ends its loop.
        let _ = writeln!(out, "{text}").and_then(|()| out.flush());
    }

    /// Tells the core something, without waiting for an answer.
    pub fn notify(&self, method: &str, params: &impl Serialize) {
        self.line(&json!({ "jsonrpc": "2.0", "method": method, "params": params }).to_string());
    }
}

/// How a request is answered, now or later; dropped, it is not (a notification has nothing to answer).
#[derive(Clone)]
pub struct Reply {
    id: Option<Value>,
    out: Output,
}

impl Reply {
    pub fn send(&self, result: Result<Value, Refusal>) {
        if let Some(id) = &self.id {
            self.out.line(&answer(id, result));
        }
    }
}

/// `serve` for a plugin whose answers may wait (an HTTP client): `handler` answers by its `Reply`, from wherever it wants,
/// and goes back to reading at once, so that a long call does not hold the others.
pub fn serve_later<R: BufRead>(
    input: R,
    output: Output,
    capabilities: Value,
    mut handler: impl FnMut(&str, Value, Reply),
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
            output.line(&answer(&Value::Null, Err(refusal)));
            continue;
        };
        let reply = Reply {
            id: request.id.clone(),
            out: output.clone(),
        };
        match request.method.as_str() {
            method::INITIALIZE => {
                greeted(&request.params);
                reply.send(Ok(initialized(capabilities.clone())));
            }
            method::SHUTDOWN => {
                reply.send(Ok(Value::Null));
                return Ok(());
            }
            other => handler(other, request.params, reply),
        }
    }
    Ok(())
}

/// The JSON value of a harness's line. A harness written in JavaScript escapes a lone half of a
/// UTF-16 pair (`\ud83c`), which JSON allows and serde refuses: the output of a tool that read a
/// binary file was lost whole (ESC-290). Such a half is read as `�` (�).
pub fn json_line(line: &str) -> serde_json::Result<serde_json::Value> {
    match serde_json::from_str(line) {
        Err(e) if line.contains("\\ud") || line.contains("\\uD") => {
            serde_json::from_str(&lone_halves_replaced(line)).map_err(|_| e)
        }
        other => other,
    }
}

/// `json` with each escaped half of a UTF-16 pair that has no other half written `�`.
fn lone_halves_replaced(json: &str) -> String {
    let half = |s: &str| -> Option<u16> {
        let hex = s.strip_prefix("\\u").or_else(|| s.strip_prefix("\\U"))?;
        u16::from_str_radix(hex.get(..4)?, 16)
            .ok()
            .filter(|c| (0xd800..=0xdfff).contains(c))
    };
    let mut out = String::with_capacity(json.len());
    let mut i = 0;
    while i < json.len() {
        let rest = &json[i..];
        if !rest.starts_with('\\') {
            let c = rest.chars().next().unwrap_or_default();
            out.push(c);
            i += c.len_utf8().max(1);
            continue;
        }
        match half(rest) {
            Some(0xd800..=0xdbff)
                if half(&rest[6..]).is_some_and(|low| (0xdc00..=0xdfff).contains(&low)) =>
            {
                out.push_str(&rest[..12]);
                i += 12;
            }
            Some(_) => {
                out.push_str("\\ufffd");
                i += 6;
            }
            // Any other escape, kept whole: `\\u…` is a backslash, then text.
            None => {
                let next = rest[1..].chars().next().map_or(0, char::len_utf8);
                out.push_str(&rest[..1 + next]);
                i += 1 + next;
            }
        }
    }
    out
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
        assert!(v.get("lang").is_none(), "{v}");
    }

    #[test]
    fn a_language_comes_with_the_hello_and_an_older_hello_still_reads() {
        let mut hello = Hello::new("2.2.0", "dev.escouade.minimal", "2.2.0", json!({}));
        hello.lang = Some("en".into());
        let v = serde_json::to_value(&hello).unwrap();
        assert_eq!(v["lang"], "en");
        assert_eq!(serde_json::from_value::<Hello>(v).unwrap(), hello);
        hello.lang = None;
        let v = serde_json::to_value(&hello).unwrap();
        assert_eq!(serde_json::from_value::<Hello>(v).unwrap(), hello);
    }

    #[test]
    fn what_escouade_says_of_itself_is_read_from_the_params_of_initialize() {
        let params = json!({ "lang": "en", "escouade": { "version": "2.2.0" } });
        let said = Greeting::of(&params);
        assert_eq!(
            (said.lang.as_deref(), said.version.as_str()),
            (Some("en"), "2.2.0")
        );
        assert_eq!(Greeting::of(&json!({})), Greeting::default());
    }
}
