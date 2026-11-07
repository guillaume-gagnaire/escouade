//! « Vue Vibe »: the agent's conversation as Mistral Vibe's own interface (`vibe`, in a terminal)
//! writes it in the journal of its session's store (`logs/session/unified/<id>/journal/
//! <sequence>.jsonl` of the agent's `VIBE_HOME`, `engine/vibe/terminal.rs`), read as it is
//! written (`tui`) and translated by its plugin (`view/translate`,
//! `crates/escouade-acp/src/view.rs`), and what its hooks tell.
//!
//! Its hooks (`pre_tool`, `post_tool`, `post_agent`) tell the session of the interface: a new one
//! (its interface started with none) is the agent's from then on, its journal read from its start.

use super::*;
use std::path::Path;

impl AgentRt {
    /// One record of the journal its Vibe interface writes.
    #[cfg(test)]
    pub(crate) fn vibe_log(&mut self, record: &Value, fx: &mut Effects) {
        self.view_line(record, fx);
    }

    /// One of the hooks its Vibe interface runs (`escouade-hook`): the session it is in.
    pub(crate) fn vibe_hook(&mut self, h: &Value, fx: &mut Effects) -> HookOutcome {
        let mut out = HookOutcome::default();
        if let Some(run) = self.tui.as_mut() {
            run.spoke = true;
        }
        // A subagent's hooks name its own session.
        if !h["parent_session_id"].is_null() {
            return out;
        }
        let Some(sid) = h["session_id"].as_str().filter(|s| !s.is_empty()) else {
            return out;
        };
        if self.meta.session_id.as_deref() == Some(sid) {
            return out;
        }
        // A session of its own (its interface started with none, or `/new`): its journal is read
        // from its start.
        if self.meta.session_id.is_some() {
            self.reset_plan(fx);
            self.notice(
                "info",
                tr!(
                    "Nouvelle conversation Vibe (contexte vidé)",
                    "New Vibe conversation (context cleared)"
                ),
                fx,
            );
        }
        self.meta.session_id = Some(sid.to_string());
        self.meta.last_entry = None;
        self.meta.fork_of = None;
        self.meta.fork_at = None;
        self.unwritten_session = None;
        fx.save = true;
        fx.agent_changed = true;
        out.transcript = h["transcript_path"]
            .as_str()
            .and_then(|dir| crate::engine::vibe::terminal::latest_segment(Path::new(dir)))
            .map(|p| p.to_string_lossy().into_owned());
        out
    }
}

#[cfg(test)]
#[path = "vibe_tui_tests.rs"]
mod tests;
