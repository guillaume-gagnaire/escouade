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
    /// Stop the running turn (`AgentRef` → `Writes`).
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
    /// For a terminal view: the trust bands its interface asks, shown before it opens.
    #[serde(default)]
    pub trust: Vec<String>,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRef {
    pub agent: String,
}

/// A batch of what the harness wrote: each line as it read it, never read again by the core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frames {
    pub agent: String,
    pub frames: Vec<String>,
}

/// What a batch made.
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnswerQuestion {
    pub agent: String,
    pub request: String,
    pub call: String,
    /// The answers, by question.
    pub answers: Value,
}

/// The quota its process read (`usage::parse_windows` reads its windows); null when none.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Usage {
    #[serde(default)]
    pub usage: Value,
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
    },
    /// Questions to the user (each `{question, header, multiSelect, options: [{label, description}]}`).
    Question {
        request: String,
        call: String,
        questions: Value,
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
    },
    /// A line of the conversation from Escouade or the harness, not the model (`info`, `warning`, `error`).
    Notice {
        text: String,
        #[serde(default = "info")]
        level: String,
    },
    /// A line for the harness's input, written by the core in the order of the items.
    Write { line: String },
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
            };
            let back: Item = serde_json::from_value(serde_json::to_value(&turn).unwrap()).unwrap();
            assert_eq!(back, turn);
        }
    }
}
