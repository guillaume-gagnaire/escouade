//! An `agent acp` process of Cursor CLI, driven over ACP v1 (JSON-RPC 2.0, one message a line on
//! its standard input and output; docs/TODO-1.8.md, point 0, « Cursor CLI »).
//!
//! The process answers its own requests (`initialize`, `session/new`, `session/load`…) and passes
//! to `on_frame` the updates of its session and of its subagents (`session/update`, each under its
//! own `sessionId`, the subagents' told by `subagent_spawned`), the requests Cursor makes of the
//! client that wait for an answer (`session/request_permission`, `cursor/ask_question`,
//! `cursor/create_plan`: kept until answered, each under an id of this process's own, Cursor's
//! starting again at 0 with each process), and those whose answer it ignores
//! (`cursor/update_todos`, `cursor/task`: answered at once). Its own frames (`escouade.*`) tell the
//! session it opened, the start and the end of each prompt, and a mode it set.
//!
//! A message sent while a turn runs waits for its end: a second `session/prompt` would cancel the
//! running turn (tried on 2026.10.22). The history `session/load` replays before it answers is not
//! passed on: the conversation has it already.

use super::{acp_mode, Settings, ToldOf};
use crate::engine::{
    BoxFuture, EngineProcess, Message, OnExit, OnFrame, PermissionAnswer, Program, Ready, Remote,
    Start,
};
use crate::i18n::{self, Lang};
use crate::job::{Job, JobUsage};
use anyhow::{anyhow, bail, Context, Result};
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>;

/// How long a request of the process to Cursor may take (a prompt aside: a turn is not bounded).
const REQUEST_LIMIT: Duration = Duration::from_secs(60);

/// How long a process whose input was closed is given before it is killed (`close_input`).
const CLOSE_GRACE: Duration = Duration::from_secs(2);

/// The version of ACP Escouade speaks.
pub(super) const PROTOCOL: u64 = 1;

/// The variables that make Cursor's shell Git Bash (Escouade started from Git Bash): its hooks,
/// wrapped in PowerShell, then fail and block every turn (tried on 2026.10.22).
pub(super) const GIT_BASH_VARS: [&str; 2] = ["MSYSTEM", "EXEPATH"];

/// The requests of Cursor that wait for the user's answer.
const ASKED: [&str; 3] = [
    "session/request_permission",
    "cursor/ask_question",
    "cursor/create_plan",
];

/// A request of Cursor waiting for its answer: its method, its own id and its parameters.
#[derive(Debug, Clone)]
pub(super) struct Asked {
    pub method: String,
    pub id: Value,
    pub params: Value,
}

/// What the process knows of its conversation.
#[derive(Default)]
pub(super) struct State {
    /// Its session (the agent's), once opened.
    pub session: Option<String>,
    /// The sessions of its subagents (`subagent_spawned`).
    pub children: HashSet<String>,
    /// A prompt sent, its answer not come.
    pub busy: bool,
    /// The messages sent while a turn ran, each a prompt's blocks.
    pub queue: VecDeque<Value>,
    /// `session/load` replays its history: what comes meanwhile is not passed on.
    pub loading: bool,
    /// The instructions added to the agent, still to go with its next prompt (a new session's
    /// first: `session/load` keeps them, tried).
    pub instructions: Option<String>,
    /// What was said in the agent's interface (« Vue Cursor »), which the session does not have:
    /// still to go with its next prompt, after the instructions.
    pub relay: Option<String>,
    pub settings: Settings,
    /// The models of the account, as `session/new` listed them.
    pub models: Vec<ToldOf>,
    /// The requests of Cursor waiting for an answer, by the id this process gave them.
    pub asked: HashMap<String, Asked>,
}

pub struct CursorProcess {
    tx: Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>>,
    pending: Pending,
    next_id: AtomicU64,
    kill_tx: Mutex<Option<oneshot::Sender<()>>>,
    exited: Arc<AtomicBool>,
    job: Option<Arc<Job>>,
    state: Arc<Mutex<State>>,
    start: Start,
    cwd: String,
    /// Escouade's server, as the session is given it (`mcpServers`): its address and token.
    escouade: Option<(String, String)>,
    /// The paths its tools are refused (« Projet privé »), as it names them: a permission it asks
    /// that names one is refused at once (`fenced`).
    fence: Vec<String>,
    /// The prefix of the ids of Cursor's requests, this process's own.
    tag: String,
    /// The last lines of its standard error: what its exit tells in the conversation.
    tail: Arc<Mutex<VecDeque<String>>>,
    on_frame: Arc<OnFrame>,
    /// Itself, for what goes on on a task of its own (`prompt`, the mode after a plan).
    me: std::sync::Weak<Self>,
}

