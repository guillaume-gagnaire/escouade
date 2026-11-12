//! « Vue <harness> »: an agent run in its harness's own interface, in a terminal, in place of the
//! process that Escouade's conversation drives (`tui`). What that takes of a harness, behind one
//! interface (`TerminalView`, `Engine::terminal`):
//! - the launch: its interface's arguments, in the agent's folder, with its account's environment
//!   and Escouade's MCP server, and its hooks pointed at Escouade's hooks' server;
//! - the state: what each of its hooks tells (a turn starts or ends, a permission or a question
//!   waits for the user);
//! - the conversation: its session's log, read as it is written, each entry translated;
//! - the resume: the session found where the interface will look for it, and what the interface
//!   says when it does not find it;
//! - the paste of a message, and the keys that interrupt a turn and end the interface.
//!
//! A harness that calls its hooks as commands (not over HTTP) is given `escouade-hook`
//! (`src/bin/escouade-hook.rs`), which posts what it reads on its input to the same server.

use crate::agent::{AgentRt, Effects, HookOutcome};
use anyhow::Result;
use serde_json::Value;
use std::path::{Path, PathBuf};

use super::Program;

/// How an interface joins the process of its agent (`EngineProcess::attach`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Attach {
    pub args: Vec<String>,
    /// The variables it gets besides its account's: what is secret (a password) goes here, never
    /// in `args`, which the system shows to anyone who lists its processes.
    pub env: Vec<(String, String)>,
}

/// The interface of a harness in a terminal (« Vue <harness> »).
pub trait TerminalView: Send + Sync {
    // ----- the launch -----

    /// The name of its terminal, as the window lists it (`claude`).
    fn shell(&self) -> &'static str;
    /// The program of its interface, from its account's (`program`, the one its agents' processes
    /// run): the same, unless its harness has its interface apart (Mistral Vibe: `vibe` beside
    /// `vibe-acp`).
    fn program(&self, program: &Program) -> Program {
        program.clone()
    }
    /// Variables of the core's own it gives its interface (Cursor CLI: where the core reads its workspaces, for the
    /// tests' fake), beside those of its plugin's launch (`terminal/launch`).
    fn core_env(&self) -> Vec<(String, String)> {
        Vec::new()
    }
    /// What its interface left of the agent once the agent is deleted, in the folder `config_dir` of its account,
    /// beside the files of its own its plugin wrote next to its private file `private_file` (which the core takes away
    /// itself). Nothing, unless its harness keeps something there.
    fn forget(&self, _private_file: &Path, _config_dir: &Path) {}
    /// Its interface cannot copy a session itself: the copy is made before it starts, by its plugin (`account/read`
    /// of `fork`, its answer `{ "session": <the copy's id> }`), and resumed then. False: it copies one itself.
    fn copies_first(&self) -> bool {
        false
    }
    /// The core writes the fence of a private project for its interface itself, in the agent's home, as for its agents
    /// (Mistral Vibe's `config.toml`, Gemini CLI's rules: `BuiltinHarness::prepare`): its plugin's launch need not spell
    /// it.
    fn fence_by_core(&self) -> bool {
        false
    }
    /// Its interface may resume `session`, from the folder `config_dir`: its log is there, by default.
    fn can_resume(&self, config_dir: &Path, session: &str) -> bool {
        self.find_log(config_dir, session).is_some()
    }
    /// Escouade names a new session before the interface starts (its log is then known before any
    /// hook); false: the interface names its own, which its first hook tells.
    fn names_its_session(&self) -> bool;
    /// Its interface opens any session it is given, a new one under that id when it has none
    /// (Cursor): the session to resume is not looked for first, nor ever lost.
    fn resumes_any(&self) -> bool {
        false
    }
    /// Where its interface, started in the agent's folder `cwd`, reads what Escouade tells it of
    /// the conversation so far, when it keeps the conversations of its interface apart from its
    /// session's (Cursor): a file in that folder, written as the view opens, kept out of git and
    /// taken away on the way back. None: the interface resumes the conversation itself.
    fn relay_path(&self, _cwd: &Path) -> Option<PathBuf> {
        None
    }
    /// What that file holds for the relay `text` (`handover::relay`).
    fn relay_file(&self, text: &str) -> Vec<u8> {
        text.as_bytes().to_vec()
    }

    // ----- the state -----

