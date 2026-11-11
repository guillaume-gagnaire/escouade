//! The demonstration plugin (`dev.escouade.minimal`), a module (`run.wasm`) of the builds of development and the tests: the
//! least a plugin of the first hand is, which proves the way from the registry to an instance. It does the handshake and
//! answers `minimal/echo` with what it is given and `minimal/hello` with the version of Escouade it was told.
//!
//! It is also the harness « Minimal » in process (INT-9, docs/PLUGINS-DESIGN.md §4.5): the minimal fake engine of the
//! conformance bench made a plugin. Its harness is `tests/fixtures/fake-minimal.mjs`, which the core launches from the
//! description `engine/launch` gives; its frames come here in batches (`agent/frames`) and go
//! back as the normalized items of `escouade_plugin_protocol::engine` (the translation `agent/minimal.rs` does in the
//! core for the in-process fake). What only serves the translation (the calls under way) is kept here, by agent;
//! what the plugin cannot know (its folder, its session) the core gives it (`agent/attach`).

use escouade_plugin_protocol::engine::{
    method, AgentRef, AnswerPermission, AnswerQuestion, Attach, Frames, Hook, HookAnswer, Item,
    Items, Launch, LaunchParams, OneShotLaunch, OneShotOutput, OneShotParams, OneShotText,
    SendMessage, SetValue, StartAt, TerminalParams, TurnUsage, Usage, Writes,
};
use escouade_plugin_protocol::Refusal;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// What it keeps of an agent: what the core told it, and the calls under way (their name, and the file of a patch).
#[derive(Default)]
struct Agent {
    /// Its key, as the core names it (`fs/*` reaches its folder by it).
    key: String,
    cwd: String,
    calls: HashMap<String, (String, Option<String>)>,
}

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

fn read<T: DeserializeOwned>(params: Value) -> Result<T, Refusal> {
    serde_json::from_value(params).map_err(invalid)
}

fn give<T: Serialize>(v: T) -> Result<Value, Refusal> {
    serde_json::to_value(v).map_err(invalid)
}

fn writes(lines: Vec<Value>) -> Result<Value, Refusal> {
    give(Writes {
        writes: lines.into_iter().map(|l| l.to_string()).collect(),
    })
}

pub fn run() -> i32 {
    let mut plugin = Plugin::default();
    escouade_plugin_protocol::turn::run(
        "minimal",
        json!({ "harnesses": ["minimal"] }),
        move |m, params| plugin.handle(m, params),
    )
}

escouade_plugin_protocol::module!(run);

/// Whether `path` is there, read by the host (`fs/read`, under the folder of the agent `agent`).
#[cfg(target_family = "wasm")]
fn exists(agent: &str, path: &str) -> bool {
    escouade_plugin_protocol::turn::host_call("fs/read", json!({ "agent": agent, "path": path }))
        .is_ok()
}

/// Writes an empty `path`, by the host (`fs/write`, under the folder of the agent `agent`).
#[cfg(target_family = "wasm")]
fn touch(agent: &str, path: &str) {
    let _ = escouade_plugin_protocol::turn::host_call(
        "fs/write",
        json!({ "agent": agent, "path": path, "text": "" }),
    );
}

#[cfg(not(target_family = "wasm"))]
fn exists(_: &str, path: &str) -> bool {
    std::path::Path::new(path).exists()
}

#[cfg(not(target_family = "wasm"))]
fn touch(_: &str, path: &str) {
    let _ = std::fs::write(path, "");
}