/// How its process is started, and its conversation.
pub(super) struct Opts {
    pub program: Program,
    pub cwd: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub start: Start,
    pub settings: Settings,
    pub instructions: Option<String>,
    pub relay: Option<String>,
    pub escouade: Option<(String, String)>,
    pub fence: Vec<String>,
}

/// The program and the arguments `program` is started with: the `node` and the `index.js` of its
/// newest version for the launcher its installer left (`locate::native_of`), else itself.
pub(super) fn command_of(
    program: &Program,
    cwd: &str,
    args: &[String],
    env: &[(String, String)],
) -> Result<crate::engine::Launch> {
    if let Program::Native(launcher) = program {
        if let Some((node, index)) = super::locate::native_of(launcher) {
            let mut all = vec![index.to_string_lossy().into_owned()];
            all.extend_from_slice(args);
            return Ok(crate::engine::Launch {
                program: node,
                args: all,
                env: env.to_vec(),
            });
        }
    }
    program.launch(cwd, args, env)
}

/// `msg` as Cursor reads it: one line.
fn line(msg: &Value) -> Vec<u8> {
    let mut out = msg.to_string().into_bytes();
    out.push(b'\n');
    out
}

impl CursorProcess {
    pub(super) fn spawn(opts: Opts, on_frame: OnFrame, on_exit: OnExit) -> Result<Arc<Self>> {
        let Opts {
            program,
            cwd,
            args,
            env,
            start,
            settings,
            instructions,
            relay,
            escouade,
            fence,
        } = opts;
        let launch = command_of(&program, &cwd, &args, &env)?;
        let mut cmd = Command::new(&launch.program);
        cmd.args(&launch.args)
            .current_dir(&cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        for var in GIT_BASH_VARS {
            cmd.env_remove(var);
        }
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
                "cursor agent (pid {:?}) is not in a job: its tree is not killed as a whole",
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
                    log::debug!("cursor stderr: {line}");
                    note(&tail, line);
                }
            });
        }

        let pending: Pending = Arc::default();
        let state = Arc::new(Mutex::new(State {
            settings,
            instructions: instructions.filter(|i| !i.trim().is_empty()),
            relay: relay.filter(|r| !r.trim().is_empty()),
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
            tag: format!("cursor-{}", &uuid::Uuid::new_v4().simple().to_string()[..8]),
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
                                Err(e) => log::warn!("unparsable cursor message ({e}), {} bytes", text.len()),
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

    /// Refused to start for `reason`: the process stops, its exit telling `reason` in the
    /// conversation (as its standard error's last line).
    async fn refuse(&self, reason: String) -> anyhow::Error {
        note(&self.tail, reason.clone());
        self.kill();
        self.wait_exit(Duration::from_secs(5)).await;
        anyhow!(reason)
    }

    /// Sends the request `method`; its answer comes on the receiver.
    fn send(
        &self,
        method: &str,
        params: Value,
    ) -> Result<oneshot::Receiver<Result<Value, String>>> {
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

    /// A request to Cursor, its result.
    pub async fn request(&self, method: &str, params: Value, timeout: Duration) -> Result<Value> {
        let rx = self.send(method, params)?;
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(v))) => Ok(v),
            Ok(Ok(Err(e))) => Err(anyhow!("{method}: {e}")),
            Ok(Err(_)) => Err(anyhow!("{method}: dropped")),
            Err(_) => Err(anyhow!("{method}: no answer in time")),
        }
    }

    fn session(&self) -> Result<String> {
        self.state
            .lock()
            .session
            .clone()
            .ok_or_else(|| anyhow!("Cursor has no session yet"))
    }

    fn frame(&self, method: &str, params: Value) {
        (self.on_frame)(json!({ "method": method, "params": params }));
    }

    /// One message of Cursor: an answer to a request of the process, a request of Cursor, or a
    /// notification.
    fn read(self: &Arc<Self>, msg: Value) {
        let Some(method) = msg["method"].as_str().map(str::to_string) else {
            let Some(id) = msg["id"].as_u64() else {
                return;
            };
            let error = msg.get("error").filter(|e| !e.is_null());
            if let Some(tx) = self.pending.lock().remove(&id) {
                let _ = tx.send(match error {
                    Some(e) => Err(e["message"].as_str().unwrap_or("error").to_string()),
                    None => Ok(msg["result"].clone()),
                });
            }
            return;
        };
        let params = &msg["params"];
        if !msg["id"].is_null() {
            // A request of Cursor to the client.
            // « Projet privé »: refused at once, never a card the user could accept.
            if method == "session/request_permission" && super::fenced(params, &self.fence) {
                log::info!("cursor: a permission naming a path of another project refused (private project)");
                let option = super::option_of(params, "reject_once");
                let _ = self.write(&json!({
                    "jsonrpc": "2.0", "id": msg["id"],
                    "result": { "outcome": { "outcome": "selected", "optionId": option } },
                }));
                return;
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
            match method.as_str() {
                // Told for the record: Cursor reads no answer of theirs.
                "cursor/update_todos" | "cursor/task" | "cursor/generate_image" => {
                    let _ = self.write(&json!({ "jsonrpc": "2.0", "id": msg["id"], "result": {} }));
                    (self.on_frame)(json!({ "method": method, "params": params }));
                }
                other => {
                    log::debug!("cursor asks {other}: refused");
                    let _ = self.write(&json!({
                        "jsonrpc": "2.0", "id": msg["id"],
                        "error": { "code": -32601, "message": format!("unsupported: {other}") },
                    }));
                }
            }
            return;
        }
        if method != "session/update" {
            return;
        }
        let own = {
            let mut s = self.state.lock();
            if s.loading {
                return;
            }
            let update = &params["update"];
            if update["sessionUpdate"] == "subagent_spawned" {
                if let Some(child) = update["subagentSessionId"].as_str() {
                    s.children.insert(child.to_string());
                }
            }
            params["sessionId"]
                .as_str()
                .is_some_and(|id| s.session.as_deref() == Some(id) || s.children.contains(id))
        };
        if own {
            (self.on_frame)(msg);
        }
    }

    /// The blocks of a prompt, the instructions and the relay still to give put before them.
    fn with_instructions(&self, mut blocks: Value) -> Value {
        let (instructions, relay) = {
            let mut s = self.state.lock();
            (s.instructions.take(), s.relay.take())
        };
        let heads = [
            instructions.map(|i| super::preamble(&i)),
            relay.map(|r| super::relayed(&r)),
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
        self.state.lock().busy = true;
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
                Ok(Err(e)) => me.frame(
                    "escouade.prompt_end",
                    json!({ "sessionId": session, "error": e }),
                ),
                // The process is gone: its exit tells it.
                Err(_) => return,
            }
            if let Some(blocks) = next {
                me.frame("escouade.prompt_start", json!({ "sessionId": session }));
                if let Err(e) = me.prompt(blocks) {
                    log::warn!("cursor: a message waiting was not sent: {e:#}");
                }
            }
        });
        Ok(())
    }

    /// What `session/new` and `session/load` are given of MCP servers: Escouade's, as the agent
    /// (its token in a header; ACP passes it on the process's input, never its command line).
    fn mcp_servers(&self) -> Value {
        super::mcp_servers(self.escouade.as_ref())
    }

    /// Opens the agent's session as `start` says: its id, how it was opened (`new`, `resume`), and
    /// what Cursor answered (its modes and models). A session to resume that is gone gives a new
    /// one. Cursor once failed a first `session/load` (« Failed to initialize session services »)
    /// that went through at the second try: it is tried twice.
    async fn open(&self) -> Result<(String, &'static str, Value)> {
        if let Start::Resume { session } = &self.start {
            let params =
                json!({ "sessionId": session, "cwd": self.cwd, "mcpServers": self.mcp_servers() });
            for attempt in 0..2 {
                self.state.lock().loading = true;
                let got = self
                    .request("session/load", params.clone(), REQUEST_LIMIT)
                    .await;
                self.state.lock().loading = false;
                match got {
                    Ok(r) => return Ok((session.clone(), "resume", r)),
                    Err(e) => {
                        log::warn!("cursor: session {session} not loaded: {e:#}");
                        if e.to_string().contains("not found") || attempt == 1 {
                            break;
                        }
                    }
                }
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

    /// The model and effort of `settings` given to the session: the model by its full id among
    /// those listed (its options in it), then its effort by the option it has for it. Cursor writes
    /// the model in its config (`CURSOR_CONFIG_DIR`, the agent's own): it is given at each start.
    async fn apply_model(&self, session: &str) -> Result<()> {
        let (model, effort, told) = {
            let s = self.state.lock();
            let told = s
                .models
                .iter()
                .find(|m| m.base == s.settings.model)
                .cloned();
            (s.settings.model.clone(), s.settings.effort.clone(), told)
        };
        if model.is_empty() {
            return Ok(());
        }
        let full = told
            .as_ref()
            .map_or_else(|| super::full_id(&model), |t| t.full.clone());
        self.request(
            "session/set_config_option",
            json!({ "sessionId": session, "configId": "model", "value": full }),
            REQUEST_LIMIT,
        )
        .await?;
        if let Some(key) = told
            .and_then(|t| t.effort_key)
            .filter(|_| !effort.is_empty())
        {
            self.request(
                "session/set_config_option",
                json!({ "sessionId": session, "configId": key, "value": effort }),
                REQUEST_LIMIT,
            )
            .await?;
        }
        Ok(())
    }

    async fn apply_mode(&self, session: &str, mode: &str) -> Result<()> {
        self.request(
            "session/set_mode",
            json!({ "sessionId": session, "modeId": acp_mode(mode) }),
            REQUEST_LIMIT,
        )
        .await
        .map(|_| ())
    }

    /// The models of the account: those of `session/new` (`listed`), with the efforts of each from
    /// `cursor/list_available_models` when it answers.
    async fn models(&self, listed: &Value) -> Vec<ToldOf> {
        let options = self
            .request("cursor/list_available_models", json!({}), REQUEST_LIMIT)
            .await
            .ok();
        super::models_of(listed, options.as_ref())
    }

    /// Answers the request `rid` of Cursor with `result`.
    fn answer(&self, rid: &str, result: Value) -> Result<Asked> {
        let Some(asked) = self.state.lock().asked.remove(rid) else {
            bail!("Cursor no longer waits for this answer");
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
            .ok_or_else(|| anyhow!("Cursor no longer waits for this answer"))
    }
}

impl EngineProcess for CursorProcess {
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

    /// Its input closed; `agent acp` does not exit for it (tried on 2026.10.22): it is killed a moment
    /// later, its session kept in its config folder at each turn (resumed after, tried).
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

    /// The handshake (the subagents asked for), the session (new or resumed) on its mode and
    /// model, then the models of the account.
    fn initialize(&self) -> BoxFuture<'_, Result<Ready>> {
        Box::pin(async move {
            let lang = i18n::ui();
            let hello = self
                .request(
                    "initialize",
                    json!({
                        "protocolVersion": PROTOCOL,
                        "clientCapabilities": {
                            // Announced `true`, Cursor still reads and writes by itself, outside
                            // the folder too: it never asks `fs/*` of its client (tried on
                            // 2026.10.22, ESC-43, and read in its code). Nothing to take them for.
                            "fs": { "readTextFile": false, "writeTextFile": false },
                            "terminal": false,
                            "_meta": { "subagents": true },
                        },
                        "clientInfo": { "name": "escouade", "version": env!("CARGO_PKG_VERSION") },
                    }),
                    REQUEST_LIMIT,
                )
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
            {
                let mut s = self.state.lock();
                s.session = Some(session.clone());
                // A resumed session has its instructions already.
                if how == "resume" {
                    s.instructions = None;
                }
            }
            let models = self.models(&opened["models"]).await;
            self.state.lock().models = models.clone();
            let mode = self.state.lock().settings.mode.clone();
            if opened["modes"]["currentModeId"].as_str() != Some(acp_mode(&mode)) {
                if let Err(e) = self.apply_mode(&session, &mode).await {
                    log::warn!("cursor: mode {mode} not applied: {e:#}");
                }
            }
            if let Err(e) = self.apply_model(&session).await {
                log::warn!("cursor: model not applied: {e:#}");
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

    /// A message sent while a turn runs waits for its end (a second prompt would cancel it).
    fn send_message(&self, _uuid: &str, message: &Message) -> Result<()> {
        let blocks = message.0["prompt"].clone();
        {
            let mut s = self.state.lock();
            if s.busy {
                s.queue.push_back(blocks);
                return Ok(());
            }
        }
        // `prompt` needs the process by its `Arc`: the task that reads its end keeps it.
        let me = self.self_arc()?;
        me.prompt(blocks)
    }

    /// `session/cancel`: the prompt ends `cancelled` half a second later, its command stopped.
    fn interrupt(&self, cancel_queued: bool, _timeout: Duration) -> BoxFuture<'_, Result<()>> {
        Box::pin(async move {
            let Ok(session) = self.session() else {
                return Ok(());
            };
            if cancel_queued {
                self.state.lock().queue.clear();
            }
            self.write(&json!({
                "jsonrpc": "2.0", "method": "session/cancel", "params": { "sessionId": session },
            }))
        })
    }

    /// At once, even during a turn (`session/set_config_option`).
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
        if asked.method == "cursor/create_plan" {
            // The plan approved: its work starts in the mode Agent (its edits accepted with
            // « Approuver et accepter les modifications »); sent back, the planning goes on.
            let (outcome, mode) = match &answer {
                PermissionAnswer::Allow { resume, .. } => (
                    json!({ "outcome": "accepted" }),
                    Some(resume.clone().unwrap_or_else(|| "default".into())),
                ),
                PermissionAnswer::Always { .. } => (
                    json!({ "outcome": "accepted" }),
                    Some("acceptEdits".to_string()),
                ),
                PermissionAnswer::Deny { message, .. } => {
                    (json!({ "outcome": "rejected", "reason": message }), None)
                }
            };
            self.answer(request_id, json!({ "outcome": outcome }))?;
            if let Some(mode) = mode {
                self.state.lock().settings.mode = mode.clone();
                let session = self.session()?;
                // Told and set on a task of its own: the answer is given with the agent held,
                // which its frames need.
                let me = self.self_arc()?;
                tokio::spawn(async move {
                    if let Err(e) = me.apply_mode(&session, &mode).await {
                        log::warn!("cursor: mode {mode} not set after the plan: {e:#}");
                    }
                    me.frame(
                        "escouade.mode",
                        json!({ "sessionId": session, "mode": mode }),
                    );
                });
            }
            return Ok(());
        }
        let kind = match &answer {
            PermissionAnswer::Allow { .. } => "allow_once",
            PermissionAnswer::Always { .. } => "allow_always",
            PermissionAnswer::Deny { .. } => "reject_once",
        };
        let option = super::option_of(&asked.params, kind);
        self.answer(
            request_id,
            json!({ "outcome": { "outcome": "selected", "optionId": option } }),
        )
        .map(|_| ())
    }

    /// `input.questions` are the card's, `input.answers` its answers by question: each answer
    /// given back by the ids of its choices; one that names none of them (written in the message
    /// field) skips the questions with it as its reason, which the model reads.
    fn answer_question(&self, request_id: &str, _call: &str, input: Value) -> Result<()> {
        let asked = self.asked(request_id)?;
        let outcome = super::question_outcome(&asked.params, &input["answers"]);
        self.answer(request_id, json!({ "outcome": outcome }))
            .map(|_| ())
    }

    /// Nothing to read in ACP (its usage of the month is in its interface only, `/usage`).
    fn read_usage(&self) -> BoxFuture<'_, Result<Option<Value>>> {
        Box::pin(async { Ok(None) })
    }

    /// ACP names no session: its title is Cursor's own (`session_info_update`).
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

    /// A request in ACP: `{"method", "params"}`.
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
                    bail!("Cursor no longer waits for this answer");
                };
                self.write(&json!({
                    "jsonrpc": "2.0", "id": asked.id, "error": { "code": -32601, "message": message },
                }))
            }
        }
    }
}

impl CursorProcess {
    /// The process by its `Arc`, for what goes on on a task of its own.
    fn self_arc(&self) -> Result<Arc<Self>> {
        self.me.upgrade().ok_or_else(|| anyhow!("process gone"))
    }
}

fn unsupported() -> String {
    tr!("Cursor ne le permet pas", "Cursor doesn’t support it")
}

/// Keeps `line` among the last 30 of `tail`.
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
