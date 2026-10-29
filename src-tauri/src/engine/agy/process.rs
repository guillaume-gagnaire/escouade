//! The process of an agent on Antigravity CLI: `agy -p= --input-format stream-json
//! --output-format stream-json`, one message a line on its input, one event a line on its output
//! (docs/TODO-1.8.md, point 0, « Antigravity CLI »; docs/PROTOCOL.md, « Antigravity CLI »).
//!
//! `agy` takes its model, effort and mode at its launch only, and has no message that stops a turn:
//! its program is started again on the same conversation (`--conversation <id>`) when one of them
//! changes (before the next message) and when a turn is stopped (killed, then started again at the
//! next message). Its program may thus be several in a row; the process of the agent is all of
//! them, and ends (`on_exit`) when its input is closed or it is killed, or when its program stops of
//! itself.
//!
//! In `-p`, `agy` asks nothing: what needs a permission is refused at once (its step ends in
//! `ERROR`, its `result` lists `denied_actions`) and its questions (`ask_question`) are skipped. At
//! the end of such a turn, the process asks the user (`session/request_permission` and
//! `cursor/ask_question`, in the form Cursor's translation reads), writes the rules allowed in the
//! agent's own `settings.json` (`agy` reads them again before each tool, tried) and has the turn go
//! on with a message saying what was allowed and answered: one turn of Escouade for both.
//!
//! What its events leave out (a call's arguments, a write's text, its reasoning) is read from the
//! conversation's transcript (`brain/<id>/.system_generated/logs/transcript_full.jsonl`), written
//! before the event that tells the step over.

use super::{AgyRules, Settings};
use crate::engine::{
    BoxFuture, EngineProcess, Message, OnExit, OnFrame, PermissionAnswer, Program, Ready, Remote,
    Start,
};
use crate::i18n::{self, Lang};
use crate::job::{Job, JobUsage};
use anyhow::{anyhow, bail, Context, Result};
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

/// How long its program may take to say it is ready (`init`): it signs in again first.
const READY_LIMIT: Duration = Duration::from_secs(90);

/// How long a program whose input was closed is given before it is killed (`close_input`).
const CLOSE_GRACE: Duration = Duration::from_secs(3);

/// How long the transcript is waited for, a step told over before it is written.
const TRANSCRIPT_WAIT: Duration = Duration::from_millis(600);

/// A request of the process waiting for the user's answer.
#[derive(Debug, Clone)]
pub(super) enum Asked {
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

/// A call `agy` refused for want of a permission.
#[derive(Debug, Clone)]
pub(super) struct Denied {
    pub step: u64,
    pub tool: String,
    pub args: Value,
}

/// What the process knows of its conversation.
#[derive(Default)]
pub(super) struct State {
    pub session: Option<String>,
    pub busy: bool,
    pub queue: VecDeque<String>,
    pub instructions: Option<String>,
    pub settings: Settings,
    /// Its program must start again before the next message (a model, effort or mode changed).
    pub stale: bool,
    /// What the user allowed for good (« Toujours »), and for the next turn only.
    pub always: Vec<String>,
    pub once: Vec<String>,
    /// The turn under way: its calls (by step, their name and arguments), the calls of the step
    /// being written not told yet, what was refused, the questions asked, the plan written, the
    /// tokens spent, whether its step wrote text.
    pub args: HashMap<u64, (String, Value)>,
    pub calls: VecDeque<(String, Value)>,
    pub denied: Vec<Denied>,
    pub questions: Vec<Value>,
    pub plan_file: Option<String>,
    pub usage: Usage,
    pub wrote_text: bool,
    pub asked: HashMap<String, Asked>,
    answers: Answers,
}

/// The tokens a turn spent, as its steps told them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Usage {
    pub input: u64,
    pub output: u64,
    pub cached: u64,
}

impl Usage {
    fn add(&mut self, u: &Value) {
        let n = |k: &str| u[k].as_u64().unwrap_or(0);
        self.input += n("input_tokens");
        self.output += n("output_tokens") + n("thinking_tokens");
        self.cached += n("cache_read_tokens");
    }
}

/// One program of the agent's process.
struct Program1 {
    gen: u64,
    tx: Option<mpsc::UnboundedSender<Vec<u8>>>,
    kill_tx: Option<oneshot::Sender<()>>,
    job: Option<Arc<Job>>,
}

