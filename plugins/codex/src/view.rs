//! « Vue Codex » (`view/translate`, docs/PLUGINS.md §8): what Codex's own interface writes in its session's rollout
//! (`sessions/…/rollout-<date>-<session>.jsonl`), read by the core a line at a time and translated here.
//!
//! The rollout's events (`event_msg`) tell what the app-server's notifications tell, in Codex's inner form (snake_case, items
//! in PascalCase): each is turned into the notification it stands for and goes through the same translation as a process's
//! (`translate`). The rest of the rollout (the model's own `response_item`s, the turns' contexts) says nothing more. What the
//! user types in the interface is told as typed (`Item::Typed`).

use crate::client::Own;
use crate::translate::{Ctx, Seen, Translator};
use escouade_plugin_protocol::engine::{Item, ViewParams};
use serde_json::{json, Value};

/// A `snake_case` key in `camelCase`.
fn camel(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    let mut up = false;
    for c in key.chars() {
        if c == '_' {
            up = true;
        } else if up {
            out.extend(c.to_uppercase());
            up = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// An object whose keys are turned into `camelCase` (its own only, not those of its values).
fn camel_keys(v: &Value) -> Value {
    match v.as_object() {
        Some(map) => Value::Object(map.iter().map(|(k, v)| (camel(k), v.clone())).collect()),
        None => v.clone(),
    }
}

/// An item of the rollout (`UserMessage`, `AgentMessage`, `CommandExecution`…) as the app-server
/// sends it (`userMessage`, `agentMessage`, `commandExecution`…).
pub fn rollout_item(item: &Value) -> Value {
    let mut out = camel_keys(item);
    let kind = item["type"].as_str().unwrap_or_default();
    let mut chars = kind.chars();
    let kind = match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect::<String>(),
        None => String::new(),
    };
    out["type"] = json!(kind);
    let texts = |v: &Value| -> String {
        v.as_array()
            .into_iter()
            .flatten()
            .filter_map(|c| c["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n")
    };
    match kind.as_str() {
        "agentMessage" | "plan" if out["text"].is_null() => {
            out["text"] = json!(texts(&item["content"]));
        }
        "reasoning" => {
            if out["summary"].is_null() {
                out["summary"] = item["summary_text"].clone();
            }
            if out["content"].is_null() {
                out["content"] = item["raw_content"].clone();
            }
        }
        "commandExecution" => {
            // The command as it was run (its shell and all), and as the model wrote it.
            if let Some(argv) = item["command"].as_array() {
                let words: Vec<&str> = argv.iter().filter_map(Value::as_str).collect();
                out["command"] = json!(words.join(" "));
            }
            let actions: Vec<Value> = item["parsed_cmd"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|a| a["cmd"].as_str())
                .map(|c| json!({ "command": c }))
                .collect();
            out["commandActions"] = json!(actions);
        }
        "fileChange" => {
            // By path in the rollout (`{ path: { type, unified_diff | content, move_path } }`), a
            // list in the app-server's.
            if let Some(by_path) = item["changes"].as_object() {
                let changes: Vec<Value> = by_path
                    .iter()
                    .map(|(path, c)| {
                        let diff = c["unified_diff"]
                            .as_str()
                            .or_else(|| c["content"].as_str())
                            .unwrap_or_default();
                        json!({
                            "path": path, "diff": diff,
                            "kind": { "type": c["type"], "move_path": c["move_path"] },
                        })
                    })
                    .collect();
                out["changes"] = json!(changes);
            }
            if out["status"].is_null() {
                out["status"] = json!("completed");
            }
        }
        _ => {}
    }
    if let Some(status) = out["status"].as_str() {
        out["status"] = json!(camel(&status.to_lowercase()));
    }
    out
}

/// The rate limits of a rollout's `token_count`, as `account/rateLimits/updated` gives them.
fn rate_limits(r: &Value) -> Value {
    let window = |w: &Value| {
        if !w.is_object() {
            return Value::Null;
        }
        json!({
            "usedPercent": w["used_percent"],
            "windowDurationMins": w["window_minutes"],
            "resetsAt": w["resets_at"],
        })
    };
    json!({
        "primary": window(&r["primary"]),
        "secondary": window(&r["secondary"]),
        "rateLimitReachedType": r["rate_limit_reached_type"],
    })
}

fn notification(method: &str, params: Value) -> Value {
    json!({ "method": method, "params": params })
}

/// The view of one agent: the translation of its session, and what it told of its conversation.
#[derive(Default)]
pub struct View {
    told: Translator,
    own: Own,
}

impl View {
    /// What the core's conversation holds, for a view that starts on it.
    pub fn knows(&mut self, p: &ViewParams) {
        self.own.knows(&p.known);
    }

    /// One line of the rollout, or a note of the core.
    pub fn line(&mut self, line: &str, p: &ViewParams, out: &mut Vec<Item>) {
        let Ok(e) = serde_json::from_str::<Value>(line) else {
            return;
        };
        match e["escouade"].as_str() {
            Some("frame") => self.frame(&e["frame"], p, out),
            Some("end") => {
                // Told as Codex tells a turn's end.
                let status = if p.interrupted {
                    "interrupted"
                } else {
                    "completed"
                };
                let turn = json!({ "id": self.told.turn, "status": status });
                self.frame(
                    &json!({ "method": "turn/completed", "params": { "turn": turn } }),
                    p,
                    out,
                );
            }
            _ => self.entry(&e, p, out),
        }
        out.push(Item::ViewTurn {
            id: self.told.turn.clone(),
        });
    }

    fn frame(&mut self, f: &Value, p: &ViewParams, out: &mut Vec<Item>) {
        let from = out.len();
        let c = Ctx {
            cwd: &p.cwd,
            mode: &p.mode,
            model: &p.model,
            active: p.active,
            interrupted: p.interrupted,
            fenced: false,
            process: false,
            lang: &p.lang,
            session: p.session.as_deref(),
            copy: p.copy,
            seen: &self.own,
        };
        self.told.frame(f, &c, out);
        self.own.note(&out[from..]);
    }

    fn entry(&mut self, e: &Value, p: &ViewParams, out: &mut Vec<Item>) {
        if e["type"] != "event_msg" {
            return;
        }
        let d = &e["payload"];
        let own = p.session.clone();
        let frame = match d["type"].as_str().unwrap_or_default() {
            "task_started" => notification(
                "turn/started",
                json!({ "threadId": own, "turn": { "id": d["turn_id"] } }),
            ),
            "task_complete" | "turn_aborted" => {
                // Ended already, by the core, after a `Stop` whose end came late.
                if !p.active {
                    return;
                }
                // That of an earlier turn, the core ended, read once the next one started.
                if let (Some(running), Some(ended)) = (&self.told.turn, d["turn_id"].as_str()) {
                    if running != ended {
                        return;
                    }
                }
                let status = if d["type"] == "turn_aborted" {
                    "interrupted"
                } else {
                    "completed"
                };
                let turn =
                    json!({ "id": d["turn_id"], "status": status, "durationMs": d["duration_ms"] });
                notification("turn/completed", json!({ "threadId": own, "turn": turn }))
            }
            kind @ ("item_started" | "item_completed") => {
                let item = &d["item"];
                if item["type"] == "UserMessage" {
                    if kind == "item_completed" {
                        typed(item, out);
                    }
                    return;
                }
                // What a turn already over tells late (its interrupted command) is left out.
                let id = item["id"].as_str().unwrap_or_default();
                if self.told.turn.is_none() && !self.own.has(id) {
                    return;
                }
                let method = if kind == "item_started" {
                    "item/started"
                } else {
                    "item/completed"
                };
                notification(
                    method,
                    json!({ "threadId": d["thread_id"], "turnId": d["turn_id"], "item": rollout_item(item) }),
                )
            }
            "token_count" => {
                if d["rate_limits"].is_object() {
                    let limits = rate_limits(&d["rate_limits"]);
                    self.frame(
                        &notification(
                            "account/rateLimits/updated",
                            json!({ "rateLimits": limits }),
                        ),
                        p,
                        out,
                    );
                }
                let info = &d["info"];
                if !info.is_object() {
                    return;
                }
                let usage = json!({
                    "total": camel_keys(&info["total_token_usage"]),
                    "last": camel_keys(&info["last_token_usage"]),
                    "modelContextWindow": info["model_context_window"],
                });
                notification(
                    "thread/tokenUsage/updated",
                    json!({ "threadId": own, "tokenUsage": usage }),
                )
            }
            "error" => notification(
                "error",
                json!({ "threadId": own, "error": { "message": d["message"] } }),
            ),
            _ => return,
        };
        self.frame(&frame, p, out);
    }
}

/// What the user typed in the interface (or Escouade pasted there): their message.
fn typed(item: &Value, out: &mut Vec<Item>) {
    let content = item["content"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let text = content
        .iter()
        .filter_map(|c| c["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let images = content
        .iter()
        .filter(|c| {
            matches!(
                c["type"].as_str(),
                Some("image" | "local_image" | "localImage")
            )
        })
        .count() as u32;
    if text.trim().is_empty() && images == 0 {
        return;
    }
    out.push(Item::Typed {
        id: item["id"].as_str().map(str::to_string),
        text,
        images,
        prompt: None,
    });
}
