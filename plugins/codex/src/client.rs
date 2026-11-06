//! The client of `codex app-server`, over its JSON-RPC protocol (one message a line on stdio, docs/PROTOCOL.md,
//! « Codex app-server »). It holds no process: the core launches Codex from `Agent::launch` and hands its lines here;
//! `Agent` is a state machine that answers the items of the conversation and the messages to write back. A step that
//! waited for an answer is a value of `Ours`, taken up when the answer comes.
//!
//! It keeps what a turn needs that the core does not hold: its thread, the turn running, and the model, effort and mode
//! of the next turn (Codex takes them at `turn/start`), and the requests Codex made that wait for their answer.

use crate::translate::{Ctx, Seen, Translator};
use crate::words::{self, id_key, is_plan_approval, mode_params, Settings};
use escouade_acp::vibe::worded;
use escouade_plugin_protocol::engine::{
    AnswerPermission, Attach, EnvVar, Item, Launch, LaunchParams, StartAt,
};
use escouade_plugin_protocol::turn;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// How many pages of `model/list` are read at most (a cursor that never ends ends there).
const MODEL_PAGES: usize = 50;

/// What an answer of Codex takes up.
enum Ours {
    Initialize,
    /// `thread/start`, `thread/resume` or `thread/fork`.
    Thread(&'static str),
    /// A page of `model/list`, what was read before it, and how many pages that is.
    Models(Vec<Value>, usize),
    /// A request whose refusal alone is told: `turn/start`, `turn/steer`.
    Fired(&'static str),
    Usage,
    Compact,
    /// A request whose answer tells nothing.
    Said,
}

/// A request of Codex that waits for its answer: its id, method and parameters.
struct Asked {
    id: Value,
    method: String,
    params: Value,
}

/// What the conversation holds, as the translation asks it: the core keeps the conversation, this is the plugin's
/// memory of it, no more than each call's name and whether it runs.
#[derive(Default)]
struct Own {
    ids: HashSet<String>,
    calls: HashMap<String, (String, &'static str)>,
}

impl Seen for Own {
    fn has(&self, id: &str) -> bool {
        self.ids.contains(id) || self.calls.contains_key(id)
    }
    fn call(&self, id: &str) -> Option<Value> {
        let (name, status) = self.calls.get(id)?;
        Some(json!({ "name": name, "status": status }))
    }
}

impl Own {
    fn note(&mut self, items: &[Item]) {
        for item in items {
            match item {
                Item::Delta { id, .. }
                | Item::Text { id: Some(id), .. }
                | Item::Thinking { id: Some(id), .. } => {
                    self.ids.insert(id.clone());
                }
                Item::Tool { id, name, .. } => {
                    self.calls.insert(id.clone(), (name.clone(), "running"));
                }
                Item::ToolDone { id, .. } => {
                    if let Some(c) = self.calls.get_mut(id) {
                        c.1 = "ok";
                    }
                }
                // What ran is closed by the turn's end, and by the card of a plan, the turn over on it.
                Item::Turn { .. }
                | Item::Permission {
                    ends_stream: true, ..
                } => {
                    for c in self.calls.values_mut().filter(|c| c.1 == "running") {
                        c.1 = "interrupted";
                    }
                }
                _ => {}
            }
        }
    }
}

/// The client of one agent's Codex.
pub struct Agent {
    /// How the core names it: what `fs/read` takes.
    key: String,
    /// The folder of its account, as the host resolved it (`roots` of `agent/attach`).
    home: Option<PathBuf>,
    cwd: String,
    lang: String,
    settings: Settings,
    start: StartAt,
    /// The instructions added to a new thread (`developerInstructions`).
    instructions: Option<String>,
    /// Its thread, once `thread/start`, `thread/resume` or `thread/fork` answered.
    thread: Option<String>,
    /// The agent's session, as the core knows it: its thread once told, the one it means to resume before.
    session: Option<String>,
    /// The turn of its thread under way, from `turn/started` to `turn/completed`.
    turn: Option<String>,
    /// A `turn/start` sent, its `turn/started` not come yet: a message meanwhile waits for it.
    starting: bool,
    /// Messages that came while a turn was starting: steered into it once it has its id.
    held: Vec<Value>,
    /// Messages that came before its thread was open.
    queue: Vec<Value>,
    /// A turn runs, as the core sees it.
    active: bool,
    /// The user stopped the turn.
    interrupted: bool,
    /// The requests Codex made that wait for their answer, by id.
    asked: HashMap<String, Asked>,
    next: u64,
    ours: HashMap<u64, Ours>,
    told: Translator,
    own: Own,
}

fn agent_home(a: &Attach) -> Option<PathBuf> {
    a.roots.get("agentHome").map(PathBuf::from)
}

fn write(line: String) -> Item {
    Item::Write { line }
}

impl Agent {
    pub fn new(a: &Attach) -> Self {
        Self {
            key: a.agent.clone(),
            home: agent_home(a),
            cwd: a.cwd.clone(),
            lang: a.lang.clone(),
            settings: Settings {
                model: a.model.clone(),
                effort: a.effort.clone(),
                mode: a.mode.clone(),
                ..Settings::default()
            },
            start: StartAt::New,
            instructions: None,
            // Given again after a restart, its harness still there: its thread is open.
            thread: a.session.clone(),
            session: a.session.clone(),
            turn: None,
            starting: false,
            held: Vec::new(),
            queue: Vec::new(),
            active: false,
            interrupted: false,
            // The requests waiting after a restart: their method is lost, not their id.
            asked: a
                .pending
                .iter()
                .map(|r| {
                    let id = r.parse::<u64>().map_or_else(|_| json!(r), |n| json!(n));
                    let asked = Asked {
                        id,
                        method: String::new(),
                        params: Value::Null,
                    };
                    (r.clone(), asked)
                })
                .collect(),
            // Its ids are told apart from those of a process of this plugin before it restarted.
            next: turn::instance_id() << 20,
            ours: HashMap::new(),
            told: Translator::default(),
            own: Own::default(),
        }
    }

    /// The agent given again: what may have changed.
    pub fn attach(&mut self, a: &Attach) {
        self.settings.model = a.model.clone();
        self.settings.effort = a.effort.clone();
        self.settings.mode = a.mode.clone();
        self.home = agent_home(a);
    }

    /// The description of its launch, and the first line it is written.
    pub fn launch(&mut self, p: &LaunchParams) -> Launch {
        self.cwd = p.cwd.clone();
        self.lang = p.lang.clone();
        self.settings = Settings {
            model: p.model.clone(),
            effort: p.effort.clone(),
            mode: p.mode.clone(),
            fence: p.fence.clone(),
            read_only: p.read_only,
        };
        self.start = p.start.clone();
        self.session = match &p.start {
            StartAt::Resume { session } => Some(session.clone()),
            _ => None,
        };
        self.thread = None;
        self.instructions = p.append_prompt.clone().filter(|i| !i.trim().is_empty());
        let home = p
            .home
            .as_deref()
            .filter(|h| !h.is_empty())
            .map(PathBuf::from)
            .or_else(|| self.home.clone());
        let key = self.key.clone();
        let args = words::app_server_args(
            &p.fence,
            &p.mode,
            p.read_only,
            p.escouade.as_ref(),
            // Only for a refused server: an entry of the user's of its name is turned off.
            || {
                home.and_then(|h| escouade_acp::plugin::agent_file(&key, &h.join("config.toml")))
                    .is_some_and(|text| words::declares_escouade(&text))
            },
        );
        let env = p
            .escouade
            .iter()
            .map(|g| EnvVar {
                name: words::TOKEN_VAR.into(),
                value: g.token.clone(),
            })
            .collect();
        let initialize = self.request(
            "initialize",
            json!({
                "clientInfo": { "name": "escouade", "title": "Escouade", "version": env!("CARGO_PKG_VERSION") },
                // For the mode Plan alone (`collaborationMode` of `turn/start`).
                "capabilities": { "experimentalApi": true },
            }),
            Ours::Initialize,
        );
        Launch {
            args,
            env,
            fence_applied: !p.fence.is_empty(),
            read_only: p.read_only,
            writes: vec![initialize],
            ..Launch::default()
        }
    }

    fn id(&mut self) -> u64 {
        self.next += 1;
        self.next
    }

    /// A request whose answer is taken up as `ours`.
    fn request(&mut self, method: &str, params: Value, ours: Ours) -> String {
        let id = self.id();
        self.ours.insert(id, ours);
        json!({ "id": id, "method": method, "params": params }).to_string()
    }

    /// What Codex wrote: the items for the conversation, and the messages to write back (`Item::Write`).
    pub fn frames(&mut self, lines: &[String]) -> Vec<Item> {
        let mut out = Vec::new();
        for l in lines {
            if let Ok(msg) = serde_json::from_str::<Value>(l) {
                self.read(&msg, &mut out);
            }
        }
        out
    }

    /// One message of Codex: an answer to a request of the client, or what goes to the translation. The turn under way
    /// is followed here.
    fn read(&mut self, msg: &Value, out: &mut Vec<Item>) {
        let Some(method) = msg["method"].as_str() else {
            let Some(ours) = msg["id"].as_u64().and_then(|id| self.ours.remove(&id)) else {
                return;
            };
            let error = msg
                .get("error")
                .filter(|e| !e.is_null())
                .map(|e| e["message"].as_str().unwrap_or("error").to_string());
            self.answered(ours, &msg["result"], error, out);
            return;
        };
        let p = &msg["params"];
        let own = self.thread.is_some() && p["threadId"].as_str() == self.thread.as_deref();
        let mut held = Vec::new();
        match method {
            "turn/started" if own => {
                self.turn = p["turn"]["id"].as_str().map(str::to_string);
                self.starting = false;
                self.active = true;
                held = std::mem::take(&mut self.held);
            }
            "turn/completed" if own && self.turn.as_deref() == p["turn"]["id"].as_str() => {
                self.turn = None;
            }
            "serverRequest/resolved" => {
                self.asked.remove(&id_key(&p["requestId"]));
            }
            m if !msg["id"].is_null() => {
                // A request of Codex: kept until it is answered.
                let asked = Asked {
                    id: msg["id"].clone(),
                    method: m.to_string(),
                    params: p.clone(),
                };
                self.asked.insert(id_key(&msg["id"]), asked);
            }
            _ => {}
        }
        self.translate(msg, out);
        // Messages held while the turn started go into it now.
        for input in held {
            let line = self.steer(input);
            out.extend(line.into_iter().map(write));
        }
    }

    /// `f` through the translation, on what the agent is now; the requests its rules accepted answered.
    fn translate(&mut self, f: &Value, out: &mut Vec<Item>) {
        let from = out.len();
        {
            let c = Ctx {
                cwd: &self.cwd,
                mode: &self.settings.mode,
                model: &self.settings.model,
                active: self.active,
                interrupted: self.interrupted,
                process: true,
                lang: &self.lang,
                session: self.session.as_deref(),
                copy: matches!(self.start, StartAt::Fork { .. }),
                seen: &self.own,
            };
            self.told.frame(f, &c, out);
        }
        self.own.note(&out[from..]);
        for item in &out[from..] {
            match item {
                Item::Session { session } => self.session = Some(session.clone()),
                Item::Turn { .. } => {
                    self.active = false;
                    self.interrupted = false;
                }
                _ => {}
            }
        }
        for accepted in std::mem::take(&mut self.told.approved) {
            if let Some(a) = self.asked.remove(&accepted.request) {
                out.push(write(respond(&a.id, json!({ "decision": "accept" }))));
            }
        }
    }

    /// The answer to one of the client's requests.
    fn answered(&mut self, ours: Ours, result: &Value, error: Option<String>, out: &mut Vec<Item>) {
        match ours {
            Ours::Initialize => {
                if let Some(e) = error {
                    out.push(Item::Stop {
                        reason: format!("initialize: {e}"),
                    });
                    return;
                }
                out.push(write(json!({ "method": "initialized" }).to_string()));
                let (method, params) = self.thread_request();
                out.push(write(self.request(method, params, Ours::Thread(method))));
            }
            Ours::Thread(method) => {
                let thread = result["thread"].clone();
                let id = thread["id"].as_str().map(str::to_string);
                let (None, Some(id)) = (error.as_ref(), id) else {
                    let why = error.unwrap_or_else(|| "no thread".into());
                    out.push(Item::Stop {
                        reason: format!("{method}: {why}"),
                    });
                    return;
                };
                self.thread = Some(id);
                // The translation learns the agent's session from it (a resumed thread says none).
                self.translate(
                    &json!({ "method": "thread/started", "params": { "thread": thread } }),
                    out,
                );
                out.push(write(self.request(
                    "model/list",
                    json!({}),
                    Ours::Models(Vec::new(), 0),
                )));
            }
            Ours::Models(mut data, pages) => {
                if error.is_none() {
                    data.extend(result["data"].as_array().cloned().unwrap_or_default());
                    if let Some(cursor) = result["nextCursor"].as_str() {
                        if pages + 1 < MODEL_PAGES {
                            let params = json!({ "cursor": cursor });
                            let ours = Ours::Models(data, pages + 1);
                            out.push(write(self.request("model/list", params, ours)));
                            return;
                        }
                    }
                }
                let models = words::catalog(&json!({ "data": data })).unwrap_or(Value::Null);
                out.push(Item::Ready { models });
                // What came before its thread was open.
                for input in std::mem::take(&mut self.queue) {
                    let line = self.steer(input);
                    out.extend(line.into_iter().map(write));
                }
            }
            Ours::Fired(what) => {
                if let Some(message) = error {
                    if what == "turn/start" {
                        self.starting = false;
                    }
                    // A refusal of theirs is an `error` notification, for the turn it was to start.
                    self.translate(
                        &json!({ "method": "error", "params": {
                            "error": { "message": message, "codexErrorInfo": null },
                            "willRetry": false, "refused": what,
                        } }),
                        out,
                    );
                }
            }
            Ours::Usage => {
                let meters = words::meters(&result["rateLimits"]);
                let usage = if meters.is_empty() {
                    Value::Null
                } else {
                    json!({ "meters": meters })
                };
                out.push(Item::Usage { usage });
            }
            // Refused, no turn will end it.
            Ours::Compact => {
                if let Some(e) = error {
                    out.push(Item::Turn {
                        interrupted: false,
                        limited: None,
                        usage: Default::default(),
                        error: Some(words::said(
                            &self.lang,
                            format!("Codex n’a pas compacté le contexte : {e}"),
                            format!("Codex didn’t compact the context: {e}"),
                        )),
                        entry: None,
                    });
                }
            }
            Ours::Said => {}
        }
    }

    /// The parameters of `thread/start`, `thread/resume` or `thread/fork`.
    fn thread_request(&self) -> (&'static str, Value) {
        let s = &self.settings;
        let mut p = mode_params(s, &self.cwd);
        // Taken at the thread's level: `collaborationMode` is a turn's.
        if let Some(o) = p.as_object_mut() {
            o.remove("collaborationMode");
            o.remove("sandboxPolicy");
        }
        p["cwd"] = json!(self.cwd);
        p["model"] = json!(s.model);
        // Fenced, its mode's profile (`mode_params`), declared by its process's options
        // (`fence_args`); none in « Sans permissions », whose sandbox is none.
        if p.get("permissions").is_none() {
            p["sandbox"] = json!(words::sandbox_mode(s.sandboxed()));
        }
        match &self.start {
            StartAt::New => {
                if let Some(i) = &self.instructions {
                    p["developerInstructions"] = json!(i);
                }
                ("thread/start", p)
            }
            // Its added instructions are in its rollout already (tried: they survive a resume).
            StartAt::Resume { session } => {
                p["threadId"] = json!(session);
                ("thread/resume", p)
            }
            StartAt::Fork { of, at } => {
                p["threadId"] = json!(of);
                if let Some(at) = at {
                    p["lastTurnId"] = json!(at);
                }
                ("thread/fork", p)
            }
        }
    }

    /// Starts a turn with `input`, as the settings of the next turn say.
    fn start_turn(&mut self, input: Value) -> Option<String> {
        let thread = self.thread.clone()?;
        let s = &self.settings;
        let mut p = mode_params(s, &self.cwd);
        p["threadId"] = json!(thread);
        p["input"] = input;
        p["model"] = json!(s.model);
        if !s.effort.is_empty() {
            p["effort"] = json!(s.effort);
        }
        p["summary"] = json!("auto");
        self.starting = true;
        self.active = true;
        Some(self.request("turn/start", p, Ours::Fired("turn/start")))
    }

    /// `input` given to the turn under way (`turn/steer`); held until it has its id when it is just starting; a turn of
    /// its own when none runs.
    fn steer(&mut self, input: Value) -> Option<String> {
        match (self.turn.clone(), self.starting) {
            (Some(turn), _) => {
                let params = json!({
                    "threadId": self.thread, "input": input, "expectedTurnId": turn,
                });
                Some(self.request("turn/steer", params, Ours::Fired("turn/steer")))
            }
            (None, true) => {
                self.held.push(input);
                None
            }
            (None, false) => self.start_turn(input),
        }
    }

    /// A message of the user: `content` as the core made it (its images and text files), else its text alone.
    pub fn send(&mut self, text: &str, content: Option<&Value>) -> Vec<String> {
        let input = content
            .cloned()
            .unwrap_or_else(|| json!([{ "type": "text", "text": text }]));
        if self.thread.is_none() {
            self.queue.push(input);
            return Vec::new();
        }
        self.steer(input).into_iter().collect()
    }

    /// Stops the turn under way, if it has one yet.
    pub fn interrupt(&mut self) -> Vec<String> {
        self.interrupted = true;
        let (Some(thread), Some(turn)) = (self.thread.clone(), self.turn.clone()) else {
            return Vec::new();
        };
        let params = json!({ "threadId": thread, "turnId": turn });
        vec![self.request("turn/interrupt", params, Ours::Said)]
    }

    /// Codex takes the model, the effort and the mode at the next `turn/start`.
    pub fn set_model(&mut self, model: &str) {
        self.settings.model = model.to_string();
    }

    pub fn set_effort(&mut self, effort: &str) {
        self.settings.effort = effort.to_string();
    }

    pub fn set_mode(&mut self, mode: &str) {
        self.settings.mode = mode.to_string();
    }

    /// The answer of a permission's card, or of the plan's.
    pub fn answer_permission(&mut self, a: &AnswerPermission) -> Vec<String> {
        // The plan of the mode Plan, which Codex ends its turn on: approved, its work starts in
        // the mode left for it; sent back, the planning goes on with what the user said.
        if is_plan_approval(&a.request) {
            let text = if a.allow {
                self.settings.mode = if a.always {
                    "acceptEdits".into()
                } else {
                    a.resume.clone().unwrap_or_else(|| "default".into())
                };
                words::said(&self.lang, "Mets le plan en œuvre.", "Implement the plan.")
            } else {
                a.message.clone().unwrap_or_default()
            };
            return self
                .start_turn(json!([{ "type": "text", "text": text }]))
                .into_iter()
                .collect();
        }
        let Some(asked) = self.asked.remove(&a.request) else {
            return Vec::new();
        };
        let result = if asked.method == "item/permissions/requestApproval" {
            if a.allow {
                json!({
                    "permissions": asked.params["permissions"],
                    "scope": if a.always { "session" } else { "turn" },
                })
            } else {
                json!({ "permissions": {} })
            }
        } else {
            json!({ "decision": match (a.allow, a.always) {
                (true, false) => "accept",
                (true, true) => "acceptForSession",
                (false, _) => "decline",
            } })
        };
        let mut lines = vec![respond(&asked.id, result)];
        // Codex hears no reason with a refusal: the agent is told the one given, within its turn
        // (the words of a refusal without one would tell it nothing more).
        if let Some(message) = a.message.as_deref().filter(|m| !a.allow && worded(m)) {
            lines.extend(self.steer(json!([{ "type": "text", "text": message }])));
        }
        lines
    }

    /// `answers` are the card's, by question; the questions are Codex's, with their ids.
    pub fn answer_question(&mut self, request: &str, answers: &Value) -> Vec<String> {
        let Some(asked) = self.asked.remove(request) else {
            return Vec::new();
        };
        let input = json!({ "questions": asked.params["questions"], "answers": answers });
        vec![respond(
            &asked.id,
            json!({ "answers": words::answers(&input) }),
        )]
    }

    /// `account/rateLimits/read`, whose answer comes as `Item::Usage`.
    pub fn read_usage(&mut self) -> Vec<String> {
        vec![self.request("account/rateLimits/read", json!({}), Ours::Usage)]
    }

    /// The thread's title.
    pub fn rename(&mut self, title: &str) -> Vec<String> {
        let Some(thread) = self.thread.clone() else {
            return Vec::new();
        };
        let params = json!({ "threadId": thread, "name": title });
        vec![self.request("thread/name/set", params, Ours::Said)]
    }

    /// `thread/compact/start` (ESC-205): it takes no instruction, and answers at once; the compaction is a turn of its
    /// own (`contextCompaction`).
    pub fn compact(&mut self) -> Option<Vec<String>> {
        let thread = self.thread.clone()?;
        let params = json!({ "threadId": thread });
        Some(vec![self.request(
            "thread/compact/start",
            params,
            Ours::Compact,
        )])
    }
}

/// The answer to the request `id` Codex made.
fn respond(id: &Value, result: Value) -> String {
    json!({ "id": id, "result": result }).to_string()
}
