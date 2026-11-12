//! « Vue Codex »: Codex's own interface (`codex`, its TUI) in a terminal (`TerminalView`), as the
//! real run of Codex 0.163.0 found it (docs/TODO-1.8.md, point 0, « Vue terminal »).
//!
//! - Its hooks are commands (`escouade-hook`), declared for this process alone by `-c
//!   hooks.<Event>=…`, never in the user's `config.toml`. Codex runs none it was not told to
//!   trust: their trust is given for this process too (`-c hooks.state=…`), with the keys and the
//!   hashes that `hooks/list` of an app-server started with the same hooks gives (`prepare`). The
//!   hooks already trusted of the user's are trusted again with them.
//! - Its log is the session's rollout (`sessions/YYYY/MM/DD/rollout-<date>-<session>.jsonl`),
//!   translated by `AgentRt::codex_rollout`; its hooks by `AgentRt::codex_hook`.
//! - It names its own sessions: a new one is told by its first hook (`SessionStart`, which comes
//!   with the first message).
//! - A session is resumed by `codex resume <id>`, a copy by `codex fork <id>`, in the same
//!   `CODEX_HOME` as the app-server of Escouade's view (tried both ways).
//! - On a project it does not know yet, it asks whether to trust the folder before anything else:
//!   Escouade never answers for the user, and pastes no message meanwhile (`paste_refusal`).

use super::super::terminal::{bracketed, TerminalView};
use crate::agent::{AgentRt, Effects, HookOutcome};
use crate::tui::Registered;
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// What Codex says, before it quits, of a session to resume that its folder does not have
/// (« No saved session found with ID <id>. », tried on 0.163.0).
const NO_SESSION: &str = "No saved session found";

/// The types of the images Codex takes as a path pasted in it.
const IMAGE_TYPES: [&str; 2] = ["image/png", "image/jpeg"];

/// Codex's interface.
pub struct CodexTerminal;

pub static CODEX_TERMINAL: CodexTerminal = CodexTerminal;

/// The rollout of `session` in Codex's folder `config_dir`: `sessions/<y>/<m>/<d>/rollout-<date>-
/// <session>.jsonl`, the one written last if several have it.
pub fn find_rollout(config_dir: &Path, session: &str) -> Option<PathBuf> {
    let suffix = format!("-{session}.jsonl");
    let mut found: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    let dirs = |p: &Path| -> Vec<PathBuf> {
        std::fs::read_dir(p)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect()
    };
    for year in dirs(&config_dir.join("sessions")) {
        for month in dirs(&year) {
            for day in dirs(&month) {
                for file in std::fs::read_dir(&day).into_iter().flatten().flatten() {
                    let name = file.file_name().to_string_lossy().into_owned();
                    if name.starts_with("rollout-") && name.ends_with(&suffix) {
                        let at = file.metadata().and_then(|m| m.modified()).ok();
                        found.push((at.unwrap_or(std::time::UNIX_EPOCH), file.path()));
                    }
                }
            }
        }
    }
    found.into_iter().max_by_key(|(at, _)| *at).map(|(_, p)| p)
}

/// A folder as Codex's `config.toml` names it, to compare: forward slashes, no trailing one, and
/// (Windows) no case.
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

/// The folders a `config.toml` of Codex has a trust level for (`[projects.'<folder>']` with a
/// `trust_level`, trusted or not: Codex asks for neither again).
pub(crate) fn trusted_folders(config: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut table: Option<String> = None;
    for line in config.lines().map(str::trim) {
        if line.starts_with('[') {
            table = line
                .strip_prefix("[projects.")
                .and_then(|r| r.strip_suffix(']'))
                .map(|r| r.trim().trim_matches(['\'', '"']).replace("\\\\", "\\"));
            continue;
        }
        if let Some(t) = &table {
            if line.split('=').next().map(str::trim) == Some("trust_level") {
                out.push(folder_key(t));
            }
        }
    }
    out
}

/// The repository root of the folder `cwd` (that of the project for a worktree), as Codex asks
/// the trust of it.
pub(crate) fn repo_root(cwd: &Path) -> Option<PathBuf> {
    let mut dir = Some(cwd);
    while let Some(d) = dir {
        let git = d.join(".git");
        if git.is_dir() {
            return Some(d.to_path_buf());
        }
        if git.is_file() {
            let text = std::fs::read_to_string(&git).ok()?;
            let target = text
                .trim()
                .strip_prefix("gitdir:")?
                .trim()
                .replace('\\', "/");
            return match target.split_once("/.git/worktrees/") {
                Some((root, _)) => Some(PathBuf::from(root)),
                None => Some(d.to_path_buf()),
            };
        }
        dir = d.parent();
    }
    None
}

/// Whether Codex, in `cwd` with the folder `config_dir`, opens on its question about the trust of
/// the folder: no trust level for it, its repository's root, nor a folder above it.
pub(crate) fn asks_trust(config_dir: &Path, cwd: &str) -> bool {
    let config = std::fs::read_to_string(config_dir.join("config.toml")).unwrap_or_default();
    let known = trusted_folders(&config);
    let cwd_path = Path::new(cwd);
    let mut candidates = vec![folder_key(cwd)];
    candidates.extend(repo_root(cwd_path).map(|r| folder_key(&r.to_string_lossy())));
    !candidates.iter().any(|c| {
        known
            .iter()
            .any(|k| c == k || c.starts_with(&format!("{k}/")))
    })
}

