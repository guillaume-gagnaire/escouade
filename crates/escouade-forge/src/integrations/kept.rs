//! The plugins' secrets in a file readable by its owner alone, where the system has no keychain (Linux, WSL: issue
//! #37, ESC-499). Never taken in silence: a secret goes there only once the user chose it at the field
//! (`keep_in_file`), and the choice, made for this machine (the data folder, never a project's), is the file itself.
//! Réglages › Plugins shows it and takes it back (`give_up`).
//!
//! - **What it holds**: the secrets of plugins (`plugin:<id>:<key>`), in clear, in `plugin-secrets.json` of the data
//!   folder, written by `paths::write_private` (the state's atomic way, ESC-469: a temporary file opened with the owner's
//!   rights alone, Unix `0600`, from its creation, then put in its place). Never in the copies of the state
//!   (`backups::FILES` does not list it), nor in a journal: no `Debug`, no value in an error or a log line.
//! - **How it is read**: first, before the keychain, by every reader of the store (a plugin's `secrets/get`, a
//!   webhook's `{{secrets.<key>}}`): the protocol does not change. A keychain that comes later moves nothing by itself:
//!   « Déplacer vers le trousseau » (`move_to_keychain`) does it, secret by secret.
//! - **What goes elsewhere**: the accounts' secrets keep their own ways (`integrations.json` for a ticket source or a
//!   code host, the keychain alone for an API key or a voice service); `set` writes the file only for a name it holds.

