//! The MCP server's activity log: what it was asked and how it answered, refusals included. The
//! last `KEPT` stay in memory (none is saved); the window is sent each one as it comes
//! (`UiEvent::McpActivity`) and reads them all with the command `mcp_activity`. The app writes an
//! entry (`entry`) with the caller's name it reads from its agents.

use escouade_model::model::now_ms;
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::VecDeque;

/// How many entries the log keeps: the oldest go first.
pub const KEPT: usize = 200;

/// The longest summary or message an entry keeps, in characters: a request's own text (its path,
/// a header) could be anything.
const TEXT_MAX: usize = 300;

/// The longest message of a call that says what it changed (`Outcome::Changed`), in characters:
/// one line too, but every setting a call set, with its value before and after, may not fit in the
/// `TEXT_MAX` of a request's own text.
const CHANGED_MAX: usize = 1_500;

/// How a call ended, or why a request was refused, as the log says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Ok,
    /// Ended well, and says what it changed (`set_board`: each setting from its value to its new
    /// one): logged as "ok", with that text as its message.
    Changed(String),
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
    pub fn clear(&self) {
        self.entries.lock().clear();
    }
}

/// The entry of what the server was asked and how it ended, by `caller` (as the log names it). No
/// token may be in `summary` or the message: they are cut to a line.
pub fn entry(caller: String, tool: &str, summary: &str, outcome: Outcome) -> ActivityEntry {
    let (outcome, message, longest) = match outcome {
        Outcome::Ok => ("ok", None, TEXT_MAX),
        Outcome::Changed(m) => ("ok", Some(m), CHANGED_MAX),
        Outcome::Refused(m) => ("refused", Some(m), TEXT_MAX),
        Outcome::Error(m) => ("error", Some(m), TEXT_MAX),
    };
    ActivityEntry {
        at: now_ms(),
        caller,
        tool: tool.to_string(),
        summary: clip(summary),
        outcome,
        message: message.map(|m| clip_to(&m, longest)),
    }
}

/// `text` on one line, of `TEXT_MAX` characters at most (cut with « … »).
pub fn clip(text: &str) -> String {
    clip_to(text, TEXT_MAX)
}

/// `text` on one line, of `max` characters at most (cut with « … »).
fn clip_to(text: &str, max: usize) -> String {
    let line: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if line.chars().count() <= max {
        return line;
    }
    let mut cut: String = line.chars().take(max - 1).collect();
    cut.push('…');
    cut
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_activity_log_is_cleared_on_demand() {
        let log = Activity::default();
        log.push(ActivityEntry {
            at: 1,
            caller: "Claude (hors Escouade)".into(),
            tool: "list_projects".into(),
            summary: String::new(),
            outcome: "ok",
            message: None,
        });
        log.clear();
        assert!(log.entries().is_empty());
    }
}
