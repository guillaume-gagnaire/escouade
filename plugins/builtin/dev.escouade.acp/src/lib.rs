//! The generic ACP plugin (`dev.escouade.acp`, PLG-6, docs/PLUGINS.md §1.1): it serves the harnesses other plugins only
//! describe (`contributes.harnesses[].acp` of their manifest: an agent that speaks ACP, its program and arguments, its
//! modes and models), on the contract of the harnesses in process (INT-9). The core finds the agent's program itself,
//! checks the launch this plugin describes and starts it; what the agent writes comes here in batches (`agent/frames`)
//! and goes back as the items of the conversation and the lines to write to it, by the client of `escouade_acp`.
//!
//! The description comes with each agent (`described` of `engine/launch` and `agent/attach`): this plugin holds no
//! harness of its own, and keeps nothing on disk.

use escouade_acp::Agent;
use escouade_plugin_protocol::engine::{
    method, AgentRef, AnswerPermission, AnswerQuestion, Attach, Frames, Items, LaunchParams,
    SendMessage, SetValue, Usage, Writes,
};
use escouade_plugin_protocol::{serve, Refusal};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Default)]
struct Plugin {
    version: String,
    agents: HashMap<String, Agent>,
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

pub fn run() -> i32 {
    let mut plugin = Plugin {
        version: std::env::var("ESCOUADE_VERSION").unwrap_or_default(),
        ..Plugin::default()
    };
    let stdin = std::io::stdin();
    let result = serve(
        stdin.lock(),
        std::io::stdout().lock(),
        json!({ "harnesses": ["acp"] }),
        |m, params| plugin.handle(m, params),
    );
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("escouade-plugins acp: {e}");
            1
        }
    }
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
                let Some(decl) = a.described.clone() else {
                    return Err(refused(
                        "dev.escouade.acp serves the harnesses a manifest describes: none given"
                            .into(),
                    ));
                };
                match self.agents.get_mut(&a.agent) {
                    Some(agent) => agent.attach(&a),
                    None => {
                        let agent = Agent::new(&a, &decl, &self.version);
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
                    let Some(decl) = p.described.clone() else {
                        return Err(refused("no harness described".into()));
                    };
                    let a = Attach {
                        agent: p.agent.clone(),
                        cwd: p.cwd.clone(),
                        lang: p.lang.clone(),
                        ..Attach::default()
                    };
                    self.agents
                        .insert(p.agent.clone(), Agent::new(&a, &decl, &self.version));
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
                writes(self.agent(&s.agent)?.send(&s.text))
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
