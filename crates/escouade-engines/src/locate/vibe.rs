//! Where Mistral Vibe's ACP server is: the program of a setting, else `vibe-acp` on the PATH, else
//! where `uv tool install mistral-vibe` leaves it (`~/.local/bin`, which the PATH of a process
//! started before the installation may not have, then the tool's own `Scripts` or `bin` folder).
//!
//! A setting may name `vibe` (the interface) rather than `vibe-acp`: the `vibe-acp` beside it is
//! taken, the agents' processes speaking ACP (docs/TODO-1.8.md, point 0, « Mistral Vibe »).

use crate::program::Program;
use std::path::{Path, PathBuf};

/// Its ACP server's name.
pub const ACP: &str = "vibe-acp";

pub fn find(configured: &str) -> Option<Program> {
    let configured = configured.trim();
    // The end-to-end tests: the fake Vibe, or none (`none`), never the machine's own.
    let forced = from_env();
    let configured = match (configured.is_empty(), forced.as_deref()) {
        (true, Some("none")) => return None,
        (true, Some(path)) => path,
        _ => configured,
    };
    let path = if configured.is_empty() {
        // Not from the unit tests: the machine's own Vibe would answer for one that has none.
        if cfg!(any(test, feature = "testing")) {
            return None;
        }
        crate::which::find(ACP).or_else(installed)?
    } else {
        acp_of(given(configured, dirs::home_dir().as_deref())?)
    };
    Some(Program::Native(path))
}

/// Where `uv tool install` leaves it: its launcher in `~/.local/bin`, else the tool's own folder
/// (`%APPDATA%\uv\tools\mistral-vibe\Scripts` under Windows, `~/.local/share/uv/tools/…/bin`
/// elsewhere).
fn installed() -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        format!("{ACP}.exe")
    } else {
        ACP.to_string()
    };
    let home = dirs::home_dir()?;
    let mut dirs = vec![home.join(".local").join("bin")];
    if cfg!(windows) {
        if let Some(roaming) = dirs::data_dir() {
            dirs.push(
                roaming
                    .join("uv")
                    .join("tools")
                    .join("mistral-vibe")
                    .join("Scripts"),
            );
        }
    } else {
        dirs.push(
            home.join(".local")
                .join("share")
                .join("uv")
                .join("tools")
                .join("mistral-vibe")
                .join("bin"),
        );
    }
    dirs.into_iter().map(|d| d.join(&exe)).find(|p| p.is_file())
}

/// `vibe-acp` beside the interface `vibe` a setting named; any other program as it is (a fake).
fn acp_of(path: PathBuf) -> PathBuf {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if stem != "vibe" {
        return path;
    }
    let beside = match path.extension() {
        Some(ext) => path.with_file_name(format!("{ACP}.{}", ext.to_string_lossy())),
        None => path.with_file_name(ACP),
    };
    if beside.is_file() {
        beside
    } else {
        path
    }
}

/// The interface `vibe` beside the ACP server `vibe-acp` (its `-p` writes the texts, ESC-267);
/// any other program as it is (a fake plays both).
pub fn interface_of(path: &Path) -> PathBuf {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if stem != ACP {
        return path.to_path_buf();
    }
    let beside = match path.extension() {
        Some(ext) => path.with_file_name(format!("vibe.{}", ext.to_string_lossy())),
        None => path.with_file_name("vibe"),
    };
    if beside.is_file() {
        beside
    } else {
        path.to_path_buf()
    }
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

/// In a debug build, `ESCOUADE_VIBE` when set (end-to-end tests): the program found where none is
/// set, `none` for none. Never in the unit tests, nor in a release.
fn from_env() -> Option<String> {
    if cfg!(any(test, feature = "testing")) || !cfg!(debug_assertions) {
        return None;
    }
    std::env::var("ESCOUADE_VIBE")
        .ok()
        .filter(|v| !v.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_setting_naming_the_interface_gives_the_acp_server_beside_it() {
        let dir = escouade_base::testing::test_dir("esc266-vibe-locate");
        let ext = if cfg!(windows) { ".exe" } else { "" };
        let vibe = dir.join(format!("vibe{ext}"));
        std::fs::write(&vibe, "").unwrap();
        // Alone, the interface is kept as it is.
        assert_eq!(acp_of(vibe.clone()), vibe);
        let acp = dir.join(format!("vibe-acp{ext}"));
        std::fs::write(&acp, "").unwrap();
        assert_eq!(acp_of(vibe.clone()), acp);
        // Another program (a fake) is taken as it is.
        let fake = dir.join("fake-vibe.cmd");
        assert_eq!(acp_of(fake.clone()), fake);
        // The texts go to the interface beside the ACP server (ESC-267); a fake plays both.
        assert_eq!(interface_of(&acp), vibe);
        assert_eq!(interface_of(&fake), fake);
        std::fs::remove_file(&vibe).unwrap();
        assert_eq!(interface_of(&acp), acp);
        if cfg!(windows) {
            // Its extension left out, as a setting may.
            assert_eq!(
                given(&dir.join("vibe-acp").to_string_lossy(), None),
                Some(acp.clone())
            );
        }
        assert_eq!(
            given(&dir.join("nothing.exe").to_string_lossy(), None),
            None
        );
        // Not from the unit tests: none set, none found.
        assert!(find("").is_none());
    }
}
