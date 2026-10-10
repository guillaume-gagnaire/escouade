//! Git integration through the `git` CLI (always present with Git for Windows).

use crate::model::{Commit, FileChange};
use anyhow::{bail, Result};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use serde::Serialize;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

pub(crate) fn command(cwd: &str, args: &[&str]) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new("git");
    cmd.arg("-C")
        .arg(cwd)
        .arg("--no-optional-locks")
        // Accented paths verbatim (UTF-8) instead of "\303\251" escapes in diff headers.
        .args(["-c", "core.quotepath=false"])
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(crate::claude::CREATE_NO_WINDOW);
    cmd
}

pub async fn run(cwd: &str, args: &[&str]) -> Result<Vec<u8>> {
    let out = command(cwd, args).output().await?;
    checked(out, args)
}

/// Time limits of the commands that reach a remote.
const NET_TIMEOUT: Duration = Duration::from_secs(180);
const NET_TIMEOUT_BACKGROUND: Duration = Duration::from_secs(60);

/// Nothing may ask for credentials in the background: no Git Credential Manager window, no
/// askpass program (GIT_ASKPASS set but empty also skips core.askPass and SSH_ASKPASS), no ssh
/// passphrase prompt.
fn never_prompt(cmd: &mut tokio::process::Command) {
    cmd.env("GCM_INTERACTIVE", "never")
        .env("GIT_ASKPASS", "")
        .env_remove("SSH_ASKPASS")
        .env("SSH_ASKPASS_REQUIRE", "never");
}

/// Runs a command that reaches a remote, killed past its time limit. A background one never
/// asks for credentials and dies with everything it started (credential helper, ssh…). The
/// user's may ask: Git Credential Manager can then start a browser, which must outlive the
/// command, so no job object for those.
async fn run_net(cwd: &str, args: &[&str], background: bool) -> Result<Vec<u8>> {
    run_net_command(command(cwd, args), args, background).await
}

/// `cmd` with git's messages in English, whatever the system's language: for the commands whose
/// refusals are told apart by their text.
fn in_english(mut cmd: tokio::process::Command) -> tokio::process::Command {
    cmd.env("LC_ALL", "C");
    cmd
}

/// `run_net`, with git's messages in English (`in_english`).
async fn run_net_english(cwd: &str, args: &[&str], background: bool) -> Result<Vec<u8>> {
    run_net_command(in_english(command(cwd, args)), args, background).await
}

async fn run_net_command(
    mut cmd: tokio::process::Command,
    args: &[&str],
    background: bool,
) -> Result<Vec<u8>> {
    cmd.kill_on_drop(true);
    if background {
        never_prompt(&mut cmd);
        crate::job::isolate(&mut cmd);
    }
    let child = cmd.spawn()?;
    let job = if background {
        crate::job::Job::for_child(&child)
    } else {
        None
    };
    let limit = if background {
        NET_TIMEOUT_BACKGROUND
    } else {
        NET_TIMEOUT
    };
    match tokio::time::timeout(limit, child.wait_with_output()).await {
        Ok(out) => checked(out?, args),
        Err(_) => {
            if let Some(j) = &job {
                j.terminate();
            }
            bail!(tr!(
                "git {cmd} interrompu : toujours pas terminé après {s} s",
                "git {cmd} stopped: still not done after {s} s",
                cmd = args.first().unwrap_or(&""),
                s = limit.as_secs()
            ))
        }
    }
}

fn checked(out: std::process::Output, args: &[&str]) -> Result<Vec<u8>> {
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let msg = if err.is_empty() {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        } else {
            err
        };
        bail!(if msg.is_empty() {
            tr!("git {a} a échoué", "git {a} failed", a = args.join(" "))
        } else {
            msg
        });
    }
    Ok(out.stdout)
}

pub async fn text(cwd: &str, args: &[&str]) -> Result<String> {
    Ok(String::from_utf8_lossy(&run(cwd, args).await?)
        .trim()
        .to_string())
}

pub async fn toplevel(path: &str) -> Option<String> {
    text(path, &["rev-parse", "--show-toplevel"])
        .await
        .ok()
        .filter(|s| !s.is_empty())
}

pub async fn current_branch(path: &str) -> String {
    text(path, &["rev-parse", "--abbrev-ref", "HEAD"])
        .await
        .unwrap_or_default()
}

/// The branch HEAD is on, even one without any commit yet (a new repository); empty on a detached
/// HEAD.
pub async fn head_branch(path: &str) -> String {
    text(path, &["symbolic-ref", "--short", "-q", "HEAD"])
        .await
        .unwrap_or_default()
}

pub async fn init_repo(path: &str) -> Result<()> {
    run(path, &["init"]).await.map(|_| ())
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub path: String,
    /// 'M', 'A' or 'D'.
    pub status: char,
    /// The path before a rename.
    pub orig: Option<String>,
    /// Not in the index yet (a new file never added).
    pub untracked: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Status {
    /// "(detached)" for a detached HEAD.
    pub branch: String,
    /// The branch it tracks ("origin/main"), if any.
    pub upstream: Option<String>,
    /// The upstream no longer exists on the remote (deleted, e.g. once merged).
    pub upstream_gone: bool,
    /// Commits not in the upstream / in the upstream only, as of the last fetch.
    pub ahead: u32,
    pub behind: u32,
    pub entries: Vec<Entry>,
}

pub async fn status(cwd: &str) -> Result<Status> {
    let out = run(
        cwd,
        &[
            "status",
            "--porcelain=v2",
            "-z",
            "--branch",
            "--untracked-files=all",
        ],
    )
    .await?;
    Ok(parse_status(&out))
}

pub fn parse_status(out: &[u8]) -> Status {
    let text = String::from_utf8_lossy(out);
    let tokens: Vec<&str> = text.split('\0').collect();
    let mut st = Status::default();
    let mut counted = false;
    let mut i = 0;
    while i < tokens.len() {
        let t = tokens[i];
        i += 1;
        if let Some(head) = t.strip_prefix("# branch.head ") {
            st.branch = head.to_string();
            continue;
        }
        if let Some(up) = t.strip_prefix("# branch.upstream ") {
            st.upstream = Some(up.to_string());
            continue;
        }
        if let Some(ab) = t.strip_prefix("# branch.ab ") {
            // "+<ahead> -<behind>"
            let mut n = ab
                .split(' ')
                .map(|s| s.trim_start_matches(['+', '-']).parse().unwrap_or(0));
            st.ahead = n.next().unwrap_or(0);
            st.behind = n.next().unwrap_or(0);
            counted = true;
            continue;
        }
        let mut orig = None;
        let (xy, path) = match t.chars().next() {
            Some('1') => {
                let f: Vec<&str> = t.splitn(9, ' ').collect();
                (f.get(1).copied().unwrap_or(".."), f.get(8).copied())
            }
            Some('2') => {
                let f: Vec<&str> = t.splitn(10, ' ').collect();
                orig = tokens.get(i).map(|o| o.to_string());
                i += 1;
                (f.get(1).copied().unwrap_or(".."), f.get(9).copied())
            }
            Some('u') => {
                let f: Vec<&str> = t.splitn(11, ' ').collect();
                (f.get(1).copied().unwrap_or(".."), f.get(10).copied())
            }
            Some('?') => ("??", t.get(2..)),
            _ => continue,
        };
        let Some(path) = path.filter(|p| !p.is_empty()) else {
            continue;
        };
        let mut c = xy.chars();
        let (x, y) = (c.next().unwrap_or('.'), c.next().unwrap_or('.'));
        let status = if x == 'D' || y == 'D' {
            'D'
        } else if x == 'A' || xy == "??" {
            'A'
        } else {
            'M'
        };
        st.entries.push(Entry {
            path: path.to_string(),
            status,
            orig,
            untracked: xy == "??",
        });
    }
    // Git counts only against an upstream that still exists.
    st.upstream_gone = st.upstream.is_some() && !counted;
    st
}

/// Added/deleted line counts per path (relative to the repository root), against HEAD.
pub async fn numstat(root: &str) -> HashMap<String, (u32, u32)> {
    let out = match run(root, &["diff", "HEAD", "--numstat", "-z"]).await {
        Ok(o) => o,
        Err(_) => run(root, &["diff", "--cached", "--numstat", "-z"])
            .await
            .unwrap_or_default(),
    };
    parse_numstat(&out)
}

pub fn parse_numstat(out: &[u8]) -> HashMap<String, (u32, u32)> {
    let text = String::from_utf8_lossy(out);
    let tokens: Vec<&str> = text.split('\0').collect();
    let mut map = HashMap::new();
    let mut i = 0;
    while i < tokens.len() {
        let parts: Vec<&str> = tokens[i].splitn(3, '\t').collect();
        i += 1;
        if parts.len() < 3 {
            continue;
        }
        let (a, d) = (parts[0].parse().unwrap_or(0), parts[1].parse().unwrap_or(0));
        let path = if parts[2].is_empty() {
            // Rename: "<a>\t<d>\t\0<old>\0<new>\0"
            let p = tokens.get(i + 1).copied().unwrap_or_default();
            i += 2;
            p
        } else {
            parts[2]
        };
        if !path.is_empty() {
            map.insert(path.to_string(), (a, d));
        }
    }
    map
}

fn count_lines(path: &Path) -> u32 {
    let Ok(meta) = std::fs::metadata(path) else {
        return 0;
    };
    if meta.len() > 2_000_000 {
        return 0;
    }
    match std::fs::read(path) {
        Ok(bytes) if !bytes.contains(&0) => {
            let n = bytes.iter().filter(|&&b| b == b'\n').count();
            (n + usize::from(!bytes.is_empty() && !bytes.ends_with(b"\n"))) as u32
        }
        _ => 0,
    }
}

/// Dirty files of a working tree with line stats. Paths are relative to `root`.
pub async fn file_changes(root: &str) -> Result<Vec<FileChange>> {
    let st = status(root).await?;
    let stats = numstat(root).await;
    Ok(st
        .entries
        .into_iter()
        .map(|e| {
            let (add, del) = stats.get(&e.path).copied().unwrap_or_else(|| {
                if e.status == 'A' {
                    (count_lines(&Path::new(root).join(&e.path)), 0)
                } else {
                    (0, 0)
                }
            });
            FileChange {
                path: e.path,
                status: e.status.to_string(),
                add,
                del,
                agent_id: None,
                in_worktree: false,
            }
        })
        .collect())
}

/// `paths` cut into runs that fit one command line. Windows refuses one of more than about 32,000
/// characters, which a few hundred listed files (new ones included) can reach.
fn command_line_chunks(paths: &[String]) -> Vec<&[String]> {
    const BUDGET: usize = 16_000;
    let mut chunks = Vec::new();
    let (mut start, mut len) = (0, 0);
    for (i, path) in paths.iter().enumerate() {
        if len > 0 && len + path.len() + 1 > BUDGET {
            chunks.push(&paths[start..i]);
            (start, len) = (i, 0);
        }
        len += path.len() + 1;
    }
    if start < paths.len() {
        chunks.push(&paths[start..]);
    }
    chunks
}

/// The most diff text one file sends to the interface; past it the file is only flagged.
const MAX_FILE_DIFF: usize = 4_000_000;

/// The line that stands, in a file's header, for the diff that was not sent. The interface shows
/// « Diff trop volumineux pour être affiché. » for a file carrying it (`TOO_LARGE` in `diff.ts`).
const TOO_LARGE: &str = "Diff too large";

/// `diff`, with the file diffs over `limit` bytes cut down to their header and the `TOO_LARGE` line.
fn cap_file_diffs(diff: &str, limit: usize) -> Cow<'_, str> {
    cap_diff(diff, limit, usize::MAX)
}

/// `cap_file_diffs`, and the files that would bring the whole past `budget` bytes flagged the same:
/// a comparison of two distant branches can span thousands of files.
fn cap_diff(diff: &str, limit: usize, budget: usize) -> Cow<'_, str> {
    // No file can pass the cap, nor the whole the budget, when the whole text does not.
    if diff.len() <= limit.min(budget) {
        return Cow::Borrowed(diff);
    }
    // A hunk line starts with its sign (+, -, space or \), so a line starting "diff --git " is
    // always the header of a file.
    let mut starts: Vec<usize> = diff
        .match_indices("\ndiff --git ")
        .map(|(at, _)| at + 1)
        .collect();
    if diff.starts_with("diff --git ") {
        starts.insert(0, 0);
    }
    let mut out = String::new();
    let mut from = 0;
    for (i, &start) in starts.iter().enumerate() {
        out.push_str(&diff[from..start]);
        let end = starts.get(i + 1).copied().unwrap_or(diff.len());
        let file = &diff[start..end];
        if file.len() > limit || out.len() + file.len() > budget {
            out.push_str(&flagged(file));
        } else {
            out.push_str(file);
        }
        from = end;
    }
    out.push_str(&diff[from..]);
    Cow::Owned(out)
}

/// What stands for the diff of a file that is too large: its header (name, mode, rename,
/// `---`/`+++` lines: what comes before the first hunk) and the `TOO_LARGE` line. The header lets
/// the interface tell a new file from a deleted one, and `diff` that the file is covered.
fn flagged(file_diff: &str) -> String {
    // Git writes about ten header lines at most; the bound keeps a diff without any hunk from
    // being sent whole.
    let mut header: String = file_diff
        .split_inclusive('\n')
        .take(12)
        .take_while(|line| !line.starts_with("@@"))
        .collect();
    header.push_str(TOO_LARGE);
    header.push('\n');
    header
}

/// Past this much diff text in one `diff` call, the untracked files left are listed without being read.
const DIFF_BUDGET: usize = 4_000_000;

/// How many of those are listed: a tree of 50,000 new files must not become 50,000 entries.
const UNREAD_LISTED: usize = 500;

/// A name as git writes it in a diff header: verbatim, or in C-style quotes when it holds a
/// quote, a backslash or a control character. A name with a line break would otherwise write
/// header lines of its own (`Diff too large`, `Binary files`…).
fn header_name(name: &str) -> String {
    let plain = |c: char| c >= ' ' && c != '"' && c != '\\' && c != '\x7f';
    if name.chars().all(plain) {
        return name.to_string();
    }
    let mut quoted = String::from('"');
    for c in name.chars() {
        match c {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\x07' => quoted.push_str("\\a"),
            '\x08' => quoted.push_str("\\b"),
            '\t' => quoted.push_str("\\t"),
            '\n' => quoted.push_str("\\n"),
            '\x0b' => quoted.push_str("\\v"),
            '\x0c' => quoted.push_str("\\f"),
            '\r' => quoted.push_str("\\r"),
            c if plain(c) => quoted.push(c),
            c => quoted.push_str(&format!("\\{:03o}", c as u32)),
        }
    }
    quoted.push('"');
    quoted
}

/// The `a/` and `b/` names of `path` in the header git would write for it.
fn header_names(path: &str) -> (String, String) {
    (
        header_name(&format!("a/{path}")),
        header_name(&format!("b/{path}")),
    )
}

/// The header git would write for the new file `path`.
fn new_file_header(path: &str) -> String {
    let (a, b) = header_names(path);
    format!("diff --git {a} {b}\nnew file mode 100644\n--- /dev/null\n+++ {b}\n")
}

/// Unified diff of the given paths (all dirty files when empty), untracked files included.
pub async fn diff(root: &str, paths: &[String]) -> Result<String> {
    let st = status(root).await?;
    let listed: HashSet<&str> = paths.iter().map(String::as_str).collect();
    let wanted = |p: &str| paths.is_empty() || listed.contains(p);
    let untracked: Vec<String> = st
        .entries
        .iter()
        .filter(|e| e.status == 'A' && wanted(&e.path))
        .map(|e| e.path.clone())
        .collect();
    // Without a path git reads every dirty file by itself; a long list is read a run at a time.
    let runs = if paths.is_empty() {
        vec![paths]
    } else {
        command_line_chunks(paths)
    };
    let mut out = String::new();
    for run_paths in runs {
        // File names, not patterns: `[.]env` must not also bring `.env`'s changes.
        let mut args: Vec<&str> = vec![
            "--literal-pathspecs",
            "diff",
            "HEAD",
            "--no-color",
            "--no-ext-diff",
            "--",
        ];
        args.extend(run_paths.iter().map(String::as_str));
        if let Ok(o) = run(root, &args).await {
            out.push_str(&cap_file_diffs(&String::from_utf8_lossy(&o), MAX_FILE_DIFF));
        }
    }
    // The new files git's own diff already shows (those in the index). Git ends the `+++` line of
    // a name holding a space with a tab, a flagged file's header included.
    let covered: HashSet<String> = out
        .lines()
        .filter(|l| l.starts_with("+++ "))
        .map(|l| l.trim_end_matches('\t').to_string())
        .collect();
    let mut unread = 0;
    for path in untracked {
        let (a, b) = header_names(&path);
        if covered.contains(&format!("+++ {b}")) {
            continue;
        }
        // Once the budget is spent the files left are listed, flagged, without being read; past
        // `UNREAD_LISTED` of them they are only counted, in a last entry.
        if out.len() > DIFF_BUDGET {
            if unread < UNREAD_LISTED {
                out.push_str(&flagged(&new_file_header(&path)));
            }
            unread += 1;
            continue;
        }
        let full = Path::new(root).join(&path);
        // A file this large is not even read: it would be flagged all the same.
        let Ok(size) = std::fs::metadata(&full).map(|m| m.len()) else {
            continue;
        };
        if size > MAX_FILE_DIFF as u64 {
            out.push_str(&flagged(&new_file_header(&path)));
            continue;
        }
        let Ok(bytes) = std::fs::read(&full) else {
            continue;
        };
        if bytes.contains(&0) {
            out.push_str(&format!(
                "diff --git {a} {b}\nnew file\nBinary files /dev/null and {b} differ\n"
            ));
            continue;
        }
        let content = String::from_utf8_lossy(&bytes);
        let lines: Vec<&str> = content.lines().collect();
        let mut file = new_file_header(&path);
        file.push_str(&format!("@@ -0,0 +1,{} @@\n", lines.len()));
        for l in lines {
            file.push('+');
            file.push_str(l);
            file.push('\n');
        }
        // The signs and line ends added to its text can take it past the cap.
        out.push_str(&if file.len() > MAX_FILE_DIFF {
            flagged(&file)
        } else {
            file
        });
    }
    if unread > UNREAD_LISTED {
        let left = unread - UNREAD_LISTED;
        let entry = more_files(crate::i18n::ui(), left);
        out.push_str(&flagged(&new_file_header(&entry)));
    }
    Ok(out)
}

/// The last entry of a diff that lists `left` more files without them: in the interface's language,
/// the diff is shown (and the commit's message Claude proposes reads it too, past 4 MB only).
fn more_files(lang: crate::i18n::Lang, left: usize) -> String {
    tr_n_in!(
        lang,
        left,
        "… et {left} autre fichier",
        "… et {left} autres fichiers",
        "… and {left} more file",
        "… and {left} more files"
    )
}

/// Puts one dirty file of `root` back as in HEAD: a tracked file loses its changes (staged ones
/// included), a renamed one gets its name back, a new file is deleted. Only a path listed by
/// `git status` is accepted.
pub async fn discard(root: &str, path: &str) -> Result<()> {
    let entries: Vec<Entry> = status(root)
        .await?
        .entries
        .into_iter()
        .filter(|e| e.path == path)
        .collect();
    if entries.is_empty() {
        bail!(tr!(
            "« {path} » n'a pas de modification à annuler",
            "“{path}” has no changes to discard"
        ));
    }
    // The path is a file name, not a pattern (`a[b].txt` must not also match `ab.txt`).
    const LITERAL: &str = "--literal-pathspecs";
    const RESTORE: [&str; 4] = [LITERAL, "restore", "--source=HEAD", "--staged"];
    if let Some(orig) = entries.iter().find_map(|e| e.orig.as_deref()) {
        run(root, &[&RESTORE[..], &["--worktree", "--", orig]].concat()).await?;
    }
    let in_head = run(root, &["cat-file", "-e", &format!("HEAD:{path}")])
        .await
        .is_ok();
    if in_head {
        // Only deleted from the index (`git rm --cached`): what is on disk stays.
        let kept = entries.iter().any(|e| e.status == 'D') && Path::new(root).join(path).exists();
        let worktree: &[&str] = if kept { &[] } else { &["--worktree"] };
        run(root, &[&RESTORE[..], worktree, &["--", path]].concat()).await?;
    } else {
        let args = [
            LITERAL,
            "rm",
            "--cached",
            "--force",
            "--quiet",
            "--ignore-unmatch",
        ];
        run(root, &[&args[..], &["--", path]].concat()).await?;
        run(root, &[LITERAL, "clean", "--force", "--quiet", "--", path]).await?;
    }
    Ok(())
}

/// Tracked + untracked (non-ignored) files, for @-mention completion.
const LOG_FORMAT: &str = "--format=%H%x1f%P%x1f%an%x1f%at%x1f%D%x1f%s%x1e";

