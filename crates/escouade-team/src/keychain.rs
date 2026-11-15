//! The device's keys and its checkpoint, in the system's keychain (`docs/TEAM-SERVER.md` §3.2).
//!
//! - [`Keychain`] is the trait the core gives, with the app's `SecretStore` (`src-tauri/src/integrations/secrets.rs`:
//!   Windows' Credential Manager, macOS' Keychain): the same three calls, a secret as text under a name. The crate
//!   gives [`MemoryKeychain`] for the tests.
//! - [`Vault`] keeps each secret the way the app keeps its own (`integrations::secrets::keep_secret`): put, then read
//!   back to check. When the keychain refuses (Linux without Secret Service, a keychain out of reach), the secret goes
//!   to a file readable by its owner alone, beside the app's data, and the vault says so ([`KeyPlace::File`]): the
//!   interface tells the user, as it does for the accounts. A file left there is moved into the keychain as soon as
//!   it takes it.
//! - The keys never leave: [`DeviceKeys`] has no accessor to its secret halves outside the crate, no `Debug` that
//!   prints them, and is wiped from memory when dropped.
//! - The **checkpoint** (the highest epoch seen and its head) has its own entry, apart from the state on the disk:
//!   an old backup of the state put back does not bring an old checkpoint with it (point 8 of the audit).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use escouade_team_crypto::Checkpoint;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::Error;

/// Where secrets are kept, one entry per name: the system's keychain (the core's `SecretStore`), or memory in tests.
pub trait Keychain: Send + Sync {
    /// The secret kept under `name`, if there is one.
    fn get(&self, name: &str) -> Result<Option<String>, String>;
    /// Keeps `secret` under `name`, in place of the one there.
    fn set(&self, name: &str, secret: &str) -> Result<(), String>;
    /// Forgets the secret kept under `name` (none there is fine).
    fn delete(&self, name: &str) -> Result<(), String>;
}

/// The secrets in memory: for the tests, and to play a keychain that refuses (Linux without Secret Service).
#[derive(Default)]
pub struct MemoryKeychain {
    entries: std::sync::Mutex<std::collections::BTreeMap<String, String>>,
    /// Every call fails, as with no keychain.
    pub refuse: std::sync::atomic::AtomicBool,
}

impl MemoryKeychain {
    /// What it holds under `name`.
    pub fn entry(&self, name: &str) -> Option<String> {
        self.lock().get(name).cloned()
    }

    /// The names it holds.
    pub fn names(&self) -> Vec<String> {
        self.lock().keys().cloned().collect()
    }

    /// Puts `secret` under `name` without the vault: a test that plays an old entry.
    pub fn force(&self, name: &str, secret: &str) {
        self.lock().insert(name.into(), secret.into());
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, std::collections::BTreeMap<String, String>> {
        self.entries.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn reached(&self) -> Result<(), String> {
        if self.refuse.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("no keychain here".into());
        }
        Ok(())
    }
}

impl Keychain for MemoryKeychain {
    fn get(&self, name: &str) -> Result<Option<String>, String> {
        self.reached()?;
        Ok(self.entry(name))
    }

    fn set(&self, name: &str, secret: &str) -> Result<(), String> {
        self.reached()?;
        self.lock().insert(name.into(), secret.into());
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<(), String> {
        self.reached()?;
        self.lock().remove(name);
        Ok(())
    }
}

/// Where a secret ended up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyPlace {
    /// In the system's keychain.
    Keychain,
    /// In this file, readable by its owner alone: the keychain refused it. The interface says so.
    File(PathBuf),
}

/// The team's secrets, in the keychain or, when it refuses, in files of `fallback` (the app's data folder).
#[derive(Clone)]
pub struct Vault {
    keychain: Arc<dyn Keychain>,
    fallback: PathBuf,
}

impl Vault {
    pub fn new(keychain: Arc<dyn Keychain>, fallback: impl Into<PathBuf>) -> Self {
        Self {
            keychain,
            fallback: fallback.into(),
        }
    }

