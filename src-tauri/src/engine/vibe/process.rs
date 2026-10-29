//! A `vibe-acp` process of Mistral Vibe, driven over ACP v1 (JSON-RPC 2.0, one message a line on
//! its standard input and output; docs/TODO-1.8.md, point 0, « Mistral Vibe »).
//!
//! As Cursor's (`engine::cursor::process`), the process answers its own requests and passes to
//! `on_frame` the updates of its session and the requests Vibe makes of the client that wait for
//! an answer (`session/request_permission`, `elicitation/create`), each under an id of this
//! process's own. Its own frames (`escouade.*`) tell the session it opened, the start and the end
//! of each prompt, a mode it set, and the plan of a turn in Escouade's mode Plan.
//!
//! What it does that Cursor's does not:
//! - Vibe's permission names its call alone (`toolCall.toolCallId`): the call as its `tool_call`
//!   told it (its command, its file, its diff) is put back in it, for the fence and the card;
//! - Vibe has no mode Plan: in Escouade's, its session is in `ask`, each prompt says to plan
//!   without changing anything, its writes are refused, and the turn's last words are the plan to
//!   approve (`escouade/plan`, the card « Plan à approuver »), approved by a prompt that has it
//!   carried out in the mode chosen;
//! - a refusal of the user's cuts the turn (`session/cancel`): Vibe would otherwise try the command
//!   again under other forms, each asking again (nine times in the real run, ESC-265).

use super::{vibe_mode, Settings, ToldModel};
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

/// What an error of Vibe says (its `message`).
fn error_text(e: &Value) -> String {
    e["message"].as_str().unwrap_or("error").to_string()
}

/// The requests of the process waiting for Vibe's answer: its result, or its error (`code`,
/// `message`).
type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, Value>>>>>;

/// How long a request of the process to Vibe may take (a prompt aside: a turn is not bounded).
const REQUEST_LIMIT: Duration = Duration::from_secs(60);

/// How long a process whose input was closed is given before it is killed (`close_input`).
const CLOSE_GRACE: Duration = Duration::from_secs(2);

/// The version of ACP Escouade speaks.
pub(super) const PROTOCOL: u64 = 1;

/// The requests of Vibe that wait for the user's answer.
const ASKED: [&str; 2] = ["session/request_permission", "elicitation/create"];

use super::PLAN_REQUEST as PLAN;

/// Vibe's « Always allow », kept in its `VIBE_HOME`.
const PERMANENT: &str = "allow_always_permanent";

/// The most calls kept for the permissions that name them (`State::calls`).
const CALLS_KEPT: usize = 200;

/// A request waiting for its answer: its method, Vibe's own id (none for the process's own) and
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
    /// `session/load` replays its history: what comes meanwhile is not passed on.
    pub loading: bool,
    pub instructions: Option<String>,
    pub settings: Settings,
    pub models: Vec<ToldModel>,
    /// The thinking levels the session offers (`thinking`: `off`, `high`).
    pub thinking: Vec<String>,
    pub asked: HashMap<String, Asked>,
    /// The calls of the session as their `tool_call` told them, by id (the last `CALLS_KEPT`).
    pub calls: HashMap<String, Value>,
    pub call_order: VecDeque<String>,
    /// The agent's words in the prompt under way (`agent_message_chunk`): a plan's text.
    pub words: String,
    /// The prompt under way was cut by a refusal: no plan is asked of it.
    pub cut: bool,
}

