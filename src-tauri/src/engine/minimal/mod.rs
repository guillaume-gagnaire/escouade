//! The minimal fake engine: the least an engine writes to plug into the conformance bench
//! (docs/ENGINES.md, « Le banc de conformité »). Nothing of the core nor of the bench changed for
//! it: it is declared here and its translation in `agent/minimal.rs`, as any engine's.
//!
//! Its process is a script played in the app's own process, on a thread, in a protocol of its own
//! (`agent/minimal.rs` lists its frames); its message picks the scenario: « raisonnement »,
//! « commande », « fichier », « question », « lent », « sous-agent », anything else a plain reply.
//! It cannot copy a conversation nor read a quota, and has no mode `plan`: its capabilities say
//! so, and the bench leaves those scenarios out. What the bench never calls is unreachable.

use super::bench::{bench_dir, replay, run, Scenario, Target};
use super::registry::{
    CostKind, EngineCaps, ForkPoint, Install, MidTurn, ModeCap, ModelDecl, QuotaKind, Tested,
};
use super::{
    mcp, BoxFuture, Engine, EngineProcess, Escouade, Message, OneShot, PermissionAnswer, Process,
    Program, Ready, Remote, Room, Spawn, Start, StartLost,
};
use crate::agent::{AgentRt, Effects};
use crate::core::Attachment;
use crate::job::JobUsage;
use crate::mcp::install::{Entry, Standing};
use crate::model::EngineId;
use anyhow::{bail, Result};
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(crate) struct Minimal;

pub(crate) static MINIMAL: Minimal = Minimal;

impl Engine for Minimal {
    /// The id the tests of the handover run it under, beside Claude Code's fake (the bench reads
    /// none).
    fn id(&self) -> EngineId {
        EngineId::BENCH
    }

