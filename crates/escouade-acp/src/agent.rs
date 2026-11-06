//! One agent on an ACP harness, as the client sees it: what it asked of the agent and waits for, what the agent asks
//! and waits for (a permission, a question), the session, the turn under way, the calls and their diffs.
//!
//! - **The session.** The launch's first line is `initialize`; its answer opens the session (`session/load` of the one
//!   to resume when the agent can, else `session/new`, Escouade's MCP server given with its token in a header), then
//!   the mode and the model are set. A message sent before the session is open, or while a turn runs (a second
//!   `session/prompt` would cancel it), waits for it. The instructions go before the first message (`preamble`).
//! - **The turn.** Its texts and reasonings gathered and given whole when the next thing begins; its calls under Claude
//!   Code's names (`tools`), their input as it comes, their end with their result (a write's diff); the end of the
//!   prompt ends the turn (`cancelled`: interrupted).
//! - **The permissions** go to the user, but those Escouade's rules answer: a call that names a path of a private
//!   project is refused at once; in « Tout accepter » all is accepted; in « Accepter les modifications » the writes in
//!   the agent's folder; a read-only agent writes and runs nothing.
//! - **The files** the agent reads and writes through the client (`fs/read_text_file`, `fs/write_text_file`): those of a
//!   private project are refused, which is the fence that holds; a read or a write the agent made by itself on a path
//!   while a fence is set is said in the conversation, once (the fence does not hold for it).

use crate::{cursor, gemini, texts, tools, vibe};
use escouade_plugin_protocol::engine::{
    AcpDecl, Attach, Diff, EnvVar, HarnessDecl, Item, Launch, LaunchParams, StartAt, TurnUsage,
};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};

/// A request of the client to the agent, waiting for its answer.
#[derive(Debug, Clone)]
enum Ours {
    Initialize,
    New,
    Load(String),
    Prompt,
    /// Vibe's `session/fork` of a session (a copy of all of it).
    Fork,
    /// A request whose answer ends the turn a plan held (Vibe's mode Plan, played here): interrupted or not.
    EndHeld(bool),
    /// A setting (`session/set_mode`, `session/set_model`…): only an error of it is said.
    Setting(String),
    /// Cursor's `cursor/list_available_models`: the option of each model's effort.
    Models,
    /// The request whose answer tells Gemini's replay of a `session/load` is behind (`Agent::settling`).
    Settled,
}

/// A request of the agent waiting for the user's answer.
#[derive(Debug, Clone)]
struct Asked {
    id: Value,
    method: String,
    params: Value,
}

/// A text or a reasoning being written: who writes it (`""` the agent, else the call of its subagent), its kind, its
/// item's id, what was written, and how much of it went out as `Delta`s.
#[derive(Debug, Clone)]
struct Writing {
    who: String,
    kind: &'static str,
    id: String,
    text: String,
    sent: usize,
}

/// A call of the agent's, as it was shown.
#[derive(Debug, Clone)]
struct Call {
    name: String,
    input: Value,
    kind: String,
    done: bool,
}

/// The agent of an ACP harness.
pub struct Agent {
    /// How the core names it (`Attach::agent`): what the `fs/*` of the host take.
    key: String,
    name: String,
    lang: String,
    version: String,
    acp: AcpDecl,
    /// It speaks Cursor's extensions (`cursor/*`).
    cursor: bool,
    /// It is Mistral Vibe (the extension `vibe`): its tools, its forms, Escouade's mode Plan played here, its cost.
    vibe: bool,
    /// It is Antigravity CLI (the extension `agy`): its client puts its stream in Cursor's form (`Agent::heard`), its
    /// refusals are errors of their own, not its texts.
    agy: bool,
    /// It is Gemini CLI (the extension `gemini`): its calls, its modes told as texts, its plan's file, its turns held for
    /// their questions, its tokens.
    gemini: bool,
    /// The models Gemini's session offers (`models` of its `session/new`).
    gemini_models: Vec<String>,
    /// A plan of Gemini approved: the mode it goes on in is set once Gemini has left its mode plan.
    gemini_resume: bool,
    /// Gemini's `session/load` replays the history after its answer: nothing is sent, and nothing it says counts, until
    /// a request sent after that answer is answered too (a prompt sent at once would take the replay for its turn).
    settling: bool,
    /// Vibe's calls as their `tool_call` told them, in Cursor's form, by their own id (the last `VIBE_CALLS`).
    vibe_calls: HashMap<String, Value>,
    vibe_order: VecDeque<String>,
    /// What Vibe's session had spent when its turn began, and what it has spent so far (`usage_update`).
    spent_base: Option<vibe::Spent>,
    spent_last: vibe::Spent,
    /// The prompt under way was cut by a refusal (Vibe): no plan is asked of it.
    cut: bool,
    /// The code of the error of the last answer to a request of the client (Vibe's rate limit).
    error_code: Option<i64>,
    /// When the agent goes on after its harness's rate limit (ms), for the turn's end.
    limited_at: Option<i64>,
    /// A recorded stream is replayed (`recorded`): its permissions were answered by the trial, never by Escouade's
    /// rules in its place.
    replaying: bool,
    cwd: String,
    mode: String,
    model: String,
    effort: String,
    start: StartAt,
    session: Option<String>,
    /// The session was given to the core (`Item::Start`).
    told: bool,
    escouade: Option<(String, String)>,
    fence: Vec<String>,
    /// The fence as the files through the client are compared to it (`paths::fence_of`).
    fence_keys: Vec<std::path::PathBuf>,
    read_only: bool,
    /// Its account goes by its key (`LaunchParams::by_key`): what a refusal of Cursor's says.
    by_key: bool,
    instructions: Option<String>,
    /// What was said in its own interface, to go before the next message (`LaunchParams::relay`).
    relay: Option<String>,
    next: u64,
    ours: HashMap<u64, Ours>,
    asked: HashMap<String, Asked>,
    seq: u64,
    /// What tells the ids of its items from those of the agent's processes before it (the end of the key of its process,
    /// new at each start): a resumed session numbers its items from 1 again.
    tag: String,
    /// The messages waiting, by their blocks.
    queue: VecDeque<Vec<Value>>,
    busy: bool,
    open: bool,
    can_load: bool,
    /// `session/load` replays the history: what comes meanwhile is the conversation's already.
    loading: bool,
    /// What the session told of its configuration (`configOptions` of `session/new`).
    options: Value,
    /// Cursor's models and their options (`cursor/list_available_models`): the option of each one's effort.
    listed: Value,
    /// The texts and reasonings being written, in the order they began: who (`""` the agent, else the call of its
    /// subagent), the kind, the text.
    writing: Vec<Writing>,
    calls: HashMap<String, Call>,
    /// The sessions of its subagents, and the call of each (Cursor's).
    children: HashMap<String, String>,
    /// What each subagent said last, by its call: its report.
    reports: HashMap<String, String>,
    /// The diff of a write, by its call: its file, before and after.
    diffs: HashMap<String, (String, Option<String>, String)>,
    /// The writes through the client, by their path: before and after.
    written: HashMap<String, (Option<String>, String)>,
    /// The paths read or written through the client.
    through: Vec<String>,
    leak_said: bool,
    /// The agent's text of the turn (Cursor's refusals are its last words).
    turn_text: String,
    error: Option<String>,
    todos: cursor::Todos,
    /// The question's block the agent's text ended on (`questions_in_text`).
    block: Option<String>,
    /// A mode Escouade set by the agent's answer (a plan approved), to tell the core with the next frames.
    told_mode: Option<String>,
    /// What the turn spent, as the answer to its prompt told it.
    usage: TurnUsage,
}

fn line(v: Value) -> String {
    v.to_string()
}

fn reply(id: &Value, result: Value) -> String {
    line(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

fn reply_error(id: &Value, code: i64, message: &str) -> String {
    line(json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }))
}

fn write(line: String) -> Item {
    Item::Write { line }
}

/// The tag of the ids of the items of the process `key` (`Agent::tag`): the last characters of its key.
fn tag_of(key: &str) -> String {
    let end: String = key
        .chars()
        .rev()
        .take_while(char::is_ascii_alphanumeric)
        .take(8)
        .collect();
    if end.is_empty() {
        String::new()
    } else {
        format!("{}-", end.chars().rev().collect::<String>())
    }
}

/// The request of the client's own that holds the plan of a turn in Vibe's mode Plan, played here.
const PLAN: &str = "escouade/plan";

/// The request of the client's own that holds a question the agent's text ended on.
const QUESTION: &str = "escouade/question";

/// The most of Vibe's calls kept for the permissions and results that name them.
const VIBE_CALLS: usize = 200;

/// The time (ms since the epoch).
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

fn notice(text: String, level: &str) -> Item {
    Item::Notice {
        text,
        level: level.into(),
    }
}

