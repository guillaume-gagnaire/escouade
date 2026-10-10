//! isola (github.com/cyucelen/isola) runs each git worktree's services on ports and
//! `*.localhost` addresses of its own, from the `.isola.toml` the project commits. When it is
//! installed and a worktree has that file, the worktree's test launches go through it (`isola up`,
//! the addresses `isola ls --json` gives, `isola down`) instead of a recipe on reserved ports, and
//! it tears the worktree's services and databases down (`isola destroy`) before the worktree goes.

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use std::time::Duration;
#[cfg(not(test))]
use std::time::Instant;

/// The file that says a worktree's services are isola's.
pub const CONFIG: &str = ".isola.toml";

/// What a test launch runs. Its input is not a terminal: isola never waits there for a password
/// (to trust its HTTPS certificate), it says how to do it instead.
pub const UP: &str = if cfg!(windows) {
    "isola up"
} else {
    "isola up < /dev/null"
};

/// How long `isola ls`, `down` or `destroy` may take.
pub const LIMIT: Duration = Duration::from_secs(60);

/// The last look for the CLI on the PATH, and when it was made.
#[cfg(not(test))]
static CLI: Mutex<Option<(Option<PathBuf>, Instant)>> = Mutex::new(None);

/// The CLI the tests use (never the machine's own).
#[cfg(test)]
pub static TEST_CLI: Mutex<Option<PathBuf>> = Mutex::new(None);

/// The isola CLI, looked for on the PATH at most every 30 s: installed while the app runs, it is
/// found soon after.
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
        let found = crate::which::find("isola");
        *cached = Some((found.clone(), Instant::now()));
        found
    }
}

/// The folder `dir` has isola's configuration.
pub fn configured(dir: &str) -> bool {
    Path::new(dir).join(CONFIG).is_file()
}

/// The largest `.isola.toml` the user is asked to read: a bigger one is refused, never cut.
const CONFIG_LIMIT: u64 = 256 * 1024;

/// The content of the `.isola.toml` in `dir`, whole, as `isola up` reads it: what the user approves, and
/// what is compared with it when the services start.
pub fn read_config(dir: &str) -> Result<String> {
    let path = Path::new(dir).join(CONFIG);
    let len = std::fs::metadata(&path)
        .map_err(|e| anyhow!("{CONFIG} illisible : {e}"))?
        .len();
    if len > CONFIG_LIMIT {
        bail!("{CONFIG} est trop gros pour être relu avant le lancement ({len} octets).");
    }
    let bytes = std::fs::read(&path).map_err(|e| anyhow!("{CONFIG} illisible : {e}"))?;
    String::from_utf8(bytes).map_err(|_| anyhow!("{CONFIG} n'est pas en UTF-8."))
}

/// `rev` of the repository at `repo` has isola's configuration (committed).
pub async fn configured_on(repo: &str, rev: &str) -> bool {
    !rev.starts_with('-')
        && crate::git::run(repo, &["cat-file", "-e", &format!("{rev}:{CONFIG}")])
            .await
            .is_ok()
}

/// isola runs the services of the worktree at `dir`: it is installed, and configured there.
pub fn manages(dir: &str) -> bool {
    configured(dir) && cli().is_some()
}

/// A line of `isola ls --json`: a service of a worktree.
#[derive(Debug, Deserialize)]
struct Entry {
    #[serde(default)]
    worktree: String,
    #[serde(default)]
    service: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    direct_url: String,
}

/// A service of a worktree, for its test launch.
#[derive(Debug, Clone, Serialize, PartialEq)]
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

