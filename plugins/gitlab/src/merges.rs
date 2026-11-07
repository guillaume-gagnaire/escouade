//! GitLab's client: merge requests (a draft by `Draft:`, « Closes #n », reviewers and labels; state, approvals,
//! discussions, conflict), merge and auto-merge, pipelines and jobs (a job's log by `/jobs/:id/trace`, a pipeline's
//! failed jobs run again), discussions and their answers, and the deployments of a branch, on gitlab.com and the
//! self-managed instances (the base of its API comes from the account: `<instance>/api/v4`). A project is named by its
//! path, groups and subgroups included (`gitlab-org/api/client-go`), encoded as one segment.
//!
//! Written from GitLab's documentation and sources, and tried against answers recorded from gitlab.com
//! (`tests/fixtures/forges/gitlab/`): not yet against a real instance with a token (docs/TODO-2.2.md).
//!
//! It spares the API's limits: a merge request's open threads are only read when GitLab says some are not resolved
//! (`blocking_discussions_resolved`), a fork's path is read once, and the client slows down as the limit comes near
//! (`RateLimit-*`), and waits what a 429 asks (`Retry-After`) before trying once more.

use anyhow::{anyhow, Result};
use escouade_plugin_kit::errors::HttpError;
use escouade_plugin_kit::forge::*;
use escouade_plugin_kit::http::encode;
use escouade_plugin_kit::http::redirect::{self, Reply};
use escouade_plugin_kit::http::reqwest;
use escouade_plugin_kit::tr;
use parking_lot::Mutex;
use reqwest::header::HeaderMap;
use reqwest::Method;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const NAME: &str = "GitLab";

/// Pages followed at most for a list.
const MAX_PAGES: usize = 10;

/// How long a 429 is waited for when GitLab does not say (`Retry-After`), at most `Pace::max_wait`.
const LIMITED_WAIT: Duration = Duration::from_secs(60);

/// The deployments read to find those of a branch: GitLab does not filter them by ref.
const DEPLOYMENTS_READ: usize = 50;

/// What a call answered.
pub struct Gitlab {
    http: reqwest::Client,
    /// `https://gitlab.com/api/v4`.
    base: String,
    token: String,
    /// The last limit GitLab said.
    limit: Mutex<Option<RateLimit>>,
    /// The paths of the projects read by their id (a fork's), kept.
    projects: Mutex<HashMap<u64, String>>,
    /// The ids of the users named as reviewers, kept.
    users: Mutex<HashMap<String, u64>>,
    pace: Pace,
    /// Milliseconds spent slowing down.
    waited: AtomicU64,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn header<'a>(h: &'a HeaderMap, name: &str) -> Option<&'a str> {
    h.get(name).and_then(|v| v.to_str().ok())
}

fn unreadable() -> anyhow::Error {
    anyhow!(tr!(
        "{NAME} : réponse illisible",
        "{NAME}: unreadable answer"
    ))
}

fn not_found(number: u64) -> anyhow::Error {
    anyhow!(tr!(
        "{NAME} : MR !{number} introuvable",
        "{NAME}: merge request !{number} not found"
    ))
}

/// The address of the next page in a `Link` header.
fn next_link(link: &str) -> Option<String> {
    link.split(',').find_map(|part| {
        let (url, rel) = part.split_once(';')?;
        rel.contains("rel=\"next\"").then(|| {
            url.trim()
                .trim_start_matches('<')
                .trim_end_matches('>')
                .to_string()
        })
    })
}

/// A project's path in an address: groups, subgroups and project as one segment.
fn project(repo: &RepoRef) -> String {
    format!("/projects/{}", encode(&repo.full()))
}

/// A title without the draft's mark GitLab keeps in it (`Draft:`, `[Draft]`, `(Draft)`).
pub fn bare_title(title: &str) -> &str {
    let t = title.trim_start();
    for mark in ["Draft:", "[Draft]", "(Draft)", "draft:", "DRAFT:"] {
        if let Some(rest) = t.strip_prefix(mark) {
            return rest.trim_start();
        }
    }
    title
}

/// A pipeline's status in one word.
pub fn pipeline_state(status: &str) -> CheckState {
    match status {
        "success" => CheckState::Success,
        "failed" | "canceled" | "canceling" => CheckState::Failure,
        "skipped" => CheckState::Skipped,
        _ => CheckState::Pending,
    }
}

/// A job's state: one that may fail does not fail the pipeline (GitLab's warning); a manual one waits for nobody unless
/// it blocks the pipeline.
pub fn job_state(job: &Value) -> CheckState {
    let may_fail = job["allow_failure"].as_bool().unwrap_or(false);
    match job["status"].as_str().unwrap_or("") {
        "success" => CheckState::Success,
        "failed" if may_fail => CheckState::Success,
        "failed" | "canceled" | "canceling" => CheckState::Failure,
        "skipped" => CheckState::Skipped,
        "manual" if may_fail => CheckState::Skipped,
        _ => CheckState::Pending,
    }
}

