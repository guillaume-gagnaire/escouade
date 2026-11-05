//! The accounts shared by a plugin's points (INT-6, ESC-212; `docs/PLUGINS.md` §4.4): what a kind
//! of account is (`AccountKind`: its hosts, how one connects, the fields to type, the rights its
//! tokens need, the page that makes one), which plugin serves an account, its name in the keychain
//! (`plugin:<id>:account:<account>`) and its budget of calls.
//!
//! **One connection, every use.** An account serves every point of its plugin: github.com's serves
//! GitHub Issues and the pull requests, a GitLab instance's GitLab Issues and the merge requests,
//! an Azure DevOps organization's Azure Boards and the pull requests. `AccountKind::sources` and
//! `AccountKind::forge` say which points; `points` says them for an account.
//!
//! **The kinds Escouade knows** are those of its integrations, still in the core
//! (`forge::Registry::standard`, `integrations::source::Registry::standard`), each under the id of
//! the plugin of the first hand that will serve it (`dev.escouade.github`…), and those an active
//! plugin declares (`contributes.accounts`, with its `ticketSources` and `forges` that name it).
//! When two plugins offer the same kind (a « Jira Data Center » of one's own beside Escouade's
//! Jira), the user chooses which one serves an account when adding it (`Account::plugin`,
//! `ForgeAccount::plugin`); never in silence.
//!
//! **The keychain.** An account's secret is kept under `plugin:<id>:account:<account>`: the
//! account is its instance's host for a code host (`gitlab.com`, `git.acme.dev`, several per
//! kind), and the kind's id for a kind of one account (`jira`, `trello`, `linear`, `clickup`).
//! What it holds is what the former entry held: a token, or for the kinds with more than one
//! secret (Trello's key and token) `{"key":…,"token":…}`. A plugin reads it by `secrets/get` of
//! `account:<account>`, under its own id only: never another plugin's.
//!
//! **The migration, reversible** (decided on 2026-11-01): the former entries (`jira`, `github`,
//! `forge:<host>`) stay beside the new ones, written and deleted with them, so that a version from
//! before reads its accounts as they are; `integrations.json` keeps its shape and says `vault: 2`
//! (`VAULT`), which a version from before ignores. While they are kept, the former entries are read
//! first and copied under the new names when these differ (`secrets::read_entry`): a token that a
//! version from before changed after a return to it is taken back. `revert` goes back: the former
//! entries hold the secrets, the new ones go. A version after this one drops the former entries.
//!
//! **The Mistral account** (ESC-327, INT-10.6 bis, `docs/PLUGINS.md` §4.13): the plugin `dev.escouade.mistral` declares
//! the kind `mistral`, whose points are its voice and Mistral Vibe. Its account `mistral` is the key of Réglages › Voix
//! (former name `dictation-mistral`); a Vibe account that has a key of its own keeps it, as an account `vibe-<id>` of
//! the same plugin (former name `vibe:<id>`), so that two Mistral organizations stay two accounts. The key goes to the
//! voice in each call and to Vibe by `MISTRAL_API_KEY`, as before: the plugin reads no secret. Each side falls back on
//! the other: a Vibe account without a key goes by the plugin's, the voice without a key by the first Vibe account's.

use super::manifest::Manifest;
use super::runtime::secret_name;
use crate::forge::{self, Provider, RateLimit};
use crate::integrations::secrets::SecretStore;
use crate::integrations::{source, Account, Accounts};
use crate::model::Service;
use serde::Serialize;
use serde_json::Value;

/// What `integrations.json` says once the secrets are also under the plugins' names.
pub const VAULT: u32 = 2;

/// The prefix of the ids of the plugins of Escouade's own hand.
const FIRST_HAND: &str = "dev.escouade.";

/// How an account connects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "by")]
pub enum Connect {
    /// A token typed in its form (with the fields of `AccountKind::fields`).
    Token,
    /// The token of the service's CLI, asked when needed (`gh auth token`).
    Cli { program: String, login: String },
}

