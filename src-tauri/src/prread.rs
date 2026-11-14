//! A pull request read whole (ESC-452, PR-1 of the PR in Escouade: `design/pr-2-3/README.md`): its
//! description, its commits, its files with their lines, and its threads with their lines, their
//! answers and their state. The views of PR-2 to PR-9 show it; the tools give it to an agent.
//!
//! - **The host** gives what it declares (`Capabilities::reading`, its plugin's `forge/description`,
//!   `forge/commits`, `forge/files`, `forge/threads`); a call its plugin does not serve
//!   (`unsupported`) leaves that part empty, without an error.
//! - **The diff is git's**: the tip and the base are fetched into `refs/escouade/review/<n>/read/…`
//!   (ESC-66's refs, a leaf of their own), removed at once. No branch moves, nothing is pushed.
//! - **Kept**: read again only once `FRESH` is past and the pull request changed: what the host
//!   says of it (its state, its checks, its reviews, its open threads), its tip's commit or when it
//!   last changed (`PrDescription`). The following of the pull requests (the project's list, an
//!   agent's, a ticket's) marks a kept one stale when it sees it change, and tells the window
//!   (`UiEvent::PrChanged`).
//! - **A third party's words**: every text of it was written by whoever can open a pull request
//!   on the host, maybe a stranger. `third_party` says it to the window, which shows it as text;
//!   the tools give it inside their mark (`mcp::prs::marked`); never an instruction.

use crate::core::Core;
use crate::forge::{
    FileStatus, PrCommit, PrDescription, PrFile, PullRequest, RateLimit, Reading, RepoRef,
    ReviewThread, Unsupported,
};
use crate::model::{now_ms, UiEvent};
use crate::prnews::limit_near;
use crate::prreview::generated;
use crate::prview::{failure, FailureKind, Read};
use crate::text::truncate;
use anyhow::{anyhow, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Runtime;

/// How long a pull request read whole is given again without asking the host.
pub const FRESH: Duration = Duration::from_secs(20);
/// The lines of the diff kept, of a file and of the whole pull request: past them, a file says it
/// is cut (`PrFileRead::cut`).
pub const FILE_LINES_MAX: usize = 3_000;
pub const LINES_MAX: usize = 30_000;
/// The longest line kept, in bytes.
const LINE_MAX: usize = 1_000;
/// The most pull requests kept read, the one read longest ago going first.
const KEPT_MAX: usize = 40;

/// One git reading at a time: two would share the refs of a pull request.
static GIT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// A line of a file's diff: `+` added, `-` removed, ` ` around them, `@` a hunk's head (its text
/// is the header, `@@ -1,3 +1,4 @@ fn x`). Its number in the base (`old`) and in the tip (`new`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrLine {
    pub kind: char,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new: Option<u32>,
    pub text: String,
}

/// A file of the pull request: what the host counts of it, and its lines as git gives them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrFileRead {
    #[serde(flatten)]
    pub file: PrFile,
    pub lines: Vec<PrLine>,
    /// git says it is binary: no line to show.
    pub binary: bool,
    /// A lockfile, a minified file: what nobody reads line by line, folded by the view.
    pub generated: bool,
    /// Lines past `FILE_LINES_MAX` (or `LINES_MAX` for the pull request) are left out.
    pub cut: bool,
}

/// A pull request read whole.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrContents {
    /// Always true: its texts (title, description, commits' messages, paths, threads, names) are
    /// a third party's, data to show, never instructions.
    pub third_party: bool,
    /// The host's word: « PR », « MR ».
    pub word: String,
    pub pr: PullRequest,
    /// None when the host does not read it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<PrDescription>,
    pub commits: Vec<PrCommit>,
    pub files: Vec<PrFileRead>,
    /// On lines, then the conversation's (`ReviewThread::on_lines`).
    pub threads: Vec<ReviewThread>,
    /// What the host reads: what is missing from the rest is not there, not empty.
    pub reading: Reading,
    /// The diff was asked: its lines are git's, or `diff_failure` says why there are none.
    pub diff: bool,
    /// The commit the lines are of.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub tip: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_failure: Option<String>,
    /// When it was read whole, and when the host last said it had not changed (ms).
    pub read_at: i64,
    pub checked_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimit>,
}

/// A pull request kept read, and what tells it changed.
#[derive(Debug, Clone)]
pub struct Kept {
    pub(crate) at: Instant,
    /// The following of the pull requests saw it change: read again at the next ask.
    pub(crate) stale: bool,
    tracked: String,
    moved: Option<(String, u64)>,
    host_repo: String,
    pub(crate) contents: PrContents,
}

