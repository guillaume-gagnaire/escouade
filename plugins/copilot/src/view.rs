//! « Vue Copilot » (`view/translate`, docs/PLUGINS.md §8): what Copilot's own interface writes in its session's log
//! (`session-state/<id>/events.jsonl`), read by the core a line at a time and translated here.
//!
//! The log's entries are the events of the session, as `copilot --server --stdio` sends them (`session.event`), less those it
//! never keeps (`ephemeral`: the deltas, `assistant.usage`, `session.idle`): each goes through the same translation as a
//! process's (`translate`). What is missing is made up: the user's message from its `user.message`, the usage of each call
//! from its `session.usage_record`, the end of a turn from the `agentStop` hook, which the core tells (or `abort` for Escape).

use crate::client::Own;
use crate::translate::{tool_of, Ctx, Translator};
use escouade_plugin_protocol::engine::{Item, ViewParams};
use serde_json::{json, Value};

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

    /// One line of the log, or a note of the core.
    pub fn line(&mut self, line: &str, p: &ViewParams, out: &mut Vec<Item>) {
        let Ok(e) = serde_json::from_str::<Value>(line) else {
            return;
        };
        match e["escouade"].as_str() {
            Some("frame") => {
                let f = &e["frame"];
                self.translated(p, out, |t, c, out| t.frame(f, c, out));
            }
            Some("end") => {
                let from = out.len();
                self.told.turn_end(out);
                self.own.note(&out[from..]);
            }
            // What it is about to ask (`permissionRequest`), as Escouade names its tools.
            Some("hook") => {
                let h = &e["hook"];
                let tool = h["toolName"].as_str().unwrap_or_default();
                let (tool, input) = tool_of(tool, &h["toolInput"], None, &p.cwd);
                out.push(Item::Asks { tool, input });
            }
            _ => self.entry(&e, p, out),
        }
    }

    fn translated(
        &mut self,
        p: &ViewParams,
        out: &mut Vec<Item>,
        read: impl FnOnce(&mut Translator, &Ctx, &mut Vec<Item>),
    ) {
        let from = out.len();
        let c = Ctx {
            cwd: &p.cwd,
            mode: &p.mode,
            active: p.active,
            discussion: p.discussion,
            fenced: false,
            process: false,
            lang: &p.lang,
            resumed: p.session.as_deref().filter(|_| !p.copy),
            seen: &self.own,
        };
        read(&mut self.told, &c, out);
        self.own.note(&out[from..]);
    }

    fn entry(&mut self, e: &Value, p: &ViewParams, out: &mut Vec<Item>) {
        // Written since it started: it is past its question about the folder.
        out.push(Item::Spoke);
        let Some(kind) = e["type"].as_str() else {
            return;
        };
        let own = e["agentId"].is_null();
        if own && (kind.starts_with("user.") || kind.starts_with("assistant.")) {
            out.push(Item::Wrote);
        }
        let d = &e["data"];
        let said = |key: &str| d[key].as_str().unwrap_or_default().to_string();
        match kind {
            "user.message" if own => typed(e, out),
            // Answered in its interface, never by Escouade: what waits for the user is told by
            // its hook (a permission) or by its event (the rest).
            "permission.requested" => {}
            "user_input.requested" => out.push(Item::Waits {
                body: said("question"),
            }),
            "elicitation.requested" => out.push(Item::Waits {
                body: said("message"),
            }),
            "exit_plan_mode.requested" => {
                let plan = d["planContent"].as_str().or(d["summary"].as_str());
                out.push(Item::WaitsOn {
                    tool: "ExitPlanMode".into(),
                    input: json!({ "plan": plan }),
                });
            }
            "permission.completed"
            | "user_input.completed"
            | "elicitation.completed"
            | "exit_plan_mode.completed" => out.push(Item::Goes),
            // Each call of the model, with its usage and its cost (`assistant.usage` is not kept).
            "session.usage_record" => {
                let usage = json!({ "type": "assistant.usage", "data": d["usage"], "agentId": e["agentId"] });
                self.translated(p, out, |t, c, out| t.event(&usage, c, out));
            }
            // Before any turn (its start: a model of `--model` it does not have, and takes `auto`
            // instead, tried on a Free account): said, never the error of the next turn.
            "session.error" if own && !p.active => {
                let message = d["message"].as_str().unwrap_or_default().trim();
                if !message.is_empty() {
                    out.push(Item::Notice {
                        text: message.to_string(),
                        level: "warn".into(),
                    });
                }
            }
            // Escape: the turn is over, as interrupted (no `agentStop` comes for it).
            "abort" if own => {
                if p.active {
                    self.told.aborted = true;
                    out.push(Item::ViewEnd {
                        interrupted: false,
                        duration: None,
                        error: None,
                    });
                }
            }
            _ => self.translated(p, out, |t, c, out| t.event(e, c, out)),
        }
    }
}

/// What the user typed in the interface (or Escouade pasted there): their message.
fn typed(e: &Value, out: &mut Vec<Item>) {
    let d = &e["data"];
    let text = d["content"].as_str().unwrap_or_default().to_string();
    let images = d["attachments"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|a| a["type"] == "file" || a["type"] == "image")
        .count() as u32;
    if text.trim().is_empty() && images == 0 {
        return;
    }
    out.push(Item::Typed {
        id: d["messageId"]
            .as_str()
            .or(e["id"].as_str())
            .map(str::to_string),
        text,
        images,
        prompt: None,
    });
}
