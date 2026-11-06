//! The client of `agy -p= --input-format stream-json --output-format stream-json`, one message a line on its input, one
//! event a line on its output (docs/PROTOCOL.md, « Antigravity CLI »): a state machine without a process, the core runs
//! the program.
//!
//! `agy` takes its model, effort and mode at its launch only, and has no message that stops a turn: its program is started
//! again on the same conversation (`--conversation <id>`) when one of them changes (before the next message) and when a turn
//! is stopped (`Item::Pause`, then `Item::Relaunch` at the next message). Its program may thus be several in a row; what one
//! wrote and is read after it was stopped is left aside, up to the `init` of the next.
//!
//! In `-p`, `agy` asks nothing: what needs a permission is refused at once (its step ends in `ERROR`, its `result` lists
//! `denied_actions`) and its questions (`ask_question`) are skipped. At the end of such a turn, the client asks the user,
//! writes the rules allowed in the agent's own `settings.json` (`Item::File`: `agy` reads them again before each tool,
//! tried) and has the turn go on with a message saying what was allowed and answered: one turn of Escouade for both.
//!
//! What its events leave out (a call's arguments, a write's text, its reasoning) is read from the conversation's transcript
//! (`brain/<id>/.system_generated/logs/transcript_full.jsonl`), written before the event that tells the step over: read
//! here. The rare time it is late, nothing waits inside a turn: what `agy` said from that step on is held, in its order, and read
//! again on the host's timer (`tick`), a few hundred milliseconds at most for a batch. `agent/frames` answers what came
//! before it, and what was held goes out as `agent/items` (docs/PLUGINS.md §8.4.6).

use crate::host;
use crate::translate::Translator;
use crate::words::{self, AgyRules, Denied, Settings, Usage};
use escouade_plugin_protocol::engine::{
    AnswerPermission, Attach, EnvVar, FileToWrite, Gone, Item, Launch, LaunchParams, StartAt,
};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

/// How often the transcript is read again, a step told over before it is written.
pub const TRANSCRIPT_EVERY_MS: u64 = 40;

/// How many times it is read again for a batch: 600 ms.
const TRANSCRIPT_TRIES: u32 = 15;

/// A request of the client waiting for the user's answer.
#[derive(Debug, Clone)]
enum Asked {
    /// A permission: the rule that allows the call, and how its call reads to the model.
    Permission { rule: String, said: String },
    /// The questions `ask_question` asked (their params, as `cursor/ask_question` gives them).
    Question { params: Value },
    /// The plan of a turn in the mode Plan.
    Plan,
}

/// What the user answered, kept until all is answered.
#[derive(Debug, Default)]
struct Answers {
    allowed: Vec<String>,
    refused: Vec<String>,
    words: Vec<String>,
    replies: Vec<String>,
}

/// One agent.
pub struct Client {
    /// Its key, as the host knows it (`fs/*`).
    agent: String,
    translator: Translator,
    lang: String,
    /// The agent's own home.
    home: Option<PathBuf>,
    rules: AgyRules,
    by_key: bool,
    tag: String,
    seq: u64,
    /// The conversation to resume, until its program opened one.
    resumed: Option<String>,
    /// That conversation could not be resumed.
    lost: bool,
    /// Its first program said its conversation (`init`).
    opened: bool,
    /// A program runs.
    up: bool,
    /// What is read is that of a program stopped, until the `init` of the next.
    stale_lines: bool,
    session: Option<String>,
    busy: bool,
    queue: VecDeque<String>,
    instructions: Option<String>,
    settings: Settings,
    /// Its program must start again before the next message (a model, effort or mode changed).
    stale: bool,
    /// What the user allowed for good (« Toujours »), and for the next turn only.
    always: Vec<String>,
    once: Vec<String>,
    /// The turn under way: its calls (by step, their name and arguments), the calls of the step being written not told
    /// yet, what was refused, the questions asked, the plan written, the tokens spent, whether its step wrote text.
    args: HashMap<u64, (String, Value)>,
    calls: VecDeque<(String, Value)>,
    denied: Vec<Denied>,
    questions: Vec<Value>,
    plan_file: Option<String>,
    usage: Usage,
    wrote_text: bool,
    asked: HashMap<String, Asked>,
    answers: Answers,
    /// What its program said from the step whose transcript is late on, in its order.
    held: VecDeque<String>,
    /// That step's line of the transcript, once read.
    found: Option<Value>,
    /// How many times this batch may still read the transcript again.
    patience: u32,
    /// `held` began in this very turn of the module: what it answers goes out first.
    fresh: bool,
}

