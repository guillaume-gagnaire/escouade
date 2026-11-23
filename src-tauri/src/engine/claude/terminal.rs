//! « Vue Claude Code »: Claude Code's own interface (its TUI) in a terminal (`TerminalView`).
//!
//! Its hooks are HTTP hooks to Escouade's server, given with `--settings`; its log is the
//! session's transcript (`projects/<folder>/<session>.jsonl`), whose entries have the shape of the
//! stream's frames (`agent::transcript`); it lists its running interfaces in the registry of its
//! sessions (`<config folder>/sessions`). See docs/PROTOCOL.md, « Vue Claude Code ».

use super::super::terminal::{bracketed, TerminalView};
use crate::agent::{AgentRt, Effects, HookOutcome};
use crate::tui::{find_registered, Registered};
use anyhow::Result;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// What Claude Code says, before it quits, of a session to resume that its folder does not have.
const NO_CONVERSATION: &str = "No conversation found";

/// The types of the images Claude Code reads.
const IMAGE_TYPES: [&str; 4] = ["image/png", "image/jpeg", "image/gif", "image/webp"];

/// Claude Code's interface.
pub struct ClaudeTerminal;

pub static CLAUDE_TERMINAL: ClaudeTerminal = ClaudeTerminal;

/// The transcript of `session` in an account's folder: `projects/<folder>/<session>.jsonl`, the
/// one written last if several folders have it.
pub fn find_transcript(config_dir: &Path, session: &str) -> Option<PathBuf> {
    let file = format!("{session}.jsonl");
    std::fs::read_dir(config_dir.join("projects"))
        .ok()?
        .flatten()
        .filter_map(|folder| {
            let path = folder.path().join(&file);
            let at = std::fs::metadata(&path).ok()?.modified().ok()?;
            Some((at, path))
        })
        .max_by_key(|(at, _)| *at)
        .map(|(_, path)| path)
}

/// How much of a transcript is read for the entry of Code Mode: it comes before the first message.
const CODE_MODE_HEAD: u64 = 1 << 20;

/// The entry Claude Code writes in the transcript of a session in Code Mode (2.1.329: `{"type":"repl-mode",
/// "replMode":true}`, before the first message and after each turn; none for a session without it).
pub fn says_code_mode(entry: &Value) -> bool {
    entry["type"] == "repl-mode" && entry["replMode"] == true
}

/// Whether the transcript `path` is of a session in Code Mode (ESC-553): its first MiB read. Claude Code refuses to
/// resume it anywhere but in its interface (« Failed to resume session: This session was started in Code Mode
/// (experimental), which isn't available because for now it only works in the interactive terminal… », 2.1.329).
pub fn code_mode_session(path: &Path) -> bool {
    use std::io::{BufRead, Read};
    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };
    std::io::BufReader::new(file.take(CODE_MODE_HEAD))
        .lines()
        .map_while(std::result::Result::ok)
        .filter(|l| l.contains("\"repl-mode\""))
        .any(|l| serde_json::from_str::<Value>(&l).is_ok_and(|e| says_code_mode(&e)))
}

/// The variables that route Claude Code through a cloud provider when they say true.
const PROVIDERS: [&str; 6] = [
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "CLAUDE_CODE_USE_FOUNDRY",
    "CLAUDE_CODE_USE_MANTLE",
    "CLAUDE_CODE_USE_ANTHROPIC_AWS",
    "CLAUDE_CODE_USE_ANTHROPIC_GOOGLE_CLOUD",
];
/// The variables that send its requests to another address, or with another credential, when they hold anything.
const ELSEWHERE: [&str; 2] = ["ANTHROPIC_BASE_URL", "ANTHROPIC_AUTH_TOKEN"];

