//! Codex in the agent's conversation. Since INT-10.4 its agents' messages are translated by its plugin
//! (`dev.escouade.codex`, `plugins/codex/src/translate.rs`) and applied as items
//! (`AgentRt::apply_items`, through `Engine::translate`). Its terminal view stays the core's (`agent/codex_tui.rs`): it
//! reads the session's rollout through the same translation, as a library, for the same conversation in both.

use super::*;
use escouade_plugin_codex::translate::{Ctx, Seen};
use escouade_plugin_protocol::engine::Item;

pub(crate) use escouade_plugin_codex::translate::Translator as CodexState;

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
    /// One message of Codex, as its interface's rollout tells it or in a recorded stream: its items applied at once, the
    /// next message reads what they made.
    pub(crate) fn codex_frame(&mut self, f: &Value, fx: &mut Effects) {
        let session = self.meta.session_id.clone();
        let seen = Seeing(std::cell::RefCell::new(&mut self.conv));
        let c = Ctx {
            cwd: &self.meta.cwd,
            mode: &self.meta.mode,
            model: &self.meta.model,
            active: self.meta.status.is_active(),
            interrupted: self.interrupted,
            process: false,
            lang: i18n::ui().tag(),
            session: session.as_deref(),
            copy: self.meta.fork_of.is_some(),
            seen: &seen,
        };
        let mut items: Vec<Item> = Vec::new();
        self.codex.frame(f, &c, &mut items);
        for item in items {
            self.apply_item(item, fx);
        }
    }

    /// The turn of its thread under way.
    pub(super) fn codex_turn(&self) -> Option<String> {
        self.codex.turn.clone()
    }

    /// Another conversation (or another process of it): what was kept of the last one is gone.
    pub(super) fn codex_new_conversation(&mut self) {
        self.codex = CodexState::default();
    }
}

#[cfg(test)]
use escouade_plugin_codex::translate::same_dir;

#[cfg(test)]
#[path = "codex_tests.rs"]
mod tests;
