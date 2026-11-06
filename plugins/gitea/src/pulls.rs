//! The client of Gitea and Forgejo (Codeberg is a Forgejo): pull requests, merge and auto-merge, combined statuses,
//! Actions (logs and reruns), reviews and their threads, on the API `/api/v1` of an instance.
//!
//! The two share their API for what Escouade asks, but for these (written from the swagger documents of Forgejo 16 and
//! of Gitea's main branch, to try for real: docs/TODO-2.2.md):
//!
//! | | Gitea | Forgejo |
//! |---|---|---|
//! | the merge's body | `do`, `merge_when_checks_succeed` | `Do`, `merge_when_checks_succeed` (Go reads both) |
//! | a pull request to open | `reviewers`, `labels` (ids) | `labels` (ids); reviewers asked after |
//! | the runs of a commit | `workflow_runs[]`: `run_number`, `status` and `conclusion` | `workflow_runs[]`: `index_in_repo`, `status` |
//! | the jobs of a run | `{ jobs: [...] }` with `conclusion` and dates | `[...]`, a status only |
//! | a job's log, a rerun | logs 1.25+, reruns 1.26+ | logs 12+, no rerun |
//! | answering and resolving a thread | 1.26+ | none |
//!
//! A draft is a title that starts with `WIP:` (neither has a draft field to write); its `draft` is read back. The
//! instance's version (`/version`: Forgejo's carries `+gitea-<version>`) is asked once per client, and what its
//! version lacks is `Unsupported`.

use anyhow::{anyhow, Result};
use escouade_plugin_kit::errors::Refused;
use escouade_plugin_kit::forge::*;
use escouade_plugin_kit::http::encode as enc;
use escouade_plugin_kit::http::redirect;
use escouade_plugin_kit::http::reqwest;
use escouade_plugin_kit::tr;
use parking_lot::Mutex;
use reqwest::Method;
use serde_json::{json, Value};
use std::collections::BTreeSet;

const NAME: &str = "Gitea";

/// Page size of the lists, and the pages followed at most.
const PAGE: usize = 50;
const MAX_PAGES: usize = 10;

/// The prefix that makes a pull request a draft.
const WIP: &str = "WIP: ";

/// The first version of Gitea whose API has the jobs and the logs of Actions (`/actions/runs/{run}/jobs`,
/// `/actions/jobs/{job}/logs`), and that of Forgejo. From their sources: to check on an instance.
const GITEA_ACTIONS: (u32, u32) = (1, 25);
const FORGEJO_ACTIONS: (u32, u32) = (12, 0);
/// Gitea's API to run jobs again (`/actions/runs/{run}/rerun-failed-jobs`); Forgejo's has none.
const GITEA_RERUN: (u32, u32) = (1, 26);
/// Gitea's API to answer in a thread and to resolve it; Forgejo's has none.
const GITEA_THREADS: (u32, u32) = (1, 26);

/// What an instance's `/version` says it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instance {
    pub forgejo: bool,
    pub version: (u32, u32),
}

impl Instance {
    /// `1.24.2` (Gitea), `11.0.3+gitea-1.22.0` (Forgejo, from its 7), `16.0.0-dev-798-568c49e1+gitea-1.22.0`.
    pub fn parse(version: &str) -> Self {
        let forgejo = version.contains("+gitea-");
        let mut nums = version
            .trim_start_matches('v')
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .map(|s| s.parse::<u32>().unwrap_or(0));
        Self {
            forgejo,
            version: (nums.next().unwrap_or(0), nums.next().unwrap_or(0)),
        }
    }

    /// The jobs and the logs of Actions are in its API.
    pub fn actions(&self) -> bool {
        self.version
            >= if self.forgejo {
                FORGEJO_ACTIONS
            } else {
                GITEA_ACTIONS
            }
    }

    pub fn rerun(&self) -> bool {
        !self.forgejo && self.version >= GITEA_RERUN
    }

    pub fn threads(&self) -> bool {
        !self.forgejo && self.version >= GITEA_THREADS
    }
}

pub struct Gitea {
    http: reqwest::Client,
    base: String,
    token: String,
    instance: Mutex<Option<Instance>>,
}

