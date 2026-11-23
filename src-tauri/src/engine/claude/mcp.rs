//! Escouade's MCP server in Claude Code: given to an agent's process by `--mcp-config <private
//! file>`, refused by `--disallowedTools mcp__escouade` (with `--settings` and its
//! `deniedMcpServers` in the stream), and declared for the user in each account's `.claude.json`, written by Escouade
//! as `claude mcp add --scope user` would (docs/PROTOCOL.md, « Déclaration dans Claude Code » and « Les agents
//! d'Escouade »).

#[cfg(test)]
use crate::engine::mcp::tool_name;
use crate::engine::mcp::SERVER;
use crate::engine::userconfig::{Format, UserConfig};
use serde_json::{json, Value};

/// What `--disallowedTools` is given to refuse an agent every tool of Escouade's server, whatever
/// config declares it: the start of their names (`mcp__escouade__<tool>`).
#[cfg(test)]
pub(crate) fn all_tools() -> String {
    tool_name(SERVER, "").trim_end_matches('_').to_string()
}

/// The file `--mcp-config` reads, as `claude mcp add --transport http` declares a server and under
/// the same name: the user's own entry of that name, if any, gives way to this one (« dynamic »
/// before « user »).
pub(super) fn grant_file(url: &str, token: &str) -> Vec<u8> {
    json!({ "mcpServers": { SERVER: {
        "type": "http",
        "url": url,
        "headers": { "Authorization": format!("Bearer {token}") },
    } } })
    .to_string()
    .into_bytes()
}

/// Refused, the settings that keep the server itself out of the process (`--settings`): its tools
/// refused alone, Claude Code still connects an entry of that name of the user's config, and the
/// agent reads the server's instructions with none of its tools (ESC-91, tried on Claude Code
/// 2.1.300: `deniedMcpServers` leaves the server unconnected, never asked).
pub(super) fn refusal() -> Value {
    json!({ "deniedMcpServers": [{ "serverName": SERVER }] })
}

/// Where Claude Code keeps a user-scope MCP server: `mcpServers.escouade` of the account's `.claude.json`, `{ "type":
/// "http", "url", "headers": { "Authorization": "Bearer …" } }`, as `claude mcp add --scope user --transport http`
/// writes it. Escouade writes it there itself (SEC-07, ESC-595): `claude mcp add --header` put the token on a command
/// line, which the other programs of the machine read in the list of the processes while it runs. The token stays in
/// the file, readable by its owner only: the entry serves a Claude Code launched outside Escouade, where no variable of
/// Escouade's is set (Claude Code expands `${VAR}` in a header, but a terminal of the user's has no such variable).
/// The file is the account's `.claude.json` (`accounts::claude_json`), not one of its folder: `files` names it for
/// `file_in` alone.
pub(super) static USER_CONFIG: UserConfig = UserConfig {
    files: &[".claude.json"],
    format: Format::Json,
    servers: &["mcpServers"],
    url: "url",
    headers: "headers",
    fixed: &[("type", "http")],
};
