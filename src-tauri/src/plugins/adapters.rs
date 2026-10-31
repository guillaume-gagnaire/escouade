//! The in-process adapters of the ticket sources and the code hosts (INT-5, ESC-211;
//! `docs/PLUGINS.md` §4.5): `ProcessTicketSource: TicketSource` and `ProcessForge: Forge` answer
//! the core's calls by JSON-RPC to the plugin's process (`tickets/*`, `forge/*`, `account/me`),
//! so that the rest of the core does not see the difference.
//!
//! What the contract asks of each call, here: a delay (`runtime::timeout_of`), typed errors, and
//! a plugin that does not answer. The plugin says an HTTP error by the code `-(32000 + status)`
//! (401 is `-32401`: « reconnect the account »), a refusal that asking again would not change by
//! `-32010`, and a call its host cannot do by `-32601`. A plugin that crashed, is stopped or is
//! too slow is a `PluginDown`: the sync waits and tries again, the pull requests read last stay.
//! It gets its token itself, by `secrets/get` of `account:<id>`: the adapters never put it in a
//! call, and mask nothing because they carry nothing.

// Nothing in the app asks a plugin for a source or a host yet: the registries read the manifests
// with INT-6, and the tests play these adapters until then.
#![allow(dead_code)]

use super::runtime::{CallError, Runtime, METHOD_NOT_FOUND};
use crate::forge::{
    BoxFuture, Check, Credentials, Deployment, Forge, Identity, MergeStrategy, PullRequest,
    PullRequestDraft, RateLimit, RepoRef, Review, ReviewComment, ReviewDraft, Unsupported,
};
use crate::integrations::source::{TicketSource, TicketSourceKind};
use crate::integrations::{
    Account, Bases, Container, HttpError, IssuePage, Query, Refused, StatesView,
};
use crate::model::{ExternalRef, ExternalState};
use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::sync::Arc;

/// The code with which a plugin says that the service refused for good (`integrations::Refused`).
pub const REFUSED: i64 = -32010;

/// The code with which a plugin says the HTTP error `status` of its service (the Rust plugins of
/// the first hand answer with it; the tests too).
pub fn http_code(status: u16) -> i64 {
    -(32_000 + i64::from(status))
}

/// The status `code` says, when it is one of `http_code`'s.
fn http_status(code: i64) -> Option<u16> {
    u16::try_from(-code - 32_000)
        .ok()
        .filter(|s| (400..600).contains(s))
}

/// The plugin gave no answer: stopped, crashed, too slow, or out of reach. Not a refusal of the
/// service: nothing is lost, and the call goes again later.
#[derive(Debug)]
pub struct PluginDown {
    pub plugin: String,
    pub why: String,
}

impl std::fmt::Display for PluginDown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (plugin, why) = (&self.plugin, &self.why);
        f.write_str(&tr!(
            "Le plugin {plugin} ne répond pas : {why}",
            "The plugin {plugin} is not answering: {why}"
        ))
    }
}

impl std::error::Error for PluginDown {}

/// Whether `e` is a plugin that did not answer.
pub fn is_down(e: &anyhow::Error) -> bool {
    e.downcast_ref::<PluginDown>().is_some()
}

/// What a plugin's failed call is in the core: the typed error its code says, or `PluginDown`.
/// `what` names the call a host that cannot do it is asked for.
fn typed(plugin: &str, e: CallError, what: &'static str) -> anyhow::Error {
    match e {
        CallError::Remote { code, message } => {
            if let Some(status) = http_status(code) {
                HttpError { status, message }.into()
            } else if code == REFUSED {
                Refused(message).into()
            } else if code == METHOD_NOT_FOUND {
                Unsupported(what).into()
            } else {
                anyhow!("{message} ({code})")
            }
        }
        CallError::Invalid(why) => anyhow!(
            "{}",
            tr!(
                "Réponse illisible du plugin {plugin} : {why}",
                "Unreadable answer from the plugin {plugin}: {why}"
            )
        ),
        down => PluginDown {
            plugin: plugin.to_string(),
            why: down.to_string(),
        }
        .into(),
    }
}

