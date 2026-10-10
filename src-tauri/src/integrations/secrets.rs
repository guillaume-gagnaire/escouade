//! The accounts' secrets (API keys and tokens) in the system's keychain: Windows' Credential
//! Manager, macOS' Keychain. `integrations.json` keeps the rest of the accounts, and the secrets
//! the keychain refuses stay there, readable by their owner only.

use super::{Account, Accounts, VIA_GH};
use crate::model::Service;
use crate::paths::{self, DataDir};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Where the secrets are kept, one entry per integration: the system's keychain, or memory in
/// tests (they never touch the machine's).
pub trait SecretStore: Send + Sync {
    /// The secret kept under `name`, if there is one.
    fn get(&self, name: &str) -> Result<Option<String>>;
    /// Keeps `secret` under `name`, in place of the one there.
    fn set(&self, name: &str, secret: &str) -> Result<()>;
    /// Forgets the secret kept under `name` (none there is fine).
    fn delete(&self, name: &str) -> Result<()>;
}

/// The keychain's service of the app's entries, its identifier; a sandboxed run's (end-to-end
/// tests, demos) are apart, so that its accounts never take the place of the user's.
#[cfg_attr(not(any(windows, target_os = "macos")), allow(dead_code))]
pub(crate) fn keychain_service(sandboxed: bool) -> String {
    if sandboxed {
        format!("{}.sandbox", paths::IDENTIFIER)
    } else {
        paths::IDENTIFIER.to_string()
    }
}

/// The keychain's entry of an integration: "jira", "trello", "github".
pub(crate) fn entry(service: Service) -> String {
    service.label().to_lowercase()
}

/// What of an account is secret, as its entry holds it (JSON): Trello's key and token, the
/// token of the others. Never `Debug`: it would print them.
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct Secrets {
    key: String,
    token: String,
}

impl Secrets {
    fn of(a: &Account) -> Self {
        Self {
            key: a.key.clone(),
            token: a.token.clone(),
        }
    }

    fn is_empty(&self) -> bool {
        self.key.is_empty() && self.token.is_empty()
    }
}

/// The system's keychain: Windows' Credential Manager, macOS' Keychain (the user's), an entry per
/// integration under `service`.
#[cfg(all(not(test), any(windows, target_os = "macos")))]
struct Keychain {
    service: String,
}

#[cfg(all(not(test), any(windows, target_os = "macos")))]
impl Keychain {
    /// The entry `name`, in the platform's own store: never the in-memory one `keyring` falls
    /// back to without it, which would lose the secrets at the next start.
    fn entry(&self, name: &str) -> keyring::Result<keyring::Entry> {
        #[cfg(windows)]
        let credential =
            keyring::windows::WinCredential::new_with_target(None, &self.service, name)?;
        #[cfg(target_os = "macos")]
        let credential = keyring::macos::MacCredential::new_with_target(None, &self.service, name)?;
        Ok(keyring::Entry::new_with_credential(Box::new(credential)))
    }
}

