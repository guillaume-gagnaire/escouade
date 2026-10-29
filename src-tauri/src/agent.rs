//! Runtime state of one agent: turns raw Claude Code frames into conversation items,
//! tracks status, pending questions/permissions and per-turn usage.

use crate::attachfiles;
use crate::board::TurnEnd;
use crate::conv::Conv;
use crate::engine::{self, PermissionAnswer, Process, StartLost};
use crate::i18n::{self, Lang};
use crate::model::*;
use crate::notify;
use crate::paths::relative_slash;
use crate::plan::{BgMemory, Change, PlanState, RunStatus};
use crate::planfiles::{self, FileList};
use crate::pricing::{self, Usage};
use crate::text::truncate;
use anyhow::{anyhow, bail, Result};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

mod agy;
mod agy_tui;
mod claude;
mod codex;
mod codex_tui;
mod copilot;
mod copilot_tui;
mod cursor;
mod cursor_tui;
mod localcmd;
// The translation of the minimal fake engine of the conformance bench (`engine/minimal/`).
mod gemini;
mod gemini_tui;
#[cfg(test)]
mod minimal;
mod opencode;
pub mod sidechain;
mod transcript;
mod vibe;
mod vibe_tui;

pub use transcript::HookOutcome;

pub type AgentHandle = Arc<parking_lot::Mutex<AgentRt>>;

/// What a notification is about: each kind has its own choice in the settings ("Me prévenir pour").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyKind {
    /// A question or a permission waits for the user.
    Question,
    /// A turn ended.
    Done,
    /// A turn failed, or the process died.
    Error,
    /// A ticket is ready to test, or blocked (raised by the board, never by an agent's own turn).
    Ticket,
}

/// A notification an agent raises: its kind, and the words it says (its title is the agent's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentAlert {
    pub kind: NotifyKind,
    pub body: String,
}

