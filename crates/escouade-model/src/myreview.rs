//! « Ta relecture » (ESC-494, PR-7 of the PR in Escouade): the user's own review of a pull request, waiting on this
//! machine until they publish it. Its text, and its remarks under their lines, each « en attente · pas publiée »; it
//! survives the view closed and the app started again (`PersistedState::pending_reviews`), and goes at its publication
//! (in one review of the host, with its verdict) or when the user gives it up. Nothing of it leaves without their click.

use crate::forge::DiffSide;
use serde::{Deserialize, Serialize};

/// A remark of the user's waiting under its line: the last line of its range on its side (the base's for a line removed),
/// and the text of its lines when it was written, to follow them when a commit moves them, or to say it is stale when
/// a commit takes them away.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PendingNote {
    pub id: u64,
    pub path: String,
    pub line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
    pub side: DiffSide,
    pub body: String,
    /// The text of its lines (the first to the last) in the diff it was written on.
    pub code: Vec<String>,
    /// When it was written (ms).
    pub created: i64,
    /// Its line is no longer in the diff last read (a commit took it away): it goes as a comment of the pull request,
    /// or the user takes it out. Said by the core at each reading, never by the window.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub stale: bool,
}

/// The user's review of one pull request, waiting: the pull request of a project (`host_repo` empty), or one held by an
/// agent or a ticket on another repository (`owner/name`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PendingReview {
    pub project_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub host_repo: String,
    pub number: u64,
    /// Its text, in Markdown: what goes above the remarks; needed to ask for changes.
    pub body: String,
    pub notes: Vec<PendingNote>,
    /// When it last changed (ms).
    pub updated_at: i64,
}

impl PendingReview {
    /// Of this pull request.
    pub fn is_of(&self, project_id: &str, number: u64, host_repo: &str) -> bool {
        self.project_id == project_id
            && self.number == number
            && self.host_repo.eq_ignore_ascii_case(host_repo)
    }

    /// Nothing waits in it: no text, no remark. It is not kept.
    pub fn is_empty(&self) -> bool {
        self.body.trim().is_empty() && self.notes.is_empty()
    }

    /// The id of a remark to come.
    pub fn next_id(&self) -> u64 {
        self.notes.iter().map(|n| n.id).max().unwrap_or(0) + 1
    }
}
