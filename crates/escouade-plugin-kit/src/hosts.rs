//! The service of the code hosts and of the Kanban's sources, for every plugin that serves some (docs/PLUGINS.md §4.4,
//! §4.5): GitHub today, GitLab, Bitbucket, Gitea and Azure DevOps after it. A plugin gives its kinds — a
//! [`ForgeKind`] for each host it serves, a [`SourceKind`] for each source — and [`Hosts`] answers the core's adapters
//! (`ProcessForge`, `ProcessTicketSource`) for them:
//!
//! - **the account** is the call's (`account`: an instance's address, where its API is when the core says it); its
//!   token is asked of Escouade by `secrets/get` of `account:<name>` (an instance's host, `github.com`; else the
//!   source's id), never carried by the call. The core answers it from its accounts: the one the user gave, or the
//!   one of the host's CLI (`gh auth token`);
//! - **one client per account and token**, kept: what a client learns (a host's conditional reads, its limits) serves
//!   the next call. A token changed makes a new one;
//! - **the typed errors** as the protocol's codes: an HTTP error `-(32000 + status)` (401: « reconnect the account »),
//!   a lasting refusal `-32010`, a call the host cannot do `-32601`;
//! - **the budget of calls** in each answer of a host: `{ value, rateLimit }`, which the core keeps for the account
//!   (the sources and the hosts of a plugin spend the same).

use crate::errors::{HttpError, Refused};
use crate::forge::{BoxFuture, Forge, RateLimit, Unsupported};
use crate::http::{host_of, Network};
use crate::serve::{Cx, Plugin, Refusal};
use crate::tr;
use anyhow::Result;
use escouade_plugin_protocol::code;
use escouade_plugin_protocol::tickets::{Container, ExternalRef, ExternalState, IssuePage, Query};
use parking_lot::Mutex;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

/// What an account gives its client: where its instance is, and its credentials.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Credentials {
    /// The instance's address (`https://github.com`, `https://ghe.acme.dev`), without a trailing slash; empty for a
    /// source of an account of its own.
    pub url: String,
    /// The e-mail some hosts' tokens go with (Bitbucket's).
    pub email: String,
    pub token: String,
    /// Who the account is for the service (a source's « mine » filters on it); empty when the core does not say.
    pub user: String,
    /// Where its API is, in place of where the address says (github.com's as the core reaches it, a fake one).
    pub api: Option<String>,
}

/// A kind of code host a plugin serves (`contributes.forges`).
pub trait ForgeKind: Send + Sync {
    /// Its id, as the core names it (`github`).
    fn id(&self) -> &str;
    /// Its name, as the user reads it (`GitHub`).
    fn name(&self) -> &str;
    /// Its public instance (`https://github.com`), the account's when the call names none.
    fn default_url(&self) -> Option<&str> {
        None
    }
    /// The client of one account.
    fn client(&self, http: crate::http::reqwest::Client, account: &Credentials) -> Box<dyn Forge>;
}

/// The states of a container and the state each column gives by default (`doing`, `review`, `done`).
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize)]
pub struct StatesView {
    pub states: Vec<ExternalState>,
    pub defaults: std::collections::BTreeMap<String, ExternalState>,
}

/// A source's client for one account, the plugin's side of `tickets/*`.
pub trait TicketSource: Send + Sync {
    /// The account as the window shows it (`@ada`) and its login.
    fn me(&self) -> BoxFuture<'_, Result<(String, String)>>;
    fn containers(&self) -> BoxFuture<'_, Result<Vec<Container>>>;
    fn states<'a>(&'a self, container: &'a str) -> BoxFuture<'a, Result<Vec<ExternalState>>>;
    /// The states with the state each column gives by default, for a source that knows better than the names Escouade
    /// guesses them by (Azure Boards: by its categories). None: Escouade guesses, and `states` is what is answered.
    fn states_view<'a>(&'a self, _container: &'a str) -> BoxFuture<'a, Result<Option<StatesView>>> {
        Box::pin(async { Ok(None) })
    }
    fn issues<'a>(&'a self, container: &'a str, q: &'a Query) -> BoxFuture<'a, Result<IssuePage>>;
    fn set_state<'a>(
        &'a self,
        r: &'a ExternalRef,
        state: &'a ExternalState,
        mapped: &'a [ExternalState],
    ) -> BoxFuture<'a, Result<()>>;
    fn comment<'a>(&'a self, r: &'a ExternalRef, text: &'a str) -> BoxFuture<'a, Result<()>>;
    /// What its service's last answer said of the account's limit, when it says it.
    fn rate_limit(&self) -> Option<RateLimit> {
        None
    }
}

