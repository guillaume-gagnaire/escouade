//! The pull requests linked to an agent: those it opens (`gh pr create` in a command of its
//! conversation, the `create_pull_request` tool of an MCP server), whose address is read in the
//! call's result, and those of its own branch, asked to GitHub with `gh` along with their state
//! (open, draft, merged, closed), their checks and their reviews.

use crate::core::Core;
use crate::git;
use crate::model::{now_ms, PullRequestLink};
use anyhow::{anyhow, bail, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Runtime;

/// How long `gh` may take to tell of pull requests.
const GH_LIMIT: Duration = Duration::from_secs(30);
/// What `gh` is asked of a pull request, but its checks.
const FIELDS: &str = "number,url,title,state,isDraft,createdAt,reviewDecision,headRepositoryOwner";
/// Its checks, asked with the rest: a token may not be allowed to read them (a fine-grained one
/// without the checks), and the rest is then asked alone.
const CHECKS: &str = "statusCheckRollup";
/// At most this many pull requests of a branch are asked for: the latest ones.
const BRANCH_LIMIT: &str = "20";
/// The least time between two readings of an agent's pull requests that nothing new calls for
/// (its card clicked again and again).
const EVERY: Duration = Duration::from_secs(20);

/// Where the reading of an agent's pull requests stands (`Core::refresh_pull_requests`): one at a
/// time.
#[derive(Default)]
pub struct Reading {
    running: bool,
    /// Asked meanwhile for something new (a pull request just opened): read once more.
    again: bool,
    last: Option<Instant>,
}

/// A command line that opens a pull request: one of its commands (what `;`, `&&`, `|`, a line
/// break, a substitution `$(…)` or backticks part) runs `gh pr create`, or `gh pr new`. A text
/// that holds one of those signs can be taken for a command (`-m "wip; gh pr create later"`):
/// only an address in the call's result links anything.
pub fn opens_pull_request(command: &str) -> bool {
    command
        .split(|c: char| {
            matches!(
                c,
                ';' | '&' | '|' | '\n' | '\r' | '(' | ')' | '`' | '{' | '}'
            )
        })
        .any(runs_pr_create)
}

/// `command` (one command, no operator) runs `gh pr create`: `gh`, by its name or its path (in
/// quotes when it holds spaces, as `C:\Program Files\GitHub CLI\gh.exe` does), after the
/// variables set for it and the words that run it (`GH_TOKEN=x`, `env`), then `pr create` past
/// its own options (`-R owner/repo`).
fn runs_pr_create(command: &str) -> bool {
    let mut rest = command.trim_start();
    loop {
        let (word, after) = first_word(rest);
        if word.is_empty() {
            return false;
        }
        let sets_variable = word.split_once('=').is_some_and(|(name, _)| {
            !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        });
        if sets_variable || matches!(word, "env" | "command" | "exec" | "time" | "nohup") {
            rest = after.trim_start();
            continue;
        }
        let program = word.rsplit(['/', '\\']).next().unwrap_or(word);
        if !matches!(program.to_ascii_lowercase().as_str(), "gh" | "gh.exe") {
            return false;
        }
        let words: Vec<&str> = after.split_whitespace().collect();
        return words
            .iter()
            .take(5)
            .position(|w| *w == "pr")
            .is_some_and(|at| matches!(words.get(at + 1), Some(&"create") | Some(&"new")));
    }
}

/// The first word of `command` and what follows it: up to a space, or between its quotes.
fn first_word(command: &str) -> (&str, &str) {
    match command.chars().next() {
        Some(quote @ ('"' | '\'')) => match command[1..].split_once(quote) {
            Some((word, after)) => (word, after),
            None => (&command[1..], ""),
        },
        _ => command
            .split_once(char::is_whitespace)
            .unwrap_or((command, "")),
    }
}

/// A tool that opens a pull request: the GitHub MCP server's, whatever the server is named
/// (`mcp__github__create_pull_request`).
pub fn is_create_tool(name: &str) -> bool {
    name.starts_with("mcp__") && name.ends_with("create_pull_request")
}

/// The pull request a call that opens one gave. A tool's JSON answer: its own page (`html_url`),
/// not one its description names. A command: the last address it printed (`gh pr create` prints
/// its pull request last, after what the commands run before it said of others), else the last
/// of all it wrote (`gh` names in its error the pull request a branch has already).
pub fn opened(stdout: Option<&str>, text: &str) -> Option<(u32, String)> {
    if let Ok(answer) = serde_json::from_str::<Value>(text.trim()) {
        if let Some(own) = answer["html_url"].as_str().and_then(address) {
            return Some(own);
        }
    }
    stdout.and_then(last).or_else(|| last(text))
}

/// The last pull request named in `text`: its number and its address,
/// `https://<host>/<owner>/<repo>/pull/<n>`, without what follows.
pub fn last(text: &str) -> Option<(u32, String)> {
    let mut found = None;
    let mut rest = text;
    while let Some(at) = rest.find("https://") {
        let from = &rest[at..];
        let end = from
            .find(|c: char| {
                c.is_whitespace()
                    || matches!(c, '"' | '\'' | '<' | '>' | ')' | ']' | '}' | ',' | '\\')
            })
            .unwrap_or(from.len());
        found = address(&from[..end]).or(found);
        rest = &from["https://".len()..];
    }
    found
}

/// `url` as a pull request's: its number and its address up to it.
fn address(url: &str) -> Option<(u32, String)> {
    let path = url.strip_prefix("https://")?;
    let mut parts = path.split('/');
    let (host, owner, repo) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next()? != "pull" || [host, owner, repo].iter().any(|p| p.is_empty()) {
        return None;
    }
    let digits: String = parts
        .next()?
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    let number: u32 = digits.parse().ok().filter(|n| *n > 0)?;
    Some((
        number,
        format!("https://{host}/{owner}/{repo}/pull/{number}"),
    ))
}

/// A pull request as `gh … --json` tells it (`FIELDS`, `CHECKS`), read now; None when it has no
/// address. The second value: when it was opened (ms since epoch), 0 when `gh` did not say.
pub fn parse(v: &Value, now: i64) -> Option<(PullRequestLink, i64)> {
    let (number, url) = address(v["url"].as_str()?)?;
    let state = match v["state"].as_str().unwrap_or("") {
        "MERGED" => "merged",
        "CLOSED" => "closed",
        "OPEN" if v["isDraft"] == true => "draft",
        "OPEN" => "open",
        _ => "",
    };
    // Of a pull request still to merge only: the rest is history.
    let open = matches!(state, "open" | "draft");
    let review = match v["reviewDecision"].as_str().unwrap_or("") {
        "APPROVED" if open => "approved",
        "CHANGES_REQUESTED" if open => "changes",
        "REVIEW_REQUIRED" if open => "required",
        _ => "",
    };
    let opened = v["createdAt"]
        .as_str()
        .and_then(|at| chrono::DateTime::parse_from_rfc3339(at).ok())
        .map_or(0, |at| at.timestamp_millis());
    Some((
        PullRequestLink {
            url,
            number,
            title: v["title"].as_str().unwrap_or("").trim().to_string(),
            state: state.into(),
            checks: if open {
                checks(&v["statusCheckRollup"]).into()
            } else {
                String::new()
            },
            review: review.into(),
            checked_at: now,
        },
        opened,
    ))
}

/// What a pull request's checks say as a whole (`statusCheckRollup`: check runs and commit
/// statuses): "failing" as soon as one failed, else "pending" while one runs or waits, else
/// "passing"; empty without any. A check run again is listed with its earlier runs: only the
/// latest of each (by its name and its workflow's) counts, as `gh pr checks` does.
fn checks(rollup: &Value) -> &'static str {
    let Some(all) = rollup.as_array().filter(|a| !a.is_empty()) else {
        return "";
    };
    let word = |c: &Value, key: &str| c[key].as_str().unwrap_or("").to_string();
    let mut latest: HashMap<(String, String), &Value> = HashMap::new();
    let mut counted: Vec<&Value> = Vec::new();
    for c in all {
        let name = c["name"].as_str().or(c["context"].as_str()).unwrap_or("");
        if name.is_empty() {
            counted.push(c);
            continue;
        }
        let run = latest
            .entry((word(c, "workflowName"), name.to_string()))
            .or_insert(c);
        // Dates written the same way: compared as written.
        if word(c, "startedAt") >= word(run, "startedAt") {
            *run = c;
        }
    }
    counted.extend(latest.into_values());
    let failed = |c: &Value| {
        matches!(
            word(c, "conclusion").as_str(),
            "FAILURE" | "TIMED_OUT" | "CANCELLED" | "ACTION_REQUIRED" | "STARTUP_FAILURE"
        ) || matches!(word(c, "state").as_str(), "FAILURE" | "ERROR")
    };
    // A check run says where it is with `status`, a commit status with `state`.
    let running = |c: &Value| match c["status"].as_str() {
        Some(status) => status != "COMPLETED",
        None => matches!(word(c, "state").as_str(), "PENDING" | "EXPECTED"),
    };
    if counted.iter().any(|c| failed(c)) {
        "failing"
    } else if counted.iter().any(|c| running(c)) {
        "pending"
    } else {
        "passing"
    }
}

