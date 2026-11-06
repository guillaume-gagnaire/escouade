//! The generic ACP plugin (`dev.escouade.acp`, PLG-6, docs/PLUGINS.md §1.1): it serves the harnesses other plugins only
//! describe (`contributes.harnesses[].acp` of their manifest: an agent that speaks ACP, its program and arguments, its
//! modes and models), on the contract of the harnesses in process (INT-9). The core finds the agent's program itself,
//! checks the launch this plugin describes and starts it; what the agent writes comes here in batches (`agent/frames`)
//! and goes back as the items of the conversation and the lines to write to it, by the client of `escouade_acp`.
//!
//! The description comes with each agent (`described` of `engine/launch` and `agent/attach`): this plugin holds no
//! harness of its own, and keeps nothing on disk. Its service is `escouade_acp::plugin`, which Cursor CLI, Gemini CLI and
//! Mistral run too.

pub use escouade_acp::plugin::{handler, harnesses_of, serve_as};

pub fn run() -> i32 {
    serve_as("acp", None)
}
