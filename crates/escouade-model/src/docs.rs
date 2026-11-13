//! The data of the spec and the decisions that the agents, the tickets and the discussions keep (`docs`): how a
//! decision is cited, a page, a gap, a decision. The app's `docs` reads and writes them in the docs branch.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

/// A name as a path segment: accents taken off, lower case, other characters a `-`.
pub fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.nfd().filter(|c| !('\u{300}'..='\u{36f}').contains(c)) {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_string();
    out.chars()
        .take(60)
        .collect::<String>()
        .trim_end_matches('-')
        .to_string()
}

/// How a decision is cited (ESC-303): an accepted one by its number, a proposal by its id (the
/// slug of its title), as it has no number yet.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum Ref {
    Number(u32),
    Id(String),
}

impl Ref {
    /// From what a page, a tool or the window gives: digits (« 0004 », « #4 ») a number, else an id.
    pub fn of(s: &str) -> Option<Ref> {
        let s = s.trim();
        let n = s.trim_start_matches('#');
        if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) {
            return n.parse().ok().map(Ref::Number);
        }
        Some(slug(s)).filter(|s| !s.is_empty()).map(Ref::Id)
    }

    /// Itself made plain: a number given as a string is a number, an id a slug.
    pub fn plain(&self) -> Option<Ref> {
        match self {
            Ref::Number(n) => Some(Ref::Number(*n)),
            Ref::Id(s) => Ref::of(s),
        }
    }
}

impl std::fmt::Display for Ref {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ref::Number(n) => write!(f, "{n:04}"),
            Ref::Id(s) => f.write_str(s),
        }
    }
}

/// A page of the spec.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    /// `subject/page`, without `.md`.
    pub path: String,
    pub title: String,
    pub subject: String,
    /// Where it comes from: documents, files, links.
    pub sources: Vec<String>,
    /// The keys of the tickets that implement or changed it.
    pub tickets: Vec<String>,
    /// The decisions it follows: the numbers of those accepted, the ids of the proposals.
    pub decisions: Vec<Ref>,
    /// When it was last compared to the code (ISO 8601); empty: never.
    pub compared: String,
    /// What the last comparison found the code does otherwise (ESC-258): written by the
    /// comparison, shown on top of the page. Left out of the header when there is none.
    pub gaps: Vec<Gap>,
    /// A draft: not settled yet (a brainstorm's, a page begun). Left out of the header when not.
    pub draft: bool,
    /// Its place among the pages of its subject, then the subject's among the others (an import
    /// keeps the order of the spec it came from); 0: by name, after those placed.
    pub order: u32,
    pub body: String,
}

/// A gap between a page and the code (ESC-258): what the code does otherwise, and where.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Gap {
    /// What differs, in words (« La spec dit 3 fois au plus ; le code accepte 4 échéances »).
    pub text: String,
    /// The files of the code concerned, relative to the project, with `/`.
    pub code: Vec<String>,
}

/// The lines added and removed going from `old` to `new` (lines compared as a set of counts, not
/// by position: enough to say « +3 −1 » of a page).
pub fn line_delta(old: &str, new: &str) -> (u32, u32) {
    let mut count: HashMap<&str, i64> = HashMap::new();
    // Blank lines are not counted: they are layout.
    for l in old.lines().filter(|l| !l.trim().is_empty()) {
        *count.entry(l).or_default() -= 1;
    }
    for l in new.lines().filter(|l| !l.trim().is_empty()) {
        *count.entry(l).or_default() += 1;
    }
    let added = count.values().filter(|n| **n > 0).sum::<i64>();
    let removed = -count.values().filter(|n| **n < 0).sum::<i64>();
    (added as u32, removed as u32)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    #[default]
    Proposed,
    Accepted,
    Superseded,
}

impl State {
    pub fn name(self) -> &'static str {
        match self {
            State::Proposed => "proposed",
            State::Accepted => "accepted",
            State::Superseded => "superseded",
        }
    }

    pub fn parse(s: &str) -> Option<State> {
        Some(match s.trim().to_lowercase().as_str() {
            "proposed" | "proposée" | "proposee" => State::Proposed,
            "accepted" | "acceptée" | "acceptee" => State::Accepted,
            "superseded" | "remplacée" | "remplacee" => State::Superseded,
            _ => return None,
        })
    }
}

/// What a decision changed in a page of the spec.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub page: String,
    pub added: u32,
    pub removed: u32,
}

/// Read from the window with only the fields it sends (« Proposer une décision »).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Decision {
    /// Given when it is accepted (ESC-303): a proposal has none (but those written before).
    pub number: Option<u32>,
    /// How a proposal is cited, and the end of its file's name: the slug of its title, made
    /// unique in the register when it is proposed; kept once accepted.
    pub id: String,
    pub title: String,
    /// `YYYY-MM-DD`.
    pub date: String,
    pub state: State,
    /// Where it comes from: brainstorm, discussion, ticket or document; empty when unknown.
    pub origin: String,
    /// The ticket's key, the discussion's or the document's name.
    pub origin_ref: String,
    pub pages: Vec<String>,
    pub tickets: Vec<String>,
    pub documents: Vec<String>,
    /// The numbers of the decisions it replaces, once accepted.
    pub supersedes: Vec<u32>,
    pub superseded_by: Option<u32>,
    pub changes: Vec<Change>,
    /// When and by whom a human accepted it.
    pub accepted_at: String,
    pub accepted_by: String,
    pub context: String,
    pub decision: String,
    pub consequences: String,
}

impl Decision {
    /// The name of its file: `<id>.md` for a proposal, `NNNN-<id>.md` once it has a number.
    pub fn file_name(&self) -> String {
        match (self.number, self.id.is_empty()) {
            (Some(n), true) => format!("{n:04}.md"),
            (Some(n), false) => format!("{n:04}-{}.md", self.id),
            (None, _) => format!("{}.md", self.id),
        }
    }

    /// How it is cited: its number when it has one, else its id.
    pub fn reference(&self) -> Ref {
        match self.number {
            Some(n) => Ref::Number(n),
            None => Ref::Id(self.id.clone()),
        }
    }

    /// Whether `r` cites it: its number, or its id.
    pub fn is(&self, r: &Ref) -> bool {
        match r {
            Ref::Number(n) => self.number == Some(*n),
            Ref::Id(s) => !self.id.is_empty() && self.id == *s,
        }
    }
}