/// `links` with `read` (what GitHub just told of some pull requests) in it: one linked already
/// takes its new state and keeps its place, a new one comes last. True when anything changed
/// beyond the time of the reading.
pub fn merge(links: &mut Vec<PullRequestLink>, read: Vec<PullRequestLink>) -> bool {
    let mut changed = false;
    for new in read {
        match links.iter_mut().find(|l| l.url == new.url) {
            Some(known) => {
                let same = PullRequestLink {
                    checked_at: known.checked_at,
                    ..new.clone()
                } == *known;
                changed |= !same;
                *known = new;
            }
            None => {
                links.push(new);
                changed = true;
            }
        }
    }
    changed
}

/// A pull request whose state GitHub may still change: one never read, or not merged (a closed
/// one can be opened again).
pub fn may_change(link: &PullRequestLink) -> bool {
    link.state != "merged"
}

/// The pull requests of the branch `branch` in the repository of `cwd`, as GitHub has them now:
/// those still open, and those opened since `since` (ms since epoch; the agent's creation: a
/// branch's name may have served before, or be a long-lived one). `owners`: who has that branch
/// among the repository's remotes (`pushed_to`); when known, a pull request from someone else's
/// fork, whose branch only bears the same name, is not the agent's.
pub async fn of_branch(
    gh: &Path,
    cwd: &str,
    branch: &str,
    owners: &[String],
    since: i64,
    now: i64,
) -> Result<Vec<PullRequestLink>> {
    // A name that is an option would be obeyed.
    if branch.is_empty() || branch.starts_with('-') {
        return Ok(Vec::new());
    }
    let args = [
        "pr",
        "list",
        "--head",
        branch,
        "--state",
        "all",
        "--limit",
        BRANCH_LIMIT,
    ];
    let out = ask(gh, cwd, &args).await?;
    let ours = |v: &Value| match v["headRepositoryOwner"]["login"].as_str() {
        Some(owner) if !owners.is_empty() => owners.contains(&owner.to_lowercase()),
        _ => true,
    };
    let mut found: Vec<PullRequestLink> = out
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter(|v| ours(v))
        .filter_map(|v| parse(v, now))
        .filter(|(link, opened)| {
            matches!(link.state.as_str(), "open" | "draft") || *opened >= since
        })
        .map(|(link, _)| link)
        .collect();
    found.sort_by_key(|l| l.number);
    Ok(found)
}

