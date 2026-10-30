//! The index's tools for the real agents of the bench (ESC-299): a stdio MCP server of its own,
//! over the index of a copy of the repository, never the app's. It serves the app's six tools as
//! the app's server does (`crate::mcp::index`, the same schemas from `mcp::tools`), their answers
//! and their metrics line counted in one session (`accounting`), and notes each call in a log of
//! JSON lines: the tool, its arguments, the answer's size and how long it took.
//!
//! The ESC-253 hook comes with it: `hook` reads Claude Code's `PreToolUse` on its input and writes
//! what the app's server answers (`hint::reply`), never a refusal.

use super::super::accounting::{Call, Ledger};
use super::super::{format, hint, Service, Target};
use crate::i18n::Lang;
use crate::mcp::index::{self, Text};
use crate::mcp::tools::{ToolError, Tools};
use crate::model::EngineId;
use crate::paths::DataDir;
use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// The language of the answers, the rule and the hook: the user's, as in the app.
pub const LANG: Lang = Lang::Fr;
const PROJECT: &str = "agents";

/// The index of `dir` (in `data`), as the server reads it.
pub fn target(dir: &Path) -> Target {
    Target {
        project_id: PROJECT.into(),
        root: dir.to_path_buf(),
        dir: dir.to_path_buf(),
        fence: Default::default(),
        exclude: Vec::new(),
    }
}

/// The server: one JSON-RPC message per line on `input`, the answers on `out`, until `input`
/// ends.
pub struct Server {
    svc: Service,
    target: Target,
    log: Option<PathBuf>,
    ledger: Ledger,
    tools: Value,
    schema_tokens: u64,
}

impl Server {
    pub fn new(dir: &Path, data: &Path, log: Option<PathBuf>) -> Self {
        let tools = crate::mcp::tools::index_tools(&Tools::<tauri::Wry>::router());
        let schema_tokens = super::super::accounting::schema_overhead_tokens(&tools);
        Self {
            svc: Service::new(DataDir::new(data.to_path_buf())),
            target: target(dir),
            log,
            ledger: Ledger::default(),
            tools,
            schema_tokens,
        }
    }

    pub fn serve(&self, input: impl BufRead, mut out: impl Write) -> Result<()> {
        for line in input.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let Ok(msg) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if let Some(answer) = self.answer(&msg) {
                writeln!(out, "{answer}")?;
                out.flush()?;
            }
        }
        Ok(())
    }

    /// The answer to a message; none to a notification.
    pub fn answer(&self, msg: &Value) -> Option<Value> {
        let id = msg.get("id")?.clone();
        let method = msg["method"].as_str().unwrap_or_default();
        let result = match method {
            "initialize" => json!({
                "protocolVersion": msg["params"]["protocolVersion"].as_str().unwrap_or("2025-06-18"),
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "escouade-bench", "version": env!("CARGO_PKG_VERSION") },
            }),
            "ping" => json!({}),
            "tools/list" => json!({ "tools": self.tools }),
            "tools/call" => self.call(&msg["params"]),
            _ => {
                return Some(json!({ "jsonrpc": "2.0", "id": id,
                    "error": { "code": -32601, "message": format!("unknown method {method}") } }))
            }
        };
        Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
    }

    fn call(&self, params: &Value) -> Value {
        let tool = params["name"].as_str().unwrap_or_default();
        let arguments = params
            .get("arguments")
            .cloned()
            .filter(Value::is_object)
            .unwrap_or_else(|| json!({}));
        let t = Instant::now();
        let text = self.text(tool, arguments.clone());
        let ms = t.elapsed().as_millis() as u64;
        let (content, is_error, tokens) = match text {
            Ok((text, files_tokens)) => {
                let m = self.ledger.account(
                    "",
                    &Call {
                        tool,
                        arguments: &arguments,
                        response: &text,
                        full_file_tokens: files_tokens,
                        schema_tokens: self.schema_tokens,
                    },
                    |m| format::metrics_line(LANG, m),
                );
                let mut blocks = vec![json!({ "type": "text", "text": text })];
                if m.before > 0 {
                    blocks.push(json!({ "type": "text", "text": format::metrics_line(LANG, &m) }));
                }
                let tokens = super::super::tokens_of(
                    blocks
                        .iter()
                        .map(|b| b["text"].as_str().unwrap_or_default().len() as u64)
                        .sum(),
                );
                (blocks, false, tokens)
            }
            Err(ToolError::Failed(m) | ToolError::Refused(m)) => {
                let tokens = super::super::tokens_of(m.len() as u64);
                (vec![json!({ "type": "text", "text": m })], true, tokens)
            }
        };
        self.note(json!({ "tool": tool, "arguments": arguments, "ms": ms,
            "tokens": tokens, "error": is_error }));
        json!({ "content": content, "isError": is_error })
    }

    fn text(&self, tool: &str, arguments: Value) -> Result<Text, ToolError> {
        fn args<T: serde::de::DeserializeOwned>(v: Value) -> Result<T, ToolError> {
            serde_json::from_value(v).map_err(|e| ToolError::Failed(e.to_string()))
        }
        let r = self
            .svc
            .reader(&self.target)
            .map_err(|e| ToolError::Failed(e.to_string()))?;
        match tool {
            "index_search" => index::search_on(r, args(arguments)?, LANG),
            "index_context" => index::context_on(r, args(arguments)?, LANG),
            "index_body" => index::body_on(r, args(arguments)?, LANG),
            "index_callers" => index::callers_on(r, args(arguments)?),
            "index_callees" => index::callees_on(r, args(arguments)?),
            "index_impact" => index::impact_on(r, args(arguments)?),
            _ => Err(ToolError::Failed(format!("unknown tool {tool}"))),
        }
    }

    fn note(&self, line: Value) {
        let Some(log) = &self.log else { return };
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log)
        {
            let _ = writeln!(f, "{line}");
        }
    }
}

