//! The client of `claude` (stream-json and its control protocol, docs/PROTOCOL.md), a state machine without a process of
//! its own: what it is asked turns into lines to write, what the process wrote into items. The core runs the process.

use crate::translate::Translator;
use escouade_plugin_protocol::engine::{
    AnswerPermission, Asked, Attach, EnvVar, Item, Launch, LaunchParams, RemoteLink, StartAt,
};
use serde_json::{json, Value};
use std::collections::HashMap;

/// The tools a discussion of the Docs tab is refused: what writes and what runs a command (ESC-261).
pub const WRITES: [&str; 5] = ["Edit", "Write", "MultiEdit", "NotebookEdit", "Bash"];

/// Refused, every tool of Escouade's server, whatever config declares it: the start of their names.
pub const ESCOUADE_TOOLS: &str = "mcp__escouade";

/// The arguments of a stream of its.
pub const BASE_ARGS: &[&str] = &[
    "--output-format",
    "stream-json",
    "--verbose",
    "--input-format",
    "stream-json",
    "--permission-prompt-tool",
    "stdio",
    "--include-partial-messages",
    // Messages sent from claude.ai (Remote Control) come back on stdout, so the app shows them.
    "--replay-user-messages",
    "--thinking-display",
    "summarized",
    "--allow-dangerously-skip-permissions",
];

/// Whether a model takes an effort, before its account's `claude` has told.
pub fn supports_effort(model: &str) -> bool {
    !model.to_lowercase().contains("haiku")
}

/// What the arguments of a conversation are made of.
pub struct Conversation<'a> {
    pub model: &'a str,
    pub effort: &'a str,
    pub takes_effort: Option<bool>,
    pub mode: &'a str,
    pub read_only: bool,
    pub append_prompt: Option<&'a str>,
    pub start: &'a StartAt,
}

/// The arguments of a conversation: its model, effort and mode, its added instructions, and where it starts from.
pub fn conversation_args(c: &Conversation) -> Vec<String> {
    let mut a: Vec<String> = BASE_ARGS.iter().map(|s| s.to_string()).collect();
    a.extend(["--model".into(), c.model.to_string()]);
    if c.takes_effort.unwrap_or_else(|| supports_effort(c.model)) {
        a.extend(["--effort".into(), c.effort.to_string()]);
    }
    a.extend(["--permission-mode".into(), c.mode.to_string()]);
    // A discussion (ESC-261): its tools that write or run a command taken away, whatever its mode becomes (a plan
    // approved puts it in another). In one value, comma separated, as the Agent SDK gives them: what follows is not read
    // as more.
    if c.read_only {
        a.push(format!("--disallowedTools={}", WRITES.join(",")));
    }
    if let Some(p) = c.append_prompt.filter(|p| !p.trim().is_empty()) {
        // On one line: a `.cmd` launcher (npm's claude.cmd) takes no argument with line breaks.
        let one_line = p.split_whitespace().collect::<Vec<_>>().join(" ");
        a.extend(["--append-system-prompt".into(), one_line]);
    }
    match c.start {
        StartAt::New => {}
        StartAt::Resume { session } => a.push(format!("--resume={session}")),
        StartAt::Fork { of, at } => {
            // A copy's conversation so far is its original's, under a new session: the original's is left as it is.
            a.extend([format!("--resume={of}"), "--fork-session".into()]);
            // As it was when copied: the turns the original ran since stay out.
            if let Some(at) = at {
                a.push(format!("--resume-session-at={at}"));
            }
        }
    }
    a
}

/// All the arguments of its process: its conversation's, then the folders it reads without a question, then what it is
/// given of Escouade's server. Never the token, only the path of the file that holds it (a `.cmd` launcher takes 8,191
/// characters at most, and the app's log writes the arguments). `--disallowedTools` and `--mcp-config` take several
/// values: given last, nothing after them is read as one.
fn launch_args(p: &LaunchParams) -> Vec<String> {
    let mut a = conversation_args(&Conversation {
        model: &p.model,
        effort: &p.effort,
        takes_effort: p.takes_effort,
        mode: &p.mode,
        read_only: p.read_only,
        append_prompt: p.append_prompt.as_deref(),
        start: &p.start,
    });
    a.extend(p.read_dirs.iter().map(|d| format!("--add-dir={d}")));
    match &p.escouade {
        Some(_) => {
            if let Some(own) = &p.settings {
                a.extend(["--settings".into(), own.clone()]);
            }
            if let Some(file) = &p.escouade_file {
                a.extend(["--mcp-config".into(), file.clone()]);
            }
        }
        None => {
            // Its settings of its own hold the refusal too: Claude Code reads the last `--settings` only.
            if let Some(file) = p.settings.as_ref().or(p.refusal_file.as_ref()) {
                a.extend(["--settings".into(), file.clone()]);
            }
            a.extend(["--disallowedTools".into(), ESCOUADE_TOOLS.into()]);
        }
    }
    a
}

