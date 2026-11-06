//! Bitbucket Cloud's client: pull requests (drafts too), merge, commit statuses, pipelines and the logs of their steps,
//! comments on lines and their threads, reviews, deployments. Written from Bitbucket's API 2.0 documentation and
//! tried on a fake server fed with its answers (nobody tried it on bitbucket.org yet: docs/TODO-2.2.md).
//!
//! **No auto-merge.** Bitbucket's API merges at once (BCLOUD-22055, BCLOUD-22062 ask for more): the capability is
//! declared absent, and `auto_merge` here says `Unsupported`.
//!
//! **The limit by the hour.** Bitbucket Cloud allows 1000 calls an hour (more for big workspaces) and does not say
//! how many remain: the client counts its own calls over the hour, believes the `X-RateLimit-Limit` it is told, takes
//! `X-RateLimit-NearLimit` for what it says, and slows down as the hour's budget runs low. A refusal (429) is waited
//! out (its `Retry-After`, at most `Pace::max_wait`) and the call sent once more.

use crate::redirect::{self, Reply};
use anyhow::{anyhow, Result};
use escouade_plugin_kit::errors::HttpError;
use escouade_plugin_kit::forge::*;
use escouade_plugin_kit::http::reqwest;
use escouade_plugin_kit::tr;
use parking_lot::Mutex;
use reqwest::header::HeaderMap;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const NAME: &str = "Bitbucket";

/// Pages followed at most for a list.
const MAX_PAGES: usize = 10;

/// What Bitbucket Cloud allows an hour when it does not say otherwise.
pub const HOURLY_LIMIT: u32 = 1000;

/// How long a refusal (429) is waited for when Bitbucket does not say.
const REFUSED_WAIT: Duration = Duration::from_secs(60);

/// What is kept of a step's log: its end, where a failure is.
const LOG_MAX: usize = 256 * 1024;

/// Deployments looked through at most.
const DEPLOYMENT_PAGES: usize = 3;

/// How many of a commit's deployments are given.
const DEPLOYMENTS: usize = 10;

/// What a call answered.
/// The calls of the current hour.
struct Budget {
    limit: u32,
    /// When the hour began (seconds since 1970); 0 before the first call.
    start: u64,
    used: u32,
}

impl Budget {
    fn roll(&mut self, now: u64) {
        if self.start == 0 || now >= self.start + 3600 {
            self.start = now;
            self.used = 0;
        }
    }

    fn rate(&self) -> RateLimit {
        RateLimit {
            limit: self.limit,
            remaining: self.limit.saturating_sub(self.used),
            reset: self.start + 3600,
        }
    }
}

pub struct Bitbucket {
    http: reqwest::Client,
    base: String,
    email: String,
    token: String,
    budget: Mutex<Budget>,
    pace: Pace,
    /// Milliseconds spent slowing down.
    waited: AtomicU64,
    /// Between two looks at a merge that goes on in the background.
    poll: Duration,
}