/// A field of the form that adds an account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    /// `site`, `email`, `key`, `token`: what `integrations::Account` holds today.
    pub key: String,
    /// Kept in the keychain, never in `integrations.json` nor sent to the window.
    pub secret: bool,
}

impl Field {
    fn new(key: &str, secret: bool) -> Self {
        Self {
            key: key.into(),
            secret,
        }
    }
}

/// A kind of account, as a plugin declares it (`contributes.accounts`) and as Escouade knows those
/// of its integrations.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountKind {
    /// What the accounts know it by: `github`, `gitlab`, `jira`. Two plugins may declare the same
    /// one: each serves its own accounts of it.
    pub id: String,
    /// The plugin that declares it.
    pub plugin: String,
    /// That plugin's name (« Escouade » for its own hand's).
    pub plugin_name: String,
    /// As the window says it.
    pub name: String,
    /// Its public instance (`github.com`), if it has one.
    pub default_host: Option<String>,
    /// It has instances of one's own (GitLab's, a Jira site), told by their address, or by asking
    /// them (`probe`).
    pub self_hosted: bool,
    /// What tells an instance of it whose name says nothing: a path to ask.
    pub probe: Option<String>,
    /// How one connects, the first offered first.
    pub connect: Vec<Connect>,
    /// What its form asks.
    pub fields: Vec<Field>,
    /// The rights its tokens need, as the service names them.
    pub scopes: Vec<String>,
    /// The page that makes a token on its public instance.
    pub token_page: Option<String>,
    /// The ticket sources its accounts serve (`ExternalRef::service`).
    pub sources: Vec<String>,
    /// The code host its accounts serve (`ForgeAccount::kind`), if any.
    pub forge: Option<String>,
    /// The harnesses its accounts serve (`contributes.harnesses[].account`).
    pub harnesses: Vec<String>,
    /// The voice services its accounts serve (`voice` of the kind: they are no point of `contributes`).
    pub voice: Vec<String>,
    /// Its sources are served by its public instance's account only (GitHub Issues: github.com's,
    /// not a GitHub Enterprise Server's).
    pub sources_public_only: bool,
}

/// The plugin of Escouade's own hand that serves (or will serve, INT-7 and INT-8) the kind `kind`
/// of the core.
pub fn first_hand(kind: &str) -> String {
    match kind {
        "azure" => format!("{FIRST_HAND}azure-devops"),
        _ => format!("{FIRST_HAND}{kind}"),
    }
}

/// Whether `plugin` is of Escouade's own hand.
pub fn is_first_hand(plugin: &str) -> bool {
    plugin.is_empty() || plugin.starts_with(FIRST_HAND)
}

/// The kind of a code host of the core, from its provider: the sources of its accounts are those
/// the registry of sources serves by it.
fn of_provider(p: &dyn Provider) -> AccountKind {
    let kind = p.kind();
    let sources: Vec<String> = source::registry()
        .all()
        .iter()
        .filter(|s| s.host() == Some(kind.as_str()))
        .map(|s| s.id().to_string())
        .collect();
    let mut connect = vec![Connect::Token];
    if let Some(cli) = p.cli(p.default_host().unwrap_or("")) {
        connect.push(Connect::Cli {
            program: cli.program,
            login: cli.login,
        });
    }
    let mut fields = Vec::new();
    // Every host has instances of one's own (GitHub Enterprise, an Azure DevOps Server, a
    // self-managed GitLab, Gitea and Forgejo) but Bitbucket, whose Cloud alone is served.
    if kind != "bitbucket" {
        fields.push(Field::new("url", false));
    }
    if p.needs_email() {
        fields.push(Field::new("email", false));
    }
    fields.push(Field::new("token", true));
    AccountKind {
        plugin: first_hand(&kind),
        plugin_name: "Escouade".into(),
        name: p.name().into(),
        default_host: p.default_host().map(str::to_string),
        self_hosted: fields.iter().any(|f| f.key == "url"),
        probe: p.probe().map(|pr| pr.path),
        connect,
        fields,
        scopes: p.scopes(),
        token_page: p
            .default_host()
            .map(|h| p.token_page(&format!("https://{h}"))),
        sources_public_only: kind == "github",
        sources,
        forge: Some(kind.clone()),
        harnesses: Vec::new(),
        voice: Vec::new(),
        id: kind,
    }
}