    fn name(&self) -> &'static str {
        "Minimal"
    }

    fn speaker(&self) -> &'static str {
        "Minimal"
    }

    fn caps(&self) -> EngineCaps {
        EngineCaps {
            modes: vec![ModeCap {
                id: "default",
                meaning: None,
            }],
            effort: false,
            images: false,
            pdf: false,
            fork: ForkPoint::None,
            resume_other_account: false,
            mid_turn: MidTurn::Refused,
            slash_commands: false,
            quota: QuotaKind::None,
            quota_windows: Vec::new(),
            cost: CostKind::None,
            terminal_view: false,
            remote_control: false,
            background_tasks: false,
            todo_tools: false,
            default_refusal: false,
            sandbox: false,
            usage_page: None,
            accounts_by_key: false,
            read_only: false,
        }
    }

    fn accounts(&self) -> crate::engine::registry::AccountStore {
        crate::engine::registry::AccountStore::ConfigDir {
            var: "MINIMAL_HOME",
        }
    }

    fn writer(&self, _: crate::engine::Weight) -> &'static str {
        "Minimal"
    }

    fn terminal(&self) -> Option<&'static dyn crate::engine::TerminalView> {
        None
    }

    fn spawn(
        &self,
        spawn: Spawn,
        on_frame: super::OnFrame,
        on_exit: super::OnExit,
    ) -> Result<Process> {
        if matches!(spawn.start, Start::Fork { .. }) {
            bail!("Minimal cannot copy a conversation");
        }
        let (tx, rx) = channel();
        let alive = Arc::new(AtomicBool::new(true));
        let still = alive.clone();
        std::thread::spawn(move || {
            Script::new(spawn.start, rx, on_frame).play();
            still.store(false, Ordering::Release);
            on_exit(Some(0), String::new());
        });
        Ok(Arc::new(MinimalProcess {
            input: Mutex::new(Some(tx)),
            alive,
        }))
    }

    fn message(&self, text: &str, attachments: &[Attachment]) -> Result<Message> {
        if !attachments.is_empty() {
            bail!("Minimal takes no file");
        }
        Ok(Message(json!(text)))
    }

    fn translate(&self, rt: &mut AgentRt, frame: &Value, fx: &mut Effects) {
        rt.minimal_frame(frame, fx);
    }

    fn start_lost(&self, _stderr: &str) -> Option<StartLost> {
        None
    }

    // ----- what the bench never calls -----

    fn instructions_room(&self) -> Option<Room> {
        None
    }
    fn subagent_relay(&self, _: crate::i18n::Lang, _: &str, _: &str, _: &str) -> Option<String> {
        None
    }
    /// Played in the app's process: no program is looked for.
    fn locate(&self, _: &str, _: &str) -> Option<Program> {
        Some(Program::Native(PathBuf::new()))
    }
    fn tested(&self) -> Tested {
        Tested {
            oldest: "0",
            newest: "0",
        }
    }
    fn oldest_reason(&self, _: crate::i18n::Lang) -> String {
        unreachable!("not on the bench")
    }
    fn update_command(&self) -> &'static str {
        unreachable!("not on the bench")
    }
    /// Asked by the core before a start: no program answers, no version is read.
    fn version_args(&self) -> &'static [&'static str] {
        &["--version"]
    }
    fn install(&self) -> Install {
        unreachable!("not on the bench")
    }
    fn models(&self) -> Vec<ModelDecl> {
        ["grand", "petit"]
            .into_iter()
            .map(|id| ModelDecl {
                id,
                label: id,
                effort: false,
                auto: false,
            })
            .collect()
    }
    /// A small window, for the tests of what bounds the handover file.
    fn context_window(&self, _: &str) -> u64 {
        4_000
    }
    fn efforts(&self) -> &'static [&'static str] {
        &[]
    }
    fn supports_effort(&self, _: &str) -> bool {
        false
    }
    fn supports_auto(&self, _: &str) -> bool {
        false
    }
    fn one_shot<'a>(&'a self, _: OneShot<'a>) -> BoxFuture<'a, Result<String>> {
        unreachable!("not on the bench")
    }
    fn grant_file(&self, _: &str, _: &str) -> Option<Vec<u8>> {
        None
    }
    fn refusal(&self) -> Option<Value> {
        None
    }
    fn escouade_launch(&self, _: &Escouade) -> mcp::Launch {
        unreachable!("not on the bench")
    }
    fn declared(&self, _: &Path) -> Standing {
        unreachable!("not on the bench")
    }
    fn add_declaration<'a>(
        &'a self,
        _: &'a crate::mcp::install::Target,
        _: &'a Entry,
        _: &'a [&'a str],
    ) -> BoxFuture<'a, Result<()>> {
        unreachable!("not on the bench")
    }
    fn remove_declaration<'a>(
        &'a self,
        _: &'a crate::mcp::install::Target,
        _: &'a [&'a str],
    ) -> BoxFuture<'a, Result<()>> {
        unreachable!("not on the bench")
    }
    fn manual_declaration(&self, _: bool, _: &str, _: Option<&str>, _: &Entry, _: bool) -> String {
        unreachable!("not on the bench")
    }
}

/// What the process is told.
enum Input {
    Say(String),
    Answer { request: String, value: Value },
    Interrupt,
}

/// The live process: its script's thread, fed by its input.
struct MinimalProcess {
    input: Mutex<Option<Sender<Input>>>,
    alive: Arc<AtomicBool>,
}

impl MinimalProcess {
    fn tell(&self, input: Input) -> Result<()> {
        match self.input.lock().as_ref().map(|tx| tx.send(input)) {
            Some(Ok(())) => Ok(()),
            _ => bail!("Minimal is gone"),
        }
    }
}

fn ready<'a, T: Send + 'a>(v: T) -> BoxFuture<'a, T> {
    Box::pin(std::future::ready(v))
}

