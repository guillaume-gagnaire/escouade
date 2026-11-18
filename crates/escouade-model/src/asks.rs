//! The asynchronous requests of the agents (« À toi », docs/INBOX.md § 3): what an agent waits for
//! from the user without stopping (a choice, a yes or no, an answer, a task), kept in the state
//! (`PersistedState::asks`) with its files' list, its answer and its history. The files themselves
//! are copies under `<données>/asks/<id>/files/` (`DataDir::ask_dir`), never in the JSON. What the
//! core does with them (the bounds, the states, the answer given back) is the app's (`asks`).

use serde::{Deserialize, Serialize};

/// The sort of a request (§ 3.2).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AskKind {
    /// One option (or several, `multi`) among 2 to 6.
    #[default]
    Choice,
    /// Yes or no: a go-ahead.
    Confirm,
    /// A sentence written by the user.
    Answer,
    /// Something the user alone can do, its steps ticked.
    Task,
}

impl AskKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Choice => "choice",
            Self::Confirm => "confirm",
            Self::Answer => "answer",
            Self::Task => "task",
        }
    }
}

/// Where a request stands (§ 3.4).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AskState {
    /// It counts in the box.
    #[default]
    Open,
    /// « Plus tard », until `Ask::snoozed_until`.
    Snoozed,
    /// Answered: the answer goes (or went) to its author, who has not read it yet.
    Answered,
    /// Its author got the answer (its message, or `get_ask` / `wait_ask` after it).
    Read,
    /// Taken back by its author (`withdraw_ask`), with its reason.
    Withdrawn,
    /// Its date passed unanswered.
    Expired,
    /// Taken out by Escouade: its agent deleted or archived, its ticket closed, its project removed.
    Retired,
}

impl AskState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Snoozed => "snoozed",
            Self::Answered => "answered",
            Self::Read => "read",
            Self::Withdrawn => "withdrawn",
            Self::Expired => "expired",
            Self::Retired => "retired",
        }
    }

    /// Still waiting for the user (open, or put off for later).
    pub fn waiting(self) -> bool {
        matches!(self, Self::Open | Self::Snoozed)
    }

    /// Over: answered and read, withdrawn, expired or retired. An answer not read yet is not.
    pub fn closed(self) -> bool {
        !self.waiting() && self != Self::Answered
    }
}

/// How urgent a request is.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Urgency {
    #[default]
    Normal,
    /// « Aujourd'hui ».
    Today,
}

/// Who left a request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AskAuthor {
    /// The agent of Escouade that left it; None for Claude outside Escouade (a client of the server).
    pub agent: Option<String>,
    /// Its name when it left it (the agent may be renamed, or gone).
    pub name: String,
    /// It was its project's squad lead (`Project::chef`) then.
    pub chef: bool,
}

/// An option of a choice.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AskOption {
    /// `a` to `f`.
    pub id: String,
    pub label: String,
    pub description: String,
}

/// The agent's recommendation: an option (a choice), a text (an answer), and why.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AskRecommend {
    pub option: Option<String>,
    pub text: Option<String>,
    pub why: String,
}

/// What a file holds, as its content says (never its name, nor what the agent said).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AskFileKind {
    /// PNG, JPEG, GIF, WebP or AVIF.
    Image,
    Pdf,
    Markdown,
    /// A Word document (`.docx`).
    Word,
    /// CSV or TSV.
    Csv,
    /// Anything else (an SVG included: it may hold a script).
    #[default]
    Other,
}

/// A file of a request: its copy is `<ask dir>/files/<file>`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AskFile {
    /// Where it is under the request's `files` folder: `<uniq>/<name>`.
    pub file: String,
    /// Its name, as given (made safe).
    pub name: String,
    pub kind: AskFileKind,
    /// Its media type, read from its content.
    pub media_type: String,
    pub size: u64,
    /// The option it goes with (its id); None: the request's own.
    pub option: Option<String>,
    /// Joined by the user to the answer.
    pub by_user: bool,
}

/// How an answer was given.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AskVia {
    #[default]
    Click,
    Keyboard,
    Voice,
    Phone,
    /// By the agent the user left it to.
    Delegated,
}

/// The answer to a request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AskAnswer {
    /// A choice: the options chosen, by id.
    pub options: Vec<String>,
    /// A yes or no.
    pub yes: Option<bool>,
    /// An answer's text.
    pub text: Option<String>,
    /// A task: done or not, and its steps ticked.
    pub done: Option<bool>,
    pub steps: Vec<bool>,
    /// A word for the agent.
    pub note: Option<String>,
    /// Who answered: None for the user; the agent the user left it to (its id), otherwise.
    pub by: Option<String>,
    /// That agent's name.
    pub by_name: Option<String>,
    pub via: AskVia,
    pub at: i64,
    /// When it goes to its author: 10 seconds after `at`, the time to take it back (§ 8.6). None
    /// once it went.
    pub send_at: Option<i64>,
    /// When it went (its message, or kept for `get_ask` when no message is sent).
    pub sent_at: Option<i64>,
}

/// A line of a request's history (§ 3.5): what happened, and when.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AskEvent {
    pub at: i64,
    /// deposited, updated, snoozed, woke, delegated, answered, reopened, sent, read, withdrawn,
    /// expired, retired, filesErased.
    pub what: String,
    /// What it says of it: who, what changed, until when (never a text of the request).
    pub detail: String,
}

/// An asynchronous request of an agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Ask {
    /// `a-<n>`.
    pub id: String,
    pub project_id: String,
    /// The ticket it is about (its author's, for the agent of a ticket), by id.
    pub ticket_id: Option<String>,
    /// That ticket was under way when it was left: sent back to « À faire », it takes the request out.
    pub ticket_started: bool,
    pub author: AskAuthor,
    pub kind: AskKind,
    pub title: String,
    pub body: String,
    pub options: Vec<AskOption>,
    pub multi: bool,
    pub steps: Vec<String>,
    pub recommend: Option<AskRecommend>,
    pub unblocks: String,
    /// The tickets that wait for its answer, by id (§ 3.6): the autopilot leaves them until then.
    pub tickets: Vec<String>,
    pub urgency: Urgency,
    pub expires_at: i64,
    /// The answer settles a decision of the product: its author is told to propose it.
    pub decision: bool,
    /// The answer wakes its author at rest (a message); false: it reads it by `get_ask`.
    pub wake: bool,
    pub yes_label: String,
    pub no_label: String,
    /// A go-ahead (publish, sign, delete, spend): the user's alone, never left to an agent.
    pub gate: bool,
    pub files: Vec<AskFile>,
    pub state: AskState,
    pub created_at: i64,
    pub updated_at: i64,
    pub snoozed_until: Option<i64>,
    /// It came back from « Plus tard » and was not looked at since.
    pub woke: bool,
    /// The agent the user left it to (§ 8.5), by id.
    pub delegated_to: Option<String>,
    pub answer: Option<AskAnswer>,
    pub read_at: Option<i64>,
    /// When it was withdrawn, expired, retired, or its answer read.
    pub closed_at: Option<i64>,
    /// Why it was withdrawn or retired, as the box says it once.
    pub reason: Option<String>,
    /// Its files were erased (30 days after it closed).
    pub files_erased_at: Option<i64>,
    /// The user saw it in the box since it was left or changed.
    pub seen: bool,
    pub history: Vec<AskEvent>,
}

impl Ask {
    /// The bytes its files weigh, while they are kept.
    pub fn weight(&self) -> u64 {
        if self.files_erased_at.is_some() {
            return 0;
        }
        self.files.iter().map(|f| f.size).sum()
    }
}
