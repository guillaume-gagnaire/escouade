//! Antigravity CLI in the agent's conversation: what its agents say is translated by its plugin
//! (`dev.escouade.antigravity-cli`, `plugins/antigravity/src/translate.rs`) and applied as items (`AgentRt::apply_items`, through
//! `Engine::translate`), its terminal view too (`plugins/antigravity/src/view.rs`). Here are the tests, on recorded streams
//! (`tests/fixtures/agy/`) read by the module.

#[cfg(test)]
use super::*;

#[cfg(test)]
impl AgentRt {
    /// One event of a recorded stream.
    pub(crate) fn agy_frame(&mut self, f: &Value, fx: &mut Effects) {
        self.view_frame(f, fx);
    }
}

#[cfg(test)]
#[path = "agy_tests.rs"]
mod tests;
