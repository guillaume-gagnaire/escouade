//! « Vue Gemini »: Gemini CLI's own interface (`gemini`, no `--acp` nor `-p`) in a terminal
//! (`TerminalView`), as its code (0.66.0) and its documentation tell it (ESC-275; no account, so
//! never tried on a real turn: docs/TODO-1.8.md, « Gemini CLI »).
//!
//! - Its interface and `gemini --acp` keep their sessions in the same store, the agent's own home
//!   (`GEMINI_CLI_HOME`, `.gemini/tmp/<project>/chats/session-<time>-<id8>.jsonl`): `--resume <id>`
//!   opens the agent's session, its history there. A session `--acp` opened and never wrote has no
//!   file: it starts under that id (`--session-id`), which it takes for a new one.
//! - Its log rewrites a message each time it changes (a line per state of it): it is not read. Its
//!   hooks tell all Escouade follows (`settings.json` of its home, `hooks`, run by `escouade-hook
//!   <event>`): `SessionStart` its session, `BeforeAgent` the user's message, `BeforeTool` and
//!   `AfterTool` each call, `Notification` (`ToolPermission`) a permission waiting for the user,
//!   `AfterAgent` the end of a turn with its whole answer. They run in its shell (PowerShell under
//!   Windows, `sh` elsewhere): under Windows by their name, its folder first on the `PATH`. They are
//!   taken away when the agent's `--acp` process starts (`write_settings`).
//! - Escouade's MCP server is in its home's settings (`mcpServers`), its token in the file, never on
//!   its command line; its hooks' variables kept from its redaction of the environment.
//! - Its folder is trusted (`GEMINI_CLI_TRUST_WORKSPACE`), the rules of a private project and of a
//!   discussion are its `--policy`, as for its `--acp` process. Its account's sign-in is given as
//!   for that process (`accounts::launch_env`).

use super::super::terminal::{bracketed, TerminalArgs, TerminalLaunch, TerminalView};
use super::super::{mcp, Engine, Escouade};
use super::{forget_sign_in, policy, POLICY_FILE, TRUST};
use crate::agent::{AgentRt, Effects, HookOutcome};
use crate::tui::Registered;
use anyhow::Result;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Its interface's name.
const INTERFACE: &str = "gemini";

/// How long a hook may run, in milliseconds (its unit).
const HOOK_LIMIT_MS: u64 = 10_000;

/// The events its hooks tell Escouade.
pub(crate) const EVENTS: [&str; 6] = [
    "SessionStart",
    "BeforeAgent",
    "BeforeTool",
    "AfterTool",
    "Notification",
    "AfterAgent",
];

/// What its interface says, and quits, of a session it does not find.
const NOT_FOUND: &str = "Error resuming session";

/// Gemini CLI's interface.
pub struct GeminiTerminal;

pub static GEMINI_TERMINAL: GeminiTerminal = GeminiTerminal;

/// Its settings in the agent's home.
pub(crate) fn settings_file(home: &Path) -> PathBuf {
    home.join(".gemini").join("settings.json")
}

/// The file of the session `id` in the agent's home: `session-<time>-<id8>.jsonl` (`.json` before
/// 0.66) in one of its projects' `chats`, or `<id>.json`. None when it has none (yet).
pub(crate) fn session_file(home: &Path, id: &str) -> Option<PathBuf> {
    if id.len() < 8 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }
    let short = &id[..8];
    let ends = [format!("-{short}.jsonl"), format!("-{short}.json")];
    let whole = format!("{id}.json");
    std::fs::read_dir(home.join(".gemini").join("tmp"))
        .ok()?
        .flatten()
        .filter_map(|project| std::fs::read_dir(project.path().join("chats")).ok())
        .flat_map(|chats| chats.flatten())
        .map(|f| f.path())
        .find(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            name == whole
                || (name.starts_with("session-") && ends.iter().any(|e| name.ends_with(e.as_str())))
        })
}

