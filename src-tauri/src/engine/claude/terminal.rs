//! « Vue Claude Code »: Claude Code's own interface (its TUI) in a terminal (`TerminalView`).
//!
//! Its hooks are HTTP hooks to Escouade's server, given with `--settings`; its log is the
//! session's transcript (`projects/<folder>/<session>.jsonl`), whose entries have the shape of the
//! stream's frames (`agent::transcript`); it lists its running interfaces in the registry of its
//! sessions (`<config folder>/sessions`). See docs/PROTOCOL.md, « Vue Claude Code ».

use super::super::terminal::{bracketed, TerminalView};
use crate::agent::{AgentRt, Effects, HookOutcome};
use crate::tui::{find_registered, Registered};
use anyhow::Result;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// What Claude Code says, before it quits, of a session to resume that its folder does not have.
const NO_CONVERSATION: &str = "No conversation found";

/// The types of the images Claude Code reads.
const IMAGE_TYPES: [&str; 4] = ["image/png", "image/jpeg", "image/gif", "image/webp"];

/// Claude Code's interface.
pub struct ClaudeTerminal;

pub static CLAUDE_TERMINAL: ClaudeTerminal = ClaudeTerminal;

/// The transcript of `session` in an account's folder: `projects/<folder>/<session>.jsonl`, the
/// one written last if several folders have it.
pub fn find_transcript(config_dir: &Path, session: &str) -> Option<PathBuf> {
    let file = format!("{session}.jsonl");
    std::fs::read_dir(config_dir.join("projects"))
        .ok()?
        .flatten()
        .filter_map(|folder| {
            let path = folder.path().join(&file);
            let at = std::fs::metadata(&path).ok()?.modified().ok()?;
            Some((at, path))
        })
        .max_by_key(|(at, _)| *at)
        .map(|(_, path)| path)
}

/// Whether the output says Claude Code could not find the session it was told to resume.
pub(crate) fn says_no_conversation(output: &[u8]) -> bool {
    String::from_utf8_lossy(output).contains(NO_CONVERSATION)
}

impl TerminalView for ClaudeTerminal {
    fn shell(&self) -> &'static str {
        "claude"
    }

    fn names_its_session(&self) -> bool {
        true
    }

    fn on_hook(&self, rt: &mut AgentRt, hook: &Value, fx: &mut Effects) -> HookOutcome {
        rt.on_hook(hook, fx)
    }

    fn running_dir(&self, config_dir: &Path) -> Option<PathBuf> {
        Some(config_dir.join("sessions"))
    }

    fn find_running(&self, dir: &Path, session: &str, known: Option<&Path>) -> Option<Registered> {
        find_registered(dir, session, known)
    }

    fn find_log(&self, config_dir: &Path, session: &str) -> Option<PathBuf> {
        find_transcript(config_dir, session)
    }

    fn on_log_entry(&self, rt: &mut AgentRt, entry: &Value, fx: &mut Effects) {
        rt.on_transcript(entry, fx);
    }

    fn says_session_lost(&self, output: &[u8]) -> bool {
        says_no_conversation(output)
    }

    fn copy_session(&self, from: &Path, to: &Path, session: &str) -> Result<()> {
        crate::accounts::copy_session(from, to, session)
    }

    fn paste(&self, text: &str) -> Vec<u8> {
        bracketed(text)
    }

    fn submit(&self) -> &'static [u8] {
        b"\r"
    }

    fn image_types(&self) -> &'static [&'static str] {
        &IMAGE_TYPES
    }

    fn interrupt_key(&self) -> &'static [u8] {
        b"\x1b"
    }

    fn quit_key(&self) -> &'static [u8] {
        b"\x03"
    }
}