fn enc(s: &str) -> String {
    escouade_plugin_kit::http::encode(s)
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

fn text_of(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

/// A person as Bitbucket tells it: the nickname, else the display name.
fn person(u: &Value) -> String {
    u["nickname"]
        .as_str()
        .or(u["display_name"].as_str())
        .unwrap_or("?")
        .to_string()
}

impl Bitbucket {
    pub fn new(http: reqwest::Client, base: String, email: String, token: String) -> Self {
        Self {
            http,
            base: base.trim_end_matches('/').to_string(),
            email: email.trim().to_string(),
            token: token.trim().to_string(),
            budget: Mutex::new(Budget {
                limit: HOURLY_LIMIT,
                start: 0,
                used: 0,
            }),
            pace: Pace::default(),
            waited: AtomicU64::new(0),
            poll: Duration::from_secs(1),
        }
    }

    pub fn with_pace(mut self, pace: Pace) -> Self {
        self.pace = pace;
        self
    }

    /// The calls allowed an hour, when it is not the usual thousand.
    pub fn with_limit(self, limit: u32) -> Self {
        self.budget.lock().limit = limit;
        self
    }

    pub fn with_poll(mut self, poll: Duration) -> Self {
        self.poll = poll;
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
            .basic_auth(&self.email, Some(&self.token))
            .header("Accept", "application/json")
            .header("User-Agent", "escouade")
    }

    async fn pause(&self, delay: Duration) {
        if !delay.is_zero() {
            self.waited
                .fetch_add(delay.as_millis() as u64, Ordering::Relaxed);
            escouade_plugin_kit::time::sleep(delay).await;
        }
    }

    /// Waits as long as what remains of the hour's calls asks.
    async fn slow_down(&self) {
        let delay = {
            let mut b = self.budget.lock();
            b.roll(now());
            self.pace.delay(&b.rate(), now())
        };
        self.pause(delay).await;
    }

    /// What Bitbucket's answer says of the limit: its total, and whether the hour is nearly spent.
    fn learn(&self, headers: &HeaderMap) {
        let mut b = self.budget.lock();
        if let Some(limit) =
            header(headers, "x-ratelimit-limit").and_then(|v| v.trim().parse::<u32>().ok())
        {
            if limit > 0 {
                b.limit = limit;
            }
        }
        if header(headers, "x-ratelimit-nearlimit")
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
        {
            // « Near » is a fifth left at most.
            let spent = b.limit - b.limit / 5;
            b.used = b.used.max(spent);
        }
    }

    async fn transmit(&self, rb: reqwest::RequestBuilder) -> Result<Reply> {
        self.slow_down().await;
        {
            let mut b = self.budget.lock();
            b.roll(now());
            b.used = b.used.saturating_add(1);
        }
        let reply = redirect::reply(NAME, rb).await?;
        self.learn(&reply.headers);
        Ok(reply)
    }

    /// Sends a call after the wait the hour's budget asks. A refusal for the limit (429) is waited out, at most
    /// `Pace::max_wait`, then the call is sent once more; if it is refused again the hour is taken as spent.
    async fn attempt(&self, rb: reqwest::RequestBuilder) -> Result<Reply> {
        let again = rb.try_clone();
        let mut reply = self.transmit(rb).await?;
        if reply.status == 429 {
            if let Some(again) = again {
                let asked = header(&reply.headers, "retry-after")
                    .and_then(|v| v.trim().parse::<u64>().ok())
                    .map_or(REFUSED_WAIT, Duration::from_secs);
                self.pause(asked.min(self.pace.max_wait)).await;
                reply = self.transmit(again).await?;
            }
            if reply.status == 429 {
                let mut b = self.budget.lock();
                b.used = b.limit;
            }
        }
        Ok(reply)
    }

    /// A call of the API, as `attempt` sends it, and wherever it is redirected. A status other than a success is an
    /// error that says what Bitbucket said; another host gets neither the credentials nor the body.
    async fn send(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Reply> {
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
        serde_json::from_str(&reply.text).map_err(|_| {
            anyhow!(tr!(
                "{NAME} : réponse illisible",
                "{NAME}: unreadable answer"
            ))
        })
    }

    async fn get(&self, path: &str) -> Result<Value> {
        let reply = self.send(reqwest::Method::GET, path, None).await?;
        Self::json(&reply)
    }

    /// Every value of a list, page after page (`values`, and `next` while there is one: only on Bitbucket's own
    /// address, the credentials go nowhere else).
    async fn values(&self, path: &str) -> Result<Vec<Value>> {
        let mut out = Vec::new();
        let mut url = path.to_string();
        for _ in 0..MAX_PAGES {
            let page = self.get(&url).await?;
            out.extend(page["values"].as_array().cloned().unwrap_or_default());
            match page["next"].as_str() {
                Some(n) if n.starts_with(&self.base) => url = n.to_string(),
                _ => break,
            }
        }
        Ok(out)
    }

    async fn write(&self, path: &str, body: Value) -> Result<Reply> {
        self.send(reqwest::Method::POST, path, Some(&body)).await
    }

    fn repo_path(repo: &RepoRef) -> String {
        format!("/repositories/{}/{}", enc(&repo.namespace), enc(&repo.name))
    }

    fn pr_path(repo: &RepoRef, number: u64) -> String {
        format!("{}/pullrequests/{number}", Self::repo_path(repo))
    }

    /// The comments of a pull request, deleted ones left out.
    async fn comment_list(&self, repo: &RepoRef, number: u64) -> Result<Vec<Value>> {
        Ok(self
            .values(&format!(
                "{}/comments?pagelen=100",
                Self::pr_path(repo, number)
            ))
            .await?
            .into_iter()
            .filter(|c| c["deleted"] != true)
            .collect())
    }

    async fn status_list(&self, repo: &RepoRef, number: u64) -> Result<Vec<Check>> {
        let all = self
            .values(&format!(
                "{}/statuses?pagelen=100&sort=-updated_on",
                Self::pr_path(repo, number)
            ))
            .await?;
        Ok(last_statuses(&all))
    }

    /// The statuses of the last commit of a branch (ESC-63); none when the branch is not found.
    async fn branch_status_list(&self, repo: &RepoRef, branch: &str) -> Result<Vec<Check>> {
        let Some(sha) = self.commit_of(repo, branch).await else {
            return Ok(Vec::new());
        };
        let all = self
            .values(&format!(
                "{}/commit/{sha}/statuses?pagelen=100&sort=-updated_on",
                Self::repo_path(repo)
            ))
            .await?;
        Ok(last_statuses(&all))
    }
}

/// The last word of each status (they come newest first).
fn last_statuses(all: &[Value]) -> Vec<Check> {
    let mut seen: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for s in all {
        // The statuses come newest first: a key's first one is its last word.
        let key = s["key"]
            .as_str()
            .or(s["name"].as_str())
            .unwrap_or("?")
            .to_string();
        if seen.contains(&key) {
            continue;
        }
        seen.push(key.clone());
        out.push(check_of(s, &key));
    }
    out
}

impl Bitbucket {
    /// What reading a pull request costs after its own call: its open threads and the statuses of its last commit,
    /// for a pull request still open (the others no longer wait for anything).
    async fn enrich(&self, repo: &RepoRef, pr: &mut PullRequest) -> Result<()> {
        if pr.state != PrState::Open {
            return Ok(());
        }
        pr.open_threads = open_threads(&self.comment_list(repo, pr.number).await?);
        pr.checks = overall(&self.status_list(repo, pr.number).await?);
        Ok(())
    }

    /// The commit a ref (a branch, a tag, or a hash) is at.
    async fn commit_of(&self, repo: &RepoRef, git_ref: &str) -> Option<String> {
        if (7..=40).contains(&git_ref.len()) && git_ref.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(git_ref.to_ascii_lowercase());
        }
        let base = Self::repo_path(repo);
        for kind in ["branches", "tags"] {
            if let Ok(r) = self
                .get(&format!("{base}/refs/{kind}/{}", enc(git_ref)))
                .await
            {
                if let Some(hash) = r["target"]["hash"].as_str() {
                    return Some(hash.to_ascii_lowercase());
                }
            }
        }
        None
    }

    /// Waits for a merge Bitbucket does in the background (a 202 with the address of its task).
    async fn wait_merge(&self, task: &str) -> Result<()> {
        for _ in 0..30 {
            self.pause(self.poll).await;
            let status = self.get(task).await?;
            match status["task_status"].as_str() {
                Some("SUCCESS") => return Ok(()),
                Some("PENDING") | None => {}
                Some(other) => {
                    return Err(anyhow!(tr!(
                        "{NAME} : le merge a échoué ({other})",
                        "{NAME}: the merge failed ({other})"
                    )))
                }
            }
        }
        Err(anyhow!(tr!(
            "{NAME} : le merge n'est pas fini",
            "{NAME}: the merge is not done yet"
        )))
    }
}

/// A commit status as a check. The id of a Pipelines run (the number in the status's address) is what `job_log` asks
/// for; another CI keeps its logs.
fn check_of(s: &Value, key: &str) -> Check {
    let url = s["url"]
        .as_str()
        .filter(|u| !u.is_empty())
        .map(str::to_string);
    let (state, cancelled) = match s["state"].as_str() {
        Some("SUCCESSFUL") => (CheckState::Success, false),
        Some("FAILED") => (CheckState::Failure, false),
        Some("STOPPED") => (CheckState::Failure, true),
        _ => (CheckState::Pending, false),
    };
    let ended = state != CheckState::Pending;
    Check {
        name: s["name"]
            .as_str()
            .filter(|n| !n.is_empty())
            .unwrap_or(key)
            .to_string(),
        state,
        job: url.as_deref().and_then(pipeline_of),
        url,
        started: instant(&s["created_on"]),
        finished: ended.then(|| instant(&s["updated_on"])).flatten(),
        cancelled,
    }
}

/// The run of Bitbucket Pipelines an address names (`…/pipelines/results/12`, `…/addon/pipelines/home#!/results/12`).
fn pipeline_of(url: &str) -> Option<String> {
    if !url.contains("pipelines") {
        return None;
    }
    let (_, rest) = url.rsplit_once("/results/")?;
    let n: String = rest.chars().take_while(char::is_ascii_digit).collect();
    (!n.is_empty()).then_some(n)
}

/// The checks of a commit in one word: a failure wins, then a wait; none when it has none.
fn overall(checks: &[Check]) -> Option<CheckState> {
    if checks.is_empty() {
        None
    } else if checks.iter().any(|c| c.state == CheckState::Failure) {
        Some(CheckState::Failure)
    } else if checks.iter().any(|c| c.state == CheckState::Pending) {
        Some(CheckState::Pending)
    } else {
        Some(CheckState::Success)
    }
}

/// The threads of comments on lines that nobody resolved: the first comment of each, a reply being part of its thread.
fn open_threads(comments: &[Value]) -> u32 {
    comments
        .iter()
        .filter(|c| !c["inline"].is_null() && c["parent"].is_null() && c["resolution"].is_null())
        .count() as u32
}

/// A pull request as Bitbucket tells it. What costs other calls (its threads, its checks) is left: `enrich`.
fn pr_of(p: &Value) -> Option<PullRequest> {
    let number = p["id"].as_u64()?;
    let participants: Vec<&Value> = p["participants"].as_array().into_iter().flatten().collect();
    let source = p["source"]["repository"]["full_name"]
        .as_str()
        .unwrap_or("");
    let dest = p["destination"]["repository"]["full_name"]
        .as_str()
        .unwrap_or("");
    let fork = !source.is_empty() && !dest.is_empty() && source != dest;
    Some(PullRequest {
        number,
        url: p["links"]["html"]["href"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        title: text_of(&p["title"]),
        state: match p["state"].as_str() {
            Some("MERGED") => PrState::Merged,
            Some("DECLINED" | "SUPERSEDED") => PrState::Closed,
            _ => PrState::Open,
        },
        draft: p["draft"].as_bool().unwrap_or(false),
        head: text_of(&p["source"]["branch"]["name"]),
        base: text_of(&p["destination"]["branch"]["name"]),
        // Bitbucket does not say whether a pull request conflicts before one tries to merge it.
        mergeable: None,
        approvals: participants
            .iter()
            .filter(|u| u["approved"] == true)
            .count() as u32,
        changes_requested: participants
            .iter()
            .any(|u| u["state"] == "changes_requested"),
        author: person(&p["author"]),
        // A reviewer who has not said yet.
        reviewers: participants
            .iter()
            .filter(|u| u["role"] == "REVIEWER" && u["approved"] != true && u["state"].is_null())
            .map(|u| person(&u["user"]))
            .collect(),
        created: instant(&p["created_on"]).unwrap_or(0),
        fork,
        head_repo: if fork {
            source.to_string()
        } else {
            String::new()
        },
        merge_commit: text_of(&p["merge_commit"]["hash"]),
        ..Default::default()
    })
}

/// Bitbucket's name of a strategy.
fn strategy_name(strategy: MergeStrategy) -> &'static str {
    match strategy {
        MergeStrategy::Merge => "merge_commit",
        MergeStrategy::Squash => "squash",
        MergeStrategy::Rebase => "rebase_fast_forward",
        MergeStrategy::SemiLinear => "rebase_merge",
    }
}

/// A deployment's state, in the words of the other hosts.
/// What a deployment tells besides its environment, its state and its address: its commit, its
/// date and its pipeline (Bitbucket does not say who deployed).
fn deployment_facts(d: &Value) -> Deployment {
    Deployment {
        sha: text_of(&d["release"]["commit"]["hash"]),
        created: deployed_at(d),
        log: d["release"]["url"]
            .as_str()
            .filter(|u| !u.is_empty())
            .map(str::to_string),
        ..Default::default()
    }
}

/// When a deployment began, else when it ended, else when its release was made.
fn deployed_at(d: &Value) -> u64 {
    [
        &d["state"]["started_on"],
        &d["state"]["completed_on"],
        &d["release"]["created_on"],
    ]
    .into_iter()
    .find_map(instant)
    .unwrap_or(0)
}

/// The deployments of a repository as `environments` gives them (ESC-67): newest first, of each
/// environment those down to its newest success. Its environments have no address on Bitbucket.
pub fn environments_of(environments: &[Value], mut all: Vec<Value>) -> Vec<Deployment> {
    all.sort_by_key(|d| std::cmp::Reverse(deployed_at(d)));
    let name_of = |uuid: &str| {
        environments
            .iter()
            .find(|e| e["uuid"].as_str() == Some(uuid))
            .and_then(|e| e["name"].as_str())
            .unwrap_or("?")
            .to_string()
    };
    let mut out = Vec::new();
    let mut settled: Vec<String> = Vec::new();
    for d in &all {
        let environment = name_of(d["environment"]["uuid"].as_str().unwrap_or(""));
        if settled.contains(&environment) {
            continue;
        }
        let state = deployment_state(d).to_string();
        if state == "success" {
            settled.push(environment.clone());
        }
        out.push(Deployment {
            environment,
            state,
            url: None,
            ..deployment_facts(d)
        });
    }
    out
}

fn deployment_state(d: &Value) -> &'static str {
    match d["state"]["name"].as_str() {
        Some("COMPLETED") => match d["state"]["status"]["name"].as_str() {
            Some("SUCCESSFUL") => "success",
            Some("FAILED") => "failure",
            _ => "inactive",
        },
        Some("IN_PROGRESS") => "in_progress",
        Some("UNDEPLOYED") => "inactive",
        _ => "pending",
    }
}

fn not_found(number: u64) -> anyhow::Error {
    anyhow!(tr!(
        "{NAME} : PR #{number} introuvable",
        "{NAME}: pull request #{number} not found"
    ))
}

fn is_status(e: &anyhow::Error, status: u16) -> bool {
    e.downcast_ref::<HttpError>()
        .is_some_and(|h| h.status == status)
}

/// The end of `s`, `max` bytes at most.
fn tail(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut from = s.len() - max;
    while !s.is_char_boundary(from) {
        from += 1;
    }
    &s[from..]
}

/// A reviewer as the creation of a pull request wants it: `{uuid}` or an account id.
fn reviewer(who: &str) -> Value {
    if who.starts_with('{') {
        json!({ "uuid": who })
    } else {
        json!({ "account_id": who })
    }
}

/// What the target branch of a pull request allows (`destination.branch.merge_strategies`, by its branching model);
/// none told, the four. Bitbucket has no auto-merge: Escouade waits for the checks and merges itself.
pub fn merge_options_of(pr: &Value) -> MergeOptions {
    let all = [
        MergeStrategy::Merge,
        MergeStrategy::Squash,
        MergeStrategy::Rebase,
        MergeStrategy::SemiLinear,
    ];
    let strategies = match pr["destination"]["branch"]["merge_strategies"].as_array() {
        Some(told) => all
            .into_iter()
            .filter(|s| told.iter().any(|t| t == strategy_name(*s)))
            .collect(),
        None => all.to_vec(),
    };
    MergeOptions {
        strategies,
        auto_merge: false,
    }
}

impl Forge for Bitbucket {
    fn me(&self) -> BoxFuture<'_, Result<Identity>> {
        Box::pin(async move {
            let me = self.get("/user").await?;
            let user = me["account_id"]
                .as_str()
                .or(me["uuid"].as_str())
                .unwrap_or("?")
                .to_string();
            let name = me["nickname"]
                .as_str()
                .or(me["display_name"].as_str())
                .unwrap_or(&self.email);
            Ok(Identity {
                label: format!("@{name}"),
                user,
                // Bitbucket does not list a token's rights.
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
            let mut body = json!({
                "title": pr.title,
                "description": description,
                "source": { "branch": { "name": pr.head } },
                "destination": { "branch": { "name": pr.base } },
            });
            if pr.draft {
                body["draft"] = json!(true);
            }
            let path = format!("{}/pullrequests", Self::repo_path(repo));
            let made = if pr.reviewers.is_empty() {
                self.write(&path, body).await?
            } else {
                let mut with = body.clone();
                with["reviewers"] = pr.reviewers.iter().map(|r| reviewer(r)).collect();
                // A reviewer Bitbucket does not know (a login is no account id) refuses the whole request: the pull
                // request is opened without reviewers rather than not at all.
                match self.write(&path, with).await {
                    Ok(r) => r,
                    Err(e) if is_status(&e, 400) => self.write(&path, body).await?,
                    Err(e) => return Err(e),
                }
            };
            // Bitbucket has no labels on pull requests: `pr.labels` goes nowhere.
            pr_of(&Self::json(&made)?).ok_or_else(|| {
                anyhow!(tr!(
                    "{NAME} : réponse illisible",
                    "{NAME}: unreadable answer"
                ))
            })
        })
    }

    fn pull_request<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
    ) -> BoxFuture<'a, Result<PullRequest>> {
        Box::pin(async move {
            let mut pr = pr_of(&self.get(&Self::pr_path(repo, number)).await?)
                .ok_or_else(|| not_found(number))?;
            self.enrich(repo, &mut pr).await?;
            Ok(pr)
        })
    }

    fn pull_requests_of_branch<'a>(
        &'a self,
        repo: &'a RepoRef,
        branch: &'a str,
    ) -> BoxFuture<'a, Result<Vec<PullRequest>>> {
        Box::pin(async move {
            let q = format!(
                "source.branch.name=\"{}\"",
                branch.replace('\\', "\\\\").replace('"', "\\\"")
            );
            let page = self
                .get(&format!(
                    "{}/pullrequests?q={}&state=OPEN&state=MERGED&state=DECLINED&state=SUPERSEDED&sort=-created_on&pagelen=10",
                    Self::repo_path(repo),
                    enc(&q)
                ))
                .await?;
            let mut out: Vec<PullRequest> = page["values"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(pr_of)
                .collect();
            for pr in &mut out {
                self.enrich(repo, pr).await?;
            }
            Ok(out)
        })
    }

    fn parent<'a>(&'a self, repo: &'a RepoRef) -> BoxFuture<'a, Result<Option<RepoRef>>> {
        Box::pin(async move {
            let r = self.get(&Self::repo_path(repo)).await?;
            Ok(r["parent"]["full_name"].as_str().and_then(|full| {
                let (namespace, name) = full.split_once('/')?;
                Some(RepoRef {
                    namespace: namespace.to_string(),
                    name: name.to_string(),
                })
            }))
        })
    }

    fn open_pull_requests<'a>(
        &'a self,
        repo: &'a RepoRef,
    ) -> BoxFuture<'a, Result<Vec<PullRequest>>> {
        Box::pin(async move {
            let page = self
                .get(&format!(
                    "{}/pullrequests?state=OPEN&sort=-updated_on&pagelen={OPEN_PRS}",
                    Self::repo_path(repo)
                ))
                .await?;
            let mut out: Vec<PullRequest> = page["values"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(pr_of)
                .take(OPEN_PRS)
                .collect();
            for pr in &mut out {
                self.enrich(repo, pr).await?;
            }
            Ok(out)
        })
    }

    /// One after the other, Bitbucket having no call for several: a pull request that is gone is left out.
    fn pull_requests<'a>(
        &'a self,
        repo: &'a RepoRef,
        numbers: &'a [u64],
    ) -> BoxFuture<'a, Result<Vec<PullRequest>>> {
        Box::pin(async move {
            let mut out = Vec::new();
            for n in numbers {
                match self.pull_request(repo, *n).await {
                    Ok(pr) => out.push(pr),
                    Err(e) if is_status(&e, 404) => {}
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
            let reply = self
                .write(
                    &format!("{}/merge", Self::pr_path(repo, number)),
                    json!({ "type": "pullrequest_merge_parameters", "merge_strategy": strategy_name(strategy) }),
                )
                .await?;
            // A long merge is answered 202, with where to look at it.
            if reply.status == 202 {
                if let Some(task) = header(&reply.headers, "location") {
                    if task.starts_with(&self.base) {
                        return self.wait_merge(task).await;
                    }
                }
            }
            Ok(())
        })
    }

    fn merge_options<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
    ) -> BoxFuture<'a, Result<MergeOptions>> {
        Box::pin(async move {
            Ok(merge_options_of(
                &self.get(&Self::pr_path(repo, number)).await?,
            ))
        })
    }

    // `auto_merge` is not here: Bitbucket's API has none (`Unsupported`, and the capability is declared absent).

    fn checks<'a>(&'a self, repo: &'a RepoRef, number: u64) -> BoxFuture<'a, Result<Vec<Check>>> {
        Box::pin(async move { self.status_list(repo, number).await })
    }

    /// The statuses of the last commit of a branch. Its failed jobs do not run again from here:
    /// Bitbucket Pipelines' rerun is to verify (`Unsupported`).
    fn branch_checks<'a>(
        &'a self,
        repo: &'a RepoRef,
        branch: &'a str,
    ) -> BoxFuture<'a, Result<Vec<Check>>> {
        Box::pin(async move { self.branch_status_list(repo, branch).await })
    }

    /// The logs of the steps of a Pipelines run, one after the other under the step's name (`job` is the run's number,
    /// or `<run>/<step uuid>` for one step); the end of each when it is long.
    fn job_log<'a>(&'a self, repo: &'a RepoRef, job: &'a str) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            let (run, step) = match job.split_once('/') {
                Some((run, step)) => (run, Some(step)),
                None => (job, None),
            };
            let run_path = format!("{}/pipelines/{}", Self::repo_path(repo), enc(run));
            let steps: Vec<(String, String)> = match step {
                Some(step) => vec![(step.to_string(), String::new())],
                None => self
                    .values(&format!("{run_path}/steps?pagelen=100"))
                    .await?
                    .iter()
                    .filter_map(|s| {
                        let uuid = s["uuid"].as_str()?.to_string();
                        let result = s["state"]["result"]["name"]
                            .as_str()
                            .or(s["state"]["name"].as_str())
                            .unwrap_or("?");
                        Some((
                            uuid,
                            format!("{} ({result})", s["name"].as_str().unwrap_or("?")),
                        ))
                    })
                    .collect(),
            };
            let mut out = String::new();
            for (uuid, title) in &steps {
                let log = match self
                    .send(
                        reqwest::Method::GET,
                        &format!("{run_path}/steps/{}/log", enc(uuid)),
                        None,
                    )
                    .await
                {
                    Ok(reply) => reply.text,
                    // A step that did not run has no log.
                    Err(e) if is_status(&e, 404) => continue,
                    Err(e) => return Err(e),
                };
                if !title.is_empty() {
                    out.push_str(&format!("=== {title} ===\n"));
                }
                out.push_str(tail(&log, LOG_MAX));
                if !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            Ok(out)
        })
    }

    fn reviews<'a>(&'a self, repo: &'a RepoRef, number: u64) -> BoxFuture<'a, Result<Vec<Review>>> {
        Box::pin(async move {
            let pr = self.get(&Self::pr_path(repo, number)).await?;
            // A reviewer's word is an approval or a request for changes; a comment is told by `comments`.
            Ok(pr["participants"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|u| {
                    let state = if u["state"] == "changes_requested" {
                        ReviewState::ChangesRequested
                    } else if u["approved"] == true {
                        ReviewState::Approved
                    } else {
                        return None;
                    };
                    Some(Review {
                        author: person(&u["user"]),
                        state,
                        body: String::new(),
                        created: 0,
                    })
                })
                .collect())
        })
    }

    fn comments<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
    ) -> BoxFuture<'a, Result<Vec<ReviewComment>>> {
        Box::pin(async move {
            let all = self.comment_list(repo, number).await?;
            // A thread is resolved when its first comment is; its replies go with it.
            let resolved = |c: &Value| {
                let root = c["parent"]["id"]
                    .as_u64()
                    .and_then(|p| all.iter().find(|x| x["id"].as_u64() == Some(p)))
                    .unwrap_or(c);
                !root["resolution"].is_null()
            };
            Ok(all
                .iter()
                .map(|c| ReviewComment {
                    id: c["id"].as_u64().map(|i| i.to_string()).unwrap_or_default(),
                    author: person(&c["user"]),
                    body: text_of(&c["content"]["raw"]),
                    path: c["inline"]["path"].as_str().map(str::to_string),
                    line: c["inline"]["to"]
                        .as_u64()
                        .or(c["inline"]["from"].as_u64())
                        .map(|l| l as u32),
                    resolved: resolved(c),
                    created: instant(&c["created_on"]).unwrap_or(0),
                    excerpt: None,
                })
                .collect())
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
                &format!("{}/comments", Self::pr_path(repo, number)),
                json!({ "content": { "raw": body } }),
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
            let comments = format!("{}/comments", Self::pr_path(repo, number));
            for l in &review.lines {
                self.write(
                    &comments,
                    json!({ "content": { "raw": l.body }, "inline": { "path": l.path, "to": l.line } }),
                )
                .await?;
            }
            if !review.body.trim().is_empty() {
                self.write(&comments, json!({ "content": { "raw": review.body } }))
                    .await?;
            }
            let verdict = match review.state {
                ReviewState::Approved => Some("approve"),
                ReviewState::ChangesRequested => Some("request-changes"),
                ReviewState::Commented => None,
            };
            if let Some(verdict) = verdict {
                self.write(
                    &format!("{}/{verdict}", Self::pr_path(repo, number)),
                    json!({}),
                )
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
            let parent: u64 = comment.parse().map_err(|_| {
                anyhow!(tr!(
                    "{NAME} : commentaire inconnu ({comment})",
                    "{NAME}: unknown comment ({comment})"
                ))
            })?;
            self.write(
                &format!("{}/comments", Self::pr_path(repo, number)),
                json!({ "content": { "raw": body }, "parent": { "id": parent } }),
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
                &format!(
                    "{}/comments/{}/resolve",
                    Self::pr_path(repo, number),
                    enc(comment)
                ),
                json!({}),
            )
            .await?;
            Ok(())
        })
    }

    fn rate_limit(&self) -> Option<RateLimit> {
        let mut b = self.budget.lock();
        if b.start == 0 {
            return None;
        }
        b.roll(now());
        Some(b.rate())
    }

    /// The deployments of a commit (the ref's), each with its environment's name.
    fn deployments<'a>(
        &'a self,
        repo: &'a RepoRef,
        git_ref: &'a str,
    ) -> BoxFuture<'a, Result<Vec<Deployment>>> {
        Box::pin(async move {
            let Some(sha) = self.commit_of(repo, git_ref).await else {
                return Ok(Vec::new());
            };
            let base = Self::repo_path(repo);
            let environments = self
                .values(&format!("{base}/environments?pagelen=100"))
                .await?;
            let name_of = |uuid: &str| {
                environments
                    .iter()
                    .find(|e| e["uuid"].as_str() == Some(uuid))
                    .and_then(|e| e["name"].as_str())
                    .unwrap_or("?")
                    .to_string()
            };
            let mut out = Vec::new();
            let mut url = format!("{base}/deployments?pagelen=100");
            for _ in 0..DEPLOYMENT_PAGES {
                let page = self.get(&url).await?;
                for d in page["values"].as_array().into_iter().flatten() {
                    let on = d["release"]["commit"]["hash"].as_str().unwrap_or("");
                    if on.is_empty() || !(on.starts_with(sha.as_str()) || sha.starts_with(on)) {
                        continue;
                    }
                    out.push(Deployment {
                        environment: name_of(d["environment"]["uuid"].as_str().unwrap_or("")),
                        state: deployment_state(d).to_string(),
                        url: d["release"]["url"]
                            .as_str()
                            .filter(|u| !u.is_empty())
                            .map(str::to_string),
                        ..deployment_facts(d)
                    });
                }
                match page["next"].as_str() {
                    Some(n) if n.starts_with(&self.base) && out.len() < DEPLOYMENTS => {
                        url = n.to_string()
                    }
                    _ => break,
                }
            }
            out.truncate(DEPLOYMENTS);
            Ok(out)
        })
    }

    fn environments<'a>(&'a self, repo: &'a RepoRef) -> BoxFuture<'a, Result<Vec<Deployment>>> {
        Box::pin(async move {
            let base = Self::repo_path(repo);
            let environments = self
                .values(&format!("{base}/environments?pagelen=100"))
                .await?;
            let mut all = Vec::new();
            let mut url = format!("{base}/deployments?pagelen=100");
            for _ in 0..DEPLOYMENT_PAGES {
                let page = self.get(&url).await?;
                all.extend(page["values"].as_array().into_iter().flatten().cloned());
                match page["next"].as_str() {
                    Some(n) if n.starts_with(&self.base) => url = n.to_string(),
                    _ => break,
                }
            }
            Ok(environments_of(&environments, all))
        })
    }
}

#[cfg(test)]
#[path = "pulls_tests.rs"]
mod client_tests;