/// A plugin and the runtime that talks to it.
#[derive(Clone)]
struct Line {
    rt: Arc<Runtime>,
    plugin: String,
}

impl Line {
    async fn call<T: DeserializeOwned>(
        &self,
        method: &str,
        what: &'static str,
        params: Value,
    ) -> Result<T> {
        let v = self
            .rt
            .call(&self.plugin, method, params)
            .await
            .map_err(|e| typed(&self.plugin, e, what))?;
        serde_json::from_value(v).map_err(|e| {
            typed(
                &self.plugin,
                CallError::Invalid(format!("{method}: {e}")),
                what,
            )
        })
    }
}

/// The account as the plugin is told it: its fields, never its secrets.
fn account_json(a: &Account) -> Value {
    json!({
        "site": a.site,
        "email": a.email,
        "label": a.label,
        "user": a.user,
    })
}

/// A ticket source for one account, served by a plugin.
pub struct ProcessTicketSource {
    line: Line,
    source: String,
    account: Value,
}

impl ProcessTicketSource {
    pub fn new(rt: Arc<Runtime>, plugin: &str, source: &str, account: &Account) -> Self {
        Self {
            line: Line {
                rt,
                plugin: plugin.to_string(),
            },
            source: source.to_string(),
            account: account_json(account),
        }
    }

    fn params(&self, extra: Value) -> Value {
        let mut p = json!({ "source": self.source, "account": self.account });
        if let (Some(p), Value::Object(extra)) = (p.as_object_mut(), extra) {
            p.extend(extra);
        }
        p
    }
}

impl TicketSource for ProcessTicketSource {
    fn me(&self) -> BoxFuture<'_, Result<(String, String)>> {
        Box::pin(async move {
            let who: Value = self
                .line
                .call(
                    "account/me",
                    "me",
                    self.params(json!({ "point": "ticketSource" })),
                )
                .await?;
            let text = |k: &str| who.get(k).and_then(Value::as_str).unwrap_or("").to_string();
            Ok((text("label"), text("user")))
        })
    }

    fn containers(&self) -> BoxFuture<'_, Result<Vec<Container>>> {
        Box::pin(async move {
            self.line
                .call("tickets/containers", "containers", self.params(json!({})))
                .await
        })
    }

    fn states<'a>(&'a self, container: &'a str) -> BoxFuture<'a, Result<Vec<ExternalState>>> {
        Box::pin(async move {
            self.line
                .call(
                    "tickets/states",
                    "states",
                    self.params(json!({ "container": container })),
                )
                .await
        })
    }

    /// The plugin may answer the states with the columns' defaults (`{states, defaults}`); a
    /// bare list is guessed by names, as every source's.
    fn states_view<'a>(&'a self, container: &'a str) -> BoxFuture<'a, Result<StatesView>> {
        Box::pin(async move {
            let v: Value = self
                .line
                .call(
                    "tickets/states",
                    "states",
                    self.params(json!({ "container": container })),
                )
                .await?;
            if v.is_array() {
                let states: Vec<ExternalState> = serde_json::from_value(v)?;
                return Ok(StatesView {
                    defaults: crate::integrations::text::default_states(&states),
                    states,
                });
            }
            Ok(serde_json::from_value(v)?)
        })
    }

    fn issues<'a>(&'a self, container: &'a str, q: &'a Query) -> BoxFuture<'a, Result<IssuePage>> {
        Box::pin(async move {
            self.line
                .call(
                    "tickets/issues",
                    "issues",
                    self.params(json!({ "container": container, "query": q })),
                )
                .await
        })
    }

    fn set_state<'a>(
        &'a self,
        r: &'a ExternalRef,
        state: &'a ExternalState,
        mapped: &'a [ExternalState],
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            self.line
                .call::<Value>(
                    "tickets/setState",
                    "set_state",
                    self.params(json!({ "ref": r, "state": state, "mapped": mapped })),
                )
                .await
                .map(|_| ())
        })
    }

    fn comment<'a>(&'a self, r: &'a ExternalRef, text: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            self.line
                .call::<Value>(
                    "tickets/comment",
                    "comment",
                    self.params(json!({ "ref": r, "text": text })),
                )
                .await
                .map(|_| ())
        })
    }
}

