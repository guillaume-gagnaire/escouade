#![doc = include_str!("../README.md")]
//!
//! ## Internals
//!
//! The ACP client of Escouade (Agent Client Protocol v1: JSON-RPC 2.0, one message a line on the agent's input and
//! output), drawn from the one Cursor CLI was driven with (`engine/cursor/`, `agent/cursor.rs` of the app), with what is
//! Cursor's own (`cursor/*`) kept apart in `cursor` (docs/PLUGINS.md §1.1, PLG-6).
//!
//! It holds no process: `Agent` is a state machine that reads what the agent wrote (`Agent::frames`) and answers the
//! normalized items of the conversation (`escouade_plugin_protocol::engine::Item`) and the lines to write back to it
//! (`Item::Write`, or the `writes` of an answer), as the contract of the harnesses in process wants it (INT-9). The plugin
//! `dev.escouade.acp` (`escouade-plugins acp`) runs one for each agent of a harness a manifest describes
//! (`contributes.harnesses[].acp`).
//!
//! `plugin` is the service of the protocol around them, which a plugin of a harness on this client runs.
//!
//! The helpers at the top are those of the ACP harnesses, and of Antigravity CLI, whose client puts its stream in the
//! form of this one (`plugins/antigravity`, INT-10.8).

pub mod agent;
pub mod cursor;
mod files;
pub mod gemini;
pub mod git;
pub mod limits;
pub mod paths;
pub mod plugin;
pub mod questions;
mod texts;
pub mod tools;
pub mod vibe;

pub use agent::Agent;
pub use tools::{call_id, content_text, diff_of, line_hunks, tool_of};

use serde_json::{json, Value};

/// The version of ACP Escouade speaks.
pub const PROTOCOL: u64 = 1;

/// The name of Escouade's MCP server, as its agents know it.
pub const SERVER: &str = "escouade";

/// What `session/new` and `session/load` are given of MCP servers: Escouade's (its address and token; the token in a
/// header, on the process's input, never its command line), or none. An entry of the user's of that name is replaced
/// by it (tried with Cursor CLI).
pub fn mcp_servers(escouade: Option<(&str, &str)>) -> Value {
    match escouade {
        Some((url, token)) => json!([{
            "type": "http", "name": SERVER, "url": url,
            "headers": [{ "name": "Authorization", "value": format!("Bearer {token}") }],
        }]),
        None => json!([]),
    }
}

/// The instructions added to an agent (a ticket's protocol), as they go before its first message: ACP has no system
/// message, and `session/load` keeps them (tried with Cursor CLI).
pub fn preamble(instructions: &str) -> String {
    format!("<instructions>\n{}\n</instructions>", instructions.trim())
}

/// What was said in the agent's own interface (« Vue Cursor », the core's relay), as it goes before its next message.
pub fn relayed(relay: &str) -> String {
    format!("<relais_escouade>\n{}\n</relais_escouade>", relay.trim())
}

/// The option of a permission (`session/request_permission`) of `kind` (`allow_once`, `allow_always`, `reject_once`),
/// by the id the agent gave it.
pub fn option_of(params: &Value, kind: &str) -> String {
    params["options"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|o| o["kind"] == kind)
        .and_then(|o| o["optionId"].as_str())
        .map(str::to_string)
        .unwrap_or_else(|| kind.replace('_', "-"))
}

/// Whether the system of `path` ignores the case of its paths. A module is compiled for no system and runs on all of them:
/// there, the path itself tells.
fn case_ignored(path: &str) -> bool {
    if cfg!(target_family = "wasm") {
        windows_path(path)
    } else {
        cfg!(windows)
    }
}

/// Whether `path` is a path of Windows: a drive, or the two backslashes of a share.
fn windows_path(path: &str) -> bool {
    let b = path.as_bytes();
    path.starts_with(r"\\") || (b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':')
}

/// Whether `text` (a path, a command) names `path` or a path under it, whatever its separators and, under Windows, its
/// case. A guard: a longer name that starts alike counts too.
pub fn names(text: &str, path: &str) -> bool {
    let folded = case_ignored(path);
    let norm = |s: &str| {
        let s = s.replace('\\', "/");
        let s = s.trim_end_matches('/').to_string();
        if folded {
            s.to_lowercase()
        } else {
            s
        }
    };
    let p = norm(path);
    !p.is_empty() && norm(text).contains(&p)
}

/// « Projet privé » (ESC-140): whether the permission an agent asks (`params` of `session/request_permission`) names a
/// path of `fence` (as its program names them): its command, its title, the files it writes or reads. A command can
/// reach a path it does not name: a guard, not a barrier; what holds is the files read and written through the client
/// (`Agent`, `fs/*`).
pub fn fenced(params: &Value, fence: &[String]) -> bool {
    if fence.is_empty() {
        return false;
    }
    let tc = &params["toolCall"];
    let mut texts: Vec<&str> = tc["title"].as_str().into_iter().collect();
    for v in tc["rawInput"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(_, v)| v)
    {
        texts.extend(v.as_str());
    }
    for k in ["content", "locations"] {
        texts.extend(
            tc[k]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|c| c["path"].as_str()),
        );
    }
    texts.iter().any(|t| fence.iter().any(|p| names(t, p)))
}

/// Whether `path` is in the folder `dir` (or is it), whatever its slashes and case.
pub fn inside(path: &str, dir: &str) -> bool {
    let norm = |p: &str| p.replace('\\', "/").trim_end_matches('/').to_lowercase();
    let (p, d) = (norm(path), norm(dir));
    !d.is_empty() && (p == d || p.starts_with(&format!("{d}/")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_tells_whether_it_is_of_windows() {
        for path in ["C:\\Prive", "c:/prive", r"\\serveur\partage"] {
            assert!(windows_path(path), "{path}");
        }
        for path in ["/home/moi", "prive", ""] {
            assert!(!windows_path(path), "{path}");
        }
    }

    #[test]
    fn the_token_goes_in_a_header_never_elsewhere() {
        let s = mcp_servers(Some(("http://127.0.0.1:1/mcp", "s3cr3t")));
        assert_eq!(s[0]["url"], "http://127.0.0.1:1/mcp");
        assert_eq!(s[0]["headers"][0]["value"], "Bearer s3cr3t");
        assert_eq!(mcp_servers(None), json!([]));
    }

    #[test]
    fn a_permission_naming_a_fenced_path_is_fenced() {
        let p = json!({ "toolCall": { "title": "cat C:\\Prive\\a.txt", "locations": [] } });
        assert!(
            fenced(&p, &["c:/prive".into()]) == cfg!(windows) || fenced(&p, &["C:\\Prive".into()])
        );
        assert!(!fenced(&p, &[]));
        let q = json!({ "toolCall": { "locations": [{ "path": "/srv/secret/x" }] } });
        assert!(fenced(&q, &["/srv/secret".into()]));
        assert!(!fenced(&q, &["/srv/other".into()]));
    }

    #[test]
    fn an_option_is_found_by_its_kind_or_named_after_it() {
        let p = json!({ "options": [{ "kind": "allow_once", "optionId": "yes" }] });
        assert_eq!(option_of(&p, "allow_once"), "yes");
        assert_eq!(option_of(&p, "reject_once"), "reject-once");
    }
}
