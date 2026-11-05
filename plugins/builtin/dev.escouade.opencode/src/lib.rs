//! The plugin OpenCode (`dev.escouade.opencode`, INT-10.2, docs/PLUGINS.md §4): OpenCode 1 (`opencode-ai`, 1.19) as a
//! harness, driven through `opencode serve` (HTTP + SSE, docs/PROTOCOL.md, « OpenCode serve »).
//!
//! OpenCode is no process with an input and an output for each agent: one server serves the agents of an account, each
//! a session of its own. The core finds `opencode`, starts that server (`Launch::shared`) in its Job object, and keeps
//! it from the account's first agent to its last. Everything else is here: the client of the server (`server`), the
//! agent's session on it (`session`), and the translation of its events into the items of the conversation
//! (`translate`), which go to the core as they come (`agent/items`): only this plugin reads its event stream.
//!
//! What it does as Escouade drives it comes from the real runs of OpenCode 1.19.3 with local models (ESC-25), whose
//! streams are in `tests/fixtures/opencode/` and whose schema is `tests/fixtures/engines/opencode-1.19.3.openapi.json`.

pub mod rules;
mod server;
mod session;
pub mod translate;

pub use rules::{catalog, listed, no_model, split_model};
pub use translate::{absolute, patch_file, Translator};

use escouade_plugin_protocol::engine::{
    method, AgentRef, AnswerPermission, AnswerQuestion, Attach, Close, Compact, Compacted, Control,
    EnvVar, Frames, Interrupt, Items, Launch, LaunchParams, Open, SendMessage, SetValue,
    SharedLaunch, SharedRef, Usage, Writes,
};
use escouade_plugin_protocol::{serve_later, Output, Refusal, Reply};
use parking_lot::Mutex;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use server::Server;
use session::Session;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// The manifest, as the app reads it.
pub const MANIFEST: &str = include_str!("../escouade-plugin.json");

/// What the line that says a server is ready holds, its address after it.
pub(crate) const LISTENING: &str = "listening on ";

struct Plugin {
    out: Output,
    agents: Mutex<HashMap<String, Arc<Session>>>,
    /// The clients of the servers, by what the core calls them (`SharedRef::key`).
    servers: Mutex<HashMap<String, Arc<Server>>>,
}

fn invalid(e: impl std::fmt::Display) -> Refusal {
    Refusal {
        code: -32602,
        message: format!("invalid params: {e}"),
    }
}

fn failed(e: anyhow::Error) -> Refusal {
    Refusal {
        code: -32000,
        message: format!("{e:#}"),
    }
}

fn read<T: DeserializeOwned>(params: Value) -> Result<T, Refusal> {
    serde_json::from_value(params).map_err(invalid)
}

fn give<T: Serialize>(v: T) -> Result<Value, Refusal> {
    serde_json::to_value(v).map_err(invalid)
}

fn no_writes() -> Result<Value, Refusal> {
    give(Writes::default())
}

/// The program of the plugin: `escouade-plugins opencode`.
pub fn run() -> i32 {
    let tokio = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("escouade-plugins opencode: {e}");
            return 1;
        }
    };
    let _in_it = tokio.enter();
    let out = Output::new(std::io::stdout());
    let plugin = Arc::new(Plugin {
        out: out.clone(),
        agents: Mutex::default(),
        servers: Mutex::default(),
    });
    let stdin = std::io::stdin();
    let result = serve_later(
        stdin.lock(),
        out,
        json!({ "harnesses": ["opencode"] }),
        |m, params, reply| plugin.handle(m, params, reply),
    );
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("escouade-plugins opencode: {e}");
            1
        }
    }
}

impl Plugin {
    fn agent(&self, key: &str) -> Result<Arc<Session>, Refusal> {
        self.agents.lock().get(key).cloned().ok_or_else(|| Refusal {
            code: -32602,
            message: format!("no agent {key}: it was not attached"),
        })
    }

