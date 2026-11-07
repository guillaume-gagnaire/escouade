//! « Vue Claude Code » (`view/translate`, docs/PLUGINS.md §8): what Claude Code's own interface (its TUI, in a terminal)
//! writes in its transcript, read by the core a line at a time and translated here.
//!
//! The transcript's `assistant` and `user` entries have the shape of the stream's frames: they go through the same
//! translation. What the stream alone tells is made up from the rest: a turn ends with its `system`/`turn_duration` entry
//! (or the marker of an interruption), and its cost, worked out by the core from the usage of its API messages at list
//! prices, comes back with the end the core asks for (the interface reporting none per turn).

use crate::localcmd::{self, Part};
use crate::translate::Translator;
use crate::words::strip_tags;
use escouade_plugin_protocol::engine::{Item, ViewParams};
use serde_json::{json, Value};

/// The texts Claude Code writes as the user's when a turn is interrupted.
const INTERRUPTED: &[&str] = &[
    "[Request interrupted by user",
    "[Request interrupted by user for tool use",
];

/// The view of one agent: the translation of its session.
#[derive(Default)]
pub struct View {
    told: Translator,
}

impl View {
    /// The model the agent ran on, for a view that goes on after it.
    pub fn knows(&mut self, p: &ViewParams) {
        self.told.main_model = p.main_model.clone();
    }

    /// One entry of the transcript, or a note of the core.
    pub fn line(&mut self, line: &str, p: &ViewParams, out: &mut Vec<Item>) {
        let Ok(e) = serde_json::from_str::<Value>(line) else {
            return;
        };
        match e["escouade"].as_str() {
            Some("frame") => self.told.frame(&e["frame"], &p.lang, out),
            Some("end") => {
                let error = e["error"].as_str();
                let result = json!({
                    "type": "result",
                    "subtype": if error.is_some() { "error_during_execution" } else { "success" },
                    "is_error": error.is_some(),
                    "result": error,
                    "duration_ms": e["durationMs"],
                    "modelUsage": e["models"],
                });
                self.told.frame(&result, &p.lang, out);
            }
            _ => self.entry(&e, p, out),
        }
    }

    fn entry(&mut self, e: &Value, p: &ViewParams, out: &mut Vec<Item>) {
        // A subagent's entries are in files of their own; one here would be told as the agent's.
        if e["isSidechain"] == true {
            return;
        }
        // Something of its session is written: its absence now would be a loss.
        if matches!(e["type"].as_str(), Some("user" | "assistant")) {
            out.push(Item::Wrote);
        }
        match e["type"].as_str().unwrap_or("") {
            "user" => self.user(e, p, out),
            "assistant" => self.assistant(e, p, out),
            "system" => match e["subtype"].as_str().unwrap_or("") {
                "turn_duration" => out.push(Item::ViewEnd {
                    interrupted: false,
                    duration: e["durationMs"].as_u64(),
                    error: None,
                }),
                "compact_boundary" => self.told.frame(e, &p.lang, out),
                // A local command's output, failed ones' (2.1.302: `/compact` that fails).
                "local_command" => {
                    let failed = e["commandOutcome"]["kind"] == "failed";
                    let text = e["content"].as_str().unwrap_or("");
                    local_command(e["uuid"].as_str(), text, failed, p, out);
                }
                // Started with Remote Control, the interface tells the link of its session.
                "bridge_status" => {
                    out.push(Item::Bridged {
                        url: e["url"].as_str().map(str::to_string),
                        said: e["content"].as_str().unwrap_or_default().to_string(),
                    });
                }
                _ => {}
            },
            "bridge-session" => {
                out.push(Item::Spoke);
                if e["bridgeSessionId"].as_str().is_some_and(|s| !s.is_empty()) {
                    out.push(Item::Linked);
                }
            }
            // « Shift+Tab » in the interface: the agent's mode is the one it shows.
            "permission-mode" => {
                if let Some(mode) = e["permissionMode"].as_str() {
                    out.push(Item::Mode {
                        mode: mode.to_string(),
                    });
                }
            }
            _ => {}
        }
    }

