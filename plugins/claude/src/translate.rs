//! Claude Code's frames (stream-json, docs/PROTOCOL.md) translated into the normalized items of the conversation (`Item`),
//! which the core applies to the agent. The state of the agent stays the core's, the plan's and the requests' too: what
//! this keeps between its frames is the stream's own, the blocks of the message under way, the tokens counted so far.
//!
//! The transcript of the Claude Code view has the shape of these frames: it goes through this same translation (`view`).

use crate::localcmd::{self, Part};
use crate::words::{rate_meters, said, strip_tags, tool_result_text};
use escouade_plugin_protocol::engine::{Item, ModelTurn};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

/// What Claude Code says of a sign-in its process could not refresh: refused (`OAuthRefreshDeadError`),
/// or held by another process past its wait (`OAuthRefreshLockTimeoutError`).
const LOST: [&str; 2] = [
    "OAuth session expired and could not be refreshed",
    "Failed to refresh OAuth token: another Claude Code process is refreshing it",
];

/// Whether `text`, the error of a turn, is a sign-in Claude Code could not refresh.
pub fn lost(text: &str) -> bool {
    LOST.iter().any(|l| text.contains(l))
}

struct Block {
    index: u64,
    item_id: String,
    kind: &'static str,
    finalized: bool,
}

#[derive(Debug, Clone, Default)]
struct Counters {
    input: u64,
    output: u64,
    cache: u64,
    cost: f64,
}

/// What the translation keeps of Claude Code's stream between its frames (not saved): one per process.
#[derive(Default)]
pub struct Translator {
    blocks: HashMap<String, Vec<Block>>,
    current_msg: HashMap<String, String>,
    last_usage: HashMap<String, Counters>,
    /// Model of the conversation's latest message (it may be switched mid-process).
    pub main_model: String,
    /// This turn met the usage limit, and when the limit resets.
    pub rate_limited: bool,
    pub limit_resets_at: Option<i64>,
    /// Tasks run in the foreground: their tool rows show them end.
    foreground_tasks: HashSet<String>,
}

impl Translator {
    /// One frame of its `claude` process, or of the view's transcript, in the language `lang`.
    pub fn frame(&mut self, f: &Value, lang: &str, out: &mut Vec<Item>) {
        match f["type"].as_str().unwrap_or("") {
            "system" => self.system(f, lang, out),
            "stream_event" => self.stream(f, out),
            "assistant" => self.assistant(f, out),
            "user" => self.user(f, out),
            "result" => self.result(f, lang, out),
            "control_request" => control_request(f, out),
            "control_cancel_request" => {
                if let Some(rid) = f["request_id"].as_str() {
                    out.push(Item::Answered {
                        request: rid.to_string(),
                        decision: None,
                        answers: None,
                    });
                }
            }
            "rate_limit_event" => {
                let info = &f["rate_limit_info"];
                if info["status"] == "rejected" {
                    self.limit_resets_at = info["resetsAt"].as_i64().map(|s| s * 1000);
                    // A model's own window (`seven_day_opus`, `seven_day_overage_included`: Fable's), not the account's
                    // (ESC-312): it holds that model's tickets alone.
                    let scoped = info["rateLimitType"]
                        .as_str()
                        .is_some_and(|t| t.starts_with("seven_day_"));
                    out.push(Item::Limit { scoped });
                }
                out.push(Item::Quota {
                    meters: rate_meters(info),
                });
                out.push(Item::Overage {
                    on: info["isUsingOverage"].as_bool().unwrap_or(false),
                });
            }
            _ => {}
        }
    }

