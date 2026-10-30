//! GitHub's client: pull requests, merge and auto-merge, checks, Actions logs and reruns, reviews,
//! comments and deployments, on github.com, GHE.com and GitHub Enterprise Server (the base of its
//! API comes from the account).
//!
//! It spares the API's quota in three ways: the pull requests a project follows are read in one
//! GraphQL request, the REST reads are conditional (`If-None-Match`: a 304 does not count against
//! the limit, the answer kept from before is served), and it slows down as the limit comes near
//! (`X-RateLimit-*`).

use super::*;
use parking_lot::Mutex;
use serde_json::json;
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const NAME: &str = "GitHub";

/// Pages followed at most for a list.
const MAX_PAGES: usize = 10;

/// Addresses whose answer is kept for the conditional reads: the oldest one goes first.
pub const CACHE_MAX: usize = 500;

/// How long a secondary limit is waited for when GitHub says nothing about when (its advice).
const SECONDARY_WAIT: Duration = Duration::from_secs(60);

/// When and how long the client slows down.
#[derive(Debug, Clone, Copy)]
pub struct Pace {
    /// Below this share of the limit left, calls are spread over what remains of the window.
    pub slow_below: f64,
    /// The longest a call waits (an hour's window is not waited for).
    pub max_wait: Duration,
}

impl Default for Pace {
    fn default() -> Self {
        Self {
            slow_below: 0.1,
            max_wait: Duration::from_secs(30),
        }
    }
}

impl Pace {
    /// How long to wait before the next call, `now` being seconds since 1970: none while the limit
    /// is far; then what remains of the window shared among the calls that remain; the whole rest
    /// of the window when none remains.
    pub fn delay(&self, rl: &RateLimit, now: u64) -> Duration {
        let left = rl.reset.saturating_sub(now);
        let wait = if rl.remaining == 0 {
            left.saturating_add(1) as f64
        } else if rl.limit == 0 || f64::from(rl.remaining) / f64::from(rl.limit) >= self.slow_below
        {
            return Duration::ZERO;
        } else {
            left as f64 / f64::from(rl.remaining + 1)
        };
        Duration::from_secs_f64(wait).min(self.max_wait)
    }
}

/// What a call answered.
struct Reply {
    status: u16,
    headers: HeaderMap,
    text: String,
}

/// The answers of conditional reads, by URL: the entity tag and the JSON it went with. At most
/// `CACHE_MAX` of them; the one stored first is the one let go.
#[derive(Default)]
struct Cache {
    answers: HashMap<String, (String, Value)>,
    /// The URLs of `answers`, the oldest first.
    order: VecDeque<String>,
}

impl Cache {
    fn get(&self, url: &str) -> Option<&(String, Value)> {
        self.answers.get(url)
    }

    /// Keeps `answer` for `url`; an answer already kept for it is replaced where it stands.
    fn put(&mut self, url: String, answer: (String, Value)) {
        if self.answers.insert(url.clone(), answer).is_none() {
            self.order.push_back(url);
            if self.order.len() > CACHE_MAX {
                if let Some(oldest) = self.order.pop_front() {
                    self.answers.remove(&oldest);
                }
            }
        }
    }
}

pub struct Github {
    http: reqwest::Client,
    base: String,
    graphql: String,
    token: String,
    cache: Mutex<Cache>,
    /// The last limit each resource (`core`, `graphql`) said.
    limits: Mutex<HashMap<String, RateLimit>>,
    pace: Pace,
    /// Milliseconds spent slowing down.
    waited: AtomicU64,
}

/// Where GraphQL is, given where REST is: `/api/graphql` beside a Server's `/api/v3`.
pub fn graphql_url(base: &str) -> String {
    match base.strip_suffix("/api/v3") {
        Some(root) => format!("{root}/api/graphql"),
        None => format!("{base}/graphql"),
    }
}

fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
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

