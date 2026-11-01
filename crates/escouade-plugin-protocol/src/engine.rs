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
    /// Its text; a subagent's under `parent`.
    #[serde(rename_all = "camelCase")]
    Text {
        text: String,
        #[serde(default)]
        parent: Option<String>,
    },
    Thinking {
        text: String,
        #[serde(default)]
        parent: Option<String>,
    },
    /// A tool called, named as Escouade names them (Claude Code's names: `Bash`, `Edit`, `Task`…).
    Tool {
        id: String,
        name: String,
        #[serde(default)]
        input: Value,
        #[serde(default)]
        parent: Option<String>,
    },
    /// Its result.
    ToolDone {
        id: String,
        #[serde(default)]
        output: String,
        #[serde(default)]
        error: bool,
    },
    /// A file it changed (the effect `files_changed`).
    Touched { path: String },
    /// The approval of the call `call`, asked as the request `request`.
    Permission { request: String, call: String },
    /// Questions to the user (each `{question, header, multiSelect, options: [{label, description}]}`).
    Question {
        request: String,
        call: String,
        questions: Value,
    },
    /// The turn is over.
    Turn {
        #[serde(default)]
        interrupted: bool,
        /// Stopped by the usage limit, which resets then (ms), if told.
        #[serde(default)]
        limited: Option<Option<i64>>,
        #[serde(default)]
        usage: TurnUsage,
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
    /// Where it keeps what it remembers of a project, offered once to the shared memory: a folder of files, with
    /// `{configDir}` and `{projectKey}` (the project's path, every character but a letter or a digit a dash).
    #[serde(default)]
    pub memory: Option<MemoryDecl>,
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
    pub modes: Vec<String>,
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
                parent: None
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
    }
}