/// A run of Actions of a commit, with its jobs in the order the instance shows them.
struct Run {
    id: u64,
    number: u64,
    jobs: Vec<Job>,
}

struct Job {
    id: u64,
    state: Option<String>,
    started: Option<u64>,
    finished: Option<u64>,
}

impl Gitea {
    pub fn new(http: reqwest::Client, base: String, token: String) -> Self {
        Self {
            http,
            base: base.trim_end_matches('/').to_string(),
            token: token.trim().to_string(),
            instance: Mutex::new(None),
        }
    }

    /// The request of a call to `url`: with the token while it is the instance's (`own`), and `body`.
    fn request(
        &self,
        method: Method,
        url: &str,
        own: bool,
        body: Option<&Value>,
    ) -> reqwest::RequestBuilder {
        let mut rb = self.http.request(method, url);
        if own {
            rb = rb
                .header("Authorization", format!("token {}", self.token))
                .header("Accept", "application/json");
        }
        match body {
            Some(body) => rb.json(body),
            None => rb,
        }
    }

    /// A call of the API, wherever it is redirected (a renamed repository): its JSON.
    async fn send(&self, method: Method, path: &str, body: Option<&Value>) -> Result<Value> {
        let url = format!("{}{path}", self.base);
        let make = |method, url: &str, own, body| self.request(method, url, own, body);
        Ok(redirect::send_named(NAME, method, url, body, make).await?.0)
    }

    async fn get(&self, path: &str) -> Result<Value> {
        self.send(Method::GET, path, None).await
    }

    /// The text of a call (a job's log), whatever it holds.
    async fn text(&self, path: &str) -> Result<String> {
        let url = format!("{}{path}", self.base);
        let none: Option<&Value> = None;
        let reply = redirect::follow(
            NAME,
            Method::GET,
            url,
            none,
            |method, url, own, body| async move {
                let rb = self.request(method, &url, own, body);
                redirect::reply(NAME, rb.header("Accept", "text/plain")).await
            },
        )
        .await?;
        Ok(redirect::success(NAME, reply)?.text)
    }

    async fn write(&self, method: Method, path: &str, body: Value) -> Result<Value> {
        self.send(method, path, Some(&body)).await
    }

    /// Every item of a list, page after page. The list is the answer, or the array under `key`.
    async fn list(&self, path: &str, key: Option<&str>) -> Result<Vec<Value>> {
        let sep = if path.contains('?') { '&' } else { '?' };
        let mut out = Vec::new();
        for page in 1..=MAX_PAGES {
            let v = self
                .get(&format!("{path}{sep}page={page}&limit={PAGE}"))
                .await?;
            let items = key.map_or(&v, |k| &v[k]);
            let items = items.as_array().cloned().unwrap_or_default();
            let n = items.len();
            out.extend(items);
            if n < PAGE {
                break;
            }
        }
        Ok(out)
    }

    /// Which Gitea or Forgejo it is, and how recent: asked once.
    pub async fn instance(&self) -> Result<Instance> {
        if let Some(i) = *self.instance.lock() {
            return Ok(i);
        }
        let v = self.get("/version").await?;
        let i = Instance::parse(v["version"].as_str().unwrap_or(""));
        *self.instance.lock() = Some(i);
        Ok(i)
    }

    fn repo_path(repo: &RepoRef) -> String {
        format!("/repos/{}/{}", enc(&repo.namespace), enc(&repo.name))
    }

    async fn raw_pull(&self, repo: &RepoRef, number: u64) -> Result<Value> {
        self.get(&format!("{}/pulls/{number}", Self::repo_path(repo)))
            .await
    }

    /// What the reviews and the statuses add to a pull request: approvals, changes asked, the combined state of its
    /// last commit and, when `threads`, the comments on lines not resolved yet.
    async fn enrich(
        &self,
        repo: &RepoRef,
        pr: &mut PullRequest,
        sha: &str,
        threads: bool,
    ) -> Result<()> {
        let base = Self::repo_path(repo);
        let reviews = self
            .list(&format!("{base}/pulls/{}/reviews", pr.number), None)
            .await?;
        let last = last_words(&reviews);
        pr.approvals = last
            .iter()
            .filter(|r| r.state == ReviewState::Approved)
            .count() as u32;
        pr.changes_requested = last
            .iter()
            .any(|r| r.state == ReviewState::ChangesRequested);
        if threads {
            let mut open = 0;
            for r in reviews
                .iter()
                .filter(|r| r["comments_count"].as_u64() > Some(0) && r["state"] != "PENDING")
            {
                for c in self.review_comments(repo, pr.number, r).await? {
                    open += u32::from(!c.resolved);
                }
            }
            pr.open_threads = open;
        }
        if !sha.is_empty() {
            let s = self
                .get(&format!("{base}/commits/{}/status", enc(sha)))
                .await?;
            pr.checks = combined(&s);
        }
        Ok(())
    }