/// A kind of source a plugin serves (`contributes.ticketSources`).
pub trait SourceKind: Send + Sync {
    /// Its id, as the core names it (`github`).
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    /// The client of one account (`url`: the instance of a code host's account; empty for a source's own).
    fn client(
        &self,
        http: crate::http::reqwest::Client,
        account: &Credentials,
    ) -> Box<dyn TicketSource>;
    /// The name of the account's secret (`account:<name>`) when the instance is `url`: by default its host, else the
    /// source's own id. A source whose instance is only an address typed by the user (Jira's site) keeps its own id.
    fn account_name(&self, url: &str) -> String {
        if url.is_empty() {
            self.id().to_string()
        } else {
            host_of(url)
        }
    }
}

/// The account a call names, as the core's adapters write it.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct Named {
    url: String,
    /// A source's account: its instance (GitLab Issues on a GitLab's).
    site: String,
    email: String,
    user: String,
    api: Option<String>,
}

impl Named {
    fn address(&self) -> &str {
        if self.url.trim().is_empty() {
            self.site.trim()
        } else {
            self.url.trim()
        }
    }
}

/// The service of a plugin's code hosts and sources: a `serve::Plugin` that answers `account/me`, `forge/*` and
/// `tickets/*`, and refuses the rest.
pub struct Hosts {
    forges: Vec<Box<dyn ForgeKind>>,
    sources: Vec<Box<dyn SourceKind>>,
    network: Network,
    forge_clients: Mutex<HashMap<String, Arc<dyn Forge>>>,
    source_clients: Mutex<HashMap<String, Arc<dyn TicketSource>>>,
}

impl Hosts {
    /// The service of these kinds, on the network Escouade's settings say (`Network::from_env`).
    pub fn new(forges: Vec<Box<dyn ForgeKind>>, sources: Vec<Box<dyn SourceKind>>) -> Self {
        Self::on(Network::from_env(), forges, sources)
    }

    pub fn on(
        network: Network,
        forges: Vec<Box<dyn ForgeKind>>,
        sources: Vec<Box<dyn SourceKind>>,
    ) -> Self {
        Self {
            forges,
            sources,
            network,
            forge_clients: Mutex::default(),
            source_clients: Mutex::default(),
        }
    }

    /// The token of the account `name`, as Escouade gives it.
    async fn token(&self, cx: &Cx, kind: &str, name: &str) -> Result<String, Refusal> {
        let token = cx
            .secret(&format!("account:{name}"))
            .await?
            .unwrap_or_default();
        if token.trim().is_empty() {
            return Err(Refusal {
                code: code::http(401),
                message: tr!(
                    "{kind} : aucun jeton pour {name}. Reconnecte le compte dans Réglages › Intégrations.",
                    "{kind}: no token for {name}. Connect the account again in Settings › Integrations."
                ),
            });
        }
        Ok(token.trim().to_string())
    }

    fn http(&self) -> Result<crate::http::reqwest::Client, Refusal> {
        self.network.client().map_err(|e| Refusal {
            code: -32000,
            message: format!("{e:#}"),
        })
    }

    /// The client of the host the call names, for its account: the one kept, or a new one.
    async fn forge(&self, cx: &Cx, params: &Value) -> Result<(Arc<dyn Forge>, String), Refusal> {
        let id = params["forge"].as_str().unwrap_or("");
        let kind = self
            .forges
            .iter()
            .find(|k| k.id() == id)
            .ok_or_else(|| invalid(format!("this plugin serves no code host {id:?}")))?;
        let account: Named = arg(params, "account")?;
        let url = match account.address().trim_end_matches('/') {
            "" => kind.default_url().unwrap_or("").to_string(),
            url => url.to_string(),
        };
        let host = host_of(&url);
        if host.is_empty() {
            return Err(invalid("the account names no instance".into()));
        }
        let token = self.token(cx, kind.name(), &host).await?;
        let key = format!("{id}\n{url}\n{:?}\n{token}", account.api);
        if let Some(c) = self.forge_clients.lock().get(&key) {
            return Ok((c.clone(), host));
        }
        let creds = Credentials {
            url,
            email: account.email,
            token,
            user: account.user,
            api: account.api,
        };
        let client: Arc<dyn Forge> = Arc::from(kind.client(self.http()?, &creds));
        let mut clients = self.forge_clients.lock();
        // A token that changed leaves its account's former client.
        clients.retain(|k, _| !k.starts_with(&format!("{id}\n{}\n", creds.url)));
        Ok((clients.entry(key).or_insert(client).clone(), host))
    }

