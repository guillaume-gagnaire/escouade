//! The client of `copilot --server --stdio`, over the protocol of its SDK (JSON-RPC 2.0, protocol 3, docs/PROTOCOL.md,
//! « Copilot SDK »; neither the SDK's crate nor ACP, chosen by the user on 2026-10-18). It holds no process: the core
//! launches Copilot from `Agent::launch`, frames its messages (`framing` of the manifest: `Content-Length` headers) and
//! hands them here; `Agent` is a state machine that answers the items of the conversation and the messages to write
//! back. A step that waited for an answer is a value of `Ours`, taken up when the answer comes.
//!
//! It keeps what a card's answer needs: the requests of the session that wait (permissions, questions, forms, the end
//! of a plan) and the requests Copilot makes of the client for the same question or plan (`userInput.request`,
//! `exitPlanMode.request`): each is answered both ways, the first answer wins.
//!
//! Never a token anywhere: `account.getAllUsers` and `account.getCurrentAuth` give it in clear, and this client calls
//! neither; `auth.getStatus` tells how it is signed in, no more.

use crate::translate::{form_questions, Ctx, Seen, Translator};
use crate::words::{self, agent_mode, permission_mode, takes_effort};
use escouade_plugin_protocol::engine::{
    AnswerPermission, Attach, EnvVar, Item, Launch, LaunchParams, StartAt,
};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

/// A request of the session waiting for its answer (its event, by its `requestId`).
#[derive(Debug, Clone)]
struct Asked {
    /// `permission`, `user_input`, `elicitation`, `exit_plan_mode` or `session_limits_exhausted`.
    kind: String,
    data: Value,
}

/// What an answer of Copilot takes up.
enum Ours {
    Connect,
    Auth,
    /// `session.resume` of the session to go on with.
    Resume,
    Events,
    Fork,
    /// `session.resume` of a copy.
    ResumeCopy,
    Create,
    /// The modes, the server refused and the model given to the session, each after the last.
    Permissions,
    AgentMode,
    McpOff,
    SwitchModel,
    Models,
    SessionModels,
    Send,
    Todos,
    /// The end of a plan answered: the mode its work goes on in, told to the core.
    PlanAnswered(Option<String>),
    Usage,
    Compact,
}

/// What the conversation holds, as the translation asks it: the core keeps the conversation, this is the plugin's
/// memory of the running turn.
#[derive(Default)]
struct Own {
    ids: HashSet<String>,
    calls: HashMap<String, Value>,
}

impl Seen for Own {
    fn has(&self, id: &str) -> bool {
        self.ids.contains(id) || self.calls.contains_key(id)
    }
    fn call(&self, id: &str) -> Option<Value> {
        self.calls.get(id).cloned()
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
                Item::PermissionCard { request, .. } | Item::QuestionCard { request, .. } => {
                    self.ids.insert(request.clone());
                }
                Item::Tool {
                    id, name, input, ..
                } => {
                    let call = json!({ "name": name, "input": input, "status": "running" });
                    self.calls.insert(id.clone(), call);
                }
                Item::ToolDone { id, .. } => {
                    if let Some(c) = self.calls.get_mut(id) {
                        c["status"] = json!("ok");
                    }
                }
                Item::Turn { .. } => {
                    self.ids.clear();
                    self.calls.clear();
                }
                _ => {}
            }
        }
    }
}

