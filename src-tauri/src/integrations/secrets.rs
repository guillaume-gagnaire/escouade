//! The accounts' secrets (API keys and tokens) in the system's keychain: Windows' Credential
//! Manager, macOS' Keychain. `integrations.json` keeps the rest of the accounts, and the secrets
//! the keychain refuses stay there, readable by their owner only.

use super::{Account, Accounts};
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
/// former account of the service left goes.
pub(crate) fn place(store: &dyn SecretStore, service: Service, a: &mut Account) {
    let secrets = Secrets::of(a);
    a.in_file = false;
    if secrets.is_empty() {
        forget(store, service);
        return;
    }
    if let Err(e) = keep(store, service, &secrets) {
        log::warn!(
            "{}: secrets left in integrations.json, the system keychain refused them: {e:#}",
            service.label()
        );
        a.in_file = true;
    }
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

/// The service's secrets leave the store (its account goes, or has none now).
pub(crate) fn forget(store: &dyn SecretStore, service: Service) {
    if let Err(e) = store.delete(&entry(service)) {
        log::warn!(
            "{}: the system keychain did not forget its secrets: {e:#}",
            service.label()
        );
    }
}

/// The account's secrets, from the store when it holds them.
fn fill(store: &dyn SecretStore, service: Service, a: &mut Account) {
    match store.get(&entry(service)) {
        Ok(Some(text)) => match serde_json::from_str::<Secrets>(&text) {
            Ok(s) => {
                a.key = s.key;
                a.token = s.token;
            }
            Err(_) => log::warn!(
                "{}: its entry in the system keychain is unreadable",
                service.label()
            ),
        },
        // GitHub may have none: the CLI's token.
        Ok(None) if service == Service::Github => {}
        Ok(None) => log::warn!("{}: no secrets in the system keychain", service.label()),
        Err(e) => log::warn!(
            "{}: the system keychain did not give its secrets: {e:#}",
            service.label()
        ),
    }
}

/// The accounts of `data` (none when the file is missing or unreadable: logged, set aside),
/// with their secrets. Those the file holds (left by a version before 1.6, or refused by the
/// keychain last time) move into the store, and leave the file once the store gives them back.
pub(crate) fn load_accounts(data: &DataDir, store: &dyn SecretStore) -> Accounts {
    let path = data.integrations_file();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Accounts::default();
    };
    let mut accounts: Accounts = match serde_json::from_str(&text) {
        Ok(a) => a,
        Err(e) => {
            log::error!("invalid {}: {e}", path.display());
            let _ = std::fs::copy(&path, path.with_extension("broken.json"));
            return Accounts::default();
        }
    };
    let mut moved = false;
    for service in Service::ALL {
        let Some(a) = accounts.get_mut(service) else {
            continue;
        };
        if Secrets::of(a).is_empty() {
            fill(store, service, a);
        } else {
            place(store, service, a);
            moved |= !a.in_file;
        }
    }
    // Written again once secrets left it, and while some stay: readable by its owner only.
    let kept = accounts.views().iter().any(|v| v.in_file);
    if moved || kept {
        if let Err(e) = save_accounts(data, &accounts) {
            log::error!("{e:#}");
        }
    }
    accounts
}

/// Writes the accounts to `integrations.json`, without the secrets the store holds.
pub(crate) fn save_accounts(data: &DataDir, accounts: &Accounts) -> Result<()> {
    let mut saved = accounts.clone();
    for service in Service::ALL {
        if let Some(a) = saved.get_mut(service).filter(|a| !a.in_file) {
            a.key.clear();
            a.token.clear();
        }
    }
    let bytes = serde_json::to_vec_pretty(&saved)?;
    paths::write_private(&data.integrations_file(), &bytes)
        .context("enregistrement des comptes")?;
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
        let a = load_accounts(&d, &store);
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
        // The next start finds them in the keychain, and leaves the file as it is.
        assert_eq!(load_accounts(&d, &store), a);
        assert_eq!(file(&d), text);
    }

    #[test]
    fn a_file_without_secrets_is_left_as_it_is() {
        let d = DataDir::new(test_dir("secrets-none"));
        let text = r#"{ "github": { "label": "@ada · via gh", "user": "ada" } }"#;
        std::fs::write(d.integrations_file(), text).unwrap();
        let store = MemoryStore::default();
        let a = load_accounts(&d, &store);
        assert_eq!(a.github.as_ref().unwrap().label, "@ada · via gh");
        assert!(a.jira.is_none() && a.trello.is_none());
        assert_eq!(file(&d), text);
        assert_eq!(store.entry("github"), None);
        // Nor is one that cannot be read.
        let broken = r#"{ "jira": { "token": "jira-secret" "#;
        std::fs::write(d.integrations_file(), broken).unwrap();
        assert_eq!(load_accounts(&d, &store), Accounts::default());
        assert_eq!(file(&d), broken);
        assert_eq!(store.entry("jira"), None);
    }

    #[test]
    fn a_secret_the_keychain_refuses_stays_in_the_file_until_it_takes_it() {
        let d = older("secrets-refused");
        let store = MemoryStore::default();
        store.refuse.store(true, Ordering::SeqCst);
        let a = load_accounts(&d, &store);
        assert_eq!(a.jira.as_ref().unwrap().token, "jira-secret");
        let text = file(&d);
        for s in SECRETS {
            assert!(text.contains(s), "{text}");
        }
        // The window is told, for the accounts that have one.
        assert_eq!(in_file(&a), [true, true, false]);
        // Back: the next start moves them.
        store.refuse.store(false, Ordering::SeqCst);
        let a = load_accounts(&d, &store);
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
        let a = load_accounts(&d, &store);
        let text = file(&d);
        for s in SECRETS {
            assert!(text.contains(s), "{text}");
        }
        assert_eq!(in_file(&a), [true, true, false]);
        assert_eq!(a.trello.as_ref().unwrap().token, "trello-secret");
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
        load_accounts(&d, &store);
        assert!(file(&d).contains("jira-secret"));
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
