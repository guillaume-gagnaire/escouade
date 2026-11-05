//! isola (github.com/cyucelen/isola) runs each git worktree's services on ports and `*.localhost` addresses of its
//! own, from the `.isola.toml` the project commits. Since INT-14 it is a launch environment, served by the plugin the
//! app gives (`plugins/isola/`, `launchenv`): the core only knows where the CLI is (`cli`) and
//! the shape of a service (`Service`). `isola up`, `ls`, `down` and `destroy` are the plugin's and the test launch's.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
#[cfg(not(test))]
use std::time::{Duration, Instant};

/// The last look for the CLI on the PATH, and when it was made.
#[cfg(not(test))]
static CLI: Mutex<Option<(Option<PathBuf>, Instant)>> = Mutex::new(None);

/// The CLI the tests use (never the machine's own).
#[cfg(test)]
pub static TEST_CLI: Mutex<Option<PathBuf>> = Mutex::new(None);

/// The isola CLI, looked for on the PATH at most every 30 s: installed while the app runs, it is
/// found soon after. As the host looks for it (`proc/*`): what the plugin cannot run is not installed.
pub fn cli() -> Option<PathBuf> {
    #[cfg(test)]
    {
        TEST_CLI.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
    #[cfg(not(test))]
    {
        let mut cached = CLI.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((found, at)) = cached.as_ref() {
            if at.elapsed() < Duration::from_secs(30) {
                return found.clone();
            }
        }
        let found = crate::plugins::procs::locate("isola");
        *cached = Some((found.clone(), Instant::now()));
        found
    }
}

/// A service of a worktree, for its test launch (the answer of the plugin's `launch/ls`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    pub name: String,
    /// "running", "stopped"…, as isola says it.
    pub status: String,
    /// Its address through isola's proxy, else on its own port; empty for a background process.
    pub url: String,
    /// What tells it is up: its own port (the proxy answers, with an error page, before it does),
    /// else `url`.
    pub probe: String,
}

#[cfg(test)]
pub(crate) use tests::{run, LIMIT, UP};

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::paths::test_dir;
    use std::path::Path;
    use std::process::Stdio;
    use std::time::Duration;

    /// What a test launch runs (the manifest of the plugin says `isola up`).
    pub const UP: &str = if cfg!(windows) {
        "isola up"
    } else {
        "isola up < /dev/null"
    };

    /// How long `isola ls`, `down` or `destroy` may take.
    pub const LIMIT: Duration = Duration::from_secs(60);

    /// The fake isola (tests/fixtures/fake-isola.mjs), set as the CLI of every test: only a
    /// worktree with an `.isola.toml` is its.
    pub(crate) fn use_fake() -> PathBuf {
        let fake = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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

    /// `isola <args>` in `dir`: what it printed (the test double stands in for the plugin where the test needs the
    /// worktree up).
    pub(crate) async fn run(dir: &str, args: &[&str], limit: Duration) -> anyhow::Result<String> {
        let cli = cli().ok_or_else(|| anyhow::anyhow!("isola not found"))?;
        let mut cmd = tokio::process::Command::new(&cli);
        cmd.args(args)
            .current_dir(dir)
            .env("NO_COLOR", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(windows)]
        cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
        let out = tokio::time::timeout(limit, cmd.output()).await??;
        anyhow::ensure!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    /// The calls the fake isola got in `dir`: their arguments (its log is kept out of the folder,
    /// which may go).
    pub(crate) fn calls(dir: &Path) -> Vec<String> {
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

    #[tokio::test]
    async fn the_fake_cli_runs_in_the_folder_and_its_failure_says_why() {
        use_fake();
        let d = test_dir("isola-run");
        let dir = d.to_string_lossy().to_string();
        std::fs::write(d.join(".isola-branch"), "feat").unwrap();
        assert_eq!(
            run(&dir, &["ls", "--json"], LIMIT).await.unwrap().trim(),
            "[]"
        );
        run(&dir, &["up"], LIMIT).await.unwrap();
        assert!(run(&dir, &["ls", "--json"], LIMIT)
            .await
            .unwrap()
            .contains("feat.demo.localhost"));
        assert_eq!(calls(&d), ["ls --json", "up", "ls --json"]);
        std::fs::write(d.join(".isola-fail"), "").unwrap();
        let e = run(&dir, &["down"], LIMIT).await.unwrap_err().to_string();
        assert!(e.contains("the fake isola fails"), "{e}");
    }
}