/// The client of one agent's Copilot.
pub struct Agent {
    cwd: String,
    lang: String,
    model: String,
    effort: String,
    mode: String,
    start: StartAt,
    /// The session the agent means to resume.
    resumed: Option<String>,
    /// Its session, once opened.
    session: Option<String>,
    /// Its session is open and the core was told: a message goes at once.
    open: bool,
    /// The instructions added to its session (`systemMessage`, given again at each resume: Copilot forgets them
    /// otherwise, tried).
    instructions: Option<String>,
    /// Escouade's server is given to it (`--additional-mcp-config`): its entry stays on.
    escouade: bool,
    /// The paths its tools are refused (« Projet privé »), as it names them: a permission it asks that names one is
    /// refused by Escouade at once (`words::fenced`).
    fence: Vec<String>,
    /// A discussion of the Docs tab.
    read_only: bool,
    /// Its account has a folder of its own (`COPILOT_HOME`): not its Principal.
    own_home: bool,
    /// How it is signed in, and how its session was opened: told to the core once it is.
    auth_type: Value,
    how: &'static str,
    /// The answer to `models.list`.
    listed: Option<Value>,
    next: u64,
    ours: HashMap<u64, Ours>,
    /// A message sent, its turn not over (`session.idle` not come): the next one goes in at once.
    busy: bool,
    /// A turn runs, as the core sees it.
    active: bool,
    /// The requests of the session that wait, by their `requestId`.
    asked: HashMap<String, Asked>,
    /// The requests Copilot made of the client for a question or a plan (`userInput.request`, `exitPlanMode.request`),
    /// not answered yet: their id, method and parameters.
    requests: Vec<(Value, String, Value)>,
    /// Tasks being read (`session.todos_changed`): another change meanwhile reads them again.
    reading_todos: bool,
    todos_again: bool,
    /// The messages waiting for the session.
    queue: Vec<Value>,
    told: Translator,
    own: Own,
}

fn write(line: String) -> Item {
    Item::Write { line }
}

impl Agent {
    pub fn new(a: &Attach) -> Self {
        Self {
            cwd: a.cwd.clone(),
            lang: a.lang.clone(),
            model: a.model.clone(),
            effort: a.effort.clone(),
            mode: a.mode.clone(),
            start: StartAt::New,
            resumed: a.session.clone(),
            // Given again after a restart, in a turn or between two: a new process for an agent that runs.
            session: a.session.clone(),
            open: a.session.is_some(),
            instructions: None,
            escouade: false,
            fence: Vec::new(),
            read_only: false,
            own_home: false,
            auth_type: Value::Null,
            how: "new",
            listed: None,
            // Its ids are told apart from those of a process of this plugin before it restarted.
            next: escouade_plugin_protocol::turn::instance_id() << 20,
            ours: HashMap::new(),
            busy: false,
            active: a.session.is_some(),
            asked: HashMap::new(),
            requests: Vec::new(),
            reading_todos: false,
            todos_again: false,
            queue: Vec::new(),
            told: Translator::default(),
            own: Own::default(),
        }
    }

    /// The agent given again: what may have changed.
    pub fn attach(&mut self, a: &Attach) {
        self.model = a.model.clone();
        self.effort = a.effort.clone();
        self.mode = a.mode.clone();
    }

