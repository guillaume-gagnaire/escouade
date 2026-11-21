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
    /// The team the file says the project belongs to (its key `team`, TS-7), as checked here: what the band of the
    /// invitation shows. Never saved: read again with the file.
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub team: Option<TeamHint>,
    /// « Ne plus proposer pour ce projet »: the fingerprint of the team the band named (its server and its id). A file
    /// that names another team shows the band again.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_dismissed: Option<String>,
    /// What the project's team sets on its plugins (TS-13, ESC-532): the keys `plugins.<id>.<key>` of its file, as the
    /// page Plugins of the project marks them. Never saved: read again with the file.
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub team_plugins: Option<TeamPlugins>,
}

/// The keys `plugins.<id>.<key>` a team's file sets on its projects (TS-13, ESC-532), by their whole key: `defaults`
/// recommends (rank 2, under the repository's file and this machine), `enforced` imposes (rank 5, over them). Judged
/// against each plugin's manifest where they are read: a secret, a profile, a value imported, what is kept off the files
/// (`repo: false`), a key unknown or of a wrong type is said and ignored.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct TeamPlugins {
    /// The team's name, as the lock says it.
    pub team: String,
    pub defaults: BTreeMap<String, Value>,
    pub enforced: BTreeMap<String, Value>,
}

/// The key `team` of a repository's `.escouade.yml` (TS-7): where the project's team is, nothing secret. It comes from
/// the repository, so nothing of it is trusted: its name is cleaned, its address checked, and nothing goes to that
/// address before the user's gesture.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct TeamHint {
    /// The server's base (`https://team.acme.fr`), and its host as the band names it (`team.acme.fr`).
    pub server: String,
    pub host: String,
    /// The team's id (hex, 32 digits).
    pub team: String,
    /// The id of the project in the team's list (hex), when the file gives one.
    pub project: Option<String>,
    /// The team's name, as the file says it (cleaned: no control or direction character, 60 characters at most).
    pub name: String,
    /// The fingerprint the band's « Ne plus proposer » keeps.
    pub fingerprint: String,
}
