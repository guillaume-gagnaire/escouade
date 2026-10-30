//! The product's docs, in the repository (ESC-256, `docs/FACTORY.md` § 3): everything under
//! `.escouade/` of the branch of the docs, as plain files the team shares by git (Escouade itself
//! never commits nor pushes them):
//!
//! - `spec/<subject>/<page>.md`: the specification, a Markdown page with a header (title,
//!   subject, sources, linked tickets and decisions, when it was last compared to the code);
//! - `decisions/<NNNN>-<slug>.md`: the register, a file per decision (number, date, state,
//!   context / decision / consequences, what it changed in the spec, its links and what it
//!   replaces). An accepted decision is never rewritten: a new one replaces it. A proposal has no
//!   number yet (ESC-303): its file is `<id>.md`, cited by that id (the slug of its title) until
//!   it is accepted, when it takes the number after the register of the branch it goes to;
//! - `documents/<folders>/<file>`: the files the team dropped.
//!
//! What can be worked out again lives in Escouade's own folder, beside the histories: the text
//! extracted from the documents (`Space::text_dir`), and what the comparison to the code found of
//! each page, its time and its gaps (`Space::checks`, ESC-301): kept there, never in the page, so
//! that a comparison leaves git nothing to commit and the next validation is not refused.
//!
//! `Space` is the files of one branch of the docs, without the core: every path it reads or
//! writes goes through `Space::dir` / `Space::file`, which refuse a link on the way and anything
//! that leaves the project (`workflowfiles::unsafe_dir`, as the shared tickets do). The core finds
//! the space (`Core::docs_space`): the branch the caller works on, or the fixed branch of the
//! project in a worktree of its own, which Escouade never merges anywhere.
//!
//! The headers are one `key: value` per line, a value being JSON (a string, a list, a number),
//! which YAML reads: they read back as they were written (`front_write` / `front_read`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

use crate::workflowfiles::{reserved_stem, unsafe_dir};

pub const ROOT: &str = ".escouade";
pub const SPEC: &str = "spec";
pub const DECISIONS: &str = "decisions";
pub const DOCUMENTS: &str = "documents";

/// The deepest a page's path goes (`subject/sub-subject/page`).
const MAX_DEPTH: usize = 4;
/// The deepest a document's folder goes.
const MAX_FOLDER_DEPTH: usize = 6;
/// A text page, a decision: the most Escouade reads of one file.
const MAX_TEXT: u64 = 2 * 1024 * 1024;
/// The most files a folder is listed with.
const MAX_FILES: usize = 5000;
/// What a joined file may weigh before the window warns (FACTORY.md: 50 MB).
pub const LARGE_FILE: u64 = 50 * 1024 * 1024;
/// The most characters of a document's text one reading gives.
pub const TEXT_CHUNK: usize = 100_000;

// ---------- names ----------

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

/// A page's path as it is kept: `subject/page`, each segment a `slug`, a trailing `.md` dropped.
/// None when it is no path (empty, absolute, `..`, a drive, too deep, a name Windows keeps).
pub fn page_path(rel: &str) -> Option<String> {
    let rel = rel.trim();
    let rel = rel.strip_suffix(".md").unwrap_or(rel);
    if rel.is_empty() || rel.contains(['\\', ':']) || rel.starts_with('/') {
        return None;
    }
    let mut out = Vec::new();
    for seg in rel.split('/') {
        if seg == "." || seg == ".." {
            return None;
        }
        let s = slug(seg);
        if s.is_empty() || reserved_stem(&s) {
            return None;
        }
        out.push(s);
    }
    (out.len() <= MAX_DEPTH).then(|| out.join("/"))
}

/// A path inside `documents/`: segments `safe_name` leaves as they are, no way up.
pub fn document_path(rel: &str) -> Option<String> {
    let rel = rel.trim();
    if rel.starts_with('/') {
        return None;
    }
    let rel = rel.trim_end_matches('/');
    if rel.is_empty() || rel.contains(['\\', ':']) {
        return None;
    }
    let segs: Vec<&str> = rel.split('/').collect();
    if segs.len() > MAX_FOLDER_DEPTH + 1 {
        return None;
    }
    segs.iter()
        .all(|s| !s.is_empty() && *s != "." && *s != ".." && crate::attachfiles::safe_name(s) == *s)
        .then(|| segs.join("/"))
}

// ---------- headers ----------

fn front_value<T: Serialize + ?Sized>(v: &T) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "null".into())
}

/// `---`, a `key: value` line each, `---`, a blank line, the body.
fn front_write(lines: &[(&str, String)], body: &str) -> String {
    let mut out = String::from("---\n");
    for (k, v) in lines {
        out.push_str(&format!("{k}: {v}\n"));
    }
    out.push_str("---\n");
    let body = body.trim_end_matches(['\n', '\r']);
    if !body.is_empty() {
        out.push('\n');
        out.push_str(body);
        out.push('\n');
    }
    out
}

/// A file's header and body. None when it has no header.
fn front_read(text: &str) -> Option<(HashMap<String, serde_json::Value>, String)> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.split_inclusive('\n');
    if lines.next()?.trim_end() != "---" {
        return None;
    }
    let mut head = HashMap::new();
    let mut used = 4;
    let mut closed = false;
    for l in lines {
        used += l.len();
        if l.trim_end() == "---" {
            closed = true;
            break;
        }
        if let Some((k, v)) = l.split_once(':') {
            let v = v.trim();
            let value = serde_json::from_str(v)
                .unwrap_or_else(|_| serde_json::Value::String(v.trim_matches(['"', '\'']).into()));
            head.insert(k.trim().to_string(), value);
        }
    }
    if !closed {
        return None;
    }
    let body = text.get(used..).unwrap_or_default();
    let body = body.strip_prefix('\n').unwrap_or(body);
    Some((head, body.trim_end_matches(['\n', '\r']).to_string()))
}

