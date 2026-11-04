//! Gemini CLI in the agent's conversation. Since INT-10.7 its agents' updates (`gemini --acp`, ACP v1) are translated by
//! its plugin (`dev.escouade.gemini-cli`, the client of `crates/escouade-acp` with its extension `gemini`) and applied as
//! items (`AgentRt::apply_items`, through `Engine::translate`); what stays in the core is its terminal view's
//! (`agent/gemini_tui.rs`).

#[cfg(test)]
#[path = "gemini_tests.rs"]
mod tests;
