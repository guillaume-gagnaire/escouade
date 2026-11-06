//! The plugin Antigravity CLI (`dev.escouade.antigravity-cli`, INT-10.8, docs/PLUGINS.md §4): Google's agent in a terminal
//! (`agy`) as a harness, by its mode without a screen in stream-json. It is no ACP harness: the core finds `agy`, checks the
//! launch this plugin describes and starts it, as many times as its client asks (`relaunches`: it takes its model, effort and
//! mode at its launch alone, and has no way to stop a turn); what it writes comes here (`agent/frames`) and goes back as the
//! items of the conversation and the messages to write to it.
//!
//! `translate` and `words` are also read by the core, as a library, for what stays in it: its « Vue Antigravity » writes the
//! same rules and reads its calls in the same form.
//!
//! It is a module (`run.wasm`): it reads the agent's home by `fs/read` (`host`), and a transcript written late on the host's
//! timer (`client`, docs/PLUGINS.md §8.4.6).

pub mod client;
mod host;
pub mod translate;
pub mod words;

use client::Client;
use escouade_plugin_protocol::engine::{
    method, AgentRef, AnswerPermission, AnswerQuestion, Attach, Frames, Gone, Interrupt, Item,
    Items, LaunchParams, SendMessage, SetValue,
};
use escouade_plugin_protocol::{turn, Refusal};
use serde_json::{json, Value};
use std::collections::HashMap;

/// The manifest, as the app reads it.
pub const MANIFEST: &str = include_str!("../escouade-plugin.json");

#[derive(Default)]
struct Plugin {
    agents: HashMap<String, Client>,
    /// The host's timer is set: one is enough for all the agents.
    armed: bool,
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

fn items(items: Vec<Item>) -> Result<Value, Refusal> {
    give(Items { items })
}

fn refused(message: String) -> Refusal {
    Refusal {
        code: -32000,
        message,
    }
}

/// What its module runs, on its first turn.
pub fn run() -> i32 {
    let mut plugin = Plugin::default();
    turn::run("agy", json!({ "harnesses": ["agy"] }), move |m, params| {
        plugin.handle(m, params)
    })
}

escouade_plugin_protocol::module!(run);

impl Plugin {
    fn agent(&mut self, key: &str) -> Result<&mut Client, Refusal> {
        self.agents.get_mut(key).ok_or_else(|| Refusal {
            code: -32602,
            message: format!("no agent {key}: it was not attached"),
        })
    }

    /// One call, then the timer set if an agent holds what `agy` said.
    fn handle(&mut self, m: &str, params: Value) -> Result<Value, Refusal> {
        let answer = self.answer(m, params);
        if !self.armed && self.agents.values().any(Client::waits) {
            self.armed = true;
            host::timer(client::TRANSCRIPT_EVERY_MS);
        }
        answer
    }

    /// The timer rang: what each agent held and can now read goes to the core.
    fn rang(&mut self) {
        self.armed = false;
        for (agent, client) in &mut self.agents {
            let items = client.tick();
            if !items.is_empty() {
                host::push(agent, items);
            }
        }
    }

    fn answer(&mut self, m: &str, params: Value) -> Result<Value, Refusal> {
        match m {
            "timer/fired" => {
                self.rang();
                Ok(Value::Null)
            }
            method::ATTACH => {
                let a: Attach = read(params)?;
                self.agents
                    .entry(a.agent.clone())
                    .or_insert_with(|| Client::new(&a));
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
                items(self.agent(&f.agent)?.frames(&f.frames))
            }
            method::SEND_MESSAGE => {
                let s: SendMessage = read(params)?;
                items(self.agent(&s.agent)?.send(&s.text))
            }
            method::INTERRUPT => {
                let a: Interrupt = read(params)?;
                items(self.agent(&a.agent)?.interrupt(a.cancel_queued))
            }
            method::SET_MODEL => {
                let v: SetValue = read(params)?;
                self.agent(&v.agent)?.set_model(&v.value);
                items(Vec::new())
            }
            method::SET_EFFORT => {
                let v: SetValue = read(params)?;
                self.agent(&v.agent)?.set_effort(&v.value);
                items(Vec::new())
            }
            method::SET_MODE => {
                let v: SetValue = read(params)?;
                items(self.agent(&v.agent)?.set_mode(&v.value))
            }
            method::ANSWER_PERMISSION => {
                let a: AnswerPermission = read(params)?;
                items(
                    self.agent(&a.agent)?
                        .answer_permission(&a)
                        .map_err(refused)?,
                )
            }
            method::ANSWER_QUESTION => {
                let a: AnswerQuestion = read(params)?;
                items(
                    self.agent(&a.agent)?
                        .answer_question(&a.request, &a.answers)
                        .map_err(refused)?,
                )
            }
            method::PROGRAM_GONE => {
                let g: Gone = read(params)?;
                items(self.agent(&g.agent)?.program_gone(&g))
            }
            // `agy -p` tells no quota (its `/usage` is its interface's alone).
            method::READ_USAGE => Ok(json!({})),
            other => Err(Refusal::not_found(other)),
        }
    }
}

#[cfg(test)]
mod tests;
