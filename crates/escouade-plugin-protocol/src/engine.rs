//! The contract of a harness in process (INT-9, docs/PLUGINS.md §4.5 and §4.6): what the core and the plugin of a
//! harness say to each other. The core launches the harness itself, from the description the plugin gives
//! (`engine/launch`), after it checked it (the fence of private projects, read-only, the token of Escouade's server
//! never among the arguments, the files written in the agent's own folder only). What the harness says comes to the
//! plugin in batches of raw lines (`agent/frames`), which it translates into normalized items and effects (`Item`)
//! that the core applies to the agent. What the plugin has to write to the harness comes back in its answers
//! (`writes`), and the core writes it on the harness's input. The PTY of a terminal view never goes through the
//! plugin: only its launch (`terminal/launch`) and its hooks (`terminal/hook`), both ways.
//!
//! Every type tolerates what it does not know: a field added later is left aside, an item of an unknown `op` is
//! `Item::Unknown`, which the core ignores.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The methods of the contract, all called by the core.
pub mod method {
    /// Stateless: the description of a harness's launch for an agent (`LaunchParams` → `Launch`).
    pub const LAUNCH: &str = "engine/launch";
    /// Stateless: the launch of a one-shot question (`OneShotParams` → `OneShotLaunch`).
    pub const ONE_SHOT: &str = "engine/oneShot";
    /// Stateless: the answer of a one-shot question, from what its process wrote (`OneShotOutput` → `OneShotText`).
    pub const ONE_SHOT_READ: &str = "engine/oneShotRead";
    /// An agent given to the plugin, before its launch and again after a restart (`Attach`).
    pub const ATTACH: &str = "agent/attach";
    /// The agent's process is gone: its state may be dropped (a notification, `AgentRef`).
    pub const DETACH: &str = "agent/detach";
    /// What the harness wrote, in a batch of raw lines (`Frames` → `Items`).
    pub const FRAMES: &str = "agent/frames";
    /// A message to send (`SendMessage` → `Writes`).
    pub const SEND_MESSAGE: &str = "agent/sendMessage";
    /// Stop the running turn (`Interrupt` → `Writes`).
    pub const INTERRUPT: &str = "agent/interrupt";
    /// A change of model, effort or mode (`SetValue` → `Writes`).
    pub const SET_MODEL: &str = "agent/setModel";
    pub const SET_EFFORT: &str = "agent/setEffort";
    pub const SET_MODE: &str = "agent/setMode";
    /// The answer to a permission or a question the harness asked (`AnswerPermission`, `AnswerQuestion` → `Writes`).
    pub const ANSWER_PERMISSION: &str = "agent/answerPermission";
    pub const ANSWER_QUESTION: &str = "agent/answerQuestion";
    /// The quota of the agent's account, as its process reads it (`AgentRef` → `Usage`).
    pub const READ_USAGE: &str = "agent/readUsage";
    /// The launch of the harness's own interface in a terminal (`TerminalParams` → `Launch`).
    pub const TERMINAL_LAUNCH: &str = "terminal/launch";
    /// One of its hooks, with what the core answers it (`Hook` → `HookAnswer`).
    pub const TERMINAL_HOOK: &str = "terminal/hook";
    /// The lines the core read of what the harness's own interface writes, and its notes (`ViewParams` → `Items`).
    pub const VIEW_TRANSLATE: &str = "view/translate";
    /// A harness shared by agents (`Launch::shared`): the agent's session opened on it (`Open` → `Opened`).
    pub const OPEN: &str = "agent/open";
    /// The agent stops using its shared harness (`Close`, a call or a notification): gently, its turn finished first and
    /// `Item::Closed` said once it is; or at once, its turn stopped.
    pub const CLOSE: &str = "agent/close";
    /// The agent was the last in its folder: its shared harness lets that folder go (`AgentRef`).
    pub const LET_GO: &str = "agent/letGo";
    /// Compact its context (`Compact` → `Compacted`), as `Engine::compaction` says it can.
    pub const COMPACT: &str = "agent/compact";
    /// Give its session a title (`SetValue`).
    pub const RENAME: &str = "agent/rename";
    /// A request in the harness's own protocol, its answer: for the tests, and what has no method yet (`Control`).
    pub const CONTROL: &str = "agent/control";
    /// From the plugin, without being asked (a notification, `Pushed`): what its harness said by a way only the plugin
    /// reads (the event stream of a shared server).
    pub const ITEMS: &str = "agent/items";
    /// Its program ended by itself, for a harness that declares `relaunches` (`Gone` → `Items`): `Item::Relaunch` goes
    /// on with another, `Item::Stop` says why not, nothing ends the agent's process.
    pub const PROGRAM_GONE: &str = "agent/programGone";
    /// Stop one of its background tasks (`Task` → `Asked`).
    pub const STOP_TASK: &str = "agent/stopTask";
    /// What a background task wrote so far (`Task` → `Asked`).
    pub const TASK_OUTPUT: &str = "agent/taskOutput";
    /// Link its session to its vendor's site, or end the link (`RemoteLink` → `Asked`).
    pub const REMOTE: &str = "agent/remote";
}

/// How many bytes of lines a batch of `agent/frames` carries at most. A batch is what was read while the last one
/// was with the plugin: none waits for more lines, a lone line goes at once.
pub const BATCH_BYTES: usize = 64 * 1024;

/// Where a conversation starts from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StartAt {
    #[default]
    New,
    Resume {
        session: String,
    },
    Fork {
        of: String,
        #[serde(default)]
        at: Option<String>,
    },
}

/// Escouade's MCP server given to the agent. The plugin may put the token in a variable or a file of the agent's own,
/// never among the arguments: the core refuses such a launch.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grant {
    pub url: String,
    pub token: String,
}

impl std::fmt::Debug for Grant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Grant")
            .field("url", &self.url)
            .field("token", &"…")
            .finish()
    }
}

