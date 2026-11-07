//! The plugin OpenCode (`dev.escouade.opencode`, INT-10.2, docs/PLUGINS.md §4, §8): OpenCode 1 (`opencode-ai`, 1.19) as a
//! harness, driven through `opencode serve` (HTTP + SSE, docs/PROTOCOL.md, « OpenCode serve »). A module (`run.wasm`)
//! without tokio: its tasks turn on the events of the host (`host`, `Reactor`).
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

use escouade_plugin_kit::host::Host;
use escouade_plugin_protocol::engine::{
    method, AgentRef, AnswerPermission, AnswerQuestion, Attach, Close, Compact, Compacted, Control,
    EnvVar, Frames, Interrupt, Items, Launch, LaunchParams, Open, SendMessage, SetValue,
    SharedLaunch, SharedRef, Usage, Writes,
};
use escouade_plugin_protocol::{
    answer, code, greeted, initialized, method as handshake, Refusal, Request,
};
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

/// That line, as the core reads the port in it (`local:shared`, docs/PLUGINS.md §8.4.2).
const LISTENS: &str = "opencode server listening on http://{host}:{port}";

struct Plugin {
    host: Arc<Host>,
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

/// The plugin between two turns: what comes in by lines, what goes out by lines. A module has one; a test drives its own.
pub struct Reactor(Arc<Plugin>);

impl Default for Reactor {
    fn default() -> Self {
        Self(Arc::new(Plugin {
            host: Arc::default(),
            agents: Mutex::default(),
            servers: Mutex::default(),
        }))
    }
}

impl Reactor {
    /// One turn on what came: the lines to write.
    pub fn turn(&self, lines: impl IntoIterator<Item = String>) -> Vec<String> {
        for line in lines {
            if let Some(msg) = self.0.host.arrived(&line) {
                self.0.called(msg);
            }
        }
        self.0.host.run();
        self.0.host.lines()
    }
}

/// The plugin of this module, once its first turn made it.
#[cfg(target_family = "wasm")]
static PLUGIN: std::sync::OnceLock<Reactor> = std::sync::OnceLock::new();

#[cfg(target_family = "wasm")]
fn turn() {
    use escouade_plugin_protocol::turn;
    let Some(plugin) = PLUGIN.get() else { return };
    for line in plugin.turn(turn::lines()) {
        turn::write(&line);
    }
    turn::flush();
}

/// What the module's first turn runs: the plugin kept for the turns to come.
#[cfg(target_family = "wasm")]
fn run() -> i32 {
    let _ = PLUGIN.set(Reactor::default());
    escouade_plugin_protocol::turn::set_turn(turn);
    0
}

escouade_plugin_protocol::module!(run);

/// How a request is answered, now or by the task that waits; a notification has nothing to answer.
#[derive(Clone)]
struct Reply {
    id: Option<Value>,
    host: Arc<Host>,
}

impl Reply {
    fn send(&self, result: Result<Value, Refusal>) {
        if let Some(id) = &self.id {
            self.host.write(answer(id, result));
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

    /// A line that is a call: the handshake is answered here, the rest by `handle`.
    fn called(self: &Arc<Self>, msg: Value) {
        let Ok(request) = serde_json::from_value::<Request>(msg) else {
            let refusal = Refusal {
                code: code::PARSE,
                message: "not a JSON-RPC request".into(),
            };
            self.host.write(answer(&Value::Null, Err(refusal)));
            return;
        };
        let reply = Reply {
            id: request.id,
            host: self.host.clone(),
        };
        match request.method.as_str() {
            handshake::INITIALIZE => {
                greeted(&request.params);
                let can = json!({ "harnesses": ["opencode"] });
                reply.send(Ok(initialized(can)));
            }
            handshake::SHUTDOWN => reply.send(Ok(Value::Null)),
            other => self.handle(other, request.params, reply),
        }
    }

    /// The client of the server `s`, one made when it is new to this instance.
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
        let server = Server::connect(&self.host, base, password);
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
                            .or_insert_with(|| Session::new(&a, self.host.clone()))
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
                        self.host
                            .spawn(async move { session.listen_again(running).await });
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
            | method::SET_MODE
            | method::LET_GO
            | method::RENAME
            | method::CONTROL => reply.clone(),
            _ => return Some(now()),
        };
        let me = self.clone();
        let m = m.to_string();
        self.host.spawn(async move {
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
            method::SET_MODE => {
                let v: SetValue = read(params)?;
                self.agent(&v.agent)?
                    .set_mode(&v.value)
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
            listens: Some(LISTENS.into()),
            ready: LISTENING.into(),
            one_agent_a_folder: true,
            keep: json!({ "password": password }),
        }),
        ..Launch::default()
    }
}

#[cfg(test)]
mod tests;
