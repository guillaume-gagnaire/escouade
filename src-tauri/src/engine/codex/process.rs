//! A `codex app-server` process driven over its JSON-RPC protocol (one message a line on stdio,
//! docs/PROTOCOL.md, « Codex app-server »).
//!
//! The process answers its own requests (`initialize`, `thread/start`…) and passes everything
//! else to `on_frame`: the notifications of the thread and of its subagents, and the requests
//! Codex makes (approvals, questions), which the translation turns into cards and the answers to
//! which come back through `answer_permission` and `answer_question`. It keeps what a turn needs
//! that the core does not hold: its thread, the turn running, and the model, effort and mode of
//! the next turn (Codex takes them at `turn/start`).

use super::{mode_params, Settings};
use crate::engine::{
    BoxFuture, EngineProcess, Message, OnExit, OnFrame, PermissionAnswer, Program, Ready, Remote,
    Start,
};
use crate::i18n::{self, Lang};
use crate::job::{Job, JobUsage};
use crate::text::truncate;
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

type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>;

/// How long a request of the process to Codex may take.
const REQUEST_LIMIT: Duration = Duration::from_secs(60);

/// How many pages of `model/list` are read at most (a cursor that never ends ends there).
const MODEL_PAGES: usize = 50;

/// What the process knows of its conversation.
#[derive(Default)]
pub(super) struct State {
    /// Its thread (the agent's session), once `thread/start`, `thread/resume` or `thread/fork`
    /// answered.
    pub thread: Option<String>,
    /// The turn of its thread under way, from `turn/started` to `turn/completed`.
    pub turn: Option<String>,
    /// A `turn/start` sent, its `turn/started` not come yet: a message meanwhile waits for it.
    pub starting: bool,
    /// Messages that came while a turn was starting: steered into it once it has its id.
    pub held: Vec<Value>,
    /// The model, effort and mode of the next turn.
    pub settings: Settings,
    /// The requests Codex made that wait for their answer: their method and parameters, by id.
    pub asked: HashMap<String, (Value, String, Value)>,
}

/// The ids of the requests sent without waiting for their answer (`turn/start`, `turn/steer`): a
/// refusal of theirs is passed on as an `error` notification, for the turn it was to start.
type Fired = Arc<Mutex<HashMap<u64, &'static str>>>;

pub struct CodexProcess {
    tx: Mutex<Option<mpsc::UnboundedSender<String>>>,
    pending: Pending,
    fired: Fired,
    next_id: AtomicU64,
    kill_tx: Mutex<Option<oneshot::Sender<()>>>,
    exited: Arc<AtomicBool>,
    job: Option<Arc<Job>>,
    state: Arc<Mutex<State>>,
    /// Where it starts from, and what `thread/start` is given.
    start: Start,
    cwd: String,
    instructions: Option<String>,
    /// Where the turns of its thread pass on what they say: `turn/started` and `turn/completed`
    /// are read here first, for the turn under way.
    on_frame: Arc<OnFrame>,
}

/// How its app-server is started, and its conversation.
pub(super) struct Opts {
    pub program: Program,
    pub cwd: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub start: Start,
    pub settings: Settings,
    /// The instructions added to a new thread (`developerInstructions`).
    pub instructions: Option<String>,
}