/// What `engine/launch` is told.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LaunchParams {
    /// The key of the agent's process, as `agent/*` names it.
    pub agent: String,
    pub cwd: String,
    pub model: String,
    #[serde(default)]
    pub effort: String,
    pub mode: String,
    #[serde(default)]
    pub append_prompt: Option<String>,
    /// What was said in the harness's own interface since the agent last ran (« Vue Cursor », a relay of the core's):
    /// it goes before the next message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relay: Option<String>,
    /// Its account goes by its key (`HarnessDecl::key_var` given), not by the machine's sign-in: what a refusal says.
    #[serde(default)]
    pub by_key: bool,
    #[serde(default)]
    pub start: StartAt,
    #[serde(default)]
    pub read_dirs: Vec<String>,
    /// Escouade's server, None when refused.
    #[serde(default)]
    pub escouade: Option<Grant>,
    /// The paths refused to its tools (« Projet privé »), as its program spells them: each must be in the launch,
    /// and `Launch::fence_applied` true, or the core does not launch it.
    #[serde(default)]
    pub fence: Vec<String>,
    /// A discussion of the Docs tab: it changes no file and runs no command. `Launch::read_only` must say it holds.
    #[serde(default)]
    pub read_only: bool,
    /// The private file the core made for Escouade's server (`Engine::grant_file`), which a plugin's launch may name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub escouade_file: Option<String>,
    /// Its program, as the user knows it.
    #[serde(default)]
    pub program: String,
    /// The folder of the agent's own, where the files of the launch may be written; None: no file may be.
    #[serde(default)]
    pub home: Option<String>,
    /// For a plugin that serves the harnesses other plugins describe (`dev.escouade.acp`, PLG-6): the harness as its
    /// manifest declares it (`HarnessDecl`, its `acp` included). None for a plugin's own harness.
    #[serde(default)]
    pub described: Option<HarnessDecl>,
    /// The language of the app (`fr`, `en`), for the lines the plugin writes in the conversation.
    #[serde(default)]
    pub lang: String,
    /// Whether its model takes an effort, as its account's harness told it; None: the plugin's own rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub takes_effort: Option<bool>,
    /// A settings file the core wrote for it (`Engine::fence_settings`): it holds the fence, and a launch that names
    /// it needs not spell the paths (`fence`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<String>,
    /// The file the core wrote to keep Escouade's server out of it (`Engine::refusal`), when it is refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refusal_file: Option<String>,
}

/// A variable of a launch.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

impl std::fmt::Debug for EnvVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}=…", self.name)
    }
}

/// A file the core writes before the launch, in the agent's own folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileToWrite {
    pub path: String,
    pub text: String,
}

/// The description of a launch (`engine/launch`, `terminal/launch`): never its program, which the core found itself
/// (`locate`), and runs in its Job object, under WSL when need be.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Launch {
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: Vec<EnvVar>,
    /// The variables of the app's own environment it must not get (`MSYSTEM` for Cursor).
    #[serde(default)]
    pub remove_env: Vec<String>,
    #[serde(default)]
    pub files: Vec<FileToWrite>,
    /// The fence of `LaunchParams::fence` is in the launch (its arguments, variables or files).
    #[serde(default)]
    pub fence_applied: bool,
    /// The harness is held read-only by this launch.
    #[serde(default)]
    pub read_only: bool,
    /// The fence of `LaunchParams::fence` is held by the plugin itself, on the reads and writes the harness makes
    /// through it (ACP's `fs/read_text_file`, `fs/write_text_file` and the permissions it asks), not by the launch:
    /// the core then takes the launch without the paths in it (PLG-6).
    #[serde(default)]
    pub fence_by_client: bool,
    /// The first lines written on its input, once it started.
    #[serde(default)]
    pub writes: Vec<String>,
    /// One process for several agents (`opencode serve`, 0,7 Go): the core runs it for those of the same program and
    /// variables, and does not start it again for the next; the plugin speaks to it by a way of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared: Option<SharedLaunch>,
    /// For a terminal view: the trust bands its interface asks, shown before it opens.
    #[serde(default)]
    pub trust: Vec<String>,
}

/// How a process shared by agents starts and is shared. It runs in a folder of its own, none of the agents': a worktree
/// removed would find it held. It lives from its first agent to its last, in the core's Job object.
#[derive(Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SharedLaunch {
    /// What the line that says it is ready holds: the line goes to the plugin (`SharedRef::line`).
    pub ready: String,
    /// The template of the line where the server says its address (`http://{host}:{port}`): the port the core reads
    /// there is the one `local:shared` lets the plugin reach (`{host}` is `127.0.0.1` and nothing else, ESC-332).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listens: Option<String>,
    /// A folder has one agent per process: another agent in that folder gets a process of its own.
    #[serde(default)]
    pub one_agent_a_folder: bool,
    /// What the plugin wants back with each agent on that process (its password): kept by the core, never read.
    #[serde(default)]
    pub keep: Value,
}

impl std::fmt::Debug for SharedLaunch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedLaunch")
            .field("ready", &self.ready)
            .field("listens", &self.listens)
            .field("one_agent_a_folder", &self.one_agent_a_folder)
            .field("keep", &"…")
            .finish()
    }
}

/// The shared process an agent is on: enough for a plugin that started again to find it.
#[derive(Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SharedRef {
    /// Which process, the same for each of its agents.
    pub key: String,
    /// The line that said it is ready.
    pub line: String,
    /// `SharedLaunch::keep`, as given by the launch that started it.
    #[serde(default)]
    pub keep: Value,
}

impl std::fmt::Debug for SharedRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedRef")
            .field("key", &self.key)
            .field("line", &self.line)
            .field("keep", &"…")
            .finish()
    }
}

/// An agent given to the plugin: what it cannot know by itself, given again after each restart and each change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Attach {
    pub agent: String,
    pub cwd: String,
    #[serde(default)]
    pub session: Option<String>,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub effort: String,
    #[serde(default)]
    pub mode: String,
    /// The requests of the harness waiting for the user's answer, by their ids.
    #[serde(default)]
    pub pending: Vec<String>,
    /// As `LaunchParams::described`.
    #[serde(default)]
    pub described: Option<HarnessDecl>,
    /// As `LaunchParams::lang`.
    #[serde(default)]
    pub lang: String,
    /// The shared process it is on (`Launch::shared`), once it runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared: Option<SharedRef>,
    /// Its folders the plugin declares (`permissions.files`), by root (`agent`, `agentRepo`, `agentHome`, `temp`), resolved
    /// by the core, for `fs/*`.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub roots: std::collections::BTreeMap<String, String>,
    /// As `LaunchParams::fence`, `read_only` and `append_prompt`, for a plugin that holds them itself, or starts again:
    /// on a session of its own (`Launch::shared`), in rules it writes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fence: Vec<String>,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub append_prompt: Option<String>,
    /// A turn of its runs, as far as the core knows: one that ended while the plugin was down is told so.
    #[serde(default)]
    pub running: bool,
    /// As `LaunchParams::home`, `read_dirs` and `by_key`, for a plugin that starts again and writes in that folder (its
    /// rules, a harness that `relaunches`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub read_dirs: Vec<String>,
    #[serde(default)]
    pub by_key: bool,
}