impl Github {
    pub fn new(http: reqwest::Client, base: String, token: String) -> Self {
        Self {
            http,
            graphql: graphql_url(&base),
            base,
            token: token.trim().to_string(),
            cache: Mutex::default(),
            limits: Mutex::default(),
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

    fn request(&self, method: reqwest::Method, url: &str) -> reqwest::RequestBuilder {
        let url = if url.starts_with("http") {
            url.to_string()
        } else {
            format!("{}{url}", self.base)
        };
        self.http
            .request(method, url)
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "escouade")
    }

    /// Waits `delay`, which counts in `waited`.
    async fn pause(&self, delay: Duration) {
        if !delay.is_zero() {
            self.waited
                .fetch_add(delay.as_millis() as u64, Ordering::Relaxed);
            tokio::time::sleep(delay).await;
        }
    }

    /// Waits as long as the limit of `resource` asks.
    async fn slow_down(&self, resource: &str) {
        let delay = match self.limits.lock().get(resource) {
            Some(rl) => self.pace.delay(rl, now()),
            None => Duration::ZERO,
        };
        self.pause(delay).await;
    }

    fn learn_limit(&self, headers: &HeaderMap) {
        let num = |n: &str| header(headers, n).and_then(|v| v.trim().parse::<u64>().ok());
        if let (Some(limit), Some(remaining), Some(reset)) = (
            num("x-ratelimit-limit"),
            num("x-ratelimit-remaining"),
            num("x-ratelimit-reset"),
        ) {
            let resource = header(headers, "x-ratelimit-resource").unwrap_or("core");
            self.limits.lock().insert(
                resource.to_string(),
                RateLimit {
                    limit: limit as u32,
                    remaining: remaining as u32,
                    reset,
                },
            );
        }
    }

    /// How long GitHub asks to wait before a call is made again when it refuses it for its
    /// secondary limit: the `retry-after` it sends, else the advice of a minute when its message
    /// names the limit. None for any other answer.
    fn secondary_limit(reply: &Reply) -> Option<Duration> {
        if reply.status != 403 && reply.status != 429 {
            return None;
        }
        match header(&reply.headers, "retry-after").and_then(|v| v.trim().parse::<u64>().ok()) {
            Some(seconds) => Some(Duration::from_secs(seconds)),
            None => reply
                .text
                .to_lowercase()
                .contains("secondary rate limit")
                .then_some(SECONDARY_WAIT),
        }
    }

    /// Sends a call once, after the wait the limit of `resource` asks for, whatever it answers.
    async fn transmit(&self, resource: &str, rb: reqwest::RequestBuilder) -> Result<Reply> {
        self.slow_down(resource).await;
        let resp = rb.send().await.map_err(|e| {
            let e = anyhow::Error::from(e.without_url());
            anyhow!(tr!(
                "{NAME} injoignable : {e:#}",
                "{NAME} can’t be reached: {e:#}"
            ))
        })?;
        let status = resp.status().as_u16();
        let headers = resp.headers().clone();
        let text = resp.text().await.unwrap_or_default();
        self.learn_limit(&headers);
        Ok(Reply {
            status,
            headers,
            text,
        })
    }

    /// Sends a call after the wait the limit asks for. A secondary limit is waited out, at most
    /// `Pace::max_wait`, then the call is sent once more. A status other than a success or a 304
    /// is an error that says what GitHub said.
    async fn send(&self, resource: &str, rb: reqwest::RequestBuilder) -> Result<Reply> {
        let again = rb.try_clone();
        let mut reply = self.transmit(resource, rb).await?;
        if let (Some(again), Some(asked)) = (again, Self::secondary_limit(&reply)) {
            self.pause(asked.min(self.pace.max_wait)).await;
            reply = self.transmit(resource, again).await?;
        }
        if !(200..300).contains(&reply.status) && reply.status != 304 {
            return Err(crate::integrations::HttpError {
                status: reply.status,
                message: crate::integrations::http_error_in(
                    crate::i18n::ui(),
                    NAME,
                    reply.status,
                    &reply.text,
                ),
            }
            .into());
        }
        Ok(reply)
    }

    fn json(reply: &Reply) -> Result<Value> {
        if reply.text.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&reply.text).map_err(|_| {
            anyhow!(tr!(
                "{NAME} : réponse illisible",
                "{NAME}: unreadable answer"
            ))
        })
    }