    fn system(&mut self, f: &Value, lang: &str, out: &mut Vec<Item>) {
        let subtype = f["subtype"].as_str().unwrap_or("");
        // The tasks of the plan (the rows of events below are about the same frames).
        if matches!(
            subtype,
            "task_started"
                | "task_updated"
                | "task_notification"
                | "task_progress"
                | "background_tasks_changed"
        ) {
            out.push(Item::Tasks { frame: f.clone() });
        }
        match subtype {
            "init" => out.push(Item::Conversation {
                session: f["session_id"].as_str().unwrap_or_default().to_string(),
                notice: said(
                    lang,
                    "Nouvelle conversation Claude (contexte vidé)",
                    "New Claude conversation (context cleared)",
                )
                .to_string(),
            }),
            "status" => {
                if let Some(mode) = f["permissionMode"].as_str() {
                    out.push(Item::Mode { mode: mode.into() });
                }
            }
            "compact_boundary" => {
                // Its session's chain starts again at the boundary: the entries before it are no longer in it (a copy
                // forking at one would be refused), and the next one to come is where a copy forks it.
                out.push(Item::Entry { entry: None });
                out.push(Item::Notice {
                    text: said(lang, "Contexte compacté", "Context compacted").to_string(),
                    level: "info".into(),
                });
            }
            "task_started" => {
                if let (Some(id), false) = (f["task_id"].as_str(), f["is_backgrounded"] == true) {
                    self.foreground_tasks.insert(id.to_string());
                }
            }
            // A task ended. Idle, Claude Code only tells this before starting a turn by itself; mid-turn, it also replays
            // the notification to Claude (not shown again).
            "task_notification" => {
                let Some(id) = f["task_id"].as_str() else {
                    return;
                };
                if self.foreground_tasks.remove(id) {
                    return;
                }
                let field = |k: &str| f[k].as_str().unwrap_or_default();
                // Written as Claude Code writes it for Claude, read the same way.
                let text = format!(
                    "<task-notification>\n<task-id>{id}</task-id>\n<tool-use-id>{}</tool-use-id>\n<status>{}</status>\n<summary>{}</summary>\n</task-notification>",
                    field("tool_use_id"),
                    field("status"),
                    field("summary")
                );
                out.push(Item::Event {
                    id: f["uuid"].as_str().map(str::to_string),
                    source: "task".into(),
                    text,
                    from: None,
                });
            }
            "bridge_state" => out.push(Item::Remote {
                state: f["state"].as_str().map(str::to_string),
            }),
            "local_command_output" => {
                let text = f["content"].as_str().unwrap_or("");
                let parts = localcmd::parse(text).unwrap_or_else(|| {
                    let plain = strip_tags(text);
                    let plain = (!plain.is_empty()).then_some(Part::Output(plain));
                    plain.into_iter().collect()
                });
                local_output(f["uuid"].as_str(), &parts, out);
            }
            _ => {}
        }
    }

    fn stream(&mut self, f: &Value, out: &mut Vec<Item>) {
        let ev = &f["event"];
        let parent = f["parent_tool_use_id"].as_str().map(str::to_string);
        let pkey = parent.clone().unwrap_or_default();
        match ev["type"].as_str().unwrap_or("") {
            "message_start" => {
                let Some(mid) = ev["message"]["id"].as_str() else {
                    return;
                };
                self.current_msg.insert(pkey, mid.to_string());
                self.blocks.entry(mid.to_string()).or_default();
                let model = ev["message"]["model"].as_str().unwrap_or_default();
                if parent.is_none() && !model.is_empty() {
                    self.main_model = model.to_string();
                }
                out.push(Item::Message {
                    id: mid.to_string(),
                    model: model.to_string(),
                    usage: ev["message"]["usage"].clone(),
                    main: parent.is_none(),
                });
            }
            "content_block_start" => {
                let Some(mid) = self.current_msg.get(&pkey).cloned() else {
                    return;
                };
                let index = ev["index"].as_u64().unwrap_or(0);
                let cb = &ev["content_block"];
                let (kind, item_id) = match cb["type"].as_str().unwrap_or("") {
                    kind @ ("text" | "thinking") => {
                        let id = format!("{mid}:{index}");
                        out.push(Item::Delta {
                            id: id.clone(),
                            text: String::new(),
                            thinking: kind == "thinking",
                            parent,
                        });
                        (if kind == "text" { "text" } else { "thinking" }, id)
                    }
                    "tool_use" | "server_tool_use" => {
                        let id = cb["id"].as_str().unwrap_or_default().to_string();
                        out.push(Item::Tool {
                            id: id.clone(),
                            name: cb["name"].as_str().unwrap_or_default().to_string(),
                            input: json!({}),
                            parent,
                            unplanned: true,
                        });
                        ("tool", id)
                    }
                    _ => return,
                };
                self.blocks.entry(mid).or_default().push(Block {
                    index,
                    item_id,
                    kind,
                    finalized: false,
                });
            }
            "content_block_delta" => {
                let Some(mid) = self.current_msg.get(&pkey) else {
                    return;
                };
                let index = ev["index"].as_u64().unwrap_or(0);
                let Some(block) = self
                    .blocks
                    .get(mid)
                    .and_then(|b| b.iter().find(|b| b.index == index))
                else {
                    return;
                };
                let d = &ev["delta"];
                let text = match d["type"].as_str().unwrap_or("") {
                    "text_delta" => d["text"].as_str(),
                    "thinking_delta" => d["thinking"].as_str(),
                    _ => None,
                };
                if let Some(text) = text.filter(|t| !t.is_empty()) {
                    out.push(Item::Delta {
                        id: block.item_id.clone(),
                        text: text.to_string(),
                        thinking: block.kind == "thinking",
                        parent,
                    });
                }
            }
            // The message's final output token count (message_start only announces its input).
            "message_delta" => {
                if let (Some(mid), Some(output)) = (
                    self.current_msg.get(&pkey),
                    ev["usage"]["output_tokens"].as_u64(),
                ) {
                    out.push(Item::MessageOutput {
                        id: mid.clone(),
                        output,
                    });
                }
            }
            _ => {}
        }
    }

