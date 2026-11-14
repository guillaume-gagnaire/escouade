//! The side panel's « PR et CI » tab (ESC-305): what the code host says of the pull request of
//! the agent open (its checks, the log of a failed one, its reviews and comments, its
//! deployments), the failed checks run again on a click, and the open pull requests of the
//! project when no agent is open. Read only while the tab is shown: nothing here polls by itself,
//! and nothing is published on the host in the user's name but « Relancer les échecs ».

use crate::agentprs::log_tail;
use crate::core::Core;
use crate::forge::{
    Capabilities, Check, Deployment, Forge, PullRequest, RateLimit, Reading, RepoRef, Review,
    ReviewComment, Unsupported,
};
use crate::integrations::HttpError;
use crate::model::now_ms;
use crate::prnews::limit_near;
use anyhow::Result;
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Runtime;

/// How much of a job's log the tab shows: its end, where the failure is told.
pub const LOG_VIEW: usize = 20_000;

/// How long a project's list is taken again rather than read (the view reads it every minute).
const LIST_FRESH: Duration = Duration::from_secs(55);
/// How often the list of the project on screen is read in the background, for the PR tab's badge
/// of the new ones (ESC-403): never sooner after any other reading of it.
pub const WATCH_EVERY: Duration = Duration::from_secs(120);
/// `WATCH_EVERY`, unless `ESCOUADE_PR_WATCH_EVERY_MS` says otherwise (the end-to-end tests).
fn watch_every() -> Duration {
    std::env::var("ESCOUADE_PR_WATCH_EVERY_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .map_or(WATCH_EVERY, Duration::from_millis)
}
/// The longest wait of the background reading after failures in a row.
const WATCH_LONGEST: Duration = Duration::from_secs(30 * 60);
/// How long a pull request read whole by the tools is given again.
const WHOLE_FRESH: Duration = Duration::from_secs(20);

/// Why the host's answer is missing, as the tab draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FailureKind {
    /// The token was refused (401): expired or revoked.
    Refused,
    /// The host's limit of calls is reached until `reset`.
    RateLimit,
    /// The host did not answer, or answered with a server's error.
    Unreachable,
    /// Anything else the host said (rights, a pull request gone).
    Other,
    /// Not the host's say but the project's: no remote, no client yet, no account.
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    pub kind: FailureKind,
    pub message: String,
    /// When the host's limit starts over (seconds since 1970), for `RateLimit`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset: Option<u64>,
    /// Réglages › Services connectés settles it (a host unknown, no account).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub settings: bool,
}

/// What a read gave: the data, else why it is missing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Read<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<Failure>,
}

impl<T> Read<T> {
    pub(crate) fn ok(data: T) -> Self {
        Self {
            data: Some(data),
            failure: None,
        }
    }
    pub(crate) fn failed(e: &anyhow::Error, limit: Option<RateLimit>) -> Self {
        Self {
            data: None,
            failure: Some(failure(e, limit)),
        }
    }
    /// Not asked of the host: its limit of calls is near.
    pub(crate) fn limited(reset: Option<u64>) -> Self {
        let message = tr!(
            "Limite d'API de l'hébergeur presque atteinte : la lecture attend.",
            "The host's API limit is nearly reached: the reading waits."
        );
        Self {
            data: None,
            failure: Some(Failure {
                kind: FailureKind::RateLimit,
                message,
                reset,
                settings: false,
            }),
        }
    }
}

/// The kind of a host's error, told by its status and by what the client last learned of its
/// limit (`limit`): a refused token, the limit reached (403 or 429 with no call left, or the
/// host's message says so), the host out of reach (no answer, a server's error), or else.
pub fn failure(e: &anyhow::Error, limit: Option<RateLimit>) -> Failure {
    let message = format!("{e:#}");
    let status = e.downcast_ref::<HttpError>().map(|h| h.status);
    let spent = limit.is_some_and(|l| l.remaining == 0);
    let kind = match status {
        Some(401) => FailureKind::Refused,
        Some(403 | 429) if spent || message.to_lowercase().contains("rate limit") => {
            FailureKind::RateLimit
        }
        Some(s) if s >= 500 => FailureKind::Unreachable,
        Some(_) => FailureKind::Other,
        // No status: the host was not reached (or its answer was not read).
        None => FailureKind::Unreachable,
    };
    Failure {
        kind,
        message,
        reset: (kind == FailureKind::RateLimit)
            .then(|| limit.map(|l| l.reset))
            .flatten(),
        settings: false,
    }
}

