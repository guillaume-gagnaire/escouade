//! Mistral Vibe in the agent's conversation. Since INT-10.6 its agents' updates (`vibe-acp`, ACP v1) are translated by
//! its plugin (`dev.escouade.mistral`, the client of `crates/escouade-acp` with its extension `vibe`) and applied
//! as items (`AgentRt::apply_items`, through `Engine::translate`); what stays here is its terminal view's
//! (`agent/vibe_tui.rs`).

/// What the conversation keeps of Vibe's terminal view between its readings (not saved).
#[derive(Default)]
pub(crate) struct VibeState {
    /// Its interface (« Vue Vibe », `agent/vibe_tui.rs`): the last turn of its journal ended, which a segment read
    /// again does not start again.
    pub(super) tui_ended: Option<String>,
}

#[cfg(test)]
#[path = "vibe_tests.rs"]
mod tests;