/// The line its hooks run for `event`: `escouade-hook` by its whole path (`hook_program`, which
/// PowerShell runs as `sh` does, ESC-430).
fn hook_line(command: &Path, event: &str) -> String {
    format!(
        "{} {event}",
        super::super::terminal::hook_program(command, cfg!(windows))
    )
}

/// Its settings `before` with what its interface needs of Escouade: its hooks (`command`, none
/// without it), Escouade's server (`escouade`: its address and token) or none, the variables of its
/// hooks kept from its redaction.
pub(crate) fn view_settings(
    before: &str,
    command: Option<&Path>,
    escouade: Option<(&str, &str)>,
    hook_vars: &[&str],
) -> Value {
    let mut doc: Value = serde_json::from_str(before)
        .ok()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    if let Some(command) = command {
        let mut hooks = json!({});
        for event in EVENTS {
            hooks[event] = json!([{ "hooks": [{
                "name": "escouade", "type": "command", "command": hook_line(command, event),
                "timeout": HOOK_LIMIT_MS,
            }] }]);
        }
        doc["hooks"] = hooks;
        doc["hooksConfig"] = json!({ "enabled": true });
        if !doc["security"].is_object() {
            doc["security"] = json!({});
        }
        let mut allowed: Vec<Value> = doc["security"]["environmentVariableRedaction"]["allowed"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for v in hook_vars {
            if !allowed.iter().any(|a| a == v) {
                allowed.push(json!(v));
            }
        }
        if !doc["security"]["environmentVariableRedaction"].is_object() {
            doc["security"]["environmentVariableRedaction"] = json!({});
        }
        doc["security"]["environmentVariableRedaction"]["allowed"] = json!(allowed);
    }
    if !doc["mcpServers"].is_object() {
        doc["mcpServers"] = json!({});
    }
    match escouade {
        Some((url, token)) => {
            doc["mcpServers"]["escouade"] = json!({
                "httpUrl": url, "headers": { "Authorization": format!("Bearer {token}") },
                "trust": false,
            });
        }
        None => {
            if let Some(o) = doc["mcpServers"].as_object_mut() {
                o.remove("escouade");
            }
        }
    }
    doc
}

/// What its interface's view left in the agent's settings (`doc`) taken away, its `--acp` process
/// about to start: its hooks would run there too, for nothing, and its session is given the server.
pub(crate) fn forget_view(doc: &mut Value) {
    if let Some(o) = doc.as_object_mut() {
        o.remove("hooks");
        o.remove("hooksConfig");
    }
    let empty = match doc["mcpServers"].as_object_mut() {
        Some(o) => {
            o.remove("escouade");
            o.is_empty()
        }
        None => false,
    };
    if let (true, Some(d)) = (empty, doc.as_object_mut()) {
        d.remove("mcpServers");
    }
}

/// Its `--approval-mode` for Escouade's mode.
fn approval_mode(mode: &str) -> &'static str {
    match mode {
        "plan" => "plan",
        "acceptEdits" => "auto_edit",
        "bypassPermissions" => "yolo",
        _ => "default",
    }
}

/// The arguments of an agent's interface: its session (resumed when its store has it, else started
/// under its id), its model, its mode, its rules.
pub(crate) fn tui_args(
    session: Option<&str>,
    resumable: bool,
    model: &str,
    mode: &str,
    policy: Option<&Path>,
) -> Vec<String> {
    let mut a: Vec<String> = Vec::new();
    if let Some(s) = session {
        let flag = if resumable {
            "--resume"
        } else {
            "--session-id"
        };
        a.extend([flag.into(), s.to_string()]);
    }
    let model = model.trim();
    if !model.is_empty() && model != super::DEFAULT && !model.contains(char::is_whitespace) {
        a.extend(["--model".into(), model.to_string()]);
    }
    a.extend(["--approval-mode".into(), approval_mode(mode).into()]);
    if let Some(p) = policy {
        a.extend(["--policy".into(), p.to_string_lossy().into_owned()]);
    }
    a
}

