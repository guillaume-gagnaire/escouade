//! « Vue Gemini »: Gemini CLI's own interface (`gemini`, no `--acp` nor `-p`) in a terminal
//! (`TerminalView`), as its code (0.66.0) and its documentation tell it (ESC-275; no account, so
//! never tried on a real turn: docs/TODO-1.8.md, « Gemini CLI »).
//!
//! - Its interface and `gemini --acp` keep their sessions in the same store, the agent's own home
//!   (`GEMINI_CLI_HOME`, `.gemini/tmp/<project>/chats/session-<time>-<id8>.jsonl`): `--resume <id>`
//!   opens the agent's session, its history there. A session `--acp` opened and never wrote has no
//!   file: it starts under that id (`--session-id`), which it takes for a new one.
//! - Its log rewrites a message each time it changes (a line per state of it): it is not read. Its
//!   hooks tell all Escouade follows (`settings.json` of its home, `hooks`, run by `escouade-hook
//!   <event>`): `SessionStart` its session, `BeforeAgent` the user's message, `BeforeTool` and
//!   `AfterTool` each call, `Notification` (`ToolPermission`) a permission waiting for the user,
//!   `AfterAgent` the end of a turn with its whole answer. They run in its shell (PowerShell under
//!   Windows, `sh` elsewhere): under Windows by their name, its folder first on the `PATH`. They are
//!   taken away when the agent's `--acp` process starts (`write_settings`).
//! - Escouade's MCP server is in its home's settings (`mcpServers`), its token in the file, never on
//!   its command line; its hooks' variables kept from its redaction of the environment.
//! - Its folder is trusted (`GEMINI_CLI_TRUST_WORKSPACE`), the rules of a private project and of a
//!   discussion are its `--policy`, as for its `--acp` process. Its account's sign-in is given as
//!   for that process (`accounts::launch_env`).

use super::super::terminal::{bracketed, TerminalView};
use crate::agent::{AgentRt, Effects, HookOutcome};
use crate::tui::Registered;
use anyhow::Result;
#[cfg(test)]
use serde_json::json;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Its interface's name.
const INTERFACE: &str = "gemini";

/// What its interface says, and quits, of a session it does not find.
const NOT_FOUND: &str = "Error resuming session";

/// Gemini CLI's interface.
pub struct GeminiTerminal;

pub static GEMINI_TERMINAL: GeminiTerminal = GeminiTerminal;

/// The file of the session `id` in the agent's home: `session-<time>-<id8>.jsonl` (`.json` before
/// 0.66) in one of its projects' `chats`, or `<id>.json`. None when it has none (yet).
pub(crate) fn session_file(home: &Path, id: &str) -> Option<PathBuf> {
    if id.len() < 8 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }
    let short = &id[..8];
    let ends = [format!("-{short}.jsonl"), format!("-{short}.json")];
    let whole = format!("{id}.json");
    std::fs::read_dir(home.join(".gemini").join("tmp"))
        .ok()?
        .flatten()
        .filter_map(|project| std::fs::read_dir(project.path().join("chats")).ok())
        .flat_map(|chats| chats.flatten())
        .map(|f| f.path())
        .find(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            name == whole
                || (name.starts_with("session-") && ends.iter().any(|e| name.ends_with(e.as_str())))
        })
}

/// What its interface's view left in the agent's settings (`doc`) taken away, its `--acp` process
/// about to start: its hooks would run there too, for nothing, and its session is given the server.
pub(crate) fn forget_view(doc: &mut Value) {
    if let Some(o) = doc.as_object_mut() {
        o.remove("hooks");
        o.remove("hooksConfig");
    }
    let empty = match doc["mcpServers"].as_object_mut() {
        Some(o) => {
            o.remove("escouade");
            o.is_empty()
        }
        None => false,
    };
    if let (true, Some(d)) = (empty, doc.as_object_mut()) {
        d.remove("mcpServers");
    }
}

impl TerminalView for GeminiTerminal {
    fn shell(&self) -> &'static str {
        INTERFACE
    }

    /// Its rules (`--policy`) are written by the core (`prepare`).
    fn fence_by_core(&self) -> bool {
        true
    }

    /// Its log is not read, but its session's file says whether it may resume it.
    fn can_resume(&self, home: &Path, session: &str) -> bool {
        session_file(home, session).is_some()
    }

    /// Escouade names it (`--session-id`): its hooks tell it all the same.
    fn names_its_session(&self) -> bool {
        true
    }

    /// A session it does not have starts under its id (`--session-id`).
    fn resumes_any(&self) -> bool {
        true
    }

    fn on_hook(&self, rt: &mut AgentRt, hook: &Value, fx: &mut Effects) -> HookOutcome {
        rt.gemini_hook(hook, fx)
    }

    fn running_dir(&self, _config_dir: &Path) -> Option<PathBuf> {
        None
    }

    fn find_running(&self, _: &Path, _: &str, _: Option<&Path>) -> Option<Registered> {
        None
    }

    /// Its log is not read: its hooks tell the conversation.
    fn find_log(&self, _home: &Path, _session: &str) -> Option<PathBuf> {
        None
    }

    fn on_log_entry(&self, _rt: &mut AgentRt, _entry: &Value, _fx: &mut Effects) {}

    fn says_session_lost(&self, output: &[u8]) -> bool {
        String::from_utf8_lossy(output).contains(NOT_FOUND)
    }

    /// Its sessions are in the agent's own home, whatever its account: nothing to copy.
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

    /// Escape (« esc to cancel »).
    fn interrupt_key(&self) -> &'static [u8] {
        b"\x1b"
    }

    /// Ctrl+C twice (« Press Ctrl+C again to exit »).
    fn quit_key(&self) -> &'static [u8] {
        b"\x03"
    }

    /// No hook tells the end of a turn stopped by Escape: it ends here, interrupted.
    fn interrupted(&self, rt: &mut AgentRt, fx: &mut Effects) {
        rt.gemini_interrupted(fx);
    }
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
