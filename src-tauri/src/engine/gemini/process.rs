//! A `gemini --acp` process of Gemini CLI, driven over ACP v1 (JSON-RPC 2.0, one message a line on
//! its standard input and output; docs/TODO-1.8.md, point 0, « Gemini CLI »).
//!
//! As Vibe's (`engine::vibe::process`), the process answers its own requests and passes to
//! `on_frame` the updates of its session and the permissions Gemini asks, each under an id of this
//! process's own. Its own frames (`escouade.*`) tell the session it opened, the start and the end of
//! each prompt (with its tokens), and a mode Gemini changed to.
//!
//! What it does that Vibe's does not:
//! - the updates of its session are passed on while a prompt runs only: `session/load` replays the
//!   history after its answer too (tried, ESC-273);
//! - Gemini's text « [MODE_UPDATE] <mode> » is left out of the thread, read as its change of mode;
//! - a question's block (`<escouade_question>`) is kept out of the thread: the turn it ends on
//!   waits for the card's answers (`cursor/ask_question`), which go on in the same turn;
//! - the permission of its `exit_plan_mode` is given the plan, read from the file it names: the
//!   card « Plan à approuver ».
//!
//! As Vibe's, a refusal of the user's cuts the turn (`session/cancel`), its word sent after.

use super::{gemini_mode, QuestionFilter, Settings, ToldModel};
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
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

/// What an error of Gemini says (its `message`).
fn error_text(e: &Value) -> String {
    e["message"].as_str().unwrap_or("error").to_string()
}

/// The requests of the process waiting for Gemini's answer: its result, or its error.
type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, Value>>>>>;

/// How long a request of the process to Gemini may take (a prompt aside: a turn is not bounded).
const REQUEST_LIMIT: Duration = Duration::from_secs(60);

/// How long a process whose input was closed is given before it is killed (`close_input`).
const CLOSE_GRACE: Duration = Duration::from_secs(2);

/// The version of ACP Escouade speaks.
pub(super) const PROTOCOL: u64 = 1;

/// The method of a question held at the end of a turn (the process's own).
const QUESTION: &str = "cursor/ask_question";

/// A request waiting for its answer: its method, Gemini's own id (none for the process's own) and
/// its parameters.
#[derive(Debug, Clone)]
pub(super) struct Asked {
    pub method: String,
    pub id: Value,
    pub params: Value,
}

/// What the process knows of its conversation.
#[derive(Default)]
pub(super) struct State {
    pub session: Option<String>,
    pub busy: bool,
    pub queue: VecDeque<Value>,
    pub instructions: Option<String>,
    pub settings: Settings,
    pub models: Vec<ToldModel>,
    pub asked: HashMap<String, Asked>,
    /// The text of the prompt under way, a question's block kept out.
    pub filter: QuestionFilter,
    /// The prompt under way was cut by a refusal: no question is asked of it.
    pub cut: bool,
    /// The mode a plan approved goes on in (ESC-277), set once Gemini has left its mode plan
    /// (« [MODE_UPDATE] default »), else at the end of the turn.
    pub resume: Option<String>,
}

pub struct GeminiProcess {
    tx: Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>>,
    pending: Pending,
    next_id: AtomicU64,
    kill_tx: Mutex<Option<oneshot::Sender<()>>>,
    exited: Arc<AtomicBool>,
    job: Option<Arc<Job>>,
    state: Arc<Mutex<State>>,
    start: Start,
    cwd: String,
    escouade: Option<(String, String)>,
    fence: Vec<String>,
    tag: String,
    tail: Arc<Mutex<VecDeque<String>>>,
    on_frame: Arc<OnFrame>,
    me: std::sync::Weak<Self>,
}

/// How its process is started, and its conversation.
pub(super) struct Opts {
    pub program: Program,
    pub args: Vec<String>,
    pub cwd: String,
    pub env: Vec<(String, String)>,
    pub start: Start,
    pub settings: Settings,
    pub instructions: Option<String>,
    pub escouade: Option<(String, String)>,
    pub fence: Vec<String>,
}

fn line(msg: &Value) -> Vec<u8> {
    let mut out = msg.to_string().into_bytes();
    out.push(b'\n');
    out
}