/// A deployment's status in the words of the window (GitHub's).
pub fn deployment_state(status: &str) -> &str {
    match status {
        "success" => "success",
        "failed" => "failure",
        "running" => "in_progress",
        "created" => "queued",
        "blocked" => "pending",
        "canceled" | "skipped" => "inactive",
        other => other,
    }
}

/// What a deployment tells besides its environment, its state and its address: its commit, its date,
/// who made it and its job.
fn deployment_facts(d: &Value) -> Deployment {
    Deployment {
        sha: d["sha"].as_str().unwrap_or("").to_string(),
        created: instant(&d["created_at"]).unwrap_or(0),
        author: d["user"]["username"].as_str().unwrap_or("").to_string(),
        log: d["deployable"]["web_url"]
            .as_str()
            .filter(|u| !u.is_empty())
            .map(str::to_string),
        ..Default::default()
    }
}

/// The deployments of a project (newest first) as `environments` gives them (ESC-67): of each
/// environment, those down to its newest success, with the environment's address alone.
pub fn environments_of(list: &[Value]) -> Vec<Deployment> {
    let mut out = Vec::new();
    let mut settled: Vec<String> = Vec::new();
    for d in list {
        let environment = d["environment"]["name"].as_str().unwrap_or("?").to_string();
        if settled.contains(&environment) {
            continue;
        }
        let state = deployment_state(d["status"].as_str().unwrap_or("created")).to_string();
        if state == "success" {
            settled.push(environment.clone());
        }
        out.push(Deployment {
            environment,
            state,
            url: d["environment"]["external_url"]
                .as_str()
                .filter(|u| !u.is_empty())
                .map(str::to_string),
            ..deployment_facts(d)
        });
    }
    out
}

/// A merge request as a list of GitLab tells it: what is known without asking more (no approvals, no threads, no
/// pipeline: a list does not say them).
pub fn mr_of(m: &Value) -> Result<PullRequest> {
    let number = m["iid"].as_u64().ok_or_else(unreadable)?;
    let state = match m["state"].as_str() {
        Some("merged") => PrState::Merged,
        Some("closed") => PrState::Closed,
        // `locked`: being merged.
        _ => PrState::Open,
    };
    let open = state == PrState::Open;
    let detailed = m["detailed_merge_status"].as_str().unwrap_or("");
    Ok(PullRequest {
        number,
        url: m["web_url"].as_str().unwrap_or("").to_string(),
        title: bare_title(m["title"].as_str().unwrap_or("")).to_string(),
        state,
        draft: m["draft"].as_bool().unwrap_or(false)
            || m["work_in_progress"].as_bool().unwrap_or(false),
        head: m["source_branch"].as_str().unwrap_or("").to_string(),
        base: m["target_branch"].as_str().unwrap_or("").to_string(),
        // A conflict is known; while GitLab checks, nothing is; once merged or closed, nothing is said.
        mergeable: if !open {
            None
        } else if m["has_conflicts"].as_bool() == Some(true) || detailed == "conflict" {
            Some(false)
        } else if matches!(
            detailed,
            "checking" | "unchecked" | "preparing" | "approvals_syncing" | ""
        ) {
            None
        } else {
            Some(true)
        },
        changes_requested: detailed == "requested_changes",
        auto_merge: open && m["merge_when_pipeline_succeeds"].as_bool().unwrap_or(false),
        author: m["author"]["username"].as_str().unwrap_or("").to_string(),
        reviewers: m["reviewers"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|r| r["username"].as_str())
            .map(str::to_string)
            .collect(),
        created: instant(&m["created_at"]).unwrap_or(0),
        checks: m["head_pipeline"]["status"].as_str().map(pipeline_state),
        fork: matches!(
            (m["source_project_id"].as_u64(), m["target_project_id"].as_u64()),
            (Some(s), Some(t)) if s != t
        ),
        // Squashed: its squash commit; merged: its merge commit; fast-forwarded: its last commit.
        merge_commit: match state {
            PrState::Merged => ["squash_commit_sha", "merge_commit_sha", "sha"]
                .iter()
                .find_map(|k| m[*k].as_str().filter(|s| !s.is_empty()))
                .unwrap_or("")
                .to_string(),
            _ => String::new(),
        },
        ..Default::default()
    })
}

/// The threads of a merge request not resolved yet: those that can be, whose notes are not all resolved.
pub fn open_threads(discussions: &[Value]) -> u32 {
    discussions
        .iter()
        .filter(|d| {
            let notes = d["notes"].as_array().map(Vec::as_slice).unwrap_or(&[]);
            notes.iter().any(|n| n["resolvable"] == true)
                && notes.iter().any(|n| n["resolved"] == false)
        })
        .count() as u32
}