impl EngineProcess for MinimalProcess {
    fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
    }
    fn usage(&self) -> Option<JobUsage> {
        None
    }
    fn still_running(&self) -> u32 {
        u32::from(self.is_alive())
    }
    fn pids(&self) -> Vec<u32> {
        Vec::new()
    }
    fn close_input(&self) {
        self.input.lock().take();
    }
    fn kill(&self) {
        self.close_input();
    }
    fn wait_exit(&self, within: Duration) -> BoxFuture<'_, bool> {
        Box::pin(async move {
            let deadline = Instant::now() + within;
            while self.is_alive() && Instant::now() < deadline {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            !self.is_alive()
        })
    }
    fn wait_gone(&self, within: Duration) -> BoxFuture<'_, bool> {
        self.wait_exit(within)
    }
    fn initialize(&self) -> BoxFuture<'_, Result<Ready>> {
        ready(Ok(Ready::default()))
    }
    fn send_message(&self, _uuid: &str, message: &Message) -> Result<()> {
        self.tell(Input::Say(
            message.0.as_str().unwrap_or_default().to_string(),
        ))
    }
    fn interrupt(&self, _cancel_queued: bool, _timeout: Duration) -> BoxFuture<'_, Result<()>> {
        ready(self.tell(Input::Interrupt))
    }
    fn answer_permission(&self, request_id: &str, answer: PermissionAnswer) -> Result<()> {
        let value = match answer {
            PermissionAnswer::Allow { .. } | PermissionAnswer::Always { .. } => {
                json!({ "allow": true })
            }
            PermissionAnswer::Deny { message, .. } => json!({ "allow": false, "message": message }),
        };
        self.tell(Input::Answer {
            request: request_id.to_string(),
            value,
        })
    }
    fn answer_question(&self, request_id: &str, _call: &str, input: Value) -> Result<()> {
        self.tell(Input::Answer {
            request: request_id.to_string(),
            value: input["answers"].clone(),
        })
    }
    fn set_model<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<()>> {
        ready(Ok(()))
    }
    fn set_effort<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<()>> {
        ready(Ok(()))
    }
    fn set_mode<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<()>> {
        ready(Ok(()))
    }
    fn read_usage(&self) -> BoxFuture<'_, Result<Option<Value>>> {
        ready(Ok(None))
    }
    fn rename<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<()>> {
        ready(Ok(()))
    }
    fn stop_task<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<()>> {
        ready(Err(anyhow::anyhow!("not supported")))
    }
    fn task_output<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<Value>> {
        ready(Err(anyhow::anyhow!("not supported")))
    }
    fn remote_control(&self, _: Remote, _: Duration) -> BoxFuture<'_, Result<Value>> {
        ready(Err(anyhow::anyhow!("not supported")))
    }
    fn control(&self, _: Value, _: Duration) -> BoxFuture<'_, Result<Value>> {
        ready(Err(anyhow::anyhow!("not supported")))
    }
    fn reply(&self, _: &str, _: std::result::Result<Value, String>) -> Result<()> {
        bail!("not supported")
    }
}

/// The sessions it made, numbered.
static SESSIONS: AtomicU32 = AtomicU32::new(0);

/// What its process plays, a turn a message, until its input is closed.
struct Script {
    session: String,
    input: Receiver<Input>,
    out: super::OnFrame,
    /// Its calls and requests, numbered.
    n: u32,
}

/// A turn cut short: interrupted, or the input closed.
enum Cut {
    Interrupted,
    Closed,
}

impl Script {
    fn new(start: Start, input: Receiver<Input>, out: super::OnFrame) -> Self {
        let session = match start {
            Start::Resume { session } => session,
            _ => format!("min-{}", SESSIONS.fetch_add(1, Ordering::Relaxed)),
        };
        Script {
            session,
            input,
            out,
            n: 0,
        }
    }

    fn play(mut self) {
        while let Ok(input) = self.input.recv() {
            if let Input::Say(text) = input {
                match self.turn(&text) {
                    Ok(()) => {
                        self.out(json!({ "type": "end", "usage": { "input": 120, "output": 30 } }))
                    }
                    Err(Cut::Interrupted) => {
                        self.out(json!({ "type": "end", "interrupted": true }))
                    }
                    Err(Cut::Closed) => return,
                }
            }
        }
    }

    fn out(&self, frame: Value) {
        (self.out)(frame);
    }

    fn say(&self, text: &str) {
        self.out(json!({ "type": "say", "text": text }));
    }

    fn id(&mut self, prefix: &str) -> String {
        self.n += 1;
        format!("{prefix}-{}", self.n)
    }