/// The pull request at `url`, as GitHub has it now, under the address it was asked at (the one
/// it is linked by: GitHub answers with another once its repository was renamed). `cwd`: any
/// folder that exists (the address names the repository).
pub async fn at(gh: &Path, cwd: &str, url: &str, now: i64) -> Result<PullRequestLink> {
    // Only what a call's result or `gh` gave is ever asked for.
    if address(url).is_none() {
        bail!("not a pull request: {url}");
    }
    let out = ask(gh, cwd, &["pr", "view", url]).await?;
    let (link, _) =
        parse(&out, now).ok_or_else(|| anyhow!("gh pr view: no pull request in its answer"))?;
    Ok(PullRequestLink {
        url: url.to_string(),
        ..link
    })
}

/// What `gh <args> --json …` answers: the pull requests with their checks, or without them when
/// `gh` cannot tell them with the rest.
async fn ask(gh: &Path, cwd: &str, args: &[&str]) -> Result<Value> {
    let with = |fields: &str| {
        let mut all: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        all.extend(["--json".to_string(), fields.to_string()]);
        all
    };
    let out = match run(gh, cwd, &with(&format!("{FIELDS},{CHECKS}"))).await {
        Ok(out) => out,
        Err(e) if checks_refused(&e.to_string()) => run(gh, cwd, &with(FIELDS)).await?,
        Err(e) => return Err(e),
    };
    Ok(serde_json::from_str(&out)?)
}

/// What `gh` answers when GitHub refuses the checks to its token (a fine-grained one without
/// them, an app's): « GraphQL: Resource not accessible by … (repository.pullRequest.
/// statusCheckRollup…) ». Any other failure (not signed in, no network, no such pull request)
/// would fail the same without them.
fn checks_refused(error: &str) -> bool {
    error.contains(CHECKS) || error.contains("Resource not accessible")
}

