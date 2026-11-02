//! Code hosts (GitHub, GitLab, Bitbucket Cloud, Gitea and Forgejo, Azure DevOps): one interface for all of them,
//! as for the harnesses. A host is a `Provider` (what it is called, the addresses it knows, the
//! rights its tokens need, what it can do) that gives a `Forge` client per account; the rest of
//! Escouade only talks to these two traits, so that adding a host comes down to its client.

// The pull requests' calls and their types serve the hosts' clients (ESC-55 to ESC-58) and the
// PR's cycle (ESC-64): declared here, used there.
#![allow(dead_code)]

pub mod accounts;
pub mod azure;
#[cfg(test)]
mod azure_tests;
pub mod bitbucket;
#[cfg(test)]
mod bitbucket_tests;
pub mod connect;
pub mod gitea;
#[cfg(test)]
mod gitea_tests;
pub mod github;
#[cfg(test)]
mod github_tests;
pub mod gitlab;
#[cfg(test)]
mod gitlab_tests;
pub mod remote;
#[cfg(test)]
mod tests;

use anyhow::{anyhow, Result};
use reqwest::header::HeaderMap;
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;

pub use remote::RepoRef;

/// What a client's calls give back.
pub use escouade_plugin_kit::forge::BoxFuture;

// The data of the hosts, shared with the plugins that serve them (docs/PLUGINS.md §4.5).
#[cfg(test)]
pub use escouade_plugin_protocol::forge::ReviewLine;
pub use escouade_plugin_protocol::forge::{
    Check, CheckState, Deployment, Identity, MergeOptions, MergeStrategy, PrState, PullRequest,
    PullRequestDraft, RateLimit, Review, ReviewComment, ReviewDraft, ReviewState,
};

/// What a host can do, declared by its provider: the window and the PR's cycle offer only these.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    /// Its client opens pull requests and reads them (state, checks, reviews): GitHub's since
    /// ESC-55, the others with their clients. Without it, an agent's card shows none, without an
    /// error.
    pub pull_requests: bool,
    /// A pull request may open as a draft.
    pub draft: bool,
    /// The host merges by itself once the checks pass.
    pub auto_merge: bool,
    /// The checks and statuses of a pull request.
    pub checks: bool,
    /// The log of a check's job.
    pub job_logs: bool,
    /// Approvals and changes requested.
    pub reviews: bool,
    /// Comments on lines, in threads that resolve.
    pub review_threads: bool,
    /// Deployments and their environments.
    pub deployments: bool,
    /// The failed jobs of a pull request can run again.
    pub rerun: bool,
    pub merge_strategies: Vec<MergeStrategy>,
}

// The contract of a code host's client, shared with the plugins that serve one (`escouade_plugin_kit::forge`): the
// core's adapter of a plugin (`plugins::adapters::ProcessForge`) and the hosts still in the core implement it.
#[cfg(test)]
pub use escouade_plugin_kit::forge::Pace;
pub use escouade_plugin_kit::forge::{Forge, Unsupported};

/// What an account gives its client: where its instance is, and its credentials.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Credentials {
    /// The instance's address (`https://gitlab.com`, `https://git.acme.dev`), without a trailing
    /// slash.
    pub url: String,
    /// Bitbucket: the Atlassian account's e-mail its API token belongs to.
    pub email: String,
    pub token: String,
    /// Where its API is, in place of where the instance's address says (a fake one; github.com's
    /// as GitHub Issues reaches it).
    pub api: Option<String>,
}

/// What a host's CLI printed and where the token is in it: declared, so that a plugin can say it
/// without code.
#[derive(Debug, Clone, PartialEq)]
pub enum CliRead {
    /// The whole of stdout, trimmed (`gh auth token`).
    Stdout,
    /// Stdout, trimmed, when it is one word (`az account get-access-token`).
    StdoutWord,
    /// After a label, on a line of stdout then stderr, when it is one word without a `*`
    /// (`glab auth status --show-token` prints `✓ Token: glpat-…`).
    After(String),
}

impl CliRead {
    /// The token in what the command printed.
    pub fn token(&self, stdout: &str, stderr: &str) -> Option<String> {
        match self {
            Self::Stdout => Some(stdout.trim().to_string()).filter(|t| !t.is_empty()),
            Self::StdoutWord => {
                let t = stdout.trim();
                (!t.is_empty() && !t.contains(char::is_whitespace)).then(|| t.to_string())
            }
            Self::After(label) => format!("{stdout}\n{stderr}").lines().find_map(|line| {
                let (_, t) = line.split_once(label.as_str())?;
                let t = t.trim();
                (!t.is_empty() && !t.contains('*') && !t.contains(' ')).then(|| t.to_string())
            }),
        }
    }
}

/// A command that prints the token of a host's CLI (`gh auth token --hostname <host>`).
#[derive(Debug, Clone)]
pub struct CliToken {
    pub program: String,
    pub args: Vec<String>,
    /// How the user logs it in (`gh auth login`, `az login`).
    pub login: String,
    /// The token in what the command printed.
    pub read: CliRead,
}

