//! Connecting, checking and forgetting the code hosts' accounts, and what the projects' remotes
//! say of them: through `Registry` and `Provider` only, whatever the host.

use super::accounts::{self, ForgeAccount, GITHUB_COM};
use super::*;
use crate::core::Core;
use crate::integrations::{jira, Account};
use crate::model::Service;
use crate::model::UiEvent;
use crate::plugins::runtime::Trial;
use anyhow::bail;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::process::Stdio;
use std::time::Duration;
use tauri::Runtime;

/// A project's code host, as the sidebar and the board need it (ESC-390).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeHost {
    pub kind: String,
    pub name: String,
    /// Its word for a pull request (« PR », « MR » on GitLab).
    pub word: String,
    pub pull_requests: bool,
    pub deployments: bool,
}

/// How an account stands, as its last check found it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Health {
    /// Not checked since the app started.
    #[default]
    Unchecked,
    Connected,
    /// Its token is refused (401): expired or revoked.
    Expired,
    /// Its token lacks rights (`Status::missing`).
    MissingScopes,
    /// The instance did not answer, or answered an error.
    Unreachable,
    /// The keychain did not give its token at this start.
    Unread,
}

/// What an account's check found.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub health: Health,
    /// The rights its token lacks, as the host names them.
    pub missing: Vec<String>,
    /// What the host said, when it refused or failed.
    pub error: Option<String>,
}

/// Asks the host who the token is: its state, and who it is when it answered. A host whose client
/// left the core (`Provider::plugin`) is asked through its plugin, as the core does (the tests').
#[cfg(test)]
pub async fn check(
    provider: &dyn Provider,
    http: reqwest::Client,
    creds: &Credentials,
) -> (Status, Option<Identity>) {
    let client = match provider.plugin() {
        Some(plugin) => crate::plugins::builtin::served_forge(plugin, &provider.kind(), creds),
        None => provider.client(http, creds),
    };
    check_client(&*client, &provider.scopes()).await
}

/// `check`, with the client of the account (a plugin's, `plugins::adapters::ProcessForge`) and
/// the rights its tokens need.
pub async fn check_client(client: &dyn Forge, scopes: &[String]) -> (Status, Option<Identity>) {
    match client.me().await {
        Ok(id) => {
            let missing = id
                .scopes
                .as_deref()
                .map(|s| missing_scopes(scopes, s))
                .unwrap_or_default();
            let health = if missing.is_empty() {
                Health::Connected
            } else {
                Health::MissingScopes
            };
            let status = Status {
                health,
                missing,
                error: None,
            };
            (status, Some(id))
        }
        Err(e) => {
            let health = if refused(&e) {
                Health::Expired
            } else {
                Health::Unreachable
            };
            let status = Status {
                health,
                missing: Vec::new(),
                error: Some(format!("{e:#}")),
            };
            (status, None)
        }
    }
}

/// The kind of the code host at `url`: by its name, else by asking it what each self-hosted one
/// answers (`Provider::probe`).
pub async fn guess(registry: &Registry, http: &reqwest::Client, url: &str) -> Option<String> {
    let url = instance_url(url);
    if let Some((p, _)) = registry.of_host(bare_host(&host_of(&url))) {
        return Some(p.kind().to_string());
    }
    for p in registry.all() {
        let Some(probe) = p.probe() else { continue };
        let Ok(resp) = http
            .get(format!("{url}{}", probe.path))
            .timeout(Duration::from_secs(8))
            .send()
            .await
        else {
            continue;
        };
        let status = resp.status().as_u16();
        let body: Value = resp.json().await.unwrap_or(Value::Null);
        if probe.matches(status, &body) {
            return Some(p.kind().to_string());
        }
    }
    None
}