/// What the following of the pull requests sees of one: when it differs, it changed.
pub fn tracked(pr: &PullRequest) -> String {
    serde_json::json!([
        pr.state,
        pr.draft,
        pr.title,
        pr.head,
        pr.base,
        pr.mergeable,
        pr.approvals,
        pr.changes_requested,
        pr.open_threads,
        pr.checks,
        pr.auto_merge,
        pr.reviewers,
        pr.merge_commit,
    ])
    .to_string()
}

fn key_of(project_id: &str, host_repo: &str, number: u64) -> String {
    format!("{project_id}|{}|{number}", host_repo.to_lowercase())
}

fn unsupported(e: &anyhow::Error) -> bool {
    e.downcast_ref::<Unsupported>().is_some()
}

/// A part the host does not serve: empty, without an error.
fn or_none<T>(read: Result<Vec<T>>) -> Result<Vec<T>> {
    match read {
        Err(e) if unsupported(&e) => Ok(Vec::new()),
        other => other,
    }
}

/// A read made once more when the host did not answer, or answered with a server's error.
macro_rules! again {
    ($read:expr) => {{
        match $read.await {
            Err(e) if failure(&e, None).kind == FailureKind::Unreachable => $read.await,
            other => other,
        }
    }};
}

/// The counts `@@ -a,b +c,d @@` start a hunk with: where each side starts and its lines.
fn hunk_of(line: &str) -> Option<(u32, u32, u32, u32)> {
    let mut parts = line.strip_prefix("@@ -")?.split(' ');
    let count = |s: &str| {
        let (start, n) = s.split_once(',').unwrap_or((s, "1"));
        Some((start.parse::<u32>().ok()?, n.parse::<u32>().ok()?))
    };
    let (old_start, old) = count(parts.next()?)?;
    let (new_start, new) = count(parts.next()?.strip_prefix('+')?)?;
    Some((old_start, old, new_start, new))
}

/// A file as git's diff tells it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GitFile {
    pub path: String,
    pub old_path: Option<String>,
    pub status: FileStatus,
    pub binary: bool,
    pub lines: Vec<PrLine>,
}

/// `git diff`'s text, file by file, with each line's number on both sides. The counts of each
/// hunk tell its lines from the headers, which a line of code may look like.
pub fn parse_files(text: &str) -> Vec<GitFile> {
    let mut files: Vec<GitFile> = Vec::new();
    let (mut old_left, mut new_left, mut old_next, mut new_next) = (0u32, 0u32, 0u32, 0u32);
    for line in text.lines() {
        if old_left > 0 || new_left > 0 {
            let Some(file) = files.last_mut() else {
                continue;
            };
            let (kind, rest) = match line.chars().next() {
                Some(k @ ('+' | '-' | ' ')) => (k, &line[1..]),
                Some('\\') => continue,
                None => (' ', ""),
                _ => continue,
            };
            let old = (kind != '+').then_some(old_next);
            let new = (kind != '-').then_some(new_next);
            if old.is_some() {
                old_next += 1;
                old_left = old_left.saturating_sub(1);
            }
            if new.is_some() {
                new_next += 1;
                new_left = new_left.saturating_sub(1);
            }
            file.lines.push(PrLine {
                kind,
                old,
                new,
                text: truncate(rest, LINE_MAX),
            });
            continue;
        }
        if let Some(rest) = line.strip_prefix("diff --git ") {
            let path = rest.rsplit_once(" b/").map_or(rest, |(_, p)| p);
            files.push(GitFile {
                path: path.to_string(),
                ..Default::default()
            });
            continue;
        }
        let Some(file) = files.last_mut() else {
            continue;
        };
        if let Some(path) = line.strip_prefix("+++ b/") {
            file.path = path.trim_end_matches('\t').to_string();
        } else if line.starts_with("+++ /dev/null") {
            file.status = FileStatus::Removed;
        } else if let Some(path) = line.strip_prefix("--- a/") {
            // Removed, its `+++` says /dev/null: its path is the base's.
            file.path = path.trim_end_matches('\t').to_string();
        } else if line.starts_with("new file mode") {
            file.status = FileStatus::Added;
        } else if line.starts_with("deleted file mode") {
            file.status = FileStatus::Removed;
        } else if let Some(from) = line.strip_prefix("rename from ") {
            file.status = FileStatus::Renamed;
            file.old_path = Some(from.to_string());
        } else if let Some(from) = line.strip_prefix("copy from ") {
            file.status = FileStatus::Copied;
            file.old_path = Some(from.to_string());
        } else if let Some(to) = line
            .strip_prefix("rename to ")
            .or_else(|| line.strip_prefix("copy to "))
        {
            file.path = to.to_string();
        } else if line.starts_with("Binary files ") || line == "GIT binary patch" {
            file.binary = true;
        } else if let Some((old_start, old, new_start, new)) = hunk_of(line) {
            file.lines.push(PrLine {
                kind: '@',
                old: None,
                new: None,
                text: truncate(line, LINE_MAX),
            });
            (old_left, new_left, old_next, new_next) = (old, new, old_start, new_start);
        }
    }
    files
}