impl TerminalView for CodexTerminal {
    fn shell(&self) -> &'static str {
        "codex"
    }

    fn names_its_session(&self) -> bool {
        false
    }

    fn on_hook(&self, rt: &mut AgentRt, hook: &Value, fx: &mut Effects) -> HookOutcome {
        rt.codex_hook(hook, fx)
    }

    fn running_dir(&self, _config_dir: &Path) -> Option<PathBuf> {
        None
    }

    fn find_running(&self, _: &Path, _: &str, _: Option<&Path>) -> Option<Registered> {
        None
    }

    fn find_log(&self, config_dir: &Path, session: &str) -> Option<PathBuf> {
        find_rollout(config_dir, session)
    }

    fn on_log_entry(&self, rt: &mut AgentRt, entry: &Value, fx: &mut Effects) {
        rt.view_line(entry, fx);
    }

    fn says_session_lost(&self, output: &[u8]) -> bool {
        String::from_utf8_lossy(output).contains(NO_SESSION)
    }

    /// Its rollout, at the same place in the other folder.
    fn copy_session(&self, from: &Path, to: &Path, session: &str) -> Result<()> {
        let file = find_rollout(from, session)
            .ok_or_else(|| anyhow!("no rollout of {session} in {}", from.display()))?;
        let rel = file.strip_prefix(from)?;
        let target = to.join(rel);
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::copy(&file, &target)?;
        Ok(())
    }

    fn paste_refusal(&self, config_dir: &Path, cwd: &str, _spoke: bool) -> Option<String> {
        asks_trust(config_dir, cwd).then(|| {
            tr!(
                "Codex demande si tu fais confiance à ce dossier : réponds dans son terminal, puis renvoie ton message.",
                "Codex asks whether you trust this folder: answer in its terminal, then send your message again."
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
        &IMAGE_TYPES
    }

    /// Escape (« esc to interrupt »), which its `Interrupt` hook confirms.
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

    #[test]
    fn a_folder_known_to_codex_is_not_asked_about_again() {
        let config = "model = \"x\"\n[projects.'C:\\Users\\u\\proj']\ntrust_level = \"trusted\"\n\n[projects.\"/home/u/other\"]\ntrust_level = \"untrusted\"\n[mcp_servers.x]\nurl = \"y\"\n";
        let known = trusted_folders(config);
        assert_eq!(known.len(), 2, "{known:?}");
        assert!(known.contains(&folder_key("C:/Users/u/proj")));
        assert!(known.contains(&folder_key("/home/u/other")));
    }

    #[test]
    fn the_root_of_a_worktree_is_its_projects() {
        let dir =
            std::env::temp_dir().join(format!("escouade-codex-root-{}", crate::model::new_id()));
        let wt = dir.join("wt");
        std::fs::create_dir_all(dir.join("proj").join(".git")).unwrap();
        std::fs::create_dir_all(&wt).unwrap();
        let gitdir = dir.join("proj").join(".git").join("worktrees").join("wt");
        std::fs::write(wt.join(".git"), format!("gitdir: {}\n", gitdir.display())).unwrap();
        let root = repo_root(&wt).unwrap();
        assert_eq!(
            folder_key(&root.to_string_lossy()),
            folder_key(&dir.join("proj").to_string_lossy())
        );
        assert_eq!(
            folder_key(&repo_root(&dir.join("proj")).unwrap().to_string_lossy()),
            folder_key(&dir.join("proj").to_string_lossy())
        );
        // Codex asks about it until its config has the project.
        let home = dir.join("home");
        std::fs::create_dir_all(&home).unwrap();
        assert!(asks_trust(&home, &wt.to_string_lossy()));
        std::fs::write(
            home.join("config.toml"),
            format!(
                "[projects.'{}']\ntrust_level = \"trusted\"\n",
                dir.join("proj").display()
            ),
        )
        .unwrap();
        assert!(!asks_trust(&home, &wt.to_string_lossy()));
    }

    #[test]
    fn the_rollout_of_a_session_is_found_in_its_dated_folders() {
        let home = std::env::temp_dir().join(format!(
            "escouade-codex-rollouts-{}",
            crate::model::new_id()
        ));
        let day = home.join("sessions").join("2026").join("10").join("17");
        std::fs::create_dir_all(&day).unwrap();
        let file = day.join("rollout-2026-10-17T10-00-00-s1.jsonl");
        std::fs::write(&file, "").unwrap();
        std::fs::write(day.join("rollout-2026-10-17T10-00-00-s2.jsonl"), "").unwrap();
        assert_eq!(find_rollout(&home, "s1"), Some(file.clone()));
        assert_eq!(find_rollout(&home, "s3"), None);
        // Copied into another account's folder, at the same place.
        let other = home.join("other");
        CODEX_TERMINAL.copy_session(&home, &other, "s1").unwrap();
        assert!(find_rollout(&other, "s1").is_some());
    }
}