/// Parses `git log` output in the `LOG_FORMAT` layout.
pub fn parse_log(out: &[u8]) -> Vec<Commit> {
    String::from_utf8_lossy(out)
        .split('\x1e')
        .filter_map(|rec| {
            let f: Vec<&str> = rec.trim_start_matches(['\n', '\r']).split('\x1f').collect();
            let [hash, parents, author, time, refs, subject] = f[..] else {
                return None;
            };
            let refs = refs
                .split(", ")
                .filter(|r| !r.is_empty())
                .flat_map(|r| match r.strip_prefix("HEAD -> ") {
                    Some(branch) => vec!["HEAD".to_string(), branch.to_string()],
                    None => vec![r.to_string()],
                })
                .collect();
            Some(Commit {
                hash: hash.to_string(),
                parents: parents.split_whitespace().map(str::to_string).collect(),
                author: author.to_string(),
                time: time.parse().unwrap_or(0),
                refs,
                subject: subject.to_string(),
            })
        })
        .collect()
}

/// The latest `limit` commits of every branch, remote branch and tag (children before parents).
pub async fn log(repo: &str, limit: usize) -> Result<Vec<Commit>> {
    let n = format!("-n{limit}");
    let out = run(
        repo,
        &[
            "log",
            "--date-order",
            "--decorate=short",
            LOG_FORMAT,
            &n,
            "--branches",
            "--remotes",
            "--tags",
            "HEAD",
        ],
    )
    .await?;
    Ok(parse_log(&out))
}

/// The patch of one commit (against its first parent for a merge).
pub async fn show(repo: &str, hash: &str) -> Result<String> {
    if hash.len() < 4 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!(tr!("commit invalide : {hash}", "invalid commit: {hash}"));
    }
    let out = run(
        repo,
        &[
            "show",
            "--format=",
            "--patch",
            "-M",
            "-m",
            "--first-parent",
            hash,
        ],
    )
    .await?;
    Ok(cap_file_diffs(&String::from_utf8_lossy(&out), MAX_FILE_DIFF).into_owned())
}

pub async fn list_files(cwd: &str) -> Result<Vec<String>> {
    let out = run(
        cwd,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
    )
    .await?;
    let mut files: Vec<String> = String::from_utf8_lossy(&out)
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    files.sort();
    files.dedup();
    Ok(files)
}

/// The folders a folder that is not a repository is walked without: what builds and installs
/// leave there (the search of such a folder leaves them out too).
pub const WALK_SKIPPED: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".venv",
    "__pycache__",
    ".next",
];

/// Fallback file listing for folders that are not git repositories.
pub fn walk_files(root: &str, limit: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![std::path::PathBuf::from(root)];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() {
                if !WALK_SKIPPED.contains(&name.as_str()) {
                    stack.push(e.path());
                }
            } else if let Ok(rel) = e.path().strip_prefix(root) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
                if out.len() >= limit {
                    return out;
                }
            }
        }
    }
    out
}

pub async fn ensure_excluded(repo: &str, pattern: &str) -> Result<()> {
    let common = text(repo, &["rev-parse", "--git-common-dir"]).await?;
    let common = if Path::new(&common).is_absolute() {
        common
    } else {
        Path::new(repo).join(common).to_string_lossy().to_string()
    };
    let file = Path::new(&common).join("info").join("exclude");
    let current = std::fs::read_to_string(&file).unwrap_or_default();
    if !current.lines().any(|l| l.trim() == pattern) {
        std::fs::create_dir_all(file.parent().unwrap_or(Path::new(".")))?;
        let sep = if current.is_empty() || current.ends_with('\n') {
            ""
        } else {
            "\n"
        };
        std::fs::write(&file, format!("{current}{sep}{pattern}\n"))?;
    }
    Ok(())
}

/// True when `repo` has a local branch of exactly that name: `show-ref --verify` takes a ref and
/// nothing else, where `rev-parse` would read `main~1` or `main@{1}` as `refs/heads/main~1` and say
/// yes to a revision.
pub async fn branch_exists(repo: &str, branch: &str) -> bool {
    ref_exists(repo, &format!("refs/heads/{branch}")).await
}

/// The local branches, the checked-out one first.
pub async fn branches(repo: &str) -> Result<Vec<String>> {
    let out = text(
        repo,
        &["for-each-ref", "--format=%(refname:short)", "refs/heads"],
    )
    .await?;
    let current = current_branch(repo).await;
    let mut list: Vec<String> = out
        .lines()
        .map(str::to_string)
        .filter(|b| !b.is_empty())
        .collect();
    list.sort_by_key(|b| *b != current);
    Ok(list)
}

/// Creates `<repo>/.claude/worktrees/<name>` on a new branch. Returns (path, branch, base branch).
pub async fn worktree_add(repo: &str, name: &str) -> Result<(String, String, String)> {
    let base = current_branch(repo).await;
    if base.is_empty() || base == "HEAD" {
        bail!(tr!(
            "le dépôt n'a pas encore de commit : impossible de créer un worktree",
            "the repository has no commit yet: a worktree can’t be created"
        ));
    }
    ensure_excluded(repo, ".claude/worktrees/").await?;
    let mut branch = format!("{}{name}", crate::paths::BRANCH_PREFIX);
    let mut n = 2;
    while branch_exists(repo, &branch).await {
        branch = format!("{}{name}-{n}", crate::paths::BRANCH_PREFIX);
        n += 1;
    }
    let dir_name = branch
        .trim_start_matches(crate::paths::BRANCH_PREFIX)
        .to_string();
    let path = Path::new(repo)
        .join(".claude")
        .join("worktrees")
        .join(&dir_name);
    let path_s = path.to_string_lossy().to_string();
    run(repo, &["worktree", "add", "-b", &branch, &path_s, "HEAD"]).await?;
    Ok((path_s, branch, base))
}

/// Creates `<repo>/.claude/worktrees/<last part of branch>` on a new branch `branch` (`branch-2`…
/// when it, or its folder, is taken) starting from `base`. Returns (path, branch).
pub async fn worktree_add_on(repo: &str, branch: &str, base: &str) -> Result<(String, String)> {
    // A branch name never starts with `-`: this is an option (`--lock`…) that git would obey.
    if base.starts_with('-') {
        bail!(tr!(
            "la branche de départ « {base} » n'est pas un nom de branche valide",
            "the starting branch “{base}” isn’t a valid branch name"
        ));
    }
    ensure_excluded(repo, ".claude/worktrees/").await?;
    let dir_of = |b: &str| {
        Path::new(repo)
            .join(".claude")
            .join("worktrees")
            .join(b.rsplit('/').next().unwrap_or(b))
    };
    let mut name = branch.to_string();
    let mut n = 2;
    while branch_exists(repo, &name).await || dir_of(&name).exists() {
        name = format!("{branch}-{n}");
        n += 1;
    }
    let path = dir_of(&name).to_string_lossy().to_string();
    // `--`: whatever `base` is, git reads it as the commit-ish to start from.
    run(repo, &["worktree", "add", "-b", &name, &path, "--", base]).await?;
    Ok((path, name))
}

/// Removes a worktree and its branch, tolerating a worktree folder that is already gone.
pub async fn worktree_remove(repo: &str, path: &str, branch: &str) -> Result<()> {
    worktree_remove_dir(repo, path).await?;
    if branch_exists(repo, branch).await {
        run(repo, &["branch", "-D", branch]).await?;
    }
    Ok(())
}

/// Removes a worktree, its branch kept, tolerating a worktree folder that is already gone.
pub async fn worktree_remove_dir(repo: &str, path: &str) -> Result<()> {
    if run(repo, &["worktree", "remove", "--force", path])
        .await
        .is_err()
        && Path::new(path).exists()
    {
        std::fs::remove_dir_all(path)
            .map_err(|e| anyhow::anyhow!(tr!("{path} : {e}", "{path}: {e}")))?;
    }
    let _ = run(repo, &["worktree", "prune"]).await;
    Ok(())
}

/// Checks `branch` out again at `path`, a worktree folder that went (after `worktree_remove_dir`).
pub async fn worktree_restore(repo: &str, path: &str, branch: &str) -> Result<()> {
    if branch.starts_with('-') {
        bail!(invalid_branch(branch));
    }
    ensure_excluded(repo, ".claude/worktrees/").await?;
    let _ = run(repo, &["worktree", "prune"]).await;
    run(repo, &["worktree", "add", path, branch]).await?;
    Ok(())
}

fn invalid_branch(branch: &str) -> anyhow::Error {
    anyhow::anyhow!(tr!(
        "« {branch} » n'est pas un nom de branche valide",
        "“{branch}” isn’t a valid branch name"
    ))
}

/// Checks the existing local branch `branch` out in `repo`; git's refusal (untracked files in the
/// way, the branch held by another worktree…) comes back as it is.
pub async fn switch(repo: &str, branch: &str) -> Result<()> {
    if branch.starts_with('-') {
        bail!(invalid_branch(branch));
    }
    // `--no-guess`: a branch only the remote has is not made, the base is a local one.
    run(repo, &["switch", "--no-guess", branch])
        .await
        .map(|_| ())
}

/// Commits of `branch` that `base` does not have; an error when git cannot count them.
pub async fn ahead_of(repo: &str, base: &str, branch: &str) -> Result<u32> {
    let n = text(repo, &["rev-list", "--count", &format!("{base}..{branch}")]).await?;
    n.trim().parse().map_err(|_| {
        anyhow::anyhow!(tr!(
            "git rev-list a répondu « {n} »",
            "git rev-list answered “{n}”",
            n = n.trim()
        ))
    })
}

/// The folders of the repository's worktrees, the main one included.
pub async fn worktree_paths(repo: &str) -> Result<Vec<String>> {
    let out = text(repo, &["worktree", "list", "--porcelain"]).await?;
    Ok(out
        .lines()
        .filter_map(|l| l.strip_prefix("worktree "))
        .map(str::to_string)
        .collect())
}

/// Where `branch` is checked out: the folder of the worktree that has it (the main one included).
pub async fn checkout_of(repo: &str, branch: &str) -> Result<Option<String>> {
    let out = text(repo, &["worktree", "list", "--porcelain"]).await?;
    let want = format!("branch refs/heads/{branch}");
    let mut path = None;
    for line in out.lines() {
        if let Some(p) = line.strip_prefix("worktree ") {
            path = Some(p.to_string());
        } else if line == want {
            return Ok(path);
        }
    }
    Ok(None)
}

// ---------- branches ----------

/// A branch as the branch picker lists it: a local one, or a remote one (`origin/feat`).
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchInfo {
    /// A local branch's name (`feat/x`), a remote one's with its remote (`origin/feat/x`).
    pub name: String,
    pub remote: bool,
    /// Checked out in the repository's own folder.
    pub current: bool,
    /// The branch a local one tracks (`origin/feat/x`).
    pub upstream: Option<String>,
    /// That upstream no longer exists (deleted on the remote, as of the last fetch).
    pub upstream_gone: bool,
    /// Commits not in the upstream / in the upstream only, as of the last fetch.
    pub ahead: u32,
    pub behind: u32,
    /// The folder of the worktree that has it checked out, the repository's own included.
    pub worktree: Option<String>,
    /// A remote one: the local branch that tracks it.
    pub tracked_by: Option<String>,
    /// The agent whose worktree has it checked out (its id): told by the core, never by git.
    pub agent: Option<String>,
    /// When its latest commit was made (committer date), ms since epoch.
    pub last_commit_at: i64,
}

/// The fields of a local branch, NUL-separated: its ref, its upstream's, how it stands against that
/// upstream (`ahead 1, behind 2`, `gone`), its tip's committer date, `*` when checked out in the
/// repository's folder, and the folder of the worktree that has it checked out.
const LOCAL_BRANCH: &str = "--format=%(refname)%00%(upstream)%00%(upstream:track,nobracket)\
                            %00%(committerdate:unix)%00%(HEAD)%00%(worktreepath)";

/// The fields of a remote branch: its ref, the ref it stands for (a remote's HEAD), its tip's date.
const REMOTE_BRANCH: &str = "--format=%(refname)%00%(symref)%00%(committerdate:unix)";

/// A branch's short name: `feat` for `refs/heads/feat`, `origin/feat` for `refs/remotes/origin/feat`.
fn short_ref(refname: &str) -> &str {
    refname
        .strip_prefix("refs/heads/")
        .or_else(|| refname.strip_prefix("refs/remotes/"))
        .unwrap_or(refname)
}

/// (ahead, behind, gone) from `%(upstream:track,nobracket)`: `ahead 1, behind 2`, `gone` or nothing.
fn parse_track(track: &str) -> (u32, u32, bool) {
    let (mut ahead, mut behind) = (0, 0);
    for part in track.split(", ") {
        if let Some(n) = part.strip_prefix("ahead ") {
            ahead = n.parse().unwrap_or(0);
        } else if let Some(n) = part.strip_prefix("behind ") {
            behind = n.parse().unwrap_or(0);
        }
    }
    (ahead, behind, track == "gone")
}

/// The repository's branches: the local ones (the one checked out in `repo` first), then the
/// remote ones, those a local branch tracks included, a remote's HEAD left out.
pub async fn branch_list(repo: &str) -> Result<Vec<BranchInfo>> {
    let (locals, remotes) = tokio::join!(
        text(repo, &["for-each-ref", LOCAL_BRANCH, "refs/heads"]),
        text(repo, &["for-each-ref", REMOTE_BRANCH, "refs/remotes"]),
    );
    let (locals, remotes) = (locals?, remotes?);
    let mut list = Vec::new();
    // The local branch that tracks each remote one.
    let mut tracked_by: HashMap<&str, &str> = HashMap::new();
    for line in locals.lines() {
        let fields: Vec<&str> = line.split('\0').collect();
        let [refname, upstream, track, date, head, worktree] = fields[..] else {
            continue;
        };
        let Some(name) = refname.strip_prefix("refs/heads/") else {
            continue;
        };
        if !upstream.is_empty() {
            tracked_by.insert(upstream, name);
        }
        let (ahead, behind, upstream_gone) = parse_track(track);
        list.push(BranchInfo {
            name: name.to_string(),
            current: head == "*",
            upstream: (!upstream.is_empty()).then(|| short_ref(upstream).to_string()),
            upstream_gone,
            ahead,
            behind,
            worktree: (!worktree.is_empty()).then(|| worktree.to_string()),
            last_commit_at: date.parse::<i64>().unwrap_or(0) * 1000,
            ..BranchInfo::default()
        });
    }
    list.sort_by_key(|b| !b.current);
    for line in remotes.lines() {
        let fields: Vec<&str> = line.split('\0').collect();
        let [refname, symref, date] = fields[..] else {
            continue;
        };
        let Some(name) = refname.strip_prefix("refs/remotes/") else {
            continue;
        };
        // `origin/HEAD` stands for the remote's default branch, listed for itself.
        if !symref.is_empty() || name.ends_with("/HEAD") {
            continue;
        }
        list.push(BranchInfo {
            name: name.to_string(),
            remote: true,
            tracked_by: tracked_by.get(refname).map(|b| b.to_string()),
            last_commit_at: date.parse::<i64>().unwrap_or(0) * 1000,
            ..BranchInfo::default()
        });
    }
    Ok(list)
}

/// Refused: `name` is no name git takes for a branch, with what makes one.
fn not_a_branch_name(name: &str) -> anyhow::Error {
    anyhow::anyhow!(tr!(
        "« {name} » n’est pas un nom de branche valide : ni espace, ni « .. », « ~ », « ^ », « : », « ? », « * », « [ » ou « \\ », ni « - » au début, ni « / », « . » ou « .lock » à la fin.",
        "“{name}” isn’t a valid branch name: no spaces, “..”, “~”, “^”, “:”, “?”, “*”, “[” or “\\”, no “-” at the start, no “/”, “.” or “.lock” at the end."
    ))
}

/// Refused: a branch of that name exists already.
pub fn branch_taken(name: &str) -> anyhow::Error {
    anyhow::anyhow!(tr!(
        "La branche « {name} » existe déjà.",
        "The branch “{name}” already exists."
    ))
}

fn no_branch(name: &str) -> anyhow::Error {
    anyhow::anyhow!(tr!(
        "La branche « {name} » est introuvable.",
        "The branch “{name}” wasn’t found."
    ))
}

/// Checks `name` for a new branch the way git does (`git check-ref-format --branch`), and refuses
/// what git would read as something else: an option (`-x`), HEAD (`@`), an earlier branch (`@{-1}`).
pub async fn check_ref_format(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        bail!(tr!("Donne un nom à la branche.", "Give the branch a name."));
    }
    let other_meaning = name.starts_with('-') || name == "@" || name.contains("@{");
    // Outside any repository: none has a say on a plain name (`@{-1}`, which would read one, is
    // refused above).
    let anywhere = std::env::temp_dir().to_string_lossy().to_string();
    if other_meaning
        || run(&anywhere, &["check-ref-format", "--branch", name])
            .await
            .is_err()
    {
        return Err(not_a_branch_name(name));
    }
    Ok(())
}

/// Checks `name` for a new branch of `repo`: a name git takes, that no local branch has, and that
/// does not start like a branch of a remote (`origin/feat`): the local one would hide it, and
/// every `origin/feat` would then be read as the local branch.
pub async fn check_new_branch(repo: &str, name: &str) -> Result<()> {
    check_ref_format(name).await?;
    if branch_exists(repo, name).await {
        return Err(branch_taken(name));
    }
    if let Some(remote) = remotes(repo)
        .await
        .into_iter()
        .find(|r| name.starts_with(&format!("{r}/")))
    {
        bail!(tr!(
            "« {name} » commence comme une branche du dépôt distant « {remote} » : une branche locale de ce nom la cacherait. Choisis un autre nom.",
            "“{name}” starts like a branch of the remote “{remote}”: a local branch of that name would hide it. Pick another name."
        ));
    }
    Ok(())
}

/// The commit `rev` names (a branch, a remote branch, a tag, a hash); refused when it names none,
/// or looks like an option.
pub async fn commit_of(repo: &str, rev: &str) -> Result<String> {
    if !rev.is_empty() && !rev.starts_with('-') {
        let peeled = format!("{rev}^{{commit}}");
        if let Ok(hash) = text(repo, &["rev-parse", "--verify", "--quiet", &peeled]).await {
            if !hash.is_empty() {
                return Ok(hash);
            }
        }
    }
    bail!(tr!(
        "« {rev} » ne désigne aucun commit.",
        "“{rev}” names no commit."
    ))
}

/// Creates the branch `name` at `start` (a branch, a remote branch, a commit; HEAD when empty),
/// tracking nothing: one made from `origin/main` must not push to main. Nothing is checked out.
pub async fn branch_create(repo: &str, name: &str, start: &str) -> Result<()> {
    check_new_branch(repo, name).await?;
    let start = match start.trim() {
        "" => "HEAD",
        s => s,
    };
    let commit = commit_of(repo, start).await?;
    run(repo, &["branch", "--no-track", "--", name, &commit])
        .await
        .map(|_| ())
}

/// True when `refname` (a full name: `refs/heads/feat`, `refs/remotes/origin/feat`) is a ref
/// of `repo`, and not a revision that starts with one.
pub async fn ref_exists(repo: &str, refname: &str) -> bool {
    run(repo, &["show-ref", "--verify", "--quiet", refname])
        .await
        .is_ok()
}

/// The remote of a remote branch's name and the branch's name there (`origin/feat/x`: `origin`,
/// `feat/x`), the longest remote name matching: one may hold a `/`.
pub async fn split_remote(repo: &str, name: &str) -> Option<(String, String)> {
    let remotes = remotes(repo).await;
    let remote = remotes
        .iter()
        .filter(|r| name.starts_with(&format!("{r}/")))
        .max_by_key(|r| r.len())?;
    Some((remote.clone(), name[remote.len() + 1..].to_string()))
}

/// The branch a local branch tracks on a remote, while the remote has it (as of the last fetch).
#[derive(Debug, Clone, PartialEq)]
pub struct Upstream {
    /// Its tracking branch here: `refs/remotes/origin/feat`.
    pub tracking: String,
    pub remote: String,
    /// Its name on the remote: `feat`.
    pub branch: String,
}

pub async fn upstream_of(repo: &str, name: &str) -> Option<Upstream> {
    let local = format!("refs/heads/{name}");
    let out = text(
        repo,
        &[
            "for-each-ref",
            "--format=%(upstream)%00%(upstream:remotename)%00%(upstream:remoteref)",
            &local,
        ],
    )
    .await
    .ok()?;
    let [tracking, remote, remote_ref] = out.split('\0').collect::<Vec<_>>()[..] else {
        return None;
    };
    // `.`: it tracks a local branch.
    let branch = remote_ref.strip_prefix("refs/heads/")?;
    if remote.is_empty() || remote == "." || !ref_exists(repo, tracking).await {
        return None;
    }
    Some(Upstream {
        tracking: tracking.to_string(),
        remote: remote.to_string(),
        branch: branch.to_string(),
    })
}