impl CliToken {
    /// The token in what the command printed (stdout, then stderr).
    pub fn parse(&self, stdout: &str, stderr: &str) -> Option<String> {
        self.read.token(stdout, stderr)
    }
}

/// A path to ask an instance whose host is unknown, and what in its answer says it is this host's:
/// declared (a status, or a status with a key holding a string), so that a plugin can say it
/// without code.
#[derive(Debug, Clone, Default)]
pub struct Probe {
    pub path: String,
    /// Statuses that say it by themselves (GitLab's 401 without a token).
    pub statuses: Vec<u16>,
    /// The status that says it with `key`.
    pub key_status: u16,
    /// The key of the answer that holds a string (`version`); empty: none is asked.
    pub key: String,
}

impl Probe {
    pub fn matches(&self, status: u16, body: &Value) -> bool {
        self.statuses.contains(&status)
            || (status == self.key_status && !self.key.is_empty() && body[&self.key].is_string())
    }
}

/// How the branch of a pull request opened from a fork is fetched (`Provider::pull_fetch`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PullFetch {
    /// A reference of the targeted repository that is its head (`refs/pull/<n>/head`).
    Head(String),
    /// A reference of the targeted repository that is its merge into its base, whose second parent
    /// is its head (Azure DevOps' `refs/pull/<n>/merge`).
    Merge(String),
    /// Its branch, in the fork's repository on the same instance (Bitbucket Cloud keeps no reference
    /// of a pull request).
    ForkBranch,
}

/// A code host: what Escouade knows of it, and the client of an account.
pub trait Provider: Send + Sync {
    /// What the accounts and the window know it by: `github`, `gitlab`, `bitbucket`, `gitea`.
    fn kind(&self) -> String;
    /// As the window says it.
    fn name(&self) -> &'static str;
    /// Its word for a pull request: « MR » on GitLab, « PR » elsewhere.
    fn request_word(&self) -> &'static str {
        "PR"
    }
    fn capabilities(&self) -> Capabilities;
    /// How a pull request opened from a fork is fetched from the repository it targets (« Lancer
    /// un agent sur cette PR », ESC-65); none when this host keeps no reference of it.
    fn pull_fetch(&self, _number: u64) -> Option<PullFetch> {
        None
    }
    /// Its public instance (`github.com`), if it has one.
    fn default_host(&self) -> Option<&'static str>;
    /// The host of its public instance or of one it hosts (`acme.ghe.com`), and the other names of
    /// its ssh (`ssh.github.com`): its own name, else none.
    fn own_host(&self, host: &str) -> Option<String>;
    /// The repository a remote's path names, as this host lays its paths out.
    fn repo_of(&self, segments: &[String]) -> Option<RepoRef> {
        RepoRef::two(segments)
    }
    /// `repo_of`, for a host whose name holds part of the repository (`<org>.visualstudio.com`).
    fn repo_in(&self, _host: &str, segments: &[String]) -> Option<RepoRef> {
        self.repo_of(segments)
    }
    /// The address of the instance a remote's repository is on, given its host's (`url`): an
    /// Azure DevOps organization's (`https://dev.azure.com/acme`).
    fn instance_of(&self, url: String, _repo: &RepoRef) -> String {
        url
    }
    /// The instance's address as the account keeps it, from the one typed (an Azure DevOps
    /// organization's former `https://acme.visualstudio.com` becomes `https://dev.azure.com/acme`).
    fn instance(&self, url: String) -> String {
        url
    }
    /// What the window shows in the address field: its public instance, else an example.
    fn address_hint(&self) -> Option<&'static str> {
        self.default_host()
    }
    /// What tells a self-hosted instance of it.
    fn probe(&self) -> Option<Probe> {
        None
    }
    /// Its tokens belong to an e-mail (Bitbucket's Atlassian API tokens).
    fn needs_email(&self) -> bool {
        false
    }
    /// The rights its tokens need, as the host names them.
    fn scopes(&self) -> Vec<String>;
    /// The page that makes a token on `url`'s instance.
    fn token_page(&self, url: &str) -> String;
    /// The command of its CLI that gives the token of `host`, when it has one.
    fn cli(&self, _host: &str) -> Option<CliToken> {
        None
    }
    /// The plugin of Escouade's own hand that serves its accounts, once its client left the core (GitHub's:
    /// `dev.escouade.github`, INT-8.1): the core asks that plugin (`ProcessForge`), and `client` is not called.
    fn plugin(&self) -> Option<&'static str> {
        None
    }
    fn client(&self, http: reqwest::Client, account: &Credentials) -> Box<dyn Forge>;
}

/// A host the core does not serve itself: its plugin does (`Provider::plugin`).
pub struct Unserved(pub &'static str);

impl Forge for Unserved {
    fn me(&self) -> BoxFuture<'_, Result<Identity>> {
        let name = self.0;
        Box::pin(async move {
            Err(anyhow!(tr!(
                "{name} est servi par son plugin, pas par le cœur d'Escouade",
                "{name} is served by its plugin, not by the core of Escouade"
            )))
        })
    }
}

/// The code hosts Escouade knows.
#[derive(Clone)]
pub struct Registry {
    providers: Vec<Arc<dyn Provider>>,
}