/// What the host can show of a pull request, as the tab offers it.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Can {
    pub checks: bool,
    pub job_logs: bool,
    pub rerun: bool,
    pub reviews: bool,
    pub deployments: bool,
    /// What it reads of a pull request whole (ESC-452, `prread`): not the tab's.
    #[serde(skip)]
    pub reading: Reading,
}

impl From<&Capabilities> for Can {
    fn from(c: &Capabilities) -> Self {
        Self {
            checks: c.checks,
            job_logs: c.job_logs,
            rerun: c.rerun,
            reviews: c.reviews,
            deployments: c.deployments,
            reading: c.reading,
        }
    }
}

/// The pull request of the agent open, as its tab shows it below its head.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrDetail {
    pub checks: Vec<Check>,
    pub reviews: Vec<Review>,
    pub comments: Vec<ReviewComment>,
    pub deployments: Vec<Deployment>,
    pub can: Can,
    /// How much of its calls the host leaves, when it says it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimit>,
}

/// The open pull requests of a project, with its host's word.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPrs {
    pub word: String,
    pub prs: Vec<PullRequest>,
    /// The login of the account the host was read with: « Les miennes » and « Relecture
    /// demandée » (ESC-65); empty when the account does not say it.
    pub me: String,
}

/// Why an agent has no « Ouvrir une PR… »: the reason, and whether Réglages › Services connectés is
/// where it is settled (a host unknown, no account).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoPr {
    pub reason: String,
    pub settings: bool,
}

/// A read made once more when the host did not answer or answered with a server's error: GitHub's
/// check runs answer 500 now and then.
macro_rules! again {
    ($read:expr) => {{
        match $read.await {
            Err(e) if failure(&e, None).kind == FailureKind::Unreachable => $read.await,
            other => other,
        }
    }};
}

/// A project's list of open pull requests, for the overview's group (ESC-68).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewPrs {
    pub project_id: String,
    pub word: String,
    pub prs: Vec<PullRequest>,
    pub me: String,
}

/// A pull request read whole, for `get_pull_request` (ESC-68).
#[derive(Debug, Clone, PartialEq)]
pub struct PrWhole {
    pub word: String,
    pub pr: PullRequest,
    pub detail: PrDetail,
}

/// A call the host's client does not do: nothing to show, without an error.
fn or_none<T>(read: Result<Vec<T>>) -> Result<Vec<T>> {
    match read {
        Err(e) if e.downcast_ref::<Unsupported>().is_some() => Ok(Vec::new()),
        other => other,
    }
}

/// Its checks, its reviews and comments and its deployments, read together.
async fn detail_of(
    client: &dyn Forge,
    repo: &RepoRef,
    number: u64,
    can: Can,
    pr: &PullRequest,
) -> Result<PrDetail> {
    let (checks, reviews, comments, deployments) = tokio::join!(
        async {
            match can.checks {
                true => or_none(again!(client.checks(repo, number))),
                false => Ok(Vec::new()),
            }
        },
        async {
            match can.reviews {
                true => or_none(again!(client.reviews(repo, number))),
                false => Ok(Vec::new()),
            }
        },
        async {
            match can.reviews {
                true => or_none(again!(client.comments(repo, number))),
                false => Ok(Vec::new()),
            }
        },
        async {
            // A fork's branch is deployed in its own repository: none of them here.
            match can.deployments && !pr.fork {
                true => or_none(again!(client.deployments(repo, &pr.head))),
                false => Ok(Vec::new()),
            }
        },
    );
    Ok(PrDetail {
        checks: checks?,
        reviews: reviews?,
        comments: comments?,
        deployments: deployments?,
        can,
        rate_limit: client.rate_limit(),
    })
}