    /// The name of a team's entry: `escouade-team.<team>.<what>`.
    pub fn name(team: &[u8; 16], what: &str) -> String {
        format!("escouade-team.{}.{what}", hex::encode(team))
    }

    fn file(&self, name: &str) -> PathBuf {
        self.fallback.join(format!("{name}.secret"))
    }

    /// Keeps `secret` under `name`: in the keychain, read back to check; else in its file, which it says.
    pub fn put(&self, name: &str, secret: &str) -> Result<KeyPlace, Error> {
        match self.keep(name, secret) {
            Ok(()) => {
                let file = self.file(name);
                if file.exists() {
                    let _ = std::fs::remove_file(&file);
                }
                Ok(KeyPlace::Keychain)
            }
            Err(e) => {
                let file = self.file(name);
                log::warn!(
                    "team: {name} kept in a file readable by its owner only, the system keychain refused it: {e}"
                );
                if let Some(dir) = file.parent() {
                    std::fs::create_dir_all(dir).map_err(|e| Error::Storage(e.to_string()))?;
                }
                write_private(&file, secret.as_bytes())
                    .map_err(|e| Error::Storage(format!("{}: {e}", file.display())))?;
                Ok(KeyPlace::File(file))
            }
        }
    }

    fn keep(&self, name: &str, secret: &str) -> Result<(), String> {
        self.keychain.set(name, secret)?;
        if self.keychain.get(name)?.as_deref() != Some(secret) {
            return Err("it does not give it back as it was given".into());
        }
        Ok(())
    }

    /// The secret under `name`, and where it is. A file left by a keychain that refused is newer than any entry: it
    /// is read first, and moved into the keychain when it takes it.
    pub fn get(&self, name: &str) -> Result<Option<(Zeroizing<String>, KeyPlace)>, Error> {
        let file = self.file(name);
        if let Ok(bytes) = std::fs::read(&file) {
            let text = Zeroizing::new(
                String::from_utf8(bytes).map_err(|_| Error::Storage("a secret file".into()))?,
            );
            return match self.keep(name, &text) {
                Ok(()) => {
                    let _ = std::fs::remove_file(&file);
                    Ok(Some((text, KeyPlace::Keychain)))
                }
                Err(_) => Ok(Some((text, KeyPlace::File(file)))),
            };
        }
        match self.keychain.get(name) {
            Ok(Some(text)) => Ok(Some((Zeroizing::new(text), KeyPlace::Keychain))),
            Ok(None) => Ok(None),
            Err(e) => Err(Error::Keychain(e)),
        }
    }

    pub fn delete(&self, name: &str) -> Result<(), Error> {
        let _ = std::fs::remove_file(self.file(name));
        self.keychain.delete(name).map_err(Error::Keychain)
    }

    pub(crate) fn put_keys(&self, keys: &DeviceKeys) -> Result<KeyPlace, Error> {
        let text = Zeroizing::new(serde_json::to_string(keys).map_err(encoding)?);
        self.put(&Self::name(&keys.team, "keys"), &text)
    }

    pub(crate) fn keys(&self, team: &[u8; 16]) -> Result<Option<(DeviceKeys, KeyPlace)>, Error> {
        match self.get(&Self::name(team, "keys"))? {
            Some((text, place)) => Ok(Some((
                serde_json::from_str(&text).map_err(encoding)?,
                place,
            ))),
            None => Ok(None),
        }
    }

    pub(crate) fn put_checkpoint(
        &self,
        team: &[u8; 16],
        checkpoint: &Checkpoint,
    ) -> Result<KeyPlace, Error> {
        let text = serde_json::to_string(checkpoint).map_err(encoding)?;
        self.put(&Self::name(team, "checkpoint"), &text)
    }

    pub(crate) fn checkpoint(&self, team: &[u8; 16]) -> Result<Option<Checkpoint>, Error> {
        match self.get(&Self::name(team, "checkpoint"))? {
            Some((text, _)) => Ok(Some(serde_json::from_str(&text).map_err(encoding)?)),
            None => Ok(None),
        }
    }