/// How its programs are started.
pub(super) struct Opts {
    pub program: Program,
    pub cwd: String,
    pub env: Vec<(String, String)>,
    /// The agent's own home (`USERPROFILE`, `HOME`): its `.gemini`, never the user's.
    pub home: PathBuf,
    pub start: Start,
    pub settings: Settings,
    pub instructions: Option<String>,
    pub rules: AgyRules,
}

pub struct AgyProcess {
    opts: Opts,
    current: Mutex<Option<Program1>>,
    gen: AtomicU64,
    /// Its end is under way (its input closed, or killed): no program starts again.
    closing: AtomicBool,
    exited: Arc<AtomicBool>,
    on_exit: Mutex<Option<OnExit>>,
    on_frame: Arc<OnFrame>,
    state: Arc<Mutex<State>>,
    ready: Mutex<Option<oneshot::Sender<String>>>,
    tag: String,
    tail: Arc<Mutex<VecDeque<String>>>,
    me: std::sync::Weak<Self>,
}

fn line(msg: &Value) -> Vec<u8> {
    let mut out = msg.to_string().into_bytes();
    out.push(b'\n');
    out
}

/// What `agy`'s error says of a refusal for want of a permission (its step's `error.message`).
pub(crate) fn is_denial(message: &str) -> bool {
    message.contains("permission check failed") || message.contains("denied permission")
}

impl AgyProcess {
    pub(super) fn new(opts: Opts, on_frame: OnFrame, on_exit: OnExit) -> Arc<Self> {
        let state = State {
            settings: opts.settings.clone(),
            instructions: opts.instructions.clone().filter(|i| !i.trim().is_empty()),
            always: super::read_always(&opts.home),
            ..Default::default()
        };
        Arc::new_cyclic(|me| Self {
            opts,
            current: Mutex::new(None),
            gen: AtomicU64::new(0),
            closing: AtomicBool::new(false),
            exited: Arc::new(AtomicBool::new(false)),
            on_exit: Mutex::new(Some(on_exit)),
            on_frame: Arc::new(on_frame),
            state: Arc::new(Mutex::new(state)),
            ready: Mutex::new(None),
            tag: format!("agy-{}", &uuid::Uuid::new_v4().simple().to_string()[..8]),
            tail: Arc::default(),
            me: me.clone(),
        })
    }

    fn self_arc(&self) -> Result<Arc<Self>> {
        self.me.upgrade().ok_or_else(|| anyhow!("process gone"))
    }

    fn frame(&self, method: &str, params: Value) {
        (self.on_frame)(json!({ "method": method, "params": params }));
    }

    /// The rules of the agent's `settings.json` as they stand: its mode's, its fence's, and what
    /// the user allowed.
    fn write_rules(&self) {
        let (mode, always, once) = {
            let s = self.state.lock();
            (s.settings.mode.clone(), s.always.clone(), s.once.clone())
        };
        let allowed: Vec<String> = always.into_iter().chain(once).collect();
        if let Err(e) = super::write_settings(&self.opts.home, &self.opts.rules, &mode, &allowed) {
            log::warn!("agy: its settings not written: {e:#}");
        }
    }

