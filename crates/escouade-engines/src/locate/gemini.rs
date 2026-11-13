//! Where Gemini CLI is: the program of a setting, else `gemini` on the PATH, else where
//! `npm install -g @google/gemini-cli` leaves it (npm's global folder, which the PATH of a process
//! started before the installation may not have: `%APPDATA%\npm` or Node's own folder under
//! Windows, where nvm links it; Homebrew's and `/usr/local/bin` elsewhere).
//!
//! Under Windows, npm's launcher `gemini.cmd` is started as it is: it runs `node` on the package's
//! `bundle/gemini.js`, both in the agent's Job (docs/TODO-1.8.md, point 0, « Gemini CLI », point 9).

use crate::program::Program;
use std::path::{Path, PathBuf};

pub fn find(configured: &str) -> Option<Program> {
    let configured = configured.trim();
    // The end-to-end tests: the fake Gemini, or none (`none`), never the machine's own.
    let forced = from_env();
    let configured = match (configured.is_empty(), forced.as_deref()) {
        (true, Some("none")) => return None,
        (true, Some(path)) => path,
        _ => configured,
    };
    let path = if configured.is_empty() {
        // Not from the unit tests: the machine's own Gemini would answer for one that has none.
        if cfg!(any(test, feature = "testing")) {
            return None;
        }
        crate::which::find("gemini").or_else(|| installed(&npm_bins()))?
    } else {
        given(configured, dirs::home_dir().as_deref())?
    };
    Some(Program::Native(path))
}

/// The folders where npm and Homebrew put their programs.
fn npm_bins() -> Vec<PathBuf> {
    let mut bins = Vec::new();
    if cfg!(windows) {
        bins.extend(dirs::data_dir().map(|d| d.join("npm")));
        if let Some(pf) = std::env::var_os("ProgramFiles") {
            bins.push(PathBuf::from(pf).join("nodejs"));
        }
    } else {
        bins.extend(dirs::home_dir().map(|h| h.join(".npm-global").join("bin")));
        bins.extend(["/opt/homebrew/bin", "/usr/local/bin"].map(PathBuf::from));
    }
    bins
}

/// Its launcher in one of the folders `bins`: `gemini.cmd` under Windows, `gemini` elsewhere.
fn installed(bins: &[PathBuf]) -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "gemini.cmd"
    } else {
        "gemini"
    };
    bins.iter().map(|d| d.join(name)).find(|p| p.is_file())
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

/// In a debug build, `ESCOUADE_GEMINI` when set (end-to-end tests): the program found where none
/// is set, `none` for none. Never in the unit tests, nor in a release.
fn from_env() -> Option<String> {
    if cfg!(any(test, feature = "testing")) || !cfg!(debug_assertions) {
        return None;
    }
    std::env::var("ESCOUADE_GEMINI")
        .ok()
        .filter(|v| !v.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npms_launcher_is_found_in_its_global_folder_and_a_setting_without_its_extension() {
        let dir = escouade_base::testing::test_dir("esc274-gemini-locate");
        let bin = dir.join("npm");
        std::fs::create_dir_all(&bin).unwrap();
        assert_eq!(installed(std::slice::from_ref(&bin)), None);
        let name = if cfg!(windows) {
            "gemini.cmd"
        } else {
            "gemini"
        };
        std::fs::write(bin.join(name), "").unwrap();
        assert_eq!(
            installed(&[dir.join("none"), bin.clone()]),
            Some(bin.join(name))
        );
        if cfg!(windows) {
            assert_eq!(
                given(&bin.join("gemini").to_string_lossy(), None),
                Some(bin.join("gemini.cmd"))
            );
        }
        assert_eq!(
            given(&bin.join("nothing.cmd").to_string_lossy(), None),
            None
        );
        // Not from the unit tests: none set, none found.
        assert!(find("").is_none());
    }
}