/// A kind of source a plugin serves (`contributes.ticketSources`); the registry builds them from
/// the manifests of the active plugins (INT-6).
pub struct ProcessTicketSourceKind {
    pub rt: Arc<Runtime>,
    pub plugin: String,
    pub id: &'static str,
    pub label: &'static str,
    pub short: &'static str,
}

impl TicketSourceKind for ProcessTicketSourceKind {
    fn id(&self) -> &'static str {
        self.id
    }
    fn label(&self) -> &'static str {
        self.label
    }
    fn short(&self) -> &'static str {
        self.short
    }
    fn build(
        &self,
        account: &Account,
        _http: reqwest::Client,
        _bases: &Bases,
    ) -> Box<dyn TicketSource> {
        Box::new(ProcessTicketSource::new(
            self.rt.clone(),
            &self.plugin,
            self.id,
            account,
        ))
    }
}

/// A code host for one account, served by a plugin. Each answer of `forge/*` is
/// `{ "value": …, "rateLimit": { limit, remaining, reset }? }`: the host's budget comes with it.
pub struct ProcessForge {
    line: Line,
    forge: String,
    account: Value,
    rate: Mutex<Option<RateLimit>>,
}

impl ProcessForge {
    pub fn new(rt: Arc<Runtime>, plugin: &str, forge: &str, account: &Credentials) -> Self {
        Self {
            line: Line {
                rt,
                plugin: plugin.to_string(),
            },
            forge: forge.to_string(),
            account: json!({ "url": account.url, "email": account.email, "api": account.api }),
            rate: Mutex::new(None),
        }
    }

    async fn ask<T: DeserializeOwned>(
        &self,
        method: &str,
        what: &'static str,
        extra: Value,
    ) -> Result<T> {
        let mut p = json!({ "forge": self.forge, "account": self.account });
        if let (Some(p), Value::Object(extra)) = (p.as_object_mut(), extra) {
            p.extend(extra);
        }
        let mut answer: Value = self.line.call(method, what, p).await?;
        if let Some(r) = answer
            .get("rateLimit")
            .and_then(|r| serde_json::from_value::<RateLimit>(r.clone()).ok())
        {
            *self.rate.lock() = Some(r);
        }
        let value = answer
            .get_mut("value")
            .map(Value::take)
            .unwrap_or(Value::Null);
        serde_json::from_value(value).map_err(|e| {
            typed(
                &self.line.plugin,
                CallError::Invalid(format!("{method}: {e}")),
                what,
            )
        })
    }
}