/// The local branch `name` stands for: itself, or for a remote branch (`origin/feat`) the local one
/// that tracks it, whatever its name; None when there is none yet (`switch_to` makes it). Refused
/// when it is neither, or when the local branch it would make exists and tracks something else.
pub async fn local_of(repo: &str, name: &str) -> Result<Option<String>> {
    if name.is_empty() || name.starts_with('-') {
        bail!(invalid_branch(name));
    }
    if branch_exists(repo, name).await {
        return Ok(Some(name.to_string()));
    }
    let tracking = format!("refs/remotes/{name}");
    if !ref_exists(repo, &tracking).await {
        return Err(no_branch(name));
    }
    let locals = text(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname)%00%(upstream)",
            "refs/heads",
        ],
    )
    .await?;
    let tracked = locals.lines().find_map(|l| {
        let (local, upstream) = l.split_once('\0')?;
        (upstream == tracking).then(|| short_ref(local).to_string())
    });
    if tracked.is_some() {
        return Ok(tracked);
    }
    // `switch --track` names it after the remote's branch.
    let Some((_, short)) = split_remote(repo, name).await else {
        return Err(no_branch(name));
    };
    check_ref_format(&short).await?;
    if branch_exists(repo, &short).await {
        bail!(tr!(
            "Une branche locale « {short} » existe déjà et ne suit pas {name} : passe sur elle, ou renomme-la d’abord.",
            "A local branch “{short}” already exists and doesn’t track {name}: switch to it, or rename it first."
        ));
    }
    Ok(None)
}

/// Checks `name` out in `repo`: a local branch, or a remote one through the local branch that
/// tracks it, made (`switch --track`) when there is none. Returns the local branch.
pub async fn switch_to(repo: &str, name: &str) -> Result<String> {
    match local_of(repo, name).await? {
        Some(local) => {
            switch(repo, &local).await?;
            Ok(local)
        }
        None => {
            run(repo, &["switch", "--track", name]).await?;
            Ok(head_branch(repo).await)
        }
    }
}

const LATEST_STASH: [&str; 4] = ["rev-parse", "--verify", "--quiet", "refs/stash"];

/// Puts the uncommitted changes of tracked files aside (staged ones included) in a stash named
/// `message`; untracked files stay where they are. Returns the stash's commit; an error when there
/// was nothing to put aside.
pub async fn stash_push(repo: &str, message: &str) -> Result<String> {
    let before = text(repo, &LATEST_STASH).await.ok();
    run(repo, &["stash", "push", "--quiet", "-m", message]).await?;
    match text(repo, &LATEST_STASH).await {
        Ok(stash) if Some(&stash) != before.as_ref() => Ok(stash),
        _ => bail!(tr!(
            "Rien à mettre de côté : aucun changement non commité.",
            "Nothing to stash: no uncommitted change."
        )),
    }
}

/// Puts the latest stash back (what was staged staged again) and drops it.
pub async fn stash_pop(repo: &str) -> Result<()> {
    run(repo, &["stash", "pop", "--index", "--quiet"])
        .await
        .map(|_| ())
}

/// Deletes the local branch `name`: once merged into HEAD or its upstream (git's own check,
/// `branch -d`), whatever it holds with `force`.
pub async fn branch_delete(repo: &str, name: &str, force: bool) -> Result<()> {
    if name.is_empty() || name.starts_with('-') {
        bail!(invalid_branch(name));
    }
    let flag = if force { "-D" } else { "-d" };
    run(repo, &["branch", flag, "--", name]).await.map(|_| ())
}

/// A remote branch to delete is no longer where the last fetch saw it.
fn remote_moved(lang: crate::i18n::Lang) -> String {
    tr_in!(
        lang,
        "La branche distante a bougé : récupère d’abord (fetch).",
        "The remote branch has moved: fetch first."
    )
}

/// Deletes the branch `name` of `remote` (`git push <remote> :refs/heads/<name>`), and its tracking
/// branch here, but only while the remote still has it where the last fetch saw it: a commit
/// someone pushed since is not deleted unseen (`--force-with-lease`). A branch the remote no longer
/// has is no error. The user's: credentials may be asked.
pub async fn branch_delete_remote(repo: &str, remote: &str, name: &str) -> Result<()> {
    if name.is_empty() || name.starts_with('-') {
        bail!(invalid_branch(name));
    }
    if remote.starts_with('-') || !remotes(repo).await.iter().any(|r| r == remote) {
        bail!(tr!(
            "Le dépôt distant « {remote} » est introuvable.",
            "The remote “{remote}” wasn’t found."
        ));
    }
    // What the lease holds the remote to; without a fetch of it there is nothing to compare.
    let tracking = format!("refs/remotes/{remote}/{name}");
    let seen = match ref_exists(repo, &tracking).await {
        true => text(repo, &["rev-parse", "--verify", &tracking]).await.ok(),
        false => None,
    };
    let Some(seen) = seen else {
        bail!(tr!(
            "La branche distante {remote}/{name} n’a pas été récupérée ici : récupère d’abord (fetch).",
            "The remote branch {remote}/{name} wasn’t fetched here: fetch first."
        ));
    };
    let full = format!("refs/heads/{name}");
    let lease = format!("--force-with-lease={full}:{seen}");
    let refspec = format!(":{full}");
    let pushed = run_net_english(repo, &["push", &lease, remote, &refspec], false).await;
    if let Err(e) = pushed {
        if !e.to_string().contains("stale info") {
            return Err(e);
        }
        // Refused for it not being where it was seen: gone altogether (deleted by someone else,
        // or with the merge of a pull request) is nothing left to delete, anywhere else is a
        // branch that moved.
        let there = run_net(repo, &["ls-remote", remote, &full], false).await?;
        let there = String::from_utf8_lossy(&there);
        if there.lines().any(|l| l.ends_with(&format!("\t{full}"))) {
            bail!(remote_moved(crate::i18n::ui()));
        }
    }
    // A push that deleted it took its tracking branch along; one that found nothing did not.
    let _ = run(repo, &["update-ref", "-d", &tracking, &seen]).await;
    Ok(())
}

/// Why a remote branch stays when the local branch that tracks it is deleted with its remote copy.
#[derive(Debug, Clone, PartialEq)]
pub enum RemoteKept {
    /// The remote's default branch (its HEAD).
    Default,
    /// The upstream of the project's base branch.
    Base,
    /// It is called otherwise there: `feature` tracking `origin/main` has no copy of its own.
    OtherName,
    /// Another local branch tracks it too.
    TrackedBy(String),
}

impl RemoteKept {
    /// Said after the local branch alone went; `shown` is the remote branch (`origin/main`).
    pub fn text(&self, lang: crate::i18n::Lang, shown: &str) -> String {
        match self {
            RemoteKept::Default => tr_in!(
                lang,
                "« {shown} » est la branche par défaut du dépôt distant : seule la branche locale est supprimée.",
                "“{shown}” is the remote’s default branch: only the local branch is deleted."
            ),
            RemoteKept::Base => tr_in!(
                lang,
                "« {shown} » est la branche distante de la base du projet : seule la branche locale est supprimée.",
                "“{shown}” is the remote branch of the project’s base: only the local branch is deleted."
            ),
            RemoteKept::OtherName => tr_in!(
                lang,
                "La branche locale suit « {shown} », qui ne porte pas son nom : seule la branche locale est supprimée.",
                "The local branch tracks “{shown}”, which isn’t named like it: only the local branch is deleted."
            ),
            RemoteKept::TrackedBy(other) => tr_in!(
                lang,
                "La branche « {other} » suit aussi « {shown} » : seule la branche locale est supprimée.",
                "The branch “{other}” also tracks “{shown}”: only the local branch is deleted."
            ),
        }
    }

    /// Said when asked to delete that remote branch itself (the two that may never go).
    pub fn refusal(&self, lang: crate::i18n::Lang, shown: &str) -> String {
        match self {
            RemoteKept::Default => tr_in!(
                lang,
                "« {shown} » est la branche par défaut du dépôt distant : elle n’est pas supprimée d’ici.",
                "“{shown}” is the remote’s default branch: it isn’t deleted from here."
            ),
            _ => tr_in!(
                lang,
                "« {shown} » est la branche distante de la base du projet : elle n’est pas supprimée d’ici.",
                "“{shown}” is the remote branch of the project’s base: it isn’t deleted from here."
            ),
        }
    }
}

/// What goes on the remote with a local branch deleted along with its remote copy.
#[derive(Debug, Clone, PartialEq)]
pub enum RemoteCopy {
    /// Nothing: the branch has no upstream, or the remote no longer has it.
    Absent,
    /// Its own copy: the branch `branch` of `remote`.
    Delete { remote: String, branch: String },
    /// Not its own, left alone.
    Keep { shown: String, why: RemoteKept },
}

impl RemoteCopy {
    /// What to tell the user of a copy that stays.
    pub fn note(&self, lang: crate::i18n::Lang) -> Option<String> {
        match self {
            RemoteCopy::Keep { shown, why } => Some(why.text(lang, shown)),
            _ => None,
        }
    }
}

/// Of the remote branch `tracking` (`refs/remotes/origin/main`) of `remote`, why it may never be
/// deleted: it is the remote's default branch (`refs/remotes/<remote>/HEAD`), or one of `bases`, the
/// project's base and its upstream (full names).
pub async fn remote_guard(
    repo: &str,
    remote: &str,
    tracking: &str,
    bases: &[String],
) -> Option<RemoteKept> {
    let head = format!("refs/remotes/{remote}/HEAD");
    if text(repo, &["symbolic-ref", "-q", &head])
        .await
        .ok()
        .as_deref()
        == Some(tracking)
    {
        return Some(RemoteKept::Default);
    }
    bases
        .iter()
        .any(|b| b == tracking)
        .then_some(RemoteKept::Base)
}

/// The remote copy of the local branch `name` to delete along with it: its upstream, when that is
/// the branch of the same name, no other local branch tracks and `remote_guard` allows: a branch
/// tracking `origin/main` is no copy of it.
pub async fn remote_copy(repo: &str, name: &str, bases: &[String]) -> Result<RemoteCopy> {
    let Some(up) = upstream_of(repo, name).await else {
        return Ok(RemoteCopy::Absent);
    };
    let shown = format!("{}/{}", up.remote, up.branch);
    let keep = |why| RemoteCopy::Keep {
        shown: shown.clone(),
        why,
    };
    if let Some(why) = remote_guard(repo, &up.remote, &up.tracking, bases).await {
        return Ok(keep(why));
    }
    if up.branch != name {
        return Ok(keep(RemoteKept::OtherName));
    }
    let tracks = text(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname)%00%(upstream)",
            "refs/heads",
        ],
    )
    .await?;
    let other = tracks.lines().find_map(|l| {
        let (local, upstream) = l.split_once('\0')?;
        (upstream == up.tracking && short_ref(local) != name).then(|| short_ref(local).to_string())
    });
    Ok(match other {
        Some(other) => keep(RemoteKept::TrackedBy(other)),
        None => RemoteCopy::Delete {
            remote: up.remote,
            branch: up.branch,
        },
    })
}

/// The tree of the commit `rev` names; refused like `commit_of`.
async fn tree_of(repo: &str, rev: &str) -> Result<String> {
    let commit = commit_of(repo, rev).await?;
    text(repo, &["rev-parse", &format!("{commit}^{{tree}}")]).await
}

/// True when merging `refname` into `base`, whose tree is `tree`, would leave that tree as it is:
/// what it changed is in `base` already, squashed or picked. False when git cannot tell: a
/// conflict, unrelated histories, a git older than 2.38 (no `merge-tree --write-tree`).
async fn brings_nothing(repo: &str, base: &str, tree: &str, refname: &str) -> bool {
    text(
        repo,
        &["merge-tree", "--write-tree", "--no-messages", base, refname],
    )
    .await
    .is_ok_and(|out| out.lines().next() == Some(tree))
}

/// True when the work of `refname` (a full ref: `refs/heads/feat`) is in `base`: part of it, or
/// merging it would change nothing (squash-merged).
pub async fn is_merged(repo: &str, refname: &str, base: &str) -> bool {
    if !refname.starts_with("refs/") {
        return false;
    }
    let Ok(tree) = tree_of(repo, base).await else {
        return false;
    };
    run(repo, &["merge-base", "--is-ancestor", refname, base])
        .await
        .is_ok()
        || brings_nothing(repo, base, &tree, refname).await
}

/// The local branches whose work `base` has, `base` aside: those it contains, and those whose merge
/// would bring it nothing (squash-merged, picked). By name.
pub async fn merged_into(repo: &str, base: &str) -> Result<Vec<String>> {
    let tree = tree_of(repo, base).await?;
    let (merged, unmerged) = (format!("--merged={base}"), format!("--no-merged={base}"));
    let in_base = ["for-each-ref", "--format=%(refname)", &merged, "refs/heads"];
    let others = [
        "for-each-ref",
        "--format=%(refname)",
        &unmerged,
        "refs/heads",
    ];
    let (merged, unmerged) = tokio::join!(text(repo, &in_base), text(repo, &others));
    let mut names: Vec<String> = merged?
        .lines()
        .filter_map(|r| r.strip_prefix("refs/heads/"))
        .map(str::to_string)
        .collect();
    for refname in unmerged?.lines() {
        if brings_nothing(repo, base, &tree, refname).await {
            names.extend(refname.strip_prefix("refs/heads/").map(str::to_string));
        }
    }
    names.retain(|n| n != base && format!("refs/heads/{n}") != base);
    names.sort();
    Ok(names)
}

/// How many commits of `refs` (full names: `refs/heads/feat`, `refs/remotes/origin/feat`) no other
/// branch has, local or remote: what deleting them all would lose.
pub async fn unique_commits(repo: &str, refs: &[String]) -> Result<u32> {
    if let Some(r) = refs.iter().find(|r| !r.starts_with("refs/")) {
        bail!(tr!(
            "« {r} » n’est pas une référence de branche",
            "“{r}” isn’t a branch reference"
        ));
    }
    let mut args: Vec<String> = vec!["rev-list".into(), "--count".into()];
    args.extend(refs.iter().cloned());
    args.push("--not".into());
    // `--exclude` takes the names `--branches` and `--remotes` see, and holds for the next one.
    let excluded = |prefix: &str| -> Vec<String> {
        refs.iter()
            .filter_map(|r| r.strip_prefix(prefix))
            .map(|n| format!("--exclude={n}"))
            .collect()
    };
    args.extend(excluded("refs/heads/"));
    args.push("--branches".into());
    args.extend(excluded("refs/remotes/"));
    // A remote's HEAD stands for one of its branches: maybe one that goes.
    args.push("--exclude=*/HEAD".into());
    args.push("--remotes".into());
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let n = text(repo, &args).await?;
    n.parse().map_err(|_| {
        anyhow::anyhow!(tr!(
            "git rev-list a répondu « {n} »",
            "git rev-list answered “{n}”"
        ))
    })
}

/// What changes from `a` to `b` (`git diff a b`: two branches, two commits), each file's diff
/// capped as a commit's is, and the whole kept within `DIFF_BUDGET`.
pub async fn diff_refs(repo: &str, a: &str, b: &str) -> Result<String> {
    let (from, to) = (commit_of(repo, a).await?, commit_of(repo, b).await?);
    let out = run(
        repo,
        &[
            "diff",
            "--no-color",
            "--no-ext-diff",
            "-M",
            &from,
            &to,
            "--",
        ],
    )
    .await?;
    Ok(cap_diff(&String::from_utf8_lossy(&out), MAX_FILE_DIFF, DIFF_BUDGET).into_owned())
}

/// Of `paths`, those that `branch` changed since it left `base` (`base...branch`), whatever
/// `base` did since: what its tree brings.
pub async fn changed_in(
    repo: &str,
    base: &str,
    branch: &str,
    paths: &[String],
) -> Result<Vec<String>> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let range = format!("{base}...{branch}");
    let mut args = vec![
        "--literal-pathspecs",
        "diff",
        "--name-only",
        "--no-renames",
        &range,
        "--",
    ];
    args.extend(paths.iter().map(String::as_str));
    Ok(text(repo, &args)
        .await?
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect())
}

/// Of `paths`, those that a commit of `branch` not in `base` added, changed or removed, merge
/// commits and the side branches they brought included: what merging `branch` would bring into
/// `base`'s history, even when a later commit took it out of the tree again.
pub async fn touched_by(
    repo: &str,
    base: &str,
    branch: &str,
    paths: &[String],
) -> Result<Vec<String>> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let range = format!("{base}..{branch}");
    let mut args = vec![
        "--literal-pathspecs",
        "log",
        "--format=",
        "--name-only",
        "--no-renames",
        // Every commit that touched them: by default, a merge that ends up as one of its parents
        // hides the other side's commits.
        "--full-history",
        // A merge commit's own files, those it differs from all of its parents on: by default a
        // merge shows none, and one that brought a file (`git add -A` while resolving) would go
        // unseen. Against each parent instead, merging `base` would count every file `base`
        // changed as the branch's; what a merged side brought through its own commits is listed
        // with those commits.
        "--diff-merges=combined",
        &range,
        "--",
    ];
    args.extend(paths.iter().map(String::as_str));
    let out = text(repo, &args).await?;
    let mut files: Vec<String> = Vec::new();
    for f in out.lines().filter(|l| !l.is_empty()) {
        if !files.iter().any(|x| x == f) {
            files.push(f.to_string());
        }
    }
    Ok(files)
}

/// Stages every change of `cwd` but the files `keep_out` (copied `.env` files…), even staged
/// already. True when something is staged.
pub async fn stage_all(cwd: &str, keep_out: &[String]) -> Result<bool> {
    run(cwd, &["add", "-A"]).await?;
    for p in keep_out {
        run(
            cwd,
            &[
                "--literal-pathspecs",
                "rm",
                "--cached",
                "-q",
                "--ignore-unmatch",
                "--",
                p,
            ],
        )
        .await?;
    }
    // `diff --quiet` fails when there is a difference.
    Ok(run(cwd, &["diff", "--cached", "--quiet"]).await.is_err())
}

pub async fn commit_staged(cwd: &str, message: &str) -> Result<()> {
    run(cwd, &["commit", "-q", "-m", message]).await.map(|_| ())
}

/// A command given `input` on its standard input (`--stdin`, `--pathspec-from-file=-`): a list of
/// paths of any length, where a command line has a limit (see `command_line_chunks`).
async fn run_input(cwd: &str, args: &[&str], input: &[u8]) -> Result<std::process::Output> {
    use tokio::io::AsyncWriteExt;
    let mut cmd = command(cwd, args);
    cmd.stdin(Stdio::piped());
    let mut child = cmd.spawn()?;
    let mut stdin = child.stdin.take().ok_or_else(|| {
        anyhow::anyhow!(tr!(
            "git {a} : pas d'entrée",
            "git {a}: no input",
            a = args.join(" ")
        ))
    })?;
    // Written while its output is read: a long list could otherwise fill both pipes.
    let write = async move {
        let written = stdin.write_all(input).await;
        drop(stdin);
        written
    };
    let (written, out) = tokio::join!(write, child.wait_with_output());
    let out = out?;
    // A command that stopped reading (it failed) says why better than the broken pipe.
    if out.status.success() {
        written?;
    }
    Ok(out)
}

/// `paths` as `--pathspec-file-nul` and `-z` read them.
fn nul_separated<S: AsRef<str>>(paths: &[S]) -> Vec<u8> {
    let mut out = Vec::new();
    for p in paths {
        out.extend_from_slice(p.as_ref().as_bytes());
        out.push(0);
    }
    out
}

/// Of `paths` (relative to `cwd`), those that git's ignore rules ignore, even once forced into
/// the index (where git no longer applies them).
pub async fn ignored(cwd: &str, paths: &[String]) -> Result<Vec<String>> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    // Each name is tested as it is against the rules (a `*` in it is no wildcard): git refuses
    // `--literal-pathspecs` here, and needs none.
    let args = ["check-ignore", "--no-index", "--stdin", "-z"];
    let out = run_input(cwd, &args, &nul_separated(paths)).await?;
    // 1: none of them is ignored; anything else but 0 is git failing, which must not pass for
    // "none".
    if out.status.code() == Some(1) && out.stderr.is_empty() {
        return Ok(Vec::new());
    }
    Ok(String::from_utf8_lossy(&checked(out, &args)?)
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect())
}