impl Plugin {
    fn handle(&mut self, m: &str, params: Value) -> Result<Value, Refusal> {
        match m {
            "minimal/echo" => Ok(params),
            "minimal/hello" => Ok(json!({
                "escouade": escouade_plugin_protocol::greeting().map(|g| g.version.clone()).unwrap_or_default()
            })),
            method::LAUNCH => give(launch(read(params)?)?),
            method::ATTACH => {
                let a: Attach = read(params)?;
                self.agents.entry(a.agent).or_default().cwd = a.cwd;
                Ok(json!({}))
            }
            method::DETACH => {
                let a: AgentRef = read(params)?;
                self.agents.remove(&a.agent);
                Ok(json!({}))
            }
            method::FRAMES => {
                let f: Frames = read(params)?;
                let agent = self.agents.entry(f.agent.clone()).or_default();
                agent.key = f.agent;
                let mut items = Vec::new();
                for line in &f.frames {
                    // A line that is no JSON is left aside, as the core leaves it today.
                    if let Ok(frame) = serde_json::from_str::<Value>(line) {
                        agent.translate(&frame, &mut items);
                    }
                }
                give(Items { items })
            }
            method::SEND_MESSAGE => {
                let s: SendMessage = read(params)?;
                writes(vec![json!({ "say": s.text })])
            }
            method::INTERRUPT => writes(vec![json!({ "interrupt": true })]),
            method::SET_MODEL | method::SET_EFFORT | method::SET_MODE => {
                let _: SetValue = read(params)?;
                writes(Vec::new())
            }
            method::ANSWER_PERMISSION => {
                let a: AnswerPermission = read(params)?;
                let value = if a.allow {
                    json!({ "allow": true })
                } else {
                    json!({ "allow": false, "message": a.message })
                };
                writes(vec![
                    json!({ "answer": { "request": a.request, "value": value } }),
                ])
            }
            method::ANSWER_QUESTION => {
                let a: AnswerQuestion = read(params)?;
                writes(vec![
                    json!({ "answer": { "request": a.request, "value": a.answers } }),
                ])
            }
            method::READ_USAGE => give(Usage::default()),
            method::ONE_SHOT => {
                let q: OneShotParams = read(params)?;
                give(OneShotLaunch {
                    args: vec![
                        "minimal-harness".into(),
                        "--once".into(),
                        format!("{} {}", q.system, q.prompt).trim().to_string(),
                    ],
                    ..OneShotLaunch::default()
                })
            }
            method::ONE_SHOT_READ => {
                let o: OneShotOutput = read(params)?;
                if o.code != Some(0) {
                    return Err(Refusal {
                        code: -32000,
                        message: format!(
                            "the harness ended with {:?}: {}",
                            o.code,
                            o.stderr.trim()
                        ),
                    });
                }
                give(OneShotText {
                    text: o.stdout.trim().to_string(),
                })
            }
            method::TERMINAL_LAUNCH => give(terminal_launch(read(params)?)),
            method::TERMINAL_HOOK => give(hook(read(params)?)),
            other => Err(Refusal::not_found(other)),
        }
    }
}

/// How its harness is launched: the fence and read-only spelled on its command line, Escouade's token in a variable.
fn launch(p: LaunchParams) -> Result<Launch, Refusal> {
    let mut args = vec!["minimal-harness".to_string()];
    match &p.start {
        StartAt::New => {}
        StartAt::Resume { session } => args.extend(["--session".into(), session.clone()]),
        StartAt::Fork { .. } => {
            return Err(Refusal {
                code: -32010,
                message: "Minimal cannot copy a conversation".into(),
            })
        }
    }
    for path in &p.fence {
        args.extend(["--deny".into(), path.clone()]);
    }
    if p.read_only {
        args.push("--read-only".into());
    }
    let mut env = Vec::new();
    if let Some(g) = &p.escouade {
        env.push(escouade_plugin_protocol::engine::EnvVar {
            name: "MINIMAL_ESCOUADE_URL".into(),
            value: g.url.clone(),
        });
        env.push(escouade_plugin_protocol::engine::EnvVar {
            name: "MINIMAL_ESCOUADE_TOKEN".into(),
            value: g.token.clone(),
        });
    }
    Ok(Launch {
        args,
        env,
        fence_applied: true,
        read_only: p.read_only,
        ..Launch::default()
    })
}

/// Its interface in a terminal: the same program, its hooks posted to Escouade's server by `escouade-hook`.
fn terminal_launch(p: TerminalParams) -> Launch {
    let mut args = vec!["minimal-harness".to_string(), "--tui".into()];
    if let Some(s) = &p.session {
        args.extend(["--session".into(), s.clone()]);
    }
    for path in &p.fence {
        args.extend(["--deny".into(), path.clone()]);
    }
    if p.read_only {
        args.push("--read-only".into());
    }
    let mut env = vec![escouade_plugin_protocol::engine::EnvVar {
        name: "MINIMAL_HOOK_URL".into(),
        value: p.hook_url.clone(),
    }];
    if let Some(command) = &p.hook_command {
        env.push(escouade_plugin_protocol::engine::EnvVar {
            name: "MINIMAL_HOOK_COMMAND".into(),
            value: command.clone(),
        });
    }
    Launch {
        args,
        env,
        fence_applied: true,
        read_only: p.read_only,
        trust: vec!["Minimal lit ce dossier.".into()],
        ..Launch::default()
    }
}

