//! The folders `escouade-plugin` reads, and how it writes a file; the app's `paths` takes them from here.

use std::path::{Path, PathBuf};

/// The name of the app's data folder in the home.
pub const DATA_DIR_NAME: &str = ".escouade";

/// A sandboxed data folder (end-to-end tests, demos): `$ESCOUADE_DATA_DIR`, or the former
/// `$CCM_DATA_DIR`.
pub fn sandbox_dir() -> Option<PathBuf> {
    sandbox_dir_from(|k| std::env::var_os(k))
}

pub fn sandbox_dir_from(var: impl Fn(&str) -> Option<std::ffi::OsString>) -> Option<PathBuf> {
    ["ESCOUADE_DATA_DIR", "CCM_DATA_DIR"]
        .into_iter()
        .find_map(|k| var(k).filter(|d| !d.is_empty()))
        .map(PathBuf::from)
}

/// The app's data folder as `escouade-plugin` reads it (the pinned Javy, the modules of the `run.js` plugins), moving
/// nothing: the sandboxed folder, else `~/.escouade`.
pub fn tools_data_dir() -> PathBuf {
    sandbox_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(DATA_DIR_NAME)
    })
}

/// The cache of the modules of the `run.js` plugins, in the data folder `data`.
pub fn plugin_wasm_dir(data: &Path) -> PathBuf {
    data.join("cache").join("plugins-wasm")
}

/// Where the `run.wasm` plugins compile (copies of their folders, cargo's `target`), in the data folder `data`.
pub fn plugin_rust_dir(data: &Path) -> PathBuf {
    data.join("cache").join("plugins-rust")
}

/// The programs Escouade downloads for itself (the pinned Javy of the `run.js` plugins, the toolchain of the `run.wasm`
/// ones), in the data folder `data`.
pub fn tools_dir(data: &Path) -> PathBuf {
    data.join("tools")
}

/// Writes a file atomically (temp file + rename) so a crash never leaves a truncated file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}
