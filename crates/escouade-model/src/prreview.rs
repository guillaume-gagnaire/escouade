//! The review of a pull request written by an agent (`prreview`, ESC-66). Since PR-5 (ESC-490) each of its remarks is a
//! thread of the agent under its line, of a severity, maybe with a suggestion: the user drops it (its thread resolved
//! here) or publishes it (a comment of theirs on the host), one by one or those kept at once. Nothing leaves without a
//! click of the user.

use serde::{Deserialize, Serialize};

/// What the user decided of a remark: still open, kept to publish with the others, dropped (its thread resolved, folded,
/// kept), or published on the host (its thread is then the host's).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NoteChoice {
    #[default]
    Open,
    Kept,
    Dropped,
    Published,
}

impl NoteChoice {
    /// Still waits for the user: open or kept.
    pub fn live(self) -> bool {
        matches!(self, Self::Open | Self::Kept)
    }
}

/// How much a remark weighs, the heaviest first: a security hole, a bug, a gap with the spec, a detail.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Security,
    Bug,
    Spec,
    #[default]
    Detail,
}

impl Severity {
    pub const ALL: [Severity; 4] = [Self::Security, Self::Bug, Self::Spec, Self::Detail];

    /// The agent's word, in English or in French, any case; a detail when it says none known.
    pub fn of_word(word: &str) -> Self {
        match word.trim().to_lowercase().as_str() {
            "security" | "sécurité" | "securite" | "critical" | "critique" => Self::Security,
            "bug" | "bogue" | "error" | "erreur" => Self::Bug,
            "spec" | "specification" | "spécification" => Self::Spec,
            _ => Self::Detail,
        }
    }
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

/// A remark of the review: on a line (`line`, from `start_line` for a range), on a file (`line` 0) or on the pull
/// request (`path` empty).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReviewNote {
    pub id: u32,
    pub path: String,
    pub line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
    pub severity: Severity,
    /// What will be published, as the user left it.
    pub body: String,
    /// What the agent wrote: the window says a comment was edited, and brings it back.
    pub proposed: String,
    pub choice: NoteChoice,
    /// The lines of the diff it is written on, the last one its own.
    pub excerpt: Vec<DiffLine>,
    /// Its line is not in the diff: the host refuses it there, it goes in the conversation.
    pub outside: bool,
    /// What the agent proposes in place of its lines (`start_line` to `line`), whole: read and copied, never run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// The text of its lines when it was written: a reading after a push tells by it a remark that still holds.
    pub code: Vec<String>,
    /// Its code changed since it was written (a reading after a push): said apart, like an outdated thread.
    pub stale: bool,
    /// When it was published on the host (ms).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<i64>,
}

impl Default for ReviewNote {
    fn default() -> Self {
        Self {
            id: 0,
            path: String::new(),
            line: 0,
            start_line: None,
            severity: Severity::Detail,
            body: String::new(),
            proposed: String::new(),
            choice: NoteChoice::Open,
            excerpt: Vec::new(),
            outside: false,
            suggestion: None,
            code: Vec::new(),
            stale: false,
            published_at: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReviewPhase {
    /// The agent reads; kept in memory only: a start does not take it up again.
    #[default]
    Reading,
    /// Written: its remarks wait for the user.
    Ready,
    /// Its remarks going to the host, on the user's click (ESC-66's whole publication before PR-5): a start makes it
    /// `Ready`.
    Publishing,
    Failed,
    /// Published whole (ESC-66, before PR-5): a start makes it `Ready`, its kept remarks published.
    Published,
}

/// The review of a pull request: one per pull request, the latest, with the remarks of the readings before it that
/// still hold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PrReview {
    pub project_id: String,
    pub number: u64,
    /// The host's word (`PR`, `MR`) and its name (`GitHub`).
    pub word: String,
    pub host: String,
    /// The host's kind (`github`…): a suggestion is published in its form.
    pub kind: String,
    /// Who opened it.
    pub author: String,
    pub title: String,
    pub head: String,
    pub base: String,
    pub url: String,
    /// The commit read.
    pub head_sha: String,
    pub phase: ReviewPhase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The agent's view of the whole.
    pub summary: String,
    pub notes: Vec<ReviewNote>,
    /// The files the agent did not read: too long, or generated.
    pub left_out: Vec<String>,
    /// Who read it, as the settings of the written texts name it (« Claude Sonnet »).
    pub by: String,
    /// When it was asked (ms): tells a reading from the one that replaced it.
    pub at: i64,
    /// When the last reading ended (ms).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<i64>,
}