/// What sends an account's requests elsewhere than Anthropic's API, which Code Mode needs (ESC-553): the name of the
/// variable (`var`: its launch's variables, then the app's environment, then the `env` of its `settings.json`), or
/// `apiKeyHelper`. Only names, never a value.
pub fn gateway(
    var: &dyn Fn(&str) -> Option<String>,
    settings_json: Option<&Value>,
) -> Option<String> {
    let of = |name: &str| {
        var(name).or_else(|| {
            settings_json?.get("env")?.get(name).map(|v| match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
        })
    };
    let truthy = |v: &str| {
        matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    };
    PROVIDERS
        .iter()
        .find(|n| of(n).is_some_and(|v| truthy(&v)))
        .or_else(|| {
            ELSEWHERE
                .iter()
                .find(|n| of(n).is_some_and(|v| !v.trim().is_empty()))
        })
        .map(|n| n.to_string())
        .or_else(|| {
            settings_json?
                .get("apiKeyHelper")
                .filter(|v| v.as_str().is_some_and(|s| !s.trim().is_empty()))
                .map(|_| "apiKeyHelper".to_string())
        })
}

/// Whether the output says Claude Code could not find the session it was told to resume.
pub(crate) fn says_no_conversation(output: &[u8]) -> bool {
    String::from_utf8_lossy(output).contains(NO_CONVERSATION)
}

impl TerminalView for ClaudeTerminal {
    fn shell(&self) -> &'static str {
        "claude"
    }

    fn names_its_session(&self) -> bool {
        true
    }

    fn on_hook(&self, rt: &mut AgentRt, hook: &Value, fx: &mut Effects) -> HookOutcome {
        rt.on_hook(hook, fx)
    }

    fn running_dir(&self, config_dir: &Path) -> Option<PathBuf> {
        Some(config_dir.join("sessions"))
    }

    fn find_running(&self, dir: &Path, session: &str, known: Option<&Path>) -> Option<Registered> {
        find_registered(dir, session, known)
    }

    fn find_log(&self, config_dir: &Path, session: &str) -> Option<PathBuf> {
        find_transcript(config_dir, session)
    }

    fn on_log_entry(&self, rt: &mut AgentRt, entry: &Value, fx: &mut Effects) {
        if says_code_mode(entry) {
            rt.mark_code_mode(fx);
        }
        rt.on_transcript(entry, fx);
    }

    fn says_session_lost(&self, output: &[u8]) -> bool {
        says_no_conversation(output)
    }

    fn copy_session(&self, from: &Path, to: &Path, session: &str) -> Result<()> {
        crate::accounts::copy_session(from, to, session)
    }

    fn paste(&self, text: &str) -> Vec<u8> {
        bracketed(text)
    }

    fn submit(&self) -> &'static [u8] {
        b"\r"
    }

    fn image_types(&self) -> &'static [&'static str] {
        &IMAGE_TYPES
    }

    fn interrupt_key(&self) -> &'static [u8] {
        b"\x1b"
    }

    fn quit_key(&self) -> &'static [u8] {
        b"\x03"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// ESC-553: what routes an account elsewhere than Anthropic's API, by its name alone, in its launch's variables, the
    /// app's or its `settings.json`.
    #[test]
    fn a_gateway_or_a_cloud_provider_is_found_by_its_name_alone() {
        let none = |_: &str| None;
        assert_eq!(gateway(&none, None), None);
        assert_eq!(
            gateway(&none, Some(&json!({ "env": { "FOO": "1" } }))),
            None
        );
        let bedrock = |n: &str| (n == "CLAUDE_CODE_USE_BEDROCK").then(|| "1".to_string());
        assert_eq!(
            gateway(&bedrock, None).as_deref(),
            Some("CLAUDE_CODE_USE_BEDROCK")
        );
        // A provider turned off is none.
        let off = |n: &str| (n == "CLAUDE_CODE_USE_VERTEX").then(|| "0".to_string());
        assert_eq!(gateway(&off, None), None);
        let base = json!({ "env": { "ANTHROPIC_BASE_URL": "https://gw.example/v1" } });
        assert_eq!(
            gateway(&none, Some(&base)).as_deref(),
            Some("ANTHROPIC_BASE_URL")
        );
        let foundry = json!({ "env": { "CLAUDE_CODE_USE_FOUNDRY": true } });
        assert_eq!(
            gateway(&none, Some(&foundry)).as_deref(),
            Some("CLAUDE_CODE_USE_FOUNDRY")
        );
        let helper = json!({ "apiKeyHelper": "~/bin/key.sh" });
        assert_eq!(
            gateway(&none, Some(&helper)).as_deref(),
            Some("apiKeyHelper")
        );
        // Never a value.
        let token = |n: &str| (n == "ANTHROPIC_AUTH_TOKEN").then(|| "sk-secret".to_string());
        assert_eq!(
            gateway(&token, None).as_deref(),
            Some("ANTHROPIC_AUTH_TOKEN")
        );
    }

    /// ESC-553: a session in Code Mode by the entry Claude Code 2.1.329 writes in its transcript, whatever the order of
    /// its keys; none without it.
    #[test]
    fn a_session_in_code_mode_is_told_by_its_transcript() {
        let dir = std::env::temp_dir().join(format!("esc553-transcript-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let on = dir.join("on.jsonl");
        std::fs::write(
            &on,
            "{\"type\":\"mode\",\"mode\":\"normal\"}\n{\"replMode\":true,\"sessionId\":\"s\",\"type\":\"repl-mode\"}\n",
        )
        .unwrap();
        let off = dir.join("off.jsonl");
        std::fs::write(
            &off,
            "{\"type\":\"user\",\"message\":{\"content\":\"\\\"repl-mode\\\"\"}}\n",
        )
        .unwrap();
        assert!(code_mode_session(&on));
        assert!(!code_mode_session(&off));
        assert!(!code_mode_session(&dir.join("absent.jsonl")));
        assert!(says_code_mode(
            &json!({ "type": "repl-mode", "replMode": true })
        ));
        assert!(!says_code_mode(
            &json!({ "type": "repl-mode", "replMode": false })
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