/// The kind of a source of the core with an account of its own (Jira, Trello, Linear, ClickUp).
fn of_source(s: &dyn source::TicketSourceKind) -> AccountKind {
    let id = s.id();
    let (host, fields, page): (Option<&str>, &[(&str, bool)], &str) = match id {
        "jira" => (
            None,
            &[("site", false), ("email", false), ("token", true)],
            "https://id.atlassian.com/manage-profile/security/api-tokens",
        ),
        "trello" => (
            Some("trello.com"),
            &[("key", true), ("token", true)],
            "https://trello.com/power-ups/admin",
        ),
        "linear" => (
            Some("linear.app"),
            &[("token", true)],
            "https://linear.app/settings/account/security",
        ),
        "clickup" => (
            Some("app.clickup.com"),
            &[("token", true)],
            "https://app.clickup.com/settings/apps",
        ),
        _ => (None, &[("token", true)], ""),
    };
    AccountKind {
        id: id.into(),
        plugin: first_hand(id),
        plugin_name: "Escouade".into(),
        name: s.label().into(),
        default_host: host.map(str::to_string),
        self_hosted: host.is_none(),
        probe: None,
        connect: vec![Connect::Token],
        fields: fields.iter().map(|(k, s)| Field::new(k, *s)).collect(),
        scopes: Vec::new(),
        token_page: (!page.is_empty()).then(|| page.to_string()),
        sources: vec![id.into()],
        forge: None,
        harnesses: Vec::new(),
        voice: Vec::new(),
        sources_public_only: false,
    }
}

/// The kinds of the integrations of the core: the code hosts', then the sources' with an account
/// of their own.
pub fn standard() -> Vec<AccountKind> {
    let mut kinds: Vec<AccountKind> = forge::Registry::standard()
        .all()
        .iter()
        .map(|p| of_provider(&**p))
        .collect();
    for s in source::registry().all() {
        if s.own_account() && s.host().is_none() {
            kinds.push(of_source(&**s));
        }
    }
    kinds
}

/// The kinds a plugin declares (`contributes.accounts`), each with the sources and the host of its
/// points that name it (`account`). What it leaves out is empty: a token typed in a form.
pub fn of_manifest(m: &Manifest) -> Vec<AccountKind> {
    let items = |point: &str| -> Vec<&Value> {
        m.contributes
            .get(point)
            .and_then(Value::as_array)
            .map(|a| a.iter().collect())
            .unwrap_or_default()
    };
    let text = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    let words = |v: &Value, k: &str| -> Vec<String> {
        v.get(k)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|w| w.as_str().map(str::to_string))
            .collect()
    };
    let serving = |point: &str, id: &str| -> Vec<String> {
        items(point)
            .into_iter()
            .filter(|i| i.get("account").and_then(Value::as_str) == Some(id))
            .filter_map(|i| text(i, "id"))
            .collect()
    };
    items("accounts")
        .into_iter()
        .filter_map(|a| {
            let id = text(a, "id")?;
            let connect: Vec<Connect> = a
                .get("connect")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|c| match c {
                    Value::String(s) if s == "token" => Some(Connect::Token),
                    Value::Object(o) => Some(Connect::Cli {
                        program: o.get("cli")?.as_str()?.to_string(),
                        login: o
                            .get("login")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                    }),
                    _ => None,
                })
                .collect();
            let mut fields: Vec<Field> = a
                .get("fields")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|f| {
                    Some(Field {
                        key: text(f, "key")?,
                        secret: f.get("secret").and_then(Value::as_bool).unwrap_or(false),
                    })
                })
                .collect();
            if fields.is_empty() {
                fields.push(Field::new("token", true));
            }
            let sources = serving("ticketSources", &id);
            Some(AccountKind {
                name: text(a, "name").unwrap_or_else(|| id.clone()),
                plugin: m.id.clone(),
                plugin_name: m.name.get(crate::i18n::ui()).to_string(),
                default_host: text(a, "defaultHost"),
                self_hosted: a
                    .get("selfHosted")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                probe: text(a, "probe"),
                connect: if connect.is_empty() {
                    vec![Connect::Token]
                } else {
                    connect
                },
                fields,
                scopes: words(a, "scopes"),
                token_page: text(a, "tokenPage"),
                forge: serving("forges", &id).into_iter().next(),
                harnesses: serving("harnesses", &id),
                voice: words(a, "voice"),
                sources,
                sources_public_only: a
                    .get("sourcesPublicOnly")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                id,
            })
        })
        .collect()
}