    /// Forgets a team's secrets (the device left it).
    pub fn forget_team(&self, team: &[u8; 16]) -> Result<(), Error> {
        self.delete(&Self::name(team, "keys"))?;
        self.delete(&Self::name(team, "checkpoint"))
    }
}

fn encoding(e: serde_json::Error) -> Error {
    Error::Encoding(e.to_string())
}

/// A file only its owner reads (0600 on Unix; under the user's profile on Windows, as the app's), written whole
/// then moved in place.
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let tmp = path.with_extension("tmp");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut f = options.open(&tmp)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        f.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    f.write_all(bytes)?;
    f.sync_all()?;
    drop(f);
    std::fs::rename(&tmp, path)
}

/// A device's keys in a team, made on the device and never exported: its server key (Ed25519, it signs the
/// server's challenges), its MLS leaf key (Ed25519, it signs its commits and objects), and the key its state is
/// encrypted under on the disk. Its ids: the member, the device (the server's id, the same in the MLS identity), the
/// team.
#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub(crate) struct DeviceKeys {
    #[serde(with = "hex::serde")]
    pub team: [u8; 16],
    #[serde(with = "hex::serde")]
    pub member: [u8; 16],
    #[serde(with = "hex::serde")]
    pub device: [u8; 16],
    #[serde(with = "hex::serde")]
    pub auth: [u8; 32],
    /// The seed of the MLS leaf key (Ed25519, as `openmls_basic_credential` keeps it).
    #[serde(with = "hex::serde")]
    pub mls: [u8; 32],
    #[serde(with = "hex::serde")]
    pub state_key: [u8; 32],
}

impl std::fmt::Debug for DeviceKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceKeys")
            .field("team", &hex::encode(&self.team[..4]))
            .field("device", &hex::encode(&self.device[..4]))
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn a_secret_goes_to_the_keychain_and_comes_back() {
        let dir = tempfile::tempdir().unwrap();
        let keychain = Arc::new(MemoryKeychain::default());
        let vault = Vault::new(keychain.clone(), dir.path());
        assert_eq!(vault.put("a", "s1").unwrap(), KeyPlace::Keychain);
        let (text, place) = vault.get("a").unwrap().unwrap();
        assert_eq!((text.as_str(), place), ("s1", KeyPlace::Keychain));
        assert_eq!(keychain.entry("a").as_deref(), Some("s1"));
        assert!(std::fs::read_dir(dir.path()).unwrap().next().is_none());
        vault.delete("a").unwrap();
        assert!(vault.get("a").unwrap().is_none());
    }

    #[test]
    fn without_a_keychain_the_secret_goes_to_a_private_file_and_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let keychain = Arc::new(MemoryKeychain::default());
        keychain.refuse.store(true, Ordering::SeqCst);
        let vault = Vault::new(keychain.clone(), dir.path());
        let place = vault.put("a", "s1").unwrap();
        let KeyPlace::File(file) = place else {
            panic!("the keychain refused: {place:?}")
        };
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "s1");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&file).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        let (text, place) = vault.get("a").unwrap().unwrap();
        assert_eq!(text.as_str(), "s1");
        assert!(matches!(place, KeyPlace::File(_)));

        // The keychain is back: the file moves into it.
        keychain.refuse.store(false, Ordering::SeqCst);
        let (_, place) = vault.get("a").unwrap().unwrap();
        assert_eq!(place, KeyPlace::Keychain);
        assert!(!file.exists());
        assert_eq!(keychain.entry("a").as_deref(), Some("s1"));
    }

    #[test]
    fn the_keys_never_print() {
        let keys = DeviceKeys {
            team: [1; 16],
            member: [2; 16],
            device: [3; 16],
            auth: [0xaa; 32],
            mls: [0xbb; 32],
            state_key: [0xdd; 32],
        };
        let shown = format!("{keys:?}");
        assert!(!shown.contains("aaaa") && !shown.contains("bbbb") && !shown.contains("dddd"));
    }
}