impl TerminalView for GeminiTerminal {
    fn shell(&self) -> &'static str {
        INTERFACE
    }

    /// Its settings (hooks, Escouade's server) and rules in the agent's home.
    fn launch(&self, l: &TerminalLaunch) -> TerminalArgs {
        let session = l.meta.session_id.as_deref().or(l.session);
        let mut files = Vec::new();
        let mut env = vec![
            (TRUST.0.to_string(), TRUST.1.to_string()),
            (l.hooks.url_var.to_string(), l.hooks.url.to_string()),
        ];
        let mut policy_file = None;
        let mut resumable = false;
        if let Some(home) = l.home {
            resumable = session.is_some_and(|s| session_file(home, s).is_some());
            // Its account's sign-in, not one refreshed here before.
            forget_sign_in(home);
            let escouade = match l.access {
                Escouade::Granted { url, token, .. } => Some((url.as_str(), token.as_str())),
                Escouade::Denied { .. } => None,
            };
            let file = settings_file(home);
            let before = std::fs::read_to_string(&file).unwrap_or_default();
            let doc = view_settings(
                &before,
                l.hooks.command,
                escouade,
                &[l.hooks.url_var, l.hooks.token_var],
            );
            files.push((
                file,
                serde_json::to_string_pretty(&doc)
                    .unwrap_or_default()
                    .into_bytes(),
            ));
            let fence = crate::fence::spelled(l.fence, l.program);
            if let Some(rules) = policy(&fence, l.meta.discussion.is_some()) {
                let path = home.join(POLICY_FILE);
                files.push((path.clone(), rules.into_bytes()));
                policy_file = Some(path);
            }
            if let (Some(command), true) = (l.hooks.command, cfg!(windows)) {
                // Its folder first on the PATH, and `cmd` kept from its current folder (ESC-430).
                let path = std::env::var_os("PATH").unwrap_or_default();
                env.extend(super::super::terminal::hook_env(command, &path));
            }
        }
        TerminalArgs {
            args: tui_args(
                session,
                resumable,
                &l.meta.model,
                &l.meta.mode,
                policy_file.as_deref(),
            ),
            file: None,
            files,
            env,
        }
    }

    /// In its home's settings (`launch`): nothing on its command line nor in its variables.
    fn escouade(&self, _harness: &dyn Engine, _access: &Escouade, _: &Path) -> mcp::Launch {
        mcp::Launch {
            args: Vec::new(),
            env: Vec::new(),
        }
    }

    fn read_dir_args(&self, dir: &Path) -> Vec<String> {
        vec!["--include-directories".into(), dir.display().to_string()]
    }

    /// Escouade names it (`--session-id`): its hooks tell it all the same.
    fn names_its_session(&self) -> bool {
        true
    }

    /// A session it does not have starts under its id (`--session-id`).
    fn resumes_any(&self) -> bool {
        true
    }

    fn on_hook(&self, rt: &mut AgentRt, hook: &Value, fx: &mut Effects) -> HookOutcome {
        rt.gemini_hook(hook, fx)
    }

    fn running_dir(&self, _config_dir: &Path) -> Option<PathBuf> {
        None
    }

    fn find_running(&self, _: &Path, _: &str, _: Option<&Path>) -> Option<Registered> {
        None
    }

    /// Its log is not read: its hooks tell the conversation.
    fn find_log(&self, _home: &Path, _session: &str) -> Option<PathBuf> {
        None
    }

    fn on_log_entry(&self, _rt: &mut AgentRt, _entry: &Value, _fx: &mut Effects) {}

    fn says_session_lost(&self, output: &[u8]) -> bool {
        String::from_utf8_lossy(output).contains(NOT_FOUND)
    }

    /// Its sessions are in the agent's own home, whatever its account: nothing to copy.
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

    /// Escape (« esc to cancel »).
    fn interrupt_key(&self) -> &'static [u8] {
        b"\x1b"
    }

    /// Ctrl+C twice (« Press Ctrl+C again to exit »).
    fn quit_key(&self) -> &'static [u8] {
        b"\x03"
    }

    /// No hook tells the end of a turn stopped by Escape: it ends here, interrupted.
    fn interrupted(&self, rt: &mut AgentRt, fx: &mut Effects) {
        rt.gemini_interrupted(fx);
    }
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