impl Agent {
    /// The agent `a` of the harness `decl`, for Escouade `version`.
    pub fn new(a: &Attach, decl: &HarnessDecl, version: &str) -> Self {
        let acp = decl.acp.clone().unwrap_or_default();
        let mut agent = Agent {
            key: a.agent.clone(),
            name: decl.speaker.clone().unwrap_or_else(|| decl.name.clone()),
            lang: String::new(),
            version: version.into(),
            cursor: acp.extensions.iter().any(|e| e == "cursor" || e == "agy"),
            agy: acp.extensions.iter().any(|e| e == "agy"),
            vibe: acp.extensions.iter().any(|e| e == "vibe"),
            gemini: acp.extensions.iter().any(|e| e == "gemini"),
            gemini_models: Vec::new(),
            gemini_resume: false,
            settling: false,
            vibe_calls: HashMap::new(),
            vibe_order: VecDeque::new(),
            spent_base: None,
            spent_last: vibe::Spent::default(),
            cut: false,
            error_code: None,
            limited_at: None,
            replaying: false,
            acp,
            cwd: String::new(),
            mode: String::new(),
            model: String::new(),
            effort: String::new(),
            start: StartAt::New,
            session: None,
            told: false,
            escouade: None,
            fence: Vec::new(),
            fence_keys: Vec::new(),
            read_only: false,
            by_key: false,
            instructions: None,
            relay: None,
            next: 0,
            ours: HashMap::new(),
            asked: HashMap::new(),
            seq: 0,
            tag: tag_of(&a.agent),
            queue: VecDeque::new(),
            busy: false,
            open: false,
            can_load: false,
            loading: false,
            options: Value::Null,
            listed: Value::Null,
            writing: Vec::new(),
            calls: HashMap::new(),
            children: HashMap::new(),
            reports: HashMap::new(),
            diffs: HashMap::new(),
            written: HashMap::new(),
            through: Vec::new(),
            leak_said: false,
            turn_text: String::new(),
            error: None,
            todos: cursor::Todos::default(),
            block: None,
            told_mode: None,
            usage: TurnUsage::default(),
        };
        agent.attach(a);
        agent
    }

    /// What the core tells of the agent, again after a restart.
    pub fn attach(&mut self, a: &Attach) {
        self.cwd = a.cwd.clone();
        self.mode = a.mode.clone();
        self.model = a.model.clone();
        self.effort = a.effort.clone();
        if !a.lang.is_empty() {
            self.lang = a.lang.clone();
        }
        if let Some(s) = &a.session {
            if self.session.is_none() {
                self.start = StartAt::Resume { session: s.clone() };
            }
        }
    }