impl GeminiProcess {
    pub(super) fn spawn(opts: Opts, on_frame: OnFrame, on_exit: OnExit) -> Result<Arc<Self>> {
        let Opts {
            program,
            args,
            cwd,
            env,
            start,
            settings,
            instructions,
            escouade,
            fence,
        } = opts;
        let launch = program.launch(&cwd, &args, &env)?;
        let mut cmd = Command::new(&launch.program);
        cmd.args(&launch.args)
            .current_dir(&cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        for (k, v) in &launch.env {
            cmd.env(k, v);
        }
        #[cfg(windows)]
        cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
        crate::job::isolate(&mut cmd);
        crate::proc::no_inherited_fds(&mut cmd);
        let mut child = cmd
            .spawn()
            .with_context(|| cannot_start(i18n::ui(), &program.describe()))?;
        let job = Job::for_child(&child).map(Arc::new);
        if job.is_none() {
            log::warn!(
                "gemini (pid {:?}) is not in a job: its tree is not killed as a whole",
                child.id()
            );
        }
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

        let tail = Arc::new(Mutex::new(VecDeque::<String>::new()));
        {
            let tail = tail.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).split(b'\n');
                while let Ok(Some(line)) = lines.next_segment().await {
                    let line = String::from_utf8_lossy(&line)
                        .trim_end_matches('\r')
                        .to_string();
                    log::debug!("gemini stderr: {line}");
                    note(&tail, line);
                }
            });
        }

        let pending: Pending = Arc::default();
        let state = Arc::new(Mutex::new(State {
            settings,
            instructions: instructions.filter(|i| !i.trim().is_empty()),
            ..Default::default()
        }));
        let on_frame = Arc::new(on_frame);
        let (kill_tx, mut kill_rx) = oneshot::channel::<()>();
        let exited = Arc::new(AtomicBool::new(false));
        let proc = Arc::new_cyclic(|me| Self {
            tx: Mutex::new(Some(tx)),
            pending: pending.clone(),
            next_id: AtomicU64::new(1),
            kill_tx: Mutex::new(Some(kill_tx)),
            exited: exited.clone(),
            job: job.clone(),
            state,
            start,
            cwd,
            escouade,
            fence,
            tag: format!("gemini-{}", &uuid::Uuid::new_v4().simple().to_string()[..8]),
            tail: tail.clone(),
            on_frame,
            me: me.clone(),
        });
        let weak = Arc::downgrade(&proc);
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
                                    Some(p) => p.read(msg),
                                    None => break,
                                },
                                Err(e) => log::warn!("unparsable gemini message ({e}), {} bytes", text.len()),
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
            exited.store(true, Ordering::Release);
            let said = tail.lock().iter().cloned().collect::<Vec<_>>().join("\n");
            on_exit(code, said);
            for (_, tx) in pending.lock().drain() {
                let _ = tx.send(Err("process exited".into()));
            }
            drop(job);
        });
        Ok(proc)
    }

    fn write(&self, msg: &Value) -> Result<()> {
        if !self.is_alive() {
            bail!("process exited");
        }
        let guard = self.tx.lock();
        let tx = guard
            .as_ref()
            .ok_or_else(|| anyhow!("process input closed"))?;
        tx.send(line(msg))
            .map_err(|_| anyhow!("process input closed"))
    }

    fn id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    async fn refuse(&self, reason: String) -> anyhow::Error {
        note(&self.tail, reason.clone());
        self.kill();
        self.wait_exit(Duration::from_secs(5)).await;
        anyhow!(reason)
    }

    /// A request to Gemini, its answer to come.
    pub(super) fn send(
        &self,
        method: &str,
        params: Value,
    ) -> Result<oneshot::Receiver<Result<Value, Value>>> {
        let id = self.id();
        let (tx, rx) = oneshot::channel();
        self.pending.lock().insert(id, tx);
        if let Err(e) =
            self.write(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
        {
            self.pending.lock().remove(&id);
            return Err(e);
        }
        Ok(rx)
    }

    /// A request to Gemini, its result.
    pub async fn request(&self, method: &str, params: Value, timeout: Duration) -> Result<Value> {
        let rx = self.send(method, params)?;
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(v))) => Ok(v),
            Ok(Ok(Err(e))) => Err(anyhow!("{method}: {}", error_text(&e))),
            Ok(Err(_)) => Err(anyhow!("{method}: dropped")),
            Err(_) => Err(anyhow!("{method}: no answer in time")),
        }
    }

    fn session(&self) -> Result<String> {
        self.state
            .lock()
            .session
            .clone()
            .ok_or_else(|| anyhow!("Gemini has no session yet"))
    }

    fn frame(&self, method: &str, params: Value) {
        (self.on_frame)(json!({ "method": method, "params": params }));
    }

    fn answer_gemini(&self, id: &Value, result: Value) {
        let _ = self.write(&json!({ "jsonrpc": "2.0", "id": id, "result": result }));
    }

    /// One message of Gemini: an answer to a request of the process, a request of Gemini, or a
    /// notification.
    fn read(self: &Arc<Self>, msg: Value) {
        let Some(method) = msg["method"].as_str().map(str::to_string) else {
            let Some(id) = msg["id"].as_u64() else {
                return;
            };
            let error = msg.get("error").filter(|e| !e.is_null());
            if let Some(tx) = self.pending.lock().remove(&id) {
                let _ = tx.send(match error {
                    Some(e) => Err(e.clone()),
                    None => Ok(msg["result"].clone()),
                });
            }
            return;
        };
        let mut params = msg["params"].clone();
        if !msg["id"].is_null() {
            if method == "session/request_permission" {
                // « Projet privé »: refused at once, never a card the user could accept.
                if crate::engine::cursor::fenced(&params, &self.fence) {
                    log::info!("gemini: a permission naming a path of another project refused (private project)");
                    let option = crate::engine::cursor::option_of(&params, "reject_once");
                    self.answer_gemini(
                        &msg["id"],
                        json!({ "outcome": { "outcome": "selected", "optionId": option } }),
                    );
                    return;
                }
                // The plan its `exit_plan_mode` asks to approve, read from its file.
                let tc = &params["toolCall"];
                if tc["toolCallId"].as_str().is_some_and(super::is_plan_exit) {
                    let file = tc["title"].as_str().and_then(super::plan_file);
                    let plan = file
                        .and_then(|f| std::fs::read_to_string(f).ok())
                        .unwrap_or_else(|| file.unwrap_or_default().to_string());
                    params["plan"] = json!(plan.trim());
                }
                let rid = format!("{}-{}", self.tag, msg["id"]);
                self.state.lock().asked.insert(
                    rid.clone(),
                    Asked {
                        method: method.clone(),
                        id: msg["id"].clone(),
                        params: params.clone(),
                    },
                );
                (self.on_frame)(json!({ "method": method, "id": rid, "params": params }));
                return;
            }
            log::debug!("gemini asks {method}: refused");
            let _ = self.write(&json!({
                "jsonrpc": "2.0", "id": msg["id"],
                "error": { "code": -32601, "message": format!("unsupported: {method}") },
            }));
            return;
        }
        if method != "session/update" {
            return;
        }
        let mut msg = msg;
        {
            let mut s = self.state.lock();
            // Its history replayed by `session/load`, before and after its answer, is not passed on.
            if !s.busy || params["sessionId"].as_str() != s.session.as_deref() {
                return;
            }
            let update = &params["update"];
            if update["sessionUpdate"] == "agent_message_chunk" {
                let text = update["content"]["text"].as_str().unwrap_or_default();
                // Its change of mode, told as a text.
                if let Some(mode) = super::mode_update(text) {
                    // Out of its mode plan: the mode the plan goes on in, now.
                    if mode == "default" {
                        if let Some(resume) = s.resume.take() {
                            drop(s);
                            self.resume_in(resume);
                            return;
                        }
                    }
                    let changed = s.settings.mode != mode
                        && !(mode == "default" && s.settings.mode.is_empty());
                    if changed {
                        s.settings.mode = mode.to_string();
                    }
                    let session = s.session.clone();
                    drop(s);
                    if changed {
                        self.frame(
                            "escouade.mode",
                            json!({ "sessionId": session, "mode": mode }),
                        );
                    }
                    return;
                }
                let shown = s.filter.push(text);
                if shown.is_empty() {
                    return;
                }
                msg["params"]["update"]["content"]["text"] = json!(shown);
            } else if update["sessionUpdate"] == "agent_thought_chunk"
                || update["sessionUpdate"] == "tool_call"
            {
                // What was held back of the text goes before what comes next.
                let held = s.filter.flush();
                if !held.is_empty() {
                    drop(s);
                    self.chunk(&params["sessionId"], &held);
                }
            }
        }
        (self.on_frame)(msg);
    }

    /// A piece of the agent's text, as Gemini would have sent it.
    fn chunk(&self, session: &Value, text: &str) {
        (self.on_frame)(json!({ "method": "session/update", "params": {
            "sessionId": session,
            "update": { "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": text } },
        } }));
    }

    /// The blocks of a prompt: the instructions still to give before them.
    fn with_instructions(&self, mut blocks: Value) -> Value {
        let instructions = self.state.lock().instructions.take();
        if let (Some(text), Some(list)) = (instructions, blocks.as_array_mut()) {
            list.insert(0, json!({ "type": "text", "text": text }));
        }
        blocks
    }

    /// Sends a prompt; its end, then the next message waiting, on a task of its own.
    fn prompt(self: &Arc<Self>, blocks: Value) -> Result<()> {
        let session = self.session()?;
        let blocks = self.with_instructions(blocks);
        {
            let mut s = self.state.lock();
            s.busy = true;
            s.filter = QuestionFilter::default();
            s.cut = false;
        }
        let rx = match self.send(
            "session/prompt",
            json!({ "sessionId": session, "prompt": blocks }),
        ) {
            Ok(rx) => rx,
            Err(e) => {
                self.state.lock().busy = false;
                return Err(e);
            }
        };
        let me = self.clone();
        tokio::spawn(async move {
            let got = rx.await;
            let (held, block, cut, resume) = {
                let mut s = me.state.lock();
                (
                    s.filter.flush(),
                    s.filter.take_block(),
                    s.cut,
                    s.resume.take(),
                )
            };
            if let Some(mode) = resume {
                me.resume_in(mode);
            }
            if !held.is_empty() {
                me.chunk(&json!(session), &held);
            }
            // A turn that ended on a question waits for its answers.
            if let (Ok(Ok(r)), Some(block), false) = (&got, &block, cut) {
                let call = format!("{session}-question-{}", me.id());
                match super::question_params(&call, block) {
                    Some(params) if r["stopReason"] == "end_turn" => {
                        let rid = format!("{}-q-{}", me.tag, me.id());
                        me.frame("escouade.spent", json!({ "usage": r["usage"] }));
                        me.state.lock().asked.insert(
                            rid.clone(),
                            Asked {
                                method: QUESTION.into(),
                                id: Value::Null,
                                params: params.clone(),
                            },
                        );
                        (me.on_frame)(json!({ "method": QUESTION, "id": rid, "params": params }));
                        return;
                    }
                    // A block that is no question: shown as it was written.
                    _ => me.chunk(&json!(session), block),
                }
            }
            let next = {
                let mut s = me.state.lock();
                s.busy = false;
                s.queue.pop_front()
            };
            match got {
                Ok(Ok(r)) => me.frame(
                    "escouade.prompt_end",
                    json!({ "sessionId": session, "stopReason": r["stopReason"], "usage": r["usage"] }),
                ),
                // Its code tells a rate limit (429) from the other errors.
                Ok(Err(e)) => me.frame(
                    "escouade.prompt_end",
                    json!({ "sessionId": session, "error": error_text(&e), "code": e["code"] }),
                ),
                Err(_) => return,
            }
            if let Some(blocks) = next {
                me.frame("escouade.prompt_start", json!({ "sessionId": session }));
                if let Err(e) = me.prompt(blocks) {
                    log::warn!("gemini: a message waiting was not sent: {e:#}");
                }
            }
        });
        Ok(())
    }

    fn mcp_servers(&self) -> Value {
        crate::engine::cursor::mcp_servers(self.escouade.as_ref())
    }

    /// Opens the agent's session as `start` says: its id, how it was opened (`new`, `resume`), and
    /// what Gemini answered (its modes and models). A session to resume that is gone, or a copy
    /// (Gemini has no `session/fork`), gives a new one.
    async fn open(&self) -> Result<(String, &'static str, Value)> {
        if let Start::Resume { session } = &self.start {
            let params =
                json!({ "sessionId": session, "cwd": self.cwd, "mcpServers": self.mcp_servers() });
            match self.request("session/load", params, REQUEST_LIMIT).await {
                Ok(r) => return Ok((session.clone(), "resume", r)),
                Err(e) => log::warn!("gemini: session {session} not loaded: {e:#}"),
            }
        }
        let r = self
            .request(
                "session/new",
                json!({ "cwd": self.cwd, "mcpServers": self.mcp_servers() }),
                REQUEST_LIMIT,
            )
            .await?;
        let id = r["sessionId"]
            .as_str()
            .ok_or_else(|| anyhow!("session/new: no session"))?
            .to_string();
        Ok((id, "new", r))
    }

    /// The model given to the session (`session/set_model`): one the session does not offer
    /// (another harness's, after a handover) is left to Gemini's default.
    async fn apply_model(&self, session: &str) -> Result<()> {
        let model = {
            let s = self.state.lock();
            let listed = s.models.iter().any(|m| m.id == s.settings.model);
            listed.then(|| s.settings.model.clone())
        };
        if let Some(model) = model {
            self.request(
                "session/set_model",
                json!({ "sessionId": session, "modelId": model }),
                REQUEST_LIMIT,
            )
            .await?;
        }
        Ok(())
    }

    async fn apply_mode(&self, session: &str, mode: &str) -> Result<()> {
        self.request(
            "session/set_mode",
            json!({ "sessionId": session, "modeId": gemini_mode(mode) }),
            REQUEST_LIMIT,
        )
        .await
        .map(|_| ())
    }

    fn answer(&self, rid: &str, result: Value) -> Result<Asked> {
        let Some(asked) = self.state.lock().asked.remove(rid) else {
            bail!("Gemini no longer waits for this answer");
        };
        self.write(&json!({ "jsonrpc": "2.0", "id": asked.id, "result": result }))?;
        Ok(asked)
    }

    fn asked(&self, rid: &str) -> Result<Asked> {
        self.state
            .lock()
            .asked
            .get(rid)
            .cloned()
            .ok_or_else(|| anyhow!("Gemini no longer waits for this answer"))
    }

    fn self_arc(&self) -> Result<Arc<Self>> {
        self.me.upgrade().ok_or_else(|| anyhow!("process gone"))
    }

    /// The mode a plan approved goes on in, set in Gemini on a task of its own.
    fn resume_in(self: &Arc<Self>, mode: String) {
        let me = self.clone();
        tokio::spawn(async move {
            let Ok(session) = me.session() else {
                return;
            };
            if let Err(e) = me.apply_mode(&session, &mode).await {
                log::warn!("gemini: mode {mode} not set after the plan: {e:#}");
            }
            me.state.lock().settings.mode = mode.clone();
        });
    }

    /// The end of a turn held for a question that is no longer asked.
    fn end_held(self: &Arc<Self>, session: &str, reason: &str) {
        let next = {
            let mut s = self.state.lock();
            s.busy = false;
            s.queue.pop_front()
        };
        self.frame(
            "escouade.prompt_end",
            json!({ "sessionId": session, "stopReason": reason }),
        );
        if let Some(blocks) = next {
            self.frame("escouade.prompt_start", json!({ "sessionId": session }));
            if let Err(e) = self.prompt(blocks) {
                log::warn!("gemini: a message waiting was not sent: {e:#}");
            }
        }
    }
}

