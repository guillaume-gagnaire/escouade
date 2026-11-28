//! The secrets of a server (ESC-514, `docs/REMOTE.md`, « Ce qui change de machine »): a VPS has no keychain (no
//! Secret Service, no session), so `escouade-server` keeps the core's secrets (the keys of the integrations, of
//! Cursor, of the voices online and of the plugins, the devices' tokens) in `secrets.json`, in its data folder,
//! readable by its owner alone (mode 0600 under Unix; written as the state is, a crash leaves the former content or
//! the new one whole). The client is told so (`welcome`: `secretsInFile`).
//!
//! Read again when another process changed it (`escouade-server devices revoke` beside the server), written by one
//! process at a time (`headless::with_file_lock`). Its errors never say a secret.

use crate::headless::with_file_lock;
use crate::integrations::secrets::SecretStore;
use crate::paths::DataDir;
use anyhow::{Context, Result};
use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The file of the secrets, in the data folder.
pub const FILE: &str = "secrets.json";

type Entries = BTreeMap<String, String>;

/// The secrets in their file.
pub struct FileStore {
    path: PathBuf,
    /// What the file held when it was last read or written (its time and length then).
    cache: Mutex<Option<((SystemTime, u64), Entries)>>,
}

fn stamp_of(path: &Path) -> Option<(SystemTime, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

impl FileStore {
    /// The secrets of `data`. A file readable by others (copied by hand, a backup put back) is made its owner's
    /// alone again, and the log says so.
    pub fn open(data: &DataDir) -> Result<Self> {
        let path = data.root().join(FILE);
        #[cfg(unix)]
        if let Ok(meta) = std::fs::metadata(&path) {
            use std::os::unix::fs::PermissionsExt;
            if meta.permissions().mode() & 0o077 != 0 {
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                    .with_context(|| format!("{FILE}: made its owner's alone"))?;
                log::warn!("{FILE} was readable by others: it is its owner's alone again (0600)");
            }
        }
        let store = Self {
            path,
            cache: Mutex::new(None),
        };
        // Unreadable, it is said now rather than at the first secret asked for.
        store.entries()?;
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// What the file holds now (none yet: nothing).
    fn entries(&self) -> Result<Entries> {
        let stamp = stamp_of(&self.path);
        let mut cache = self.cache.lock();
        if let (Some(now), Some((seen, entries))) = (stamp, cache.as_ref()) {
            if now == *seen {
                return Ok(entries.clone());
            }
        }
        let entries: Entries = match std::fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| anyhow::anyhow!("{FILE} unreadable (line {})", e.line()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Entries::new(),
            Err(e) => return Err(anyhow::anyhow!("{FILE}: {e}")),
        };
        *cache = stamp.map(|s| (s, entries.clone()));
        Ok(entries)
    }

    /// `change` made to the entries as the file holds them now, then written: one process at a time.
    fn change(&self, change: impl FnOnce(&mut Entries) -> bool) -> Result<()> {
        with_file_lock(&self.path, || {
            let mut entries = self.entries()?;
            if !change(&mut entries) {
                return Ok(());
            }
            let bytes = serde_json::to_vec_pretty(&entries)?;
            crate::paths::write_private(&self.path, &bytes)
                .with_context(|| format!("writing {FILE}"))?;
            *self.cache.lock() = stamp_of(&self.path).map(|s| (s, entries));
            Ok(())
        })
    }
}

impl FileStore {
    /// The secret `name` changed from what the file holds now (`None`: none) to what `change` gives (`None`:
    /// forgotten), one process at a time: what the connectors' codes need (RP-4), written by the command and by the
    /// server beside it.
    pub fn update(
        &self,
        name: &str,
        change: &mut dyn FnMut(Option<String>) -> Option<String>,
    ) -> Result<()> {
        self.change(|entries| {
            let before = entries.get(name).cloned();
            let after = change(before.clone());
            if after == before {
                return false;
            }
            match after {
                Some(v) => entries.insert(name.to_string(), v),
                None => entries.remove(name),
            };
            true
        })
    }
}

impl SecretStore for FileStore {
    fn get(&self, name: &str) -> Result<Option<String>> {
        Ok(self.entries()?.get(name).cloned())
    }

    fn set(&self, name: &str, secret: &str) -> Result<()> {
        self.change(|entries| {
            entries.insert(name.to_string(), secret.to_string());
            true
        })
    }

    fn delete(&self, name: &str) -> Result<()> {
        self.change(|entries| entries.remove(name).is_some())
    }
}
