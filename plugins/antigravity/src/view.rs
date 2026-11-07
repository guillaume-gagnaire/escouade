//! « Vue Antigravity » (`view/translate`, docs/PLUGINS.md §8): what Antigravity CLI's own interface (`agy`, in a terminal)
//! writes in its transcript (`brain/<id>/.system_generated/logs/transcript_full.jsonl` of the agent's home), read by the core a
//! line at a time and translated here.
//!
//! A line of the transcript is a step once written, tried on 1.4.9 (`tests/fixtures/agy/tui-transcript-*.jsonl`): `USER_INPUT`
//! (the user's message, in `<USER_REQUEST>`), `PLANNER_RESPONSE` (the model's text `content`, its calls `tool_calls`, its
//! reasoning `thinking`), `GENERIC` (the run of a call: the steps after its answer, one a call, `DONE` or `ERROR`),
//! `SYSTEM_MESSAGE` (not shown). Its calls are put in ACP's form, as those of its `-p` stream, named by the step of their run.
//! An answer without a call ends the turn: the last one, or the text it had written when Escape stopped it (no hook then).

use crate::translate::{output, Translator};
use crate::words::{acp_call, call_id, diff_of, path_of};
use escouade_acp::view::{decl, Speaker};
use escouade_plugin_protocol::engine::{Attach, Item, ViewParams};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

/// The text the user typed, out of what the interface wrapped it in (`<USER_REQUEST>`, followed by its metadata).
pub fn typed_text(content: &str) -> String {
    match content.split_once("<USER_REQUEST>") {
        Some((_, rest)) => rest
            .split_once("</USER_REQUEST>")
            .map_or(rest, |(t, _)| t)
            .trim()
            .to_string(),
        None => content.trim().to_string(),
    }
}

/// The view of one agent.
pub struct View {
    acp: Speaker,
    /// The calls its transcript told, by their id: their tool and arguments, until their end.
    calls: HashMap<String, (String, Value)>,
    /// The calls the conversation holds.
    seen: HashSet<String>,
    /// The translation of a recorded `-p` stream (a note `frame`).
    recorded: Translator,
}

impl View {
    pub fn new(p: &ViewParams) -> Self {
        View {
            acp: Speaker::new(p, &decl("agy", "Antigravity")),
            calls: HashMap::new(),
            seen: p.known.iter().map(|k| k.id.clone()).collect(),
            recorded: Translator::new(&Attach {
                agent: p.agent.clone(),
                cwd: p.cwd.clone(),
                model: p.model.clone(),
                mode: p.mode.clone(),
                lang: p.lang.clone(),
                ..Attach::default()
            }),
        }
    }

    /// One step of the transcript, or a note of the core.
    pub fn line(&mut self, line: &str, p: &ViewParams, out: &mut Vec<Item>) {
        let Ok(e) = serde_json::from_str::<Value>(line) else {
            return;
        };
        match e["escouade"].as_str() {
            Some("end") => {
                let error = e["error"].as_str().map(str::to_string);
                out.extend(self.acp.end(p.interrupted, error));
            }
            Some("frame") => out.extend(self.recorded.recorded(&e["frame"])),
            Some(_) => {}
            None => self.step(&e, p, out),
        }
    }

    fn step(&mut self, e: &Value, p: &ViewParams, out: &mut Vec<Item>) {
        out.push(Item::Spoke);
        let Some(step) = e["step_index"].as_u64() else {
            return;
        };
        match (e["type"].as_str(), e["source"].as_str()) {
            (Some("USER_INPUT"), Some("USER_EXPLICIT")) => {
                let text = typed_text(e["content"].as_str().unwrap_or_default());
                if !text.is_empty() {
                    self.acp.begin();
                    out.push(Item::Wrote);
                    out.push(Item::Typed {
                        id: None,
                        text,
                        images: 0,
                        prompt: None,
                    });
                }
            }
            (Some("PLANNER_RESPONSE"), _) => self.answer(step, e, p, out),
            (Some("GENERIC"), _) => self.run(step, e, p, out),
            _ => {}
        }
    }

    fn session(p: &ViewParams) -> &str {
        p.session.as_deref().unwrap_or_default()
    }

    fn answer(&mut self, step: u64, e: &Value, p: &ViewParams, out: &mut Vec<Item>) {
        out.push(Item::Runs);
        let text = e["content"].as_str().unwrap_or_default();
        if !text.trim().is_empty() {
            out.extend(self.acp.say(text));
        }
        let calls = e["tool_calls"].as_array().map(Vec::as_slice);
        let calls = calls.unwrap_or_default();
        for (i, c) in calls.iter().enumerate() {
            let id = call_id(Self::session(p), step + 1 + i as u64);
            if !self.seen.insert(id.clone()) {
                continue;
            }
            let name = c["name"].as_str().unwrap_or_default();
            self.calls
                .insert(id.clone(), (name.to_string(), c["args"].clone()));
            let mut call = acp_call(name, &c["args"]);
            call["toolCallId"] = json!(id);
            call["sessionUpdate"] = json!("tool_call");
            call["status"] = json!("pending");
            out.extend(self.acp.update(call));
            if name == "ask_question" {
                let question = c["args"]["questions"][0]["question"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                out.push(Item::Waits { body: question });
            }
        }
        if calls.is_empty() && !text.trim().is_empty() {
            out.push(Item::ViewEnd {
                interrupted: false,
                duration: None,
                error: None,
            });
        }
    }

    fn run(&mut self, step: u64, e: &Value, p: &ViewParams, out: &mut Vec<Item>) {
        let id = call_id(Self::session(p), step);
        if !self.seen.contains(&id) {
            return;
        }
        out.push(Item::Goes);
        let failed = e["status"] == "ERROR";
        let said = if failed {
            e["error"].as_str().unwrap_or("error")
        } else {
            e["content"].as_str().unwrap_or_default()
        };
        let mut end = match self.calls.remove(&id) {
            Some((name, args)) => {
                let mut call = acp_call(&name, &args);
                if !failed && call["kind"] == "edit" && diff_of(&name, &args).is_none() {
                    if let Some(path) = path_of(&args) {
                        call["content"] =
                            json!([{ "type": "diff", "path": path, "oldText": "", "newText": "" }]);
                    }
                }
                call
            }
            None => json!({ "kind": "other" }),
        };
        end["sessionUpdate"] = json!("tool_call_update");
        end["toolCallId"] = json!(id);
        end["status"] = json!(if failed { "failed" } else { "completed" });
        end["rawOutput"] = output(&end["kind"], said, failed);
        out.extend(self.acp.update(end));
    }
}