use super::secrets::{keep_secret, SecretStore};
use crate::paths::{self, DataDir};
use anyhow::{Context, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

/// The file's version: one that a later Escouade writes with more is still read for its `secrets`.
const VERSION: u32 = 1;

/// `plugin-secrets.json`. Never `Debug`: it would print them.
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct SecretsFile {
    version: u32,
    secrets: BTreeMap<String, String>,
}

/// The system's keychain, and the file the user chose for the plugins' secrets it cannot keep.
pub struct Kept {
    keychain: Arc<dyn SecretStore>,
    path: PathBuf,
    /// The file's secrets, by name; `None` while it is not chosen (no file).
    file: Mutex<Option<BTreeMap<String, String>>>,
}

impl Kept {
    /// The keychain of `data`, and its file when the user chose it. One that cannot be read is set aside (its rights
    /// kept), said in the log without its content, and the choice is to make again.
    pub fn new(keychain: Arc<dyn SecretStore>, data: &DataDir) -> Self {
        let path = data.plugin_secrets_file();
        let file = match std::fs::read(&path) {
            Err(_) => None,
            Ok(bytes) => match serde_json::from_slice::<SecretsFile>(&bytes) {
                Ok(f) => Some(f.secrets),
                Err(e) => {
                    let aside = path.with_extension("unreadable.json");
                    log::error!(
                        "{} unreadable (line {}), set aside as {}",
                        path.display(),
                        e.line(),
                        aside.display()
                    );
                    let _ = std::fs::rename(&path, &aside);
                    None
                }
            },
        };
        Self {
            keychain,
            path,
            file: Mutex::new(file),
        }
    }

    /// The keychain alone, as the accounts and the voice services keep their secrets.
    pub fn keychain(&self) -> &Arc<dyn SecretStore> {
        &self.keychain
    }

    /// Where the file is (or would be).
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Whether the user chose the file on this machine.
    pub fn chosen(&self) -> bool {
        self.file.lock().is_some()
    }

    /// Whether the secret `name` is in the file.
    pub fn in_file(&self, name: &str) -> bool {
        self.file
            .lock()
            .as_ref()
            .is_some_and(|f| f.contains_key(name))
    }

    /// The names of the secrets the file holds.
    pub fn names(&self) -> Vec<String> {
        self.file
            .lock()
            .as_ref()
            .map(|f| f.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// The user's choice, at the field: `secret` kept in the file under `name`, the file made if it is not there
    /// (readable by its owner alone from its creation). Its entry in the keychain, if any, goes: one place for a secret.
    pub fn keep_in_file(&self, name: &str, secret: &str) -> Result<()> {
        let mut file = self.file.lock();
        let mut next = file.clone().unwrap_or_default();
        next.insert(name.to_string(), secret.to_string());
        self.write(&next)?;
        *file = Some(next);
        drop(file);
        if self.keychain.present() {
            let _ = self.keychain.delete(name);
        }
        Ok(())
    }

    /// « Déplacer vers le trousseau »: the secret `name` put in the keychain, read back to check, then out of the file.
    /// The file stays (the choice holds) even with nothing in it.
    pub fn move_to_keychain(&self, name: &str) -> Result<()> {
        let Some(secret) = self.file.lock().as_ref().and_then(|f| f.get(name).cloned()) else {
            return Ok(());
        };
        keep_secret(&*self.keychain, name, &secret)?;
        self.remove_from_file(name)
    }

    /// The choice taken back (Réglages › Plugins): the file goes, with the secrets it held (they were nowhere else).
    pub fn give_up(&self) -> Result<()> {
        let mut file = self.file.lock();
        match std::fs::remove_file(&self.path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e).with_context(|| self.path.display().to_string()),
        }
        *file = None;
        Ok(())
    }

    fn remove_from_file(&self, name: &str) -> Result<()> {
        let mut file = self.file.lock();
        let Some(mut next) = file.clone() else {
            return Ok(());
        };
        if next.remove(name).is_none() {
            return Ok(());
        }
        self.write(&next)?;
        *file = Some(next);
        Ok(())
    }

    fn write(&self, secrets: &BTreeMap<String, String>) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(&SecretsFile {
            version: VERSION,
            secrets: secrets.clone(),
        })?;
        paths::write_private(&self.path, &bytes).with_context(|| self.path.display().to_string())
    }
}

impl SecretStore for Kept {
    /// The file's first: the user put it there, and a keychain that came since moves nothing by itself.
    fn get(&self, name: &str) -> Result<Option<String>> {
        if let Some(s) = self.file.lock().as_ref().and_then(|f| f.get(name).cloned()) {
            return Ok(Some(s));
        }
        self.keychain.get(name)
    }

    /// In the file for a name it holds (kept again where the user put it), else in the keychain.
    fn set(&self, name: &str, secret: &str) -> Result<()> {
        if self.in_file(name) {
            return self.keep_in_file(name, secret);
        }
        self.keychain.set(name, secret)
    }

    /// Out of the file and out of the keychain; a keychain that refuses is fine for a secret that was in the file.
    fn delete(&self, name: &str) -> Result<()> {
        let was = self.in_file(name);
        self.remove_from_file(name)?;
        match self.keychain.delete(name) {
            Err(_) if was => Ok(()),
            done => done,
        }
    }

    fn present(&self) -> bool {
        self.keychain.present()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::secrets::MemoryStore;
    use crate::paths::test_dir;
    use std::sync::atomic::Ordering;

    const NAME: &str = "plugin:dev.test.vpn:password";

    fn kept(name: &str) -> (Arc<MemoryStore>, Kept, DataDir) {
        let d = DataDir::new(test_dir(name));
        let keychain = Arc::new(MemoryStore::default());
        let k = Kept::new(keychain.clone(), &d);
        (keychain, k, d)
    }

    #[test]
    fn without_a_keychain_nothing_goes_to_the_file_until_the_user_chooses_it() {
        let (keychain, k, d) = kept("kept-absent");
        keychain.absent.store(true, Ordering::SeqCst);
        assert!(!k.present());
        let e = keep_secret(&k, NAME, "hunter2-secret").unwrap_err();
        assert!(escouade_plugins::secrets::is_no_keychain(&e), "{e:#}");
        assert!(!k.chosen() && !d.plugin_secrets_file().exists());
        assert_eq!(k.get(NAME).unwrap(), None);
        // Chosen: kept, given back, the file made.
        k.keep_in_file(NAME, "hunter2-secret").unwrap();
        assert!(k.chosen() && k.in_file(NAME));
        assert_eq!(k.get(NAME).unwrap().as_deref(), Some("hunter2-secret"));
        assert_eq!(k.names(), [NAME]);
        // Kept again where it is.
        keep_secret(&k, NAME, "hunter3-secret").unwrap();
        assert_eq!(k.get(NAME).unwrap().as_deref(), Some("hunter3-secret"));
        // Read again by the next start.
        let again = Kept::new(keychain.clone(), &d);
        assert_eq!(again.get(NAME).unwrap().as_deref(), Some("hunter3-secret"));
        // Forgotten (« Oublier »): out of the file, which stays chosen.
        again.delete(NAME).unwrap();
        assert_eq!(again.get(NAME).unwrap(), None);
        assert!(again.chosen() && d.plugin_secrets_file().exists());
        // Taken back: the file goes.
        again.keep_in_file(NAME, "x-secret-x").unwrap();
        again.give_up().unwrap();
        assert!(!again.chosen() && !d.plugin_secrets_file().exists());
        assert_eq!(again.get(NAME).unwrap(), None);
    }

    #[cfg(unix)]
    #[test]
    fn the_file_is_readable_by_its_owner_alone_from_its_creation() {
        use std::os::unix::fs::PermissionsExt;
        let (_, k, d) = kept("kept-mode");
        // A temporary file left by a crash, readable by all: emptied and made private before anything is written.
        std::fs::write(d.plugin_secrets_file().with_extension("tmp"), "").unwrap();
        std::fs::set_permissions(
            d.plugin_secrets_file().with_extension("tmp"),
            std::fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        k.keep_in_file(NAME, "hunter2-secret").unwrap();
        let mode = std::fs::metadata(d.plugin_secrets_file())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn a_keychain_that_comes_later_moves_nothing_until_asked() {
        let (keychain, k, d) = kept("kept-later");
        keychain.absent.store(true, Ordering::SeqCst);
        k.keep_in_file(NAME, "hunter2-secret").unwrap();
        // The keychain is there now: the file's is still the one read, and nothing moved.
        keychain.absent.store(false, Ordering::SeqCst);
        keychain.set(NAME, "older-secret").unwrap();
        let k = Kept::new(keychain.clone(), &d);
        assert_eq!(k.get(NAME).unwrap().as_deref(), Some("hunter2-secret"));
        assert!(k.in_file(NAME));
        // « Déplacer vers le trousseau »: in the keychain, out of the file.
        k.move_to_keychain(NAME).unwrap();
        assert_eq!(keychain.entry(NAME).as_deref(), Some("hunter2-secret"));
        assert!(!k.in_file(NAME));
        assert!(!std::fs::read_to_string(d.plugin_secrets_file())
            .unwrap()
            .contains("hunter2-secret"));
        assert_eq!(k.get(NAME).unwrap().as_deref(), Some("hunter2-secret"));
        // A keychain that refuses: left in the file, said.
        k.keep_in_file(NAME, "hunter4-secret").unwrap();
        assert_eq!(keychain.entry(NAME), None, "one place for a secret");
        keychain.refuse.store(true, Ordering::SeqCst);
        assert!(k.move_to_keychain(NAME).is_err());
        assert_eq!(k.get(NAME).unwrap().as_deref(), Some("hunter4-secret"));
    }

    #[test]
    fn only_a_name_the_file_holds_is_written_there() {
        let (keychain, k, _) = kept("kept-others");
        k.keep_in_file(NAME, "hunter2-secret").unwrap();
        // An account's token, a voice key: the keychain, as before.
        k.set("forge:gitlab.com", "gl-token-1234").unwrap();
        assert_eq!(
            keychain.entry("forge:gitlab.com").as_deref(),
            Some("gl-token-1234")
        );
        assert_eq!(k.names(), [NAME]);
    }

    #[test]
    fn an_unreadable_file_is_set_aside_and_the_choice_is_to_make_again() {
        let d = DataDir::new(test_dir("kept-broken"));
        std::fs::write(
            d.plugin_secrets_file(),
            r#"{ "secrets": { "a": "hunter2-se"#,
        )
        .unwrap();
        let k = Kept::new(Arc::new(MemoryStore::default()), &d);
        assert!(!k.chosen());
        assert!(!d.plugin_secrets_file().exists());
        assert!(d
            .plugin_secrets_file()
            .with_extension("unreadable.json")
            .exists());
    }
}
