//! The harnesses that run Escouade's agents, behind one interface (see docs/ENGINES.md).
//!
//! An engine (« un harness » in the interface: Claude Code first) starts the process of an agent
//! (a new conversation, a resumed one or a copy), and that process takes its messages, its
//! interruptions, its changes of model, effort and mode, and the answers to its permissions and
//! questions. What the process says comes back as frames, which the engine translates into the
//! agent's conversation (`Engine::translate`). The rest of the backend speaks to an engine through
//! these two traits only.

mod absent;
pub use absent::line as absent_line;
pub(crate) mod agy;
mod claude;
pub(crate) mod codex;
pub(crate) mod copilot;
pub(crate) mod cursor;
pub(crate) mod gemini;
pub mod jsonc;
pub mod mcp;
pub mod oneshot;
pub(crate) mod opencode;
mod program;
pub mod registry;
pub mod terminal;
pub mod userconfig;
pub(crate) mod vibe;

#[cfg(test)]
pub(crate) mod bench;
#[cfg(test)]
pub(crate) mod minimal;
#[cfg(test)]
mod userconfig_tests;

#[cfg(test)]
pub(crate) use agy::bench::fake_agy;
pub use agy::AGY;
pub(crate) use claude::terminal as claude_terminal;
#[cfg(test)]
pub use claude::told_auto as claude_told_auto;
pub use claude::{
    catalog as claude_catalog, missing_line as claude_missing_line,
    told_model as claude_told_model, CLAUDE,
};
pub(crate) use codex::git as codex_git;
pub use codex::CODEX;
#[cfg(test)]
pub(crate) use codex::{fake_codex, is_plan_approval as codex_is_plan_approval};
pub(crate) use codex::{
    id_key as codex_request_id, meters as codex_meters, mode_of as codex_mode_of, plan_approval,
};
#[cfg(test)]
pub(crate) use copilot::fake_copilot;
#[cfg(test)]
pub(crate) use copilot::limit_answer as copilot_limit_answer;
pub use copilot::COPILOT;
pub(crate) use copilot::{
    call_nano as copilot_call_nano, credits as copilot_credits,
    limit_choices as copilot_limit_choices, limit_question as copilot_limit_question,
    no as copilot_no, yes as copilot_yes,
};
#[cfg(test)]
pub(crate) use cursor::fake_cursor;
pub use cursor::CURSOR;
pub use gemini::GEMINI;
pub use mcp::Escouade;
#[cfg(test)]
pub(crate) use opencode::fake_opencode;
pub use opencode::OPENCODE;
pub use program::{Launch, Located, Program};
pub use terminal::TerminalView;
#[cfg(test)]
pub(crate) use vibe::bench::fake_vibe;
pub use vibe::VIBE;

#[cfg(test)]
pub(crate) use claude::{
    all_tools as claude_all_tools, claude_args, escaped_len, ClaudeProcess, SpawnOpts,
    PROTOCOL_BUDGET,
};

use crate::agent::{AgentRt, Effects};
use crate::core::Attachment;
use crate::job::JobUsage;
use crate::mcp::install::{Entry, Standing, Target};
use crate::model::{AgentMeta, EngineId};
use anyhow::Result;
use serde_json::Value;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

/// What an asynchronous method of the traits gives (they are used as trait objects).
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The live process of an agent, whatever its engine.
pub type Process = Arc<dyn EngineProcess>;

/// Receives every frame of the process but the answers to the requests it was asked.
pub type OnFrame = Box<dyn Fn(Value) + Send + Sync>;

/// Runs once the process is gone, with its exit code and the end of what it said besides its
/// frames (its stderr).
pub type OnExit = Box<dyn FnOnce(Option<i32>, String) + Send>;

/// The engine of an agent, as its `AgentMeta.engine` says.
pub fn of(meta: &AgentMeta) -> &'static dyn Engine {
    by_id(meta.engine)
}

/// The engine `id` names, from the registry: one it does not have is « harness absent »
/// (`absent`), which starts nothing.
pub fn by_id(id: EngineId) -> &'static dyn Engine {
    #[cfg(test)]
    if id == EngineId::BENCH {
        return &minimal::MINIMAL;
    }
    registry::find(id).unwrap_or_else(|| absent::of(id))
}

/// Whether the harness `id` names is written (the registry has it): only those take an agent,
/// made or handed over; the others are « harness absent ».
pub fn written(id: EngineId) -> bool {
    #[cfg(test)]
    if id == EngineId::BENCH {
        return true;
    }
    registry::find(id).is_some()
}

/// The name of the harness `id` names, as the interface says it: its own when the registry has
/// it, else its id (the data of a later version, or of a plugin gone, may name one this version
/// does not have).
pub fn by_name(id: EngineId) -> &'static str {
    by_id(id).name()
}

// The engine of what has no agent yet (a question about a project, the settings).
pub fn default() -> &'static dyn Engine {
    &CLAUDE
}