/// One of its hooks: `Stop` ends the turn, `PreToolUse` takes the core's answer (context for the model, a refusal)
/// in the form its harness reads.
fn hook(h: Hook) -> HookAnswer {
    let reply = h.reply.unwrap_or_default();
    match h.hook["event"].as_str().unwrap_or_default() {
        "Stop" => HookAnswer {
            items: vec![Item::Turn {
                interrupted: false,
                limited: None,
                usage: TurnUsage::default(),
                error: None,
                entry: None,
            }],
            answer: None,
        },
        "PreToolUse" => HookAnswer {
            items: Vec::new(),
            answer: Some(match (reply.deny, reply.context) {
                (Some(why), _) => json!({ "decision": "block", "reason": why }),
                (None, Some(context)) => json!({ "additionalContext": context }),
                (None, None) => json!({}),
            }),
        },
        _ => HookAnswer::default(),
    }
}

impl Agent {
    /// One frame of its harness, as `agent/minimal.rs` reads it in the core.
    fn translate(&mut self, f: &Value, items: &mut Vec<Item>) {
        let s = |k: &str| f[k].as_str().map(str::to_string);
        let parent = s("parent");
        match f["type"].as_str().unwrap_or("") {
            "start" => items.push(Item::Start {
                session: s("session"),
            }),
            "thinking" => items.push(Item::Thinking {
                text: s("text").unwrap_or_default(),
                parent,
                id: None,
            }),
            "say" => items.push(Item::Text {
                text: s("text").unwrap_or_default(),
                parent,
                id: None,
            }),
            "call" => {
                let (Some(id), Some(tool)) = (s("id"), s("tool")) else {
                    return;
                };
                let input = &f["input"];
                let (name, input, file) = match tool.as_str() {
                    "shell" => ("Bash", json!({ "command": input["command"] }), None),
                    "patch" => {
                        let file = format!(
                            "{}/{}",
                            self.cwd.trim_end_matches(['/', '\\']),
                            input["path"].as_str().unwrap_or_default()
                        );
                        let i = json!({ "file_path": file, "diff": input["diff"] });
                        ("Edit", i, Some(file))
                    }
                    "agent" => (
                        "Task",
                        json!({ "description": input["task"], "prompt": input["task"] }),
                        None,
                    ),
                    other => (other, input.clone(), None),
                };
                self.calls.insert(id.clone(), (name.to_string(), file));
                items.push(Item::Tool {
                    id,
                    name: name.to_string(),
                    input,
                    parent,
                    unplanned: false,
                });
            }
            "done" => {
                let Some(id) = s("call") else { return };
                let error = f["error"] == true;
                let touched = self.calls.remove(&id).and_then(|(_, file)| file);
                items.push(Item::ToolDone {
                    id,
                    output: s("output").unwrap_or_default(),
                    error,
                    result: Value::Null,
                });
                if let (Some(path), false) = (touched, error) {
                    items.push(Item::Touched { path });
                }
            }
            "ask" => {
                if let (Some(request), Some(call)) = (s("request"), s("call")) {
                    items.push(Item::Permission {
                        request,
                        call,
                        tool: None,
                        input: None,
                        description: None,
                        reason: None,
                        diff: None,
                        can_always: false,
                        parent: None,
                        hunks: None,
                        always_by_harness: false,
                        ends_stream: false,
                    });
                }
            }
            "question" => {
                let (Some(request), Some(call)) = (s("request"), s("call")) else {
                    return;
                };
                let options: Vec<Value> = f["choices"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|c| json!({ "label": c, "description": "" }))
                    .collect();
                let questions = json!([{
                    "question": f["text"], "header": "", "multiSelect": false, "options": options,
                }]);
                items.push(Item::Question {
                    request,
                    call,
                    questions,
                    parent: None,
                });
            }
            "end" => {
                self.calls.clear();
                let n = |k: &str| f["usage"][k].as_u64().unwrap_or(0);
                items.push(Item::Turn {
                    interrupted: f["interrupted"] == true,
                    limited: None,
                    usage: TurnUsage {
                        input: n("input"),
                        output: n("output"),
                        ..TurnUsage::default()
                    },
                    error: None,
                    entry: None,
                });
            }
            // The scenario « plantage »: the plugin crashes in the middle of a turn, once (its flag written) or each
            // time.
            "crash" => {
                let flag = s("flag");
                let first = flag.as_deref().is_some_and(|p| !exists(&self.key, p));
                if f["always"] == true || first {
                    if let Some(p) = &flag {
                        touch(&self.key, p);
                    }
                    eprintln!("minimal: crashing, as the scenario asks");
                    panic!("minimal: crashing, as the scenario asks");
                }
            }
            _ => {}
        }
    }
}