/// The kinds Escouade knows: its integrations', then those the plugins `manifests` declare (a
/// plugin of the first hand that declares one of the core's takes its place, with the sources of the core its accounts
/// still serve: GitLab Issues on the accounts of the plugin « GitLab » until it carries them, INT-8.2).
pub fn kinds(manifests: &[Manifest]) -> Vec<AccountKind> {
    let mut all = standard();
    for m in manifests {
        for mut k in of_manifest(m) {
            if let Some(core) = all.iter().find(|x| x.id == k.id && x.plugin == k.plugin) {
                for s in &core.sources {
                    if !k.sources.contains(s) {
                        k.sources.push(s.clone());
                    }
                }
            }
            all.retain(|x| !(x.id == k.id && x.plugin == k.plugin));
            all.push(k);
        }
    }
    all
}

/// The kinds the active plugins of `plugins` declare, with the core's.
pub fn kinds_of(plugins: &super::Plugins) -> Vec<AccountKind> {
    let manifests: Vec<Manifest> = plugins
        .list(crate::i18n::Lang::En)
        .into_iter()
        .filter(|v| v.choices.enabled)
        .filter_map(|v| v.manifest)
        .collect();
    kinds(&manifests)
}

/// A plugin that can serve the accounts of a kind, as the window offers it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub plugin: String,
    /// The plugin's name, as the form offers it.
    pub name: String,
    /// Of Escouade's own hand: the one offered first.
    pub first_hand: bool,
}

/// The plugins that serve the kind `id`: Escouade's first, then the others by id.
pub fn servers(kinds: &[AccountKind], id: &str) -> Vec<Server> {
    let mut list: Vec<Server> = kinds
        .iter()
        .filter(|k| k.id == id)
        .map(|k| Server {
            first_hand: is_first_hand(&k.plugin),
            plugin: k.plugin.clone(),
            name: k.plugin_name.clone(),
        })
        .collect();
    list.sort_by(|a, b| (!a.first_hand, &a.plugin).cmp(&(!b.first_hand, &b.plugin)));
    list.dedup_by(|a, b| a.plugin == b.plugin);
    list
}

/// The plugin chosen to serve a new account of the kind `id`: none (`""`) for Escouade's own
/// hand, which the accounts do not write; an error for a plugin that does not serve it.
pub fn chosen(kinds: &[AccountKind], id: &str, plugin: &str) -> Result<String, String> {
    let plugin = plugin.trim();
    if plugin.is_empty() || plugin == first_hand(id) {
        return Ok(String::new());
    }
    if servers(kinds, id).iter().any(|s| s.plugin == plugin) {
        return Ok(plugin.to_string());
    }
    Err(tr!(
        "Le plugin {plugin} ne sert pas les comptes {id}.",
        "The plugin {plugin} does not serve {id} accounts."
    ))
}

/// The plugin that serves an account (`plugin` as it keeps it: empty for Escouade's own hand) of
/// the kind `kind`.
pub fn serving(kind: &str, plugin: &str) -> String {
    if plugin.is_empty() {
        first_hand(kind)
    } else {
        plugin.to_string()
    }
}

/// The kind of account of a source: the code host's whose accounts serve it, else its own.
pub fn kind_of_source(service: Service) -> String {
    source::registry()
        .get(service)
        .and_then(|k| k.host())
        .unwrap_or(service.as_str())
        .to_string()
}