/// Where a conversation starts from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Start {
    /// A new conversation.
    New,
    /// The session `session`, where it was left.
    Resume { session: String },
    /// A copy of the session `of`, under a session of its own (the original's is left as it is):
    /// as it was at `at` (the original's last message when it was copied), or all of it.
    Fork { of: String, at: Option<String> },
}

impl Start {
    /// What the agent of `meta` starts from: its session, else its original's for a copy that has
    /// none yet.
    pub fn of(meta: &AgentMeta) -> Self {
        if let Some(session) = &meta.session_id {
            Start::Resume {
                session: session.clone(),
            }
        } else if let Some(of) = &meta.fork_of {
            Start::Fork {
                of: of.clone(),
                at: meta.fork_at.clone(),
            }
        } else {
            Start::New
        }
    }
}

/// How to start the process of an agent.
pub struct Spawn {
    /// Its harness's program, of its account.
    pub program: Program,
    /// Its folder.
    pub cwd: String,
    /// The variables it gets on top of the app's.
    pub env: Vec<(String, String)>,
    /// Its model, effort, mode and added instructions (`AgentMeta`), and where it starts from.
    pub model: String,
    pub effort: String,
    pub mode: String,
    pub append_prompt: Option<String>,
    /// What was said in its harness's own interface, which its session does not have: told with its
    /// next message (`AgentMeta::relay`, Cursor).
    pub relay: Option<String>,
    pub start: Start,
    /// Folders it reads without a question: those of the files joined to its messages and of its
    /// ticket's files.
    pub read_dirs: Vec<PathBuf>,
    pub escouade: Escouade,
    /// The paths its tools are refused (« Projet privé », `fence`): each harness is told them its
    /// way.
    pub fence: crate::fence::Fence,
    /// A settings file of its own (Claude Code: `--settings`, `Engine::fence_settings`), written
    /// for its fence: it holds the refusal of Escouade's server too, Claude Code reading one only.
    pub settings: Option<PathBuf>,
    /// Who it is and the account it runs on, for the log.
    pub label: String,
    pub account: String,
    /// Whether its model takes an effort, as the `claude` of its account told it (ESC-124); None:
    /// its harness's rule (`Engine::supports_effort`).
    pub takes_effort: Option<bool>,
    /// A discussion of the Docs tab (ESC-261): it changes no file and runs no command, in any mode,
    /// held by its harness itself (`crate::discussion`).
    pub read_only: bool,
}

impl Spawn {
    /// The start of the agent of `meta` with `program`, in its folder, from where it was left.
    pub fn of(meta: &AgentMeta, program: Program, env: Vec<(String, String)>) -> Self {
        Spawn {
            program,
            cwd: meta.cwd.clone(),
            env,
            model: meta.model.clone(),
            effort: meta.effort.clone(),
            mode: meta.mode.clone(),
            append_prompt: meta.append_prompt.clone(),
            relay: meta.relay.clone(),
            start: Start::of(meta),
            read_dirs: Vec::new(),
            escouade: Escouade::Denied { file: None },
            fence: crate::fence::Fence::default(),
            settings: None,
            label: format!("agent {}", meta.id),
            account: meta.account.clone(),
            takes_effort: None,
            read_only: meta.discussion.is_some(),
        }
    }
}

/// What a process says once it is ready.
#[derive(Debug, Clone, Default)]
pub struct Ready {
    /// Its slash commands, as the composer lists them.
    pub commands: Vec<Value>,
    /// The models it offers, as it tells them (`ModelInfo::list` reads them: Claude Code's).
    pub models: Value,
    /// The models of its account, as a harness that lists them tells them (Codex: `model/list`):
    /// the composer's menu of its agents, their efforts and what they read.
    pub catalog: Option<Vec<registry::ToldModel>>,
}

/// How `/compact` typed in an agent's conversation goes to its harness (ESC-205).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compaction {
    /// As any message: its harness reads the command itself (Claude Code), or has none.
    Message,
    /// By a request of its protocol (`EngineProcess::compact`) that takes no instruction.
    Bare,
    /// By a request of its protocol that takes the instruction written after the command.
    WithInstructions,
    /// Not from Escouade: its protocol has no request for it, and its model would take the
    /// command for a message. Its interface's own command does it (Cursor CLI: `/summarize`).
    InItsInterface(&'static str),
}

/// The command, the user's instruction after it (`/compact keep the decisions`) when it is
/// `/compact`; None for any other text.
pub fn compact_command(text: &str) -> Option<&str> {
    let rest = text.trim().strip_prefix("/compact")?;
    (rest.is_empty() || rest.starts_with(char::is_whitespace)).then(|| rest.trim())
}

