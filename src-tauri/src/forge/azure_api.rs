//! Azure DevOps' client, on the REST API 7.1 of Azure Repos and Azure Pipelines: pull requests
//! (drafts, reviewers' votes, conflicts), merge by the strategies the branch's policy allows and
//! auto-complete, checks (the build and status policies' evaluations, the statuses, the builds),
//! the builds' logs and their retry, threads of comments, answers and resolution, and a validated
//! review (its comments, then a vote). Written from Microsoft's documentation, tried against a
//! fake server; `real_azure_devops` tries it on a real organization.
//!
//! A refused token comes back as a 401, or as a 203 or a sign-in page (HTML): all three are a
//! refusal. A limit (429, or 503 with `Retry-After`) is waited out once, at most
//! `Pace::max_wait`; the `X-RateLimit-*` headers slow the calls down as for GitHub.

use super::azure::{parts, NAME};
use super::github_api::Pace;
use super::*;
use parking_lot::Mutex;
use serde_json::json;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const VERSION: &str = "api-version=7.1";

/// The policy evaluations' API is still a preview in 7.1.
const PREVIEW: &str = "api-version=7.1-preview.1";

/// The policy types a pull request's checks are: a build, and a status posted by a service.
const BUILD_POLICY: &str = "0609b952-1397-4640-95ec-e00a01b2c241";
const STATUS_POLICY: &str = "cbdc66da-9728-4af8-aada-9a5a32e4a226";
/// The policy that limits the merge strategies of a branch.
const MERGE_POLICY: &str = "fa4e907d-c16b-4a4c-9dfa-4916e5d171ab";

/// Azure refuses a description longer than this.
const MAX_DESCRIPTION: usize = 4000;

fn enc(s: &str) -> String {
    crate::board::encode(s, false)
}

/// `url` with the API's version.
fn v(url: String, version: &str) -> String {
    let sep = if url.contains('?') { '&' } else { '?' };
    format!("{url}{sep}{version}")
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

/// What a call answered.
struct Reply {
    status: u16,
    headers: HeaderMap,
    text: String,
}

fn refusal() -> anyhow::Error {
    crate::integrations::HttpError {
        status: 401,
        message: crate::integrations::http_error_in(crate::i18n::ui(), NAME, 401, ""),
    }
    .into()
}

fn unreadable() -> anyhow::Error {
    anyhow!(tr!(
        "{NAME} : réponse illisible",
        "{NAME}: unreadable answer"
    ))
}

/// A branch's name without `refs/heads/`.
fn branch(r: &Value) -> String {
    let s = r.as_str().unwrap_or("");
    s.strip_prefix("refs/heads/").unwrap_or(s).to_string()
}

fn strategy_name(s: MergeStrategy) -> &'static str {
    match s {
        MergeStrategy::Merge => "noFastForward",
        MergeStrategy::Squash => "squash",
        MergeStrategy::Rebase => "rebase",
        MergeStrategy::SemiLinear => "rebaseMerge",
    }
}

fn strategy_word(s: MergeStrategy) -> &'static str {
    match s {
        MergeStrategy::Merge => "merge",
        MergeStrategy::Squash => "squash",
        MergeStrategy::Rebase => "rebase",
        MergeStrategy::SemiLinear => "semi-linear",
    }
}

/// The strategies a merge strategy policy's settings allow (its former `useSquashMerge` too).
pub fn allowed_by(settings: &Value) -> Vec<MergeStrategy> {
    let keys = [
        ("allowNoFastForward", MergeStrategy::Merge),
        ("allowSquash", MergeStrategy::Squash),
        ("allowRebase", MergeStrategy::Rebase),
        ("allowRebaseMerge", MergeStrategy::SemiLinear),
    ];
    if keys.iter().any(|(k, _)| settings.get(*k).is_some()) {
        return keys
            .iter()
            .filter(|(k, _)| settings[*k] == true)
            .map(|(_, s)| *s)
            .collect();
    }
    if settings["useSquashMerge"] == true {
        vec![MergeStrategy::Squash]
    } else {
        vec![MergeStrategy::Merge]
    }
}

