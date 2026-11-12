//! « Vue Vibe »: Mistral Vibe's own interface (`vibe`) in a terminal (`TerminalView`), as the real
//! runs of Mistral Vibe 3.0.3 found it (docs/TODO-1.8.md, point 0, « Mistral Vibe », ESC-265 and
//! ESC-268).
//!
//! - Its interface and its ACP server keep their sessions in the same store, the agent's own
//!   `VIBE_HOME` (`logs/session/unified/<id>/`): `vibe --resume <id>` opens the agent's session, its
//!   history there, and Escouade's view loads again what was said in it (no relay, unlike Cursor).
//!   A session ACP opened and never wrote (no message yet) is not in the store: its interface says
//!   « Session not found » and goes on in a new one, so it is not given; the new one is told by
//!   its first hook.
//! - Its log is the journal of the session's store (`journal/<sequence>.jsonl`): the user's
//!   messages, the agent's, its tools and their end, the callbacks it waits on (a permission, a
//!   question) and the end of each turn, translated by `AgentRt::vibe_log`. The store starts a
//!   segment of its journal at the end of each turn and keeps the one before (tried): the log
//!   moves (`log_moves`), the latest segment read once the one before is read to its end.
//! - Its hooks (`hooks.toml` of its `VIBE_HOME`, `pre_tool`, `post_tool`, `post_agent`) run
//!   `escouade-hook`, which tells the session of the interface (a new one included). The file is
//!   written as the view opens, and taken away when the agent's ACP process starts (`forget_hooks`).
//!   They run in a shell, Git Bash or PowerShell under Windows: `escouade-hook` is found by the
//!   `PATH`, its folder put first, as both read the same line.
//! - Escouade's MCP server goes by its variables (`VIBE_MCP_SERVERS`, the token in another
//!   variable its `api_key_env` names), never on its command line nor in a file (tried).
//! - It never asks about the trust of the agent's folder: Escouade writes it in the agent's
//!   `VIBE_HOME` (`trusted_folders.toml`), as for its ACP server; and the refusals of a private
//!   project's paths in its `config.toml`, its agent `auto-approve` kept off with them (it would
//!   pass over them).

use super::super::terminal::{bracketed, TerminalView};
use super::super::Program;
use crate::agent::{AgentRt, Effects, HookOutcome};
use crate::tui::Registered;
use anyhow::Result;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Its interface's name.
const INTERFACE: &str = "vibe";

/// The hooks Escouade writes in the agent's `VIBE_HOME` for its interface.
pub(crate) const HOOKS: &str = "hooks.toml";

/// What its interface says, and goes on in a new session, of a session not in its store.
const NOT_FOUND: &str = "Failed to resume session";

/// Mistral Vibe's interface.
pub struct VibeTerminal;

pub static VIBE_TERMINAL: VibeTerminal = VibeTerminal;

/// The folder of the session `session` in the `VIBE_HOME` `home`.
pub(crate) fn session_dir(home: &Path, session: &str) -> PathBuf {
    home.join("logs")
        .join("session")
        .join("unified")
        .join(session)
}

/// The latest segment of the journal of a session's folder `dir` (they are named by the sequence
/// of their first record, on 16 digits).
pub(crate) fn latest_segment(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir.join("journal"))
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl") && p.is_file())
        .max()
}

/// The interface beside its ACP server (`vibe-acp` → `vibe`, its extension kept); any other program
/// as it is (a fake, which plays both).
pub(crate) fn interface_of(program: &Program) -> Program {
    let Program::Native(path) = program else {
        return program.clone();
    };
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if stem != super::locate::ACP {
        return program.clone();
    }
    let beside = match path.extension() {
        Some(ext) => path.with_file_name(format!("{INTERFACE}.{}", ext.to_string_lossy())),
        None => path.with_file_name(INTERFACE),
    };
    if beside.is_file() {
        Program::Native(beside)
    } else {
        program.clone()
    }
}

/// The hooks of its interface taken away from the agent's `VIBE_HOME` `home`, the agent's ACP
/// process about to start (they would run there too, for nothing).
pub(crate) fn forget_hooks(home: &Path) {
    let _ = std::fs::remove_file(home.join(HOOKS));
}

impl TerminalView for VibeTerminal {
    fn shell(&self) -> &'static str {
        INTERFACE
    }

    fn program(&self, program: &Program) -> Program {
        interface_of(program)
    }

    /// The refusals of a private project are in its `config.toml`, which the core writes (`prepare`).
    fn fence_by_core(&self) -> bool {
        true
    }

    /// It names its own: the session ACP opened and never wrote is not in its store.
    fn names_its_session(&self) -> bool {
        false
    }

    fn on_hook(&self, rt: &mut AgentRt, hook: &Value, fx: &mut Effects) -> HookOutcome {
        rt.vibe_hook(hook, fx)
    }

    fn running_dir(&self, _config_dir: &Path) -> Option<PathBuf> {
        None
    }

    fn find_running(&self, _: &Path, _: &str, _: Option<&Path>) -> Option<Registered> {
        None
    }

    /// The latest segment of the session's journal, in the agent's `VIBE_HOME`.
    fn find_log(&self, home: &Path, session: &str) -> Option<PathBuf> {
        if session.is_empty() || session.contains(['/', '\\', '.']) {
            return None;
        }
        latest_segment(&session_dir(home, session))
    }

    fn log_moves(&self) -> bool {
        true
    }

    fn on_log_entry(&self, rt: &mut AgentRt, entry: &Value, fx: &mut Effects) {
        rt.view_line(entry, fx);
    }

    fn says_session_lost(&self, output: &[u8]) -> bool {
        String::from_utf8_lossy(output).contains(NOT_FOUND)
    }

    /// Its sessions are in the agent's own `VIBE_HOME`, whatever its account: nothing to copy.
    fn copy_session(&self, _from: &Path, _to: &Path, _session: &str) -> Result<()> {
        Ok(())
    }

    fn paste(&self, text: &str) -> Vec<u8> {
        bracketed(text)
    }

    fn submit(&self) -> &'static [u8] {
        b"\r"
    }

    fn image_types(&self) -> &'static [&'static str] {
        &[]
    }

    /// Escape (« Esc/Ctrl+C to interrupt »), which its journal tells as a turn `interrupted`.
    fn interrupt_key(&self) -> &'static [u8] {
        b"\x1b"
    }

    /// Ctrl+C twice (« Press Ctrl+C again to quit »).
    fn quit_key(&self) -> &'static [u8] {
        b"\x03"
    }
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
