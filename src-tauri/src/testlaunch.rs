//! Test launches of a worktree and what goes with them: its reserved ports, the files copied from
//! the project into a new worktree, the tests run before a validation, the readiness of a server,
//! and what a step of its recipe runs.

// Used by the orchestrator (tickets.rs) as the tasks go; this allowance goes in task 17.
#![allow(dead_code)]

use crate::board::PORT_BLOCK;
use crate::model::AgentMeta;
use crate::paths;
use crate::pty::{self, ShellInfo};
use anyhow::{anyhow, bail, Result};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

// ---------- ports ----------

/// The port binds on 127.0.0.1: nothing listens on it.
pub fn port_free(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// The variables of a reserved block, exported to its test launches.
pub fn port_env(base: Option<u16>) -> Vec<(String, String)> {
    match base {
        Some(b) => vec![
            ("ESCOUADE_PORT_BASE".into(), b.to_string()),
            (
                "ESCOUADE_PORT_END".into(),
                b.saturating_add(PORT_BLOCK - 1).to_string(),
            ),
        ],
        None => Vec::new(),
    }
}

// ---------- files copied into a new worktree ----------

/// `path` (forward slashes) matches `pattern`: `*` and `?` within a segment, `**` for any number
/// of segments.
pub fn glob_match(pattern: &str, path: &str) -> bool {
    let p: Vec<&str> = pattern
        .trim()
        .trim_start_matches("./")
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    let s: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    match_segments(&p, &s)
}

fn match_segments(p: &[&str], s: &[&str]) -> bool {
    match p.first() {
        None => s.is_empty(),
        Some(&"**") => (0..=s.len()).any(|i| match_segments(&p[1..], &s[i..])),
        Some(seg) => !s.is_empty() && match_one(seg, s[0]) && match_segments(&p[1..], &s[1..]),
    }
}

fn match_one(pattern: &str, name: &str) -> bool {
    fn go(p: &[char], s: &[char]) -> bool {
        match p.first() {
            None => s.is_empty(),
            Some('*') => (0..=s.len()).any(|i| go(&p[1..], &s[i..])),
            Some('?') => !s.is_empty() && go(&p[1..], &s[1..]),
            Some(c) => s.first() == Some(c) && go(&p[1..], &s[1..]),
        }
    }
    let (p, s): (Vec<char>, Vec<char>) = (pattern.chars().collect(), name.chars().collect());
    go(&p, &s)
}

/// Files of `dir` that git does not track (ignored ones included, folders wholly untracked left
/// out) matching one of `patterns`, relative with forward slashes.
pub async fn matching_untracked(dir: &str, patterns: &[String]) -> Vec<String> {
    if patterns.iter().all(|p| p.trim().is_empty()) {
        return Vec::new();
    }
    let out = crate::git::run(
        dir,
        &[
            "ls-files",
            "-z",
            "--others",
            "--directory",
            "--no-empty-directory",
        ],
    )
    .await
    .unwrap_or_default();
    String::from_utf8_lossy(&out)
        .split('\0')
        .filter(|f| !f.is_empty() && !f.ends_with('/'))
        .filter(|f| patterns.iter().any(|p| glob_match(p, f)))
        .map(str::to_string)
        .collect()
}

/// Copies into `worktree` the untracked files of `project` that match `patterns` (`.env*`…).
pub async fn copy_worktree_files(
    project: &str,
    worktree: &str,
    patterns: &[String],
) -> Result<Vec<String>> {
    let files = matching_untracked(project, patterns).await;
    for f in &files {
        let to = Path::new(worktree).join(f);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(Path::new(project).join(f), &to)?;
    }
    Ok(files)
}

// ---------- readiness ----------

/// One HTTP request to `url`, without any proxy, 2 s at most: true for any answer, whatever its
/// status (a 404 still means the server is up).
pub async fn http_ready(url: &str) -> bool {
    let Ok(client) = reqwest::Client::builder()
        // A local server must never be reached through the user's proxy.
        .no_proxy()
        .timeout(Duration::from_secs(2))
        .build()
    else {
        return false;
    };
    client.get(url).send().await.is_ok()
}

// ---------- tests before a validation ----------

/// What the tests run before a validation gave.
#[derive(Debug, Clone, PartialEq)]
pub struct TestRun {
    pub passed: bool,
    /// Their last lines, for the agent.
    pub tail: String,
}

pub fn tail_lines(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.trim_end().lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

/// The arguments making `shell` run `command` and end with its exit code, as launch commands do.
fn shell_args(shell_id: &str, command: &str) -> Vec<String> {
    match shell_id {
        "pwsh" | "powershell" => vec![
            "-NoLogo".into(),
            "-Command".into(),
            pty::with_exit_code(command),
        ],
        "bash" => vec!["--login".into(), "-c".into(), command.into()],
        "wsl" => vec!["--".into(), "bash".into(), "-lc".into(), command.into()],
        _ => vec!["-l".into(), "-c".into(), command.into()],
    }
}

/// Runs `command` in `cwd` with `shell`, `env` added; killed with what it started past `limit`.
pub async fn run_tests(
    shell: &ShellInfo,
    cwd: &str,
    command: &str,
    env: &[(String, String)],
    limit: Duration,
) -> Result<TestRun> {
    let mut cmd = tokio::process::Command::new(&shell.path);
    cmd.args(shell_args(&shell.id, command))
        .current_dir(cwd)
        .envs(env.iter().cloned())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if shell.id == "bash" {
        // Git Bash stays in the working directory instead of going home.
        cmd.env("CHERE_INVOKING", "1");
    }
    #[cfg(windows)]
    cmd.creation_flags(crate::claude::CREATE_NO_WINDOW);
    crate::job::isolate(&mut cmd);
    let child = cmd.spawn()?;
    let job = crate::job::Job::for_child(&child);
    match tokio::time::timeout(limit, child.wait_with_output()).await {
        Ok(out) => {
            let out = out?;
            let text = format!(
                "{}\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            Ok(TestRun {
                passed: out.status.success(),
                tail: tail_lines(&crate::agent::strip_ansi(&text), 80),
            })
        }
        Err(_) => {
            if let Some(j) = &job {
                j.terminate();
            }
            Ok(TestRun {
                passed: false,
                tail: format!(
                    "Les tests n'ont pas fini en {} min.",
                    (limit.as_secs() / 60).max(1)
                ),
            })
        }
    }
}

// ---------- steps of a recipe ----------

/// What a step of a recipe runs, where, with which variables.
#[derive(Debug, Clone, PartialEq)]
pub struct RunSpec {
    pub name: String,
    pub command: String,
    pub cwd: String,
    pub env: Vec<(String, String)>,
}

/// Step `index` of the agent's recipe: its preparation ("prep") or its processes ("run"). Its
/// folder must stay inside the worktree; the reserved ports come first in its variables.
pub fn run_spec(meta: &AgentMeta, kind: &str, index: usize) -> Result<RunSpec> {
    let recipe = meta
        .recipe
        .as_ref()
        .ok_or_else(|| anyhow!("Cet agent n'a pas de recette de lancement."))?;
    let root = meta
        .worktree
        .as_ref()
        .map(|w| w.path.clone())
        .unwrap_or_else(|| meta.cwd.clone());
    let (name, command, dir, extra): (String, String, String, Vec<(String, String)>) = match kind {
        "prep" => {
            let s = recipe
                .prepare
                .get(index)
                .ok_or_else(|| anyhow!("Étape de préparation introuvable."))?;
            (
                format!("Préparation {}", index + 1),
                s.command.clone(),
                s.dir.clone(),
                Vec::new(),
            )
        }
        _ => {
            let p = recipe
                .processes
                .get(index)
                .ok_or_else(|| anyhow!("Processus introuvable."))?;
            let name = if p.name.trim().is_empty() {
                format!("processus {}", index + 1)
            } else {
                p.name.clone()
            };
            let env = p.env.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            (name, p.command.clone(), p.dir.clone(), env)
        }
    };
    let dir = dir.trim();
    let cwd = if dir.is_empty() || dir == "." {
        root.clone()
    } else {
        paths::contained(Path::new(&root), dir)?
            .to_string_lossy()
            .into_owned()
    };
    if !Path::new(&cwd).is_dir() {
        bail!("Le dossier « {dir} » n'existe pas dans le worktree");
    }
    let mut env = port_env(meta.port_base);
    env.extend(extra);
    Ok(RunSpec {
        name,
        command,
        cwd,
        env,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{RecipeProcess, RecipeStep, Settings, TestRecipe, Worktree};
    use crate::paths::test_dir;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn git(dir: &Path, args: &[&str]) {
        let ok = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?}");
    }

    #[test]
    fn globs_match_one_segment_or_any_depth() {
        assert!(glob_match(".env*", ".env") && glob_match(".env*", ".env.local"));
        assert!(!glob_match(".env*", "web/.env"));
        assert!(glob_match("**/.env*", "web/.env.local") && glob_match("**/.env*", ".env"));
        assert!(glob_match("config/?.json", "config/a.json"));
        assert!(!glob_match("config/?.json", "config/ab.json"));
        assert!(glob_match("./secrets/*.key", "secrets/a.key"));
        assert!(!glob_match("", ".env"));
    }

    #[tokio::test]
    async fn untracked_files_matching_the_patterns_are_copied_into_a_worktree() {
        let p = test_dir("launch-copy");
        git(&p, &["init", "-q", "-b", "main"]);
        std::fs::create_dir_all(p.join("web")).unwrap();
        std::fs::write(p.join("web").join("index.ts"), "x").unwrap();
        std::fs::write(p.join(".gitignore"), ".env*\nnode_modules/\n").unwrap();
        git(&p, &["add", "-A"]);
        git(
            &p,
            &[
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "commit",
                "-qm",
                "init",
            ],
        );
        std::fs::write(p.join(".env"), "A=1").unwrap();
        std::fs::write(p.join("web").join(".env.local"), "B=2").unwrap();
        std::fs::create_dir_all(p.join("node_modules").join("x")).unwrap();
        std::fs::write(p.join("node_modules").join("x").join(".env"), "no").unwrap();
        std::fs::write(p.join("notes.txt"), "n").unwrap();
        let wt = test_dir("launch-copy-wt");
        let patterns = vec![".env*".to_string(), "**/.env*".to_string()];
        let (from, to) = (
            p.to_string_lossy().to_string(),
            wt.to_string_lossy().to_string(),
        );
        let mut copied = copy_worktree_files(&from, &to, &patterns).await.unwrap();
        copied.sort();
        assert_eq!(copied, [".env", "web/.env.local"]);
        assert_eq!(
            std::fs::read_to_string(wt.join("web").join(".env.local")).unwrap(),
            "B=2"
        );
        assert!(!wt.join("notes.txt").exists() && !wt.join("node_modules").exists());
        assert!(copy_worktree_files(&from, &to, &[])
            .await
            .unwrap()
            .is_empty());
    }

    #[test]
    fn a_port_is_free_only_when_it_binds() {
        let busy = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = busy.local_addr().unwrap().port();
        assert!(!port_free(port));
        drop(busy);
        assert!(port_free(port));
    }

    #[test]
    fn a_block_exports_its_first_and_last_ports_and_a_corrupt_one_cannot_overflow() {
        assert!(port_env(None).is_empty());
        assert_eq!(
            port_env(Some(4100))[1],
            ("ESCOUADE_PORT_END".to_string(), "4109".to_string())
        );
        // A base read back from a damaged file must not panic on the last port.
        assert_eq!(port_env(Some(u16::MAX))[1].1, "65535");
    }

    /// A server answering every request with a 404, on a port of its own.
    fn not_found_server() -> u16 {
        let l = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for mut s in l.incoming().flatten() {
                let mut buf = [0u8; 2048];
                let _ = s.read(&mut buf);
                let _ = s.write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
            }
        });
        port
    }

    #[tokio::test]
    async fn a_server_is_ready_once_it_answers_whatever_its_status() {
        let port = not_found_server();
        assert!(http_ready(&format!("http://localhost:{port}/connexion")).await);
        let closed = TcpListener::bind(("127.0.0.1", 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        assert!(!http_ready(&format!("http://127.0.0.1:{closed}/")).await);
        assert!(!http_ready("pas une adresse").await);
    }

    fn default_shell() -> Option<ShellInfo> {
        pty::detect_shells(&Settings::default()).into_iter().next()
    }

    #[tokio::test]
    async fn tests_report_their_outcome_and_their_last_lines_with_the_ports_exported() {
        let Some(shell) = default_shell() else { return };
        let dir = test_dir("launch-tests").to_string_lossy().to_string();
        let env = port_env(Some(4130));
        let ok = run_tests(
            &shell,
            &dir,
            "node -e \"process.exit(process.env.ESCOUADE_PORT_END === '4139' ? 0 : 4)\"",
            &env,
            Duration::from_secs(120),
        )
        .await
        .unwrap();
        assert!(ok.passed, "{}", ok.tail);
        let ko = run_tests(
            &shell,
            &dir,
            "node -e \"for (let i = 1; i <= 100; i++) console.log('ligne ' + i); process.exit(2)\"",
            &env,
            Duration::from_secs(120),
        )
        .await
        .unwrap();
        assert!(!ko.passed);
        assert_eq!(ko.tail.lines().count(), 80, "{}", ko.tail);
        assert!(ko.tail.trim_end().ends_with("ligne 100"), "{}", ko.tail);
    }

    #[tokio::test]
    async fn tests_running_past_their_limit_are_stopped() {
        let Some(shell) = default_shell() else { return };
        let dir = test_dir("launch-tests-slow").to_string_lossy().to_string();
        let r = run_tests(
            &shell,
            &dir,
            "node -e \"setTimeout(() => {}, 60000)\"",
            &[],
            Duration::from_secs(4),
        )
        .await
        .unwrap();
        assert!(!r.passed);
        assert!(r.tail.contains("n'ont pas fini"), "{}", r.tail);
    }

    #[test]
    fn a_recipe_step_runs_in_its_folder_with_the_ports_and_its_variables() {
        let wt = test_dir("launch-spec");
        std::fs::create_dir_all(wt.join("web")).unwrap();
        let recipe = TestRecipe {
            prepare: vec![RecipeStep {
                command: "npm install".into(),
                dir: "web".into(),
            }],
            processes: vec![
                RecipeProcess {
                    name: "web".into(),
                    command: "npm run dev".into(),
                    env: [("PORT".to_string(), "4101".to_string())].into(),
                    url: "http://localhost:4101".into(),
                    ..Default::default()
                },
                RecipeProcess {
                    name: "x".into(),
                    command: "y".into(),
                    dir: "../dehors".into(),
                    ..Default::default()
                },
            ],
            open: String::new(),
        };
        let root = wt.to_string_lossy().to_string();
        let meta = AgentMeta {
            cwd: root.clone(),
            worktree: Some(Worktree {
                path: root.clone(),
                branch: "ticket/x".into(),
                base_branch: "main".into(),
            }),
            port_base: Some(4100),
            recipe: Some(recipe),
            ..Default::default()
        };
        let prep = run_spec(&meta, "prep", 0).unwrap();
        assert_eq!(
            Path::new(&prep.cwd).canonicalize().unwrap(),
            wt.join("web").canonicalize().unwrap()
        );
        assert_eq!(
            (prep.name.as_str(), prep.command.as_str()),
            ("Préparation 1", "npm install")
        );
        let run = run_spec(&meta, "run", 0).unwrap();
        assert_eq!(run.cwd, root);
        assert_eq!(
            run.env,
            vec![
                ("ESCOUADE_PORT_BASE".to_string(), "4100".to_string()),
                ("ESCOUADE_PORT_END".to_string(), "4109".to_string()),
                ("PORT".to_string(), "4101".to_string()),
            ]
        );
        assert!(run_spec(&meta, "run", 1).is_err());
        assert!(run_spec(&meta, "run", 5).is_err());
        assert!(run_spec(&AgentMeta::default(), "run", 0).is_err());
    }
}
