//! The parts of the harnesses that stand without the app (docs/CRATES.md, § 10): where each harness's program is
//! (`locate`, `program`, `which`), the user configs Escouade edits itself (`userconfig`, `jsonc`), Escouade's MCP server
//! as a harness is handed it (`mcp`), Claude Code's sign-in (`signin`) and a subagent's transcript (`sidechain`). The
//! app re-exports each where it was (`engine::*`, `engine::<harness>::locate`, `agent::sidechain`, `which`).

#[macro_use]
extern crate escouade_plugin_cli;

/// A text in the interface's language: `tr!("français", "English", args…)`, as the app's `tr!`.
macro_rules! tr {
    ($($t:tt)+) => {
        tr_in!(escouade_base::i18n::ui(), $($t)+)
    };
}

pub mod jsonc;
pub mod mcp;
pub mod program;
pub mod shape;
pub mod sidechain;
pub mod signin;
pub mod userconfig;
pub mod which;

/// Where the program of each harness is: its setting, the PATH, the places its installers use.
pub mod locate {
    pub mod agy;
    pub mod claude;
    pub mod codex;
    pub mod copilot;
    pub mod cursor;
    pub mod gemini;
    pub mod opencode;
    pub mod vibe;
}