impl Forge for ProcessForge {
    fn me(&self) -> BoxFuture<'_, Result<Identity>> {
        Box::pin(async move {
            let p = json!({ "point": "forge", "forge": self.forge, "account": self.account });
            self.line.call("account/me", "me", p).await
        })
    }

    fn create_pr<'a>(
        &'a self,
        repo: &'a RepoRef,
        pr: &'a PullRequestDraft,
    ) -> BoxFuture<'a, Result<PullRequest>> {
        Box::pin(self.ask(
            "forge/createPr",
            "create_pr",
            json!({ "repo": repo, "pr": pr }),
        ))
    }

    fn pull_request<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
    ) -> BoxFuture<'a, Result<PullRequest>> {
        Box::pin(self.ask(
            "forge/pullRequest",
            "pull_request",
            json!({ "repo": repo, "number": number }),
        ))
    }

    fn pull_requests_of_branch<'a>(
        &'a self,
        repo: &'a RepoRef,
        branch: &'a str,
    ) -> BoxFuture<'a, Result<Vec<PullRequest>>> {
        Box::pin(self.ask(
            "forge/pullRequestsOfBranch",
            "pull_requests_of_branch",
            json!({ "repo": repo, "branch": branch }),
        ))
    }

    fn parent<'a>(&'a self, repo: &'a RepoRef) -> BoxFuture<'a, Result<Option<RepoRef>>> {
        Box::pin(async move {
            match self
                .ask("forge/parent", "parent", json!({ "repo": repo }))
                .await
            {
                Err(e) if e.downcast_ref::<Unsupported>().is_some() => Ok(None),
                other => other,
            }
        })
    }

    fn open_pull_requests<'a>(
        &'a self,
        repo: &'a RepoRef,
    ) -> BoxFuture<'a, Result<Vec<PullRequest>>> {
        Box::pin(self.ask(
            "forge/openPullRequests",
            "open_pull_requests",
            json!({ "repo": repo }),
        ))
    }

    fn pull_requests<'a>(
        &'a self,
        repo: &'a RepoRef,
        numbers: &'a [u64],
    ) -> BoxFuture<'a, Result<Vec<PullRequest>>> {
        Box::pin(self.ask(
            "forge/pullRequests",
            "pull_requests",
            json!({ "repo": repo, "numbers": numbers }),
        ))
    }

    fn merge<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
        strategy: MergeStrategy,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            self.ask::<Value>(
                "forge/merge",
                "merge",
                json!({ "repo": repo, "number": number, "strategy": strategy }),
            )
            .await
            .map(|_| ())
        })
    }

    fn auto_merge<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
        strategy: MergeStrategy,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            self.ask::<Value>(
                "forge/autoMerge",
                "auto_merge",
                json!({ "repo": repo, "number": number, "strategy": strategy }),
            )
            .await
            .map(|_| ())
        })
    }

    fn checks<'a>(&'a self, repo: &'a RepoRef, number: u64) -> BoxFuture<'a, Result<Vec<Check>>> {
        Box::pin(self.ask(
            "forge/checks",
            "checks",
            json!({ "repo": repo, "number": number }),
        ))
    }

    fn job_log<'a>(&'a self, repo: &'a RepoRef, job: &'a str) -> BoxFuture<'a, Result<String>> {
        Box::pin(self.ask(
            "forge/jobLog",
            "job_log",
            json!({ "repo": repo, "job": job }),
        ))
    }

    fn reviews<'a>(&'a self, repo: &'a RepoRef, number: u64) -> BoxFuture<'a, Result<Vec<Review>>> {
        Box::pin(self.ask(
            "forge/reviews",
            "reviews",
            json!({ "repo": repo, "number": number }),
        ))
    }

    fn comments<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
    ) -> BoxFuture<'a, Result<Vec<ReviewComment>>> {
        Box::pin(self.ask(
            "forge/comments",
            "comments",
            json!({ "repo": repo, "number": number }),
        ))
    }

    fn comment<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
        body: &'a str,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            self.ask::<Value>(
                "forge/comment",
                "comment",
                json!({ "repo": repo, "number": number, "body": body }),
            )
            .await
            .map(|_| ())
        })
    }

    fn rerun_failed<'a>(&'a self, repo: &'a RepoRef, number: u64) -> BoxFuture<'a, Result<usize>> {
        Box::pin(self.ask(
            "forge/rerunFailed",
            "rerun_failed",
            json!({ "repo": repo, "number": number }),
        ))
    }

    fn review<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
        review: &'a ReviewDraft,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            self.ask::<Value>(
                "forge/review",
                "review",
                json!({ "repo": repo, "number": number, "review": review }),
            )
            .await
            .map(|_| ())
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
            self.ask::<Value>(
                "forge/reply",
                "reply",
                json!({ "repo": repo, "number": number, "comment": comment, "body": body }),
            )
            .await
            .map(|_| ())
        })
    }

    fn resolve<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
        comment: &'a str,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            self.ask::<Value>(
                "forge/resolve",
                "resolve",
                json!({ "repo": repo, "number": number, "comment": comment }),
            )
            .await
            .map(|_| ())
        })
    }

    fn rate_limit(&self) -> Option<RateLimit> {
        *self.rate.lock()
    }

    fn deployments<'a>(
        &'a self,
        repo: &'a RepoRef,
        git_ref: &'a str,
    ) -> BoxFuture<'a, Result<Vec<Deployment>>> {
        Box::pin(self.ask(
            "forge/deployments",
            "deployments",
            json!({ "repo": repo, "ref": git_ref }),
        ))
    }
}