/// What `agent/open` is told: the agent's session, on the shared process of its `agent/attach`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Open {
    pub agent: String,
    #[serde(default)]
    pub start: StartAt,
    /// Escouade's server, None when refused.
    #[serde(default)]
    pub escouade: Option<Grant>,
}

/// What `agent/open` found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Opened {
    /// How its harness's interface joins it in a terminal, what is secret in a variable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attach: Option<Launch>,
    /// The models of its account, as `ToldModel`s of the core.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Close {
    pub agent: String,
    /// Its turn stopped first, not waited for.
    #[serde(default)]
    pub now: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Compact {
    pub agent: String,
    #[serde(default)]
    pub instructions: String,
}

/// `EngineProcess::compact`'s answer: it is over; false, it runs as a turn the harness ends itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Compacted {
    #[serde(default)]
    pub over: bool,
    /// What the core writes to the harness to ask it: `Item::Compacted` then says the end.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub writes: Vec<String>,
    /// Written, it runs as a turn the harness ends itself (Codex): no `Item::Compacted` to wait for.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub turn: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Control {
    pub agent: String,
    pub request: Value,
    /// How long the answer may take, in milliseconds.
    #[serde(default)]
    pub within_ms: u64,
}

/// `method::ITEMS`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pushed {
    pub agent: String,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRef {
    pub agent: String,
}

/// What `agent/interrupt` is told (a plugin may read it as an `AgentRef`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Interrupt {
    pub agent: String,
    /// The messages queued after the turn go too.
    #[serde(default)]
    pub cancel_queued: bool,
}

/// `method::PROGRAM_GONE`: how it ended, and what it said last on its error output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gone {
    pub agent: String,
    #[serde(default)]
    pub code: Option<i32>,
    #[serde(default)]
    pub said: String,
}

/// A batch of what the harness wrote: each line as it read it, never read again by the core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frames {
    pub agent: String,
    pub frames: Vec<String>,
}

/// What a batch made. The answer to any call that answers `Writes` may be this instead: its items applied, in their
/// order (a message that starts a program again: `Item::Relaunch`, then its `Item::Write`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Items {
    #[serde(default)]
    pub items: Vec<Item>,
}

/// What the plugin writes to the harness, line by line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Writes {
    #[serde(default)]
    pub writes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendMessage {
    pub agent: String,
    pub uuid: String,
    pub text: String,
    /// The message as its harness takes it, when the core made it (ACP's `prompt` blocks: its images and its text
    /// files, INT-10.1); None: `text` alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetValue {
    pub agent: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerPermission {
    pub agent: String,
    pub request: String,
    pub allow: bool,
    /// Allowed for the rest of the session.
    #[serde(default)]
    pub always: bool,
    /// The tool's input, maybe changed, when allowed.
    #[serde(default)]
    pub input: Value,
    /// What the agent is told, when refused.
    #[serde(default)]
    pub message: Option<String>,
    /// A plan approved: the mode its work goes on in (the one the agent was in before Plan).
    #[serde(default)]
    pub resume: Option<String>,
    /// The call it answers (`Item::Ask::call`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub call: String,
    /// « Toujours »: the rules of `Item::Ask::suggestions` it goes by.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub rules: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnswerQuestion {
    pub agent: String,
    pub request: String,
    pub call: String,
    /// The answers, by question.
    pub answers: Value,
    /// The questions they answer, as the card has them, for a harness that wants them in its order.
    #[serde(default)]
    pub questions: Value,
    /// The call's input with its answers, for a harness that takes it whole.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub input: Value,
}

/// The quota its process read (`usage::parse_windows` reads its windows); null when none.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Usage {
    #[serde(default)]
    pub usage: Value,
}

/// The answer of `agent/readUsage` when the quota is to be asked of the harness: `writes` go to its input, and
/// `Item::Usage` brings what it said.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UsageAsked {
    #[serde(default)]
    pub writes: Vec<String>,
}

/// The answer of a call whose end is the harness's to tell (`agent/interrupt`, `agent/setModel`, `agent/control`…): `writes` go
/// to its input, and `Item::Answer` under `ask` brings what it said. An answer without `ask` is over as it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Asked {
    #[serde(default)]
    pub writes: Vec<String>,
    #[serde(default)]
    pub ask: String,
}

/// One of its background tasks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub agent: String,
    pub task: String,
}

/// `method::REMOTE`: its session linked (`name` on the vendor's site, kept when it ends if `keep`, joining the session
/// `reattach`), or unlinked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteLink {
    pub agent: String,
    pub enabled: bool,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub keep: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reattach: Option<String>,
}

/// What a model cost in the turn that ends (`Item::Result`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ModelTurn {
    pub model: String,
    #[serde(default)]
    pub input: u64,
    #[serde(default)]
    pub output: u64,
    #[serde(default)]
    pub cache: u64,
    #[serde(default)]
    pub cost: f64,
}

/// The tokens and cost of a turn.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct TurnUsage {
    #[serde(default)]
    pub input: u64,
    #[serde(default)]
    pub output: u64,
    #[serde(default)]
    pub cache: u64,
    #[serde(default)]
    pub cost: f64,
}

