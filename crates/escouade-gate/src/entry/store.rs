//! What the server keeps of its connectors (§ 4.5): their list and the trace of those removed, in a file beside the data
//! (`connectors.json`, never a key), and their keys and the pending codes in the server's secrets (`secrets.json` in
//! server mode, readable by its owner alone). The core gives both through [`Store`]; the tests keep them in memory
//! ([`MemoryStore`]).

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The file of the connectors, in the data folder.
pub const FILE: &str = "connectors.json";

/// The secret of the server's static key (its private half, in hexadecimal): made at the first pairing, kept.
pub const SERVER_KEY: &str = "connector-server-key";

/// The secret of the codes that wait for their connector: a JSON list of [`Pending`] (each code's pre-shared key,
/// never the code).
pub const PENDING: &str = "connector-pairing";

/// The secret of a connector's static key (its public half, in hexadecimal), pinned at its pairing.
pub fn key_secret(id: &str) -> String {
    format!("connector-key-{id}")
}

/// The traces of removed connectors kept: the most recent.
pub const MAX_REMOVED: usize = 32;

/// A connector paired with the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paired {
    pub id: String,
    /// What the user called it (« Acme »).
    pub name: String,
    /// When it was paired (ms since 1970).
    pub paired_at: u64,
    /// Its last connection, begun or ended (ms since 1970).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<u64>,
    /// Connected since (ms since 1970), as the server that holds its channel says: what the command beside it reads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connected_since: Option<u64>,
    /// Its version, as it last said it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// A version of it the server last refused (too old, or newer than the server), until one is accepted again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused_version: Option<String>,
    /// Its ceiling, as it last sent it: its rules as it writes them.
    #[serde(default)]
    pub ceiling: Vec<String>,
}

/// The trace of a removed connector: never its key, its SHA-256.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Removed {
    pub id: String,
    pub name: String,
    /// When (ms since 1970).
    pub at: u64,
    /// From where: `escouade-server` (the command), or the space (a device's name).
    pub by: String,
    /// The SHA-256 of its public key, in hexadecimal.
    pub key_hash: String,
}

/// What `connectors.json` holds.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Saved {
    #[serde(default)]
    pub connectors: Vec<Paired>,
    #[serde(default)]
    pub removed: Vec<Removed>,
    /// The entry's address, as the server that runs serves it (`wss://…/connector`): what `connector pair` prints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
}

/// A code that waits for its connector, as the secrets keep it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pending {
    /// The connector's name to be.
    pub name: String,
    /// The code's pre-shared key (`pairing::CodeKey`), in hexadecimal.
    pub key: String,
    /// When it ends (ms since 1970).
    pub expires_at: u64,
}

/// Where the connectors are kept. Each change is made to what is kept now, by one process at a time (the command
/// beside the server changes them too).
pub trait Store: Send + Sync + 'static {
    /// The list as it is now.
    fn read(&self) -> Result<Saved, String>;
    /// `change` made to the list as it is now, and written when it says so.
    fn change(&self, change: &mut dyn FnMut(&mut Saved) -> bool) -> Result<(), String>;
    /// A secret.
    fn secret(&self, name: &str) -> Result<Option<String>, String>;
    /// A secret changed from what it is now (`None`: none) to what `change` gives (`None`: forgotten).
    fn change_secret(
        &self,
        name: &str,
        change: &mut dyn FnMut(Option<String>) -> Option<String>,
    ) -> Result<(), String>;
}

/// A store in memory, for the tests.
#[derive(Default)]
pub struct MemoryStore {
    saved: Mutex<Saved>,
    secrets: Mutex<BTreeMap<String, String>>,
}

impl MemoryStore {
    /// The secrets, as a test looks at them.
    pub fn secrets(&self) -> BTreeMap<String, String> {
        self.secrets.lock().clone()
    }
}

impl Store for MemoryStore {
    fn read(&self) -> Result<Saved, String> {
        Ok(self.saved.lock().clone())
    }

    fn change(&self, change: &mut dyn FnMut(&mut Saved) -> bool) -> Result<(), String> {
        let mut saved = self.saved.lock();
        let mut next = saved.clone();
        if change(&mut next) {
            *saved = next;
        }
        Ok(())
    }

    fn secret(&self, name: &str) -> Result<Option<String>, String> {
        Ok(self.secrets.lock().get(name).cloned())
    }

    fn change_secret(
        &self,
        name: &str,
        change: &mut dyn FnMut(Option<String>) -> Option<String>,
    ) -> Result<(), String> {
        let mut secrets = self.secrets.lock();
        match change(secrets.get(name).cloned()) {
            Some(v) => secrets.insert(name.to_string(), v),
            None => secrets.remove(name),
        };
        Ok(())
    }
}
