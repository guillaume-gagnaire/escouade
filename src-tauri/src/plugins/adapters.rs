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
//! It gets its token itself, by `secrets/get` of `account:<account>` (`plugins::accounts`: an
//! instance's host, or a kind's id): the adapters never put it in a call, and mask nothing because
//! they carry nothing. Each answer may carry the account's budget (`{ value, rateLimit }`), kept
//! for the account: the plugin's sources and hosts spend the same (`Runtime::budget`).

// The app asks a plugin for an account another plugin than Escouade's own hand serves
// (`Account::plugin`, INT-6); the registries of sources and hosts read the manifests with INT-7
// and INT-8, so that `ProcessTicketSourceKind` serves only the tests until then.
#![allow(dead_code)]

use super::runtime::{CallError, Runtime, METHOD_NOT_FOUND};
use crate::forge::{
    BoxFuture, Check, Credentials, Deployment, Forge, Identity, MergeOptions, MergeStrategy,
    PullRequest, PullRequestDraft, RateLimit, RepoRef, Review, ReviewComment, ReviewDraft,
    Unsupported,
};
use crate::integrations::source::{TicketSource, TicketSourceKind};
use crate::integrations::{
    Account, Bases, Container, HttpError, IssuePage, Query, Refused, StatesView,
};
use crate::model::{ExternalRef, ExternalState};
use anyhow::{anyhow, Result};
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

/// A plugin and the runtime that talks to it (for one account: `for_account`).
#[derive(Clone)]
pub(super) struct Line {
    rt: Arc<Runtime>,
    pub(super) plugin: String,
    /// The account's budget of calls (`accounts::budget_key`), for a line of one account:
    /// every point of the plugin spends the same.
    budget: Option<String>,
}

impl Line {
    pub(super) fn new(rt: Arc<Runtime>, plugin: &str) -> Self {
        Self {
            rt,
            plugin: plugin.to_string(),
            budget: None,
        }
    }

    /// The line of one account (`account`: its name in the keychain's), whose budget it keeps.
    fn for_account(rt: Arc<Runtime>, plugin: &str, account: &str) -> Self {
        Self {
            budget: Some(super::accounts::budget_key(plugin, account)),
            ..Self::new(rt, plugin)
        }
    }

    /// The answer to `method`: its value, the budget it carries (`{ value, rateLimit }`) kept as
    /// the account's.
    pub(super) async fn call<T: DeserializeOwned>(
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
        let (v, left) = super::accounts::split_budget(v);
        if let (Some(budget), Some(left)) = (&self.budget, left) {
            self.rt.spend(budget, left);
        }
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
        // Its account: its instance's host (GitLab Issues on a GitLab's), else the source's own.
        let name = if account.site.trim().is_empty() {
            source.to_string()
        } else {
            crate::forge::host_of(&account.site)
        };
        Self {
            line: Line::for_account(rt, plugin, &name),
            source: source.to_string(),
            account: account_json(account),
        }
    }

    /// The account where its API is when the core says it (github.com's as GitHub Issues reaches
    /// it: `ESCOUADE_GITHUB_API`, a fake one).
    pub fn with_api(mut self, api: &str) -> Self {
        self.account["api"] = json!(api);
        self
    }