/// A normalized item or an effect, which the core applies to the agent: the same as the conversation's (`user`,
/// `text`, `thinking`, `tool`, `question`, `permission`, `turn`, `notice`) and the effects (`Effects` of
/// `agent.rs`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum Item {
    /// A turn begins, in the session `session` (the agent's from now on).
    Start {
        #[serde(default)]
        session: Option<String>,
    },
    /// The agent's session, opened (new, resumed or copied) before any turn: the agent's from now on, its status left
    /// as it is (INT-10.6: a copy of Mistral Vibe's has its own session as soon as its process starts).
    Session { session: String },
    /// Its text; a subagent's under `parent`. With `id`, the whole of the text streamed under that id (`Delta`).
    #[serde(rename_all = "camelCase")]
    Text {
        text: String,
        #[serde(default)]
        parent: Option<String>,
        #[serde(default)]
        id: Option<String>,
    },
    Thinking {
        text: String,
        #[serde(default)]
        parent: Option<String>,
        #[serde(default)]
        id: Option<String>,
    },
    /// A piece of a text (or a reasoning) as it is written, under `id`: the first one makes its item, the others add to
    /// it; `Text` (`Thinking`) of the same id ends it.
    Delta {
        id: String,
        text: String,
        #[serde(default)]
        thinking: bool,
        #[serde(default)]
        parent: Option<String>,
    },
    /// The model a subagent runs on, under its call.
    Subagent { call: String, model: String },
    /// A tool called, named as Escouade names them (Claude Code's names: `Bash`, `Edit`, `Task`…).
    Tool {
        id: String,
        name: String,
        #[serde(default)]
        input: Value,
        #[serde(default)]
        parent: Option<String>,
        /// Not a row of the plan: its task list comes by `Todos`.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        unplanned: bool,
    },
    /// What is known later of a call: its input (an edit's file), or the name it turns out to have (`Write` for a
    /// file added).
    ToolInput {
        id: String,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        input: Value,
    },
    /// Its result; `result` the structured one Claude Code gives (`{ filePath, structuredPatch }` of an edit,
    /// `{ stdout, stderr }` of a command), which shows its diff.
    ToolDone {
        id: String,
        #[serde(default)]
        output: String,
        #[serde(default)]
        error: bool,
        #[serde(default)]
        result: Value,
    },
    /// A file it changed (the effect `files_changed`).
    Touched { path: String },
    /// The approval of the call `call`, asked as the request `request`: the call's tool and input unless `tool`
    /// says another (`ExitPlanMode` and its `plan`), what the card says of it (`description`, `reason`), the
    /// write it would make (`diff`, shown on the card), and whether « Pour cet agent » is offered.
    #[serde(rename_all = "camelCase")]
    Permission {
        request: String,
        call: String,
        #[serde(default)]
        tool: Option<String>,
        #[serde(default)]
        input: Option<Value>,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        reason: Option<String>,
        #[serde(default)]
        diff: Option<Diff>,
        #[serde(default)]
        can_always: bool,
        /// The call of the subagent that asks it.
        #[serde(default)]
        parent: Option<String>,
        /// The hunks of a patch the harness gave as they are, in place of `diff` (`hunks_of`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hunks: Option<Value>,
        /// « Always » is the harness's, for its session: not worded as « Pour cet agent ».
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        always_by_harness: bool,
        /// The turn stays open on this card, what was streaming closed (a plan to approve at the end of a turn).
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        ends_stream: bool,
    },
    /// Questions to the user (each `{question, header, multiSelect, options: [{label, description}]}`).
    Question {
        request: String,
        call: String,
        questions: Value,
        /// The call of the subagent that asks them.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent: Option<String>,
    },
    /// A request answered in the harness's own interface: its card says so and goes. `decision` of a permission
    /// (`allow`, `always`, `deny`), `answers` of questions (for each in its order, the labels chosen), else cancelled.
    Answered {
        request: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        decision: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        answers: Option<Value>,
    },
    /// The user's message typed in the harness's own interface: shown as they typed it.
    User { id: String, text: String },
    /// What the user typed in the harness's own interface (or Escouade pasted there), read in its view: their message, under
    /// `id` when it has one, and a turn starts. `prompt`: the id of its prompt, when the interface writes one message
    /// twice (a command, as typed then as `<command-name>`).
    Typed {
        #[serde(default)]
        id: Option<String>,
        text: String,
        #[serde(default)]
        images: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        prompt: Option<String>,
    },
    /// The view's file was written since its interface started: the interface is up.
    Spoke,
    /// Something of its session is written: its absence now would be a loss.
    Wrote,
    /// The interface waits for the user, for this.
    Waits { body: String },
    /// The same, for the call of this tool, which the core words.
    WaitsOn {
        tool: String,
        #[serde(default)]
        input: Value,
    },
    /// What the interface is about to ask, which it waits for once a hook says so.
    Asks {
        tool: String,
        #[serde(default)]
        input: Value,
    },
    /// The agent works: a turn runs from now on, if none did.
    Runs,
    /// What it waited for is answered: it works again.
    Goes,
    /// The turn of its harness under way (the end the core makes after a hook is only that turn's).
    ViewTurn {
        #[serde(default)]
        id: Option<String>,
    },
    /// The view's turn is over: the core closes it, and gives the note `end` back with what it knows of it.
    ViewEnd {
        #[serde(default)]
        interrupted: bool,
        /// Its length, in ms.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    /// The usage of a message of the view, as its API gave it, which the core prices at the end of the turn.
    ViewUsage {
        id: String,
        model: String,
        #[serde(default)]
        usage: Value,
    },
    /// What went wrong in the view's turn, for its end.
    ViewError { error: String },
    /// A tool of the view whose run is not told, shown as `status`: again under the same id, its status changes.
    Row {
        #[serde(default)]
        id: Option<String>,
        name: String,
        #[serde(default)]
        input: Value,
        status: String,
    },
    /// The interface tells how the link of its session to claude.ai went: at `url`, or, with none, that it did not open.
    Bridged {
        #[serde(default)]
        url: Option<String>,
        #[serde(default)]
        said: String,
    },
    /// A sign that the interface's session is linked to claude.ai.
    Linked,
    /// The model the harness took in place of the agent's (one its account does not offer).
    Model { model: String },
    /// What the agent's context holds (`tokens`), and the window of its model.
    Context {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tokens: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        window: Option<u64>,
    },
    /// The task list of the agent, as Claude Code's `TodoWrite` gives it (`[{ content, status, activeForm }]`),
    /// for its plan.
    Todos { todos: Value },
    /// The harness changed its mode by itself (out of its mode Plan): Escouade's mode it is in now.
    Mode { mode: String },
    /// The turn is over.
    Turn {
        #[serde(default)]
        interrupted: bool,
        /// Stopped by the usage limit, which resets then (ms), if told: `null` for a limit whose reset is not told,
        /// left out for none.
        #[serde(
            default,
            deserialize_with = "present",
            skip_serializing_if = "Option::is_none"
        )]
        limited: Option<Option<i64>>,
        #[serde(default)]
        usage: TurnUsage,
        /// It ended in error, and why (a model refused, a key refused).
        #[serde(default)]
        error: Option<String>,
        /// The last entry of its session: where a copy forks it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        entry: Option<String>,
    },
    /// A line of the conversation from Escouade or the harness, not the model (`info`, `warning`, `error`).
    Notice {
        text: String,
        #[serde(default = "info")]
        level: String,
    },
    /// A line for the harness's input, written by the core in the order of the items.
    Write { line: String },
    /// The approval of a call, its card written whole: the tool asked, what it says of it, the hunks of the write it
    /// would make (Claude Code's `structuredPatch`), how far « Toujours » goes (`alwaysScope`, left out for a plan).
    #[serde(rename_all = "camelCase")]
    PermissionCard {
        request: String,
        call: String,
        tool: String,
        #[serde(default)]
        input: Value,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        reason: Option<String>,
        #[serde(default)]
        hunks: Value,
        #[serde(default)]
        can_always: bool,
        #[serde(default)]
        always_scope: Option<String>,
        #[serde(default)]
        parent: Option<String>,
    },
    /// Questions asked by a subagent (`parent`), or a form (`form`: what it is for, `{ message, source }`).
    QuestionCard {
        request: String,
        call: String,
        questions: Value,
        #[serde(default)]
        form: Value,
        #[serde(default)]
        parent: Option<String>,
    },
    /// The account's quota as a call told it: its meters (`usage::parse_windows`'s).
    Quota {
        #[serde(default)]
        meters: Value,
    },
    /// What `agent/readUsage` asked of the harness (`UsageAsked`), as `Usage::usage`.
    Usage {
        #[serde(default)]
        usage: Value,
    },
    /// The turn ending next stopped at a limit it waits for: told in its turn, the agent at rest and not in error.
    LimitRest,
    /// A harness that declares `opens`: its session is open and its first message may go; `models` are the account's
    /// (`ToldModel`s), for the menu.
    Ready {
        #[serde(default)]
        models: Value,
    },
    /// The end of what `agent/compact` asked, and why it failed.
    Compacted {
        #[serde(default)]
        error: Option<String>,
    },
    /// The harness is of no use: the core stops it, and `reason` is what its end says (its protocol, no sign-in).
    Stop { reason: String },
    /// The last entry of its session a copy may start after, when no `Turn` tells it (a plan to approve).
    Entry {
        #[serde(default)]
        entry: Option<String>,
    },
    /// What the agent is doing, shown on its row, when the item that follows does not tell it: a command that only
    /// reads a file is a `Read` (`tool`, `input`), a reasoning begun says so before it has a word (`block`: `thinking`).
    Activity {
        #[serde(default)]
        tool: Option<String>,
        #[serde(default)]
        input: Value,
        #[serde(default)]
        block: Option<String>,
    },
    /// How long the turn that ends took, as its harness counted it (ms), told just before its `Turn`; left out, the core
    /// counts it.
    Took { ms: i64 },
    /// A harness that `relaunches`: its program stopped, and another started from this description, checked as the
    /// first one was; the lines written after it go to the new one.
    Relaunch { launch: Box<Launch> },
    /// A harness that `relaunches`: its program stopped, the agent kept; `Relaunch` starts the next one.
    Pause,
    /// A file written now, in the agent's own folder alone, checked as those of a launch: rules read while it runs.
    File { path: String, text: String },
    /// Asked to stop gently (`method::CLOSE`), its turn is over: the agent has left its shared process.
    Closed,
    /// The conversation it runs (a harness that says so at each start): another than the agent's starts anew, its plan
    /// forgotten; `notice` is what the agent's conversation says when that happens to a process that had one.
    Conversation {
        session: String,
        #[serde(default)]
        notice: String,
    },
    /// A message of the model begins, under `id`: what it reads (`usage`, as the API tells it) counts in the running
    /// turn until `Item::Result`; `main`: of the agent itself, not of a subagent.
    Message {
        id: String,
        #[serde(default)]
        model: String,
        #[serde(default)]
        usage: Value,
        #[serde(default)]
        main: bool,
    },
    /// What the message `id` wrote so far, in tokens.
    MessageOutput { id: String, output: u64 },
    /// What the harness says of its background tasks, for the plan (`task_started`, `task_progress`…).
    Tasks { frame: Value },
    /// A line of the conversation that comes from neither the user nor the model (a task that ended, a message of
    /// another agent): `source` says which.
    Event {
        #[serde(default)]
        id: Option<String>,
        source: String,
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from: Option<String>,
    },
    /// What a local command printed (`output`, `error`).
    Output {
        #[serde(default)]
        id: Option<String>,
        level: String,
        text: String,
    },
    /// The whole call of a tool, once its message is whole: `Tool` began it if it was streamed, its row is then told
    /// its input; the plan hears of it either way.
    Call {
        id: String,
        name: String,
        #[serde(default)]
        input: Value,
        #[serde(default)]
        parent: Option<String>,
    },
    /// Its result, whatever the row says by now (`ToolDone` leaves a row that ended as it is).
    ToolResult {
        id: String,
        #[serde(default)]
        output: String,
        #[serde(default)]
        error: bool,
        #[serde(default)]
        result: Value,
    },
    /// A user's message the harness says back: the echo of one sent by the core, or one sent another way. `origin` is
    /// who it comes from when no person wrote it (`task-notification`, `peer`…), `from` the agent that sent it.
    Replayed {
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        origin: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from: Option<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        synthetic: bool,
        #[serde(default)]
        images: u32,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        files: Vec<String>,
    },
    /// The harness asks leave for a call, or asks the user (`tool` `AskUserQuestion`): `suggestions` are the rules
    /// « Toujours » would keep, given back in `AnswerPermission::rules`; `task` the subagent that asks. The core answers
    /// by itself a read of a file the user joined.
    #[serde(rename_all = "camelCase")]
    Ask {
        request: String,
        call: String,
        tool: String,
        #[serde(default)]
        input: Value,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        reason: Option<String>,
        #[serde(default)]
        suggestions: Value,
        #[serde(default)]
        no_always: bool,
        #[serde(default)]
        default_no: bool,
        #[serde(default)]
        task: Option<String>,
    },
    /// The end of a turn the harness counts itself: what each model cost, the context `window` of the one that ran,
    /// whether it failed and why. `limited`: stopped by the usage limit, which resets at `resets` (ms) if told;
    /// `sign_in`: by a sign-in of its account that could not be refreshed; `failure` is what a failure says, `reply`
    /// what a good end says when the model wrote nothing.
    #[serde(rename_all = "camelCase")]
    Result {
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        failed: bool,
        #[serde(default)]
        error: Option<String>,
        #[serde(default)]
        failure: String,
        #[serde(default)]
        reply: String,
        #[serde(default)]
        limited: bool,
        #[serde(default)]
        resets: Option<i64>,
        #[serde(default)]
        sign_in: bool,
        #[serde(default)]
        duration: Value,
        #[serde(default)]
        window: Option<u64>,
        #[serde(default)]
        cost: f64,
        #[serde(default)]
        tokens: u64,
        #[serde(default)]
        models: Vec<ModelTurn>,
    },
    /// A usage limit was met: `scoped` to the model it runs on, not to its account.
    Limit {
        #[serde(default)]
        scoped: bool,
    },
    /// Whether its account answers as extra usage.
    Overage { on: bool },
    /// The state of its link to its vendor's site (`ready`, `connected`…).
    Remote {
        #[serde(default)]
        state: Option<String>,
    },
    /// What its harness said when it started: its commands and its models (as it tells them), which `Ready` follows.
    Told {
        #[serde(default)]
        commands: Value,
        #[serde(default)]
        models: Value,
    },
    /// What the harness said of the call `ask` (`Asked`): its answer, or why it refused.
    Answer {
        ask: String,
        #[serde(default)]
        answer: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    /// What the harness wrote that is none of its protocol, kept for what its end says.
    Said { line: String },
    /// An item of a later version of the contract: left aside.
    #[serde(other)]
    Unknown,
}