    fn assistant(&mut self, f: &Value, out: &mut Vec<Item>) {
        let msg = &f["message"];
        let mid = msg["id"].as_str().unwrap_or("").to_string();
        let parent = f["parent_tool_use_id"].as_str().map(str::to_string);
        if let Some(err) = f["error"].as_str() {
            if err == "rate_limit" && parent.is_none() {
                self.rate_limited = true;
            }
            let text = msg["content"][0]["text"].as_str().unwrap_or(err);
            // Its turn says it, and is taken up by itself (`signin`): no error in red before.
            if parent.is_none() && lost(text) {
                return;
            }
            out.push(Item::Notice {
                text: text.to_string(),
                level: "error".into(),
            });
            return;
        }
        // A subagent's message tells the model it runs on, which its call may not name.
        if let (Some(p), Some(model)) = (parent.as_deref(), msg["model"].as_str()) {
            out.push(Item::Subagent {
                call: p.to_string(),
                model: model.to_string(),
            });
        }
        // An entry of its session's main chain (not a subagent's, nor one Claude Code makes up): where a copy made
        // afterwards forks the session.
        if parent.is_none() && f["is_meta"] != true {
            if let Some(uuid) = f["uuid"].as_str() {
                out.push(Item::Entry {
                    entry: Some(uuid.to_string()),
                });
            }
        }
        let Some(content) = msg["content"].as_array() else {
            return;
        };
        for block in content {
            let kind = match block["type"].as_str().unwrap_or("") {
                "text" => "text",
                "thinking" | "redacted_thinking" => "thinking",
                "tool_use" | "server_tool_use" => "tool",
                _ => continue,
            };
            let streamed = self
                .blocks
                .get_mut(&mid)
                .and_then(|v| v.iter_mut().find(|b| !b.finalized && b.kind == kind))
                .map(|b| {
                    b.finalized = true;
                    b.item_id.clone()
                });
            let id = streamed.clone().unwrap_or_else(|| {
                let n = self.blocks.get(&mid).map_or(0, Vec::len);
                match kind {
                    "tool" => block["id"].as_str().unwrap_or_default().to_string(),
                    _ => format!("{mid}:a{n}"),
                }
            });
            if streamed.is_none() {
                self.blocks.entry(mid.clone()).or_default().push(Block {
                    index: u64::MAX,
                    item_id: id.clone(),
                    kind,
                    finalized: true,
                });
            }
            // A block that says nothing and did not stream has a row all the same: no id, or it would be left out.
            let whole =
                |text: &str| (streamed.is_some() || !text.trim().is_empty()).then_some(id.clone());
            out.push(match kind {
                "text" => {
                    let text = block["text"].as_str().unwrap_or_default();
                    Item::Text {
                        text: text.to_string(),
                        parent: parent.clone(),
                        id: whole(text),
                    }
                }
                "thinking" => {
                    let text = block["thinking"].as_str().unwrap_or_default();
                    Item::Thinking {
                        text: text.to_string(),
                        parent: parent.clone(),
                        id: whole(text),
                    }
                }
                _ => Item::Call {
                    id: id.clone(),
                    name: block["name"].as_str().unwrap_or_default().to_string(),
                    input: block["input"].clone(),
                    parent: parent.clone(),
                },
            });
        }
    }