impl CodexProcess {
    pub(super) fn spawn(opts: Opts, on_frame: OnFrame, on_exit: OnExit) -> Result<Arc<Self>> {
        let Opts {
            program,
            cwd,
            args,
            env,
            start,
            settings,
            instructions,
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
        // Its sandbox's commands and their runner stay in it (tried, docs/TODO-1.8.md, point 0).
        let job = Job::for_child(&child).map(Arc::new);
        if job.is_none() {
            log::warn!(
                "codex (pid {:?}) is not in a job: its tree is not killed as a whole",
                child.id()
            );
        }
        let mut stdin = child.stdin.take().context("stdin")?;
        let stdout = child.stdout.take().context("stdout")?;
        let stderr = child.stderr.take().context("stderr")?;

        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        tokio::spawn(async move {
            while let Some(line) = rx.recv().await {
                if stdin.write_all(line.as_bytes()).await.is_err()
                    || stdin.write_all(b"\n").await.is_err()
                    || stdin.flush().await.is_err()
                {
                    break;
                }
            }
            // Its input closed: the app-server ends once what it does is over.
        });

        let tail = Arc::new(Mutex::new(VecDeque::<String>::new()));
        {
            let tail = tail.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).split(b'\n');
                while let Ok(Some(line)) = lines.next_segment().await {
                    let line = text_line(&line);
                    log::debug!("codex stderr: {line}");
                    note(&tail, line);
                }
            });
        }

        let pending: Pending = Arc::default();
        let fired: Fired = Arc::default();
        let state: Arc<Mutex<State>> = Arc::new(Mutex::new(State {
            settings,
            ..Default::default()
        }));
        let on_frame = Arc::new(on_frame);
        let (kill_tx, mut kill_rx) = oneshot::channel::<()>();
        let exited = Arc::new(AtomicBool::new(false));
        let proc = Arc::new(Self {
            tx: Mutex::new(Some(tx)),
            pending: pending.clone(),
            fired: fired.clone(),
            next_id: AtomicU64::new(1),
            kill_tx: Mutex::new(Some(kill_tx)),
            exited: exited.clone(),
            job: job.clone(),
            state: state.clone(),
            start,
            cwd,
            instructions,
            on_frame: on_frame.clone(),
        });
        let weak = Arc::downgrade(&proc);
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).split(b'\n');
            loop {
                tokio::select! {
                    line = lines.next_segment() => match line {
                        Ok(Some(line)) => {
                            let line = text_line(&line);
                            if line.trim().is_empty() {
                                continue;
                            }
                            match crate::text::json_line(&line) {
                                Ok(msg) => {
                                    let held = read(&msg, &pending, &fired, &state, &on_frame);
                                    // Messages held while the turn started go into it now.
                                    if let (Some(p), false) = (weak.upgrade(), held.is_empty()) {
                                        for input in held {
                                            let _ = p.steer(input);
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::warn!("unparsable codex line ({e}): {}", truncate(&line, 200));
                                    note(&tail, truncate(&line, 500));
                                }
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
        tx.send(msg.to_string())
            .map_err(|_| anyhow!("process input closed"))
    }

    fn id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Every page of `model/list` (ESC-186: it gives them by pages, `nextCursor`), as one answer
    /// (`data`), up to `MODEL_PAGES` of them.
    pub async fn list_models(&self) -> Result<Value> {
        let mut data = Vec::new();
        let mut cursor: Option<String> = None;
        for _ in 0..MODEL_PAGES {
            let params = match &cursor {
                Some(c) => json!({ "cursor": c }),
                None => json!({}),
            };
            let page = self.request("model/list", params, REQUEST_LIMIT).await?;
            data.extend(page["data"].as_array().cloned().unwrap_or_default());
            cursor = page["nextCursor"].as_str().map(str::to_string);
            if cursor.is_none() {
                return Ok(json!({ "data": data }));
            }
        }
        log::warn!("codex: model/list: more than {MODEL_PAGES} pages, the rest left out");
        Ok(json!({ "data": data }))
    }

    /// A request to Codex, its result.
    pub async fn request(&self, method: &str, params: Value, timeout: Duration) -> Result<Value> {
        let id = self.id();
        let (tx, rx) = oneshot::channel();
        self.pending.lock().insert(id, tx);
        if let Err(e) = self.write(&json!({ "id": id, "method": method, "params": params })) {
            self.pending.lock().remove(&id);
            return Err(e);
        }
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(v))) => Ok(v),
            Ok(Ok(Err(e))) => Err(anyhow!("{method}: {e}")),
            Ok(Err(_)) => Err(anyhow!("{method}: dropped")),
            Err(_) => {
                self.pending.lock().remove(&id);
                Err(anyhow!("{method}: no answer in time"))
            }
        }
    }

    /// A request whose answer is not waited for: a refusal of it becomes an `error` frame.
    fn fire(&self, method: &'static str, params: Value) -> Result<()> {
        let id = self.id();
        self.fired.lock().insert(id, method);
        self.write(&json!({ "id": id, "method": method, "params": params }))
            .inspect_err(|_| {
                self.fired.lock().remove(&id);
            })
    }

    fn thread(&self) -> Result<String> {
        self.state
            .lock()
            .thread
            .clone()
            .ok_or_else(|| anyhow!("Codex has no thread yet"))
    }

    /// Starts a turn with `input`, as the settings of the next turn say.
    fn start_turn(&self, input: Value) -> Result<()> {
        let thread = self.thread()?;
        let params = {
            let mut s = self.state.lock();
            s.starting = true;
            let mut p = mode_params(&s.settings, &self.cwd);
            p["threadId"] = json!(thread);
            p["input"] = input;
            p["model"] = json!(s.settings.model);
            if !s.settings.effort.is_empty() {
                p["effort"] = json!(s.settings.effort);
            }
            p["summary"] = json!("auto");
            p
        };
        self.fire("turn/start", params).inspect_err(|_| {
            self.state.lock().starting = false;
        })
    }

    /// `input` given to the turn under way (`turn/steer`); held until it has its id when it is
    /// just starting; a turn of its own when none runs.
    fn steer(&self, input: Value) -> Result<()> {
        let (thread, turn) = {
            let mut s = self.state.lock();
            match (&s.turn, s.starting) {
                (Some(t), _) => (s.thread.clone(), t.clone()),
                (None, true) => {
                    s.held.push(input);
                    return Ok(());
                }
                (None, false) => {
                    drop(s);
                    return self.start_turn(input);
                }
            }
        };
        self.fire(
            "turn/steer",
            json!({ "threadId": thread, "input": input, "expectedTurnId": turn }),
        )
    }

    /// Answers the request `id` Codex made.
    fn respond(&self, id: &Value, result: Value) -> Result<()> {
        self.write(&json!({ "id": id, "result": result }))
    }

    /// The parameters of `thread/start`, `thread/resume` or `thread/fork`.
    fn thread_request(&self) -> (&'static str, Value) {
        let s = self.state.lock();
        let mut p = mode_params(&s.settings, &self.cwd);
        // Taken at the thread's level: `collaborationMode` is a turn's.
        if let Some(o) = p.as_object_mut() {
            o.remove("collaborationMode");
            o.remove("sandboxPolicy");
        }
        p["cwd"] = json!(self.cwd);
        p["model"] = json!(s.settings.model);
        // Fenced, its mode's profile (`mode_params`), declared by its process's options
        // (`fence_args`); none in « Sans permissions », whose sandbox is none.
        if p.get("permissions").is_none() {
            p["sandbox"] = json!(super::sandbox_mode(s.settings.sandboxed()));
        }
        match &self.start {
            Start::New => {
                if let Some(i) = &self.instructions {
                    p["developerInstructions"] = json!(i);
                }
                ("thread/start", p)
            }
            // Its added instructions are in its rollout already (tried: they survive a resume).
            Start::Resume { session } => {
                p["threadId"] = json!(session);
                ("thread/resume", p)
            }
            Start::Fork { of, at } => {
                p["threadId"] = json!(of);
                if let Some(at) = at {
                    p["lastTurnId"] = json!(at);
                }
                ("thread/fork", p)
            }
        }
    }
}

/// The quota of an account none of whose agents runs (`account/rateLimits/read`): an app-server of
/// its own (`env`: its `CODEX_HOME`), started in `cwd` for that and stopped once it answered. It
/// starts no thread, and costs little (0,6 s and 62 Mo on Codex 0.163.0, tried).
pub(super) async fn read_limits(
    program: Program,
    env: Vec<(String, String)>,
    cwd: String,
) -> Result<Value> {
    let opts = Opts {
        program,
        cwd,
        args: vec!["app-server".into()],
        env,
        start: Start::New,
        settings: Settings::default(),
        instructions: None,
    };
    let p = CodexProcess::spawn(opts, Box::new(|_| {}), Box::new(|_, _| {}))?;
    let read = async {
        p.request(
            "initialize",
            json!({
                "clientInfo": { "name": "escouade", "title": "Escouade", "version": env!("CARGO_PKG_VERSION") },
            }),
            REQUEST_LIMIT,
        )
        .await?;
        p.write(&json!({ "method": "initialized" }))?;
        p.request("account/rateLimits/read", json!({}), REQUEST_LIMIT)
            .await
    };
    let read = read.await;
    p.kill();
    p.wait_gone(Duration::from_secs(5)).await;
    Ok(read?["rateLimits"].clone())
}

/// The models of an account none of whose agents runs (`model/list`, every page): an app-server of
/// its own (`env`: its `CODEX_HOME`), started in `cwd` for that and stopped once it answered, as
/// `read_limits` (ESC-186). It starts no thread: nothing is spent.
pub(super) async fn read_models(
    program: Program,
    env: Vec<(String, String)>,
    cwd: String,
) -> Result<Value> {
    let opts = Opts {
        program,
        cwd,
        args: vec!["app-server".into()],
        env,
        start: Start::New,
        settings: Settings::default(),
        instructions: None,
    };
    let p = CodexProcess::spawn(opts, Box::new(|_| {}), Box::new(|_, _| {}))?;
    let read = async {
        p.request(
            "initialize",
            json!({
                "clientInfo": { "name": "escouade", "title": "Escouade", "version": env!("CARGO_PKG_VERSION") },
            }),
            REQUEST_LIMIT,
        )
        .await?;
        p.write(&json!({ "method": "initialized" }))?;
        p.list_models().await
    };
    let read = read.await;
    p.kill();
    p.wait_gone(Duration::from_secs(5)).await;
    read
}

/// One message of Codex: an answer to one of the process's requests (resolved here), or what goes
/// to the translation. The turn under way is followed here. Gives the messages to steer into the
/// turn that just started.
fn read(
    msg: &Value,
    pending: &Pending,
    fired: &Fired,
    state: &Mutex<State>,
    on_frame: &OnFrame,
) -> Vec<Value> {
    let method = msg["method"].as_str();
    if method.is_none() {
        let Some(id) = msg["id"].as_u64() else {
            return Vec::new();
        };
        let error = msg.get("error").filter(|e| !e.is_null());
        if let Some(tx) = pending.lock().remove(&id) {
            let _ = tx.send(match error {
                Some(e) => Err(e["message"].as_str().unwrap_or("error").to_string()),
                None => Ok(msg["result"].clone()),
            });
        } else if let Some(what) = fired.lock().remove(&id) {
            if let Some(e) = error {
                if what == "turn/start" {
                    state.lock().starting = false;
                }
                on_frame(json!({ "method": "error", "params": {
                    "error": { "message": e["message"], "codexErrorInfo": null },
                    "willRetry": false, "refused": what,
                } }));
            }
        }
        return Vec::new();
    }
    let p = &msg["params"];
    let own = {
        let s = state.lock();
        s.thread.is_some() && p["threadId"].as_str() == s.thread.as_deref()
    };
    let mut held = Vec::new();
    match method.unwrap_or_default() {
        "turn/started" if own => {
            let mut s = state.lock();
            s.turn = p["turn"]["id"].as_str().map(str::to_string);
            s.starting = false;
            held = std::mem::take(&mut s.held);
        }
        "turn/completed" if own => {
            let mut s = state.lock();
            if s.turn.as_deref() == p["turn"]["id"].as_str() {
                s.turn = None;
            }
        }
        "serverRequest/resolved" => {
            let id = id_key(&p["requestId"]);
            state.lock().asked.remove(&id);
        }
        m if !msg["id"].is_null() => {
            // A request of Codex: kept until it is answered.
            state.lock().asked.insert(
                id_key(&msg["id"]),
                (msg["id"].clone(), m.to_string(), p.clone()),
            );
        }
        _ => {}
    }
    on_frame(msg.clone());
    held
}

/// The id of a request, as the conversation's cards keep it: a string.
pub(crate) fn id_key(id: &Value) -> String {
    match id {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn ready<'a, T: Send + 'a>(v: T) -> BoxFuture<'a, T> {
    Box::pin(std::future::ready(v))
}

impl EngineProcess for CodexProcess {
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

    fn close_input(&self) {
        self.tx.lock().take();
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

    /// The handshake, then the thread (new, resumed or copied), then the models of the account.
    fn initialize(&self) -> BoxFuture<'_, Result<Ready>> {
        Box::pin(async move {
            self.request(
                "initialize",
                json!({
                    "clientInfo": { "name": "escouade", "title": "Escouade", "version": env!("CARGO_PKG_VERSION") },
                    // For the mode Plan alone (`collaborationMode` of `turn/start`).
                    "capabilities": { "experimentalApi": true },
                }),
                REQUEST_LIMIT,
            )
            .await?;
            self.write(&json!({ "method": "initialized" }))?;
            let (method, params) = self.thread_request();
            let result = self.request(method, params, REQUEST_LIMIT).await?;
            let thread = result["thread"].clone();
            let id = thread["id"]
                .as_str()
                .ok_or_else(|| anyhow!("{method}: no thread"))?
                .to_string();
            self.state.lock().thread = Some(id);
            // The translation learns the agent's session from it (a resumed thread says none).
            (self.on_frame)(json!({ "method": "thread/started", "params": { "thread": thread } }));
            let models = match self.list_models().await {
                Ok(list) => super::catalog(&list),
                Err(e) => {
                    log::warn!("codex: model/list: {e}");
                    None
                }
            };
            Ok(Ready {
                commands: Vec::new(),
                models: Value::Null,
                catalog: models,
            })
        })
    }

    fn send_message(&self, _uuid: &str, message: &Message) -> Result<()> {
        self.steer(message.0.clone())
    }

    fn interrupt(&self, _cancel_queued: bool, timeout: Duration) -> BoxFuture<'_, Result<()>> {
        Box::pin(async move {
            let (thread, turn) = {
                let s = self.state.lock();
                (s.thread.clone(), s.turn.clone())
            };
            let (Some(thread), Some(turn)) = (thread, turn) else {
                return Ok(());
            };
            self.request(
                "turn/interrupt",
                json!({ "threadId": thread, "turnId": turn }),
                timeout,
            )
            .await
            .map(|_| ())
        })
    }

    /// Codex takes the model, the effort and the mode at the next `turn/start`.
    fn set_model<'a>(&'a self, model: &'a str) -> BoxFuture<'a, Result<()>> {
        self.state.lock().settings.model = model.to_string();
        ready(Ok(()))
    }

    fn set_effort<'a>(&'a self, effort: &'a str) -> BoxFuture<'a, Result<()>> {
        self.state.lock().settings.effort = effort.to_string();
        ready(Ok(()))
    }

    fn set_mode<'a>(&'a self, mode: &'a str) -> BoxFuture<'a, Result<()>> {
        self.state.lock().settings.mode = mode.to_string();
        ready(Ok(()))
    }

    fn answer_permission(&self, request_id: &str, answer: PermissionAnswer) -> Result<()> {
        // The plan of the mode Plan, which Codex ends its turn on: approved, its work starts in
        // the mode left for it; sent back, the planning goes on with what the user said.
        if super::is_plan_approval(request_id) {
            let lang = i18n::claude();
            let text = match answer {
                PermissionAnswer::Allow { resume, .. } => {
                    self.state.lock().settings.mode = resume.unwrap_or_else(|| "default".into());
                    implement_plan(lang)
                }
                PermissionAnswer::Always { .. } => {
                    self.state.lock().settings.mode = "acceptEdits".into();
                    implement_plan(lang)
                }
                PermissionAnswer::Deny { message, .. } => message,
            };
            return self.start_turn(json!([{ "type": "text", "text": text }]));
        }
        let Some((id, method, params)) = self.state.lock().asked.remove(request_id) else {
            bail!("Codex no longer waits for this answer");
        };
        let (allowed, session) = match &answer {
            PermissionAnswer::Allow { .. } => (true, false),
            PermissionAnswer::Always { .. } => (true, true),
            PermissionAnswer::Deny { .. } => (false, false),
        };
        let result = if method == "item/permissions/requestApproval" {
            match (allowed, session) {
                (true, s) => json!({
                    "permissions": params["permissions"],
                    "scope": if s { "session" } else { "turn" },
                }),
                (false, _) => json!({ "permissions": {} }),
            }
        } else {
            json!({ "decision": match (allowed, session) {
                (true, false) => "accept",
                (true, true) => "acceptForSession",
                (false, _) => "decline",
            } })
        };
        self.respond(&id, result)?;
        // Codex hears no reason with a refusal: the agent is told the one given, within its turn
        // (the words of a refusal without one would tell it nothing more).
        if let PermissionAnswer::Deny { message, .. } = answer {
            let bare = [Lang::Fr, Lang::En]
                .iter()
                .any(|l| crate::agent::refused_action(*l) == message);
            if !message.trim().is_empty() && !bare {
                let _ = self.steer(json!([{ "type": "text", "text": message }]));
            }
        }
        Ok(())
    }

    /// `input.questions` are Codex's, with their ids; `input.answers` the card's, by question.
    fn answer_question(&self, request_id: &str, _call: &str, input: Value) -> Result<()> {
        let Some((id, _, _)) = self.state.lock().asked.remove(request_id) else {
            bail!("Codex no longer waits for this answer");
        };
        self.respond(&id, json!({ "answers": super::answers(&input) }))
    }

    fn read_usage(&self) -> BoxFuture<'_, Result<Option<Value>>> {
        Box::pin(async move {
            let read = self
                .request("account/rateLimits/read", json!({}), REQUEST_LIMIT)
                .await?;
            let meters = super::meters(&read["rateLimits"]);
            Ok((!meters.is_empty()).then(|| json!({ "meters": meters })))
        })
    }

    fn rename<'a>(&'a self, title: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let thread = self.thread()?;
            self.request(
                "thread/name/set",
                json!({ "threadId": thread, "name": title }),
                REQUEST_LIMIT,
            )
            .await
            .map(|_| ())
        })
    }

    fn stop_task<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<()>> {
        ready(Err(anyhow!(unsupported())))
    }

    fn task_output<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<Value>> {
        ready(Err(anyhow!(unsupported())))
    }

    fn remote_control(&self, _: Remote, _: Duration) -> BoxFuture<'_, Result<Value>> {
        ready(Err(anyhow!(unsupported())))
    }

    /// `thread/compact/start` (ESC-205): it takes no instruction, and answers at once; the
    /// compaction is a turn of its own (`contextCompaction`).
    fn compact<'a>(&'a self, _instructions: &'a str) -> BoxFuture<'a, Result<bool>> {
        Box::pin(async move {
            let thread = self.thread()?;
            self.request(
                "thread/compact/start",
                json!({ "threadId": thread }),
                REQUEST_LIMIT,
            )
            .await
            .map(|_| false)
        })
    }

    /// A request in Codex's protocol: `{"method", "params"}`.
    fn control(&self, request: Value, timeout: Duration) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            let method = request["method"].as_str().unwrap_or_default().to_string();
            self.request(&method, request["params"].clone(), timeout)
                .await
        })
    }

    fn reply(&self, request_id: &str, answer: std::result::Result<Value, String>) -> Result<()> {
        let id = self
            .state
            .lock()
            .asked
            .remove(request_id)
            .map(|(id, _, _)| id)
            .unwrap_or_else(|| json!(request_id));
        match answer {
            Ok(result) => self.respond(&id, result),
            Err(message) => self.write(&json!({
                "id": id, "error": { "code": -32601, "message": message },
            })),
        }
    }
}

/// What the agent is told when its plan is approved.
fn implement_plan(lang: Lang) -> String {
    tr_in!(lang, "Mets le plan en œuvre.", "Implement the plan.")
}

fn unsupported() -> String {
    tr!("Codex ne le permet pas", "Codex doesn’t support it")
}

/// A line a process wrote, as text: invalid UTF-8 replaced, without its `\r`.
fn text_line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim_end_matches('\r')
        .replace('\0', "")
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