fn info() -> String {
    "info".into()
}

/// A field that is there, `null` included (`Some(None)`); left out, it is `None` (`default`).
fn present<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Option<i64>>, D::Error> {
    Ok(Some(Option::<i64>::deserialize(d)?))
}

/// The hunks of a unified diff, as Claude Code's `structuredPatch` has them.
pub fn hunks_of(diff: &str) -> Value {
    let mut hunks: Vec<Value> = Vec::new();
    for line in diff.lines() {
        if let Some(head) = line.strip_prefix("@@ ") {
            let num = |sign: char| {
                head.split_whitespace()
                    .find_map(|w| w.strip_prefix(sign))
                    .and_then(|w| w.split(',').next())
                    .and_then(|n| n.parse::<u64>().ok())
                    .unwrap_or(0)
            };
            hunks.push(
                serde_json::json!({ "oldStart": num('-'), "newStart": num('+'), "lines": [] }),
            );
            continue;
        }
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        if hunks.is_empty() {
            if !(line.starts_with('+') || line.starts_with('-') || line.starts_with(' ')) {
                continue;
            }
            hunks.push(serde_json::json!({ "oldStart": 1, "newStart": 1, "lines": [] }));
        }
        if let Some(lines) = hunks.last_mut().and_then(|h| h["lines"].as_array_mut()) {
            lines.push(serde_json::json!(line));
        }
    }
    Value::Array(hunks)
}

