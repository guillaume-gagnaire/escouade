//! Escouade's MCP server as every engine hands it to the process of an agent, or keeps it out of
//! it (docs/ENGINES.md, « Le serveur MCP d'Escouade »).
//!
//! The core decides (`Core::agent_access`): the server as the agent itself, with a token of its
//! own, or refused. The engine says how: what a private file holds (`Engine::grant_file`), what a
//! refused process reads (`Engine::refusal`), and the arguments and variables of its process
//! (their plugin's launch, `engine/launch`). The token is never one of the arguments: a process's command line
//! is seen by every process of the user, and the app's log writes it.

use std::path::PathBuf;

/// The name every engine gives Escouade's server: its tools are `mcp__escouade__<tool>` in the
/// conversation (`tool_name`), whatever the harness.
pub const SERVER: &str = "escouade";

/// The name a call of the tool `tool` of the MCP server `server` takes in the conversation,
/// whatever the harness: Claude Code's, which `tools.ts`, the permission card, the activity line
/// and `convsearch.rs` read. Every adapter names an MCP call with it (docs/TODO-1.8.md, « Un
/// moteur par agent », table of the tools).
#[cfg(test)]
pub fn tool_name(server: &str, tool: &str) -> String {
    format!("mcp__{server}__{tool}")
}

/// What the process of an agent is given of Escouade's server.
#[derive(Clone, PartialEq, Eq)]
pub enum Escouade {
    /// Its tools, as the agent itself: the server at `url`, reached with `token` (the agent's own,
    /// valid until the process stops), which `file` holds when its engine wants it in a file
    /// (`Engine::grant_file`: private, gone with the process).
    Granted {
        url: String,
        token: String,
        file: Option<PathBuf>,
    },
    /// None of them, whatever the user's own config declares; `file`, when its engine has a
    /// refusal (`Engine::refusal`) and it could be written, holds it.
    Denied { file: Option<PathBuf> },
}

impl Escouade {
    /// The agent's token, when it has one.
    pub fn token(&self) -> Option<&str> {
        match self {
            Escouade::Granted { token, .. } => Some(token),
            Escouade::Denied { .. } => None,
        }
    }
}

/// Written by hand: the token is a secret, and a log or a failed assertion prints a `{:?}`.
impl std::fmt::Debug for Escouade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Escouade::Granted { url, file, .. } => f
                .debug_struct("Granted")
                .field("url", url)
                .field("token", &"<masked>")
                .field("file", file)
                .finish(),
            Escouade::Denied { file } => f.debug_struct("Denied").field("file", file).finish(),
        }
    }
}
