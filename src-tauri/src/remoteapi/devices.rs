//! The devices that may connect to the remote API (`docs/REMOTE.md`, « La fondation commune », point 6): one token
//! each, revocable one by one. The list (`remote-devices.json`: id, name, dates) holds no token; each token is in the
//! keychain (`remote-device-<id>`), as Escouade's other secrets, or in the list's file, readable by its owner only,
//! when the keychain refuses it (Linux has none), as the MCP server's token.
//!
//! A presented token is compared with every device's in constant time, each time: the time an answer takes says
//! nothing of how close a token came.

use crate::integrations::secrets::{keep_secret, SecretStore};
use crate::model::{new_id, now_ms};
use crate::paths::DataDir;
use anyhow::{bail, Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

/// The file of the devices, in the data folder.
pub const FILE: &str = "remote-devices.json";

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
    /// Each device with its token, read once.
    known: RwLock<Vec<(Device, String, bool)>>,
}

fn entry(id: &str) -> String {
    format!("remote-device-{id}")
}

impl Devices {
    /// The devices of `data`, their tokens read from `store` (or the file).
    pub fn load(data: &DataDir, store: Arc<dyn SecretStore>) -> Self {
        let file = data.root().join(FILE);
        let kept: Vec<Kept> = match std::fs::read(&file) {
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
        Self {
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
        }
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
        self.known
            .write()
            .push((device.clone(), token.clone(), in_file));
        self.save()?;
        Ok((device, token))
    }

    /// The device `id` revoked: its token forgotten, it can no longer connect. False when there is none.
    pub fn revoke(&self, id: &str) -> Result<bool> {
        let gone = {
            let mut known = self.known.write();
            let before = known.len();
            known.retain(|(d, _, _)| d.id != id);
            known.len() != before
        };
        if gone {
            if let Err(e) = self.store.delete(&entry(id)) {
                log::warn!("remote API: the system keychain did not forget a revoked device's token: {e:#}");
            }
            self.save()?;
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
        let changed = {
            let mut known = self.known.write();
            let mut changed = false;
            for (d, _, _) in known.iter_mut().filter(|(d, _, _)| d.id == id) {
                d.last_seen = Some(now_ms());
                changed = true;
            }
            changed
        };
        if changed {
            if let Err(e) = self.save() {
                log::warn!("remote API: {FILE} not written: {e:#}");
            }
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