    fn user(&mut self, f: &Value, out: &mut Vec<Item>) {
        if f["isReplay"] == true {
            return replay(f, out);
        }
        let content = &f["message"]["content"];
        if let Some(text) = content.as_str() {
            if let Some(parts) = localcmd::parse(text) {
                local_output(f["uuid"].as_str(), &parts, out);
            }
            return;
        }
        let Some(blocks) = content.as_array() else {
            return;
        };
        if !blocks.iter().any(|b| b["type"] == "tool_result") {
            let text: Vec<&str> = blocks.iter().filter_map(|b| b["text"].as_str()).collect();
            if let Some(parts) = localcmd::parse(&text.join("\n")) {
                local_output(f["uuid"].as_str(), &parts, out);
            }
            return;
        }
        // A tool's result in its main chain is an entry of its session too: the last one of a turn stopped (or failed)
        // before any reply, which a copy must not lose.
        if f["parent_tool_use_id"].is_null() {
            if let Some(uuid) = f["uuid"].as_str() {
                out.push(Item::Entry {
                    entry: Some(uuid.to_string()),
                });
            }
        }
        let tur = &f["tool_use_result"];
        for b in blocks.iter().filter(|b| b["type"] == "tool_result") {
            let Some(id) = b["tool_use_id"].as_str() else {
                continue;
            };
            if let (true, Some(path)) =
                (tur["structuredPatch"].is_array(), tur["filePath"].as_str())
            {
                out.push(Item::Touched { path: path.into() });
            }
            out.push(Item::ToolResult {
                id: id.to_string(),
                output: tool_result_text(&b["content"]),
                error: b["is_error"].as_bool().unwrap_or(false),
                result: tur.clone(),
            });
        }
    }

