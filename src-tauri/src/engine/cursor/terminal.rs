//! « Vue Cursor »: Cursor CLI's own interface (`agent`) in a terminal (`TerminalView`), as the real
//! runs of Cursor CLI 2026.10.22 found it (docs/TODO-1.8.md, point 0, « Cursor CLI », ESC-24 and
//! ESC-49).
//!
//! - Cursor keeps the conversations of its interface apart from its sessions over ACP (`chats/`
//!   and `acp-sessions/` of its configuration folder): `agent --resume <id>` of an ACP session opens
//!   an empty conversation of the interface under the same id, and `session/load` does not find a
//!   conversation of the interface. Its interface is started with the agent's session's id, its
//!   own conversation under that id kept from one opening to the next, and each side is told what
//!   was said in the other (`handover::relay`): the interface by a rule of its own in the agent's
//!   folder (`relay_path`, `.cursor/rules`, which it reads; a rule of a plugin or of a folder added
//!   by `--add-dir`, it does not), the session with its next message (`AgentMeta::relay`).
//! - Its log is the transcript it writes in `~/.cursor/projects/<folder>/agent-transcripts/<id>/`
//!   (under the real home, whatever `CURSOR_CONFIG_DIR` says): the user's messages, the agent's
//!   texts and tools (without their results), and the end of each turn (`turn_ended`), translated
//!   by `AgentRt::cursor_log`. It tells no permission nor question waiting, nor tokens.
//! - Its hooks are not used: those of a plugin (`--plugin-dir`) are merged once it has started,
//!   and the events of a turn reach them, or not, as that goes (tried). Its shell would be Git Bash
//!   with `MSYSTEM` in its environment (hooks of the user's then block each message): emptied, as
//!   for its process (`process::GIT_BASH_VARS`).
//! - It asks whether to trust a folder it does not know yet (`.workspace-trusted` of its folder in
//!   `~/.cursor/projects`, each root of the workspace its own): Escouade never answers for the
//!   user, and pastes no message until it is past the question (`paste_refusal`).

use super::super::terminal::{bracketed, TerminalArgs, TerminalLaunch, TerminalView};
use super::{base_of, AUTO};
use crate::agent::{AgentRt, Effects, HookOutcome};
use crate::tui::Registered;
use anyhow::Result;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// The variable that names another folder than `~/.cursor/projects` (the end-to-end tests, whose
/// fake `agent` writes there too).
pub(crate) const PROJECTS_VAR: &str = "ESCOUADE_CURSOR_PROJECTS";

/// The file, in a workspace's folder of `projects_dir`, that says it is trusted.
const TRUSTED: &str = ".workspace-trusted";

/// The rule Escouade writes in the agent's folder for its interface (`relay_path`).
pub(crate) const RELAY_RULE: &str = "escouade-relais.mdc";

/// Cursor's interface.
pub struct CursorTerminal;

pub static CURSOR_TERMINAL: CursorTerminal = CursorTerminal;

/// Where Cursor keeps what it knows of each workspace (its transcripts, its trust): `~/.cursor/
/// projects`, under the real home (tried with `CURSOR_CONFIG_DIR` set); `ESCOUADE_CURSOR_PROJECTS`
/// when set. In the unit tests, never the machine user's (`accounts::app_env`).
pub(crate) fn projects_dir() -> PathBuf {
    match std::env::var_os(PROJECTS_VAR).filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => crate::accounts::app_env()
            .1
            .join(".cursor")
            .join("projects"),
    }
}