    /// Starts a program of the agent on its conversation (`conversation`), its model, effort and
    /// mode as they stand.
    fn start_program(self: &Arc<Self>, conversation: Option<&str>) -> Result<()> {
        self.write_rules();
        let settings = self.state.lock().settings.clone();
        let args = super::launch_args(&settings, conversation);
        let o = &self.opts;
        let launch = o.program.launch(&o.cwd, &args, &o.env)?;
        let mut cmd = Command::new(&launch.program);
        cmd.args(&launch.args)
            .current_dir(&o.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        for (k, v) in &launch.env {
            cmd.env(k, v);
        }
        // A key of the machine's would take the account's place (docs/TODO-1.8.md, « Gemini CLI »).
        for k in super::KEY_VARS {
            if !o.env.iter().any(|(e, _)| e == k) {
                cmd.env_remove(k);
            }
        }
        #[cfg(windows)]
        cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
        crate::job::isolate(&mut cmd);
        crate::proc::no_inherited_fds(&mut cmd);
        let mut child = cmd
            .spawn()
            .with_context(|| cannot_start(i18n::ui(), &o.program.describe()))?;
        let job = Job::for_child(&child).map(Arc::new);
        let mut stdin = child.stdin.take().context("stdin")?;
        let stdout = child.stdout.take().context("stdout")?;
        let stderr = child.stderr.take().context("stderr")?;
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
        tokio::spawn(async move {
            while let Some(bytes) = rx.recv().await {
                if stdin.write_all(&bytes).await.is_err() || stdin.flush().await.is_err() {
                    break;
                }
            }
        });
        {
            let tail = self.tail.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).split(b'\n');
                while let Ok(Some(line)) = lines.next_segment().await {
                    let line = String::from_utf8_lossy(&line)
                        .trim_end_matches('\r')
                        .to_string();
                    // Its logs name the account: only what it says of a turn is kept.
                    if line.contains('@') {
                        continue;
                    }
                    log::debug!("agy stderr: {line}");
                    note(&tail, line);
                }
            });
        }
        let gen = self.gen.fetch_add(1, Ordering::AcqRel) + 1;
        let (kill_tx, mut kill_rx) = oneshot::channel::<()>();
        *self.current.lock() = Some(Program1 {
            gen,
            tx: Some(tx),
            kill_tx: Some(kill_tx),
            job: job.clone(),
        });
        let weak = self.me.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).split(b'\n');
            loop {
                tokio::select! {
                    next = lines.next_segment() => match next {
                        Ok(Some(bytes)) => {
                            let text = String::from_utf8_lossy(&bytes);
                            let text = text.trim();
                            if text.is_empty() {
                                continue;
                            }
                            match crate::text::json_line(text) {
                                Ok(msg) => match weak.upgrade() {
                                    Some(p) => p.read(gen, msg).await,
                                    None => break,
                                },
                                Err(e) => log::warn!("unparsable agy event ({e}), {} bytes", text.len()),
                            }
                        }
                        _ => break,
                    },
                    _ = &mut kill_rx => {
                        let _ = child.start_kill();
                        break;
                    }
                }
            }
            let code = child.wait().await.ok().and_then(|s| s.code());
            drop(job);
            if let Some(p) = weak.upgrade() {
                p.program_gone(gen, code);
            }
        });
        Ok(())
    }

    /// A program of the agent gone: the process ends with it unless it was stopped to start again.
    fn program_gone(&self, gen: u64, code: Option<i32>) {
        let current = self.current.lock().as_ref().map(|c| c.gen);
        if current != Some(gen) {
            return;
        }
        self.current.lock().take();
        // Gone before it was ready: its opening says why, and may start another (`initialize`).
        if let Some(r) = self.ready.lock().take() {
            drop(r);
            return;
        }
        self.finish(code);
    }

    /// The process is over: told once.
    fn finish(&self, code: Option<i32>) {
        if self.exited.swap(true, Ordering::AcqRel) {
            return;
        }
        // The token of Escouade's server goes with it.
        super::remove_grant(&self.opts.home);
        if let Some(r) = self.ready.lock().take() {
            drop(r);
        }
        let said = self
            .tail
            .lock()
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        if let Some(f) = self.on_exit.lock().take() {
            f(code, said);
        }
    }

    /// Stops its program, to start another (its end not the process's).
    fn stop_program(&self) {
        if let Some(mut p) = self.current.lock().take() {
            p.tx.take();
            if let Some(job) = &p.job {
                job.terminate();
            }
            if let Some(k) = p.kill_tx.take() {
                let _ = k.send(());
            }
        }
    }

    fn write(&self, msg: &Value) -> Result<()> {
        let guard = self.current.lock();
        let tx = guard
            .as_ref()
            .and_then(|p| p.tx.as_ref())
            .ok_or_else(|| anyhow!("Antigravity's input is closed"))?;
        tx.send(line(msg))
            .map_err(|_| anyhow!("Antigravity's input is closed"))
    }

    /// The transcript of the conversation (`transcript_full.jsonl`), its step `step` once written.
    async fn transcript_step(&self, step: u64) -> Option<Value> {
        let session = self.state.lock().session.clone()?;
        let file = super::transcript(&self.opts.home, &session);
        let started = std::time::Instant::now();
        loop {
            if let Ok(text) = tokio::fs::read_to_string(&file).await {
                let found = text
                    .lines()
                    .filter_map(|l| serde_json::from_str::<Value>(l).ok())
                    .find(|v| v["step_index"].as_u64() == Some(step));
                if found.is_some() {
                    return found;
                }
            }
            if started.elapsed() >= TRANSCRIPT_WAIT {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(40)).await;
        }
    }

    /// One event of its program.
    async fn read(self: &Arc<Self>, gen: u64, mut msg: Value) {
        if self.current.lock().as_ref().map(|c| c.gen) != Some(gen) {
            return;
        }
        match msg["event"].as_str().unwrap_or_default() {
            "init" => {
                if let Some(id) = msg["conversation_id"].as_str().filter(|i| !i.is_empty()) {
                    self.state.lock().session = Some(id.to_string());
                    if let Some(r) = self.ready.lock().take() {
                        let _ = r.send(id.to_string());
                    }
                }
            }
            "step_update" => {
                self.step(&mut msg).await;
                (self.on_frame)(msg);
            }
            "result" => self.turn_over(&msg["result"]),
            _ => {}
        }
    }

    /// A step of the turn: what the process needs of it kept, and what its event leaves out put in
    /// it (`escouade`: its call's arguments, its reasoning).
    async fn step(&self, msg: &mut Value) {
        let s = &msg["step_update"];
        let step = s["step_index"].as_u64().unwrap_or(0);
        let state = s["state"].as_str().unwrap_or_default().to_string();
        match s["step_type"].as_str().unwrap_or_default() {
            "agent_response" => {
                if s["text_delta"]
                    .as_str()
                    .is_some_and(|t| !t.trim().is_empty())
                {
                    self.state.lock().wrote_text = true;
                }
                if state != "DONE" {
                    return;
                }
                self.state.lock().usage.add(&s["usage"]);
                let written = self.transcript_step(step).await;
                let wrote_text = std::mem::take(&mut self.state.lock().wrote_text);
                if let Some(t) = written {
                    let mut st = self.state.lock();
                    for c in t["tool_calls"].as_array().into_iter().flatten() {
                        let name = c["name"].as_str().unwrap_or_default().to_string();
                        if name == "ask_question" {
                            st.questions.push(c["args"].clone());
                        } else {
                            st.calls.push_back((name, c["args"].clone()));
                        }
                    }
                    drop(st);
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
                let args = {
                    let mut st = self.state.lock();
                    match st.args.get(&step) {
                        Some((_, a)) => a.clone(),
                        None => {
                            let at = st.calls.iter().position(|(n, _)| *n == name);
                            let args = at
                                .and_then(|i| st.calls.remove(i))
                                .map(|(_, a)| a)
                                .unwrap_or_else(|| s["tool_info"]["parameters"].clone());
                            st.args.insert(step, (name.clone(), args.clone()));
                            args
                        }
                    }
                };
                msg["step_update"]["escouade"]["args"] = args.clone();
                // A write refused ends `DONE` all the same (ESC-269): its transcript says it.
                let mut state = state;
                if state == "DONE"
                    && super::rule_of(&name, &args)
                        .is_some_and(|(r, _)| r.starts_with("write_file("))
                {
                    if let Some(t) = self
                        .transcript_step(step)
                        .await
                        .filter(|t| t["status"] == "ERROR")
                    {
                        let e = t["error"].as_str().unwrap_or("error").to_string();
                        msg["step_update"]["state"] = json!("ERROR");
                        msg["step_update"]["tool_info"]["error"] =
                            json!({ "type": "TOOL_ERROR", "message": e });
                        state = "ERROR".into();
                    }
                }
                let s = &msg["step_update"];
                let error = s["tool_info"]["error"]["message"].as_str().unwrap_or("");
                let mut st = self.state.lock();
                if state == "ERROR" && is_denial(error) {
                    st.denied.push(Denied {
                        step,
                        tool: name.clone(),
                        args,
                    });
                } else if state == "DONE" && super::is_plan_file(&name, &args) {
                    st.plan_file = args["Path"].as_str().map(str::to_string);
                }
            }
            _ => {}
        }
    }

    /// The end of a turn of `agy` (`result`): over for Escouade, or what it was refused and asked
    /// asked of the user first, or its plan to approve.
    fn turn_over(self: &Arc<Self>, r: &Value) {
        let (denied, questions, plan_file, usage, planning, read_only) = {
            let mut s = self.state.lock();
            // What was allowed for this turn only is taken back.
            let once = !s.once.is_empty();
            s.once.clear();
            let out = (
                std::mem::take(&mut s.denied),
                std::mem::take(&mut s.questions),
                s.plan_file.take(),
                std::mem::take(&mut s.usage),
                s.settings.mode == "plan",
                self.opts.rules.read_only,
            );
            s.args.clear();
            s.calls.clear();
            drop(s);
            if once {
                self.write_rules();
            }
            out
        };
        let session = self.state.lock().session.clone().unwrap_or_default();
        let usage = json!({ "input": usage.input, "output": usage.output, "cached": usage.cached });
        if r["status"] == "ERROR" {
            self.end_turn(json!({
                "sessionId": session, "error": r["error"].as_str().unwrap_or("error"), "usage": usage,
            }));
            return;
        }
        // Spent so far, told with the turn's end.
        self.state.lock().usage = Usage::default();
        let mut asks = Vec::new();
        if !read_only {
            for d in &denied {
                let Some((rule, said)) = super::rule_of(&d.tool, &d.args) else {
                    continue;
                };
                // « Projet privé »: refused at once, never a card the user could accept.
                if super::fenced(&d.args, &self.opts.rules.fence) {
                    log::info!(
                        "agy: a call naming a path of another project refused (private project)"
                    );
                    continue;
                }
                let rid = format!("{}-{}", self.tag, d.step);
                let params = super::permission_params(&session, d);
                asks.push((
                    rid,
                    Asked::Permission { rule, said },
                    "session/request_permission",
                    params,
                ));
            }
        }
        for (i, q) in questions.iter().enumerate() {
            let rid = format!("{}-q{}", self.tag, uuid::Uuid::new_v4().simple());
            let params = super::question_params(&format!("{session}-q{i}"), q);
            asks.push((
                rid,
                Asked::Question {
                    params: params.clone(),
                },
                "cursor/ask_question",
                params,
            ));
        }
        if asks.is_empty() && planning {
            if let Some(plan) = plan_file.and_then(|f| std::fs::read_to_string(f).ok()) {
                let rid = format!("{}-plan-{}", self.tag, uuid::Uuid::new_v4().simple());
                let params = json!({ "sessionId": session, "plan": plan.trim() });
                self.state.lock().asked.insert(rid.clone(), Asked::Plan);
                (self.on_frame)(
                    json!({ "method": super::PLAN_REQUEST, "id": rid, "params": params, "usage": usage }),
                );
                return;
            }
        }
        if asks.is_empty() {
            self.end_turn(
                json!({ "sessionId": session, "stopReason": "end_turn", "usage": usage }),
            );
            return;
        }
        // Its tokens told now: the turn goes on after the answers.
        self.frame("escouade.spent", json!({ "usage": usage }));
        {
            let mut s = self.state.lock();
            s.answers = Answers::default();
            for (rid, asked, _, _) in &asks {
                s.asked.insert(rid.clone(), asked.clone());
            }
        }
        for (rid, _, method, params) in asks {
            (self.on_frame)(json!({ "method": method, "id": rid, "params": params }));
        }
    }

    /// The turn over: told, then the next message waiting sent.
    fn end_turn(self: &Arc<Self>, params: Value) {
        let next = {
            let mut s = self.state.lock();
            s.busy = false;
            s.asked.clear();
            s.queue.pop_front()
        };
        self.frame("escouade.prompt_end", params);
        if let Some(text) = next {
            let session = self.state.lock().session.clone();
            self.frame("escouade.prompt_start", json!({ "sessionId": session }));
            if let Err(e) = self.prompt(text) {
                log::warn!("agy: a message waiting was not sent: {e:#}");
            }
        }
    }

    /// Sends `text` as a turn of its own, its program started again first when it must be.
    fn prompt(self: &Arc<Self>, text: String) -> Result<()> {
        if self.closing.load(Ordering::Acquire) {
            bail!("Antigravity's process is ending");
        }
        let (stale, session, text) = {
            let mut s = self.state.lock();
            let mut heads: Vec<String> = Vec::new();
            if let Some(i) = s.instructions.take() {
                heads.push(crate::engine::cursor::preamble(&i));
            }
            let text = if heads.is_empty() {
                text
            } else {
                heads.push(text);
                heads.join("\n\n")
            };
            s.busy = true;
            s.wrote_text = false;
            (std::mem::take(&mut s.stale), s.session.clone(), text)
        };
        if stale || self.current.lock().is_none() {
            self.stop_program();
            if let Err(e) = self.start_program(session.as_deref()) {
                self.state.lock().busy = false;
                return Err(e);
            }
        }
        let sent = self.write(&json!({ "event": "user", "message": { "content": text } }));
        if sent.is_err() {
            self.state.lock().busy = false;
        }
        sent
    }

    /// All asked is answered: the turn goes on with what was allowed and answered, else ends (a
    /// refusal's words sent after it).
    fn answered(self: &Arc<Self>) -> Result<()> {
        let (go_on, words, allowed_any) = {
            let mut s = self.state.lock();
            if !s.asked.is_empty() {
                return Ok(());
            }
            let a = std::mem::take(&mut s.answers);
            let words = a.words.clone();
            let allowed_any = !a.allowed.is_empty();
            (
                super::follow_up(&a.allowed, &a.refused, &a.replies),
                words,
                allowed_any,
            )
        };
        let session = self.state.lock().session.clone();
        match go_on {
            Some(text) => {
                self.write_rules();
                // Read again by `agy` only some time after (tried: the message sent at once was
                // refused again): its program starts again on the conversation, which reads them.
                if allowed_any {
                    self.state.lock().stale = true;
                }
                let text = if words.is_empty() {
                    text
                } else {
                    format!("{text}\n\n{}", words.join("\n\n"))
                };
                self.prompt(text)
            }
            None => {
                {
                    let mut s = self.state.lock();
                    for w in words.into_iter().rev() {
                        s.queue.push_front(w);
                    }
                }
                self.end_turn(json!({ "sessionId": session, "stopReason": "end_turn" }));
                Ok(())
            }
        }
    }

    /// The plan answered: approved, carried out in the mode the agent was in before Plan (ESC-277,
    /// `AgentMeta::plan_resume`; « Accepter les modifications » for « Toujours »), its program
    /// started again on that mode; sent back with words, planned again; without, the turn ends.
    fn answer_plan(self: &Arc<Self>, answer: PermissionAnswer) -> Result<()> {
        let session = self.state.lock().session.clone();
        let (mode, text) = match answer {
            PermissionAnswer::Allow { resume, .. } => (
                Some(resume.unwrap_or_else(|| "default".into())),
                super::PLAN_GO.to_string(),
            ),
            PermissionAnswer::Always { .. } => {
                (Some("acceptEdits".to_string()), super::PLAN_GO.to_string())
            }
            PermissionAnswer::Deny { message, .. } => (None, message),
        };
        if mode.is_none() && !worded(&text) {
            self.end_turn(json!({ "sessionId": session, "stopReason": "end_turn" }));
            return Ok(());
        }
        if let Some(mode) = mode {
            {
                let mut s = self.state.lock();
                s.settings.mode = mode.clone();
                s.stale = true;
            }
            self.frame(
                "escouade.mode",
                json!({ "sessionId": session, "mode": mode }),
            );
        }
        self.prompt(text)
    }

    /// `then` on a task of its own: an answer is given with the agent held, which the frames it
    /// sends need.
    fn later(self: &Arc<Self>, then: impl FnOnce(&Arc<Self>) -> Result<()> + Send + 'static) {
        let me = self.clone();
        tokio::spawn(async move {
            if let Err(e) = then(&me) {
                log::warn!("agy: the turn didn't go on after the answers: {e:#}");
            }
        });
    }

    /// Asks its first program to start, and waits for its conversation.
    async fn open(self: &Arc<Self>, conversation: Option<&str>) -> Result<String> {
        let (tx, rx) = oneshot::channel();
        *self.ready.lock() = Some(tx);
        self.start_program(conversation)?;
        match tokio::time::timeout(READY_LIMIT, rx).await {
            Ok(Ok(id)) => Ok(id),
            Ok(Err(_)) => Err(anyhow!("{}", self.said())),
            Err(_) => Err(anyhow!("init: no answer in time")),
        }
    }

    fn said(&self) -> String {
        let tail = self.tail.lock();
        tail.iter()
            .rev()
            .find(|l| !l.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| "exited".into())
    }
}

