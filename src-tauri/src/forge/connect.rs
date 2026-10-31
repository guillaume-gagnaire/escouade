//! Connecting, checking and forgetting the code hosts' accounts, and what the projects' remotes
//! say of them: through `Registry` and `Provider` only, whatever the host.

use super::accounts::{self, ForgeAccount, GITHUB_COM};
use super::*;
use crate::core::Core;
use crate::integrations::{jira, Account};
use crate::model::Service;
use crate::model::UiEvent;
use anyhow::bail;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::process::Stdio;
use std::time::Duration;
use tauri::Runtime;

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

/// Asks the host who the token is: its state, and who it is when it answered.
pub async fn check(
    provider: &dyn Provider,
    http: reqwest::Client,
    creds: &Credentials,
) -> (Status, Option<Identity>) {
    match provider.client(http, creds).me().await {
        Ok(id) => {
            let missing = id
                .scopes
                .as_deref()
                .map(|s| missing_scopes(provider.scopes(), s))
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
        if (probe.matches)(status, &body) {
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
    /// github.com's: GitHub Issues' account too.
    pub issues: bool,
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

/// Réglages › Intégrations › Hébergeurs de code.
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
        let (program, login) = (cli.program, cli.login);
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
        let token = (cli.parse)(&stdout, &stderr)
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
        Ok(creds)
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
        let client = self.provider(&a.kind)?.client(self.http()?, &creds);
        Ok((client, a))
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

    /// Réglages › Intégrations › Hébergeurs de code, with the state each account was last found in.
    pub async fn forges_view(&self) -> ForgesView {
        let registry = self.forges.read().clone();
        let providers = registry
            .all()
            .iter()
            .map(|p| ProviderView {
                kind: p.kind().into(),
                name: p.name().into(),
                request_word: p.request_word().into(),
                default_host: p.default_host().map(str::to_string),
                needs_email: p.needs_email(),
                cli: p.cli("").map(|c| c.program.to_string()),
                cli_login: p.cli("").map(|c| c.login.to_string()),
                address_hint: p.address_hint().map(str::to_string),
                scopes: p.scopes().iter().map(|s| s.to_string()).collect(),
                capabilities: p.capabilities(),
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
                    // GitLab Issues takes every GitLab account, Azure Boards every Azure DevOps one.
                    issues: host == GITHUB_COM || a.kind == "gitlab" || a.kind == "azure",
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
            Ok::<_, anyhow::Error>(check(&*p, self.http()?, &creds).await.0)
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
        if provider.kind() == "github" && host == GITHUB_COM {
            // One connection, two uses: Issues' account.
            let form = Account {
                token,
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
            api: None,
        };
        let (status, id) = check(&*provider, self.http()?, &creds).await;
        let Some(id) = id else {
            bail!(status.error.unwrap_or_default());
        };
        let mut account = ForgeAccount {
            kind: provider.kind().into(),
            url,
            email,
            token,
            label: id.label,
            user: id.user,
            via_cli,
            in_file: false,
            unread: false,
        };
        let left = !accounts::place(&*self.secrets, &mut account);
        {
            let mut all = self.accounts.write();
            all.forget_forges.retain(|h| *h != host);
            if left {
                all.forget_forges.push(host.clone());
            }
            all.forges.retain(|a| a.host() != host);
            all.forges.push(account);
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
        let had = self
            .accounts
            .read()
            .forges
            .iter()
            .find(|a| a.host() == host)
            .map(|a| !a.via_cli && !a.in_file);
        let forgotten = accounts::forget(&*self.secrets, &host);
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
    let remote = crate::git::sync_remote(&remotes, upstream.as_deref())?;
    crate::git::remote_url(path, remote).await
}