/// The line of a control request.
fn request_line(id: &str, request: Value) -> String {
    json!({ "type": "control_request", "request_id": id, "request": request }).to_string()
}

/// The answer to a control request of the CLI.
pub fn respond(request_id: &str, response: Value) -> String {
    json!({
        "type": "control_response",
        "response": { "subtype": "success", "request_id": request_id, "response": response }
    })
    .to_string()
}

/// The refusal of a control request of the CLI.
pub fn respond_error(request_id: &str, error: &str) -> String {
    json!({
        "type": "control_response",
        "response": { "subtype": "error", "request_id": request_id, "error": error }
    })
    .to_string()
}

/// The answer to a control request of the CLI that lets a tool run.
fn allowed(call: &str, input: Value) -> Value {
    json!({ "behavior": "allow", "updatedInput": input, "toolUseID": call })
}

/// The first request of a process, which asks nothing of the API.
pub fn initialize_line(id: &str) -> String {
    request_line(id, json!({ "subtype": "initialize" }))
}

/// What the answer `frame` to `initialize` of id `id` says: its answer, or why it refused. None for any other line.
pub fn initialized(frame: &Value, id: &str) -> Option<Result<Value, String>> {
    let r = &frame["response"];
    if frame["type"] != "control_response" || r["request_id"] != id {
        return None;
    }
    Some(answer_of(r))
}

fn answer_of(r: &Value) -> Result<Value, String> {
    if r["subtype"] == "success" {
        Ok(r.get("response").cloned().unwrap_or_else(|| json!({})))
    } else {
        Err(r["error"].as_str().unwrap_or("error").to_string())
    }
}

/// What a request waits for from its answer.
enum Waiting {
    Initialize,
    Usage,
    Asked,
}

/// One agent's process, as the plugin knows it.
pub struct Agent {
    translator: Translator,
    lang: String,
    next_id: u64,
    waiting: HashMap<String, Waiting>,
}

impl Agent {
    pub fn new(a: &Attach) -> Self {
        Self {
            translator: Translator::default(),
            lang: a.lang.clone(),
            next_id: 1,
            waiting: HashMap::new(),
        }
    }

    /// Given again (after a restart, a change): what the plugin cannot know by itself.
    pub fn attach(&mut self, a: &Attach) {
        self.lang = a.lang.clone();
    }

    fn id(&mut self) -> String {
        let id = format!("ccm_{}", self.next_id);
        self.next_id += 1;
        id
    }

    /// The launch of its process, its first request on its input.
    pub fn launch(&mut self, p: &LaunchParams) -> Launch {
        let id = self.id();
        self.waiting.insert(id.clone(), Waiting::Initialize);
        Launch {
            args: launch_args(p),
            env: Vec::<EnvVar>::new(),
            fence_applied: !p.fence.is_empty() && p.settings.is_some(),
            read_only: p.read_only,
            writes: vec![initialize_line(&id)],
            ..Launch::default()
        }
    }

    /// What its process wrote.
    pub fn frames(&mut self, lines: &[String]) -> Vec<Item> {
        let mut out = Vec::new();
        for line in lines {
            // Lines are read as bytes by the core: without the NULs of a message written in UTF-16.
            let line = line.replace('\0', "");
            if line.trim().is_empty() {
                continue;
            }
            match escouade_plugin_protocol::json_line(&line) {
                Ok(f) if f["type"] == "control_response" => self.answered(&f["response"], &mut out),
                Ok(f) => self.translator.frame(&f, &self.lang, &mut out),
                Err(_) => out.push(Item::Said { line }),
            }
        }
        out
    }