/// A write a permission would make: its file, before (None for a file added) and after.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diff {
    pub path: String,
    #[serde(default)]
    pub old_text: Option<String>,
    #[serde(default)]
    pub new_text: String,
}

/// What `engine/oneShot` is told: one question, no session, no settings of the user's, no MCP server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OneShotParams {
    /// `light` (a short text, no tool) or `reading` (after reading its folder, tools that only read).
    pub weight: String,
    pub cwd: String,
    pub system: String,
    pub prompt: String,
    #[serde(default)]
    pub model: String,
}

/// How the core runs it: the arguments, the variables, and what goes on its input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OneShotLaunch {
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: Vec<EnvVar>,
    #[serde(default)]
    pub remove_env: Vec<String>,
    #[serde(default)]
    pub input: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OneShotOutput {
    pub stdout: String,
    pub stderr: String,
    #[serde(default)]
    pub code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OneShotText {
    pub text: String,
}

/// What `terminal/launch` is told: the agent's interface in a terminal, its hooks pointed at Escouade.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TerminalParams {
    pub agent: String,
    pub cwd: String,
    #[serde(default)]
    pub session: Option<String>,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub home: Option<String>,
    /// Where its hooks post (`http://127.0.0.1:<port>/hook`), the variable that holds their token, and
    /// `escouade-hook` for a harness that runs its hooks as commands.
    pub hook_url: String,
    pub hook_token_var: String,
    #[serde(default)]
    pub hook_command: Option<String>,
    #[serde(default)]
    pub escouade: Option<Grant>,
    #[serde(default)]
    pub fence: Vec<String>,
    #[serde(default)]
    pub read_only: bool,
}

/// One of its hooks, and what the core has to answer it: the hook goes both ways (what Escouade adds for the model, a call it refuses).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hook {
    pub agent: String,
    pub hook: Value,
    #[serde(default)]
    pub reply: Option<HookReply>,
}

/// What the core answers a hook, which the plugin puts in its harness's form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HookReply {
    /// Context added for the model.
    #[serde(default)]
    pub context: Option<String>,
    /// The call refused, and why.
    #[serde(default)]
    pub deny: Option<String>,
}

/// What a hook made: items for the agent, and the hook's answer as its harness reads it (None: nothing).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HookAnswer {
    #[serde(default)]
    pub items: Vec<Item>,
    #[serde(default)]
    pub answer: Option<Value>,
}

/// What the conversation holds of an id, for a view that starts on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ViewKnown {
    pub id: String,
    /// A call's tool, and how it stands (`running`, `ok`…): none for what is no call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// `view/translate`: the terminal view of an agent. The core reads what the harness's own interface writes (its transcript,
/// its rollout, its journal) and hands it on here a line at a time, what it holds of the agent beside it; the plugin answers
/// the items. The core keeps the terminal, the file and the agent's state; the plugin, what the lines mean.
///
/// A line is what the file holds, or a note of the core, an object with a field `escouade`:
/// - `{"escouade":"end","error":…,"durationMs":…,"models":…}`: the running turn is over, for a reason the core knows (its
///   interface gone, a hook, what `Item::ViewEnd` asked): the plugin answers its `Item::Turn`;
/// - `{"escouade":"hook","hook":…}`: a hook of the interface that says what the file does not;
/// - `{"escouade":"frame","frame":…}`: a message of the harness, read as a process's (a recorded stream).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ViewParams {
    /// The view, by a key of its own: another one is another conversation, which starts from nothing.
    pub agent: String,
    /// The harness, for a plugin that has several.
    #[serde(default)]
    pub harness: String,
    /// The first lines of this view: `known` is given, and `gone`, the view it replaces, may be dropped.
    #[serde(default)]
    pub fresh: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub known: Vec<ViewKnown>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gone: Option<String>,
    pub cwd: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub model: String,
    /// The language of the app.
    #[serde(default)]
    pub lang: String,
    /// The model the agent last ran on.
    #[serde(default)]
    pub main_model: String,
    #[serde(default)]
    pub session: Option<String>,
    /// A turn runs.
    #[serde(default)]
    pub active: bool,
    /// The user stopped it.
    #[serde(default)]
    pub interrupted: bool,
    /// The agent is a copy, its session not its own yet.
    #[serde(default)]
    pub copy: bool,
    /// A discussion of the Docs tab: it writes nothing.
    #[serde(default)]
    pub discussion: bool,
    /// The message of the turn under way, as the interface took it.
    #[serde(default)]
    pub prompt: Option<String>,
    pub lines: Vec<String>,
}

