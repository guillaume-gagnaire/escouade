//! The plugin Codex (`dev.escouade.codex`, INT-10.4, docs/PLUGINS.md §4): OpenAI's Codex as a harness. It is no ACP
//! harness: the core finds `codex`, checks the launch this plugin describes and starts its `app-server`; what Codex
//! writes comes here (`agent/frames`) and goes back as the items of the conversation and the messages to write to it,
//! by the client of `client`. It is a module (`run.wasm`): what it needs of the disk comes through the host (`fs/*`).
//!
//! `translate`, `words` and `prices` are also read by the core, as a library, until P8, for what stays in it: its « Vue Codex »
//! reads the session's rollout through the same translation, and the launch of its interface, an account's quota and
//! models speak the same words.

pub mod client;
pub mod prices;
pub mod translate;
pub mod words;

use client::Agent;
use escouade_plugin_protocol::engine::{
    method, AgentRef, AnswerPermission, AnswerQuestion, Attach, Compact, Compacted, Frames, Items,
    LaunchParams, SendMessage, SetValue, UsageAsked, Writes,
};
use escouade_plugin_protocol::{turn, Refusal};
use serde_json::{json, Value};
use std::collections::HashMap;

/// The manifest, as the app reads it.
pub const MANIFEST: &str = include_str!("../escouade-plugin.json");

#[derive(Default)]
struct Plugin {
    agents: HashMap<String, Agent>,
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
        "codex",
        json!({ "harnesses": ["codex"] }),
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
                self.agent(&v.agent)?.set_model(&v.value);
                writes(Vec::new())
            }
            method::SET_EFFORT => {
                let v: SetValue = read(params)?;
                self.agent(&v.agent)?.set_effort(&v.value);
                writes(Vec::new())
            }
            method::SET_MODE => {
                let v: SetValue = read(params)?;
                self.agent(&v.agent)?.set_mode(&v.value);
                writes(Vec::new())
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
                match self.agent(&c.agent)?.compact() {
                    // A turn of its own, which ends it.
                    Some(writes) => give(Compacted {
                        writes,
                        turn: true,
                        ..Compacted::default()
                    }),
                    None => Err(Refusal {
                        code: -32000,
                        message: "Codex has no thread yet".into(),
                    }),
                }
            }
            method::RENAME => {
                let v: SetValue = read(params)?;
                writes(self.agent(&v.agent)?.rename(&v.value))
            }
            other => Err(Refusal::not_found(other)),
        }
    }
}

#[cfg(test)]
mod tests;