    async fn review_comments(
        &self,
        repo: &RepoRef,
        number: u64,
        review: &Value,
    ) -> Result<Vec<ReviewComment>> {
        let Some(id) = review["id"].as_u64() else {
            return Ok(Vec::new());
        };
        let list = self
            .get(&format!(
                "{}/pulls/{number}/reviews/{id}/comments",
                Self::repo_path(repo)
            ))
            .await?;
        Ok(list
            .as_array()
            .into_iter()
            .flatten()
            .map(|c| ReviewComment {
                id: c["id"].as_u64().map(|i| i.to_string()).unwrap_or_default(),
                author: c["user"]["login"].as_str().unwrap_or("?").to_string(),
                body: c["body"].as_str().unwrap_or("").to_string(),
                path: c["path"]
                    .as_str()
                    .filter(|p| !p.is_empty())
                    .map(str::to_string),
                line: [&c["position"], &c["original_position"]]
                    .iter()
                    .find_map(|l| l.as_u64().filter(|l| *l > 0))
                    .map(|l| l as u32),
                resolved: !c["resolver"].is_null(),
                created: instant(&c["created_at"]).unwrap_or(0),
                excerpt: c["diff_hunk"]
                    .as_str()
                    .filter(|h| !h.is_empty())
                    .map(str::to_string),
            })
            .collect())
    }