/// The services `ls` (the output of `isola ls --json`) lists for the worktree on `branch`.
pub fn services_of(ls: &str, branch: &str) -> Result<Vec<Service>> {
    let ls = ls.trim();
    // An empty list may come out as `null`.
    let entries: Vec<Entry> = if ls.is_empty() || ls == "null" {
        Vec::new()
    } else {
        serde_json::from_str(ls).map_err(|e| anyhow!("réponse d'isola illisible : {e}"))?
    };
    Ok(entries
        .into_iter()
        .filter(|e| e.worktree == branch)
        .map(|e| {
            let url = if e.url.is_empty() {
                e.direct_url.clone()
            } else {
                e.url
            };
            let probe = if e.direct_url.is_empty() {
                url.clone()
            } else {
                e.direct_url
            };
            Service {
                name: e.service,
                status: e.status,
                url,
                probe,
            }
        })
        .collect())
}

/// `isola <args>` in `dir`, `limit` at most: what it printed. Not in a job of the app's: the
/// services `up` starts outlive it on purpose.
pub async fn run(dir: &str, args: &[&str], limit: Duration) -> Result<String> {
    let cli = cli().ok_or_else(|| anyhow!("isola introuvable"))?;
    let mut cmd = tokio::process::Command::new(&cli);
    cmd.args(args)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(crate::claude::CREATE_NO_WINDOW);
    let out = tokio::time::timeout(limit, cmd.output())
        .await
        .map_err(|_| {
            anyhow!(
                "isola {} n'a pas répondu en {} s",
                args.join(" "),
                limit.as_secs()
            )
        })??;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let line = err
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("");
        bail!("isola {} a échoué : {}", args.join(" "), line.trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Stops the services of the worktree at `dir` (`isola down`) in the background; a failure is
/// only logged.
pub fn down_later(dir: String) {
    tauri::async_runtime::spawn(async move {
        if let Err(e) = run(&dir, &["down"], LIMIT).await {
            log::warn!("{dir}: {e:#}");
        }
    });
}

/// The services of the worktree at `dir`, on `branch`.
pub async fn services(dir: &str, branch: &str) -> Result<Vec<Service>> {
    services_of(&run(dir, &["ls", "--json"], LIMIT).await?, branch)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::paths::test_dir;

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

    /// The calls the fake isola got in `dir`: their arguments (its log is kept out of the folder,
    /// which may go).
    pub(crate) fn calls(dir: &Path) -> Vec<String> {
        let key: String = dir
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        std::fs::read_to_string(std::env::temp_dir().join(format!("fake-isola-{key}.log")))
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn the_worktrees_services_open_on_their_proxy_address_and_are_probed_on_their_own() {
        let ls = r#"[
            {"worktree":"main","service":"web","port":3100,"status":"running","pid":1,"url":"http://main.demo.localhost:3000","direct_url":"http://localhost:3100"},
            {"worktree":"ticket/dem-1","service":"api","port":8117,"status":"running","pid":2,"direct_url":"http://localhost:8117"},
            {"worktree":"ticket/dem-1","service":"web","port":3117,"status":"running","pid":3,"url":"http://ticket-dem-1.demo.localhost:3000","direct_url":"http://localhost:3117"},
            {"worktree":"ticket/dem-1","service":"worker","port":0,"status":"stopped","pid":0}
        ]"#;
        let s = services_of(ls, "ticket/dem-1").unwrap();
        let got: Vec<(&str, &str, &str, &str)> = s
            .iter()
            .map(|s| {
                (
                    s.name.as_str(),
                    s.status.as_str(),
                    s.url.as_str(),
                    s.probe.as_str(),
                )
            })
            .collect();
        // The proxy answers (502) before the service does: the service itself is probed.
        assert_eq!(
            got,
            [
                (
                    "api",
                    "running",
                    "http://localhost:8117",
                    "http://localhost:8117"
                ),
                (
                    "web",
                    "running",
                    "http://ticket-dem-1.demo.localhost:3000",
                    "http://localhost:3117"
                ),
                ("worker", "stopped", "", ""),
            ]
        );
        assert!(services_of("null\n", "main").unwrap().is_empty());
        assert!(services_of("", "main").unwrap().is_empty());
        assert!(services_of("[]", "main").unwrap().is_empty());
        assert!(services_of("isola: not a repo", "main").is_err());
    }

    #[test]
    fn only_a_folder_with_its_configuration_is_isolas() {
        use_fake();
        let d = test_dir("isola-configured");
        let dir = d.to_string_lossy().to_string();
        assert!(!manages(&dir));
        std::fs::write(d.join(CONFIG), "setup = \"npm ci\"\n").unwrap();
        assert!(configured(&dir) && manages(&dir));
    }

    #[test]
    fn the_configuration_is_read_whole_or_refused_never_cut() {
        let d = test_dir("isola-read-config");
        let dir = d.to_string_lossy().to_string();
        assert!(read_config(&dir).is_err());
        let text = "[services.web]\r\ncommand = \"npm run dev\"\r\n";
        std::fs::write(d.join(CONFIG), text).unwrap();
        // As it is on disk, line endings included: this is what is compared when the services start.
        assert_eq!(read_config(&dir).unwrap(), text);
        std::fs::write(d.join(CONFIG), b"setup = \"\xff\"").unwrap();
        assert!(read_config(&dir).is_err());
        std::fs::write(d.join(CONFIG), "#".repeat(CONFIG_LIMIT as usize + 1)).unwrap();
        let e = read_config(&dir).unwrap_err().to_string();
        assert!(e.contains("trop gros"), "{e}");
        std::fs::write(d.join(CONFIG), "#".repeat(CONFIG_LIMIT as usize)).unwrap();
        assert_eq!(read_config(&dir).unwrap().len(), CONFIG_LIMIT as usize);
    }

    #[tokio::test]
    async fn a_branch_is_isolas_once_it_has_its_configuration_committed() {
        let d = test_dir("isola-on-branch");
        let git = |args: &[&str]| {
            let ok = std::process::Command::new("git")
                .arg("-C")
                .arg(&d)
                .args(["-c", "user.email=t@t", "-c", "user.name=t"])
                .args(args)
                .status()
                .unwrap()
                .success();
            assert!(ok, "git {args:?}");
        };
        git(&["init", "-q", "-b", "main"]);
        git(&["commit", "-q", "--allow-empty", "-m", "init"]);
        git(&["branch", "before"]);
        std::fs::write(d.join(CONFIG), "").unwrap();
        let repo = d.to_string_lossy().to_string();
        // In the folder, not yet committed: no branch has it.
        assert!(!configured_on(&repo, "main").await);
        git(&["add", CONFIG]);
        git(&["commit", "-q", "-m", "isola"]);
        assert!(configured_on(&repo, "main").await);
        assert!(!configured_on(&repo, "before").await);
        assert!(!configured_on(&repo, "--output=x").await);
    }

    #[tokio::test]
    async fn the_cli_runs_in_the_folder_and_its_failure_says_why() {
        use_fake();
        let d = test_dir("isola-run");
        let dir = d.to_string_lossy().to_string();
        std::fs::write(d.join(CONFIG), "").unwrap();
        // The fake lists its services once `up` ran, on the branch it was given.
        std::fs::write(d.join(".isola-branch"), "feat").unwrap();
        assert!(services(&dir, "feat").await.unwrap().is_empty());
        run(&dir, &["up"], LIMIT).await.unwrap();
        let s = services(&dir, "feat").await.unwrap();
        assert_eq!(s[0].name, "web");
        assert_eq!(s[0].url, "http://feat.demo.localhost:3000");
        assert_eq!(s[0].probe, "http://127.0.0.1:9");
        assert_eq!(calls(&d), ["ls --json", "up", "ls --json"]);
        std::fs::write(d.join(".isola-fail"), "").unwrap();
        let e = run(&dir, &["down"], LIMIT).await.unwrap_err().to_string();
        assert_eq!(e, "isola down a échoué : Error: the fake isola fails");
    }
}