pub struct VibeProcess {
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

/// `call` with what `update` tells of it (its keys over the call's).
fn merged(call: &mut Value, update: &Value) {
    if !call.is_object() {
        *call = json!({});
    }
    for (k, v) in update.as_object().into_iter().flatten() {
        if !v.is_null() && k != "sessionUpdate" {
            call[k] = v.clone();
        }
    }
}

impl VibeProcess {
    pub(super) fn spawn(opts: Opts, on_frame: OnFrame, on_exit: OnExit) -> Result<Arc<Self>> {
        let Opts {
            program,
            cwd,
            env,
            start,
            settings,
            instructions,
            escouade,
            fence,
        } = opts;
        let launch = program.launch(&cwd, &[], &env)?;
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
                "vibe-acp (pid {:?}) is not in a job: its tree is not killed as a whole",
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
                    log::debug!("vibe stderr: {line}");
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
            tag: format!("vibe-{}", &uuid::Uuid::new_v4().simple().to_string()[..8]),
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
                                Err(e) => log::warn!("unparsable vibe message ({e}), {} bytes", text.len()),
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

    fn send(&self, method: &str, params: Value) -> Result<oneshot::Receiver<Result<Value, Value>>> {
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

    /// A request to Vibe, its result.
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
            .ok_or_else(|| anyhow!("Vibe has no session yet"))
    }

    fn frame(&self, method: &str, params: Value) {
        (self.on_frame)(json!({ "method": method, "params": params }));
    }

    fn answer_vibe(&self, id: &Value, result: Value) {
        let _ = self.write(&json!({ "jsonrpc": "2.0", "id": id, "result": result }));
    }

    /// One message of Vibe: an answer to a request of the process, a request of Vibe, or a
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
                // The call as its `tool_call` told it: Vibe's permission names it alone.
                let call = params["toolCall"]["toolCallId"]
                    .as_str()
                    .and_then(|id| self.state.lock().calls.get(id).cloned());
                if let Some(mut call) = call {
                    merged(&mut call, &params["toolCall"]);
                    params["toolCall"] = call;
                }
                // « Projet privé »: refused at once, never a card the user could accept.
                if crate::engine::cursor::fenced(&params, &self.fence) {
                    log::info!("vibe: a permission naming a path of another project refused (private project)");
                    let option = crate::engine::cursor::option_of(&params, "reject_once");
                    self.answer_vibe(
                        &msg["id"],
                        json!({ "outcome": { "outcome": "selected", "optionId": option } }),
                    );
                    return;
                }
                // Escouade's mode Plan changes nothing: a write is refused, the model told so.
                let planning = self.state.lock().settings.mode == "plan";
                if planning
                    && matches!(
                        params["toolCall"]["kind"].as_str(),
                        Some("edit" | "delete" | "move")
                    )
                {
                    log::info!("vibe: a write refused in the mode Plan");
                    let option = crate::engine::cursor::option_of(&params, "reject_once");
                    self.answer_vibe(
                        &msg["id"],
                        json!({ "outcome": { "outcome": "selected", "optionId": option } }),
                    );
                    return;
                }
            }
            if ASKED.contains(&method.as_str()) {
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
            log::debug!("vibe asks {method}: refused");
            let _ = self.write(&json!({
                "jsonrpc": "2.0", "id": msg["id"],
                "error": { "code": -32601, "message": format!("unsupported: {method}") },
            }));
            return;
        }
        if method != "session/update" {
            return;
        }
        {
            let mut s = self.state.lock();
            let update = &params["update"];
            // What the session spent so far, told as it opens (before its id is known, or while
            // it is loaded): where its first turn's cost starts (ESC-267).
            let opening = s.loading || s.session.is_none();
            if opening && update["sessionUpdate"] == "usage_update" {
                drop(s);
                (self.on_frame)(msg);
                return;
            }
            if s.loading || params["sessionId"].as_str() != s.session.as_deref() {
                return;
            }
            match update["sessionUpdate"].as_str() {
                Some("tool_call" | "tool_call_update") => {
                    if let Some(id) = update["toolCallId"].as_str() {
                        if !s.calls.contains_key(id) {
                            s.call_order.push_back(id.to_string());
                            if s.call_order.len() > CALLS_KEPT {
                                if let Some(old) = s.call_order.pop_front() {
                                    s.calls.remove(&old);
                                }
                            }
                        }
                        let call = s.calls.entry(id.to_string()).or_insert(Value::Null);
                        merged(call, update);
                    }
                }
                Some("agent_message_chunk") => {
                    if let Some(t) = update["content"]["text"].as_str() {
                        s.words.push_str(t);
                    }
                }
                // The narration between two tools is reasoning: the words are the last ones.
                Some("agent_thought_chunk") if !s.words.is_empty() => {
                    s.words.push_str("\n\n");
                }
                _ => {}
            }
        }
        (self.on_frame)(msg);
    }