/// The `/` commands the composer lists for a harness that tells none to a client (`Ready`): its
/// `/compact` when it compacts by a request (`Compaction`), none otherwise. Known without
/// starting its process.
pub fn fixed_commands(harness: &dyn Engine) -> Vec<Value> {
    let how = harness.compaction();
    if !matches!(how, Compaction::Bare | Compaction::WithInstructions) {
        return Vec::new();
    }
    let mut entry = serde_json::json!({
        "name": "compact",
        "description": tr!("Compacter le contexte de la conversation", "Compact the conversation’s context"),
    });
    if how == Compaction::WithInstructions {
        entry["argumentHint"] = serde_json::json!(tr!("[ce qu’il faut garder]", "[what to keep]"));
    }
    vec![entry]
}

/// A message to an agent, in its engine's own form (`Engine::message`).
#[derive(Debug, Clone, PartialEq)]
pub struct Message(pub(crate) Value);

/// The answer to a permission the agent asks (`call` is the tool call it is about).
#[derive(Debug, Clone, PartialEq)]
pub enum PermissionAnswer {
    /// Allowed once, with `input` (the tool's, maybe changed). A plan approved: its work goes on
    /// in the mode `resume` (`AgentMeta::plan_resume`, ESC-277), `default` without one.
    Allow {
        call: String,
        input: Value,
        resume: Option<String>,
    },
    /// Allowed, and the `rules` the engine suggested kept for the rest of the session.
    Always {
        call: String,
        input: Value,
        rules: Value,
    },
    /// Refused, the agent told `message`.
    Deny { call: String, message: String },
}

/// The weight of a one-shot question (`Engine::one_shot`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weight {
    /// A short text (a name, a title, a message): the engine's fastest model, and no tool.
    Light,
    /// A text written after reading its folder: a stronger model, with tools that only read.
    Reading,
}

/// One question asked of a harness with no session, no settings of the user's and no MCP server.
pub struct OneShot<'a> {
    pub program: Program,
    pub env: Vec<(String, String)>,
    pub weight: Weight,
    pub cwd: &'a std::path::Path,
    pub system: &'a str,
    pub prompt: &'a str,
    /// Past it, the process is killed with all it started.
    pub limit: Duration,
    /// The model it is asked of, for a harness that takes the one Escouade chooses
    /// (`Engine::writer_takes_model`: OpenCode); empty for the others, which pick their own.
    pub model: String,
}

/// The room of the instructions added to an agent (`Engine::instructions_room`): at most `max`,
/// each text weighing what `weigh` says (its weight once passed, escaped, to the process).
#[derive(Clone, Copy)]
pub struct Room {
    pub max: usize,
    pub weigh: fn(&str) -> usize,
}

/// Why a process that never got going stopped, as the engine read it from what it said.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartLost {
    /// The session it was to resume (or a copy's original's) is gone.
    Session,
    /// The point of the original's session a copy was to start from is not there.
    ForkPoint,
}

/// Why a harness cannot be started, its program not found, in `lang`: for Claude Code, its
/// installation cut half-way or not found, with the command that puts it back (ESC-133).
pub fn missing_line(id: crate::model::EngineId, lang: crate::i18n::Lang) -> String {
    let harness = by_id(id);
    if let Some(line) = harness.missing_repair(lang) {
        return line;
    }
    let name = harness.name();
    tr_in!(
        lang,
        "{name} introuvable. Installe-le ou indique son chemin dans les réglages.",
        "{name} not found. Install it or give its path in the settings."
    )
}

/// `Engine::no_self_update` as variables to give a process.
pub fn no_self_update_env(engine: &dyn Engine) -> impl Iterator<Item = (String, String)> {
    engine
        .no_self_update()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
}

/// A harness: Claude Code, then Codex, Copilot CLI, Cursor CLI, OpenCode.
pub trait Engine: Send + Sync {
    /// Its id, as the data keeps it (`claude`).
    fn id(&self) -> EngineId;
    /// Its name, as the interface says it (`Claude Code`).
    fn name(&self) -> &'static str;
    /// The name of its agents as the texts speak of them (« Claude attend ta réponse »): `Claude`.
    fn speaker(&self) -> &'static str;

    /// What the instructions added to an agent (`Spawn.append_prompt`: a ticket's protocol) may
    /// weigh as it passes them; None when they are not bounded.
    fn instructions_room(&self) -> Option<Room>;
    /// What the agent is told, in `lang`, to pass `text` on to its subagent `title` (its task
    /// `task`, as the engine named it); None when its subagents cannot be written to so.
    fn subagent_relay(
        &self,
        lang: crate::i18n::Lang,
        title: &str,
        task: &str,
        text: &str,
    ) -> Option<String>;

    /// Its program, from the path of a setting (empty: where it usually is) and the distribution
    /// of WSL of the settings.
    fn locate(&self, configured: &str, wsl_distro: &str) -> Option<Program>;