    /// The client of the source the call names, for its account.
    async fn source(&self, cx: &Cx, params: &Value) -> Result<Arc<dyn TicketSource>, Refusal> {
        let id = params["source"].as_str().unwrap_or("");
        let kind = self
            .sources
            .iter()
            .find(|k| k.id() == id)
            .ok_or_else(|| invalid(format!("this plugin serves no ticket source {id:?}")))?;
        let account: Named = arg(params, "account")?;
        let url = account.address().trim_end_matches('/').to_string();
        // Its account: its instance's host (github.com's), else the source's own (`SourceKind::account_name`).
        let name = kind.account_name(&url);
        let token = self.token(cx, kind.name(), &name).await?;
        // The user is part of the key: a client made before the account knew it (at « Connecter ») must not serve the
        // calls after it (ClickUp's « Mes tâches » filters on it).
        let key = format!("{id}\n{url}\n{:?}\n{token}\n{}", account.api, account.user);
        if let Some(c) = self.source_clients.lock().get(&key) {
            return Ok(c.clone());
        }
        let creds = Credentials {
            url,
            email: account.email,
            token,
            user: account.user,
            api: account.api,
        };
        let client: Arc<dyn TicketSource> = Arc::from(kind.client(self.http()?, &creds));
        let mut clients = self.source_clients.lock();
        clients.retain(|k, _| !k.starts_with(&format!("{id}\n{}\n", creds.url)));
        Ok(clients.entry(key).or_insert(client).clone())
    }

    async fn answer(&self, cx: &Cx, method: &str, params: Value) -> Result<Value, Refusal> {
        if method == "account/me" {
            return if params["point"] == "ticketSource" {
                let s = self.source(cx, &params).await?;
                let (label, user) = s.me().await.map_err(refusal)?;
                budgeted(
                    json!({ "label": label, "user": user }),
                    s.rate_limit(),
                    false,
                )
            } else {
                let (f, _) = self.forge(cx, &params).await?;
                let me = f.me().await.map_err(refusal)?;
                budgeted(to_value(me)?, f.rate_limit(), false)
            };
        }
        if let Some(name) = method.strip_prefix("forge/") {
            let (f, host) = self.forge(cx, &params).await?;
            let v = forge_call(&*f, name, &params).await;
            if let (Err(_), Some(rl)) = (&v, f.rate_limit()) {
                // A refusal carries no budget: said before it (a 403 of the limit says when it starts over).
                cx.notify(
                    "account/budget",
                    json!({ "account": host, "rateLimit": to_value(rl)? }),
                );
            }
            return budgeted(v?, f.rate_limit(), true);
        }
        if let Some(name) = method.strip_prefix("tickets/") {
            let s = self.source(cx, &params).await?;
            let v = source_call(&*s, name, &params).await?;
            return budgeted(v, s.rate_limit(), false);
        }
        Err(Refusal::not_found(method))
    }
}

impl Plugin for Hosts {
    fn call<'a>(
        &'a self,
        cx: &'a Cx,
        method: &'a str,
        params: Value,
    ) -> BoxFuture<'a, Result<Value, Refusal>> {
        Box::pin(self.answer(cx, method, params))
    }
}

fn invalid(message: String) -> Refusal {
    Refusal {
        code: -32602,
        message,
    }
}

fn arg<T: DeserializeOwned>(params: &Value, key: &str) -> Result<T, Refusal> {
    serde_json::from_value(params.get(key).cloned().unwrap_or(Value::Null))
        .map_err(|e| invalid(format!("{key}: {e}")))
}

fn to_value<T: serde::Serialize>(v: T) -> Result<Value, Refusal> {
    serde_json::to_value(v).map_err(|e| Refusal {
        code: -32603,
        message: e.to_string(),
    })
}

/// What a failed call of a client is on the protocol: the code of its type, its message as it reads.
pub fn refusal(e: anyhow::Error) -> Refusal {
    if let Some(h) = e.downcast_ref::<HttpError>() {
        return Refusal {
            code: code::http(h.status),
            message: h.message.clone(),
        };
    }
    if let Some(r) = e.downcast_ref::<Refused>() {
        return Refusal {
            code: code::REFUSED,
            message: r.0.clone(),
        };
    }
    if e.downcast_ref::<Unsupported>().is_some() {
        return Refusal {
            code: code::METHOD_NOT_FOUND,
            message: format!("{e}"),
        };
    }
    Refusal {
        code: -32000,
        message: format!("{e:#}"),
    }
}

/// An answer with the account's budget beside it, when its client says it: `{ value, rateLimit }`. A host's answer is
/// always `{ value }` at least; a source's is bare without a budget.
fn budgeted(v: Value, limit: Option<RateLimit>, wrapped: bool) -> Result<Value, Refusal> {
    Ok(match limit {
        Some(rl) => json!({ "value": v, "rateLimit": to_value(rl)? }),
        None if wrapped => json!({ "value": v }),
        None => v,
    })
}