    /// The runs of Actions of a commit and their jobs. Empty when the instance has no Actions API.
    async fn runs(&self, repo: &RepoRef, sha: &str) -> Result<Vec<Run>> {
        if !self.instance().await?.actions() {
            return Ok(Vec::new());
        }
        let base = Self::repo_path(repo);
        let runs = self
            .list(
                &format!("{base}/actions/runs?head_sha={}", enc(sha)),
                Some("workflow_runs"),
            )
            .await?;
        let mut out = Vec::new();
        for r in &runs {
            let Some(id) = r["id"].as_u64() else { continue };
            let number = r["run_number"]
                .as_u64()
                .or_else(|| r["index_in_repo"].as_u64())
                .unwrap_or(0);
            let jobs = self.get(&format!("{base}/actions/runs/{id}/jobs")).await?;
            // Gitea wraps them (`{ jobs }`), Forgejo does not.
            let jobs = if jobs.is_array() {
                &jobs
            } else {
                &jobs["jobs"]
            };
            let mut jobs: Vec<Job> = jobs
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|j| {
                    Some(Job {
                        id: j["id"].as_u64()?,
                        state: j["conclusion"]
                            .as_str()
                            .filter(|c| !c.is_empty())
                            .or_else(|| j["status"].as_str())
                            .map(str::to_string),
                        started: instant(&j["started_at"]),
                        finished: instant(&j["completed_at"]),
                    })
                })
                .collect();
            jobs.sort_by_key(|j| j.id);
            out.push(Run { id, number, jobs });
        }
        Ok(out)
    }

    async fn check_list(&self, repo: &RepoRef, number: u64) -> Result<Vec<Check>> {
        let sha = self.head_sha(repo, number).await?;
        self.checks_at(repo, &sha).await
    }

    /// The last commit of a branch (ESC-63).
    async fn branch_sha(&self, repo: &RepoRef, branch: &str) -> Result<String> {
        let b = self
            .get(&format!(
                "{}/branches/{}",
                Self::repo_path(repo),
                enc(branch)
            ))
            .await?;
        b["commit"]["id"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| {
                anyhow!(tr!(
                    "{NAME} : branche {branch} sans commit",
                    "{NAME}: branch {branch} without commit"
                ))
            })
    }

    /// The statuses of a commit, with the Actions job each one points to.
    async fn checks_at(&self, repo: &RepoRef, sha: &str) -> Result<Vec<Check>> {
        let base = Self::repo_path(repo);
        let statuses = self
            .list(&format!("{base}/commits/{}/statuses", enc(sha)), None)
            .await?;
        // Actions post their statuses with the address of the job: `.../actions/runs/<number>/jobs/<index>`.
        let runs = self.runs(repo, sha).await.unwrap_or_default();
        // The statuses are listed newest first: a context counts once, by its latest.
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        for s in &statuses {
            let name = s["context"].as_str().unwrap_or("?").to_string();
            if !seen.insert(name.clone()) {
                continue;
            }
            let url = s["target_url"].as_str().filter(|u| !u.is_empty());
            let job = url
                .and_then(job_of_url)
                .and_then(|(run, index)| runs.iter().find(|r| r.number == run)?.jobs.get(index));
            out.push(Check {
                name,
                state: status_state(s["status"].as_str().or_else(|| s["state"].as_str())),
                url: url.map(str::to_string),
                job: job.map(|j| j.id.to_string()),
                started: job.and_then(|j| j.started),
                finished: job.and_then(|j| j.finished),
                cancelled: job.is_some_and(|j| j.state.as_deref() == Some("cancelled")),
            });
        }
        Ok(out)
    }

    /// The failed Actions jobs of the commit `sha` run again, once per run: how many runs.
    async fn rerun_at(&self, repo: &RepoRef, sha: &str) -> Result<usize> {
        let runs = self.runs(repo, sha).await?;
        let mut failed = BTreeSet::new();
        for c in self.checks_at(repo, sha).await? {
            let (CheckState::Failure, Some(job)) = (c.state, c.job) else {
                continue;
            };
            if let Some(r) = runs
                .iter()
                .find(|r| r.jobs.iter().any(|j| j.id.to_string() == job))
            {
                failed.insert(r.id);
            }
        }
        for run in &failed {
            self.write(
                Method::POST,
                &format!(
                    "{}/actions/runs/{run}/rerun-failed-jobs",
                    Self::repo_path(repo)
                ),
                json!({}),
            )
            .await?;
        }
        Ok(failed.len())
    }

    async fn head_sha(&self, repo: &RepoRef, number: u64) -> Result<String> {
        let pr = self.raw_pull(repo, number).await?;
        pr["head"]["sha"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| {
                anyhow!(tr!(
                    "{NAME} : PR sans commit",
                    "{NAME}: pull request without commit"
                ))
            })
    }

    /// A pull request, read whole: itself, its reviews and the statuses of its last commit.
    async fn whole(&self, repo: &RepoRef, number: u64) -> Result<PullRequest> {
        let raw = self.raw_pull(repo, number).await?;
        let mut pr = pr_of(&raw)?;
        let sha = raw["head"]["sha"].as_str().unwrap_or("").to_string();
        self.enrich(repo, &mut pr, &sha, true).await?;
        Ok(pr)
    }
}

/// The run and the index of the job an Actions status points to.
fn job_of_url(url: &str) -> Option<(u64, usize)> {
    let rest = url.split("/actions/runs/").nth(1)?;
    let mut parts = rest.split('/');
    let run = parts.next()?.parse().ok()?;
    if parts.next()? != "jobs" {
        return None;
    }
    let index = parts.next()?.split(['?', '#']).next()?.parse().ok()?;
    Some((run, index))
}

fn status_state(s: Option<&str>) -> CheckState {
    match s {
        Some("success" | "warning") => CheckState::Success,
        Some("failure" | "error") => CheckState::Failure,
        Some("skipped") => CheckState::Skipped,
        _ => CheckState::Pending,
    }
}

/// The combined status of a commit; none when no status was posted.
fn combined(s: &Value) -> Option<CheckState> {
    if s["total_count"].as_u64() == Some(0) || s["statuses"].as_array().is_some_and(Vec::is_empty) {
        return None;
    }
    Some(status_state(s["state"].as_str()))
}