/// A job's reference: its id, with its project's when it runs in another project than the merge request's.
fn job_ref(project_id: Option<u64>, target: Option<u64>, job: u64) -> String {
    match (project_id, target) {
        (Some(p), Some(t)) if p != t => format!("{p}:{job}"),
        _ => job.to_string(),
    }
}

impl Gitlab {
    pub fn new(http: reqwest::Client, base: String, token: String) -> Self {
        Self {
            http,
            base: base.trim_end_matches('/').to_string(),
            token: token.trim().to_string(),
            limit: Mutex::default(),
            projects: Mutex::default(),
            users: Mutex::default(),
            pace: Pace::default(),
            waited: AtomicU64::new(0),
        }
    }

    pub fn with_pace(mut self, pace: Pace) -> Self {
        self.pace = pace;
        self
    }

    /// How long the client has slowed down so far.
    pub fn waited(&self) -> Duration {
        Duration::from_millis(self.waited.load(Ordering::Relaxed))
    }

    fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        let url = if path.starts_with("http") {
            path.to_string()
        } else {
            format!("{}{path}", self.base)
        };
        self.http
            .request(method, url)
            .header("PRIVATE-TOKEN", &self.token)
            .header("User-Agent", "escouade")
    }

    async fn pause(&self, delay: Duration) {
        if !delay.is_zero() {
            self.waited
                .fetch_add(delay.as_millis() as u64, Ordering::Relaxed);
            escouade_plugin_kit::time::sleep(delay).await;
        }
    }

    fn learn_limit(&self, headers: &HeaderMap) {
        let num = |n: &str| header(headers, n).and_then(|v| v.trim().parse::<u64>().ok());
        if let (Some(limit), Some(remaining), Some(reset)) = (
            num("ratelimit-limit"),
            num("ratelimit-remaining"),
            num("ratelimit-reset"),
        ) {
            *self.limit.lock() = Some(RateLimit {
                limit: limit as u32,
                remaining: remaining as u32,
                reset,
            });
        }
    }

    /// Sends a call once, after the wait the limit asks for, whatever it answers.
    async fn transmit(&self, rb: reqwest::RequestBuilder) -> Result<Reply> {
        let delay = match *self.limit.lock() {
            Some(rl) => self.pace.delay(&rl, now()),
            None => Duration::ZERO,
        };
        self.pause(delay).await;
        let reply = redirect::reply(NAME, rb).await?;
        self.learn_limit(&reply.headers);
        Ok(reply)
    }

    /// Sends a call; a 429 is waited out (`Retry-After`, at most `Pace::max_wait`) and the call sent once more.
    async fn attempt(&self, rb: reqwest::RequestBuilder) -> Result<Reply> {
        let again = rb.try_clone();
        let mut reply = self.transmit(rb).await?;
        if let (429, Some(again)) = (reply.status, again) {
            let asked = header(&reply.headers, "retry-after")
                .and_then(|v| v.trim().parse::<u64>().ok())
                .map_or(LIMITED_WAIT, Duration::from_secs);
            self.pause(asked.min(self.pace.max_wait)).await;
            reply = self.transmit(again).await?;
        }
        Ok(reply)
    }

    /// A call of the API, as `attempt` sends it, and wherever it is redirected (an instance moved to https). A status
    /// other than a success is an error that says what GitLab said; another host gets neither the token nor the body.
    async fn send(&self, method: Method, path: &str, body: Option<&Value>) -> Result<Reply> {
        let url = if path.starts_with("http") {
            path.to_string()
        } else {
            format!("{}{path}", self.base)
        };
        let reply = redirect::follow(
            NAME,
            method,
            url,
            body,
            |method, url, own, body| async move {
                let rb = if own {
                    self.request(method, &url)
                } else {
                    self.http
                        .request(method, &url)
                        .header("User-Agent", "escouade")
                };
                self.attempt(match body {
                    Some(body) => rb.json(body),
                    None => rb,
                })
                .await
            },
        )
        .await?;
        redirect::success(NAME, reply)
    }

    fn json(reply: &Reply) -> Result<Value> {
        if reply.text.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&reply.text).map_err(|_| unreadable())
    }

    async fn get(&self, path: &str) -> Result<Value> {
        Self::json(&self.send(Method::GET, path, None).await?)
    }

    /// Every item of a list, page after page (`Link`).
    async fn list(&self, path: &str) -> Result<Vec<Value>> {
        let mut out = Vec::new();
        let mut url = path.to_string();
        for _ in 0..MAX_PAGES {
            let reply = self.send(Method::GET, &url, None).await?;
            out.extend(Self::json(&reply)?.as_array().cloned().unwrap_or_default());
            // On the API only: the token goes with this address, and `send` keeps it from a redirection, not from it.
            match header(&reply.headers, "link").and_then(next_link) {
                Some(n) if n.starts_with(&format!("{}/", self.base)) => url = n,
                _ => break,
            }
        }
        Ok(out)
    }

    async fn write(&self, method: Method, path: &str, body: Value) -> Result<Value> {
        Self::json(&self.send(method, path, Some(&body)).await?)
    }

    fn mr_path(repo: &RepoRef, number: u64) -> String {
        format!("{}/merge_requests/{number}", project(repo))
    }

    /// The path of a project known by its id, read once.
    async fn project_path(&self, id: u64) -> Result<String> {
        if let Some(p) = self.projects.lock().get(&id) {
            return Ok(p.clone());
        }
        let p = self.get(&format!("/projects/{id}")).await?;
        let path = p["path_with_namespace"]
            .as_str()
            .ok_or_else(unreadable)?
            .to_string();
        self.projects.lock().insert(id, path.clone());
        Ok(path)
    }

    /// A merge request whole: what GitLab says of it, its approvals, its open threads (read only when some are) and,
    /// for a fork's, the fork's path.
    async fn full(&self, repo: &RepoRef, m: &Value) -> Result<PullRequest> {
        let mut pr = mr_of(m)?;
        let path = Self::mr_path(repo, pr.number);
        // GitLab's free tier has its approvals too; an instance that refuses them leaves none.
        if let Ok(a) = self.get(&format!("{path}/approvals")).await {
            pr.approvals = a["approved_by"].as_array().map_or(0, |v| v.len() as u32);
        }
        if m["blocking_discussions_resolved"] == false {
            pr.open_threads = open_threads(
                &self
                    .list(&format!("{path}/discussions?per_page=100"))
                    .await?,
            );
        }
        if pr.fork {
            if let Some(id) = m["source_project_id"].as_u64() {
                pr.head_repo = self.project_path(id).await.unwrap_or_default();
            }
        }
        Ok(pr)
    }

    /// The pipeline a merge request's checks are: its head pipeline (id and project), and the project of the merge
    /// request.
    async fn head_pipeline(
        &self,
        repo: &RepoRef,
        number: u64,
    ) -> Result<Option<(u64, u64, Option<u64>)>> {
        let m = self.get(&Self::mr_path(repo, number)).await?;
        let target = m["target_project_id"].as_u64();
        let p = &m["head_pipeline"];
        Ok(
            match (p["id"].as_u64(), p["project_id"].as_u64().or(target)) {
                (Some(id), Some(pid)) => Some((id, pid, target)),
                _ => None,
            },
        )
    }

    async fn check_list(
        &self,
        repo: &RepoRef,
        number: u64,
    ) -> Result<(Vec<Check>, Option<(u64, u64)>)> {
        let Some((id, pid, target)) = self.head_pipeline(repo, number).await? else {
            return Ok((Vec::new(), None));
        };
        self.pipeline_jobs(id, pid, target).await
    }

    /// The last pipeline of a branch (id and project), none when it has none yet.
    async fn branch_pipeline(&self, repo: &RepoRef, branch: &str) -> Result<Option<(u64, u64)>> {
        let list = self
            .get(&format!(
                "{}/pipelines?ref={}&per_page=1",
                project(repo),
                encode(branch)
            ))
            .await?;
        let p = &list[0];
        Ok(match (p["id"].as_u64(), p["project_id"].as_u64()) {
            (Some(id), Some(pid)) => Some((id, pid)),
            _ => None,
        })
    }

    /// The checks of the last pipeline of a branch (ESC-63), with that pipeline.
    async fn branch_check_list(
        &self,
        repo: &RepoRef,
        branch: &str,
    ) -> Result<(Vec<Check>, Option<(u64, u64)>)> {
        let Some((id, pid)) = self.branch_pipeline(repo, branch).await? else {
            return Ok((Vec::new(), None));
        };
        self.pipeline_jobs(id, pid, Some(pid)).await
    }

    /// The jobs of a pipeline `id` of the project `pid`, as checks (`target`: the project of the
    /// merge request, which names a job of another project by `<project>:<job>`).
    async fn pipeline_jobs(
        &self,
        id: u64,
        pid: u64,
        target: Option<u64>,
    ) -> Result<(Vec<Check>, Option<(u64, u64)>)> {
        let jobs = self
            .list(&format!("/projects/{pid}/pipelines/{id}/jobs?per_page=100"))
            .await?;
        let checks = jobs
            .iter()
            .map(|j| Check {
                name: j["name"].as_str().unwrap_or("?").to_string(),
                state: job_state(j),
                url: j["web_url"].as_str().map(str::to_string),
                job: j["id"].as_u64().map(|job| job_ref(Some(pid), target, job)),
                started: instant(&j["started_at"]),
                finished: instant(&j["finished_at"]),
                cancelled: matches!(j["status"].as_str(), Some("canceled" | "canceling")),
            })
            .collect();
        Ok((checks, Some((pid, id))))
    }

    /// A pipeline whose jobs failed retried in one call: one when some failed, none otherwise.
    async fn retry(&self, found: (Vec<Check>, Option<(u64, u64)>)) -> Result<usize> {
        let (checks, pipeline) = found;
        let Some((pid, id)) = pipeline else {
            return Ok(0);
        };
        if !checks.iter().any(|c| c.state == CheckState::Failure) {
            return Ok(0);
        }
        self.write(
            Method::POST,
            &format!("/projects/{pid}/pipelines/{id}/retry"),
            json!({}),
        )
        .await?;
        Ok(1)
    }

    /// The id of a user by their login, read once.
    async fn user_id(&self, login: &str) -> Result<Option<u64>> {
        if let Some(id) = self.users.lock().get(login) {
            return Ok(Some(*id));
        }
        let found = self
            .get(&format!("/users?username={}", encode(login)))
            .await?;
        let id = found[0]["id"].as_u64();
        if let Some(id) = id {
            self.users.lock().insert(login.to_string(), id);
        }
        Ok(id)
    }
}

