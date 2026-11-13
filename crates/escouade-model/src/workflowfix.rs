//! The user's corrections of a project's workflows (`workflowfix`, ESC-195), kept with the project.

use serde::{Deserialize, Serialize};

/// What tells a step added by hand: a command typed, a subagent launched, a file written.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum Signal {
    Command(String),
    Agent(String),
    File(String),
}

/// A step as corrected.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct FixStep {
    /// As the user named it.
    pub name: String,
    /// The steps of the reading it stands for (their `name`), several once merged; none for a
    /// step added by hand.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub from: Vec<String>,
    /// What tells a step added by hand.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub signals: Vec<Signal>,
}

/// The corrections of one workflow.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Fix {
    /// The workflow's id, as the reading gives it.
    pub workflow: String,
    /// « Ce n'est pas un workflow ».
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub dismissed: bool,
    /// Its steps in their order; empty: the reading's.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<FixStep>,
    /// Steps of the reading taken out (a guessed one « Retirer », or « Retirer l'étape »).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub removed: Vec<String>,
}

pub type Fixes = Vec<Fix>;

impl Fix {
    /// Whether it changes nothing.
    pub fn is_empty(&self) -> bool {
        !self.dismissed && self.steps.is_empty() && self.removed.is_empty()
    }
}
