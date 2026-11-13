//! Where Cursor CLI is: the program of a setting, else the launcher its installer leaves in
//! `%LOCALAPPDATA%\cursor-agent` (Windows), else `cursor-agent` or `agent` on the PATH.
//!
//! Its launcher (`agent.cmd`, `cursor-agent.cmd`, or the script of `~/.local/bin`) runs the
//! `node` and the `index.js` of its newest version (`versions/<YYYY.MM.DD-commit>`), through
//! PowerShell under Windows: Escouade starts those two itself for an agent's process, so that it
//! is the one in the agent's Job and no staged update is installed under it (docs/TODO-1.8.md,
//! point 0, « Cursor CLI »). A launcher with no version beside it (the tests' fake) is started as
//! it is.

use crate::program::Program;
use std::path::{Path, PathBuf};

/// The names of its launcher, its new one first.
const NAMES: [&str; 2] = ["agent", "cursor-agent"];

pub fn find(configured: &str) -> Option<Program> {
    let configured = configured.trim();
    // The end-to-end tests: the fake agent, or none (`none`), never the machine's own.
    let forced = from_env();
    let configured = match (configured.is_empty(), forced.as_deref()) {
        (true, Some("none")) => return None,
        (true, Some(path)) => path,
        _ => configured,
    };
    let path = if configured.is_empty() {
        // Not from the unit tests: the machine's own agent would answer for one that has none.
        if cfg!(any(test, feature = "testing")) {
            return None;
        }
        installed().or_else(|| {
            ["cursor-agent", "agent"]
                .iter()
                .find_map(|n| crate::which::find(n))
        })?
    } else {
        given(configured, dirs::home_dir().as_deref())?
    };
    Some(Program::Native(path))
}

/// The launcher its installer leaves under Windows, which the PATH of a process started before
/// the installation may not have.
fn installed() -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    let dir = dirs::data_local_dir()?.join("cursor-agent");
    NAMES
        .iter()
        .map(|n| dir.join(format!("{n}.cmd")))
        .find(|p| p.is_file())
}

/// The program a setting names: `~` for the home folder and, under Windows, its extension left
/// out.
fn given(configured: &str, home: Option<&Path>) -> Option<PathBuf> {
    let path = match configured.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with(['/', '\\']) => {
            home?.join(rest.trim_start_matches(['/', '\\']))
        }
        _ => PathBuf::from(configured),
    };
    if path.is_file() {
        return Some(path);
    }
    if !cfg!(windows) || path.extension().is_some() {
        return None;
    }
    ["cmd", "exe"]
        .iter()
        .map(|e| path.with_extension(e))
        .find(|p| p.is_file())
}

/// The `node` and the `index.js` its launcher `launcher` runs: those beside it (a launcher in a
/// version's folder), else those of the newest version of the `versions` folder beside it. None:
/// the launcher is started as it is.
pub fn native_of(launcher: &Path) -> Option<(PathBuf, PathBuf)> {
    // `~/.local/bin/agent` is a link to the version's own script (never canonical otherwise: under
    // Windows, that would give a verbatim path).
    let linked = std::fs::symlink_metadata(launcher).is_ok_and(|m| m.file_type().is_symlink());
    let real = if linked {
        std::fs::canonicalize(launcher).unwrap_or_else(|_| launcher.to_path_buf())
    } else {
        launcher.to_path_buf()
    };
    let dir = real.parent()?;
    if let Some(pair) = pair_in(dir) {
        return Some(pair);
    }
    newest(&dir.join("versions"))
        .or_else(|| newest(&launcher.parent()?.join("versions")))
        .and_then(|v| pair_in(&v))
}

/// The `node` and the `index.js` of a version's folder, both there.
fn pair_in(dir: &Path) -> Option<(PathBuf, PathBuf)> {
    let node = dir.join(if cfg!(windows) { "node.exe" } else { "node" });
    let index = dir.join("index.js");
    (node.is_file() && index.is_file()).then_some((node, index))
}

/// The newest version of `versions` (`2026.10.22-27dcf5f`, or `2026.10.22-10-30-00-27dcf5f`), by
/// its date then its name, as its launcher picks it.
fn newest(versions: &Path) -> Option<PathBuf> {
    let mut found: Vec<(escouade_plugin_cli::manifest::Version, String, PathBuf)> =
        std::fs::read_dir(versions)
            .ok()?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let date = escouade_plugin_cli::manifest::Version::find(&name)?;
                Some((date, name, e.path()))
            })
            .filter(|(_, _, p)| pair_in(p).is_some())
            .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    found.pop().map(|(_, _, p)| p)
}

/// In a debug build, `ESCOUADE_CURSOR` when set (end-to-end tests): the program found where none
/// is set, `none` for none. Never in the unit tests, nor in a release.
fn from_env() -> Option<String> {
    if cfg!(any(test, feature = "testing")) || !cfg!(debug_assertions) {
        return None;
    }
    std::env::var("ESCOUADE_CURSOR")
        .ok()
        .filter(|v| !v.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_launcher_gives_way_to_the_node_and_script_of_its_newest_version() {
        let dir = escouade_base::testing::test_dir("esc42-cursor-locate");
        let launcher = dir.join("agent.cmd");
        std::fs::write(&launcher, "@echo off\r\n").unwrap();
        // No version beside it (the tests' fake): the launcher itself.
        assert_eq!(native_of(&launcher), None);
        let node = if cfg!(windows) { "node.exe" } else { "node" };
        for v in [
            "2026.10.21-aaaaaaa",
            "2026.10.22-27dcf5f",
            "2026.9.30-bbbbbbb",
        ] {
            let d = dir.join("versions").join(v);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join(node), "").unwrap();
            std::fs::write(d.join("index.js"), "").unwrap();
        }
        // A newer folder without its program is no version.
        std::fs::create_dir_all(dir.join("versions").join("2026.11.01-ccccccc")).unwrap();
        let newest = dir.join("versions").join("2026.10.22-27dcf5f");
        assert_eq!(
            native_of(&launcher),
            Some((newest.join(node), newest.join("index.js")))
        );
        if cfg!(windows) {
            // Its extension left out, as a setting may.
            assert_eq!(
                given(&dir.join("agent").to_string_lossy(), None),
                Some(launcher.clone())
            );
        }
        assert_eq!(
            given(&dir.join("nothing.cmd").to_string_lossy(), None),
            None
        );
    }
}
