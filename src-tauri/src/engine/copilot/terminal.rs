//! « Vue Copilot »: Copilot CLI's own interface (`copilot`) in a terminal (`TerminalView`), as the
//! real runs of Copilot CLI 1.0.98 found it (docs/TODO-1.8.md, point 0, « Vue terminal », ESC-23
//! and ESC-48).
//!
//! - Its hooks are commands (`escouade-hook <event>`), in a plugin of the agent's own given by
//!   `--plugin-dir` (`<plugin>/plugin.json`, `<plugin>/hooks/hooks.json`): for this process alone,
//!   never in the `COPILOT_HOME` of the account, which the user's own sessions read too (tried:
//!   `hooks.discover` lists them as the plugin's, a server started without the option has none).
//!   Their payloads do not say their event: its name is the command's argument.
//! - Its log is the session's `session-state/<id>/events.jsonl`, the events of the session as its
//!   server sends them, translated by `AgentRt::copilot_log`; its hooks by `AgentRt::copilot_hook`.
//! - Escouade names a new session (`--session-id <uuid>`); a session is resumed by `--resume <id>`,
//!   in the same `COPILOT_HOME` as the server of Escouade's view (tried both ways). Its interface
//!   cannot copy a session: a copy is made first by a server of its own (`sessions.fork`).
//! - On a folder its account does not trust (`trustedFolders` of its `config.json`, the folder or
//!   one above it), it asks first, each time it starts unless told to remember the folder.
//!   Escouade never answers for the user, and pastes no message until it is past the question
//!   (`paste_refusal`).

use super::super::terminal::{bracketed, TerminalView};
use crate::agent::{AgentRt, Effects, HookOutcome};
use crate::tui::Registered;
use anyhow::{bail, Result};
#[cfg(test)]
use serde_json::json;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// What Copilot says, before it quits, of a session to resume that its folder does not have
/// (« Error: No session, task, or name matched '<id>'. », tried on 1.0.98).
const NO_SESSION: &str = "No session, task, or name matched";

/// How long after a first Escape the second goes, which interrupts the turn.
const INTERRUPT_AGAIN: Duration = Duration::from_millis(400);

/// Copilot's interface.
pub struct CopilotTerminal;

pub static COPILOT_TERMINAL: CopilotTerminal = CopilotTerminal;

/// The plugin of the agent's interface, beside its private file.
pub(crate) fn plugin_dir(private_file: &Path) -> PathBuf {
    private_file.with_extension("copilot-plugin")
}