fn str_of(h: &HashMap<String, serde_json::Value>, k: &str) -> String {
    match h.get(k) {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn strs_of(h: &HashMap<String, serde_json::Value>, k: &str) -> Vec<String> {
    match h.get(k) {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(|i| match i {
                serde_json::Value::String(s) => Some(s.clone()),
                serde_json::Value::Number(n) => Some(n.to_string()),
                _ => None,
            })
            .collect(),
        Some(serde_json::Value::String(s)) if !s.is_empty() => vec![s.clone()],
        _ => Vec::new(),
    }
}

fn nums_of(h: &HashMap<String, serde_json::Value>, k: &str) -> Vec<u32> {
    strs_of(h, k)
        .iter()
        .filter_map(|s| s.trim_start_matches('#').parse().ok())
        .collect()
}

fn refs_of(h: &HashMap<String, serde_json::Value>, k: &str) -> Vec<Ref> {
    strs_of(h, k).iter().filter_map(|s| Ref::of(s)).collect()
}

/// How a decision is cited (ESC-303): an accepted one by its number, a proposal by its id (the
/// slug of its title), as it has no number yet.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
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

// ---------- the specification ----------

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

/// What the comparison to the code found of a page (ESC-301), kept out of the page.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Check {
    pub compared: String,
    pub gaps: Vec<Gap>,
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

pub fn page_write(p: &Page) -> String {
    let compared = if p.compared.is_empty() {
        "null".to_string()
    } else {
        front_value(&p.compared)
    };
    let mut lines = vec![
        ("title", front_value(&p.title)),
        ("subject", front_value(&p.subject)),
        ("sources", front_value(&p.sources)),
        ("tickets", front_value(&p.tickets)),
        ("decisions", front_value(&p.decisions)),
        ("compared", compared),
    ];
    // Written only when set: a page without them reads as it always did.
    if !p.gaps.is_empty() {
        lines.push(("gaps", front_value(&p.gaps)));
    }
    if p.draft {
        lines.push(("draft", "true".into()));
    }
    if p.order > 0 {
        lines.push(("order", p.order.to_string()));
    }
    front_write(&lines, &p.body)
}

/// A page from its file's text. A file without a header (written by hand) is a page too: its
/// title is its first `# ` heading, else its name; its subject its folder.
pub fn page_read(path: &str, text: &str) -> Page {
    let subject_of = |path: &str| {
        path.split('/')
            .next()
            .filter(|_| path.contains('/'))
            .unwrap_or("general")
            .to_string()
    };
    let name = path.rsplit('/').next().unwrap_or(path).to_string();
    match front_read(text) {
        Some((h, body)) => {
            let title = Some(str_of(&h, "title"))
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| heading(&body).unwrap_or(name));
            let subject = Some(str_of(&h, "subject"))
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| subject_of(path));
            Page {
                path: path.into(),
                title,
                subject,
                sources: strs_of(&h, "sources"),
                tickets: strs_of(&h, "tickets"),
                decisions: refs_of(&h, "decisions"),
                compared: str_of(&h, "compared"),
                gaps: h
                    .get("gaps")
                    .and_then(|v| serde_json::from_value::<Vec<Gap>>(v.clone()).ok())
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|g| !g.text.trim().is_empty())
                    .collect(),
                draft: matches!(h.get("draft"), Some(serde_json::Value::Bool(true))),
                order: h
                    .get("order")
                    .and_then(|v| v.as_u64())
                    .map_or(0, |n| n.min(u32::MAX as u64) as u32),
                body,
            }
        }
        None => {
            let body = text.trim_end_matches(['\n', '\r']).to_string();
            Page {
                path: path.into(),
                title: heading(&body).unwrap_or(name),
                subject: subject_of(path),
                body,
                ..Default::default()
            }
        }
    }
}

fn heading(body: &str) -> Option<String> {
    body.lines()
        .find_map(|l| l.strip_prefix("# "))
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
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

/// A page as the tree lists it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub path: String,
    pub title: String,
    pub compared: String,
    pub tickets: Vec<String>,
    pub decisions: Vec<Ref>,
    /// How many gaps with the code its last comparison found.
    pub gaps: u32,
    pub draft: bool,
}

/// The pages of a subject.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Subject {
    /// Its folder under `spec/`.
    pub name: String,
    /// The name it is shown under: the `subject` its pages give, else its folder.
    pub title: String,
    pub pages: Vec<PageInfo>,
}

/// What a search finds in a page.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub path: String,
    pub title: String,
    /// The lines that hold the words, the first few.
    pub lines: Vec<String>,
}

// ---------- the decisions ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    #[default]
    Proposed,
    Accepted,
    Superseded,
}

impl State {
    fn name(self) -> &'static str {
        match self {
            State::Proposed => "proposed",
            State::Accepted => "accepted",
            State::Superseded => "superseded",
        }
    }

    fn parse(s: &str) -> Option<State> {
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

/// The register of the branch a decision goes to, as its acceptance looks at it (ESC-303): the
/// next number there, and the numbers it has, each with its id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Register {
    pub next: u32,
    pub taken: Vec<(u32, String)>,
}

impl Register {
    /// From the names of its files (`0004-un-titre.md`).
    pub fn of_names<'a>(names: impl IntoIterator<Item = &'a str>) -> Register {
        let taken: Vec<(u32, String)> = names
            .into_iter()
            .filter(|n| n.ends_with(".md"))
            .filter_map(|n| {
                let digits: String = n.chars().take_while(char::is_ascii_digit).collect();
                Some((digits.parse().ok()?, id_of_file(Path::new(n))))
            })
            .collect();
        Register {
            next: taken.iter().map(|(n, _)| n + 1).max().unwrap_or(1),
            taken,
        }
    }

    /// Both: the next after each, every number each has.
    pub fn and(mut self, other: Register) -> Register {
        self.next = self.next.max(other.next);
        self.taken.extend(other.taken);
        self
    }
}

/// The id a title gives: its slug, 40 characters at most.
pub fn decision_id(title: &str) -> String {
    let s: String = slug(title).chars().take(40).collect();
    let s = s.trim_end_matches('-');
    if s.is_empty() {
        "decision".into()
    } else if s.chars().all(|c| c.is_ascii_digit()) {
        // Digits alone would read as a number.
        format!("d-{s}")
    } else {
        s.into()
    }
}

/// The id a file's name gives: its name without `.md` nor its number (`0004-un-titre.md` →
/// `un-titre`).
pub(crate) fn id_of_file(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let rest = stem.trim_start_matches(|c: char| c.is_ascii_digit());
    match rest.strip_prefix('-') {
        Some(r) if rest.len() < stem.len() && !r.is_empty() => r.to_string(),
        _ if rest.len() < stem.len() && rest.is_empty() => format!("d-{stem}"),
        _ => stem,
    }
}

/// The three parts of a decision, titled as the interface's language says; both languages read.
fn parts() -> [String; 3] {
    [
        tr!("Contexte", "Context"),
        tr!("Décision", "Decision"),
        tr!("Conséquences", "Consequences"),
    ]
}

pub fn decision_write(d: &Decision) -> String {
    let superseded_by = d
        .superseded_by
        .map_or("null".to_string(), |n| n.to_string());
    let body = {
        let [c, dd, cons] = parts();
        format!(
            "## {c}\n\n{}\n\n## {dd}\n\n{}\n\n## {cons}\n\n{}",
            d.context.trim(),
            d.decision.trim(),
            d.consequences.trim()
        )
    };
    front_write(
        &[
            (
                "number",
                d.number.map_or("null".to_string(), |n| n.to_string()),
            ),
            ("id", front_value(&d.id)),
            ("title", front_value(&d.title)),
            ("date", front_value(&d.date)),
            ("state", front_value(d.state.name())),
            ("origin", front_value(&d.origin)),
            ("origin_ref", front_value(&d.origin_ref)),
            ("pages", front_value(&d.pages)),
            ("tickets", front_value(&d.tickets)),
            ("documents", front_value(&d.documents)),
            ("supersedes", front_value(&d.supersedes)),
            ("superseded_by", superseded_by),
            ("changes", front_value(&d.changes)),
            ("accepted_at", front_value(&d.accepted_at)),
            ("accepted_by", front_value(&d.accepted_by)),
        ],
        &body,
    )
}

/// The text under each of the headings `## …` of a decision's body.
fn sections(body: &str) -> HashMap<String, String> {
    let mut out: HashMap<String, String> = HashMap::new();
    let mut at: Option<String> = None;
    for l in body.lines() {
        if let Some(h) = l.strip_prefix("## ") {
            at = Some(h.trim().to_lowercase());
            out.entry(h.trim().to_lowercase()).or_default();
        } else if let Some(k) = &at {
            let e = out.get_mut(k).unwrap();
            e.push_str(l);
            e.push('\n');
        }
    }
    out.values_mut().for_each(|v| *v = v.trim().to_string());
    out
}

