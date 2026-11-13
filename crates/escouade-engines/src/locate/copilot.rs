//! Where `copilot` is: the program of a setting, else the one on the PATH.
//!
//! Under Windows, npm's launcher (`copilot.cmd`, `copilot.ps1`) runs a script of Node that starts
//! the native `copilot.exe` of the platform's package (`@github/copilot-win32-x64`): Escouade starts
//! that one itself, so that its process is the one in the agent's Job (docs/TODO-1.8.md, point 0,
//! « Copilot CLI »). A launcher with no package beside it (the tests' fake) is started as it is.

use crate::program::Program;
use std::path::{Path, PathBuf};

/// Where npm's package keeps the native program, from the folder of its launcher.
const NATIVE: [&str; 2] = [
    "node_modules/@github/copilot/node_modules/@github/copilot-win32-x64/copilot.exe",
    "node_modules/@github/copilot-win32-x64/copilot.exe",
];

pub fn find(configured: &str) -> Option<Program> {
    let configured = configured.trim();
    // The end-to-end tests: the fake copilot, or none (`none`), never the machine's own.
    let forced = from_env();
    let configured = match (configured.is_empty(), forced.as_deref()) {
        (true, Some("none")) => return None,
        (true, Some(path)) => path,
        _ => configured,
    };
    let path = if configured.is_empty() {
        // Not from the unit tests: the machine's own copilot would answer for one that has none.
        if cfg!(any(test, feature = "testing")) {
            return None;
        }
        crate::which::find("copilot")?
    } else {
        given(configured, dirs::home_dir().as_deref())?
    };
    Some(Program::Native(native_of(&path)))
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
    ["exe", "cmd"]
        .iter()
        .map(|e| path.with_extension(e))
        .find(|p| p.is_file())
}

/// The native program behind npm's launcher, else the program itself.
fn native_of(path: &Path) -> PathBuf {
    let launcher = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "cmd" | "ps1"));
    if !cfg!(windows) || !launcher {
        return path.to_path_buf();
    }
    let dir = path.parent().unwrap_or(Path::new(""));
    NATIVE
        .iter()
        .map(|rel| dir.join(rel))
        .find(|p| p.is_file())
        .unwrap_or_else(|| path.to_path_buf())
}

/// In a debug build, `ESCOUADE_COPILOT` when set (end-to-end tests): the program found where none
/// is set, `none` for none. Never in the unit tests, nor in a release.
fn from_env() -> Option<String> {
    if cfg!(any(test, feature = "testing")) || !cfg!(debug_assertions) {
        return None;
    }
    std::env::var("ESCOUADE_COPILOT")
        .ok()
        .filter(|v| !v.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npms_launcher_gives_way_to_the_native_program_beside_it() {
        let dir = escouade_base::testing::test_dir("esc39-copilot-locate");
        let launcher = dir.join("copilot.cmd");
        std::fs::write(&launcher, "@echo off\r\n").unwrap();
        // No package beside it (the tests' fake): the launcher itself.
        assert_eq!(native_of(&launcher), launcher);
        let native = dir.join(NATIVE[1]);
        std::fs::create_dir_all(native.parent().unwrap()).unwrap();
        std::fs::write(&native, "").unwrap();
        if cfg!(windows) {
            assert_eq!(native_of(&launcher), native);
            // Its extension left out, as a setting may.
            assert_eq!(
                given(&dir.join("copilot").to_string_lossy(), None),
                Some(dir.join("copilot.cmd"))
            );
        }
        assert_eq!(
            given(&dir.join("nothing.exe").to_string_lossy(), None),
            None
        );
    }
}