/// The files as the host counts them, each with its lines from git; a file git has and the host
/// did not list (a host that reads no file, a list cut short) comes after them, counted from its
/// lines. Lines past the bounds are left out and said.
pub fn merged(host: Vec<PrFile>, git: Option<Vec<GitFile>>) -> Vec<PrFileRead> {
    let mut git = git.unwrap_or_default();
    let mut out: Vec<PrFileRead> = Vec::new();
    for file in host {
        let found = git.iter().position(|g| g.path == file.path);
        let g = found.map(|i| git.remove(i)).unwrap_or_default();
        out.push(PrFileRead {
            generated: generated(&file.path),
            file,
            lines: g.lines,
            binary: g.binary,
            cut: false,
        });
    }
    for g in git {
        let count = |k: char| g.lines.iter().filter(|l| l.kind == k).count() as u64;
        out.push(PrFileRead {
            file: PrFile {
                path: g.path.clone(),
                previous_path: g.old_path.clone(),
                status: g.status,
                additions: count('+'),
                deletions: count('-'),
            },
            generated: generated(&g.path),
            lines: g.lines,
            binary: g.binary,
            cut: false,
        });
    }
    let mut left = LINES_MAX;
    for f in &mut out {
        let max = FILE_LINES_MAX.min(left);
        if f.lines.len() > max {
            f.lines.truncate(max);
            f.cut = true;
        }
        left -= f.lines.len();
    }
    out
}

impl<R: Runtime> Core<R> {
    /// The pull request `number` of the project read whole (through the agent that holds it,
    /// else its ticket, else on the project's target, as `project_pr`), its lines with it when
    /// `diff`. Kept: given again while `FRESH`, and after it as long as the host says it did not
    /// change (two calls: the pull request and its description, conditional on GitHub).
    pub async fn pr_contents(
        self: &Arc<Self>,
        project_id: &str,
        number: u64,
        host_repo: &str,
        diff: bool,
    ) -> Read<PrContents> {
        let key = key_of(project_id, host_repo, number);
        let kept = self.pr_contents.lock().get(&key).cloned();
        let enough = |k: &Kept| !k.stale && (k.contents.diff || !diff);
        if let Some(k) = kept
            .as_ref()
            .filter(|k| enough(k) && k.at.elapsed() < FRESH)
        {
            return Read::ok(k.contents.clone());
        }
        let (client, repo, can) = match self.pr_place(project_id, number, host_repo).await {
            Ok(t) => t,
            Err(e) => return Read::failed(&e, None),
        };
        let reading = can.reading;
        let pr = match again!(client.pull_request(&repo, number)) {
            Ok(pr) => pr,
            Err(e) => return Read::failed(&e, client.rate_limit()),
        };
        let limit = client.rate_limit();
        if limit_near(limit, (now_ms() / 1000) as u64) {
            // The last of the host's limit: what was read stays, else nothing is asked.
            return match kept {
                Some(k) => Read::ok(k.contents),
                None => Read::limited(limit.map(|l| l.reset)),
            };
        }
        let description = match reading.description {
            true => match again!(client.description(&repo, number)) {
                Ok(d) => Some(d),
                Err(e) if unsupported(&e) => None,
                Err(e) => return Read::failed(&e, client.rate_limit()),
            },
            false => None,
        };
        let tracked = tracked(&pr);
        let moved = description
            .as_ref()
            .map(|d| (d.head_sha.clone(), d.updated));
        let same = |k: &Kept| enough(k) && k.tracked == tracked && k.moved == moved;
        if let Some(k) = kept.filter(same) {
            let mut contents = k.contents;
            contents.pr = pr;
            contents.description = description;
            contents.checked_at = now_ms();
            contents.rate_limit = client.rate_limit();
            self.pr_contents_keep(key, host_repo, tracked, moved, &contents);
            return Read::ok(contents);
        }
        let (commits, files, threads) = tokio::join!(
            async {
                match reading.commits {
                    true => or_none(again!(client.commits(&repo, number))),
                    false => Ok(Vec::new()),
                }
            },
            async {
                match reading.files {
                    true => or_none(again!(client.files(&repo, number))),
                    false => Ok(Vec::new()),
                }
            },
            async {
                match reading.threads {
                    true => or_none(again!(client.threads(&repo, number))),
                    false => Ok(Vec::new()),
                }
            },
        );
        let (commits, files, threads) = match (commits, files, threads) {
            (Ok(c), Ok(f), Ok(t)) => (c, f, t),
            (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
                return Read::failed(&e, client.rate_limit())
            }
        };
        let (lines, tip, diff_failure) = match diff {
            false => (None, String::new(), None),
            true => match self.pr_git_diff(project_id, &repo, &pr).await {
                Ok((sha, text)) => (Some(parse_files(&text)), sha, None),
                Err(e) => (None, String::new(), Some(format!("{e:#}"))),
            },
        };
        let word = self
            .project_forge(project_id)
            .await
            .and_then(|r| r.kind)
            .and_then(|k| self.forges.read().get(&k))
            .map_or("PR", |p| p.request_word())
            .to_string();
        let now = now_ms();
        let contents = PrContents {
            third_party: true,
            word,
            pr,
            description,
            commits,
            files: merged(files, lines),
            threads,
            reading,
            diff,
            tip,
            diff_failure,
            read_at: now,
            checked_at: now,
            rate_limit: client.rate_limit(),
        };
        self.pr_contents_keep(key, host_repo, tracked, moved, &contents);
        Read::ok(contents)
    }