    /// Waits for the answer to `request`, at most `within`; None when none came by then.
    fn wait(&self, request: &str, within: Duration) -> Result<Option<Value>, Cut> {
        let deadline = Instant::now() + within;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.input.recv_timeout(left) {
                Ok(Input::Answer { request: r, value }) if r == request => return Ok(Some(value)),
                Ok(Input::Interrupt) => return Err(Cut::Interrupted),
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout) => return Ok(None),
                Err(RecvTimeoutError::Disconnected) => return Err(Cut::Closed),
            }
        }
    }

    fn turn(&mut self, text: &str) -> Result<(), Cut> {
        self.out(json!({ "type": "start", "session": self.session }));
        if text.contains("raisonnement") {
            self.out(
                json!({ "type": "thinking", "text": "Deux pistes ; la seconde est la bonne." }),
            );
            self.say("Réponse réfléchie.");
        } else if text.contains("commande") {
            let (call, request) = (self.id("call"), self.id("ask"));
            self.out(json!({ "type": "call", "id": call, "tool": "shell", "input": { "command": "rm -rf build" } }));
            self.out(json!({ "type": "ask", "request": request, "call": call }));
            let answer = self
                .wait(&request, Duration::from_secs(60))?
                .unwrap_or_default();
            if answer["allow"] == true {
                self.out(json!({ "type": "done", "call": call, "output": "build supprimé" }));
                self.say("Commande exécutée.");
            } else {
                self.out(
                    json!({ "type": "done", "call": call, "output": "refusée", "error": true }),
                );
                self.say("Compris.");
            }
        } else if text.contains("fichier") {
            let call = self.id("call");
            self.out(json!({ "type": "call", "id": call, "tool": "patch", "input": { "path": "src/app.ts", "diff": "-const a = 1;\n+const a = 2;" } }));
            self.out(json!({ "type": "done", "call": call, "output": "1 fichier modifié" }));
            self.say("Fichier modifié.");
        } else if text.contains("question") {
            let (call, request) = (self.id("call"), self.id("question"));
            self.out(json!({
                "type": "question", "request": request, "call": call,
                "text": "Quelle base de données ?", "choices": ["PostgreSQL", "SQLite"],
            }));
            let answers = self
                .wait(&request, Duration::from_secs(60))?
                .unwrap_or_default();
            let chosen: Vec<&str> = answers
                .as_object()
                .into_iter()
                .flat_map(|o| o.values().filter_map(Value::as_str))
                .collect();
            self.say(&format!("Choix retenu : {}", chosen.join(", ")));
        } else if text.contains("lent") {
            self.say("Je commence…");
            self.wait("", Duration::from_secs(30))?;
        } else if text.contains("sous-agent") {
            let call = self.id("call");
            self.out(json!({ "type": "call", "id": call, "tool": "agent", "input": { "task": "Chercher la cause du bug" } }));
            self.out(json!({ "type": "say", "text": "Cause trouvée : le filtre refuse les PDF.", "parent": call }));
            self.out(
                json!({ "type": "done", "call": call, "output": "Le filtre refuse les PDF." }),
            );
            self.say("Rapport reçu.");
        } else {
            self.say(&format!("Bonjour, tu as dit : {text}"));
        }
        Ok(())
    }
}

/// The minimal fake on the bench.
struct OnBench;

impl Target for OnBench {
    fn engine(&self) -> &'static dyn Engine {
        &MINIMAL
    }

    /// Played in the app's process: no program is run.
    fn program(&self) -> Program {
        Program::Native(PathBuf::new())
    }

    fn prompt(&self, scenario: Scenario) -> Option<String> {
        Some(
            match scenario {
                Scenario::Text | Scenario::Resume => "bonjour",
                Scenario::Thinking => "raisonnement",
                Scenario::Approval => "commande",
                Scenario::Edit => "fichier",
                Scenario::Question => "question",
                Scenario::Interrupt => "lent",
                Scenario::Subagents => "sous-agent",
                // Its capabilities leave them out.
                Scenario::PlanMode | Scenario::Limit | Scenario::Fork | Scenario::Quota => {
                    return None
                }
            }
            .to_string(),
        )
    }
}

#[tokio::test]
async fn the_minimal_fake_engine_plugs_into_the_bench_and_passes_what_its_capabilities_call_for() {
    let dir = bench_dir("esc35-bench-minimal");
    let played = run(&OnBench, &dir).await;
    assert_eq!(
        played,
        [
            Scenario::Text,
            Scenario::Thinking,
            Scenario::Approval,
            Scenario::Edit,
            Scenario::Question,
            Scenario::Interrupt,
            Scenario::Subagents,
            Scenario::Resume,
        ]
    );
}

#[test]
fn the_recorded_flux_of_the_minimal_fake_make_the_items_expected() {
    replay::check_all(&MINIMAL, "minimal");
}