impl<R: Runtime> Core<R> {
    /// The client, the repository and what the host can do for the agent's pull request
    /// `number` (read in `host_repo`, else on the target), with that pull request.
    async fn pr_of_agent(
        &self,
        id: &str,
        number: u64,
        host_repo: &str,
    ) -> Result<(Box<dyn Forge>, RepoRef, Can, PullRequest)> {
        let meta = self.agent(id)?.lock().meta.clone();
        let known = meta
            .prs
            .iter()
            .find(|p| p.number == number && p.host_repo.eq_ignore_ascii_case(host_repo))
            .cloned();
        let (client, repo, _, _) = self.fix_target(id, &meta, known.as_ref()).await?;
        let can = self
            .project_forge(&meta.project_id)
            .await
            .and_then(|r| r.kind)
            .and_then(|k| self.forges.read().get(&k))
            .map(|p| Can::from(&p.capabilities()))
            .unwrap_or_default();
        let pr = match known {
            Some(p) => p,
            None => client.pull_request(&repo, number).await?,
        };
        Ok((client, repo, can, pr))
    }

    /// Its checks, its reviews and comments and its deployments, read together.
    pub async fn agent_pr_detail(
        self: &Arc<Self>,
        id: &str,
        number: u64,
        host_repo: &str,
    ) -> Read<PrDetail> {
        let (client, repo, can, pr) = match self.pr_of_agent(id, number, host_repo).await {
            Ok(t) => t,
            Err(e) => return Read::failed(&e, None),
        };
        match detail_of(&*client, &repo, number, can, &pr).await {
            Ok(d) => Read::ok(d),
            Err(e) => Read::failed(&e, client.rate_limit()),
        }
    }

    /// The end of the log of a check's job (`job`, as `Check::job` says it), cut at a line.
    pub async fn agent_pr_log(
        self: &Arc<Self>,
        id: &str,
        number: u64,
        host_repo: &str,
        job: &str,
    ) -> Read<String> {
        let (client, repo, _, _) = match self.pr_of_agent(id, number, host_repo).await {
            Ok(t) => t,
            Err(e) => return Read::failed(&e, None),
        };
        match again!(client.job_log(&repo, job)) {
            Ok(log) => Read::ok(log_tail(&log, LOG_VIEW)),
            Err(e) => Read::failed(&e, client.rate_limit()),
        }
    }

    /// « Relancer les échecs »: the failed jobs of the pull request's last commit run again, on
    /// the user's click only; how many runs started.
    pub async fn agent_pr_rerun(
        self: &Arc<Self>,
        id: &str,
        number: u64,
        host_repo: &str,
    ) -> Result<usize> {
        let (client, repo, _, _) = self.pr_of_agent(id, number, host_repo).await?;
        client.rerun_failed(&repo, number).await
    }

    /// Why the agent has no « Ouvrir une PR… »; none when it can open one.
    pub async fn agent_no_pr(self: &Arc<Self>, id: &str) -> Result<Option<NoPr>> {
        let project_id = self.agent(id)?.lock().meta.project_id.clone();
        let Err(e) = self.pr_target(id).await else {
            return Ok(None);
        };
        Ok(Some(NoPr {
            reason: format!("{e:#}"),
            settings: self.settled_in_settings(&project_id).await,
        }))
    }

    /// The project's host is unknown, or has no account: Réglages › Services connectés settles it.
    async fn settled_in_settings(&self, project_id: &str) -> bool {
        let Some(remote) = self.project_forge(project_id).await else {
            return false;
        };
        let provider = remote
            .kind
            .as_deref()
            .and_then(|k| self.forges.read().get(k));
        match provider {
            None => true,
            Some(p) => {
                p.capabilities().pull_requests && self.forge_client(&remote.host).await.is_err()
            }
        }
    }