    /// The blocks of a prompt: the instructions still to give before them, and in the mode Plan
    /// what that mode asks.
    fn with_instructions(&self, mut blocks: Value) -> Value {
        let (instructions, planning) = {
            let mut s = self.state.lock();
            (s.instructions.take(), s.settings.mode == "plan")
        };
        let heads = [
            instructions.map(|i| crate::engine::cursor::preamble(&i)),
            planning.then(|| super::PLAN_MODE.to_string()),
        ];
        if let Some(list) = blocks.as_array_mut() {
            for (at, text) in heads.into_iter().flatten().enumerate() {
                list.insert(at, json!({ "type": "text", "text": text }));
            }
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
            s.words.clear();
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
            // In the mode Plan, a turn that ended on its words has its plan approved first.
            let plan = {
                let s = me.state.lock();
                match &got {
                    Ok(Ok(r))
                        if r["stopReason"] == "end_turn"
                            && s.settings.mode == "plan"
                            && !s.cut
                            && !s.words.trim().is_empty() =>
                    {
                        Some(s.words.trim().to_string())
                    }
                    _ => None,
                }
            };
            if let Some(plan) = plan {
                let rid = format!("{}-plan-{}", me.tag, me.id());
                let params = json!({ "sessionId": session, "plan": plan });
                me.state.lock().asked.insert(
                    rid.clone(),
                    Asked {
                        method: PLAN.into(),
                        id: Value::Null,
                        params: params.clone(),
                    },
                );
                (me.on_frame)(json!({ "method": PLAN, "id": rid, "params": params }));
                return;
            }
            let next = {
                let mut s = me.state.lock();
                s.busy = false;
                s.queue.pop_front()
            };
            match got {
                Ok(Ok(r)) => me.frame(
                    "escouade.prompt_end",
                    json!({ "sessionId": session, "stopReason": r["stopReason"] }),
                ),
                // Its code tells a rate limit (`RATE_LIMITED`) from the other errors.
                Ok(Err(e)) => me.frame(
                    "escouade.prompt_end",
                    json!({ "sessionId": session, "error": error_text(&e), "code": e["code"] }),
                ),
                Err(_) => return,
            }
            if let Some(blocks) = next {
                me.frame("escouade.prompt_start", json!({ "sessionId": session }));
                if let Err(e) = me.prompt(blocks) {
                    log::warn!("vibe: a message waiting was not sent: {e:#}");
                }
            }
        });
        Ok(())
    }

    /// The end of a turn whose plan was sent back without a word: the turn ends there.
    fn end_held(self: &Arc<Self>, session: &str) {
        let next = {
            let mut s = self.state.lock();
            s.busy = false;
            s.queue.pop_front()
        };
        self.frame(
            "escouade.prompt_end",
            json!({ "sessionId": session, "stopReason": "end_turn" }),
        );
        if let Some(blocks) = next {
            self.frame("escouade.prompt_start", json!({ "sessionId": session }));
            if let Err(e) = self.prompt(blocks) {
                log::warn!("vibe: a message waiting was not sent: {e:#}");
            }
        }
    }

    fn mcp_servers(&self) -> Value {
        crate::engine::cursor::mcp_servers(self.escouade.as_ref())
    }

    /// Opens the agent's session as `start` says: its id, how it was opened (`new`, `resume`,
    /// `fork`), and what Vibe answered (its modes and options). A session to resume that is gone
    /// gives a new one.
    async fn open(&self) -> Result<(String, &'static str, Value)> {
        match &self.start {
            Start::Resume { session } => {
                let params = json!({ "sessionId": session, "cwd": self.cwd, "mcpServers": self.mcp_servers() });
                self.state.lock().loading = true;
                let got = self.request("session/load", params, REQUEST_LIMIT).await;
                self.state.lock().loading = false;
                match got {
                    Ok(r) => return Ok((session.clone(), "resume", r)),
                    Err(e) => log::warn!("vibe: session {session} not loaded: {e:#}"),
                }
            }
            // A copy of all of it: Vibe's `session/fork` takes no point (the original is idle).
            Start::Fork { of, .. } => {
                let params =
                    json!({ "sessionId": of, "cwd": self.cwd, "mcpServers": self.mcp_servers() });
                match self.request("session/fork", params, REQUEST_LIMIT).await {
                    Ok(r) => {
                        if let Some(id) = r["sessionId"].as_str() {
                            return Ok((id.to_string(), "fork", r));
                        }
                    }
                    Err(e) => log::warn!("vibe: session {of} not copied: {e:#}"),
                }
            }
            Start::New => {}
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

    /// The model and its thinking given to the session (`session/set_config_option`).
    async fn apply_model(&self, session: &str) -> Result<()> {
        // A model the session does not offer (another harness's, after a handover) is left to
        // Vibe's default.
        let (model, thinking) = {
            let s = self.state.lock();
            let thinking = super::thinking_of(&s.settings.effort, &s.thinking);
            let listed = s.models.iter().any(|m| m.id == s.settings.model);
            (listed.then(|| s.settings.model.clone()), thinking)
        };
        if let Some(model) = model {
            self.request(
                "session/set_config_option",
                json!({ "sessionId": session, "configId": "model", "value": model }),
                REQUEST_LIMIT,
            )
            .await?;
        }
        if let Some(thinking) = thinking {
            self.request(
                "session/set_config_option",
                json!({ "sessionId": session, "configId": "thinking", "value": thinking }),
                REQUEST_LIMIT,
            )
            .await?;
        }
        Ok(())
    }

    /// Vibe's agent for Escouade's `mode`: with a fence, Bypass stays in `ask`, whose asks
    /// Escouade's rules accept but those naming its paths. Its `auto-approve` lets every call
    /// through, the `denylist` of its file tools too (read in Vibe 3.0.3, `ToolGate.BYPASS`, and
    /// tried: a file of the fence read).
    fn agent_of(&self, mode: &str) -> &'static str {
        match vibe_mode(mode) {
            "auto-approve" if !self.fence.is_empty() => "ask",
            agent => agent,
        }
    }

    async fn apply_mode(&self, session: &str, mode: &str) -> Result<()> {
        self.request(
            "session/set_mode",
            json!({ "sessionId": session, "modeId": self.agent_of(mode) }),
            REQUEST_LIMIT,
        )
        .await
        .map(|_| ())
    }

    fn answer(&self, rid: &str, result: Value) -> Result<Asked> {
        let Some(asked) = self.state.lock().asked.remove(rid) else {
            bail!("Vibe no longer waits for this answer");
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
            .ok_or_else(|| anyhow!("Vibe no longer waits for this answer"))
    }

    fn self_arc(&self) -> Result<Arc<Self>> {
        self.me.upgrade().ok_or_else(|| anyhow!("process gone"))
    }

    /// The plan of the mode Plan answered: approved, it is carried out in the mode chosen, in the
    /// same turn; sent back with a word, the planning goes on with it; without, the turn ends.
    fn answer_plan(&self, rid: &str, answer: PermissionAnswer) -> Result<()> {
        let Some(asked) = self.state.lock().asked.remove(rid) else {
            bail!("Vibe no longer waits for this answer");
        };
        let session = asked.params["sessionId"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let me = self.self_arc()?;
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
        if mode.is_none() && text.trim().is_empty() {
            me.end_held(&session);
            return Ok(());
        }
        if let Some(mode) = &mode {
            self.state.lock().settings.mode = mode.clone();
        }
        // Set and sent on a task of its own: the answer is given with the agent held, which its
        // frames need.
        tokio::spawn(async move {
            if let Some(mode) = mode {
                if let Err(e) = me.apply_mode(&session, &mode).await {
                    log::warn!("vibe: mode {mode} not set after the plan: {e:#}");
                }
                me.frame(
                    "escouade.mode",
                    json!({ "sessionId": session, "mode": mode }),
                );
            }
            if let Err(e) = me.prompt(json!([{ "type": "text", "text": text }])) {
                log::warn!("vibe: the plan's answer was not sent: {e:#}");
                me.end_held(&session);
            }
        });
        Ok(())
    }
}

impl EngineProcess for VibeProcess {
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
    /// its `VIBE_HOME` at each turn (resumed after, tried).
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

    /// The handshake (the questions asked for: `elicitation.form`), the session (new, resumed or
    /// copied) on its mode, model and thinking.
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
            let models = super::models_of(&opened["configOptions"]);
            let thinking = super::thinking_levels(&opened["configOptions"]);
            let mode = {
                let mut s = self.state.lock();
                s.session = Some(session.clone());
                if how != "new" {
                    s.instructions = None;
                }
                s.models = models.clone();
                s.thinking = thinking;
                s.settings.mode.clone()
            };
            let current = opened["modes"]["currentModeId"]
                .as_str()
                .or(super::option_value(&opened["configOptions"], "mode"));
            if current != Some(self.agent_of(&mode)) {
                if let Err(e) = self.apply_mode(&session, &mode).await {
                    log::warn!("vibe: mode {mode} not applied: {e:#}");
                }
            }
            if let Err(e) = self.apply_model(&session).await {
                log::warn!("vibe: model not applied: {e:#}");
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

    /// A message sent while a turn runs waits for its end.
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

    /// `session/cancel`: the prompt ends `cancelled` at once (tried); a plan waiting for its
    /// answer ends the turn.
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
                .filter(|(_, a)| a.method == PLAN)
                .map(|(k, _)| k.clone())
                .collect();
            if !held.is_empty() {
                for rid in held {
                    self.state.lock().asked.remove(&rid);
                }
                let me = self.self_arc()?;
                me.state.lock().busy = false;
                me.frame(
                    "escouade.prompt_end",
                    json!({ "sessionId": session, "stopReason": "cancelled" }),
                );
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

    fn set_effort<'a>(&'a self, effort: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            self.state.lock().settings.effort = effort.to_string();
            let session = self.session()?;
            self.apply_model(&session).await
        })
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
        if asked.method == PLAN {
            return self.answer_plan(request_id, answer);
        }
        let (kind, cut) = match &answer {
            PermissionAnswer::Allow { .. } => ("allow_once", false),
            PermissionAnswer::Always { .. } => ("allow_always", false),
            PermissionAnswer::Deny { .. } => ("reject_once", true),
        };
        // « Toujours » for this agent: Vibe writes its rule in the agent's own `VIBE_HOME`
        // (`allow_always_permanent`), as Cursor in its own folder; its other `allow_always` is for
        // the session alone.
        let permanent = kind == "allow_always"
            && asked.params["options"]
                .as_array()
                .is_some_and(|o| o.iter().any(|x| x["optionId"] == PERMANENT));
        let option = if permanent {
            PERMANENT.to_string()
        } else {
            crate::engine::cursor::option_of(&asked.params, kind)
        };
        self.answer(
            request_id,
            json!({ "outcome": { "outcome": "selected", "optionId": option } }),
        )?;
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

    /// `input.answers` are the card's answers by question: given back by the keys of Vibe's form
    /// (`q0`, `q1`…), a choice by its value, several as a list; an answer written is taken as it
    /// is (Vibe accepts it, tried).
    fn answer_question(&self, request_id: &str, _call: &str, input: Value) -> Result<()> {
        let asked = self.asked(request_id)?;
        let content = super::elicitation_content(&asked.params, &input["answers"]);
        self.answer(
            request_id,
            json!({ "action": "accept", "content": content }),
        )
        .map(|_| ())
    }

    /// ACP tells no quota (its cost comes with ESC-267).
    fn read_usage(&self) -> BoxFuture<'_, Result<Option<Value>>> {
        Box::pin(async { Ok(None) })
    }

    /// Its title is Vibe's own (`session_info_update`).
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
                    bail!("Vibe no longer waits for this answer");
                };
                self.write(&json!({
                    "jsonrpc": "2.0", "id": asked.id, "error": { "code": -32601, "message": message },
                }))
            }
        }
    }
}

/// Whether a refusal's message says something of the user's own: not Escouade's words for a
/// refusal without one (`agent::refused_action`, « Refusé par … via Escouade. »), which would start
/// a turn nobody asked for.
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
        "Mistral Vibe ne le permet pas",
        "Mistral Vibe doesn’t support it"
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
