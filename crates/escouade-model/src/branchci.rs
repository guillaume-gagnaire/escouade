//! The CI of a branch as its host last told it (`branchci`, ESC-63): the target of a project, or the branch a ticket
//! pushed.

use serde::{Deserialize, Serialize};

use crate::forge::{CheckState, RepoRef};
use crate::model::FailedCheck;

/// The CI of a branch, as its host last told it: the target of a project, or the branch a ticket
/// pushed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct BranchCi {
    /// The code host's kind (`github`, `gitlab`…) and its instance's host, which give its client.
    pub kind: String,
    pub host: String,
    /// The repository on the host (`acme/demo`).
    pub namespace: String,
    pub name: String,
    pub branch: String,
    /// Its checks in one word (a failure wins, then a wait); none while it has none.
    pub checks: Option<CheckState>,
    /// How many checks it has (the skipped ones aside), and how many of them failed.
    pub checks_total: u32,
    pub checks_failed: u32,
    /// The checks that failed, each with whether its host keeps its log.
    pub failed: Vec<FailedCheck>,
    /// Red since when (ms): the first failed check's end when the host says it, else the reading
    /// that first saw it red. None while it is not red.
    pub red_since: Option<i64>,
    /// When it was last read well (ms).
    pub checked_at: Option<i64>,
    /// Why the last reading failed, and since when (ms): the pill says it and keeps the last state.
    pub error: Option<String>,
    pub error_since: Option<i64>,
    /// Its host runs failed jobs again (« Relancer les jobs en échec »).
    pub rerun: bool,
}

impl BranchCi {
    pub fn repo(&self) -> RepoRef {
        RepoRef {
            namespace: self.namespace.clone(),
            name: self.name.clone(),
        }
    }

    pub fn red(&self) -> bool {
        self.checks == Some(CheckState::Failure)
    }

    /// The same, but for when it was read: what the window is told again only when it changed.
    pub fn shown(&self) -> Self {
        Self {
            checked_at: None,
            ..self.clone()
        }
    }
}
