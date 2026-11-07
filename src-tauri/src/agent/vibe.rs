//! Mistral Vibe in the agent's conversation: its agents' updates (`vibe-acp`, ACP v1) are translated by its plugin
//! (`dev.escouade.mistral`, the client of `crates/escouade-acp` with its extension `vibe`) and applied as items
//! (`AgentRt::apply_items`, through `Engine::translate`), its terminal view too (`crates/escouade-acp/src/view.rs`).

#[cfg(test)]
#[path = "vibe_tests.rs"]
mod tests;
