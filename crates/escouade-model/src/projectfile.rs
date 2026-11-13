//! What a project keeps of its `.escouade.yml` (`projectfile`, ESC-190): the keys set on this machine, the commands
//! trusted, and the file as last read.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Something of the file that was left out, or that the user should know.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Warning {
    /// From 1; none when it is the whole file's.
    pub line: Option<u32>,
    pub text: String,
}

/// Commands of the file the user has not trusted yet: inactive until then.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Pending {
    /// Their fingerprint: what « Faire confiance à ces commandes » trusts, no other.
    pub hash: String,
    /// By key (`worktrees.setup`, `launch`…), as the file writes them.
    pub commands: BTreeMap<String, Value>,
}

/// The file as the project last read it (never saved: read again at each start).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SharedFile {
    /// The keys it sets.
    pub keys: Vec<String>,
    /// What the window says of each key (`get`): those kept on this machine show their file's value.
    pub values: BTreeMap<String, Value>,
    pub warnings: Vec<Warning>,
    /// Why it could not be read at all.
    pub error: Option<Warning>,
    pub version: u32,
    pub pending: Option<Pending>,
}

/// A project's own record of its file: saved with it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ProjectShared {
    /// The keys of the file set otherwise on this machine: their own value goes first.
    pub local: Vec<String>,
    /// The fingerprint of the file's commands the user trusted (`Pending::hash`).
    pub trusted: String,
    /// The file as last read: None while the project has none.
    #[serde(skip_deserializing)]
    pub file: Option<SharedFile>,
    /// The fingerprint of the text last read ("" for no file): a file the same is not read again.
    #[serde(skip)]
    pub seen: Option<String>,
}