    /// The versions Escouade was tested with: an older one is refused (`oldest_reason` says why),
    /// a newer one taken with a warning (`registry`).
    fn tested(&self) -> registry::Tested;
    /// Why `version` is refused whatever the tested range, in `lang`: a line of the harness
    /// Escouade does not drive (OpenCode 2). None for the others.
    fn unsupported_line(
        &self,
        _lang: crate::i18n::Lang,
        _version: &registry::Version,
    ) -> Option<String> {
        None
    }
    /// Why a version older than `tested().oldest` is refused, in `lang`.
    fn oldest_reason(&self, lang: crate::i18n::Lang) -> String;
    /// The command that updates it, as the refusal tells it.
    fn update_command(&self) -> &'static str;
    /// The arguments that make its program print its version.
    fn version_args(&self) -> &'static [&'static str];
    /// What it can do, as the interface reads it (`registry::EngineCaps`).
    fn caps(&self) -> registry::EngineCaps;
    /// What tells its accounts apart (`registry::AccountStore`): for Claude Code, a configuration
    /// folder of their own given by `CLAUDE_CONFIG_DIR`.
    fn accounts(&self) -> registry::AccountStore;
    /// The variable that gives each agent a configuration folder of its own (`DataDir::agent_home`),
    /// whatever its account: what the harness keeps there holds for that agent alone. Cursor CLI:
    /// `CURSOR_CONFIG_DIR`, where its « Toujours » writes its rules (decided by the user on
    /// 2026-10-25). None: its account's folder.
    fn agent_home_var(&self) -> Option<&'static str> {
        None
    }
    /// The variables every process of it is given so that it never updates itself (ESC-133):
    /// Escouade stops its processes with their whole tree, so an update it started (`npm install
    /// -g` under Claude Code) would be cut half-way, the program gone. Escouade updates it itself
    /// (`selfupdate`).
    fn no_self_update(&self) -> &'static [(&'static str, &'static str)] {
        &[]
    }

    /// Its terminal view (« Vue <harness> ») is its interface joined to the agent's process
    /// (`EngineProcess::attach`), which goes on: both views work at once (OpenCode). False: its
    /// interface takes the process's place (`terminal`).
    fn attaches(&self) -> bool {
        false
    }
    /// Its own interface in a terminal (« Vue <harness> ») that takes the process's place; None
    /// when it has none (`caps().terminal_view` says whether it has one, joined or not).
    fn terminal(&self) -> Option<&'static dyn TerminalView>;
    /// The command that installs it (« Aucun harness trouvé »).
    fn install(&self) -> registry::Install;
    /// The models the composer offers, in its menu's order.
    fn models(&self) -> Vec<registry::ModelDecl>;
    /// The efforts the composer offers, in its menu's order (none without `caps().effort`).
    fn efforts(&self) -> &'static [&'static str];

    /// The model an agent handed over to it runs on unless « Modèle de passation » says another:
    /// its best, the first of its menu.
    fn handover_model(&self) -> String {
        self.models()
            .first()
            .map(|m| m.id.to_string())
            .unwrap_or_default()
    }
    /// The size of the context window of `model`, in tokens: what bounds the handover file it is
    /// given (`handover`).
    fn context_window(&self, _model: &str) -> u64 {
        200_000
    }

    /// The models of an account none of whose agents has listed them yet, read by a process of its
    /// own (`program` with `env`, in `dir`): what an agent whose model is none of them starts on.
    /// None when the harness reads none so (its processes list them at their start, `Ready`).
    fn list_models<'a>(
        &'a self,
        _program: Program,
        _env: Vec<(String, String)>,
        _dir: &'a Path,
    ) -> BoxFuture<'a, Result<Option<Vec<registry::ToldModel>>>> {
        Box::pin(async { Ok(None) })
    }

    /// The quota of an account none of whose agents runs, read by a process of its own (`program`
    /// with `env`, its folder `dir`): None when the harness reads none so (Claude Code's endpoint
    /// is asked by the core).
    fn read_quota<'a>(
        &'a self,
        _program: Program,
        _env: Vec<(String, String)>,
        _dir: &'a Path,
    ) -> BoxFuture<'a, Result<Option<crate::usage::Reading>>> {
        Box::pin(async { Ok(None) })
    }

    /// Whether `model` takes an effort.
    fn supports_effort(&self, model: &str) -> bool;
    /// Whether `model` takes the mode `auto`.
    fn supports_auto(&self, model: &str) -> bool;

    /// Starts the process of an agent: a new conversation, a resumed one or a copy, as
    /// `spawn.start` says. `on_frame` and `on_exit` are called from the tasks that read it.
    fn spawn(&self, spawn: Spawn, on_frame: OnFrame, on_exit: OnExit) -> Result<Process>;

    /// `text` with the files `attachments`, as its process takes a message; refused when a file
    /// is of a kind it does not read, or too big.
    fn message(&self, text: &str, attachments: &[Attachment]) -> Result<Message>;

    /// One frame of the agent's process, translated into its conversation: the normalized items
    /// (`user`, `text`, `thinking`, `tool`, `question`, `permission`, `turn`, `notice`, `event`),
    /// the changes of its plan, its status and usage.
    fn translate(&self, rt: &mut AgentRt, frame: &Value, fx: &mut Effects);

    /// Why a process that stopped before it was ready could not start, from its stderr; None for
    /// any other reason.
    fn start_lost(&self, stderr: &str) -> Option<StartLost>;

    /// One question, its answer (`OneShot`).
    fn one_shot<'a>(&'a self, ask: OneShot<'a>) -> BoxFuture<'a, Result<String>>;
    /// The model that answers its one-shot questions of `weight`, as the interface names it
    /// (« Modèle des textes rédigés »: `Claude Haiku`).
    fn writer(&self, weight: Weight) -> &'static str;
    /// Its one-shot questions go to the model Escouade chooses (`OneShot::model`: the setting
    /// « Modèle » of the texts, else its default one), rather than one of its own: OpenCode, whose
    /// models are those of the providers connected to it.
    fn writer_takes_model(&self) -> bool {
        false
    }

    // ----- Escouade's MCP server, for its agents (`engine::mcp`) -----

    /// What the private file of a process given Escouade's server at `url` with `token` holds
    /// (written before the process starts, gone when it stops); None: the engine passes the
    /// server otherwise (`escouade_launch`), and no file is written.
    fn grant_file(&self, url: &str, token: &str) -> Option<Vec<u8>>;
    /// What a process refused the server reads from a file, the same for every agent; None: its
    /// arguments alone keep the server out.
    fn refusal(&self) -> Option<Value>;
    /// The arguments and variables of a process given or refused the server, as `escouade` says.
    /// Never the token among the arguments: a file or a variable holds it.
    fn escouade_launch(&self, escouade: &Escouade) -> mcp::Launch;

    /// The settings that tell its process the paths of `fence` refused, as `program` names them,
    /// with its refusal of Escouade's server when `refused` (`refusal`): written in a file of the
    /// agent's own (`Spawn::settings`). None: the harness is told its fence otherwise.
    fn fence_settings(
        &self,
        _refused: bool,
        _fence: &crate::fence::Fence,
        _program: &Program,
    ) -> Option<Value> {
        None
    }

    // ----- « <harness> peut piloter Escouade » (`mcp::install`) -----

    /// The user config Escouade writes the server's entry in itself, for a harness whose CLI
    /// cannot (`userconfig`): with it, the methods below need not be written. None: the harness
    /// declares the server by its own commands (Claude Code: `claude mcp add`), or not at all.
    fn user_config(&self) -> Option<&'static userconfig::UserConfig> {
        None
    }
    /// How `/compact` goes to it.
    fn compaction(&self) -> Compaction {
        Compaction::Message
    }
    /// What its user config `config` (`Target.config`) says of Escouade's server.
    fn declared(&self, config: &Path) -> Standing {
        match self.user_config() {
            Some(user) => user.declared(config),
            None => Standing::NoFile,
        }
    }
    /// `entry` added to the user config of the account of `target`, which has none of that name;
    /// `secrets` (the tokens in play) hidden from what an error says.
    fn add_declaration<'a>(
        &'a self,
        target: &'a Target,
        entry: &'a Entry,
        _secrets: &'a [&'a str],
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            match self.user_config() {
                Some(user) => user.add(&target.config, entry),
                None => userconfig::none(),
            }
        })
    }
    /// The server's entry removed from the user config of the account of `target`.
    fn remove_declaration<'a>(
        &'a self,
        target: &'a Target,
        _secrets: &'a [&'a str],
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            match self.user_config() {
                Some(user) => user.remove(&target.config),
                None => userconfig::none(),
            }
        })
    }
    /// The commands that declare `entry` by hand, in the shell of the platform (`windows`:
    /// PowerShell, else a POSIX shell), for an account of folder `config_dir` (none for the
    /// default one), with `program` as the user named it; `replace`: an entry is there, removed
    /// first. A harness with a `user_config`: the entry to paste in it, in place of one there.
    fn manual_declaration(
        &self,
        _windows: bool,
        _program: &str,
        _config_dir: Option<&str>,
        entry: &Entry,
        _replace: bool,
    ) -> String {
        self.user_config()
            .map(|user| user.snippet(entry))
            .unwrap_or_default()
    }

    // ----- what the core asks of a harness, rather than which one it is (INT-3) -----

    // ----- its accounts, as `accounts.rs` asks of them (INT-3) -----

    /// How its accounts keep their folder: where its Principal's is, the variable that moves it,
    /// what a new account may share of it, the arguments that sign in. Nothing by default.
    fn store(&self) -> crate::accounts::Store {
        crate::accounts::Store::NONE
    }
    /// The configuration folder of `account` when it is not found as its `store` says (Claude
    /// Code: the app's `CLAUDE_CONFIG_DIR`, the Linux home folder of a `claude` of WSL).
    fn config_dir(
        &self,
        _account: &crate::model::Account,
        _settings: &crate::model::Settings,
    ) -> Option<PathBuf> {
        None
    }
    /// The path of its program the settings give, its accounts' own aside (« Chemin de claude »).
    fn settings_path<'a>(&self, _settings: &'a crate::model::Settings) -> &'a str {
        ""
    }
    /// The model of its new agents until an account says another (Claude Code's Sonnet); empty:
    /// its own default, `Core::model_for`.
    fn default_model(&self) -> &'static str {
        ""
    }
    /// Which of its accounts go by an API key of their own, in the keychain.
    fn keys(&self) -> crate::accounts::KeyUse {
        crate::accounts::KeyUse::None
    }
    /// What the processes of `account` get besides its folder's variable (`launch_env`): its key,
    /// and the variables of the app's (`inherited` says which are set) that would make it another
    /// account, given empty. Never its command line.
    fn account_env(
        &self,
        _account: &crate::model::Account,
        _inherited: &dyn Fn(&str) -> bool,
    ) -> Vec<(String, String)> {
        Vec::new()
    }
    /// Whether `account`, signed in by its interface rather than by a key, can start a process
    /// (Gemini CLI's signed in to Google in its folder): None for one that goes by its key, or
    /// a harness whose accounts sign in otherwise.
    fn sign_in_ready(&self, _account: &crate::model::Account) -> Option<Result<()>> {
        None
    }
    /// Refused: its account `name`, which goes by its key, has none.
    fn key_missing(&self, name: &str) -> String {
        let harness = self.name();
        tr!(
            "Le compte {harness} « {name} » n’a pas de clé d’API : donne-la dans Réglages › Comptes, « Changer la clé… ».",
            "The {harness} account “{name}” has no API key: give it in Settings › Accounts, “Change the key…”."
        )
    }
    /// Refused: a key given that is no key of its.
    fn key_malformed(&self) -> String {
        let harness = self.name();
        tr!(
            "Ce n’est pas une clé d’API de {harness} : copie-la entière.",
            "That isn’t a {harness} API key: copy it whole."
        )
    }
    /// Whether `account` is signed in, as whom, and its folder. Not signed in by default.
    fn account_status<'a>(
        &'a self,
        account: &'a crate::model::Account,
        settings: &'a crate::model::Settings,
    ) -> BoxFuture<'a, crate::accounts::AccountStatus> {
        let dir = crate::accounts::config_dir(account, settings);
        Box::pin(async move {
            crate::accounts::AccountStatus {
                connected: false,
                email: None,
                dir: dir.to_string_lossy().into_owned(),
                stamp: None,
            }
        })
    }
    /// Its machine's account goes by a key or Vertex AI of the app's alone (Gemini CLI, ESC-275):
    /// whether the app has one. None for a harness whose machine's account signs in.
    fn machine_key(&self) -> Option<bool> {
        None
    }
    /// Its accounts are signed in by its own interface, after they are added (Antigravity CLI).
    fn signs_in_later(&self) -> bool {
        false
    }
    /// Its sessions are in its agents' own folder whatever their account (Mistral Vibe,
    /// Antigravity CLI): none is copied when an agent changes account.
    fn sessions_in_agent_home(&self) -> bool {
        false
    }
    /// The name of the terminal of « Se connecter… ».
    fn login_shell(&self) -> String {
        self.name().to_lowercase()
    }
    /// What « Se connecter… » sets before its terminal starts, in the folder of `account` and in
    /// `env` (Gemini CLI: its interface straight to the Google sign-in, the home folder trusted).
    fn before_login(&self, _account: &crate::model::Account, _env: &mut Vec<(String, String)>) {}

    /// Its models go by aliases it resolves itself (Claude Code: `opus`, `sonnet[1m]`): any name
    /// is taken as it is, the versions they run told per account by its start (`Ready::models`),
    /// and whether one takes an effort or the mode `auto` told by them (`told_effort`).
    fn models_by_alias(&self) -> bool {
        false
    }
    /// Whether `model` takes an effort, as `models` (what its start told for an account) say:
    /// None when they say nothing of it, the harness's rule then (`supports_effort`).
    fn told_effort(&self, _models: &[crate::model::ModelInfo], _model: &str) -> Option<bool> {
        None
    }
    /// `told_effort` for the mode `auto` (`supports_auto`).
    fn told_auto(&self, _models: &[crate::model::ModelInfo], _model: &str) -> Option<bool> {
        None
    }
    /// What a harness whose `models_by_alias` tells of its models at its start, asked of a process
    /// of its own (`program` with `env`, in `dir`) without a turn (Claude Code's `initialize`,
    /// ESC-124): Null when it tells none.
    fn read_told_models<'a>(
        &'a self,
        _program: Program,
        _env: Vec<(String, String)>,
        _dir: &'a Path,
    ) -> BoxFuture<'a, Result<Value>> {
        Box::pin(async { Ok(Value::Null) })
    }
    /// Its declared models stand in only while its account listed none (Codex's GPT-6-Luna,
    /// ESC-186); the others' declare aliases their lists lack (Copilot's `auto`, Cursor's
    /// `default`), which always go.
    fn listed_models_only(&self) -> bool {
        false
    }
    /// The models an account of its lists (`list_models`), with what its folder `config_dir` adds
    /// (Codex: the model its `config.toml` names, ESC-186).
    fn account_models(
        &self,
        listed: Vec<registry::ToldModel>,
        _config_dir: &Path,
    ) -> Vec<registry::ToldModel> {
        listed
    }
    /// What it says when its program is not found, in place of « introuvable » (Claude Code: its
    /// installation cut half-way, or the command that installs it, ESC-133). Reads the disk.
    fn missing_repair(&self, _lang: crate::i18n::Lang) -> Option<String> {
        None
    }
    /// What is added to the reason it stopped when it stopped before it started (Claude Code: its
    /// installation cut half-way, with its repair, ESC-133).
    fn interrupted_line(&self, _lang: crate::i18n::Lang) -> Option<String> {
        None
    }
    /// Its accounts sign in to claude.ai by OAuth, whose sign-in Escouade reads and watches
    /// (`usage.rs`, `signin.rs`): their quota read at its usage endpoint, the refresh of an
    /// out-of-date token taken in turn before a process starts (Claude Code).
    fn oauth_sign_in(&self) -> bool {
        false
    }
    /// The folder where it keeps what it remembers of the project at `project_path`, for an
    /// account of folder `config_dir`, offered once to the shared memory (Claude Code's
    /// `projects/<path>/memory`, `memory::claude_offer`): None when it keeps none so.
    fn memory_dir(&self, _config_dir: &Path, _project_path: &str) -> Option<PathBuf> {
        None
    }
    /// The folder it was in at the latest entry of its session `session_id`, kept in the account's
    /// folder `config_dir` (Claude Code: each entry's `cwd`, which follows `EnterWorktree` and the
    /// `cd` of its commands, `agentprs::last_cwd`): the worktree an agent went into by itself
    /// (ESC-161). None when its session does not say it.
    fn session_cwd(&self, _config_dir: &Path, _session_id: &str) -> Option<String> {
        None
    }
    /// It declares Escouade's MCP server by its own program (Claude Code: `claude mcp add`,
    /// `add_declaration`), without a user config Escouade writes (`user_config`).
    fn declares_by_program(&self) -> bool {
        false
    }
    /// How its accounts' configuration folders keep their sessions, which the closing of private
    /// projects refuses (`fence::homes`): as Claude Code's unless it says otherwise; None when they
    /// are kept elsewhere (OpenCode's data folder).
    fn session_home(&self) -> Option<crate::fence::HomeKind> {
        Some(crate::fence::HomeKind::Claude)
    }
    /// The user's own folder of it keeps sessions of every project, refused too (`~/.claude`,
    /// `~/.codex`, `~/.copilot`).
    fn principal_sessions(&self) -> bool {
        false
    }
    /// The folders of a repository its commands, skills and agents are read from for the
    /// Workflow tab (ESC-191), and what each holds: none for a harness that has none.
    fn workflow_folders(&self) -> crate::workflow::Folders {
        &[]
    }
    /// Its user's own folder the Workflow tab reads too, a project's of the same name hiding it.
    fn workflow_user(&self) -> Option<crate::workflow::UserRoot> {
        None
    }
    /// What an agent's track in a workflow cannot say on it (`workflow_track::support_of`), in its
    /// conversation or in its own interface (`view`): `subagents`, `cost`, `timing`.
    fn track_missing(&self, _view: bool) -> &'static [&'static str] {
        &[]
    }
    /// The answer of its hooks goes into its agent's context (`additionalContext` of a
    /// `PreToolUse`: Claude Code's HTTP hooks, Codex's commands): the index's hint (ESC-294).
    fn hook_context(&self) -> bool {
        false
    }
    /// It signs in to an account of its service (OpenCode signs in to none: its providers are its
    /// own, ESC-45).
    fn signs_in(&self) -> bool {
        true
    }
    /// The account is signed in, as its files tell it without asking its program (Claude Code:
    /// its `.claude.json` names the claude.ai account it signed in with).
    fn signed_in_by_files(
        &self,
        _account: &crate::model::Account,
        _settings: &crate::model::Settings,
    ) -> bool {
        false
    }
    /// Its program as the user types it, which a short error names (`claude introuvable`): its
    /// name for most.
    fn command(&self) -> &'static str {
        self.name()
    }
    /// The model its texts are priced by (`pricing`), for `weight`: its writer's (Claude Code: its
    /// alias, `haiku`).
    fn writer_priced(&self, weight: Weight) -> String {
        self.writer(weight).to_string()
    }
}

