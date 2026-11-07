//! The plugin Copilot CLI (`dev.escouade.copilot-cli`, INT-10.3, docs/PLUGINS.md §4): GitHub Copilot CLI as a harness.
//! It is no ACP harness: the core finds `copilot`, checks the launch this plugin describes and starts it, its messages
//! framed by `Content-Length` headers (`framing` of the manifest); what Copilot writes comes here (`agent/frames`) and
//! goes back as the items of the conversation and the messages to write to it, by the client of `client`. It is a module
//! (`run.wasm`).
//!
//! Its « Vue Copilot » is read here too (`view`): the core hands on the lines of the session's log (`view/translate`).

pub mod client;
pub mod translate;
pub mod view;
pub mod words;

use client::Agent;
use escouade_plugin_protocol::engine::{
    method, AgentRef, AnswerPermission, AnswerQuestion, Attach, Compact, Compacted, Frames, Items,
    LaunchParams, SendMessage, SetValue, UsageAsked, ViewParams, Writes,
};
use escouade_plugin_protocol::{turn, Refusal};
use serde_json::{json, Value};
use std::collections::HashMap;

/// The manifest, as the app reads it.
pub const MANIFEST: &str = include_str!("../escouade-plugin.json");

#[derive(Default)]
struct Plugin {
    agents: HashMap<String, Agent>,
    views: HashMap<String, view::View>,
}

fn invalid(e: impl std::fmt::Display) -> Refusal {
    Refusal {
        code: -32602,
        message: format!("invalid params: {e}"),
    }
}

fn read<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, Refusal> {
    serde_json::from_value(params).map_err(invalid)
}

fn give<T: serde::Serialize>(v: T) -> Result<Value, Refusal> {
    serde_json::to_value(v).map_err(invalid)
}

fn writes(lines: Vec<String>) -> Result<Value, Refusal> {
    give(Writes { writes: lines })
}

pub fn run() -> i32 {
    let mut plugin = Plugin::default();
    turn::run(
        "copilot",
        json!({ "harnesses": ["copilot"] }),
        move |m, params| plugin.handle(m, params),
    )
}

escouade_plugin_protocol::module!(run);

impl Plugin {
    fn agent(&mut self, key: &str) -> Result<&mut Agent, Refusal> {
        self.agents.get_mut(key).ok_or_else(|| Refusal {
            code: -32602,
            message: format!("no agent {key}: it was not attached"),
        })
    }

    fn handle(&mut self, m: &str, params: Value) -> Result<Value, Refusal> {
        match m {
            method::ATTACH => {
                let a: Attach = read(params)?;
                match self.agents.get_mut(&a.agent) {
                    Some(agent) => agent.attach(&a),
                    None => {
                        self.agents.insert(a.agent.clone(), Agent::new(&a));
                    }
                }
                Ok(json!({}))
            }
            method::DETACH => {
                let a: AgentRef = read(params)?;
                self.agents.remove(&a.agent);
                Ok(json!({}))
            }
            method::LAUNCH => {
                let p: LaunchParams = read(params)?;
                let agent = self.agent(&p.agent)?;
                give(agent.launch(&p))
            }
            method::FRAMES => {
                let f: Frames = read(params)?;
                let items = self.agent(&f.agent)?.frames(&f.frames);
                give(Items { items })
            }
            method::SEND_MESSAGE => {
                let s: SendMessage = read(params)?;
                writes(self.agent(&s.agent)?.send(&s.text, s.content.as_ref()))
            }
            method::INTERRUPT => {
                let a: AgentRef = read(params)?;
                writes(self.agent(&a.agent)?.interrupt())
            }
            method::SET_MODEL => {
                let v: SetValue = read(params)?;
                writes(self.agent(&v.agent)?.set_model(&v.value))
            }
            method::SET_EFFORT => {
                let v: SetValue = read(params)?;
                writes(self.agent(&v.agent)?.set_effort(&v.value))
            }
            method::SET_MODE => {
                let v: SetValue = read(params)?;
                writes(self.agent(&v.agent)?.set_mode(&v.value))
            }
            method::ANSWER_PERMISSION => {
                let a: AnswerPermission = read(params)?;
                writes(self.agent(&a.agent)?.answer_permission(&a))
            }
            method::ANSWER_QUESTION => {
                let a: AnswerQuestion = read(params)?;
                writes(
                    self.agent(&a.agent)?
                        .answer_question(&a.request, &a.answers),
                )
            }
            method::READ_USAGE => {
                let a: AgentRef = read(params)?;
                give(UsageAsked {
                    writes: self.agent(&a.agent)?.read_usage(),
                })
            }
            method::COMPACT => {
                let c: Compact = read(params)?;
                give(Compacted {
                    writes: self.agent(&c.agent)?.compact(&c.instructions),
                    ..Compacted::default()
                })
            }
            method::RENAME => {
                let v: SetValue = read(params)?;
                writes(self.agent(&v.agent)?.rename(&v.value))
            }
            method::VIEW_TRANSLATE => {
                let p: ViewParams = read(params)?;
                if let Some(gone) = &p.gone {
                    self.views.remove(gone);
                }
                let view = self.views.entry(p.agent.clone()).or_default();
                if p.fresh {
                    view.knows(&p);
                }
                let mut items = Vec::new();
                for line in &p.lines {
                    view.line(line, &p, &mut items);
                }
                give(Items { items })
            }
            other => Err(Refusal::not_found(other)),
        }
    }
}

#[cfg(test)]
mod tests;