// Its errors say what failed (the platform's code, the attribute too long), never the secret.
#[cfg(all(not(test), any(windows, target_os = "macos")))]
impl SecretStore for Keychain {
    fn get(&self, name: &str) -> Result<Option<String>> {
        match self.entry(name)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn set(&self, name: &str, secret: &str) -> Result<()> {
        Ok(self.entry(name)?.set_password(secret)?)
    }

    fn delete(&self, name: &str) -> Result<()> {
        match self.entry(name)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

/// No keychain the app knows of (Linux): the secrets stay in the file.
#[cfg(all(not(test), not(any(windows, target_os = "macos"))))]
struct NoKeychain;

#[cfg(all(not(test), not(any(windows, target_os = "macos"))))]
impl SecretStore for NoKeychain {
    fn get(&self, _name: &str) -> Result<Option<String>> {
        Ok(None)
    }

    fn set(&self, _name: &str, _secret: &str) -> Result<()> {
        bail!("no system keychain here")
    }

    fn delete(&self, _name: &str) -> Result<()> {
        Ok(())
    }
}

/// The store of the accounts of `data`: the system's keychain (the same for every folder, a
/// sandboxed run's service apart).
#[cfg(all(not(test), any(windows, target_os = "macos")))]
pub(crate) fn store_for(_data: &DataDir) -> Arc<dyn SecretStore> {
    Arc::new(Keychain {
        service: keychain_service(paths::sandbox_dir().is_some()),
    })
}

#[cfg(all(not(test), not(any(windows, target_os = "macos"))))]
pub(crate) fn store_for(_data: &DataDir) -> Arc<dyn SecretStore> {
    Arc::new(NoKeychain)
}

/// The secrets in memory (tests).
#[cfg(test)]
#[derive(Default)]
pub(crate) struct MemoryStore {
    entries: parking_lot::Mutex<std::collections::BTreeMap<String, String>>,
    /// Every call fails, as with a keychain out of reach.
    pub refuse: std::sync::atomic::AtomicBool,
    /// Says it kept a secret, and keeps nothing.
    pub lose: std::sync::atomic::AtomicBool,
}

#[cfg(test)]
impl MemoryStore {
    /// What it holds under `name`.
    pub fn entry(&self, name: &str) -> Option<String> {
        self.entries.lock().get(name).cloned()
    }

    fn reached(&self) -> Result<()> {
        if self.refuse.load(std::sync::atomic::Ordering::SeqCst) {
            bail!("keychain out of reach");
        }
        Ok(())
    }
}

#[cfg(test)]
impl SecretStore for MemoryStore {
    fn get(&self, name: &str) -> Result<Option<String>> {
        self.reached()?;
        Ok(self.entry(name))
    }

    fn set(&self, name: &str, secret: &str) -> Result<()> {
        self.reached()?;
        if !self.lose.load(std::sync::atomic::Ordering::SeqCst) {
            self.entries.lock().insert(name.into(), secret.into());
        }
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<()> {
        self.reached()?;
        self.entries.lock().remove(name);
        Ok(())
    }
}

/// The memory store of the data folder `data` (tests): one per folder, so that a core loaded again
/// on the same folder finds what the one before kept, as it would in the keychain.
#[cfg(test)]
pub(crate) fn memory_of(data: &DataDir) -> Arc<MemoryStore> {
    static STORES: std::sync::Mutex<
        std::collections::BTreeMap<std::path::PathBuf, Arc<MemoryStore>>,
    > = std::sync::Mutex::new(std::collections::BTreeMap::new());
    STORES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .entry(data.integrations_file())
        .or_default()
        .clone()
}

/// The store of the accounts of `data`: its memory store.
#[cfg(test)]
pub(crate) fn store_for(data: &DataDir) -> Arc<dyn SecretStore> {
    memory_of(data)
}

/// Where the account's secrets go: into the store, read back to check, else they stay in the file
/// (`Account::in_file`). An account without any (GitHub through `gh`) leaves no entry: the one a
/// former account of the service left goes; false when it stays (to forget at the next start,
/// `Accounts::to_forget`).
pub(crate) fn place(store: &dyn SecretStore, service: Service, a: &mut Account) -> bool {
    let secrets = Secrets::of(a);
    a.in_file = false;
    if secrets.is_empty() {
        return forget(store, service);
    }
    if let Err(e) = keep(store, service, &secrets) {
        log::warn!(
            "{}: secrets left in integrations.json, the system keychain refused them: {e:#}",
            service.label()
        );
        a.in_file = true;
    }
    true
}

/// Puts `secrets` into the service's entry, then reads them back: Ok once the store gives them as
/// they were given.
fn keep(store: &dyn SecretStore, service: Service, secrets: &Secrets) -> Result<()> {
    let name = entry(service);
    let text = serde_json::to_string(secrets)?;
    store.set(&name, &text)?;
    if store.get(&name)?.as_deref() != Some(text.as_str()) {
        bail!("it does not give them back as they were given");
    }
    Ok(())
}

/// The service's secrets leave the store (its account goes, or has none now); false when they
/// could not (logged).
pub(crate) fn forget(store: &dyn SecretStore, service: Service) -> bool {
    if let Err(e) = store.delete(&entry(service)) {
        log::warn!(
            "{}: the system keychain did not forget its secrets: {e:#}",
            service.label()
        );
        return false;
    }
    true
}

/// A development build on the user's own data folder (not sandboxed) leaves the file a copy of
/// the secrets the keychain holds: the installed Escouade, maybe older, still reads them there (a
/// development build never moves the user's data). Never in tests (throwaway folders).
pub(crate) fn file_keeps_copy() -> bool {
    !cfg!(test) && cfg!(debug_assertions) && paths::sandbox_dir().is_none()
}

/// A GitHub account that uses the CLI's token: it has none of its own.
pub(crate) fn through_gh(service: Service, a: &Account) -> bool {
    service == Service::Github && a.label.ends_with(VIA_GH)
}

/// The account's secrets, from the store. Missing (out of reach, refused, the entry gone or
/// unreadable), the account says so (`Account::unread`): a call without them would be refused,
/// or for GitHub go out with the CLI's token, as whoever it is logged in as.
fn fill(store: &dyn SecretStore, service: Service, a: &mut Account) {
    match store.get(&entry(service)) {
        Ok(Some(text)) => match serde_json::from_str::<Secrets>(&text) {
            Ok(s) => {
                a.key = s.key;
                a.token = s.token;
                return;
            }
            Err(_) => log::warn!(
                "{}: its entry in the system keychain is unreadable",
                service.label()
            ),
        },
        Ok(None) => log::warn!("{}: no secrets in the system keychain", service.label()),
        Err(e) => log::warn!(
            "{}: the system keychain did not give its secrets: {e:#}",
            service.label()
        ),
    }
    a.unread = true;
}

/// The accounts of `data` (none when the file is missing or unreadable: logged, set aside),
/// with their secrets. Those the file holds (left by a version before 1.6, or refused by the
/// keychain last time) move into the store, and leave the file once the store gives them back,
/// unless it keeps a copy (`file_copy`, `file_keeps_copy`). The entries a « Déconnecter » could
/// not delete (`Accounts::to_forget`) go; no other, even of a service no account lists: a file
/// written again after a broken one, or by another version, would cost the others their tokens.
pub(crate) fn load_accounts(data: &DataDir, store: &dyn SecretStore, file_copy: bool) -> Accounts {
    let path = data.integrations_file();
    let none = || Accounts {
        file_copy,
        ..Default::default()
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return none();
    };
    let mut accounts: Accounts = match serde_json::from_str(&text) {
        Ok(a) => a,
        Err(e) => {
            log::error!("invalid {}: {e}", path.display());
            let _ = std::fs::copy(&path, path.with_extension("broken.json"));
            return none();
        }
    };
    accounts.file_copy = file_copy;
    let pending = std::mem::take(&mut accounts.to_forget);
    for &service in &pending {
        let done = match accounts.get(service) {
            // Connected again since with a token of its own: the entry is that account's.
            Some(a) if !through_gh(service, a) => true,
            _ => forget(store, service),
        };
        if !done {
            accounts.to_forget.push(service);
        }
    }
    let forgot = accounts.to_forget != pending;
    let mut moved = false;
    for service in Service::ALL {
        match accounts.get_mut(service) {
            None => {}
            Some(a) if !Secrets::of(a).is_empty() => {
                place(store, service, a);
                moved |= !a.in_file;
            }
            // The CLI's token: an entry a former account left is never read.
            Some(a) if through_gh(service, a) => {}
            Some(a) => fill(store, service, a),
        }
    }
    // Written again once secrets or entries to forget left it, and while secrets stay: readable
    // by its owner only.
    let kept = accounts.views().iter().any(|v| v.in_file);
    if (moved && !file_copy) || forgot || kept {
        if let Err(e) = save_accounts(data, &accounts) {
            log::error!("{e:#}");
        }
    }
    accounts
}

/// Writes the accounts to `integrations.json`, without the secrets the store holds (unless the
/// file keeps a copy of them).
pub(crate) fn save_accounts(data: &DataDir, accounts: &Accounts) -> Result<()> {
    let mut saved = accounts.clone();
    for service in Service::ALL {
        if let Some(a) = saved
            .get_mut(service)
            .filter(|a| !a.in_file && !accounts.file_copy)
        {
            a.key.clear();
            a.token.clear();
        }
    }
    let bytes = serde_json::to_vec_pretty(&saved)?;
    paths::write_private(&data.integrations_file(), &bytes)
        .with_context(|| tr!("enregistrement des comptes", "saving the accounts"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::test_dir;
    use std::sync::atomic::Ordering;

    #[test]
    fn the_memory_store_keeps_gives_back_and_forgets_a_secret() {
        let s = MemoryStore::default();
        assert_eq!(s.get("jira").unwrap(), None);
        s.set("jira", "t1").unwrap();
        s.set("jira", "t2").unwrap();
        assert_eq!(s.get("jira").unwrap().as_deref(), Some("t2"));
        assert_eq!(s.get("trello").unwrap(), None);
        s.delete("jira").unwrap();
        assert_eq!(s.get("jira").unwrap(), None);
        // None there: nothing to do.
        s.delete("jira").unwrap();
        s.refuse.store(true, Ordering::SeqCst);
        assert!(s.set("jira", "t3").is_err());
        assert!(s.get("jira").is_err() && s.delete("jira").is_err());
        assert_eq!(s.entry("jira"), None);
    }

    #[test]
    fn the_apps_entries_are_under_its_identifier_and_a_sandboxs_apart() {
        assert_eq!(keychain_service(false), "dev.gagnaire.escouade");
        assert_eq!(keychain_service(true), "dev.gagnaire.escouade.sandbox");
        assert_eq!(Service::ALL.map(entry), ["jira", "trello", "github"]);
    }

    /// `integrations.json` as a version before 1.6 wrote it: the secrets with the rest.
    const OLD_FILE: &str = r#"{
  "jira": {
    "site": "https://atlas.atlassian.net",
    "email": "ada@atlas.dev",
    "key": "",
    "token": "jira-secret",
    "label": "ada@atlas.dev · atlas.atlassian.net",
    "user": "557058"
  },
  "trello": {
    "site": "",
    "email": "",
    "key": "trello-key",
    "token": "trello-secret",
    "label": "@ada",
    "user": "m1"
  },
  "github": {
    "site": "",
    "email": "",
    "key": "",
    "token": "",
    "label": "@ada · via gh",
    "user": "ada"
  }
}"#;

    const SECRETS: [&str; 3] = ["jira-secret", "trello-key", "trello-secret"];

    /// A data folder whose `integrations.json` is `OLD_FILE`.
    fn older(name: &str) -> DataDir {
        let d = DataDir::new(test_dir(name));
        std::fs::write(d.integrations_file(), OLD_FILE).unwrap();
        d
    }

    fn file(d: &DataDir) -> String {
        std::fs::read_to_string(d.integrations_file()).unwrap()
    }

    fn in_file(a: &Accounts) -> Vec<bool> {
        a.views().iter().map(|v| v.in_file).collect()
    }

    #[test]
    fn the_secrets_of_an_older_file_move_to_the_keychain_and_leave_the_file() {
        let d = older("secrets-move");
        let store = MemoryStore::default();
        let a = load_accounts(&d, &store, false);
        // In use as before.
        let jira = a.jira.as_ref().unwrap();
        assert_eq!(
            (jira.token.as_str(), jira.email.as_str()),
            ("jira-secret", "ada@atlas.dev")
        );
        let trello = a.trello.as_ref().unwrap();
        assert_eq!(
            (trello.key.as_str(), trello.token.as_str()),
            ("trello-key", "trello-secret")
        );
        // One entry per integration, none for GitHub through gh.
        assert_eq!(
            store.entry("jira").as_deref(),
            Some(r#"{"key":"","token":"jira-secret"}"#)
        );
        assert_eq!(
            store.entry("trello").as_deref(),
            Some(r#"{"key":"trello-key","token":"trello-secret"}"#)
        );
        assert_eq!(store.entry("github"), None);
        // Gone from the file, which keeps the rest.
        let text = file(&d);
        for s in SECRETS {
            assert!(!text.contains(s), "{text}");
        }
        let saved: Accounts = serde_json::from_str(&text).unwrap();
        assert_eq!(
            saved.jira.as_ref().unwrap().label,
            "ada@atlas.dev · atlas.atlassian.net"
        );
        assert_eq!(saved.trello.as_ref().unwrap().user, "m1");
        assert_eq!(saved.github.as_ref().unwrap().label, "@ada · via gh");
        assert_eq!(in_file(&a), [false, false, false]);
        // No entry to delete in a file of before.
        assert!(a.to_forget.is_empty() && saved.to_forget.is_empty());
        // The next start finds them in the keychain, and leaves the file as it is.
        assert_eq!(load_accounts(&d, &store, false), a);
        assert_eq!(file(&d), text);
    }

    #[test]
    fn a_file_without_secrets_is_left_as_it_is() {
        let d = DataDir::new(test_dir("secrets-none"));
        let text = r#"{ "github": { "label": "@ada · via gh", "user": "ada" } }"#;
        std::fs::write(d.integrations_file(), text).unwrap();
        let store = MemoryStore::default();
        let a = load_accounts(&d, &store, false);
        assert_eq!(a.github.as_ref().unwrap().label, "@ada · via gh");
        assert!(a.jira.is_none() && a.trello.is_none());
        assert_eq!(file(&d), text);
        assert_eq!(store.entry("github"), None);
        // Nor is one that cannot be read.
        let broken = r#"{ "jira": { "token": "jira-secret" "#;
        std::fs::write(d.integrations_file(), broken).unwrap();
        assert_eq!(load_accounts(&d, &store, false), Accounts::default());
        assert_eq!(file(&d), broken);
        assert_eq!(store.entry("jira"), None);
    }

    #[test]
    fn a_secret_the_keychain_refuses_stays_in_the_file_until_it_takes_it() {
        let d = older("secrets-refused");
        let store = MemoryStore::default();
        store.refuse.store(true, Ordering::SeqCst);
        let a = load_accounts(&d, &store, false);
        assert_eq!(a.jira.as_ref().unwrap().token, "jira-secret");
        let text = file(&d);
        for s in SECRETS {
            assert!(text.contains(s), "{text}");
        }
        // The window is told, for the accounts that have one.
        assert_eq!(in_file(&a), [true, true, false]);
        // Back: the next start moves them.
        store.refuse.store(false, Ordering::SeqCst);
        let a = load_accounts(&d, &store, false);
        assert_eq!(in_file(&a), [false, false, false]);
        assert_eq!(a.jira.as_ref().unwrap().token, "jira-secret");
        let text = file(&d);
        for s in SECRETS {
            assert!(!text.contains(s), "{text}");
        }
    }

    #[test]
    fn a_secret_the_keychain_does_not_give_back_stays_in_the_file() {
        let d = older("secrets-lost");
        let store = MemoryStore::default();
        store.lose.store(true, Ordering::SeqCst);
        let a = load_accounts(&d, &store, false);
        let text = file(&d);
        for s in SECRETS {
            assert!(text.contains(s), "{text}");
        }
        assert_eq!(in_file(&a), [true, true, false]);
        assert_eq!(a.trello.as_ref().unwrap().token, "trello-secret");
    }

    fn unread(a: &Accounts) -> Vec<bool> {
        a.views().iter().map(|v| v.unread).collect()
    }

    #[test]
    fn secrets_the_keychain_does_not_give_at_a_start_are_said_missing_and_left_where_they_are() {
        let d = older("secrets-unread");
        let store = MemoryStore::default();
        load_accounts(&d, &store, false);
        let text = file(&d);
        let entries = || (store.entry("jira"), store.entry("trello"));
        let kept = entries();
        // Out of reach at the next start: GitHub through gh has none to read.
        store.refuse.store(true, Ordering::SeqCst);
        let a = load_accounts(&d, &store, false);
        assert_eq!(unread(&a), [true, true, false]);
        assert_eq!(a.jira.as_ref().unwrap().token, "");
        assert_eq!(in_file(&a), [false, false, false]);
        // Nothing lost: the file and the entries as they were.
        assert_eq!(file(&d), text);
        assert_eq!(entries(), kept);
        // Back, without the Jira entry (removed by hand): only Jira's are missing.
        store.refuse.store(false, Ordering::SeqCst);
        store.delete("jira").unwrap();
        let a = load_accounts(&d, &store, false);
        assert_eq!(unread(&a), [true, false, false]);
        assert_eq!(a.trello.as_ref().unwrap().token, "trello-secret");
        // A GitHub token of its own that is missing is never the CLI's.
        let mine = r#"{ "github": { "label": "@work", "user": "work" } }"#;
        std::fs::write(d.integrations_file(), mine).unwrap();
        assert_eq!(
            unread(&load_accounts(&d, &store, false)),
            [false, false, true]
        );
        store.set("github", "not json").unwrap();
        assert_eq!(
            unread(&load_accounts(&d, &store, false)),
            [false, false, true]
        );
        assert_eq!(file(&d), mine);
    }

    #[test]
    fn an_entry_goes_at_a_start_only_when_a_disconnect_left_it() {
        let d = DataDir::new(test_dir("secrets-leftover"));
        let store = MemoryStore::default();
        let jira = r#"{"key":"","token":"jira-secret"}"#;
        let trello = r#"{"key":"trello-key","token":"trello-secret"}"#;
        store.set("jira", jira).unwrap();
        store.set("trello", trello).unwrap();
        store
            .set("github", r#"{"key":"","token":"ghp-former"}"#)
            .unwrap();
        // No account lists them (a file written again after a broken one, another version's):
        // they may be the only copy of their tokens, and stay.
        let text = r#"{ "github": { "label": "@ada · via gh", "user": "ada" } }"#;
        std::fs::write(d.integrations_file(), text).unwrap();
        let a = load_accounts(&d, &store, false);
        assert_eq!(store.entry("jira").as_deref(), Some(jira));
        assert_eq!(store.entry("trello").as_deref(), Some(trello));
        // Through gh, a former token is never read.
        assert!(store.entry("github").is_some());
        let github = a.github.as_ref().unwrap();
        assert_eq!((github.token.as_str(), github.unread), ("", false));
        assert_eq!(file(&d), text);
        // Those a « Déconnecter » left go; one whose service has an account again stays.
        let text =
            r#"{ "trello": { "label": "@ada", "user": "m1" }, "toForget": ["jira", "trello"] }"#;
        std::fs::write(d.integrations_file(), text).unwrap();
        let a = load_accounts(&d, &store, false);
        assert_eq!(store.entry("jira"), None);
        assert_eq!(store.entry("trello").as_deref(), Some(trello));
        assert_eq!(a.trello.as_ref().unwrap().token, "trello-secret");
        assert!(store.entry("github").is_some());
        let saved: Accounts = serde_json::from_str(&file(&d)).unwrap();
        assert!(a.to_forget.is_empty() && saved.to_forget.is_empty());
        assert!(!file(&d).contains("trello-secret"));
        // Still refused: kept for the start after.
        store.set("jira", jira).unwrap();
        std::fs::write(d.integrations_file(), r#"{ "toForget": ["jira"] }"#).unwrap();
        store.refuse.store(true, Ordering::SeqCst);
        let a = load_accounts(&d, &store, false);
        assert_eq!(a.to_forget, [Service::Jira]);
        let saved: Accounts = serde_json::from_str(&file(&d)).unwrap();
        assert_eq!(saved.to_forget, [Service::Jira]);
        store.refuse.store(false, Ordering::SeqCst);
        assert!(load_accounts(&d, &store, false).to_forget.is_empty());
        assert_eq!(store.entry("jira"), None);
    }

    #[test]
    fn a_service_a_later_version_left_to_forget_does_not_cost_the_accounts_their_place() {
        let d = DataDir::new(test_dir("secrets-later-service"));
        let store = MemoryStore::default();
        let trello = r#"{"key":"trello-key","token":"trello-secret"}"#;
        store
            .set("jira", r#"{"key":"","token":"jira-secret"}"#)
            .unwrap();
        store.set("trello", trello).unwrap();
        // Written by a later version (then rolled back): a service this one does not know.
        let text =
            r#"{ "trello": { "label": "@ada", "user": "m1" }, "toForget": ["linear", "jira"] }"#;
        std::fs::write(d.integrations_file(), text).unwrap();
        let a = load_accounts(&d, &store, false);
        assert_eq!(a.trello.as_ref().unwrap().token, "trello-secret");
        assert!(!d.integrations_file().with_extension("broken.json").exists());
        // The ones it knows go as before; the unknown one is left out.
        assert_eq!(store.entry("jira"), None);
        assert_eq!(store.entry("trello").as_deref(), Some(trello));
        assert!(a.to_forget.is_empty());
        let saved: Accounts = serde_json::from_str(&file(&d)).unwrap();
        assert_eq!(saved.trello.as_ref().unwrap().label, "@ada");
    }

    #[test]
    fn a_development_build_leaves_the_file_a_copy_of_the_secrets() {
        let d = older("secrets-dev-copy");
        let store = MemoryStore::default();
        let mut a = load_accounts(&d, &store, true);
        // In the keychain, and still where an installed Escouade reads them.
        assert_eq!(
            store.entry("trello").as_deref(),
            Some(r#"{"key":"trello-key","token":"trello-secret"}"#)
        );
        assert!(store.entry("jira").is_some());
        assert_eq!(file(&d), OLD_FILE);
        assert_eq!(in_file(&a), [false, false, false]);
        // Saved again (an account connected): they stay.
        a.set(Service::Github, None);
        save_accounts(&d, &a).unwrap();
        let text = file(&d);
        for s in SECRETS {
            assert!(text.contains(s), "{text}");
        }
        // A release build moves them.
        load_accounts(&d, &store, false);
        let text = file(&d);
        for s in SECRETS {
            assert!(!text.contains(s), "{text}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_file_that_keeps_a_secret_is_readable_by_its_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let d = older("secrets-mode");
        let path = d.integrations_file();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        let store = MemoryStore::default();
        store.refuse.store(true, Ordering::SeqCst);
        load_accounts(&d, &store, false);
        assert!(file(&d).contains("jira-secret"));
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
