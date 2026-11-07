//! « Vue Gemini »: the agent's conversation as Gemini CLI's own interface (`gemini`, in a terminal,
//! `engine/gemini/terminal.rs`) tells it by its hooks (`escouade-hook <event>`, its payload as its
//! code builds it, 0.66.0): its log, which writes a message anew each time it changes, is not read.
//! Each hook goes to its plugin (`view/translate`, `crates/escouade-acp/src/view.rs`); here is what
//! the core keeps of them: the session it runs, and the turn it left without a word.

use super::*;

impl AgentRt {
    /// Its turn stopped by Escape: no hook tells it, it ends here.
    pub(crate) fn gemini_interrupted(&mut self, fx: &mut Effects) {
        if self.meta.status.is_active() && self.tui_prompt.is_some() {
            self.interrupted = true;
            self.end_tui_turn(None, fx);
        }
    }

    /// One of the hooks its Gemini interface runs (`escouade-hook <event>`).
    pub(crate) fn gemini_hook(&mut self, h: &Value, fx: &mut Effects) -> HookOutcome {
        let mut out = HookOutcome::default();
        if let Some(run) = self.tui.as_mut() {
            run.spoke = true;
        }
        if let Some(sid) = h["session_id"].as_str().filter(|s| !s.is_empty()) {
            if self.meta.session_id.as_deref() != Some(sid) {
                // A session of its own (`/clear`, `/resume`): the conversation goes on there.
                if self.meta.session_id.is_some() {
                    self.reset_plan(fx);
                    self.notice(
                        "info",
                        tr!(
                            "Nouvelle session Gemini (contexte vidé)",
                            "New Gemini session (context cleared)"
                        ),
                        fx,
                    );
                }
                self.meta.session_id = Some(sid.to_string());
                self.meta.last_entry = None;
                self.meta.fork_of = None;
                self.meta.fork_at = None;
                fx.save = true;
            }
            self.unwritten_session = None;
        }
        let event = h[crate::tui::EVENT_KEY]
            .as_str()
            .or(h["hook_event_name"].as_str())
            .unwrap_or_default();
        // A turn it ended without a word (Escape typed there, an error): over, interrupted.
        if event == "BeforeAgent" {
            self.gemini_interrupted(fx);
        }
        self.view_note(json!({ "escouade": "hook", "hook": h }), fx);
        out.stopped = event == "AfterAgent";
        fx.agent_changed = true;
        out
    }
}