    fn user(&mut self, e: &Value, p: &ViewParams, out: &mut Vec<Item>) {
        // Claude Code's own (the caveat of a local command, a compaction's summary).
        if e["isMeta"] == true || e["isCompactSummary"] == true {
            return;
        }
        let content = &e["message"]["content"];
        let blocks = content.as_array().map(Vec::as_slice).unwrap_or_default();
        if blocks.iter().any(|b| b["type"] == "tool_result") {
            let frame = json!({
                "type": "user", "message": e["message"], "uuid": e["uuid"],
                "parent_tool_use_id": null, "tool_use_result": e["toolUseResult"],
            });
            return self.told.frame(&frame, &p.lang, out);
        }
        let text = match content.as_str() {
            Some(s) => s.to_string(),
            None => blocks
                .iter()
                .filter_map(|b| b["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        };
        if INTERRUPTED.iter().any(|m| text.starts_with(m)) {
            if p.active {
                out.push(Item::ViewEnd {
                    interrupted: true,
                    duration: None,
                    error: None,
                });
            }
            return;
        }
        // A local command (`/model opus`, `! ls`): shown as typed, then what it printed.
        let text = match localcmd::parse(&text) {
            Some(parts) => match parts.into_iter().next() {
                Some(Part::Typed(typed)) => typed,
                // Its caveat, Claude Code's own.
                None if text.contains("<local-command-caveat>") => return,
                _ => return local_command(e["uuid"].as_str(), &text, false, p, out),
            },
            None => text,
        };
        let kind = e["origin"]["kind"].as_str();
        if kind.is_some_and(|k| k != "human") {
            // A subagent's report, a background task that ended…: as the stream tells them.
            let frame = json!({
                "type": "user", "isReplay": true, "uuid": e["uuid"], "message": e["message"],
                "origin": e["origin"],
            });
            return self.told.frame(&frame, &p.lang, out);
        }
        let images = blocks.iter().filter(|b| b["type"] == "image").count() as u32;
        if text.trim().is_empty() && images == 0 {
            return;
        }
        let text = if text.trim().is_empty() {
            String::new()
        } else {
            text
        };
        out.push(Item::Typed {
            id: e["uuid"].as_str().map(str::to_string),
            text,
            images,
            // The same prompt again: a command, as typed then as `<command-name>`.
            prompt: Some(e["promptId"].as_str().unwrap_or_default().to_string()),
        });
    }

    fn assistant(&mut self, e: &Value, p: &ViewParams, out: &mut Vec<Item>) {
        let msg = &e["message"];
        if let (Some(mid), Some(model)) = (msg["id"].as_str(), msg["model"].as_str()) {
            // Each block of a message is an entry of its own, with the message's usage.
            if model != "<synthetic>" {
                self.told.main_model = model.to_string();
                out.push(Item::ViewUsage {
                    id: mid.to_string(),
                    model: model.to_string(),
                    usage: msg["usage"].clone(),
                });
            }
        }
        let mut frame = json!({
            "type": "assistant", "message": msg, "uuid": e["uuid"], "parent_tool_use_id": null,
        });
        if e["isApiErrorMessage"] == true {
            let error = e["error"].as_str().unwrap_or("api_error");
            frame["error"] = json!(error);
            if error == "rate_limit" {
                self.told.rate_limited = true;
            }
            let text = msg["content"][0]["text"].as_str().unwrap_or(error);
            out.push(Item::ViewError {
                error: text.to_string(),
            });
        }
        out.push(Item::Runs);
        self.told.frame(&frame, &p.lang, out);
    }
}

/// What a local command printed (`text`, its `<local-command-stdout>` or `-stderr>`, the `!` mode's `<bash-stdout>`…, in the
/// entry `id`): shown, and its turn over, as an error when it failed (`failed`, or its error output), the error then on the
/// turn's card only. Claude Code ends such a turn with no `turn_duration` (2.1.302, ESC-12).
fn local_command(id: Option<&str>, text: &str, failed: bool, p: &ViewParams, out: &mut Vec<Item>) {
    let mut parts = localcmd::parse(text).unwrap_or_else(|| {
        let plain = strip_tags(text);
        let plain = (!plain.is_empty()).then_some(Part::Output(plain));
        plain.into_iter().collect()
    });
    // Only the command's own turn: one typed while the model runs a turn leaves that turn be.
    let commands = p
        .prompt
        .as_deref()
        .is_some_and(|t| t.starts_with('/') || t.starts_with("! "));
    let ends = p.active && commands;
    let mut error = None;
    if ends {
        let wrong = |p: &Part| match p {
            Part::Error(t) => Some(t.clone()),
            _ => None,
        };
        let errors: Vec<String> = parts.iter().filter_map(wrong).collect();
        if !errors.is_empty() {
            error = Some(errors.join("\n"));
            parts.retain(|p| wrong(p).is_none());
        } else if failed {
            error = Some(strip_tags(text));
        }
    }
    for (k, part) in parts.iter().enumerate() {
        let (level, text) = match part {
            Part::Output(t) => ("output", t),
            Part::Error(t) => ("error", t),
            Part::Typed(_) => continue,
        };
        out.push(Item::Output {
            id: id.map(|id| format!("{id}:out{k}")),
            level: level.to_string(),
            text: text.clone(),
        });
    }
    if ends {
        out.push(Item::ViewEnd {
            interrupted: false,
            duration: None,
            error,
        });
    }
}