    fn pr_contents_keep(
        &self,
        key: String,
        host_repo: &str,
        tracked: String,
        moved: Option<(String, u64)>,
        contents: &PrContents,
    ) {
        let mut kept = self.pr_contents.lock();
        kept.insert(
            key,
            Kept {
                at: Instant::now(),
                stale: false,
                tracked,
                moved,
                host_repo: host_repo.to_string(),
                contents: contents.clone(),
            },
        );
        while kept.len() > KEPT_MAX {
            let Some(oldest) = kept
                .iter()
                .min_by_key(|(_, k)| k.at)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            kept.remove(&oldest);
        }
    }

    /// The diff of `pr` from git, on the project's remote: none for a pull request of another
    /// repository (an agent's upstream), whose refs the project does not fetch.
    async fn pr_git_diff(
        self: &Arc<Self>,
        project_id: &str,
        repo: &RepoRef,
        pr: &PullRequest,
    ) -> Result<(String, String)> {
        let remote = self.project_forge(project_id).await.ok_or_else(|| {
            anyhow!(tr!(
                "Ce projet n'a pas de dépôt distant chez un hébergeur de code.",
                "This project has no remote on a code host."
            ))
        })?;
        let full = repo.full();
        if !remote.repo.full().eq_ignore_ascii_case(&full) {
            anyhow::bail!(tr!(
                "Les lignes de {full} ne se lisent pas depuis ce projet.",
                "The lines of {full} aren’t read from this project."
            ));
        }
        let provider = remote
            .kind
            .as_deref()
            .and_then(|k| self.forges.read().get(k))
            .ok_or_else(|| anyhow!("unknown code host"))?;
        let project = self.project(project_id)?;
        let root = self
            .toplevel(&project.path)
            .await
            .ok_or_else(crate::core::not_a_repo)?;
        let name = crate::forge::connect::project_remote_name(&project.path, &project.board.target)
            .await
            .ok_or_else(|| anyhow!("no remote"))?;
        let plan = crate::pragent::fetch_plan(
            pr,
            &name,
            &remote.url,
            provider.pull_fetch(pr.number),
            provider.request_word(),
            false,
        )?;
        let _one = GIT.lock().await;
        crate::prreview::git_diff(&root, &plan, &name, pr, "read/").await
    }

    /// What the following of the pull requests read of the project's (its list, an agent's, a
    /// ticket's, `repo` when it names their repository): a kept one that changed is read again at
    /// the next ask, and the window is told.
    pub(crate) fn pr_contents_seen(
        &self,
        project_id: &str,
        prs: &[PullRequest],
        repo: Option<&str>,
    ) {
        let prefix = format!("{project_id}|");
        let mut changed = Vec::new();
        {
            let mut kept: parking_lot::MutexGuard<'_, HashMap<String, Kept>> =
                self.pr_contents.lock();
            for (key, k) in kept.iter_mut() {
                if !key.starts_with(&prefix) || k.stale {
                    continue;
                }
                let found = prs.iter().find(|p| {
                    let theirs = repo.unwrap_or(&p.host_repo);
                    p.number == k.contents.pr.number
                        && (k.host_repo.is_empty() || k.host_repo.eq_ignore_ascii_case(theirs))
                });
                if let Some(pr) = found {
                    if tracked(pr) != k.tracked {
                        k.stale = true;
                        changed.push(pr.number);
                    }
                }
            }
        }
        for number in changed {
            self.hub.emit(UiEvent::PrChanged {
                project_id: project_id.to_string(),
                number,
            });
        }
    }
}