/// `gh <args>` in `cwd`, never asking anything: what it printed.
async fn run(gh: &Path, cwd: &str, args: &[String]) -> Result<String> {
    let mut cmd = tokio::process::Command::new(gh);
    cmd.args(args)
        .current_dir(cwd)
        .env("GH_PROMPT_DISABLED", "1")
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(crate::claude::CREATE_NO_WINDOW);
    crate::job::isolate(&mut cmd);
    let child = cmd.spawn()?;
    // Dropped when this returns: whatever it left running (a `.cmd`'s node…) ends with it.
    let _job = crate::job::Job::for_child(&child);
    // Not the end of its pipes: a process started at the same moment may hold them (`proc`).
    let out = tokio::time::timeout(GH_LIMIT, crate::proc::wait_output(child))
        .await
        .map_err(|_| {
            anyhow!(
                "gh {} still not done after {} s",
                args[..2.min(args.len())].join(" "),
                GH_LIMIT.as_secs()
            )
        })??;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Who has the branch `branch` of the repository of `cwd` among its remotes, as of the last push
/// or fetch here: the owner of each one's repository (lowercase), when its address tells. None
/// while no remote has it: it has no pull request then, and GitHub is not asked.
async fn pushed_to(cwd: &str, branch: &str) -> Option<Vec<String>> {
    let pattern = format!("refs/remotes/*/{branch}");
    let refs = git::text(cwd, &["for-each-ref", "--format=%(refname)", &pattern])
        .await
        .ok()?;
    let tail = format!("/{branch}");
    let remotes: Vec<&str> = refs
        .lines()
        .filter_map(|r| r.strip_prefix("refs/remotes/")?.strip_suffix(&tail))
        .collect();
    if remotes.is_empty() {
        return None;
    }
    let mut owners = Vec::new();
    for remote in remotes {
        let url = git::remote_url(cwd, remote).await;
        owners.extend(url.as_deref().and_then(owner_of));
    }
    Some(owners)
}

/// The owner of the repository at a remote's address, lowercase: `https://host/owner/repo`,
/// `ssh://git@host/owner/repo`, `git@host:owner/repo`. None for a folder.
fn owner_of(url: &str) -> Option<String> {
    let url = url.trim();
    let path = match url.split_once("://") {
        Some(("http" | "https" | "ssh" | "git", rest)) => rest.split_once('/')?.1,
        Some(_) => return None,
        None => {
            let (host, path) = url.split_once(':')?;
            // A path with a drive, or a folder whose name holds a colon.
            if host.len() < 2 || host.contains(['/', '\\']) {
                return None;
            }
            path
        }
    };
    let owner = path.trim_start_matches('/').split('/').next()?;
    (!owner.is_empty()).then(|| owner.to_lowercase())
}

impl<R: Runtime> Core<R> {
    /// `refresh_pull_requests`, in the background.
    pub(crate) fn ask_pull_requests(self: &Arc<Self>, id: &str, now: bool) {
        let (core, id) = (self.clone(), id.to_string());
        tauri::async_runtime::spawn(async move { core.refresh_pull_requests(&id, now).await });
    }

    /// Asks GitHub what became of the agent's pull requests, and which ones its branch has
    /// (`read_pull_requests`); the agent is sent to the window again when anything changed. One
    /// reading at a time per agent. `now`: something new calls for it (it just opened one); else
    /// it is skipped when they were read less than `EVERY` ago.
    pub async fn refresh_pull_requests(self: &Arc<Self>, id: &str, now: bool) {
        // Nothing is kept of an agent that is gone.
        if self.agent(id).is_err() {
            return;
        }
        {
            let mut reads = self.pr_reads.lock();
            let r = reads.entry(id.to_string()).or_default();
            if r.running {
                r.again |= now;
                return;
            }
            if !now && r.last.is_some_and(|last| last.elapsed() < EVERY) {
                return;
            }
            r.running = true;
        }
        loop {
            self.read_pull_requests(id).await;
            // Deleted meanwhile: what its deletion forgot before this ended goes now.
            let gone = self.agent(id).is_err();
            let mut reads = self.pr_reads.lock();
            if gone {
                reads.remove(id);
                return;
            }
            let Some(r) = reads.get_mut(id) else {
                return;
            };
            r.last = Some(Instant::now());
            if !std::mem::take(&mut r.again) {
                r.running = false;
                return;
            }
        }
    }

    /// Once, shortly after the app started: what became meanwhile of the pull requests of every
    /// agent that may have some (one linked that may still change, a branch of its own), one
    /// agent at a time. After that, an agent's are read when it is selected.
    pub(crate) async fn refresh_all_pull_requests(self: &Arc<Self>) {
        if self.gh_cli().is_none() {
            return;
        }
        let ids: Vec<String> = self
            .agents
            .read()
            .values()
            .filter_map(|h| {
                let rt = h.lock();
                let some =
                    rt.meta.worktree.is_some() || rt.meta.pull_requests.iter().any(may_change);
                (!rt.meta.archived && some).then(|| rt.meta.id.clone())
            })
            .collect();
        for id in ids {
            self.refresh_pull_requests(&id, false).await;
        }
    }

    /// Links to the agent the pull request the app just opened for its ticket, as `gh pr create`
    /// printed it (`last`), with the title it was given, open or a draft. GitHub is not asked: it
    /// was told this very moment.
    pub(crate) fn link_opened(
        self: &Arc<Self>,
        agent_id: &str,
        (number, url): (u32, String),
        title: &str,
        draft: bool,
    ) {
        let Ok(h) = self.agent(agent_id) else {
            return;
        };
        let opened = PullRequestLink {
            url,
            number,
            title: title.trim().to_string(),
            state: if draft { "draft" } else { "open" }.into(),
            checked_at: now_ms(),
            ..Default::default()
        };
        if merge(&mut h.lock().meta.pull_requests, vec![opened]) {
            self.request_save();
            self.emit_agent(&h);
        }
    }

    /// Reads with `gh` the pull requests of the agent's own branch (an agent with a worktree,
    /// once a remote has its branch), then, all at once, those linked to it that are not among
    /// them and may still change. Nothing without `gh`; what `gh` cannot tell (not signed in, no
    /// network) stays as it was last told.
    async fn read_pull_requests(self: &Arc<Self>, id: &str) {
        let Some(gh) = self.gh_cli() else {
            return;
        };
        let Ok(h) = self.agent(id) else {
            return;
        };
        let (cwd, worktree, known, since, project_id) = {
            let rt = h.lock();
            let m = &rt.meta;
            (
                m.cwd.clone(),
                m.worktree.clone(),
                m.pull_requests.clone(),
                m.created_at,
                m.project_id.clone(),
            )
        };
        let now = now_ms();
        let mut read = Vec::new();
        if let Some(wt) = worktree.filter(|wt| Path::new(&wt.path).is_dir()) {
            if let Some(owners) = pushed_to(&wt.path, &wt.branch).await {
                match of_branch(&gh, &wt.path, &wt.branch, &owners, since, now).await {
                    Ok(found) => read = found,
                    Err(e) => {
                        log::info!("agent {id}: pull requests of {} not read: {e:#}", wt.branch)
                    }
                }
            }
        }
        // Anywhere that is there: the address names the repository.
        let project = self.project(&project_id).map(|p| p.path).ok();
        let folder = [Some(cwd), project]
            .into_iter()
            .flatten()
            .find(|dir| Path::new(dir).is_dir());
        if let Some(folder) = folder {
            let asked: Vec<_> = known
                .iter()
                .filter(|l| may_change(l) && !read.iter().any(|r| r.url == l.url))
                .map(|link| {
                    let (gh, folder, url) = (gh.clone(), folder.clone(), link.url.clone());
                    let view = async move { at(&gh, &folder, &url, now).await };
                    (link.number, tauri::async_runtime::spawn(view))
                })
                .collect();
            for (number, view) in asked {
                match view.await {
                    Ok(Ok(pr)) => read.push(pr),
                    Ok(Err(e)) => log::info!("agent {id}: pull request #{number} not read: {e:#}"),
                    Err(e) => log::warn!("agent {id}: pull request #{number} not read: {e}"),
                }
            }
        }
        if read.is_empty() {
            return;
        }
        if merge(&mut h.lock().meta.pull_requests, read) {
            self.request_save();
            self.emit_agent(&h);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_command_that_runs_gh_pr_create_opens_a_pull_request_and_no_other_does() {
        for command in [
            "gh pr create --title \"Fix\" --body \"…\"",
            "gh pr new --fill",
            "cd web && gh pr create --fill",
            "git push&&gh pr create --fill",
            "git push -u origin HEAD\ngh  pr   create --draft",
            "GH_TOKEN=x gh -R acme/demo pr create --fill",
            "env GH_TOKEN=x gh pr create --fill",
            "/opt/homebrew/bin/gh pr create",
            "C:\\Tools\\gh.exe pr create --fill",
            // Its address kept by the shell, or printed by another command.
            "PR_URL=$(gh pr create --fill) && echo $PR_URL",
            "echo \"$(gh pr create --fill)\"",
            "url=`gh pr create --fill`",
            "(cd web; gh pr create --fill) | tee pr.txt",
            // Where the installer puts it under Windows: a path with spaces, in quotes.
            "& 'C:\\Program Files\\GitHub CLI\\gh.exe' pr create --fill",
            "\"C:\\Program Files\\GitHub CLI\\gh.exe\" pr create --fill",
            // A title that holds what parts commands comes after the command.
            "gh pr create --title \"Fix (login) & co; part 2\"",
        ] {
            assert!(opens_pull_request(command), "{command}");
        }
        for command in [
            "gh pr list",
            "gh pr view 12",
            "gh pr edit 9 --base main",
            "gh issue create --title pr",
            "echo gh pr created",
            "echo gh pr create",
            "git commit -m 'gh pr create later'",
            "git commit -m \"gh pr create later\"",
            "ghx pr create",
            "'gh pr create'",
            "",
        ] {
            assert!(!opens_pull_request(command), "{command}");
        }
        assert!(is_create_tool("mcp__github__create_pull_request"));
        assert!(is_create_tool(
            "mcp__plugin_github_github__create_pull_request"
        ));
        assert!(!is_create_tool("create_pull_request"));
        assert!(!is_create_tool("mcp__github__list_pull_requests"));
    }

    #[test]
    fn the_last_pull_request_of_a_text_is_found_whatever_surrounds_its_address() {
        let pr = |n: u32| Some((n, format!("https://github.com/acme/demo/pull/{n}")));
        // What `gh pr create` prints, its notes on stderr first.
        assert_eq!(
            last("Creating pull request for feat into main in acme/demo\n\nhttps://github.com/acme/demo/pull/12\n"),
            pr(12)
        );
        // In a sentence, with what follows the number left out.
        assert_eq!(
            last("a pull request for branch \"feat\" already exists:\nhttps://github.com/acme/demo/pull/3#issuecomment-1 (open)"),
            pr(3)
        );
        assert_eq!(
            last("see <https://github.com/acme/demo/pull/8/files>."),
            pr(8)
        );
        // Several: the last one, past an address that is none.
        assert_eq!(
            last("https://github.com/acme/demo/pull/9 then https://github.com/acme/demo/pull/14 (see https://github.com/acme/demo/issues/2)"),
            pr(14)
        );
        // Another host: GitHub Enterprise.
        assert_eq!(
            last("https://git.corp.example/team/app/pull/41"),
            Some((41, "https://git.corp.example/team/app/pull/41".into()))
        );
        for none in [
            "",
            "rien",
            "https://github.com/acme/demo/pulls",
            "https://github.com/acme/demo/pull/",
            "https://github.com/acme/demo/pull/abc",
            "https://github.com/acme/demo/pull/0",
            "https://github.com/acme/demo/issues/12",
            "https://github.com/acme/pull/12",
            "http://github.com/acme/demo/pull/12",
        ] {
            assert_eq!(last(none), None, "{none}");
        }
    }

    #[test]
    fn the_pull_request_a_call_opened_is_the_one_it_printed_last_or_its_tools_own() {
        let pr = |n: u32| Some((n, format!("https://github.com/acme/demo/pull/{n}")));
        // `gh pr comment 9 …; gh pr create`: the comment's address comes first, the new pull
        // request's last.
        let out = "https://github.com/acme/demo/pull/9#issuecomment-1\nhttps://github.com/acme/demo/pull/14\n";
        let text = format!("{out}\nCreating pull request for feat into main in acme/demo");
        assert_eq!(opened(Some(out), &text), pr(14));
        // What it printed counts before what it wrote as notes or errors.
        assert_eq!(
            opened(
                Some("https://github.com/acme/demo/pull/14\n"),
                "https://github.com/acme/demo/pull/14\n\nwarning: see https://github.com/acme/demo/pull/2"
            ),
            pr(14)
        );
        // Nothing printed: the pull request its error names (the branch has one already).
        assert_eq!(
            opened(
                Some(""),
                "a pull request for branch \"feat\" already exists:\nhttps://github.com/acme/demo/pull/3"
            ),
            pr(3)
        );
        assert_eq!(opened(None, "https://github.com/acme/demo/pull/5"), pr(5));
        // An MCP tool's JSON: its own page, not the API's address nor one its description names.
        assert_eq!(
            opened(
                None,
                r#"{"url":"https://api.github.com/repos/acme/demo/pulls/7","html_url":"https://github.com/acme/demo/pull/7","number":7,
                    "body":"Supersedes https://github.com/acme/demo/pull/6","_links":{"issue":{"href":"https://api.github.com/repos/acme/demo/issues/7"}}}"#
            ),
            pr(7)
        );
        // A JSON answer without it: as a text.
        assert_eq!(
            opened(None, r#"{"created":"https://github.com/acme/demo/pull/8"}"#),
            pr(8)
        );
        assert_eq!(opened(Some("done"), "done"), None);
    }

    #[test]
    fn only_a_refusal_of_the_checks_has_the_rest_asked_without_them() {
        assert!(checks_refused(
            "GraphQL: Resource not accessible by personal access token (repository.pullRequest.statusCheckRollup)"
        ));
        assert!(checks_refused(
            "GraphQL: Resource not accessible by integration (repository.pullRequests.nodes.0.statusCheckRollup.contexts)"
        ));
        for other in [
            "no pull requests found for https://github.com/acme/demo/pull/7",
            "To get started with GitHub CLI, please run:  gh auth login",
            "error connecting to api.github.com",
            "",
        ] {
            assert!(!checks_refused(other), "{other}");
        }
    }

    #[test]
    fn the_owner_of_a_remotes_repository_is_read_in_its_address() {
        for (url, owner) in [
            ("https://github.com/Acme/demo.git", Some("acme")),
            ("https://token@github.com/acme/demo", Some("acme")),
            ("git@github.com:acme/demo.git", Some("acme")),
            ("ssh://git@git.corp.example:2222/team/app.git", Some("team")),
            ("git://github.com/acme/demo", Some("acme")),
            // A folder, here or as an address: no owner.
            ("/srv/git/demo.git", None),
            ("C:\\git\\demo.git", None),
            ("file:///srv/git/demo.git", None),
            ("../demo.git", None),
            ("", None),
        ] {
            assert_eq!(owner_of(url).as_deref(), owner, "{url}");
        }
    }

    fn told(state: &str, draft: bool, review: &str, rollup: Value) -> Value {
        json!({
            "number": 12, "url": "https://github.com/acme/demo/pull/12", "title": " Fix the login ",
            "state": state, "isDraft": draft, "createdAt": "2026-10-13T10:00:00Z",
            "reviewDecision": review, "statusCheckRollup": rollup,
        })
    }

    #[test]
    fn what_gh_tells_of_a_pull_request_is_its_state_its_checks_and_its_reviews() {
        let (pr, opened) = parse(&told("OPEN", false, "APPROVED", json!([])), 5).unwrap();
        assert_eq!(
            pr,
            PullRequestLink {
                url: "https://github.com/acme/demo/pull/12".into(),
                number: 12,
                title: "Fix the login".into(),
                state: "open".into(),
                checks: String::new(),
                review: "approved".into(),
                checked_at: 5,
            }
        );
        assert_eq!(opened, 1_791_885_600_000);
        let state = |v: Value| parse(&v, 0).unwrap().0.state;
        assert_eq!(state(told("OPEN", true, "", json!([]))), "draft");
        assert_eq!(state(told("MERGED", false, "", json!([]))), "merged");
        assert_eq!(state(told("CLOSED", false, "", json!([]))), "closed");
        let review = |r: &str| {
            parse(&told("OPEN", false, r, json!([])), 0)
                .unwrap()
                .0
                .review
        };
        assert_eq!(review("CHANGES_REQUESTED"), "changes");
        assert_eq!(review("REVIEW_REQUIRED"), "required");
        assert_eq!(review(""), "");
        // Once merged or closed, its checks and its reviews are history.
        let merged = parse(
            &told(
                "MERGED",
                false,
                "APPROVED",
                json!([{ "status": "COMPLETED", "conclusion": "SUCCESS" }]),
            ),
            0,
        )
        .unwrap()
        .0;
        assert_eq!((merged.checks.as_str(), merged.review.as_str()), ("", ""));
        // Without an address, or with another's, it is none.
        assert_eq!(parse(&json!({ "number": 12 }), 0), None);
        assert_eq!(
            parse(
                &json!({ "url": "https://github.com/acme/demo/issues/1" }),
                0
            ),
            None
        );
    }

    #[test]
    fn checks_fail_as_soon_as_one_failed_and_run_while_one_is_not_done() {
        let of = |rollup: Value| parse(&told("OPEN", false, "", rollup), 0).unwrap().0.checks;
        let run = |status: &str, conclusion: &str| json!({ "__typename": "CheckRun", "status": status, "conclusion": conclusion });
        let status = |state: &str| json!({ "__typename": "StatusContext", "state": state });
        assert_eq!(of(json!([])), "");
        assert_eq!(of(Value::Null), "");
        assert_eq!(
            of(json!([
                run("COMPLETED", "SUCCESS"),
                run("COMPLETED", "SKIPPED"),
                status("SUCCESS")
            ])),
            "passing"
        );
        assert_eq!(of(json!([run("COMPLETED", "NEUTRAL")])), "passing");
        assert_eq!(
            of(json!([run("COMPLETED", "SUCCESS"), run("IN_PROGRESS", "")])),
            "pending"
        );
        assert_eq!(of(json!([run("QUEUED", "")])), "pending");
        assert_eq!(of(json!([status("PENDING")])), "pending");
        assert_eq!(
            of(json!([run("IN_PROGRESS", ""), run("COMPLETED", "FAILURE")])),
            "failing"
        );
        assert_eq!(of(json!([run("COMPLETED", "TIMED_OUT")])), "failing");
        assert_eq!(of(json!([status("ERROR"), status("SUCCESS")])), "failing");
    }

    #[test]
    fn a_check_run_again_counts_for_its_latest_run_only() {
        let of = |rollup: Value| parse(&told("OPEN", false, "", rollup), 0).unwrap().0.checks;
        let run = |name: &str, workflow: &str, at: &str, status: &str, conclusion: &str| {
            json!({ "__typename": "CheckRun", "name": name, "workflowName": workflow,
                "startedAt": at, "status": status, "conclusion": conclusion })
        };
        let status = |context: &str, at: &str, state: &str| json!({ "__typename": "StatusContext", "context": context, "startedAt": at, "state": state });
        // Failed, run again, green: green, whatever the order they are listed in.
        let failed = run("test", "CI", "2026-10-13T10:00:00Z", "COMPLETED", "FAILURE");
        let passed = run("test", "CI", "2026-10-13T10:30:00Z", "COMPLETED", "SUCCESS");
        assert_eq!(of(json!([failed.clone(), passed.clone()])), "passing");
        assert_eq!(of(json!([passed.clone(), failed.clone()])), "passing");
        // Run again and still running.
        let again = run("test", "CI", "2026-10-13T11:00:00Z", "IN_PROGRESS", "");
        assert_eq!(
            of(json!([failed.clone(), passed.clone(), again])),
            "pending"
        );
        // The same name in another workflow is another check: its failure stays.
        let other = run(
            "test",
            "Nightly",
            "2026-10-13T09:00:00Z",
            "COMPLETED",
            "FAILURE",
        );
        assert_eq!(
            of(json!([failed.clone(), passed.clone(), other])),
            "failing"
        );
        // A commit status set again.
        assert_eq!(
            of(json!([
                status("deploy", "2026-10-13T10:00:00Z", "ERROR"),
                status("deploy", "2026-10-13T10:05:00Z", "SUCCESS"),
                passed
            ])),
            "passing"
        );
    }

    #[test]
    fn a_reading_updates_the_pull_requests_linked_in_place_and_adds_the_new_ones_last() {
        let pr = |n: u32, state: &str, at: i64| PullRequestLink {
            url: format!("https://github.com/acme/demo/pull/{n}"),
            number: n,
            state: state.into(),
            checked_at: at,
            ..Default::default()
        };
        let mut links = vec![pr(12, "", 0), pr(9, "open", 1)];
        // Told for the first time, and one more of the branch.
        assert!(merge(
            &mut links,
            vec![pr(14, "draft", 2), pr(12, "open", 2)]
        ));
        assert_eq!(
            links,
            [pr(12, "open", 2), pr(9, "open", 1), pr(14, "draft", 2)]
        );
        // The same again: only the time of the reading moves.
        assert!(!merge(&mut links, vec![pr(12, "open", 3)]));
        assert_eq!(links[0].checked_at, 3);
        assert!(merge(&mut links, vec![pr(12, "merged", 4)]));
        assert_eq!(links[0], pr(12, "merged", 4));
        assert!(!merge(&mut links, Vec::new()));
        // Merged: never asked again. Closed, or never read: asked.
        assert!(!may_change(&links[0]));
        assert!(may_change(&pr(1, "closed", 1)) && may_change(&pr(1, "", 0)));
    }
}
