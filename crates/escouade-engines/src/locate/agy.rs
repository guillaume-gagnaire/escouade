//! Where Antigravity CLI is: the program of a setting, else `agy` on the PATH, else where its
//! installer leaves it (`%LOCALAPPDATA%\agy\bin` under Windows, `~/.local/bin` elsewhere), which the
//! PATH of a process started before the installation may not have (docs/TODO-1.8.md, point 0,
//! « Antigravity CLI »).

use crate::program::Program;
use std::path::{Path, PathBuf};

/// Its program's name.
const AGY: &str = "agy";

pub fn find(configured: &str) -> Option<Program> {
    let configured = configured.trim();
    // The end-to-end tests: the fake Antigravity, or none (`none`), never the machine's own.
    let forced = from_env();
    let configured = match (configured.is_empty(), forced.as_deref()) {
        (true, Some("none")) => return None,
        (true, Some(path)) => path,
        _ => configured,
    };
    let path = if configured.is_empty() {
        // Not from the unit tests: the machine's own Antigravity would answer for one that has none.
        if cfg!(any(test, feature = "testing")) {
            return None;
        }
        crate::which::find(AGY).or_else(installed)?
    } else {
        given(configured, dirs::home_dir().as_deref())?
    };
    Some(Program::Native(path))
}

/// Where its installer leaves it: `%LOCALAPPDATA%\agy\bin\agy.exe` under Windows,
/// `~/.local/bin/agy` elsewhere.
fn installed() -> Option<PathBuf> {
    let path = if cfg!(windows) {
        dirs::data_local_dir()?
            .join("agy")
            .join("bin")
            .join("agy.exe")
    } else {
        dirs::home_dir()?.join(".local").join("bin").join(AGY)
    };
    path.is_file().then_some(path)
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

/// In a debug build, `ESCOUADE_AGY` when set (end-to-end tests): the program found where none is
/// set, `none` for none. Never in the unit tests, nor in a release.
fn from_env() -> Option<String> {
    if cfg!(any(test, feature = "testing")) || !cfg!(debug_assertions) {
        return None;
    }
    std::env::var("ESCOUADE_AGY")
        .ok()
        .filter(|v| !v.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_setting_names_the_program_its_extension_left_out_and_none_is_found_in_the_tests() {
        let dir = escouade_base::testing::test_dir("esc270-agy-locate");
        let ext = if cfg!(windows) { ".exe" } else { "" };
        let agy = dir.join(format!("agy{ext}"));
        std::fs::write(&agy, "").unwrap();
        assert_eq!(given(&agy.to_string_lossy(), None), Some(agy.clone()));
        if cfg!(windows) {
            assert_eq!(given(&dir.join("agy").to_string_lossy(), None), Some(agy));
        }
        assert_eq!(given("~/nothing-here", Some(&dir)), None);
        // Not from the unit tests: none set, none found.
        assert!(find("").is_none());
    }
}