    /// The account's budget of calls, as the last answer of any point of its plugin said it (a
    /// source's answer may carry it as a host's does: `{ value, rateLimit }`).
    pub fn rate_limit(&self) -> Option<RateLimit> {
        self.line
            .budget
            .as_deref()
            .and_then(|b| self.line.rt.budget(b))
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
        Box::pin(async move { Ok(self.states_view(container).await?.states) })
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

/// A source whose accounts are organizations (Azure Boards: an Azure DevOps organization, a Server's collection), each
/// served by its own client: its containers are `<organization>/<project>`, so that a call goes to the organization
/// its container names. The containers of all are listed together.
pub struct ByOrganization {
    sources: Vec<(String, Box<dyn TicketSource>)>,
}

impl ByOrganization {
    pub fn new(sources: Vec<(String, Box<dyn TicketSource>)>) -> Self {
        Self { sources }
    }

    /// The client of the organization a container (`<organization>/<project>`) names.
    fn of(&self, container: &str) -> Result<&dyn TicketSource> {
        let org = container.split_once('/').map_or(container, |(o, _)| o);
        self.sources
            .iter()
            .find(|(o, _)| o.eq_ignore_ascii_case(org))
            .map(|(_, s)| &**s)
            .ok_or_else(|| {
                anyhow!(tr!(
                    "Aucun compte Azure DevOps connecté pour l'organisation {org}",
                    "No Azure DevOps account connected for the {org} organization"
                ))
            })
    }
}

impl TicketSource for ByOrganization {
    fn me(&self) -> BoxFuture<'_, Result<(String, String)>> {
        Box::pin(async move {
            match self.sources.first() {
                Some((_, s)) => s.me().await,
                None => Err(anyhow!(tr!(
                    "Aucun compte Azure DevOps connecté",
                    "No Azure DevOps account connected"
                ))),
            }
        })
    }

    fn containers(&self) -> BoxFuture<'_, Result<Vec<Container>>> {
        Box::pin(async move {
            let mut out = Vec::new();
            for (_, s) in &self.sources {
                out.extend(s.containers().await?);
            }
            Ok(out)
        })
    }

    fn states<'a>(&'a self, container: &'a str) -> BoxFuture<'a, Result<Vec<ExternalState>>> {
        Box::pin(async move { self.of(container)?.states(container).await })
    }

    fn states_view<'a>(&'a self, container: &'a str) -> BoxFuture<'a, Result<StatesView>> {
        Box::pin(async move { self.of(container)?.states_view(container).await })
    }

    fn issues<'a>(&'a self, container: &'a str, q: &'a Query) -> BoxFuture<'a, Result<IssuePage>> {
        Box::pin(async move { self.of(container)?.issues(container, q).await })
    }

    fn set_state<'a>(
        &'a self,
        r: &'a ExternalRef,
        state: &'a ExternalState,
        mapped: &'a [ExternalState],
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move { self.of(&r.container)?.set_state(r, state, mapped).await })
    }

    fn comment<'a>(&'a self, r: &'a ExternalRef, text: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move { self.of(&r.container)?.comment(r, text).await })
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
}

impl ProcessForge {
    pub fn new(rt: Arc<Runtime>, plugin: &str, forge: &str, account: &Credentials) -> Self {
        Self {
            line: Line::for_account(rt, plugin, &crate::forge::host_of(&account.url)),
            forge: forge.to_string(),
            account: json!({ "url": account.url, "email": account.email, "api": account.api }),
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
        // `{ value, rateLimit }`: the budget is kept by `Line::call`; a bare `{ value }` too.
        let answer: Value = self.line.call(method, what, p).await?;
        let value = match answer {
            Value::Object(mut o) if o.len() == 1 && o.contains_key("value") => {
                o.remove("value").unwrap_or(Value::Null)
            }
            other => other,
        };
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

    fn merge_options<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: u64,
    ) -> BoxFuture<'a, Result<MergeOptions>> {
        Box::pin(self.ask(
            "forge/mergeOptions",
            "merge_options",
            json!({ "repo": repo, "number": number }),
        ))
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

    fn branch_checks<'a>(
        &'a self,
        repo: &'a RepoRef,
        branch: &'a str,
    ) -> BoxFuture<'a, Result<Vec<Check>>> {
        Box::pin(self.ask(
            "forge/branchChecks",
            "branch_checks",
            json!({ "repo": repo, "branch": branch }),
        ))
    }

    fn rerun_failed_of_branch<'a>(
        &'a self,
        repo: &'a RepoRef,
        branch: &'a str,
    ) -> BoxFuture<'a, Result<usize>> {
        Box::pin(self.ask(
            "forge/rerunFailedOfBranch",
            "rerun_failed_of_branch",
            json!({ "repo": repo, "branch": branch }),
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

    /// The account's budget: what the last answer of any point of its plugin said.
    fn rate_limit(&self) -> Option<RateLimit> {
        self.line
            .budget
            .as_deref()
            .and_then(|b| self.line.rt.budget(b))
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

    fn environments<'a>(&'a self, repo: &'a RepoRef) -> BoxFuture<'a, Result<Vec<Deployment>>> {
        Box::pin(self.ask(
            "forge/environments",
            "environments",
            json!({ "repo": repo }),
        ))
    }
}