/// The name of the folder of the data of the plugin `plugin` (given by `--plugin-dir`) in
/// `plugin-data/_direct` of the account's folder: the SHA-256 of its source as Copilot writes it,
/// `{"source":"local","path":"<its folder>"}` (tried on 1.0.98, ESC-122: the folder resolved,
/// absolute as Escouade gives it).
pub(crate) fn plugin_data_id(plugin: &Path) -> String {
    use sha2::Digest as _;
    let path = serde_json::to_string(&plugin.to_string_lossy()).unwrap_or_default();
    let source = format!(r#"{{"source":"local","path":{path}}}"#);
    sha2::Sha256::digest(source.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The folder of the session `session` in Copilot's folder `config_dir`.
fn session_dir(config_dir: &Path, session: &str) -> PathBuf {
    config_dir.join("session-state").join(session)
}

/// The log of `session`, once something is written in it (Copilot does not resume a session with
/// no event, tried).
pub fn find_events(config_dir: &Path, session: &str) -> Option<PathBuf> {
    let file = session_dir(config_dir, session).join("events.jsonl");
    std::fs::metadata(&file)
        .is_ok_and(|m| m.is_file() && m.len() > 0)
        .then_some(file)
}

/// A folder as Copilot's `config.json` names it, to compare: forward slashes, no trailing one,
/// and (Windows) no case.
fn folder_key(p: &str) -> String {
    let s = p
        .trim()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string();
    let s = s.strip_prefix("//?/").unwrap_or(&s).to_string();
    if cfg!(windows) {
        s.to_lowercase()
    } else {
        s
    }
}

/// The folders a `config.json` of Copilot trusts (`trustedFolders`; its first lines are comments).
pub(crate) fn trusted_folders(config: &str) -> Vec<String> {
    let json: String = config
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    serde_json::from_str::<Value>(&json)
        .ok()
        .and_then(|v| v["trustedFolders"].as_array().cloned())
        .into_iter()
        .flatten()
        .filter_map(|f| f.as_str().map(folder_key))
        .collect()
}

/// Whether Copilot, in `cwd` with the folder `config_dir`, opens on its question about the trust
/// of the folder: neither it nor a folder above it trusted (tried: a parent trusted is enough).
pub(crate) fn asks_trust(config_dir: &Path, cwd: &str) -> bool {
    let config = std::fs::read_to_string(config_dir.join("config.json")).unwrap_or_default();
    let cwd = folder_key(cwd);
    !trusted_folders(&config)
        .iter()
        .any(|k| cwd == *k || cwd.starts_with(&format!("{k}/")))
}

/// Copies the folder `from` into `to`, its files and folders (not its locks).
fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if name.to_string_lossy().ends_with(".lock") {
            continue;
        }
        let (src, dst) = (entry.path(), to.join(&name));
        if entry.file_type()?.is_dir() {
            copy_dir(&src, &dst)?;
        } else {
            std::fs::copy(&src, &dst)?;
        }
    }
    Ok(())
}

impl TerminalView for CopilotTerminal {
    fn shell(&self) -> &'static str {
        "copilot"
    }

    /// Its plugin, and the folder of the plugin's data Copilot made for it in the account's folder
    /// and leaves there, empty (ESC-122): never one Copilot wrote something in.
    fn forget(&self, private_file: &Path, config_dir: &Path) {
        let plugin = plugin_dir(private_file);
        let data = config_dir
            .join("plugin-data")
            .join("_direct")
            .join(plugin_data_id(&plugin));
        let _ = std::fs::remove_dir(data);
        let _ = std::fs::remove_dir_all(plugin);
    }

    /// Its interface copies no session: its plugin does, by a server of the account (`fork`).
    fn copies_first(&self) -> bool {
        true
    }

    fn names_its_session(&self) -> bool {
        true
    }

    fn on_hook(&self, rt: &mut AgentRt, hook: &Value, fx: &mut Effects) -> HookOutcome {
        rt.copilot_hook(hook, fx)
    }

    fn running_dir(&self, _config_dir: &Path) -> Option<PathBuf> {
        None
    }

    fn find_running(&self, _: &Path, _: &str, _: Option<&Path>) -> Option<Registered> {
        None
    }

    fn find_log(&self, config_dir: &Path, session: &str) -> Option<PathBuf> {
        find_events(config_dir, session)
    }

    fn on_log_entry(&self, rt: &mut AgentRt, entry: &Value, fx: &mut Effects) {
        rt.view_line(entry, fx);
    }

    fn says_session_lost(&self, output: &[u8]) -> bool {
        String::from_utf8_lossy(output).contains(NO_SESSION)
    }

    /// Its folder (its log, its workspace), at the same place in the other folder.
    fn copy_session(&self, from: &Path, to: &Path, session: &str) -> Result<()> {
        let dir = session_dir(from, session);
        if find_events(from, session).is_none() {
            bail!("no events of {session} in {}", from.display());
        }
        copy_dir(&dir, &session_dir(to, session))
    }

    /// Until it told something, an interface on a folder its account does not trust may still be
    /// on its question about it (answered « Yes » for this session only, nothing is written).
    fn paste_refusal(&self, config_dir: &Path, cwd: &str, spoke: bool) -> Option<String> {
        (!spoke && asks_trust(config_dir, cwd)).then(|| {
            tr!(
                "Copilot demande si tu fais confiance à ce dossier : réponds dans son terminal, puis renvoie ton message (« Yes, and remember this folder » évite la question aux prochains lancements).",
                "Copilot asks whether you trust this folder: answer in its terminal, then send your message again (“Yes, and remember this folder” spares you the question next time)."
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

    /// Escape (« esc interrupt »), which its log tells as `abort`.
    fn interrupt_key(&self) -> &'static [u8] {
        b"\x1b"
    }

    /// Escape again (« esc again to interrupt »): once only asks for it (tried, ESC-48).
    fn interrupt_again(&self) -> Option<Duration> {
        Some(INTERRUPT_AGAIN)
    }

    fn quit_key(&self) -> &'static [u8] {
        b"\x03"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_folder_of_its_plugins_data_is_named_as_copilot_names_it() {
        // As Copilot CLI 1.0.98 named it, for this `--plugin-dir` (ESC-122).
        assert_eq!(
            plugin_data_id(Path::new(
                r"C:\Users\guill\AppData\Local\Temp\esc122\plug\a1.copilot-plugin"
            )),
            "f80c2e5503860b741dc7be243d8483040555b6d8455b2939e86854ee98da12b1"
        );
    }

    #[test]
    fn a_folder_trusted_or_under_a_trusted_one_is_not_asked_about() {
        let config = "// User settings belong in settings.json.\n// This file is managed automatically.\n{\n  \"firstLaunchAt\": \"2026-10-18T09:59:47.522Z\",\n  \"trustedFolders\": [\n    \"C:\\\\Users\\\\u\\\\proj\"\n  ]\n}\n";
        assert_eq!(trusted_folders(config), [folder_key("C:/Users/u/proj")]);
        let home =
            std::env::temp_dir().join(format!("escouade-copilot-trust-{}", crate::model::new_id()));
        std::fs::create_dir_all(&home).unwrap();
        let proj = home.join("proj");
        let wt = proj.join("wt");
        let cwd = wt.to_string_lossy();
        assert!(asks_trust(&home, &cwd));
        assert!(COPILOT_TERMINAL.paste_refusal(&home, &cwd, false).is_some());
        // Past it (it told something since it started): nothing is refused.
        assert!(COPILOT_TERMINAL.paste_refusal(&home, &cwd, true).is_none());
        let trusted = json!({ "trustedFolders": [proj.to_string_lossy()] });
        std::fs::write(home.join("config.json"), format!("// x\n{trusted}")).unwrap();
        assert!(!asks_trust(&home, &cwd));
        assert!(!asks_trust(&home, &proj.to_string_lossy()));
        assert!(asks_trust(&home, &home.to_string_lossy()));
    }

    #[test]
    fn a_sessions_log_is_found_once_written_and_copied_with_its_folder() {
        let home = std::env::temp_dir().join(format!(
            "escouade-copilot-sessions-{}",
            crate::model::new_id()
        ));
        let dir = session_dir(&home, "s1");
        std::fs::create_dir_all(dir.join("checkpoints")).unwrap();
        std::fs::write(dir.join("workspace.yaml"), "id: s1\n").unwrap();
        std::fs::write(dir.join(".workspace-fork.lock"), "").unwrap();
        // Created, not written yet: Copilot would not resume it.
        std::fs::write(dir.join("events.jsonl"), "").unwrap();
        assert_eq!(find_events(&home, "s1"), None);
        std::fs::write(dir.join("events.jsonl"), "{\"type\":\"session.start\"}\n").unwrap();
        assert_eq!(find_events(&home, "s1"), Some(dir.join("events.jsonl")));
        std::fs::write(dir.join("checkpoints").join("index.md"), "x").unwrap();
        let other = home.join("other");
        COPILOT_TERMINAL.copy_session(&home, &other, "s1").unwrap();
        assert!(find_events(&other, "s1").is_some());
        assert!(session_dir(&other, "s1")
            .join("checkpoints")
            .join("index.md")
            .is_file());
        assert!(!session_dir(&other, "s1")
            .join(".workspace-fork.lock")
            .exists());
        assert!(COPILOT_TERMINAL.copy_session(&home, &other, "s2").is_err());
        assert!(COPILOT_TERMINAL.says_session_lost(
            b"Error: No session, task, or name matched 's9'.\r\nTo resume by session or task ID:"
        ));
    }
}
