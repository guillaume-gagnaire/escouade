//! The review of a pull request written by an agent (`prreview`, ESC-66), kept until the user publishes it.

use serde::{Deserialize, Serialize};

/// What the user decided of a comment. `Open` holds the publication back: each one is decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NoteChoice {
    #[default]
    Open,
    Kept,
    Dropped,
}

/// A line of a file's diff: `+`, `-`, ` `, or `@` between two hunks. `line`: its number in the new
/// version, what a comment anchors to (none for a line removed).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub kind: char,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    pub text: String,
}

/// A comment of the review, on a file and a line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReviewNote {
    pub id: u32,
    pub path: String,
    pub line: u32,
    /// What will be published, as the user left it.
    pub body: String,
    /// What the agent wrote: the window says a comment was edited, and brings it back.
    pub proposed: String,
    pub choice: NoteChoice,
    /// The lines of the diff it is written on, the last one its own.
    pub excerpt: Vec<DiffLine>,
    /// Its line is not in the diff: the host refuses it there, it goes in the review's text.
    pub outside: bool,
}

impl Default for ReviewNote {
    fn default() -> Self {
        Self {
            id: 0,
            path: String::new(),
            line: 0,
            body: String::new(),
            proposed: String::new(),
            choice: NoteChoice::Open,
            excerpt: Vec::new(),
            outside: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReviewPhase {
    /// The agent reads; kept in memory only: a start does not take it up again.
    #[default]
    Reading,
    /// Written: it waits for the user.
    Ready,
    /// Going to the host, on the user's click.
    Publishing,
    Failed,
    Published,
}

/// The review of a pull request: one per pull request, the latest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PrReview {
    pub project_id: String,
    pub number: u64,
    /// The host's word (`PR`, `MR`) and its name (`GitHub`).
    pub word: String,
    pub host: String,
    /// Who opened it: the host refuses their own verdict on it.
    pub author: String,
    pub title: String,
    pub head: String,
    pub base: String,
    pub url: String,
    /// The commit read: a publication after a new push says so first.
    pub head_sha: String,
    pub phase: ReviewPhase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The review's own text, above its comments.
    pub summary: String,
    pub notes: Vec<ReviewNote>,
    /// The files the agent did not read: too long, or generated.
    pub left_out: Vec<String>,
    /// Who read it, as the settings of the written texts name it (« Claude Sonnet »).
    pub by: String,
    /// When it was asked (ms): tells a reading from the one that replaced it.
    pub at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<i64>,
}