    /// The client of the server `s`, one made when it is new to this process.
    fn server(&self, s: &SharedRef) -> anyhow::Result<Arc<Server>> {
        let base = server::listening(&s.line)?;
        let password = s.keep["password"].as_str().unwrap_or_default().to_string();
        let mut servers = self.servers.lock();
        // The same key after a server stopped and another started: another address or password.
        if let Some(known) = servers
            .get(&s.key)
            .filter(|k| k.base() == base && k.password() == password)
        {
            return Ok(known.clone());
        }
        let server = Server::connect(base, password)?;
        servers.insert(s.key.clone(), server.clone());
        Ok(server)
    }

    fn handle(self: &Arc<Self>, m: &str, params: Value, reply: Reply) {
        let answer = self.answer(m, params, &reply);
        if let Some(answer) = answer {
            reply.send(answer);
        }
    }

    /// `reply` answered later, by the task that waits: None.
    fn answer(
        self: &Arc<Self>,
        m: &str,
        params: Value,
        reply: &Reply,
    ) -> Option<Result<Value, Refusal>> {
        let now = || -> Result<Value, Refusal> {
            match m {
                method::ATTACH => {
                    let a: Attach = read(params.clone())?;
                    let session = {
                        let mut agents = self.agents.lock();
                        agents
                            .entry(a.agent.clone())
                            .or_insert_with(|| Session::new(&a, self.out.clone()))
                            .clone()
                    };
                    let server = a
                        .shared
                        .as_ref()
                        .map(|s| self.server(s))
                        .transpose()
                        .map_err(failed)?;
                    session.attached(&a, server);
                    // Heard from now on when the plugin started again, its session known.
                    if a.shared.is_some() {
                        let running = a.running;
                        tokio::spawn(async move { session.listen_again(running).await });
                    }
                    Ok(json!({}))
                }
                method::DETACH => {
                    let a: AgentRef = read(params.clone())?;
                    if let Some(s) = self.agents.lock().remove(&a.agent) {
                        s.forget();
                    }
                    // A client no agent uses any more stops reading its events.
                    if self.agents.lock().is_empty() {
                        self.servers.lock().clear();
                    }
                    Ok(json!({}))
                }
                method::LAUNCH => {
                    let p: LaunchParams = read(params.clone())?;
                    give(launch(&p))
                }
                method::FRAMES => {
                    let f: Frames = read(params.clone())?;
                    give(Items {
                        items: self.agent(&f.agent)?.frames(&f.frames),
                    })
                }
                method::SEND_MESSAGE => {
                    let s: SendMessage = read(params.clone())?;
                    let parts = match s.content {
                        Some(Value::Array(parts)) => parts,
                        _ => vec![json!({ "type": "text", "text": s.text })],
                    };
                    self.agent(&s.agent)?.send(parts).map_err(failed)?;
                    no_writes()
                }
                method::SET_MODEL => {
                    let v: SetValue = read(params.clone())?;
                    self.agent(&v.agent)?.set_model(&v.value);
                    no_writes()
                }
                method::SET_EFFORT => {
                    let v: SetValue = read(params.clone())?;
                    self.agent(&v.agent)?.set_effort(&v.value);
                    no_writes()
                }
                method::SET_MODE => {
                    let v: SetValue = read(params.clone())?;
                    self.agent(&v.agent)?.set_mode(&v.value);
                    no_writes()
                }
                method::ANSWER_PERMISSION => {
                    let a: AnswerPermission = read(params.clone())?;
                    self.agent(&a.agent)?
                        .answer_permission(&a)
                        .map_err(failed)?;
                    no_writes()
                }
                method::ANSWER_QUESTION => {
                    let a: AnswerQuestion = read(params.clone())?;
                    self.agent(&a.agent)?
                        .answer_question(&a.request, &a.questions, &a.answers);
                    no_writes()
                }
                // OpenCode has no quota of its own: it goes through the providers' APIs.
                method::READ_USAGE => give(Usage::default()),
                method::CLOSE => {
                    let c: Close = read(params.clone())?;
                    self.agent(&c.agent)?.close(c.now);
                    no_writes()
                }
                method::COMPACT => {
                    let c: Compact = read(params.clone())?;
                    self.agent(&c.agent)?.compact().map_err(failed)?;
                    give(Compacted {
                        over: false,
                        ..Compacted::default()
                    })
                }
                other => Err(Refusal::not_found(other)),
            }
        };
        // What waits for the server answers from a task of its own.
        let reply = match m {
            method::OPEN
            | method::INTERRUPT
            | method::LET_GO
            | method::RENAME
            | method::CONTROL => reply.clone(),
            _ => return Some(now()),
        };
        let me = self.clone();
        let m = m.to_string();
        tokio::spawn(async move {
            let answer = me.later(&m, params).await;
            reply.send(answer);
        });
        None
    }