/// The account of the source `service` with an account of its own, as the keychain names it: the
/// kind's id, or for GitHub Issues github.com's host.
pub fn source_account(service: Service) -> String {
    match source::registry().get(service).and_then(|k| k.host()) {
        Some("github") => forge::accounts::GITHUB_COM.to_string(),
        _ => service.as_str().to_string(),
    }
}

/// The keychain's name of the secret of the account `account` that `plugin` serves.
pub fn vault_name(plugin: &str, account: &str) -> String {
    secret_name(plugin, &format!("account:{account}"))
}

/// The keychain's name of the account of the source `service` (`a.plugin` serving it).
pub fn source_vault_name(service: Service, a: &Account) -> String {
    vault_name(
        &serving(&kind_of_source(service), &a.plugin),
        &source_account(service),
    )
}

/// The keychain's name of a code host's account.
pub fn forge_vault_name(a: &forge::accounts::ForgeAccount) -> String {
    vault_name(&serving(&a.kind, &a.plugin), &a.host())
}

/// The kind of the plugin Mistral, and the name of its account: the key the voice and Vibe share.
pub const MISTRAL: &str = "mistral";

/// The keychain's name of the Mistral account (its former name is the voice's, `dictation-mistral`).
pub fn mistral_vault_name() -> String {
    vault_name(super::builtin::MISTRAL, MISTRAL)
}

/// The keychain's name of the key of the Vibe account `id` (its former name is `vibe:<id>`).
pub fn vibe_vault_name(id: &str) -> String {
    vault_name(super::builtin::MISTRAL, &format!("vibe-{id}"))
}

/// The names an instance's token may have in the keychain whatever its kind: those of the code
/// hosts of the first hand (a « Déconnecter » that failed knows only the host).
pub fn forge_vault_names(host: &str) -> Vec<String> {
    forge::Registry::standard()
        .all()
        .iter()
        .map(|p| vault_name(&first_hand(&p.kind()), host))
        .collect()
}

/// The hosts the user typed for the accounts `plugin` serves (`git.acme.dev:8443`, a Jira site's): all `{account}` of
/// `permissions.network` reaches (ESC-332).
pub fn hosts_of(accounts: &Accounts, plugin: &str) -> Vec<String> {
    let mut hosts: Vec<String> = forge::accounts::all(accounts)
        .iter()
        .filter(|a| serving(&a.kind, &a.plugin) == plugin)
        .map(|a| a.host())
        .collect();
    for service in source::registry().services() {
        if let Some(a) = accounts.get(service) {
            if !a.site.is_empty() && serving(&kind_of_source(service), &a.plugin) == plugin {
                hosts.push(forge::host_of(&a.site));
            }
        }
    }
    hosts.retain(|h| !h.is_empty());
    hosts.sort();
    hosts.dedup();
    hosts
}

/// One account and the points it serves, as Réglages and the plugins see it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shared {
    /// Its kind (`gitlab`).
    pub kind: String,
    /// The plugin that serves it.
    pub plugin: String,
    /// Its name in the keychain's (`gitlab.com`, `jira`).
    pub account: String,
    pub label: String,
    /// The points it serves: `ticketSources:<id>`, `forges:<id>`, `harnesses:<id>`, `voice:<id>`.
    pub points: Vec<String>,
}

/// The points an account of the kind `kind` on `host` serves.
pub fn points(kind: &AccountKind, host: Option<&str>) -> Vec<String> {
    let public = host.is_none() || host == kind.default_host.as_deref();
    let mut list: Vec<String> = kind
        .sources
        .iter()
        .filter(|_| public || !kind.sources_public_only)
        .map(|s| format!("ticketSources:{s}"))
        .collect();
    if let Some(f) = &kind.forge {
        list.push(format!("forges:{f}"));
    }
    list.extend(kind.harnesses.iter().map(|h| format!("harnesses:{h}")));
    list.extend(kind.voice.iter().map(|v| format!("voice:{v}")));
    list
}