/// Commits the changes of `paths` in `cwd`, and only them: new, changed and deleted files alike,
/// a renamed file's former name with it. What else is staged stays staged, out of the commit; a
/// refused commit (a hook) leaves the index as it was.
pub async fn commit_paths(cwd: &str, paths: &[String], message: &str) -> Result<()> {
    const LITERAL: &str = "--literal-pathspecs";
    const FROM_STDIN: [&str; 2] = ["--pathspec-from-file=-", "--pathspec-file-nul"];
    let entries: HashMap<String, Entry> = status(cwd)
        .await?
        .entries
        .into_iter()
        .map(|e| (e.path.clone(), e))
        .collect();
    let mut all: Vec<&str> = Vec::new();
    let mut new: Vec<&str> = Vec::new();
    for p in paths {
        all.push(p);
        match entries.get(p) {
            Some(e) if e.untracked => new.push(p),
            Some(Entry {
                orig: Some(orig), ..
            }) => all.push(orig),
            _ => {}
        }
    }
    // A new file is unknown to the commit until added. The others are not: `--only` takes them as
    // they are on disk without touching what the index holds for them (a file staged in part
    // stays so until the commit is made).
    if !new.is_empty() {
        let args = [&[LITERAL, "add"][..], &FROM_STDIN].concat();
        checked(run_input(cwd, &args, &nul_separated(&new)).await?, &args)?;
    }
    let args = [
        &[LITERAL, "commit", "-q", "--only", "-m", message][..],
        &FROM_STDIN,
    ]
    .concat();
    let committed = checked(run_input(cwd, &args, &nul_separated(&all)).await?, &args);
    if committed.is_err() && !new.is_empty() {
        // Refused: the new files go back to untracked, as they were (their content stays on disk).
        let args = [
            &[
                LITERAL,
                "rm",
                "--cached",
                "--force",
                "--quiet",
                "--ignore-unmatch",
            ][..],
            &FROM_STDIN,
        ]
        .concat();
        let unstaged = run_input(cwd, &args, &nul_separated(&new)).await;
        if let Err(e) = unstaged.and_then(|out| checked(out, &args)) {
            log::warn!("{cwd}: the new files of a refused commit stay staged: {e:#}");
        }
    }
    committed.map(|_| ())
}

/// What the branch checked out in `cwd` changed since it left `target`, staged changes included.
pub async fn staged_stat(cwd: &str, target: &str) -> String {
    let Ok(base) = text(cwd, &["merge-base", target, "HEAD"]).await else {
        return String::new();
    };
    text(cwd, &["diff", "--cached", "--stat", &base])
        .await
        .unwrap_or_default()
}

/// How a branch went into its target.
#[derive(Debug, PartialEq)]
pub enum Integrated {
    Done,
    /// Stopped on conflicts (undone): the files.
    Conflict(Vec<String>),
}

