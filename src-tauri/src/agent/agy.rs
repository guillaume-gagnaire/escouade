//! Antigravity CLI in the agent's conversation. Since INT-10.8 what its agents say is translated by its plugin
//! (`dev.escouade.antigravity-cli`, `plugins/antigravity/src/translate.rs`) and applied as items
//! (`AgentRt::apply_items`, through `Engine::translate`). What stays is what its « Vue Antigravity » keeps
//! (`agent/agy_tui.rs`), and, for the tests, the recorded streams (`tests/fixtures/agy/`) read through the same translation.

use super::*;
pub(super) use escouade_plugin_agy::translate::output;

/// What the translation keeps of Antigravity's conversation between its events (not saved).
#[derive(Default)]
pub(crate) struct AgyState {
    /// The translation of a recorded stream.
    #[cfg(test)]
    recorded: Option<escouade_plugin_agy::translate::Translator>,
    /// The calls its interface's transcript told and did not run yet (« Vue Antigravity »), by
    /// their id: their tool and arguments, for their end.
    pub(super) tui_calls: std::collections::HashMap<String, (String, Value)>,
}

#[cfg(test)]
impl AgentRt {
    /// One event of a recorded stream.
    pub(crate) fn agy_frame(&mut self, f: &Value, fx: &mut Effects) {
        let attach = escouade_plugin_protocol::engine::Attach {
            agent: self.meta.id.clone(),
            cwd: self.meta.cwd.clone(),
            model: self.meta.model.clone(),
            mode: self.meta.mode.clone(),
            lang: i18n::ui().tag().to_string(),
            ..Default::default()
        };
        let t = self
            .agy
            .recorded
            .get_or_insert_with(|| escouade_plugin_agy::translate::Translator::new(&attach));
        for item in t.recorded(f) {
            self.apply_item(item, fx);
        }
    }
}

#[cfg(test)]
#[path = "agy_tests.rs"]
mod tests;