    /// A read, conditional when it was made before: GitHub answers 304 (not counted against the
    /// limit) when nothing changed, and the answer kept is served. With the next page's address.
    async fn read(&self, url: &str) -> Result<(Value, Option<String>)> {
        let mut rb = self.request(reqwest::Method::GET, url);
        let key = if url.starts_with("http") {
            url.to_string()
        } else {
            format!("{}{url}", self.base)
        };
        if let Some((tag, _)) = self.cache.lock().get(&key) {
            rb = rb.header("If-None-Match", tag);
        }
        let reply = self.send("core", rb).await?;
        let next = header(&reply.headers, "link").and_then(next_link);
        if reply.status == 304 {
            if let Some((_, v)) = self.cache.lock().get(&key) {
                return Ok((v.clone(), next));
            }
            // Nothing kept to serve (not supposed to happen): ask again without the condition.
            let reply = self
                .send("core", self.request(reqwest::Method::GET, url))
                .await?;
            return Ok((Self::json(&reply)?, next));
        }
        let value = Self::json(&reply)?;
        if let Some(tag) = header(&reply.headers, "etag") {
            self.cache.lock().put(key, (tag.to_string(), value.clone()));
        }
        Ok((value, next))
    }

    async fn get(&self, path: &str) -> Result<Value> {
        Ok(self.read(path).await?.0)
    }

    /// Every item of a list, page after page.
    async fn list(&self, path: &str) -> Result<Vec<Value>> {
        self.pages(path, None).await
    }

    /// Every item of a list, page after page: the answer is the list itself, or an object holding
    /// it under `key` (the check runs of a commit are one).
    async fn pages(&self, path: &str, key: Option<&str>) -> Result<Vec<Value>> {
        let mut out = Vec::new();
        let mut url = path.to_string();
        for _ in 0..MAX_PAGES {
            let (page, next) = self.read(&url).await?;
            let items = match key {
                Some(key) => &page[key],
                None => &page,
            };
            out.extend(items.as_array().cloned().unwrap_or_default());
            match next {
                Some(n) => url = n,
                None => break,
            }
        }
        Ok(out)
    }

    async fn write(&self, method: reqwest::Method, path: &str, body: Value) -> Result<Value> {
        let reply = self
            .send("core", self.request(method, path).json(&body))
            .await?;
        Self::json(&reply)
    }

    /// A GraphQL operation; its data. Errors that leave data (a pull request that does not exist
    /// among others that do) are left to the caller, who finds a null.
    async fn graphql(&self, operation: &str, query: &str, variables: Value) -> Result<Value> {
        let rb = self
            .http
            .post(&self.graphql)
            .bearer_auth(&self.token)
            .header("User-Agent", "escouade");
        let reply = self
            .send(
                "graphql",
                rb.json(&json!({
                    "operationName": operation,
                    "query": query,
                    "variables": variables,
                })),
            )
            .await?;
        let v = Self::json(&reply)?;
        if v["data"].is_null() {
            let said = v["errors"]
                .as_array()
                .map(|e| {
                    e.iter()
                        .filter_map(|e| e["message"].as_str())
                        .collect::<Vec<_>>()
                        .join(" ; ")
                })
                .unwrap_or_default();
            return Err(anyhow!("{NAME} (GraphQL) : {said}"));
        }
        Ok(v["data"].clone())
    }

    fn repo_path(repo: &RepoRef) -> String {
        format!("/repos/{}/{}", enc(&repo.namespace), enc(&repo.name))
    }