/// The last word of each reviewer; a pending (unpublished) or dismissed review is none, a comment alone does not
/// take back an approval or a request for changes.
fn last_words(all: &[Value]) -> Vec<Review> {
    let mut last: Vec<Review> = Vec::new();
    for r in all {
        if r["dismissed"].as_bool() == Some(true) {
            continue;
        }
        let state = match r["state"].as_str() {
            Some("APPROVED") => ReviewState::Approved,
            Some("REQUEST_CHANGES" | "CHANGES_REQUESTED") => ReviewState::ChangesRequested,
            Some("COMMENT" | "COMMENTED") => ReviewState::Commented,
            _ => continue,
        };
        let author = r["user"]["login"].as_str().unwrap_or("?").to_string();
        let review = Review {
            author: author.clone(),
            state,
            body: r["body"].as_str().unwrap_or("").to_string(),
            created: instant(&r["submitted_at"]).unwrap_or(0),
        };
        match last.iter_mut().find(|x| x.author == author) {
            Some(prev) if state == ReviewState::Commented => {
                if prev.state == ReviewState::Commented {
                    *prev = review;
                }
            }
            Some(prev) => *prev = review,
            None => last.push(review),
        }
    }
    last
}

/// A pull request as the API tells it.
fn pr_of(p: &Value) -> Result<PullRequest> {
    let number = p["number"].as_u64().ok_or_else(|| {
        anyhow!(tr!(
            "{NAME} : réponse illisible",
            "{NAME}: unreadable answer"
        ))
    })?;
    let merged = p["merged"].as_bool().unwrap_or(false) || p["merged_at"].is_string();
    let state = if merged {
        PrState::Merged
    } else if p["state"] == "closed" {
        PrState::Closed
    } else {
        PrState::Open
    };
    let fork = match (p["head"]["repo_id"].as_u64(), p["base"]["repo_id"].as_u64()) {
        (Some(h), Some(b)) => h != b,
        _ => false,
    };
    Ok(PullRequest {
        number,
        url: p["html_url"].as_str().unwrap_or("").to_string(),
        title: p["title"].as_str().unwrap_or("").to_string(),
        state,
        draft: p["draft"].as_bool().unwrap_or(false),
        head: p["head"]["ref"].as_str().unwrap_or("").to_string(),
        base: p["base"]["ref"].as_str().unwrap_or("").to_string(),
        // Said for an open pull request only.
        mergeable: p["mergeable"].as_bool().filter(|_| state == PrState::Open),
        author: p["user"]["login"].as_str().unwrap_or("").to_string(),
        reviewers: p["requested_reviewers"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|r| r["login"].as_str())
            .map(str::to_string)
            .collect(),
        created: instant(&p["created_at"]).unwrap_or(0),
        fork,
        head_repo: p["head"]["repo"]["full_name"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        ..Default::default()
    })
}

/// Gitea's name of a strategy; `rebase` is a fast-forward after the rebase (GitHub's), `rebase-merge` adds the merge
/// commit (the semi-linear merge).
fn method_name(strategy: MergeStrategy) -> &'static str {
    match strategy {
        MergeStrategy::Merge => "merge",
        MergeStrategy::Squash => "squash",
        MergeStrategy::Rebase => "rebase",
        MergeStrategy::SemiLinear => "rebase-merge",
    }
}

/// What a repository's settings allow (`GET /repos/{owner}/{repo}`), and whether its pull request's target branch is
/// protected, where alone Gitea merges by itself once the checks pass (`merge_when_checks_succeed`).
pub fn merge_options_of(repo: &Value, protected: bool) -> MergeOptions {
    let keys = [
        ("allow_merge_commits", MergeStrategy::Merge),
        ("allow_squash_merge", MergeStrategy::Squash),
        ("allow_rebase", MergeStrategy::Rebase),
        ("allow_rebase_explicit", MergeStrategy::SemiLinear),
    ];
    MergeOptions {
        strategies: keys
            .iter()
            .filter(|(k, _)| repo[*k] != false)
            .map(|(_, s)| *s)
            .collect(),
        auto_merge: protected,
    }
}

