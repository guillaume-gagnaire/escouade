//! The data folder and the paths, in `escouade-model` (docs/CRATES.md), re-exported here; the folders and links of the
//! app's tests stay.

pub use escouade_model::paths::*;
#[cfg(test)]
use std::path::{Path, PathBuf};

/// Cargo puts in the `PATH` of a test every folder a build script gives the linker, and that of
/// whisper-rs-sys gives each folder of whisper.cpp's cmake tree (46 of them): the `PATH` then
/// passes the 8191 characters cmd.exe reads, and the fake `claude.cmd` no longer finds `node`.
/// Those folders hold no program: they go, once, before a test starts anything.
#[cfg(all(test, windows))]
pub(crate) fn trim_test_path() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let Some(path) = std::env::var_os("PATH") else {
            return;
        };
        let kept: Vec<PathBuf> = std::env::split_paths(&path)
            .filter(|p| !p.to_string_lossy().contains("whisper-rs-sys-"))
            .collect();
        if let Ok(joined) = std::env::join_paths(kept) {
            // Edition 2021: safe; done once, before the test spawns anything.
            std::env::set_var("PATH", joined);
        }
    });
}

#[cfg(test)]
pub fn test_dir(name: &str) -> PathBuf {
    #[cfg(windows)]
    trim_test_path();
    let dir = std::env::temp_dir().join(format!("ccm-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // The real path (macOS: /private/var, not /var), as the processes started in it report it.
    if cfg!(windows) {
        dir
    } else {
        dir.canonicalize().unwrap()
    }
}

#[cfg(test)]
pub fn make_dir_link(target: &Path, link: &Path) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
    #[cfg(windows)]
    {
        // Try symlink first (requires admin/developer mode)
        if std::os::windows::fs::symlink_dir(target, link).is_ok() {
            return true;
        }
        // Fall back to junction (no special privileges needed)
        std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
pub fn make_file_link(target: &Path, link: &Path) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(target, link).is_ok()
    }
}
