//! The contract of a code host's client (docs/PLUGINS.md §4.5): what a plugin's client does for one account, and what
//! the core's adapter of a plugin (`ProcessForge`) and the hosts still in the core do the same way. A call a host
//! cannot do says `Unsupported`, which crosses the protocol as « no such method » (-32601).

pub use escouade_plugin_protocol::forge::*;

use crate::tr;
use anyhow::Result;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// What a client's calls give back.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// An instant of a host's answer (RFC 3339) in seconds since 1970; none when absent or unread.
pub fn instant(v: &Value) -> Option<u64> {
    let t = chrono::DateTime::parse_from_rfc3339(v.as_str()?)
        .ok()?
        .timestamp();
    // Azure's « 0001-01-01T00:00:00 » of a build not started yet.
    u64::try_from(t).ok().filter(|t| *t > 0)
}

/// What a host was asked and cannot do (`Capabilities`), or its client does not do yet.
#[derive(Debug)]
pub struct Unsupported(pub &'static str);

impl std::fmt::Display for Unsupported {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let what = self.0;
        f.write_str(&tr!(
            "Cet hébergeur ne sait pas faire : {what}",
            "This code host can’t do it: {what}"
        ))
    }
}

impl std::error::Error for Unsupported {}

pub fn unsupported<'a, T: Send + 'a>(what: &'static str) -> BoxFuture<'a, Result<T>> {
    Box::pin(async move { Err(Unsupported(what).into()) })
}

/// A client of one host for one account. Its provider's `Capabilities` say which calls it
/// answers; the others say `Unsupported`.
pub trait Forge: Send + Sync {
    /// Who the token is, and its rights when the host tells them.
    fn me(&self) -> BoxFuture<'_, Result<Identity>>;

    fn create_pr<'a>(
        &'a self,
        _repo: &'a RepoRef,
        _pr: &'a PullRequestDraft,
    ) -> BoxFuture<'a, Result<PullRequest>> {
        unsupported("create_pr")
    }

    fn pull_request<'a>(
        &'a self,
        _repo: &'a RepoRef,
        _number: u64,
    ) -> BoxFuture<'a, Result<PullRequest>> {
        unsupported("pull_request")
    }

    /// The pull requests (open, merged or closed) whose branch is `branch`, newest first.
    fn pull_requests_of_branch<'a>(
        &'a self,
        _repo: &'a RepoRef,
        _branch: &'a str,
    ) -> BoxFuture<'a, Result<Vec<PullRequest>>> {
        unsupported("pull_requests_of_branch")
    }

    /// The repository `repo` was forked from; none when it is no fork, or the host does not say.
    fn parent<'a>(&'a self, _repo: &'a RepoRef) -> BoxFuture<'a, Result<Option<RepoRef>>> {
        Box::pin(async { Ok(None) })
    }

    /// The open pull requests of a repository, the most recently updated first (`OPEN_PRS` at
    /// most).
    fn open_pull_requests<'a>(
        &'a self,
        _repo: &'a RepoRef,
    ) -> BoxFuture<'a, Result<Vec<PullRequest>>> {
        unsupported("open_pull_requests")
    }

    /// Several pull requests of a repository, in as few calls as the host allows.
    fn pull_requests<'a>(
        &'a self,
        _repo: &'a RepoRef,
        _numbers: &'a [u64],
    ) -> BoxFuture<'a, Result<Vec<PullRequest>>> {
        unsupported("pull_requests")
    }

    fn merge<'a>(
        &'a self,
        _repo: &'a RepoRef,
        _number: u64,
        _strategy: MergeStrategy,
    ) -> BoxFuture<'a, Result<()>> {
        unsupported("merge")
    }

    /// How the pull request `number` may merge, as the repository's settings say: the strategies they allow, and
    /// whether the host's auto-merge is there for it. Unsupported: the core takes its provider's `Capabilities`.
    fn merge_options<'a>(
        &'a self,
        _repo: &'a RepoRef,
        _number: u64,
    ) -> BoxFuture<'a, Result<MergeOptions>> {
        unsupported("merge_options")
    }

    /// Merge once the checks pass.
    fn auto_merge<'a>(
        &'a self,
        _repo: &'a RepoRef,
        _number: u64,
        _strategy: MergeStrategy,
    ) -> BoxFuture<'a, Result<()>> {
        unsupported("auto_merge")
    }

    fn checks<'a>(&'a self, _repo: &'a RepoRef, _number: u64) -> BoxFuture<'a, Result<Vec<Check>>> {
        unsupported("checks")
    }

    fn job_log<'a>(&'a self, _repo: &'a RepoRef, _job: &'a str) -> BoxFuture<'a, Result<String>> {
        unsupported("job_log")
    }

    fn reviews<'a>(
        &'a self,
        _repo: &'a RepoRef,
        _number: u64,
    ) -> BoxFuture<'a, Result<Vec<Review>>> {
        unsupported("reviews")
    }

    fn comments<'a>(
        &'a self,
        _repo: &'a RepoRef,
        _number: u64,
    ) -> BoxFuture<'a, Result<Vec<ReviewComment>>> {
        unsupported("comments")
    }

    fn comment<'a>(
        &'a self,
        _repo: &'a RepoRef,
        _number: u64,
        _body: &'a str,
    ) -> BoxFuture<'a, Result<()>> {
        unsupported("comment")
    }

    /// Run the failed jobs of a pull request's last commit again; how many runs started.
    fn rerun_failed<'a>(
        &'a self,
        _repo: &'a RepoRef,
        _number: u64,
    ) -> BoxFuture<'a, Result<usize>> {
        unsupported("rerun_failed")
    }

    /// The checks of the last commit of a branch (the target, a branch pushed without a pull
    /// request), as `checks` gives a pull request's (ESC-63).
    fn branch_checks<'a>(
        &'a self,
        _repo: &'a RepoRef,
        _branch: &'a str,
    ) -> BoxFuture<'a, Result<Vec<Check>>> {
        unsupported("branch_checks")
    }

    /// Run the failed jobs of the last commit of a branch again; how many runs started (ESC-63).
    fn rerun_failed_of_branch<'a>(
        &'a self,
        _repo: &'a RepoRef,
        _branch: &'a str,
    ) -> BoxFuture<'a, Result<usize>> {
        unsupported("rerun_failed_of_branch")
    }

    /// Publish a review the user validated.
    fn review<'a>(
        &'a self,
        _repo: &'a RepoRef,
        _number: u64,
        _review: &'a ReviewDraft,
    ) -> BoxFuture<'a, Result<()>> {
        unsupported("review")
    }

    /// Answer under a comment of a line (`ReviewComment::id`).
    fn reply<'a>(
        &'a self,
        _repo: &'a RepoRef,
        _number: u64,
        _comment: &'a str,
        _body: &'a str,
    ) -> BoxFuture<'a, Result<()>> {
        unsupported("reply")
    }

    /// Resolves the thread of a comment of a line (`ReviewComment::id`).
    fn resolve<'a>(
        &'a self,
        _repo: &'a RepoRef,
        _number: u64,
        _comment: &'a str,
    ) -> BoxFuture<'a, Result<()>> {
        unsupported("resolve")
    }

    /// What the host's last answer said of its limit, when it says it.
    fn rate_limit(&self) -> Option<RateLimit> {
        None
    }

    fn deployments<'a>(
        &'a self,
        _repo: &'a RepoRef,
        _git_ref: &'a str,
    ) -> BoxFuture<'a, Result<Vec<Deployment>>> {
        unsupported("deployments")
    }

    /// The recent deployments of the repository, whatever their branch, newest first (ESC-67): of
    /// each environment at least its newest one and its newest one that succeeded (what runs
    /// there), with their commit, their date, their author and their job.
    fn environments<'a>(&'a self, _repo: &'a RepoRef) -> BoxFuture<'a, Result<Vec<Deployment>>> {
        unsupported("environments")
    }
}

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
