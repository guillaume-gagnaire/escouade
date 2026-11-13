//! What a memory of the project's shared memory is about (`write_memory`'s `type`): the app's
//! `memory` keeps the memories themselves.

use rmcp::schemars;
use serde::{Deserialize, Serialize};

/// What a memory is about, as Claude Code sorts its own.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// A way to work, a correction or a confirmation of how to do something.
    #[default]
    Feedback,
    /// The work under way, a constraint of the project.
    Project,
    /// Where to find something (a page, a dashboard, a tool).
    Reference,
    /// Who the user is, what they prefer.
    User,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Feedback => "feedback",
            Self::Project => "project",
            Self::Reference => "reference",
            Self::User => "user",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        [Self::Feedback, Self::Project, Self::Reference, Self::User]
            .into_iter()
            .find(|k| k.as_str() == s.trim().to_lowercase())
    }
}