/// « Connecter un hébergeur… »: its kind, its instance (its public one when empty), and a token
/// (empty: its CLI's).
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ForgeForm {
    pub kind: String,
    pub url: String,
    pub email: String,
    pub token: String,
    /// The plugin chosen to serve it, when several do (`ProviderView::servers`); empty for
    /// Escouade's own hand.
    pub plugin: String,
}

/// A code host as the window offers it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderView {
    pub kind: String,
    pub name: String,
    pub request_word: String,
    pub default_host: Option<String>,
    pub needs_email: bool,
    /// Its CLI, whose token serves when none is given.
    pub cli: Option<String>,
    /// How its CLI is logged in (`gh auth login`, `az login`).
    pub cli_login: Option<String>,
    /// What the address field shows when empty (`dev.azure.com/acme`).
    pub address_hint: Option<String>,
    pub scopes: Vec<String>,
    pub capabilities: Capabilities,
    /// The plugins that may serve a new account of it, Escouade's first: the form offers the
    /// choice when there are several.
    pub servers: Vec<crate::plugins::accounts::Server>,
    /// The plugin the app gives that serves its accounts, once its client left the core (GitHub's,
    /// INT-8.1), read from the plugins' registry: whether it is on, and why it stopped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub served_by: Option<ServedBy>,
}

/// A plugin the app gives, as the view of the code hosts shows it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServedBy {
    pub plugin: String,
    pub name: String,
    /// Switched on for the app (Réglages › Plugins).
    pub on: bool,
    /// It stopped by itself (crashes, memory, a start refused), in words; none while it runs or
    /// waits for its first call.
    pub stopped: Option<String>,
}

/// An account as the window knows it: never its token.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgeAccountView {
    pub kind: String,
    pub name: String,
    pub host: String,
    pub url: String,
    pub label: String,
    pub via_cli: bool,
    pub in_file: bool,
    /// It serves its plugin's ticket source too (github.com's GitHub Issues, a GitLab's GitLab
    /// Issues, an Azure DevOps organization's Azure Boards): `plugins::accounts::points`.
    pub issues: bool,
    /// The plugin that serves it, when another than Escouade's own hand.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub plugin: String,
    pub status: Status,
    /// The projects whose remote is on its instance.
    pub projects: Vec<String>,
}

/// A host of the projects' remotes without an account: « Connecter <hôte> ».
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UnknownHost {
    pub host: String,
    /// Its instance's address, as the remote tells it.
    pub url: String,
    /// When its name says it.
    pub kind: Option<String>,
    pub projects: Vec<String>,
}

/// Réglages › Services connectés › Hébergeurs de code.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgesView {
    pub providers: Vec<ProviderView>,
    pub accounts: Vec<ForgeAccountView>,
    pub unknown: Vec<UnknownHost>,
    /// The ticket systems' accounts: github.com's is GitHub Issues' one.
    pub issues: Vec<crate::integrations::AccountView>,
}

/// How long a CLI may take to give its token.
const CLI_TOKEN_LIMIT: Duration = Duration::from_secs(15);

impl<R: Runtime> Core<R> {
    fn http(&self) -> Result<reqwest::Client> {
        crate::usage::http_client(&self.settings.read())
    }

    fn provider(&self, kind: &str) -> Result<Arc<dyn Provider>> {
        self.forges
            .read()
            .get(kind)
            .ok_or_else(|| unknown_kind(kind))
    }