/// GitLab's merge: a merge commit, or its commits squashed; the project's own setting says whether it rebases first
/// (fast-forward, semi-linear), which a call cannot choose.
fn squash_of(strategy: MergeStrategy) -> Result<bool> {
    match strategy {
        MergeStrategy::Merge => Ok(false),
        MergeStrategy::Squash => Ok(true),
        MergeStrategy::Rebase => Err(Unsupported("rebase merge").into()),
        MergeStrategy::SemiLinear => Err(Unsupported("semi-linear merge").into()),
    }
}

/// A review comment's reference: its discussion and its note (`<discussion>:<note>`); an answer and a resolution go to
/// the discussion.
fn discussion_of(comment: &str) -> &str {
    comment.split_once(':').map_or(comment, |(d, _)| d)
}

/// What a project's settings allow (`GET /projects/:id`): a merge commit unless squashing is « always », squashed
/// unless it is « never » (`squash_option`). Auto-merge is GitLab's own (`merge_when_pipeline_succeeds`).
pub fn merge_options_of(project: &Value) -> MergeOptions {
    let squash = project["squash_option"].as_str().unwrap_or("default_off");
    let mut strategies = Vec::new();
    if squash != "always" {
        strategies.push(MergeStrategy::Merge);
    }
    if squash != "never" {
        strategies.push(MergeStrategy::Squash);
    }
    MergeOptions {
        strategies,
        auto_merge: true,
    }
}