impl EngineProcess for AgyProcess {
    fn is_alive(&self) -> bool {
        !self.exited.load(Ordering::Acquire)
    }

    fn usage(&self) -> Option<JobUsage> {
        self.current.lock().as_ref()?.job.as_ref()?.usage()
    }

    fn still_running(&self) -> u32 {
        let in_job = self.usage().map_or(0, |u| u.processes);
        in_job.max(u32::from(self.is_alive()))
    }

    fn pids(&self) -> Vec<u32> {
        self.current
            .lock()
            .as_ref()
            .and_then(|p| p.job.as_ref().map(|j| j.pids()))
            .unwrap_or_default()
    }

    /// Its input closed, its program ends the turn under way and exits; killed a moment later if
    /// it is still there. Its conversation is kept by `agy` at each step.
    fn close_input(&self) {
        self.closing.store(true, Ordering::Release);
        let had = {
            let mut c = self.current.lock();
            match c.as_mut() {
                Some(p) => {
                    p.tx.take();
                    true
                }
                None => false,
            }
        };
        let me = self.me.clone();
        match tokio::runtime::Handle::try_current() {
            Ok(rt) => {
                rt.spawn(async move {
                    if had {
                        tokio::time::sleep(CLOSE_GRACE).await;
                    }
                    if let Some(p) = me.upgrade().filter(|p| p.is_alive()) {
                        p.kill();
                    }
                });
            }
            Err(_) => self.kill(),
        }
    }

