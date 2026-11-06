//! The service of an ACP harness: what `dev.escouade.acp` is, and what a plugin of a harness on this client runs (Cursor
//! CLI, Gemini CLI, and Mistral beside its voice). It answers the contract of the harnesses in process (INT-9) with one
//! `Agent` for each agent, the harness being described with each agent (`described` of `engine/launch` and `agent/attach`)
//! or the plugin's own, as its manifest declares it.
//!
//! It lives in this crate and not in a plugin: a plugin is compiled from its folder alone (docs/PLUGINS.md §8.5.3), so it
//! cannot depend on another by a path.

use crate::Agent;
use escouade_plugin_protocol::engine::{
    method, AgentRef, AnswerPermission, AnswerQuestion, Attach, Frames, HarnessDecl, Items,
    LaunchParams, SendMessage, SetValue, Usage, Writes,
};
use escouade_plugin_protocol::{greeting, turn, Refusal};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;

#[derive(Default)]
struct Plugin {
    agents: HashMap<String, Agent>,
    /// The harness of a plugin of Escouade's own on this client (Cursor CLI, INT-10.1), as its manifest declares it:
    /// taken when the core describes none (`described` is for the harnesses other plugins describe).
    own: Option<HarnessDecl>,
}

fn invalid(e: impl std::fmt::Display) -> Refusal {
    Refusal {
        code: -32602,
        message: format!("invalid params: {e}"),
    }
}

fn read<T: DeserializeOwned>(params: Value) -> Result<T, Refusal> {
    serde_json::from_value(params).map_err(invalid)
}

fn give<T: Serialize>(v: T) -> Result<Value, Refusal> {
    serde_json::to_value(v).map_err(invalid)
}

fn writes(lines: Vec<String>) -> Result<Value, Refusal> {
    give(Writes { writes: lines })
}

/// A refusal that will not change (`Refused`): the launch cannot hold what is asked.
fn refused(message: String) -> Refusal {
    Refusal {
        code: -32010,
        message,
    }
}

/// The version of Escouade, as `initialize` said it: read when an agent comes, which is after it.
fn version() -> String {
    greeting().map(|g| g.version.clone()).unwrap_or_default()
}

/// The harnesses the manifest `manifest` declares (its `contributes.harnesses`), those that read.
pub fn harnesses_of(manifest: &str) -> Vec<HarnessDecl> {
    let v: Value = serde_json::from_str(manifest).unwrap_or_default();
    v["contributes"]["harnesses"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|h| serde_json::from_value(h.clone()).ok())
        .collect()
}

/// The answer of this client to a request of the protocol, for a plugin that serves other things beside it (Mistral, its
/// voice beside Vibe): `own` as for `serve_as`.
pub fn handler(own: Option<HarnessDecl>) -> impl FnMut(&str, Value) -> Result<Value, Refusal> {
    let mut plugin = Plugin {
        own,
        ..Plugin::default()
    };
    move |m, params| plugin.handle(m, params)
}

/// Serves the protocol as the harness `name`: the harnesses described with each agent, and `own`, the harness of a plugin
/// of Escouade's own on this client, when the core describes none (Cursor CLI, INT-10.1). `turn::run`, on both targets.
pub fn serve_as(name: &str, own: Option<HarnessDecl>) -> i32 {
    turn::run(name, json!({ "harnesses": [name] }), handler(own))
}

/// A file of the agent `agent`, as a state machine must have it before it answers: from the disk natively; in a module
/// through the host, which judges it against the folders of that agent (`fs/read`, docs/PLUGINS.md §8.4.6) and the
/// plugin's `permissions.files`. None when it cannot be read, or is refused.
#[cfg(not(target_family = "wasm"))]
pub fn agent_file(_agent: &str, path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// See the native `agent_file`.
#[cfg(target_family = "wasm")]
pub fn agent_file(agent: &str, path: &Path) -> Option<String> {
    let params = json!({ "agent": agent, "path": path.to_string_lossy() });
    let v = turn::host_call("fs/read", params).ok()?;
    v["text"].as_str().map(str::to_string)
}

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
                let Some(decl) = a.described.clone().or_else(|| self.own.clone()) else {
                    return Err(refused(
                        "dev.escouade.acp serves the harnesses a manifest describes: none given"
                            .into(),
                    ));
                };
                match self.agents.get_mut(&a.agent) {
                    Some(agent) => agent.attach(&a),
                    None => {
                        let agent = Agent::new(&a, &decl, &version());
                        self.agents.insert(a.agent.clone(), agent);
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
                if !self.agents.contains_key(&p.agent) {
                    let Some(decl) = p.described.clone().or_else(|| self.own.clone()) else {
                        return Err(refused("no harness described".into()));
                    };
                    let a = Attach {
                        agent: p.agent.clone(),
                        cwd: p.cwd.clone(),
                        lang: p.lang.clone(),
                        ..Attach::default()
                    };
                    self.agents
                        .insert(p.agent.clone(), Agent::new(&a, &decl, &version()));
                }
                let launch = self.agent(&p.agent)?.launch(&p).map_err(refused)?;
                give(launch)
            }
            method::FRAMES => {
                let f: Frames = read(params)?;
                let items = self.agent(&f.agent)?.frames(&f.frames);
                give(Items { items })
            }
            method::SEND_MESSAGE => {
                let s: SendMessage = read(params)?;
                writes(
                    self.agent(&s.agent)?
                        .send_content(&s.text, s.content.as_ref()),
                )
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
                let lines = self
                    .agent(&a.agent)?
                    .answer_permission(
                        &a.request,
                        a.allow,
                        a.always,
                        a.message.as_deref(),
                        a.resume.as_deref(),
                    )
                    .map_err(refused)?;
                writes(lines)
            }
            method::ANSWER_QUESTION => {
                let a: AnswerQuestion = read(params)?;
                let lines = self
                    .agent(&a.agent)?
                    .answer_question(&a.request, &a.answers)
                    .map_err(refused)?;
                writes(lines)
            }
            // ACP tells no quota.
            method::READ_USAGE => give(Usage::default()),
            other => Err(Refusal::not_found(other)),
        }
    }
}
