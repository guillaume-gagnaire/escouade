//! Where the isola CLI is (`proc/*` of a plugin runs it, the app's `isola` re-exports `cli`): looked for on the PATH, or
//! the fake of the tests (feature `testing`).

use std::path::PathBuf;
use std::sync::Mutex;
#[cfg(not(any(test, feature = "testing")))]
use std::time::{Duration, Instant};

/// The last look for the CLI on the PATH, and when it was made.
#[cfg(not(any(test, feature = "testing")))]
static CLI: Mutex<Option<(Option<PathBuf>, Instant)>> = Mutex::new(None);

/// The CLI the tests use (never the machine's own).
#[cfg(any(test, feature = "testing"))]
pub static TEST_CLI: Mutex<Option<PathBuf>> = Mutex::new(None);

/// The isola CLI, looked for on the PATH at most every 30 s: installed while the app runs, it is
/// found soon after. As the host looks for it (`proc/*`): what the plugin cannot run is not installed.
pub fn cli() -> Option<PathBuf> {
    #[cfg(any(test, feature = "testing"))]
    {
        TEST_CLI.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
    #[cfg(not(any(test, feature = "testing")))]
    {
        let mut cached = CLI.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((found, at)) = cached.as_ref() {
            if at.elapsed() < Duration::from_secs(30) {
                return found.clone();
            }
        }
        let found = crate::procs::locate("isola");
        *cached = Some((found.clone(), Instant::now()));
        found
    }
}

/// The fake isola of the tests and what it was asked.
#[cfg(any(test, feature = "testing"))]
pub mod tests {
    use super::TEST_CLI;
    use std::path::{Path, PathBuf};

    /// The fake isola (tests/fixtures/fake-isola.mjs), set as the CLI of every test: only a
    /// worktree with an `.isola.toml` is its.
    pub fn use_fake() -> PathBuf {
        let fake = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("fixtures")
            .join(if cfg!(windows) {
                "fake-isola.cmd"
            } else {
                "fake-isola"
            });
        *TEST_CLI.lock().unwrap() = Some(fake.clone());
        fake
    }

    /// The calls the fake isola got in `dir`: their arguments (its log is kept out of the folder,
    /// which may go).
    pub fn calls(dir: &Path) -> Vec<String> {
        let log = |dir: &Path| {
            let key: String = dir
                .to_string_lossy()
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                .collect();
            std::fs::read_to_string(std::env::temp_dir().join(format!("fake-isola-{key}.log")))
                .unwrap_or_default()
        };
        // The fake names its log after node's `process.cwd()`, the real path: under macOS a temporary folder
        // (/var/folders/…) is a link to /private/var/folders/….
        let mut text = log(dir);
        if text.is_empty() {
            if let Ok(real) = dir.canonicalize() {
                text = log(&real);
            }
        }
        text.lines().map(str::to_string).collect()
    }
}