/// Files left unmerged in `cwd` by a merge or a rebase that stopped.
pub async fn unmerged(cwd: &str) -> Vec<String> {
    text(cwd, &["diff", "--name-only", "--diff-filter=U"])
        .await
        .map(|s| {
            s.lines()
                .filter(|l| !l.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Brings `branch` into the branch checked out in `dir`: a merge commit (git's own message), a
/// squash committed with `message`, or a rebase of `branch` on `target` (in `branch_dir`, where it
/// is checked out) followed by a fast-forward. A conflict is undone and its files returned.
pub async fn integrate(
    dir: &str,
    branch: &str,
    strategy: &str,
    message: &str,
    branch_dir: &str,
    target: &str,
) -> Result<Integrated> {
    match strategy {
        "merge" => {
            if let Err(e) = run(dir, &["merge", "--no-ff", "--no-edit", branch]).await {
                let files = unmerged(dir).await;
                let _ = run(dir, &["merge", "--abort"]).await;
                return if files.is_empty() {
                    Err(e)
                } else {
                    Ok(Integrated::Conflict(files))
                };
            }
        }
        "rebase" => {
            if let Err(e) = run(branch_dir, &["rebase", target]).await {
                let files = unmerged(branch_dir).await;
                let _ = run(branch_dir, &["rebase", "--abort"]).await;
                return if files.is_empty() {
                    Err(e)
                } else {
                    Ok(Integrated::Conflict(files))
                };
            }
            run(dir, &["merge", "--ff-only", branch]).await?;
        }
        _ => {
            if let Err(e) = run(dir, &["merge", "--squash", branch]).await {
                let files = unmerged(dir).await;
                let _ = run(dir, &["reset", "--merge"]).await;
                return if files.is_empty() {
                    Err(e)
                } else {
                    Ok(Integrated::Conflict(files))
                };
            }
            if let Err(e) = run(dir, &["commit", "-q", "-m", message]).await {
                let _ = run(dir, &["reset", "--merge"]).await;
                return Err(e);
            }
        }
    }
    Ok(Integrated::Done)
}

/// True when tracked files have uncommitted changes (staged or not).
pub async fn has_tracked_changes(repo: &str) -> Result<bool> {
    Ok(
        !run(repo, &["status", "--porcelain", "--untracked-files=no"])
            .await?
            .is_empty(),
    )
}

pub async fn merge(repo: &str, branch: &str, squash: bool, message: &str) -> Result<String> {
    if squash {
        if let Err(e) = run(repo, &["merge", "--squash", branch]).await {
            // A conflicting squash leaves markers and unmerged entries behind: undo them.
            let _ = run(repo, &["reset", "--merge"]).await;
            return Err(e);
        }
        match run(repo, &["commit", "-m", message]).await {
            Ok(o) => Ok(String::from_utf8_lossy(&o).trim().to_string()),
            Err(e) => {
                let _ = run(repo, &["reset", "--merge"]).await;
                Err(e)
            }
        }
    } else {
        match run(repo, &["merge", "--no-ff", "-m", message, branch]).await {
            Ok(o) => Ok(String::from_utf8_lossy(&o).trim().to_string()),
            Err(e) => {
                let _ = run(repo, &["merge", "--abort"]).await;
                Err(e)
            }
        }
    }
}

pub async fn rename_current_branch(worktree: &str, new_name: &str) -> Result<()> {
    run(worktree, &["branch", "-m", new_name]).await.map(|_| ())
}

fn no_remote(lang: crate::i18n::Lang) -> String {
    tr_in!(
        lang,
        "Aucun dépôt distant n'est configuré pour ce dépôt.",
        "No remote is set up for this repository."
    )
}

pub async fn remotes(repo: &str) -> Vec<String> {
    text(repo, &["remote"])
        .await
        .map(|s| s.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

/// The remote to sync with: the upstream's, else origin, else the only one.
pub fn sync_remote<'a>(remotes: &'a [String], upstream: Option<&str>) -> Option<&'a str> {
    let of_upstream = upstream.and_then(|u| {
        remotes
            .iter()
            .filter(|r| u.starts_with(&format!("{r}/")))
            .max_by_key(|r| r.len())
    });
    of_upstream
        .or_else(|| remotes.iter().find(|r| *r == "origin"))
        .or(match remotes {
            [only] => Some(only),
            _ => None,
        })
        .map(String::as_str)
}

/// When the checkout at `root` was last fetched (the date of its FETCH_HEAD), ms since epoch.
pub fn last_fetch(root: &str) -> Option<i64> {
    let dot = Path::new(root).join(".git");
    let dir = if dot.is_dir() {
        dot
    } else {
        // A linked worktree or a submodule: ".git" is a file pointing at the real folder.
        let text = std::fs::read_to_string(&dot).ok()?;
        Path::new(root).join(text.trim().strip_prefix("gitdir:")?.trim())
    };
    let at = std::fs::metadata(dir.join("FETCH_HEAD"))
        .ok()?
        .modified()
        .ok()?;
    Some(at.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis() as i64)
}

/// What a pull brought in: « 1 commit tiré », « 3 commits tirés ».
fn pulled(lang: crate::i18n::Lang, n: u32) -> String {
    tr_n_in!(
        lang,
        n,
        "{n} commit tiré",
        "{n} commits tirés",
        "{n} commit pulled",
        "{n} commits pulled"
    )
}

/// What a push sent: « Rien à pousser », « 1 commit poussé », « 3 commits poussés ».
fn pushed(lang: crate::i18n::Lang, n: u32) -> String {
    if n == 0 {
        return tr_in!(lang, "Rien à pousser", "Nothing to push");
    }
    tr_n_in!(
        lang,
        n,
        "{n} commit poussé",
        "{n} commits poussés",
        "{n} commit pushed",
        "{n} commits pushed"
    )
}

/// The checked-out branch, which a detached HEAD does not have.
fn sync_branch(st: &Status) -> Result<&str> {
    if st.branch.is_empty() || st.branch == "(detached)" {
        bail!(tr!(
            "HEAD détachée : aucune branche à synchroniser.",
            "Detached HEAD: no branch to sync."
        ));
    }
    Ok(&st.branch)
}

/// Fetches the remote the current branch syncs with (every remote when that is ambiguous).
pub async fn fetch(repo: &str, background: bool) -> Result<()> {
    let (st, remotes) = tokio::join!(status(repo), remotes(repo));
    if remotes.is_empty() {
        bail!(no_remote(crate::i18n::ui()));
    }
    let upstream = st.ok().and_then(|s| s.upstream);
    let target = sync_remote(&remotes, upstream.as_deref()).unwrap_or("--all");
    let mut args = vec!["fetch", "--quiet", "--prune"];
    if background {
        // Windows cannot run it detached: it would count against the time limit.
        args.push("--no-auto-maintenance");
    }
    args.push(target);
    run_net(repo, &args, background).await.map(|_| ())
}

/// What a fetch brought, for the user.
pub fn fetch_summary(st: &Status) -> String {
    fetch_summary_in(crate::i18n::ui(), st)
}

fn fetch_summary_in(lang: crate::i18n::Lang, st: &Status) -> String {
    match (&st.upstream, st.behind) {
        (None, _) => tr_in!(lang, "Fetch terminé", "Fetch done"),
        (Some(up), _) if st.upstream_gone => tr_in!(
            lang,
            "Fetch terminé : la branche distante {up} n'existe plus",
            "Fetch done: the remote branch {up} no longer exists"
        ),
        (Some(_), 0) => tr_in!(
            lang,
            "Fetch terminé : déjà à jour",
            "Fetch done: already up to date"
        ),
        (Some(_), n) => tr_n_in!(
            lang,
            n,
            "Fetch terminé : {n} commit à tirer",
            "Fetch terminé : {n} commits à tirer",
            "Fetch done: {n} commit to pull",
            "Fetch done: {n} commits to pull"
        ),
    }
}

/// Fetches, then brings the upstream's new commits in, fast-forward only (`git pull --ff-only`).
pub async fn pull(repo: &str) -> Result<String> {
    let st = status(repo).await?;
    let branch = sync_branch(&st)?;
    let Some(upstream) = st.upstream.clone() else {
        bail!(tr!(
            "La branche {branch} ne suit aucune branche distante : rien à tirer.",
            "The branch {branch} doesn’t track a remote branch: nothing to pull."
        ));
    };
    fetch(repo, false).await?;
    let st = status(repo).await?;
    if st.upstream_gone {
        bail!(tr!(
            "La branche distante {upstream} n'existe plus : rien à tirer.",
            "The remote branch {upstream} no longer exists: nothing to pull."
        ));
    }
    if st.behind == 0 {
        return Ok(tr!("Déjà à jour", "Already up to date"));
    }
    if st.ahead > 0 {
        bail!(tr!(
            "La branche locale et {upstream} ont divergé : pull impossible en avance rapide. \
             Rebase ou merge à faire à la main (ou demande à un agent).",
            "The local branch and {upstream} have diverged: a fast-forward pull isn’t possible. \
             Rebase or merge by hand (or ask an agent)."
        ));
    }
    if let Err(e) = run(repo, &["merge", "--ff-only", "--quiet", "@{upstream}"]).await {
        if e.to_string().contains("would be overwritten") {
            bail!(tr!(
                "Des modifications non commitées seraient écrasées par le pull : commite-les ou mets-les de côté d'abord.",
                "The pull would overwrite uncommitted changes: commit or stash them first."
            ));
        }
        return Err(e);
    }
    Ok(pulled(crate::i18n::ui(), st.behind))
}

/// A push the remote refused because it has commits the branch lacks, told as such.
fn rejected(e: anyhow::Error) -> anyhow::Error {
    let msg = e.to_string();
    if msg.contains("[rejected]") || msg.contains("fetch first") {
        anyhow::anyhow!(tr!(
            "Le dépôt distant a des commits que tu n'as pas : récupère-les d'abord \
             (pull, ou rebase si les branches ont divergé).",
            "The remote has commits you don’t have: get them first \
             (pull, or rebase if the branches have diverged)."
        ))
    } else {
        e
    }
}

/// Pushes the current branch. A branch without upstream, or whose upstream was deleted, is
/// published on the remote (the upstream's, else origin, else the only one) and tracks it.
pub async fn push(repo: &str) -> Result<String> {
    let st = status(repo).await?;
    let branch = sync_branch(&st)?;
    if st.upstream.is_some() && !st.upstream_gone {
        run_net(repo, &["push"], false).await.map_err(rejected)?;
        return Ok(pushed(crate::i18n::ui(), st.ahead));
    }
    let remotes = remotes(repo).await;
    let Some(remote) = sync_remote(&remotes, st.upstream.as_deref()) else {
        if remotes.is_empty() {
            bail!(no_remote(crate::i18n::ui()));
        }
        bail!(tr!(
            "Plusieurs dépôts distants et aucun ne s'appelle origin : publie la branche à la main (git push -u <dépôt> {branch}).",
            "Several remotes and none is called origin: publish the branch by hand (git push -u <remote> {branch})."
        ));
    };
    run_net(repo, &["push", "-u", remote, branch], false)
        .await
        .map_err(rejected)?;
    Ok(tr!(
        "Branche {branch} publiée sur {remote}",
        "Branch {branch} published on {remote}"
    ))
}

/// Publishes `branch` from `cwd` on the repository's remote (`-u`): origin, else the only one.
/// Returns the remote's name.
pub async fn push_branch(cwd: &str, branch: &str) -> Result<String> {
    let remotes = remotes(cwd).await;
    let Some(remote) = sync_remote(&remotes, None).map(str::to_string) else {
        if remotes.is_empty() {
            bail!(no_remote(crate::i18n::ui()));
        }
        bail!(tr!(
            "Plusieurs dépôts distants et aucun ne s'appelle origin : pousse {branch} à la main.",
            "Several remotes and none is called origin: push {branch} by hand."
        ));
    };
    run_net(cwd, &["push", "-u", &remote, branch], false)
        .await
        .map_err(rejected)?;
    Ok(remote)
}

/// The address of `remote` as configured (before any `insteadOf` rewriting).
pub async fn remote_url(repo: &str, remote: &str) -> Option<String> {
    text(repo, &["config", "--get", &format!("remote.{remote}.url")])
        .await
        .ok()
        .filter(|s| !s.is_empty())
}

/// File list of a folder and when it was read.
type CachedFiles = (Instant, Arc<Vec<String>>);

/// Watches project folders and coalesces change notifications per project.
pub struct GitService {
    tx: mpsc::UnboundedSender<String>,
    watchers: Mutex<HashMap<String, (RecommendedWatcher, Arc<AtomicBool>)>>,
    files: Mutex<HashMap<String, CachedFiles>>,
}

impl GitService {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<String>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Self {
                tx,
                watchers: Mutex::default(),
                files: Mutex::default(),
            },
            rx,
        )
    }

    pub fn watch(&self, project_id: &str, path: &str) {
        let flag = Arc::new(AtomicBool::new(false));
        let (tx, pid, f) = (self.tx.clone(), project_id.to_string(), flag.clone());
        let handler = move |res: notify::Result<notify::Event>| {
            let Ok(ev) = res else { return };
            if !ev.paths.iter().any(|p| is_relevant(p)) {
                return;
            }
            if !f.swap(true, Ordering::AcqRel) {
                let _ = tx.send(pid.clone());
            }
        };
        match notify::recommended_watcher(handler) {
            Ok(mut w) => {
                if let Err(e) = w.watch(Path::new(path), RecursiveMode::Recursive) {
                    log::warn!("cannot watch {path}: {e}");
                    return;
                }
                self.watchers
                    .lock()
                    .insert(project_id.to_string(), (w, flag));
            }
            Err(e) => log::warn!("watcher error: {e}"),
        }
        self.refresh(project_id);
    }

    pub fn unwatch(&self, project_id: &str) {
        self.watchers.lock().remove(project_id);
    }

    pub fn refresh(&self, project_id: &str) {
        let _ = self.tx.send(project_id.to_string());
    }

    /// Called by the refresh loop before recomputing, so new events are signalled again.
    pub fn take_flag(&self, project_id: &str) {
        if let Some((_, f)) = self.watchers.lock().get(project_id) {
            f.store(false, Ordering::Release);
        }
    }

    pub fn invalidate_files(&self) {
        self.files.lock().clear();
    }

    pub async fn file_index(&self, cwd: &str) -> Arc<Vec<String>> {
        if let Some((at, files)) = self.files.lock().get(cwd) {
            if at.elapsed() < Duration::from_secs(20) {
                return files.clone();
            }
        }
        let files = match list_files(cwd).await {
            Ok(f) => f,
            Err(_) => {
                let root = cwd.to_string();
                tokio::task::spawn_blocking(move || walk_files(&root, 50_000))
                    .await
                    .unwrap_or_default()
            }
        };
        let files = Arc::new(files);
        self.files
            .lock()
            .insert(cwd.to_string(), (Instant::now(), files.clone()));
        files
    }
}

fn is_relevant(p: &Path) -> bool {
    let s = p.to_string_lossy();
    let git_idx = s.find("\\.git\\").or_else(|| s.find("/.git/"));
    match git_idx {
        None => !s.ends_with(".git"),
        Some(i) => {
            let inner = &s[i + 6..];
            inner == "index"
                || inner == "HEAD"
                || inner.starts_with("refs")
                || inner.ends_with("\\index")
                || inner.ends_with("/index")
                || inner.ends_with("HEAD")
        }
    }
}

/// Fuzzy file matching for @-mentions: subsequence match, basename hits and short paths first.
pub fn fuzzy_files(files: &[String], query: &str, limit: usize) -> Vec<String> {
    let q = query.to_lowercase().replace('\\', "/");
    if q.is_empty() {
        return files.iter().take(limit).cloned().collect();
    }
    let mut scored: Vec<(i64, &String)> = files
        .iter()
        .filter_map(|f| {
            let lf = f.to_lowercase();
            let base = lf.rsplit('/').next().unwrap_or(&lf);
            let mut score: i64 = if base.starts_with(&q) {
                1000
            } else if base.contains(&q) {
                700
            } else if lf.contains(&q) {
                400
            } else {
                let mut it = lf.chars();
                if !q.chars().all(|c| it.any(|x| x == c)) {
                    return None;
                }
                100
            };
            score -= lf.len() as i64;
            Some((score, f))
        })
        .collect();
    scored.sort_by_key(|s| std::cmp::Reverse(s.0));
    scored
        .into_iter()
        .take(limit)
        .map(|(_, f)| f.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The diff of a file named `name` with `lines` added lines.
    fn file_diff(name: &str, lines: usize) -> String {
        format!(
            "diff --git a/{name} b/{name}\nnew file mode 100644\n--- /dev/null\n+++ b/{name}\n@@ -0,0 +1,{lines} @@\n{}",
            "+x\n".repeat(lines)
        )
    }

    fn flagged_file(name: &str) -> String {
        format!(
            "diff --git a/{name} b/{name}\nnew file mode 100644\n--- /dev/null\n+++ b/{name}\n{TOO_LARGE}\n"
        )
    }

    #[test]
    fn a_file_diff_past_the_cap_is_flagged_and_its_neighbours_are_untouched() {
        let (before, big, after) = (
            file_diff("before.txt", 3),
            file_diff("big.txt", 50),
            file_diff("after.txt", 3),
        );
        let all = format!("{before}{big}{after}");
        assert!(before.len() < 200 && big.len() > 200, "{}", big.len());
        assert_eq!(
            cap_file_diffs(&all, 200),
            format!("{before}{}{after}", flagged_file("big.txt"))
        );
    }

    #[test]
    fn a_diff_within_the_cap_is_sent_as_it_is() {
        let all = format!("{}{}", file_diff("a.txt", 3), file_diff("b.txt", 4));
        assert_eq!(cap_file_diffs(&all, all.len()), all);
        // The cap is on one file's diff, not on the whole text, and a diff of exactly the cap passes.
        let longest = file_diff("b.txt", 4).len();
        assert!(all.len() > longest);
        assert_eq!(cap_file_diffs(&all, longest), all);
        assert_eq!(cap_file_diffs("", 10), "");
    }

    #[test]
    fn past_the_budget_the_files_left_are_flagged() {
        let (a, b, c) = (
            file_diff("a.txt", 30),
            file_diff("b.txt", 60),
            file_diff("c.txt", 3),
        );
        let all = format!("{a}{b}{c}");
        // Each fits the cap; once the first spent the budget, the others are only flagged.
        assert_eq!(
            cap_diff(&all, 1_000, a.len()),
            format!("{a}{}{}", flagged_file("b.txt"), flagged_file("c.txt"))
        );
        // A file too large for what is left is flagged, a later one that fits is sent.
        let budget = a.len() + flagged_file("b.txt").len() + c.len();
        assert_eq!(
            cap_diff(&all, 1_000, budget),
            format!("{a}{}{c}", flagged_file("b.txt"))
        );
        assert_eq!(cap_diff(&all, 1_000, all.len()), all);
        // The cap on one file still holds.
        assert_eq!(
            cap_diff(&all, a.len() - 1, all.len()),
            format!("{}{}{c}", flagged_file("a.txt"), flagged_file("b.txt"))
        );
    }

    #[test]
    fn a_line_quoting_a_file_header_does_not_split_the_diff() {
        // Inside a hunk the line starts with its +/-/space sign: only a real header starts a file.
        let quoted = "+diff --git a/x b/x\n".repeat(20);
        let one = format!(
            "diff --git a/doc.md b/doc.md\n--- /dev/null\n+++ b/doc.md\n@@ -0,0 +1,20 @@\n{quoted}"
        );
        let capped = cap_file_diffs(&one, 100);
        assert_eq!(
            capped,
            format!("diff --git a/doc.md b/doc.md\n--- /dev/null\n+++ b/doc.md\n{TOO_LARGE}\n")
        );
    }

    #[test]
    fn a_name_is_written_in_a_header_the_way_git_does() {
        // Verbatim, accents and spaces included.
        assert_eq!(
            header_name("b/src/résumé final.md"),
            "b/src/résumé final.md"
        );
        // C-style quotes for what could end the line or the name.
        assert_eq!(header_name("b/a\nb"), r#""b/a\nb""#);
        assert_eq!(header_name("b/a\"b"), r#""b/a\"b""#);
        assert_eq!(header_name(r"b/a\b"), r#""b/a\\b""#);
        assert_eq!(header_name("b/a\tb\r\x01\x7f"), r#""b/a\tb\r\001\177""#);
    }

    #[test]
    fn a_path_with_a_line_break_cannot_write_header_lines_of_its_own() {
        let header = new_file_header("x\nDiff too large\nnew file");
        assert_eq!(header.lines().count(), 4, "{header}");
        assert!(header.lines().all(|l| l != TOO_LARGE), "{header}");
        let name = r#""b/x\nDiff too large\nnew file""#;
        assert_eq!(
            header,
            format!(
                "diff --git {} {name}\nnew file mode 100644\n--- /dev/null\n+++ {name}\n",
                r#""a/x\nDiff too large\nnew file""#
            )
        );
    }

    #[test]
    fn a_flagged_file_keeps_what_the_header_says_of_it() {
        let rename = format!(
            "diff --git a/old.txt b/new.txt\nsimilarity index 90%\nrename from old.txt\nrename to new.txt\nindex 1..2 100644\n--- a/old.txt\n+++ b/new.txt\n@@ -1 +1,40 @@\n{}",
            "+x\n".repeat(40)
        );
        let capped = cap_file_diffs(&rename, 100);
        assert!(capped.contains("rename from old.txt"), "{capped}");
        assert!(
            capped.ends_with(&format!("+++ b/new.txt\n{TOO_LARGE}\n")),
            "{capped}"
        );
        assert!(!capped.contains("@@") && !capped.contains("+x"), "{capped}");
    }

    #[test]
    fn parses_porcelain_v2() {
        let raw = b"# branch.oid abc\0# branch.head feat/x\x001 .M N... 100644 100644 100644 a b src/app.ts\x001 A. N... 0 100644 100644 0 b new file.ts\x002 R. N... 100644 100644 100644 a b R100 renamed.ts\0old.ts\0? notes.txt\x001 .D N... 100644 100644 0 a 0 gone.ts\0";
        let st = parse_status(raw);
        assert_eq!(st.branch, "feat/x");
        let got: Vec<(String, char)> = st
            .entries
            .iter()
            .map(|e| (e.path.clone(), e.status))
            .collect();
        assert_eq!(
            got,
            vec![
                ("src/app.ts".into(), 'M'),
                ("new file.ts".into(), 'A'),
                ("renamed.ts".into(), 'M'),
                ("notes.txt".into(), 'A'),
                ("gone.ts".into(), 'D'),
            ]
        );
        assert_eq!(st.entries[2].orig.as_deref(), Some("old.ts"));
        assert_eq!(st.entries[0].orig, None);
    }

    #[test]
    fn parses_the_upstream_and_the_ahead_behind_counts() {
        let raw = b"# branch.oid abc\0# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -3\x001 .M N... 100644 100644 100644 a b src/app.ts\0";
        let st = parse_status(raw);
        assert_eq!(st.branch, "main");
        assert_eq!(st.upstream.as_deref(), Some("origin/main"));
        assert!(!st.upstream_gone);
        assert_eq!((st.ahead, st.behind), (2, 3));
        assert_eq!(st.entries.len(), 1);
    }

    #[test]
    fn a_branch_without_upstream_has_nothing_to_sync() {
        let st = parse_status(b"# branch.oid abc\0# branch.head feat/x\0");
        assert_eq!(st.branch, "feat/x");
        assert_eq!(st.upstream, None);
        assert!(!st.upstream_gone);
        assert_eq!((st.ahead, st.behind), (0, 0));
    }

    #[test]
    fn an_upstream_gone_from_the_remote_has_no_counts() {
        // `branch.ab` is left out when the upstream branch no longer exists.
        let st = parse_status(
            b"# branch.oid abc\0# branch.head feat/x\0# branch.upstream origin/feat/x\0",
        );
        assert_eq!(st.upstream.as_deref(), Some("origin/feat/x"));
        assert!(st.upstream_gone);
        assert_eq!((st.ahead, st.behind), (0, 0));
    }

    #[test]
    fn a_detached_head_has_no_upstream() {
        let st = parse_status(b"# branch.oid abc\0# branch.head (detached)\0");
        assert_eq!(st.branch, "(detached)");
        assert_eq!(st.upstream, None);
    }

    #[test]
    fn syncs_with_the_upstreams_remote_else_origin_else_the_only_one() {
        let v = |names: &[&str]| names.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let (both, one, two) = (v(&["fork", "origin"]), v(&["fork"]), v(&["a", "b"]));
        assert_eq!(sync_remote(&both, Some("fork/main")), Some("fork"));
        assert_eq!(sync_remote(&both, None), Some("origin"));
        assert_eq!(sync_remote(&one, None), Some("fork"));
        assert_eq!(sync_remote(&two, None), None);
        assert_eq!(sync_remote(&[], None), None);
        // A remote name may contain a slash: the longest match wins.
        let nested = v(&["team", "team/eu"]);
        assert_eq!(sync_remote(&nested, Some("team/eu/main")), Some("team/eu"));
    }

    #[test]
    fn parses_log_records_with_parents_and_refs() {
        let raw = "a1\x1fb2 c3\x1fAda\x1f1790000000\x1fHEAD -> main, origin/main, tag: v0.1.0\x1fMerge branch 'ccm/x'\x1e\n\
                   b2\x1fd4\x1fBob\x1f1789990000\x1fccm/x\x1ffix: a | b, c\x1e\n\
                   d4\x1f\x1fAda\x1f1789980000\x1f\x1finit\x1e\n";
        let log = parse_log(raw.as_bytes());
        assert_eq!(
            log[0],
            Commit {
                hash: "a1".into(),
                parents: vec!["b2".into(), "c3".into()],
                author: "Ada".into(),
                time: 1790000000,
                refs: vec![
                    "HEAD".into(),
                    "main".into(),
                    "origin/main".into(),
                    "tag: v0.1.0".into()
                ],
                subject: "Merge branch 'ccm/x'".into(),
            }
        );
        assert_eq!(log[1].refs, vec!["ccm/x".to_string()]);
        assert_eq!(log[1].subject, "fix: a | b, c");
        assert!(log[2].parents.is_empty() && log[2].refs.is_empty());
        assert_eq!(log.len(), 3);
    }

    #[test]
    fn parses_numstat_with_renames() {
        let raw = b"3\t1\tsrc/a.ts\0-\t-\timg.png\x005\t0\t\0old.ts\0new.ts\0";
        let m = parse_numstat(raw);
        assert_eq!(m["src/a.ts"], (3, 1));
        assert_eq!(m["img.png"], (0, 0));
        assert_eq!(m["new.ts"], (5, 0));
    }

    #[test]
    fn a_long_list_of_paths_is_cut_into_command_lines_that_fit() {
        let paths: Vec<String> = (0..2000)
            .map(|i| format!("generated/a-rather-long-file-name-{i:04}.txt"))
            .collect();
        let chunks = command_line_chunks(&paths);
        assert!(chunks.len() > 1);
        assert!(chunks
            .iter()
            .all(|c| c.iter().map(|p| p.len() + 1).sum::<usize>() <= 16_000));
        // Nothing lost, nothing reordered.
        assert_eq!(chunks.concat(), paths);
        assert!(command_line_chunks(&[]).is_empty());
        // A path longer than the budget goes through alone rather than being dropped.
        let long = vec!["x".repeat(20_000), "y".to_string()];
        assert_eq!(command_line_chunks(&long).len(), 2);
    }

    #[test]
    fn fuzzy_prefers_basename() {
        let files = vec![
            "docs/SPEC.md".to_string(),
            "src/spec/helpers.ts".to_string(),
            "README.md".to_string(),
        ];
        let r = fuzzy_files(&files, "spec", 10);
        assert_eq!(r[0], "docs/SPEC.md");
        assert!(!r.contains(&"README.md".to_string()));
    }

    #[test]
    fn watcher_filter() {
        assert!(is_relevant(Path::new("C:\\p\\src\\a.ts")));
        assert!(is_relevant(Path::new("C:\\p\\.git\\index")));
        assert!(!is_relevant(Path::new("C:\\p\\.git\\objects\\ab\\cd")));
        assert!(!is_relevant(Path::new("C:\\p\\.git\\index.lock")));
    }
}

#[cfg(test)]
mod repo_tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;

    #[test]
    fn tells_in_english_what_a_sync_did_with_the_plural_of_each_language() {
        use crate::i18n::Lang::{En, Fr};
        let st = |upstream: Option<&str>, upstream_gone: bool, behind: u32| Status {
            branch: "main".into(),
            upstream: upstream.map(str::to_string),
            upstream_gone,
            behind,
            ..Status::default()
        };
        let main = Some("origin/main");
        assert_eq!(
            [
                st(None, false, 0),
                st(main, false, 0),
                st(main, false, 1),
                st(main, false, 3),
                st(Some("origin/x"), true, 0),
            ]
            .map(|s| fetch_summary_in(En, &s)),
            [
                "Fetch done",
                "Fetch done: already up to date",
                "Fetch done: 1 commit to pull",
                "Fetch done: 3 commits to pull",
                "Fetch done: the remote branch origin/x no longer exists",
            ]
        );
        assert_eq!(
            [1, 2].map(|n| pulled(En, n)),
            ["1 commit pulled", "2 commits pulled"]
        );
        assert_eq!(
            [0, 1, 2].map(|n| pushed(En, n)),
            ["Nothing to push", "1 commit pushed", "2 commits pushed"]
        );
        assert_eq!(
            [0, 1, 2].map(|n| pushed(Fr, n)),
            ["Rien à pousser", "1 commit poussé", "2 commits poussés"]
        );
        assert_eq!(no_remote(En), "No remote is set up for this repository.");
        assert_eq!(
            [1, 25].map(|n| more_files(En, n)),
            ["… and 1 more file", "… and 25 more files"]
        );
    }

    fn repo(name: &str) -> String {
        let dir = crate::paths::test_dir(name);
        let g = |args: &[&str]| {
            assert!(Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(args)
                .status()
                .unwrap()
                .success())
        };
        g(&["init", "-q", "-b", "main"]);
        g(&["config", "user.email", "t@t"]);
        g(&["config", "user.name", "t"]);
        std::fs::write(dir.join("résumé.md"), "a\n").unwrap();
        g(&["add", "-A"]);
        g(&["commit", "-qm", "init"]);
        dir.to_string_lossy().to_string()
    }

    #[tokio::test]
    async fn accented_paths_are_not_escaped() {
        let r = repo("git-accents");
        std::fs::write(Path::new(&r).join("résumé.md"), "b\n").unwrap();
        let changes = file_changes(&r).await.unwrap();
        assert_eq!(changes[0].path, "résumé.md");
        assert_eq!((changes[0].add, changes[0].del), (1, 1));
        let d = diff(&r, &[]).await.unwrap();
        assert!(d.contains("+++ b/résumé.md"), "{d}");
    }

    fn git(r: &str, args: &[&str]) {
        assert!(Command::new("git")
            .arg("-C")
            .arg(r)
            .args(args)
            .status()
            .unwrap()
            .success())
    }

    #[tokio::test]
    async fn a_worktree_is_added_on_a_branch_of_its_own_from_a_base() {
        let r = repo("git-worktree-on");
        git(&r, &["checkout", "-qb", "release"]);
        std::fs::write(Path::new(&r).join("release.txt"), "r\n").unwrap();
        git(&r, &["add", "-A"]);
        git(&r, &["commit", "-qm", "release"]);
        git(&r, &["checkout", "-q", "main"]);
        let (path, branch) = worktree_add_on(&r, "ticket/dem-1", "release")
            .await
            .unwrap();
        assert_eq!(branch, "ticket/dem-1");
        assert!(Path::new(&path).ends_with(".claude/worktrees/dem-1"));
        assert!(Path::new(&path).join("release.txt").exists());
        // The branch taken, then the folder taken, then the same last part under another prefix.
        let (second, branch) = worktree_add_on(&r, "ticket/dem-1", "main").await.unwrap();
        assert_eq!(branch, "ticket/dem-1-2");
        assert!(Path::new(&second).ends_with("dem-1-2"));
        assert!(!Path::new(&second).join("release.txt").exists());
        std::fs::create_dir_all(Path::new(&r).join(".claude/worktrees/dem-2")).unwrap();
        let (third, branch) = worktree_add_on(&r, "ticket/dem-2", "main").await.unwrap();
        assert_eq!(branch, "ticket/dem-2-2");
        assert!(Path::new(&third).ends_with("dem-2-2"));
        let (fourth, branch) = worktree_add_on(&r, "autre/dem-1", "main").await.unwrap();
        assert_eq!(branch, "autre/dem-1-3");
        assert!(Path::new(&fourth).ends_with("dem-1-3"));
        // The folder is kept out of the repository's own status.
        assert!(dirty(&r).await.is_empty());
        // A base that does not exist: an error, nothing left behind.
        assert!(worktree_add_on(&r, "ticket/dem-9", "nowhere")
            .await
            .is_err());
        assert!(!branch_exists(&r, "ticket/dem-9").await);
    }

    #[tokio::test]
    async fn a_base_that_looks_like_a_git_option_is_refused_and_nothing_is_created() {
        let r = repo("git-worktree-on-option");
        for base in ["-x", "--lock", "--detach", "-b"] {
            let e = worktree_add_on(&r, "ticket/dem-5", base)
                .await
                .expect_err(base);
            assert!(format!("{e:#}").contains(base), "{base}: {e:#}");
            assert!(!branch_exists(&r, "ticket/dem-5").await, "{base}");
            assert!(
                !Path::new(&r).join(".claude/worktrees/dem-5").exists(),
                "{base}"
            );
            // Only the repository's own worktree is left (none locked, none added).
            let listed = text(&r, &["worktree", "list", "--porcelain"])
                .await
                .unwrap();
            let worktrees = listed
                .lines()
                .filter(|l| l.starts_with("worktree "))
                .count();
            assert_eq!(worktrees, 1, "{base}: {listed}");
            assert!(!listed.contains("locked"), "{base}: {listed}");
        }
        // A real base still works with the `--` before it.
        let (path, _) = worktree_add_on(&r, "ticket/dem-5", "main").await.unwrap();
        assert!(Path::new(&path).join("résumé.md").exists());
    }

    #[tokio::test]
    async fn branches_are_listed_the_current_one_first() {
        let r = repo("git-branches");
        git(&r, &["branch", "aaa"]);
        git(&r, &["branch", "zzz"]);
        assert_eq!(branches(&r).await.unwrap(), ["main", "aaa", "zzz"]);
    }

    #[tokio::test]
    async fn the_checkout_of_a_branch_is_found_among_the_worktrees() {
        let r = repo("git-checkout-of");
        git(&r, &["branch", "free"]);
        let wt = crate::paths::test_dir("git-checkout-of-wt")
            .join("side")
            .to_string_lossy()
            .to_string();
        git(&r, &["worktree", "add", "-q", "-b", "side", &wt]);
        let same = |a: &str, b: &str| {
            Path::new(a).canonicalize().unwrap() == Path::new(b).canonicalize().unwrap()
        };
        assert!(same(&checkout_of(&r, "main").await.unwrap().unwrap(), &r));
        assert!(same(&checkout_of(&r, "side").await.unwrap().unwrap(), &wt));
        assert_eq!(checkout_of(&r, "free").await.unwrap(), None);
        assert_eq!(ahead_of(&r, "main", "side").await.unwrap(), 0);
        git(&r, &["commit", "-q", "--allow-empty", "-m", "one more"]);
        assert_eq!(ahead_of(&r, "side", "main").await.unwrap(), 1);
        // A count git could not make is not "none": a ticket must not be taken as without change.
        assert!(ahead_of(&r, "main", "nowhere").await.is_err());
    }

    #[tokio::test]
    async fn only_a_local_branch_is_switched_to() {
        let r = repo("git-switch");
        git(&r, &["branch", "side"]);
        switch(&r, "side").await.unwrap();
        assert_eq!(head_branch(&r).await, "side");
        // A branch of the remote only: git would make a local one from it, a base is never guessed.
        git(
            &r,
            &["remote", "add", "origin", "https://example.invalid/r.git"],
        );
        git(&r, &["update-ref", "refs/remotes/origin/feat", "HEAD"]);
        assert!(switch(&r, "feat").await.is_err());
        assert!(!branch_exists(&r, "feat").await);
        // A name git would read as an option.
        assert!(switch(&r, "--detach").await.is_err());
        assert_eq!(head_branch(&r).await, "side");
    }

    #[tokio::test]
    async fn staging_leaves_out_the_files_asked_and_tells_when_nothing_is_left() {
        let r = repo("git-stage");
        let root = Path::new(&r);
        std::fs::write(root.join(".env"), "SECRET=1\n").unwrap();
        assert!(!stage_all(&r, &[".env".into()]).await.unwrap());
        std::fs::write(root.join("a.txt"), "a\n").unwrap();
        assert!(stage_all(&r, &[".env".into()]).await.unwrap());
        assert!(staged_stat(&r, "main").await.contains("a.txt"));
        commit_staged(&r, "feat: a [DEM-1]").await.unwrap();
        assert_eq!(text(&r, &["ls-files"]).await.unwrap(), "a.txt\nrésumé.md");
        assert_eq!(
            text(&r, &["log", "-1", "--format=%s"]).await.unwrap(),
            "feat: a [DEM-1]"
        );
        // A file is left out even once staged (an agent's `git add`).
        git(&r, &["add", ".env"]);
        assert!(!stage_all(&r, &[".env".into()]).await.unwrap());
        assert_eq!(
            text(&r, &["status", "--porcelain"]).await.unwrap(),
            "?? .env"
        );
    }

    #[tokio::test]
    async fn committing_paths_takes_those_and_nothing_else() {
        let r = repo("git-commit-paths");
        let root = Path::new(&r);
        git(&r, &["config", "core.autocrlf", "false"]);
        for f in ["a.txt", "b.txt", "gone.txt", "old.txt"] {
            std::fs::write(root.join(f), format!("{f}\n")).unwrap();
        }
        git(&r, &["add", "-A"]);
        git(&r, &["commit", "-qm", "files"]);
        // Chosen: a change, a deletion, a rename (staged), a new file.
        std::fs::write(root.join("a.txt"), "2\n").unwrap();
        std::fs::remove_file(root.join("gone.txt")).unwrap();
        git(&r, &["mv", "old.txt", "new.txt"]);
        std::fs::write(root.join("n.txt"), "n\n").unwrap();
        // Not chosen: a change the user staged, a new file.
        std::fs::write(root.join("b.txt"), "2\n").unwrap();
        git(&r, &["add", "b.txt"]);
        std::fs::write(root.join("other.txt"), "o\n").unwrap();
        let chosen: Vec<String> = ["a.txt", "gone.txt", "new.txt", "n.txt"]
            .map(String::from)
            .to_vec();
        commit_paths(&r, &chosen, "feat: les fichiers choisis\n\nAvec un corps.")
            .await
            .unwrap();
        assert_eq!(
            text(&r, &["log", "-1", "--format=%B"]).await.unwrap(),
            "feat: les fichiers choisis\n\nAvec un corps."
        );
        assert_eq!(
            text(&r, &["show", "--name-status", "--format=", "-M", "HEAD"])
                .await
                .unwrap()
                .replace('\t', " "),
            "M a.txt\nD gone.txt\nA n.txt\nR100 old.txt new.txt"
        );
        // What was not chosen is as it was: staged, or new.
        assert_eq!(
            text(&r, &["status", "--porcelain"]).await.unwrap(),
            "M  b.txt\n?? other.txt"
        );
    }

    #[tokio::test]
    async fn a_list_of_paths_longer_than_a_command_line_is_committed_whole() {
        let r = repo("git-commit-many");
        // About 50,000 characters of paths: twice what one Windows command line takes.
        let paths: Vec<String> = (0..700)
            .map(|i| format!("generated/a-rather-long-file-name-for-the-command-line-{i:04}.txt"))
            .collect();
        std::fs::create_dir_all(Path::new(&r).join("generated")).unwrap();
        for p in &paths {
            std::fs::write(Path::new(&r).join(p), "x\n").unwrap();
        }
        assert!(paths.iter().map(|p| p.len() + 1).sum::<usize>() > 40_000);
        commit_paths(&r, &paths, "chore: beaucoup de fichiers")
            .await
            .unwrap();
        let committed = text(&r, &["show", "--name-only", "--format=", "HEAD"])
            .await
            .unwrap();
        assert_eq!(committed.lines().count(), 700);
        assert!(status(&r).await.unwrap().entries.is_empty());
    }

    #[tokio::test]
    async fn the_paths_git_ignores_are_told_even_once_in_the_index() {
        let r = repo("git-ignored");
        let root = Path::new(&r);
        git(&r, &["config", "core.autocrlf", "false"]);
        std::fs::write(root.join(".git/info/exclude"), ".env\n").unwrap();
        std::fs::write(root.join(".env"), "SECRET=1\n").unwrap();
        std::fs::write(root.join(".env.example"), "SECRET=\n").unwrap();
        // Forced into the index: git no longer says it is ignored, its rules still do.
        git(&r, &["add", "-f", ".env"]);
        let asked: Vec<String> = [".env", ".env.example", "résumé.md"]
            .map(String::from)
            .to_vec();
        assert_eq!(ignored(&r, &asked).await.unwrap(), [".env"]);
        assert!(ignored(&r, &asked[1..]).await.unwrap().is_empty());
        assert!(ignored(&r, &[]).await.unwrap().is_empty());
        // Not a repository: an error, never "nothing is ignored".
        let elsewhere = crate::paths::test_dir("git-ignored-not-a-repo");
        assert!(ignored(&elsewhere.to_string_lossy(), &asked).await.is_err());
    }

    #[tokio::test]
    async fn a_diff_reads_the_names_listed_as_they_are() {
        let r = repo("git-diff-literal");
        let root = Path::new(&r);
        git(&r, &["config", "core.autocrlf", "false"]);
        std::fs::write(root.join(".git/info/exclude"), ".env\n").unwrap();
        std::fs::write(root.join(".env"), "SECRET=1\n").unwrap();
        git(&r, &["add", "-f", ".env"]);
        // A name that, read as a pattern, would match `.env`.
        std::fs::write(root.join("[.]env"), "x\n").unwrap();
        let d = diff(&r, &["[.]env".to_string()]).await.unwrap();
        assert!(d.contains("+++ b/[.]env"), "{d}");
        assert!(!d.contains("SECRET"), "{d}");
    }

    #[tokio::test]
    async fn a_staged_new_file_with_a_space_in_its_name_is_listed_once() {
        let r = repo("git-diff-staged-space");
        let root = Path::new(&r);
        git(&r, &["config", "core.autocrlf", "false"]);
        // Git ends the `+++` line of such a name with a tab.
        std::fs::write(root.join("my file.txt"), "x\n").unwrap();
        std::fs::write(root.join("big file.txt"), big_text(MAX_FILE_DIFF + 100_000)).unwrap();
        git(&r, &["add", "my file.txt", "big file.txt"]);
        let d = diff(&r, &[]).await.unwrap();
        assert_eq!(d.matches("diff --git a/my file.txt").count(), 1, "{d:.600}");
        assert_eq!(
            d.matches("diff --git a/big file.txt").count(),
            1,
            "{d:.600}"
        );
        assert!(d.contains(TOO_LARGE), "{d:.600}");
    }

    /// A text of `len` bytes, in lines of 100.
    fn big_text(len: usize) -> String {
        format!("{}\n", "x".repeat(99)).repeat(len / 100)
    }

    #[tokio::test]
    async fn a_tracked_file_whose_diff_passes_the_cap_is_flagged_instead_of_sent() {
        let r = repo("git-diff-cap-tracked");
        let root = Path::new(&r);
        git(&r, &["config", "core.autocrlf", "false"]);
        std::fs::write(root.join("lock.json"), "{}\n").unwrap();
        std::fs::write(root.join("small.txt"), "1\n").unwrap();
        git(&r, &["add", "-A"]);
        git(&r, &["commit", "-qm", "lock"]);
        std::fs::write(root.join("lock.json"), big_text(MAX_FILE_DIFF + 100_000)).unwrap();
        std::fs::write(root.join("small.txt"), "2\n").unwrap();
        // A new file already in the index: git lists it, the untracked pass must not add it again.
        std::fs::write(root.join("staged.txt"), big_text(MAX_FILE_DIFF + 100_000)).unwrap();
        git(&r, &["add", "staged.txt"]);

        let d = diff(&r, &[]).await.unwrap();
        assert!(d.len() < 10_000, "{} bytes", d.len());
        assert!(
            d.contains(&format!("+++ b/lock.json\n{TOO_LARGE}\n")),
            "{d}"
        );
        assert!(
            d.contains(&format!("+++ b/staged.txt\n{TOO_LARGE}\n")),
            "{d}"
        );
        assert_eq!(d.matches("diff --git a/staged.txt").count(), 1, "{d}");
        assert!(d.contains("+2\n"), "{d}");
        // Named on its own, it is flagged just the same.
        let one = diff(&r, &["lock.json".to_string()]).await.unwrap();
        assert!(one.len() < 1_000 && one.contains(TOO_LARGE), "{one}");
    }

    #[tokio::test]
    async fn an_untracked_file_is_flagged_past_the_cap_and_binary_only_when_it_is() {
        let r = repo("git-diff-cap-untracked");
        let root = Path::new(&r);
        std::fs::write(root.join("huge.log"), big_text(MAX_FILE_DIFF + 100_000)).unwrap();
        // Over the old 1 MB limit, under the cap: shown, not taken for a binary file.
        std::fs::write(root.join("mid.log"), big_text(1_500_000)).unwrap();
        std::fs::write(root.join("tool.bin"), [0u8, 1, 2]).unwrap();
        let d = diff(&r, &[]).await.unwrap();
        assert!(
            d.contains(&format!("+++ b/huge.log\n{TOO_LARGE}\n")),
            "{d:.300}"
        );
        assert!(d.contains("+++ b/mid.log\n@@ -0,0 +1,15000 @@"), "{d:.300}");
        assert!(d.contains("Binary files /dev/null and b/tool.bin differ"));
        assert!(!d.contains(&format!("b/tool.bin\n{TOO_LARGE}")));
    }

    #[tokio::test]
    async fn untracked_files_past_the_diff_budget_are_flagged_not_dropped() {
        let r = repo("git-diff-budget-flags");
        let root = Path::new(&r);
        for name in ["a.log", "b.log", "c.log", "d.log"] {
            std::fs::write(root.join(name), big_text(2_000_000)).unwrap();
        }
        let d = diff(&r, &[]).await.unwrap();
        // The first two fill the budget; the others are listed, flagged, without being read.
        for name in ["a.log", "b.log"] {
            assert!(
                d.contains(&format!("+++ b/{name}\n@@ -0,0 +1,20000 @@")),
                "{name}"
            );
        }
        for name in ["c.log", "d.log"] {
            assert!(
                d.contains(&format!(
                    "diff --git a/{name} b/{name}\nnew file mode 100644\n--- /dev/null\n+++ b/{name}\n{TOO_LARGE}\n"
                )),
                "{name}"
            );
        }
    }

    /// A repository whose two untracked logs spend the diff budget, then `count` small new files.
    fn spent_budget(name: &str, count: usize) -> String {
        let r = repo(name);
        let root = Path::new(&r);
        for log in ["a1.log", "a2.log"] {
            std::fs::write(root.join(log), big_text(3_000_000)).unwrap();
        }
        for i in 0..count {
            std::fs::write(root.join(format!("n{i:04}.txt")), "x\n").unwrap();
        }
        r
    }

    #[tokio::test]
    async fn only_so_many_untracked_files_are_listed_once_the_budget_is_spent() {
        let r = spent_budget("git-diff-budget-listed", UNREAD_LISTED + 25);
        let d = diff(&r, &[]).await.unwrap();
        // The listed ones and the entry that counts the others.
        assert_eq!(d.matches(TOO_LARGE).count(), UNREAD_LISTED + 1);
        assert!(d.contains("diff --git a/n0000.txt b/n0000.txt\n"));
        assert!(d.contains("+++ b/… et 25 autres fichiers\n"));
        assert!(!d.contains("n0525.txt"));
        // The two logs, and a few lines per listed file.
        assert!(d.len() < 6_200_000 + 200_000, "{} bytes", d.len());
    }

    #[tokio::test]
    async fn the_entry_counting_the_unlisted_files_is_singular_for_one() {
        let r = spent_budget("git-diff-budget-singular", UNREAD_LISTED + 1);
        let d = diff(&r, &[]).await.unwrap();
        assert!(d.contains("+++ b/… et 1 autre fichier\n"));
        assert_eq!(d.matches(TOO_LARGE).count(), UNREAD_LISTED + 1);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn an_untracked_file_named_with_a_line_break_writes_no_header_of_its_own() {
        let r = repo("git-diff-newline-name");
        let name = "x\nDiff too large\nnew file";
        std::fs::write(Path::new(&r).join(name), "hello\n").unwrap();
        let d = diff(&r, &[]).await.unwrap();
        assert!(d.lines().all(|l| l != TOO_LARGE), "{d}");
        assert_eq!(
            d.lines().filter(|l| l.starts_with("diff --git ")).count(),
            1,
            "{d}"
        );
        assert!(d.contains("+hello\n"), "{d}");
    }

    #[tokio::test]
    async fn a_commit_patch_past_the_cap_is_flagged_instead_of_sent() {
        let r = repo("git-show-cap");
        let root = Path::new(&r);
        git(&r, &["config", "core.autocrlf", "false"]);
        std::fs::write(root.join("lock.json"), big_text(MAX_FILE_DIFF + 100_000)).unwrap();
        std::fs::write(root.join("small.txt"), "kept\n").unwrap();
        git(&r, &["add", "-A"]);
        git(&r, &["commit", "-qm", "lockfile"]);
        let hash = text(&r, &["rev-parse", "HEAD"]).await.unwrap();
        let shown = show(&r, &hash).await.unwrap();
        assert!(shown.len() < 10_000, "{} bytes", shown.len());
        assert!(
            shown.contains(&format!("+++ b/lock.json\n{TOO_LARGE}\n")),
            "{shown}"
        );
        assert!(shown.contains("+kept\n"), "{shown}");
    }

    /// A pre-commit hook in the repository at `r` that refuses every commit.
    fn refusing_hook(r: &str) {
        let hook = Path::new(r).join(".git").join("hooks").join("pre-commit");
        std::fs::create_dir_all(hook.parent().unwrap()).unwrap();
        std::fs::write(&hook, "#!/bin/sh\necho 'lint en échec' >&2\nexit 1\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    #[tokio::test]
    async fn a_refused_commit_leaves_the_index_as_it_was() {
        let r = repo("git-commit-refused");
        let root = Path::new(&r);
        git(&r, &["config", "core.autocrlf", "false"]);
        std::fs::write(root.join("b.txt"), "1\n").unwrap();
        git(&r, &["add", "b.txt"]);
        git(&r, &["commit", "-qm", "b"]);
        // Partly staged: version 2 in the index, version 3 on disk. And a new file.
        std::fs::write(root.join("b.txt"), "2\n").unwrap();
        git(&r, &["add", "b.txt"]);
        std::fs::write(root.join("b.txt"), "3\n").unwrap();
        std::fs::write(root.join("n.txt"), "n\n").unwrap();
        let index = || async { text(&r, &["ls-files", "--stage"]).await.unwrap() };
        let before = (
            index().await,
            text(&r, &["status", "--porcelain"]).await.unwrap(),
        );
        refusing_hook(&r);
        let chosen = ["b.txt", "n.txt"].map(String::from).to_vec();
        let e = commit_paths(&r, &chosen, "fix: refusé").await.unwrap_err();
        assert!(format!("{e:#}").contains("lint en échec"), "{e:#}");
        let after = (
            index().await,
            text(&r, &["status", "--porcelain"]).await.unwrap(),
        );
        assert_eq!(after, before);
        assert_eq!(text(&r, &["show", ":b.txt"]).await.unwrap(), "2");
        assert_eq!(text(&r, &["log", "-1", "--format=%s"]).await.unwrap(), "b");
    }

    /// `main` and a branch `feat` that changed the same file (`conflict`) or another one.
    fn diverged(name: &str, conflict: bool) -> String {
        let r = repo(name);
        git(&r, &["checkout", "-qb", "feat"]);
        let file = if conflict { "résumé.md" } else { "b.txt" };
        std::fs::write(Path::new(&r).join(file), "feat\n").unwrap();
        git(&r, &["add", "-A"]);
        git(&r, &["commit", "-qm", "feat"]);
        git(&r, &["checkout", "-q", "main"]);
        std::fs::write(Path::new(&r).join("résumé.md"), "main\n").unwrap();
        git(&r, &["commit", "-qam", "main"]);
        r
    }

    #[tokio::test]
    async fn a_file_a_merge_commit_brought_is_found_in_the_branch_and_its_history() {
        let r = diverged("git-merge-brings", false);
        let env = vec![".env".to_string()];
        // The agent merges its target and stages everything to commit the merge: the untracked
        // .env goes in with it.
        git(&r, &["checkout", "-q", "feat"]);
        git(&r, &["merge", "-q", "--no-ff", "--no-commit", "main"]);
        std::fs::write(Path::new(&r).join(".env"), "SECRET=1\n").unwrap();
        git(&r, &["add", "-A"]);
        git(&r, &["commit", "-qm", "merge main"]);
        git(&r, &["checkout", "-q", "main"]);
        assert_eq!(changed_in(&r, "main", "feat", &env).await.unwrap(), env);
        assert_eq!(touched_by(&r, "main", "feat", &env).await.unwrap(), env);
        // Taken out again by a later commit: only the history has it.
        git(&r, &["checkout", "-q", "feat"]);
        git(&r, &["rm", "--cached", "-q", ".env"]);
        git(&r, &["commit", "-qm", "sans env"]);
        git(&r, &["checkout", "-q", "main"]);
        assert!(changed_in(&r, "main", "feat", &env)
            .await
            .unwrap()
            .is_empty());
        assert_eq!(touched_by(&r, "main", "feat", &env).await.unwrap(), env);
        // Only what is asked; the branch's own other files are not.
        assert_eq!(
            changed_in(&r, "main", "feat", &["b.txt".to_string(), ".env".into()])
                .await
                .unwrap(),
            ["b.txt"]
        );
        assert!(changed_in(&r, "main", "feat", &[])
            .await
            .unwrap()
            .is_empty());
        assert!(changed_in(&r, "main", "nowhere", &env).await.is_err());
    }

    #[tokio::test]
    async fn the_files_a_branchs_own_commits_touched_are_found_even_when_removed_since() {
        let r = diverged("git-touched-by", false);
        let paths = |l: &[&str]| l.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let asked = paths(&["b.txt", "résumé.md", ".env"]);
        // Only what `feat` did: `main`'s own change of résumé.md is not counted.
        assert_eq!(
            touched_by(&r, "main", "feat", &asked).await.unwrap(),
            ["b.txt"]
        );
        // Committed, then taken out again: gone from the tree, still in the history.
        git(&r, &["checkout", "-q", "feat"]);
        std::fs::write(Path::new(&r).join(".env"), "SECRET=1\n").unwrap();
        git(&r, &["add", ".env"]);
        git(&r, &["commit", "-qm", "env"]);
        git(&r, &["rm", "--cached", "-q", ".env"]);
        git(&r, &["commit", "-qm", "sans env"]);
        // On a side branch merged in too.
        git(&r, &["checkout", "-q", "main"]);
        assert_eq!(
            text(&r, &["diff", "--name-only", "main...feat"])
                .await
                .unwrap(),
            "b.txt"
        );
        let mut touched = touched_by(&r, "main", "feat", &asked).await.unwrap();
        touched.sort();
        assert_eq!(touched, [".env", "b.txt"]);
        git(&r, &["checkout", "-qb", "side"]);
        std::fs::write(Path::new(&r).join("c.env"), "X=1\n").unwrap();
        git(&r, &["add", "c.env"]);
        git(&r, &["commit", "-qm", "c"]);
        git(&r, &["rm", "-q", "c.env"]);
        git(&r, &["commit", "-qm", "sans c"]);
        git(&r, &["checkout", "-q", "feat"]);
        git(&r, &["merge", "-q", "--no-edit", "side"]);
        assert_eq!(
            touched_by(&r, "main", "feat", &paths(&["c.env"]))
                .await
                .unwrap(),
            ["c.env"]
        );
        assert!(touched_by(&r, "main", "main", &asked)
            .await
            .unwrap()
            .is_empty());
        assert!(touched_by(&r, "main", "feat", &[])
            .await
            .unwrap()
            .is_empty());
        assert!(touched_by(&r, "main", "nowhere", &asked).await.is_err());
    }

    #[tokio::test]
    async fn a_merge_of_the_base_does_not_count_the_bases_own_files_as_the_branchs() {
        let r = diverged("git-touched-merge-base", false);
        let local = vec![".env.local".to_string()];
        // The target tracks .env.local, a file also copied into the worktrees...
        std::fs::write(Path::new(&r).join(".env.local"), "A=1\n").unwrap();
        git(&r, &["add", ".env.local"]);
        git(&r, &["commit", "-qm", "main tracks .env.local"]);
        // ...and the branch merges the target: the merge brings it from the target, not the branch.
        git(&r, &["checkout", "-q", "feat"]);
        git(&r, &["merge", "-q", "--no-edit", "main"]);
        git(&r, &["checkout", "-q", "main"]);
        assert!(touched_by(&r, "main", "feat", &local)
            .await
            .unwrap()
            .is_empty());
        // Changed by the branch after that: its own.
        git(&r, &["checkout", "-q", "feat"]);
        std::fs::write(Path::new(&r).join(".env.local"), "A=2\n").unwrap();
        git(&r, &["commit", "-qam", "feat changes .env.local"]);
        git(&r, &["checkout", "-q", "main"]);
        assert_eq!(touched_by(&r, "main", "feat", &local).await.unwrap(), local);
    }

    #[tokio::test]
    async fn a_branch_goes_in_by_squash_merge_commit_or_rebase() {
        let r = diverged("git-integrate-squash", false);
        assert_eq!(
            integrate(&r, "feat", "squash", "feat: b [DEM-1]", &r, "main")
                .await
                .unwrap(),
            Integrated::Done
        );
        assert_eq!(
            text(&r, &["log", "-1", "--format=%s"]).await.unwrap(),
            "feat: b [DEM-1]"
        );
        let r = diverged("git-integrate-merge", false);
        assert_eq!(
            integrate(&r, "feat", "merge", "", &r, "main")
                .await
                .unwrap(),
            Integrated::Done
        );
        assert_eq!(
            text(&r, &["rev-list", "--count", "--merges", "HEAD"])
                .await
                .unwrap(),
            "1"
        );
        let r = diverged("git-integrate-rebase", false);
        let wt = crate::paths::test_dir("git-integrate-rebase-wt")
            .join("feat")
            .to_string_lossy()
            .to_string();
        git(&r, &["worktree", "add", "-q", &wt, "feat"]);
        assert_eq!(
            integrate(&r, "feat", "rebase", "", &wt, "main")
                .await
                .unwrap(),
            Integrated::Done
        );
        assert_eq!(
            text(&r, &["rev-list", "--count", "--merges", "HEAD"])
                .await
                .unwrap(),
            "0"
        );
        assert!(Path::new(&r).join("b.txt").exists());
    }

    #[tokio::test]
    async fn a_conflict_is_undone_and_its_files_named() {
        for strategy in ["merge", "squash"] {
            let r = diverged(&format!("git-conflict-{strategy}"), true);
            assert_eq!(
                integrate(&r, "feat", strategy, "m", &r, "main")
                    .await
                    .unwrap(),
                Integrated::Conflict(vec!["résumé.md".into()])
            );
            assert_eq!(text(&r, &["status", "--porcelain"]).await.unwrap(), "");
        }
        let r = diverged("git-conflict-rebase", true);
        let wt = crate::paths::test_dir("git-conflict-rebase-wt")
            .join("feat")
            .to_string_lossy()
            .to_string();
        git(&r, &["worktree", "add", "-q", &wt, "feat"]);
        assert_eq!(
            integrate(&r, "feat", "rebase", "", &wt, "main")
                .await
                .unwrap(),
            Integrated::Conflict(vec!["résumé.md".into()])
        );
        assert_eq!(text(&wt, &["status", "--porcelain"]).await.unwrap(), "");
        // A merge stopped on its conflicts names them until they are resolved.
        let r = diverged("git-unmerged", true);
        assert!(run(&r, &["merge", "feat"]).await.is_err());
        assert_eq!(unmerged(&r).await, ["résumé.md"]);
    }

    async fn dirty(r: &str) -> Vec<(String, char)> {
        status(r)
            .await
            .unwrap()
            .entries
            .into_iter()
            .map(|e| (e.path, e.status))
            .collect()
    }

    #[tokio::test]
    async fn discarding_puts_files_back_as_committed() {
        let r = repo("git-discard");
        git(&r, &["config", "core.autocrlf", "false"]);
        let root = Path::new(&r);
        std::fs::write(root.join("keep.txt"), "kept\n").unwrap();
        git(&r, &["add", "-A"]);
        git(&r, &["commit", "-qm", "more"]);

        // A modified file, its change partly staged.
        std::fs::write(root.join("résumé.md"), "staged\n").unwrap();
        git(&r, &["add", "résumé.md"]);
        std::fs::write(root.join("résumé.md"), "staged\nand not\n").unwrap();
        discard(&r, "résumé.md").await.unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("résumé.md")).unwrap(),
            "a\n"
        );

        // New files, untracked or staged, are deleted.
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src").join("new.ts"), "x\n").unwrap();
        std::fs::write(root.join("staged.ts"), "y\n").unwrap();
        git(&r, &["add", "staged.ts"]);
        discard(&r, "src/new.ts").await.unwrap();
        discard(&r, "staged.ts").await.unwrap();
        assert!(!root.join("src").join("new.ts").exists());
        assert!(!root.join("staged.ts").exists());

        // A deleted file comes back.
        std::fs::remove_file(root.join("keep.txt")).unwrap();
        discard(&r, "keep.txt").await.unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("keep.txt")).unwrap(),
            "kept\n"
        );

        // A path is a file name, not a pattern: `a[b].txt` leaves `ab.txt` alone.
        std::fs::write(root.join("a[b].txt"), "new\n").unwrap();
        std::fs::write(root.join("ab.txt"), "new too\n").unwrap();
        discard(&r, "a[b].txt").await.unwrap();
        assert!(!root.join("a[b].txt").exists());
        assert_eq!(dirty(&r).await, vec![("ab.txt".to_string(), 'A')]);
    }

    #[tokio::test]
    async fn discarding_a_rename_puts_the_file_back_under_its_name() {
        let r = repo("git-discard-rename");
        git(&r, &["config", "core.autocrlf", "false"]);
        let root = Path::new(&r);
        git(&r, &["mv", "résumé.md", "cv.md"]);
        std::fs::write(root.join("cv.md"), "a\nb\n").unwrap();
        assert_eq!(dirty(&r).await, vec![("cv.md".to_string(), 'M')]);
        discard(&r, "cv.md").await.unwrap();
        assert!(!root.join("cv.md").exists());
        assert_eq!(
            std::fs::read_to_string(root.join("résumé.md")).unwrap(),
            "a\n"
        );
        assert_eq!(dirty(&r).await, vec![]);
    }

    #[tokio::test]
    async fn restoring_a_file_kept_on_disk_does_not_overwrite_it() {
        // `git rm --cached`: deleted from the index, the local copy (maybe edited) still there.
        let r = repo("git-discard-kept");
        let root = Path::new(&r);
        git(&r, &["rm", "--cached", "-q", "résumé.md"]);
        std::fs::write(root.join("résumé.md"), "local\n").unwrap();
        let st = status(&r).await.unwrap();
        assert!(st
            .entries
            .iter()
            .any(|e| e.path == "résumé.md" && e.status == 'D'));
        discard(&r, "résumé.md").await.unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("résumé.md")).unwrap(),
            "local\n"
        );
        // Tracked again, with the local content as a change.
        assert_eq!(dirty(&r).await, vec![("résumé.md".to_string(), 'M')]);
    }

    #[tokio::test]
    async fn discarding_only_touches_a_listed_file() {
        let r = repo("git-discard-guard");
        let root = Path::new(&r);
        std::fs::write(root.join("notes.txt"), "x\n").unwrap();
        std::fs::write(root.join("résumé.md"), "b\n").unwrap();
        for bad in [".", "", "../x", "*", "résumé.md/..", "clean.txt"] {
            assert!(discard(&r, bad).await.is_err(), "{bad:?} was accepted");
        }
        assert_eq!(
            dirty(&r).await,
            vec![
                ("résumé.md".to_string(), 'M'),
                ("notes.txt".to_string(), 'A')
            ]
        );
    }

    #[tokio::test]
    async fn logs_every_branch_and_shows_a_commit_diff() {
        let r = repo("git-log-branches");
        let g = |args: &[&str]| {
            assert!(Command::new("git")
                .arg("-C")
                .arg(&r)
                .args(args)
                .status()
                .unwrap()
                .success())
        };
        g(&["checkout", "-qb", "ccm/agent"]);
        std::fs::write(Path::new(&r).join("a.txt"), "agent\n").unwrap();
        g(&["add", "-A"]);
        g(&["commit", "-qm", "agent work"]);
        g(&["checkout", "-q", "main"]);
        std::fs::write(Path::new(&r).join("b.txt"), "main\n").unwrap();
        g(&["add", "-A"]);
        g(&["commit", "-qm", "main work"]);
        g(&["merge", "-q", "--no-ff", "-m", "merge agent", "ccm/agent"]);
        g(&["stash", "list"]);

        let log = log(&r, 50).await.unwrap();
        let subjects: Vec<&str> = log.iter().map(|c| c.subject.as_str()).collect();
        assert_eq!(subjects[0], "merge agent");
        assert_eq!(log[0].parents.len(), 2);
        assert!(log[0].refs.contains(&"main".to_string()));
        assert!(subjects.contains(&"agent work") && subjects.contains(&"main work"));
        assert_eq!(*subjects.last().unwrap(), "init");
        let agent = log.iter().find(|c| c.subject == "agent work").unwrap();
        assert!(agent.refs.contains(&"ccm/agent".to_string()));

        let shown = show(&r, &agent.hash).await.unwrap();
        assert!(
            shown.contains("+++ b/a.txt") && shown.contains("+agent"),
            "{shown}"
        );
        // A merge shows what it brought to its first parent.
        let merged = show(&r, &log[0].hash).await.unwrap();
        assert!(merged.contains("+++ b/a.txt"), "{merged}");
        assert!(show(&r, "--output=x").await.is_err());
    }

    fn git_in(dir: &Path, args: &[&str]) -> String {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn commit_file(dir: &Path, file: &str, content: &str) {
        std::fs::write(dir.join(file), content).unwrap();
        git_in(dir, &["add", "-A"]);
        git_in(dir, &["commit", "-qm", &format!("edit {file}")]);
    }

    /// A bare remote with one commit on main, and two clones of it: the project's checkout and
    /// someone else's.
    fn with_remote(name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let dir = crate::paths::test_dir(name);
        let bare = dir.join("remote.git");
        git_in(&dir, &["init", "-q", "--bare", "-b", "main", "remote.git"]);
        let clone = |to: &str| {
            let d = dir.join(to);
            git_in(&dir, &["clone", "-q", &bare.to_string_lossy(), to]);
            git_in(&d, &["config", "user.email", "t@t"]);
            git_in(&d, &["config", "user.name", "t"]);
            git_in(&d, &["config", "core.autocrlf", "false"]);
            d
        };
        let local = clone("local");
        git_in(&local, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        commit_file(&local, "a.txt", "a\n");
        git_in(&local, &["push", "-qu", "origin", "main"]);
        let other = clone("other");
        (local, other, bare)
    }

    fn s(p: &Path) -> String {
        p.to_string_lossy().to_string()
    }

    #[tokio::test]
    async fn a_fetch_shows_the_commits_to_pull_and_a_pull_brings_them_in() {
        let (local, other, _) = with_remote("git-sync-pull");
        let st = status(&s(&local)).await.unwrap();
        assert_eq!(st.upstream.as_deref(), Some("origin/main"));
        assert_eq!((st.ahead, st.behind), (0, 0));
        assert_eq!(last_fetch(&s(&local)), None);

        commit_file(&other, "b.txt", "b\n");
        git_in(&other, &["push", "-q"]);
        fetch(&s(&local), true).await.unwrap();
        let st = status(&s(&local)).await.unwrap();
        assert_eq!((st.ahead, st.behind), (0, 1));
        assert_eq!(fetch_summary(&st), "Fetch terminé : 1 commit à tirer");
        let fetched = last_fetch(&s(&local)).expect("fetch date");
        assert!((crate::model::now_ms() - fetched).abs() < 60_000);

        assert_eq!(pull(&s(&local)).await.unwrap(), "1 commit tiré");
        let st = status(&s(&local)).await.unwrap();
        assert_eq!((st.ahead, st.behind), (0, 0));
        assert_eq!(std::fs::read_to_string(local.join("b.txt")).unwrap(), "b\n");
        assert_eq!(pull(&s(&local)).await.unwrap(), "Déjà à jour");
    }

    #[tokio::test]
    async fn a_push_sends_the_local_commits() {
        let (local, _, bare) = with_remote("git-sync-push");
        commit_file(&local, "c.txt", "c\n");
        commit_file(&local, "d.txt", "d\n");
        assert_eq!(status(&s(&local)).await.unwrap().ahead, 2);
        assert_eq!(push(&s(&local)).await.unwrap(), "2 commits poussés");
        assert_eq!(status(&s(&local)).await.unwrap().ahead, 0);
        assert_eq!(
            git_in(&bare, &["rev-parse", "main"]),
            git_in(&local, &["rev-parse", "HEAD"])
        );
    }

    #[tokio::test]
    async fn pushing_a_new_branch_publishes_it_and_tracks_it() {
        let (local, _, bare) = with_remote("git-sync-publish");
        git_in(&local, &["checkout", "-qb", "feat/x"]);
        commit_file(&local, "e.txt", "e\n");
        assert_eq!(status(&s(&local)).await.unwrap().upstream, None);
        assert_eq!(
            push(&s(&local)).await.unwrap(),
            "Branche feat/x publiée sur origin"
        );
        let st = status(&s(&local)).await.unwrap();
        assert_eq!(st.upstream.as_deref(), Some("origin/feat/x"));
        assert_eq!((st.ahead, st.behind), (0, 0));
        assert_eq!(
            git_in(&bare, &["rev-parse", "feat/x"]),
            git_in(&local, &["rev-parse", "HEAD"])
        );
    }

    #[tokio::test]
    async fn a_branch_deleted_from_the_remote_can_be_published_again() {
        let (local, other, bare) = with_remote("git-sync-gone");
        git_in(&local, &["checkout", "-qb", "feat/x"]);
        commit_file(&local, "e.txt", "e\n");
        push(&s(&local)).await.unwrap();
        // Merged and deleted on the remote; the next (pruning) fetch notices.
        git_in(&other, &["push", "-q", "origin", "--delete", "feat/x"]);
        commit_file(&local, "f.txt", "f\n");
        fetch(&s(&local), true).await.unwrap();
        let st = status(&s(&local)).await.unwrap();
        assert!(st.upstream_gone);
        assert_eq!(
            fetch_summary(&st),
            "Fetch terminé : la branche distante origin/feat/x n'existe plus"
        );
        let err = pull(&s(&local)).await.unwrap_err().to_string();
        assert!(
            err.contains("La branche distante origin/feat/x n'existe plus"),
            "{err}"
        );
        assert_eq!(
            push(&s(&local)).await.unwrap(),
            "Branche feat/x publiée sur origin"
        );
        let st = status(&s(&local)).await.unwrap();
        assert!(!st.upstream_gone);
        assert_eq!(
            git_in(&bare, &["rev-parse", "feat/x"]),
            git_in(&local, &["rev-parse", "HEAD"])
        );
    }

    #[tokio::test]
    async fn a_branch_published_again_over_one_the_remote_moved_on_says_to_get_its_commits() {
        let (local, other, bare) = with_remote("git-push-branch-diverged");
        // A branch of the same name already on the remote, with commits of someone else's.
        git_in(&other, &["checkout", "-qb", "ticket/dem-1"]);
        commit_file(&other, "theirs.txt", "theirs\n");
        git_in(&other, &["push", "-qu", "origin", "ticket/dem-1"]);
        git_in(&local, &["checkout", "-qb", "ticket/dem-1"]);
        commit_file(&local, "mine.txt", "mine\n");
        let err = push_branch(&s(&local), "ticket/dem-1")
            .await
            .unwrap_err()
            .to_string();
        assert_eq!(
            err,
            "Le dépôt distant a des commits que tu n'as pas : récupère-les d'abord \
             (pull, ou rebase si les branches ont divergé)."
        );
        assert_eq!(
            git_in(&bare, &["rev-parse", "ticket/dem-1"]),
            git_in(&other, &["rev-parse", "HEAD"])
        );
        // A new one is published on origin, and tracks it.
        git_in(&local, &["checkout", "-qb", "ticket/dem-2"]);
        assert_eq!(
            push_branch(&s(&local), "ticket/dem-2").await.unwrap(),
            "origin"
        );
        assert_eq!(
            status(&s(&local)).await.unwrap().upstream.as_deref(),
            Some("origin/ticket/dem-2")
        );
    }

    #[tokio::test]
    async fn diverged_branches_are_neither_pushed_nor_pulled() {
        let (local, other, _) = with_remote("git-sync-diverged");
        commit_file(&other, "b.txt", "theirs\n");
        git_in(&other, &["push", "-q"]);
        commit_file(&local, "c.txt", "mine\n");
        let head = git_in(&local, &["rev-parse", "HEAD"]);

        let err = push(&s(&local)).await.unwrap_err().to_string();
        assert!(err.contains("récupère-les d'abord"), "{err}");
        let err = pull(&s(&local)).await.unwrap_err().to_string();
        assert!(
            err.contains("La branche locale et origin/main ont divergé"),
            "{err}"
        );
        assert_eq!(git_in(&local, &["rev-parse", "HEAD"]), head);
        let st = status(&s(&local)).await.unwrap();
        assert_eq!((st.ahead, st.behind), (1, 1));
    }

    /// Asks git for credentials the way a fetch does, with an askpass program configured (as
    /// `core.askPass`, or `SSH_ASKPASS` which Git Bash exports). True when it was run.
    async fn askpass_runs(name: &str, background: bool) -> bool {
        use tokio::io::AsyncWriteExt;
        let dir = crate::paths::test_dir(name);
        let marker = dir.join("asked");
        let script = dir.join("askpass.sh");
        let slash = |p: &Path| p.to_string_lossy().replace('\\', "/");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\necho asked > '{}'\necho secret\n",
                slash(&marker)
            ),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let config = format!("core.askPass={}", slash(&script));
        let mut cmd = command(
            &s(&dir),
            &[
                "-c",
                &config,
                "-c",
                "credential.helper=",
                "credential",
                "fill",
            ],
        );
        cmd.env_remove("GIT_ASKPASS")
            .env("SSH_ASKPASS", &script)
            .stdin(Stdio::piped());
        if background {
            never_prompt(&mut cmd);
        }
        let mut child = cmd.spawn().unwrap();
        let mut stdin = child.stdin.take().unwrap();
        stdin
            .write_all(b"protocol=https\nhost=example.invalid\n\n")
            .await
            .unwrap();
        drop(stdin);
        child.wait_with_output().await.unwrap();
        marker.exists()
    }

    #[tokio::test]
    async fn background_commands_never_run_an_askpass_program() {
        // The setup does detect a prompt...
        assert!(askpass_runs("git-askpass-user", false).await);
        // ...which a background command never shows.
        assert!(!askpass_runs("git-askpass-background", true).await);
    }

    #[tokio::test]
    async fn a_repository_without_remote_has_nothing_to_sync() {
        let r = repo("git-sync-none");
        assert!(remotes(&r).await.is_empty());
        let err = push(&r).await.unwrap_err().to_string();
        assert!(err.contains("Aucun dépôt distant"), "{err}");
        let err = fetch(&r, false).await.unwrap_err().to_string();
        assert!(err.contains("Aucun dépôt distant"), "{err}");
        let err = pull(&r).await.unwrap_err().to_string();
        assert!(err.contains("ne suit aucune branche distante"), "{err}");
    }

    /// The same folder, however git and Windows spell it (slashes, short names).
    fn same(a: &str, b: &Path) -> bool {
        Path::new(a).canonicalize().unwrap() == b.canonicalize().unwrap()
    }

    #[tokio::test]
    async fn branches_are_listed_local_then_remote_with_their_upstream_and_worktree() {
        let (local, other, _) = with_remote("git-g1-list");
        let l = s(&local);
        // feat: published, then one commit more of its own.
        git_in(&local, &["checkout", "-qb", "feat"]);
        commit_file(&local, "f.txt", "f\n");
        git_in(&local, &["push", "-qu", "origin", "feat"]);
        commit_file(&local, "g.txt", "g\n");
        git_in(&local, &["checkout", "-q", "main"]);
        // gone: published, then deleted on the remote.
        git_in(&local, &["branch", "gone"]);
        git_in(&local, &["push", "-qu", "origin", "gone"]);
        git_in(&other, &["push", "-q", "origin", "--delete", "gone"]);
        // main: one commit behind origin/main; a branch only the remote has.
        commit_file(&other, "b.txt", "b\n");
        git_in(&other, &["push", "-q"]);
        git_in(&other, &["push", "-q", "origin", "main:theirs"]);
        git_in(&local, &["fetch", "-q", "--prune"]);
        git_in(&local, &["remote", "set-head", "origin", "main"]);
        // One a worktree has checked out, one nothing has.
        let wt = local.parent().unwrap().join("wt");
        git_in(&local, &["worktree", "add", "-q", "-b", "agent", &s(&wt)]);
        git_in(&local, &["branch", "aaa"]);

        let list = branch_list(&l).await.unwrap();
        let names: Vec<&str> = list.iter().map(|b| b.name.as_str()).collect();
        // The current one first, the other local ones in order, then the remote ones (its HEAD
        // left out).
        assert_eq!(
            names,
            [
                "main",
                "aaa",
                "agent",
                "feat",
                "gone",
                "origin/feat",
                "origin/main",
                "origin/theirs"
            ]
        );
        let get = |n: &str| list.iter().find(|b| b.name == n).unwrap();
        let main = get("main");
        assert!(main.current && !main.remote);
        assert_eq!(main.upstream.as_deref(), Some("origin/main"));
        assert_eq!((main.ahead, main.behind, main.upstream_gone), (0, 1, false));
        assert!(same(main.worktree.as_deref().unwrap(), &local));
        let feat = get("feat");
        assert!(!feat.current && !feat.remote);
        assert_eq!(feat.upstream.as_deref(), Some("origin/feat"));
        assert_eq!((feat.ahead, feat.behind), (1, 0));
        assert_eq!(feat.worktree, None);
        let gone = get("gone");
        assert_eq!(gone.upstream.as_deref(), Some("origin/gone"));
        assert!(gone.upstream_gone);
        assert!(same(get("agent").worktree.as_deref().unwrap(), &wt));
        assert_eq!(get("aaa").upstream, None);
        // A remote branch tracked here is still listed, with the local branch that tracks it.
        let tracked = get("origin/feat");
        assert!(tracked.remote && !tracked.current);
        assert_eq!(tracked.tracked_by.as_deref(), Some("feat"));
        assert_eq!(
            (tracked.upstream.clone(), tracked.worktree.clone()),
            (None, None)
        );
        assert_eq!(get("origin/main").tracked_by.as_deref(), Some("main"));
        assert_eq!(get("origin/theirs").tracked_by, None);
        // Git never names an agent.
        assert!(list.iter().all(|b| b.agent.is_none()));
        // In ms, made just now.
        let now = crate::model::now_ms();
        assert!(
            list.iter()
                .all(|b| (now - b.last_commit_at).abs() < 3_600_000),
            "{list:?}"
        );
    }

    #[tokio::test]
    async fn a_detached_head_has_no_current_branch() {
        let r = repo("git-g1-list-detached");
        git(&r, &["switch", "-q", "--detach"]);
        let list = branch_list(&r).await.unwrap();
        assert_eq!(list.len(), 1);
        assert!(!list[0].current && list[0].name == "main");
        assert_eq!(list[0].worktree, None);
    }

    #[tokio::test]
    async fn a_branch_name_is_checked_the_way_git_does() {
        for ok in ["feat/x", "ticket/dem-1", "résumé", "fix_2.0"] {
            check_ref_format(ok)
                .await
                .unwrap_or_else(|e| panic!("{ok}: {e:#}"));
        }
        for bad in [
            "a..b", "-x", "x.lock", "a b", "x/", "HEAD", "@", "@{-1}", "a~1", "a:b", "*",
        ] {
            let e = check_ref_format(bad).await.expect_err(bad);
            assert!(e.to_string().contains(bad), "{bad}: {e}");
        }
        for empty in ["", "  "] {
            assert_eq!(
                check_ref_format(empty).await.unwrap_err().to_string(),
                "Donne un nom à la branche."
            );
        }
    }

    #[tokio::test]
    async fn a_branch_is_created_from_a_commit_or_from_another_branch() {
        let r = repo("git-g1-create");
        let first = text(&r, &["rev-parse", "HEAD"]).await.unwrap();
        git(&r, &["checkout", "-qb", "side"]);
        std::fs::write(Path::new(&r).join("side.txt"), "s\n").unwrap();
        git(&r, &["add", "-A"]);
        git(&r, &["commit", "-qm", "side"]);
        git(&r, &["checkout", "-q", "main"]);
        git(&r, &["commit", "-q", "--allow-empty", "-m", "second"]);
        let at = |b: &str| {
            let r = r.clone();
            let b = b.to_string();
            async move { text(&r, &["rev-parse", &b]).await.unwrap() }
        };

        branch_create(&r, "from-commit", &first[..7]).await.unwrap();
        assert_eq!(at("from-commit").await, first);
        branch_create(&r, "from-side", "side").await.unwrap();
        assert_eq!(at("from-side").await, at("side").await);
        // At HEAD when no start is given; the folder stays on its branch.
        branch_create(&r, "here", "").await.unwrap();
        assert_eq!(at("here").await, at("main").await);
        assert_eq!(head_branch(&r).await, "main");
        // Refused, nothing made or moved: a name taken, a name git refuses, a start that names
        // nothing or looks like an option.
        let side = at("side").await;
        let e = branch_create(&r, "side", "main").await.unwrap_err();
        assert!(e.to_string().contains("existe déjà"), "{e}");
        assert_eq!(at("side").await, side);
        assert!(branch_create(&r, "a..b", "main").await.is_err());
        for start in ["nowhere", "--orphan", "-f"] {
            let e = branch_create(&r, "lost", start).await.expect_err(start);
            assert!(e.to_string().contains(start), "{start}: {e}");
            assert!(!branch_exists(&r, "lost").await, "{start}");
        }
        assert_eq!(head_branch(&r).await, "main");
    }

    #[tokio::test]
    async fn a_stash_puts_the_changes_aside_and_leaves_the_untracked_files() {
        let r = repo("git-g1-stash");
        let root = Path::new(&r);
        git(&r, &["config", "core.autocrlf", "false"]);
        std::fs::write(root.join("résumé.md"), "changed\n").unwrap();
        std::fs::write(root.join("staged.txt"), "staged\n").unwrap();
        git(&r, &["add", "staged.txt"]);
        std::fs::write(root.join("notes.txt"), "mine\n").unwrap();

        let hash = stash_push(&r, "escouade: avant de passer sur side")
            .await
            .unwrap();
        assert_eq!(text(&r, &["rev-parse", "stash@{0}"]).await.unwrap(), hash);
        assert_eq!(
            text(&r, &["stash", "list", "--format=%gs"]).await.unwrap(),
            "On main: escouade: avant de passer sur side"
        );
        let kept = text(&r, &["stash", "show", "-p", "stash@{0}"])
            .await
            .unwrap();
        assert!(
            kept.contains("+changed") && kept.contains("+staged"),
            "{kept}"
        );
        assert_eq!(
            text(&r, &["status", "--porcelain"]).await.unwrap(),
            "?? notes.txt"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("résumé.md")).unwrap(),
            "a\n"
        );
        // Nothing left to put aside (the untracked file stays out): an error, no new stash.
        let e = stash_push(&r, "again").await.unwrap_err();
        assert!(e.to_string().contains("Rien à mettre de côté"), "{e}");
        assert_eq!(
            text(&r, &["stash", "list"]).await.unwrap().lines().count(),
            1
        );
    }

    #[tokio::test]
    async fn a_remote_branch_is_switched_to_through_the_local_branch_that_tracks_it() {
        let (local, other, _) = with_remote("git-g1-track");
        let l = s(&local);
        for b in ["feat/r", "other-name", "clash"] {
            git_in(&other, &["push", "-q", "origin", &format!("main:{b}")]);
        }
        git_in(&local, &["fetch", "-q"]);
        // None yet: one is made, which tracks it.
        assert_eq!(switch_to(&l, "origin/feat/r").await.unwrap(), "feat/r");
        assert_eq!(head_branch(&l).await, "feat/r");
        assert_eq!(
            status(&l).await.unwrap().upstream.as_deref(),
            Some("origin/feat/r")
        );
        // Then that one, whatever its name.
        assert_eq!(switch_to(&l, "main").await.unwrap(), "main");
        assert_eq!(switch_to(&l, "origin/feat/r").await.unwrap(), "feat/r");
        git_in(
            &local,
            &["branch", "-q", "--track", "mine", "origin/other-name"],
        );
        assert_eq!(switch_to(&l, "origin/other-name").await.unwrap(), "mine");
        assert!(!branch_exists(&l, "other-name").await);
        // A local branch of its name that does not track it: refused, nothing changes.
        git_in(&local, &["branch", "-q", "clash", "main"]);
        let e = switch_to(&l, "origin/clash").await.unwrap_err();
        assert!(e.to_string().contains("« clash »"), "{e}");
        // Neither a branch nor an option.
        assert!(switch_to(&l, "origin/nowhere").await.is_err());
        assert!(switch_to(&l, "--detach").await.is_err());
        assert_eq!(head_branch(&l).await, "mine");
        // A branch made from a remote one tracks nothing: it would push there.
        branch_create(&l, "copy", "origin/main").await.unwrap();
        let list = branch_list(&l).await.unwrap();
        let copy = list.iter().find(|b| b.name == "copy").unwrap();
        assert_eq!(copy.upstream, None);
    }

    #[tokio::test]
    async fn a_branch_is_deleted_once_merged_or_forced_and_on_its_remote() {
        let (local, other, bare) = with_remote("git-g1-delete");
        let l = s(&local);
        git_in(&local, &["branch", "merged"]);
        git_in(&local, &["checkout", "-qb", "open"]);
        commit_file(&local, "o.txt", "o\n");
        git_in(&local, &["checkout", "-q", "main"]);
        branch_delete(&l, "merged", false).await.unwrap();
        assert!(!branch_exists(&l, "merged").await);
        // Git's own check (merged into HEAD), unless forced.
        assert!(branch_delete(&l, "open", false).await.is_err());
        assert!(branch_exists(&l, "open").await);
        branch_delete(&l, "open", true).await.unwrap();
        assert!(!branch_exists(&l, "open").await);
        assert!(branch_delete(&l, "-D", true).await.is_err());

        let tracking = |b: &str| {
            let (l, b) = (l.clone(), format!("refs/remotes/origin/{b}"));
            async move {
                text(&l, &["rev-parse", "--verify", "--quiet", &b])
                    .await
                    .is_ok()
            }
        };
        // On the remote: the branch there, and its tracking branch here.
        git_in(&local, &["push", "-q", "origin", "main:pub"]);
        assert!(tracking("pub").await);
        branch_delete_remote(&l, "origin", "pub").await.unwrap();
        assert_eq!(git_in(&bare, &["branch", "--list", "pub"]), "");
        assert!(!tracking("pub").await);
        // Already deleted there by someone else: no error, the tracking branch goes.
        git_in(&local, &["push", "-q", "origin", "main:theirs"]);
        git_in(&other, &["push", "-q", "origin", "--delete", "theirs"]);
        assert!(tracking("theirs").await);
        branch_delete_remote(&l, "origin", "theirs").await.unwrap();
        assert!(!tracking("theirs").await);
        // Not a remote, not a branch name.
        assert!(branch_delete_remote(&l, "nowhere", "main").await.is_err());
        assert!(branch_delete_remote(&l, "origin", "-x").await.is_err());
        assert_eq!(
            git_in(&bare, &["branch", "--format=%(refname:short)"]),
            "main"
        );
    }

    #[tokio::test]
    async fn a_remote_branch_someone_pushed_to_since_the_last_fetch_is_not_deleted() {
        let (local, other, bare) = with_remote("git-g1f-lease");
        let l = s(&local);
        let tracking = |b: &str| {
            let (l, b) = (l.clone(), format!("refs/remotes/origin/{b}"));
            async move { ref_exists(&l, &b).await }
        };
        git_in(&local, &["push", "-q", "origin", "main:feat"]);
        git_in(&local, &["push", "-q", "origin", "main:calm"]);
        assert!(tracking("feat").await);
        // Someone else adds a commit to feat; here nothing is fetched yet.
        git_in(&other, &["fetch", "-q"]);
        git_in(&other, &["checkout", "-q", "-b", "feat", "origin/feat"]);
        commit_file(&other, "theirs.txt", "theirs\n");
        git_in(&other, &["push", "-q", "origin", "feat"]);
        let theirs = git_in(&bare, &["rev-parse", "feat"]);

        let e = branch_delete_remote(&l, "origin", "feat")
            .await
            .unwrap_err();
        assert_eq!(
            e.to_string(),
            "La branche distante a bougé : récupère d’abord (fetch)."
        );
        // Nothing went: their commit is on the remote, the tracking branch is as it was.
        assert_eq!(git_in(&bare, &["rev-parse", "feat"]), theirs);
        assert!(tracking("feat").await);
        // Fetched, the user has seen where it stands: it goes.
        git_in(&local, &["fetch", "-q"]);
        branch_delete_remote(&l, "origin", "feat").await.unwrap();
        assert_eq!(git_in(&bare, &["branch", "--list", "feat"]), "");
        assert!(!tracking("feat").await);
        // One nobody touched goes at once; one that was never fetched here cannot be checked.
        branch_delete_remote(&l, "origin", "calm").await.unwrap();
        assert_eq!(git_in(&bare, &["branch", "--list", "calm"]), "");
        let e = branch_delete_remote(&l, "origin", "never")
            .await
            .unwrap_err();
        assert!(e.to_string().contains("origin/never"), "{e}");
    }

    #[test]
    fn a_command_told_apart_by_its_text_runs_git_in_english() {
        let locale = |cmd: &tokio::process::Command| {
            cmd.as_std()
                .get_envs()
                .find(|(k, _)| *k == "LC_ALL")
                .and_then(|(_, v)| v.map(|v| v.to_string_lossy().to_string()))
        };
        assert_eq!(locale(&command("x", &["push"])), None);
        assert_eq!(
            locale(&in_english(command("x", &["push"]))).as_deref(),
            Some("C")
        );
    }

    #[test]
    fn a_remote_branch_that_moved_is_told_in_both_languages() {
        use crate::i18n::Lang::{En, Fr};
        assert_eq!(
            [remote_moved(Fr), remote_moved(En)],
            [
                "La branche distante a bougé : récupère d’abord (fetch).",
                "The remote branch has moved: fetch first."
            ]
        );
    }

    #[test]
    fn a_remote_branch_that_stays_is_told_in_both_languages() {
        use crate::i18n::Lang::{En, Fr};
        let all = [
            RemoteKept::Default,
            RemoteKept::Base,
            RemoteKept::OtherName,
            RemoteKept::TrackedBy("twin".into()),
        ];
        assert_eq!(
            all.clone().map(|k| k.text(En, "origin/main")),
            [
                "“origin/main” is the remote’s default branch: only the local branch is deleted.",
                "“origin/main” is the remote branch of the project’s base: only the local branch is deleted.",
                "The local branch tracks “origin/main”, which isn’t named like it: only the local branch is deleted.",
                "The branch “twin” also tracks “origin/main”: only the local branch is deleted."
            ]
        );
        assert_eq!(
            all[3].text(Fr, "origin/main"),
            "La branche « twin » suit aussi « origin/main » : seule la branche locale est supprimée."
        );
        assert_eq!(
            [&all[0], &all[1]].map(|k| k.refusal(En, "origin/main")),
            [
                "“origin/main” is the remote’s default branch: it isn’t deleted from here.",
                "“origin/main” is the remote branch of the project’s base: it isn’t deleted from here."
            ]
        );
    }

    #[tokio::test]
    async fn the_remote_copy_of_a_branch_is_an_upstream_of_its_name_that_nothing_else_needs() {
        let (local, _, _) = with_remote("git-g1f-copy");
        let l = s(&local);
        let track = |branch: &str, upstream: &str| {
            git_in(&local, &["branch", "-q", branch, "main"]);
            git_in(&local, &["branch", "-q", "-u", upstream, branch]);
        };
        git_in(&local, &["push", "-q", "origin", "main:own"]);
        git_in(&local, &["branch", "-q", "--track", "own", "origin/own"]);
        git_in(&local, &["branch", "-q", "plain", "main"]);
        track("renamed", "origin/own");
        let copy = |name: &str, bases: &[&str]| {
            let (l, name) = (l.clone(), name.to_string());
            let bases: Vec<String> = bases.iter().map(|b| b.to_string()).collect();
            async move { remote_copy(&l, &name, &bases).await.unwrap() }
        };
        let keep = |shown: &str, why| RemoteCopy::Keep {
            shown: shown.to_string(),
            why,
        };
        // No upstream: nothing. Another branch on the same copy keeps it for itself.
        assert_eq!(copy("plain", &[]).await, RemoteCopy::Absent);
        assert_eq!(
            copy("own", &[]).await,
            keep("origin/own", RemoteKept::TrackedBy("renamed".into()))
        );
        assert_eq!(
            copy("renamed", &[]).await,
            keep("origin/own", RemoteKept::OtherName)
        );
        git_in(&local, &["branch", "-qD", "renamed"]);
        assert_eq!(
            copy("own", &[]).await,
            RemoteCopy::Delete {
                remote: "origin".into(),
                branch: "own".into()
            }
        );
        // A base's upstream is kept; so is the remote's default branch (its HEAD).
        assert_eq!(
            copy("own", &["refs/remotes/origin/own"]).await,
            keep("origin/own", RemoteKept::Base)
        );
        git_in(&local, &["remote", "set-head", "origin", "own"]);
        assert_eq!(
            copy("own", &[]).await,
            keep("origin/own", RemoteKept::Default)
        );
        assert_eq!(
            copy("main", &[]).await,
            RemoteCopy::Delete {
                remote: "origin".into(),
                branch: "main".into()
            }
        );
        assert_eq!(RemoteCopy::Absent.note(crate::i18n::Lang::En), None);
    }

    #[tokio::test]
    async fn a_remote_branch_deleted_and_made_again_by_someone_else_is_not_taken_for_gone() {
        let (local, other, bare) = with_remote("git-g1f-lease-again");
        let l = s(&local);
        git_in(&local, &["push", "-q", "origin", "main:feat"]);
        // Deleted, then published again with another commit: not the branch the user saw.
        git_in(&other, &["push", "-q", "origin", "--delete", "feat"]);
        git_in(&other, &["checkout", "-q", "-b", "feat"]);
        commit_file(&other, "again.txt", "again\n");
        git_in(&other, &["push", "-q", "origin", "feat"]);
        let again = git_in(&bare, &["rev-parse", "feat"]);
        let e = branch_delete_remote(&l, "origin", "feat")
            .await
            .unwrap_err();
        assert!(e.to_string().contains("bougé"), "{e}");
        assert_eq!(git_in(&bare, &["rev-parse", "feat"]), again);
    }

    #[tokio::test]
    async fn the_branches_merged_into_a_base_include_the_squashed_ones() {
        let r = repo("git-g1-merged");
        git(&r, &["config", "core.autocrlf", "false"]);
        let root = Path::new(&r);
        git(&r, &["branch", "behind"]);
        let work = |b: &str| {
            git(&r, &["checkout", "-qb", b, "main"]);
            std::fs::write(root.join(format!("{b}.txt")), format!("{b}\n")).unwrap();
            git(&r, &["add", "-A"]);
            git(&r, &["commit", "-qm", b]);
            git(&r, &["checkout", "-q", "main"]);
        };
        work("merged");
        git(&r, &["merge", "-q", "--no-ff", "-m", "merge", "merged"]);
        work("squashed");
        git(&r, &["merge", "-q", "--squash", "squashed"]);
        git(&r, &["commit", "-qm", "squash"]);
        work("open");

        let all = ["behind", "merged", "squashed"];
        assert_eq!(merged_into(&r, "main").await.unwrap(), all);
        assert_eq!(merged_into(&r, "refs/heads/main").await.unwrap(), all);
        assert!(merged_into(&r, "behind").await.unwrap().is_empty());
        assert!(is_merged(&r, "refs/heads/squashed", "main").await);
        assert!(is_merged(&r, "refs/heads/behind", "main").await);
        assert!(!is_merged(&r, "refs/heads/open", "main").await);
        assert!(merged_into(&r, "nowhere").await.is_err());
        assert!(merged_into(&r, "--all").await.is_err());
    }

    #[tokio::test]
    async fn the_commits_no_other_branch_has_are_counted() {
        let (local, _, _) = with_remote("git-g1-unique");
        let l = s(&local);
        git_in(&local, &["checkout", "-qb", "feat"]);
        commit_file(&local, "f.txt", "f\n");
        commit_file(&local, "g.txt", "g\n");
        git_in(&local, &["checkout", "-q", "main"]);
        let count = |refs: &[&str]| {
            let l = l.clone();
            let refs: Vec<String> = refs.iter().map(|r| r.to_string()).collect();
            async move { unique_commits(&l, &refs).await }
        };
        assert_eq!(count(&["refs/heads/feat"]).await.unwrap(), 2);
        // origin/main has main's.
        assert_eq!(count(&["refs/heads/main"]).await.unwrap(), 0);
        // Another branch has them: none would be lost, unless it goes too.
        git_in(&local, &["branch", "copy", "feat"]);
        assert_eq!(count(&["refs/heads/feat"]).await.unwrap(), 0);
        assert_eq!(
            count(&["refs/heads/feat", "refs/heads/copy"])
                .await
                .unwrap(),
            2
        );
        git_in(&local, &["branch", "-qD", "copy"]);
        // Published: the remote branch has them, unless it goes too.
        git_in(&local, &["push", "-q", "origin", "feat"]);
        assert_eq!(count(&["refs/heads/feat"]).await.unwrap(), 0);
        assert_eq!(count(&["refs/remotes/origin/feat"]).await.unwrap(), 0);
        assert_eq!(
            count(&["refs/heads/feat", "refs/remotes/origin/feat"])
                .await
                .unwrap(),
            2
        );
        assert!(count(&["--all"]).await.is_err());
    }

    #[tokio::test]
    async fn two_refs_are_compared_the_way_git_diff_does() {
        let r = diverged("git-g1-diff-refs", false);
        // From main to feat: feat's file comes, main's own change goes back.
        let d = diff_refs(&r, "main", "feat").await.unwrap();
        assert!(d.contains("+++ b/b.txt") && d.contains("+feat"), "{d}");
        assert!(d.contains("-main") && d.contains("+a"), "{d}");
        let back = diff_refs(&r, "feat", "main").await.unwrap();
        assert!(back.contains("-feat") && back.contains("+main"), "{back}");
        let first = text(&r, &["rev-list", "--max-parents=0", "HEAD"])
            .await
            .unwrap();
        let from_first = diff_refs(&r, &first[..8], "main").await.unwrap();
        assert!(from_first.contains("+main"), "{from_first}");
        assert_eq!(diff_refs(&r, "main", "main").await.unwrap(), "");
        for bad in ["nowhere", "--output=x", "-R", ""] {
            assert!(diff_refs(&r, bad, "main").await.is_err(), "{bad}");
            assert!(diff_refs(&r, "main", bad).await.is_err(), "{bad}");
        }
        assert!(!Path::new(&r).join("x").exists());
    }

    #[tokio::test]
    async fn a_branch_is_found_by_its_name_never_by_a_revision_that_starts_with_it() {
        let r = repo("git-g1f-exact");
        git(&r, &["commit", "-q", "--allow-empty", "-m", "second"]);
        git(&r, &["branch", "side"]);
        assert!(branch_exists(&r, "main").await);
        assert!(branch_exists(&r, "side").await);
        for revision in ["main~1", "main^", "main^{commit}", "main@{0}", "side~1", ""] {
            assert!(!branch_exists(&r, revision).await, "{revision}");
            assert!(
                !ref_exists(&r, &format!("refs/heads/{revision}")).await,
                "{revision}"
            );
        }
        // So a revision is no branch to switch to: nothing moves.
        let e = switch_to(&r, "main~1").await.unwrap_err();
        assert!(e.to_string().contains("main~1"), "{e}");
        assert_eq!(head_branch(&r).await, "main");
        assert!(local_of(&r, "side^").await.is_err());
    }

    #[tokio::test]
    async fn a_new_branch_cannot_take_the_name_of_a_remote_one() {
        let (local, _, _) = with_remote("git-g1f-shadow");
        let l = s(&local);
        // `origin/feat` is how a branch of the remote reads: a local one of that name hides it.
        let e = branch_create(&l, "origin/feat", "main").await.unwrap_err();
        assert!(e.to_string().contains("origin"), "{e}");
        assert!(!branch_exists(&l, "origin/feat").await);
        let e = check_new_branch(&l, "origin/x").await.unwrap_err();
        assert!(e.to_string().contains("origin"), "{e}");
        // Names that only start like the remote's are fine.
        for ok in ["origin-feat", "originals/x"] {
            check_new_branch(&l, ok).await.unwrap();
        }
        branch_create(&l, "originals/x", "main").await.unwrap();
        // A name git refuses or a branch has: told as before.
        assert!(check_new_branch(&l, "a..b").await.is_err());
        let e = check_new_branch(&l, "originals/x").await.unwrap_err();
        assert!(e.to_string().contains("existe déjà"), "{e}");
        let e = check_new_branch(&l, "main~1").await;
        assert!(e.is_err(), "a revision is no name git takes");
    }
}
