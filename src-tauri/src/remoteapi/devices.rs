//! The devices that may connect to the remote API (`docs/REMOTE.md`, « La fondation commune », point 6): one token
//! each, revocable one by one. The list (`remote-devices.json`: id, name, dates) holds no token; each token is in the
//! keychain (`remote-device-<id>`), as Escouade's other secrets, or in the list's file, readable by its owner only,
//! when the keychain refuses it (Linux has none), as the MCP server's token.
//!
//! A presented token is compared with every device's in constant time, each time: the time an answer takes says
//! nothing of how close a token came.
//!
//! A revoked device leaves a trace beside the list (`remote-revoked.json`, ESC-537): its name, when and by whom, and
//! its token's SHA-256 (never the token), so that a client that still presents it is told « revoked on …, from … »
//! rather than « unknown ». The last [`MAX_REVOKED`] are kept.

use crate::integrations::secrets::{keep_secret, SecretStore};
use crate::model::{new_id, now_ms};
use crate::paths::DataDir;
use anyhow::{bail, Context, Result};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

/// The file of the devices, in the data folder.
pub const FILE: &str = "remote-devices.json";

/// The file of the revoked devices' traces (ESC-537), beside it.
pub const REVOKED_FILE: &str = "remote-revoked.json";

/// The revoked devices whose trace is kept.
pub const MAX_REVOKED: usize = 32;

/// A revoked device's trace: never its token, its SHA-256.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Trace {
    name: String,
    at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    by: Option<String>,
    hash: String,
}

fn token_hash(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"escouade-device:");
    h.update(token.as_bytes());
    hex::encode(h.finalize())
}

/// A device, as the list keeps it and the interface will show it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    /// The name the user gave it (« Téléphone », « Portable du bureau »).
    pub name: String,
    pub created_at: i64,
    #[serde(default)]
    pub last_seen: Option<i64>,
}

/// A device as its file keeps it: its token there only when the keychain refused it.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Kept {
    #[serde(flatten)]
    device: Device,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    token: Option<String>,
}

/// The devices and their tokens.
pub struct Devices {
    file: Option<PathBuf>,
    store: Arc<dyn SecretStore>,
    /// Each device with its token, read from the file and the store.
    known: RwLock<Vec<(Device, String, bool)>>,
    /// The file as it was last read or written (its time and length): another process of the same data folder
    /// (`escouade-server devices revoke`, ESC-514) changes it, and `refresh` reads it again.
    stamp: Mutex<Option<(SystemTime, u64)>>,
}

fn entry(id: &str) -> String {
    format!("remote-device-{id}")
}

fn read_traces(file: &Path) -> Vec<Trace> {
    std::fs::read(file)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn stamp_of(file: &Path) -> Option<(SystemTime, u64)> {
    let meta = std::fs::metadata(file).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

/// The devices of `file`, their tokens from the file or from `store`.
fn read_known(file: &Path, store: &dyn SecretStore) -> Vec<(Device, String, bool)> {
    let kept: Vec<Kept> = match std::fs::read(file) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
            log::error!("remote API: {FILE} unreadable ({e}): no device");
            Vec::new()
        }),
        Err(_) => Vec::new(),
    };
    let mut known = Vec::new();
    for k in kept {
        let (token, in_file) = match k.token {
            Some(t) if !t.is_empty() => (Some(t), true),
            _ => (store.get(&entry(&k.device.id)).ok().flatten(), false),
        };
        match token {
            Some(token) if !token.is_empty() => known.push((k.device, token, in_file)),
            _ => log::warn!(
                "remote API: the device {} has no token any more: it cannot connect",
                k.device.id
            ),
        }
    }
    known
}

impl Devices {
    /// The devices of `data`, their tokens read from `store` (or the file).
    pub fn load(data: &DataDir, store: Arc<dyn SecretStore>) -> Self {
        let file = data.root().join(FILE);
        let known = read_known(&file, &*store);
        Self {
            stamp: Mutex::new(stamp_of(&file)),
            file: Some(file),
            store,
            known: RwLock::new(known),
        }
    }

    /// A device of the tests alone, kept in memory (`ESCOUADE_REMOTE_API_TOKEN`, debug builds): no file, no keychain.
    #[cfg(debug_assertions)]
    pub fn of_test(name: &str, token: String, store: Arc<dyn SecretStore>) -> Self {
        let device = Device {
            id: "test".into(),
            name: name.into(),
            created_at: now_ms(),
            last_seen: None,
        };
        Self {
            file: None,
            store,
            known: RwLock::new(vec![(device, token, true)]),
            stamp: Mutex::new(None),
        }
    }

    /// The file read again when another process changed it since: true when it did.
    pub fn refresh(&self) -> bool {
        let Some(file) = &self.file else {
            return false;
        };
        let now = stamp_of(file);
        let mut stamp = self.stamp.lock();
        if *stamp == now {
            return false;
        }
        *self.known.write() = read_known(file, &*self.store);
        *stamp = now;
        true
    }