impl EngineProcess for GeminiProcess {
    fn is_alive(&self) -> bool {
        !self.exited.load(Ordering::Acquire)
    }

    fn usage(&self) -> Option<JobUsage> {
        self.job.as_ref()?.usage()
    }

    fn still_running(&self) -> u32 {
        let in_job = self.usage().map_or(0, |u| u.processes);
        in_job.max(u32::from(self.is_alive()))
    }

    fn pids(&self) -> Vec<u32> {
        self.job.as_ref().map(|j| j.pids()).unwrap_or_default()
    }

    /// Its input closed, it is killed a moment later if it is still there; its session is kept in
    /// its `GEMINI_CLI_HOME` as it goes (`chats/`).
    fn close_input(&self) {
        self.tx.lock().take();
        let me = self.me.clone();
        match tokio::runtime::Handle::try_current() {
            Ok(rt) => {
                rt.spawn(async move {
                    tokio::time::sleep(CLOSE_GRACE).await;
                    if let Some(p) = me.upgrade().filter(|p| p.is_alive()) {
                        p.kill();
                    }
                });
            }
            Err(_) => self.kill(),
        }
    }

    fn kill(&self) {
        self.tx.lock().take();
        if let Some(job) = &self.job {
            job.terminate();
        }
        if let Some(k) = self.kill_tx.lock().take() {
            let _ = k.send(());
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
        Box::pin(async move {
            let started = std::time::Instant::now();
            if !self.wait_exit(within).await {
                return false;
            }
            #[cfg(windows)]
            while self.usage().is_some_and(|u| u.processes > 0) {
                if started.elapsed() >= within {
                    return false;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            #[cfg(not(windows))]
            let _ = started;
            true
        })
    }

    /// The handshake, the session (new or resumed) on its mode and model.
    fn initialize(&self) -> BoxFuture<'_, Result<Ready>> {
        Box::pin(async move {
            let lang = i18n::ui();
            let hello = self
                .request("initialize", super::hello(), REQUEST_LIMIT)
                .await?;
            if hello["protocolVersion"].as_u64() != Some(PROTOCOL) {
                return Err(self
                    .refuse(super::protocol_refused(lang, &hello["protocolVersion"]))
                    .await);
            }
            let (session, how, opened) = match self.open().await {
                Ok(opened) => opened,
                Err(e) => return Err(self.refuse(super::session_refused(lang, &e)).await),
            };
            let models = super::models_of(&opened["models"]);
            let mode = {
                let mut s = self.state.lock();
                s.session = Some(session.clone());
                if how != "new" {
                    s.instructions = None;
                }
                s.models = models.clone();
                s.settings.mode.clone()
            };
            if opened["modes"]["currentModeId"].as_str() != Some(gemini_mode(&mode)) {
                if let Err(e) = self.apply_mode(&session, &mode).await {
                    log::warn!("gemini: mode {mode} not applied: {e:#}");
                }
            }
            if let Err(e) = self.apply_model(&session).await {
                log::warn!("gemini: model not applied: {e:#}");
            }
            self.frame(
                "escouade.session",
                json!({ "sessionId": session, "start": how }),
            );
            Ok(Ready {
                commands: Vec::new(),
                models: Value::Null,
                catalog: Some(super::catalog(&models)),
            })
        })
    }

    /// A message sent while a turn runs waits for its end: a second `session/prompt` would cancel
    /// the first (read in its code).
    fn send_message(&self, _uuid: &str, message: &Message) -> Result<()> {
        let blocks = message.0["prompt"].clone();
        {
            let mut s = self.state.lock();
            if s.busy {
                s.queue.push_back(blocks);
                return Ok(());
            }
        }
        let me = self.self_arc()?;
        me.prompt(blocks)
    }

    /// `session/cancel`: the prompt ends `cancelled` at once (tried); a question waiting for its
    /// answers ends the turn.
    fn interrupt(&self, cancel_queued: bool, _timeout: Duration) -> BoxFuture<'_, Result<()>> {
        Box::pin(async move {
            let Ok(session) = self.session() else {
                return Ok(());
            };
            if cancel_queued {
                self.state.lock().queue.clear();
            }
            let held: Vec<String> = self
                .state
                .lock()
                .asked
                .iter()
                .filter(|(_, a)| a.method == QUESTION)
                .map(|(k, _)| k.clone())
                .collect();
            if !held.is_empty() {
                for rid in held {
                    self.state.lock().asked.remove(&rid);
                }
                let me = self.self_arc()?;
                me.end_held(&session, "cancelled");
                return Ok(());
            }
            self.write(&json!({
                "jsonrpc": "2.0", "method": "session/cancel", "params": { "sessionId": session },
            }))
        })
    }

    fn set_model<'a>(&'a self, model: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            self.state.lock().settings.model = model.to_string();
            let session = self.session()?;
            self.apply_model(&session).await
        })
    }

    /// No effort to set from ACP (its thinking is in its settings, read at its start).
    fn set_effort<'a>(&'a self, _effort: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async { Ok(()) })
    }

    fn set_mode<'a>(&'a self, mode: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            self.state.lock().settings.mode = mode.to_string();
            let session = self.session()?;
            self.apply_mode(&session, mode).await
        })
    }

    fn answer_permission(&self, request_id: &str, answer: PermissionAnswer) -> Result<()> {
        let asked = self.asked(request_id)?;
        let plan = asked.params["toolCall"]["toolCallId"]
            .as_str()
            .is_some_and(super::is_plan_exit);
        let (kind, cut) = match &answer {
            PermissionAnswer::Allow { .. } => ("allow_once", false),
            // « Toujours » on a plan: carried out with its writes accepted.
            PermissionAnswer::Always { .. } if plan => ("allow_once", false),
            PermissionAnswer::Always { .. } => ("allow_always", false),
            PermissionAnswer::Deny { .. } => ("reject_once", true),
        };
        let option = crate::engine::cursor::option_of(&asked.params, kind);
        self.answer(
            request_id,
            json!({ "outcome": { "outcome": "selected", "optionId": option } }),
        )?;
        if plan && !cut {
            // Gemini goes back to its `default` once the plan is approved: the mode it was in
            // before Plan (ESC-277), or `autoEdit` for « Toujours », is set once it has.
            let mode = match &answer {
                PermissionAnswer::Allow { resume, .. } => {
                    resume.clone().unwrap_or_else(|| "default".into())
                }
                _ => "acceptEdits".to_string(),
            };
            {
                let mut s = self.state.lock();
                s.settings.mode = mode.clone();
                s.resume = (mode != "default").then(|| mode.clone());
            }
            let session = self.session()?;
            let me = self.self_arc()?;
            // Told on a task of its own: the answer is given with the agent held.
            tokio::spawn(async move {
                me.frame(
                    "escouade.mode",
                    json!({ "sessionId": session, "mode": mode }),
                );
            });
        }
        // A refusal cuts the turn, its word (if any) sent once it has ended.
        if cut {
            let session = self.session()?;
            if let PermissionAnswer::Deny { message, .. } = answer {
                if worded(&message) {
                    self.state
                        .lock()
                        .queue
                        .push_front(json!([{ "type": "text", "text": message }]));
                }
            }
            self.state.lock().cut = true;
            self.write(&json!({
                "jsonrpc": "2.0", "method": "session/cancel", "params": { "sessionId": session },
            }))?;
        }
        Ok(())
    }

    /// The card's answers by question, told to the model in a prompt that goes on in the same
    /// turn.
    fn answer_question(&self, request_id: &str, _call: &str, input: Value) -> Result<()> {
        let Some(asked) = self.state.lock().asked.remove(request_id) else {
            bail!("Gemini no longer waits for this answer");
        };
        if asked.method != QUESTION {
            bail!("not a question");
        }
        let text = super::answers_message(&asked.params, &input["answers"]);
        let me = self.self_arc()?;
        // Sent on a task of its own: the answer is given with the agent held, which its frames need.
        tokio::spawn(async move {
            if let Err(e) = me.prompt(json!([{ "type": "text", "text": text }])) {
                log::warn!("gemini: the answers were not sent: {e:#}");
                if let Ok(session) = me.session() {
                    me.end_held(&session, "end_turn");
                }
            }
        });
        Ok(())
    }

    /// ACP tells no quota.
    fn read_usage(&self) -> BoxFuture<'_, Result<Option<Value>>> {
        Box::pin(async { Ok(None) })
    }

    /// Its sessions have no title in ACP.
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

    fn control(&self, request: Value, timeout: Duration) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            let method = request["method"].as_str().unwrap_or_default().to_string();
            self.request(&method, request["params"].clone(), timeout)
                .await
        })
    }

    fn reply(&self, request_id: &str, answer: std::result::Result<Value, String>) -> Result<()> {
        match answer {
            Ok(result) => self.answer(request_id, result).map(|_| ()),
            Err(message) => {
                let Some(asked) = self.state.lock().asked.remove(request_id) else {
                    bail!("Gemini no longer waits for this answer");
                };
                self.write(&json!({
                    "jsonrpc": "2.0", "id": asked.id, "error": { "code": -32601, "message": message },
                }))
            }
        }
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
        "Gemini CLI ne le permet pas",
        "Gemini CLI doesn’t support it"
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