    /// The answer to one of its requests.
    fn answered(&mut self, r: &Value, out: &mut Vec<Item>) {
        let Some(id) = r["request_id"].as_str() else {
            return;
        };
        let Some(waiting) = self.waiting.remove(id) else {
            return;
        };
        let answer = answer_of(r);
        let item = match (waiting, answer) {
            (Waiting::Initialize, Ok(v)) => {
                out.push(Item::Told {
                    commands: v["commands"].clone(),
                    models: v["models"].clone(),
                });
                Item::Ready { models: json!([]) }
            }
            (Waiting::Initialize, Err(reason)) => Item::Stop { reason },
            (Waiting::Usage, Ok(v)) => {
                let windows = &v["rate_limits"];
                Item::Usage {
                    usage: if windows.is_object() {
                        windows.clone()
                    } else {
                        Value::Null
                    },
                }
            }
            (Waiting::Usage, Err(_)) => Item::Usage { usage: Value::Null },
            (Waiting::Asked, Ok(answer)) => Item::Answer {
                ask: id.to_string(),
                answer,
                error: None,
            },
            (Waiting::Asked, Err(error)) => Item::Answer {
                ask: id.to_string(),
                answer: Value::Null,
                error: Some(error),
            },
        };
        out.push(item);
    }

    /// A request to the process, whose answer the core waits for.
    pub fn ask(&mut self, request: Value) -> Asked {
        let ask = self.id();
        self.waiting.insert(ask.clone(), Waiting::Asked);
        Asked {
            writes: vec![request_line(&ask, request)],
            ask,
        }
    }

    pub fn send(&self, uuid: &str, text: &str, content: Option<&Value>) -> Vec<String> {
        let content = content.cloned().unwrap_or_else(|| json!(text));
        vec![json!({
            "type": "user",
            "message": { "role": "user", "content": content },
            "parent_tool_use_id": null,
            "uuid": uuid,
        })
        .to_string()]
    }

    pub fn interrupt(&mut self, cancel_queued: bool) -> Asked {
        self.ask(if cancel_queued {
            json!({ "subtype": "interrupt", "cancel_queued": true })
        } else {
            json!({ "subtype": "interrupt" })
        })
    }

    pub fn set_model(&mut self, model: &str) -> Asked {
        self.ask(json!({ "subtype": "set_model", "model": model }))
    }

    pub fn set_effort(&mut self, effort: &str) -> Asked {
        self.ask(json!({ "subtype": "apply_flag_settings", "settings": { "effortLevel": effort } }))
    }

    pub fn set_mode(&mut self, mode: &str) -> Asked {
        self.ask(json!({ "subtype": "set_permission_mode", "mode": mode }))
    }

    pub fn answer_permission(&self, a: &AnswerPermission) -> Vec<String> {
        let response = if !a.allow {
            json!({ "behavior": "deny", "toolUseID": a.call, "message": a.message })
        } else {
            let mut r = allowed(&a.call, a.input.clone());
            if a.always {
                r["updatedPermissions"] = a.rules.clone();
            } else if let Some(mode) = &a.resume {
                // A plan approved: Claude Code would leave Plan for `default` (ESC-277).
                r["updatedPermissions"] =
                    json!([{ "type": "setMode", "mode": mode, "destination": "session" }]);
            }
            r
        };
        vec![respond(&a.request, response)]
    }

    pub fn answer_question(&self, request: &str, call: &str, input: &Value) -> Vec<String> {
        vec![respond(request, allowed(call, input.clone()))]
    }

    /// The quota of its account, which its answer brings (`Item::Usage`).
    pub fn read_usage(&mut self) -> Vec<String> {
        let id = self.id();
        self.waiting.insert(id.clone(), Waiting::Usage);
        vec![request_line(
            &id,
            json!({ "subtype": "get_usage", "skip_behaviors": true }),
        )]
    }

    pub fn rename(&mut self, title: &str) -> Asked {
        self.ask(json!({ "subtype": "rename_session", "title": title, "source": "host" }))
    }

    pub fn stop_task(&mut self, task: &str) -> Asked {
        self.ask(json!({ "subtype": "stop_task", "task_id": task }))
    }

    pub fn task_output(&mut self, task: &str) -> Asked {
        self.ask(json!({ "subtype": "get_task_output", "task_id": task }))
    }

    pub fn remote(&mut self, r: &RemoteLink) -> Asked {
        let mut request = json!({ "subtype": "remote_control", "enabled": r.enabled });
        if r.enabled {
            request["name"] = json!(r.name);
            request["keep_session_on_exit"] = json!(r.keep);
            if let Some(s) = &r.reattach {
                request["reattach_session_id"] = json!(s);
            }
        }
        self.ask(request)
    }
}