pub fn decision_read(text: &str) -> Result<Decision> {
    let Some((h, body)) = front_read(text) else {
        bail!(tr!(
            "la décision n’a pas d’en-tête (--- … ---)",
            "the decision has no header (--- … ---)"
        ));
    };
    // A proposal has no number (ESC-303); one that does not read is refused.
    let number = match h.get("number") {
        None | Some(serde_json::Value::Null) => None,
        Some(_) => Some(str_of(&h, "number").parse::<u32>().map_err(|_| {
            anyhow::anyhow!(tr!(
                "le numéro de la décision ne se lit pas",
                "the decision's number does not read"
            ))
        })?),
    };
    let secs = sections(&body);
    let pick = |names: &[&str]| -> String {
        names
            .iter()
            .find_map(|n| secs.get(*n).cloned())
            .unwrap_or_default()
    };
    let changes = match h.get("changes") {
        Some(v @ serde_json::Value::Array(_)) => {
            serde_json::from_value(v.clone()).unwrap_or_default()
        }
        _ => Vec::new(),
    };
    Ok(Decision {
        number,
        id: str_of(&h, "id"),
        title: str_of(&h, "title"),
        date: str_of(&h, "date"),
        state: State::parse(&str_of(&h, "state")).unwrap_or_default(),
        origin: str_of(&h, "origin"),
        origin_ref: str_of(&h, "origin_ref"),
        pages: strs_of(&h, "pages"),
        tickets: strs_of(&h, "tickets"),
        documents: strs_of(&h, "documents"),
        supersedes: nums_of(&h, "supersedes"),
        superseded_by: str_of(&h, "superseded_by").parse().ok(),
        changes,
        accepted_at: str_of(&h, "accepted_at"),
        accepted_by: str_of(&h, "accepted_by"),
        context: pick(&["contexte", "context"]),
        decision: pick(&["décision", "decision"]),
        consequences: pick(&["conséquences", "consequences"]),
    })
}

/// Whether going from `old` to `new` rewrites an accepted decision. Accepting a proposal, and an
/// accepted decision being replaced (state `superseded` and the number that replaces it), are
/// the only changes a decision that was decided can have.
pub fn rewrites(old: &Decision, new: &Decision) -> bool {
    match old.state {
        State::Proposed => false,
        State::Accepted => {
            let mut same = new.clone();
            if new.state == State::Superseded {
                same.state = State::Accepted;
                same.superseded_by = old.superseded_by;
            }
            same != *old
        }
        State::Superseded => new != old,
    }
}

// ---------- the documents ----------

/// A file of the drive.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocInfo {
    /// Under `documents/`.
    pub path: String,
    pub name: String,
    pub folder: String,
    pub size: u64,
    /// text, pdf, word, image or other.
    pub kind: String,
    /// Whether Escouade reads its text (text, PDF, Word, OpenDocument).
    pub readable: bool,
    /// When its file was last changed, in milliseconds since the epoch.
    pub modified: u64,
    /// Who added it last, as git says (the user of this machine when it is not committed yet);
    /// filled by the core.
    pub author: String,
    /// How many times the agents read it (`read_document`); filled by the core.
    pub reads: u32,
    /// How many pages of the spec and decisions cite it; filled by the core.
    pub cited: u32,
}

/// A folder of the drive.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    /// Under `documents/`, with `/`.
    pub path: String,
    pub name: String,
    /// The files in it and in its folders.
    pub files: usize,
}

pub fn kind_of(name: &str) -> &'static str {
    match ext(name).as_str() {
        "md" | "markdown" | "txt" | "text" | "csv" | "tsv" | "json" | "yaml" | "yml" | "xml"
        | "html" | "htm" | "rst" | "log" | "toml" | "ini" => "text",
        "pdf" => "pdf",
        "doc" | "docx" | "odt" | "rtf" => "word",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" => "image",
        _ => "other",
    }
}

fn ext(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, e)| e.to_lowercase())
        .unwrap_or_default()
}

/// A document's text, a chunk of it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocText {
    pub path: String,
    pub kind: String,
    /// None: Escouade does not extract the text of this format (yet).
    pub text: Option<String>,
    /// The characters of the whole text.
    pub total: usize,
    pub from: usize,
    pub truncated: bool,
    /// What is known of a file whose content is not read (an image: its format and size).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<String>,
}

// ---------- a spec found in the project ----------

/// A spec the project already has, in its own folders, that could be imported.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    /// Relative to the project, with `/`.
    pub path: String,
    /// file or folder.
    pub kind: String,
    /// The Markdown files it holds (1 for a file).
    pub files: usize,
    pub bytes: u64,
}

// ---------- the space ----------

/// The docs of one branch: the folder that holds its `.escouade/`.
#[derive(Debug, Clone)]
pub struct Space {
    pub root: PathBuf,
    /// Where the text extracted from the documents is kept (Escouade's folder); None: not kept.
    pub text_dir: Option<PathBuf>,
    /// The folder that holds the `documents/` of this space when they live elsewhere (ESC-263: the
    /// copy a ticket's agent writes its spec in, on a fixed branch, reads and adds documents on
    /// that branch); None: under `root`.
    pub documents_root: Option<PathBuf>,
    /// Where its pages and decisions are told written (PLG-5), with its project's id; None for
    /// a space the core did not hand out.
    pub facts: Option<(crate::facts::Emitter, String)>,
    /// The file of Escouade's folder that keeps what the comparison found of each page, by path
    /// (ESC-301): read over the page's own header, and where its time and gaps are written. None:
    /// in the page's header, as before (a space the core did not hand out).
    pub checks: Option<PathBuf>,
}

fn is_link(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink())
}

fn refuse_link(p: &Path) -> anyhow::Error {
    anyhow::anyhow!(tr!(
        "{p} est un lien symbolique : ignoré",
        "{p} is a symbolic link: ignored",
        p = p.display()
    ))
}

