//! The side panel's « PR et CI » tab (ESC-305): what the code host says of the pull request of
//! the agent open (its checks, the log of a failed one, its reviews and comments, its
//! deployments), the failed checks run again on a click, and the open pull requests of the
//! project when no agent is open. Read only while the tab is shown: nothing here polls by itself,
//! and nothing is published on the host in the user's name but « Relancer les échecs ».

use crate::agentprs::log_tail;
use crate::core::Core;
use crate::forge::{
    Capabilities, Check, Deployment, Forge, PullRequest, RateLimit, RepoRef, Review, ReviewComment,
    Unsupported,
};
use crate::integrations::HttpError;
use anyhow::Result;
use serde::Serialize;
use std::sync::Arc;
use tauri::Runtime;

/// How much of a job's log the tab shows: its end, where the failure is told.
pub const LOG_VIEW: usize = 20_000;

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
    /// Réglages › Intégrations settles it (a host unknown, no account).
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
    fn ok(data: T) -> Self {
        Self {
            data: Some(data),
            failure: None,
        }
    }
    fn failed(e: &anyhow::Error, limit: Option<RateLimit>) -> Self {
        Self {
            data: None,
            failure: Some(failure(e, limit)),
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
}

impl From<&Capabilities> for Can {
    fn from(c: &Capabilities) -> Self {
        Self {
            checks: c.checks,
            job_logs: c.job_logs,
            rerun: c.rerun,
            reviews: c.reviews,
            deployments: c.deployments,
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
}

/// Why an agent has no « Ouvrir une PR… »: the reason, and whether Réglages › Intégrations is
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

/// A call the host's client does not do: nothing to show, without an error.
fn or_none<T>(read: Result<Vec<T>>) -> Result<Vec<T>> {
    match read {
        Err(e) if e.downcast_ref::<Unsupported>().is_some() => Ok(Vec::new()),
        other => other,
    }
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
        let (checks, reviews, comments, deployments) = tokio::join!(
            async {
                match can.checks {
                    true => or_none(again!(client.checks(&repo, number))),
                    false => Ok(Vec::new()),
                }
            },
            async {
                match can.reviews {
                    true => or_none(again!(client.reviews(&repo, number))),
                    false => Ok(Vec::new()),
                }
            },
            async {
                match can.reviews {
                    true => or_none(again!(client.comments(&repo, number))),
                    false => Ok(Vec::new()),
                }
            },
            async {
                // A fork's branch is deployed in its own repository: none of them here.
                match can.deployments && !pr.fork {
                    true => or_none(again!(client.deployments(&repo, &pr.head))),
                    false => Ok(Vec::new()),
                }
            },
        );
        let detail = (|| {
            Ok::<_, anyhow::Error>(PrDetail {
                checks: checks?,
                reviews: reviews?,
                comments: comments?,
                deployments: deployments?,
                can: can.clone(),
                rate_limit: client.rate_limit(),
            })
        })();
        match detail {
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

    /// The project's host is unknown, or has no account: Réglages › Intégrations settles it.
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
            Ok(prs) => Read::ok(ProjectPrs {
                word: provider.request_word().to_string(),
                prs,
            }),
            Err(e) => Read::failed(&e, client.rate_limit()),
        }
    }
}