/// A host's call `name` (`forge/<name>`), with the arguments the core's `ProcessForge` writes.
async fn forge_call(f: &dyn Forge, name: &str, p: &Value) -> Result<Value, Refusal> {
    let repo = || arg::<crate::forge::RepoRef>(p, "repo");
    let number = || arg::<u64>(p, "number");
    let text = |k: &str| arg::<String>(p, k);
    let done = |r: Result<()>| r.map(|()| Value::Null).map_err(refusal);
    match name {
        "createPr" => to_value(
            f.create_pr(&repo()?, &arg(p, "pr")?)
                .await
                .map_err(refusal)?,
        ),
        "pullRequest" => to_value(f.pull_request(&repo()?, number()?).await.map_err(refusal)?),
        "pullRequestsOfBranch" => to_value(
            f.pull_requests_of_branch(&repo()?, &text("branch")?)
                .await
                .map_err(refusal)?,
        ),
        "parent" => to_value(f.parent(&repo()?).await.map_err(refusal)?),
        "openPullRequests" => to_value(f.open_pull_requests(&repo()?).await.map_err(refusal)?),
        "pullRequests" => {
            let numbers: Vec<u64> = arg(p, "numbers")?;
            to_value(f.pull_requests(&repo()?, &numbers).await.map_err(refusal)?)
        }
        "mergeOptions" => to_value(
            f.merge_options(&repo()?, number()?)
                .await
                .map_err(refusal)?,
        ),
        "merge" => done(f.merge(&repo()?, number()?, arg(p, "strategy")?).await),
        "autoMerge" => done(f.auto_merge(&repo()?, number()?, arg(p, "strategy")?).await),
        "checks" => to_value(f.checks(&repo()?, number()?).await.map_err(refusal)?),
        "jobLog" => to_value(f.job_log(&repo()?, &text("job")?).await.map_err(refusal)?),
        "rerunFailed" => to_value(f.rerun_failed(&repo()?, number()?).await.map_err(refusal)?),
        "branchChecks" => to_value(
            f.branch_checks(&repo()?, &text("branch")?)
                .await
                .map_err(refusal)?,
        ),
        "rerunFailedOfBranch" => to_value(
            f.rerun_failed_of_branch(&repo()?, &text("branch")?)
                .await
                .map_err(refusal)?,
        ),
        "reviews" => to_value(f.reviews(&repo()?, number()?).await.map_err(refusal)?),
        "comments" => to_value(f.comments(&repo()?, number()?).await.map_err(refusal)?),
        "comment" => done(f.comment(&repo()?, number()?, &text("body")?).await),
        "review" => done(f.review(&repo()?, number()?, &arg(p, "review")?).await),
        "reply" => done(
            f.reply(&repo()?, number()?, &text("comment")?, &text("body")?)
                .await,
        ),
        "resolve" => done(f.resolve(&repo()?, number()?, &text("comment")?).await),
        "deployments" => to_value(
            f.deployments(&repo()?, &text("ref")?)
                .await
                .map_err(refusal)?,
        ),
        "environments" => to_value(f.environments(&repo()?).await.map_err(refusal)?),
        other => Err(Refusal::not_found(&format!("forge/{other}"))),
    }
}

/// A source's call `name` (`tickets/<name>`), with the arguments the core's `ProcessTicketSource` writes.
async fn source_call(s: &dyn TicketSource, name: &str, p: &Value) -> Result<Value, Refusal> {
    let container = || arg::<String>(p, "container");
    let r = || arg::<ExternalRef>(p, "ref");
    match name {
        "containers" => to_value(s.containers().await.map_err(refusal)?),
        "states" => match s.states_view(&container()?).await.map_err(refusal)? {
            Some(view) => to_value(view),
            None => to_value(s.states(&container()?).await.map_err(refusal)?),
        },
        "issues" => {
            let q: Query = arg(p, "query")?;
            to_value(s.issues(&container()?, &q).await.map_err(refusal)?)
        }
        "setState" => {
            let mapped: Vec<ExternalState> = arg(p, "mapped")?;
            s.set_state(&r()?, &arg(p, "state")?, &mapped)
                .await
                .map(|()| Value::Null)
                .map_err(refusal)
        }
        "comment" => s
            .comment(&r()?, &arg::<String>(p, "text")?)
            .await
            .map(|()| Value::Null)
            .map_err(refusal),
        other => Err(Refusal::not_found(&format!("tickets/{other}"))),
    }
}