impl Forge for Gitea {
    fn me(&self) -> BoxFuture<'_, Result<Identity>> {
        Box::pin(async move {
            let me = self.get("/user").await?;
            let login = me["login"].as_str().unwrap_or("?").to_string();
            Ok(Identity {
                label: format!("@{login}"),
                user: login,
                // Neither tells the rights of a token.
                scopes: None,
            })
        })
    }

    fn create_pr<'a>(
        &'a self,
        repo: &'a RepoRef,
        pr: &'a PullRequestDraft,
    ) -> BoxFuture<'a, Result<PullRequest>> {
        Box::pin(async move {
            let base = Self::repo_path(repo);
            let mut body = pr.body.clone();
            if let Some(n) = pr.closes {
                let line = format!("Closes #{n}");
                if !body.contains(&line) {
                    if !body.is_empty() {
                        body.push_str("\n\n");
                    }
                    body.push_str(&line);
                }
            }
            let title = if pr.draft && !pr.title.starts_with("WIP:") {
                format!("{WIP}{}", pr.title)
            } else {
                pr.title.clone()
            };
            // Labels are given by id; one that does not exist (or a list that cannot be read) is left out.
            let mut labels = Vec::new();
            if !pr.labels.is_empty() {
                for l in self
                    .list(&format!("{base}/labels"), None)
                    .await
                    .unwrap_or_default()
                {
                    if let (Some(id), Some(name)) = (l["id"].as_u64(), l["name"].as_str()) {
                        if pr.labels.iter().any(|w| w.eq_ignore_ascii_case(name)) {
                            labels.push(id);
                        }
                    }
                }
            }
            let made = self
                .write(
                    Method::POST,
                    &format!("{base}/pulls"),
                    json!({
                        "title": title,
                        "body": body,
                        "head": pr.head,
                        "base": pr.base,
                        "labels": labels,
                    }),
                )
                .await?;
            let created = pr_of(&made)?;
            // The pull request exists: a reviewer the instance refuses (the author, no access) does not take it back.
            if !pr.reviewers.is_empty() {
                let _ = self
                    .write(
                        Method::POST,
                        &format!("{base}/pulls/{}/requested_reviewers", created.number),
                        json!({ "reviewers": pr.reviewers }),
                    )
                    .await;
            }
            Ok(created)
        })
    }

    fn pull_request<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
    ) -> BoxFuture<'a, Result<PullRequest>> {
        Box::pin(async move { self.whole(repo, number).await })
    }

    fn pull_requests<'a>(
        &'a self,
        repo: &'a RepoRef,
        numbers: &'a [u64],
    ) -> BoxFuture<'a, Result<Vec<PullRequest>>> {
        Box::pin(async move {
            let mut out = Vec::new();
            for n in numbers {
                out.push(self.whole(repo, *n).await?);
            }
            Ok(out)
        })
    }

    fn pull_requests_of_branch<'a>(
        &'a self,
        repo: &'a RepoRef,
        branch: &'a str,
    ) -> BoxFuture<'a, Result<Vec<PullRequest>>> {
        Box::pin(async move {
            let all = self
                .get(&format!(
                    "{}/pulls?state=all&sort=recentupdate&limit={PAGE}",
                    Self::repo_path(repo)
                ))
                .await?;
            let mut prs: Vec<PullRequest> = all
                .as_array()
                .into_iter()
                .flatten()
                .filter(|p| p["head"]["ref"] == branch)
                .filter_map(|p| pr_of(p).ok())
                .collect();
            prs.sort_by_key(|p| std::cmp::Reverse(p.number));
            prs.truncate(10);
            Ok(prs)
        })
    }

    fn open_pull_requests<'a>(
        &'a self,
        repo: &'a RepoRef,
    ) -> BoxFuture<'a, Result<Vec<PullRequest>>> {
        Box::pin(async move {
            let all = self
                .get(&format!(
                    "{}/pulls?state=open&sort=recentupdate&limit={OPEN_PRS}",
                    Self::repo_path(repo)
                ))
                .await?;
            let mut out = Vec::new();
            for p in all.as_array().into_iter().flatten().take(OPEN_PRS) {
                let mut pr = pr_of(p)?;
                // Reviews and statuses, not the threads: that is a call for each review.
                self.enrich(
                    repo,
                    &mut pr,
                    p["head"]["sha"].as_str().unwrap_or(""),
                    false,
                )
                .await?;
                out.push(pr);
            }
            Ok(out)
        })
    }

    fn parent<'a>(&'a self, repo: &'a RepoRef) -> BoxFuture<'a, Result<Option<RepoRef>>> {
        Box::pin(async move {
            let r = self.get(&Self::repo_path(repo)).await?;
            let parent = &r["parent"];
            Ok(
                match (parent["owner"]["login"].as_str(), parent["name"].as_str()) {
                    (Some(owner), Some(name)) => Some(RepoRef {
                        namespace: owner.to_string(),
                        name: name.to_string(),
                    }),
                    _ => None,
                },
            )
        })
    }

    fn merge<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
        strategy: MergeStrategy,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            self.write(
                Method::POST,
                &format!("{}/pulls/{number}/merge", Self::repo_path(repo)),
                json!({ "Do": method_name(strategy) }),
            )
            .await?;
            Ok(())
        })
    }

    /// `merge_when_checks_succeed`: the instance only schedules it on a protected branch; the client asks the branch
    /// first, so that the refusal says why.
    fn merge_options<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
    ) -> BoxFuture<'a, Result<MergeOptions>> {
        Box::pin(async move {
            let base = Self::repo_path(repo);
            let settings = self.get(&base).await?;
            let pr = self.raw_pull(repo, number).await?;
            let branch = pr["base"]["ref"].as_str().unwrap_or("").to_string();
            let protected = self
                .get(&format!("{base}/branches/{}", enc(&branch)))
                .await
                .is_ok_and(|b| b["protected"] == true);
            Ok(merge_options_of(&settings, protected))
        })
    }

    fn auto_merge<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
        strategy: MergeStrategy,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let base = Self::repo_path(repo);
            let pr = self.raw_pull(repo, number).await?;
            let branch = pr["base"]["ref"].as_str().unwrap_or("").to_string();
            let b = self
                .get(&format!("{base}/branches/{}", enc(&branch)))
                .await?;
            if b["protected"].as_bool() != Some(true) {
                return Err(Refused(tr!(
                    "{NAME} : l'auto-merge n'existe que sur une branche protégée, et {branch} ne l'est pas",
                    "{NAME}: auto-merge exists only on a protected branch, and {branch} is not one"
                ))
                .into());
            }
            self.write(
                Method::POST,
                &format!("{base}/pulls/{number}/merge"),
                json!({ "Do": method_name(strategy), "merge_when_checks_succeed": true }),
            )
            .await?;
            Ok(())
        })
    }

    fn checks<'a>(&'a self, repo: &'a RepoRef, number: u64) -> BoxFuture<'a, Result<Vec<Check>>> {
        Box::pin(async move { self.check_list(repo, number).await })
    }

    fn job_log<'a>(&'a self, repo: &'a RepoRef, job: &'a str) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            if !self.instance().await?.actions() {
                return Err(Unsupported("job_log").into());
            }
            let path = format!("{}/actions/jobs/{}/logs", Self::repo_path(repo), enc(job));
            self.text(&path).await
        })
    }

    fn rerun_failed<'a>(&'a self, repo: &'a RepoRef, number: u64) -> BoxFuture<'a, Result<usize>> {
        Box::pin(async move {
            if !self.instance().await?.rerun() {
                return Err(Unsupported("rerun_failed").into());
            }
            let sha = self.head_sha(repo, number).await?;
            self.rerun_at(repo, &sha).await
        })
    }

    fn branch_checks<'a>(
        &'a self,
        repo: &'a RepoRef,
        branch: &'a str,
    ) -> BoxFuture<'a, Result<Vec<Check>>> {
        Box::pin(async move {
            let sha = self.branch_sha(repo, branch).await?;
            self.checks_at(repo, &sha).await
        })
    }

    fn rerun_failed_of_branch<'a>(
        &'a self,
        repo: &'a RepoRef,
        branch: &'a str,
    ) -> BoxFuture<'a, Result<usize>> {
        Box::pin(async move {
            if !self.instance().await?.rerun() {
                return Err(Unsupported("rerun_failed_of_branch").into());
            }
            let sha = self.branch_sha(repo, branch).await?;
            self.rerun_at(repo, &sha).await
        })
    }

    fn reviews<'a>(&'a self, repo: &'a RepoRef, number: u64) -> BoxFuture<'a, Result<Vec<Review>>> {
        Box::pin(async move {
            let all = self
                .list(
                    &format!("{}/pulls/{number}/reviews", Self::repo_path(repo)),
                    None,
                )
                .await?;
            Ok(last_words(&all))
        })
    }

    fn comments<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
    ) -> BoxFuture<'a, Result<Vec<ReviewComment>>> {
        Box::pin(async move {
            let base = Self::repo_path(repo);
            let reviews = self
                .list(&format!("{base}/pulls/{number}/reviews"), None)
                .await?;
            let mut out = Vec::new();
            for r in reviews
                .iter()
                .filter(|r| r["comments_count"].as_u64() > Some(0) && r["state"] != "PENDING")
            {
                out.extend(self.review_comments(repo, number, r).await?);
            }
            // The conversation under the description: no file, no line, no thread to resolve.
            for c in self
                .list(&format!("{base}/issues/{number}/comments"), None)
                .await?
            {
                out.push(ReviewComment {
                    id: c["id"].as_u64().map(|i| i.to_string()).unwrap_or_default(),
                    author: c["user"]["login"].as_str().unwrap_or("?").to_string(),
                    body: c["body"].as_str().unwrap_or("").to_string(),
                    path: None,
                    line: None,
                    resolved: false,
                    created: instant(&c["created_at"]).unwrap_or(0),
                    excerpt: None,
                });
            }
            Ok(out)
        })
    }

    fn comment<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
        body: &'a str,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            self.write(
                Method::POST,
                &format!("{}/issues/{number}/comments", Self::repo_path(repo)),
                json!({ "body": body }),
            )
            .await?;
            Ok(())
        })
    }

    fn review<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
        review: &'a ReviewDraft,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let event = match review.state {
                ReviewState::Approved => "APPROVED",
                ReviewState::ChangesRequested => "REQUEST_CHANGES",
                ReviewState::Commented => "COMMENT",
            };
            let lines: Vec<Value> = review
                .lines
                .iter()
                .map(|l| json!({ "path": l.path, "new_position": l.line, "body": l.body }))
                .collect();
            self.write(
                Method::POST,
                &format!("{}/pulls/{number}/reviews", Self::repo_path(repo)),
                json!({ "event": event, "body": review.body, "comments": lines }),
            )
            .await?;
            Ok(())
        })
    }

    fn reply<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
        comment: &'a str,
        body: &'a str,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            if !self.instance().await?.threads() {
                return Err(Unsupported("reply").into());
            }
            self.write(
                Method::POST,
                &format!(
                    "{}/pulls/{number}/comments/{}/replies",
                    Self::repo_path(repo),
                    enc(comment)
                ),
                json!({ "body": body }),
            )
            .await?;
            Ok(())
        })
    }

    fn resolve<'a>(
        &'a self,
        repo: &'a RepoRef,
        _number: u64,
        comment: &'a str,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            if !self.instance().await?.threads() {
                return Err(Unsupported("resolve").into());
            }
            self.write(
                Method::POST,
                &format!(
                    "{}/pulls/comments/{}/resolve",
                    Self::repo_path(repo),
                    enc(comment)
                ),
                json!({}),
            )
            .await?;
            Ok(())
        })
    }

    // `deployments`: neither Gitea nor Forgejo has an API for them (`Unsupported`, the trait's).
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_tells_which_it_is() {
        let g = Instance::parse("1.24.2");
        assert!(!g.forgejo && g.version == (1, 24) && !g.actions() && !g.rerun());
        let g = Instance::parse("1.26.0+rc1");
        assert!(!g.forgejo && g.actions() && g.rerun() && g.threads());
        let f = Instance::parse("11.0.3+gitea-1.22.0");
        assert!(f.forgejo && f.version == (11, 0) && !f.actions());
        let f = Instance::parse("16.0.0-dev-798-568c49e1+gitea-1.22.0");
        assert!(f.forgejo && f.actions() && !f.rerun() && !f.threads());
    }

    #[test]
    fn an_actions_status_points_to_its_job() {
        assert_eq!(
            job_of_url("https://git.acme.dev/ada/notes/actions/runs/12/jobs/1"),
            Some((12, 1))
        );
        assert_eq!(job_of_url("https://ci.acme.dev/builds/7"), None);
    }
}