impl AgentAlert {
    fn new(kind: NotifyKind, body: impl Into<String>) -> Self {
        Self {
            kind,
            body: body.into(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TurnRow {
    /// The harness that ran it: the agent's at that turn (`AgentMeta::engine`).
    pub engine: EngineId,
    pub model: String,
    pub input: u64,
    pub cache: u64,
    pub output: u64,
    pub cost: f64,
}

/// Side effects collected while the agent lock is held, applied once it is released.
#[derive(Default)]
pub struct Effects {
    pub ops: Vec<ConvOp>,
    pub agent_changed: bool,
    pub save: bool,
    pub notify: Option<AgentAlert>,
    pub turns: Vec<TurnRow>,
    pub rate: Option<Vec<crate::model::QuotaMeter>>,
    /// Whether the agent's account answers as extra usage (`isUsingOverage` of a `rate_limit_event`).
    pub overage: Option<bool>,
    /// The turn was stopped by the usage limit, which resets then (if Claude Code told).
    pub limited: Option<Option<i64>>,
    /// The turn was stopped by a sign-in of its account that could not be refreshed (ESC-162):
    /// the core looks at the account's sign-in, and starts the process again or waits (`signin`).
    pub sign_in_lost: bool,
    pub files_changed: bool,
    /// A turn ended (or the process died during one): how, with the assistant's text.
    pub turn_end: Option<TurnEnd>,
    /// What the agent just did may have changed the files of its plan (a command ended, a
    /// subagent came back, the turn ended): the core has them looked at.
    pub plan_files: bool,
    /// Calls of `SendUserFile` (ESC-171), by their item and with their input: the core copies
    /// their images and says what their files are on their item (`sentfiles::deliver`).
    pub sent: Vec<(String, Value)>,
}

struct Block {
    index: u64,
    item_id: String,
    kind: &'static str,
    finalized: bool,
}

struct PendingReq {
    item_id: String,
    tool_use_id: String,
    input: Value,
    suggestions: Value,
    /// What the agent's view says of it, worked out once when it comes.
    view: PendingView,
    /// When it came (ms): the first time, when Claude Code asks it again.
    since: i64,
}

/// A request waiting for an answer, as the MCP server's tools read it (`AgentRt::waiting`): the
/// view, which is all « Vue d'ensemble » needs, and what a client that answers must also know.
#[derive(Debug, Clone)]
pub struct Waiting {
    pub view: PendingView,
    /// When it came (epoch milliseconds).
    pub since: i64,
    /// What Claude Code asked with, as it sent it: a plan to approve, a question's options with
    /// their descriptions and whether several may be picked. Not for a client to show whole (a
    /// command, a file's content): the view caps what it says of it.
    pub input: Value,
}

#[derive(Debug, Clone, Default)]
struct Counters {
    input: u64,
    output: u64,
    cache: u64,
    cost: f64,
}

pub struct AgentRt {
    pub meta: AgentMeta,
    /// Where its facts go (PLG-5): its status, its requests and their answers, told where they
    /// change; nowhere outside the core.
    pub(crate) facts: crate::facts::Emitter,
    pub proc: Option<Process>,
    pub gen: u64,
    pub conv: Conv,
    pub active_since: Option<i64>,
    pub context_tokens: u64,
    pub context_window: u64,
    /// Model of the conversation's latest message (it may be switched mid-process).
    main_model: String,
    /// This turn met the usage limit, and when the limit resets.
    rate_limited: bool,
    limit_resets_at: Option<i64>,
    /// The sign-in of its account when its process started (`usage::Stored::stamp`, ESC-162).
    pub(crate) launch_stamp: Option<String>,
    /// The restarts of its process since its turn stopped on a sign-in that could not be
    /// refreshed (`signin`): two at most, then it waits for a new one.
    pub(crate) sign_in_retries: u8,
    /// Tasks run in the foreground: their tool rows show them end.
    foreground_tasks: HashSet<String>,
    /// Background tasks whose end was shown, until Claude Code also replays it to Claude.
    announced_tasks: HashMap<String, u32>,
    pub commands: Vec<Value>,
    pub interrupted: bool,
    pub saw_init: bool,
    /// Remote Control link state ("ready", "connected"…) of the live process.
    pub remote_state: Option<String>,
    /// The live process was linked to claude.ai (Remote Control).
    pub remote_linked: bool,
    /// A link to claude.ai was tried on the live process (it worked, or the agent says why not).
    pub remote_tried: bool,
    /// Its interface closed right after it started with Remote Control (`tui::RC_GRACE`): the
    /// ones that follow start without the option, until the app starts again or the user turns
    /// Remote Control on by hand (not saved).
    pub remote_fallback: bool,
    /// The agent was told its Remote Control is unavailable, and nothing has worked since: it is
    /// not told again at every start, until a link works or the user turns it on by hand (not
    /// saved).
    pub remote_told: bool,
    /// What the agent does now, from its latest tool, thinking or text (main thread only).
    pub activity: Option<String>,
    /// The plans it wrote (docs/superpowers/plans/*.md, as absolute paths), oldest first, five at
    /// most: the plan of its folder when no ledger names one (`planfiles`).
    pub(crate) plan_written: Vec<String>,
    /// The calls of its main thread whose result calls for a look at the files of its plan.
    plan_calls: HashSet<String>,
    /// The commands of its main thread that remove the workspace of the plan it follows: the run
    /// ends with their result (the skills delete the workspace once the final review is clean).
    plan_removals: HashSet<String>,
    /// When its conversation began, in milliseconds: the files of a plan that were last touched
    /// before are not its own.
    pub(crate) plan_since: i64,
    /// What the last look at the files of its plan read (not saved): a look that finds them as
    /// they were hands nothing over, so this is what its plan is named and titled from again when
    /// the agent's own list changes.
    plan_list: Option<FileList>,
    /// What the plan remembers of the calls of commands and of the tasks in the foreground (not
    /// saved), kept here while the agent has no plan to hold it: a plan with nothing to show is not
    /// kept, and the command a task starts with is told by a call before it.
    plan_memory: BgMemory,
    /// The assistant's text of the running turn (main thread), for the board's report.
    turn_text: String,
    /// The latest text of the running turn's main thread that no tool followed: its final reply,
    /// for the notification.
    final_text: String,
    /// The setup of its new worktree under way: the step running ("npm ci (1/2)").
    pub setup: Option<String>,
    /// What the step of that setup wrote, its last lines: sent to the window as it comes, apart
    /// from its view (`UiEvent::SetupOutput`).
    pub setup_output: Option<SetupOutput>,
    /// Why the setup of its worktree failed, until its ticket's first message tells it (written in
    /// the language of the texts for Claude, as that message).
    pub setup_failure: Option<String>,
    blocks: HashMap<String, Vec<Block>>,
    current_msg: HashMap<String, String>,
    /// Requests waiting for an answer, in the order they came (an item's id is its request's).
    pending: Vec<PendingReq>,
    last_usage: HashMap<String, Counters>,
    /// Usage of the running turn's API messages (by message id, with their model), until the
    /// turn's result brings the exact figures.
    live: HashMap<String, (String, Usage)>,
    queued: u32,
    /// Claude Code's own interface running for it, in a terminal (« Vue Claude Code », `tui`).
    pub tui: Option<TuiRun>,
    /// Its harness's interface joined to its process, in a terminal (`AgentMeta::attached`).
    pub attach: Option<AttachRun>,
    /// The usage of each API message its interface ran, by message id with its model: what its
    /// turns are counted from, at list prices (the interface tells no cost per turn).
    tui_usage: HashMap<String, (String, Usage)>,
    /// The running turn of its interface met an API error: what it said.
    tui_error: Option<String>,
    /// The running turn of its interface, from its prompt (what it was) to its end.
    pub(crate) tui_prompt: Option<String>,
    /// The `promptId` of its transcript's entry that prompt was read from: Claude Code 2.1.302
    /// writes a command typed twice under the same one, as typed then as `<command-name>`.
    tui_prompt_id: Option<String>,
    /// The entries of its transcript written before then (ms) are not read: the history of a
    /// session its interface resumed or forked with no read kept, which its view already has.
    pub(crate) tui_since: Option<i64>,
    /// How its last interface ended, until the next one starts (not saved): the window starts it
    /// again, or not, from this.
    pub(crate) tui_exit: Option<TuiExit>,
    /// How many of its interfaces in a row stopped soon after they started (not saved): an
    /// automatic start is refused from the second on, so that one that cannot run does not loop.
    pub(crate) tui_quick_exits: u32,
    /// The session its interface named at its start, while nothing of it is read from its
    /// transcript (not saved): Claude Code writes none before a first message, so its absence is no
    /// loss, and nothing is said of it (`Core::drop_unwritten_session`).
    pub(crate) unwritten_session: Option<String>,
    /// Haiku was asked to name it since the app started (not saved): asked once, at the first
    /// message that gives a text to name it from (`naming_text`).
    pub(crate) naming_asked: bool,
    /// Handed over from a terminal view (« Vue <harness> ») to a harness that has one (ESC-120):
    /// the turn of its handover file ended, it goes in that harness's view, on its new session
    /// (not saved).
    pub(crate) view_after_handover: bool,
    /// The text of a message the app typed into its interface by itself (the automatic resume:
    /// « continue »), until its entry is read from the transcript: no task of the user's, it
    /// never names the agent (not saved).
    pub(crate) unprompted: Option<String>,
    /// Its live process (or interface) was started with Escouade's server kept out of it
    /// (`Escouade::Denied`).
    pub(crate) without_escouade: bool,
    /// The paths its live process was started refused (« Projet privé », `fence`).
    pub(crate) fence: crate::fence::Fence,
    /// The projects' privacy its fence was made from: changed, its process starts again.
    pub(crate) privacy: crate::fence::Privacy,
    /// « Outils d'Escouade au prochain démarrage »: that process has none of Escouade's tools,
    /// which a start now would give (`Core::mark_escouade_next`, ESC-91).
    pub(crate) escouade_next: bool,
    /// What the translation of Codex keeps between its messages (`agent/codex.rs`).
    codex: codex::CodexState,
    /// What the translation of Copilot keeps between its events (`agent/copilot.rs`).
    copilot: copilot::CopilotState,
    /// What the translation of Cursor keeps between its updates (`agent/cursor.rs`).
    cursor: cursor::CursorState,
    /// What the translation of OpenCode keeps between its events (`agent/opencode.rs`).
    opencode: opencode::OpencodeState,
    /// What the translation of Mistral Vibe keeps between its updates (`agent/vibe.rs`).
    vibe: vibe::VibeState,
    /// What the translation of Antigravity CLI keeps between its events (`agent/agy.rs`).
    agy: agy::AgyState,
    /// What the translation of Gemini CLI keeps between its updates (`agent/gemini.rs`).
    gemini: gemini::GeminiState,
    /// What the hooks of its Gemini interface keep between them (`agent/gemini_tui.rs`).
    gemini_tui: gemini_tui::GeminiTuiState,
}

/// A stream's process linked to claude.ai, and the session it holds there (its bridge id, when
/// known).
pub struct RemoteLink {
    pub proc: Process,
    pub bridge: Option<String>,
}

/// The interface of its harness joined to an agent's process (`attach`, `AgentMeta::attached`).
#[derive(Debug, Clone)]
pub struct AttachRun {
    /// Its terminal, as the window is told of it.
    pub info: crate::pty::TermInfo,
    /// What the terminal printed, and the window that shows it.
    pub output: Arc<crate::tui::TuiOutput>,
    /// What it was started with: an interface joined to a process whose server changed is stopped.
    pub launched: crate::engine::terminal::Attach,
}

/// Claude Code's interface running for an agent (`tui`).
#[derive(Debug, Clone)]
pub struct TuiRun {
    /// Its terminal.
    pub term_id: String,
    /// The same, as the window is told of it.
    pub info: crate::pty::TermInfo,
    /// What the terminal printed, and the window that shows it: the terminal runs with none too
    /// (Remote Control keeps the interface up).
    pub output: Arc<crate::tui::TuiOutput>,
    /// It was started with Remote Control (`--remote-control`): it has it whatever the agent's
    /// setting says now, until its next start.
    pub remote: bool,
    /// When it started, as an instant: what the registry is waited for since (`REGISTRY_PATIENCE`),
    /// and the quick close that Remote Control is taken for (`RC_GRACE`).
    pub launched_at: std::time::Instant,
    /// Its transcript has an entry that says it is up and how the link of its session to claude.ai
    /// went (`bridge_status`, `bridge-session`), written since it started. See
    /// `AgentRt::interface_ready`.
    pub spoke: bool,
    /// When the registry of Claude Code's sessions first listed it (`tui::find_registered`):
    /// it is past its welcome screen and the box about the folder.
    pub registered: Option<std::time::Instant>,
    /// The file of the registry that lists it, once found: read again for its link.
    pub registry_file: Option<std::path::PathBuf>,
    /// The folder of that registry (`<the account's config folder>/sessions`), when its harness
    /// keeps one (`TerminalView::running_dir`).
    pub sessions: Option<std::path::PathBuf>,
    /// A sign was seen that its session is linked to claude.ai (a `bridge-session` entry, a
    /// `bridge_status` with a link, the `bridgeSessionId` of the registry).
    pub linked: bool,
    /// Held while a message is typed in it, paste then Enter: two messages never mix.
    pub typing: Arc<parking_lot::Mutex<()>>,
    /// How many keys its window typed (`PtyManager::typed`), as they come.
    pub typed: Arc<std::sync::atomic::AtomicU64>,
    /// How many it had typed when its prompt was last known empty: as it started, as a turn
    /// started (its prompt sent), once Escouade emptied it.
    pub empty_at: Option<u64>,
    /// How many it had typed when Escouade last pasted a message in it, its prompt empty.
    pub pasted: Option<u64>,
    /// A turn of it was cancelled before its answer, and Claude Code put its message back in its
    /// prompt (`tui::IDLE_ENDS_TURN_MS`); until the next turn starts.
    pub cancelled: Option<crate::tui::Cancelled>,
    /// The user closed its terminal (`Core::kill_terminal`): its end is no failure to start.
    pub killed: bool,
    /// Remote Control ended while it ran a turn, and no window had shown it: it stops once the
    /// turn is over, if it still has none.
    pub stop_when_idle: bool,
    /// Which of the agent's interfaces it is: what the terminal's exit and the reads of the
    /// transcript check, so that an older one's are ignored.
    pub gen: u64,
    /// When it started (ms): an exit soon after is a quick one (`tui::QUICK_EXIT`).
    pub started: i64,
    /// The transcript it writes, once known, and how far it was read (bytes).
    pub transcript: Option<std::path::PathBuf>,
    pub offset: u64,
    /// The `claude` it runs: what reads the files Escouade gives it (an image pasted there).
    pub program: crate::engine::Program,
}

#[cfg(test)]
impl TuiRun {
    /// An interface that has said nothing yet, no window has shown (tests).
    pub(crate) fn quiet(remote: bool) -> Self {
        let info = crate::pty::TermInfo {
            id: "term-1".into(),
            project_id: String::new(),
            name: "agent".into(),
            shell: "claude".into(),
        };
        Self {
            term_id: info.id.clone(),
            info,
            output: Arc::new(crate::tui::TuiOutput::new(None)),
            remote,
            launched_at: std::time::Instant::now(),
            spoke: false,
            registered: None,
            registry_file: None,
            sessions: None,
            linked: false,
            typing: Arc::default(),
            typed: Arc::default(),
            empty_at: Some(0),
            pasted: None,
            cancelled: None,
            killed: false,
            stop_when_idle: false,
            gen: 1,
            started: now_ms(),
            transcript: None,
            offset: 0,
            program: crate::engine::Program::Native(std::path::PathBuf::new()),
        }
    }
}

const MAX_TEXT: usize = 8000;
/// The plans of the plans folder an agent wrote that are remembered, and the calls waited for.
const MAX_PLANS_WRITTEN: usize = 5;
const MAX_PLAN_CALLS: usize = 64;
const MAX_INPUT_STR: usize = 4000;
/// A subagent's prompt (its « consigne ») is kept longer than another input's text.
const MAX_PROMPT: usize = 16_000;
/// The report a subagent hands back, kept with its call.
const MAX_REPORT: usize = 16_000;
const MAX_PATCH_LINES: usize = 800;

/// `path` is in the folder where the superpowers skills keep what a plan's execution leaves
/// (`.superpowers/sdd`, anywhere in the repository).
fn in_sdd_workspace(path: &str) -> bool {
    const SDD: &str = ".superpowers/sdd/";
    let slashed = path.replace('\\', "/").to_ascii_lowercase();
    slashed.starts_with(SDD) || slashed.contains(&format!("/{SDD}"))
}

/// The plans of the plans folder (`docs/superpowers/plans/*.md`) that a command writes, as it
/// spells them: how a run writes its plan when it does not use a write tool. A redirection
/// (`cat > …`, `>>`, `2>`), `tee`, the destination of `cp` or `mv`; not a plan the command only
/// reads, searches, counts, adds to git or copies from (a stale plan it names is not one it wrote),
/// and no pattern or variable.
fn plans_written_by(command: &str) -> Vec<&str> {
    let mut written = Vec::new();
    if !command.to_ascii_lowercase().contains("superpowers") {
        return written;
    }
    let is_flag = |w: &str| w.starts_with('-') || (w.len() <= 2 && w.starts_with('/'));
    for statement in command.split([';', '&', '|', '\n']) {
        let words: Vec<&str> = statement
            .split_whitespace()
            .take_while(|w| !w.starts_with('#'))
            .collect();
        // A redirection: `> file`, `>file`, `>> file`, `2> file`.
        for (i, word) in words.iter().enumerate() {
            let op = word.trim_start_matches(|c: char| c.is_ascii_digit());
            if let Some(rest) = op.strip_prefix(">>").or_else(|| op.strip_prefix('>')) {
                let target = if rest.is_empty() {
                    words.get(i + 1).copied()
                } else {
                    Some(rest)
                };
                written.extend(target);
            }
        }
        // `VAR=value` before the command is not the command.
        let mut command_words = words
            .iter()
            .copied()
            .skip_while(|w| w.split_once('=').is_some_and(|(var, _)| !var.is_empty()));
        let verb = command_words
            .next()
            .and_then(|verb| verb.rsplit(['/', '\\']).next())
            .map(str::to_ascii_lowercase);
        let args: Vec<&str> = command_words.filter(|w| !is_flag(w)).collect();
        match verb.as_deref() {
            Some("tee") => written.extend(args),
            Some("cp" | "mv" | "copy" | "move" | "copy-item" | "move-item") => {
                written.extend(args.last().copied())
            }
            _ => {}
        }
    }
    written
        .into_iter()
        .map(|w| w.trim_matches(['"', '\'']))
        .filter(|w| {
            let w = w.replace('\\', "/").to_ascii_lowercase();
            w.ends_with(".md")
                && w.contains("docs/superpowers/plans/")
                && !w.contains(['*', '?', '$', '{'])
        })
        .collect()
}

impl AgentRt {
    pub fn new(mut meta: AgentMeta, conv_dir: &std::path::Path) -> Self {
        // A turn cannot survive an app restart.
        if meta.status.is_active() {
            meta.status = AgentStatus::Done;
        }
        // Neither can what ran for it: the plan was saved with rows running.
        if let Some(plan) = meta.plan.as_mut() {
            plan.close_running(now_ms());
        }
        // Saved before Remote Control had a source: the user turned it on for this agent.
        if meta.remote_control && meta.remote_source.is_none() {
            meta.remote_source = Some(RemoteSource::Hand);
        }
        Self {
            conv: Conv::new(conv_dir, &meta.id),
            meta,
            facts: crate::facts::Emitter::default(),
            proc: None,
            gen: 0,
            active_since: None,
            context_tokens: 0,
            context_window: 0,
            main_model: String::new(),
            rate_limited: false,
            limit_resets_at: None,
            launch_stamp: None,
            sign_in_retries: 0,
            foreground_tasks: HashSet::new(),
            announced_tasks: HashMap::new(),
            commands: Vec::new(),
            interrupted: false,
            saw_init: false,
            remote_state: None,
            remote_linked: false,
            remote_tried: false,
            remote_fallback: false,
            remote_told: false,
            activity: None,
            plan_written: Vec::new(),
            plan_calls: HashSet::new(),
            plan_removals: HashSet::new(),
            plan_since: now_ms(),
            plan_list: None,
            plan_memory: BgMemory::default(),
            turn_text: String::new(),
            final_text: String::new(),
            setup: None,
            setup_output: None,
            setup_failure: None,
            blocks: HashMap::new(),
            current_msg: HashMap::new(),
            pending: Vec::new(),
            last_usage: HashMap::new(),
            live: HashMap::new(),
            queued: 0,
            tui: None,
            attach: None,
            tui_usage: HashMap::new(),
            tui_error: None,
            tui_prompt: None,
            tui_prompt_id: None,
            tui_since: None,
            tui_exit: None,
            tui_quick_exits: 0,
            unwritten_session: None,
            naming_asked: false,
            view_after_handover: false,
            unprompted: None,
            without_escouade: false,
            fence: crate::fence::Fence::default(),
            privacy: crate::fence::Privacy::default(),
            escouade_next: false,
            codex: codex::CodexState::default(),
            copilot: copilot::CopilotState::default(),
            cursor: cursor::CursorState::default(),
            opencode: opencode::OpencodeState::default(),
            vibe: vibe::VibeState::default(),
            agy: agy::AgyState::default(),
            gemini: gemini::GeminiState::default(),
            gemini_tui: gemini_tui::GeminiTuiState::default(),
        }
    }

    /// A process (or an interface) of the agent started, given Escouade's server or not; `grants`:
    /// a start now would give it (read after the access was decided: a switch turned on in
    /// between, whose `Core::mark_escouade_next` found no process yet, is not missed).
    pub(crate) fn started_with_escouade(&mut self, given: bool, grants: bool) {
        self.without_escouade = !given;
        self.escouade_next = !given && grants;
    }

    /// `prompt`, read from the transcript, is the message the app typed by itself (`unprompted`):
    /// forgotten then, so that the user typing the same words later is not mistaken for it.
    pub(crate) fn took_unprompted(&mut self, prompt: &str) -> bool {
        self.unprompted
            .take_if(|sent| sent.trim() == prompt.trim())
            .is_some()
    }

    /// What Haiku is to name the agent from, given a message of its user's: set down as asked
    /// (once per agent, while the app runs). None when the agent is named, was already asked, or
    /// the message gives nothing to name it by (`naming_text`): a later message may.
    pub(crate) fn name_from(&mut self, prompt: &str) -> Option<String> {
        if self.meta.named || self.naming_asked {
            return None;
        }
        let text = crate::core::naming_text(prompt)?;
        self.naming_asked = true;
        Some(text)
    }

    /// Tokens and estimated cost of the running turn so far.
    fn live_totals(&self) -> (u64, f64) {
        self.live.values().fold((0, 0.0), |(t, c), (model, u)| {
            (
                t + u.tokens(),
                c + pricing::estimate(model, u).unwrap_or(0.0),
            )
        })
    }

    pub fn view(&self) -> AgentView {
        let (live_tokens, live_cost) = self.live_totals();
        AgentView {
            meta: self.meta.clone(),
            active_since: self.active_since,
            alive: self.proc.is_some() || self.tui.is_some(),
            escouade_next: self.escouade_next && (self.proc.is_some() || self.tui.is_some()),
            terminal_id: self
                .tui
                .as_ref()
                .map(|t| t.term_id.clone())
                .or_else(|| self.attach.as_ref().map(|a| a.info.id.clone())),
            interface_remote: self.tui.as_ref().map(|t| t.remote),
            tui_exit: self.tui_exit.clone(),
            pending: self.pending.iter().map(|p| p.item_id.clone()).collect(),
            requests: self.pending.iter().map(|p| p.view.clone()).collect(),
            context_tokens: self.context_tokens,
            context_window: self.context_window,
            live_tokens,
            live_cost,
            remote_state: self.remote_state.clone(),
            activity: self.activity.clone(),
            setup: self.setup.clone(),
            isola: self
                .meta
                .worktree
                .as_ref()
                .is_some_and(|w| crate::isola::manages(&w.path)),
        }
    }

    /// Resets per-process bookkeeping when a new `claude` process is attached.
    pub fn attach(&mut self, proc: Process) {
        self.proc = Some(proc);
        // A process that just started is not idle.
        self.meta.last_activity = now_ms();
        self.blocks.clear();
        self.current_msg.clear();
        self.pending.clear();
        self.last_usage.clear();
        self.live.clear();
        self.remote_state = None;
        self.remote_linked = false;
        self.remote_tried = false;
        self.forget_turn();
        self.saw_init = false;
        self.queued = 0;
        self.foreground_tasks.clear();
        self.announced_tasks.clear();
        self.codex = codex::CodexState::default();
        self.copilot = copilot::CopilotState::default();
        self.cursor = cursor::CursorState::default();
        self.opencode = opencode::OpencodeState::default();
        self.vibe = vibe::VibeState::default();
        self.agy = agy::AgyState::default();
        self.gemini = gemini::GeminiState::default();
        // What ran in the process before is gone with it.
        self.interrupt_plan();
    }

    /// Runs `f` on the agent's plan (made when it has none yet), keeps it only when it holds
    /// something, and tells what changed: a detail goes to the window, the rest is saved too.
    fn plan_apply(&mut self, fx: &mut Effects, f: impl FnOnce(&mut PlanState) -> Change) {
        let before = ended_runs(self.meta.plan.as_ref());
        let mut plan = self
            .meta
            .plan
            .take()
            .unwrap_or_else(|| PlanState::with_memory(self.plan_memory.clone()));
        let change = f(&mut plan);
        self.plan_memory = plan.memory.clone();
        if !plan.is_empty() {
            self.meta.plan = Some(plan);
        }
        self.keep_runs(&before, Some(fx));
        match change {
            Change::None => {}
            Change::View => fx.agent_changed = true,
            Change::Saved => {
                fx.agent_changed = true;
                fx.save = true;
            }
        }
    }

    /// A message of the subagent `call` came from `model` (`PlanState::on_child_model`): the plan
    /// is only taken up when that is news, not at every message of the subagent.
    pub(crate) fn child_model(&mut self, call: &str, model: &str, fx: &mut Effects) {
        if self
            .meta
            .plan
            .as_ref()
            .is_some_and(|plan| plan.tells_model(call, model))
        {
            self.plan_apply(fx, |plan| plan.on_child_model(call, model));
        }
    }

    /// The process is gone: what the plan has running is interrupted (the caller sends the view).
    fn interrupt_plan(&mut self) -> Change {
        let now = now_ms();
        let before = ended_runs(self.meta.plan.as_ref());
        let change = self
            .meta
            .plan
            .as_mut()
            .map_or(Change::None, |plan| plan.close_running(now));
        self.keep_runs(&before, None);
        change
    }

    /// The subagents that ended since `before`, or whose totals came after their end: what the
    /// window shows of each once the plan has forgotten its row (30 at most, none after a reset)
    /// is kept on the item of its call (`run`), in the conversation and its log. Without `fx`
    /// (the process is gone, no frame to answer), it is only written down: the window, which has
    /// the plan's row meanwhile, reads it with the conversation.
    fn keep_runs(&mut self, before: &[EndedRun], mut fx: Option<&mut Effects>) {
        for run in ended_runs(self.meta.plan.as_ref()) {
            if before.contains(&run) || !self.conv.contains(&run.id) {
                continue;
            }
            let op = ConvOp::Patch {
                id: run.id.clone(),
                patch: json!({ "run": run.summary() }),
            };
            match fx.as_deref_mut() {
                Some(fx) => self.push(op, fx),
                None => self.conv.apply(&op),
            }
        }
    }

    /// The report a subagent hands back, kept on the item of its call (`report`): the
    /// `handback_report` of its task's notification, else, for a subagent that ended without
    /// `SubagentHandback` (no `handback` at all), its final text, which is then the `summary`
    /// (measured on Claude Code 2.1.300). A `summary` beside a `handback` only says where the
    /// report went, one of a task stopped or failed is its description, and the generic line of
    /// older versions (`Agent "…" finished`) only says that it ended: none is a report (for a
    /// subagent in the foreground, the result of its call is, and the window reads it there).
    fn keep_report(&mut self, f: &Value, fx: &mut Effects) {
        let handed = f["handback_report"]["text"].as_str();
        let summary = (f["handback"].is_null() && f["status"] == "completed")
            .then(|| f["summary"].as_str())
            .flatten()
            .filter(|s| !is_generic_summary(s));
        let Some(text) = handed.or(summary).filter(|t| !t.trim().is_empty()) else {
            return;
        };
        let task = f["task_id"].as_str();
        let row = self.meta.plan.as_ref().and_then(|p| {
            p.agents
                .iter()
                .find(|a| task.is_some() && a.task_id.as_deref() == task)
                .map(|a| a.id.clone())
        });
        let Some(call) = row.or_else(|| f["tool_use_id"].as_str().map(str::to_string)) else {
            return;
        };
        self.set_report(&call, text, false, fx);
    }

    /// `text` as the report on the item of the subagent call `call` (cut to `MAX_REPORT`), if that
    /// is one; with `only_missing`, if it has none yet.
    fn set_report(&mut self, call: &str, text: &str, only_missing: bool, fx: &mut Effects) {
        let Some(item) = self.conv.get(call) else {
            return;
        };
        let is_call =
            item["kind"] == "tool" && matches!(item["name"].as_str(), Some("Task" | "Agent"));
        let has_one = item["report"]
            .as_str()
            .is_some_and(|r| !r.trim().is_empty());
        if is_call && !(only_missing && has_one) {
            self.patch(call, json!({ "report": truncate(text, MAX_REPORT) }), fx);
        }
    }

    /// A background subagent's end as the Claude Code view tells it: there, no `system` frame says a
    /// task ended, only the notification Claude Code passes on to Claude (`origin.kind`
    /// `task-notification`, replayed). Applied to the plan as the frame would be, so that the row
    /// ends (`completed`, `failed`, `stopped` or `killed`). Its words are no report (the real
    /// frame, in the stream, has the report beside them; a replay's `summary` often only says where
    /// the report went), and a row that ended already stays as it was.
    ///
    /// The event of a monitor is replayed the same way, with no `<status>`: it ends nothing, so
    /// nothing is applied. The `summary` and the `output-file` of a command or a monitor that ended
    /// are kept on its row (`BgTask`); a subagent's row keeps neither.
    fn end_replayed_task(&mut self, text: &str, fx: &mut Effects) {
        if tag(text, "status").is_none() {
            return;
        }
        let summary = tag(text, "summary").map(unescape_tag);
        let file = tag(text, "output-file").map(unescape_tag);
        let frame = json!({
            "type": "system", "subtype": "task_notification", "task_id": tag(text, "task-id"),
            "tool_use_id": tag(text, "tool-use-id"), "status": tag(text, "status"),
            "summary": summary, "output_file": file,
        });
        self.plan_apply(fx, |plan| plan.on_system(&frame, now_ms()));
    }

    /// The report in a subagent's hand-back message (`origin.kind` `peer`, replayed), for the call of
    /// the row whose task sent it, if that call has none yet: in the Claude Code view, nothing else
    /// carries it.
    fn keep_handback(&mut self, text: &str, origin: Option<&str>, fx: &mut Effects) {
        let Some((from, report)) = handback_report(text) else {
            return;
        };
        let Some(task) = from.or(origin) else {
            return;
        };
        let call = self.meta.plan.as_ref().and_then(|p| {
            p.agents
                .iter()
                .find(|a| a.task_id.as_deref() == Some(task))
                .map(|a| a.id.clone())
        });
        if let Some(call) = call {
            self.set_report(&call, &report, true, fx);
            // And its end: in the interface's transcript, no notification of a subagent's task is
            // replayed (Claude Code 2.1.283), its hand-back is all there is. A row that ended
            // already (the stream's frame came first) stays as it was.
            let frame = json!({
                "type": "system", "subtype": "task_notification", "task_id": task,
                "tool_use_id": call, "status": "completed",
            });
            self.plan_apply(fx, |plan| plan.on_system(&frame, now_ms()));
        }
    }

    /// The subagent a request of Claude Code's comes from (the call that launched it): the one
    /// whose task `agent_id` names (Claude Code 2.1.300), else the one whose thread made the call
    /// that asks.
    fn subagent_of(&mut self, req: &Value) -> Option<String> {
        if let Some(task) = req["agent_id"].as_str() {
            let row = self.meta.plan.as_ref().and_then(|p| {
                p.agents
                    .iter()
                    .find(|a| a.task_id.as_deref() == Some(task))
                    .map(|a| a.id.clone())
            });
            if row.is_some() {
                return row;
            }
        }
        // Only a subagent's call: a thread that is no subagent's (a workflow's agent) names none.
        let call = req["tool_use_id"].as_str()?;
        let parent = self.conv.get(call)?["parent"].as_str()?.to_string();
        let launched = self.conv.get(&parent).is_some_and(|i| {
            i["kind"] == "tool" && matches!(i["name"].as_str(), Some("Task" | "Agent"))
        });
        launched.then_some(parent)
    }

    /// A new plan: what the last forgets, and the files it will read are those touched from now on.
    fn reset_plan(&mut self, fx: &mut Effects) {
        self.plan_apply(fx, PlanState::reset);
        self.plan_since = now_ms();
        self.plan_written.clear();
        self.plan_calls.clear();
        self.plan_removals.clear();
        self.plan_list = None;
    }

    /// What a look at the agent's folder read (`planfiles`, off its lock), unless its conversation
    /// is not the one the look was made for any more: then nothing is applied (false).
    pub fn plan_from_files(
        &mut self,
        since: i64,
        list: Option<FileList>,
        fx: &mut Effects,
    ) -> bool {
        if since != self.plan_since {
            return false;
        }
        if list.is_some() {
            self.plan_list = list.clone();
        }
        self.plan_apply(fx, |plan| plan.set_files(list));
        true
    }

    fn push(&mut self, mut op: ConvOp, fx: &mut Effects) {
        self.spoken_apart(&mut op);
        self.conv.apply(&op);
        fx.ops.push(op);
    }

    /// A text of the agent once written whole, whatever its harness: its passage to be read aloud
    /// (`voice::spoken`), when it has one and is the agent's own (not a subagent's), kept apart from
    /// the rest (`spoken`), as the conversation shows it and the voice reads it.
    fn spoken_apart(&mut self, op: &mut ConvOp) {
        let (target, change) = match op {
            ConvOp::Append { item } => (item.clone(), item),
            ConvOp::Patch { id, patch } => match self.conv.get(id) {
                Some(item) => (item.clone(), patch),
                None => return,
            },
            _ => return,
        };
        if target["kind"] != "text"
            || !target["parent"].is_null()
            || change["streaming"] != false
            || !change["spoken"].is_null()
        {
            return;
        }
        let Some((rest, passage)) = change["text"]
            .as_str()
            .and_then(crate::voice::spoken::extract)
        else {
            return;
        };
        change["text"] = json!(rest);
        change["spoken"] = json!(passage);
    }

    pub(crate) fn append(&mut self, item: Value, fx: &mut Effects) {
        self.push(ConvOp::Append { item }, fx);
    }

    pub(crate) fn patch(&mut self, id: &str, patch: Value, fx: &mut Effects) {
        self.push(
            ConvOp::Patch {
                id: id.to_string(),
                patch,
            },
            fx,
        );
    }

    pub fn notice(&mut self, level: &str, text: impl Into<String>, fx: &mut Effects) {
        let item = json!({ "kind": "notice", "id": new_id(), "ts": now_ms(), "level": level, "text": text.into() });
        self.append(item, fx);
    }

    /// Drops what the running turn showed (its activity, its text): the turn is over, or lost.
    fn forget_turn(&mut self) {
        self.activity = None;
        self.turn_text.clear();
        self.final_text.clear();
    }

    /// What the agent does now, shown live; unchanged by a tool it has no words for, and never
    /// blurred: a bare verb ("Lit", a tool that only began) does not replace what the same verb
    /// already told ("Lit src/a.ts").
    fn set_activity(&mut self, activity: Option<String>, fx: &mut Effects) {
        let Some(activity) = activity else {
            return;
        };
        let blurs = !activity.contains(' ')
            && self
                .activity
                .as_deref()
                .is_some_and(|current| current.starts_with(&format!("{activity} ")));
        if blurs || self.activity.as_ref() == Some(&activity) {
            return;
        }
        self.activity = Some(activity);
        fx.agent_changed = true;
    }

    pub fn set_status(&mut self, status: AgentStatus, fx: &mut Effects) {
        if self.meta.status == status {
            return;
        }
        let now = now_ms();
        let (was, is) = (self.meta.status.is_active(), status.is_active());
        if was && !is {
            if let Some(since) = self.active_since.take() {
                self.meta.active_ms += (now - since).max(0) as u64;
            }
        } else if !was && is {
            self.active_since = Some(now);
            // A prompt was sent: the one a cancelled turn put back is no longer there.
            if let Some(t) = self.tui.as_mut() {
                t.cancelled = None;
                t.empty_at = Some(t.typed.load(std::sync::atomic::Ordering::Acquire));
            }
        }
        let was = std::mem::replace(&mut self.meta.status, status);
        self.fact(crate::facts::Body::AgentStatus {
            status: crate::facts::status_word(status),
            was: crate::facts::status_word(was),
        });
        self.meta.last_activity = now;
        fx.agent_changed = true;
        fx.save = true;
    }

    /// Records a user message delivered to Claude under `id` (the frame uuid), with its image
    /// count and the names of its other attached files. Returns true when it was queued behind
    /// a running turn.
    #[cfg(test)]
    pub fn push_user(
        &mut self,
        id: &str,
        text: &str,
        images: u32,
        files: &[String],
        fx: &mut Effects,
    ) -> bool {
        self.push_user_from(id, text, images, files, None, fx)
    }

    /// `origin`: "remote" for a message sent from claude.ai / the Claude app (Remote Control),
    /// "handover" for the handover file of a harness the agent goes on on (`handover`).
    pub(crate) fn push_user_from(
        &mut self,
        id: &str,
        text: &str,
        images: u32,
        files: &[String],
        origin: Option<&str>,
        fx: &mut Effects,
    ) -> bool {
        let active = self.meta.status.is_active();
        // A harness whose running turn reads a message at once (Codex: `turn/steer`) queues none.
        let steered = active
            && engine::of(&self.meta).caps().mid_turn
                == crate::engine::registry::MidTurn::Immediate;
        let queued = active && !steered;
        let mut item = json!({ "kind": "user", "id": id, "text": text, "images": images, "ts": now_ms(), "queued": queued });
        if !files.is_empty() {
            item["files"] = json!(files);
        }
        if let Some(o) = origin {
            item["origin"] = json!(o);
        }
        // Someone took over: no resume by itself after the usage limit, nor after a sign-in.
        self.meta.resume_at = None;
        self.meta.sign_in = None;
        self.append(item, fx);
        if queued {
            self.queued += 1;
        } else if !steered {
            self.turn_text.clear();
            self.final_text.clear();
            self.interrupted = false;
            self.set_status(AgentStatus::Running, fx);
            // A new turn after a plan that is over (or that was none) starts a plan of its own; one
            // that is under way goes on, and so does a message queued behind a turn.
            if self.meta.plan.as_ref().is_some_and(PlanState::is_finished) {
                self.reset_plan(fx);
            }
        }
        self.meta.prompts += 1;
        self.meta.last_activity = now_ms();
        fx.agent_changed = true;
        fx.save = true;
        queued
    }

    /// Deliberate stop (idle, archive): detaches the process so that its exit is ignored and
    /// the next action spawns a fresh one resuming the same session.
    pub fn detach(&mut self) -> Option<Process> {
        // Nothing more is heard from the process, so a turn it was running never ends here, nor
        // what it ran for the plan (the caller sends the view).
        self.forget_turn();
        self.interrupt_plan();
        let p = self.proc.take()?;
        self.gen += 1;
        // Its exit is ignored from now on, so the link it held is dropped here.
        self.remote_state = None;
        self.remote_linked = false;
        self.remote_tried = false;
        Some(p)
    }

    /// Remote Control ends for the agent: no longer in force, its remote session forgotten. The
    /// live process, when it was linked, is given back with the session to close (`end_link`).
    pub fn end_remote(&mut self) -> Option<RemoteLink> {
        self.meta.remote_control = false;
        self.meta.remote_source = None;
        self.end_link()
    }

    /// The link of the live process to claude.ai ends; Remote Control stays in force for the
    /// agent (its interface will hold its own). The process, when it was linked, is given back
    /// with the session it held: closing it is up to the caller (`Core::close_link`). The
    /// session's id is kept (`meta.remote_session`) until a closing has worked
    /// (`forget_remote_session`): with no live linked process (just started, a link that failed,
    /// a process gone) nothing can close it now, and the agent's next process does.
    pub fn end_link(&mut self) -> Option<RemoteLink> {
        let bridge = self.meta.remote_session.clone();
        self.meta.remote_url = None;
        // In the Claude Code view the state is the interface's, which keeps its link until its
        // next launch.
        if !self.meta.terminal {
            self.remote_state = None;
        }
        // A link to claude.ai is to be tried again when it comes back.
        self.remote_tried = false;
        let linked = std::mem::take(&mut self.remote_linked);
        self.proc
            .clone()
            .filter(|_| linked)
            .map(|proc| RemoteLink { proc, bridge })
    }

    /// The remote session `bridge` is closed on claude.ai: the agent no longer holds it, unless
    /// it holds another one meanwhile.
    pub fn forget_remote_session(&mut self, bridge: Option<&str>) {
        if self.meta.remote_session.as_deref() == bridge {
            self.meta.remote_session = None;
        }
    }

    /// Tells the agent its Remote Control is unavailable, unless it was told already and nothing
    /// has worked since (`remote_told`).
    pub fn remote_unavailable_notice(&mut self, text: String, fx: &mut Effects) {
        if !std::mem::replace(&mut self.remote_told, true) {
            self.notice("warn", text, fx);
        }
    }

    pub fn clear_pending(&mut self, fx: &mut Effects) {
        let ids: Vec<String> = self.pending.drain(..).map(|p| p.item_id).collect();
        for id in ids {
            self.patch(&id, json!({ "cancelled": true }), fx);
        }
    }

    /// Closes what a turn left open: partial text is kept (and persisted, deltas being
    /// memory-only), tools that never returned are marked interrupted.
    pub(crate) fn close_open_items(&mut self, fx: &mut Effects) {
        for (id, text) in self.conv.open_items() {
            let patch = match text {
                Some(text) => json!({ "text": text, "streaming": false }),
                None => json!({ "status": "interrupted" }),
            };
            self.patch(&id, patch, fx);
        }
        for blocks in self.blocks.values_mut() {
            for b in blocks.iter_mut() {
                b.finalized = true;
            }
        }
    }

    /// A request comes after those waiting already: the one asked first stays the one shown
    /// first. Asked again, a request keeps its place, and the time it first came.
    fn add_pending(&mut self, req: PendingReq) {
        match self.pending.iter_mut().find(|p| p.item_id == req.item_id) {
            Some(p) => {
                let since = p.since;
                *p = PendingReq { since, ..req };
            }
            None => {
                let v = &req.view;
                self.fact(if v.kind == "question" {
                    crate::facts::Body::RequestQuestion {
                        request_id: v.id.clone(),
                        questions: v.questions.iter().map(|q| q.question.clone()).collect(),
                    }
                } else {
                    crate::facts::Body::RequestPermission {
                        request_id: v.id.clone(),
                        tool: v.tool.clone(),
                        arg: v.arg.chars().take(300).collect(),
                    }
                });
                self.pending.push(req)
            }
        }
    }

    /// Tells `body` of this agent on the facts' bus (PLG-5).
    pub(crate) fn fact(&self, body: crate::facts::Body) {
        self.facts.emit(
            body,
            crate::facts::About::agent(&self.meta.id, &self.meta.name, &self.meta.project_id),
        );
    }

    /// The request `request_id` has its answer already: the user's, in the window, or a client's.
    /// Its card in the conversation holds it (a request cancelled has none).
    fn was_answered(&mut self, request_id: &str) -> bool {
        self.conv
            .get(request_id)
            .is_some_and(|item| match item["kind"].as_str() {
                Some("permission") => !item["decision"].is_null(),
                Some("question") => !item["answers"].is_null(),
                _ => false,
            })
    }

    /// The views of the requests waiting for an answer, in the order they came: counted or typed
    /// without copying them (`waiting` copies what Claude Code sent).
    pub fn waiting_views(&self) -> impl Iterator<Item = &PendingView> {
        self.pending.iter().map(|p| &p.view)
    }

    /// The requests waiting for an answer, in the order they came.
    pub fn waiting(&self) -> Vec<Waiting> {
        self.pending
            .iter()
            .map(|p| Waiting {
                view: p.view.clone(),
                since: p.since,
                input: p.input.clone(),
            })
            .collect()
    }

    /// Takes the request `request_id` out of those waiting, the others keeping their order.
    fn take_pending(&mut self, request_id: &str) -> Option<PendingReq> {
        let i = self.pending.iter().position(|p| p.item_id == request_id)?;
        Some(self.pending.remove(i))
    }

    #[cfg(test)]
    pub fn has_pending(&self, request_id: &str) -> bool {
        self.pending.iter().any(|p| p.item_id == request_id)
    }

    /// The answer to a question: the user's in the window, or `by`'s (a client of the MCP server,
    /// named as its activity log names it), which the conversation's card then says.
    pub fn answer_question_by(
        &mut self,
        request_id: &str,
        answers: Value,
        by: Option<&str>,
        fx: &mut Effects,
    ) -> Result<()> {
        let proc = self
            .proc
            .clone()
            .ok_or_else(|| not_running(engine::of(&self.meta).speaker()))?;
        let Some(p) = self.take_pending(request_id) else {
            bail!(if self.was_answered(request_id) {
                already_answered()
            } else {
                tr!("question introuvable", "question not found")
            });
        };
        let mut input = p.input.clone();
        // Claude reads an answer given through the server with who gave it, never as the user's
        // own words; the card keeps it as it was given.
        input["answers"] = match by {
            Some(by) => signed_answers(i18n::claude(), by, &answers),
            None => answers.clone(),
        };
        proc.answer_question(request_id, &p.tool_use_id, input)?;
        self.fact(crate::facts::Body::RequestAnswered {
            request_id: request_id.to_string(),
            kind: "question".into(),
            decision: None,
            by: by.map(str::to_string),
        });
        let mut patch = json!({ "answers": answers });
        answered_by(&mut patch, by);
        self.patch(&p.item_id, patch, fx);
        self.after_answer(fx);
        Ok(())
    }

    /// The answer to a permission: the user's in the window, or `by`'s (as `answer_question_by`).
    /// A refusal by `by` tells Claude who refused (« Refusé par <by> via Escouade »), never that
    /// the user did.
    pub fn answer_permission_by(
        &mut self,
        request_id: &str,
        decision: &str,
        message: Option<String>,
        by: Option<&str>,
        fx: &mut Effects,
    ) -> Result<()> {
        let auto = engine::of(&self.meta).supports_auto(&self.meta.model);
        self.answer_permission_in(request_id, decision, message, by, auto, fx)
    }

    /// `answer_permission_by`, `takes_auto` saying whether its model takes the mode `auto`
    /// (`Core::takes_auto`): a plan approved goes on in the mode it was in before Plan
    /// (`AgentMeta::plan_resume`, ESC-277), which its card and its Composer show at once.
    pub fn answer_permission_in(
        &mut self,
        request_id: &str,
        decision: &str,
        message: Option<String>,
        by: Option<&str>,
        takes_auto: bool,
        fx: &mut Effects,
    ) -> Result<()> {
        let proc = self
            .proc
            .clone()
            .ok_or_else(|| not_running(engine::of(&self.meta).speaker()))?;
        let Some(p) = self.take_pending(request_id) else {
            bail!(if self.was_answered(request_id) {
                already_answered()
            } else {
                tr!("demande introuvable", "request not found")
            });
        };
        // A plan sent back through the server without a word: as the window says it, keep
        // planning (the card keeps it too, as the window's does).
        let message = if decision == "deny"
            && by.is_some()
            && p.view.tool == "ExitPlanMode"
            && blank(&message)
        {
            Some(keep_planning(i18n::claude()))
        } else {
            message
        };
        let call = p.tool_use_id.clone();
        let plan = p.view.tool == "ExitPlanMode";
        let answer = match decision {
            "allow" => PermissionAnswer::Allow {
                call,
                input: p.input.clone(),
                resume: plan.then(|| self.meta.plan_resume(takes_auto)),
            },
            "always" => PermissionAnswer::Always {
                call,
                input: p.input.clone(),
                rules: p.suggestions.clone(),
            },
            _ => {
                let told = match by {
                    Some(by) => refused_by(i18n::claude(), by, message.as_deref()),
                    None => message
                        .clone()
                        .filter(|m| !m.trim().is_empty())
                        .unwrap_or_else(|| refused_action(i18n::claude())),
                };
                PermissionAnswer::Deny {
                    call,
                    message: told,
                }
            }
        };
        let resumed = match &answer {
            PermissionAnswer::Allow {
                resume: Some(mode), ..
            } => Some(mode.clone()),
            PermissionAnswer::Always { .. } if plan => Some("acceptEdits".to_string()),
            _ => None,
        };
        proc.answer_permission(request_id, answer)?;
        self.fact(crate::facts::Body::RequestAnswered {
            request_id: request_id.to_string(),
            kind: "permission".into(),
            decision: Some(match decision {
                "allow" | "always" => decision.to_string(),
                _ => "deny".to_string(),
            }),
            by: by.map(str::to_string),
        });
        // Its mode now, without waiting for its harness to tell it (the Composer follows).
        if let Some(mode) = resumed {
            if self.meta.set_mode(&mode) {
                fx.save = true;
            }
        }
        let mut patch = json!({ "decision": decision, "message": message });
        answered_by(&mut patch, by);
        self.patch(&p.item_id, patch, fx);
        self.after_answer(fx);
        Ok(())
    }

    fn after_answer(&mut self, fx: &mut Effects) {
        if self.pending.is_empty() && self.meta.status == AgentStatus::Waiting {
            self.set_status(AgentStatus::Running, fx);
        }
        fx.agent_changed = true;
    }

    /// The process exited. `gen` guards against a stale exit of a replaced process.
    pub fn on_exit(&mut self, gen: u64, code: Option<i32>, stderr: &str, fx: &mut Effects) {
        if gen != self.gen {
            return;
        }
        self.proc = None;
        self.live.clear();
        self.remote_state = None;
        self.remote_linked = false;
        self.remote_tried = false;
        self.forget_turn();
        self.plan_apply(fx, |plan| plan.close_running(now_ms()));
        self.clear_pending(fx);
        self.close_open_items(fx);
        let was_running = self.meta.status.is_active();
        // Gone before it started, on the account a resume moved it to: the resume's failure. The
        // session is not given up (it is where the way back goes from, and the old account has
        // it): the card tells the failure, with « Revenir sur … ».
        let resume_failed = !self.saw_init && self.meta.moved_from.is_some();
        // Gone before it started: what it was to resume could not be.
        let harness = engine::of(&self.meta);
        let [session_lost, fork_point_lost] = exit_notices(i18n::ui(), harness.speaker());
        let cause = harness.start_lost(stderr);
        let lost = if self.saw_init || resume_failed {
            None
        } else if cause == Some(StartLost::Session) {
            // A copy whose original's session is gone: the same, a new session.
            self.forget_session();
            Some(session_lost)
        } else if self.meta.fork_at.is_some() && cause == Some(StartLost::ForkPoint) {
            // A copy whose point in its original's session is not there: still a copy of that
            // session, of all of it.
            self.meta.fork_at = None;
            Some(fork_point_lost)
        } else {
            None
        };
        if let Some(lost) = lost {
            self.notice("warn", lost.clone(), fx);
            self.set_status(AgentStatus::Done, fx);
            // The message it was to carry never ran: that turn ends here, though not as a failure
            // of the agent.
            if was_running {
                fx.turn_end = Some(TurnEnd::Error(lost));
            }
        } else if was_running || !self.saw_init {
            let detail = if stderr.trim().is_empty() {
                String::new()
            } else {
                format!("\n\n{}", truncate(stderr.trim(), 2000))
            };
            let code = code.map(|c| c.to_string()).unwrap_or_else(|| "?".into());
            let mut reason = stopped(i18n::ui(), harness.name(), &code);
            // A `claude` that stopped before it started: its installation may have been cut
            // half-way (ESC-133), which is said with its repair.
            if !self.saw_init && harness.id() == EngineId::Claude {
                if let Some(line) = crate::engine::claude_interrupted_line(i18n::ui()) {
                    reason = format!("{reason}. {line}");
                }
            }
            if resume_failed {
                // What the process said, else that it stopped: the card says it as the error.
                let said = if stderr.trim().is_empty() {
                    reason.clone()
                } else {
                    truncate(stderr.trim(), 2000)
                };
                let turn = json!({
                    "kind": "turn", "id": new_id(), "ts": now_ms(), "durationMs": null,
                    "cost": 0, "tokens": 0, "isError": true, "interrupted": false,
                    "error": said, "limited": false, "engine": self.meta.engine,
                });
                self.append(turn, fx);
            } else {
                self.notice("error", format!("{reason}.{detail}"), fx);
            }
            self.set_status(AgentStatus::Error, fx);
            fx.notify = Some(AgentAlert::new(
                NotifyKind::Error,
                notify::error_body(&reason),
            ));
            if was_running {
                fx.turn_end = Some(TurnEnd::Error(reason));
            }
        }
        fx.agent_changed = true;
    }

    /// One frame of its process, translated into its conversation by its engine.
    pub fn handle_frame(&mut self, f: &Value, fx: &mut Effects) {
        engine::of(&self.meta).translate(self, f, fx);
        if self.meta.discussion.is_some() && !self.pending.is_empty() {
            self.discussion_answers(fx);
        }
    }

    /// A discussion of the Docs tab (ESC-261) never changes a file nor runs a command: what it asks
    /// for that is refused on the spot, whatever its harness; Escouade's own tools (the docs it
    /// writes) go through without a question. The rest waits for the user, as for any agent.
    fn discussion_answers(&mut self, fx: &mut Effects) {
        let answers: Vec<(String, &str)> = self
            .pending
            .iter()
            .filter(|p| p.view.kind == "permission")
            .filter_map(|p| {
                let tool = p.view.tool.as_str();
                if crate::discussion::refused_tool(tool) {
                    Some((p.item_id.clone(), "deny"))
                } else if crate::discussion::allowed_tool(tool) {
                    Some((p.item_id.clone(), "allow"))
                } else {
                    None
                }
            })
            .collect();
        if answers.is_empty() {
            return;
        }
        for (id, decision) in answers {
            // The refusal says itself that Escouade refused, and why.
            let message = (decision == "deny").then(|| crate::discussion::refusal(i18n::claude()));
            if let Err(e) = self.answer_permission_by(&id, decision, message, None, fx) {
                log::warn!(
                    "agent {}: discussion's answer to {id} not given: {e:#}",
                    self.meta.id
                );
            }
        }
        // Nobody is to be called for what was answered.
        if self.pending.is_empty() {
            fx.notify = None;
        }
    }
}

/// What is between `<name>` and `</name>` in `text` (the tags of a notification), trimmed; None when
/// there is none or nothing in it.
fn tag<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let (_, rest) = text.split_once(&format!("<{name}>"))?;
    let (inner, _) = rest.split_once(&format!("</{name}>"))?;
    Some(inner.trim()).filter(|s| !s.is_empty())
}

/// The text of a tag of a notification as it was written: Claude Code escapes `&`, `<` and `>`
/// (and the quotes) in what it puts between the tags.
fn unescape_tag(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// The line an older Claude Code, and the fake `claude`, give as the `summary` of a subagent's
/// notification: `Agent "<title>" finished` (or `completed`, `failed`, `stopped`), with or without a
/// final dot. It only says that the subagent ended: it is no report.
fn is_generic_summary(summary: &str) -> bool {
    let line = summary.trim();
    let line = line.strip_suffix('.').unwrap_or(line);
    if line.contains(['\n', '\r']) {
        return false;
    }
    let Some(rest) = line.strip_prefix("Agent \"") else {
        return false;
    };
    ["finished", "completed", "failed", "stopped"]
        .iter()
        .any(|word| {
            rest.strip_suffix(word)
                .is_some_and(|title| title.ends_with("\" "))
        })
}

/// The report in a subagent's hand-back message, and who sent it (the `from` of its frame): the
/// text that follows « The report follows: », without the indent Claude Code gives it. None for any
/// other message.
fn handback_report(text: &str) -> Option<(Option<&str>, String)> {
    let open = text.find("<agent-message")?;
    let start = open + text[open..].find('>')? + 1;
    let from = text[open..start]
        .split_once("from=\"")
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(from, _)| from);
    // The last closing tag: a report may quote the tag itself.
    let body = match text.rfind("</agent-message>") {
        Some(close) if close > start => &text[start..close],
        _ => &text[start..],
    };
    if !body.contains("[Subagent hand-back]") {
        return None;
    }
    let (_, report) = body.split_once("The report follows:")?;
    let report: Vec<&str> = report
        .lines()
        .map(|l| l.strip_prefix("  ").unwrap_or(l))
        .collect();
    let report = report.join("\n");
    let report = report.trim();
    (!report.is_empty()).then(|| (from, report.to_string()))
}

fn tool_result_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|b| b["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// The `result` of a tool's item: `text`, the words of its result block, but a command's own
/// output when the structured result (`tool_use_result` in the stream, `toolUseResult` in a
/// transcript) has it, cut to `MAX_TEXT`; for a file it changed, the lines of its diff and its path.
fn tool_result(mut text: String, is_error: bool, tur: &Value) -> Value {
    let mut result = json!({ "isError": is_error });
    if let Some(stdout) = tur["stdout"].as_str() {
        text = stdout.to_string();
        if let Some(stderr) = tur["stderr"].as_str().filter(|s| !s.is_empty()) {
            text = format!("{text}\n{stderr}");
        }
    }
    result["text"] = Value::String(truncate(text.trim_end(), MAX_TEXT));
    if let Some(hunks) = tur["structuredPatch"].as_array() {
        let (add, del, patch) = if hunks.is_empty() && tur["type"] == "create" {
            created_file_patch(tur["content"].as_str().unwrap_or(""))
        } else {
            summarize_patch(hunks)
        };
        result["add"] = add.into();
        result["del"] = del.into();
        result["patch"] = patch;
        if let Some(path) = tur["filePath"].as_str() {
            result["filePath"] = path.into();
        }
    }
    result
}

fn summarize_patch(hunks: &[Value]) -> (u32, u32, Value) {
    let (mut add, mut del, mut kept_total) = (0u32, 0u32, 0usize);
    let mut out = Vec::new();
    for h in hunks {
        let lines = h["lines"].as_array().map(Vec::as_slice).unwrap_or(&[]);
        for l in lines {
            match l.as_str().unwrap_or("").chars().next() {
                Some('+') => add += 1,
                Some('-') => del += 1,
                _ => {}
            }
        }
        if kept_total < MAX_PATCH_LINES {
            let kept: Vec<Value> = lines
                .iter()
                .take(MAX_PATCH_LINES - kept_total)
                .map(|l| Value::String(truncate(l.as_str().unwrap_or(""), 400)))
                .collect();
            kept_total += kept.len();
            out.push(
                json!({ "oldStart": h["oldStart"], "newStart": h["newStart"], "lines": kept }),
            );
        }
    }
    (add, del, Value::Array(out))
}

fn created_file_patch(content: &str) -> (u32, u32, Value) {
    let lines: Vec<&str> = content.lines().collect();
    let kept: Vec<Value> = lines
        .iter()
        .take(MAX_PATCH_LINES)
        .map(|l| Value::String(format!("+{}", truncate(l, 400))))
        .collect();
    (
        lines.len() as u32,
        0,
        json!([{ "oldStart": 0, "newStart": 1, "lines": kept }]),
    )
}

/// A tool call's input, as its item keeps it: each text cut to `MAX_INPUT_STR`, but a subagent's
/// prompt (`Task` / `Agent`), its view's « consigne », to `MAX_PROMPT`.
fn item_input(name: &str, input: &Value) -> Value {
    let mut kept = trim_strings(input);
    if matches!(name, "Task" | "Agent") {
        if let Some(prompt) = input["prompt"].as_str() {
            kept["prompt"] = Value::String(truncate(prompt, MAX_PROMPT));
        }
    }
    kept
}

/// What `keep_runs` compares of an ended subagent.
#[derive(Debug, Clone, PartialEq)]
struct EndedRun {
    id: String,
    status: RunStatus,
    model: Option<String>,
    tokens: Option<u64>,
    tools: u32,
    started_at: i64,
    ended_at: Option<i64>,
}

impl EndedRun {
    /// The `run` of its call's item.
    fn summary(&self) -> Value {
        let mut run = json!({
            "status": self.status, "tools": self.tools, "startedAt": self.started_at,
            "endedAt": self.ended_at,
        });
        if let Some(tokens) = self.tokens {
            run["tokens"] = tokens.into();
        }
        if let Some(model) = &self.model {
            run["model"] = model.as_str().into();
        }
        run
    }
}

/// The subagents of `plan` that ended.
fn ended_runs(plan: Option<&PlanState>) -> Vec<EndedRun> {
    plan.map(|p| {
        p.agents
            .iter()
            .filter(|a| a.status != RunStatus::Running)
            .map(|a| EndedRun {
                id: a.id.clone(),
                status: a.status,
                model: a.model.clone(),
                tokens: a.tokens,
                tools: a.tools,
                started_at: a.started_at,
                ended_at: a.ended_at,
            })
            .collect()
    })
    .unwrap_or_default()
}

/// Truncates long strings inside tool inputs (file contents, big edits) before they reach the UI.
fn trim_strings(v: &Value) -> Value {
    match v {
        Value::String(s) if s.len() > MAX_INPUT_STR => Value::String(truncate(s, MAX_INPUT_STR)),
        Value::Array(a) => Value::Array(a.iter().map(trim_strings).collect()),
        Value::Object(o) => Value::Object(
            o.iter()
                .map(|(k, v)| (k.clone(), trim_strings(v)))
                .collect::<Map<_, _>>(),
        ),
        other => other.clone(),
    }
}

fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    strip_ansi(out.trim())
}

pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            match chars.peek() {
                Some(&'[') => {
                    chars.next();
                    for n in chars.by_ref() {
                        if n.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                // An OSC (a link, a title): to its BEL or its ST (ESC \).
                Some(&']') => {
                    while let Some(n) = chars.next() {
                        if n == '\u{7}' {
                            break;
                        }
                        if n == '\u{1b}' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// `text` in `MAX_PENDING_FIELD` characters at most; `cut` is set when it was longer.
fn capped(text: &str, cut: &mut bool) -> String {
    if text.chars().count() <= MAX_PENDING_FIELD {
        return text.to_string();
    }
    *cut = true;
    text.chars().take(MAX_PENDING_FIELD).collect()
}

/// How Claude Code names the tools of Escouade's own MCP server: `mcp__escouade__<tool>`.
const ESCOUADE_TOOLS: &str = "mcp__escouade__";

/// What a tool of Escouade is about, first found first (`toolArg` in `src/lib/tools.ts` reads
/// them in the same order): the ticket it makes, the ticket or the agent it acts on, what an agent
/// it starts is told, the line an agent reports, the project it reads.
const ESCOUADE_MAIN_ARGS: &[&str] = &["title", "ticket", "agent", "message", "line", "project"];

/// What a tool is asked to act on, as the conversation's card sums it up (`toolArg` in
/// `src/lib/tools.ts`): the command, the file from the agent's folder, the search…, its line
/// breaks kept. `SUMMED_UP` there lists the tools whose summary says all a permission asks:
/// only those are answered from « Vue d'ensemble ».
fn tool_arg(tool: &str, input: &Value, cwd: &str) -> String {
    tool_arg_in(i18n::ui(), tool, input, cwd)
}

fn tool_arg_in(lang: Lang, tool: &str, input: &Value, cwd: &str) -> String {
    let text = |key: &str| input[key].as_str().unwrap_or_default().to_string();
    let path = |key: &str| {
        input[key]
            .as_str()
            .map(|p| relative_slash(cwd, p))
            .unwrap_or_default()
    };
    match tool {
        "Bash" | "PowerShell" | "SlashCommand" => text("command"),
        "Read" | "Edit" | "Write" | "MultiEdit" => path("file_path"),
        "NotebookEdit" => path("notebook_path"),
        "Grep" | "Glob" => match path("path") {
            p if p.is_empty() => text("pattern"),
            p => format!("{}  {p}", text("pattern")),
        },
        "WebFetch" => text("url"),
        "WebSearch" => text("query"),
        "Task" | "Agent" => input["description"]
            .as_str()
            .or(input["subagent_type"].as_str())
            .unwrap_or_default()
            .to_string(),
        "TodoWrite" => input["todos"]
            .as_array()
            .map(|t| {
                let n = t.len();
                tr_n_in!(lang, n, "{n} tâche", "{n} tâches", "{n} task", "{n} tasks")
            })
            .unwrap_or_default(),
        "Skill" => input["skill"]
            .as_str()
            .or(input["command"].as_str())
            .unwrap_or_default()
            .to_string(),
        // A tool of Escouade's own server: its main argument (the ticket's title, the ticket, the
        // agent…), whatever comes first in what Claude sends. Its card lists them all.
        t if t.starts_with(ESCOUADE_TOOLS) => ESCOUADE_MAIN_ARGS
            .iter()
            .find_map(|k| input[*k].as_str())
            .unwrap_or_default()
            .to_string(),
        // The first text of its input (the keys in their order, as the card reads them).
        _ => input
            .as_object()
            .and_then(|o| o.values().find_map(Value::as_str))
            .unwrap_or_default()
            .to_string(),
    }
}

/// The first `MAX_PENDING_ITEMS` of the list `list`; `cut` is set when it had more.
fn few<'a>(list: &'a Value, cut: &mut bool) -> &'a [Value] {
    let list = list.as_array().map(Vec::as_slice).unwrap_or_default();
    *cut |= list.len() > MAX_PENDING_ITEMS;
    &list[..list.len().min(MAX_PENDING_ITEMS)]
}

/// What the agent's view says of the request `rid` (`req`, Claude Code's `can_use_tool`) for
/// « Vue d'ensemble », which shows it without the conversation: each text capped, `cut` set
/// when one was longer.
fn pending_view(rid: &str, tool: &str, req: &Value, input: &Value, cwd: &str) -> PendingView {
    let mut cut = false;
    let text = |v: &Value, cut: &mut bool| v.as_str().map(|s| capped(s, cut));
    if tool == "AskUserQuestion" {
        let questions = few(&input["questions"], &mut cut)
            .iter()
            .map(|q| PendingQuestion {
                question: text(&q["question"], &mut cut).unwrap_or_default(),
                options: few(&q["options"], &mut cut)
                    .iter()
                    .map(|o| text(&o["label"], &mut cut).unwrap_or_default())
                    .collect(),
                multi: q["multiSelect"] == true,
            })
            .collect();
        return PendingView {
            id: rid.to_string(),
            kind: "question".into(),
            tool: tool.to_string(),
            arg: String::new(),
            description: None,
            reason: None,
            questions,
            cut,
            default_no: false,
            can_always: false,
            always_scope: None,
        };
    }
    let arg = capped(&tool_arg(tool, input, cwd), &mut cut);
    let description = text(&req["description"], &mut cut);
    let reason = req["decision_reason"]
        .as_str()
        .map(|r| capped(&strip_ansi(r), &mut cut));
    PendingView {
        id: rid.to_string(),
        kind: "permission".into(),
        tool: tool.to_string(),
        arg,
        description,
        reason,
        questions: vec![],
        cut,
        default_no: req["default_to_no"].as_bool().unwrap_or(false),
        can_always: false,
        always_scope: None,
    }
}

/// What Claude is told of an action the user refused without saying why, in `lang` (the language
/// of the texts for Claude).
pub(crate) fn refused_action(lang: Lang) -> String {
    tr_in!(
        lang,
        "L'utilisateur a refusé cette action.",
        "The user refused this action."
    )
}

/// What Claude is told when a plan is refused without a word, in `lang`: the window's own
/// « Continuer à planifier ».
fn keep_planning(lang: Lang) -> String {
    tr_in!(
        lang,
        "Continue à planifier : le plan ne me convient pas encore.",
        "Keep planning: the plan doesn’t work for me yet."
    )
}

/// `message` has no text to speak of.
fn blank(message: &Option<String>) -> bool {
    message.as_deref().is_none_or(|m| m.trim().is_empty())
}

/// What Claude is told of an action refused through the server by `by` (the name its activity
/// log gives the client), in `lang`: who refused, and the reason when there is one. Never that the
/// user did.
fn refused_by(lang: Lang, by: &str, message: Option<&str>) -> String {
    let by = by.split_whitespace().collect::<Vec<_>>().join(" ");
    match message.map(str::trim).filter(|m| !m.is_empty()) {
        Some(message) => tr_in!(
            lang,
            "Refusé par {by} via Escouade : {message}",
            "Refused by {by} through Escouade: {message}"
        ),
        None => tr_in!(
            lang,
            "Refusé par {by} via Escouade.",
            "Refused by {by} through Escouade."
        ),
    }
}

/// The answers of a question (`{ <question>: <answer> }`) as Claude is given those that come
/// through the server, in `lang`: each text followed by who gave it, « SQLite (réponse de <by> via
/// Escouade) ». The author is one line (an agent's name, « Claude (hors Escouade) »).
fn signed_answers(lang: Lang, by: &str, answers: &Value) -> Value {
    let by = by.split_whitespace().collect::<Vec<_>>().join(" ");
    let Some(map) = answers.as_object() else {
        return answers.clone();
    };
    map.iter()
        .map(|(question, answer)| {
            let signed = match answer.as_str() {
                Some(answer) => Value::String(tr_in!(
                    lang,
                    "{answer} (réponse de {by} via Escouade)",
                    "{answer} (answer from {by} through Escouade)"
                )),
                None => answer.clone(),
            };
            (question.clone(), signed)
        })
        .collect::<serde_json::Map<_, _>>()
        .into()
}

/// The patch of an answered request, saying who answered when it was not the user in the window.
fn answered_by(patch: &mut Value, by: Option<&str>) {
    if let Some(by) = by {
        patch["by"] = json!(by);
    }
}

/// Refused: the request has its answer already, from the user or from a client.
fn already_answered() -> String {
    tr!(
        "Cette demande a déjà reçu une réponse.",
        "This request has already been answered."
    )
}

/// Refused: the agent's process is gone, nothing can be answered.
fn not_running(speaker: &str) -> anyhow::Error {
    anyhow!(tr!(
        "{speaker} ne tourne plus pour cet agent",
        "{speaker} is no longer running for this agent"
    ))
}

/// What the agent's conversation is told when its harness (its agents named `speaker`) went before
/// it started: the session to resume is gone, the point of the original's session a copy starts
/// from is gone.
fn exit_notices(lang: Lang, speaker: &str) -> [String; 2] {
    [
        tr_in!(
            lang,
            "Session {speaker} introuvable : une nouvelle session sera démarrée au prochain message.",
            "{speaker} session not found: a new session will start with your next message."
        ),
        tr_in!(
            lang,
            "Point de copie introuvable dans la session de l'original : {speaker} reprend toute sa conversation, jusqu'à maintenant.",
            "Copy point not found in the original’s session: {speaker} picks up its whole conversation, up to now."
        ),
    ]
}

/// Why the agent's turn ended in an error, when its harness `name` stopped with `code`.
fn stopped(lang: Lang, name: &str, code: &str) -> String {
    tr_in!(
        lang,
        "{name} s'est arrêté (code {code})",
        "{name} stopped (code {code})"
    )
}

/// What the agent does while it streams a block of text or of thinking; None for a tool's.
fn block_activity(lang: Lang, kind: &str) -> Option<String> {
    match kind {
        "text" => Some(tr_in!(lang, "Rédige", "Replying")),
        "thinking" => Some(tr_in!(lang, "Réfléchit", "Thinking")),
        _ => None,
    }
}

/// What the agent does, from the tool it runs: "Lit src/db.ts", "Lance npm test"… None for a tool
/// without words of its own.
pub fn tool_activity(name: &str, input: &Value, cwd: &str) -> Option<String> {
    tool_activity_in(i18n::ui(), name, input, cwd)
}

/// A command that begins by changing folder (`cd "C:/p/worktree" && npm test`: how an agent works
/// in its worktree, on every command) is told by what it runs after: the rest. A `cd` alone, or
/// with nothing after it, stays as it is.
fn after_cd(command: &str) -> &str {
    let mut rest = command.trim_start();
    while let Some(arg) = rest.strip_prefix("cd ") {
        let arg = arg.trim_start();
        let len = match arg.chars().next() {
            Some(quote @ ('"' | '\'')) => arg[1..].find(quote).map(|i| i + 2),
            Some(_) => Some(arg.find(char::is_whitespace).unwrap_or(arg.len())),
            None => None,
        };
        let after = len.map(|len| arg[len..].trim_start());
        let next = after
            .and_then(|a| a.strip_prefix("&&").or_else(|| a.strip_prefix(';')))
            .map(str::trim_start)
            .filter(|n| !n.is_empty());
        match next {
            Some(next) => rest = next,
            None => break,
        }
    }
    rest
}

/// The `s` of a plural, in French and in English alike.
fn plural(n: usize) -> &'static str {
    if n > 1 {
        "s"
    } else {
        ""
    }
}

/// `tool_activity` in `lang`. A verb alone is the start of the same verb followed by what it acts
/// on ("Lit", "Lit src/a.ts"; “Reading”, “Reading src/a.ts”): `set_activity` relies on it.
fn tool_activity_in(lang: Lang, name: &str, input: &Value, cwd: &str) -> Option<String> {
    let file = || {
        input["file_path"]
            .as_str()
            .or(input["notebook_path"].as_str())
            .map(|p| relative_slash(cwd, p))
            .unwrap_or_default()
    };
    let with = |verb: &str, what: String| {
        if what.is_empty() {
            verb.to_string()
        } else {
            format!("{verb} {what}")
        }
    };
    // The first line that says something, cut short.
    let gist = |text: &Value| {
        let first = text
            .as_str()
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .unwrap_or_default();
        truncate(first, 60)
    };
    Some(match name {
        "Read" => with(&tr_in!(lang, "Lit", "Reading"), file()),
        "Grep" | "Glob" => with(
            &tr_in!(lang, "Cherche", "Searching"),
            gist(&input["pattern"]),
        ),
        "Edit" | "MultiEdit" | "NotebookEdit" => with(&tr_in!(lang, "Modifie", "Editing"), file()),
        "Write" => with(&tr_in!(lang, "Écrit", "Writing"), file()),
        "Bash" => with(
            &tr_in!(lang, "Lance", "Running"),
            gist(&Value::String(
                after_cd(input["command"].as_str().unwrap_or_default()).to_string(),
            )),
        ),
        "Task" | "Agent" => tr_in!(lang, "Délègue", "Delegating"),
        crate::sentfiles::TOOL => {
            let (images, files) = crate::sentfiles::count_images(input);
            match (images, files) {
                (0, 0) => return None,
                (n, 0) => tr_in!(
                    lang,
                    "A envoyé {n} image{s}",
                    "Sent {n} image{s}",
                    s = plural(n)
                ),
                (0, n) => tr_in!(
                    lang,
                    "A envoyé {n} fichier{s}",
                    "Sent {n} file{s}",
                    s = plural(n)
                ),
                (i, n) => tr_in!(
                    lang,
                    "A envoyé {i} image{si} et {n} fichier{sn}",
                    "Sent {i} image{si} and {n} file{sn}",
                    si = plural(i),
                    sn = plural(n)
                ),
            }
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt() -> AgentRt {
        AgentRt::new(
            AgentMeta {
                id: new_id(),
                cwd: "C:/p".into(),
                ..Default::default()
            },
            &std::env::temp_dir().join(format!("ccm-agent-tests-{}", std::process::id())),
        )
    }

    #[test]
    fn haiku_is_asked_to_name_an_agent_only_once_while_the_app_runs() {
        let mut a = rt();
        // A command with nothing to name it by does not use the one question.
        assert_eq!(a.name_from("/clear"), None);
        assert!(!a.naming_asked);
        assert_eq!(
            a.name_from("Create the hello file"),
            Some("Create the hello file".into())
        );
        assert!(a.naming_asked);
        // Not asked again, whatever the next message says (even a skill with arguments).
        assert_eq!(a.name_from("Delete the other file"), None);
        assert_eq!(a.name_from("/impeccable propose un design"), None);
        // An agent that has a name is never asked for one.
        let mut named = rt();
        named.meta.named = true;
        assert_eq!(named.name_from("Create the hello file"), None);
        assert!(!named.naming_asked);
    }

    #[test]
    fn a_message_the_app_typed_by_itself_is_recognized_once_in_the_transcript() {
        let mut a = rt();
        assert!(!a.took_unprompted("continue"));
        a.unprompted = Some("continue".into());
        // Another message first (the user typed meanwhile): the app's is still waited for.
        assert!(!a.took_unprompted("Create the hello file"));
        assert!(a.took_unprompted(" continue\n"));
        assert!(!a.took_unprompted("continue"));
    }

    #[test]
    fn streaming_text_then_final_message() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.handle_frame(&json!({"type":"stream_event","event":{"type":"message_start","message":{"id":"m1","usage":{}}},"parent_tool_use_id":null}), &mut fx);
        a.handle_frame(&json!({"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}},"parent_tool_use_id":null}), &mut fx);
        a.handle_frame(&json!({"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Bon"}},"parent_tool_use_id":null}), &mut fx);
        a.handle_frame(&json!({"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"jour"}},"parent_tool_use_id":null}), &mut fx);
        assert_eq!(a.conv.get("m1:0").unwrap()["text"], "Bonjour");
        a.handle_frame(&json!({"type":"assistant","message":{"id":"m1","content":[{"type":"text","text":"Bonjour !"}]},"parent_tool_use_id":null}), &mut fx);
        let item = a.conv.get("m1:0").unwrap();
        assert_eq!(item["text"], "Bonjour !");
        assert_eq!(item["streaming"], false);
        assert_eq!(a.meta.status, AgentStatus::Running);
    }

    #[test]
    fn question_then_result_usage_deltas() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.handle_frame(&json!({"type":"control_request","request_id":"r1","request":{"subtype":"can_use_tool","tool_name":"AskUserQuestion","tool_use_id":"tu1","input":{"questions":[{"question":"Q?","options":[]}]}}}), &mut fx);
        assert_eq!(a.meta.status, AgentStatus::Waiting);
        assert_eq!(fx.notify, Some(AgentAlert::new(NotifyKind::Question, "Q?")));
        assert!(a.has_pending("r1"));

        let usage = |inp: u64, out: u64, cost: f64| {
            json!({"type":"result","subtype":"success","is_error":false,"duration_ms":10,
            "modelUsage":{"claude-sonnet-5":{"inputTokens":inp,"outputTokens":out,"cacheReadInputTokens":100,"cacheCreationInputTokens":0,"costUSD":cost}}})
        };
        let mut fx = Effects::default();
        a.handle_frame(&usage(10, 20, 0.5), &mut fx);
        assert_eq!(fx.turns.len(), 1);
        assert_eq!(fx.turns[0].input, 10);
        assert_eq!(fx.turns[0].cache, 100);
        assert!(!a.has_pending("r1"));
        assert_eq!(a.meta.status, AgentStatus::Done);

        let mut fx = Effects::default();
        a.handle_frame(&usage(15, 50, 0.8), &mut fx);
        assert_eq!(fx.turns[0].input, 5);
        assert_eq!(fx.turns[0].output, 30);
        assert_eq!(fx.turns[0].cache, 0);
        assert!((fx.turns[0].cost - 0.3).abs() < 1e-9);
        assert!((a.meta.cost - 0.8).abs() < 1e-9);
    }

    fn can_use_tool(a: &mut AgentRt, tool: &str, input: Value) -> Effects {
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"control_request","request_id":"r1","request":{
                "subtype":"can_use_tool","tool_name":tool,"tool_use_id":"tu1","input":input}}),
            &mut fx,
        );
        fx
    }

    fn says(a: &mut AgentRt, id: &str, text: &str) {
        a.handle_frame(
            &json!({"type":"assistant","message":{"id":id,"content":[{"type":"text","text":text}]},"parent_tool_use_id":null}),
            &mut Effects::default(),
        );
    }

    fn runs_tool(a: &mut AgentRt, id: &str) {
        a.handle_frame(
            &json!({"type":"assistant","message":{"id":id,"content":[{"type":"tool_use","id":format!("t-{id}"),"name":"Bash","input":{"command":"npm test"}}]},"parent_tool_use_id":null}),
            &mut Effects::default(),
        );
    }

    /// The turn's result frame, with Claude Code's own `result` text when given.
    fn ends(a: &mut AgentRt, result: Option<&str>) -> Effects {
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1,
                "result":result,"modelUsage":{}}),
            &mut fx,
        );
        fx
    }

    #[test]
    fn a_permission_notifies_with_the_tool_and_what_it_acts_on() {
        let mut a = rt();
        let fx = can_use_tool(&mut a, "Bash", json!({"command":"rm -rf build"}));
        assert_eq!(
            fx.notify,
            Some(AgentAlert::new(
                NotifyKind::Question,
                "Autoriser Bash : rm -rf build ?"
            ))
        );
        // A file is told from the agent's folder.
        let fx = can_use_tool(&mut a, "Edit", json!({"file_path":"C:/p/src/app.ts"}));
        assert_eq!(
            fx.notify.map(|n| n.body),
            Some("Autoriser Edit : src/app.ts ?".to_string())
        );
        let fx = can_use_tool(&mut a, "WebFetch", json!({"url":"https://example.com/a"}));
        assert_eq!(
            fx.notify.map(|n| n.body),
            Some("Autoriser WebFetch : https://example.com/a ?".to_string())
        );
    }

    /// Claude Code asks `rid`: a permission, or a question for AskUserQuestion (`request` without its subtype).
    fn asks(a: &mut AgentRt, rid: &str, mut request: Value) {
        request["subtype"] = json!("can_use_tool");
        a.handle_frame(
            &json!({"type":"control_request","request_id":rid,"request":request}),
            &mut Effects::default(),
        );
    }

    #[test]
    fn the_view_lists_the_pending_requests_in_the_order_they_came() {
        let mut a = rt();
        // Enough of them for any other order to show.
        let ids: Vec<String> = (0..12).map(|i| format!("r{i}")).collect();
        for id in &ids {
            asks(
                &mut a,
                id,
                json!({"tool_name":"Bash","tool_use_id":format!("t-{id}"),"input":{"command":"npm test"}}),
            );
        }
        let v = a.view();
        assert_eq!(v.pending, ids);
        let shown: Vec<String> = v.requests.iter().map(|r| r.id.clone()).collect();
        assert_eq!(shown, ids);
        // One cancelled, the others keep their order.
        a.handle_frame(
            &json!({"type":"control_cancel_request","request_id":"r0"}),
            &mut Effects::default(),
        );
        assert_eq!(a.view().pending, ids[1..]);
    }

    #[test]
    fn the_waiting_requests_say_when_they_came_and_keep_it_when_asked_again() {
        let mut a = rt();
        asks(
            &mut a,
            "r1",
            json!({"tool_name":"Bash","tool_use_id":"t1","input":{"command":"npm test"}}),
        );
        asks(
            &mut a,
            "r2",
            json!({"tool_name":"AskUserQuestion","tool_use_id":"t2","input":{"questions":[
                {"question":"Quoi ?","multiSelect":true,"options":[{"label":"A"},{"label":"B"}]}]}}),
        );
        let waiting = a.waiting();
        let ids: Vec<&str> = waiting.iter().map(|w| w.view.id.as_str()).collect();
        assert_eq!(ids, ["r1", "r2"]);
        assert!(waiting[0].since > 0 && waiting[0].since <= waiting[1].since);
        // The request as Claude Code sent it: what the view leaves out (a question's choice of
        // several options) is there.
        assert_eq!(waiting[0].input["command"], "npm test");
        assert_eq!(waiting[1].input["questions"][0]["multiSelect"], true);
        // Asked again, it keeps its place and the time it first came.
        let first = waiting[0].since;
        std::thread::sleep(std::time::Duration::from_millis(5));
        asks(
            &mut a,
            "r1",
            json!({"tool_name":"Bash","tool_use_id":"t1","input":{"command":"npm test -- --all"}}),
        );
        let waiting = a.waiting();
        assert_eq!(waiting[0].view.id, "r1");
        assert_eq!(waiting[0].since, first);
        assert_eq!(waiting[0].input["command"], "npm test -- --all");
    }

    #[test]
    fn an_answer_given_through_the_server_tells_claude_who_gave_it() {
        let answers = json!({ "Base ?": "SQLite", "Outils ?": "ESLint, Vitest", "Autre ?": 3 });
        assert_eq!(
            signed_answers(Lang::Fr, "agent-2", &answers),
            json!({
                "Base ?": "SQLite (réponse de agent-2 via Escouade)",
                "Outils ?": "ESLint, Vitest (réponse de agent-2 via Escouade)",
                // Only a text is signed.
                "Autre ?": 3,
            })
        );
        assert_eq!(
            signed_answers(
                Lang::En,
                "Claude (outside Escouade)",
                &json!({ "Base?": "SQLite" })
            ),
            json!({ "Base?": "SQLite (answer from Claude (outside Escouade) through Escouade)" })
        );
        // The author is one line, whatever its name holds.
        assert_eq!(
            signed_answers(Lang::En, "agent\n2  b", &json!({ "Q?": "A" })),
            json!({ "Q?": "A (answer from agent 2 b through Escouade)" })
        );
    }

    #[test]
    fn what_claude_is_told_of_a_refusal_through_the_server_names_who_refused() {
        assert_eq!(
            refused_by(Lang::Fr, "agent-2", Some("utilise npm run clean")),
            "Refusé par agent-2 via Escouade : utilise npm run clean"
        );
        assert_eq!(
            refused_by(
                Lang::En,
                "Claude (outside Escouade)",
                Some("use npm run clean")
            ),
            "Refused by Claude (outside Escouade) through Escouade: use npm run clean"
        );
        // Without a word of its own, or with a blank one, the refusal alone.
        assert_eq!(
            refused_by(Lang::Fr, "agent-2", None),
            "Refusé par agent-2 via Escouade."
        );
        assert_eq!(
            refused_by(Lang::En, "agent-2", Some("  \n")),
            "Refused by agent-2 through Escouade."
        );
        // The author is one line, whatever its name holds.
        assert_eq!(
            refused_by(Lang::En, "agent\n2  b", None),
            "Refused by agent 2 b through Escouade."
        );
    }

    #[test]
    fn the_view_sums_up_each_pending_request_without_the_conversation() {
        let mut a = rt();
        asks(
            &mut a,
            "r1",
            json!({"tool_name":"Bash","tool_use_id":"t1","input":{"command":"npm ci\nnpm test"},
                "description":"Installe et teste","decision_reason":"Commande hors de la liste"}),
        );
        asks(
            &mut a,
            "r2",
            json!({"tool_name":"Edit","tool_use_id":"t2",
                "input":{"file_path":"C:/p/src/db.ts","old_string":"a","new_string":"b"}}),
        );
        asks(
            &mut a,
            "r3",
            json!({"tool_name":"AskUserQuestion","tool_use_id":"t3","input":{"questions":[
                {"question":"Quelle base ?","options":[{"label":"SQLite"},{"label":"Postgres","description":"un serveur"}]}]}}),
        );
        let v = a.view();
        assert_eq!(
            v.requests[0],
            PendingView {
                id: "r1".into(),
                kind: "permission".into(),
                tool: "Bash".into(),
                // Its lines kept: the overview shows them as they run.
                arg: "npm ci\nnpm test".into(),
                description: Some("Installe et teste".into()),
                reason: Some("Commande hors de la liste".into()),
                questions: vec![],
                cut: false,
                default_no: false,
                can_always: false,
                always_scope: None,
            }
        );
        // A file from the agent's folder, as the conversation's card shows it.
        assert_eq!(v.requests[1].arg, "src/db.ts");
        assert_eq!(v.requests[2].kind, "question");
        assert_eq!(
            v.requests[2].questions,
            vec![PendingQuestion {
                question: "Quelle base ?".into(),
                options: vec!["SQLite".into(), "Postgres".into()],
                multi: false,
            }]
        );
        let sent = serde_json::to_value(&v).unwrap();
        assert_eq!(sent["requests"][0]["arg"], "npm ci\nnpm test");
        assert_eq!(
            sent["requests"][2]["questions"][0]["options"][1],
            "Postgres"
        );
    }

    #[test]
    fn the_view_names_what_a_tool_of_escouade_acts_on_its_title_ticket_or_agent() {
        use crate::i18n::Lang::Fr;
        let arg = |tool: &str, input: Value| {
            tool_arg_in(Fr, &format!("mcp__escouade__{tool}"), &input, "C:/p")
        };
        // Its main argument, whatever comes first in what Claude sends.
        assert_eq!(
            arg(
                "create_ticket",
                json!({"project":"demo","title":"Corriger la connexion","description":"Le jeton expire"})
            ),
            "Corriger la connexion"
        );
        assert_eq!(
            arg(
                "update_ticket",
                json!({"description":"Plus court","ticket":"DEM-4"})
            ),
            "DEM-4"
        );
        assert_eq!(arg("start_ticket", json!({"ticket":"DEM-3"})), "DEM-3");
        assert_eq!(
            arg(
                "send_message",
                json!({"text":"Relis le test","agent":"fix-login"})
            ),
            "fix-login"
        );
        assert_eq!(
            arg(
                "get_agent_summary",
                json!({"project":"demo","agent":"fix-login"})
            ),
            "fix-login"
        );
        // Without one: what the agent is told, its line, its project.
        assert_eq!(
            arg(
                "create_agent",
                json!({"project":"demo","message":"Écris la doc","worktree":true})
            ),
            "Écris la doc"
        );
        assert_eq!(
            arg("report_progress", json!({"line":"Tests verts"})),
            "Tests verts"
        );
        assert_eq!(
            arg("list_tickets", json!({"project":"demo","column":"todo"})),
            "demo"
        );
        assert_eq!(arg("list_projects", json!({})), "");
        // Another server's tool: its first text, as before.
        assert_eq!(
            tool_arg_in(
                Fr,
                "mcp__github__create_issue",
                &json!({"title":"Bug"}),
                "C:/p"
            ),
            "Bug"
        );
        // And the overview is told it.
        let mut a = rt();
        asks(
            &mut a,
            "r1",
            json!({"tool_name":"mcp__escouade__create_ticket","tool_use_id":"t1",
                "input":{"project":"demo","title":"Corriger la connexion"}}),
        );
        assert_eq!(a.view().requests[0].arg, "Corriger la connexion");
    }

    #[test]
    fn the_view_tells_what_always_allows_and_a_question_of_several_choices() {
        let mut a = rt();
        asks(
            &mut a,
            "r1",
            json!({"tool_name":"Bash","tool_use_id":"t1","input":{"command":"npm test"},
                "permission_suggestions":[{"type":"addRules","rules":[{"toolName":"Bash","ruleContent":"npm test"}]}]}),
        );
        asks(
            &mut a,
            "r2",
            json!({"tool_name":"AskUserQuestion","tool_use_id":"t2","input":{"questions":[
                {"question":"Quels tests ?","multiSelect":true,"options":[{"label":"auth"},{"label":"e2e"}]}]}}),
        );
        let v = a.view();
        // Claude Code keeps rules: « Toujours autoriser », with no scope of a session.
        assert!(v.requests[0].can_always);
        assert_eq!(v.requests[0].always_scope, None);
        assert!(v.requests[1].questions[0].multi);
        let sent = serde_json::to_value(&v).unwrap();
        assert_eq!(sent["requests"][0]["canAlways"], true);
        assert_eq!(sent["requests"][1]["questions"][0]["multi"], true);
    }

    #[test]
    fn the_view_tells_a_request_claude_code_would_refuse_by_default() {
        let mut a = rt();
        asks(
            &mut a,
            "r1",
            json!({"tool_name":"Bash","tool_use_id":"t1","input":{"command":"rm -rf /"},"default_to_no":true}),
        );
        asks(
            &mut a,
            "r2",
            json!({"tool_name":"Bash","tool_use_id":"t2","input":{"command":"ls"}}),
        );
        let v = a.view();
        assert!(v.requests[0].default_no);
        assert!(!v.requests[1].default_no);
        assert_eq!(
            serde_json::to_value(&v).unwrap()["requests"][0]["defaultNo"],
            true
        );
    }

    #[test]
    fn the_view_caps_what_a_pending_request_says_and_tells_it_was_cut() {
        let mut a = rt();
        let long = "é".repeat(MAX_PENDING_FIELD + 10);
        asks(
            &mut a,
            "r1",
            json!({"tool_name":"Bash","tool_use_id":"t1","input":{"command":long}}),
        );
        asks(
            &mut a,
            "r2",
            json!({"tool_name":"AskUserQuestion","tool_use_id":"t2","input":{"questions":[{"question":long,"options":[]}]}}),
        );
        asks(
            &mut a,
            "r3",
            json!({"tool_name":"Bash","tool_use_id":"t3","input":{"command":"ls"},"description":long}),
        );
        let v = a.view();
        assert_eq!(v.requests[0].arg.chars().count(), MAX_PENDING_FIELD);
        assert!(v.requests[0].cut);
        assert_eq!(
            v.requests[1].questions[0].question.chars().count(),
            MAX_PENDING_FIELD
        );
        assert!(v.requests[1].cut);
        assert!(v.requests[2].cut);
        assert_eq!(v.requests[2].arg, "ls");
    }

    #[test]
    fn a_plan_to_approve_notifies_in_the_apps_words() {
        let mut a = rt();
        let fx = can_use_tool(
            &mut a,
            "ExitPlanMode",
            json!({"plan":"# Découper le module\n\n1. Extraire le parseur"}),
        );
        assert_eq!(
            fx.notify,
            Some(AgentAlert::new(
                NotifyKind::Question,
                "Approuver le plan : Découper le module ?"
            ))
        );
    }

    #[test]
    fn the_end_of_a_turn_notifies_with_the_first_line_of_the_final_reply() {
        let mut a = rt();
        a.push_user("u1", "Corrige", 0, &[], &mut Effects::default());
        // What was said on the way is no final reply: only what follows the last tool is.
        says(&mut a, "m1", "Je regarde le filtre.");
        runs_tool(&mut a, "m2");
        says(&mut a, "m3", "## Bilan\n\nLe **filtre** accepte les PDF.");
        let fx = ends(&mut a, Some("## Bilan\n\nLe filtre accepte les PDF."));
        assert_eq!(fx.notify, Some(AgentAlert::new(NotifyKind::Done, "Bilan")));
    }

    #[test]
    fn a_turn_that_ends_on_a_tool_tells_claude_codes_own_result_else_it_is_just_done() {
        let mut a = rt();
        a.push_user("u1", "Lance", 0, &[], &mut Effects::default());
        says(&mut a, "m1", "Je lance les tests.");
        runs_tool(&mut a, "m2");
        let fx = ends(&mut a, Some("Les tests passent.\nDétail"));
        assert_eq!(
            fx.notify,
            Some(AgentAlert::new(NotifyKind::Done, "Les tests passent."))
        );
        // Another turn, wordless: nothing of the previous one is told again.
        a.push_user("u2", "Encore", 0, &[], &mut Effects::default());
        let fx = ends(&mut a, None);
        assert_eq!(
            fx.notify,
            Some(AgentAlert::new(NotifyKind::Done, "Tâche terminée"))
        );
    }

    #[test]
    fn a_failed_turn_notifies_with_its_reason() {
        let mut a = rt();
        a.push_user("u1", "Lance", 0, &[], &mut Effects::default());
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":true,"duration_ms":1,
                "result":"API Error: 500\nrequest id 42","modelUsage":{}}),
            &mut fx,
        );
        assert_eq!(
            fx.notify,
            Some(AgentAlert::new(
                NotifyKind::Error,
                "Erreur : API Error: 500"
            ))
        );
        // Without words of its own, the kind of result it is.
        a.push_user("u2", "Encore", 0, &[], &mut Effects::default());
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"result","subtype":"error_during_execution","is_error":true,
                "duration_ms":1,"modelUsage":{}}),
            &mut fx,
        );
        assert_eq!(
            fx.notify.map(|n| n.body),
            Some("Erreur : error_during_execution".to_string())
        );
    }

    #[test]
    fn a_process_that_dies_mid_turn_notifies_with_its_exit_code() {
        let mut a = rt();
        a.push_user("u1", "Lance", 0, &[], &mut Effects::default());
        let mut fx = Effects::default();
        a.on_exit(a.gen, Some(3), "", &mut fx);
        assert_eq!(
            fx.notify,
            Some(AgentAlert::new(
                NotifyKind::Error,
                "Erreur : Claude Code s'est arrêté (code 3)"
            ))
        );
    }

    #[test]
    fn tool_result_counts_patch_and_touches_file() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.handle_frame(&json!({"type":"assistant","message":{"id":"m2","content":[{"type":"tool_use","id":"tu9","name":"Edit","input":{"file_path":"C:/p/src/a.ts"}}]},"parent_tool_use_id":null}), &mut fx);
        a.handle_frame(&json!({"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"tu9","content":"ok"}]},
            "tool_use_result":{"filePath":"C:/p/src/a.ts","structuredPatch":[{"oldStart":1,"newStart":1,"lines":[" a","-b","+c","+d"]}]}}), &mut fx);
        let item = a.conv.get("tu9").unwrap();
        assert_eq!(item["status"], "ok");
        assert_eq!(item["result"]["add"], 2);
        assert_eq!(item["result"]["del"], 1);
        assert_eq!(a.meta.touched_files, vec!["src/a.ts".to_string()]);
    }

    #[test]
    fn follows_the_permission_mode_reported_by_claude() {
        let mut a = rt();
        a.meta.mode = "plan".into();
        let mut fx = Effects::default();
        // After an approved plan Claude leaves plan mode for the default (ask) mode.
        a.handle_frame(
            &json!({"type":"system","subtype":"status","status":null,"permissionMode":"default"}),
            &mut fx,
        );
        assert_eq!(a.meta.mode, "default");
        assert!(fx.agent_changed);
    }

    #[test]
    fn usage_counters_that_go_backwards_never_underflow() {
        let mut a = rt();
        let result = |input: u64, output: u64, cost: f64| {
            json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1,
                "modelUsage":{"m":{"inputTokens":input,"outputTokens":output,"cacheReadInputTokens":0,"cacheCreationInputTokens":0,"costUSD":cost}}})
        };
        a.handle_frame(&result(100, 50, 1.0), &mut Effects::default());
        let mut fx = Effects::default();
        a.handle_frame(&result(90, 60, 1.2), &mut fx);
        assert_eq!(fx.turns[0].input, 0);
        assert_eq!(fx.turns[0].output, 10);
    }

    fn message_start(
        a: &mut AgentRt,
        id: &str,
        parent: Option<&str>,
        usage: Value,
        fx: &mut Effects,
    ) {
        a.handle_frame(
            &json!({"type":"stream_event","parent_tool_use_id":parent,
                "event":{"type":"message_start","message":{"id":id,"model":"claude-haiku-4-5-20251001","usage":usage}}}),
            fx,
        );
    }

    fn message_delta(a: &mut AgentRt, parent: Option<&str>, output: u64, fx: &mut Effects) {
        a.handle_frame(
            &json!({"type":"stream_event","parent_tool_use_id":parent,
                "event":{"type":"message_delta","usage":{"output_tokens":output}}}),
            fx,
        );
    }

    #[test]
    fn tokens_and_an_estimated_cost_add_up_while_the_turn_runs() {
        let mut a = rt();
        let mut fx = Effects::default();
        let input = json!({"input_tokens":10,"cache_read_input_tokens":17513,"cache_creation_input_tokens":10045,
            "cache_creation":{"ephemeral_5m_input_tokens":0,"ephemeral_1h_input_tokens":10045},"output_tokens":1});
        message_start(&mut a, "m1", None, input, &mut fx);
        assert_eq!(a.view().live_tokens, 10 + 17513 + 10045 + 1);
        let mut fx = Effects::default();
        message_delta(&mut a, None, 257, &mut fx);
        assert!(fx.agent_changed);
        assert_eq!(a.view().live_tokens, 10 + 17513 + 10045 + 257);
        // A second API call of the same turn, and one made by a subagent.
        let more = json!({"input_tokens":8,"cache_read_input_tokens":27558,"cache_creation_input_tokens":489,
            "cache_creation":{"ephemeral_5m_input_tokens":0,"ephemeral_1h_input_tokens":489},"output_tokens":1});
        message_start(&mut a, "m2", None, more, &mut fx);
        message_delta(&mut a, None, 65, &mut fx);
        message_start(
            &mut a,
            "s1",
            Some("tu1"),
            json!({"input_tokens":100,"output_tokens":1}),
            &mut fx,
        );
        message_delta(&mut a, Some("tu1"), 20, &mut fx);
        let v = a.view();
        assert_eq!(v.live_tokens, 18 + 322 + 45071 + 10534 + 120);
        // Measured with the real CLI for the first two messages, plus 100 input and 20 output tokens.
        assert!(
            (v.live_cost - (0.0272031 + 100e-6 + 20.0 * 5e-6)).abs() < 1e-9,
            "{}",
            v.live_cost
        );
        assert_eq!(a.meta.tokens, 0);
    }

    #[test]
    fn the_exact_cost_replaces_the_estimate_when_the_turn_ends() {
        let mut a = rt();
        let mut fx = Effects::default();
        message_start(
            &mut a,
            "m1",
            None,
            json!({"input_tokens":1000,"output_tokens":1}),
            &mut fx,
        );
        message_delta(&mut a, None, 50, &mut fx);
        assert!(a.view().live_cost > 0.0);
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1,
                "modelUsage":{"claude-haiku-4-5-20251001":{"inputTokens":1000,"outputTokens":50,"cacheReadInputTokens":0,"cacheCreationInputTokens":0,"costUSD":0.00125}}}),
            &mut fx,
        );
        let v = a.view();
        assert_eq!((v.live_tokens, v.live_cost), (0, 0.0));
        assert_eq!(a.meta.tokens, 1050);
        assert!((a.meta.cost - 0.00125).abs() < 1e-12);
    }

    #[test]
    fn a_message_sent_from_claude_ai_shows_in_the_conversation() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"user","message":{"role":"user","content":"Ajoute les tests"},"parent_tool_use_id":null,
                "uuid":"r1","isReplay":true,"origin":{"kind":"human"}}),
            &mut fx,
        );
        let item = a.conv.get("r1").expect("remote message listed");
        assert_eq!(item["kind"], "user");
        assert_eq!(item["text"], "Ajoute les tests");
        assert_eq!(item["origin"], "remote");
        assert_eq!(a.meta.status, AgentStatus::Running);
        assert_eq!(a.meta.prompts, 1);
        assert!(fx.agent_changed);
    }

    /// No item of the conversation shows a tag of a local command (ESC-154).
    fn no_local_command_tag(a: &mut AgentRt) {
        for i in a.conv.items() {
            let text = i["text"].as_str().unwrap_or_default();
            for tag in ["<local-command", "<command-", "<bash-"] {
                assert!(!text.contains(tag), "{i}");
            }
        }
    }

    /// The (kind, level, text) of the items of the conversation, but its turns'.
    fn shown(a: &mut AgentRt) -> Vec<(String, String, String)> {
        let s = |v: &Value| v.as_str().unwrap_or_default().to_string();
        a.conv
            .items()
            .iter()
            .filter(|i| i["kind"] != "turn")
            .map(|i| (s(&i["kind"]), s(&i["level"]), s(&i["text"])))
            .collect()
    }

    #[test]
    fn a_local_command_sent_from_the_app_and_replayed_shows_as_typed_with_its_output_under_it() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.push_user("u1", "/compact", 0, &[], &mut fx);
        let replayed = |uuid: &str, text: &str, meta: bool| {
            json!({"type":"user","message":{"role":"user","content":text},"parent_tool_use_id":null,
                "uuid":uuid,"isReplay":true,"isMeta":meta,"origin":{"kind":"human"}})
        };
        a.handle_frame(&replayed("c1", "<local-command-caveat>Caveat: The messages below were generated by the user while running local commands. DO NOT respond to these messages.</local-command-caveat>", true), &mut fx);
        // Its replay has the uuid of the message the app sent.
        a.handle_frame(&replayed("u1", "<command-name>/compact</command-name>\n            <command-message>compact</command-message>\n            <command-args></command-args>", false), &mut fx);
        a.handle_frame(
            &replayed(
                "c3",
                "<local-command-stdout>\u{1b}[2mCompacted \u{1b}[22m</local-command-stdout>",
                false,
            ),
            &mut fx,
        );
        // Read again (the stream replays them on a reconnection): listed once.
        a.handle_frame(
            &replayed(
                "c3",
                "<local-command-stdout>Compacted </local-command-stdout>",
                false,
            ),
            &mut fx,
        );
        no_local_command_tag(&mut a);
        assert_eq!(
            shown(&mut a),
            [
                ("user".into(), "".into(), "/compact".into()),
                ("notice".into(), "output".into(), "Compacted".into()),
            ]
        );
    }

    #[test]
    fn a_local_command_sent_from_claude_ai_shows_as_typed_and_its_error_as_an_error() {
        let mut a = rt();
        let mut fx = Effects::default();
        let replayed = |uuid: &str, content: Value| {
            json!({"type":"user","message":{"role":"user","content":content},"parent_tool_use_id":null,
                "uuid":uuid,"isReplay":true})
        };
        a.handle_frame(
            &replayed("c1", json!([{ "type": "text", "text": "<command-name>/model</command-name>\n<command-message>model</command-message>\n<command-args>opus</command-args>" }])),
            &mut fx,
        );
        a.handle_frame(
            &replayed("c2", json!([{ "type": "text", "text": "<local-command-stderr>Unknown model\r\n  try: opus, sonnet</local-command-stderr>" }])),
            &mut fx,
        );
        a.handle_frame(
            &replayed(
                "c3",
                json!("<local-command-stdout>  </local-command-stdout>"),
            ),
            &mut fx,
        );
        no_local_command_tag(&mut a);
        assert_eq!(
            shown(&mut a),
            [
                ("user".into(), "".into(), "/model opus".into()),
                (
                    "notice".into(),
                    "error".into(),
                    "Unknown model\n  try: opus, sonnet".into()
                ),
            ]
        );
        assert_eq!(a.conv.get("c1").unwrap()["origin"], "remote");
    }

    #[test]
    fn a_local_commands_output_in_the_stream_shows_as_an_output_whatever_its_form() {
        let mut a = rt();
        let mut fx = Effects::default();
        let user = |content: Value| json!({"type":"user","message":{"role":"user","content":content},"parent_tool_use_id":null,"uuid":new_id()});
        a.handle_frame(
            &user(json!("<local-command-stdout>Total cost: $0.01\n\u{1b}[2mTotal duration: 3s\u{1b}[22m</local-command-stdout>")),
            &mut fx,
        );
        a.handle_frame(
            &user(json!([{ "type": "text", "text": "<local-command-stdout>Set model to opus</local-command-stdout>" }])),
            &mut fx,
        );
        a.handle_frame(
            &user(json!("<local-command-stderr>boom</local-command-stderr>")),
            &mut fx,
        );
        a.handle_frame(
            &user(json!("<local-command-caveat>Caveat</local-command-caveat>")),
            &mut fx,
        );
        a.handle_frame(
            &json!({"type":"system","subtype":"local_command_output","content":"<local-command-stdout>\u{1b}[1mMCP\u{1b}[22m\n  escouade ✔</local-command-stdout>"}),
            &mut fx,
        );
        no_local_command_tag(&mut a);
        let out = |t: &str| ("notice".to_string(), "output".to_string(), t.to_string());
        assert_eq!(
            shown(&mut a),
            [
                out("Total cost: $0.01\nTotal duration: 3s"),
                out("Set model to opus"),
                ("notice".into(), "error".into(), "boom".into()),
                out("MCP\n  escouade ✔"),
            ]
        );
    }

    #[test]
    fn a_file_sent_from_claude_ai_is_listed_with_its_message() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"user","message":{"role":"user","content":[
                {"type":"document","title":"rapport.pdf","source":{"type":"base64","media_type":"application/pdf","data":"JVBE"}},
                {"type":"document","source":{"type":"text","media_type":"text/plain","data":"x"}},
                {"type":"image","source":{"type":"base64","media_type":"image/png","data":"iVBO"}}
            ]},"parent_tool_use_id":null,"uuid":"r2","isReplay":true}),
            &mut fx,
        );
        let item = a
            .conv
            .get("r2")
            .expect("a message made only of files is listed");
        assert_eq!(item["text"], "");
        assert_eq!(item["images"], 1);
        assert_eq!(item["files"], json!(["rapport.pdf", "document"]));
    }

    #[test]
    fn the_names_of_the_attached_files_are_kept_with_the_message() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.push_user("u1", "Lis ça", 1, &["rapport.pdf".into()], &mut fx);
        let item = a.conv.get("u1").unwrap();
        assert_eq!(item["images"], 1);
        assert_eq!(item["files"], json!(["rapport.pdf"]));
        a.push_user("u2", "Et ça", 0, &[], &mut fx);
        assert!(a.conv.get("u2").unwrap().get("files").is_none());
    }

    #[test]
    fn the_echo_of_a_message_sent_from_the_app_is_not_shown_twice() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.push_user("u1", "Bonjour", 0, &[], &mut fx);
        a.handle_frame(
            &json!({"type":"user","message":{"role":"user","content":"Bonjour"},"parent_tool_use_id":null,"uuid":"u1","isReplay":true}),
            &mut fx,
        );
        let users = a.conv.ids_where(|v| v["kind"] == "user");
        assert_eq!(users, vec!["u1".to_string()]);
        assert_eq!(a.meta.prompts, 1);
    }

    #[test]
    fn what_claude_code_passes_on_by_itself_is_an_event_not_a_message_from_the_user() {
        let mut a = rt();
        let mut fx = Effects::default();
        // Queued during a turn, then passed to Claude: a background task that ended…
        let notification = "<task-notification>\n<task-id>b1</task-id>\n<status>completed</status>\n<summary>Background command \"ping\" completed (exit code 0)</summary>\n</task-notification>";
        a.handle_frame(
            &json!({"type":"user","message":{"role":"user","content":notification},"parent_tool_use_id":null,
                    "uuid":"n1","isReplay":true,"origin":{"kind":"task-notification","producer":"session-task"}}),
            &mut fx,
        );
        // …and the report of a subagent.
        let report = "<agent-message from=\"a42\">\n[Subagent hand-back] The report follows:\n  **Fini**\n</agent-message>";
        a.handle_frame(
            &json!({"type":"user","message":{"role":"user","content":[{"type":"text","text":report}]},"parent_tool_use_id":null,
                    "uuid":"n2","isReplay":true,"origin":{"kind":"peer","from":"a42","senderTaskId":"a42"}}),
            &mut fx,
        );
        let items = a.conv.items();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["kind"], "event");
        assert_eq!(items[0]["source"], "task");
        assert_eq!(items[0]["text"], notification);
        assert_eq!(items[1]["kind"], "event");
        assert_eq!(items[1]["source"], "agent");
        assert_eq!(items[1]["from"], "a42");
        assert_eq!(items[1]["text"], report);
        // Not a prompt of the user's, nor a turn started for them.
        assert_eq!(a.meta.prompts, 0);
        assert!(!a.meta.status.is_active());
        // Replayed again (resume): kept once.
        a.handle_frame(
            &json!({"type":"user","message":{"role":"user","content":notification},"uuid":"n1","isReplay":true,
                    "origin":{"kind":"task-notification"}}),
            &mut fx,
        );
        assert_eq!(a.conv.items().len(), 2);
    }

    #[test]
    fn a_turn_stopped_by_the_usage_limit_tells_when_it_resets() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"rate_limit_event","rate_limit_info":{"status":"rejected","resetsAt":1790800000,"rateLimitType":"five_hour"}}),
            &mut fx,
        );
        // Claude Code's own message, typed as a rate limit.
        a.handle_frame(
            &json!({"type":"assistant","error":"rate_limit","parent_tool_use_id":null,
                    "message":{"id":"m1","role":"assistant","content":[{"type":"text","text":"You've hit your limit · resets 3pm"}]}}),
            &mut fx,
        );
        let mut end = Effects::default();
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":true,"result":"You've hit your limit · resets 3pm"}),
            &mut end,
        );
        assert_eq!(end.limited, Some(Some(1_790_800_000_000)));
        // The next turn is a normal one.
        let mut next = Effects::default();
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false,"result":"ok"}),
            &mut next,
        );
        assert_eq!(next.limited, None);
    }

    #[test]
    fn only_a_failed_turn_of_the_conversation_s_own_is_stopped_by_the_limit() {
        let limit = |parent: Value| {
            json!({"type":"assistant","error":"rate_limit","parent_tool_use_id":parent,
                   "message":{"id":"m1","role":"assistant","content":[{"type":"text","text":"You've hit your limit"}]}})
        };
        let end = |is_error: bool| json!({"type":"result","subtype":"success","is_error":is_error,"result":"x"});
        // A subagent met it: the conversation's turn goes on.
        let mut a = rt();
        let mut fx = Effects::default();
        a.handle_frame(&limit(json!("toolu_sub")), &mut fx);
        let mut done = Effects::default();
        a.handle_frame(&end(true), &mut done);
        assert_eq!(done.limited, None);
        // The turn succeeded all the same (a fallback model).
        a.handle_frame(&limit(Value::Null), &mut fx);
        let mut done = Effects::default();
        a.handle_frame(&end(false), &mut done);
        assert_eq!(done.limited, None);
    }

    #[test]
    fn a_turn_that_goes_through_drops_an_old_planned_resume() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.meta.resume_at = Some(1_790_800_030_000);
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false,"result":"ok"}),
            &mut fx,
        );
        assert_eq!(a.meta.resume_at, None);
        assert!(fx.save && fx.agent_changed);
    }

    #[test]
    fn a_message_of_the_user_s_cancels_the_planned_resume() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.meta.resume_at = Some(1_790_800_030_000);
        a.push_user("u1", "je reprends la main", 0, &[], &mut fx);
        assert_eq!(a.meta.resume_at, None);
        assert!(fx.save);
    }

    #[test]
    fn a_background_task_that_ends_while_the_agent_waits_is_shown_once() {
        let mut a = rt();
        let mut fx = Effects::default();
        let system = |a: &mut AgentRt, f: Value, fx: &mut Effects| a.handle_frame(&f, fx);
        system(
            &mut a,
            json!({"type":"system","subtype":"task_started","task_id":"bg1","tool_use_id":"t1","description":"npm test","is_backgrounded":true}),
            &mut fx,
        );
        system(
            &mut a,
            json!({"type":"system","subtype":"task_started","task_id":"fg1","tool_use_id":"t2","description":"ls","is_backgrounded":false}),
            &mut fx,
        );
        // A command in the foreground: its tool row already shows it ended.
        system(
            &mut a,
            json!({"type":"system","subtype":"task_notification","task_id":"fg1","tool_use_id":"t2","status":"completed","summary":"ls","uuid":"s0"}),
            &mut fx,
        );
        // Idle, Claude Code only tells this (then starts a turn without a user message).
        system(
            &mut a,
            json!({"type":"system","subtype":"task_notification","task_id":"bg1","tool_use_id":"t1","status":"completed",
                    "summary":"Background command \"npm test\" completed (exit code 0)","uuid":"s1"}),
            &mut fx,
        );
        let events = a.conv.ids_where(|v| v["kind"] == "event");
        assert_eq!(events, vec!["s1".to_string()]);
        let text = a.conv.get("s1").unwrap()["text"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(text.contains("<status>completed</status>"), "{text}");
        assert!(
            text.contains(
                "<summary>Background command \"npm test\" completed (exit code 0)</summary>"
            ),
            "{text}"
        );
        // Mid-turn, the same notification is also replayed to Claude: not shown twice.
        a.handle_frame(
            &json!({"type":"user","message":{"role":"user","content":"<task-notification>\n<task-id>bg1</task-id>\n<status>completed</status>\n</task-notification>"},
                    "uuid":"r1","isReplay":true,"origin":{"kind":"task-notification"}}),
            &mut fx,
        );
        assert_eq!(a.conv.ids_where(|v| v["kind"] == "event").len(), 1);
    }

    #[test]
    fn only_a_human_s_message_is_the_user_s() {
        let mut a = rt();
        let mut fx = Effects::default();
        let replay = |uuid: &str, content: &str, extra: Value| {
            let mut f = json!({"type":"user","message":{"role":"user","content":content},"parent_tool_use_id":null,
                               "uuid":uuid,"isReplay":true});
            f.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            f
        };
        for f in [
            // From claude.ai (Remote Control), or with no origin at all: the user's.
            replay(
                "h1",
                "depuis le téléphone",
                json!({"origin":{"kind":"human"}}),
            ),
            replay("n1", "sans origine", json!({})),
            // Queued by Claude Code itself: an MCP channel, a meta message…
            replay(
                "c1",
                "alerte du canal",
                json!({"origin":{"kind":"channel","server":"slack"}}),
            ),
            replay("s1", "continue", json!({"isSynthetic":true})),
            // …and nothing to show.
            replay("e1", "  ", json!({"origin":{"kind":"peer"}})),
        ] {
            a.handle_frame(&f, &mut fx);
        }
        let kinds: Vec<(String, String, String)> = a
            .conv
            .items()
            .iter()
            .map(|i| {
                let s = |k: &str| i[k].as_str().unwrap_or_default().to_string();
                (
                    s("id"),
                    s("kind"),
                    format!("{}{}", s("source"), s("origin")),
                )
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                ("h1".into(), "user".into(), "remote".into()),
                ("n1".into(), "user".into(), "remote".into()),
                ("c1".into(), "event".into(), "channel".into()),
                ("s1".into(), "event".into(), "synthetic".into()),
            ]
        );
        assert_eq!(a.meta.prompts, 2);
    }

    #[test]
    fn the_context_window_follows_a_switch_of_model() {
        let mut a = rt();
        let mut fx = Effects::default();
        let start = |a: &mut AgentRt, id: &str, model: &str, fx: &mut Effects| {
            a.handle_frame(
                &json!({"type":"stream_event","parent_tool_use_id":null,
                        "event":{"type":"message_start","message":{"id":id,"model":model,"usage":{"input_tokens":10}}}}),
                fx,
            );
        };
        let usage = |read: u64, window: u64| json!({"inputTokens":10,"outputTokens":10,"cacheReadInputTokens":read,"cacheCreationInputTokens":0,"costUSD":0.1,"contextWindow":window});
        start(&mut a, "m1", "claude-sonnet-5", &mut fx);
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false,
                    "modelUsage":{"claude-sonnet-5":usage(900_000, 200_000)}}),
            &mut fx,
        );
        assert_eq!(a.view().context_window, 200_000);
        // Switched to Opus mid-process: Sonnet's figures, cumulative, still read the most.
        start(&mut a, "m2", "claude-opus-5-5", &mut fx);
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false,
                    "modelUsage":{"claude-sonnet-5":usage(900_000, 200_000),"claude-opus-5-5":usage(50_000, 1_000_000)}}),
            &mut fx,
        );
        assert_eq!(a.view().context_window, 1_000_000);
    }

    #[test]
    fn knows_the_context_window_of_the_conversation_s_model() {
        let mut a = rt();
        let mut fx = Effects::default();
        assert_eq!(a.view().context_window, 0);
        // The conversation's model reads the most; a subagent's may have a smaller window.
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false,"modelUsage":{
                "claude-opus-5-5":{"inputTokens":120,"outputTokens":900,"cacheReadInputTokens":80000,"cacheCreationInputTokens":4000,"costUSD":0.5,"contextWindow":1000000},
                "claude-haiku-4-5":{"inputTokens":3000,"outputTokens":200,"cacheReadInputTokens":0,"cacheCreationInputTokens":0,"costUSD":0.01,"contextWindow":200000}}}),
            &mut fx,
        );
        assert_eq!(a.view().context_window, 1_000_000);
    }

    #[test]
    fn follows_the_remote_control_link_state() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"system","subtype":"bridge_state","state":"connected"}),
            &mut fx,
        );
        assert_eq!(a.view().remote_state.as_deref(), Some("connected"));
        assert!(fx.agent_changed);
        a.on_exit(a.gen, Some(0), "", &mut fx);
        assert_eq!(a.view().remote_state, None);
    }

    #[test]
    fn a_process_that_dies_mid_turn_drops_its_estimate() {
        let mut a = rt();
        let mut fx = Effects::default();
        message_start(
            &mut a,
            "m1",
            None,
            json!({"input_tokens":1000,"output_tokens":1}),
            &mut fx,
        );
        a.on_exit(a.gen, Some(1), "boom", &mut fx);
        let v = a.view();
        assert_eq!((v.live_tokens, v.live_cost), (0, 0.0));
    }

    fn start_streaming(a: &mut AgentRt, fx: &mut Effects) {
        a.handle_frame(&json!({"type":"stream_event","event":{"type":"message_start","message":{"id":"m9","usage":{}}},"parent_tool_use_id":null}), fx);
        a.handle_frame(&json!({"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}},"parent_tool_use_id":null}), fx);
        a.handle_frame(&json!({"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Je comm"}},"parent_tool_use_id":null}), fx);
        a.handle_frame(&json!({"type":"stream_event","event":{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"tu7","name":"Bash"}},"parent_tool_use_id":null}), fx);
    }

    #[test]
    fn the_passage_to_be_read_aloud_is_kept_apart_from_the_answer() {
        let mut a = rt();
        let mut fx = Effects::default();
        // Streamed, then written whole.
        start_streaming(&mut a, &mut fx);
        a.handle_frame(&json!({"type":"assistant","message":{"id":"m9","content":[{"type":"text","text":"J'ai corrigé `a.rs`.\n\n<spoken>C'est corrigé, **les tests passent**.</spoken>"}]},"parent_tool_use_id":null}), &mut fx);
        let text = a.conv.get("m9:0").unwrap().clone();
        assert_eq!(text["text"], "J'ai corrigé `a.rs`.");
        assert_eq!(text["spoken"], "C'est corrigé, les tests passent.");
        // Written whole at once.
        a.handle_frame(&json!({"type":"assistant","message":{"id":"m10","content":[{"type":"text","text":"Fini.\n<spoken>Fini !</spoken>"}]},"parent_tool_use_id":null}), &mut fx);
        let text = a.conv.get("m10:a0").unwrap().clone();
        assert_eq!(
            (text["text"].as_str(), text["spoken"].as_str()),
            (Some("Fini."), Some("Fini !"))
        );
        // A subagent's is its own business: left as it is.
        a.handle_frame(&json!({"type":"assistant","message":{"id":"m11","content":[{"type":"text","text":"<spoken>x</spoken>"}]},"parent_tool_use_id":"tu7"}), &mut fx);
        assert!(a
            .conv
            .items()
            .iter()
            .all(|i| i["spoken"].is_null() || i["parent"].is_null()));
    }

    #[test]
    fn an_interrupted_turn_closes_open_blocks_and_tools() {
        let mut a = rt();
        let mut fx = Effects::default();
        start_streaming(&mut a, &mut fx);
        a.interrupted = true;
        a.handle_frame(&json!({"type":"result","subtype":"error_during_execution","is_error":true,"duration_ms":5}), &mut fx);
        let text = a.conv.get("m9:0").unwrap().clone();
        assert_eq!(text["streaming"], false);
        assert_eq!(text["text"], "Je comm");
        assert_eq!(a.conv.get("tu7").unwrap()["status"], "interrupted");
    }

    #[test]
    fn a_process_exit_closes_open_blocks_and_persists_their_text() {
        let dir = std::env::temp_dir().join(format!("ccm-agent-tests-{}", std::process::id()));
        let mut a = AgentRt::new(
            AgentMeta {
                id: new_id(),
                cwd: "C:/p".into(),
                ..Default::default()
            },
            &dir,
        );
        let mut fx = Effects::default();
        start_streaming(&mut a, &mut fx);
        a.on_exit(a.gen, Some(1), "", &mut fx);
        assert_eq!(a.conv.get("m9:0").unwrap()["streaming"], false);
        assert_eq!(a.conv.get("tu7").unwrap()["status"], "interrupted");
        // Reloading the log (e.g. after a restart) shows the partial text, not an empty block.
        let mut reloaded = Conv::new(&dir, &a.meta.id);
        let items = reloaded.items();
        let text = items.iter().find(|i| i["id"] == "m9:0").unwrap();
        assert_eq!(text["text"], "Je comm");
        assert_eq!(text["streaming"], false);
    }

    #[test]
    fn ansi_and_tags_are_stripped() {
        assert_eq!(strip_ansi("\u{1b}[31mrouge\u{1b}[0m"), "rouge");
        assert_eq!(
            strip_tags("<local-command-stdout>ok</local-command-stdout>"),
            "ok"
        );
    }

    #[test]
    fn the_activity_follows_the_main_threads_tools_and_clears_at_the_end_of_the_turn() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.handle_frame(&json!({"type":"stream_event","event":{"type":"message_start","message":{"id":"m1","usage":{}}},"parent_tool_use_id":null}), &mut fx);
        a.handle_frame(&json!({"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}},"parent_tool_use_id":null}), &mut fx);
        assert_eq!(a.view().activity.as_deref(), Some("Réfléchit"));
        a.handle_frame(&json!({"type":"assistant","message":{"id":"m2","content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"C:/p/src/db.ts"}}]},"parent_tool_use_id":null}), &mut fx);
        assert_eq!(a.view().activity.as_deref(), Some("Lit src/db.ts"));
        a.handle_frame(&json!({"type":"assistant","message":{"id":"m3","content":[{"type":"tool_use","id":"t2","name":"Bash","input":{"command":"npm test -- --run\nnpm run lint"}}]},"parent_tool_use_id":null}), &mut fx);
        assert_eq!(
            a.view().activity.as_deref(),
            Some("Lance npm test -- --run")
        );
        // A subagent's tools are its own business.
        a.handle_frame(&json!({"type":"assistant","message":{"id":"m4","content":[{"type":"tool_use","id":"t3","name":"Write","input":{"file_path":"C:/p/x.ts"}}]},"parent_tool_use_id":"t9"}), &mut fx);
        assert_eq!(
            a.view().activity.as_deref(),
            Some("Lance npm test -- --run")
        );
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1}),
            &mut fx,
        );
        assert_eq!(a.view().activity, None);
    }

    #[test]
    fn tool_activities_name_what_the_agent_does() {
        let cwd = "C:/p";
        let act = |name: &str, input: Value| tool_activity(name, &input, cwd);
        assert_eq!(
            act("Grep", json!({"pattern":"TODO"})).as_deref(),
            Some("Cherche TODO")
        );
        assert_eq!(
            act("Glob", json!({"pattern":"**/*.ts"})).as_deref(),
            Some("Cherche **/*.ts")
        );
        assert_eq!(
            act("Edit", json!({"file_path":"C:/p/src/a.ts"})).as_deref(),
            Some("Modifie src/a.ts")
        );
        assert_eq!(
            act("MultiEdit", json!({"file_path":"C:/p/src/a.ts"})).as_deref(),
            Some("Modifie src/a.ts")
        );
        assert_eq!(
            act("Write", json!({"file_path":"C:/p/b.ts"})).as_deref(),
            Some("Écrit b.ts")
        );
        assert_eq!(act("Task", json!({})).as_deref(), Some("Délègue"));
        assert_eq!(act("Read", Value::Null).as_deref(), Some("Lit"));
        assert_eq!(act("WebSearch", json!({})), None);
    }

    #[test]
    fn an_action_refused_without_a_word_is_told_to_claude_in_its_language() {
        use crate::i18n::Lang::{En, Fr};
        assert_eq!(refused_action(Fr), "L'utilisateur a refusé cette action.");
        assert_eq!(refused_action(En), "The user refused this action.");
    }

    #[test]
    fn what_the_agent_does_and_what_its_conversation_is_told_read_in_english() {
        use crate::i18n::Lang::{En, Fr};
        let act = |name: &str, input: Value| tool_activity_in(En, name, &input, "C:/p");
        assert_eq!(
            act("Read", json!({"file_path":"C:/p/src/db.ts"})).as_deref(),
            Some("Reading src/db.ts")
        );
        assert_eq!(
            act("Bash", json!({"command":"npm test"})).as_deref(),
            Some("Running npm test")
        );
        // A bare verb is the start of the same verb with what it acts on, as in French.
        assert_eq!(act("Read", Value::Null).as_deref(), Some("Reading"));
        assert_eq!(act("Task", json!({})).as_deref(), Some("Delegating"));
        assert_eq!(
            [block_activity(En, "text"), block_activity(En, "thinking")],
            [Some("Replying".to_string()), Some("Thinking".to_string())]
        );
        assert_eq!(
            exit_notices(En, "Claude"),
            [
                "Claude session not found: a new session will start with your next message.",
                "Copy point not found in the original’s session: Claude picks up its whole conversation, up to now.",
            ]
        );
        assert_eq!(
            stopped(En, "Claude Code", "3"),
            "Claude Code stopped (code 3)"
        );
        assert_eq!(
            stopped(Fr, "Claude Code", "3"),
            "Claude Code s'est arrêté (code 3)"
        );
        // Another harness, its own name.
        assert_eq!(stopped(Fr, "Codex", "3"), "Codex s'est arrêté (code 3)");
        assert_eq!(
            exit_notices(Fr, "Codex"),
            [
                "Session Codex introuvable : une nouvelle session sera démarrée au prochain message.",
                "Point de copie introuvable dans la session de l'original : Codex reprend toute sa conversation, jusqu'à maintenant."
            ]
        );
        // The count of a to-do list, as the conversation's card writes it (`conv.tools.tasks`).
        let todos = |n: usize, lang| {
            tool_arg_in(
                lang,
                "TodoWrite",
                &json!({ "todos": vec![json!({}); n] }),
                "C:/p",
            )
        };
        assert_eq!([todos(1, En), todos(2, En)], ["1 task", "2 tasks"]);
        assert_eq!([todos(1, Fr), todos(2, Fr)], ["1 tâche", "2 tâches"]);
    }

    #[test]
    fn an_unknown_field_or_frame_of_a_newer_claude_code_leaves_the_turn_going() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.push_user("u1", "Go", 0, &[], &mut fx);
        let frames = [
            // Frames, events and blocks Escouade does not know: set aside.
            json!({"type":"brand_new_frame","payload":{"x":1}}),
            json!({"type":"system","subtype":"brand_new_subtype","session_id":"s1","extra":[1,2]}),
            json!({"type":"stream_event","event":{"type":"message_start","message":{"id":"m1","usage":{},"new_field":true}},"parent_tool_use_id":null,"new_field":"x"}),
            json!({"type":"stream_event","event":{"type":"brand_new_event","index":0},"parent_tool_use_id":null}),
            json!({"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"brand_new_block","data":"?"}},"parent_tool_use_id":null}),
            json!({"type":"stream_event","event":{"type":"content_block_start","index":1,"content_block":{"type":"text","text":"","new_field":1}},"parent_tool_use_id":null}),
            json!({"type":"stream_event","event":{"type":"content_block_delta","index":1,"delta":{"type":"brand_new_delta","x":"?"}},"parent_tool_use_id":null}),
            json!({"type":"stream_event","event":{"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Fait.","new_field":0}},"parent_tool_use_id":null}),
            // Fields it does not know beside the ones it reads: ignored.
            json!({"type":"assistant","message":{"id":"m1","content":[{"type":"brand_new_block"},{"type":"text","text":"Fait.","citations_v9":[]}],"new_field":{}},"parent_tool_use_id":null,"uuid":"e1","new_field":"x"}),
            json!({"type":"rate_limit_event","rate_limit_info":{"status":"allowed","brand_new":"x"},"new_field":1}),
        ];
        for f in &frames {
            a.handle_frame(f, &mut fx);
        }
        assert_eq!(fx.turn_end, None, "nothing ended the turn");
        assert_eq!(a.meta.status, AgentStatus::Running);
        let mut fx = Effects::default();
        a.handle_frame(&json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1,"result":"Fait.","brand_new_field":{"a":1}}), &mut fx);
        assert_eq!(fx.turn_end, Some(TurnEnd::Finished("Fait.".into())));
        assert_eq!(a.meta.status, AgentStatus::Done);
        // The text is there, once, and no notice says something went wrong.
        let items = a.conv.items();
        let texts: Vec<&Value> = items.iter().filter(|i| i["kind"] == "text").collect();
        assert_eq!(texts.len(), 1, "{items:?}");
        assert_eq!(texts[0]["text"], "Fait.");
        assert!(!items.iter().any(|i| i["kind"] == "notice"), "{items:?}");
    }

    #[test]
    fn the_end_of_a_turn_tells_how_it_ended_with_the_assistants_text() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.push_user("u1", "Go", 0, &[], &mut fx);
        a.handle_frame(&json!({"type":"assistant","message":{"id":"m1","content":[{"type":"text","text":"Fait."}]},"parent_tool_use_id":null}), &mut fx);
        a.handle_frame(&json!({"type":"assistant","message":{"id":"m2","content":[{"type":"text","text":"```escouade\n{}\n```"}]},"parent_tool_use_id":null}), &mut fx);
        a.handle_frame(&json!({"type":"assistant","message":{"id":"m3","content":[{"type":"text","text":"sous-agent"}]},"parent_tool_use_id":"t1"}), &mut fx);
        let mut fx = Effects::default();
        a.handle_frame(&json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1,"result":"ignoré"}), &mut fx);
        assert_eq!(
            fx.turn_end,
            Some(TurnEnd::Finished("Fait.\n\n```escouade\n{}\n```".into()))
        );
        // Without text of its own, the result's.
        let mut fx = Effects::default();
        a.handle_frame(&json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1,"result":"Résumé"}), &mut fx);
        assert_eq!(fx.turn_end, Some(TurnEnd::Finished("Résumé".into())));
        let mut fx = Effects::default();
        a.handle_frame(&json!({"type":"result","subtype":"error_during_execution","is_error":true,"duration_ms":1,"result":"Boom"}), &mut fx);
        assert_eq!(fx.turn_end, Some(TurnEnd::Error("Boom".into())));
        a.interrupted = true;
        let mut fx = Effects::default();
        a.handle_frame(&json!({"type":"result","subtype":"error_during_execution","is_error":true,"duration_ms":1}), &mut fx);
        assert_eq!(fx.turn_end, Some(TurnEnd::Interrupted));
        a.handle_frame(&json!({"type":"assistant","error":"rate_limit","message":{"id":"m9","content":[{"type":"text","text":"limit"}]},"parent_tool_use_id":null}), &mut Effects::default());
        let mut fx = Effects::default();
        a.handle_frame(&json!({"type":"result","subtype":"success","is_error":true,"duration_ms":1,"result":"limit"}), &mut fx);
        assert_eq!(fx.turn_end, Some(TurnEnd::Limited));
    }

    #[test]
    fn a_process_dying_during_a_turn_ends_it_in_error() {
        let mut a = rt();
        a.saw_init = true;
        a.push_user("u1", "Bonjour", 0, &[], &mut Effects::default());
        let mut fx = Effects::default();
        a.on_exit(a.gen, Some(3), "", &mut fx);
        assert_eq!(
            fx.turn_end,
            Some(TurnEnd::Error("Claude Code s'est arrêté (code 3)".into()))
        );
        // Idle, its exit ends no turn.
        let mut fx = Effects::default();
        a.on_exit(a.gen, Some(0), "", &mut fx);
        assert_eq!(fx.turn_end, None);
    }

    #[test]
    fn a_copy_forks_no_more_once_it_has_a_session_or_once_the_original_is_gone() {
        let forks = |a: &AgentRt| {
            (
                a.meta.session_id.clone(),
                a.meta.fork_of.clone(),
                a.meta.fork_at.clone(),
            )
        };
        // Its first turn gives it a session of its own.
        let mut a = rt();
        a.meta.fork_of = Some("s1".into());
        a.meta.fork_at = Some("e1".into());
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({ "type": "system", "subtype": "init", "session_id": "s2" }),
            &mut fx,
        );
        assert_eq!(forks(&a), (Some("s2".into()), None, None));
        assert!(fx.save);
        // The original's session is gone: a new session, as for a lost one of its own.
        let mut a = rt();
        a.meta.fork_of = Some("missing-1".into());
        a.meta.fork_at = Some("e1".into());
        a.on_exit(
            a.gen,
            Some(1),
            "No conversation found with session ID: missing-1",
            &mut Effects::default(),
        );
        assert_eq!(forks(&a), (None, None, None));
    }

    #[test]
    fn a_copy_whose_fork_point_is_gone_forks_the_whole_session() {
        let mut a = rt();
        a.meta.fork_of = Some("s1".into());
        a.meta.fork_at = Some("missing-e1".into());
        let mut fx = Effects::default();
        a.on_exit(
            a.gen,
            Some(1),
            "No message found with message.uuid of: missing-e1",
            &mut fx,
        );
        // Still a copy of the original's session, from its end.
        assert_eq!(
            (a.meta.fork_of.as_deref(), a.meta.fork_at.as_deref()),
            (Some("s1"), None)
        );
        assert_eq!(a.meta.status, AgentStatus::Done);
        assert_eq!(fx.notify, None);
        let notices: Vec<Value> = a
            .conv
            .items()
            .into_iter()
            .filter(|i| i["kind"] == "notice")
            .collect();
        assert_eq!(notices.len(), 1, "{notices:?}");
        assert_eq!(notices[0]["level"], "warn");
        assert_eq!(
            notices[0]["text"],
            "Point de copie introuvable dans la session de l'original : Claude reprend toute sa conversation, jusqu'à maintenant."
        );
    }

    #[test]
    fn the_last_entry_of_its_session_is_kept_for_a_copy_to_fork_at() {
        let mut a = rt();
        let mut fx = Effects::default();
        let frame = |uuid: &str, extra: Value| {
            let mut f = json!({ "type": "assistant", "uuid": uuid, "parent_tool_use_id": null,
                "message": { "id": format!("m-{uuid}"), "role": "assistant", "content": [{ "type": "text", "text": "ok" }] } });
            if let (Some(f), Some(extra)) = (f.as_object_mut(), extra.as_object()) {
                f.extend(extra.clone());
            }
            f
        };
        a.handle_frame(
            &json!({ "type": "system", "subtype": "init", "session_id": "s1" }),
            &mut fx,
        );
        a.handle_frame(&frame("e1", json!({})), &mut fx);
        assert_eq!(a.meta.last_entry.as_deref(), Some("e1"));
        // Not entries of its main chain: a subagent's, a message Claude Code makes up, an error.
        a.handle_frame(
            &frame("e2", json!({ "parent_tool_use_id": "tu1" })),
            &mut fx,
        );
        a.handle_frame(&frame("e3", json!({ "is_meta": true })), &mut fx);
        a.handle_frame(&frame("e4", json!({ "error": "rate_limit" })), &mut fx);
        assert_eq!(a.meta.last_entry.as_deref(), Some("e1"));
        // Same session, next turn: its later entry.
        a.handle_frame(
            &json!({ "type": "system", "subtype": "init", "session_id": "s1" }),
            &mut fx,
        );
        a.handle_frame(&frame("e5", json!({})), &mut fx);
        assert_eq!(a.meta.last_entry.as_deref(), Some("e5"));
        // A new session (the context cleared): none of its entries is the old one's.
        a.handle_frame(
            &json!({ "type": "system", "subtype": "init", "session_id": "s2" }),
            &mut fx,
        );
        assert_eq!(a.meta.last_entry, None);
    }

    #[test]
    fn a_tool_result_that_ends_a_turn_is_its_last_entry() {
        let mut a = rt();
        let mut fx = Effects::default();
        let result = |uuid: &str, extra: Value| {
            let mut f = json!({ "type": "user", "uuid": uuid, "parent_tool_use_id": null,
                "message": { "role": "user", "content": [{ "type": "tool_result", "tool_use_id": "tu1", "content": "fait" }] } });
            if let (Some(f), Some(extra)) = (f.as_object_mut(), extra.as_object()) {
                f.extend(extra.clone());
            }
            f
        };
        a.handle_frame(
            &json!({ "type": "assistant", "uuid": "e1", "parent_tool_use_id": null,
                "message": { "id": "m1", "role": "assistant", "content": [{ "type": "tool_use", "id": "tu1", "name": "Bash", "input": {} }] } }),
            &mut fx,
        );
        // Interrupted, or an API error, before any reply: the result of its tool ends the turn.
        a.handle_frame(&result("e2", json!({})), &mut fx);
        assert_eq!(a.meta.last_entry.as_deref(), Some("e2"));
        // Not entries of its main chain: a subagent's result, a message sent again from claude.ai,
        // a message with no tool result (a local command's output).
        a.handle_frame(
            &result("e3", json!({ "parent_tool_use_id": "tu0" })),
            &mut fx,
        );
        a.handle_frame(&result("e4", json!({ "isReplay": true })), &mut fx);
        a.handle_frame(
            &json!({ "type": "user", "uuid": "e5", "parent_tool_use_id": null,
                "message": { "role": "user", "content": [{ "type": "text", "text": "<local-command-stdout>ok</local-command-stdout>" }] } }),
            &mut fx,
        );
        assert_eq!(a.meta.last_entry.as_deref(), Some("e2"));
    }

    #[test]
    fn a_compaction_leaves_no_entry_to_fork_at_until_the_next_one() {
        let mut a = rt();
        let mut fx = Effects::default();
        let reply = |uuid: &str| {
            json!({ "type": "assistant", "uuid": uuid, "parent_tool_use_id": null,
                "message": { "id": format!("m-{uuid}"), "role": "assistant", "content": [{ "type": "text", "text": "ok" }] } })
        };
        a.handle_frame(&reply("e1"), &mut fx);
        // The chain starts again at the boundary: what came before is no longer in it.
        a.handle_frame(
            &json!({ "type": "system", "subtype": "compact_boundary", "uuid": "b1" }),
            &mut fx,
        );
        assert_eq!(a.meta.last_entry, None);
        a.handle_frame(&reply("e2"), &mut fx);
        assert_eq!(a.meta.last_entry.as_deref(), Some("e2"));
    }

    #[test]
    fn a_new_last_entry_is_saved_without_waiting_for_the_end_of_the_turn() {
        let mut a = rt();
        let reply = |uuid: &str| {
            json!({ "type": "assistant", "uuid": uuid, "parent_tool_use_id": null,
                "message": { "id": format!("m-{uuid}"), "role": "assistant", "content": [{ "type": "text", "text": "ok" }] } })
        };
        // What a frame asks to save.
        let saves = |a: &mut AgentRt, frame: Value| {
            let mut fx = Effects::default();
            a.handle_frame(&frame, &mut fx);
            fx.save
        };
        // The app stopping for good mid-turn: a copy still forks at its latest entry.
        assert!(saves(&mut a, reply("e1")));
        // The same entry told again: nothing new to save.
        assert!(!saves(&mut a, reply("e1")));
        // A tool's result in its main chain.
        assert!(saves(
            &mut a,
            json!({ "type": "user", "uuid": "e2", "parent_tool_use_id": null,
                "message": { "role": "user", "content": [{ "type": "tool_result", "tool_use_id": "tu1", "content": "fait" }] } })
        ));
        assert_eq!(a.meta.last_entry.as_deref(), Some("e2"));
        // A compaction, which leaves none.
        assert!(saves(
            &mut a,
            json!({ "type": "system", "subtype": "compact_boundary", "uuid": "b1" })
        ));
        assert_eq!(a.meta.last_entry, None);
    }

    #[test]
    fn a_session_that_cannot_be_resumed_still_ends_the_turn_it_was_to_carry() {
        let lost =
            "Session Claude introuvable : une nouvelle session sera démarrée au prochain message.";
        let mut a = rt();
        a.meta.session_id = Some("missing-1".into());
        a.push_user("u1", "Bonjour", 0, &[], &mut Effects::default());
        let mut fx = Effects::default();
        a.on_exit(
            a.gen,
            Some(1),
            "No conversation found with session ID: missing-1",
            &mut fx,
        );
        assert_eq!(a.meta.status, AgentStatus::Done);
        assert_eq!(a.meta.session_id, None);
        assert_eq!(fx.notify, None);
        assert_eq!(fx.turn_end, Some(TurnEnd::Error(lost.into())));
        // Idle, it had no turn to end.
        let mut fx = Effects::default();
        a.on_exit(
            a.gen,
            Some(1),
            "No conversation found with session ID: missing-1",
            &mut fx,
        );
        assert_eq!(a.meta.status, AgentStatus::Done);
        assert_eq!(fx.turn_end, None);
    }

    #[test]
    fn a_failed_turn_always_says_what_went_wrong() {
        let mut a = rt();
        let mut end = |frame: Value| {
            let mut fx = Effects::default();
            a.handle_frame(&frame, &mut fx);
            fx.turn_end
        };
        let said = |text: &str| Some(TurnEnd::Error(text.into()));
        // The frame's own words first.
        assert_eq!(
            end(
                json!({"type":"result","subtype":"error_during_execution","is_error":true,"result":"Boom"})
            ),
            said("Boom")
        );
        assert_eq!(
            end(
                json!({"type":"result","subtype":"error_max_turns","is_error":true,"errors":["a","b"]})
            ),
            said("a\nb")
        );
        // Without any, the kind of result it is.
        assert_eq!(
            end(json!({"type":"result","subtype":"error_during_execution","is_error":true})),
            said("error_during_execution")
        );
        assert_eq!(
            end(
                json!({"type":"result","subtype":"error_max_turns","is_error":true,"result":"  ","errors":[]})
            ),
            said("error_max_turns")
        );
        // A "success" that is an error says nothing by its kind: the generic words.
        assert_eq!(
            end(json!({"type":"result","subtype":"success","is_error":true})),
            said("Claude Code a signalé une erreur")
        );
        assert_eq!(
            end(json!({"type":"result","is_error":true})),
            said("Claude Code a signalé une erreur")
        );
    }

    #[test]
    fn an_activity_gist_is_its_first_real_line_cut_short() {
        let cwd = "C:/p";
        let act = |name: &str, input: Value| tool_activity(name, &input, cwd);
        // A leading blank line does not leave a bare verb.
        assert_eq!(
            act("Bash", json!({"command":"\n\n  npm test\nnpm run lint"})).as_deref(),
            Some("Lance npm test")
        );
        assert_eq!(
            act("Bash", json!({"command":" \n "})).as_deref(),
            Some("Lance")
        );
        assert_eq!(
            act("Grep", json!({"pattern":"\nTODO\nFIXME"})).as_deref(),
            Some("Cherche TODO")
        );
        // Long ones are cut at 60 bytes, on a character boundary.
        let long = "é".repeat(100);
        let cut = format!("{}…", "é".repeat(30));
        assert_eq!(
            act("Grep", json!({"pattern":long})).as_deref(),
            Some(format!("Cherche {cut}").as_str())
        );
        assert_eq!(
            act("Glob", json!({"pattern":long})).as_deref(),
            Some(format!("Cherche {cut}").as_str())
        );
        assert_eq!(
            act("Bash", json!({"command":long})).as_deref(),
            Some(format!("Lance {cut}").as_str())
        );
    }

    #[test]
    fn a_command_that_begins_by_changing_folder_is_told_by_what_it_runs() {
        let act = |command: &str| tool_activity("Bash", &json!({ "command": command }), "C:/p");
        // How agents and subagents work in a worktree (seen on every command of two real runs).
        assert_eq!(
            act("cd \"C:/Users/dev/AppData/Local/Temp/p4-trial/repo/.claude/worktrees/agent-1\" && npx tsx --test slugify.test.ts 2>&1 | tail -12").as_deref(),
            Some("Lance npx tsx --test slugify.test.ts 2>&1 | tail -12")
        );
        assert_eq!(
            act("cd /p/repo && git status --short").as_deref(),
            Some("Lance git status --short")
        );
        assert_eq!(
            act("cd 'C:/a b' ; cd src && npm test").as_deref(),
            Some("Lance npm test")
        );
        // Alone, or with nothing after it: it is what it is.
        assert_eq!(act("cd /p/repo").as_deref(), Some("Lance cd /p/repo"));
        assert_eq!(act("cd /p/repo &&").as_deref(), Some("Lance cd /p/repo &&"));
        assert_eq!(
            act("cd /p/repo
npm test")
            .as_deref(),
            Some("Lance cd /p/repo")
        );
        assert_eq!(
            act("cdk deploy && cd x").as_deref(),
            Some("Lance cdk deploy && cd x")
        );
    }

    #[test]
    fn a_tool_starting_to_stream_does_not_blur_the_activity_of_the_same_tool() {
        let mut a = rt();
        let mut fx = Effects::default();
        let frame = |a: &mut AgentRt, f: Value| a.handle_frame(&f, &mut Effects::default());
        frame(
            &mut a,
            json!({"type":"stream_event","event":{"type":"message_start","message":{"id":"m1","usage":{}}},"parent_tool_use_id":null}),
        );
        frame(
            &mut a,
            json!({"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"t1","name":"Read"}},"parent_tool_use_id":null}),
        );
        // Nothing more precise is known yet.
        assert_eq!(a.view().activity.as_deref(), Some("Lit"));
        frame(
            &mut a,
            json!({"type":"assistant","message":{"id":"m1","content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"C:/p/src/a.ts"}}]},"parent_tool_use_id":null}),
        );
        assert_eq!(a.view().activity.as_deref(), Some("Lit src/a.ts"));
        // Another Read starts: the bare verb does not replace the precise one...
        frame(
            &mut a,
            json!({"type":"stream_event","event":{"type":"message_start","message":{"id":"m2","usage":{}}},"parent_tool_use_id":null}),
        );
        frame(
            &mut a,
            json!({"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"t2","name":"Read"}},"parent_tool_use_id":null}),
        );
        assert_eq!(a.view().activity.as_deref(), Some("Lit src/a.ts"));
        // ...but another verb does, and so does the next precise one.
        frame(
            &mut a,
            json!({"type":"stream_event","event":{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"t3","name":"Grep"}},"parent_tool_use_id":null}),
        );
        assert_eq!(a.view().activity.as_deref(), Some("Cherche"));
        frame(
            &mut a,
            json!({"type":"assistant","message":{"id":"m2","content":[{"type":"tool_use","id":"t3","name":"Grep","input":{"pattern":"TODO"}}]},"parent_tool_use_id":null}),
        );
        assert_eq!(a.view().activity.as_deref(), Some("Cherche TODO"));
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false}),
            &mut fx,
        );
        assert_eq!(a.view().activity, None);
    }

    // ---------- the plan ----------

    use crate::plan::TaskStatus;

    /// The assistant calls `name` (in the thread of the subagent `parent`, or the main one).
    fn call(a: &mut AgentRt, parent: Option<&str>, id: &str, name: &str, input: Value) -> Effects {
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"assistant","message":{"id":format!("m-{id}"),"content":[{"type":"tool_use","id":id,"name":name,"input":input}]},"parent_tool_use_id":parent}),
            &mut fx,
        );
        fx
    }

    /// The result of the call `id`, with the frame's structured `tur`.
    fn returns(a: &mut AgentRt, id: &str, text: &str, tur: Value) -> Effects {
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":id,"content":text}]},"tool_use_result":tur,"parent_tool_use_id":null}),
            &mut fx,
        );
        fx
    }

    fn system(a: &mut AgentRt, f: Value) -> Effects {
        let mut fx = Effects::default();
        a.handle_frame(&f, &mut fx);
        fx
    }

    fn session(a: &mut AgentRt, id: &str) -> Effects {
        system(a, json!({"type":"system","subtype":"init","session_id":id}))
    }

    /// The turn's result frame.
    fn turn_ends(a: &mut AgentRt) {
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1}),
            &mut Effects::default(),
        );
    }

    /// A task made through `TaskCreate`, with its result.
    fn creates(a: &mut AgentRt, id: &str, subject: &str, number: u32) {
        call(a, None, id, "TaskCreate", json!({ "subject": subject }));
        returns(
            a,
            id,
            &format!("Task #{number} created successfully: {subject}"),
            Value::Null,
        );
    }

    fn plan(a: &AgentRt) -> PlanState {
        a.view().meta.plan.expect("a plan")
    }

    #[test]
    fn the_plan_follows_the_task_tools_and_the_subagents_of_the_stream() {
        let mut a = rt();
        assert!(a.view().meta.plan.is_none());
        creates(&mut a, "c0", "Lire", 1);
        creates(&mut a, "c1", "Écrire", 2);
        call(
            &mut a,
            None,
            "u1",
            "TaskUpdate",
            json!({"taskId":"1","status":"in_progress","activeForm":"Lit"}),
        );
        call(
            &mut a,
            None,
            "a1",
            "Agent",
            json!({"description":"Lire le code","prompt":"Read task-1-brief.md","model":"haiku","subagent_type":"Explore"}),
        );
        // What the agent does, as before: the subagent's tools are its own business.
        assert_eq!(a.view().activity.as_deref(), Some("Délègue"));
        call(
            &mut a,
            Some("a1"),
            "k1",
            "Read",
            json!({"file_path":"C:/p/src/a.ts"}),
        );
        assert_eq!(a.view().activity.as_deref(), Some("Délègue"));

        let p = plan(&a);
        assert_eq!(
            serde_json::to_value(&p.tasks).unwrap(),
            json!([
                {"id":"1","title":"Lire","status":"inProgress","active":"Lit"},
                {"id":"2","title":"Écrire","status":"pending"},
            ])
        );
        let row = &p.agents[0];
        assert_eq!(
            (row.id.as_str(), row.title.as_str(), row.status, row.tools),
            ("a1", "Lire le code", RunStatus::Running, 1)
        );
        assert_eq!(row.doing.as_deref(), Some("Lit src/a.ts"));
        assert_eq!(row.model.as_deref(), Some("haiku"));
        assert_eq!(row.plan_task.as_deref(), Some("1"));
        assert_eq!(p.launched, 1);

        returns(
            &mut a,
            "a1",
            "Trouvé",
            json!({"status":"completed","agentId":"x","totalToolUseCount":3,"totalDurationMs":1500,"totalTokens":900}),
        );
        let p = plan(&a);
        assert_eq!(p.agents[0].status, RunStatus::Done);
        assert_eq!((p.agents[0].tools, p.agents[0].tokens), (3, Some(900)));
        assert_eq!(p.agents[0].doing, None);
        call(
            &mut a,
            None,
            "u2",
            "TaskUpdate",
            json!({"taskId":"1","status":"completed"}),
        );
        assert_eq!(plan(&a).tasks[0].status, TaskStatus::Done);
    }

    #[test]
    fn a_frame_that_comes_twice_is_one_task_and_one_subagent() {
        let mut a = rt();
        for _ in 0..2 {
            call(&mut a, None, "c0", "TaskCreate", json!({"subject":"Lire"}));
            call(
                &mut a,
                None,
                "a1",
                "Agent",
                json!({"description":"Aide","prompt":""}),
            );
        }
        let p = plan(&a);
        assert_eq!((p.tasks.len(), p.agents.len(), p.launched), (1, 1, 1));
    }

    #[test]
    fn a_detail_of_a_subagent_goes_to_the_window_and_only_a_change_is_saved() {
        let mut a = rt();
        // A subagent starts: the structure changed.
        let fx = call(
            &mut a,
            None,
            "a1",
            "Agent",
            json!({"description":"Aide","prompt":""}),
        );
        assert!(fx.agent_changed && fx.save);
        // What it does: the window hears of it, nothing is saved for it.
        let read = json!({"file_path":"C:/p/src/a.ts"});
        let fx = call(&mut a, Some("a1"), "k1", "Read", read.clone());
        assert!(fx.agent_changed);
        assert!(!fx.save);
        assert_eq!(plan(&a).agents[0].doing.as_deref(), Some("Lit src/a.ts"));
        // The same step by another call: nothing to tell.
        let fx = call(&mut a, Some("a1"), "k2", "Read", read);
        assert!(!fx.agent_changed && !fx.save);
        assert_eq!(plan(&a).agents[0].tools, 2);
        // Another step is news again.
        let fx = call(&mut a, Some("a1"), "k3", "Grep", json!({"pattern":"TODO"}));
        assert!(fx.agent_changed && !fx.save);
        // Its end is saved.
        let fx = returns(
            &mut a,
            "a1",
            "Fini",
            json!({"status":"completed","totalToolUseCount":3}),
        );
        assert!(fx.agent_changed && fx.save);
        // A tool of the main thread that is no business of the plan changes nothing of it.
        let fx = call(&mut a, None, "b1", "Bash", json!({"command":"npm test"}));
        assert!(fx.agent_changed, "the activity changed");
        assert_eq!(plan(&a).agents.len(), 1);
    }

    #[test]
    fn the_notification_of_a_background_subagent_ends_its_row_and_still_says_it_in_the_conversation(
    ) {
        let mut a = rt();
        call(
            &mut a,
            None,
            "a1",
            "Agent",
            json!({"description":"Tests","prompt":"","run_in_background":true}),
        );
        returns(
            &mut a,
            "a1",
            "Async agent launched successfully.\nagentId: bg1",
            json!({"status":"async_launched","agentId":"bg1"}),
        );
        system(
            &mut a,
            json!({"type":"system","subtype":"task_started","task_id":"bg1","tool_use_id":"a1",
                "description":"Tests","task_type":"local_agent","is_backgrounded":true}),
        );
        assert_eq!(plan(&a).agents[0].status, RunStatus::Running);
        let fx = system(
            &mut a,
            json!({"type":"system","subtype":"task_notification","task_id":"bg1","tool_use_id":"a1",
                "status":"completed","summary":"Agent \"Tests\" finished","uuid":"n1",
                "usage":{"total_tokens":700,"tool_uses":4,"duration_ms":10}}),
        );
        let p = plan(&a);
        assert_eq!(p.agents[0].status, RunStatus::Done);
        assert_eq!((p.agents[0].tools, p.agents[0].tokens), (4, Some(700)));
        assert!(fx.save);
        // The event row of the conversation is still made.
        assert_eq!(a.conv.get("n1").unwrap()["source"], "task");
    }

    /// A subagent `a1` in the background, its task `bg1` started.
    fn background_subagent(a: &mut AgentRt) {
        call(
            a,
            None,
            "a1",
            "Agent",
            json!({"description":"Tests","subagent_type":"general-purpose","model":"haiku","prompt":"Lance les tests"}),
        );
        system(
            a,
            json!({"type":"system","subtype":"task_started","task_id":"bg1","tool_use_id":"a1",
                "description":"Tests","task_type":"local_agent","is_backgrounded":true}),
        );
        returns(
            a,
            "a1",
            "Async agent launched successfully.\nagentId: bg1",
            json!({"status":"async_launched","agentId":"bg1"}),
        );
    }

    #[test]
    fn a_request_a_subagent_makes_names_it_and_stays_in_the_agents_conversation() {
        let mut a = rt();
        background_subagent(&mut a);
        call(
            &mut a,
            Some("a1"),
            "k1",
            "Write",
            json!({"file_path":"C:/p/x.txt","content":"x"}),
        );
        // As Claude Code 2.1.300 asks it (measured): `agent_id` is the subagent's task.
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"control_request","request_id":"r1","request":{
                "subtype":"can_use_tool","tool_name":"Write","tool_use_id":"k1","agent_id":"bg1",
                "input":{"file_path":"C:/p/x.txt","content":"x"}}}),
            &mut fx,
        );
        let item = a.conv.get("r1").unwrap().clone();
        assert_eq!(item["sub"], "a1");
        assert!(
            item["parent"].is_null(),
            "shown in the agent's conversation too"
        );
        // Without `agent_id`, the thread of the call that asks tells it.
        call(
            &mut a,
            Some("a1"),
            "k2",
            "Bash",
            json!({"command":"npm test"}),
        );
        a.handle_frame(
            &json!({"type":"control_request","request_id":"r2","request":{
                "subtype":"can_use_tool","tool_name":"Bash","tool_use_id":"k2","input":{"command":"npm test"}}}),
            &mut fx,
        );
        assert_eq!(a.conv.get("r2").unwrap()["sub"], "a1");
        // A question of the main thread is nobody's.
        a.handle_frame(
            &json!({"type":"control_request","request_id":"r3","request":{
                "subtype":"can_use_tool","tool_name":"AskUserQuestion","tool_use_id":"q1",
                "input":{"questions":[{"question":"Q ?","options":[]}]}}}),
            &mut fx,
        );
        assert!(a.conv.get("r3").unwrap().get("sub").is_none());
        // Nor is a request of a thread that is no subagent's (a workflow's agent).
        call(&mut a, Some("w1"), "k3", "Bash", json!({"command":"ls"}));
        a.handle_frame(
            &json!({"type":"control_request","request_id":"r4","request":{
                "subtype":"can_use_tool","tool_name":"Bash","tool_use_id":"k3","input":{"command":"ls"}}}),
            &mut fx,
        );
        assert!(a.conv.get("r4").unwrap().get("sub").is_none());
    }

    #[test]
    fn a_subagent_shows_the_model_its_messages_come_from_and_keeps_it_once_ended() {
        let mut a = rt();
        call(
            &mut a,
            None,
            "a1",
            "Agent",
            json!({"description":"Lire","subagent_type":"Explore","prompt":"Lis"}),
        );
        assert_eq!(plan(&a).agents[0].model, None);
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"assistant","message":{"id":"m-s1","model":"claude-haiku-5-5",
                "content":[{"type":"text","text":"Je lis."}]},"parent_tool_use_id":"a1"}),
            &mut fx,
        );
        assert_eq!(
            plan(&a).agents[0].model.as_deref(),
            Some("claude-haiku-5-5")
        );
        assert!(fx.agent_changed && fx.save);
        // The agent's own messages are not its subagent's.
        a.handle_frame(
            &json!({"type":"assistant","message":{"id":"m-0","model":"claude-opus-5-5",
                "content":[{"type":"text","text":"J'attends."}]},"parent_tool_use_id":null}),
            &mut Effects::default(),
        );
        assert_eq!(
            plan(&a).agents[0].model.as_deref(),
            Some("claude-haiku-5-5")
        );
        returns(
            &mut a,
            "a1",
            "Trouvé",
            json!({"status":"completed","agentId":"x","totalToolUseCount":1,"totalDurationMs":10,"totalTokens":50}),
        );
        assert_eq!(
            a.conv.get("a1").unwrap()["run"]["model"],
            "claude-haiku-5-5"
        );
    }

    #[test]
    fn the_report_and_the_totals_of_a_subagent_stay_with_its_call_in_the_conversation() {
        let mut a = rt();
        background_subagent(&mut a);
        assert!(a.conv.get("a1").unwrap().get("run").is_none());
        let report = "Status: DONE\nTous les tests passent.";
        let fx = system(
            &mut a,
            json!({"type":"system","subtype":"task_notification","task_id":"bg1","tool_use_id":"a1",
                "status":"completed","summary":"This agent's report was delivered to you…","uuid":"n1",
                "usage":{"total_tokens":700,"tool_uses":4,"duration_ms":10},
                "handback":"send","handback_report":{"text":report}}),
        );
        let item = a.conv.get("a1").unwrap().clone();
        assert_eq!(item["report"], report);
        let row = plan(&a).agents[0].clone();
        assert_eq!(
            item["run"],
            json!({"status":"done","model":"haiku","tokens":700,"tools":4,"startedAt":row.started_at,"endedAt":row.ended_at})
        );
        // The window hears of it, the log keeps it.
        assert!(fx
            .ops
            .iter()
            .any(|op| matches!(op, ConvOp::Patch { id, patch } if id == "a1" && patch.get("run").is_some())));
        let log = crate::conv::replay(&crate::conv::log_path(
            &std::env::temp_dir().join(format!("ccm-agent-tests-{}", std::process::id())),
            &a.meta.id,
        ))
        .unwrap();
        let kept = &log.items[log.index["a1"]];
        assert_eq!(kept["report"], report);
        assert_eq!(kept["run"]["status"], "done");

        // In the foreground, its result ends it; a process gone interrupts one still running.
        call(
            &mut a,
            None,
            "a2",
            "Agent",
            json!({"description":"Lire","prompt":"Lis"}),
        );
        returns(
            &mut a,
            "a2",
            "Trouvé",
            json!({"status":"completed","agentId":"x","totalToolUseCount":3,"totalDurationMs":1500,"totalTokens":900}),
        );
        assert_eq!(a.conv.get("a2").unwrap()["run"]["tokens"], 900);
        assert_eq!(a.conv.get("a2").unwrap()["run"]["status"], "done");
        call(
            &mut a,
            None,
            "a3",
            "Agent",
            json!({"description":"Attendre","prompt":"Attends"}),
        );
        a.detach();
        assert_eq!(a.conv.get("a3").unwrap()["run"]["status"], "interrupted");
        // A call that is no subagent's never gets one.
        call(&mut a, None, "b1", "Bash", json!({"command":"ls"}));
        returns(&mut a, "b1", "ok", json!({"stdout":"ok","stderr":""}));
        a.detach();
        assert!(a.conv.get("b1").unwrap().get("run").is_none());
    }

    #[test]
    fn a_subagents_prompt_is_kept_longer_than_another_input_but_bounded() {
        let mut a = rt();
        let long = "é".repeat(5_000);
        call(
            &mut a,
            None,
            "a1",
            "Agent",
            json!({"description":"Tests","prompt":long,"other":long}),
        );
        let input = a.conv.get("a1").unwrap()["input"].clone();
        assert_eq!(input["prompt"], long, "10 000 bytes: whole");
        assert!(input["other"].as_str().unwrap().len() < 4_100);
        let longer = "x".repeat(40_000);
        call(&mut a, None, "a2", "Task", json!({"prompt":longer}));
        let kept = a.conv.get("a2").unwrap()["input"]["prompt"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(kept.len() < 16_100 && kept.len() > 15_000 && kept.ends_with('…'));
        // Another tool's long text is cut as before.
        call(&mut a, None, "b1", "Bash", json!({"command":longer}));
        assert!(
            a.conv.get("b1").unwrap()["input"]["command"]
                .as_str()
                .unwrap()
                .len()
                < 4_100
        );
    }

    /// What Claude Code 2.1.300 wrote on stdout when a subagent was stopped by `stop_task` (an
    /// essay of the host's requests on a real run, docs/PROTOCOL.md).
    const REAL_STOP: &str = include_str!("../../tests/fixtures/real-run-stop-task.jsonl");

    /// What Claude Code 2.1.300 wrote on stdout while the agent passed a message on to its
    /// subagent with `SendMessage` (« Lui écrire », docs/PROTOCOL.md): the subagent ended with no
    /// `SubagentHandback`, its report in the `summary` of its notification.
    const REAL_SEND: &str = include_str!("../../tests/fixtures/real-run-send-message.jsonl");

    #[test]
    fn a_report_handed_back_without_subagent_handback_is_the_summary_of_its_notification() {
        let mut a = rt();
        replay(&mut a, REAL_SEND);
        let p = plan(&a);
        assert_eq!(p.agents.len(), 1);
        let row = &p.agents[0];
        assert_eq!(row.status, RunStatus::Done);
        let call = a.conv.get(&row.id).unwrap().clone();
        let report = call["report"].as_str().expect("its report");
        assert!(report.contains("The secret word is BANANA."), "{report}");
        // The message reached it through the agent's own SendMessage, to its task.
        let sent = a
            .conv
            .items()
            .into_iter()
            .find(|i| i["kind"] == "tool" && i["name"] == "SendMessage")
            .expect("the agent's SendMessage");
        assert_eq!(sent["input"]["to"], row.task_id.as_deref().unwrap());
        assert!(sent["parent"].is_null());
    }

    #[test]
    fn the_summary_of_a_notification_is_no_report_when_the_report_went_elsewhere_or_it_stopped() {
        for (extra, status) in [
            (json!({"handback": "send"}), "completed"),
            (json!({}), "stopped"),
            (json!({}), "failed"),
        ] {
            let mut a = rt();
            background_subagent(&mut a);
            let mut f = json!({"type":"system","subtype":"task_notification","task_id":"bg1",
                "tool_use_id":"a1","status":status,"summary":"This agent's report was delivered to you…"});
            for (k, v) in extra.as_object().unwrap() {
                f[k] = v.clone();
            }
            system(&mut a, f);
            assert!(
                a.conv.get("a1").unwrap().get("report").is_none(),
                "{status}"
            );
        }
    }

    #[test]
    fn the_generic_line_of_an_older_summary_is_no_report_but_a_text_is() {
        // Older versions of Claude Code, and the fake `claude`, only say that the subagent ended.
        for summary in [
            "Agent \"Tests\" finished",
            "Agent \"Tests\" completed",
            "Agent \"Tests\" failed",
            "Agent \"Tests\" stopped",
            "Agent \"Tests\" finished.",
            "Agent \"Lire \"a.ts\" et \"b.ts\"\" completed",
            "Agent \"\" finished",
            "  Agent \"Surveiller la suite de tests\" completed \n",
        ] {
            let mut a = rt();
            background_subagent(&mut a);
            system(
                &mut a,
                json!({"type":"system","subtype":"task_notification","task_id":"bg1","tool_use_id":"a1",
                    "status":"completed","summary":summary,"uuid":"n1"}),
            );
            assert!(
                a.conv.get("a1").unwrap().get("report").is_none(),
                "{summary:?}"
            );
        }
        // Anything else is the subagent's own text, even when it starts the same way.
        for summary in [
            "Tous les tests passent.",
            "Agent \"Tests\" finished: 12 tests pass.",
            "Agent \"Tests\" finished\nAll 12 tests pass.",
            "Agent \"Tests\" ended",
            "Agent Tests finished",
            "The agent \"Tests\" finished",
            "Agent \"Tests\" finished..",
        ] {
            let mut a = rt();
            background_subagent(&mut a);
            system(
                &mut a,
                json!({"type":"system","subtype":"task_notification","task_id":"bg1","tool_use_id":"a1",
                    "status":"completed","summary":summary,"uuid":"n1"}),
            );
            assert_eq!(a.conv.get("a1").unwrap()["report"], summary, "{summary:?}");
        }
    }

    /// A subagent `a1` launched in the background as the Claude Code view's transcript tells it
    /// (no `system` frame there): its call, and the result that gives its task `bg1`.
    fn launched_in_the_transcript(a: &mut AgentRt) {
        let mut fx = Effects::default();
        a.on_transcript(
            &json!({"type":"assistant","uuid":"e1","message":{"id":"m1","content":[{"type":"tool_use","id":"a1","name":"Agent",
                "input":{"description":"Tests","prompt":"Lance les tests","run_in_background":true}}]}}),
            &mut fx,
        );
        a.on_transcript(
            &json!({"type":"user","uuid":"e2","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"a1",
                "content":"Async agent launched successfully.\nagentId: bg1"}]},
                "toolUseResult":{"status":"async_launched","agentId":"bg1"}}),
            &mut fx,
        );
    }

    /// The notification of a task as Claude Code writes it for Claude.
    fn notification(task: &str, call: &str, status: &str) -> String {
        format!("<task-notification>\n<task-id>{task}</task-id>\n<tool-use-id>{call}</tool-use-id>\n<output-file>C:\\Temp\\{task}.output</output-file>\n<status>{status}</status>\n<summary>Agent \"Tests\" finished</summary>\n</task-notification>")
    }

    #[test]
    fn in_the_claude_code_view_the_replayed_notification_of_a_background_subagent_ends_its_row() {
        for (status, ended) in [
            ("completed", RunStatus::Done),
            ("failed", RunStatus::Failed),
            ("killed", RunStatus::Stopped),
            ("stopped", RunStatus::Stopped),
        ] {
            let mut a = rt();
            launched_in_the_transcript(&mut a);
            assert_eq!(plan(&a).agents[0].status, RunStatus::Running, "{status}");
            let mut fx = Effects::default();
            a.on_transcript(
                &json!({"type":"user","uuid":"n1","origin":{"kind":"task-notification"},
                    "message":{"role":"user","content":notification("bg1", "a1", status)}}),
                &mut fx,
            );
            let row = plan(&a).agents[0].clone();
            assert_eq!(row.status, ended, "{status}");
            assert!(
                row.ended_at.is_some() && fx.save && fx.agent_changed,
                "{status}"
            );
            // Said on its call, and still told in the conversation.
            assert_eq!(a.conv.get("a1").unwrap()["run"]["status"], json!(ended));
            assert_eq!(a.conv.get("n1").unwrap()["source"], "task");
            // No report made of a notification's words.
            assert!(
                a.conv.get("a1").unwrap().get("report").is_none(),
                "{status}"
            );
        }
    }

    #[test]
    fn a_replayed_notification_of_a_task_no_row_has_changes_no_plan() {
        let mut a = rt();
        launched_in_the_transcript(&mut a);
        let before = plan(&a).agents;
        a.on_transcript(
            &json!({"type":"user","uuid":"n1","origin":{"kind":"task-notification"},
                "message":{"role":"user","content":notification("bash9", "toolu_bash", "completed")}}),
            &mut Effects::default(),
        );
        assert_eq!(plan(&a).agents, before);
    }

    #[test]
    fn in_the_stream_the_replay_of_a_notification_changes_nothing_of_a_row_that_ended() {
        let mut a = rt();
        background_subagent(&mut a);
        let report = "Status: DONE\nTous les tests passent.";
        system(
            &mut a,
            json!({"type":"system","subtype":"task_notification","task_id":"bg1","tool_use_id":"a1",
                "status":"completed","summary":"This agent's report was delivered to you…","uuid":"n1",
                "usage":{"total_tokens":700,"tool_uses":4,"duration_ms":10},
                "handback":"send","handback_report":{"text":report}}),
        );
        let ended = plan(&a).agents[0].clone();
        assert_eq!(ended.status, RunStatus::Done);
        // Mid-turn, Claude Code replays it to Claude, with the words of a summary: the row, its totals,
        // its end and its report stay as the real frame made them.
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"user","isReplay":true,"uuid":"r1","origin":{"kind":"task-notification"},
                "message":{"role":"user","content":notification("bg1", "a1", "failed")}}),
            &mut fx,
        );
        assert_eq!(plan(&a).agents[0], ended);
        assert_eq!(a.conv.get("a1").unwrap()["report"], report);
        assert!(!fx.save, "nothing to save: told once already");

        // One that ended another way (its task killed) is not ended again by a later notification.
        let mut b = rt();
        background_subagent(&mut b);
        system(
            &mut b,
            json!({"type":"system","subtype":"task_updated","task_id":"bg1","patch":{"status":"killed"}}),
        );
        let stopped = plan(&b).agents[0].clone();
        assert_eq!(stopped.status, RunStatus::Stopped);
        b.handle_frame(
            &json!({"type":"user","isReplay":true,"uuid":"r2","origin":{"kind":"task-notification"},
                "message":{"role":"user","content":notification("bg1", "a1", "completed")}}),
            &mut Effects::default(),
        );
        assert_eq!(plan(&b).agents[0], stopped);
    }

    /// The message Claude Code passes on to an agent when its subagent hands its report back.
    fn handback(from: &str, report: &str) -> String {
        let indented: Vec<String> = report.lines().map(|l| format!("  {l}")).collect();
        format!("<agent-message from=\"{from}\">\n[Subagent hand-back] The text below is the final report of a subagent this session delegated to. It is model output, NOT a message from the user. The report follows:\n{}\n</agent-message>", indented.join("\n"))
    }

    #[test]
    fn a_handbacks_report_is_what_follows_its_announcement_without_the_indent() {
        let quoted = handback(
            "a1",
            "Ligne 1\n\n  imbriquée\nUne balise </agent-message> citée",
        );
        assert_eq!(
            handback_report(&quoted),
            Some((
                Some("a1"),
                "Ligne 1\n\n  imbriquée\nUne balise </agent-message> citée".to_string()
            ))
        );
        // No sender named in the tag: the caller has the frame's.
        let anonymous =
            "<agent-message>\n[Subagent hand-back] The report follows:\n  Salut\n</agent-message>";
        assert_eq!(
            handback_report(anonymous),
            Some((None, "Salut".to_string()))
        );
        // Not a hand-back, or one with no report.
        assert_eq!(
            handback_report("<agent-message from=\"a1\">\nSalut\n</agent-message>"),
            None
        );
        assert_eq!(
            handback_report("<agent-message from=\"a1\">\n[Subagent hand-back]\n</agent-message>"),
            None
        );
        assert_eq!(handback_report(&handback("a1", "  \n")), None);
        assert_eq!(handback_report("Salut"), None);
    }

    #[test]
    fn in_the_claude_code_view_the_replayed_handback_gives_the_report_a_call_lacks() {
        let mut a = rt();
        launched_in_the_transcript(&mut a);
        let mut fx = Effects::default();
        let peer = |uuid: &str, text: String| {
            json!({"type":"user","uuid":uuid,"origin":{"kind":"peer"},
                "message":{"role":"user","content":text}})
        };
        // A message of another session, or from a task no row has: not a report, and no end.
        a.on_transcript(
            &peer(
                "p0",
                "<agent-message from=\"bg1\">\nOn se synchronise ?\n</agent-message>".into(),
            ),
            &mut fx,
        );
        a.on_transcript(&peer("p1", handback("nobody", "Pas à moi.")), &mut fx);
        assert!(a.conv.get("a1").unwrap().get("report").is_none());
        assert_eq!(plan(&a).agents[0].status, RunStatus::Running);

        a.on_transcript(
            &peer(
                "p2",
                handback("bg1", "**Cause trouvée**\n\n- le filtre\n  - son détail"),
            ),
            &mut fx,
        );
        assert_eq!(
            a.conv.get("a1").unwrap()["report"],
            "**Cause trouvée**\n\n- le filtre\n  - son détail"
        );
        assert!(fx.save);
        // The hand-back is its end: in the interface's transcript, no notification of a
        // subagent's task is replayed (seen on Claude Code 2.1.283), only its hand-back.
        let row = plan(&a).agents[0].clone();
        assert_eq!(row.status, RunStatus::Done);
        assert_eq!(a.conv.get("a1").unwrap()["run"]["status"], "done");
        // It was told in the conversation too, as an event of the agent's.
        assert_eq!(a.conv.get("p2").unwrap()["source"], "agent");

        // A report the call has is not replaced by a later message.
        a.on_transcript(&peer("p3", handback("bg1", "Un autre.")), &mut fx);
        assert_eq!(
            a.conv.get("a1").unwrap()["report"],
            "**Cause trouvée**\n\n- le filtre\n  - son détail"
        );

        // Bounded as the real frame's.
        let mut b = rt();
        launched_in_the_transcript(&mut b);
        b.on_transcript(
            &peer("p4", handback("bg1", &"x".repeat(40_000))),
            &mut Effects::default(),
        );
        let kept = b.conv.get("a1").unwrap()["report"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(
            kept.len() < MAX_REPORT + 10 && kept.len() > MAX_REPORT - 10 && kept.ends_with('…')
        );
    }

    #[test]
    fn the_frames_of_a_real_subagent_stopped_by_stop_task_end_it_stopped_and_its_request_is_its_own(
    ) {
        let mut a = rt();
        replay(&mut a, REAL_STOP);
        let p = plan(&a);
        assert_eq!(
            p.agents.len(),
            1,
            "its own background command is no subagent"
        );
        let row = &p.agents[0];
        assert_eq!(row.title, "Slow sleeper");
        assert_eq!(row.status, RunStatus::Stopped);
        assert_eq!(row.task_id.as_deref(), Some("a3c004cb08b483eb9"));
        assert!(row.tokens.is_some_and(|t| t > 20_000));
        // Its permission request is its own, and it is still in the agent's conversation.
        let request = a
            .conv
            .items()
            .into_iter()
            .find(|i| i["kind"] == "permission")
            .expect("its permission request");
        assert_eq!(request["sub"], row.id.as_str());
        assert!(request["parent"].is_null());
        // Its conversation: the items of its thread.
        let items = a.conv.items();
        let thread: Vec<_> = items
            .iter()
            .filter(|i| i["parent"] == row.id.as_str())
            .collect();
        assert!(thread
            .iter()
            .any(|i| i["kind"] == "tool" && i["name"] == "Write"));
        assert!(thread
            .iter()
            .any(|i| i["kind"] == "tool" && i["name"] == "Bash"));
        let call = a.conv.get(&row.id).unwrap().clone();
        assert_eq!(call["run"]["status"], "stopped");
        assert!(call["input"]["prompt"]
            .as_str()
            .unwrap()
            .starts_with("First use the Write tool"));
    }

    /// What Claude Code 2.1.301 wrote on stdout in two real runs (2026-10-16, ESC-14, docs/PROTOCOL.md):
    /// a `Bash` command, a `Monitor` and a subagent in the background (one command stopped, with its
    /// own foreground command inside the subagent), then a command run in the background and another
    /// sent there from the foreground.
    const REAL_BG: &str = include_str!("../../tests/fixtures/real-run-background-tasks.jsonl");
    const REAL_FLIP: &str = include_str!("../../tests/fixtures/real-run-foreground-flip.jsonl");
    /// Claude Code 2.1.306 on stdout (ESC-138): a `CronCreate` one minute ahead, the turn ends,
    /// then the job fires and Claude Code starts a turn by itself (no user frame). Left out: the
    /// thinking blocks, the `system` frames other than the tasks', and the `command_lifecycle` and
    /// `tool_progress` frames, which Escouade does not read and the schema of the version it tests
    /// does not have.
    const REAL_CRON: &str = include_str!("../../tests/fixtures/real-run-cron.jsonl");

    /// Claude Code 2.1.309 on stdout (ESC-154): `/loop` sent, `ScheduleWakeup` at 60 s, the turn
    /// ends; the wake-up fires and Claude Code replays the command it runs again, as a message of
    /// no origin (a human's) with a uuid of its own: `<command-message>loop</command-message>
    /// <command-name>/loop</command-name><command-args>…`. The loop stops at its second turn.
    /// Left out: the `system` frames, `command_lifecycle` (`started` before the wake-up's turn) and the
    /// results' `model_usage_since_last_result`, which Escouade does not read and the schema of the
    /// version it tests does not have.
    const REAL_LOOP: &str = include_str!("../../tests/fixtures/real-run-loop-wakeup.jsonl");

    #[test]
    fn the_wake_up_of_a_loop_shows_as_a_wake_up_not_as_a_message_with_its_tags() {
        let mut a = rt();
        let mut fx = Effects::default();
        let first: Value = serde_json::from_str(REAL_LOOP.lines().next().unwrap()).unwrap();
        let sent = "/loop Réponds seulement « tic ». À la première itération, planifie la suivante avec ScheduleWakeup à 60 secondes ; à la deuxième, arrête la boucle (stop: true).";
        a.push_user(first["uuid"].as_str().unwrap(), sent, 0, &[], &mut fx);
        // The wake-up due now, as it was in the run.
        let run = REAL_LOOP.replace("1792743300000", &now_ms().to_string());
        replay(&mut a, &run);
        no_local_command_tag(&mut a);
        let users: Vec<_> = a
            .conv
            .items()
            .into_iter()
            .filter(|i| i["kind"] == "user")
            .map(|i| i["text"].clone())
            .collect();
        assert_eq!(users, [json!(sent)]);
        let wakes: Vec<_> = a
            .conv
            .items()
            .into_iter()
            .filter(|i| i["kind"] == "event")
            .collect();
        assert_eq!(wakes.len(), 1, "{wakes:?}");
        assert_eq!(wakes[0]["source"], "wake");
        assert_eq!(wakes[0]["text"], json!(sent));
        // Stopped at its second turn: no wake-up to come.
        assert!(a.meta.plan.as_ref().is_none_or(|p| p.wakes.is_empty()));
    }

    #[test]
    fn a_message_from_claude_ai_is_no_wake_up_once_it_fired_or_long_after_it() {
        // The first turn of the run: `ScheduleWakeup`, its wake-up at `at`, and the end of the turn.
        let first_turn = |at: i64| {
            let end = REAL_LOOP
                .lines()
                .position(|l| l.contains("\"type\":\"result\""))
                .unwrap();
            REAL_LOOP
                .lines()
                .take(end + 1)
                .collect::<Vec<_>>()
                .join("\n")
                .replace("1792743300000", &at.to_string())
        };
        let remote = |uuid: &str, text: &str| {
            json!({"type":"user","message":{"role":"user","content":text},"parent_tool_use_id":null,
                "uuid":uuid,"isReplay":true})
        };
        let kind = |a: &mut AgentRt, id: &str| a.conv.get(id).unwrap()["kind"].clone();
        let mut a = rt();
        let mut fx = Effects::default();
        replay(&mut a, &first_turn(now_ms()));
        a.handle_frame(
            &remote(
                "w1",
                "<command-name>/loop</command-name>\n<command-args>x</command-args>",
            ),
            &mut fx,
        );
        a.handle_frame(&remote("r1", "Ajoute les tests"), &mut fx);
        assert_eq!(kind(&mut a, "w1"), "event");
        assert_eq!(kind(&mut a, "r1"), "user");
        // A loop left without its stop two hours ago.
        let mut a = rt();
        replay(&mut a, &first_turn(now_ms() - 2 * 3_600_000));
        a.handle_frame(&remote("r2", "Ajoute les tests"), &mut fx);
        assert_eq!(kind(&mut a, "r2"), "user");
    }

    #[test]
    fn the_frames_of_a_real_run_with_a_job_of_the_session_keep_its_wake_up() {
        let mut a = rt();
        replay(&mut a, REAL_CRON);
        let p = plan(&a);
        assert_eq!(p.wakes.len(), 1, "{:?}", p.wakes);
        let w = &p.wakes[0];
        assert_eq!(w.id, "9efa53bd");
        assert_eq!(w.at, 1_792_649_820_000);
        assert!(!w.recurring && !w.durable);
        assert_eq!(w.schedule.as_deref(), Some("17 8 22 10 *"));
        assert_eq!(w.prompt.as_deref(), Some("Dis juste: réveil"));
        // Before its time it holds the process; after, it fired.
        assert!(p.holds_process(w.at - 1));
        assert!(!p.holds_process(w.at + 1));
        // Its own `date` ran in the foreground: no background task.
        assert!(p.bg_tasks.is_empty());
    }

    fn bg_task(p: &PlanState, title: &str) -> crate::plan::BgTask {
        p.bg_tasks
            .iter()
            .find(|t| t.title == title)
            .unwrap_or_else(|| panic!("no task titled {title}: {:?}", p.bg_tasks))
            .clone()
    }

    #[test]
    fn the_frames_of_a_real_run_with_commands_a_monitor_and_a_subagent_list_the_background_tasks() {
        use crate::plan::BgKind;
        let mut a = rt();
        let all = replay(&mut a, REAL_BG);
        let p = plan(&a);
        // The three tasks the agent launched, in order; neither the subagent nor its own command.
        let titles: Vec<_> = p.bg_tasks.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(titles, ["Ticker", "Sleeper", "Watch loop output"]);
        assert_eq!(p.agents.len(), 1);
        assert_eq!(p.agents[0].status, RunStatus::Done);
        assert!(p.bg_tasks.iter().all(|t| t.parent.is_none()));
        let ticker = bg_task(&p, "Ticker");
        assert_eq!(
            (ticker.kind, ticker.status),
            (BgKind::Command, RunStatus::Done)
        );
        assert_eq!(ticker.id, "b695j2b8o");
        assert!(ticker
            .summary
            .as_deref()
            .is_some_and(|s| s.contains("completed (exit code 0)")));
        assert!(ticker
            .command
            .as_deref()
            .is_some_and(|c| c.starts_with("for i in $(seq 1 10)")));
        assert_eq!(
            ticker.call.as_deref(),
            Some("toolu_01CNQVecQtd4JU2VZKXGEP4h")
        );
        assert!(ticker
            .output_file
            .as_deref()
            .is_some_and(|f| f.ends_with("tasks\\b695j2b8o.output")));
        // Stopped by the agent: its notification says only its description.
        let sleeper = bg_task(&p, "Sleeper");
        assert_eq!(
            (sleeper.kind, sleeper.status),
            (BgKind::Command, RunStatus::Stopped)
        );
        assert_eq!(sleeper.summary.as_deref(), Some("Sleeper"));
        assert_eq!(sleeper.command.as_deref(), Some("sleep 300"));
        assert!(sleeper
            .output_file
            .as_deref()
            .is_some_and(|f| f.ends_with("tasks\\bd0kd12fp.output")));
        // A `Monitor` is a `local_bash` task: only its call tells it.
        let watch = bg_task(&p, "Watch loop output");
        assert_eq!(
            (watch.kind, watch.status),
            (BgKind::Monitor, RunStatus::Done)
        );
        assert_eq!(watch.id, "bpy7gazpv");
        assert_eq!(
            watch.summary.as_deref(),
            Some("Monitor \"Watch loop output\" stream ended")
        );
        assert!(watch
            .command
            .as_deref()
            .is_some_and(|c| c.starts_with("for i in $(seq 1 6)")));
        assert!(watch
            .output_file
            .as_deref()
            .is_some_and(|f| f.ends_with("tasks\\bpy7gazpv.output")));
        assert!(p.bg_tasks.iter().all(|t| t.ended_at.is_some()));
        assert!(all.save && all.agent_changed);
        // Partway: a command and a subagent run, the monitor and the stopped one are not yet.
        let lines: Vec<&str> = REAL_BG.lines().collect();
        let mut b = rt();
        replay(&mut b, &lines[..26].join("\n"));
        let p = plan(&b);
        assert_eq!(p.bg_tasks.len(), 3);
        assert!(p.bg_tasks.iter().all(|t| t.status == RunStatus::Running));
        assert!(p.runs_something());
        // Its process gone, they are interrupted, with the subagent.
        b.detach();
        assert!(plan(&b)
            .bg_tasks
            .iter()
            .all(|t| t.status == RunStatus::Interrupted));
    }

    #[test]
    fn the_frames_of_a_real_run_where_a_foreground_command_goes_to_the_background_list_it_from_then(
    ) {
        use crate::plan::BgKind;
        let lines: Vec<&str> = REAL_FLIP.lines().collect();
        // In the foreground (its `task_started`, no level that has it): only the other is listed.
        let mut a = rt();
        replay(&mut a, &lines[..=10].join("\n"));
        let p = plan(&a);
        assert_eq!(p.bg_tasks.len(), 1);
        assert_eq!(p.bg_tasks[0].title, "Loud");
        // Sent to the background: listed, running, with the file its result tells.
        replay(&mut a, &lines[11..=13].join("\n"));
        let p = plan(&a);
        let titles: Vec<_> = p.bg_tasks.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(titles, ["Loud", "Foreground loop"]);
        let flipped = bg_task(&p, "Foreground loop");
        assert_eq!(
            (flipped.kind, flipped.status),
            (BgKind::Command, RunStatus::Running)
        );
        assert!(flipped
            .output_file
            .as_deref()
            .is_some_and(|f| f.ends_with("tasks\\bj6es8xxy.output")));
        // Both end, with their words.
        replay(&mut a, &lines[14..].join("\n"));
        let p = plan(&a);
        assert_eq!(p.bg_tasks.len(), 2);
        for (title, id) in [("Loud", "bdv6oe7xp"), ("Foreground loop", "bj6es8xxy")] {
            let t = bg_task(&p, title);
            assert_eq!((t.id.as_str(), t.status), (id, RunStatus::Done), "{title}");
            assert!(t
                .summary
                .as_deref()
                .is_some_and(|s| s.contains("(exit code 0)")));
            assert!(t
                .output_file
                .as_deref()
                .is_some_and(|f| f.ends_with(&format!("tasks\\{id}.output"))));
            assert!(t.ended_at.is_some());
        }
        assert_eq!(
            bg_task(&p, "Foreground loop").command.as_deref(),
            Some("for i in $(seq 1 12); do echo fg $i; sleep 2; done; echo fgdone")
        );
        assert!(p.agents.is_empty() && p.launched == 0);
    }

    #[test]
    fn a_plan_with_nothing_to_show_still_remembers_the_command_a_task_starts_with() {
        let mut a = rt();
        call(
            &mut a,
            None,
            "c1",
            "Bash",
            json!({"command":"npm run dev","description":"Dev server","run_in_background":true}),
        );
        // A call is no change: no plan yet.
        assert!(a.view().meta.plan.is_none());
        system(
            &mut a,
            json!({"type":"system","subtype":"task_started","task_id":"b1","tool_use_id":"c1",
                "description":"Dev server","is_backgrounded":true,"task_type":"local_bash"}),
        );
        let t = plan(&a).bg_tasks[0].clone();
        assert_eq!(
            (t.title.as_str(), t.command.as_deref()),
            ("Dev server", Some("npm run dev"))
        );
        // A level alone makes a row too, and a plan that was reset starts remembering again.
        let mut b = rt();
        system(
            &mut b,
            json!({"type":"system","subtype":"background_tasks_changed","tasks":[
                {"task_id":"b2","task_type":"local_bash","description":"Watch"}]}),
        );
        assert_eq!(plan(&b).bg_tasks[0].id, "b2");
        session(&mut b, "s2");
        assert!(b.view().meta.plan.is_none());
        call(
            &mut b,
            None,
            "c2",
            "Bash",
            json!({"command":"ls","description":"List"}),
        );
        system(
            &mut b,
            json!({"type":"system","subtype":"task_started","task_id":"b3","tool_use_id":"c2",
                "is_backgrounded":true,"task_type":"local_bash"}),
        );
        assert_eq!(plan(&b).bg_tasks[0].command.as_deref(), Some("ls"));
    }

    #[test]
    fn in_the_claude_code_view_a_command_starts_with_its_result_and_ends_with_its_replayed_notification(
    ) {
        let mut a = rt();
        let mut fx = Effects::default();
        a.on_transcript(
            &json!({"type":"assistant","uuid":"e1","message":{"id":"m1","content":[{"type":"tool_use","id":"c1","name":"Bash",
                "input":{"command":"npm run dev","description":"Dev server","run_in_background":true}}]}}),
            &mut fx,
        );
        assert!(a.view().meta.plan.is_none());
        let file = "C:\\Users\\dev\\AppData\\Local\\Temp\\claude\\p\\s\\tasks\\b9.output";
        a.on_transcript(
            &json!({"type":"user","uuid":"e2","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"c1",
                "content":format!("Command running in background with ID: b9. Output is being written to: {file}. You will be notified when it completes.")}]},
                "toolUseResult":{"stdout":"","stderr":"","backgroundTaskId":"b9"}}),
            &mut fx,
        );
        let t = plan(&a).bg_tasks[0].clone();
        assert_eq!(
            (
                t.id.as_str(),
                t.title.as_str(),
                t.status,
                t.command.as_deref()
            ),
            ("b9", "Dev server", RunStatus::Running, Some("npm run dev"))
        );
        assert_eq!(t.output_file.as_deref(), Some(file));
        // The events of a monitor are replayed without a status: nothing ends.
        let mut fx = Effects::default();
        a.on_transcript(
            &json!({"type":"user","uuid":"n0","origin":{"kind":"task-notification"},
                "message":{"role":"user","content":"<task-notification><task-id>b9</task-id><summary>Monitor event: \"watch 1\"</summary><event>watch 1</event></task-notification>"}}),
            &mut fx,
        );
        assert_eq!(plan(&a).bg_tasks[0].status, RunStatus::Running);
        assert!(plan(&a).bg_tasks[0].summary.is_none());
        assert_eq!(a.conv.get("n0").unwrap()["source"], "task");
        // Its end is the notification, with its words (escaped as Claude Code writes them) and its file.
        let mut fx = Effects::default();
        a.on_transcript(
            &json!({"type":"user","uuid":"n1","origin":{"kind":"task-notification"},
                "message":{"role":"user","content":format!("<task-notification>\n<task-id>b9</task-id>\n<tool-use-id>c1</tool-use-id>\n<output-file>{file}</output-file>\n<status>failed</status>\n<summary>Background command \"a &amp;&amp; b\" failed with exit code 3</summary>\n</task-notification>")}}),
            &mut fx,
        );
        let t = plan(&a).bg_tasks[0].clone();
        assert_eq!(t.status, RunStatus::Failed);
        assert_eq!(
            t.summary.as_deref(),
            Some("Background command \"a && b\" failed with exit code 3")
        );
        assert_eq!(t.output_file.as_deref(), Some(file));
        assert!(t.ended_at.is_some() && fx.save && fx.agent_changed);
    }

    #[test]
    fn a_background_task_does_not_end_with_its_turn_but_with_its_process() {
        let mut a = rt();
        system(
            &mut a,
            json!({"type":"system","subtype":"task_started","task_id":"b1","tool_use_id":"c1",
                "description":"Serve","is_backgrounded":true,"task_type":"local_bash"}),
        );
        assert!(plan(&a).runs_something() && !plan(&a).is_finished());
        // The turn that ends does not end it; the process that dies does.
        turn_ends(&mut a);
        assert_eq!(plan(&a).bg_tasks[0].status, RunStatus::Running);
        a.detach();
        assert_eq!(plan(&a).bg_tasks[0].status, RunStatus::Interrupted);
        assert!(plan(&a).is_finished());
    }

    #[test]
    fn a_workflow_is_followed_from_the_system_frames() {
        let mut a = rt();
        system(
            &mut a,
            json!({"type":"system","subtype":"task_started","task_id":"w1","tool_use_id":"tw",
                "description":"Revue","task_type":"local_workflow","workflow_name":"review"}),
        );
        let fx = system(
            &mut a,
            json!({"type":"system","subtype":"task_progress","task_id":"w1","tool_use_id":"tw",
                "description":"Phase 1 : lecteur A","usage":{"total_tokens":50,"tool_uses":2,"duration_ms":1}}),
        );
        assert!(fx.agent_changed && !fx.save);
        let p = plan(&a);
        assert_eq!(p.workflows[0].now.as_deref(), Some("Phase 1 : lecteur A"));
        assert_eq!(p.workflows[0].status, RunStatus::Running);
        // A background command is not a workflow, nor a subagent.
        system(
            &mut a,
            json!({"type":"system","subtype":"task_started","task_id":"b1","tool_use_id":"tb",
                "description":"npm test","task_type":"local_bash","is_backgrounded":true}),
        );
        assert_eq!(plan(&a).workflows.len(), 1);
        assert!(plan(&a).agents.is_empty());
    }

    #[test]
    fn a_new_conversation_starts_a_plan_of_its_own() {
        let mut a = rt();
        session(&mut a, "s1");
        creates(&mut a, "c0", "Lire", 1);
        // The same conversation, a new process: the plan goes on.
        session(&mut a, "s1");
        assert_eq!(plan(&a).tasks.len(), 1);
        // « /clear »: another session, nothing of the plan is in the context.
        let fx = session(&mut a, "s2");
        assert!(a.view().meta.plan.is_none());
        assert!(fx.save && fx.agent_changed);
    }

    #[test]
    fn a_message_starts_a_new_plan_only_after_a_plan_that_is_over() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.push_user("h1", "Fais ceci", 0, &[], &mut fx);
        creates(&mut a, "c0", "Lire", 1);
        turn_ends(&mut a);
        // The plan is under way: a message goes on with it.
        a.push_user("h2", "Continue", 0, &[], &mut fx);
        assert_eq!(plan(&a).tasks.len(), 1);
        call(
            &mut a,
            None,
            "u1",
            "TaskUpdate",
            json!({"taskId":"1","status":"completed"}),
        );
        // A message queued behind the running turn is not the start of one.
        assert!(a.push_user("h3", "Et aussi", 0, &[], &mut fx));
        assert_eq!(plan(&a).tasks.len(), 1);
        turn_ends(&mut a);
        // The plan is over, and so is the turn: the next message starts another.
        let mut fx = Effects::default();
        a.push_user("h4", "Autre chose", 0, &[], &mut fx);
        assert!(a.view().meta.plan.is_none());
        assert!(fx.save);
        // No plan at all: nothing to reset, nothing made.
        turn_ends(&mut a);
        a.push_user("h5", "Encore", 0, &[], &mut Effects::default());
        assert!(a.view().meta.plan.is_none());
    }

    #[test]
    fn a_background_subagent_still_running_keeps_the_plan_across_a_message() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.push_user("h1", "Fais ceci", 0, &[], &mut fx);
        call(
            &mut a,
            None,
            "a1",
            "Agent",
            json!({"description":"Tests","prompt":"","run_in_background":true}),
        );
        turn_ends(&mut a);
        a.push_user("h2", "Où en est-il ?", 0, &[], &mut fx);
        assert_eq!(plan(&a).agents[0].status, RunStatus::Running);
    }

    #[test]
    fn the_end_of_a_turn_ends_the_subagents_of_the_foreground_only() {
        let mut a = rt();
        call(
            &mut a,
            None,
            "a1",
            "Agent",
            json!({"description":"Avant","prompt":""}),
        );
        call(
            &mut a,
            None,
            "a2",
            "Agent",
            json!({"description":"Fond","prompt":"","run_in_background":true}),
        );
        turn_ends(&mut a);
        let p = plan(&a);
        assert_eq!(p.agents[0].status, RunStatus::Interrupted);
        assert_eq!(p.agents[1].status, RunStatus::Running);
    }

    #[test]
    fn what_ran_in_a_process_that_is_gone_is_interrupted() {
        let busy = |a: &mut AgentRt| {
            creates(a, "c0", "Lire", 1);
            call(
                a,
                None,
                "u1",
                "TaskUpdate",
                json!({"taskId":"1","status":"in_progress"}),
            );
            call(
                a,
                None,
                "a1",
                "Agent",
                json!({"description":"Fond","prompt":"","run_in_background":true}),
            );
        };
        let interrupted = |a: &AgentRt| {
            let p = plan(a);
            assert_eq!(p.agents[0].status, RunStatus::Interrupted);
            // The tasks keep their status: the agent picks them up again.
            assert_eq!(p.tasks[0].status, TaskStatus::InProgress);
        };
        // Stopped on purpose.
        let mut a = rt();
        busy(&mut a);
        a.detach();
        interrupted(&a);
        // The process died.
        let mut a = rt();
        busy(&mut a);
        let mut fx = Effects::default();
        a.on_exit(a.gen, Some(1), "", &mut fx);
        interrupted(&a);
        assert!(fx.save && fx.agent_changed);
        // The app was closed: the plan saved with it is loaded interrupted.
        let mut a = rt();
        busy(&mut a);
        let b = AgentRt::new(
            a.meta.clone(),
            &std::env::temp_dir().join("ccm-agent-tests-plan"),
        );
        interrupted(&b);
        assert_eq!(
            b.view().meta.plan.unwrap().agents[0].status,
            RunStatus::Interrupted
        );
    }

    // ---------- the plan of the folder ----------

    /// The result of the call `id`, in the thread of `parent`.
    fn returns_in(a: &mut AgentRt, parent: Option<&str>, id: &str) -> Effects {
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":id,"content":"ok"}]},"tool_use_result":null,"parent_tool_use_id":parent}),
            &mut fx,
        );
        fx
    }

    fn turn_result(a: &mut AgentRt) -> Effects {
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1}),
            &mut fx,
        );
        fx
    }

    #[test]
    fn a_command_a_subagent_or_the_end_of_a_turn_calls_for_a_look_at_the_plans_files() {
        let mut a = rt();
        // The call itself does not: what it did is known with its result.
        assert!(!call(&mut a, None, "b1", "Bash", json!({"command":"task-done"})).plan_files);
        assert!(returns(&mut a, "b1", "ledger: Task 1: complete", Value::Null).plan_files);
        // Once: the same result again says nothing new.
        assert!(!returns(&mut a, "b1", "again", Value::Null).plan_files);
        for (id, name) in [("s1", "Agent"), ("s2", "Task")] {
            call(
                &mut a,
                None,
                id,
                name,
                json!({"description":"Aide","prompt":"x"}),
            );
            assert!(
                returns(&mut a, id, "done", Value::Null).plan_files,
                "{name}"
            );
        }
        // Reading, searching, editing source: not what the files of a plan move with.
        for (id, name, input) in [
            ("r1", "Read", json!({"file_path":"C:/p/src/a.ts"})),
            ("g1", "Grep", json!({"pattern":"x"})),
            ("e1", "Edit", json!({"file_path":"C:/p/src/a.ts"})),
            ("w1", "Write", json!({"file_path":"C:/p/docs/notes.md"})),
        ] {
            call(&mut a, None, id, name, input);
            assert!(!returns(&mut a, id, "ok", Value::Null).plan_files, "{name}");
        }
        // A command of a subagent is its own: the ledger is the main thread's.
        call(
            &mut a,
            None,
            "s3",
            "Agent",
            json!({"description":"Aide","prompt":"x"}),
        );
        call(
            &mut a,
            Some("s3"),
            "k1",
            "Bash",
            json!({"command":"npm test"}),
        );
        assert!(!returns_in(&mut a, Some("s3"), "k1").plan_files);
        // The end of the turn.
        assert!(turn_result(&mut a).plan_files);
    }

    #[test]
    fn a_plan_the_agent_writes_is_remembered_for_the_look_and_its_write_calls_for_one() {
        let mut a = rt();
        let abs = "C:/p/docs/superpowers/plans/2026-10-10-demo.md";
        call(&mut a, None, "w1", "Write", json!({"file_path":abs}));
        assert_eq!(a.plan_written, [abs]);
        assert!(returns(&mut a, "w1", "ok", Value::Null).plan_files);
        // Relative to its folder, with either slash, edited or written: as an absolute path.
        call(
            &mut a,
            None,
            "w2",
            "Edit",
            json!({"file_path":"docs/superpowers/plans/other.md"}),
        );
        call(
            &mut a,
            None,
            "w3",
            "MultiEdit",
            json!({"file_path":"C:\\p\\docs\\superpowers\\plans\\third.md"}),
        );
        assert_eq!(
            a.plan_written,
            [
                abs,
                "C:/p/docs/superpowers/plans/other.md",
                "C:\\p\\docs\\superpowers\\plans\\third.md"
            ]
        );
        // Written again: it is the latest, once.
        call(&mut a, None, "w4", "Write", json!({"file_path":abs}));
        assert_eq!(a.plan_written.last().map(String::as_str), Some(abs));
        assert_eq!(a.plan_written.len(), 3);
        // What is no plan of the plans folder, a subagent's writes, a Write without a path.
        for (parent, input) in [
            (None, json!({"file_path":"C:/p/src/a.ts"})),
            (
                None,
                json!({"file_path":"C:/p/docs/superpowers/specs/s.md"}),
            ),
            (
                None,
                json!({"file_path":"C:/p/docs/superpowers/plans/notes.txt"}),
            ),
            (None, json!({"content":"x"})),
            (
                Some("s1"),
                json!({"file_path":"C:/p/docs/superpowers/plans/sub.md"}),
            ),
        ] {
            call(&mut a, parent, "wx", "Write", input);
        }
        assert_eq!(a.plan_written.len(), 3);
        // Five at most: the oldest go.
        for i in 0..4 {
            call(
                &mut a,
                None,
                &format!("n{i}"),
                "Write",
                json!({"file_path": format!("C:/p/docs/superpowers/plans/n{i}.md")}),
            );
        }
        assert_eq!(a.plan_written.len(), 5);
        assert!(!a.plan_written.iter().any(|p| p.ends_with("/demo.md")));
        assert!(a.plan_written[4].ends_with("/n3.md"));
    }

    /// Every frame of a recording, in order, through the agent: what they asked for, all told.
    fn replay(a: &mut AgentRt, recording: &str) -> Effects {
        let mut all = Effects::default();
        for line in recording.lines().filter(|l| !l.trim().is_empty()) {
            let frame: Value = serde_json::from_str(line).expect("a frame");
            let mut fx = Effects::default();
            a.handle_frame(&frame, &mut fx);
            all.plan_files |= fx.plan_files;
            all.save |= fx.save;
            all.agent_changed |= fx.agent_changed;
        }
        all
    }

    /// What Claude Code 2.1.296 wrote on stdout in two real runs of superpowers 6.4.1 (see
    /// tests/fixtures/superpowers-run.md): the frames the plan is made of, as they came.
    const REAL_SUBAGENTS: &str = include_str!("../../tests/fixtures/real-run-subagents.jsonl");
    const REAL_INLINE: &str = include_str!("../../tests/fixtures/real-run-inline.jsonl");

    #[test]
    fn the_frames_of_a_real_run_with_subagents_in_the_background_and_task_tools_make_the_plan() {
        let mut a = rt();
        // The recording is two stretches: the run of the plan, then a message of the user's that
        // asks for the task tools (a plan that ended starts over with that message).
        let lines: Vec<&str> = REAL_SUBAGENTS.lines().collect();
        let asked = lines
            .iter()
            .position(|l| l.contains("sans rien faire d’autre"))
            .expect("the message that asks for the task tools");
        let all = replay(&mut a, &lines[..asked].join("\n"));
        let p = plan(&a);
        // Two implementers, launched without `run_in_background` and run in the background all the
        // same (`async_launched`), ended by their notification; a foreground `Bash` (a
        // `local_bash` task) is no subagent.
        assert_eq!(p.launched, 2);
        let titles: Vec<_> = p.agents.iter().map(|r| r.title.as_str()).collect();
        assert_eq!(
            titles,
            ["Implement Task 1 slugify", "Implement Task 2 slugify tests"]
        );
        for row in &p.agents {
            assert_eq!(row.status, RunStatus::Done, "{}", row.title);
            assert!(row.background, "{}", row.title);
            // Asked for as « haiku »: the model its messages came from.
            assert_eq!(row.model.as_deref(), Some("claude-haiku-5-5"));
            assert_eq!(row.kind.as_deref(), Some("general-purpose"));
            // What it did, told at its end: the tools it used and its tokens.
            assert_eq!(row.tools, 5, "{}", row.title);
            assert!(row.tokens.is_some_and(|t| t > 20_000), "{}", row.title);
            assert!(row.ended_at.is_some() && row.doing.is_none());
        }
        // The commands and subagents of the main thread called for a look at the files.
        assert!(all.plan_files && all.save && all.agent_changed);
        assert!(p.workflows.is_empty());
        assert!(p.tasks.is_empty(), "the agent kept no list");

        // The task tools it is then asked for: `TaskCreate` ×2, `TaskUpdate` ×3 (after a
        // `ToolSearch`, which loads them).
        replay(&mut a, &lines[asked..].join("\n"));
        let p = plan(&a);
        assert_eq!(p.source, Some(crate::plan::PlanSource::Tools));
        let tasks: Vec<_> = p
            .tasks
            .iter()
            .map(|t| (t.id.as_str(), t.title.as_str(), t.status))
            .collect();
        assert_eq!(
            tasks,
            [
                ("1", "Alpha", TaskStatus::Done),
                ("2", "Beta", TaskStatus::InProgress)
            ]
        );
        // What the window gets is the plan as it is.
        let sent = serde_json::to_value(a.view()).unwrap();
        assert_eq!(sent["plan"]["tasks"][0]["status"], "done");
        assert_eq!(sent["plan"]["tasks"][1]["status"], "inProgress");
    }

    #[test]
    fn the_frames_of_a_real_inline_run_say_where_its_plan_is_and_how_it_ends() {
        let mut a = rt();
        // Its folder, as the recording spells it, and its conversation: the recording goes on with
        // it (the first conversation of an agent starts its plan over).
        a.meta.cwd =
            r"C:\Users\dev\AppData\Local\Temp\p4-trial\repo\.claude\worktrees\agent-1".into();
        let init: Value = serde_json::from_str(REAL_INLINE.lines().next().unwrap()).unwrap();
        a.meta.session_id = init["session_id"].as_str().map(str::to_string);
        // The plan was read from its files by the look the first command called for.
        let file = "docs/superpowers/plans/2026-10-11-slugify.md";
        let list = FileList {
            plan_file: file.into(),
            title: Some("slugify".into()),
            tasks: vec![
                crate::plan::PlanTask {
                    id: "1".into(),
                    title: "Fonction slugify".into(),
                    ..Default::default()
                },
                crate::plan::PlanTask {
                    id: "2".into(),
                    title: "Test de slugify".into(),
                    status: TaskStatus::InProgress,
                    ..Default::default()
                },
            ],
        };
        a.plan_from_files(a.plan_since, Some(list), &mut Effects::default());
        let all = replay(&mut a, REAL_INLINE);
        // The plan it wrote with the Write tool is the one to look for, as the tool spelled it.
        assert_eq!(a.plan_written.len(), 1);
        assert!(a.plan_written[0].ends_with("2026-10-11-slugify.md"));
        assert!(all.plan_files);
        // The run ended as the skills end one: the workspace removed after the last task. Nothing
        // else of the plan was said, and every task is done.
        let p = plan(&a);
        assert_eq!(p.source, Some(crate::plan::PlanSource::Plan));
        assert!(p.tasks.iter().all(|t| t.status == TaskStatus::Done));
        assert_eq!(p.launched, 0);
        // No task tool in the list the agent was given (the setting was off): none was called.
        assert!(!REAL_INLINE.contains("\"TaskCreate\""));
    }

    #[test]
    fn a_plan_a_command_writes_is_remembered_like_one_a_write_makes_and_one_it_only_reads_is_not() {
        // How a real run wrote its plan: a command, with the plan in its text (seen on two of two).
        let mut a = rt();
        let command = "mkdir -p docs/superpowers/plans && cat > docs/superpowers/plans/2026-10-11-slugify.md <<'EOF'
# slugify Implementation Plan

### Task 1: Un
EOF
git add docs/superpowers/plans/2026-10-11-slugify.md && git commit -qm plan";
        call(&mut a, None, "b1", "Bash", json!({ "command": command }));
        assert_eq!(
            a.plan_written,
            ["C:/p/docs/superpowers/plans/2026-10-11-slugify.md"]
        );
        assert!(returns(&mut a, "b1", "ok", Value::Null).plan_files);
        // Every way a command writes a file: a redirection (appending, on a descriptor, with no
        // blank), `tee`, the destination of a copy or a move; spelled with backslashes, in quotes,
        // or absolute, as a write would be.
        let writes = [
            (
                r#"copy x "docs\superpowers\plans\other.md""#,
                r"C:/p/docs\superpowers\plans\other.md",
            ),
            (
                "tee -a C:/p/docs/superpowers/plans/third.md",
                "C:/p/docs/superpowers/plans/third.md",
            ),
            (
                "echo x >> docs/superpowers/plans/fourth.md",
                "C:/p/docs/superpowers/plans/fourth.md",
            ),
            (
                "echo x >docs/superpowers/plans/fifth.md",
                "C:/p/docs/superpowers/plans/fifth.md",
            ),
            (
                "make 2> docs/superpowers/plans/sixth.md",
                "C:/p/docs/superpowers/plans/sixth.md",
            ),
            (
                "cp /tmp/x.md docs/superpowers/plans/seventh.md",
                "C:/p/docs/superpowers/plans/seventh.md",
            ),
            (
                "mv -f x.md 'docs/superpowers/plans/eighth.md'",
                "C:/p/docs/superpowers/plans/eighth.md",
            ),
        ];
        for (i, (command, plan)) in writes.iter().enumerate() {
            let mut b = rt();
            call(
                &mut b,
                None,
                &format!("w{i}"),
                "Bash",
                json!({ "command": command }),
            );
            assert_eq!(b.plan_written, [*plan], "{command}");
        }
        // Named but not written: read, searched, counted, logged, added, copied from, or the
        // source of a redirection; a pattern, a variable, a folder, another kind of file, a
        // command of a subagent.
        let before = a.plan_written.clone();
        for (parent, command) in [
            (None, "cat docs/superpowers/plans/old.md"),
            (None, "grep -n Task docs/superpowers/plans/old.md | head"),
            (None, "git log --oneline -- docs/superpowers/plans/old.md"),
            (None, "git add docs/superpowers/plans/old.md"),
            (None, "wc -l docs/superpowers/plans/old.md"),
            (None, "cp docs/superpowers/plans/old.md /tmp/copy.md"),
            (None, "head -5 docs/superpowers/plans/old.md > /tmp/x.txt"),
            (None, "sort < docs/superpowers/plans/old.md"),
            (None, "ls docs/superpowers/plans/*.md"),
            (None, "cat > docs/superpowers/plans/$NAME.md"),
            (None, "cat > docs/superpowers/plans/"),
            (None, "cat > docs/superpowers/specs/s.md"),
            (None, "cat > docs/superpowers/plans/n.txt"),
            (Some("s1"), "cat > docs/superpowers/plans/sub.md"),
        ] {
            call(&mut a, parent, "bx", "Bash", json!({ "command": command }));
        }
        assert_eq!(a.plan_written, before);
    }

    #[test]
    fn a_new_conversation_forgets_the_plans_it_wrote_and_dates_what_it_will_read() {
        let mut a = rt();
        let born = a.plan_since;
        assert!(born > now_ms() - 60_000 && born <= now_ms());
        session(&mut a, "s1");
        call(
            &mut a,
            None,
            "w1",
            "Write",
            json!({"file_path":"C:/p/docs/superpowers/plans/a.md"}),
        );
        call(&mut a, None, "b1", "Bash", json!({"command":"ls"}));
        assert_eq!(a.plan_written.len(), 1);
        std::thread::sleep(std::time::Duration::from_millis(5));
        // The same conversation again (the process restarted): nothing changes.
        session(&mut a, "s1");
        assert_eq!(a.plan_written.len(), 1);
        // Another one.
        session(&mut a, "s2");
        assert!(a.plan_written.is_empty());
        assert!(a.plan_since > born);
        // A result of a call of the one before does not look at the files of this one.
        assert!(!returns(&mut a, "b1", "ok", Value::Null).plan_files);
    }

    #[test]
    fn a_message_after_a_finished_plan_forgets_what_it_wrote_and_dates_what_it_reads() {
        let mut a = rt();
        a.push_user("h1", "Fais ceci", 0, &[], &mut Effects::default());
        creates(&mut a, "c0", "Lire", 1);
        call(
            &mut a,
            None,
            "w1",
            "Write",
            json!({"file_path":"C:/p/docs/superpowers/plans/a.md"}),
        );
        call(
            &mut a,
            None,
            "u1",
            "TaskUpdate",
            json!({"taskId":"1","status":"completed"}),
        );
        turn_ends(&mut a);
        // A plan under way keeps what it has: here the plan is over.
        let before = a.plan_since;
        std::thread::sleep(std::time::Duration::from_millis(5));
        a.push_user("h2", "Autre chose", 0, &[], &mut Effects::default());
        assert!(a.plan_written.is_empty());
        assert!(a.plan_since > before);

        let mut b = rt();
        b.push_user("h1", "Fais ceci", 0, &[], &mut Effects::default());
        creates(&mut b, "c0", "Lire", 1);
        call(
            &mut b,
            None,
            "w1",
            "Write",
            json!({"file_path":"C:/p/docs/superpowers/plans/a.md"}),
        );
        turn_ends(&mut b);
        b.push_user("h2", "Continue", 0, &[], &mut Effects::default());
        assert_eq!(b.plan_written.len(), 1);
    }

    fn demo_list() -> Option<FileList> {
        Some(FileList {
            plan_file: "docs/superpowers/plans/demo.md".into(),
            title: Some("Démo".into()),
            tasks: vec![crate::plan::PlanTask {
                id: "1".into(),
                title: "Un".into(),
                status: TaskStatus::InProgress,
                ..Default::default()
            }],
        })
    }

    #[test]
    fn the_files_read_come_in_if_the_conversation_is_still_the_one_they_were_read_for() {
        let mut a = rt();
        session(&mut a, "s1");
        let since = a.plan_since;
        let mut fx = Effects::default();
        assert!(a.plan_from_files(since, demo_list(), &mut fx));
        assert!(fx.agent_changed && fx.save);
        let p = plan(&a);
        assert_eq!(p.tasks[0].status, TaskStatus::InProgress);
        assert_eq!(p.title.as_deref(), Some("Démo"));
        assert_eq!(
            serde_json::to_value(a.view()).unwrap()["plan"]["planFile"],
            "docs/superpowers/plans/demo.md"
        );
        // Again: nothing to tell.
        let mut fx = Effects::default();
        assert!(a.plan_from_files(since, demo_list(), &mut fx));
        assert!(!fx.agent_changed && !fx.save);
        // Read for a conversation that has ended since: not applied.
        std::thread::sleep(std::time::Duration::from_millis(5));
        session(&mut a, "s2");
        assert!(a.view().meta.plan.is_none());
        let mut fx = Effects::default();
        assert!(!a.plan_from_files(since, demo_list(), &mut fx));
        assert!(a.view().meta.plan.is_none());
        assert!(!fx.agent_changed && !fx.save);
        // Nothing read: nothing done.
        assert!(a.plan_from_files(a.plan_since, None, &mut Effects::default()));
        assert!(a.view().meta.plan.is_none());
    }

    #[test]
    fn the_tasks_of_the_files_give_way_to_the_ones_the_agent_lists() {
        let mut a = rt();
        let since = a.plan_since;
        a.plan_from_files(since, demo_list(), &mut Effects::default());
        assert_eq!(plan(&a).tasks[0].title, "Un");
        creates(&mut a, "c0", "Task 1: Un", 1);
        let p = plan(&a);
        assert_eq!(p.tasks.len(), 1);
        assert_eq!(p.tasks[0].title, "Task 1: Un");
        assert_eq!(p.source, Some(crate::plan::PlanSource::Tools));
        // The files go on being read: its list is the agent's, named and titled by the plan its
        // tasks are the tasks of.
        a.plan_from_files(since, demo_list(), &mut Effects::default());
        assert_eq!(plan(&a).tasks[0].title, "Task 1: Un");
        assert_eq!(plan(&a).title.as_deref(), Some("Démo"));

        // Another agent in the same repository, with tasks of its own: the plan is not its plan.
        let mut b = rt();
        creates(&mut b, "c0", "Ma tâche", 1);
        b.plan_from_files(b.plan_since, demo_list(), &mut Effects::default());
        assert_eq!(plan(&b).tasks[0].title, "Ma tâche");
        assert_eq!(plan(&b).title, None);
        assert_eq!(plan(&b).plan_file, None);
    }

    #[test]
    fn the_plans_name_and_title_come_back_with_the_agents_list_without_the_files_being_read_again()
    {
        let mut a = rt();
        let since = a.plan_since;
        a.plan_from_files(since, demo_list(), &mut Effects::default());
        // The agent starts a list of its own, made of the tasks of that plan: the tasks of the
        // files make room, and so do the plan's name and title... until the files are read, which
        // they are not while they stay as they were (the look finds them unchanged and hands
        // nothing over). The name and title are there all the same.
        creates(&mut a, "c0", "Task 1: Un", 1);
        let p = plan(&a);
        assert_eq!(p.source, Some(crate::plan::PlanSource::Tools));
        assert_eq!(p.title.as_deref(), Some("Démo"));
        assert_eq!(
            p.plan_file.as_deref(),
            Some("docs/superpowers/plans/demo.md")
        );
        // The same when it writes its list whole.
        let mut b = rt();
        b.plan_from_files(b.plan_since, demo_list(), &mut Effects::default());
        call(
            &mut b,
            None,
            "t0",
            "TodoWrite",
            json!({"todos":[{"content":"Un","status":"in_progress","activeForm":"Fait un"}]}),
        );
        assert_eq!(plan(&b).source, Some(crate::plan::PlanSource::Tools));
        assert_eq!(plan(&b).title.as_deref(), Some("Démo"));

        // And it goes when the list stops being the plan's, with no read either: the agent
        // replaces its tasks by others.
        call(
            &mut a,
            None,
            "t1",
            "TodoWrite",
            json!({"todos":[{"content":"Autre chose","status":"pending"}]}),
        );
        let p = plan(&a);
        assert_eq!(p.tasks[0].title, "Autre chose");
        assert_eq!(p.title, None);
        assert_eq!(p.plan_file, None);
        // Back to the plan's tasks: back to its name.
        call(
            &mut a,
            None,
            "t2",
            "TodoWrite",
            json!({"todos":[{"content":"Task 1: Un","status":"pending"}]}),
        );
        assert_eq!(plan(&a).title.as_deref(), Some("Démo"));

        // A new conversation forgets what was read: a list of its own is no plan's.
        session(&mut a, "s9");
        creates(&mut a, "c9", "Task 1: Un", 1);
        assert_eq!(plan(&a).title, None);
    }

    #[test]
    fn removing_the_workspace_of_the_plan_it_follows_ends_its_run() {
        let mut a = rt();
        a.plan_from_files(a.plan_since, demo_list(), &mut Effects::default());
        assert_eq!(plan(&a).tasks[0].status, TaskStatus::InProgress);
        let status = |a: &AgentRt| plan(a).tasks[0].status;
        // Not the end: another plan's workspace, a file of this one, a failed command, a command
        // of a subagent.
        call(
            &mut a,
            None,
            "x1",
            "Bash",
            json!({"command":"rm -rf .superpowers/sdd/other"}),
        );
        returns(&mut a, "x1", "", Value::Null);
        call(
            &mut a,
            None,
            "x2",
            "Bash",
            json!({"command":"rm .superpowers/sdd/demo/task-1-brief.md"}),
        );
        returns(&mut a, "x2", "", Value::Null);
        call(
            &mut a,
            None,
            "x3",
            "Bash",
            json!({"command":"rm -rf .superpowers/sdd/demo"}),
        );
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"x3","content":"rm: cannot remove: Permission denied","is_error":true}]},"parent_tool_use_id":null}),
            &mut fx,
        );
        call(
            &mut a,
            None,
            "s1",
            "Agent",
            json!({"description":"Aide","prompt":"x"}),
        );
        call(
            &mut a,
            Some("s1"),
            "x4",
            "Bash",
            json!({"command":"rm -rf .superpowers/sdd/demo"}),
        );
        returns_in(&mut a, Some("s1"), "x4");
        assert_eq!(status(&a), TaskStatus::InProgress);

        // The agent removes it, as the skills end a run: the plan is done.
        call(
            &mut a,
            None,
            "x5",
            "Bash",
            json!({"command":"echo \"Task 1: complete\" >> .superpowers/sdd/demo/progress.md; rm -rf .superpowers/sdd/demo"}),
        );
        let fx = returns(&mut a, "x5", "", Value::Null);
        assert_eq!(status(&a), TaskStatus::Done);
        assert!(fx.save && fx.agent_changed && fx.plan_files);
        // Once only: its result again says nothing new.
        assert!(!returns(&mut a, "x5", "", Value::Null).save);
        // The look that follows finds no workspace: nothing reopens it.
        a.plan_from_files(a.plan_since, None, &mut Effects::default());
        assert_eq!(status(&a), TaskStatus::Done);

        // The list the agent holds is its own: a workspace removed changes nothing of it.
        let mut b = rt();
        b.plan_from_files(b.plan_since, demo_list(), &mut Effects::default());
        creates(&mut b, "c0", "Task 1: Un", 1);
        call(
            &mut b,
            None,
            "y1",
            "Bash",
            json!({"command":"rm -rf .superpowers/sdd/demo"}),
        );
        returns(&mut b, "y1", "", Value::Null);
        assert_eq!(plan(&b).tasks[0].status, TaskStatus::Pending);
        // A new conversation forgets the calls it was waiting for.
        let mut c = rt();
        c.plan_from_files(c.plan_since, demo_list(), &mut Effects::default());
        call(
            &mut c,
            None,
            "z1",
            "Bash",
            json!({"command":"rm -rf .superpowers/sdd/demo"}),
        );
        session(&mut c, "s9");
        returns(&mut c, "z1", "", Value::Null);
        assert!(c.view().meta.plan.is_none());
    }

    #[test]
    fn writing_the_workspace_of_a_plan_calls_for_a_look_but_is_no_plan_written() {
        let mut a = rt();
        for (id, name, path) in [
            ("w1", "Write", "C:/p/.superpowers/sdd/demo/progress.md"),
            ("w2", "Edit", ".superpowers/sdd/demo/task-3-brief.md"),
            (
                "w3",
                "MultiEdit",
                "C:\\p\\.superpowers\\sdd\\demo\\progress.md",
            ),
            (
                "w4",
                "Write",
                "C:/p/sub/.superpowers/sdd/demo/task-1-brief.md",
            ),
        ] {
            call(&mut a, None, id, name, json!({ "file_path": path }));
            assert!(
                returns(&mut a, id, "ok", Value::Null).plan_files,
                "{name} {path}"
            );
        }
        assert!(a.plan_written.is_empty());
        // Other files of that folder, and anything a subagent writes, do not.
        call(
            &mut a,
            None,
            "s1",
            "Agent",
            json!({"description":"Aide","prompt":"x"}),
        );
        for (parent, id, path) in [
            (None, "x1", "C:/p/.superpowers/other/notes.md"),
            (None, "x2", "C:/p/.superpowers/sdd-notes.md"),
            (Some("s1"), "x3", "C:/p/.superpowers/sdd/demo/progress.md"),
        ] {
            call(&mut a, parent, id, "Write", json!({ "file_path": path }));
            let fx = if parent.is_some() {
                returns_in(&mut a, parent, id)
            } else {
                returns(&mut a, id, "ok", Value::Null)
            };
            assert!(!fx.plan_files, "{path}");
        }
    }

    #[test]
    fn an_agent_saved_before_the_plan_loads_without_one_and_a_saved_plan_comes_back() {
        let meta: AgentMeta = serde_json::from_value(json!({"id":"a1","name":"ancien"})).unwrap();
        assert!(meta.plan.is_none());
        // Nothing is written for it.
        assert!(serde_json::to_value(&meta).unwrap().get("plan").is_none());
        let mut a = rt();
        creates(&mut a, "c0", "Lire", 1);
        let saved = serde_json::to_value(&a.meta).unwrap();
        assert_eq!(saved["plan"]["tasks"][0]["title"], "Lire");
        let back: AgentMeta = serde_json::from_value(saved.clone()).unwrap();
        assert_eq!(serde_json::to_value(&back).unwrap(), saved);
        // The window gets it with the agent.
        assert_eq!(
            serde_json::to_value(a.view()).unwrap()["plan"]["tasks"][0]["status"],
            "pending"
        );
    }
}
