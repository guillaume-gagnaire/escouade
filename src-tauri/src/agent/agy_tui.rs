//! « Vue Antigravity »: the agent's conversation as Antigravity CLI's own interface (`agy`, in a
//! terminal) writes it in its transcript (`brain/<id>/.system_generated/logs/transcript_full.jsonl`
//! of the agent's home, `engine/agy/terminal.rs`), which its plugin translates (`view/translate`,
//! `plugins/antigravity/src/view.rs`), and what its hooks tell.
//!
//! Its hooks (`escouade-hook <event>`): `SessionStart` tells a new conversation (its interface
//! started with none, its transcript then read from its start); `Stop` the end of its loop (an
//! error's too). Nothing tells that a permission waits for the user in
//! its interface: only a question it asks (`ask_question`, from its transcript).

use super::*;

impl AgentRt {
    /// One step of the transcript its Antigravity interface writes.
    #[cfg(test)]
    pub(crate) fn agy_log(&mut self, e: &Value, fx: &mut Effects) {
        self.view_line(e, fx);
    }

    /// One of the hooks its Antigravity interface runs (`escouade-hook <event>`).
    pub(crate) fn agy_hook(&mut self, h: &Value, fx: &mut Effects) -> HookOutcome {
        let mut out = HookOutcome::default();
        if let Some(run) = self.tui.as_mut() {
            run.spoke = true;
        }
        if let Some(sid) = h["conversationId"].as_str().filter(|s| !s.is_empty()) {
            if self.meta.session_id.as_deref() != Some(sid) {
                // A conversation of its own (its interface started with none, or `/clear`): its
                // transcript is read from its start.
                if self.meta.session_id.is_some() {
                    self.reset_plan(fx);
                    self.notice(
                        "info",
                        tr!(
                            "Nouvelle conversation Antigravity (contexte vidé)",
                            "New Antigravity conversation (context cleared)"
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
                out.transcript = h["transcriptPath"].as_str().map(str::to_string);
            }
        }
        match h[crate::tui::EVENT_KEY].as_str().unwrap_or_default() {
            "PreToolUse" => {
                if !self.meta.status.is_active() {
                    self.set_status(AgentStatus::Running, fx);
                }
            }
            "PostToolUse" => {
                if self.meta.status == AgentStatus::Waiting {
                    self.set_status(AgentStatus::Running, fx);
                }
            }
            "Stop" => {
                match h["terminationReason"].as_str() {
                    Some("USER_CANCELED") => self.interrupted = true,
                    Some("ERROR") => {
                        let error = h["error"].as_str().filter(|e| !e.is_empty());
                        self.tui_error = Some(error.unwrap_or("error").to_string());
                    }
                    _ => {}
                }
                out.stopped = true;
            }
            _ => {}
        }
        fx.agent_changed = true;
        out
    }
}

#[cfg(test)]
#[path = "agy_tui_tests.rs"]
mod tests;