/// The live process of an agent.
pub trait EngineProcess: Send + Sync {
    // ----- the process -----

    fn is_alive(&self) -> bool;
    /// What the process and everything it started use.
    fn usage(&self) -> Option<JobUsage>;
    /// How many processes of its tree run (at least one while it has not been reaped).
    fn still_running(&self) -> u32;
    /// The ids of the processes of its tree.
    fn pids(&self) -> Vec<u32>;
    /// Closes its input: it finishes what it does and exits, its session resumable (the gentle
    /// stop).
    fn close_input(&self);
    /// Kills it and its whole tree.
    fn kill(&self);
    /// Waits until it is gone, at most `within`; false when it still runs.
    fn wait_exit(&self, within: Duration) -> BoxFuture<'_, bool>;
    /// Waits until it and everything it started are gone, at most `within`.
    fn wait_gone(&self, within: Duration) -> BoxFuture<'_, bool>;

    // ----- the conversation -----

    /// Asks it to get ready, once started: its commands and models.
    fn initialize(&self) -> BoxFuture<'_, Result<Ready>>;
    /// Sends a message (`Engine::message`) under the id `uuid`.
    fn send_message(&self, uuid: &str, message: &Message) -> Result<()>;
    /// Stops the running turn, and the messages queued after it with `cancel_queued`.
    fn interrupt(&self, cancel_queued: bool, timeout: Duration) -> BoxFuture<'_, Result<()>>;
    fn set_model<'a>(&'a self, model: &'a str) -> BoxFuture<'a, Result<()>>;
    fn set_effort<'a>(&'a self, effort: &'a str) -> BoxFuture<'a, Result<()>>;
    fn set_mode<'a>(&'a self, mode: &'a str) -> BoxFuture<'a, Result<()>>;
    /// Answers the permission `request_id`.
    fn answer_permission(&self, request_id: &str, answer: PermissionAnswer) -> Result<()>;
    /// Answers the question `request_id` (about the call `call`): `input` is the question's, with
    /// its `answers`.
    fn answer_question(&self, request_id: &str, call: &str, input: Value) -> Result<()>;
    /// The quota of its account, as the process reads it: its windows (`usage::parse_windows`),
    /// None when it tells none.
    fn read_usage(&self) -> BoxFuture<'_, Result<Option<Value>>>;
    /// Gives its session the title `title`.
    fn rename<'a>(&'a self, title: &'a str) -> BoxFuture<'a, Result<()>>;