    /// `change` made to the devices as the file holds them now, then written, one process at a time (the file's lock):
    /// what another process of the data folder wrote meanwhile is kept.
    fn change<T>(&self, change: impl FnOnce(&mut Vec<(Device, String, bool)>) -> T) -> Result<T> {
        let Some(file) = self.file.clone() else {
            return Ok(change(&mut self.known.write()));
        };
        crate::headless::with_file_lock(&file, || {
            self.refresh();
            let out = change(&mut self.known.write());
            self.save()?;
            *self.stamp.lock() = stamp_of(&file);
            Ok(out)
        })
    }

    /// A new device named `name`, and its token: given once, to the device; kept in the keychain (or the file).
    pub fn add(&self, name: &str) -> Result<(Device, String)> {
        let name = name.trim();
        if name.is_empty() {
            bail!(tr!("un appareil a un nom", "a device has a name"));
        }
        let device = Device {
            id: new_id(),
            name: name.to_string(),
            created_at: now_ms(),
            last_seen: None,
        };
        let token = escouade_mcp::new_token();
        let in_file = match keep_secret(&*self.store, &entry(&device.id), &token) {
            Ok(()) => false,
            Err(e) => {
                log::warn!("remote API: a device's token kept in its file, the system keychain refused it: {e:#}");
                true
            }
        };
        self.change(|known| known.push((device.clone(), token.clone(), in_file)))?;
        Ok((device, token))
    }

    /// The device `id` revoked: its token forgotten, it can no longer connect. False when there is none.
    pub fn revoke(&self, id: &str) -> Result<bool> {
        self.revoke_by(id, None)
    }

    /// The device `id` revoked at the demand of the device named `by` (none: the server itself), its trace kept.
    pub fn revoke_by(&self, id: &str, by: Option<&str>) -> Result<bool> {
        let gone = self.change(|known| {
            let gone: Vec<(Device, String, bool)> = known
                .iter()
                .filter(|(d, _, _)| d.id == id)
                .cloned()
                .collect();
            known.retain(|(d, _, _)| d.id != id);
            gone
        })?;
        if let (Some(file), Some((device, token, _))) = (&self.file, gone.first()) {
            let traces = file.with_file_name(REVOKED_FILE);
            let trace = Trace {
                name: device.name.clone(),
                at: now_ms(),
                by: by.map(str::to_string),
                hash: token_hash(token),
            };
            let kept = crate::headless::with_file_lock(&traces, || {
                let mut all = read_traces(&traces);
                all.push(trace);
                let over = all.len().saturating_sub(MAX_REVOKED);
                all.drain(..over);
                let bytes = serde_json::to_vec_pretty(&all)?;
                crate::paths::write_private(&traces, &bytes)
                    .with_context(|| format!("writing {REVOKED_FILE}"))
            });
            if let Err(e) = kept {
                log::warn!("remote API: a revoked device's trace not kept: {e:#}");
            }
        }
        let gone = !gone.is_empty();
        if gone {
            if let Err(e) = self.store.delete(&entry(id)) {
                log::warn!("remote API: the system keychain did not forget a revoked device's token: {e:#}");
            }
        }
        Ok(gone)
    }

    /// The device whose token `presented` is, compared in constant time with every one of them.
    pub fn find(&self, presented: &str) -> Option<Device> {
        let mut found = None;
        for (device, token, _) in self.known.read().iter() {
            if escouade_mcp::same_token(token, presented) {
                found = Some(device.clone());
            }
        }
        found.filter(|_| !presented.is_empty())
    }

    /// The token `presented` was a device's, revoked: its name, when and by whom.
    pub fn revoked(&self, presented: &str) -> Option<crate::remoteapi::proto::Revoked> {
        let file = self.file.as_ref()?.with_file_name(REVOKED_FILE);
        if presented.is_empty() {
            return None;
        }
        let hash = token_hash(presented);
        read_traces(&file)
            .into_iter()
            .rev()
            .find(|t| escouade_mcp::same_token(&t.hash, &hash))
            .map(|t| crate::remoteapi::proto::Revoked {
                device: t.name,
                at: t.at,
                by: t.by,
            })
    }

    /// Whether the device `id` is still one (not revoked).
    pub fn holds(&self, id: &str) -> bool {
        self.known.read().iter().any(|(d, _, _)| d.id == id)
    }

    pub fn list(&self) -> Vec<Device> {
        self.known
            .read()
            .iter()
            .map(|(d, _, _)| d.clone())
            .collect()
    }

    /// The device `id` seen now.
    pub fn seen(&self, id: &str) {
        let changed = self.change(|known| {
            let mut changed = false;
            for (d, _, _) in known.iter_mut().filter(|(d, _, _)| d.id == id) {
                d.last_seen = Some(now_ms());
                changed = true;
            }
            changed
        });
        if let Err(e) = changed {
            log::warn!("remote API: {FILE} not written: {e:#}");
        }
    }

    fn save(&self) -> Result<()> {
        let Some(file) = &self.file else {
            return Ok(());
        };
        let kept: Vec<Kept> = self
            .known
            .read()
            .iter()
            .map(|(device, token, in_file)| Kept {
                device: device.clone(),
                token: in_file.then(|| token.clone()),
            })
            .collect();
        let bytes = serde_json::to_vec_pretty(&kept)?;
        crate::paths::write_private(file, &bytes).with_context(|| format!("writing {FILE}"))
    }
}