/// The files of a folder, depth first, sorted: relative paths with `/`. Links are skipped, never
/// followed (a link can point anywhere).
fn walk(base: &Path, ext_ok: impl Fn(&str) -> bool, limit: usize) -> Vec<(String, u64)> {
    fn go(
        base: &Path,
        dir: &Path,
        ext_ok: &dyn Fn(&str) -> bool,
        out: &mut Vec<(String, u64)>,
        limit: usize,
        depth: usize,
    ) {
        if depth > MAX_FOLDER_DEPTH + 1 || out.len() >= limit {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            if out.len() >= limit {
                return;
            }
            let Ok(t) = e.file_type() else { continue };
            let name = e.file_name().to_string_lossy().to_string();
            if t.is_symlink() || name.starts_with('.') {
                continue;
            }
            if t.is_dir() {
                go(base, &e.path(), ext_ok, out, limit, depth + 1);
            } else if t.is_file() && ext_ok(&name) {
                let rel = e
                    .path()
                    .strip_prefix(base)
                    .map(|r| r.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default();
                let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                out.push((rel, size));
            }
        }
    }
    let mut out = Vec::new();
    go(base, base, &ext_ok, &mut out, limit, 0);
    out
}

fn md_name(n: &str) -> bool {
    n.to_lowercase().ends_with(".md")
}

impl Space {
    pub fn new(root: impl Into<PathBuf>, text_dir: Option<PathBuf>) -> Self {
        Self {
            root: root.into(),
            text_dir,
            documents_root: None,
            facts: None,
            checks: None,
        }
    }

    /// What `checks` keeps, by page; empty when there is none or it does not read.
    pub(crate) fn checks(&self) -> HashMap<String, Check> {
        self.checks
            .as_ref()
            .and_then(|f| std::fs::read(f).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// `page` with what the comparison found of it kept in `checks`, over its header.
    fn checked(&self, mut page: Page, checks: &HashMap<String, Check>) -> Page {
        if let Some(c) = checks.get(&page.path) {
            page.compared = c.compared.clone();
            page.gaps = c.gaps.clone();
        }
        page
    }

    /// Keeps what the comparison found of the page `rel` (its time and gaps): in `checks` when
    /// the space has it, the page left as it is; else in the page's header.
    pub fn spec_check(&self, rel: &str, compared: String, gaps: Vec<Gap>) -> Result<()> {
        let Some(file) = &self.checks else {
            self.spec_write(rel, |p| {
                p.gaps = gaps;
                p.compared = compared;
            })?;
            return Ok(());
        };
        let path = page_path(rel).ok_or_else(|| bad_page(rel))?;
        let mut all = self.checks();
        all.insert(path, Check { compared, gaps });
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = file.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&all)?)?;
        std::fs::rename(&tmp, file)?;
        Ok(())
    }

    /// Tells `body` of this space's project, when the core handed it out.
    fn fact(&self, body: crate::facts::Body) {
        if let Some((e, project)) = &self.facts {
            e.emit(body, crate::facts::About::project(project));
        }
    }

    /// `.escouade/<sub>`, refused when it (or `.escouade`) is a link or leaves the project.
    pub fn dir(&self, sub: &str) -> Result<PathBuf> {
        let root = match &self.documents_root {
            Some(other) if sub == DOCUMENTS => other,
            _ => &self.root,
        };
        let dir = root.join(ROOT).join(sub);
        if let Some(why) = unsafe_dir(&dir) {
            bail!(why);
        }
        Ok(dir)
    }

    /// `rel` under `.escouade/<sub>`, with no link on the way (the file itself included when it
    /// is there). `rel` is a path `page_path` or `document_path` accepted.
    fn file(&self, sub: &str, rel: &str) -> Result<PathBuf> {
        let base = self.dir(sub)?;
        let mut at = base.clone();
        for seg in rel.split('/') {
            at.push(seg);
            if is_link(&at) {
                return Err(refuse_link(&at));
            }
        }
        // Whatever the pieces were, the whole stays under the folder.
        if let Ok(real) = at.parent().map(Path::canonicalize).transpose() {
            if let (Some(real), Ok(b)) = (real, base.canonicalize()) {
                if !real.starts_with(&b) {
                    return Err(crate::paths::outside(rel));
                }
            }
        }
        Ok(at)
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Made now, the folders are checked again: none of them may be a link.
        let mut at = path.parent();
        while let Some(p) = at {
            if p.ends_with(ROOT) {
                break;
            }
            if is_link(p) {
                return Err(refuse_link(p));
            }
            at = p.parent();
        }
        crate::paths::write_atomic(path, bytes)?;
        Ok(())
    }

    fn read_text(&self, path: &Path) -> Result<String> {
        if is_link(path) {
            return Err(refuse_link(path));
        }
        let meta = std::fs::metadata(path)?;
        if meta.len() > MAX_TEXT {
            bail!(tr!(
                "{p} est trop gros pour être lu comme une page",
                "{p} is too big to be read as a page",
                p = path.display()
            ));
        }
        Ok(String::from_utf8_lossy(&std::fs::read(path)?).into_owned())
    }

    // ----- the files as they are (ESC-263: a ticket's copy, poured at its validation) -----

    /// The Markdown files under `.escouade/<sub>` (the spec or the decisions): relative, with `/`.
    pub(crate) fn md_files(&self, sub: &str) -> Vec<String> {
        let Ok(dir) = self.dir(sub) else {
            return Vec::new();
        };
        walk(&dir, md_name, MAX_FILES)
            .into_iter()
            .map(|(rel, _)| rel)
            .collect()
    }

    pub(crate) fn raw_read(&self, sub: &str, rel: &str) -> Result<String> {
        let file = self.file(sub, rel)?;
        self.read_text(&file)
    }

    pub(crate) fn raw_write(&self, sub: &str, rel: &str, text: &str) -> Result<()> {
        let file = self.file(sub, rel)?;
        self.write(&file, text.as_bytes())
    }

    pub(crate) fn raw_remove(&self, sub: &str, rel: &str) -> Result<()> {
        let file = self.file(sub, rel)?;
        if file.is_file() {
            std::fs::remove_file(file)?;
        }
        Ok(())
    }

    // ----- spec -----

    pub(crate) fn pages(&self) -> Vec<Page> {
        let Ok(dir) = self.dir(SPEC) else {
            return Vec::new();
        };
        let checks = self.checks();
        walk(&dir, md_name, MAX_FILES)
            .into_iter()
            .filter_map(|(rel, _)| {
                let path = rel.strip_suffix(".md")?.to_string();
                let text = self.read_text(&dir.join(&rel)).ok()?;
                Some(self.checked(page_read(&path, &text), &checks))
            })
            .collect()
    }

    /// The pages by subject (the first folder of their path), in order.
    /// Pages and subjects placed by their `order` first (an import keeps its spec's), then by name.
    pub fn spec_tree(&self) -> Vec<Subject> {
        // Each subject with the smallest order of its pages (0 counts as unplaced).
        let mut subjects: Vec<(u32, Subject, Vec<u32>)> = Vec::new();
        for p in self.pages() {
            let name = if p.path.contains('/') {
                p.path.split('/').next().unwrap_or_default().to_string()
            } else {
                p.subject.clone()
            };
            let rank = if p.order == 0 { u32::MAX } else { p.order };
            let title = p.subject.trim().to_string();
            let info = PageInfo {
                path: p.path,
                title: p.title,
                compared: p.compared,
                tickets: p.tickets,
                decisions: p.decisions,
                gaps: p.gaps.len() as u32,
                draft: p.draft,
            };
            match subjects.iter_mut().find(|(_, s, _)| s.name == name) {
                Some((r, s, ranks)) => {
                    *r = (*r).min(rank);
                    if s.title == s.name && !title.is_empty() {
                        s.title = title;
                    }
                    s.pages.push(info);
                    ranks.push(rank);
                }
                None => subjects.push((
                    rank,
                    Subject {
                        title: if title.is_empty() {
                            name.clone()
                        } else {
                            title
                        },
                        name,
                        pages: vec![info],
                    },
                    vec![rank],
                )),
            }
        }
        subjects.sort_by(|a, b| (a.0, &a.1.name).cmp(&(b.0, &b.1.name)));
        subjects
            .into_iter()
            .map(|(_, mut s, ranks)| {
                let mut both: Vec<(u32, PageInfo)> = ranks.into_iter().zip(s.pages).collect();
                both.sort_by(|a, b| (a.0, &a.1.path).cmp(&(b.0, &b.1.path)));
                s.pages = both.into_iter().map(|(_, p)| p).collect();
                s
            })
            .collect()
    }

    pub fn spec_read(&self, rel: &str) -> Result<Page> {
        let path = page_path(rel).ok_or_else(|| bad_page(rel))?;
        let file = self.file(SPEC, &format!("{path}.md"))?;
        if !file.is_file() {
            bail!(tr!(
                "Aucune page « {path} » dans la spécification.",
                "No page “{path}” in the specification."
            ));
        }
        Ok(self.checked(page_read(&path, &self.read_text(&file)?), &self.checks()))
    }

    /// Creates or changes a page: what `edit` leaves of the page as it was (an empty one for a new
    /// page) is written. Returns the page and the lines it added and removed.
    /// With `checks`, the time and gaps `edit` changes go there (a discussion settles a gap) and
    /// the page's header keeps its own.
    pub fn spec_write(&self, rel: &str, edit: impl FnOnce(&mut Page)) -> Result<(Page, u32, u32)> {
        let (page, added, removed) = self.spec_edit(rel, None, edit)?;
        let file = self.file(SPEC, &format!("{}.md", page.path))?;
        let mut kept = page.clone();
        if self.checks.is_some() {
            let own = match file.is_file() {
                true => page_read(&page.path, &self.read_text(&file)?),
                false => Page {
                    path: page.path.clone(),
                    ..Default::default()
                },
            };
            let before = self.checked(own.clone(), &self.checks());
            if (&before.compared, &before.gaps) != (&page.compared, &page.gaps) {
                self.spec_check(&page.path, page.compared.clone(), page.gaps.clone())?;
            }
            kept.compared = own.compared;
            kept.gaps = own.gaps;
        }
        self.write(&file, page_write(&kept).as_bytes())?;
        self.fact(crate::facts::Body::DocsWritten {
            page: page.path.clone(),
        });
        Ok((page, added, removed))
    }

    /// What `spec_write` would write, without writing it: the page `edit` leaves of `current`
    /// (a discussion's proposal, ESC-261) or of the page as it is, and the lines it adds and
    /// removes from it.
    pub fn spec_edit(
        &self,
        rel: &str,
        current: Option<Page>,
        edit: impl FnOnce(&mut Page),
    ) -> Result<(Page, u32, u32)> {
        let path = page_path(rel).ok_or_else(|| bad_page(rel))?;
        let file = self.file(SPEC, &format!("{path}.md"))?;
        let before = match current {
            Some(p) => Some(p),
            None if file.is_file() => {
                Some(self.checked(page_read(&path, &self.read_text(&file)?), &self.checks()))
            }
            None => None,
        };
        let mut page = before.clone().unwrap_or_else(|| Page {
            path: path.clone(),
            ..Default::default()
        });
        edit(&mut page);
        page.path = path.clone();
        if page.title.trim().is_empty() {
            page.title = heading(&page.body)
                .unwrap_or_else(|| path.rsplit('/').next().unwrap_or(&path).replace('-', " "));
        }
        if page.subject.trim().is_empty() {
            page.subject = path.split('/').next().unwrap_or("general").to_string();
        }
        page.body = page.body.trim_end_matches(['\n', '\r']).to_string();
        let (added, removed) = line_delta(before.as_ref().map_or("", |b| &b.body), &page.body);
        Ok((page, added, removed))
    }

    /// The pages that hold all the words (any case), their titles first.
    pub fn spec_search(&self, query: &str, limit: usize) -> Vec<Hit> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        if words.is_empty() {
            return Vec::new();
        }
        let mut hits: Vec<(usize, Hit)> = Vec::new();
        for p in self.pages() {
            let hay = format!("{}\n{}", p.title, p.body).to_lowercase();
            if !words.iter().all(|w| hay.contains(w.as_str())) {
                continue;
            }
            let in_title = words
                .iter()
                .filter(|w| p.title.to_lowercase().contains(w.as_str()))
                .count();
            let lines: Vec<String> = p
                .body
                .lines()
                .filter(|l| {
                    let l = l.to_lowercase();
                    words.iter().any(|w| l.contains(w.as_str()))
                })
                .take(3)
                .map(|l| l.trim().chars().take(200).collect())
                .collect();
            hits.push((
                in_title,
                Hit {
                    path: p.path,
                    title: p.title,
                    lines,
                },
            ));
        }
        hits.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.path.cmp(&b.1.path)));
        hits.into_iter().map(|(_, h)| h).take(limit).collect()
    }

    // ----- decisions -----

    /// The register's files, each with its decision (None when it does not read), by number,
    /// the proposals after. An id its header leaves out comes from the file's name, its number
    /// taken off (a decision written before ESC-303).
    fn decision_files(&self) -> Vec<(PathBuf, Option<Decision>)> {
        let Ok(dir) = self.dir(DECISIONS) else {
            return Vec::new();
        };
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Vec::new();
        };
        let mut out: Vec<(PathBuf, Option<Decision>)> = entries
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .filter(|e| e.file_name().to_string_lossy().ends_with(".md"))
            .map(|e| {
                let path = e.path();
                let d = self
                    .read_text(&path)
                    .ok()
                    .and_then(|t| decision_read(&t).ok())
                    .map(|mut d| {
                        if d.id.is_empty() {
                            d.id = id_of_file(&path);
                        }
                        d
                    });
                (path, d)
            })
            .collect();
        out.sort_by_key(|(p, d)| {
            let n = d.as_ref().and_then(|d| d.number);
            (n.is_none(), n, p.clone())
        });
        out
    }

    /// Every decision: those numbered by number, then the proposals. A file that cannot be read
    /// is left out.
    pub fn decisions(&self) -> Vec<Decision> {
        self.decision_files()
            .into_iter()
            .filter_map(|(_, d)| d)
            .collect()
    }

    /// The decision `r` cites: by its number, or its id.
    pub fn decision(&self, r: &Ref) -> Result<Decision> {
        let r = r.plain().unwrap_or_else(|| r.clone());
        self.decisions()
            .into_iter()
            .find(|d| d.is(&r))
            .ok_or_else(|| match &r {
                Ref::Number(number) => anyhow::anyhow!(tr!(
                    "Aucune décision n° {number}.",
                    "No decision number {number}."
                )),
                Ref::Id(id) => {
                    anyhow::anyhow!(tr!("Aucune décision « {id} ».", "No decision “{id}”."))
                }
            })
    }

    /// The next number: after the highest one the register holds (a file's name counts too, for
    /// one that does not read).
    pub fn next_number(&self) -> u32 {
        self.decision_files()
            .iter()
            .filter_map(|(p, d)| match d {
                Some(d) => d.number,
                None => {
                    let name = p.file_name()?.to_string_lossy().to_string();
                    let digits: String = name.chars().take_while(char::is_ascii_digit).collect();
                    digits.parse().ok()
                }
            })
            .max()
            .map_or(1, |n| n + 1)
    }

    /// This register as an acceptance on another branch looks at it.
    pub fn register(&self) -> Register {
        Register {
            next: self.next_number(),
            taken: self
                .decisions()
                .into_iter()
                .filter_map(|d| Some((d.number?, d.id)))
                .collect(),
        }
    }

    /// An id for a new proposal titled `title`: its slug, made unique in the register.
    fn free_id(&self, title: &str) -> String {
        let base = decision_id(title);
        let taken: std::collections::HashSet<String> =
            self.decisions().into_iter().map(|d| d.id).collect();
        let mut id = base.clone();
        let mut i = 2;
        while taken.contains(&id) {
            id = format!("{base}-{i}");
            i += 1;
        }
        id
    }

    /// The file of `d`: the one of the same number and id.
    fn file_of(&self, d: &Decision) -> Option<PathBuf> {
        self.decision_files()
            .into_iter()
            .find(|(_, x)| {
                x.as_ref()
                    .is_some_and(|x| x.number == d.number && x.id == d.id)
            })
            .map(|(p, _)| p)
    }

    fn decision_save(&self, d: &Decision, prior: Option<&Decision>) -> Result<()> {
        if let Some(old) = prior {
            if rewrites(old, d) {
                bail!(tr!(
                    "La décision {n} est {state} : elle ne se réécrit pas, une nouvelle la remplace.",
                    "Decision {n} is {state}: it is not rewritten, a new one replaces it.",
                    n = old.reference(),
                    state = match old.state {
                        State::Accepted => tr!("acceptée", "accepted"),
                        _ => tr!("remplacée", "superseded"),
                    }
                ));
            }
        }
        // Its file keeps its name while its number stays (the title of an accepted decision never
        // changes, and a name written before stays as it is); numbered, a proposal's file is
        // renamed.
        let was = prior.and_then(|old| self.file_of(old).map(|f| (old, f)));
        let file = match &was {
            Some((old, f)) if old.number == d.number => f.clone(),
            _ => self.file(DECISIONS, &d.file_name())?,
        };
        self.write(&file, decision_write(d).as_bytes())?;
        if let Some((_, f)) = was {
            if f != file && f.is_file() {
                std::fs::remove_file(&f)?;
            }
        }
        Ok(())
    }

    /// A new proposal, with an id and no number, or the revision of a proposal still open (`r`). A
    /// decision that was decided is not touched.
    pub fn decision_propose(&self, r: Option<&Ref>, mut d: Decision) -> Result<Decision> {
        d.state = State::Proposed;
        d.superseded_by = None;
        d.accepted_at.clear();
        d.accepted_by.clear();
        if d.title.trim().is_empty() {
            bail!(tr!("Une décision a un titre.", "A decision needs a title."));
        }
        d.pages = d.pages.iter().filter_map(|p| page_path(p)).collect();
        match r {
            Some(r) => {
                let old = self.decision(r)?;
                d.number = old.number;
                d.id = old.id.clone();
                d.date = old.date.clone();
                self.decision_save(&d, Some(&old))?;
            }
            None => {
                d.number = None;
                d.id = self.free_id(&d.title);
                if d.date.is_empty() {
                    d.date = chrono::Local::now().format("%Y-%m-%d").to_string();
                }
                self.decision_save(&d, None)?;
            }
        }
        self.fact(crate::facts::Body::DecisionProposed {
            id: d.id.clone(),
            title: d.title.clone(),
        });
        Ok(d)
    }

    /// A proposal that replaces the accepted decision `old`: it takes its place once accepted.
    pub fn decision_replace(&self, old: u32, mut d: Decision) -> Result<Decision> {
        let was = self.decision(&Ref::Number(old))?;
        if was.state != State::Accepted {
            bail!(tr!(
                "Seule une décision acceptée se remplace (la n° {old} ne l’est pas) : une proposition se révise.",
                "Only an accepted decision is replaced (number {old} is not): a proposal is revised."
            ));
        }
        d.supersedes = vec![old];
        self.decision_propose(None, d)
    }

    /// A human accepts the proposal `r` (ESC-303): it takes its number, the next after this
    /// register and `on` (the register of the branch it goes to), its file is renamed, its state,
    /// date and who written, and `ticket` (the one validated) added to its tickets; the pages that
    /// cite it by its id cite its number then, and the decisions it replaces become `superseded`,
    /// each naming it. A proposal numbered before ESC-303 keeps its number while no other has it,
    /// here or on that branch.
    pub fn decision_accept(
        &self,
        r: &Ref,
        by: &str,
        on: &Register,
        ticket: Option<&str>,
    ) -> Result<Decision> {
        let old = self.decision(r)?;
        let shown = old.reference();
        match old.state {
            State::Accepted => return Ok(old),
            State::Superseded => bail!(tr!(
                "La décision {shown} est remplacée : elle ne s’accepte plus.",
                "Decision {shown} is superseded: it can no longer be accepted."
            )),
            State::Proposed => {}
        }
        let number = match old.number {
            Some(n)
                if !self
                    .decisions()
                    .iter()
                    .any(|x| x.number == Some(n) && x.id != old.id)
                    && !on.taken.iter().any(|(m, id)| *m == n && *id != old.id) =>
            {
                n
            }
            _ => self.next_number().max(on.next),
        };
        let mut d = old.clone();
        d.number = Some(number);
        d.state = State::Accepted;
        d.accepted_at = chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%S%:z")
            .to_string();
        d.accepted_by = by.to_string();
        if let Some(key) = ticket {
            if !d.tickets.iter().any(|t| t.eq_ignore_ascii_case(key)) {
                d.tickets.push(key.to_string());
            }
        }
        self.decision_save(&d, Some(&old))?;
        self.cite_number(&old, number)?;
        for n in &d.supersedes {
            let Ok(was) = self.decision(&Ref::Number(*n)) else {
                continue;
            };
            if was.state == State::Accepted {
                let mut now = was.clone();
                now.state = State::Superseded;
                now.superseded_by = Some(number);
                self.decision_save(&now, Some(&was))?;
            }
        }
        self.fact(crate::facts::Body::DecisionAccepted {
            number,
            id: d.id.clone(),
            title: d.title.clone(),
        });
        Ok(d)
    }

    /// The pages that cite the proposal `old` cite `number` then: those that cite its id, and
    /// (numbered before ESC-303, and renumbered) those it names that cite its old number.
    fn cite_number(&self, old: &Decision, number: u32) -> Result<()> {
        let cites = |p: &Page, x: &Ref| match x {
            Ref::Id(id) => *id == old.id,
            Ref::Number(n) => old.number == Some(*n) && *n != number && old.pages.contains(&p.path),
        };
        for page in self.pages() {
            if !page.decisions.iter().any(|x| cites(&page, x)) {
                continue;
            }
            let rel = format!("{}.md", page.path);
            let mut p = page_read(&page.path, &self.raw_read(SPEC, &rel)?);
            let mut cited: Vec<Ref> = Vec::new();
            for x in &p.decisions {
                let x = if cites(&page, x) {
                    Ref::Number(number)
                } else {
                    x.clone()
                };
                if !cited.contains(&x) {
                    cited.push(x);
                }
            }
            p.decisions = cited;
            self.raw_write(SPEC, &rel, &page_write(&p))?;
        }
        Ok(())
    }

    // ----- documents -----

    fn info(&self, rel: &str, size: u64, modified: u64) -> DocInfo {
        let name = rel.rsplit('/').next().unwrap_or(rel).to_string();
        let kind = kind_of(&name);
        DocInfo {
            path: rel.to_string(),
            folder: rel
                .rsplit_once('/')
                .map(|(f, _)| f.to_string())
                .unwrap_or_default(),
            readable: crate::docsextract::reads_text(&name),
            name,
            size,
            kind: kind.into(),
            modified,
            author: String::new(),
            reads: 0,
            cited: 0,
        }
    }

    fn info_of(&self, rel: &str) -> DocInfo {
        let meta = self
            .file(DOCUMENTS, rel)
            .ok()
            .and_then(|f| std::fs::metadata(f).ok());
        let size = meta.as_ref().map_or(0, |m| m.len());
        self.info(rel, size, meta.as_ref().map_or(0, modified_ms))
    }

    /// The files of the drive (those of a folder and its folders when `folder` is given).
    pub fn documents(&self, folder: &str) -> Result<Vec<DocInfo>> {
        let base = self.dir(DOCUMENTS)?;
        let folder = folder_of(folder)?;
        let folder = folder.as_str();
        let mut start = base.clone();
        if !folder.is_empty() {
            let f = document_path(folder).ok_or_else(|| bad_document(folder))?;
            start = self.file(DOCUMENTS, &f)?;
        }
        Ok(walk(&start, |_| true, MAX_FILES)
            .into_iter()
            .map(|(rel, size)| {
                let modified = std::fs::metadata(start.join(&rel)).map_or(0, |m| modified_ms(&m));
                let full = if folder.is_empty() {
                    rel
                } else {
                    format!("{folder}/{rel}")
                };
                self.info(&full, size, modified)
            })
            .collect())
    }

    /// The folders of the drive, depth first, sorted, each with how many files it holds (its
    /// folders' included). Links and hidden folders are left out.
    pub fn folders(&self) -> Result<Vec<Folder>> {
        let base = self.dir(DOCUMENTS)?;
        let files = self.documents("")?;
        let mut out = Vec::new();
        fn go(base: &Path, dir: &Path, depth: usize, out: &mut Vec<String>) {
            if depth > MAX_FOLDER_DEPTH {
                return;
            }
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            let mut entries: Vec<_> = entries.flatten().collect();
            entries.sort_by_key(|e| e.file_name().to_string_lossy().to_lowercase());
            for e in entries {
                let name = e.file_name().to_string_lossy().to_string();
                let Ok(t) = e.file_type() else { continue };
                if t.is_symlink() || !t.is_dir() || name.starts_with('.') {
                    continue;
                }
                let rel = e
                    .path()
                    .strip_prefix(base)
                    .map(|r| r.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default();
                out.push(rel);
                go(base, &e.path(), depth + 1, out);
            }
        }
        let mut dirs = Vec::new();
        go(&base, &base, 1, &mut dirs);
        for path in dirs {
            let prefix = format!("{path}/");
            out.push(Folder {
                name: path.rsplit('/').next().unwrap_or(&path).to_string(),
                files: files.iter().filter(|f| f.path.starts_with(&prefix)).count(),
                path,
            });
        }
        Ok(out)
    }

    /// « Nouveau dossier »: a folder of the drive, with a `.gitkeep` so that git keeps it while it
    /// is empty. Its path as it was made.
    pub fn folder_add(&self, folder: &str) -> Result<String> {
        let folder = folder_of(folder)?;
        if folder.is_empty() {
            bail!(tr!("Donne un nom au dossier.", "Give the folder a name."));
        }
        let rel = document_path(&folder).ok_or_else(|| bad_document(&folder))?;
        let keep = self.file(DOCUMENTS, &format!("{rel}/.gitkeep"))?;
        if !keep.exists() {
            self.write(&keep, b"")?;
        }
        Ok(rel)
    }

    /// Where the text of a document is kept, by its path and what its file is now: `<key>.txt`
    /// for a text, `<key>.meta` for what is known of an image, `<key>.none` when there is nothing
    /// to read (not tried again until the file changes).
    fn text_file(&self, rel: &str, file: &Path) -> Option<PathBuf> {
        let dir = self.text_dir.as_ref()?;
        let meta = std::fs::metadata(file).ok()?;
        let stamp = meta
            .modified()
            .ok()
            .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_millis());
        let key = Sha256::digest(format!("{rel}\n{}\n{stamp}", meta.len()).as_bytes());
        let key: String = key.iter().take(12).map(|b| format!("{b:02x}")).collect();
        Some(dir.join(key))
    }

    fn keep(cache: &Path, got: &crate::docsextract::Extracted) {
        use crate::docsextract::Extracted;
        let (ext, body) = match got {
            Extracted::Text(t) => ("txt", t.as_str()),
            Extracted::Meta(m) => ("meta", m.as_str()),
            Extracted::None => ("none", ""),
        };
        if let Some(dir) = cache.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = crate::paths::write_atomic(&cache.with_extension(ext), body.as_bytes());
    }

    /// What Escouade reads of a document (`docsextract`), read once and kept in Escouade's folder,
    /// not in the repository.
    pub fn extracted(&self, rel: &str) -> Result<crate::docsextract::Extracted> {
        use crate::docsextract::{extract, Extracted};
        let path = document_path(rel).ok_or_else(|| bad_document(rel))?;
        let file = self.file(DOCUMENTS, &path)?;
        if !file.is_file() {
            bail!(tr!("Aucun document « {path} ».", "No document “{path}”."));
        }
        let cache = self.text_file(&path, &file);
        if let Some(c) = &cache {
            if let Ok(t) = std::fs::read_to_string(c.with_extension("txt")) {
                return Ok(Extracted::Text(t));
            }
            if let Ok(m) = std::fs::read_to_string(c.with_extension("meta")) {
                return Ok(Extracted::Meta(m));
            }
            if c.with_extension("none").exists() {
                return Ok(Extracted::None);
            }
        }
        let name = path.rsplit('/').next().unwrap_or(&path).to_string();
        let got = extract(&file, &name);
        if let Some(c) = &cache {
            Self::keep(c, &got);
        }
        Ok(got)
    }

    /// Keeps what was read of the document `rel` (read before it was put in the drive), so that it
    /// is not read again.
    pub fn keep_extracted(&self, rel: &str, got: &crate::docsextract::Extracted) {
        if let Ok(file) = self.file(DOCUMENTS, rel) {
            if let Some(c) = self.text_file(rel, &file) {
                Self::keep(&c, got);
            }
        }
    }

    /// A document's text, from `from` on (`TEXT_CHUNK` characters at most). A format whose text
    /// Escouade does not read answers `text: None` (an image, its `meta`).
    pub fn document_text(&self, rel: &str, from: usize, max: usize) -> Result<DocText> {
        use crate::docsextract::Extracted;
        let path = document_path(rel).ok_or_else(|| bad_document(rel))?;
        let kind = kind_of(&path);
        let got = self.extracted(&path)?;
        let Extracted::Text(text) = got else {
            return Ok(DocText {
                path,
                kind: kind.into(),
                text: None,
                total: 0,
                from: 0,
                truncated: false,
                meta: match got {
                    Extracted::Meta(m) => Some(m),
                    _ => None,
                },
            });
        };
        let total = text.chars().count();
        let max = max.clamp(1, TEXT_CHUNK);
        let from = from.min(total);
        let chunk: String = text.chars().skip(from).take(max).collect();
        Ok(DocText {
            path,
            kind: kind.into(),
            text: Some(chunk),
            total,
            from,
            truncated: from + max < total,
            meta: None,
        })
    }

    /// The first free path for `name` under `folder`: its own, else `name-2`, `name-3`…
    fn free_path(&self, folder: &str, name: &str) -> Result<String> {
        let at = |n: &str| {
            let rel = if folder.is_empty() {
                n.to_string()
            } else {
                format!("{folder}/{n}")
            };
            document_path(&rel).ok_or_else(|| bad_document(&rel))
        };
        let mut rel = at(name)?;
        let mut n = 2;
        while self.file(DOCUMENTS, &rel)?.exists() {
            let (stem, e) = name
                .rsplit_once('.')
                .map_or((name, String::new()), |(s, e)| (s, format!(".{e}")));
            rel = at(&format!("{stem}-{n}{e}"))?;
            n += 1;
        }
        Ok(rel)
    }

    /// Puts a file in the drive under `folder`, under its name or another when that one is taken:
    /// nothing already there is overwritten. The window and the tools go by `document_copy`.
    #[cfg(test)]
    pub fn document_add(&self, folder: &str, name: &str, bytes: &[u8]) -> Result<DocInfo> {
        let name = crate::attachfiles::safe_name(name.trim());
        let folder = folder_of(folder)?;
        let rel = self.free_path(&folder, &name)?;
        let file = self.file(DOCUMENTS, &rel)?;
        self.write(&file, bytes)?;
        Ok(self.info_of(&rel))
    }

    /// `document_add` of the file `src`, copied without being held whole in memory.
    pub fn document_copy(&self, folder: &str, name: &str, src: &Path) -> Result<DocInfo> {
        let name = crate::attachfiles::safe_name(name.trim());
        let folder = folder_of(folder)?;
        let rel = self.free_path(&folder, &name)?;
        let file = self.file(DOCUMENTS, &rel)?;
        // An empty file first: the folders are made and checked as for any write.
        self.write(&file, b"")?;
        if let Err(e) = std::fs::copy(src, &file) {
            let _ = std::fs::remove_file(&file);
            return Err(e.into());
        }
        Ok(self.info_of(&rel))
    }

    /// A document moved to `folder` (the root when empty), under its name or a free one. Its new
    /// path; its text is read again there if need be.
    pub fn document_move(&self, rel: &str, folder: &str) -> Result<DocInfo> {
        let path = document_path(rel).ok_or_else(|| bad_document(rel))?;
        let from = self.file(DOCUMENTS, &path)?;
        if !from.is_file() {
            bail!(tr!("Aucun document « {path} ».", "No document “{path}”."));
        }
        let folder = folder_of(folder)?;
        let here = path.rsplit_once('/').map_or("", |(f, _)| f);
        if here == folder {
            return Ok(self.info_of(&path));
        }
        let name = path.rsplit('/').next().unwrap_or(&path).to_string();
        let to_rel = self.free_path(&folder, &name)?;
        let to = self.file(DOCUMENTS, &to_rel)?;
        // The folder made and checked as for any write, then the file moved in.
        self.write(&to, b"")?;
        std::fs::remove_file(&to)?;
        std::fs::rename(&from, &to)?;
        Ok(self.info_of(&to_rel))
    }

    // ----- a spec already there -----

    /// The files and folders of the project that look like a specification, to be offered as
    /// a candidate: `SPEC.md` (anywhere at the top, or in `docs/`), a `spec`/`specs` folder, and a
    /// `docs/` folder of Markdown. `.escouade/` itself is not one.
    pub fn detect(&self) -> Vec<Candidate> {
        let mut out = Vec::new();
        let names = |dir: &Path| -> Vec<(String, bool)> {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return Vec::new();
            };
            let mut v: Vec<(String, bool)> = entries
                .flatten()
                .filter_map(|e| {
                    let t = e.file_type().ok()?;
                    if t.is_symlink() {
                        return None;
                    }
                    Some((e.file_name().to_string_lossy().to_string(), t.is_dir()))
                })
                .collect();
            v.sort();
            v
        };
        let rel_of = |dir: &str, name: &str| {
            if dir.is_empty() {
                name.to_string()
            } else {
                format!("{dir}/{name}")
            }
        };
        let push_file = |dir: &str, name: &str, out: &mut Vec<Candidate>| {
            let path = rel_of(dir, name);
            let size = std::fs::metadata(self.root.join(&path))
                .map(|m| m.len())
                .unwrap_or(0);
            out.push(Candidate {
                path,
                kind: "file".into(),
                files: 1,
                bytes: size,
            });
        };
        let is_spec_file = |n: &str| {
            let l = n.to_lowercase();
            matches!(
                l.as_str(),
                "spec.md"
                    | "specs.md"
                    | "specification.md"
                    | "specifications.md"
                    | "spécification.md"
            )
        };
        let mut docs_dir: Option<String> = None;
        let top = names(&self.root);
        for (name, dir) in &top {
            if !dir && is_spec_file(name) {
                push_file("", name, &mut out);
            }
            if *dir && name.eq_ignore_ascii_case("docs") {
                docs_dir = Some(name.clone());
            }
        }
        let folder = |dir: &str, out: &mut Vec<Candidate>| {
            let files = walk(&self.root.join(dir), md_name, MAX_FILES);
            if !files.is_empty() {
                out.push(Candidate {
                    path: dir.to_string(),
                    kind: "folder".into(),
                    files: files.len(),
                    bytes: files.iter().map(|(_, s)| s).sum(),
                });
            }
        };
        for (name, dir) in &top {
            let l = name.to_lowercase();
            if *dir
                && matches!(
                    l.as_str(),
                    "spec" | "specs" | "specification" | "specifications"
                )
            {
                folder(name, &mut out);
            }
        }
        if let Some(docs) = docs_dir {
            for (name, dir) in names(&self.root.join(&docs)) {
                if !dir && is_spec_file(&name) {
                    push_file(&docs, &name, &mut out);
                }
                let l = name.to_lowercase();
                if dir
                    && matches!(
                        l.as_str(),
                        "spec" | "specs" | "specification" | "specifications"
                    )
                {
                    folder(&rel_of(&docs, &name), &mut out);
                }
            }
            folder(&docs, &mut out);
        }
        out
    }
}

/// A folder of the drive as it is kept: no leading `/`, none at the end; empty for the root.
fn modified_ms(m: &std::fs::Metadata) -> u64 {
    m.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as u64)
}

fn folder_of(folder: &str) -> Result<String> {
    let f = folder.trim();
    if f.starts_with('/') {
        return Err(bad_document(f));
    }
    Ok(f.trim_end_matches('/').to_string())
}

fn bad_page(rel: &str) -> anyhow::Error {
    anyhow::anyhow!(tr!(
        "« {rel} » n’est pas un chemin de page (sujet/page, sans « .. » ni lettre de lecteur).",
        "“{rel}” is not a page path (subject/page, with no “..” nor drive letter)."
    ))
}

fn bad_document(rel: &str) -> anyhow::Error {
    anyhow::anyhow!(tr!(
        "« {rel} » n’est pas un chemin de document (dossiers/fichier, sans « .. »).",
        "“{rel}” is not a document path (folders/file, with no “..”)."
    ))
}
