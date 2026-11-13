//! The MCP server's activity log: what it was asked and how it answered, refusals included. The
//! last `KEPT` stay in memory (none is saved); the window is sent each one as it comes
//! (`UiEvent::McpActivity`) and reads them all with the command `mcp_activity`.

use super::Caller;
use crate::core::Core;
use crate::model::UiEvent;
use tauri::Runtime;

// The log and its entries: `escouade-mcp`.
pub use escouade_mcp::activity::{clip, entry, Activity, ActivityEntry, Outcome};

/// Logs what the server was asked and how it ended, and tells the window. `caller`: None for a
/// request refused before its token was read. No token may be in `summary` or the message. Not
/// with an agent's lock held: it reads the agent's name.
pub fn record<R: Runtime>(
    core: &Core<R>,
    caller: Option<&Caller>,
    tool: &str,
    summary: &str,
    outcome: Outcome,
) {
    let entry = entry(caller_name(core, caller), tool, summary, outcome);
    core.mcp.activity.push(entry.clone());
    core.hub.emit(UiEvent::McpActivity { entry });
}

/// How the log names a caller: an agent by its name (its id once it is gone).
pub fn caller_name<R: Runtime>(core: &Core<R>, caller: Option<&Caller>) -> String {
    match caller {
        Some(Caller::External) => tr!("Claude (hors Escouade)", "Claude (outside Escouade)"),
        Some(Caller::Agent(id)) => core
            .agents
            .read()
            .get(id)
            .map(|h| h.lock().meta.name.clone())
            .unwrap_or_else(|| id.clone()),
        None => tr!("Client inconnu", "Unknown client"),
    }
}