/// The name of a workspace's folder in `projects_dir`, as Cursor makes it from its path: each run
/// of what is neither a letter nor a digit is a `-`, none at the ends
/// (`C:\Users\u\repo` → `C-Users-u-repo`).
pub(crate) fn project_key(cwd: &str) -> String {
    let mut out = String::new();
    for c in cwd.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

/// The transcript of the conversation `session` among the workspaces of `projects`, once it is
/// written (its first message).
pub(crate) fn find_transcript(projects: &Path, session: &str) -> Option<PathBuf> {
    if session.is_empty() || session.contains(['/', '\\', '.']) {
        return None;
    }
    std::fs::read_dir(projects)
        .ok()?
        .flatten()
        .map(|p| {
            p.path()
                .join("agent-transcripts")
                .join(session)
                .join(format!("{session}.jsonl"))
        })
        .find(|f| f.is_file())
}

/// Whether Cursor, in `cwd`, opens on its question about the trust of the folder (`projects`: its
/// folder of the workspaces).
pub(crate) fn asks_trust(projects: &Path, cwd: &str) -> bool {
    !projects.join(project_key(cwd)).join(TRUSTED).is_file()
}

/// The model its `--model` takes for Escouade's `model`: Auto by its name, another by its id
/// without its options (Cursor gives it those of the account).
fn model_arg(model: &str) -> Option<String> {
    match model {
        "" => None,
        AUTO => Some("auto".into()),
        m => Some(base_of(m).to_string()),
    }
}

/// The arguments of an agent's interface: no update looked for, its session (the agent's own, or
/// the one Escouade names), its model and its mode.
pub(crate) fn tui_args(l: &TerminalLaunch) -> Vec<String> {
    let m = l.meta;
    let mut a: Vec<String> = vec!["--disable-auto-update".into()];
    if let Some(s) = m.session_id.as_deref().or(l.session) {
        a.extend(["--resume".into(), s.to_string()]);
    }
    if let Some(model) = model_arg(&m.model) {
        a.extend(["--model".into(), model]);
    }
    // Its own modes: Plan, and « Run Everything » for « Tout accepter »; the others ask, as in
    // Escouade's view where its writes never ask.
    match m.mode.as_str() {
        "plan" => a.extend(["--mode".into(), "plan".into()]),
        "bypassPermissions" => a.push("--force".into()),
        _ => {}
    }
    a
}

impl TerminalView for CursorTerminal {
    fn shell(&self) -> &'static str {
        "agent"
    }

    /// Its folder of the workspaces given too, which Cursor does not read: the fake `agent` of the
    /// tests writes there, a pseudo-console under Windows not passing the app's own variables.
    fn launch(&self, l: &TerminalLaunch) -> TerminalArgs {
        TerminalArgs {
            args: tui_args(l),
            env: vec![(
                PROJECTS_VAR.to_string(),
                projects_dir().to_string_lossy().into_owned(),
            )],
            ..Default::default()
        }
    }

    /// Its shell PowerShell, never Git Bash (Escouade started from Git Bash): `MSYSTEM` empty is
    /// none for Cursor (`if (!process.env.MSYSTEM)`, read in 2026.10.22).
    fn masked_env(&self, _env: &[(String, String)]) -> Vec<(String, String)> {
        super::process::GIT_BASH_VARS
            .iter()
            .map(|v| (v.to_string(), String::new()))
            .collect()
    }

    /// None: each root of a workspace is asked about apart (tried with `--add-dir`), and Cursor
    /// reads the files joined to a message without being given their folder.
    fn read_dir_args(&self, _dir: &Path) -> Vec<String> {
        Vec::new()
    }

    fn names_its_session(&self) -> bool {
        true
    }

    fn resumes_any(&self) -> bool {
        true
    }

    fn relay_path(&self, cwd: &Path) -> Option<PathBuf> {
        Some(cwd.join(".cursor").join("rules").join(RELAY_RULE))
    }

    /// A rule always applied (`alwaysApply`), as Cursor's own are written.
    fn relay_file(&self, text: &str) -> Vec<u8> {
        format!(
            "---\ndescription: Escouade — la conversation d’avant / the conversation before\nalwaysApply: true\n---\n\n{}\n",
            text.trim()
        )
        .into_bytes()
    }

    /// No hook is given to it: nothing comes.
    fn on_hook(&self, _rt: &mut AgentRt, _hook: &Value, _fx: &mut Effects) -> HookOutcome {
        HookOutcome::default()
    }

    fn running_dir(&self, _config_dir: &Path) -> Option<PathBuf> {
        None
    }

    fn find_running(&self, _: &Path, _: &str, _: Option<&Path>) -> Option<Registered> {
        None
    }

    fn find_log(&self, _config_dir: &Path, session: &str) -> Option<PathBuf> {
        find_transcript(&projects_dir(), session)
    }

    fn on_log_entry(&self, rt: &mut AgentRt, entry: &Value, fx: &mut Effects) {
        rt.view_line(entry, fx);
    }

    fn says_session_lost(&self, _output: &[u8]) -> bool {
        false
    }

    /// Its conversations are in the agent's own configuration folder, whatever its account: there
    /// is nothing to copy.
    fn copy_session(&self, _from: &Path, _to: &Path, _session: &str) -> Result<()> {
        Ok(())
    }

    /// Until it told something, an interface on a folder it does not trust may still be on its
    /// question about it.
    fn paste_refusal(&self, _config_dir: &Path, cwd: &str, spoke: bool) -> Option<String> {
        (!spoke && asks_trust(&projects_dir(), cwd)).then(|| {
            tr!(
                "Cursor demande si tu fais confiance à ce dossier : réponds dans son terminal (« Trust this workspace »), puis renvoie ton message.",
                "Cursor asks whether you trust this folder: answer in its terminal (“Trust this workspace”), then send your message again."
            )
        })
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

    /// Ctrl+C (« ctrl+c to stop »), which its transcript tells as a turn `aborted`.
    fn interrupt_key(&self) -> &'static [u8] {
        b"\x03"
    }

    /// Ctrl+C twice (« Press Ctrl+C again to exit »).
    fn quit_key(&self) -> &'static [u8] {
        b"\x03"
    }
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