    async fn later(&self, m: &str, params: Value) -> Result<Value, Refusal> {
        match m {
            method::OPEN => {
                let o: Open = read(params)?;
                give(self.agent(&o.agent)?.open(&o).await.map_err(failed)?)
            }
            method::INTERRUPT => {
                let a: Interrupt = read(params)?;
                self.agent(&a.agent)?
                    .interrupt(a.cancel_queued)
                    .await
                    .map_err(failed)?;
                no_writes()
            }
            method::LET_GO => {
                let a: AgentRef = read(params)?;
                self.agent(&a.agent)?.let_go().await.map_err(failed)?;
                Ok(json!({}))
            }
            method::RENAME => {
                let v: SetValue = read(params)?;
                self.agent(&v.agent)?
                    .rename(&v.value)
                    .await
                    .map_err(failed)?;
                Ok(json!({}))
            }
            method::CONTROL => {
                let c: Control = read(params)?;
                self.agent(&c.agent)?
                    .control(c.request, Duration::from_millis(c.within_ms.max(1)))
                    .await
                    .map_err(failed)
            }
            other => Err(Refusal::not_found(other)),
        }
    }
}

/// The server of an account, as its first agent starts it: its address read on its output, its password in its
/// environment. The fence of a private project and read-only are rules of each session, which this plugin sets
/// (`fence_by_client`, believed of a plugin the app gives).
fn launch(p: &LaunchParams) -> Launch {
    let password = server::new_password();
    let env = [
        (
            "OPENCODE_CONFIG_CONTENT",
            rules::config_content(&p.model).to_string(),
        ),
        ("OPENCODE_DISABLE_AUTOUPDATE", "1".to_string()),
        ("OPENCODE_ENABLE_QUESTION_TOOL", "1".to_string()),
        ("OPENCODE_SERVER_PASSWORD", password.clone()),
    ];
    Launch {
        args: ["serve", "--hostname", "127.0.0.1", "--port", "0"]
            .map(String::from)
            .to_vec(),
        env: env
            .into_iter()
            .map(|(name, value)| EnvVar {
                name: name.into(),
                value,
            })
            .collect(),
        fence_by_client: true,
        read_only: true,
        shared: Some(SharedLaunch {
            listens: None,
            ready: LISTENING.into(),
            one_agent_a_folder: true,
            keep: json!({ "password": password }),
        }),
        ..Launch::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_server_is_described_shared_with_its_password_never_among_its_arguments() {
        let l = launch(&LaunchParams {
            model: "ollama/qwen3-coder:30b-32k".into(),
            ..LaunchParams::default()
        });
        let password = l
            .env
            .iter()
            .find(|v| v.name == "OPENCODE_SERVER_PASSWORD")
            .map(|v| v.value.clone())
            .unwrap();
        assert_eq!(password.len(), 48);
        assert!(!l.args.iter().any(|a| a.contains(&password)));
        let shared = l.shared.unwrap();
        assert!(shared.one_agent_a_folder);
        assert_eq!(shared.keep["password"], password.as_str());
        assert!(l.fence_by_client && l.read_only);
    }

    #[test]
    fn its_manifest_declares_the_harness_opencode() {
        let m: Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(m["id"], "dev.escouade.opencode");
        assert_eq!(m["run"]["args"][0], "opencode");
        let h = &m["contributes"]["harnesses"][0];
        assert_eq!(h["id"], "opencode");
        assert_eq!(h["accountVar"], "OPENCODE_CONFIG_DIR");
    }
}