impl Registry {
    /// GitHub, GitLab, Bitbucket Cloud, Gitea and Forgejo, Azure DevOps.
    pub fn standard() -> Self {
        Self::from_providers(vec![
            Arc::new(github::GithubProvider),
            Arc::new(gitlab::GitlabProvider),
            Arc::new(bitbucket::BitbucketProvider),
            Arc::new(gitea::GiteaProvider),
            Arc::new(azure::AzureProvider),
        ])
    }

    /// The hosts of a list, in its order (the first-party ones, then the plugins').
    pub fn from_providers(providers: Vec<Arc<dyn Provider>>) -> Self {
        Self { providers }
    }

    /// One more host (a later one, a fake one in tests).
    pub fn with(mut self, provider: Arc<dyn Provider>) -> Self {
        self.providers.push(provider);
        self
    }

    pub fn all(&self) -> &[Arc<dyn Provider>] {
        &self.providers
    }

    pub fn get(&self, kind: &str) -> Option<Arc<dyn Provider>> {
        self.providers.iter().find(|p| p.kind() == kind).cloned()
    }

    /// The host whose public instance (or hosted one) `host` is, and its own name for it.
    pub fn of_host(&self, host: &str) -> Option<(Arc<dyn Provider>, String)> {
        let host = host.to_ascii_lowercase();
        self.providers
            .iter()
            .find_map(|p| p.own_host(&host).map(|h| (p.clone(), h)))
    }

    /// What a remote's address names: its host, the code host it is on (by its name, else the
    /// account connected for it, `known`), and the repository as that code host reads its path.
    pub fn analyze(&self, url: &str, known: impl Fn(&str) -> Option<String>) -> Option<Remote> {
        let addr = remote::parse(url)?;
        let (provider, host) = match self.of_host(&addr.host) {
            Some((p, h)) => (Some(p), h),
            None => (
                known(&addr.host).and_then(|kind| self.get(&kind)),
                addr.host.clone(),
            ),
        };
        let repo = match &provider {
            Some(p) => p.repo_in(&addr.host, &addr.segments)?,
            // Unknown: its path as most hosts lay it out.
            None => RepoRef::nested(&addr.segments)?,
        };
        // Its instance's address: an https remote's own (its port with it); https on its host for
        // the others (their port is their ssh's).
        let url = match (addr.scheme.as_str(), addr.port) {
            ("https" | "http", Some(port)) => format!("{}://{host}:{port}", addr.scheme),
            ("https" | "http", None) => format!("{}://{host}", addr.scheme),
            _ => format!("https://{host}"),
        };
        let url = match &provider {
            Some(p) => p.instance_of(url, &repo),
            None => url,
        };
        Some(Remote {
            kind: provider.map(|p| p.kind()),
            host,
            url,
            repo,
        })
    }
}

/// A project's remote, as `Registry::analyze` reads it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Remote {
    /// The code host, when its name or an account says it.
    pub kind: Option<String>,
    /// Its instance's host (`github.com`, `git.acme.dev`), without the port of its ssh.
    pub host: String,
    /// Its instance's address, as far as the remote tells it (`https://git.acme.dev:8443`).
    pub url: String,
    pub repo: RepoRef,
}

/// The instance's address of what was typed (`gitlab.acme.dev`, `https://gitlab.acme.dev/`).
pub fn instance_url(typed: &str) -> String {
    let s = typed.trim().trim_end_matches('/');
    if s.is_empty() || s.contains("://") {
        s.to_string()
    } else {
        format!("https://{s}")
    }
}

/// The host of an instance's address (`https://git.acme.dev:8443` → `git.acme.dev:8443`), in
/// lower case: what an account is known by, one per instance.
pub fn host_of(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    rest.split('/').next().unwrap_or("").to_ascii_lowercase()
}

/// The host without its port (what a remote's address gives).
pub fn bare_host(host: &str) -> &str {
    match host.rsplit_once(':') {
        Some((h, p)) if p.chars().all(|c| c.is_ascii_digit()) => h,
        _ => host,
    }
}

/// Sends a request to a code host and reads its JSON answer (null when empty) and its headers;
/// an error says what went wrong and what the host said.
pub(crate) async fn call(name: &str, rb: reqwest::RequestBuilder) -> Result<(Value, HeaderMap)> {
    crate::integrations::send_named(name, rb).await
}

/// The token was refused (401): expired or revoked.
pub fn refused(e: &anyhow::Error) -> bool {
    e.downcast_ref::<crate::integrations::HttpError>()
        .is_some_and(|h| h.status == 401)
}

/// The rights of `wanted` that `scopes` lacks.
pub fn missing_scopes(wanted: &[String], scopes: &[String]) -> Vec<String> {
    wanted
        .iter()
        .filter(|w| !scopes.contains(w))
        .map(|w| w.to_string())
        .collect()
}

/// A provider's kind no registry knows.
pub(crate) fn unknown_kind(kind: &str) -> anyhow::Error {
    anyhow!(tr!(
        "Hébergeur inconnu : {kind}",
        "Unknown code host: {kind}"
    ))
}
