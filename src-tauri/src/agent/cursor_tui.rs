//! « Vue Cursor »: the agent's conversation as Cursor's own interface (`agent`, in a terminal) writes
//! it in its transcript (`~/.cursor/projects/<folder>/agent-transcripts/<id>/<id>.jsonl`,
//! `engine/cursor/terminal.rs`), read as it is written (`tui`) and translated by its plugin
//! (`view/translate`, `crates/escouade-acp/src/view.rs`).

#[cfg(test)]
use super::*;

#[cfg(test)]
impl AgentRt {
    /// One line of the transcript its Cursor interface writes.
    pub(crate) fn cursor_log(&mut self, e: &Value, fx: &mut Effects) {
        self.view_line(e, fx);
    }
}

#[cfg(test)]
#[path = "cursor_tui_tests.rs"]
mod tests;