fn line(msg: &Value) -> String {
    msg.to_string()
}

/// The last thing it said.
fn last_said(said: &str) -> String {
    said.lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("exited")
        .to_string()
}

impl Client {
    /// The agent `a`, given before its launch, or again to a plugin that started again (its program runs then).
    pub fn new(a: &Attach) -> Self {
        let home = a.home.as_ref().map(PathBuf::from);
        let always = Self::always(&a.agent, home.as_deref());
        let tag: String = a
            .agent
            .chars()
            .rev()
            .take_while(char::is_ascii_alphanumeric)
            .take(8)
            .collect();
        let mut translator = Translator::new(a);
        if let Some(session) = &a.session {
            translator.open(session);
        }
        Client {
            agent: a.agent.clone(),
            translator,
            lang: a.lang.clone(),
            home,
            rules: AgyRules {
                cwd: a.cwd.clone(),
                read_dirs: a.read_dirs.clone(),
                fence: a.fence.clone(),
                read_only: a.read_only,
            },
            by_key: a.by_key,
            tag: format!("agy-{tag}"),
            seq: 0,
            resumed: None,
            lost: false,
            opened: true,
            up: true,
            stale_lines: false,
            session: a.session.clone(),
            busy: a.running,
            queue: VecDeque::new(),
            instructions: None,
            settings: Settings {
                model: a.model.clone(),
                effort: a.effort.clone(),
                mode: a.mode.clone(),
            },
            stale: false,
            always,
            once: Vec::new(),
            args: HashMap::new(),
            calls: VecDeque::new(),
            denied: Vec::new(),
            questions: Vec::new(),
            plan_file: None,
            usage: Usage::default(),
            wrote_text: false,
            asked: HashMap::new(),
            answers: Answers::default(),
            held: VecDeque::new(),
            found: None,
            patience: 0,
            fresh: false,
        }
    }

    /// What the user allowed the agent `agent` for good, kept in its home.
    fn always(agent: &str, home: Option<&Path>) -> Vec<String> {
        let file = home.map(words::always_file);
        words::always_of(file.and_then(|f| host::read(agent, &f)))
    }

    fn next(&mut self) -> u64 {
        self.seq += 1;
        self.seq
    }

    /// The agent's settings, its rules as they stand: its mode's, its fence's, and what the user allowed. Its file before
    /// kept but them and its provider (its account's key, or the machine's Google sign-in, ESC-271).
    fn settings_file(&self) -> Option<FileToWrite> {
        let home = self.home.as_deref()?;
        let file = words::settings_file(home);
        let before = host::read(&self.agent, &file).unwrap_or_default();
        let allowed: Vec<String> = self.always.iter().chain(&self.once).cloned().collect();
        let mut doc = words::settings_doc(&before, &self.rules, &self.settings.mode, &allowed);
        words::set_provider(&mut doc, self.by_key);
        Some(FileToWrite {
            path: file.to_string_lossy().into_owned(),
            text: serde_json::to_string_pretty(&doc).unwrap_or_default(),
        })
    }

    #[cfg(test)]
    pub fn settings_text(&self) -> String {
        self.settings_file().map(|f| f.text).unwrap_or_default()
    }

    /// Its rules written now: `agy` reads them again before each tool.
    fn write_rules(&self, out: &mut Vec<Item>) {
        if let Some(f) = self.settings_file() {
            out.push(Item::File {
                path: f.path,
                text: f.text,
            });
        }
    }

    /// The description of a program of the agent on its conversation (`conversation`), its model, effort and mode as they
    /// stand: its home, its rules.
    fn program(&self, conversation: Option<&str>) -> Launch {
        let home = self.home.as_deref().unwrap_or(Path::new(""));
        let env = words::home_env(&[], home)
            .into_iter()
            .map(|(name, value)| EnvVar { name, value })
            .collect();
        // A key of the machine's would take the account's place; the core's variable is no business of `agy`.
        let mut remove_env: Vec<String> = words::KEY_VARS
            .iter()
            .filter(|k| !(self.by_key && **k == words::KEY_VARS[0]))
            .map(|k| k.to_string())
            .collect();
        remove_env.push(words::HOME_VAR.to_string());
        Launch {
            args: words::launch_args(&self.settings, conversation),
            env,
            remove_env,
            files: self.settings_file().into_iter().collect(),
            fence_applied: !self.rules.fence.is_empty(),
            read_only: self.rules.read_only,
            ..Launch::default()
        }
    }