/// A harness as a manifest declares it (`contributes.harnesses[]`): all the core needs without starting the plugin
/// (the welcome screen, the menus, the accounts, the memory).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessDecl {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub speaker: Option<String>,
    /// Its program: a path (`{escouade}/…` for one of the app's), or names looked for on the PATH.
    #[serde(default)]
    pub locate: Locate,
    #[serde(default = "version_args")]
    pub version_args: Vec<String>,
    #[serde(default)]
    pub tested: TestedDecl,
    #[serde(default)]
    pub caps: CapsDecl,
    #[serde(default)]
    pub models: Vec<ModelDecl>,
    #[serde(default)]
    pub efforts: Vec<String>,
    /// The variable that gives each account its configuration folder.
    #[serde(default)]
    pub account_var: Option<String>,
    /// The variable that gives an account added by its key that key (`CURSOR_API_KEY`): the agent then goes by it, not
    /// by the machine's sign-in (`LaunchParams::by_key`).
    #[serde(default)]
    pub key_var: Option<String>,
    /// It does not end when its input is closed (Cursor CLI's `agent acp`): it is killed that many milliseconds after.
    #[serde(default)]
    pub close_grace_ms: Option<u64>,
    /// What frames its messages on its input and output: lines when left out, `contentLength` for `Content-Length`
    /// headers (Copilot CLI's server).
    #[serde(default)]
    pub framing: Option<String>,
    /// A line written on its input just before it is closed (Copilot CLI's `runtime.shutdown`, which saves its session).
    #[serde(default)]
    pub close_line: Option<String>,
    /// Its plugin opens the session over the protocol once it runs: the agent is ready at `Item::Ready`, not at its
    /// launch, and `Item::Stop` is its refusal.
    #[serde(default)]
    pub opens: bool,
    /// One agent has several programs in a row (`Item::Relaunch`, `Item::Pause`): it takes its settings at its launch alone,
    /// or has no way to stop a turn. Its end by itself is told to its plugin (`method::PROGRAM_GONE`).
    #[serde(default)]
    pub relaunches: bool,
    /// The lines of its error output that hold one of these are not kept (what it logs of the account).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stderr_leaves_out: Vec<String>,
    /// Where it keeps what it remembers of a project, offered once to the shared memory: a folder of files, with
    /// `{configDir}` and `{projectKey}` (the project's path, every character but a letter or a digit a dash).
    #[serde(default)]
    pub memory: Option<MemoryDecl>,
    /// A harness that speaks ACP, described: how the plugin `dev.escouade.acp` drives it (PLG-6). A plugin that only
    /// describes such a harness runs no program of its own.
    #[serde(default)]
    pub acp: Option<AcpDecl>,
    /// How to install it, shown when it is not found.
    #[serde(default)]
    pub install: Option<InstallDecl>,
    /// The command that updates it.
    #[serde(default)]
    pub update_command: Option<String>,
    /// Who writes the texts it drafts (« rédigé par … »), its name when left out.
    #[serde(default)]
    pub writer: Option<String>,
}

/// A text in the languages of the app: one for all, or by language (`{ "fr": "…", "en": "…" }`, English when the
/// language is missing).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Text {
    One(String),
    ByLang(std::collections::BTreeMap<String, String>),
}

impl Text {
    /// In `lang`, else English, else any.
    pub fn get(&self, lang: &str) -> String {
        match self {
            Text::One(t) => t.clone(),
            Text::ByLang(m) => m
                .get(lang)
                .or_else(|| m.get("en"))
                .or_else(|| m.values().next())
                .cloned()
                .unwrap_or_default(),
        }
    }
}

/// A mode of Escouade a harness takes (`default`, `plan`, `acceptEdits`, `bypassPermissions`), and what it means
/// there, as the menu says it. Written as its id alone, or `{ "id", "meaning" }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModeDecl {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meaning: Option<Text>,
}

impl<'de> Deserialize<'de> for ModeDecl {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Form {
            Id(String),
            Full {
                id: String,
                #[serde(default)]
                meaning: Option<Text>,
            },
        }
        Ok(match Form::deserialize(d)? {
            Form::Id(id) => ModeDecl { id, meaning: None },
            Form::Full { id, meaning } => ModeDecl { id, meaning },
        })
    }
}

impl From<&str> for ModeDecl {
    fn from(id: &str) -> Self {
        ModeDecl {
            id: id.into(),
            meaning: None,
        }
    }
}

/// Its installers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InstallDecl {
    #[serde(default)]
    pub windows: String,
    #[serde(default)]
    pub unix: String,
}