    /// The open pull requests of the project, on the remote of its target.
    pub async fn project_prs(self: &Arc<Self>, project_id: &str) -> Read<ProjectPrs> {
        let (remote, provider, client) = match self.project_host(project_id).await {
            Ok(t) => t,
            Err(e) => {
                return Read {
                    data: None,
                    failure: Some(Failure {
                        kind: FailureKind::Unavailable,
                        message: format!("{e:#}"),
                        reset: None,
                        settings: self.settled_in_settings(project_id).await,
                    }),
                }
            }
        };
        match again!(client.open_pull_requests(&remote.repo)) {
            Ok(prs) => {
                let numbers: Vec<u64> = prs.iter().map(|p| p.number).collect();
                self.pr_reviews_prune(
                    project_id,
                    &numbers,
                    prs.len() < escouade_plugin_protocol::forge::OPEN_PRS,
                );
                let list = ProjectPrs {
                    word: provider.request_word().to_string(),
                    prs,
                    me: self.forge_login(&remote.host),
                };
                self.pr_list_read(project_id, &list, client.rate_limit(), now_ms());
                Read::ok(list)
            }
            Err(e) => {
                let limit = client.rate_limit();
                self.pr_lists
                    .lock()
                    .entry(project_id.to_string())
                    .or_default()
                    .limit = limit;
                Read::failed(&e, limit)
            }
        }
    }

    /// The background reading of the list of the project on screen, for the PR tab's badge
    /// (ESC-403), which the window asks for while it is visible: made only when the list was not
    /// read in `WATCH_EVERY` (by the view, the tab « PR et CI », the overview, the tools or
    /// itself), the host's limit is not near, and after a failure only once the wait is over, the
    /// wait doubling at each one up to `WATCH_LONGEST`. A failure is told to no one: the view says
    /// it when it is opened. Whether it read.
    pub async fn project_prs_watch(self: &Arc<Self>, project_id: &str) -> bool {
        if let Some(list) = self.pr_lists.lock().get(project_id) {
            let recent = list
                .read
                .as_ref()
                .is_some_and(|(at, _, _)| at.elapsed() < watch_every());
            let waiting = list.watch_after.is_some_and(|t| Instant::now() < t);
            if recent || waiting || limit_near(list.limit, (now_ms() / 1000) as u64) {
                return false;
            }
        }
        let failed = self.project_prs(project_id).await.failure.is_some();
        if failed {
            let mut lists = self.pr_lists.lock();
            let list = lists.entry(project_id.to_string()).or_default();
            list.watch_failures = list.watch_failures.saturating_add(1);
            let wait = watch_every()
                .saturating_mul(1 << list.watch_failures.min(4))
                .min(WATCH_LONGEST);
            list.watch_after = Some(Instant::now() + wait);
        }
        true
    }

    /// `project_prs` for what can wait (the overview, the tools): the last list when it is under
    /// `LIST_FRESH`, or when the host's limit of calls is near. And when it was read (ms).
    pub async fn project_prs_recent(
        self: &Arc<Self>,
        project_id: &str,
    ) -> (Read<ProjectPrs>, Option<i64>) {
        if let Some(list) = self.pr_lists.lock().get(project_id) {
            let near = limit_near(list.limit, (now_ms() / 1000) as u64);
            match &list.read {
                Some((at, read_at, prs)) if near || at.elapsed() < LIST_FRESH => {
                    return (Read::ok(prs.clone()), Some(*read_at))
                }
                None if near => return (Read::limited(list.limit.map(|l| l.reset)), None),
                _ => {}
            }
        }
        let read = self.project_prs(project_id).await;
        let at = read.data.is_some().then(now_ms);
        (read, at)
    }

    /// The lists of every project, for the overview (ESC-68): those that could not be read are
    /// left out, the view « Pull requests » says why.
    pub async fn overview_prs(self: &Arc<Self>) -> Vec<OverviewPrs> {
        let ids: Vec<String> = self.projects.read().iter().map(|p| p.id.clone()).collect();
        let reads = ids.into_iter().map(|id| async move {
            let (read, _) = self.project_prs_recent(&id).await;
            read.data.map(|list| OverviewPrs {
                project_id: id,
                word: list.word,
                prs: list.prs,
                me: list.me,
            })
        });
        futures_util::future::join_all(reads)
            .await
            .into_iter()
            .flatten()
            .collect()
    }