    /// A recorded stream replayed (`tests/fixtures/cursor/`): the request `msg` its client sent, taken as this
    /// client's own, so that the agent's answer to it reads as it would have.
    pub fn recorded(&mut self, msg: &Value) {
        self.replaying = true;
        let (Some(id), Some(method)) = (msg["id"].as_u64(), msg["method"].as_str()) else {
            return;
        };
        let ours = match method {
            "session/new" => Ours::New,
            "session/load" => {
                self.loading = true;
                Ours::Load(
                    msg["params"]["sessionId"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                )
            }
            "session/fork" => Ours::Fork,
            "session/prompt" => {
                self.busy = true;
                self.cut = false;
                self.loading = false;
                self.turn_text.clear();
                Ours::Prompt
            }
            other => Ours::Setting(other.to_string()),
        };
        self.ours.insert(id, ours);
    }

    /// What the conversation says of a session not opened: Vibe's words for it, else the client's.
    fn session_refused(&self, e: &str) -> String {
        if self.vibe {
            vibe::session_refused(&self.lang, e)
        } else if self.gemini {
            gemini::session_refused(&self.lang, e)
        } else {
            texts::session_refused(&self.lang, &self.name, e)
        }
    }

    /// Whether this agent speaks Cursor's extensions.
    pub fn speaks_cursor(&self) -> bool {
        self.cursor
    }

    fn request(&mut self, method: &str, params: Value, ours: Ours) -> String {
        self.next += 1;
        let id = self.next;
        self.ours.insert(id, ours);
        line(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
    }

    /// How its harness is launched: its arguments and variables as its manifest says them, and `initialize` first.
    /// Refused when it cannot hold what the launch asks (the fence of a private project, read-only).
    pub fn launch(&mut self, p: &LaunchParams) -> Result<Launch, String> {
        if !p.lang.is_empty() {
            self.lang = p.lang.clone();
        }
        self.cwd = p.cwd.clone();
        self.mode = p.mode.clone();
        self.model = p.model.clone();
        self.effort = p.effort.clone();
        self.start = p.start.clone();
        self.escouade = p
            .escouade
            .as_ref()
            .map(|g| (g.url.clone(), g.token.clone()));
        self.fence = p.fence.clone();
        self.fence_keys = crate::paths::fence_of(&self.fence);
        self.read_only = p.read_only;
        let mut instructions = p.append_prompt.clone().unwrap_or_default();
        if self.gemini {
            // Before its first message, whatever the start: a session to resume loaded takes them away (`Ours::Load`), one
            // that cannot be, or a copy (it has none), is a new one.
            if !instructions.trim().is_empty() {
                instructions = format!("{}\n\n", crate::preamble(&instructions));
            }
            instructions.push_str(crate::questions::QUESTIONS);
        } else if self.acp.questions_in_text {
            instructions = format!(
                "{}

{}",
                instructions.trim(),
                crate::questions::QUESTIONS
            );
        }
        self.instructions = Some(instructions)
            .filter(|i| !i.trim().is_empty() && (self.gemini || matches!(p.start, StartAt::New)));
        self.relay = p.relay.clone().filter(|r| !r.trim().is_empty());
        self.by_key = p.by_key;
        if !self.fence.is_empty() && !self.acp.fs && !self.acp.fence_on_permissions {
            return Err(texts::fence_missing(&self.lang, &self.name));
        }
        if self.read_only && self.acp.read_only_mode.is_none() {
            return Err(texts::read_only_missing(&self.lang, &self.name));
        }
        let mut meta = json!({});
        if self.cursor {
            meta["subagents"] = json!(true);
        }
        let mut caps = json!({
            "fs": { "readTextFile": self.acp.fs, "writeTextFile": self.acp.fs },
            "terminal": false,
            "_meta": meta,
        });
        // Vibe's questions are forms: without them its model has no `ask_user_question` (tried).
        if self.vibe {
            caps["elicitation"] = json!({ "form": {} });
        }
        // Gemini is offered what the study tried (ESC-273), nothing else.
        if self.gemini {
            if let Some(c) = caps.as_object_mut() {
                c.remove("_meta");
            }
        }
        let hello = self.request(
            "initialize",
            json!({
                "protocolVersion": crate::PROTOCOL,
                "clientCapabilities": caps,
                "clientInfo": { "name": "escouade", "version": self.version },
            }),
            Ours::Initialize,
        );
        Ok(Launch {
            args: self.acp.args.clone(),
            env: self
                .acp
                .env
                .iter()
                .map(|(k, v)| EnvVar {
                    name: k.clone(),
                    value: v.clone(),
                })
                .collect(),
            remove_env: self.acp.remove_env.clone(),
            fence_by_client: !self.fence.is_empty(),
            read_only: self.read_only,
            writes: vec![hello],
            ..Launch::default()
        })
    }

    /// A harness that is no ACP (Antigravity CLI): its client opened the session `session`.
    pub fn session_opened(&mut self, session: &str) -> Vec<Item> {
        let mut out = Vec::new();
        self.opened(session.to_string(), &Value::Null, &mut out);
        out
    }

    /// The same: a prompt of its client begins.
    pub fn prompt_begun(&mut self) {
        self.busy = true;
        self.cut = false;
        self.turn_text.clear();
    }

    /// The same: a message of its, in ACP's form, heard as the agent's own.
    pub fn heard(&mut self, msg: Value) -> Vec<Item> {
        let mut out = Vec::new();
        if let Some(mode) = self.told_mode.take() {
            out.push(Item::Mode { mode });
        }
        self.read(msg, &mut out);
        out
    }

    /// The same: its text or reasoning is whole.
    pub fn text_whole(&mut self) -> Vec<Item> {
        let mut out = Vec::new();
        self.close("", &mut out);
        out
    }

    /// The same: what its client answered of `request` itself.
    pub fn forget(&mut self, request: &str) {
        self.asked.remove(request);
    }

    /// The same: its prompt ended, with what it spent, why it failed, and when it goes on after a limit (ms).
    pub fn prompt_ended(
        &mut self,
        cancelled: bool,
        usage: TurnUsage,
        error: Option<String>,
        limited_at: Option<i64>,
    ) -> Vec<Item> {
        let mut out = Vec::new();
        self.usage = usage;
        self.error = error;
        self.limited_at = limited_at;
        self.turn_end(cancelled, &mut out);
        out
    }

    /// What the agent wrote: the items for the conversation, and the lines to write back to it (`Item::Write`).
    pub fn frames(&mut self, lines: &[String]) -> Vec<Item> {
        let mut out = Vec::new();
        if let Some(mode) = self.told_mode.take() {
            out.push(Item::Mode { mode });
        }
        for l in lines {
            match serde_json::from_str::<Value>(l) {
                Ok(msg) => self.read(msg, &mut out),
                // What is no JSON (a log line on its output) is left aside.
                Err(_) => continue,
            }
        }
        out
    }

    fn read(&mut self, msg: Value, out: &mut Vec<Item>) {
        let Some(method) = msg["method"].as_str().map(str::to_string) else {
            if let Some(id) = msg["id"].as_u64() {
                if let Some(ours) = self.ours.remove(&id) {
                    self.error_code = msg["error"]["code"].as_i64();
                    let error = msg
                        .get("error")
                        .filter(|e| !e.is_null())
                        .map(|e| e["message"].as_str().unwrap_or("error").to_string());
                    self.answered(ours, msg["result"].clone(), error, out);
                }
            }
            return;
        };
        let params = msg["params"].clone();
        if !msg["id"].is_null() {
            self.asks(&method, msg["id"].clone(), params, out);
        } else if method == "session/update" {
            self.update(&params, out);
        }
    }

    /// The answer to a request of the client.
    fn answered(&mut self, ours: Ours, result: Value, error: Option<String>, out: &mut Vec<Item>) {
        match ours {
            Ours::Initialize => {
                if let Some(e) = error {
                    let why = self.session_refused(&e);
                    return self.refused(why, out);
                }
                if result["protocolVersion"].as_u64() != Some(crate::PROTOCOL) {
                    let v = result["protocolVersion"].to_string();
                    let why = if self.vibe {
                        vibe::protocol_refused(&self.lang, &v)
                    } else if self.gemini {
                        gemini::protocol_refused(&self.lang, &v)
                    } else {
                        texts::protocol_refused(&self.lang, &self.name, &v)
                    };
                    return self.refused(why, out);
                }
                // Gemini is always asked (a session that cannot be loaded gives a new one).
                self.can_load = self.gemini || result["agentCapabilities"]["loadSession"] == true;
                let mcp = crate::mcp_servers(
                    self.escouade
                        .as_ref()
                        .map(|(u, t)| (u.as_str(), t.as_str())),
                );
                let line = match self.start.clone() {
                    StartAt::Resume { session } if self.can_load => {
                        self.loading = true;
                        self.request(
                            "session/load",
                            json!({ "sessionId": session, "cwd": self.cwd, "mcpServers": mcp }),
                            Ours::Load(session),
                        )
                    }
                    // A copy of all of it: Vibe's `session/fork` takes no point (the original is idle).
                    StartAt::Fork { of, .. } if self.vibe => self.request(
                        "session/fork",
                        json!({ "sessionId": of, "cwd": self.cwd, "mcpServers": mcp }),
                        Ours::Fork,
                    ),
                    _ => self.request(
                        "session/new",
                        json!({ "cwd": self.cwd, "mcpServers": mcp }),
                        Ours::New,
                    ),
                };
                out.push(write(line));
                // Cursor's models and the option of their efforts, asked beside the session (it needs none): the
                // effort is set once both are there.
                if self.cursor {
                    let line =
                        self.request("cursor/list_available_models", json!({}), Ours::Models);
                    out.push(write(line));
                }
            }
            Ours::Load(session) => {
                self.loading = false;
                match error {
                    None => {
                        // Its session has them already.
                        self.instructions = None;
                        self.settling = self.gemini;
                        self.opened(session, &result, out)
                    }
                    Some(e) => {
                        // Gone: a new session, said.
                        out.push(notice(
                            texts::not_resumed(&self.lang, &self.name),
                            "warning",
                        ));
                        let _ = e;
                        if !self.gemini {
                            self.instructions = None;
                        }
                        let mcp = crate::mcp_servers(
                            self.escouade
                                .as_ref()
                                .map(|(u, t)| (u.as_str(), t.as_str())),
                        );
                        let line = self.request(
                            "session/new",
                            json!({ "cwd": self.cwd, "mcpServers": mcp }),
                            Ours::New,
                        );
                        out.push(write(line));
                    }
                }
            }
            Ours::New => match (error, result["sessionId"].as_str()) {
                (None, Some(id)) => self.opened(id.to_string(), &result, out),
                (e, _) => {
                    let e = e.unwrap_or_else(|| "no session".into());
                    let why = self.session_refused(&e);
                    self.refused(why, out)
                }
            },
            // Not copied: a new session.
            Ours::Fork => match (error, result["sessionId"].as_str()) {
                (None, Some(id)) => self.opened(id.to_string(), &result, out),
                _ => {
                    let mcp = crate::mcp_servers(
                        self.escouade
                            .as_ref()
                            .map(|(u, t)| (u.as_str(), t.as_str())),
                    );
                    let line = self.request(
                        "session/new",
                        json!({ "cwd": self.cwd, "mcpServers": mcp }),
                        Ours::New,
                    );
                    out.push(write(line));
                }
            },
            Ours::EndHeld(cancelled) => self.close_turn(cancelled, out),
            Ours::Prompt => {
                if let Some(e) = error {
                    // Vibe's own words (an API key refused, a rate limit), as the core's client said them.
                    self.error = Some(match (self.vibe, self.error_code) {
                        (true, Some(vibe::RATE_LIMITED)) => {
                            self.limited_at = Some(now_ms() + vibe::RATE_RESUME_MS);
                            vibe::rate_limited(&self.lang, Some(&e))
                        }
                        (true, _) => e,
                        // Gemini's own words; its usage limit ends the turn « limité », its resume planned.
                        (false, code) if self.gemini => match gemini::limit_of(code, &e) {
                            Some(wait) => {
                                self.limited_at = Some(now_ms() + wait);
                                gemini::rate_limited(&self.lang, &e)
                            }
                            None => e,
                        },
                        _ => texts::not_translated(&self.lang, &self.name, &e),
                    });
                }
                let cancelled = result["stopReason"] == "cancelled";
                if self.gemini {
                    // A turn that asked a question goes on after its answers: its tokens add up.
                    let s = gemini::spent(&result["usage"]);
                    self.usage.input += s.input;
                    self.usage.output += s.output;
                    self.usage.cache += s.cache;
                } else {
                    // What the turn spent, when the agent tells it (`usage` of the answer, unstable in ACP).
                    let n = |k: &str| result["usage"][k].as_u64().unwrap_or(0);
                    self.usage = TurnUsage {
                        input: n("inputTokens"),
                        output: n("outputTokens"),
                        cache: n("cachedReadTokens") + n("cachedWriteTokens"),
                        cost: 0.0,
                    };
                }
                self.turn_end(cancelled, out);
            }
            Ours::Setting(what) => {
                if let Some(e) = error {
                    let text =
                        texts::not_translated(&self.lang, &self.name, &format!("{what}: {e}"));
                    out.push(notice(text, "warning"));
                }
            }
            Ours::Settled => {
                self.loading = false;
                self.open = true;
                if let Some(blocks) = self.queue.pop_front() {
                    let l = self.prompt(blocks);
                    out.push(write(l));
                }
            }
            // Not told, the efforts stay Cursor's own (as when it does not answer).
            Ours::Models => {
                if error.is_none() {
                    self.listed = result;
                    for l in self.effort_lines() {
                        out.push(write(l));
                    }
                }
            }
        }
    }

    /// No session: the messages waiting end in error.
    fn refused(&mut self, why: String, out: &mut Vec<Item>) {
        out.push(notice(why.clone(), "error"));
        self.queue.clear();
        out.push(Item::Turn {
            interrupted: false,
            limited: None,
            usage: TurnUsage::default(),
            error: Some(why),
            entry: None,
        });
    }

    /// The session is open: its mode and model set, the first message waiting sent.
    fn opened(&mut self, session: String, result: &Value, out: &mut Vec<Item>) {
        // The session is the agent's from now on, a turn or not (a copy has its own as soon as it starts).
        out.push(Item::Session {
            session: session.clone(),
        });
        self.session = Some(session);
        self.open = true;
        self.options = result["configOptions"].clone();
        if self.gemini {
            self.gemini_models = gemini::models_of(&result["models"]);
        }
        if !self.queue.is_empty() {
            self.tell(out);
        }
        let current = result["modes"]["currentModeId"]
            .as_str()
            .or(vibe::option_value(&self.options, "mode"))
            .map(str::to_string);
        for l in self.mode_lines(current.as_deref()) {
            out.push(write(l));
        }
        for l in self.model_lines() {
            out.push(write(l));
        }
        if std::mem::take(&mut self.settling) {
            if let Some(mode) = self.acp_mode(&self.mode.clone()) {
                self.open = false;
                self.loading = true;
                let l = self.request(
                    "session/set_mode",
                    json!({ "sessionId": self.session, "modeId": mode }),
                    Ours::Settled,
                );
                return out.push(write(l));
            }
        }
        if let Some(blocks) = self.queue.pop_front() {
            let l = self.prompt(blocks);
            out.push(write(l));
        }
    }

    /// The session given to the core, once.
    fn tell(&mut self, out: &mut Vec<Item>) {
        if !self.told {
            if let Some(s) = &self.session {
                self.told = true;
                out.push(Item::Start {
                    session: Some(s.clone()),
                });
            }
        }
    }

    /// The ACP mode for Escouade's `mode`: read-only's when the agent is, else the manifest's.
    fn acp_mode(&self, mode: &str) -> Option<String> {
        if self.read_only {
            return self.acp.read_only_mode.clone();
        }
        let m = self.acp.modes.get(mode).cloned();
        if self.vibe {
            return m.map(|m| vibe::agent_of(&m, !self.fence.is_empty()));
        }
        // A mode of another harness's, after a handover: Gemini's `default`.
        if self.gemini {
            return m.or_else(|| self.acp.modes.get("default").cloned());
        }
        m
    }

    fn mode_lines(&mut self, current: Option<&str>) -> Vec<String> {
        let Some(session) = self.session.clone() else {
            return Vec::new();
        };
        match self.acp_mode(&self.mode.clone()) {
            Some(m) if current != Some(m.as_str()) => vec![self.request(
                "session/set_mode",
                json!({ "sessionId": session, "modeId": m }),
                Ours::Setting("session/set_mode".into()),
            )],
            _ => Vec::new(),
        }
    }

    /// The value of the model option for `model`: the one listed whose id is it or starts with it and its options
    /// (`model[…]`), else Cursor's own for Auto, else the model.
    fn model_value(&self, key: &str) -> String {
        let model = &self.model;
        let listed = self
            .options
            .as_array()
            .into_iter()
            .flatten()
            .find(|o| o["id"] == key)
            .and_then(|o| o["options"].as_array().cloned())
            .unwrap_or_default();
        listed
            .iter()
            .filter_map(|o| o["value"].as_str())
            .find(|v| *v == model || v.starts_with(&format!("{model}[")))
            .map(str::to_string)
            .unwrap_or_else(|| {
                if self.cursor {
                    cursor::full_id(model)
                } else {
                    model.clone()
                }
            })
    }

    fn model_lines(&mut self) -> Vec<String> {
        let Some(session) = self.session.clone() else {
            return Vec::new();
        };
        if self.model.is_empty() {
            return Vec::new();
        }
        // A model the session of Vibe or Gemini does not offer (another harness's, after a handover) is left to its
        // default.
        let unlisted = (self.vibe
            && !self
                .acp
                .model_option
                .as_deref()
                .is_some_and(|k| vibe::option_values(&self.options, k).contains(&self.model)))
            || (self.gemini && !self.gemini_models.contains(&self.model));
        if unlisted {
            return self.effort_lines();
        }
        let mut lines = match self.acp.model_option.clone() {
            Some(key) => {
                let value = self.model_value(&key);
                vec![self.request(
                    "session/set_config_option",
                    json!({ "sessionId": session, "configId": key, "value": value }),
                    Ours::Setting("session/set_config_option".into()),
                )]
            }
            None => vec![self.request(
                "session/set_model",
                json!({ "sessionId": session, "modelId": self.model }),
                Ours::Setting("session/set_model".into()),
            )],
        };
        lines.extend(self.effort_lines());
        lines
    }

    /// The effort of Cursor's model, by the option its models' list gives it (`cursor/list_available_models`), once the
    /// session is open: nothing while the list is not there, or for Auto (it picks its effort itself).
    fn effort_lines(&mut self) -> Vec<String> {
        // Vibe's thinking (`off`, `high`), by the option its session offers.
        if self.vibe {
            let Some(session) = self.session.clone() else {
                return Vec::new();
            };
            let levels = vibe::option_values(&self.options, "thinking");
            let Some(thinking) = vibe::thinking_of(&self.effort, &levels) else {
                return Vec::new();
            };
            return vec![self.request(
                "session/set_config_option",
                json!({ "sessionId": session, "configId": "thinking", "value": thinking }),
                Ours::Setting("session/set_config_option".into()),
            )];
        }
        let Some(session) = self.session.clone().filter(|_| self.cursor) else {
            return Vec::new();
        };
        if self.effort.is_empty() {
            return Vec::new();
        }
        let Some(key) = cursor::effort_key(&self.listed, &self.model) else {
            return Vec::new();
        };
        vec![self.request(
            "session/set_config_option",
            json!({ "sessionId": session, "configId": key, "value": self.effort }),
            Ours::Setting("session/set_config_option".into()),
        )]
    }

    fn prompt(&mut self, message: Vec<Value>) -> String {
        let session = self.session.clone().unwrap_or_default();
        let mut blocks = Vec::new();
        if let Some(i) = self.instructions.take() {
            // Gemini's are made already (`launch`): its questions' rules stand beside them.
            let text = if self.gemini { i } else { crate::preamble(&i) };
            blocks.push(json!({ "type": "text", "text": text }));
        }
        // Vibe has no mode Plan: each prompt of Escouade's says what it asks.
        if self.vibe && self.mode == "plan" {
            blocks.push(json!({ "type": "text", "text": vibe::PLAN_MODE }));
        }
        self.cut = false;
        if let Some(r) = self.relay.take() {
            blocks.push(json!({ "type": "text", "text": crate::relayed(&r) }));
        }
        blocks.extend(message);
        self.busy = true;
        self.turn_text.clear();
        self.request(
            "session/prompt",
            json!({ "sessionId": session, "prompt": blocks }),
            Ours::Prompt,
        )
    }

    /// A message: sent now, or once the session is open and the turn under way is over.
    pub fn send(&mut self, text: &str) -> Vec<String> {
        self.send_content(text, None)
    }

    /// A message as the core made it for its harness (`content`: ACP's blocks, its images and text files), else its
    /// text alone.
    pub fn send_content(&mut self, text: &str, content: Option<&Value>) -> Vec<String> {
        let blocks = match content.and_then(Value::as_array) {
            Some(b) => b.clone(),
            None => vec![json!({ "type": "text", "text": text })],
        };
        if !self.open || self.busy {
            self.queue.push_back(blocks);
            return Vec::new();
        }
        vec![self.prompt(blocks)]
    }

    /// `session/cancel`; what the agent waits the user for is answered cancelled, as ACP wants it.
    pub fn interrupt(&mut self) -> Vec<String> {
        self.queue.clear();
        let Some(session) = self.session.clone() else {
            return Vec::new();
        };
        // A plan waiting for its answer (Vibe's mode Plan), or a question held at the end of a turn: no prompt runs, the
        // turn ends.
        let held = |a: &Asked| a.method == PLAN || a.method == QUESTION;
        if self.asked.values().any(held) {
            self.asked.retain(|_, a| !held(a));
            return self.end_held(true);
        }
        let mut out: Vec<String> = self
            .asked
            .drain()
            .filter(|(_, a)| a.method == "session/request_permission" || a.method == gemini::PLAN)
            .map(|(_, a)| reply(&a.id, json!({ "outcome": { "outcome": "cancelled" } })))
            .collect();
        out.push(line(json!({
            "jsonrpc": "2.0", "method": "session/cancel", "params": { "sessionId": session },
        })));
        out
    }

    pub fn set_mode(&mut self, mode: &str) -> Vec<String> {
        self.mode = mode.to_string();
        self.mode_lines(None)
    }

    pub fn set_model(&mut self, model: &str) -> Vec<String> {
        self.model = model.to_string();
        self.model_lines()
    }

    pub fn set_effort(&mut self, effort: &str) -> Vec<String> {
        self.effort = effort.to_string();
        self.effort_lines()
    }

    /// The user's answer to the permission `request`.
    pub fn answer_permission(
        &mut self,
        request: &str,
        allow: bool,
        always: bool,
        message: Option<&str>,
        resume: Option<&str>,
    ) -> Result<Vec<String>, String> {
        let asked = self
            .asked
            .remove(request)
            .ok_or_else(|| format!("{} no longer waits for this answer", self.name))?;
        if asked.method == "cursor/create_plan" {
            let (outcome, mode) = cursor::plan_answer(allow, always, message);
            let mut out = vec![reply(&asked.id, json!({ "outcome": outcome }))];
            if let Some(mode) = mode {
                let mode = if always { mode } else { resume.unwrap_or(mode) };
                out.extend(self.set_mode(mode));
                self.told_mode = Some(mode.to_string());
            }
            return Ok(out);
        }
        if asked.method == PLAN {
            return Ok(self.answer_plan(allow, always, message, resume));
        }
        // Gemini's plan, the permission of its `exit_plan_mode`: « Toujours » carries it out with its writes accepted.
        // Gemini goes back to its `default` once it is approved: the mode the plan goes on in, the one it was in before
        // Plan or `autoEdit`, is set once it has (`gemini_told_mode`).
        if asked.method == gemini::PLAN {
            let kind = if allow { "allow_once" } else { "reject_once" };
            let option = crate::option_of(&asked.params, kind);
            let mut out = vec![reply(
                &asked.id,
                json!({ "outcome": { "outcome": "selected", "optionId": option } }),
            )];
            if allow {
                let mode = if always {
                    "acceptEdits"
                } else {
                    resume.unwrap_or("default")
                };
                self.mode = mode.to_string();
                self.gemini_resume = mode != "default";
                self.told_mode = Some(mode.to_string());
            } else {
                self.cut_turn(message, &mut out);
            }
            return Ok(out);
        }
        let kind = match (allow, always) {
            (true, true) => "allow_always",
            (true, false) => "allow_once",
            _ => "reject_once",
        };
        // « Toujours » for this agent: Vibe writes its rule in the agent's own `VIBE_HOME` (`allow_always_permanent`);
        // its other `allow_always` is for the session alone.
        let permanent = self.vibe
            && kind == "allow_always"
            && asked.params["options"]
                .as_array()
                .is_some_and(|o| o.iter().any(|x| x["optionId"] == vibe::PERMANENT));
        let option = if permanent {
            vibe::PERMANENT.to_string()
        } else {
            crate::option_of(&asked.params, kind)
        };
        let mut out = vec![reply(
            &asked.id,
            json!({ "outcome": { "outcome": "selected", "optionId": option } }),
        )];
        // Vibe and Gemini: a refusal cuts the turn; else Vibe tries the command again under other forms, each asking again.
        if (self.vibe || self.gemini) && !allow {
            self.cut_turn(message, &mut out);
        }
        Ok(out)
    }

    /// The turn cut by a refusal (`session/cancel`), its word (if any) sent once it has ended.
    fn cut_turn(&mut self, message: Option<&str>, out: &mut Vec<String>) {
        if let Some(m) = message.filter(|m| vibe::worded(m)) {
            self.queue
                .push_front(vec![json!({ "type": "text", "text": m })]);
        }
        self.cut = true;
        if let Some(session) = self.session.clone() {
            out.push(line(json!({
                "jsonrpc": "2.0", "method": "session/cancel", "params": { "sessionId": session },
            })));
        }
    }

    /// The plan of Vibe's mode Plan answered: approved, it is carried out in the mode chosen, in the same turn; sent
    /// back with a word, the planning goes on with it; without, the turn ends.
    fn answer_plan(
        &mut self,
        allow: bool,
        always: bool,
        message: Option<&str>,
        resume: Option<&str>,
    ) -> Vec<String> {
        let (mode, text) = match (allow, always) {
            (true, true) => (Some("acceptEdits".to_string()), vibe::PLAN_GO.to_string()),
            (true, false) => (
                Some(resume.unwrap_or("default").to_string()),
                vibe::PLAN_GO.to_string(),
            ),
            _ => (None, message.unwrap_or_default().to_string()),
        };
        if mode.is_none() && text.trim().is_empty() {
            return self.end_held(false);
        }
        let mut out = Vec::new();
        if let Some(mode) = mode {
            out.extend(self.set_mode(&mode));
            self.told_mode = Some(mode);
        }
        out.push(self.prompt(vec![json!({ "type": "text", "text": text })]));
        out
    }

    /// Ends the turn a plan held, by a request whose answer closes it (the core is given items with frames alone): the
    /// mode Vibe is in, set again.
    fn end_held(&mut self, cancelled: bool) -> Vec<String> {
        let Some(session) = self.session.clone() else {
            return Vec::new();
        };
        let agent = self
            .acp_mode(&self.mode.clone())
            .unwrap_or_else(|| "ask".into());
        vec![self.request(
            "session/set_mode",
            json!({ "sessionId": session, "modeId": agent }),
            Ours::EndHeld(cancelled),
        )]
    }

    /// The user's answers (by question) to the question `request`.
    pub fn answer_question(
        &mut self,
        request: &str,
        answers: &Value,
    ) -> Result<Vec<String>, String> {
        let asked = self
            .asked
            .remove(request)
            .ok_or_else(|| format!("{} no longer waits for this answer", self.name))?;
        if asked.method == QUESTION {
            let text = crate::questions::answers_message(&asked.params, answers);
            return Ok(vec![
                self.prompt(vec![json!({ "type": "text", "text": text })])
            ]);
        }
        // Vibe's form, by its keys (`q0`…), a choice by its value.
        if asked.method == "elicitation/create" {
            let content = vibe::elicitation_content(&asked.params, answers);
            return Ok(vec![reply(
                &asked.id,
                json!({ "action": "accept", "content": content }),
            )]);
        }
        let outcome = cursor::question_outcome(&asked.params, answers);
        Ok(vec![reply(&asked.id, json!({ "outcome": outcome }))])
    }

    fn rid(&mut self, id: &Value) -> String {
        self.seq += 1;
        format!(
            "acp-{}{}-{}",
            self.tag,
            self.seq,
            id.to_string().trim_matches('"')
        )
    }

    /// A request of the agent to the client.
    fn asks(&mut self, method: &str, id: Value, params: Value, out: &mut Vec<Item>) {
        match method {
            "session/request_permission" => self.permission(id, &params, out),
            "fs/read_text_file" => {
                let path = params["path"].as_str().unwrap_or_default().to_string();
                self.through.push(path.clone());
                let l = match self.file_of(&path) {
                    Err(why) => reply_error(&id, -32000, &why),
                    Ok(real) => match crate::files::read(&self.key, &real) {
                        Ok(text) => {
                            let from = params["line"].as_u64().unwrap_or(1).max(1) as usize - 1;
                            let limit = params["limit"].as_u64().map(|l| l as usize);
                            let content = if from == 0 && limit.is_none() {
                                text
                            } else {
                                let lines = text.split_inclusive('\n').skip(from);
                                match limit {
                                    Some(n) => lines.take(n).collect(),
                                    None => lines.collect(),
                                }
                            };
                            reply(&id, json!({ "content": content }))
                        }
                        Err(e) => reply_error(&id, -32002, &format!("{path}: {e}")),
                    },
                };
                out.push(write(l));
            }
            "fs/write_text_file" => {
                let path = params["path"].as_str().unwrap_or_default().to_string();
                let content = params["content"].as_str().unwrap_or_default().to_string();
                self.through.push(path.clone());
                let file = self.file_of(&path);
                let l = if let Err(why) = &file {
                    reply_error(&id, -32000, why)
                } else if self.read_only {
                    reply_error(&id, -32000, &texts::read_only_file(&path))
                } else {
                    let real = file.unwrap_or_default();
                    let before = crate::plugin::agent_file(&self.key, &real);
                    match crate::files::write(&self.key, &real, &content) {
                        Ok(()) => {
                            self.written.insert(path.clone(), (before, content));
                            if crate::inside(&path, &self.cwd) {
                                out.push(Item::Touched { path: path.clone() });
                            }
                            reply(&id, Value::Null)
                        }
                        Err(e) => reply_error(&id, -32002, &format!("{path}: {e}")),
                    }
                };
                out.push(write(l));
            }
            m if self.cursor && m.starts_with("cursor/") => self.cursor_asks(m, id, params, out),
            // Vibe's question: a form, shown as a question's card.
            "elicitation/create" if self.vibe => {
                self.tell(out);
                let rid = self.rid(&id);
                let call = params["toolCallId"]
                    .as_str()
                    .map(tools::call_id)
                    .unwrap_or_else(|| rid.clone());
                let questions: Vec<Value> = vibe::elicitation_questions(&params)
                    .into_iter()
                    .map(|(_, q)| q)
                    .collect();
                self.asked.insert(
                    rid.clone(),
                    Asked {
                        id,
                        method: "elicitation/create".into(),
                        params,
                    },
                );
                out.push(Item::Question {
                    request: rid,
                    call,
                    questions: Value::Array(questions),
                    parent: None,
                });
            }
            other => out.push(write(reply_error(
                &id,
                -32601,
                &format!("unsupported: {other}"),
            ))),
        }
    }

    /// The real path of a file the agent asks to read or write through the client, the one to open; Err and why when it
    /// is refused: not absolute (ACP asks it), or in a private project (by its components, its links followed:
    /// `paths`).
    fn file_of(&self, path: &str) -> Result<std::path::PathBuf, String> {
        use crate::files::Refused;
        crate::files::real_of(&self.key, path, &self.fence_keys).map_err(|why| match why {
            Refused::NotAbsolute => texts::not_absolute(path),
            Refused::Fenced => texts::fenced_file(path),
            Refused::OutOfReach => texts::out_of_reach(path),
        })
    }

    /// Who an update is of: the agent (`Some(None)`), one of its subagents (its call), or none of them.
    fn owner(&self, session: Option<&str>) -> Option<Option<String>> {
        let session = session?;
        if self.session.as_deref() == Some(session) {
            return Some(None);
        }
        self.children.get(session).map(|c| Some(c.clone()))
    }

    /// A permission the agent asks.
    fn permission(&mut self, id: Value, p: &Value, out: &mut Vec<Item>) {
        // Vibe's names its call alone: the call as its `tool_call` told it, what the permission tells added.
        let vibe_p;
        let p = if self.vibe {
            let mut params = p.clone();
            let call = p["toolCall"]["toolCallId"].as_str().unwrap_or_default();
            if let Some(known) = self.vibe_calls.get(call) {
                let mut tc = known.clone();
                for (k, v) in p["toolCall"].as_object().into_iter().flatten() {
                    if tc.get(k).is_none_or(Value::is_null) {
                        tc[k] = v.clone();
                    }
                }
                params["toolCall"] = tc;
            } else if let Some(n) = vibe::normalized(&p["toolCall"], &self.cwd) {
                params["toolCall"] = n;
            }
            vibe_p = params;
            &vibe_p
        } else if self.gemini {
            let mut params = p.clone();
            if let Some(n) = gemini::normalized(&p["toolCall"], &self.cwd) {
                params["toolCall"] = n;
            }
            vibe_p = params;
            &vibe_p
        } else {
            p
        };
        let tc = &p["toolCall"];
        let kind = self.kind_of(tc);
        let reject = |me: &Self| {
            let option = crate::option_of(p, "reject_once");
            let _ = me;
            reply(
                &id,
                json!({ "outcome": { "outcome": "selected", "optionId": option } }),
            )
        };
        // « Projet privé »: refused at once, never a card the user could accept.
        if crate::fenced(p, &self.fence) {
            out.push(write(reject(self)));
            return;
        }
        // Gemini's plan to approve: its card, whatever comes of the rules below.
        if self.gemini && tc["toolCallId"].as_str().is_some_and(gemini::is_plan_exit) {
            self.tell(out);
            // The plan written is whole: what comes once it is approved is another text.
            self.close("", out);
            // A recorded stream names the file of its trial, not one of this agent's.
            let plan = if self.replaying {
                p["toolCall"]["title"]
                    .as_str()
                    .and_then(gemini::plan_file)
                    .unwrap_or_default()
                    .to_string()
            } else {
                gemini::plan_of_agent(p, &self.key)
            };
            let rid = self.rid(&id);
            self.asked.insert(
                rid.clone(),
                Asked {
                    id,
                    method: gemini::PLAN.into(),
                    params: p.clone(),
                },
            );
            out.push(Item::Permission {
                request: rid.clone(),
                call: rid,
                tool: Some("ExitPlanMode".into()),
                input: Some(json!({ "plan": plan })),
                description: None,
                reason: None,
                diff: None,
                can_always: true,
                parent: None,
                hunks: None,
                always_by_harness: false,
                ends_stream: false,
            });
            return;
        }
        if self.read_only && matches!(kind.as_str(), "edit" | "delete" | "move" | "execute") {
            out.push(write(reject(self)));
            return;
        }
        // Escouade's mode Plan, played for Vibe, changes nothing: a write is refused, the model told so.
        if self.vibe && self.mode == "plan" && matches!(kind.as_str(), "edit" | "delete" | "move") {
            out.push(write(reject(self)));
            return;
        }
        self.tell(out);
        let call = tc["toolCallId"]
            .as_str()
            .map(tools::call_id)
            .unwrap_or_else(|| self.rid(&id));
        let parent = self.owner(p["sessionId"].as_str()).flatten();
        let diff = tools::diff_of(&tc["content"]).or_else(|| self.whole_write(tc));
        if let Some(d) = &diff {
            self.diffs.insert(call.clone(), d.clone());
        }
        let known = self.calls.get(&call).cloned();
        let (mut tool, mut input) = match &known {
            Some(c) => (c.name.clone(), c.input.clone()),
            None => self.tool_of(tc),
        };
        if known.is_none() {
            self.close(parent.as_deref().unwrap_or_default(), out);
            self.calls.insert(
                call.clone(),
                Call {
                    name: tool.clone(),
                    input: input.clone(),
                    kind: kind.clone(),
                    done: false,
                },
            );
            out.push(Item::Tool {
                id: call.clone(),
                name: tool.clone(),
                input: input.clone(),
                parent: parent.clone(),
                unplanned: false,
            });
        }
        let mut paths: Vec<String> = diff.iter().map(|d| d.0.clone()).collect();
        paths.extend(input["file_path"].as_str().map(str::to_string));
        if let Some((path, old, _)) = &diff {
            let named = if old.as_deref().unwrap_or_default().is_empty() {
                "Write"
            } else {
                "Edit"
            };
            input = json!({ "file_path": path });
            if tool != named {
                tool = named.to_string();
                if let Some(c) = self.calls.get_mut(&call) {
                    c.name = tool.clone();
                    c.input = input.clone();
                }
                out.push(Item::ToolInput {
                    id: call.clone(),
                    name: Some(tool.clone()),
                    input: input.clone(),
                });
            }
        }
        let safe = match self.mode.as_str() {
            "bypassPermissions" => true,
            // Vibe's agent `ask` and Gemini's `default` ask for each write: in « Demander » too, their writes in the
            // agent's folder go through (the meaning of their mode, as the core's rules had it).
            "acceptEdits" | "default" if self.mode == "acceptEdits" || self.vibe || self.gemini => {
                matches!(kind.as_str(), "edit" | "delete")
                    && !paths.is_empty()
                    && paths.iter().all(|f| crate::inside(f, &self.cwd))
            }
            _ => false,
        };
        if safe && !self.replaying {
            let option = crate::option_of(p, "allow_once");
            out.push(write(reply(
                &id,
                json!({ "outcome": { "outcome": "selected", "optionId": option } }),
            )));
            return;
        }
        let mut said = Vec::new();
        if paths.iter().any(|f| !crate::inside(f, &self.cwd)) {
            said.push(texts::outside(&self.lang));
        }
        if let Some((server, t)) = cursor::mcp_of(tc, &tool) {
            said.push(texts::mcp_tool(&self.lang, &server, &t));
        }
        let reason = Some(tools::content_text(&tc["content"])).filter(|t| !t.trim().is_empty());
        let rid = self.rid(&id);
        self.asked.insert(
            rid.clone(),
            Asked {
                id,
                method: "session/request_permission".into(),
                params: p.clone(),
            },
        );
        let can_always = p["options"]
            .as_array()
            .is_none_or(|o| o.iter().any(|x| x["kind"] == "allow_always"));
        out.push(Item::Permission {
            request: rid,
            call,
            tool: None,
            input: None,
            description: (!said.is_empty()).then(|| said.join("\n")),
            reason,
            diff: diff.map(|(path, old_text, new_text)| Diff {
                path,
                old_text,
                new_text,
            }),
            can_always,
            parent,
            hunks: None,
            always_by_harness: false,
            ends_stream: false,
        });
    }

    /// The kind of ACP of the call `u`: its own, or the one of the name its manifest gives a tool of the kind `other`.
    fn kind_of(&self, u: &Value) -> String {
        let kind = u["kind"].as_str().unwrap_or_default();
        if !matches!(kind, "" | "other") {
            return kind.to_string();
        }
        match self.named(u) {
            Some(name) => tools::kind_of(&name).to_string(),
            None => kind.to_string(),
        }
    }

    /// The name of Claude Code's its manifest gives the tool of `u` (`acp.tools`).
    fn named(&self, u: &Value) -> Option<String> {
        if self.acp.tools.is_empty() {
            return None;
        }
        let own = u["_meta"]
            .as_object()
            .into_iter()
            .flatten()
            .find_map(|(_, m)| m["toolCall"]["toolName"].as_str())
            .map(str::to_string)
            .or_else(|| {
                u["title"]
                    .as_str()
                    .and_then(|t| t.split(" · ").next())
                    .map(|t| t.trim().to_string())
            })?;
        self.acp.tools.get(&own).cloned()
    }

    /// A write that tells no diff but its whole content (`rawInput.content`, Goose's `write`): its diff against the
    /// file as it is now.
    fn whole_write(&self, u: &Value) -> Option<(String, Option<String>, String)> {
        let (name, input) = self.tool_of(u);
        let content = u["rawInput"]["content"].as_str()?;
        if name != "Write" {
            return None;
        }
        let path = input["file_path"].as_str()?.to_string();
        let before = crate::plugin::agent_file(&self.key, std::path::Path::new(&path));
        Some((path, before, content.to_string()))
    }

    fn tool_of(&self, u: &Value) -> (String, Value) {
        let raw = &u["rawInput"];
        if let Some(name) = self.named(u) {
            let mut v = u.clone();
            v["kind"] = json!(tools::kind_of(&name));
            let (_, input) = tools::tool_of(&v, &self.cwd);
            return (name, input);
        }
        let named = u["title"]
            .as_str()
            .or(raw["tool"].as_str())
            .unwrap_or_default();
        if !named.is_empty() && self.acp.subagent_tools.iter().any(|t| t == named) {
            let mut input = json!({
                "description": raw["description"].as_str().or(u["title"].as_str()),
                "prompt": raw["prompt"], "subagent_type": raw["subagent_type"].as_str().or(raw["agent"].as_str()),
            });
            if let Some(o) = input.as_object_mut() {
                o.retain(|_, v| !v.is_null());
            }
            return ("Task".into(), input);
        }
        // Those of Vibe and Gemini are put in Cursor's form (`vibe::normalized`, `gemini::normalized`).
        if self.cursor || self.vibe || self.gemini {
            if let Some(t) = cursor::tool_of(u, &self.cwd) {
                return t;
            }
        }
        tools::tool_of(u, &self.cwd)
    }

    /// One update of a session (`session/update`).
    fn update(&mut self, p: &Value, out: &mut Vec<Item>) {
        let u = &p["update"];
        let kind = u["sessionUpdate"].as_str().unwrap_or_default();
        // What Vibe's session spent so far, told as it opens too (before its id is known, or while it is loaded):
        // where its first turn's cost starts (ESC-267).
        if self.vibe && kind == "usage_update" {
            let ours = self.session.is_none()
                || self.loading
                || p["sessionId"].as_str() == self.session.as_deref();
            if ours {
                let spent = vibe::Spent::of(u);
                self.spent_base.get_or_insert(spent);
                self.spent_last = spent;
            }
            return;
        }
        if self.loading {
            return;
        }
        if self.vibe {
            match kind {
                "tool_call" => return self.vibe_tool_start(p, out),
                "tool_call_update" => return self.vibe_tool_update(p, out),
                // Its mode is Escouade's to set (`ask` is « Demander » and « Plan » both); the invite it echoes is the
                // conversation's already.
                "current_mode_update" | "user_message_chunk" => return,
                // Told outside a turn too (after `session/load`, a mode set): no turn begins with them.
                "available_commands_update" | "session_info_update" => return,
                _ => {}
            }
        }
        if self.gemini {
            // `session/load` replays its history after its answer too: only what comes while a prompt runs counts.
            if !self.busy {
                return;
            }
            match kind {
                "tool_call" => return self.gemini_tool_start(p, out),
                "tool_call_update" => return self.gemini_tool_update(p, out),
                // The invite is the conversation's already; its context window, the commands of its interface: not shown.
                "user_message_chunk" | "usage_update" | "available_commands_update" => return,
                "agent_message_chunk" => {
                    let text = u["content"]["text"].as_str().unwrap_or_default();
                    if let Some(mode) = gemini::mode_update(text) {
                        return self.gemini_told_mode(mode, out);
                    }
                }
                _ => {}
            }
        }
        if self.cursor && kind == "subagent_spawned" {
            if let (Some(child), Some(call)) = cursor::spawned(u) {
                if let Some(model) = u["_meta"]["cursor"]["model"].as_str() {
                    out.push(Item::Subagent {
                        call: call.clone(),
                        model: model.to_string(),
                    });
                }
                self.children.insert(child, call);
            }
            return;
        }
        let Some(parent) = self.owner(p["sessionId"].as_str()) else {
            return;
        };
        self.tell(out);
        match kind {
            "agent_message_chunk" | "agent_thought_chunk" => {
                let Some(text) = u["content"]["text"].as_str().filter(|t| !t.is_empty()) else {
                    return;
                };
                let k = if kind == "agent_message_chunk" {
                    "text"
                } else {
                    "thinking"
                };
                self.write_piece(parent.unwrap_or_default(), k, text, out);
            }
            "tool_call" => self.tool_start(u, parent, out),
            "tool_call_update" => self.tool_update(u, out),
            "plan" if parent.is_none() => {
                let todos: Vec<Value> = u["entries"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .enumerate()
                    .map(|(i, e)| json!({ "id": i.to_string(), "content": e["content"], "status": e["status"] }))
                    .collect();
                let todos = self.todos.set(&Value::Array(todos), false);
                out.push(Item::Todos { todos });
            }
            "current_mode_update" if parent.is_none() => {
                let acp = u["currentModeId"].as_str().unwrap_or_default();
                let mode = if self.cursor {
                    cursor::mode_of(acp, &self.mode)
                } else {
                    self.acp
                        .modes
                        .iter()
                        .find(|(_, v)| v.as_str() == acp)
                        .map(|(k, _)| k.clone())
                };
                if let Some(mode) = mode.filter(|m| *m != self.mode) {
                    self.mode = mode.clone();
                    out.push(Item::Mode { mode });
                }
            }
            _ => {}
        }
    }

    /// A piece of text or reasoning of `who`: into the one being written, or a new one (the others of `who` closed).
    fn write_piece(&mut self, who: String, kind: &'static str, text: &str, out: &mut Vec<Item>) {
        let at = match self
            .writing
            .iter()
            .position(|w| w.who == who && w.kind == kind)
        {
            Some(at) => at,
            None => {
                self.close(&who, out);
                self.seq += 1;
                self.writing.push(Writing {
                    who,
                    kind,
                    id: format!("acp-text-{}{}", self.tag, self.seq),
                    text: String::new(),
                    sent: 0,
                });
                self.writing.len() - 1
            }
        };
        let hold = self.acp.questions_in_text && kind == "text";
        let w = &mut self.writing[at];
        w.text.push_str(text);
        // What can go out now: all of it, but a question's block (and what could start its tag) held back.
        let mut ready = w.text.len();
        if hold {
            let open = crate::questions::OPEN;
            ready = match w.text.find(open) {
                Some(i) => i,
                None => {
                    let keep = (1..open.len())
                        .rev()
                        .find(|&n| {
                            w.text.len() >= n
                                && w.text.is_char_boundary(w.text.len() - n)
                                && open.starts_with(&w.text[w.text.len() - n..])
                        })
                        .unwrap_or(0);
                    w.text.len() - keep
                }
            };
        }
        if ready > w.sent && w.text.is_char_boundary(ready) {
            let piece = w.text[w.sent..ready].to_string();
            w.sent = ready;
            out.push(Item::Delta {
                id: w.id.clone(),
                text: piece,
                thinking: kind == "thinking",
                parent: (!w.who.is_empty()).then(|| w.who.clone()),
            });
        }
    }

    /// What `who` was writing is whole: given, the agent's text kept for the turn, a subagent's as its report.
    fn close(&mut self, who: &str, out: &mut Vec<Item>) {
        let Some(at) = self.writing.iter().position(|w| w.who == who) else {
            return;
        };
        let Writing {
            who,
            kind,
            mut text,
            id,
            ..
        } = self.writing.remove(at);
        let id = Some(id);
        let parent = (!who.is_empty()).then(|| who.clone());
        if kind == "text" && who.is_empty() && self.acp.questions_in_text {
            let (before, block) = crate::questions::split(&text);
            if block.is_some() {
                self.block = block;
                text = before;
                if text.trim().is_empty() {
                    // Its item, if some of it went out, stays empty: given so, closed.
                    out.push(Item::Text { text, parent, id });
                    return;
                }
            }
        }
        if kind == "text" && !text.trim().is_empty() {
            if who.is_empty() {
                if !self.turn_text.is_empty() {
                    self.turn_text.push_str("\n\n");
                }
                self.turn_text.push_str(&text);
            } else {
                self.reports.insert(who.clone(), text.clone());
            }
        }
        out.push(match kind {
            "text" => Item::Text { text, parent, id },
            _ => Item::Thinking { text, parent, id },
        });
    }

    fn tool_start(&mut self, u: &Value, parent: Option<String>, out: &mut Vec<Item>) {
        let Some(call) = u["toolCallId"].as_str().map(tools::call_id) else {
            return;
        };
        if u["kind"] == "think" || self.calls.contains_key(&call) {
            return;
        }
        if self.cursor && cursor::own_tool(u) {
            return;
        }
        self.close(parent.as_deref().unwrap_or_default(), out);
        let (name, input) = self.tool_of(u);
        self.calls.insert(
            call.clone(),
            Call {
                name: name.clone(),
                input: input.clone(),
                kind: self.kind_of(u),
                done: false,
            },
        );
        out.push(Item::Tool {
            id: call,
            name,
            input,
            parent,
            unplanned: false,
        });
        // A call that ends as it starts (some agents say it whole at once).
        if matches!(u["status"].as_str(), Some("completed" | "failed")) {
            self.tool_update(u, out);
        }
    }

    /// What the agent says of a call as it goes: its file or command once known, then its end.
    fn tool_update(&mut self, u: &Value, out: &mut Vec<Item>) {
        let Some(id) = u["toolCallId"].as_str().map(tools::call_id) else {
            return;
        };
        let Some(call) = self.calls.get(&id).cloned() else {
            return;
        };
        if call.done {
            return;
        }
        let mut input = call.input.clone();
        if u["rawInput"].is_object() || u["locations"].is_array() {
            let kind = u["kind"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| call.kind.clone());
            let kind = if kind.is_empty() {
                tools::kind_of(&call.name).to_string()
            } else {
                kind
            };
            let (_, told) = self.tool_of(&json!({
                "kind": kind, "title": u["title"], "rawInput": u["rawInput"], "locations": u["locations"],
                "toolCallId": u["toolCallId"],
            }));
            let something = told
                .as_object()
                .is_some_and(|o| o.values().any(|v| !v.is_null()));
            if something && told != call.input && !call.name.starts_with("mcp__") {
                input = told;
                if let Some(c) = self.calls.get_mut(&id) {
                    c.input = input.clone();
                }
                out.push(Item::ToolInput {
                    id: id.clone(),
                    name: None,
                    input: input.clone(),
                });
            }
        }
        let status = u["status"].as_str().unwrap_or_default();
        if !matches!(status, "completed" | "failed") {
            return;
        }
        if let Some(c) = self.calls.get_mut(&id) {
            c.done = true;
        }
        let name = call.name.clone();
        let out_raw = &u["rawOutput"];
        let mut failed = status == "failed" || out_raw["exitCode"].as_i64().is_some_and(|c| c != 0);
        if let Some(d) = tools::diff_of(&u["content"]) {
            self.diffs.insert(id.clone(), d);
        }
        let path = input["file_path"].as_str().unwrap_or_default().to_string();
        // A write through the client is its diff, when the agent tells none.
        if !self.diffs.contains_key(&id) && matches!(name.as_str(), "Edit" | "Write") {
            if let Some((old, new)) = self.written.remove(&path) {
                self.diffs.insert(id.clone(), (path.clone(), old, new));
            }
        }
        let unwritten = self.cursor
            && !failed
            && matches!(name.as_str(), "Edit" | "Write")
            && !self.diffs.contains_key(&id);
        let mut text = out_raw["content"]
            .as_str()
            .or(out_raw["error"].as_str())
            .or(out_raw["output"].as_str())
            .or(out_raw.as_str())
            .unwrap_or_default()
            .to_string();
        if text.is_empty() {
            text = tools::content_text(&u["content"]);
        }
        if unwritten {
            failed = true;
            text = cursor::write_not_done(&self.lang);
        }
        let result = match name.as_str() {
            "Bash" => json!({
                "stdout": out_raw["stdout"].as_str().unwrap_or_default(),
                "stderr": out_raw["stderr"].as_str().unwrap_or_default(),
            }),
            "Edit" | "Write" if !failed => match self.diffs.remove(&id) {
                Some((file, old, new)) => {
                    if old.is_none() && name == "Edit" {
                        out.push(Item::ToolInput {
                            id: id.clone(),
                            name: Some("Write".into()),
                            input: input.clone(),
                        });
                    }
                    if crate::inside(&file, &self.cwd) {
                        out.push(Item::Touched { path: file.clone() });
                    }
                    json!({
                        "filePath": file,
                        "structuredPatch": tools::line_hunks(old.as_deref().unwrap_or_default(), &new),
                    })
                }
                None => {
                    if !path.is_empty() && crate::inside(&path, &self.cwd) {
                        out.push(Item::Touched { path: path.clone() });
                    }
                    json!({ "filePath": path, "structuredPatch": [] })
                }
            },
            "Task" => {
                self.close(&id, out);
                if let Some(report) = self.reports.remove(&id) {
                    if text.trim().is_empty() {
                        text = report;
                    }
                }
                Value::Null
            }
            _ => Value::Null,
        };
        // A read or a write made by the agent itself while a fence is set: said once (a fence held on the permissions
        // says so in its modes' meanings instead).
        if !self.fence.is_empty()
            && !self.acp.fence_on_permissions
            && !self.leak_said
            && matches!(call.kind.as_str(), "read" | "edit" | "delete")
        {
            let gone_through = path.is_empty()
                || self
                    .through
                    .iter()
                    .any(|t| crate::names(t, &path) && crate::names(&path, t));
            if !gone_through {
                self.leak_said = true;
                out.push(notice(
                    texts::fence_leak(&self.lang, &self.name, &path),
                    "warning",
                ));
            }
        }
        out.push(Item::ToolDone {
            id,
            output: text,
            error: failed,
            result,
        });
    }

    /// Vibe's call started, put in Cursor's form and kept for its permission and its result.
    fn vibe_tool_start(&mut self, p: &Value, out: &mut Vec<Item>) {
        let u = &p["update"];
        let Some(parent) = self.owner(p["sessionId"].as_str()) else {
            return;
        };
        let (Some(id), Some(n)) = (u["toolCallId"].as_str(), vibe::normalized(u, &self.cwd)) else {
            return;
        };
        self.tell(out);
        if !self.vibe_calls.contains_key(id) {
            self.vibe_order.push_back(id.to_string());
            if self.vibe_order.len() > VIBE_CALLS {
                if let Some(old) = self.vibe_order.pop_front() {
                    self.vibe_calls.remove(&old);
                }
            }
        }
        self.vibe_calls.insert(id.to_string(), n.clone());
        // A call told once it is over (its search of tools, a hook's refusal) ends there too (`tool_start`).
        self.tool_start(&n, parent, out);
    }

    /// What Vibe says of its call as it goes: its name kept from its `tool_call`, a write's diff too (told there only).
    fn vibe_tool_update(&mut self, p: &Value, out: &mut Vec<Item>) {
        let u = &p["update"];
        if self.owner(p["sessionId"].as_str()).is_none() {
            return;
        }
        let Some(id) = u["toolCallId"].as_str() else {
            return;
        };
        let known = self.vibe_calls.get(id).cloned();
        let mut whole = u.clone();
        if whole["_meta"]["tool_name"].is_null() {
            if let Some(k) = &known {
                whole["_meta"] = k["_meta"].clone();
            }
        }
        let Some(mut n) = vibe::normalized(&whole, &self.cwd) else {
            return;
        };
        self.tell(out);
        let done = n["status"] == "completed";
        if done && vibe::diff_in(&n["content"]).is_none() {
            if let Some(d) = known
                .as_ref()
                .and_then(|k| vibe::diff_in(&k["content"]))
                .cloned()
            {
                match n["content"].as_array_mut() {
                    Some(list) => list.insert(0, d),
                    None => n["content"] = json!([d]),
                }
            }
        }
        if done || n["status"] == "failed" {
            self.vibe_calls.remove(id);
            self.vibe_order.retain(|c| c != id);
        }
        self.tool_update(&n, out);
    }

    /// Gemini's change of mode, told as a text: out of its mode plan, the mode a plan approved goes on in is set.
    fn gemini_told_mode(&mut self, mode: &str, out: &mut Vec<Item>) {
        if mode == "default" && std::mem::take(&mut self.gemini_resume) {
            for l in self.mode_lines(None) {
                out.push(write(l));
            }
            return;
        }
        if self.mode != mode && !(mode == "default" && self.mode.is_empty()) {
            self.mode = mode.to_string();
            out.push(Item::Mode { mode: mode.into() });
        }
    }

    /// Gemini's call started, put in Cursor's form; a tool shown otherwise is left out.
    fn gemini_tool_start(&mut self, p: &Value, out: &mut Vec<Item>) {
        let Some(parent) = self.owner(p["sessionId"].as_str()) else {
            return;
        };
        let Some(n) = gemini::normalized(&p["update"], &self.cwd) else {
            return;
        };
        self.tell(out);
        // A call told once it is over (a rule's refusal, a replay) ends there too (`tool_start`).
        self.tool_start(&n, parent, out);
    }

    /// What Gemini says of its call as it goes; a call refused by its rules is told by its end alone.
    fn gemini_tool_update(&mut self, p: &Value, out: &mut Vec<Item>) {
        let Some(parent) = self.owner(p["sessionId"].as_str()) else {
            return;
        };
        let Some(n) = gemini::normalized(&p["update"], &self.cwd) else {
            return;
        };
        self.tell(out);
        let known = n["toolCallId"]
            .as_str()
            .is_some_and(|id| self.calls.contains_key(&tools::call_id(id)));
        if !known {
            self.tool_start(&n, parent, out);
        }
        self.tool_update(&n, out);
    }

    /// The end of the prompt: what was being written given, the turn closed; the next message waiting sent.
    fn turn_end(&mut self, cancelled: bool, out: &mut Vec<Item>) {
        self.tell(out);
        while let Some(w) = self.writing.first().cloned() {
            self.close(&w.who, out);
        }
        // A plan approved whose turn ended before Gemini told it left its mode plan.
        if std::mem::take(&mut self.gemini_resume) {
            for l in self.mode_lines(None) {
                out.push(write(l));
            }
        }
        // Vibe in Escouade's mode Plan: a turn that ended on its words has its plan approved first (the card « Plan à
        // approuver »), the turn held until it is answered.
        let plan = self.turn_text.trim().to_string();
        if self.vibe
            && !cancelled
            && self.mode == "plan"
            && !self.cut
            && self.error.is_none()
            && !plan.is_empty()
        {
            let rid = self.rid(&json!("plan"));
            self.asked.insert(
                rid.clone(),
                Asked {
                    id: Value::Null,
                    method: PLAN.into(),
                    params: json!({ "plan": plan }),
                },
            );
            out.push(Item::Permission {
                request: rid.clone(),
                call: rid,
                tool: Some("ExitPlanMode".into()),
                input: Some(json!({ "plan": plan })),
                description: None,
                reason: None,
                diff: None,
                can_always: true,
                parent: None,
                hunks: None,
                always_by_harness: false,
                ends_stream: false,
            });
            return;
        }
        self.close_turn(cancelled, out);
    }

    /// The turn closed: its question asked if it ended on one, else its end given; the next message waiting sent.
    fn close_turn(&mut self, cancelled: bool, out: &mut Vec<Item>) {
        self.busy = false;
        self.reports.clear();
        // What Vibe's session spent in the turn: what it grew by since the turn began.
        if self.vibe {
            let base = self.spent_base.replace(self.spent_last).unwrap_or_default();
            let d = self.spent_last.since(base);
            self.usage = TurnUsage {
                input: d.prompt.saturating_sub(d.cached),
                output: d.completion,
                cache: d.cached,
                cost: d.cost,
            };
        }
        // A turn that ends on a question: its card, and the turn goes on with the answers (Gemini's held till then: a
        // message waits for them). A block that is no question is shown as it was written.
        if let Some(block) = self.block.take().filter(|_| self.error.is_none()) {
            let rid = self.rid(&json!("question"));
            let params = (!cancelled)
                .then(|| crate::questions::question_params(&rid, &block))
                .flatten();
            if let Some(params) = params {
                let questions = cursor::questions(&params);
                self.busy = self.gemini;
                self.asked.insert(
                    rid.clone(),
                    Asked {
                        id: Value::Null,
                        method: QUESTION.into(),
                        params,
                    },
                );
                out.push(Item::Question {
                    request: rid.clone(),
                    call: rid,
                    questions,
                    parent: None,
                });
                return;
            }
            if !self.cut {
                out.push(Item::Text {
                    text: block,
                    parent: None,
                    id: None,
                });
            }
        }
        let mut error = self.error.take().filter(|_| !cancelled);
        // Vibe's rate limit: the agent goes on a minute later.
        let mut limited = self.limited_at.take().filter(|_| !cancelled).map(Some);
        if self.cursor && !self.agy && !cancelled && error.is_none() {
            if let Some(refusal) = cursor::refusal(&self.turn_text) {
                if cursor::at_limit(refusal) {
                    limited = Some(None);
                }
                error = Some(cursor::refusal_error(
                    &self.lang,
                    refusal,
                    &self.model,
                    self.by_key,
                ));
            }
        }
        out.push(Item::Turn {
            interrupted: cancelled,
            limited,
            usage: std::mem::take(&mut self.usage),
            error,
            entry: None,
        });
        if let Some(blocks) = self.queue.pop_front() {
            let l = self.prompt(blocks);
            out.push(write(l));
        }
    }

    /// A request of Cursor's own (`cursor/*`).
    fn cursor_asks(&mut self, method: &str, id: Value, params: Value, out: &mut Vec<Item>) {
        self.tell(out);
        match method {
            "cursor/ask_question" => {
                let rid = self.rid(&id);
                let call = params["toolCallId"]
                    .as_str()
                    .map(tools::call_id)
                    .unwrap_or_else(|| rid.clone());
                let questions = cursor::questions(&params);
                self.asked.insert(
                    rid.clone(),
                    Asked {
                        id,
                        method: method.into(),
                        params,
                    },
                );
                out.push(Item::Question {
                    request: rid,
                    call,
                    questions,
                    parent: None,
                });
            }
            "cursor/create_plan" => {
                if params["todos"].as_array().is_some_and(|t| !t.is_empty()) {
                    let todos = self.todos.set(&params["todos"], false);
                    out.push(Item::Todos { todos });
                }
                let rid = self.rid(&id);
                let plan = cursor::plan_text(&params);
                self.asked.insert(
                    rid.clone(),
                    Asked {
                        id,
                        method: method.into(),
                        params,
                    },
                );
                out.push(Item::Permission {
                    request: rid.clone(),
                    call: rid,
                    tool: Some("ExitPlanMode".into()),
                    input: Some(json!({ "plan": plan })),
                    description: None,
                    reason: None,
                    diff: None,
                    can_always: true,
                    parent: None,
                    hunks: None,
                    always_by_harness: false,
                    ends_stream: false,
                });
            }
            "cursor/update_todos" => {
                out.push(write(reply(&id, json!({}))));
                let todos = self.todos.set(&params["todos"], params["merge"] == true);
                out.push(Item::Todos { todos });
            }
            "cursor/task" | "cursor/generate_image" => out.push(write(reply(&id, json!({})))),
            other => out.push(write(reply_error(
                &id,
                -32601,
                &format!("unsupported: {other}"),
            ))),
        }
    }
}
