//! « Vue Antigravity »: Antigravity CLI's own interface (`agy` without `-p`) in a terminal
//! (`TerminalView`), as the real runs of `agy` 1.4.9 found it (docs/TODO-1.8.md, point 0,
//! « Antigravity CLI », ESC-269 and ESC-272).
//!
//! - Its interface and `agy -p` keep their conversations in the same store, the agent's own home
//!   (`ESCOUADE_AGY_HOME`, its `USERPROFILE` and `HOME`): `agy --conversation <id>` opens the agent's
//!   conversation, its history there (no relay, unlike Cursor). A conversation `-p` opened and never
//!   wrote has no transcript: its interface would say `conversation "<id>" not found` and go on in a
//!   new one, so it is not given; the new one is told by its first hook (`SessionStart`).
//! - Its log is the conversation's transcript (`brain/<id>/.system_generated/logs/
//!   transcript_full.jsonl`), a line for each step once it is written: the user's message, the
//!   model's answers and calls, each call's run; translated by `AgentRt::agy_log`.
//! - Its hooks (`.gemini/config/hooks.json` of its home: `SessionStart`, `PreToolUse`,
//!   `PostToolUse`, `Stop`) run `escouade-hook <event>`, as it runs them (`cmd /c` under Windows,
//!   `sh -c` elsewhere): under Windows by its name, its folder first on the `PATH`. They are taken
//!   away when the agent's `-p` process starts (`forget_hooks`).
//! - Escouade's MCP server is in its home's MCP config, its token in the file (`agy` expands no
//!   variable there), never on its command line; as for its `-p` process.
//! - Its first screens are passed: the choice of its colours (`cache/onboarding.json`) and the trust
//!   of the agent's folder (`trustedWorkspaces` of its settings, tried) are written in its home, with
//!   the permission rules of the agent's mode and fence, those of its plugin (no rule `allow` but in « Tout accepter »
//!   without a fence nor a discussion, ESC-376): what no rule allows, its interface asks.
//! - The account's address is kept off its screen (`AGY_CLI_HIDE_ACCOUNT_INFO`, tried).

use super::super::terminal::{bracketed, TerminalArgs, TerminalLaunch, TerminalView};
use super::super::{mcp, Engine, Escouade};
use super::{
    agy_mode, always_file, gemini_dir, grant_doc, home_env, settings_doc, settings_file, temp_env,
    temp_mark, transcript, AgyRules, AGY, EFFORTS, KEY_VARS,
};
use crate::agent::{AgentRt, Effects, HookOutcome};
use crate::tui::Registered;
use anyhow::Result;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Its interface's name.
const INTERFACE: &str = "agy";

/// How long a hook may run, in seconds.
const HOOK_LIMIT_SECS: u64 = 10;

/// The events its hooks tell Escouade.
pub(crate) const EVENTS: [&str; 4] = ["SessionStart", "PreToolUse", "PostToolUse", "Stop"];

/// What its interface says, and goes on in a new conversation, of one not in its store.
const NOT_FOUND: &str = "not found";

/// Antigravity CLI's interface.
pub struct AgyTerminal;

pub static AGY_TERMINAL: AgyTerminal = AgyTerminal;

/// Its hooks in the agent's home.
pub(crate) fn hooks_file(home: &Path) -> PathBuf {
    gemini_dir(home).join("config").join("hooks.json")
}

/// Where it keeps whether its first screens were passed.
pub(crate) fn onboarding_file(home: &Path) -> PathBuf {
    gemini_dir(home)
        .join("antigravity-cli")
        .join("cache")
        .join("onboarding.json")
}

/// The line its hooks run for `event`: `escouade-hook` by its path quoted for a POSIX shell, or
/// under Windows by its name (its folder first on the `PATH`), which `cmd /c` finds.
fn hook_line(command: &Path, event: &str) -> String {
    if cfg!(windows) {
        format!("escouade-hook {event}")
    } else {
        format!(
            "'{}' {event}",
            command.to_string_lossy().replace('\'', r"'\''")
        )
    }
}