/// Every account, and what each serves: one connection, every use of its plugin.
pub fn shared(accounts: &Accounts, kinds: &[AccountKind]) -> Vec<Shared> {
    let kind = |id: &str, plugin: &str| {
        let plugin = serving(id, plugin);
        kinds
            .iter()
            .find(|k| k.id == id && k.plugin == plugin)
            .or_else(|| kinds.iter().find(|k| k.id == id))
            .cloned()
    };
    let mut list = Vec::new();
    for a in forge::accounts::all(accounts) {
        let host = a.host();
        let points = kind(&a.kind, &a.plugin)
            .map(|k| points(&k, Some(&host)))
            .unwrap_or_default();
        list.push(Shared {
            kind: a.kind.clone(),
            plugin: serving(&a.kind, &a.plugin),
            account: host,
            label: a.label.clone(),
            points,
        });
    }
    for service in source::registry().services() {
        let Some(a) = accounts.get(service) else {
            continue;
        };
        if source::registry()
            .get(service)
            .and_then(|k| k.host())
            .is_some()
        {
            // A code host's account, listed above (github.com's).
            continue;
        }
        let id = kind_of_source(service);
        list.push(Shared {
            points: kind(&id, &a.plugin)
                .map(|k| points(&k, None))
                .unwrap_or_default(),
            plugin: serving(&id, &a.plugin),
            account: source_account(service),
            label: a.label.clone(),
            kind: id,
        });
    }
    list
}

/// The keychain under the former names only again (a version from before is to be run, or the
/// test of the round trip): each account's secrets, read by `load_accounts`, are in its former
/// entry, and the new entries go. Ok once each is done; `accounts.vault` is then 0. Nothing calls
/// it while the former entries are kept up to date beside the new ones: the version that drops
/// them keeps it as its way back.
#[cfg_attr(not(test), allow(dead_code))]
pub fn revert(store: &dyn SecretStore, accounts: &mut Accounts) -> anyhow::Result<()> {
    use crate::integrations::secrets;
    for service in source::registry().services() {
        let Some(a) = accounts.get(service) else {
            continue;
        };
        if !a.unread && !secrets::through_gh(service, a) && is_first_hand(&a.plugin) {
            secrets::keep_legacy(store, service, a)?;
        }
        store.delete(&source_vault_name(service, a))?;
    }
    for a in &accounts.forges {
        if !a.unread && !a.via_cli && !a.token.is_empty() && is_first_hand(&a.plugin) {
            secrets::keep_secret(store, &forge::accounts::entry(&a.host()), &a.token)?;
        }
        store.delete(&forge_vault_name(a))?;
    }
    accounts.vault = 0;
    Ok(())
}

/// `revert`, for the keys of the plugin Mistral: `voice` the voice's former entry, `vibe` the former and new names of
/// the Vibe accounts' keys. Each former entry holds what is read, the new one goes.
#[cfg_attr(not(test), allow(dead_code))]
pub fn revert_mistral(
    store: &dyn SecretStore,
    voice: &str,
    vibe: &[(String, String)],
) -> anyhow::Result<()> {
    use crate::integrations::secrets;
    let voice_names = (voice.to_string(), mistral_vault_name());
    for (former, name) in vibe.iter().chain(std::iter::once(&voice_names)) {
        if let Some(secret) = secrets::read_entry(store, Some(former), name, former)?
            .or(store.get(name)?.filter(|s| !s.is_empty()))
        {
            secrets::keep_secret(store, former, &secret)?;
        }
        store.delete(name)?;
    }
    Ok(())
}

/// What a plugin says of an account's budget of calls, kept for that account: the calls of all
/// its points spend it (GitHub's 5,000 an hour are its token's, Issues and pull requests alike).
pub fn budget_key(plugin: &str, account: &str) -> String {
    vault_name(plugin, account)
}

/// The budget an answer of a plugin carries (`{ value, rateLimit }`), and its value.
pub fn split_budget(v: Value) -> (Value, Option<RateLimit>) {
    match v {
        Value::Object(mut o)
            if o.contains_key("value") && o.contains_key("rateLimit") && o.len() == 2 =>
        {
            let rate = o
                .remove("rateLimit")
                .and_then(|r| serde_json::from_value::<RateLimit>(r).ok());
            (o.remove("value").unwrap_or(Value::Null), rate)
        }
        v => (v, None),
    }
}