    /// The commit a pull request's checks hang from.
    async fn head_sha(&self, repo: &RepoRef, number: u64) -> Result<String> {
        let pr = self
            .get(&format!("{}/pulls/{number}", Self::repo_path(repo)))
            .await?;
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

    async fn check_list(&self, repo: &RepoRef, number: u64) -> Result<Vec<Check>> {
        let sha = self.head_sha(repo, number).await?;
        let base = Self::repo_path(repo);
        let runs = self
            .pages(
                &format!("{base}/commits/{sha}/check-runs?per_page=100"),
                Some("check_runs"),
            )
            .await?;
        let statuses = self
            .get(&format!("{base}/commits/{sha}/status?per_page=100"))
            .await?;
        let mut out = Vec::new();
        for run in &runs {
            out.push(Check {
                name: run["name"].as_str().unwrap_or("?").to_string(),
                state: run_state(run),
                url: run["html_url"].as_str().map(str::to_string),
                // The id of an Actions job is its check run's; other apps keep their logs.
                job: (run["app"]["slug"] == "github-actions")
                    .then(|| run["id"].as_u64().map(|i| i.to_string()))
                    .flatten(),
                started: instant(&run["started_at"]),
                finished: instant(&run["completed_at"]),
                cancelled: run["conclusion"] == "cancelled",
            });
        }
        for s in statuses["statuses"].as_array().into_iter().flatten() {
            out.push(Check {
                name: s["context"].as_str().unwrap_or("?").to_string(),
                state: match s["state"].as_str() {
                    Some("success") => CheckState::Success,
                    Some("failure" | "error") => CheckState::Failure,
                    _ => CheckState::Pending,
                },
                url: s["target_url"].as_str().map(str::to_string),
                // A status says when it was set, not when its job began.
                ..Default::default()
            });
        }
        Ok(out)
    }
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

fn run_state(run: &Value) -> CheckState {
    if run["status"] != "completed" {
        return CheckState::Pending;
    }
    match run["conclusion"].as_str() {
        Some("success" | "neutral") => CheckState::Success,
        Some("skipped") => CheckState::Skipped,
        _ => CheckState::Failure,
    }
}

/// What a pull request's GraphQL fragment asks: its state, its reviews' last say, its open threads
/// and the checks of its last commit, all in one.
const PR_FIELDS: &str = "number url title state isDraft headRefName baseRefName mergeable \
    author { login } createdAt \
    isCrossRepository headRepository { nameWithOwner } \
    autoMergeRequest { enabledAt } \
    latestOpinionatedReviews(first: 50) { nodes { state } } \
    reviewThreads(first: 100) { nodes { isResolved } } \
    commits(last: 1) { nodes { commit { statusCheckRollup { state } } } }";

fn pr_of_graphql(p: &Value) -> Option<PullRequest> {
    let number = p["number"].as_u64()?;
    let reviews: Vec<&str> = p["latestOpinionatedReviews"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| r["state"].as_str())
        .collect();
    Some(PullRequest {
        repo: String::new(),
        number,
        url: p["url"].as_str().unwrap_or("").to_string(),
        title: p["title"].as_str().unwrap_or("").to_string(),
        state: match p["state"].as_str() {
            Some("MERGED") => PrState::Merged,
            Some("CLOSED") => PrState::Closed,
            _ => PrState::Open,
        },
        draft: p["isDraft"].as_bool().unwrap_or(false),
        head: p["headRefName"].as_str().unwrap_or("").to_string(),
        base: p["baseRefName"].as_str().unwrap_or("").to_string(),
        mergeable: match p["mergeable"].as_str() {
            Some("MERGEABLE") => Some(true),
            Some("CONFLICTING") => Some(false),
            _ => None,
        },
        approvals: reviews.iter().filter(|s| **s == "APPROVED").count() as u32,
        changes_requested: reviews.contains(&"CHANGES_REQUESTED"),
        open_threads: p["reviewThreads"]["nodes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|t| t["isResolved"] == false)
            .count() as u32,
        auto_merge: !p["autoMergeRequest"].is_null(),
        author: p["author"]["login"].as_str().unwrap_or("").to_string(),
        created: instant(&p["createdAt"]).unwrap_or(0),
        fork: p["isCrossRepository"].as_bool().unwrap_or(false),
        head_repo: p["headRepository"]["nameWithOwner"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        checks: match p["commits"]["nodes"][0]["commit"]["statusCheckRollup"]["state"].as_str() {
            Some("SUCCESS") => Some(CheckState::Success),
            Some("FAILURE" | "ERROR") => Some(CheckState::Failure),
            Some(_) => Some(CheckState::Pending),
            None => None,
        },
        host_repo: String::new(),
    })
}

/// A pull request as REST tells it (what a creation answers).
fn pr_of_rest(p: &Value) -> Result<PullRequest> {
    Ok(PullRequest {
        number: p["number"].as_u64().ok_or_else(|| {
            anyhow!(tr!(
                "{NAME} : réponse illisible",
                "{NAME}: unreadable answer"
            ))
        })?,
        url: p["html_url"].as_str().unwrap_or("").to_string(),
        title: p["title"].as_str().unwrap_or("").to_string(),
        state: if p["merged_at"].is_string() {
            PrState::Merged
        } else if p["state"] == "closed" {
            PrState::Closed
        } else {
            PrState::Open
        },
        draft: p["draft"].as_bool().unwrap_or(false),
        head: p["head"]["ref"].as_str().unwrap_or("").to_string(),
        base: p["base"]["ref"].as_str().unwrap_or("").to_string(),
        mergeable: p["mergeable"].as_bool(),
        head_repo: p["head"]["repo"]["full_name"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        author: p["user"]["login"].as_str().unwrap_or("").to_string(),
        created: instant(&p["created_at"]).unwrap_or(0),
        ..Default::default()
    })
}

/// GitHub's name of a strategy; it has no semi-linear merge.
fn method_name(strategy: MergeStrategy) -> Result<&'static str> {
    match strategy {
        MergeStrategy::Merge => Ok("merge"),
        MergeStrategy::Squash => Ok("squash"),
        MergeStrategy::Rebase => Ok("rebase"),
        MergeStrategy::SemiLinear => Err(Unsupported("semi-linear merge").into()),
    }
}

impl Forge for Github {
    fn me(&self) -> BoxFuture<'_, Result<Identity>> {
        Box::pin(async move {
            let reply = self
                .send("core", self.request(reqwest::Method::GET, "/user"))
                .await?;
            let me = Self::json(&reply)?;
            let login = me["login"].as_str().unwrap_or("?").to_string();
            // A classic token lists its rights; a fine-grained one, none.
            let scopes = header(&reply.headers, "x-oauth-scopes").map(|s| {
                s.split(',')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect()
            });
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
            let made = self
                .write(
                    reqwest::Method::POST,
                    &format!("{}/pulls", Self::repo_path(repo)),
                    json!({
                        "title": pr.title,
                        "body": body,
                        "head": pr.head,
                        "base": pr.base,
                        "draft": pr.draft,
                    }),
                )
                .await?;
            let created = pr_of_rest(&made)?;
            // The pull request exists: a reviewer GitHub refuses (the author, no access) or a
            // label that fails does not take it back.
            if !pr.reviewers.is_empty() {
                let _ = self
                    .write(
                        reqwest::Method::POST,
                        &format!(
                            "{}/pulls/{}/requested_reviewers",
                            Self::repo_path(repo),
                            created.number
                        ),
                        json!({ "reviewers": pr.reviewers }),
                    )
                    .await;
            }
            if !pr.labels.is_empty() {
                let _ = self
                    .write(
                        reqwest::Method::POST,
                        &format!("{}/issues/{}/labels", Self::repo_path(repo), created.number),
                        json!({ "labels": pr.labels }),
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
        Box::pin(async move {
            self.pull_requests(repo, &[number])
                .await?
                .into_iter()
                .next()
                .ok_or_else(|| {
                    anyhow!(tr!(
                        "{NAME} : PR #{number} introuvable",
                        "{NAME}: pull request #{number} not found"
                    ))
                })
        })
    }

    fn pull_requests_of_branch<'a>(
        &'a self,
        repo: &'a RepoRef,
        branch: &'a str,
    ) -> BoxFuture<'a, Result<Vec<PullRequest>>> {
        Box::pin(async move {
            let query = format!(
                "query PullsOfBranch($owner: String!, $name: String!, $branch: String!) {{ \
                 repository(owner: $owner, name: $name) {{ \
                 pullRequests(headRefName: $branch, first: 10, \
                 orderBy: {{field: CREATED_AT, direction: DESC}}) {{ nodes {{ {PR_FIELDS} }} }} }} }}"
            );
            let data = self
                .graphql(
                    "PullsOfBranch",
                    &query,
                    json!({ "owner": repo.namespace, "name": repo.name, "branch": branch }),
                )
                .await?;
            Ok(data["repository"]["pullRequests"]["nodes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(pr_of_graphql)
                .collect())
        })
    }

    fn open_pull_requests<'a>(
        &'a self,
        repo: &'a RepoRef,
    ) -> BoxFuture<'a, Result<Vec<PullRequest>>> {
        Box::pin(async move {
            let query = format!(
                "query OpenPulls($owner: String!, $name: String!) {{ \
                 repository(owner: $owner, name: $name) {{ \
                 pullRequests(states: OPEN, first: {OPEN_PRS}, \
                 orderBy: {{field: UPDATED_AT, direction: DESC}}) {{ nodes {{ {PR_FIELDS} }} }} }} }}"
            );
            let data = self
                .graphql(
                    "OpenPulls",
                    &query,
                    json!({ "owner": repo.namespace, "name": repo.name }),
                )
                .await?;
            Ok(data["repository"]["pullRequests"]["nodes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(pr_of_graphql)
                .collect())
        })
    }

    fn parent<'a>(&'a self, repo: &'a RepoRef) -> BoxFuture<'a, Result<Option<RepoRef>>> {
        Box::pin(async move {
            let data = self
                .graphql(
                    "RepoParent",
                    "query RepoParent($owner: String!, $name: String!) { \
                     repository(owner: $owner, name: $name) { parent { name owner { login } } } }",
                    json!({ "owner": repo.namespace, "name": repo.name }),
                )
                .await?;
            let parent = &data["repository"]["parent"];
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

    /// All of them in one GraphQL request, whatever their number: the poll of a project's
    /// followed pull requests costs a single call.
    fn pull_requests<'a>(
        &'a self,
        repo: &'a RepoRef,
        numbers: &'a [u64],
    ) -> BoxFuture<'a, Result<Vec<PullRequest>>> {
        Box::pin(async move {
            if numbers.is_empty() {
                return Ok(Vec::new());
            }
            let aliases: String = numbers
                .iter()
                .map(|n| format!("p{n}: pullRequest(number: {n}) {{ {PR_FIELDS} }} "))
                .collect();
            let query = format!(
                "query Pulls($owner: String!, $name: String!) {{ \
                 repository(owner: $owner, name: $name) {{ {aliases} }} }}"
            );
            let data = self
                .graphql(
                    "Pulls",
                    &query,
                    json!({ "owner": repo.namespace, "name": repo.name }),
                )
                .await?;
            Ok(numbers
                .iter()
                .filter_map(|n| pr_of_graphql(&data["repository"][format!("p{n}")]))
                .collect())
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
                reqwest::Method::PUT,
                &format!("{}/pulls/{number}/merge", Self::repo_path(repo)),
                json!({ "merge_method": method_name(strategy)? }),
            )
            .await?;
            Ok(())
        })
    }

    fn auto_merge<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
        strategy: MergeStrategy,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let method = method_name(strategy)?;
            let data = self
                .graphql(
                    "PullRequestId",
                    "query PullRequestId($owner: String!, $name: String!, $number: Int!) { \
                     repository(owner: $owner, name: $name) { pullRequest(number: $number) { id } } }",
                    json!({ "owner": repo.namespace, "name": repo.name, "number": number }),
                )
                .await?;
            let id = data["repository"]["pullRequest"]["id"]
                .as_str()
                .ok_or_else(|| {
                    anyhow!(tr!(
                        "{NAME} : PR #{number} introuvable",
                        "{NAME}: pull request #{number} not found"
                    ))
                })?;
            // GraphQL answers 200 with its errors (the repository does not allow auto-merge).
            let done = self
                .graphql(
                    "EnableAutoMerge",
                    "mutation EnableAutoMerge($id: ID!, $method: PullRequestMergeMethod!) { \
                     enablePullRequestAutoMerge(input: {pullRequestId: $id, mergeMethod: $method}) { \
                     pullRequest { number autoMergeRequest { enabledAt } } } }",
                    json!({ "id": id, "method": method.to_uppercase() }),
                )
                .await?;
            if done["enablePullRequestAutoMerge"]["pullRequest"].is_null() {
                return Err(anyhow!(tr!(
                    "{NAME} : l'auto-merge n'a pas été activé",
                    "{NAME}: auto-merge was not enabled"
                )));
            }
            Ok(())
        })
    }

    fn checks<'a>(&'a self, repo: &'a RepoRef, number: u64) -> BoxFuture<'a, Result<Vec<Check>>> {
        Box::pin(async move { self.check_list(repo, number).await })
    }

    /// The log of an Actions job: GitHub answers with a redirect to the text, which the client
    /// follows (without the token, as it leaves GitHub's host).
    fn job_log<'a>(&'a self, repo: &'a RepoRef, job: &'a str) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            let path = format!("{}/actions/jobs/{}/logs", Self::repo_path(repo), enc(job));
            let reply = self
                .send("core", self.request(reqwest::Method::GET, &path))
                .await?;
            Ok(reply.text)
        })
    }

    fn rerun_failed<'a>(&'a self, repo: &'a RepoRef, number: u64) -> BoxFuture<'a, Result<usize>> {
        Box::pin(async move {
            let mut runs = BTreeSet::new();
            for check in self.check_list(repo, number).await? {
                let (CheckState::Failure, Some(job)) = (check.state, check.job) else {
                    continue;
                };
                let j = self
                    .get(&format!(
                        "{}/actions/jobs/{}",
                        Self::repo_path(repo),
                        enc(&job)
                    ))
                    .await?;
                if let Some(run) = j["run_id"].as_u64() {
                    runs.insert(run);
                }
            }
            for run in &runs {
                self.write(
                    reqwest::Method::POST,
                    &format!(
                        "{}/actions/runs/{run}/rerun-failed-jobs",
                        Self::repo_path(repo)
                    ),
                    json!({}),
                )
                .await?;
            }
            Ok(runs.len())
        })
    }

    fn reviews<'a>(&'a self, repo: &'a RepoRef, number: u64) -> BoxFuture<'a, Result<Vec<Review>>> {
        Box::pin(async move {
            let all = self
                .list(&format!(
                    "{}/pulls/{number}/reviews?per_page=100",
                    Self::repo_path(repo)
                ))
                .await?;
            // The last word of each reviewer; a pending (unpublished) or dismissed review is none.
            let mut last: Vec<Review> = Vec::new();
            for r in &all {
                let state = match r["state"].as_str() {
                    Some("APPROVED") => ReviewState::Approved,
                    Some("CHANGES_REQUESTED") => ReviewState::ChangesRequested,
                    Some("COMMENTED") => ReviewState::Commented,
                    _ => continue,
                };
                let author = r["user"]["login"].as_str().unwrap_or("?").to_string();
                let review = Review {
                    author: author.clone(),
                    state,
                    body: r["body"].as_str().unwrap_or("").to_string(),
                };
                // A comment alone does not take back an approval or a request for changes.
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
            Ok(last)
        })
    }

    fn comments<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
    ) -> BoxFuture<'a, Result<Vec<ReviewComment>>> {
        Box::pin(async move {
            let data = self
                .graphql(
                    "Threads",
                    "query Threads($owner: String!, $name: String!, $number: Int!) { \
                     repository(owner: $owner, name: $name) { pullRequest(number: $number) { \
                     reviewThreads(first: 100) { nodes { isResolved path line \
                     comments(first: 50) { nodes { databaseId author { login } body } } } } } } }",
                    json!({ "owner": repo.namespace, "name": repo.name, "number": number }),
                )
                .await?;
            let mut out = Vec::new();
            for t in data["repository"]["pullRequest"]["reviewThreads"]["nodes"]
                .as_array()
                .into_iter()
                .flatten()
            {
                for c in t["comments"]["nodes"].as_array().into_iter().flatten() {
                    out.push(ReviewComment {
                        id: c["databaseId"]
                            .as_u64()
                            .map(|i| i.to_string())
                            .unwrap_or_default(),
                        author: c["author"]["login"].as_str().unwrap_or("?").to_string(),
                        body: c["body"].as_str().unwrap_or("").to_string(),
                        path: t["path"].as_str().map(str::to_string),
                        line: t["line"].as_u64().map(|l| l as u32),
                        resolved: t["isResolved"].as_bool().unwrap_or(false),
                    });
                }
            }
            // The conversation under the description: no file, no line, no thread to resolve.
            for c in self
                .list(&format!(
                    "{}/issues/{number}/comments?per_page=100",
                    Self::repo_path(repo)
                ))
                .await?
            {
                out.push(ReviewComment {
                    id: c["id"].as_u64().map(|i| i.to_string()).unwrap_or_default(),
                    author: c["user"]["login"].as_str().unwrap_or("?").to_string(),
                    body: c["body"].as_str().unwrap_or("").to_string(),
                    path: None,
                    line: None,
                    resolved: false,
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
                reqwest::Method::POST,
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
                ReviewState::Approved => "APPROVE",
                ReviewState::ChangesRequested => "REQUEST_CHANGES",
                ReviewState::Commented => "COMMENT",
            };
            let lines: Vec<Value> = review
                .lines
                .iter()
                .map(|l| json!({ "path": l.path, "line": l.line, "side": "RIGHT", "body": l.body }))
                .collect();
            self.write(
                reqwest::Method::POST,
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
            self.write(
                reqwest::Method::POST,
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

    fn deployments<'a>(
        &'a self,
        repo: &'a RepoRef,
        git_ref: &'a str,
    ) -> BoxFuture<'a, Result<Vec<Deployment>>> {
        Box::pin(async move {
            let base = Self::repo_path(repo);
            let list = self
                .get(&format!(
                    "{base}/deployments?ref={}&per_page=10",
                    enc(git_ref)
                ))
                .await?;
            let mut out = Vec::new();
            for d in list.as_array().into_iter().flatten() {
                let Some(id) = d["id"].as_u64() else { continue };
                let statuses = self
                    .get(&format!("{base}/deployments/{id}/statuses?per_page=1"))
                    .await?;
                let last = &statuses[0];
                out.push(Deployment {
                    environment: d["environment"].as_str().unwrap_or("?").to_string(),
                    // A deployment nobody has reported on is still waiting.
                    state: last["state"].as_str().unwrap_or("pending").to_string(),
                    url: ["environment_url", "log_url", "target_url"]
                        .iter()
                        .find_map(|k| last[*k].as_str().filter(|u| !u.is_empty()))
                        .map(str::to_string),
                });
            }
            Ok(out)
        })
    }

    fn rate_limit(&self) -> Option<RateLimit> {
        self.limits
            .lock()
            .values()
            .min_by_key(|l| l.remaining)
            .copied()
    }
}