impl Forge for Gitlab {
    fn me(&self) -> BoxFuture<'_, Result<Identity>> {
        Box::pin(async move {
            let me = self.get("/user").await?;
            let login = me["username"].as_str().unwrap_or("?").to_string();
            // A personal token tells its rights; glab's OAuth token does not.
            let scopes = match self.get("/personal_access_tokens/self").await {
                Ok(t) => t["scopes"].as_array().map(|a| {
                    a.iter()
                        .filter_map(|s| s.as_str().map(str::to_string))
                        .collect()
                }),
                Err(_) => None,
            };
            Ok(Identity {
                label: format!("@{login}"),
                user: login,
                scopes,
            })
        })
    }

    fn create_pr<'a>(
        &'a self,
        repo: &'a RepoRef,
        pr: &'a PullRequestDraft,
    ) -> BoxFuture<'a, Result<PullRequest>> {
        Box::pin(async move {
            let mut description = pr.body.clone();
            if let Some(n) = pr.closes {
                let line = format!("Closes #{n}");
                if !description.contains(&line) {
                    if !description.is_empty() {
                        description.push_str("\n\n");
                    }
                    description.push_str(&line);
                }
            }
            let title = if pr.draft && bare_title(&pr.title) == pr.title {
                format!("Draft: {}", pr.title)
            } else {
                pr.title.clone()
            };
            let mut body = json!({
                "source_branch": pr.head,
                "target_branch": pr.base,
                "title": title,
                "description": description,
            });
            if !pr.labels.is_empty() {
                body["labels"] = pr.labels.join(",").into();
            }
            // A reviewer GitLab does not know is left out: the merge request is opened all the same.
            let mut reviewers = Vec::new();
            for login in &pr.reviewers {
                if let Ok(Some(id)) = self.user_id(login.trim_start_matches('@')).await {
                    reviewers.push(id);
                }
            }
            if !reviewers.is_empty() {
                body["reviewer_ids"] = json!(reviewers);
            }
            let made = self
                .write(
                    Method::POST,
                    &format!("{}/merge_requests", project(repo)),
                    body,
                )
                .await?;
            mr_of(&made)
        })
    }

    fn pull_request<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
    ) -> BoxFuture<'a, Result<PullRequest>> {
        Box::pin(async move {
            let m = match self.get(&Self::mr_path(repo, number)).await {
                Err(e)
                    if e.downcast_ref::<HttpError>()
                        .is_some_and(|h| h.status == 404) =>
                {
                    return Err(not_found(number))
                }
                other => other?,
            };
            self.full(repo, &m).await
        })
    }

    fn pull_requests_of_branch<'a>(
        &'a self,
        repo: &'a RepoRef,
        branch: &'a str,
    ) -> BoxFuture<'a, Result<Vec<PullRequest>>> {
        Box::pin(async move {
            let list = self
                .get(&format!(
                    "{}/merge_requests?source_branch={}&order_by=created_at&sort=desc&per_page=10",
                    project(repo),
                    encode(branch)
                ))
                .await?;
            let mut out = Vec::new();
            for m in list.as_array().into_iter().flatten() {
                // A list does not say the pipeline: the merge request's own answer does.
                let Some(n) = m["iid"].as_u64() else { continue };
                let one = self.get(&Self::mr_path(repo, n)).await?;
                out.push(self.full(repo, &one).await?);
            }
            Ok(out)
        })
    }

    fn parent<'a>(&'a self, repo: &'a RepoRef) -> BoxFuture<'a, Result<Option<RepoRef>>> {
        Box::pin(async move {
            let p = self.get(&project(repo)).await?;
            Ok(p["forked_from_project"]["path_with_namespace"]
                .as_str()
                .and_then(|path| {
                    let segments: Vec<String> = path.split('/').map(str::to_string).collect();
                    RepoRef::nested(&segments)
                }))
        })
    }

    /// One call: what a list says (no approvals, threads nor pipeline).
    fn open_pull_requests<'a>(
        &'a self,
        repo: &'a RepoRef,
    ) -> BoxFuture<'a, Result<Vec<PullRequest>>> {
        Box::pin(async move {
            let list = self
                .get(&format!(
                    "{}/merge_requests?state=opened&order_by=updated_at&sort=desc&per_page={OPEN_PRS}",
                    project(repo)
                ))
                .await?;
            list.as_array().into_iter().flatten().map(mr_of).collect()
        })
    }

    /// Those GitLab finds; one that is gone is left out.
    fn pull_requests<'a>(
        &'a self,
        repo: &'a RepoRef,
        numbers: &'a [u64],
    ) -> BoxFuture<'a, Result<Vec<PullRequest>>> {
        Box::pin(async move {
            let mut out = Vec::new();
            for n in numbers {
                match self.get(&Self::mr_path(repo, *n)).await {
                    Ok(m) => out.push(self.full(repo, &m).await?),
                    Err(e)
                        if e.downcast_ref::<HttpError>()
                            .is_some_and(|h| h.status == 404) => {}
                    Err(e) => return Err(e),
                }
            }
            Ok(out)
        })
    }

    fn merge<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
        strategy: MergeStrategy,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let squash = squash_of(strategy)?;
            self.write(
                Method::PUT,
                &format!("{}/merge", Self::mr_path(repo, number)),
                json!({ "squash": squash }),
            )
            .await?;
            Ok(())
        })
    }

    fn merge_options<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
    ) -> BoxFuture<'a, Result<MergeOptions>> {
        Box::pin(async move {
            let _ = number;
            Ok(merge_options_of(&self.get(&project(repo)).await?))
        })
    }

    /// `auto_merge` since GitLab 17.11, `merge_when_pipeline_succeeds` before: both are sent, each version reads its
    /// own and leaves the other.
    fn auto_merge<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
        strategy: MergeStrategy,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let squash = squash_of(strategy)?;
            let done = self
                .write(
                    Method::PUT,
                    &format!("{}/merge", Self::mr_path(repo, number)),
                    json!({ "squash": squash, "auto_merge": true, "merge_when_pipeline_succeeds": true }),
                )
                .await?;
            if done["merge_when_pipeline_succeeds"] != true && done["state"] != "merged" {
                return Err(anyhow!(tr!(
                    "{NAME} : l'auto-merge n'a pas été activé",
                    "{NAME}: auto-merge was not enabled"
                )));
            }
            Ok(())
        })
    }

    /// The jobs of its head pipeline (the last one, retried jobs counted once).
    fn checks<'a>(&'a self, repo: &'a RepoRef, number: u64) -> BoxFuture<'a, Result<Vec<Check>>> {
        Box::pin(async move { Ok(self.check_list(repo, number).await?.0) })
    }

    fn job_log<'a>(&'a self, repo: &'a RepoRef, job: &'a str) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            let path = match job.split_once(':') {
                Some((pid, id)) => format!("/projects/{}/jobs/{}/trace", encode(pid), encode(id)),
                None => format!("{}/jobs/{}/trace", project(repo), encode(job)),
            };
            Ok(self.send(Method::GET, &path, None).await?.text)
        })
    }

    /// GitLab retries a pipeline's failed jobs in one call: one when some failed, none otherwise.
    fn rerun_failed<'a>(&'a self, repo: &'a RepoRef, number: u64) -> BoxFuture<'a, Result<usize>> {
        Box::pin(async move {
            let found = self.check_list(repo, number).await?;
            self.retry(found).await
        })
    }

    /// The jobs of the last pipeline of a branch.
    fn branch_checks<'a>(
        &'a self,
        repo: &'a RepoRef,
        branch: &'a str,
    ) -> BoxFuture<'a, Result<Vec<Check>>> {
        Box::pin(async move { Ok(self.branch_check_list(repo, branch).await?.0) })
    }

    fn rerun_failed_of_branch<'a>(
        &'a self,
        repo: &'a RepoRef,
        branch: &'a str,
    ) -> BoxFuture<'a, Result<usize>> {
        Box::pin(async move {
            let found = self.branch_check_list(repo, branch).await?;
            self.retry(found).await
        })
    }

    /// Who approved it, and the reviewers' say where GitLab tells it (`/reviewers`, GitLab 16.x on): changes asked,
    /// reviewed.
    fn reviews<'a>(&'a self, repo: &'a RepoRef, number: u64) -> BoxFuture<'a, Result<Vec<Review>>> {
        Box::pin(async move {
            let path = Self::mr_path(repo, number);
            let mut out: Vec<Review> = Vec::new();
            let approvals = self.get(&format!("{path}/approvals")).await?;
            for a in approvals["approved_by"].as_array().into_iter().flatten() {
                if let Some(login) = a["user"]["username"].as_str() {
                    out.push(Review {
                        author: login.to_string(),
                        state: ReviewState::Approved,
                        body: String::new(),
                        created: 0,
                    });
                }
            }
            let reviewers = self
                .get(&format!("{path}/reviewers"))
                .await
                .unwrap_or(Value::Null);
            for r in reviewers.as_array().into_iter().flatten() {
                let Some(login) = r["user"]["username"].as_str() else {
                    continue;
                };
                let state = match r["state"].as_str() {
                    Some("requested_changes") => ReviewState::ChangesRequested,
                    Some("reviewed") => ReviewState::Commented,
                    _ => continue,
                };
                match out.iter_mut().find(|x| x.author == login) {
                    // A request for changes after an approval takes it back; a comment does not.
                    Some(prev) if state == ReviewState::ChangesRequested => prev.state = state,
                    Some(_) => {}
                    None => out.push(Review {
                        author: login.to_string(),
                        state,
                        body: String::new(),
                        created: 0,
                    }),
                }
            }
            Ok(out)
        })
    }

    /// The notes of its discussions, system notes aside: on a line (a diff's) or not.
    fn comments<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
    ) -> BoxFuture<'a, Result<Vec<ReviewComment>>> {
        Box::pin(async move {
            let discussions = self
                .list(&format!(
                    "{}/discussions?per_page=100",
                    Self::mr_path(repo, number)
                ))
                .await?;
            let mut out = Vec::new();
            for d in &discussions {
                let Some(id) = d["id"].as_str() else { continue };
                for n in d["notes"].as_array().into_iter().flatten() {
                    if n["system"] == true {
                        continue;
                    }
                    let pos = &n["position"];
                    out.push(ReviewComment {
                        id: format!("{id}:{}", n["id"].as_u64().unwrap_or(0)),
                        author: n["author"]["username"].as_str().unwrap_or("?").to_string(),
                        body: n["body"].as_str().unwrap_or("").to_string(),
                        path: pos["new_path"]
                            .as_str()
                            .or(pos["old_path"].as_str())
                            .map(str::to_string),
                        line: pos["new_line"]
                            .as_u64()
                            .or(pos["old_line"].as_u64())
                            .map(|l| l as u32),
                        resolved: n["resolved"] == true,
                        created: instant(&n["created_at"]).unwrap_or(0),
                        excerpt: None,
                    });
                }
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
                &format!("{}/notes", Self::mr_path(repo, number)),
                json!({ "body": body }),
            )
            .await?;
            Ok(())
        })
    }

    /// Its comments on lines as discussions on the diff, its text as a note, then its approval. GitLab's API has no
    /// « request changes »: such a review is its comments and its text.
    fn review<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
        review: &'a ReviewDraft,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let path = Self::mr_path(repo, number);
            if !review.lines.is_empty() {
                let m = self.get(&path).await?;
                let refs = &m["diff_refs"];
                for l in &review.lines {
                    self.write(
                        Method::POST,
                        &format!("{path}/discussions"),
                        json!({
                            "body": l.body,
                            "position": {
                                "position_type": "text",
                                "base_sha": refs["base_sha"],
                                "start_sha": refs["start_sha"],
                                "head_sha": refs["head_sha"],
                                "old_path": l.path,
                                "new_path": l.path,
                                "new_line": l.line,
                            },
                        }),
                    )
                    .await?;
                }
            }
            if !review.body.trim().is_empty() {
                self.write(
                    Method::POST,
                    &format!("{path}/notes"),
                    json!({ "body": review.body }),
                )
                .await?;
            }
            if review.state == ReviewState::Approved {
                self.write(Method::POST, &format!("{path}/approve"), json!({}))
                    .await?;
            }
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
            self.write(
                Method::POST,
                &format!(
                    "{}/discussions/{}/notes",
                    Self::mr_path(repo, number),
                    encode(discussion_of(comment))
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
        number: u64,
        comment: &'a str,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            self.write(
                Method::PUT,
                &format!(
                    "{}/discussions/{}",
                    Self::mr_path(repo, number),
                    encode(discussion_of(comment))
                ),
                json!({ "resolved": true }),
            )
            .await?;
            Ok(())
        })
    }

    /// The last deployment of each environment for the branch (or commit) `git_ref`, among the project's latest
    /// (GitLab does not filter them by ref).
    fn deployments<'a>(
        &'a self,
        repo: &'a RepoRef,
        git_ref: &'a str,
    ) -> BoxFuture<'a, Result<Vec<Deployment>>> {
        Box::pin(async move {
            let list = self
                .get(&format!(
                    "{}/deployments?order_by=updated_at&sort=desc&per_page={DEPLOYMENTS_READ}",
                    project(repo)
                ))
                .await?;
            let mut out: Vec<Deployment> = Vec::new();
            for d in list.as_array().into_iter().flatten() {
                if d["ref"] != git_ref && d["sha"] != git_ref {
                    continue;
                }
                let environment = d["environment"]["name"].as_str().unwrap_or("?");
                if out.iter().any(|x| x.environment == environment) {
                    continue;
                }
                out.push(Deployment {
                    environment: environment.to_string(),
                    state: deployment_state(d["status"].as_str().unwrap_or("created")).to_string(),
                    url: d["environment"]["external_url"]
                        .as_str()
                        .or(d["deployable"]["web_url"].as_str())
                        .filter(|u| !u.is_empty())
                        .map(str::to_string),
                    ..deployment_facts(d)
                });
            }
            Ok(out)
        })
    }

    fn environments<'a>(&'a self, repo: &'a RepoRef) -> BoxFuture<'a, Result<Vec<Deployment>>> {
        Box::pin(async move {
            let list = self
                .get(&format!(
                    "{}/deployments?order_by=updated_at&sort=desc&per_page={DEPLOYMENTS_READ}",
                    project(repo)
                ))
                .await?;
            Ok(environments_of(
                list.as_array().map_or(&[][..], Vec::as_slice),
            ))
        })
    }

    fn rate_limit(&self) -> Option<RateLimit> {
        *self.limit.lock()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_draft_title_loses_its_mark() {
        assert_eq!(bare_title("Draft: Fix login"), "Fix login");
        assert_eq!(bare_title("[Draft] Fix login"), "Fix login");
        assert_eq!(bare_title("Fix login"), "Fix login");
    }

    #[test]
    fn a_job_that_may_fail_does_not_fail_the_pipeline() {
        assert_eq!(
            job_state(&json!({"status": "failed", "allow_failure": true})),
            CheckState::Success
        );
        assert_eq!(
            job_state(&json!({"status": "failed", "allow_failure": false})),
            CheckState::Failure
        );
        assert_eq!(
            job_state(&json!({"status": "manual", "allow_failure": true})),
            CheckState::Skipped
        );
        assert_eq!(
            job_state(&json!({"status": "manual", "allow_failure": false})),
            CheckState::Pending
        );
        assert_eq!(
            job_state(&json!({"status": "running"})),
            CheckState::Pending
        );
    }

    #[test]
    fn a_comment_answers_in_its_discussion() {
        assert_eq!(discussion_of("6a9c17:2829102301"), "6a9c17");
        assert_eq!(discussion_of("6a9c17"), "6a9c17");
    }

    #[test]
    fn a_job_of_another_project_says_it() {
        assert_eq!(job_ref(Some(1), Some(1), 9), "9");
        assert_eq!(job_ref(Some(2), Some(1), 9), "2:9");
    }
}

#[cfg(test)]
#[path = "merges_tests.rs"]
mod client_tests;