    /// How its first program is launched: Escouade's server in its MCP config, its token in the file, never on a command
    /// line (`agy` expands no variable there), none when it is refused. It is taken out once the process is over (the core,
    /// `engine::agy`).
    pub fn launch(&mut self, p: &LaunchParams) -> Launch {
        self.lang = p.lang.clone();
        self.home = p.home.as_ref().map(PathBuf::from);
        self.rules = AgyRules {
            cwd: p.cwd.clone(),
            read_dirs: p.read_dirs.clone(),
            fence: p.fence.clone(),
            read_only: p.read_only,
        };
        self.by_key = p.by_key;
        self.settings = Settings {
            model: p.model.clone(),
            effort: p.effort.clone(),
            mode: p.mode.clone(),
        };
        self.always = Self::always(&self.agent, self.home.as_deref());
        self.translator.launch(p);
        self.opened = false;
        self.up = true;
        self.resumed = match &p.start {
            StartAt::Resume { session } => Some(session.clone()),
            _ => None,
        };
        let told = [p.append_prompt.clone(), Some(words::QUESTIONS.to_string())];
        self.instructions = Some(
            told.into_iter()
                .flatten()
                .filter(|t| !t.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n\n"),
        );
        let mut launch = self.program(self.resumed.as_deref());
        if let Some(home) = self.home.as_deref() {
            let file = words::mcp_file(home);
            let before = host::read(&self.agent, &file).unwrap_or_default();
            let grant = p
                .escouade
                .as_ref()
                .map(|g| (g.url.as_str(), g.token.as_str()));
            let doc = words::grant_doc(&before, grant);
            launch.files.push(FileToWrite {
                path: file.to_string_lossy().into_owned(),
                text: serde_json::to_string_pretty(&doc).unwrap_or_default(),
            });
        }
        launch
    }

    /// The line of the step `step` in the transcript of the conversation, once written.
    fn transcript_step(&self, step: u64) -> Option<Value> {
        let file = words::transcript(self.home.as_deref()?, self.session.as_deref()?);
        host::read_end(&self.agent, &file)?
            .lines()
            .filter_map(|l| serde_json::from_str::<Value>(l).ok())
            .find(|v| v["step_index"].as_u64() == Some(step))
    }

    /// The arguments of the call of the step `s`, as `step` will take them.
    fn args_of(&self, s: &Value, name: &str) -> Value {
        let step = s["step_index"].as_u64().unwrap_or(0);
        match self.args.get(&step) {
            Some((_, a)) => a.clone(),
            None => self
                .calls
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, a)| a.clone())
                .unwrap_or_else(|| s["tool_info"]["parameters"].clone()),
        }
    }

    /// The step whose line of the transcript `msg` needs before it is read: what ends a text (its calls, its reasoning),
    /// or a write (one refused ends `DONE` all the same, ESC-269).
    fn needs(&self, msg: &Value) -> Option<u64> {
        let s = &msg["step_update"];
        if msg["event"] != "step_update"
            || self.stale_lines
            || s["state"] != "DONE"
            || self.home.is_none()
            || self.session.is_none()
        {
            return None;
        }
        let writes = |name: &str| {
            words::rule_of(name, &self.args_of(s, name))
                .is_some_and(|(r, _)| r.starts_with("write_file("))
        };
        let needed = match s["step_type"].as_str().unwrap_or_default() {
            "agent_response" => true,
            "tool" | "subagent" => writes(s["tool_name"].as_str().unwrap_or_default()),
            _ => false,
        };
        needed.then(|| s["step_index"].as_u64().unwrap_or(0))
    }

    /// What is held, read in its order up to a line whose transcript is late, while there is patience for it.
    fn go_on(&mut self, out: &mut Vec<Item>) {
        while let Some(l) = self.held.front() {
            let msg = match serde_json::from_str::<Value>(l) {
                Ok(msg) => msg,
                Err(e) => {
                    eprintln!("unparsable agy event ({e}), {} bytes", l.len());
                    self.held.pop_front();
                    continue;
                }
            };
            if let Some(step) = self.needs(&msg) {
                self.found = self.transcript_step(step);
                if self.found.is_none() && self.patience > 0 {
                    return;
                }
            }
            self.held.pop_front();
            self.read(msg, out);
        }
    }

    /// What `agy` wrote: what can be read now. What is left is held, for `tick`, behind what was held already.
    pub fn frames(&mut self, lines: &[String]) -> Vec<Item> {
        let resting = self.held.is_empty();
        self.held.extend(lines.iter().cloned());
        let mut out = Vec::new();
        if resting {
            self.patience = TRANSCRIPT_TRIES;
            self.go_on(&mut out);
            self.fresh = !self.held.is_empty();
        }
        out
    }

    /// Lines are held, read again by `tick`.
    pub fn waits(&self) -> bool {
        !self.held.is_empty()
    }

    /// The host's timer rang: the transcript read again, and what that lets through, in its order.
    pub fn tick(&mut self) -> Vec<Item> {
        let mut out = Vec::new();
        if self.held.is_empty() || std::mem::take(&mut self.fresh) {
            return out;
        }
        self.patience = self.patience.saturating_sub(1);
        self.go_on(&mut out);
        out
    }

    /// What is held read at once: what comes next would overtake it.
    fn hurry(&mut self, out: &mut Vec<Item>) {
        self.patience = 0;
        self.fresh = false;
        self.go_on(out);
    }

    /// One event of its program.
    fn read(&mut self, mut msg: Value, out: &mut Vec<Item>) {
        match msg["event"].as_str().unwrap_or_default() {
            "init" => {
                if let Some(id) = msg["conversation_id"].as_str().filter(|i| !i.is_empty()) {
                    let id = id.to_string();
                    self.stale_lines = false;
                    self.conversation(&id, out);
                }
            }
            _ if self.stale_lines => {}
            "step_update" => {
                self.step(&mut msg);
                out.extend(self.translator.step(&msg["step_update"]));
            }
            "result" => self.turn_over(&msg["result"], out),
            _ => {}
        }
    }

    /// The conversation its program is on: its first one is open, and the agent ready.
    fn conversation(&mut self, id: &str, out: &mut Vec<Item>) {
        let changed = self.session.as_deref() != Some(id);
        let first = !self.opened;
        self.session = Some(id.to_string());
        if first {
            self.opened = true;
            let resumed = self.resumed.take();
            if resumed.as_deref() == Some(id) {
                self.instructions = None;
            } else if resumed.is_some() || self.lost {
                out.push(Item::Notice {
                    text: words::not_resumed(&self.lang),
                    level: "warn".into(),
                });
            }
        }
        if first || changed {
            out.extend(self.translator.open(id));
        }
        if first {
            out.push(Item::Ready {
                models: Value::Null,
            });
        }
    }

    /// A step of the turn: what the client needs of it kept, and what its event leaves out put in it (`escouade`: its
    /// call's arguments, its reasoning).
    fn step(&mut self, msg: &mut Value) {
        let s = &msg["step_update"];
        let step = s["step_index"].as_u64().unwrap_or(0);
        let state = s["state"].as_str().unwrap_or_default().to_string();
        match s["step_type"].as_str().unwrap_or_default() {
            "agent_response" => {
                if s["text_delta"]
                    .as_str()
                    .is_some_and(|t| !t.trim().is_empty())
                {
                    self.wrote_text = true;
                }
                if state != "DONE" {
                    return;
                }
                self.usage.add(&s["usage"]);
                let written = self.found.take();
                let wrote_text = std::mem::take(&mut self.wrote_text);
                if let Some(t) = written {
                    for c in t["tool_calls"].as_array().into_iter().flatten() {
                        let name = c["name"].as_str().unwrap_or_default().to_string();
                        if name == "ask_question" {
                            self.questions.push(c["args"].clone());
                        } else {
                            self.calls.push_back((name, c["args"].clone()));
                        }
                    }
                    // Its reasoning, told before its calls; a reply's comes after its text, left out.
                    if let Some(th) = t["thinking"].as_str().filter(|t| !t.trim().is_empty()) {
                        if !wrote_text {
                            msg["step_update"]["escouade"]["thinking"] = json!(th.trim());
                        }
                    }
                }
            }
            "tool" | "subagent" => {
                let name = s["tool_name"].as_str().unwrap_or_default().to_string();
                let written = self.found.take();
                let args = match self.args.get(&step) {
                    Some((_, a)) => a.clone(),
                    None => {
                        let at = self.calls.iter().position(|(n, _)| *n == name);
                        let args = at
                            .and_then(|i| self.calls.remove(i))
                            .map(|(_, a)| a)
                            .unwrap_or_else(|| s["tool_info"]["parameters"].clone());
                        self.args.insert(step, (name.clone(), args.clone()));
                        args
                    }
                };
                msg["step_update"]["escouade"]["args"] = args.clone();
                // A write refused ends `DONE` all the same (ESC-269): its transcript says it.
                let mut state = state;
                if state == "DONE"
                    && words::rule_of(&name, &args)
                        .is_some_and(|(r, _)| r.starts_with("write_file("))
                {
                    if let Some(t) = written.filter(|t| t["status"] == "ERROR") {
                        let e = t["error"].as_str().unwrap_or("error").to_string();
                        msg["step_update"]["state"] = json!("ERROR");
                        msg["step_update"]["tool_info"]["error"] =
                            json!({ "type": "TOOL_ERROR", "message": e });
                        state = "ERROR".into();
                    }
                }
                let s = &msg["step_update"];
                let error = s["tool_info"]["error"]["message"].as_str().unwrap_or("");
                if state == "ERROR" && words::is_denial(error) {
                    self.denied.push(Denied {
                        step,
                        tool: name,
                        args,
                    });
                } else if state == "DONE" && words::is_plan_file(&name, &args) {
                    self.plan_file = args["Path"].as_str().map(str::to_string);
                }
            }
            _ => {}
        }
    }

    /// The end of a turn of `agy` (`result`): over for Escouade, or what it was refused and asked asked of the user first,
    /// or its plan to approve.
    fn turn_over(&mut self, r: &Value, out: &mut Vec<Item>) {
        let denied = std::mem::take(&mut self.denied);
        let questions = std::mem::take(&mut self.questions);
        let plan_file = self.plan_file.take();
        let spent = std::mem::take(&mut self.usage);
        let planning = self.settings.mode == "plan";
        // What was allowed for this turn only is taken back.
        if !self.once.is_empty() {
            self.once.clear();
            self.write_rules(out);
        }
        self.args.clear();
        self.calls.clear();
        self.translator.spend(spent);
        if r["status"] == "ERROR" {
            let error = r["error"].as_str().unwrap_or("error").to_string();
            return self.end_turn(false, Some(&error), out);
        }
        let session = self.session.clone().unwrap_or_default();
        let mut asks: Vec<(String, Asked, &str, Value)> = Vec::new();
        if !self.rules.read_only {
            for d in &denied {
                let Some((rule, said)) = words::rule_of(&d.tool, &d.args) else {
                    continue;
                };
                // « Projet privé »: refused at once, never a card the user could accept.
                if words::fenced(&d.args, &self.rules.fence) {
                    eprintln!(
                        "agy: a call naming a path of another project refused (private project)"
                    );
                    continue;
                }
                asks.push((
                    format!("{}-{}", self.tag, d.step),
                    Asked::Permission { rule, said },
                    "session/request_permission",
                    words::permission_params(&session, d),
                ));
            }
        }
        for (i, q) in questions.iter().enumerate() {
            let params = words::question_params(&format!("{session}-q{i}"), q);
            let n = self.next();
            asks.push((
                format!("{}-q{n}", self.tag),
                Asked::Question {
                    params: params.clone(),
                },
                "cursor/ask_question",
                params,
            ));
        }
        if asks.is_empty() && planning {
            if let Some(plan) = plan_file.and_then(|f| host::read(&self.agent, Path::new(&f))) {
                let n = self.next();
                let rid = format!("{}-plan-{n}", self.tag);
                self.asked.insert(rid.clone(), Asked::Plan);
                return out.extend(self.translator.plan(&rid, plan.trim()));
            }
        }
        if asks.is_empty() {
            return self.end_turn(false, None, out);
        }
        self.answers = Answers::default();
        for (rid, asked, _, _) in &asks {
            self.asked.insert(rid.clone(), asked.clone());
        }
        let mut allowed_by_rules = Vec::new();
        for (rid, _, method, params) in asks {
            let (items, answered) = self.translator.ask(method, &rid, params);
            out.extend(items);
            if let Some(allow) = answered {
                allowed_by_rules.push((rid, allow));
            }
        }
        // What Escouade's rules answer needs no card.
        for (rid, allow) in allowed_by_rules {
            self.answer(
                &rid,
                &AnswerPermission {
                    agent: String::new(),
                    request: rid.clone(),
                    allow,
                    always: false,
                    input: Value::Null,
                    message: None,
                    resume: None,
                    call: String::new(),
                    rules: Value::Null,
                },
                out,
            );
        }
    }

    /// The turn over: told, then the next message waiting sent.
    fn end_turn(&mut self, cancelled: bool, error: Option<&str>, out: &mut Vec<Item>) {
        self.busy = false;
        for (rid, _) in self.asked.drain() {
            self.translator.answered(&rid);
        }
        out.extend(self.translator.end(cancelled, error));
        if let Some(text) = self.queue.pop_front() {
            out.push(Item::Start {
                session: self.session.clone(),
            });
            self.prompt(text, out);
        }
    }

    /// Sends `text` as a turn of its own, its program started again first when it must be.
    fn prompt(&mut self, text: String, out: &mut Vec<Item>) {
        let text = match self.instructions.take() {
            Some(i) if !i.trim().is_empty() => {
                format!("{}\n\n{text}", escouade_acp::preamble(&i))
            }
            _ => text,
        };
        self.busy = true;
        self.wrote_text = false;
        self.translator.begin();
        if std::mem::take(&mut self.stale) || !self.up {
            let launch = self.program(self.session.as_deref());
            out.push(Item::Relaunch {
                launch: Box::new(launch),
            });
            self.up = true;
            self.stale_lines = true;
        }
        out.push(Item::Write {
            line: line(&json!({ "event": "user", "message": { "content": text } })),
        });
    }

    /// A message: sent while nothing runs, else (a turn runs, or waits for answers) at its end.
    pub fn send(&mut self, text: &str) -> Vec<Item> {
        if self.busy {
            self.queue.push_back(text.to_string());
            return Vec::new();
        }
        let mut out = Vec::new();
        self.hurry(&mut out);
        self.prompt(text.to_string(), &mut out);
        out
    }

    /// `agy` has no message that stops a turn: its program is stopped, the turn ended `cancelled`, and the next message
    /// starts it again on the conversation (resumed, tried in ESC-269).
    pub fn interrupt(&mut self, cancel_queued: bool) -> Vec<Item> {
        let mut out = Vec::new();
        self.hurry(&mut out);
        if !self.busy {
            return out;
        }
        if cancel_queued {
            self.queue.clear();
        }
        self.denied.clear();
        self.questions.clear();
        self.calls.clear();
        self.args.clear();
        self.plan_file = None;
        self.usage = Usage::default();
        self.up = false;
        self.stale_lines = true;
        out.push(Item::Pause);
        self.end_turn(true, None, &mut out);
        out
    }

    pub fn set_model(&mut self, model: &str) {
        if self.settings.model != model {
            self.settings.model = model.to_string();
            self.stale = true;
        }
        self.translator.set_model(model);
    }

    pub fn set_effort(&mut self, effort: &str) {
        if self.settings.effort != effort {
            self.settings.effort = effort.to_string();
            self.stale = true;
        }
    }

    /// Its rules written at once, its `--mode` at the next start of its program.
    pub fn set_mode(&mut self, mode: &str) -> Vec<Item> {
        if words::agy_mode(&self.settings.mode) != words::agy_mode(mode) {
            self.stale = true;
        }
        self.settings.mode = mode.to_string();
        self.translator.set_mode(mode);
        let mut out = Vec::new();
        self.write_rules(&mut out);
        out
    }

    /// Its own name of the card `request` of the conversation.
    fn known(&self, request: &str) -> Option<String> {
        self.asked
            .keys()
            .find(|k| request == k.as_str() || request.ends_with(&format!("-{k}")))
            .cloned()
    }

    /// A card that is no longer known (its plugin started again since it was shown): the turn cannot go on, it ends.
    fn lost_card(&mut self) -> Result<Vec<Item>, String> {
        if !self.busy {
            return Err("Antigravity no longer waits for this answer".into());
        }
        let mut out = Vec::new();
        self.end_turn(false, None, &mut out);
        Ok(out)
    }

    /// The user's answer to a permission, or to a plan.
    pub fn answer_permission(&mut self, a: &AnswerPermission) -> Result<Vec<Item>, String> {
        let Some(rid) = self.known(&a.request) else {
            return self.lost_card();
        };
        self.translator.answered(&a.request);
        let mut out = Vec::new();
        self.answer(&rid, a, &mut out);
        Ok(out)
    }

    fn answer(&mut self, rid: &str, a: &AnswerPermission, out: &mut Vec<Item>) {
        match self.asked.remove(rid) {
            Some(Asked::Plan) => self.answer_plan(a, out),
            Some(Asked::Permission { rule, said }) => {
                if !a.allow {
                    self.answers.refused.push(said);
                    if let Some(m) = a
                        .message
                        .as_deref()
                        .filter(|m| escouade_acp::vibe::worded(m))
                    {
                        self.answers.words.push(m.to_string());
                    }
                } else if a.always {
                    if !self.always.contains(&rule) {
                        self.always.push(rule);
                        if let Some(home) = self.home.as_deref() {
                            out.push(Item::File {
                                path: words::always_file(home).to_string_lossy().into_owned(),
                                text: json!(self.always).to_string(),
                            });
                        }
                    }
                    self.answers.allowed.push(said);
                } else {
                    self.once.push(rule);
                    self.answers.allowed.push(said);
                }
                self.answered(out);
            }
            _ => {}
        }
    }

    /// The answers of the card, by question: told to the model in the message that goes on.
    pub fn answer_question(&mut self, request: &str, answers: &Value) -> Result<Vec<Item>, String> {
        let Some(rid) = self.known(request) else {
            return self.lost_card();
        };
        self.translator.answered(request);
        let Some(Asked::Question { params }) = self.asked.remove(&rid) else {
            return Err("not a question".into());
        };
        self.answers
            .replies
            .extend(words::replies(&params, answers));
        let mut out = Vec::new();
        self.answered(&mut out);
        Ok(out)
    }

    /// All asked is answered: the turn goes on with what was allowed and answered, else ends (a refusal's words sent after
    /// it).
    fn answered(&mut self, out: &mut Vec<Item>) {
        if !self.asked.is_empty() {
            return;
        }
        let a = std::mem::take(&mut self.answers);
        match words::follow_up(&a.allowed, &a.refused, &a.replies) {
            Some(text) => {
                self.write_rules(out);
                // Read again by `agy` only some time after (tried: the message sent at once was refused again): its
                // program starts again on the conversation, which reads them.
                if !a.allowed.is_empty() {
                    self.stale = true;
                }
                let text = if a.words.is_empty() {
                    text
                } else {
                    format!("{text}\n\n{}", a.words.join("\n\n"))
                };
                self.prompt(text, out);
            }
            None => {
                for w in a.words.into_iter().rev() {
                    self.queue.push_front(w);
                }
                self.end_turn(false, None, out);
            }
        }
    }

    /// The plan answered: approved, carried out in the mode the agent was in before Plan (« Accepter les modifications » for
    /// « Toujours »), its program started again on that mode; sent back with words, planned again; without, the turn ends.
    fn answer_plan(&mut self, a: &AnswerPermission, out: &mut Vec<Item>) {
        let (mode, text) = if a.allow {
            let mode = if a.always {
                "acceptEdits".to_string()
            } else {
                a.resume.clone().unwrap_or_else(|| "default".into())
            };
            (Some(mode), words::PLAN_GO.to_string())
        } else {
            (None, a.message.clone().unwrap_or_default())
        };
        if mode.is_none() && !escouade_acp::vibe::worded(&text) {
            return self.end_turn(false, None, out);
        }
        if let Some(mode) = mode {
            self.settings.mode = mode.clone();
            self.translator.set_mode(&mode);
            self.stale = true;
            out.push(Item::Mode { mode });
        }
        self.prompt(text, out);
    }

    /// Its program ended by itself, before it said its conversation: one to resume that is gone gives a new one; else it did
    /// not start, and why is said. After that, the process ends with it.
    pub fn program_gone(&mut self, g: &Gone) -> Vec<Item> {
        if self.opened {
            return Vec::new();
        }
        if self.resumed.take().is_some() {
            self.lost = true;
            return vec![Item::Relaunch {
                launch: Box::new(self.program(None)),
            }];
        }
        vec![Item::Stop {
            reason: words::session_refused(&self.lang, &last_said(&g.said)),
        }]
    }
}