/// How an ACP agent is driven (ACP v1: JSON-RPC 2.0, one message a line on its input and output).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AcpDecl {
    /// Its arguments (`["acp"]`).
    #[serde(default)]
    pub args: Vec<String>,
    /// Variables of its own (never a secret: a manifest is read by all).
    #[serde(default)]
    pub env: std::collections::BTreeMap<String, String>,
    /// Variables of the app's environment it must not get.
    #[serde(default)]
    pub remove_env: Vec<String>,
    /// It reads and writes files through the client (`fs/read_text_file`, `fs/write_text_file`): the fence of
    /// private projects is then held there. False: it reads and writes by itself, and no private project is open to it.
    #[serde(default = "yes")]
    pub fs: bool,
    /// With `fs` false, the fence of a private project held on its permissions alone, as Cursor CLI's in the core was
    /// (ESC-140): a permission that names a path of the fence is refused at once; what it reads and writes by itself
    /// is not seen, which its modes' meanings say. Only a plugin Escouade gives is believed (`fenceByClient`).
    #[serde(default)]
    pub fence_on_permissions: bool,
    /// The ACP mode (`session/set_mode`) of each mode of Escouade; a mode left out sets none.
    #[serde(default)]
    pub modes: std::collections::BTreeMap<String, String>,
    /// Its ACP mode that changes nothing (`ask`): with it, it holds a read-only launch (a discussion of the Docs tab).
    #[serde(default)]
    pub read_only_mode: Option<String>,
    /// The id of the configuration option that picks the model (`session/set_config_option`); left out, the model
    /// goes by `session/set_model`.
    #[serde(default)]
    pub model_option: Option<String>,
    /// The extensions of the protocol it speaks beside ACP (`cursor`: `cursor/ask_question`, `cursor/create_plan`,
    /// `cursor/update_todos`, its subagents).
    #[serde(default)]
    pub extensions: Vec<String>,
    /// It has no tool to ask the user: its first message tells it to end its turn on a block
    /// `<escouade_question>`, shown as a question's card, the answers sent as the next message (as for Gemini CLI).
    #[serde(default)]
    pub questions_in_text: bool,
    /// The names of its tools that run a subagent (by their title or `rawInput.tool`): shown as Claude Code's `Task`,
    /// a row of the plan.
    #[serde(default)]
    pub subagent_tools: Vec<String>,
    /// Its tools of the kind `other` under the name Claude Code gives them (`{ "shell": "Bash", "write": "Write" }`),
    /// by their own name: `_meta.<agent>.toolCall.toolName`, else their title before « · ».
    #[serde(default)]
    pub tools: std::collections::BTreeMap<String, String>,
}

fn yes() -> bool {
    true
}

fn version_args() -> Vec<String> {
    vec!["--version".into()]
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Locate {
    #[serde(default)]
    pub program: Option<String>,
    #[serde(default)]
    pub names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestedDecl {
    pub oldest: String,
    pub newest: String,
}

impl Default for TestedDecl {
    fn default() -> Self {
        Self {
            oldest: "0".into(),
            newest: "0".into(),
        }
    }
}

/// What it can do; what is left out, it cannot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CapsDecl {
    #[serde(default)]
    pub modes: Vec<ModeDecl>,
    #[serde(default)]
    pub effort: bool,
    #[serde(default)]
    pub images: bool,
    #[serde(default)]
    pub pdf: bool,
    #[serde(default)]
    pub slash_commands: bool,
    #[serde(default)]
    pub terminal_view: bool,
    /// It holds a read-only launch (a discussion of the Docs tab): a security guarantee the core checks on the
    /// description of each launch.
    #[serde(default)]
    pub read_only: bool,
    /// Its accounts go by an API key of their own.
    #[serde(default)]
    pub accounts_by_key: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelDecl {
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub effort: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryDecl {
    pub dir: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_item_of_an_unknown_op_is_left_aside_and_the_known_ones_read() {
        let items: Items = serde_json::from_value(json!({ "items": [
            { "op": "text", "text": "Bonjour" },
            { "op": "later", "what": 1 },
            { "op": "turn", "usage": { "input": 3 } },
        ] }))
        .unwrap();
        assert_eq!(
            items.items[0],
            Item::Text {
                text: "Bonjour".into(),
                parent: None,
                id: None,
            }
        );
        assert_eq!(items.items[1], Item::Unknown);
        assert!(matches!(items.items[2], Item::Turn { usage, .. } if usage.input == 3));
    }

    #[test]
    fn a_grant_and_a_variable_never_print_their_secret() {
        let g = Grant {
            url: "http://127.0.0.1:1/mcp".into(),
            token: "secret".into(),
        };
        let v = EnvVar {
            name: "TOKEN".into(),
            value: "secret".into(),
        };
        assert!(!format!("{g:?}{v:?}").contains("secret"));
    }

    #[test]
    fn a_declaration_takes_its_defaults() {
        let d: HarnessDecl = serde_json::from_value(json!({ "id": "x", "name": "X" })).unwrap();
        assert_eq!(d.version_args, ["--version"]);
        assert_eq!(d.tested.oldest, "0");
        assert!(!d.caps.read_only && d.models.is_empty());
        assert!(d.acp.is_none() && d.install.is_none());
    }

    #[test]
    fn an_acp_harness_reads_with_its_modes_by_id_or_with_their_meaning() {
        let d: HarnessDecl = serde_json::from_value(json!({
            "id": "goose", "name": "Goose",
            "caps": { "modes": ["default", { "id": "bypassPermissions", "meaning": { "fr": "Tout", "en": "All" } }] },
            "acp": { "args": ["acp"], "modes": { "bypassPermissions": "auto" } },
            "install": { "unix": "curl …" },
        }))
        .unwrap();
        assert_eq!(d.caps.modes[0], ModeDecl::from("default"));
        assert_eq!(d.caps.modes[1].meaning.as_ref().unwrap().get("fr"), "Tout");
        assert_eq!(d.caps.modes[1].meaning.as_ref().unwrap().get("de"), "All");
        let acp = d.acp.unwrap();
        assert!(acp.fs, "the files go through the client unless it says not");
        assert_eq!(acp.args, ["acp"]);
        assert_eq!(acp.modes["bypassPermissions"], "auto");
        // Items of the contract it adds read too, and the old ones still.
        let items: Items = serde_json::from_value(json!({ "items": [
            { "op": "toolInput", "id": "c", "input": { "file_path": "a" } },
            { "op": "permission", "request": "r", "call": "c", "canAlways": true, "diff": { "path": "a", "newText": "x" } },
            { "op": "permission", "request": "r2", "call": "c2" },
            { "op": "todos", "todos": [] },
            { "op": "turn", "error": "refusé" },
        ] }))
        .unwrap();
        assert!(
            matches!(&items.items[1], Item::Permission { can_always: true, diff: Some(d), .. } if d.old_text.is_none())
        );
        assert!(matches!(&items.items[4], Item::Turn { error: Some(e), .. } if e == "refusé"));
        // A limit without its reset goes and comes back as one, none as none.
        for limited in [None, Some(None), Some(Some(5))] {
            let turn = Item::Turn {
                interrupted: false,
                limited,
                usage: TurnUsage::default(),
                error: None,
                entry: None,
            };
            let back: Item = serde_json::from_value(serde_json::to_value(&turn).unwrap()).unwrap();
            assert_eq!(back, turn);
        }
    }
}
