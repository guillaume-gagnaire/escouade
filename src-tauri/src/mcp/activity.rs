//! The MCP server's activity log: what it was asked and how it answered, refusals included. The
//! last `KEPT` stay in memory (none is saved); the window is sent each one as it comes
//! (`UiEvent::McpActivity`) and reads them all with the command `mcp_activity`.

use super::Caller;
use crate::core::Core;
use crate::model::{now_ms, UiEvent};
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::VecDeque;
use tauri::Runtime;

/// How many entries the log keeps: the oldest go first.
pub const KEPT: usize = 200;

/// The longest summary or message an entry keeps, in characters: a request's own text (its path,
/// a header) could be anything.
const TEXT_MAX: usize = 300;

/// How a call ended, or why a request was refused, as the log says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Ok,
    /// Refused by a guard (a request without a valid token, a tool the caller may not use, the
    /// autopilot's pause…): the message says why.
    Refused(String),
    /// The call failed: the message says how.
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEntry {
    pub at: i64,
    /// Who called: the agent's name, « Claude (hors Escouade) », or « Client inconnu » for a
    /// request refused before its token said who it came from.
    pub caller: String,
    /// The tool called; empty for a request refused before reaching one.
    pub tool: String,
    /// What it was asked, in short (its arguments, or the request's method and path). Never a
    /// token.
    pub summary: String,
    /// "ok", "refused" or "error".
    pub outcome: &'static str,
    /// Why it was refused, or how it failed.
    pub message: Option<String>,
}

/// The entries, oldest first.
#[derive(Default)]
pub struct Activity {
    entries: Mutex<VecDeque<ActivityEntry>>,
}

impl Activity {
    pub fn push(&self, entry: ActivityEntry) {
        let mut entries = self.entries.lock();
        while entries.len() >= KEPT {
            entries.pop_front();
        }
        entries.push_back(entry);
    }

    /// The entries kept, oldest first.
    pub fn entries(&self) -> Vec<ActivityEntry> {
        self.entries.lock().iter().cloned().collect()
    }

    /// « Effacer »: no entry left.
    // Allowed unused until the window's activity section (M5) calls it (then drop the allow).
    #[allow(dead_code)]
    pub fn clear(&self) {
        self.entries.lock().clear();
    }
}

/// Logs what the server was asked and how it ended, and tells the window. `caller`: None for a
/// request refused before its token was read. No token may be in `summary` or the message. Not
/// with an agent's lock held: it reads the agent's name.
pub fn record<R: Runtime>(
    core: &Core<R>,
    caller: Option<&Caller>,
    tool: &str,
    summary: &str,
    outcome: Outcome,
) {
    let (outcome, message) = match outcome {
        Outcome::Ok => ("ok", None),
        Outcome::Refused(m) => ("refused", Some(m)),
        Outcome::Error(m) => ("error", Some(m)),
    };
    let entry = ActivityEntry {
        at: now_ms(),
        caller: caller_name(core, caller),
        tool: tool.to_string(),
        summary: clip(summary),
        outcome,
        message: message.map(|m| clip(&m)),
    };
    core.mcp.activity.push(entry.clone());
    core.hub.emit(UiEvent::McpActivity { entry });
}

/// How the log names a caller: an agent by its name (its id once it is gone).
pub fn caller_name<R: Runtime>(core: &Core<R>, caller: Option<&Caller>) -> String {
    match caller {
        Some(Caller::External) => tr!("Claude (hors Escouade)", "Claude (outside Escouade)"),
        Some(Caller::Agent(id)) => core
            .agents
            .read()
            .get(id)
            .map(|h| h.lock().meta.name.clone())
            .unwrap_or_else(|| id.clone()),
        None => tr!("Client inconnu", "Unknown client"),
    }
}

/// `text` on one line, of `TEXT_MAX` characters at most (cut with « … »).
pub(crate) fn clip(text: &str) -> String {
    let line: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if line.chars().count() <= TEXT_MAX {
        return line;
    }
    let mut cut: String = line.chars().take(TEXT_MAX - 1).collect();
    cut.push('…');
    cut
}