    /// What the host says of a pull request of the project, whole (`get_pull_request`, ESC-68):
    /// through the agent that holds it, else its ticket, else on the project's target. Kept
    /// `WHOLE_FRESH`: an agent that asks again costs the host nothing. `host_repo`: the repository
    /// of one that is not on the target, only one an agent or a ticket of the project knows.
    pub async fn project_pr(
        self: &Arc<Self>,
        project_id: &str,
        number: u64,
        host_repo: &str,
    ) -> Read<PrWhole> {
        let key = format!("{project_id}|{}|{number}", host_repo.to_lowercase());
        if let Some((at, whole)) = self.pr_wholes.lock().get(&key) {
            if at.elapsed() < WHOLE_FRESH {
                return Read::ok(whole.clone());
            }
        }
        let read = self.project_pr_read(project_id, number, host_repo).await;
        if let Some(whole) = &read.data {
            let mut kept = self.pr_wholes.lock();
            kept.retain(|_, (at, _)| at.elapsed() < WHOLE_FRESH);
            kept.insert(key, (Instant::now(), whole.clone()));
        }
        read
    }

    /// The account the project's host is read with (a login), as its list last said it.
    pub(crate) fn pr_me(&self, project_id: &str) -> String {
        self.pr_lists
            .lock()
            .get(project_id)
            .and_then(|l| l.read.as_ref())
            .map(|(_, _, list)| list.me.clone())
            .unwrap_or_default()
    }

    /// The client, the repository and what the host can do for that pull request.
    pub(crate) async fn pr_place(
        self: &Arc<Self>,
        project_id: &str,
        number: u64,
        host_repo: &str,
    ) -> Result<(Box<dyn Forge>, RepoRef, Can)> {
        let holder = self.agents.read().values().find_map(|h| {
            let rt = h.lock();
            let held = rt.meta.project_id == project_id
                && !rt.meta.archived
                && rt
                    .meta
                    .prs
                    .iter()
                    .any(|p| p.number == number && p.host_repo.eq_ignore_ascii_case(host_repo));
            held.then(|| rt.meta.id.clone())
        });
        if let Some(id) = holder {
            let (client, repo, can, _) = self.pr_of_agent(&id, number, host_repo).await?;
            return Ok((client, repo, can));
        }
        let ticket = self.tickets.read().iter().find_map(|t| {
            let tp = t.pr.as_ref().filter(|_| t.project_id == project_id)?;
            let full = format!("{}/{}", tp.namespace, tp.name);
            let here = host_repo.is_empty() || full.eq_ignore_ascii_case(host_repo);
            (tp.number == number && here).then(|| tp.clone())
        });
        if let Some(tp) = ticket {
            let (client, _) = self.forge_client(&tp.host).await?;
            let can = self
                .forges
                .read()
                .get(&tp.kind)
                .map(|p| Can::from(&p.capabilities()))
                .unwrap_or_default();
            let repo = RepoRef {
                namespace: tp.namespace,
                name: tp.name,
            };
            return Ok((client, repo, can));
        }
        if !host_repo.is_empty() {
            anyhow::bail!(tr!(
                "#{number} introuvable sur {host_repo}",
                "#{number} not found on {host_repo}"
            ));
        }
        let (remote, provider, client) = self.project_host(project_id).await?;
        Ok((client, remote.repo, Can::from(&provider.capabilities())))
    }

    async fn project_pr_read(
        self: &Arc<Self>,
        project_id: &str,
        number: u64,
        host_repo: &str,
    ) -> Read<PrWhole> {
        let (client, repo, can) = match self.pr_place(project_id, number, host_repo).await {
            Ok(t) => t,
            Err(e) => return Read::failed(&e, None),
        };
        let pr = match again!(client.pull_request(&repo, number)) {
            Ok(pr) => pr,
            Err(e) => return Read::failed(&e, client.rate_limit()),
        };
        // Its detail is four calls more: not on the last of the host's limit.
        let limit = client.rate_limit();
        if limit_near(limit, (now_ms() / 1000) as u64) {
            return Read::limited(limit.map(|l| l.reset));
        }
        let word = self
            .project_forge(project_id)
            .await
            .and_then(|r| r.kind)
            .and_then(|k| self.forges.read().get(&k))
            .map_or("PR", |p| p.request_word())
            .to_string();
        match detail_of(&*client, &repo, number, can, &pr).await {
            Ok(detail) => Read::ok(PrWhole { word, pr, detail }),
            Err(e) => Read::failed(&e, client.rate_limit()),
        }
    }
}
