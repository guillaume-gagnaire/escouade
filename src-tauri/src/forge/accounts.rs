//! The code hosts' accounts: one per instance (its host), several per code host. They are saved
//! with the ticket systems' ones (`integrations.json`), their tokens in the system's keychain. The
//! account of github.com is GitHub Issues' one (`Accounts::github`): one connection, two uses.

use super::{host_of, Credentials};
use crate::integrations::secrets::{self, SecretStore};
use crate::integrations::{Account, Accounts, VIA_GH};
use crate::plugins::accounts;
use serde::{Deserialize, Serialize};

/// The host of github.com's account, which `Accounts::github` holds.
pub const GITHUB_COM: &str = "github.com";

/// An account of a code host's instance, as saved.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ForgeAccount {
    /// Its provider's kind (`gitlab`, `gitea`…).
    pub kind: String,
    /// Its instance's address (`https://gitlab.com`, `https://git.acme.dev`).
    pub url: String,
    /// Bitbucket: the Atlassian account's e-mail.
    pub email: String,
    /// Its token; empty once the keychain holds it, and when it is its CLI's.
    pub token: String,
    /// As the window shows it: "@ada".
    pub label: String,
    /// Its login.
    pub user: String,
    /// It has no token of its own: its CLI's (`gh`, `glab`), asked when needed.
    pub via_cli: bool,
    /// The plugin that serves it, chosen when it was added (`plugins::accounts`); empty for
    /// Escouade's own hand.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub plugin: String,
    /// The keychain refused its token: it stays in `integrations.json`. Never saved.
    #[serde(skip)]
    pub in_file: bool,
    /// The keychain did not give its token at this start. Never saved.
    #[serde(skip)]
    pub unread: bool,
}

impl ForgeAccount {
    /// What it is known by: its instance's host, one account per instance.
    pub fn host(&self) -> String {
        host_of(&self.url)
    }

    pub fn credentials(&self) -> Credentials {
        Credentials {
            url: self.url.clone(),
            email: self.email.clone(),
            token: self.token.clone(),
            api: None,
        }
    }

    /// github.com's account, as Issues' account holds it.
    pub fn of_github(a: &Account) -> Self {
        let via_cli = a.label.ends_with(VIA_GH);
        Self {
            kind: "github".into(),
            url: format!("https://{GITHUB_COM}"),
            email: String::new(),
            token: a.token.clone(),
            label: a.label.trim_end_matches(VIA_GH).to_string(),
            user: a.user.clone(),
            via_cli,
            plugin: a.plugin.clone(),
            in_file: a.in_file,
            unread: a.unread,
        }
    }
}

/// Every code host's account: github.com's (Issues' one) first, then the others by host.
pub fn all(accounts: &Accounts) -> Vec<ForgeAccount> {
    let mut list: Vec<ForgeAccount> = accounts.forges.clone();
    list.sort_by_key(|a| a.host());
    if let Some(gh) = &accounts.github {
        list.insert(0, ForgeAccount::of_github(gh));
    }
    list
}

/// The account of the instance whose host a remote names (its ssh has no port, or another one).
pub fn for_host<'a>(list: &'a [ForgeAccount], host: &str) -> Option<&'a ForgeAccount> {
    let host = host.to_ascii_lowercase();
    list.iter()
        .find(|a| a.host() == host)
        .or_else(|| list.iter().find(|a| super::bare_host(&a.host()) == host))
}

/// The kind of the account connected for `host`, if one is (an instance whose name says nothing).
pub fn kind_for(list: &[ForgeAccount], host: &str) -> Option<String> {
    for_host(list, host).map(|a| a.kind.clone())
}

/// The keychain's entry of an instance's token.
pub fn entry(host: &str) -> String {
    format!("forge:{host}")
}

/// The account's token goes into the keychain, read back to check, else it stays in the file
/// (`in_file`): under its plugin's name (`plugins::accounts`), and under its former one for a
/// version from before when Escouade's own hand serves it. Without one (its CLI's), a former
/// account's entries go: false when they stay.
pub fn place(store: &dyn SecretStore, a: &mut ForgeAccount) -> bool {
    a.in_file = false;
    if a.token.is_empty() {
        return forget_of(store, &a.host(), &a.kind, &a.plugin);
    }
    let kept =
        secrets::keep_secret(store, &accounts::forge_vault_name(a), &a.token).and_then(|()| {
            if accounts::is_first_hand(&a.plugin) {
                secrets::keep_secret(store, &entry(&a.host()), &a.token)
            } else {
                Ok(())
            }
        });
    if let Err(e) = kept {
        log::warn!(
            "{}: token left in integrations.json, the system keychain refused it: {e:#}",
            a.host()
        );
        a.in_file = true;
    }
    true
}

/// The instance's token leaves the keychain, under every name it may have (`plugin`: the plugin
/// that served it, when known); false when it could not (logged).
pub fn forget(store: &dyn SecretStore, host: &str) -> bool {
    forget_of(store, host, "", "")
}

/// `forget`, for an account of the kind `kind` (empty: unknown) that `plugin` served (empty:
/// Escouade's own hand).
pub fn forget_of(store: &dyn SecretStore, host: &str, kind: &str, plugin: &str) -> bool {
    let mut names = vec![entry(host)];
    names.extend(accounts::forge_vault_names(host));
    if !kind.is_empty() {
        names.push(accounts::vault_name(&accounts::serving(kind, plugin), host));
    }
    let gone = names.iter().try_for_each(|n| store.delete(n));
    if let Err(e) = gone {
        log::warn!("{host}: the system keychain did not forget its token: {e:#}");
        return false;
    }
    true
}

/// The account's token, from the keychain (`secrets::read_entry`: its former name first while it
/// is kept, copied under its plugin's); missing, the account says so (`unread`).
fn fill(store: &dyn SecretStore, a: &mut ForgeAccount) {
    let former = accounts::is_first_hand(&a.plugin).then(|| entry(&a.host()));
    let name = accounts::forge_vault_name(a);
    let found = secrets::read_entry(store, former.as_deref(), &name, &a.host());
    match found.map(|t| t.filter(|t| !t.is_empty())) {
        Ok(Some(token)) => {
            a.token = token;
            return;
        }
        Ok(_) => log::warn!("{}: no token in the system keychain", a.host()),
        Err(e) => log::warn!(
            "{}: the system keychain did not give its token: {e:#}",
            a.host()
        ),
    }
    a.unread = true;
}

/// At a start: the tokens the file holds move to the keychain, the others are read from it, and
/// the entries a « Déconnecter » left go. True when the file must be written again.
pub(crate) fn load(store: &dyn SecretStore, accounts: &mut Accounts) -> bool {
    let pending = std::mem::take(&mut accounts.forget_forges);
    for host in &pending {
        let reconnected = accounts
            .forges
            .iter()
            .any(|a| &a.host() == host && !a.via_cli);
        if !reconnected && !forget(store, host) {
            accounts.forget_forges.push(host.clone());
        }
    }
    let mut changed = accounts.forget_forges != pending;
    for a in &mut accounts.forges {
        if a.via_cli {
            continue;
        }
        if a.token.is_empty() {
            fill(store, a);
        } else {
            place(store, a);
            changed |= !a.in_file && !accounts.file_copy;
        }
    }
    changed || accounts.forges.iter().any(|a| a.in_file)
}

/// What the file keeps of the accounts: no token the keychain holds.
pub(crate) fn strip(accounts: &mut Accounts, file_copy: bool) {
    for a in &mut accounts.forges {
        if !a.in_file && !file_copy {
            a.token.clear();
        }
    }
}