    // ----- what Claude Code alone does for now -----

    /// Stops its background task (or subagent) `task`.
    fn stop_task<'a>(&'a self, task: &'a str) -> BoxFuture<'a, Result<()>>;
    /// What its background task `task` printed, as it answers.
    fn task_output<'a>(&'a self, task: &'a str) -> BoxFuture<'a, Result<Value>>;
    /// Links it to claude.ai or unlinks it (Remote Control): `request` says how, its answer the
    /// session it holds there.
    fn remote_control(&self, request: Remote, timeout: Duration) -> BoxFuture<'_, Result<Value>>;
    /// Compacts its conversation's context, `instructions` (none: empty) telling what to keep,
    /// as `Engine::compaction` says it can. True when it is over once its harness answered
    /// (Copilot); false when the harness took the request and runs it as a turn it ends itself
    /// (Codex). What it says meanwhile tells the compaction.
    fn compact<'a>(&'a self, _instructions: &'a str) -> BoxFuture<'a, Result<bool>> {
        Box::pin(std::future::ready(Err(anyhow::anyhow!(
            "no compaction request in this harness"
        ))))
    }

    // ----- its own protocol -----

    /// How its harness's interface joins it in a terminal (`Engine::attaches`): its arguments, and
    /// the variables that hold what is secret (never on its command line). None when it cannot
    /// (no session yet, or a harness whose interface takes the process's place).
    fn attach(&self) -> Option<terminal::Attach> {
        None
    }

    /// A request in its own protocol, its answer: for the tests, and what has no method yet.
    #[cfg_attr(not(test), allow(dead_code))]
    fn control(&self, request: Value, timeout: Duration) -> BoxFuture<'_, Result<Value>>;
    /// Answers the request `request_id` it made, in its own protocol: `Err` refuses it.
    fn reply(&self, request_id: &str, answer: std::result::Result<Value, String>) -> Result<()>;
}

/// A request of Remote Control (`EngineProcess::remote_control`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Remote {
    /// Linked under `name`, the session `reattach` taken up again when there is one; `keep`, the
    /// session stays on claude.ai once the process exits.
    On {
        name: String,
        keep: bool,
        reattach: Option<String>,
    },
    Off,
}