    /// One of its hooks, the agent's state changed by what it tells.
    fn on_hook(&self, rt: &mut AgentRt, hook: &Value, fx: &mut Effects) -> HookOutcome;
    /// The folder where it lists its running interfaces (`find_running`), in the account's
    /// config folder `config_dir`; None: it lists none.
    fn running_dir(&self, config_dir: &Path) -> Option<PathBuf>;
    /// Its interface of `session` among those `dir` lists, the file that lists it (`known`, once
    /// found) read alone: it is up, past what it asks before it reads a message. Never blocks for
    /// long nor fails.
    fn find_running(
        &self,
        dir: &Path,
        session: &str,
        known: Option<&Path>,
    ) -> Option<crate::tui::Registered>;

    // ----- the conversation -----

    /// The log of `session` in the account's config folder `config_dir` (the agent's own folder,
    /// when its harness gives it one: `Engine::agent_home_var`).
    fn find_log(&self, config_dir: &Path, session: &str) -> Option<PathBuf>;
    /// The log of a session goes on in another file while its interface runs (Mistral Vibe: a
    /// segment of its journal for each turn): `find_log` is asked again once the file read is read
    /// to its end, and the new file is read from its start.
    fn log_moves(&self) -> bool {
        false
    }
    /// One entry of its log, translated into the agent's conversation.
    fn on_log_entry(&self, rt: &mut AgentRt, entry: &Value, fx: &mut Effects);

    // ----- the resume -----

    /// Whether what the interface wrote before it quit says it did not find the session it was to
    /// resume.
    fn says_session_lost(&self, output: &[u8]) -> bool;
    /// Copies the session `session` from the account's config folder `from` into `to`'s.
    fn copy_session(&self, from: &Path, to: &Path, session: &str) -> Result<()>;

    // ----- the paste and the interruption -----

    /// Why a message may not be pasted in it now, though it is up (in the agent's folder `cwd`,
    /// its account's configuration folder `config_dir`; `spoke`: it told something since it
    /// started, a hook or an entry of its log): a question of its own on its screen, which a text
    /// typed would answer. None: it may.
    fn paste_refusal(&self, _config_dir: &Path, _cwd: &str, _spoke: bool) -> Option<String> {
        None
    }

    /// What is typed in it for a message `text`, before `submit` sends it: pasted, so that its
    /// line breaks stay in it.
    fn paste(&self, text: &str) -> Vec<u8>;
    /// The key that sends what was pasted.
    fn submit(&self) -> &'static [u8];
    /// The types of the images it takes as a path pasted in it (none: it takes no image so).
    fn image_types(&self) -> &'static [&'static str];
    /// The key that interrupts its turn.
    fn interrupt_key(&self) -> &'static [u8];
    /// When its interrupt key has to be pressed again to confirm (Copilot: « esc again to
    /// interrupt »), how long after the first: None, once is enough.
    fn interrupt_again(&self) -> Option<std::time::Duration> {
        None
    }
    /// The key that ends it for good when sent twice, `quit_gap` apart.
    fn quit_key(&self) -> &'static [u8];
    /// Its turn was interrupted from the window (`interrupt_key` sent): nothing to do, unless its
    /// interface tells nothing of a turn it stopped (Gemini CLI: no hook then), whose end is then
    /// the agent's.
    fn interrupted(&self, _rt: &mut AgentRt, _fx: &mut Effects) {}
}

/// A text pasted in a terminal (bracketed paste), the sequence's own escape taken out of it, and
/// its line breaks as a terminal pastes them (xterm.js too): `\r`. Claude Code under Windows drops
/// a `\n` pasted (ESC-12).
pub fn bracketed(text: &str) -> Vec<u8> {
    let text = text
        .replace('\x1b', "")
        .replace("\r\n", "\r")
        .replace('\n', "\r");
    format!("\x1b[200~{text}\x1b[201~").into_bytes()
}

/// The name of the `escouade-hook` command on this platform.
pub const HOOK_COMMAND: &str = if cfg!(windows) {
    "escouade-hook.exe"
} else {
    "escouade-hook"
};

/// `escouade-hook`, beside the app's program (where the installers put it, and `cargo build`
/// too). In the tests, the app's program is the test binary, in `deps`: it is one folder up.
pub fn hook_command() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let found = [Some(dir), dir.parent()]
        .into_iter()
        .flatten()
        .map(|d| d.join(HOOK_COMMAND))
        .find(|p| p.is_file());
    found
}