    /// The description of its launch, and the first line it is written.
    pub fn launch(&mut self, p: &LaunchParams) -> Launch {
        self.cwd = p.cwd.clone();
        self.lang = p.lang.clone();
        self.model = p.model.clone();
        self.effort = p.effort.clone();
        self.mode = p.mode.clone();
        self.start = p.start.clone();
        self.resumed = match &p.start {
            StartAt::Resume { session } => Some(session.clone()),
            _ => None,
        };
        self.session = None;
        self.open = false;
        self.active = false;
        self.instructions = p.append_prompt.clone().filter(|i| !i.trim().is_empty());
        self.fence = p.fence.clone();
        self.read_only = p.read_only;
        self.own_home = p.home.as_deref().is_some_and(|h| !h.is_empty());
        let mut args = words::server_args();
        let mut env = Vec::new();
        match (&p.escouade, &p.escouade_file) {
            // Its entry's file, the token in the environment.
            (Some(grant), Some(file)) => {
                self.escouade = true;
                args.extend(["--additional-mcp-config".into(), format!("@{file}")]);
                env.push(EnvVar {
                    name: words::TOKEN_VAR.into(),
                    value: grant.token.clone(),
                });
            }
            // Refused: `--disable-mcp-server` does nothing in the server mode (tried), an entry of the user's of its
            // name is also turned off for its session (`session.mcp.disable`).
            _ => {
                self.escouade = false;
                args.extend(["--disable-mcp-server".into(), words::SERVER.into()]);
            }
        }
        // An account of its own signs in by itself (`copilot login`): a token of the app's environment would make it
        // another one, the same for every account.
        let remove_env = if self.own_home {
            words::INHERITED_TOKENS.map(str::to_string).to_vec()
        } else {
            Vec::new()
        };
        let connect = self.request(
            "connect",
            json!({ "supportedTaskKinds": ["agent", "client", "shell"] }),
            Ours::Connect,
        );
        Launch {
            args,
            env,
            remove_env,
            read_only: p.read_only,
            // Refused on its permissions: a guard, as its modes' meanings say.
            fence_by_client: true,
            writes: vec![connect],
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
        json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string()
    }

    /// A request whose answer is not waited for.
    fn tell_copilot(&mut self, method: &str, params: Value) -> String {
        let id = self.id();
        json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string()
    }

    /// What Copilot wrote: the items for the conversation, and the messages to write back (`Item::Write`).
    pub fn frames(&mut self, lines: &[String]) -> Vec<Item> {
        let mut out = Vec::new();
        for l in lines {
            if let Ok(msg) = serde_json::from_str::<Value>(l) {
                self.read(msg, &mut out);
            }
        }
        out
    }

    /// One message of Copilot: an answer to a request of the client, a request of Copilot, or an event of a session.
    fn read(&mut self, msg: Value, out: &mut Vec<Item>) {
        let Some(method) = msg["method"].as_str().map(str::to_string) else {
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
        if !msg["id"].is_null() {
            // A request of Copilot to the client.
            match method.as_str() {
                "userInput.request" | "exitPlanMode.request" => {
                    self.requests
                        .push((msg["id"].clone(), method, msg["params"].clone()));
                }
                other => {
                    let refused = json!({
                        "jsonrpc": "2.0", "id": msg["id"],
                        "error": { "code": -32601, "message": format!("unsupported: {other}") },
                    });
                    out.push(write(refused.to_string()));
                }
            }
            return;
        }
        if method != "session.event" {
            return;
        }
        let p = &msg["params"];
        if self.session.is_none() || p["sessionId"].as_str() != self.session.as_deref() {
            return;
        }
        let event = &p["event"];
        let data = &event["data"];
        match event["type"].as_str().unwrap_or_default() {
            "session.idle" if event["agentId"].is_null() => self.busy = false,
            // « Projet privé »: refused at once, never a card the user could accept.
            "permission.requested" if words::fenced(&data["permissionRequest"], &self.fence) => {
                if let Some(rid) = data["requestId"].as_str() {
                    let refusal =
                        json!({ "kind": "reject", "feedback": words::fence_refusal(&self.lang) });
                    let line = self.answer_pending(
                        "session.permissions.handlePendingPermissionRequest",
                        rid,
                        "result",
                        refusal,
                    );
                    out.push(write(line));
                }
                return;
            }
            kind @ ("permission.requested"
            | "user_input.requested"
            | "elicitation.requested"
            | "exit_plan_mode.requested"
            | "session_limits_exhausted.requested") => {
                if let Some(rid) = data["requestId"].as_str() {
                    let kind = kind.trim_end_matches(".requested").to_string();
                    let data = data.clone();
                    self.asked.insert(rid.to_string(), Asked { kind, data });
                }
            }
            "permission.completed"
            | "user_input.completed"
            | "elicitation.completed"
            | "exit_plan_mode.completed"
            | "session_limits_exhausted.completed" => {
                if let Some(rid) = data["requestId"].as_str() {
                    self.asked.remove(rid);
                }
            }
            "session.todos_changed" => self.read_todos(out),
            _ => {}
        }
        self.tell(&msg, out);
    }

    /// `f` through the translation, what its rules accepted answered.
    fn tell(&mut self, f: &Value, out: &mut Vec<Item>) {
        let from = out.len();
        let c = Ctx {
            cwd: &self.cwd,
            mode: &self.mode,
            active: self.active,
            discussion: self.read_only,
            process: true,
            lang: &self.lang,
            resumed: self.resumed.as_deref(),
            seen: &self.own,
        };
        self.told.frame(f, &c, out);
        self.own.note(&out[from..]);
        for item in &out[from..] {
            match item {
                Item::Start { .. } => self.active = true,
                Item::Turn { .. } => self.active = false,
                _ => {}
            }
        }
        for rid in std::mem::take(&mut self.told.approved) {
            let line = self.answer_pending(
                "session.permissions.handlePendingPermissionRequest",
                &rid,
                "result",
                json!({ "kind": "approve-once" }),
            );
            out.push(write(line));
        }
    }

    /// The tasks of the session (`session.plan.readSqlTodosWithDependencies`), told once read.
    fn read_todos(&mut self, out: &mut Vec<Item>) {
        if self.reading_todos {
            self.todos_again = true;
            return;
        }
        let Some(session) = self.session.clone() else {
            return;
        };
        self.reading_todos = true;
        let line = self.request(
            "session.plan.readSqlTodosWithDependencies",
            json!({ "sessionId": session }),
            Ours::Todos,
        );
        out.push(write(line));
    }

    /// Stops here: its process is of no use, and its end says why in the conversation.
    fn refuse(&mut self, reason: String, out: &mut Vec<Item>) {
        out.push(Item::Stop { reason });
    }

    /// What `session.create` and `session.resume` are given.
    fn session_params(&self) -> Value {
        let mut p = json!({
            "clientName": "escouade",
            "workingDirectory": self.cwd,
            "streaming": true,
            "requestPermission": true,
            "requestUserInput": true,
            "requestElicitation": true,
            "requestExitPlanMode": true,
            "envValueMode": "direct",
        });
        // Never `auto`: it picks a model itself (docs/TODO-1.8.md, point 0).
        if !self.model.is_empty() {
            p["model"] = json!(self.model);
        }
        if !self.effort.is_empty() && takes_effort(&self.model) {
            p["reasoningEffort"] = json!(self.effort);
        }
        if let Some(i) = &self.instructions {
            p["systemMessage"] = json!({ "mode": "append", "content": i });
        }
        p
    }

    fn resume_params(&self, id: &str) -> Value {
        let mut p = self.session_params();
        p["sessionId"] = json!(id);
        p
    }

    fn create(&mut self, out: &mut Vec<Item>) {
        let line = self.request("session.create", self.session_params(), Ours::Create);
        out.push(write(line));
    }

    /// Opens the agent's session as `start` says. A session to resume that is gone gives a new one.
    fn open(&mut self, out: &mut Vec<Item>) {
        match self.start.clone() {
            StartAt::New => self.create(out),
            StartAt::Resume { session } => {
                let line =
                    self.request("session.resume", self.resume_params(&session), Ours::Resume);
                out.push(write(line));
            }
            StartAt::Fork { of, at: Some(_) } => {
                let line = self.request(
                    "sessions.readPersistedEvents",
                    json!({ "sessionId": of }),
                    Ours::Events,
                );
                out.push(write(line));
            }
            StartAt::Fork { of, at: None } => self.fork(&of, None, out),
        }
    }

    fn fork(&mut self, of: &str, boundary: Option<String>, out: &mut Vec<Item>) {
        let mut fork = json!({ "sessionId": of, "name": "Escouade" });
        if let Some(b) = boundary {
            fork["toEventId"] = json!(b);
        }
        let line = self.request("sessions.fork", fork, Ours::Fork);
        out.push(write(line));
    }

    /// Its session is opened (`how`): its modes first.
    fn opened(&mut self, id: String, how: &'static str, out: &mut Vec<Item>) {
        self.session = Some(id.clone());
        self.how = how;
        let line = self.request(
            "session.permissions.setMode",
            json!({ "sessionId": id, "mode": permission_mode(&self.mode) }),
            Ours::Permissions,
        );
        out.push(write(line));
    }

    /// What follows its modes: Escouade's server refused, an entry of the user's of its name is off for the session.
    fn after_modes(&mut self, out: &mut Vec<Item>) {
        if !self.escouade {
            let off = json!({ "sessionId": self.session, "serverName": words::SERVER });
            let line = self.request("session.mcp.disable", off, Ours::McpOff);
            out.push(write(line));
            return;
        }
        self.after_mcp(out);
    }

    /// A resumed session keeps its model; the agent's is the one it runs on.
    fn after_mcp(&mut self, out: &mut Vec<Item>) {
        if self.how != "new" && !self.model.is_empty() {
            let line = self.request(
                "session.model.switchTo",
                self.switch_params(&self.model.clone()),
                Ours::SwitchModel,
            );
            out.push(write(line));
            return;
        }
        self.after_model(out);
    }

    /// The core told of its session, then the models of the account.
    fn after_model(&mut self, out: &mut Vec<Item>) {
        let opened = json!({ "method": "escouade.session", "params": {
            "sessionId": self.session, "start": self.how, "authType": self.auth_type,
        } });
        self.tell(&opened, out);
        let line = self.request("models.list", json!({}), Ours::Models);
        out.push(write(line));
    }

    /// All is known: the agent is ready, and what waited for it goes.
    fn ready(&mut self, of_session: Option<&Value>, out: &mut Vec<Item>) {
        let models = words::catalog(self.listed.as_ref(), of_session);
        out.push(Item::Ready {
            models: Value::Array(models),
        });
        self.open = true;
        for message in std::mem::take(&mut self.queue) {
            let line = self.sent(message);
            out.push(write(line));
        }
    }

    fn switch_params(&self, model: &str) -> Value {
        let mut p = json!({ "sessionId": self.session, "modelId": model });
        if !self.effort.is_empty() && takes_effort(model) {
            p["reasoningEffort"] = json!(self.effort);
        }
        p
    }

    /// The answer to a request of the client.
    fn answered(&mut self, ours: Ours, r: &Value, error: Option<String>, out: &mut Vec<Item>) {
        let opened_as = |r: &Value, id: &str| r["sessionId"].as_str().unwrap_or(id).to_string();
        match ours {
            Ours::Connect => {
                if let Some(e) = error {
                    return self.refuse(words::session_refused(&self.lang, &e), out);
                }
                if r["protocolVersion"].as_u64() != Some(words::PROTOCOL) {
                    let why = words::protocol_refused(&self.lang, &r["protocolVersion"]);
                    return self.refuse(why, out);
                }
                // Never a token: `auth.getStatus` tells how it is signed in, no more.
                let line = self.request("auth.getStatus", json!({}), Ours::Auth);
                out.push(write(line));
            }
            Ours::Auth => {
                if let Some(e) = error {
                    return self.refuse(words::session_refused(&self.lang, &e), out);
                }
                if !words::signed_in(r, self.own_home) {
                    return self.refuse(words::not_signed_in(&self.lang), out);
                }
                self.auth_type = r["authType"].clone();
                self.open(out);
            }
            Ours::Resume => match error {
                None => {
                    let asked = self.resumed.clone().unwrap_or_default();
                    self.opened(opened_as(r, &asked), "resume", out);
                }
                // Gone: a new one, which the conversation is told.
                Some(_) => self.create(out),
            },
            Ours::Events => {
                let at = match &self.start {
                    StartAt::Fork { at, .. } => at.clone(),
                    _ => None,
                };
                let of = match &self.start {
                    StartAt::Fork { of, .. } => of.clone(),
                    _ => String::new(),
                };
                // Where a copy stops: the event after `at` (`toEventId` keeps what is before), none when it is its last
                // one or is not found (all of it copied).
                let boundary = r["events"].as_array().and_then(|list| {
                    let i = list.iter().position(|e| e["id"] == json!(at))?;
                    list.get(i + 1)?["id"].as_str().map(str::to_string)
                });
                self.fork(&of, boundary, out);
            }
            Ours::Fork => match (error, r["sessionId"].as_str()) {
                (None, Some(id)) => {
                    let line =
                        self.request("session.resume", self.resume_params(id), Ours::ResumeCopy);
                    self.resumed = Some(id.to_string());
                    out.push(write(line));
                }
                // Not copied: a new one.
                _ => self.create(out),
            },
            Ours::ResumeCopy => {
                if let Some(e) = error {
                    return self.refuse(words::session_refused(&self.lang, &e), out);
                }
                let asked = self.resumed.take().unwrap_or_default();
                self.opened(opened_as(r, &asked), "fork", out);
            }
            Ours::Create => match (error, r["sessionId"].as_str()) {
                (None, Some(id)) => self.opened(id.to_string(), "new", out),
                (Some(e), _) => self.refuse(words::session_refused(&self.lang, &e), out),
                _ => {
                    let why = words::session_refused(&self.lang, "session.create: no session");
                    self.refuse(why, out);
                }
            },
            Ours::Permissions => {
                let line = self.request(
                    "session.mode.set",
                    json!({ "sessionId": self.session, "mode": agent_mode(&self.mode) }),
                    Ours::AgentMode,
                );
                out.push(write(line));
            }
            Ours::AgentMode => self.after_modes(out),
            Ours::McpOff => self.after_mcp(out),
            Ours::SwitchModel => self.after_model(out),
            Ours::Models => {
                if error.is_none() {
                    self.told.set_prices(words::prices(r));
                    self.listed = Some(r.clone());
                }
                let line = self.request(
                    "session.model.list",
                    json!({ "sessionId": self.session }),
                    Ours::SessionModels,
                );
                out.push(write(line));
            }
            Ours::SessionModels => {
                let of_session = error.is_none().then_some(r);
                self.ready(of_session, out);
            }
            Ours::Send => {
                if let Some(e) = error {
                    self.busy = false;
                    let refused =
                        json!({ "method": "escouade.refused", "params": { "message": e } });
                    self.tell(&refused, out);
                }
            }
            Ours::Todos => {
                if error.is_none() {
                    let todos = json!({ "method": "escouade.todos", "params": { "todos": r } });
                    self.tell(&todos, out);
                }
                self.reading_todos = false;
                if std::mem::take(&mut self.todos_again) {
                    self.read_todos(out);
                }
            }
            Ours::PlanAnswered(mode) => {
                if let Some(mode) = mode {
                    out.push(Item::Mode { mode });
                }
            }
            Ours::Usage => {
                let now = chrono::Utc::now().timestamp_millis();
                let meter = error
                    .is_none()
                    .then(|| words::credits(&r["snapshots"], now))
                    .flatten();
                out.push(Item::Usage {
                    usage: meter.map_or(Value::Null, |m| json!({ "meters": [m.meter()] })),
                });
            }
            Ours::Compact => {
                let error = error.or_else(|| {
                    (r["success"] == false)
                        .then(|| r["error"].as_str().unwrap_or("failed").to_string())
                });
                out.push(Item::Compacted { error });
            }
        }
    }

    /// `message` sent: while a turn runs it goes into it at once (`immediate`); else it starts one.
    fn sent(&mut self, message: Value) -> String {
        let mut params = message;
        params["sessionId"] = json!(self.session);
        params["mode"] = json!(if self.busy { "immediate" } else { "enqueue" });
        self.busy = true;
        self.active = true;
        self.request("session.send", params, Ours::Send)
    }

    /// A message (`content`, as the core made it: its text and its files), or waiting for the session.
    pub fn send(&mut self, text: &str, content: Option<&Value>) -> Vec<String> {
        let message = content
            .cloned()
            .unwrap_or_else(|| json!({ "prompt": text }));
        if !self.open {
            self.active = true;
            self.queue.push(message);
            return Vec::new();
        }
        vec![self.sent(message)]
    }

    pub fn interrupt(&mut self) -> Vec<String> {
        let Some(session) = self.session.clone() else {
            return Vec::new();
        };
        vec![self.tell_copilot("session.abort", json!({ "sessionId": session }))]
    }

    /// At once, even during a turn (`session.model.switchTo`).
    pub fn set_model(&mut self, model: &str) -> Vec<String> {
        self.model = model.to_string();
        self.switch()
    }

    pub fn set_effort(&mut self, effort: &str) -> Vec<String> {
        self.effort = effort.to_string();
        self.switch()
    }

    fn switch(&mut self) -> Vec<String> {
        if self.session.is_none() {
            return Vec::new();
        }
        let p = self.switch_params(&self.model.clone());
        vec![self.tell_copilot("session.model.switchTo", p)]
    }

    pub fn set_mode(&mut self, mode: &str) -> Vec<String> {
        self.mode = mode.to_string();
        let Some(session) = self.session.clone() else {
            return Vec::new();
        };
        vec![
            self.tell_copilot(
                "session.permissions.setMode",
                json!({ "sessionId": session, "mode": permission_mode(mode) }),
            ),
            self.tell_copilot(
                "session.mode.set",
                json!({ "sessionId": session, "mode": agent_mode(mode) }),
            ),
        ]
    }

    /// The quota of the month, asked of Copilot (`session.quota.get`); `Item::Usage` brings it.
    pub fn read_usage(&mut self) -> Vec<String> {
        let Some(session) = self.session.clone() else {
            return Vec::new();
        };
        let line = self.request(
            "session.quota.get",
            json!({ "sessionId": session }),
            Ours::Usage,
        );
        vec![line]
    }

    /// `session.name.set`.
    pub fn rename(&mut self, title: &str) -> Vec<String> {
        let Some(session) = self.session.clone() else {
            return Vec::new();
        };
        vec![self.tell_copilot(
            "session.name.set",
            json!({ "sessionId": session, "name": title }),
        )]
    }

    /// `session.history.compact` (ESC-205), with what to keep (`customInstructions`): it answers once the compaction is
    /// over (8 s in full-turn.jsonl), told meanwhile by `session.compaction_start` and `_complete`.
    pub fn compact(&mut self, instructions: &str) -> Vec<String> {
        let mut params = json!({ "sessionId": self.session, "trigger": "manual" });
        if !instructions.is_empty() {
            params["customInstructions"] = json!(instructions);
        }
        vec![self.request("session.history.compact", params, Ours::Compact)]
    }

    /// Replies to the request Copilot made for what the event asked too (the same question, the same plan), if one is
    /// there: `result` is its answer.
    fn reply_request(
        &mut self,
        method: &str,
        matches: impl Fn(&Value) -> bool,
        result: Value,
    ) -> Option<String> {
        let at = self
            .requests
            .iter()
            .position(|(_, m, p)| m == method && matches(p))?;
        let (id, _, _) = self.requests.remove(at);
        Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string())
    }

    /// The answer to a request of the session: by its method, a result its handler takes. Its answer (`success: false`
    /// once the other way answered first) is not waited for.
    fn answer_pending(&mut self, method: &str, rid: &str, key: &str, answer: Value) -> String {
        let mut params = json!({ "sessionId": self.session, "requestId": rid });
        params[key] = answer;
        self.tell_copilot(method, params)
    }

    pub fn answer_permission(&mut self, a: &AnswerPermission) -> Vec<String> {
        // A plugin started again has lost what was asked: what the core still shows is a permission.
        let asked = self.asked.get(&a.request).cloned().unwrap_or(Asked {
            kind: "permission".into(),
            data: Value::Null,
        });
        if asked.kind == "exit_plan_mode" {
            return self.answer_plan(a, &asked);
        }
        let result = if a.allow && a.always {
            words::session_approval(&asked.data)
        } else if a.allow {
            json!({ "kind": "approve-once" })
        } else {
            let message = a.message.as_deref().unwrap_or_default();
            let bare = [
                "L'utilisateur a refusé cette action.",
                "The user refused this action.",
            ];
            if message.trim().is_empty() || bare.contains(&message) {
                json!({ "kind": "reject" })
            } else {
                json!({ "kind": "reject", "feedback": message })
            }
        };
        vec![self.answer_pending(
            "session.permissions.handlePendingPermissionRequest",
            &a.request,
            "result",
            result,
        )]
    }

    /// The plan approved: its work starts, its edits accepted with « Pour cette session »; sent back, the planning goes
    /// on with what the user said.
    fn answer_plan(&mut self, a: &AnswerPermission, asked: &Asked) -> Vec<String> {
        let (response, mode) = if a.allow && a.always {
            (
                json!({ "approved": true, "selectedAction": "interactive", "autoApproveEdits": true }),
                Some("acceptEdits".to_string()),
            )
        } else if a.allow {
            // Its agent in the mode it resumes in: `autopilot` for « Auto » (ESC-277).
            let mode = a.resume.clone().unwrap_or_else(|| "default".into());
            (
                json!({ "approved": true, "selectedAction": agent_mode(&mode) }),
                Some(mode),
            )
        } else {
            (json!({ "approved": false, "feedback": a.message }), None)
        };
        let mut params = json!({ "sessionId": self.session, "requestId": a.request });
        params["response"] = response.clone();
        let mut lines = vec![self.request(
            "session.ui.handlePendingExitPlanMode",
            params,
            Ours::PlanAnswered(mode.clone()),
        )];
        let summary = asked.data["summary"].clone();
        lines.extend(self.reply_request(
            "exitPlanMode.request",
            |p| p["summary"] == summary,
            response,
        ));
        if let Some(mode) = mode {
            self.mode = mode.clone();
            // Without permissions, Copilot is told so too (its answer unread).
            if permission_mode(&mode) != "manual" {
                let params = json!({ "sessionId": self.session, "mode": permission_mode(&mode) });
                lines.push(self.tell_copilot("session.permissions.setMode", params));
            }
        }
        lines
    }

    /// `answers` are the card's, by question.
    pub fn answer_question(&mut self, request: &str, answers: &Value) -> Vec<String> {
        let Some(asked) = self.asked.get(request).cloned() else {
            return Vec::new();
        };
        if asked.kind == "session_limits_exhausted" {
            let text = words::limit_question(&self.lang, &asked.data);
            let answer = answers[text.as_str()].as_str().unwrap_or_default();
            return vec![self.answer_pending(
                "session.ui.handlePendingSessionLimitsExhausted",
                request,
                "response",
                words::limit_answer(&asked.data, answer),
            )];
        }
        if asked.kind == "elicitation" {
            let schema = &asked.data["requestedSchema"];
            let input = json!({
                "questions": form_questions(schema, &self.lang), "answers": answers,
            });
            let content = words::form_content(schema, &input);
            return vec![self.answer_pending(
                "session.ui.handlePendingElicitation",
                request,
                "result",
                json!({ "action": "accept", "content": content }),
            )];
        }
        let question = asked.data["question"].as_str().unwrap_or_default();
        let answer = answers[question]
            .as_str()
            .map(str::to_string)
            .unwrap_or_default();
        let freeform = !asked.data["choices"]
            .as_array()
            .is_some_and(|c| c.iter().any(|x| x.as_str() == Some(&answer)));
        let response = json!({ "answer": answer, "wasFreeform": freeform });
        let mut lines = vec![self.answer_pending(
            "session.ui.handlePendingUserInput",
            request,
            "response",
            response.clone(),
        )];
        let q = asked.data["question"].clone();
        lines.extend(self.reply_request("userInput.request", |p| p["question"] == q, response));
        lines
    }
}
