//! « Vue Antigravity »: Antigravity CLI's own interface (`agy` without `-p`) in a terminal
//! (`TerminalView`), as the real runs of `agy` 1.4.9 found it (docs/TODO-1.8.md, point 0,
//! « Antigravity CLI », ESC-269 and ESC-272).
//!
//! - Its interface and `agy -p` keep their conversations in the same store, the agent's own home
//!   (`ESCOUADE_AGY_HOME`, its `USERPROFILE` and `HOME`): `agy --conversation <id>` opens the agent's
//!   conversation, its history there (no relay, unlike Cursor). A conversation `-p` opened and never
//!   wrote has no transcript: its interface would say `conversation "<id>" not found` and go on in a
//!   new one, so it is not given; the new one is told by its first hook (`SessionStart`).
//! - Its log is the conversation's transcript (`brain/<id>/.system_generated/logs/
//!   transcript_full.jsonl`), a line for each step once it is written: the user's message, the
//!   model's answers and calls, each call's run; translated by `AgentRt::agy_log`.
//! - Its hooks (`.gemini/config/hooks.json` of its home: `SessionStart`, `PreToolUse`,
//!   `PostToolUse`, `Stop`) run `escouade-hook <event>`, as it runs them (`cmd /c` under Windows,
//!   `sh -c` elsewhere): under Windows by its name, its folder first on the `PATH`. They are taken
//!   away when the agent's `-p` process starts (`forget_hooks`).
//! - Escouade's MCP server is in its home's MCP config, its token in the file (`agy` expands no
//!   variable there), never on its command line; as for its `-p` process.
//! - Its first screens are passed: the choice of its colours (`cache/onboarding.json`) and the trust
//!   of the agent's folder (`trustedWorkspaces` of its settings, tried) are written in its home, with
//!   the permission rules of the agent's mode and fence, those of its plugin (no rule `allow` but in « Tout accepter »
//!   without a fence nor a discussion, ESC-376): what no rule allows, its interface asks.
//! - The account's address is kept off its screen (`AGY_CLI_HIDE_ACCOUNT_INFO`, tried).

use super::super::terminal::{bracketed, TerminalView};
use super::{gemini_dir, transcript};
use crate::agent::{AgentRt, Effects, HookOutcome};
use crate::tui::Registered;
use anyhow::Result;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Its interface's name.
const INTERFACE: &str = "agy";

/// What its interface says, and goes on in a new conversation, of one not in its store.
const NOT_FOUND: &str = "not found";

/// Antigravity CLI's interface.
pub struct AgyTerminal;

pub static AGY_TERMINAL: AgyTerminal = AgyTerminal;

/// Its hooks in the agent's home.
pub(crate) fn hooks_file(home: &Path) -> PathBuf {
    gemini_dir(home).join("config").join("hooks.json")
}

/// The hooks of its interface taken away from the agent's home `home`, its `-p` process about to
/// start (they would run there too, for nothing).
pub(crate) fn forget_hooks(home: &Path) {
    let _ = std::fs::remove_file(hooks_file(home));
}

impl TerminalView for AgyTerminal {
    fn shell(&self) -> &'static str {
        INTERFACE
    }

    /// Where the user's tools keep their home (their git config, Rust's): the core's to know, beside the agent's home its
    /// plugin gives `agy`.
    fn core_env(&self) -> Vec<(String, String)> {
        super::tools_env(&[])
    }

    /// It names its own: a conversation `-p` opened and never wrote is not in its store.
    fn names_its_session(&self) -> bool {
        false
    }

    fn on_hook(&self, rt: &mut AgentRt, hook: &Value, fx: &mut Effects) -> HookOutcome {
        rt.agy_hook(hook, fx)
    }

    fn running_dir(&self, _config_dir: &Path) -> Option<PathBuf> {
        None
    }

    fn find_running(&self, _: &Path, _: &str, _: Option<&Path>) -> Option<Registered> {
        None
    }

    /// The transcript of the conversation, in the agent's home.
    fn find_log(&self, home: &Path, session: &str) -> Option<PathBuf> {
        if session.is_empty() || session.contains(['/', '\\', '.']) {
            return None;
        }
        Some(transcript(home, session)).filter(|p| p.is_file())
    }

    fn on_log_entry(&self, rt: &mut AgentRt, entry: &Value, fx: &mut Effects) {
        rt.view_line(entry, fx);
    }

    fn says_session_lost(&self, output: &[u8]) -> bool {
        let text = String::from_utf8_lossy(output);
        text.contains("conversation \"") && text.contains(NOT_FOUND)
    }

    /// Its conversations are in the agent's own home, whatever its account: nothing to copy.
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

    /// Escape (« esc to cancel »): the text written so far is its answer, the end of its turn.
    fn interrupt_key(&self) -> &'static [u8] {
        b"\x1b"
    }

    /// Ctrl+C twice (« press ctrl+c again to exit »).
    fn quit_key(&self) -> &'static [u8] {
        b"\x03"
    }
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