    fn result(&mut self, f: &Value, lang: &str, out: &mut Vec<Item>) {
        let resets = self.limit_resets_at.take();
        let limited = std::mem::take(&mut self.rate_limited);
        let is_error = f["is_error"].as_bool().unwrap_or(false) || f["subtype"] != "success";
        let (mut tokens, mut cost) = (0u64, 0f64);
        let mut models = Vec::new();
        let mut window = None;
        if let Some(by_model) = f["modelUsage"].as_object() {
            // The conversation's latest model, else the one that read the most (subagents may use others; the figures are
            // cumulative, so an earlier model may still read more).
            let m = &self.main_model;
            let current = by_model
                .iter()
                .find(|(k, _)| {
                    !m.is_empty() && (k.starts_with(m.as_str()) || m.starts_with(k.as_str()))
                })
                .map(|(_, u)| u);
            let main = current.or_else(|| {
                by_model.values().max_by_key(|u| {
                    [
                        "inputTokens",
                        "cacheReadInputTokens",
                        "cacheCreationInputTokens",
                    ]
                    .iter()
                    .filter_map(|k| u[*k].as_u64())
                    .sum::<u64>()
                })
            });
            window = main.and_then(|u| u["contextWindow"].as_u64());
            for (model, u) in by_model {
                let cur = Counters {
                    input: u["inputTokens"].as_u64().unwrap_or(0),
                    output: u["outputTokens"].as_u64().unwrap_or(0),
                    cache: u["cacheReadInputTokens"].as_u64().unwrap_or(0)
                        + u["cacheCreationInputTokens"].as_u64().unwrap_or(0),
                    cost: u["costUSD"].as_f64().unwrap_or(0.0),
                };
                let prev = self.last_usage.get(model).cloned().unwrap_or_default();
                let delta = if cur.cost + 1e-12 < prev.cost || cur.output < prev.output {
                    cur.clone()
                } else {
                    Counters {
                        input: cur.input.saturating_sub(prev.input),
                        output: cur.output.saturating_sub(prev.output),
                        cache: cur.cache.saturating_sub(prev.cache),
                        cost: (cur.cost - prev.cost).max(0.0),
                    }
                };
                self.last_usage.insert(model.clone(), cur);
                let total = delta.input + delta.output + delta.cache;
                if total == 0 && delta.cost == 0.0 {
                    continue;
                }
                tokens += total;
                cost += delta.cost;
                models.push(ModelTurn {
                    model: u["canonicalModel"].as_str().unwrap_or(model).to_string(),
                    input: delta.input,
                    output: delta.output,
                    cache: delta.cache,
                    cost: delta.cost,
                });
            }
        }
        let error = if is_error {
            f["result"].as_str().map(str::to_string).or_else(|| {
                f["errors"].as_array().map(|e| {
                    e.iter()
                        .filter_map(|x| x.as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                })
            })
        } else {
            None
        };
        // A sign-in that could not be refreshed: the core looks at the account's (ESC-162).
        let sign_in = is_error && !limited && error.as_deref().is_some_and(lost);
        // Always with words: the frame's, else the kind of result it is, else a generic one.
        let failure = error
            .clone()
            .filter(|e| !e.trim().is_empty())
            .or_else(|| {
                f["subtype"]
                    .as_str()
                    .filter(|s| !s.is_empty() && *s != "success")
                    .map(str::to_string)
            })
            .unwrap_or_else(|| {
                said(
                    lang,
                    "Claude Code a signalé une erreur",
                    "Claude Code reported an error",
                )
                .to_string()
            });
        self.finalize();
        out.push(Item::Result {
            id: f["uuid"].as_str().map(str::to_string),
            failed: is_error,
            error,
            failure,
            reply: f["result"].as_str().unwrap_or_default().to_string(),
            limited,
            resets,
            sign_in,
            duration: f["duration_ms"].clone(),
            window,
            cost,
            tokens,
            models,
        });
    }

    /// What streamed is over, a message read later starts anew.
    pub fn finalize(&mut self) {
        for blocks in self.blocks.values_mut() {
            for b in blocks.iter_mut() {
                b.finalized = true;
            }
        }
    }
}

/// A user message re-emitted by Claude Code (`--replay-user-messages`): the echo of one sent from the app (same uuid,
/// already listed), or one sent from claude.ai (Remote Control).
fn replay(f: &Value, out: &mut Vec<Item>) {
    let content = &f["message"]["content"];
    let blocks = content.as_array().map(Vec::as_slice).unwrap_or_default();
    let text = match content.as_str() {
        Some(s) => s.to_string(),
        None => blocks
            .iter()
            .filter_map(|b| b["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
    };
    out.push(Item::Replayed {
        id: f["uuid"].as_str().map(str::to_string),
        text,
        origin: f["origin"]["kind"].as_str().map(str::to_string),
        from: f["origin"]["from"].as_str().map(str::to_string),
        synthetic: f["isSynthetic"] == true,
        images: blocks.iter().filter(|b| b["type"] == "image").count() as u32,
        files: blocks
            .iter()
            .filter(|b| b["type"] == "document")
            .map(|b| b["title"].as_str().unwrap_or("document").to_string())
            .collect(),
    });
}

/// What a local command printed (`parts`, as `localcmd` read them), under the command: its output as an output, its
/// error as an error (ESC-154). Given the id of its entry (`id`), one read again is listed once.
fn local_output(id: Option<&str>, parts: &[Part], out: &mut Vec<Item>) {
    for (k, part) in parts.iter().enumerate() {
        let (level, text) = match part {
            Part::Output(t) => ("output", t),
            Part::Error(t) => ("error", t),
            Part::Typed(_) => continue,
        };
        out.push(Item::Output {
            id: id.map(|id| format!("{id}:out{k}")),
            level: level.into(),
            text: text.clone(),
        });
    }
}

/// A request of the CLI: leave for a tool, the only one with a card; those Escouade takes no part in are answered here.
fn control_request(f: &Value, out: &mut Vec<Item>) {
    let Some(rid) = f["request_id"].as_str() else {
        return;
    };
    let req = &f["request"];
    match req["subtype"].as_str().unwrap_or("") {
        "can_use_tool" => {
            let text = |k: &str| req[k].as_str().map(str::to_string);
            out.push(Item::Ask {
                request: rid.to_string(),
                call: req["tool_use_id"].as_str().unwrap_or("").to_string(),
                tool: req["tool_name"].as_str().unwrap_or("").to_string(),
                input: req["input"].clone(),
                title: text("title"),
                description: text("description"),
                reason: text("decision_reason"),
                suggestions: req["permission_suggestions"].clone(),
                no_always: req["suppress_always_allow_rule"].as_bool().unwrap_or(false),
                default_no: req["default_to_no"].as_bool().unwrap_or(false),
                task: text("agent_id"),
            });
        }
        "elicitation" => out.push(Item::Write {
            line: crate::client::respond(rid, json!({ "action": "decline" })),
        }),
        other => out.push(Item::Write {
            line: crate::client::respond_error(rid, &format!("unsupported request: {other}")),
        }),
    }
}
