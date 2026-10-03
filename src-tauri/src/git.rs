//! Git integration through the `git` CLI (always present with Git for Windows).

use crate::model::{Commit, FileChange};
use anyhow::{bail, Result};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

fn command(cwd: &str, args: &[&str]) -> tokio::process::Command {
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
    let mut cmd = command(cwd, args);
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
            bail!(
                "git {} interrompu : toujours pas terminé après {} s",
                args.first().unwrap_or(&""),
                limit.as_secs()
            )
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
            format!("git {} a échoué", args.join(" "))
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

/// Unified diff of the given paths (all dirty files when empty), untracked files included.
pub async fn diff(root: &str, paths: &[String]) -> Result<String> {
    let st = status(root).await?;
    let wanted = |p: &str| paths.is_empty() || paths.iter().any(|x| x == p);
    let untracked: Vec<String> = st
        .entries
        .iter()
        .filter(|e| e.status == 'A' && wanted(&e.path))
        .map(|e| e.path.clone())
        .collect();
    let mut args: Vec<&str> = vec!["diff", "HEAD", "--no-color", "--no-ext-diff", "--"];
    for p in paths {
        args.push(p);
    }
    let mut out = match run(root, &args).await {
        Ok(o) => String::from_utf8_lossy(&o).to_string(),
        Err(_) => String::new(),
    };
    for path in untracked {
        if out.contains(&format!("+++ b/{path}")) {
            continue;
        }
        let full = Path::new(root).join(&path);
        let Ok(bytes) = std::fs::read(&full) else {
            continue;
        };
        if bytes.len() > 1_000_000 || bytes.contains(&0) {
            out.push_str(&format!("diff --git a/{path} b/{path}\nnew file\nBinary files /dev/null and b/{path} differ\n"));
            continue;
        }
        let content = String::from_utf8_lossy(&bytes);
        let lines: Vec<&str> = content.lines().collect();
        out.push_str(&format!("diff --git a/{path} b/{path}\nnew file mode 100644\n--- /dev/null\n+++ b/{path}\n@@ -0,0 +1,{} @@\n", lines.len()));
        for l in lines {
            out.push('+');
            out.push_str(l);
            out.push('\n');
        }
        if out.len() > 4_000_000 {
            break;
        }
    }
    Ok(out)
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
        bail!("« {path} » n'a pas de modification à annuler");
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
        bail!("commit invalide : {hash}");
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
    Ok(String::from_utf8_lossy(&out).into_owned())
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

/// Fallback file listing for folders that are not git repositories.
pub fn walk_files(root: &str, limit: usize) -> Vec<String> {
    const SKIP: &[&str] = &[
        ".git",
        "node_modules",
        "target",
        "dist",
        "build",
        ".venv",
        "__pycache__",
        ".next",
    ];
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
                if !SKIP.contains(&name.as_str()) {
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

pub async fn branch_exists(repo: &str, branch: &str) -> bool {
    run(
        repo,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ],
    )
    .await
    .is_ok()
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
        bail!("le dépôt n'a pas encore de commit : impossible de créer un worktree");
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
    run(repo, &["worktree", "add", "-b", &name, &path, base]).await?;
    Ok((path, name))
}

/// Removes a worktree and its branch, tolerating a worktree folder that is already gone.
pub async fn worktree_remove(repo: &str, path: &str, branch: &str) -> Result<()> {
    if run(repo, &["worktree", "remove", "--force", path])
        .await
        .is_err()
        && Path::new(path).exists()
    {
        std::fs::remove_dir_all(path).map_err(|e| anyhow::anyhow!("{path} : {e}"))?;
    }
    let _ = run(repo, &["worktree", "prune"]).await;
    if branch_exists(repo, branch).await {
        run(repo, &["branch", "-D", branch]).await?;
    }
    Ok(())
}

/// Commits of `branch` not yet in the current branch of `repo`.
pub async fn ahead_count(repo: &str, branch: &str) -> u32 {
    text(repo, &["rev-list", "--count", &format!("HEAD..{branch}")])
        .await
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
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

const NO_REMOTE: &str = "Aucun dépôt distant n'est configuré pour ce dépôt.";

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

/// "1 commit", "3 commits".
fn n_commits(n: u32) -> String {
    format!("{n} commit{}", plural(n))
}

fn plural(n: u32) -> &'static str {
    if n > 1 {
        "s"
    } else {
        ""
    }
}

/// The checked-out branch, which a detached HEAD does not have.
fn sync_branch(st: &Status) -> Result<&str> {
    if st.branch.is_empty() || st.branch == "(detached)" {
        bail!("HEAD détachée : aucune branche à synchroniser.");
    }
    Ok(&st.branch)
}

/// Fetches the remote the current branch syncs with (every remote when that is ambiguous).
pub async fn fetch(repo: &str, background: bool) -> Result<()> {
    let (st, remotes) = tokio::join!(status(repo), remotes(repo));
    if remotes.is_empty() {
        bail!(NO_REMOTE);
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
    match (&st.upstream, st.behind) {
        (None, _) => "Fetch terminé".into(),
        (Some(up), _) if st.upstream_gone => {
            format!("Fetch terminé : la branche distante {up} n'existe plus")
        }
        (Some(_), 0) => "Fetch terminé : déjà à jour".into(),
        (Some(_), n) => format!("Fetch terminé : {} à tirer", n_commits(n)),
    }
}

/// Fetches, then brings the upstream's new commits in, fast-forward only (`git pull --ff-only`).
pub async fn pull(repo: &str) -> Result<String> {
    let st = status(repo).await?;
    let branch = sync_branch(&st)?;
    let Some(upstream) = st.upstream.clone() else {
        bail!("La branche {branch} ne suit aucune branche distante : rien à tirer.");
    };
    fetch(repo, false).await?;
    let st = status(repo).await?;
    if st.upstream_gone {
        bail!("La branche distante {upstream} n'existe plus : rien à tirer.");
    }
    if st.behind == 0 {
        return Ok("Déjà à jour".into());
    }
    if st.ahead > 0 {
        bail!(
            "La branche locale et {upstream} ont divergé : pull impossible en avance rapide. \
             Rebase ou merge à faire à la main (ou demande à un agent)."
        );
    }
    if let Err(e) = run(repo, &["merge", "--ff-only", "--quiet", "@{upstream}"]).await {
        if e.to_string().contains("would be overwritten") {
            bail!("Des modifications non commitées seraient écrasées par le pull : commite-les ou mets-les de côté d'abord.");
        }
        return Err(e);
    }
    Ok(format!(
        "{} tiré{}",
        n_commits(st.behind),
        plural(st.behind)
    ))
}

/// Pushes the current branch. A branch without upstream, or whose upstream was deleted, is
/// published on the remote (the upstream's, else origin, else the only one) and tracks it.
pub async fn push(repo: &str) -> Result<String> {
    let st = status(repo).await?;
    let branch = sync_branch(&st)?;
    let rejected = |e: anyhow::Error| {
        let msg = e.to_string();
        if msg.contains("[rejected]") || msg.contains("fetch first") {
            anyhow::anyhow!(
                "Le dépôt distant a des commits que tu n'as pas : récupère-les d'abord \
                 (pull, ou rebase si les branches ont divergé)."
            )
        } else {
            e
        }
    };
    if st.upstream.is_some() && !st.upstream_gone {
        run_net(repo, &["push"], false).await.map_err(rejected)?;
        return Ok(match st.ahead {
            0 => "Rien à pousser".into(),
            n => format!("{} poussé{}", n_commits(n), plural(n)),
        });
    }
    let remotes = remotes(repo).await;
    let Some(remote) = sync_remote(&remotes, st.upstream.as_deref()) else {
        if remotes.is_empty() {
            bail!(NO_REMOTE);
        }
        bail!("Plusieurs dépôts distants et aucun ne s'appelle origin : publie la branche à la main (git push -u <dépôt> {branch}).");
    };
    run_net(repo, &["push", "-u", remote, branch], false)
        .await
        .map_err(rejected)?;
    Ok(format!("Branche {branch} publiée sur {remote}"))
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
    async fn branches_are_listed_the_current_one_first() {
        let r = repo("git-branches");
        git(&r, &["branch", "aaa"]);
        git(&r, &["branch", "zzz"]);
        assert_eq!(branches(&r).await.unwrap(), ["main", "aaa", "zzz"]);
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
}