    /// The token of `host`'s CLI (`gh`, `glab`), asked once (again when `fresh`).
    async fn cli_token(&self, provider: &dyn Provider, host: &str, fresh: bool) -> Result<String> {
        if provider.kind() == "github" && host == GITHUB_COM {
            if fresh {
                *self.gh_token.lock() = None;
            }
            return self.gh_token().await;
        }
        if !fresh {
            // An Entra ID token (`az`'s) lasts an hour: asked again before it ends.
            if let Some(t) = self
                .forge_cli_tokens
                .lock()
                .get(host)
                .filter(|t| !expires_soon(t, now_secs()))
            {
                return Ok(t.clone());
            }
        }
        let name = provider.name();
        let Some(cli) = provider.cli(host) else {
            bail!(tr!("{name} : donne un jeton.", "{name}: give a token."));
        };
        let (program, login) = (cli.program.as_str(), cli.login.as_str());
        let none = || {
            anyhow!(tr!(
                "{name} : donne un jeton, ou connecte la CLI {program} à {host} ({login})",
                "{name}: give a token, or log the {program} CLI in to {host} ({login})"
            ))
        };
        let path = if program == "gh" {
            self.gh_cli()
        } else if cfg!(test) {
            None
        } else {
            crate::which::find(program)
        }
        .ok_or_else(none)?;
        let mut cmd = tokio::process::Command::new(path);
        cmd.args(&cli.args)
            .env("GH_PROMPT_DISABLED", "1")
            .env("NO_PROMPT", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(windows)]
        cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
        let out = tokio::time::timeout(CLI_TOKEN_LIMIT, cmd.output())
            .await
            .map_err(|_| none())??;
        let (stdout, stderr) = (
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
        let token = cli
            .parse(&stdout, &stderr)
            .filter(|_| out.status.success())
            .ok_or_else(none)?;
        self.forge_cli_tokens
            .lock()
            .insert(host.to_string(), token.clone());
        Ok(token)
    }

    /// Its credentials, its CLI's token asked when it has none of its own.
    pub(crate) async fn credentials(&self, a: &ForgeAccount) -> Result<Credentials> {
        let mut creds = a.credentials();
        if a.via_cli {
            creds.token = self
                .cli_token(&*self.provider(&a.kind)?, &a.host(), false)
                .await?;
        }
        if a.host() == GITHUB_COM {
            // Where GitHub Issues reaches it.
            creds.api = Some(self.bases.read().github.clone());
        }
        if a.kind == "azure" {
            // Where dev.azure.com's API is when a fake one stands in (`ESCOUADE_AZURE_DEVOPS_API`).
            creds.api = creds.api.or_else(|| azure::api_override(&creds.url));
        }
        Ok(creds)
    }

    /// The login of the account of `host`'s instance; empty without one.
    pub fn forge_login(&self, host: &str) -> String {
        let list = accounts::all(&self.accounts.read());
        accounts::for_host(&list, host).map_or_else(String::new, |a| a.user.clone())
    }

    /// A client of the account of `host`'s instance.
    pub async fn forge_client(&self, host: &str) -> Result<(Box<dyn Forge>, ForgeAccount)> {
        let list = accounts::all(&self.accounts.read());
        let a = accounts::for_host(&list, host).cloned().ok_or_else(|| {
            anyhow!(tr!(
                "Aucun compte connecté pour {host}",
                "No account connected for {host}"
            ))
        })?;
        if a.unread {
            bail!(tr!(
                "Trousseau du système illisible : relance Escouade ou reconnecte le compte de {host}.",
                "The system keychain can’t be read: restart Escouade or connect the {host} account again."
            ));
        }
        let creds = self.credentials(&a).await?;
        let client = self.client_of(&a.kind, &a.plugin, &creds, None)?;
        Ok((client, a))
    }

    /// The client of an account of the code host `kind`: its plugin's (`ProcessForge`) when
    /// another than Escouade's own hand serves it (`plugin`), else the core's.
    /// A host whose client left the core (`Provider::plugin`: GitHub's) is its plugin's too, the
    /// account's token lent to it as the core holds it (the user's, or its CLI's): for as long as the account lasts,
    /// or for the `trial` of an account not saved yet, with its host.
    fn client_of(
        &self,
        kind: &str,
        plugin: &str,
        creds: &Credentials,
        trial: Option<&Trial>,
    ) -> Result<Box<dyn Forge>> {
        let provider = self.provider(kind)?;
        let first_hand = provider.plugin().filter(|_| plugin.is_empty());
        if let Some(own) = first_hand {
            let host = host_of(&creds.url);
            match trial {
                Some(t) => t.lend(own, &format!("account:{host}"), &creds.token, &host),
                None => self.lend_account(own, &host, &creds.token),
            }
            #[cfg(any(debug_assertions, test))]
            self.plugin_runtime.set_routes(self.bases.read().faked());
        }
        if let Some(plugin) = first_hand.or((!plugin.is_empty()).then_some(plugin)) {
            return Ok(Box::new(crate::plugins::adapters::ProcessForge::new(
                self.plugin_runtime.clone(),
                plugin,
                kind,
                creds,
            )));
        }
        Ok(provider.client(self.http()?, creds))
    }

    /// Lends `own` the token of the account of `host`, unless it went while its token was read: under the lock its
    /// removal takes, so that no loan comes after `take_back`.
    pub(crate) fn lend_account(&self, own: &str, host: &str, token: &str) {
        let all = self.accounts.read();
        if accounts::all(&all).iter().any(|a| a.host() == host) {
            self.plugin_runtime
                .lend(own, &format!("account:{host}"), token);
        }
    }

    /// The plugin the app gives `id`, as the code hosts' view shows it.
    fn served_by(&self, id: &str) -> ServedBy {
        use crate::plugins::runtime::Stop;
        let v = self.plugins.get(id, crate::i18n::ui());
        let on = v
            .as_ref()
            .is_some_and(|v| v.choices.enabled && v.state == crate::plugins::State::Ready);
        let stopped = self
            .plugin_runtime
            .status(id)
            .stop
            .filter(|s| !matches!(s, Stop::Asked | Stop::Shutdown))
            .map(|s| s.describe(Duration::from_secs(300)));
        ServedBy {
            plugin: id.to_string(),
            name: v.map_or_else(|| id.to_string(), |v| v.name),
            on,
            stopped,
        }
    }

    /// The code host of a project (ESC-390): its remote recognised by a code host whose plugin is
    /// on. None without a remote, on a host Escouade does not know, or whose plugin is off: the
    /// sidebar has no « PR » tab then, and the board no « Environnements ».
    pub async fn project_code_host(&self, project_id: &str) -> Option<CodeHost> {
        let remote = self.project_forge(project_id).await?;
        let provider = remote
            .kind
            .as_deref()
            .and_then(|k| self.forges.read().get(k))?;
        if let Some(id) = provider.plugin() {
            if !self.served_by(id).on {
                return None;
            }
        }
        let can = provider.capabilities();
        Some(CodeHost {
            kind: provider.kind(),
            name: provider.name().to_string(),
            word: provider.request_word().to_string(),
            pull_requests: can.pull_requests,
            deployments: can.deployments,
        })
    }

    /// What a project's remote names (the remote of its target branch, else origin, else the
    /// only one): its code host and repository.
    pub async fn project_forge(&self, project_id: &str) -> Option<Remote> {
        let p = self.project(project_id).ok()?;
        self.remote_of(&p.path, &p.board.target).await
    }

    pub(crate) async fn remote_of(&self, path: &str, target: &str) -> Option<Remote> {
        let url = project_remote_url(path, target).await?;
        let list = accounts::all(&self.accounts.read());
        let registry = self.forges.read().clone();
        registry.analyze(&url, |host| accounts::kind_for(&list, host))
    }

    /// Réglages › Services connectés › Hébergeurs de code, with the state each account was last found in.
    pub async fn forges_view(&self) -> ForgesView {
        let registry = self.forges.read().clone();
        let kinds = crate::plugins::accounts::kinds_of(&self.plugins);
        let providers = registry
            .all()
            .iter()
            .map(|p| ProviderView {
                servers: crate::plugins::accounts::servers(&kinds, &p.kind()),
                kind: p.kind(),
                name: p.name().into(),
                request_word: p.request_word().into(),
                default_host: p.default_host().map(str::to_string),
                needs_email: p.needs_email(),
                cli: p.cli("").map(|c| c.program),
                cli_login: p.cli("").map(|c| c.login),
                address_hint: p.address_hint().map(str::to_string),
                scopes: p.scopes(),
                capabilities: p.capabilities(),
                served_by: p.plugin().map(|id| self.served_by(id)),
            })
            .collect();
        let list = accounts::all(&self.accounts.read());
        // The projects of each host.
        let projects = self.projects.read().clone();
        let mut by_host: BTreeMap<String, (Option<String>, String, Vec<String>)> = BTreeMap::new();
        for p in &projects {
            if let Some(r) = self.remote_of(&p.path, &p.board.target).await {
                let host = accounts::for_host(&list, &r.host).map_or(r.host.clone(), |a| a.host());
                let entry =
                    by_host
                        .entry(host)
                        .or_insert((r.kind.clone(), r.url.clone(), Vec::new()));
                if !entry.2.contains(&p.name) {
                    entry.2.push(p.name.clone());
                }
            }
        }
        let health = self.forge_health.lock().clone();
        let shared = crate::plugins::accounts::shared(&self.accounts.read(), &kinds);
        let accounts = list
            .iter()
            .map(|a| {
                let host = a.host();
                let mut status = health.get(&host).cloned().unwrap_or_default();
                if a.unread {
                    status.health = Health::Unread;
                }
                ForgeAccountView {
                    name: registry
                        .get(&a.kind)
                        .map_or(a.kind.clone(), |p| p.name().to_string()),
                    kind: a.kind.clone(),
                    url: a.url.clone(),
                    label: a.label.clone(),
                    via_cli: a.via_cli,
                    in_file: a.in_file,
                    // One connection, every use: GitLab Issues takes every GitLab account, Azure
                    // Boards every Azure DevOps one, GitHub Issues github.com's.
                    issues: shared
                        .iter()
                        .find(|s| s.account == host && s.kind == a.kind)
                        .is_some_and(|s| s.points.iter().any(|p| p.starts_with("ticketSources:"))),
                    plugin: a.plugin.clone(),
                    status,
                    projects: by_host.remove(&host).map(|(_, _, p)| p).unwrap_or_default(),
                    host,
                }
            })
            .collect();
        let unknown = by_host
            .into_iter()
            .map(|(host, (kind, url, projects))| UnknownHost {
                host,
                kind,
                url,
                projects,
            })
            .collect();
        ForgesView {
            providers,
            accounts,
            unknown,
            issues: self.integration_accounts(),
        }
    }

    /// Asks each account's host who its token is: connected, expired, rights missing.
    pub async fn forge_check(&self) -> ForgesView {
        let list = accounts::all(&self.accounts.read());
        for a in list.iter().filter(|a| !a.unread) {
            let status = self.forge_status(a).await;
            self.forge_health.lock().insert(a.host(), status);
        }
        self.forges_view().await
    }

    /// What the host says of the account's token now.
    async fn forge_status(&self, a: &ForgeAccount) -> Status {
        let asked = async {
            let p = self.provider(&a.kind)?;
            let creds = self.credentials(a).await?;
            let client = self.client_of(&a.kind, &a.plugin, &creds, None)?;
            Ok::<_, anyhow::Error>(check_client(&*client, &p.scopes()).await.0)
        };
        asked.await.unwrap_or_else(|e| Status {
            health: Health::Unreachable,
            missing: Vec::new(),
            error: Some(format!("{e:#}")),
        })
    }

    /// The kind of the code host at `url`, by its name or by asking it.
    pub async fn forge_guess(&self, url: &str) -> Result<Option<String>> {
        let url = instance_url(url);
        if !jira::secure(&url) {
            bail!(insecure());
        }
        let registry = self.forges.read().clone();
        Ok(guess(&registry, &self.http()?, &url).await)
    }

    /// The page that makes a token for `kind` on `url`'s instance (its public one when empty).
    pub fn forge_token_page(&self, kind: &str, url: &str) -> Result<String> {
        let p = self.provider(kind)?;
        let url = p.instance(instance_url(if url.trim().is_empty() {
            p.default_host().unwrap_or("")
        } else {
            url
        }));
        Ok(p.token_page(&url))
    }

    /// « Connecter » : the token is checked with the host (its CLI's when none is given), then the
    /// account is saved, its token in the keychain. github.com's is Issues' account.
    pub async fn forge_connect(&self, form: ForgeForm) -> Result<ForgesView> {
        let provider = self.provider(form.kind.trim())?;
        // Its plugin of the first hand, from the seed if it is not installed (ESC-373): the gesture asks for it.
        if form.plugin.trim().is_empty() {
            if let Some(p) = crate::plugins::sort::of_forge(&provider.kind()) {
                self.need_plugin(p);
            }
        }
        let typed = if form.url.trim().is_empty() {
            provider.default_host().unwrap_or("")
        } else {
            form.url.as_str()
        };
        let url = provider.instance(instance_url(typed));
        if url.is_empty() {
            bail!(tr!(
                "Indique l'adresse de l'instance.",
                "Give the instance’s address."
            ));
        }
        if !jira::secure(&url) {
            bail!(insecure());
        }
        let email = form.email.trim().to_string();
        if provider.needs_email() && email.is_empty() {
            bail!(tr!("Indique l'e-mail.", "Give the email."));
        }
        let host = host_of(&url);
        let token = form.token.trim().to_string();
        // The plugin that serves it: Escouade's own hand, unless another one is chosen.
        let plugin = crate::plugins::accounts::chosen(
            &crate::plugins::accounts::kinds_of(&self.plugins),
            &provider.kind(),
            &form.plugin,
        )
        .map_err(|e| anyhow!(e))?;
        if provider.kind() == "github" && host == GITHUB_COM {
            // One connection, two uses: Issues' account.
            let form = Account {
                token,
                plugin,
                ..Default::default()
            };
            self.integration_connect(Service::GITHUB, form).await?;
            self.forge_health.lock().remove(GITHUB_COM);
            return Ok(self.forge_check_one(GITHUB_COM).await);
        }
        let via_cli = token.is_empty();
        let used = if via_cli {
            self.cli_token(&*provider, &host, true).await?
        } else {
            token.clone()
        };
        let creds = Credentials {
            url: url.clone(),
            email: email.clone(),
            token: used,
            // Where dev.azure.com's API is when a fake one stands in (`ESCOUADE_AZURE_DEVOPS_API`).
            api: azure::api_override(&url),
        };
        let mut account = ForgeAccount {
            kind: provider.kind(),
            url,
            email,
            token,
            via_cli,
            plugin,
            ..ForgeAccount::default()
        };
        // A plugin takes the token from the keychain itself (`secrets/get`): it is there first.
        if !account.plugin.is_empty() {
            accounts::place(&*self.secrets, &mut account);
        }
        let trial = self.plugin_runtime.trial();
        let client = self.client_of(&account.kind, &account.plugin, &creds, Some(&trial))?;
        let (status, id) = check_client(&*client, &provider.scopes()).await;
        let Some(id) = id else {
            if !account.plugin.is_empty() {
                accounts::forget_of(&*self.secrets, &host, &account.kind, &account.plugin);
            }
            bail!(status.error.unwrap_or_default());
        };
        account.label = id.label;
        account.user = id.user;
        let left = !accounts::place(&*self.secrets, &mut account);
        {
            let mut all = self.accounts.write();
            all.forget_forges.retain(|h| *h != host);
            if left {
                all.forget_forges.push(host.clone());
            }
            all.forges.retain(|a| a.host() != host);
            all.forges.push(account);
            self.plugin_runtime.take_back(&format!("account:{host}"));
            trial.keep();
        }
        crate::integrations::secrets::save_accounts(&self.data, &self.accounts.read())?;
        self.forge_health.lock().insert(host, status);
        Ok(self.forges_view().await)
    }

    /// Checks the account of `host` only, then gives the view.
    async fn forge_check_one(&self, host: &str) -> ForgesView {
        let list = accounts::all(&self.accounts.read());
        if let Some(a) = accounts::for_host(&list, host) {
            let status = self.forge_status(a).await;
            self.forge_health.lock().insert(a.host(), status);
        }
        self.forges_view().await
    }

    /// « Déconnecter » : the account goes, its token with it. github.com's is Issues' account.
    pub async fn forge_disconnect(&self, host: &str) -> Result<ForgesView> {
        let host = host.trim().to_ascii_lowercase();
        self.forge_health.lock().remove(&host);
        self.forge_cli_tokens.lock().remove(&host);
        if host == GITHUB_COM {
            self.integration_disconnect(Service::GITHUB)?;
            return Ok(self.forges_view().await);
        }
        let (had, kind, plugin) = self
            .accounts
            .read()
            .forges
            .iter()
            .find(|a| a.host() == host)
            .map_or((None, String::new(), String::new()), |a| {
                (
                    Some(!a.via_cli && !a.in_file),
                    a.kind.clone(),
                    a.plugin.clone(),
                )
            });
        let forgotten = accounts::forget_of(&*self.secrets, &host, &kind, &plugin);
        if !forgotten && had == Some(true) {
            self.hub.emit(UiEvent::Toast {
                text: tr!(
                    "Le jeton de {host} n'a pas pu être retiré du trousseau du système : Escouade réessaiera au prochain démarrage, ou retire-le à la main (son nom contient « escouade »).",
                    "The {host} token couldn’t be removed from the system keychain: Escouade will try again at its next start, or remove it by hand (its name holds “escouade”)."
                ),
            });
        }
        {
            let mut all = self.accounts.write();
            if !forgotten && !all.forget_forges.contains(&host) {
                all.forget_forges.push(host.clone());
            }
            all.forges.retain(|a| a.host() != host);
            self.plugin_runtime.take_back(&format!("account:{host}"));
        }
        crate::integrations::secrets::save_accounts(&self.data, &self.accounts.read())?;
        Ok(self.forges_view().await)
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// A JWT whose `exp` comes within five minutes of `now`; any other token never expires here.
pub fn expires_soon(token: &str, now: u64) -> bool {
    use base64::Engine;
    let mut parts = token.split('.');
    let (Some(_), Some(payload), Some(_), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    let Ok(bytes) =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload.trim_end_matches('='))
    else {
        return false;
    };
    serde_json::from_slice::<Value>(&bytes)
        .ok()
        .and_then(|v| v["exp"].as_u64())
        .is_some_and(|exp| exp <= now + 300)
}

fn insecure() -> String {
    tr!(
        "L'adresse doit être en https (http seulement sur cette machine).",
        "The address must use https (http only on this machine)."
    )
}

/// The address of the remote of `target`'s upstream, else of origin, else of the only remote.
async fn project_remote_url(path: &str, target: &str) -> Option<String> {
    let remote = project_remote_name(path, target).await?;
    crate::git::remote_url(path, &remote).await
}

/// The name of the git remote the project's target syncs with (the one its code host is read on).
pub(crate) async fn project_remote_name(path: &str, target: &str) -> Option<String> {
    let remotes = crate::git::remotes(path).await;
    let upstream = if target.is_empty() {
        None
    } else {
        crate::git::text(
            path,
            &[
                "rev-parse",
                "--abbrev-ref",
                &format!("{target}@{{upstream}}"),
            ],
        )
        .await
        .ok()
        .filter(|u| !u.is_empty())
    };
    crate::git::sync_remote(&remotes, upstream.as_deref()).map(str::to_string)
}