    fn kill(&self) {
        self.closing.store(true, Ordering::Release);
        let had = self.current.lock().is_some();
        if had {
            // Its reader tells the end once it is gone.
            if let Some(p) = self.current.lock().as_mut() {
                p.tx.take();
                if let Some(job) = &p.job {
                    job.terminate();
                }
                if let Some(k) = p.kill_tx.take() {
                    let _ = k.send(());
                }
            }
            return;
        }
        // No program running (between two): the end told on a task of its own, never under the
        // caller's locks.
        let me = self.me.clone();
        match tokio::runtime::Handle::try_current() {
            Ok(rt) => {
                rt.spawn(async move {
                    if let Some(p) = me.upgrade() {
                        p.finish(None);
                    }
                });
            }
            Err(_) => self.finish(None),
        }
    }

    fn wait_exit(&self, within: Duration) -> BoxFuture<'_, bool> {
        Box::pin(async move {
            let started = std::time::Instant::now();
            while self.is_alive() {
                if started.elapsed() >= within {
                    return false;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            true
        })
    }

    fn wait_gone(&self, within: Duration) -> BoxFuture<'_, bool> {
        self.wait_exit(within)
    }

    /// Its first program started (on the conversation to resume, if any), ready once it told its
    /// conversation (`init`). A conversation to resume that is gone gives a new one.
    fn initialize(&self) -> BoxFuture<'_, Result<Ready>> {
        Box::pin(async move {
            let me = self.self_arc()?;
            let resumed = match &self.opts.start {
                Start::Resume { session } => Some(session.clone()),
                _ => None,
            };
            let (id, how) = match me.open(resumed.as_deref()).await {
                Ok(id) => {
                    let how = if resumed.as_deref() == Some(id.as_str()) {
                        "resume"
                    } else {
                        "new"
                    };
                    (id, how)
                }
                Err(e) if resumed.is_some() => {
                    log::warn!("agy: conversation not resumed: {e:#}");
                    me.stop_program();
                    let id = me
                        .open(None)
                        .await
                        .map_err(|e| anyhow!(super::session_refused(i18n::ui(), &e)))?;
                    (id, "new")
                }
                Err(e) => {
                    self.kill();
                    return Err(anyhow!(super::session_refused(i18n::ui(), &e)));
                }
            };
            if how != "new" {
                self.state.lock().instructions = None;
            }
            self.frame("escouade.session", json!({ "sessionId": id, "start": how }));
            Ok(Ready {
                commands: Vec::new(),
                models: Value::Null,
                catalog: None,
            })
        })
    }

    /// A message sent while a turn runs (or waits for answers) waits for its end.
    fn send_message(&self, _uuid: &str, message: &Message) -> Result<()> {
        let text = message.0["text"].as_str().unwrap_or_default().to_string();
        {
            let mut s = self.state.lock();
            if s.busy {
                s.queue.push_back(text);
                return Ok(());
            }
        }
        self.self_arc()?.prompt(text)
    }

    /// `agy` has no message that stops a turn: its program is killed, the turn ended `cancelled`,
    /// and the next message starts it again on the conversation (resumed, tried in ESC-269).
    fn interrupt(&self, cancel_queued: bool, _timeout: Duration) -> BoxFuture<'_, Result<()>> {
        Box::pin(async move {
            let me = self.self_arc()?;
            let session = {
                let mut s = self.state.lock();
                if !s.busy {
                    return Ok(());
                }
                if cancel_queued {
                    s.queue.clear();
                }
                s.denied.clear();
                s.questions.clear();
                s.calls.clear();
                s.args.clear();
                s.plan_file = None;
                s.usage = Usage::default();
                s.session.clone()
            };
            me.stop_program();
            me.end_turn(json!({ "sessionId": session, "stopReason": "cancelled" }));
            Ok(())
        })
    }

    fn set_model<'a>(&'a self, model: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let mut s = self.state.lock();
            if s.settings.model != model {
                s.settings.model = model.to_string();
                s.stale = true;
            }
            Ok(())
        })
    }

    fn set_effort<'a>(&'a self, effort: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let mut s = self.state.lock();
            if s.settings.effort != effort {
                s.settings.effort = effort.to_string();
                s.stale = true;
            }
            Ok(())
        })
    }

    /// Its rules written at once (read again by `agy` before each tool), its `--mode` at the next
    /// start of its program.
    fn set_mode<'a>(&'a self, mode: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let changed = {
                let mut s = self.state.lock();
                let before = super::agy_mode(&s.settings.mode);
                s.settings.mode = mode.to_string();
                if before != super::agy_mode(mode) {
                    s.stale = true;
                }
                true
            };
            if changed {
                self.write_rules();
            }
            Ok(())
        })
    }

    fn answer_permission(&self, request_id: &str, answer: PermissionAnswer) -> Result<()> {
        let me = self.self_arc()?;
        let asked = self
            .state
            .lock()
            .asked
            .remove(request_id)
            .ok_or_else(|| anyhow!("Antigravity no longer waits for this answer"))?;
        match asked {
            Asked::Plan => {
                me.later(move |me| me.answer_plan(answer));
                Ok(())
            }
            Asked::Permission { rule, said } => {
                {
                    let mut s = self.state.lock();
                    match answer {
                        PermissionAnswer::Allow { .. } => {
                            s.once.push(rule);
                            s.answers.allowed.push(said);
                        }
                        PermissionAnswer::Always { .. } => {
                            if !s.always.contains(&rule) {
                                s.always.push(rule);
                                super::write_always(&self.opts.home, &s.always);
                            }
                            s.answers.allowed.push(said);
                        }
                        PermissionAnswer::Deny { message, .. } => {
                            s.answers.refused.push(said);
                            if worded(&message) {
                                s.answers.words.push(message);
                            }
                        }
                    }
                }
                me.later(|me| me.answered());
                Ok(())
            }
            Asked::Question { .. } => bail!("not a permission"),
        }
    }

    /// The answers of the card, by question: told to the model in the message that goes on.
    fn answer_question(&self, request_id: &str, _call: &str, input: Value) -> Result<()> {
        let me = self.self_arc()?;
        let asked = self
            .state
            .lock()
            .asked
            .remove(request_id)
            .ok_or_else(|| anyhow!("Antigravity no longer waits for this answer"))?;
        let Asked::Question { params } = asked else {
            bail!("not a question");
        };
        let replies = super::replies(&params, &input["answers"]);
        self.state.lock().answers.replies.extend(replies);
        me.later(|me| me.answered());
        Ok(())
    }

    /// `agy -p` tells no quota (its `/usage` is its interface's alone).
    fn read_usage(&self) -> BoxFuture<'_, Result<Option<Value>>> {
        Box::pin(async { Ok(None) })
    }

    fn rename<'a>(&'a self, _title: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async { Ok(()) })
    }

    fn stop_task<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async { Err(anyhow!(unsupported())) })
    }

    fn task_output<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<Value>> {
        Box::pin(async { Err(anyhow!(unsupported())) })
    }

    fn remote_control(&self, _: Remote, _: Duration) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async { Err(anyhow!(unsupported())) })
    }

    fn control(&self, _request: Value, _timeout: Duration) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async { Err(anyhow!(unsupported())) })
    }

    fn reply(&self, request_id: &str, _answer: std::result::Result<Value, String>) -> Result<()> {
        self.state.lock().asked.remove(request_id);
        Ok(())
    }
}

/// Whether a refusal's message says something of the user's own: not Escouade's words for a
/// refusal without one (`agent::refused_action`), which would start a turn nobody asked for.
fn worded(message: &str) -> bool {
    let m = message.trim();
    !m.is_empty()
        && ![Lang::Fr, Lang::En]
            .into_iter()
            .any(|l| m == crate::agent::refused_action(l))
        && !m.ends_with("via Escouade.")
        && !m.ends_with("through Escouade.")
}

fn unsupported() -> String {
    tr!(
        "Antigravity CLI ne le permet pas",
        "Antigravity CLI doesn’t support it"
    )
}

fn note(tail: &Mutex<VecDeque<String>>, line: String) {
    let mut t = tail.lock();
    t.push_back(line);
    if t.len() > 30 {
        t.pop_front();
    }
}

fn cannot_start(lang: Lang, program: &str) -> String {
    tr_in!(
        lang,
        "impossible de lancer {program}",
        "can’t start {program}"
    )
}
