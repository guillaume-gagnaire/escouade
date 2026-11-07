//! Copilot CLI in the agent's conversation: its agents' events are translated by its plugin (`dev.escouade.copilot-cli`,
//! `plugins/copilot/src/translate.rs`) and applied as items (`AgentRt::apply_items`, through `Engine::translate`), its terminal
//! view too (`plugins/copilot/src/view.rs`). Here are the tests, on recorded messages read by the module.

#[cfg(test)]
use super::*;

#[cfg(test)]
impl AgentRt {
    /// One message of Copilot in a recorded stream.
    pub(crate) fn copilot_frame(&mut self, f: &Value, fx: &mut Effects) {
        self.view_frame(f, fx);
    }
}

#[cfg(test)]
#[path = "copilot_tests.rs"]
mod tests;
