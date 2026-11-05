//! Copilot CLI in the agent's conversation. Since INT-10.3 its agents' events are translated by its plugin
//! (`dev.escouade.copilot-cli`, `plugins/copilot/src/translate.rs`) and applied as items
//! (`AgentRt::apply_items`, through `Engine::translate`). Its terminal view stays the core's (`agent/copilot_tui.rs`): it
//! reads the session's log through the same translation, as a library, for the same conversation in both.

use super::*;
pub(super) use escouade_plugin_copilot::translate::tool_of;
use escouade_plugin_copilot::translate::{Ctx, Seen};
use escouade_plugin_protocol::engine::Item;

pub(crate) use escouade_plugin_copilot::translate::Translator as CopilotState;

/// The conversation, as the translation reads it (reading it may load it).
struct Seeing<'a>(std::cell::RefCell<&'a mut crate::conv::Conv>);

impl Seen for Seeing<'_> {
    fn has(&self, id: &str) -> bool {
        self.0.borrow_mut().contains(id)
    }
    fn call(&self, id: &str) -> Option<Value> {
        self.0.borrow_mut().get(id).cloned()
    }
}

impl AgentRt {
    /// `f` through `read`, on what the agent is now: its items applied in their order.
    fn cp_translated(
        &mut self,
        fx: &mut Effects,
        read: impl FnOnce(&mut CopilotState, &Ctx, &mut Vec<Item>),
    ) {
        let session = self.meta.session_id.clone();
        let seen = Seeing(std::cell::RefCell::new(&mut self.conv));
        let c = Ctx {
            cwd: &self.meta.cwd,
            mode: &self.meta.mode,
            active: self.meta.status.is_active(),
            discussion: self.meta.discussion.is_some(),
            process: false,
            lang: i18n::ui().tag(),
            resumed: session.as_deref().filter(|_| self.meta.fork_of.is_none()),
            seen: &seen,
        };
        let mut items = Vec::new();
        read(&mut self.copilot, &c, &mut items);
        for item in items {
            self.apply_item(item, fx);
        }
    }

    /// One message of Copilot, in a recorded stream (`tests/fixtures/copilot/`) or as its client tells it.
    #[cfg(test)]
    pub(crate) fn copilot_frame(&mut self, f: &Value, fx: &mut Effects) {
        // One frame's items are applied at once: the next reads what they made.
        self.cp_translated(fx, |t, c, out| t.frame(f, c, out));
    }

    /// Whether Escouade's rules answer the permission `r` without a card.
    #[cfg(test)]
    pub(super) fn cp_safe(&mut self, r: &Value) -> bool {
        let session = None;
        let seen = Seeing(std::cell::RefCell::new(&mut self.conv));
        let c = Ctx {
            cwd: &self.meta.cwd,
            mode: &self.meta.mode,
            active: self.meta.status.is_active(),
            discussion: self.meta.discussion.is_some(),
            process: true,
            lang: i18n::ui().tag(),
            resumed: session,
            seen: &seen,
        };
        CopilotState::safe(r, &c)
    }

    /// Another conversation (or another process of it): what was kept of the last one is gone.
    pub(super) fn copilot_new_conversation(&mut self) {
        self.copilot = CopilotState::default();
    }

    /// One event of the session, from its interface's log.
    pub(super) fn cp_event(&mut self, e: &Value, fx: &mut Effects) {
        self.cp_translated(fx, |t, c, out| t.event(e, c, out));
    }

    /// The end of the agent's turn in its interface.
    pub(super) fn cp_turn_end(&mut self, fx: &mut Effects) {
        let mut items = Vec::new();
        self.copilot.turn_end(&mut items);
        for item in items {
            self.apply_item(item, fx);
        }
    }
}

#[cfg(test)]
use escouade_acp::inside;

#[cfg(test)]
#[path = "copilot_tests.rs"]
mod tests;