/// The ESC-253 hook: Claude Code's event on `input`, the app's answer on `out` (nothing for a
/// search of a text). Each event is noted in `log`, with whether it was answered.
pub fn hook(input: &str, log: Option<&Path>) -> Option<Value> {
    let event: Value = serde_json::from_str(input).ok()?;
    let reply = hint::reply(EngineId::Claude, LANG, &event);
    if let Some(log) = log {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log)
        {
            let pattern = hint::pattern_of(&event).unwrap_or_default();
            let _ = writeln!(
                f,
                "{}",
                json!({ "tool": event["tool_name"], "pattern": pattern, "answered": reply.is_some() })
            );
        }
    }
    reply
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rpc(id: u64, method: &str, params: Value) -> String {
        json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string()
    }

    #[test]
    fn the_bench_server_answers_the_index_tools_over_stdio_on_its_own_copy() {
        let base = crate::paths::test_dir("agents-mcp");
        let dir = base.join("repo");
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("src").join("shop.rs"),
            "/// Adds an item to the cart.\npub fn add_to_cart(cart: &mut Vec<u32>, item: u32) {\n    cart.push(item);\n    log_change(cart.len());\n}\n\nfn log_change(n: usize) {\n    println!(\"{n}\");\n}\n",
        )
        .unwrap();
        let log = base.join("calls.jsonl");
        let server = Server::new(&dir, &base.join("data"), Some(log.clone()));
        let input = [
            rpc(1, "initialize", json!({ "protocolVersion": "2025-06-18" })),
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }).to_string(),
            rpc(2, "tools/list", json!({})),
            rpc(
                3,
                "tools/call",
                json!({ "name": "index_search", "arguments": { "query": "add_to_cart" } }),
            ),
            rpc(
                4,
                "tools/call",
                json!({ "name": "index_callers", "arguments": { "name": "log_change" } }),
            ),
            rpc(
                5,
                "tools/call",
                json!({ "name": "index_body", "arguments": {} }),
            ),
            rpc(6, "resources/list", json!({})),
        ]
        .join("\n");
        let mut out = Vec::new();
        server.serve(input.as_bytes(), &mut out).unwrap();
        let answers: Vec<Value> = String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        // The notification gets no answer.
        assert_eq!(answers.len(), 6);
        assert_eq!(answers[0]["result"]["protocolVersion"], "2025-06-18");
        let names: Vec<&str> = answers[1]["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        for n in [
            "index_search",
            "index_context",
            "index_body",
            "index_callers",
            "index_callees",
            "index_impact",
        ] {
            assert!(names.contains(&n), "{n} in {names:?}");
        }
        assert_eq!(names.len(), 6);
        // The app's answers: the symbol's line, then the metrics line.
        let search = &answers[2]["result"];
        assert_eq!(search["isError"], false);
        let text = search["content"][0]["text"].as_str().unwrap();
        assert!(
            text.contains("src/shop.rs") && text.contains("add_to_cart"),
            "{text}"
        );
        assert!(search["content"][1]["text"]
            .as_str()
            .unwrap()
            .contains("index_metrics"));
        let callers = answers[3]["result"]["content"][0]["text"].as_str().unwrap();
        assert!(callers.contains("add_to_cart"), "{callers}");
        // A call that names nothing says why, as an error.
        assert_eq!(answers[4]["result"]["isError"], true);
        assert_eq!(answers[5]["error"]["code"], -32601);
        let logged: Vec<Value> = std::fs::read_to_string(&log)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(logged.len(), 3);
        assert_eq!(logged[0]["tool"], "index_search");
        assert!(logged[0]["tokens"].as_u64().unwrap() > 0);
        assert_eq!(logged[2]["error"], true);
    }

    #[test]
    fn the_hook_answers_a_search_for_a_symbol_and_lets_a_text_by() {
        let base = crate::paths::test_dir("agents-hook");
        std::fs::create_dir_all(&base).unwrap();
        let log = base.join("hook.jsonl");
        let grep = json!({ "hook_event_name": "PreToolUse", "tool_name": "Grep",
            "tool_input": { "pattern": "plan_resume" } });
        let r = hook(&grep.to_string(), Some(&log)).unwrap();
        assert!(r["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .unwrap()
            .contains("index_search"));
        let text = json!({ "hook_event_name": "PreToolUse", "tool_name": "Grep",
            "tool_input": { "pattern": "failed to connect" } });
        assert!(hook(&text.to_string(), Some(&log)).is_none());
        let logged: Vec<Value> = std::fs::read_to_string(&log)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(logged.len(), 2);
        assert_eq!(
            (&logged[0]["answered"], &logged[1]["answered"]),
            (&json!(true), &json!(false))
        );
    }
}