/// Its `hooks.json`: `escouade-hook <event>` for each event Escouade reads.
pub(crate) fn hooks_json(command: &Path) -> Value {
    let mut spec = json!({});
    for event in EVENTS {
        let handler = json!({
            "type": "command", "command": hook_line(command, event), "timeout": HOOK_LIMIT_SECS,
        });
        spec[event] = if event.ends_with("ToolUse") {
            json!([{ "matcher": "*", "hooks": [handler] }])
        } else {
            json!([handler])
        };
    }
    json!({ "escouade": spec })
}

/// The hooks of its interface taken away from the agent's home `home`, its `-p` process about to
/// start (they would run there too, for nothing).
pub(crate) fn forget_hooks(home: &Path) {
    let _ = std::fs::remove_file(hooks_file(home));
}

/// Its settings `before` with the agent's folder `cwd` among its trusted workspaces.
pub(crate) fn trusted(mut doc: Value, cwd: &str) -> Value {
    if cwd.is_empty() {
        return doc;
    }
    let mut list: Vec<Value> = doc["trustedWorkspaces"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if !list.iter().any(|v| v.as_str() == Some(cwd)) {
        list.push(json!(cwd));
    }
    doc["trustedWorkspaces"] = json!(list);
    doc
}

/// The arguments of an agent's interface: its conversation when its store has it (`resumable`),
/// its model and effort, its mode.
pub(crate) fn tui_args(l: &TerminalLaunch, resumable: bool) -> Vec<String> {
    let m = l.meta;
    let mut a: Vec<String> = Vec::new();
    if let Some(s) = m.session_id.as_deref().filter(|_| resumable) {
        a.extend(["--conversation".into(), s.to_string()]);
    }
    let model = m.model.trim();
    if !model.is_empty() && !model.contains(char::is_whitespace) {
        a.extend(["--model".into(), model.to_string()]);
        if AGY.supports_effort(model) && EFFORTS.contains(&m.effort.as_str()) {
            a.extend(["--effort".into(), m.effort.clone()]);
        }
    }
    if let Some(mode) = agy_mode(&m.mode) {
        a.extend(["--mode".into(), mode.into()]);
    }
    a
}

/// A JSON file's text, as `agy` writes them.
fn pretty(v: &Value) -> Vec<u8> {
    serde_json::to_string_pretty(v)
        .unwrap_or_default()
        .into_bytes()
}

impl TerminalView for AgyTerminal {
    fn shell(&self) -> &'static str {
        INTERFACE
    }

    /// Its files in the agent's home: its settings (rules and trust), its first screens passed,
    /// its hooks, Escouade's server.
    fn launch(&self, l: &TerminalLaunch) -> TerminalArgs {
        let resumable = match (l.home, l.meta.session_id.as_deref()) {
            (Some(home), Some(s)) => self.find_log(home, s).is_some(),
            _ => false,
        };
        let mut files = Vec::new();
        let mut env = vec![
            ("AGY_CLI_HIDE_ACCOUNT_INFO".to_string(), "1".to_string()),
            (l.hooks.url_var.to_string(), l.hooks.url.to_string()),
        ];
        if let Some(home) = l.home {
            env.extend(home_env(&[], home));
            // Its own temporary folder, as its plugin has it: `agy` writes there with no rule nor question (ESC-389).
            temp_env(&mut env, home);
            files.push((
                temp_mark(home),
                b"Antigravity's temporary folder for this agent (TEMP, TMP), made by Escouade.\n"
                    .to_vec(),
            ));
            let rules = AgyRules {
                fence: crate::fence::spelled(l.fence, l.program),
                read_only: l.meta.discussion.is_some(),
            };
            let file = settings_file(home);
            let before = std::fs::read_to_string(&file).unwrap_or_default();
            // The rules of its plugin, nothing more (ESC-376): what the 2.2.0 kept of « Toujours » is erased, never read.
            let _ = std::fs::remove_file(always_file(home));
            let doc = settings_doc(&before, &rules, &l.meta.mode, &[]);
            files.push((file, pretty(&trusted(doc, &l.meta.cwd))));
            let onboarding = onboarding_file(home);
            let passed = std::fs::read_to_string(&onboarding)
                .ok()
                .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                .is_some_and(|v| v["onboardingComplete"] == true);
            if !passed {
                let done = json!({
                    "consumerOnboardingComplete": true, "enterpriseOnboardingComplete": true,
                    "onboardingComplete": true,
                });
                files.push((onboarding, pretty(&done)));
            }
            let escouade = match l.access {
                Escouade::Granted { url, token, .. } => Some((url.as_str(), token.as_str())),
                Escouade::Denied { .. } => None,
            };
            let mcp_file = super::mcp_file(home);
            let before = std::fs::read_to_string(&mcp_file).unwrap_or_default();
            files.push((mcp_file, pretty(&grant_doc(&before, escouade))));
            if let Some(command) = l.hooks.command {
                files.push((hooks_file(home), pretty(&hooks_json(command))));
                if cfg!(windows) {
                    if let Some(dir) = command.parent() {
                        let path = std::env::var_os("PATH").unwrap_or_default();
                        let joined = std::env::join_paths(
                            std::iter::once(dir.to_path_buf()).chain(std::env::split_paths(&path)),
                        )
                        .unwrap_or(path);
                        env.push(("PATH".into(), joined.to_string_lossy().into_owned()));
                    }
                }
            }
        }
        TerminalArgs {
            args: tui_args(l, resumable),
            file: None,
            files,
            env,
        }
    }

    /// In its home's MCP config (`launch`): nothing on its command line nor in its variables.
    fn escouade(&self, _harness: &dyn Engine, _access: &Escouade, _: &Path) -> mcp::Launch {
        mcp::Launch {
            args: Vec::new(),
            env: Vec::new(),
        }
    }

    /// A key of the machine's would take the account's place.
    fn masked_env(&self, env: &[(String, String)]) -> Vec<(String, String)> {
        KEY_VARS
            .iter()
            .filter(|k| !env.iter().any(|(e, _)| e == *k))
            .map(|k| (k.to_string(), String::new()))
            .collect()
    }

    fn read_dir_args(&self, dir: &Path) -> Vec<String> {
        vec!["--add-dir".into(), dir.display().to_string()]
    }

    /// It names its own: a conversation `-p` opened and never wrote is not in its store.
    fn names_its_session(&self) -> bool {
        false
    }

    fn on_hook(&self, rt: &mut AgentRt, hook: &Value, fx: &mut Effects) -> HookOutcome {
        rt.agy_hook(hook, fx)
    }

    fn running_dir(&self, _config_dir: &Path) -> Option<PathBuf> {
        None
    }

    fn find_running(&self, _: &Path, _: &str, _: Option<&Path>) -> Option<Registered> {
        None
    }

    /// The transcript of the conversation, in the agent's home.
    fn find_log(&self, home: &Path, session: &str) -> Option<PathBuf> {
        if session.is_empty() || session.contains(['/', '\\', '.']) {
            return None;
        }
        Some(transcript(home, session)).filter(|p| p.is_file())
    }

    fn on_log_entry(&self, rt: &mut AgentRt, entry: &Value, fx: &mut Effects) {
        rt.view_line(entry, fx);
    }

    fn says_session_lost(&self, output: &[u8]) -> bool {
        let text = String::from_utf8_lossy(output);
        text.contains("conversation \"") && text.contains(NOT_FOUND)
    }

    /// Its conversations are in the agent's own home, whatever its account: nothing to copy.
    fn copy_session(&self, _from: &Path, _to: &Path, _session: &str) -> Result<()> {
        Ok(())
    }

    fn paste(&self, text: &str) -> Vec<u8> {
        bracketed(text)
    }

    fn submit(&self) -> &'static [u8] {
        b"\r"
    }

    fn image_types(&self) -> &'static [&'static str] {
        &[]
    }

    /// Escape (« esc to cancel »): the text written so far is its answer, the end of its turn.
    fn interrupt_key(&self) -> &'static [u8] {
        b"\x1b"
    }

    /// Ctrl+C twice (« press ctrl+c again to exit »).
    fn quit_key(&self) -> &'static [u8] {
        b"\x03"
    }
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