/// The reviewers that are people (a group's vote is its members').
fn people(p: &Value) -> impl Iterator<Item = &Value> {
    p["reviewers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| r["isContainer"] != true)
}

/// A pull request as Azure tells it, without what other calls say (its threads, its checks).
fn pr_of(p: &Value) -> Option<PullRequest> {
    let number = p["pullRequestId"].as_u64()?;
    let votes: Vec<i64> = people(p).filter_map(|r| r["vote"].as_i64()).collect();
    Some(PullRequest {
        number,
        url: p["repository"]["webUrl"]
            .as_str()
            .map(|w| format!("{}/pullrequest/{number}", w.trim_end_matches('/')))
            .unwrap_or_default(),
        title: p["title"].as_str().unwrap_or("").to_string(),
        state: match p["status"].as_str() {
            Some("completed") => PrState::Merged,
            Some("abandoned") => PrState::Closed,
            _ => PrState::Open,
        },
        draft: p["isDraft"].as_bool().unwrap_or(false),
        head: branch(&p["sourceRefName"]),
        base: branch(&p["targetRefName"]),
        mergeable: match p["mergeStatus"].as_str() {
            Some("succeeded") => Some(true),
            Some("conflicts") => Some(false),
            _ => None,
        },
        // 10 approved, 5 approved with suggestions; -5 waiting for the author, -10 rejected.
        approvals: votes.iter().filter(|v| **v >= 5).count() as u32,
        changes_requested: votes.iter().any(|v| *v < 0),
        auto_merge: !p["autoCompleteSetBy"].is_null(),
        fork: !p["forkSource"].is_null(),
        author: p["createdBy"]["displayName"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        created: instant(&p["creationDate"]).unwrap_or(0),
        ..Default::default()
    })
}

/// The checks of a pull request in one word: a failure first, then what still runs.
fn summary(checks: &[Check]) -> Option<CheckState> {
    if checks.is_empty() {
        None
    } else if checks.iter().any(|c| c.state == CheckState::Failure) {
        Some(CheckState::Failure)
    } else if checks.iter().any(|c| c.state == CheckState::Pending) {
        Some(CheckState::Pending)
    } else if checks.iter().all(|c| c.state == CheckState::Skipped) {
        Some(CheckState::Skipped)
    } else {
        Some(CheckState::Success)
    }
}

fn build_state(b: &Value) -> CheckState {
    if b["status"] != "completed" {
        return CheckState::Pending;
    }
    match b["result"].as_str() {
        Some("succeeded" | "partiallySucceeded") => CheckState::Success,
        _ => CheckState::Failure,
    }
}

/// A thread is resolved once someone says so: fixed, closed, won't fix, by design.
fn resolved(status: &Value) -> bool {
    matches!(
        status.as_str(),
        Some("fixed" | "closed" | "wontFix" | "byDesign")
    )
}

/// A thread's comments people wrote (not Azure's own: votes, pushes).
fn written(t: &Value) -> impl Iterator<Item = &Value> {
    t["comments"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| c["commentType"] != "system" && c["isDeleted"] != true)
}

/// The thread and the comment of a `ReviewComment::id` (`<thread>:<comment>`).
fn thread_of(id: &str) -> Result<(&str, &str)> {
    id.split_once(':').ok_or_else(|| {
        anyhow!(tr!(
            "{NAME} : commentaire inconnu ({id})",
            "{NAME}: unknown comment ({id})"
        ))
    })
}

pub struct Azure {
    http: reqwest::Client,
    /// The account's address: an organization's (`https://dev.azure.com/acme`), a Server's
    /// collection's (`https://tfs.acme.dev/tfs/DefaultCollection`), or dev.azure.com's alone.
    url: String,
    /// In place of `https://dev.azure.com`, when it is a fake one.
    api: Option<String>,
    token: String,
    /// An Entra ID token (the `az` CLI's) goes in Bearer; a personal one in Basic.
    bearer: bool,
    pace: Pace,
    limit: Mutex<Option<RateLimit>>,
    waited: AtomicU64,
    /// The token's identity, by organization: who votes, who sets the auto-complete.
    me: Mutex<Vec<(String, String)>>,
}

impl Azure {
    pub fn new(http: reqwest::Client, url: String, api: Option<String>, token: String) -> Self {
        let token = token.trim().to_string();
        // A JWT: three parts, its header JSON's base64 (`eyJ`).
        let bearer = token.starts_with("eyJ") && token.split('.').count() == 3;
        Self {
            http,
            url: url.trim_end_matches('/').to_string(),
            api: api.map(|a| a.trim_end_matches('/').to_string()),
            token,
            bearer,
            pace: Pace::default(),
            limit: Mutex::default(),
            waited: AtomicU64::new(0),
            me: Mutex::default(),
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

    /// The account's address: its scheme and host, and the path segments after them.
    fn split_url(&self) -> (String, Vec<String>) {
        let (scheme, rest) = self.url.split_once("://").unwrap_or(("https", &self.url));
        let mut it = rest.split('/');
        let host = it.next().unwrap_or("");
        (
            format!("{scheme}://{host}"),
            it.filter(|s| !s.is_empty()).map(str::to_string).collect(),
        )
    }

    /// The organization (or collection) the account's address names, if it names one.
    fn own_org(&self) -> Option<String> {
        self.split_url().1.last().map(|s| super::azure::decode(s))
    }

    /// Where an organization (a collection) is: the account's own, or another one beside it.
    pub fn org_url(&self, org: &str) -> String {
        if let Some(api) = &self.api {
            return format!("{api}/{}", enc(org));
        }
        let (root, mut path) = self.split_url();
        path.pop();
        path.push(enc(org));
        format!("{root}/{}", path.join("/"))
    }

    fn project_api(&self, repo: &RepoRef) -> String {
        let (org, project) = parts(repo);
        format!("{}/{}/_apis", self.org_url(org), enc(project))
    }

    fn repo_api(&self, repo: &RepoRef) -> String {
        format!(
            "{}/git/repositories/{}",
            self.project_api(repo),
            enc(&repo.name)
        )
    }

    fn pr_api(&self, repo: &RepoRef, number: u64) -> String {
        format!("{}/pullRequests/{number}", self.repo_api(repo))
    }

    /// Where an organization's identities are: vssps.dev.azure.com, a Server's collection.
    fn identities_url(&self, org: &str) -> String {
        if self.api.is_none() && host_of(&self.url) == super::azure::CLOUD {
            format!("https://vssps.{}/{}", super::azure::CLOUD, enc(org))
        } else {
            self.org_url(org)
        }
    }

    fn request(&self, method: reqwest::Method, url: &str) -> reqwest::RequestBuilder {
        let rb = self
            .http
            .request(method, url)
            .header("Accept", "application/json")
            .header("User-Agent", "escouade");
        if self.bearer {
            rb.bearer_auth(&self.token)
        } else {
            rb.basic_auth("", Some(&self.token))
        }
    }

    async fn pause(&self, delay: Duration) {
        if !delay.is_zero() {
            self.waited
                .fetch_add(delay.as_millis() as u64, Ordering::Relaxed);
            tokio::time::sleep(delay).await;
        }
    }

    fn learn_limit(&self, headers: &HeaderMap) {
        let num = |n: &str| header(headers, n).and_then(|v| v.trim().parse::<u64>().ok());
        if let (Some(limit), Some(remaining), Some(reset)) = (
            num("x-ratelimit-limit"),
            num("x-ratelimit-remaining"),
            num("x-ratelimit-reset"),
        ) {
            *self.limit.lock() = Some(RateLimit {
                limit: limit as u32,
                remaining: remaining as u32,
                reset,
            });
        }
    }

    async fn transmit(&self, rb: reqwest::RequestBuilder) -> Result<Reply> {
        let delay = match *self.limit.lock() {
            Some(rl) => self.pace.delay(&rl, now()),
            None => Duration::ZERO,
        };
        self.pause(delay).await;
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

    /// Sends a call; a limit with its `Retry-After` is waited out (at most `Pace::max_wait`) and
    /// the call sent once more. A refused token (401, 203) and any other failure are errors that
    /// say what Azure said.
    async fn send(&self, rb: reqwest::RequestBuilder) -> Result<Reply> {
        let again = rb.try_clone();
        let mut reply = self.transmit(rb).await?;
        let asked = matches!(reply.status, 429 | 503)
            .then(|| header(&reply.headers, "retry-after"))
            .flatten()
            .and_then(|v| v.trim().parse::<u64>().ok());
        if let (Some(again), Some(seconds)) = (again, asked) {
            self.pause(Duration::from_secs(seconds).min(self.pace.max_wait))
                .await;
            reply = self.transmit(again).await?;
        }
        // Azure's answer to a token it does not take: its sign-in page, with a 203.
        if reply.status == 203 {
            return Err(refusal());
        }
        if !(200..300).contains(&reply.status) {
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
        let text = reply.text.trim();
        if text.is_empty() {
            return Ok(Value::Null);
        }
        // The sign-in page a refused token is sent to.
        if text.starts_with('<') {
            return Err(refusal());
        }
        serde_json::from_str(text).map_err(|_| unreadable())
    }

    async fn get(&self, url: String) -> Result<Value> {
        let reply = self
            .send(self.request(reqwest::Method::GET, &v(url, VERSION)))
            .await?;
        Self::json(&reply)
    }

    /// The items of a list (`value`).
    async fn list(&self, url: String, version: &str) -> Result<Vec<Value>> {
        let reply = self
            .send(self.request(reqwest::Method::GET, &v(url, version)))
            .await?;
        Ok(Self::json(&reply)?["value"]
            .as_array()
            .cloned()
            .unwrap_or_default())
    }

    async fn write(&self, method: reqwest::Method, url: String, body: Value) -> Result<Value> {
        let reply = self
            .send(self.request(method, &v(url, VERSION)).json(&body))
            .await?;
        Self::json(&reply)
    }

    /// A call of Azure Boards (`integrations::azure`), on this account and its pace: `url` with
    /// its API version, `body` sent as its content type (`application/json-patch+json` for a
    /// work item's fields). Its answer's JSON (null when empty), and its headers.
    pub(crate) async fn call(
        &self,
        method: reqwest::Method,
        url: &str,
        body: Option<(&Value, &str)>,
    ) -> Result<(Value, HeaderMap)> {
        let mut rb = self.request(method, url);
        if let Some((b, content_type)) = body {
            rb = rb.header("Content-Type", content_type).body(b.to_string());
        }
        let reply = self.send(rb).await?;
        Ok((Self::json(&reply)?, reply.headers))
    }

    /// The token's identity in `org`: its id and its name.
    async fn identity(&self, org: &str) -> Result<(String, Value)> {
        let data = self
            .send(self.request(
                reqwest::Method::GET,
                &format!("{}/_apis/connectionData", self.org_url(org)),
            ))
            .await?;
        let data = Self::json(&data)?;
        let user = data["authenticatedUser"].clone();
        let id = user["id"].as_str().unwrap_or("").to_string();
        // Without a token Azure answers for an anonymous user: no one.
        let anonymous = user["descriptor"]
            .as_str()
            .is_some_and(|d| d.contains("UnauthenticatedIdentity"))
            || id.is_empty();
        if anonymous {
            return Err(refusal());
        }
        let mut me = self.me.lock();
        me.retain(|(o, _)| !o.eq_ignore_ascii_case(org));
        me.push((org.to_string(), id.clone()));
        Ok((id, user))
    }

    /// The token's id in `org`, asked once.
    async fn my_id(&self, org: &str) -> Result<String> {
        if let Some((_, id)) = self
            .me
            .lock()
            .iter()
            .find(|(o, _)| o.eq_ignore_ascii_case(org))
        {
            return Ok(id.clone());
        }
        Ok(self.identity(org).await?.0)
    }

    async fn pr_value(&self, repo: &RepoRef, number: u64) -> Result<Value> {
        self.get(self.pr_api(repo, number)).await
    }

    async fn threads(&self, repo: &RepoRef, number: u64) -> Result<Vec<Value>> {
        Ok(self
            .list(format!("{}/threads", self.pr_api(repo, number)), VERSION)
            .await?
            .into_iter()
            .filter(|t| t["isDeleted"] != true)
            .collect())
    }

    /// A pull request with its open threads and its checks in one word, for an open one (a
    /// merged or abandoned one is not asked more).
    async fn full(&self, repo: &RepoRef, p: &Value) -> Result<PullRequest> {
        let mut pr = pr_of(p).ok_or_else(unreadable)?;
        if pr.state != PrState::Open {
            return Ok(pr);
        }
        let threads = self.threads(repo, pr.number).await?;
        pr.open_threads = threads
            .iter()
            .filter(|t| {
                !t["threadContext"].is_null()
                    && matches!(t["status"].as_str(), Some("active" | "pending"))
                    && written(t).next().is_some()
            })
            .count() as u32;
        pr.checks = summary(&self.check_list(repo, p).await?);
        Ok(pr)
    }

    /// The checks of a pull request (`p`, as Azure tells it): its build and status policies'
    /// evaluations, its statuses, and the builds of its merge commit no policy holds.
    async fn check_list(&self, repo: &RepoRef, p: &Value) -> Result<Vec<Check>> {
        let number = p["pullRequestId"].as_u64().unwrap_or(0);
        let (org, project) = parts(repo);
        let web = format!("{}/{}", self.org_url(org), enc(project));
        let build_url = |id: u64| format!("{web}/_build/results?buildId={id}");
        let mut out = Vec::new();
        // The builds and the pipelines a build policy holds: not said twice.
        let (mut held, mut held_pipelines) = (BTreeSet::new(), BTreeSet::new());
        // The evaluations hang from the project's id.
        let project_id = p["repository"]["project"]["id"].as_str().unwrap_or(project);
        let artifact = format!("vstfs:///CodeReview/CodeReviewId/{project_id}/{number}");
        let evaluations = self
            .list(
                format!(
                    "{}/policy/evaluations?artifactId={}",
                    self.project_api(repo),
                    enc(&artifact)
                ),
                PREVIEW,
            )
            .await?;
        for e in &evaluations {
            let config = &e["configuration"];
            let kind = config["type"]["id"].as_str().unwrap_or("");
            if kind != BUILD_POLICY && kind != STATUS_POLICY {
                // The reviewers', comments' and work items' policies are the review's.
                continue;
            }
            let build = e["context"]["buildId"].as_u64();
            if let Some(id) = build {
                held.insert(id);
            }
            if let Some(def) = config["settings"]["buildDefinitionId"].as_u64() {
                held_pipelines.insert(def);
            }
            let settings = &config["settings"];
            // A status policy is named as its status is (`<genre>/<name>`).
            let status = settings["statusName"].as_str().map(|n| {
                match settings["statusGenre"].as_str().filter(|g| !g.is_empty()) {
                    Some(g) => format!("{g}/{n}"),
                    None => n.to_string(),
                }
            });
            let name = settings["displayName"]
                .as_str()
                .filter(|s| !s.is_empty())
                .or_else(|| e["context"]["buildDefinitionName"].as_str())
                .map(str::to_string)
                .or(status)
                .or_else(|| config["type"]["displayName"].as_str().map(str::to_string))
                .unwrap_or_else(|| "Policy".into());
            out.push(Check {
                name,
                state: match e["status"].as_str() {
                    Some("approved") => CheckState::Success,
                    Some("rejected" | "broken") => CheckState::Failure,
                    Some("notApplicable") => CheckState::Skipped,
                    _ => CheckState::Pending,
                },
                url: build.map(build_url),
                job: build.map(|b| b.to_string()),
                ..Default::default()
            });
        }
        // The last status of each context.
        let mut statuses: Vec<(String, Check)> = Vec::new();
        for s in self
            .list(format!("{}/statuses", self.pr_api(repo, number)), VERSION)
            .await?
        {
            let ctx = &s["context"];
            let name = match (ctx["genre"].as_str(), ctx["name"].as_str()) {
                (Some(g), Some(n)) if !g.is_empty() => format!("{g}/{n}"),
                (_, Some(n)) => n.to_string(),
                _ => "status".to_string(),
            };
            let check = Check {
                name: name.clone(),
                state: match s["state"].as_str() {
                    Some("succeeded") => CheckState::Success,
                    Some("failed" | "error") => CheckState::Failure,
                    Some("notApplicable") => CheckState::Skipped,
                    _ => CheckState::Pending,
                },
                url: s["targetUrl"].as_str().map(str::to_string),
                ..Default::default()
            };
            statuses.retain(|(n, _)| *n != name);
            statuses.push((name, check));
        }
        // A status policy's evaluation says what its status says: kept once.
        for (_, s) in statuses {
            if !out.iter().any(|c| c.name == s.name) {
                out.push(s);
            }
        }
        // The builds of its merge commit, the newest of each pipeline.
        let builds = self
            .list(
                format!(
                    "{}/build/builds?branchName={}&queryOrder=queueTimeDescending&$top=50",
                    self.project_api(repo),
                    enc(&format!("refs/pull/{number}/merge"))
                ),
                VERSION,
            )
            .await?;
        let mut seen = BTreeSet::new();
        for b in &builds {
            let (Some(id), def) = (b["id"].as_u64(), b["definition"]["id"].as_u64()) else {
                continue;
            };
            let def = def.unwrap_or(0);
            if !seen.insert(def) || held.contains(&id) || held_pipelines.contains(&def) {
                continue;
            }
            out.push(Check {
                name: b["definition"]["name"]
                    .as_str()
                    .unwrap_or("Build")
                    .to_string(),
                state: build_state(b),
                url: b["_links"]["web"]["href"]
                    .as_str()
                    .map(str::to_string)
                    .or_else(|| Some(build_url(id))),
                job: Some(id.to_string()),
                started: instant(&b["startTime"]),
                finished: instant(&b["finishTime"]),
                cancelled: b["result"] == "canceled",
            });
        }
        Ok(out)
    }

    /// The strategies the branch's merge policy allows; none when it has no such policy (or
    /// its policies cannot be read: Azure says then).
    async fn allowed(&self, repo: &RepoRef, p: &Value) -> Option<Vec<MergeStrategy>> {
        let repo_id = p["repository"]["id"].as_str()?;
        let target = p["targetRefName"].as_str()?;
        let configs = self
            .list(
                format!(
                    "{}/git/policy/configurations?repositoryId={}&refName={}&policyType={MERGE_POLICY}",
                    self.project_api(repo),
                    enc(repo_id),
                    enc(target)
                ),
                VERSION,
            )
            .await
            .ok()?;
        let mut allowed: Option<Vec<MergeStrategy>> = None;
        for c in configs
            .iter()
            .filter(|c| c["isEnabled"] != false && c["isDeleted"] != true)
        {
            let these = allowed_by(&c["settings"]);
            allowed = Some(match allowed {
                None => these,
                Some(before) => before.into_iter().filter(|s| these.contains(s)).collect(),
            });
        }
        allowed
    }

    /// The pull request, refused when its branch's policy does not allow `strategy`.
    async fn mergeable_with(
        &self,
        repo: &RepoRef,
        number: u64,
        strategy: MergeStrategy,
    ) -> Result<Value> {
        let p = self.pr_value(repo, number).await?;
        if let Some(allowed) = self.allowed(repo, &p).await {
            if !allowed.contains(&strategy) {
                let asked = strategy_word(strategy);
                let list = allowed
                    .iter()
                    .map(|s| strategy_word(*s))
                    .collect::<Vec<_>>()
                    .join(", ");
                bail_refused(tr!(
                    "{NAME} : la branche cible ne permet pas ce merge ({asked}) ; elle permet : {list}",
                    "{NAME}: the target branch doesn’t allow this merge ({asked}); it allows: {list}"
                ))?;
            }
        }
        Ok(p)
    }
}

impl Azure {
    /// Abandons a pull request (what the real trial leaves behind it).
    pub async fn abandon(&self, repo: &RepoRef, number: u64) -> Result<()> {
        self.write(
            reqwest::Method::PATCH,
            self.pr_api(repo, number),
            json!({ "status": "abandoned" }),
        )
        .await?;
        Ok(())
    }
}

fn bail_refused(text: String) -> Result<()> {
    Err(crate::integrations::Refused(text).into())
}

impl Forge for Azure {
    /// The organization's `connectionData` (who the token is there), else the profile when the
    /// account names no organization.
    fn me(&self) -> BoxFuture<'_, Result<Identity>> {
        Box::pin(async move {
            let (user, label) = match self.own_org() {
                Some(org) => {
                    let (_, u) = self.identity(&org).await?;
                    let name = u["providerDisplayName"].as_str().unwrap_or("?").to_string();
                    let account = u["properties"]["Account"]["$value"]
                        .as_str()
                        .filter(|a| !a.is_empty())
                        .map(str::to_string);
                    (
                        account.clone().unwrap_or(name.clone()),
                        account.unwrap_or(name),
                    )
                }
                None => {
                    let root = match &self.api {
                        Some(api) => api.clone(),
                        None if host_of(&self.url) == super::azure::CLOUD => {
                            "https://app.vssps.visualstudio.com".to_string()
                        }
                        None => self.url.clone(),
                    };
                    let p = self
                        .get(format!("{root}/_apis/profile/profiles/me"))
                        .await?;
                    let name = p["displayName"].as_str().unwrap_or("?").to_string();
                    let mail = p["emailAddress"]
                        .as_str()
                        .filter(|a| !a.is_empty())
                        .map(str::to_string);
                    (mail.clone().unwrap_or(name.clone()), mail.unwrap_or(name))
                }
            };
            Ok(Identity {
                user,
                label,
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
            if description.chars().count() > MAX_DESCRIPTION {
                description = description.chars().take(MAX_DESCRIPTION - 1).collect();
                description.push('…');
            }
            let mut body = json!({
                "sourceRefName": format!("refs/heads/{}", pr.head),
                "targetRefName": format!("refs/heads/{}", pr.base),
                "title": pr.title,
                "description": description,
                "isDraft": pr.draft,
            });
            // The work item it closes once completed.
            if let Some(n) = pr.closes {
                body["workItemRefs"] = json!([{ "id": n.to_string() }]);
            }
            let made = self
                .write(
                    reqwest::Method::POST,
                    format!("{}/pullrequests", self.repo_api(repo)),
                    body,
                )
                .await?;
            let created = pr_of(&made).ok_or_else(unreadable)?;
            let at = self.pr_api(repo, created.number);
            // The pull request exists: a reviewer Azure does not find or refuses, or a label that
            // fails, does not take it back.
            let (org, _) = parts(repo);
            for login in &pr.reviewers {
                let found = self
                    .list(
                        format!(
                            "{}/_apis/identities?searchFilter=General&filterValue={}&queryMembership=None",
                            self.identities_url(org),
                            enc(login)
                        ),
                        VERSION,
                    )
                    .await;
                let Some(id) = found
                    .ok()
                    .and_then(|v| v.first().and_then(|i| i["id"].as_str().map(str::to_string)))
                else {
                    continue;
                };
                let _ = self
                    .write(
                        reqwest::Method::PUT,
                        format!("{at}/reviewers/{}", enc(&id)),
                        json!({ "vote": 0 }),
                    )
                    .await;
            }
            for label in &pr.labels {
                let _ = self
                    .write(
                        reqwest::Method::POST,
                        format!("{at}/labels"),
                        json!({ "name": label }),
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
            let p = self.pr_value(repo, number).await?;
            self.full(repo, &p).await
        })
    }

    fn pull_requests_of_branch<'a>(
        &'a self,
        repo: &'a RepoRef,
        branch: &'a str,
    ) -> BoxFuture<'a, Result<Vec<PullRequest>>> {
        Box::pin(async move {
            let mut found = self
                .list(
                    format!(
                        "{}/pullrequests?searchCriteria.sourceRefName={}&searchCriteria.status=all&$top=10",
                        self.repo_api(repo),
                        enc(&format!("refs/heads/{branch}"))
                    ),
                    VERSION,
                )
                .await?;
            found.sort_by_key(|p| std::cmp::Reverse(p["pullRequestId"].as_u64().unwrap_or(0)));
            let mut out = Vec::new();
            for p in &found {
                out.push(self.full(repo, p).await?);
            }
            Ok(out)
        })
    }

    fn open_pull_requests<'a>(
        &'a self,
        repo: &'a RepoRef,
    ) -> BoxFuture<'a, Result<Vec<PullRequest>>> {
        Box::pin(async move {
            let found = self
                .list(
                    format!(
                        "{}/pullrequests?searchCriteria.status=active&$top={OPEN_PRS}",
                        self.repo_api(repo)
                    ),
                    VERSION,
                )
                .await?;
            let reads =
                futures_util::future::join_all(found.iter().map(|p| self.full(repo, p))).await;
            reads.into_iter().collect()
        })
    }

    /// One call (and its threads and checks) per pull request: Azure reads them one by one. One
    /// gone since (404) is left out.
    fn pull_requests<'a>(
        &'a self,
        repo: &'a RepoRef,
        numbers: &'a [u64],
    ) -> BoxFuture<'a, Result<Vec<PullRequest>>> {
        Box::pin(async move {
            let reads =
                futures_util::future::join_all(numbers.iter().map(|n| self.pull_request(repo, *n)))
                    .await;
            let mut out = Vec::new();
            for read in reads {
                match read {
                    Ok(pr) => out.push(pr),
                    Err(e)
                        if e.downcast_ref::<crate::integrations::HttpError>()
                            .is_some_and(|h| h.status == 404) => {}
                    Err(e) => return Err(e),
                }
            }
            Ok(out)
        })
    }

    /// Completes the pull request with `strategy`, when its branch's policy allows it.
    fn merge<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
        strategy: MergeStrategy,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let p = self.mergeable_with(repo, number, strategy).await?;
            self.write(
                reqwest::Method::PATCH,
                self.pr_api(repo, number),
                json!({
                    "status": "completed",
                    "lastMergeSourceCommit": { "commitId": p["lastMergeSourceCommit"]["commitId"] },
                    "completionOptions": {
                        "mergeStrategy": strategy_name(strategy),
                        "transitionWorkItems": true,
                    },
                }),
            )
            .await?;
            Ok(())
        })
    }

    /// Azure's auto-complete, set by the token's identity.
    fn auto_merge<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
        strategy: MergeStrategy,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            self.mergeable_with(repo, number, strategy).await?;
            let me = self.my_id(parts(repo).0).await?;
            let done = self
                .write(
                    reqwest::Method::PATCH,
                    self.pr_api(repo, number),
                    json!({
                        "autoCompleteSetBy": { "id": me },
                        "completionOptions": {
                            "mergeStrategy": strategy_name(strategy),
                            "transitionWorkItems": true,
                        },
                    }),
                )
                .await?;
            if done["autoCompleteSetBy"].is_null() {
                return Err(anyhow!(tr!(
                    "{NAME} : l'auto-complete n'a pas été activé",
                    "{NAME}: auto-complete was not set"
                )));
            }
            Ok(())
        })
    }

    fn checks<'a>(&'a self, repo: &'a RepoRef, number: u64) -> BoxFuture<'a, Result<Vec<Check>>> {
        Box::pin(async move {
            let p = self.pr_value(repo, number).await?;
            self.check_list(repo, &p).await
        })
    }

    /// The logs of a build's failed tasks (`job` is the build's id), else of its last step.
    fn job_log<'a>(&'a self, repo: &'a RepoRef, job: &'a str) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            let build = format!("{}/build/builds/{}", self.project_api(repo), enc(job));
            let timeline = self.get(format!("{build}/timeline")).await?;
            let records: Vec<&Value> = timeline["records"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|r| r["log"]["id"].is_u64())
                .collect();
            let failed: Vec<&Value> = records
                .iter()
                .copied()
                .filter(|r| r["result"] == "failed" && r["type"] == "Task")
                .collect();
            let chosen: Vec<&Value> = if failed.is_empty() {
                records
                    .iter()
                    .copied()
                    .rfind(|r| r["type"] == "Task")
                    .into_iter()
                    .collect()
            } else {
                failed
            };
            if chosen.is_empty() {
                return Err(anyhow!(tr!(
                    "{NAME} : ce build n'a pas de journal",
                    "{NAME}: this build has no log"
                )));
            }
            let mut out = String::new();
            for r in chosen {
                let id = r["log"]["id"].as_u64().unwrap_or(0);
                let reply = self
                    .send(
                        self.request(
                            reqwest::Method::GET,
                            &v(format!("{build}/logs/{id}"), VERSION),
                        )
                        .header("Accept", "text/plain"),
                    )
                    .await?;
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&format!(
                    "##[section]{}\n",
                    r["name"].as_str().unwrap_or("Task")
                ));
                out.push_str(&reply.text);
            }
            Ok(out)
        })
    }

    /// Retries each failed build (its failed jobs run again); how many.
    fn rerun_failed<'a>(&'a self, repo: &'a RepoRef, number: u64) -> BoxFuture<'a, Result<usize>> {
        Box::pin(async move {
            let p = self.pr_value(repo, number).await?;
            let builds: BTreeSet<String> = self
                .check_list(repo, &p)
                .await?
                .into_iter()
                .filter(|c| c.state == CheckState::Failure)
                .filter_map(|c| c.job)
                .collect();
            for id in &builds {
                self.write(
                    reqwest::Method::PATCH,
                    format!(
                        "{}/build/builds/{}?retry=true",
                        self.project_api(repo),
                        enc(id)
                    ),
                    json!({}),
                )
                .await?;
            }
            Ok(builds.len())
        })
    }

    /// The reviewers' votes: approved (with suggestions too), or waiting for the author and
    /// rejected, both a request for changes.
    fn reviews<'a>(&'a self, repo: &'a RepoRef, number: u64) -> BoxFuture<'a, Result<Vec<Review>>> {
        Box::pin(async move {
            let p = self.pr_value(repo, number).await?;
            Ok(people(&p)
                .filter_map(|r| {
                    let state = match r["vote"].as_i64()? {
                        v if v >= 5 => ReviewState::Approved,
                        v if v < 0 => ReviewState::ChangesRequested,
                        _ => return None,
                    };
                    Some(Review {
                        author: r["displayName"]
                            .as_str()
                            .or_else(|| r["uniqueName"].as_str())
                            .unwrap_or("?")
                            .to_string(),
                        state,
                        body: String::new(),
                    })
                })
                .collect())
        })
    }

    /// Every comment people wrote, in threads on lines or not; its id is `<thread>:<comment>`.
    fn comments<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
    ) -> BoxFuture<'a, Result<Vec<ReviewComment>>> {
        Box::pin(async move {
            let mut out = Vec::new();
            for t in self.threads(repo, number).await? {
                let ctx = &t["threadContext"];
                let path = ctx["filePath"]
                    .as_str()
                    .map(|p| p.trim_start_matches('/').to_string());
                let line = ctx["rightFileStart"]["line"]
                    .as_u64()
                    .or_else(|| ctx["leftFileStart"]["line"].as_u64())
                    .map(|l| l as u32);
                let thread = t["id"].as_u64().unwrap_or(0);
                for c in written(&t) {
                    out.push(ReviewComment {
                        id: format!("{thread}:{}", c["id"].as_u64().unwrap_or(0)),
                        author: c["author"]["displayName"]
                            .as_str()
                            .or_else(|| c["author"]["uniqueName"].as_str())
                            .unwrap_or("?")
                            .to_string(),
                        body: c["content"].as_str().unwrap_or("").to_string(),
                        path: path.clone(),
                        line,
                        resolved: resolved(&t["status"]),
                    });
                }
            }
            Ok(out)
        })
    }

    /// A comment under the description: a thread of its own, without a status to resolve.
    fn comment<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
        body: &'a str,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            self.write(
                reqwest::Method::POST,
                format!("{}/threads", self.pr_api(repo, number)),
                json!({ "comments": [{ "parentCommentId": 0, "content": body, "commentType": 1 }] }),
            )
            .await?;
            Ok(())
        })
    }

    /// Its comments on lines and its text first, each a thread, then the token's vote: 10 to
    /// approve, -5 (waiting for the author) to ask for changes, none for a comment.
    fn review<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
        review: &'a ReviewDraft,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let at = self.pr_api(repo, number);
            for l in &review.lines {
                let spot = json!({ "line": l.line, "offset": 1 });
                self.write(
                    reqwest::Method::POST,
                    format!("{at}/threads"),
                    json!({
                        "comments": [{ "parentCommentId": 0, "content": l.body, "commentType": 1 }],
                        "status": "active",
                        "threadContext": {
                            "filePath": format!("/{}", l.path.trim_start_matches('/')),
                            "rightFileStart": spot,
                            "rightFileEnd": spot,
                        },
                    }),
                )
                .await?;
            }
            if !review.body.trim().is_empty() {
                self.write(
                    reqwest::Method::POST,
                    format!("{at}/threads"),
                    json!({
                        "comments": [{ "parentCommentId": 0, "content": review.body, "commentType": 1 }],
                        "status": "active",
                    }),
                )
                .await?;
            }
            let vote = match review.state {
                ReviewState::Approved => 10,
                ReviewState::ChangesRequested => -5,
                ReviewState::Commented => return Ok(()),
            };
            let me = self.my_id(parts(repo).0).await?;
            self.write(
                reqwest::Method::PUT,
                format!("{at}/reviewers/{}", enc(&me)),
                json!({ "vote": vote }),
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
            let (thread, parent) = thread_of(comment)?;
            self.write(
                reqwest::Method::POST,
                format!(
                    "{}/threads/{}/comments",
                    self.pr_api(repo, number),
                    enc(thread)
                ),
                json!({
                    "content": body,
                    "parentCommentId": parent.parse::<u64>().unwrap_or(1),
                    "commentType": 1,
                }),
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
            let (thread, _) = thread_of(comment)?;
            self.write(
                reqwest::Method::PATCH,
                format!("{}/threads/{}", self.pr_api(repo, number), enc(thread)),
                json!({ "status": "fixed" }),
            )
            .await?;
            Ok(())
        })
    }

    fn rate_limit(&self) -> Option<RateLimit> {
        *self.limit.lock()
    }
}
