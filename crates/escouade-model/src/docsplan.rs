//! What a planning of the spec's changes did (`docsplan`, ESC-262), kept on the discussion it was asked from.

use serde::{Deserialize, Serialize};

use crate::docs::Ref;

/// A ticket a planning made.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Made {
    pub id: String,
    pub key: String,
    pub title: String,
}

/// A ticket the planner proposed and Escouade left out: the board has it already (`key`).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Skipped {
    pub title: String,
    pub key: String,
}

/// What a planning did.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Plan {
    pub id: String,
    /// When (ms).
    pub at: i64,
    /// The discussion it was asked from (its agent); None: from the Specification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discussion: Option<String>,
    /// The changes it planned.
    pub pages: Vec<String>,
    pub decisions: Vec<Ref>,
    pub made: Vec<Made>,
    pub skipped: Vec<Skipped>,
    /// « Annuler » was clicked.
    pub cancelled: bool,
}
