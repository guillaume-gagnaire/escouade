//! Escouade's MCP server in Claude Code: given to an agent's process by `--mcp-config <private
//! file>`, refused by `--disallowedTools mcp__escouade` (with `--settings` and its
//! `deniedMcpServers` in the stream), and declared for the user by `claude mcp add --scope user`
//! in each account's `.claude.json` (docs/PROTOCOL.md, « Déclaration dans Claude Code » and « Les
//! agents d'Escouade »).

#[cfg(test)]
use crate::engine::mcp::tool_name;
use crate::engine::mcp::SERVER;
use crate::mcp::install::{run, Entry, Standing, Target};
use serde_json::{json, Value};
use std::path::Path;

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

/// What `.claude.json` (`config`) says of the server: its entry `mcpServers.escouade`, `{ "type":
/// "http", "url", "headers": { "Authorization": "Bearer …" } }`.
pub(super) fn declared(config: &Path) -> Standing {
    let text = match std::fs::read_to_string(config) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Standing::NoFile,
        Err(_) => return Standing::Unreadable,
    };
    let Ok(json) = serde_json::from_str::<Value>(&text) else {
        return Standing::Unreadable;
    };
    let Some(found) = json.get("mcpServers").and_then(|s| s.get(SERVER)) else {
        return Standing::Absent;
    };
    let http = found["type"] == "http";
    let url = found["url"].as_str().filter(|_| http).unwrap_or_default();
    let token = found["headers"]["Authorization"]
        .as_str()
        .and_then(|h| h.strip_prefix("Bearer "))
        .unwrap_or_default();
    Standing::Declared(Entry {
        url: url.to_string(),
        token: token.to_string(),
    })
}

/// The arguments of `claude mcp add`: the header last, since Claude Code lets it take every
/// argument that follows. The token is on this command line: the user's entry holds it in clear
/// anyway, and the command is never logged (its error is, the token hidden).
fn add_args(entry: &Entry) -> Vec<String> {
    let header = format!("Authorization: Bearer {}", entry.token);
    [
        "mcp",
        "add",
        "--scope",
        "user",
        "--transport",
        "http",
        SERVER,
        &entry.url,
        "--header",
        &header,
    ]
    .map(String::from)
    .to_vec()
}

fn remove_args() -> Vec<String> {
    ["mcp", "remove", SERVER, "--scope", "user"]
        .map(String::from)
        .to_vec()
}

pub(super) async fn add(target: &Target, entry: &Entry, secrets: &[&str]) -> anyhow::Result<()> {
    run(target, &add_args(entry), secrets).await
}

pub(super) async fn remove(target: &Target, secrets: &[&str]) -> anyhow::Result<()> {
    run(target, &remove_args(), secrets).await
}

/// `s` as one word of PowerShell.
fn powershell_word(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// `s` as one word of a POSIX shell.
fn posix_word(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// The command that declares `entry` by hand (`Engine::manual_declaration`). `program` is the
/// `claude` as the user named it (`claude` by default). The folder is set for the command alone:
/// not left set in the user's shell, where it would send the next `claude` to this account.
pub(super) fn manual_declaration(
    windows: bool,
    program: &str,
    config_dir: Option<&str>,
    entry: &Entry,
    replace: bool,
) -> String {
    let word = |s: &str| {
        if windows {
            powershell_word(s)
        } else {
            posix_word(s)
        }
    };
    let claude = if program == "claude" {
        program.to_string()
    } else if windows {
        format!("& {}", word(program))
    } else {
        word(program)
    };
    let header = format!("Authorization: Bearer {}", entry.token);
    let add = format!(
        "{claude} mcp add --scope user --transport http {SERVER} {} --header \"{header}\"",
        entry.url
    );
    let remove = format!("{claude} mcp remove {SERVER} --scope user");
    let commands = if replace {
        vec![remove, add]
    } else {
        vec![add]
    };
    let Some(dir) = config_dir else {
        return commands.join("; ");
    };
    if windows {
        let dir = word(dir);
        format!(
            "$old = $env:CLAUDE_CONFIG_DIR; $env:CLAUDE_CONFIG_DIR = {dir}; {}; $env:CLAUDE_CONFIG_DIR = $old",
            commands.join("; ")
        )
    } else {
        let set = format!("CLAUDE_CONFIG_DIR={}", word(dir));
        commands
            .iter()
            .map(|c| format!("{set} {c}"))
            .collect::<Vec<_>>()
            .join("; ")
    }
}
