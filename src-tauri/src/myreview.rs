//! « Ta relecture » (ESC-494, PR-7 of the PR in Escouade, direction A of `design/pr-2-3`): the user's own review of a pull
//! request. They write remarks under lines of its diff (« Ajouter à ma relecture » beside « Commenter maintenant »), each
//! waiting here, « en attente · pas publiée », and a text; then they publish it all with a verdict (comment, approve, ask
//! for changes) in **one** review of the host, through the review its plugin declares (`Forge::review`, the one an
//! agent's review used before PR-5).
//!
//! - **Kept** by pull request in the durable state (`PersistedState::pending_reviews`, written by the atomic path of
//!   ESC-469): closing the view or the app loses nothing. It goes at its publication, or when the user gives it up.
//! - **Placed** against the diff last read: a remark follows its code when a commit moves it, and is **stale** when a
//!   commit takes its line away; a stale remark goes in the review's text with its place (« comme commentaire de la
//!   PR »), or the user takes it out.
//! - **Nothing leaves without their click**: only the window's command `pr_review_submit` publishes (no tool of the MCP
//!   server, no agent). A refusal keeps it all waiting and says why; a 422 of a line gone reads the diff again and says
//!   which remarks lost their line.
//! - **On their own pull request** the hosts take a comment alone: the same rule as the list's « Mes PR » (the author is
//!   the login the host is read with, `Core::pr_me`).
//! - **The changes asked go back to the ticket's agent** when the user says so, by ESC-61's way
//!   (`ticket_pr_fix_reviews_with`): the user's text and remarks are theirs, said as such, the host's others stay a third
//!   party's data.

use crate::agentprs::FIX_MAX;
use crate::core::Core;
use crate::forge::{DiffSide, ReviewDraft, ReviewLine, ReviewState, Writing};
use crate::i18n::Lang;
use crate::integrations::HttpError;
use crate::model::now_ms;
use crate::prread::{PrContents, PrFileRead, PrLine};
use crate::prview::{Failure, FailureKind, Read};
use crate::prwrite::{write_failure, BODY_MAX};
use crate::text::truncate;
use anyhow::{anyhow, bail, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;

pub use escouade_model::myreview::{PendingNote, PendingReview};

/// The most remarks a review waits with.
pub const NOTES_MAX: usize = 100;

/// The reviews on their way to the host: a second click does not publish one twice.
static SUBMITTING: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// What the user changes of their review waiting, on a click.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PendingChange {
    /// « Ajouter à ma relecture » on a line (its range with `start_line`), on the base's side for a line removed.
    #[serde(rename_all = "camelCase")]
    Add {
        path: String,
        line: u32,
        #[serde(default)]
        start_line: Option<u32>,
        #[serde(default)]
        side: DiffSide,
        body: String,
    },
    /// A remark's text written again.
    Edit { id: u64, body: String },
    /// A remark taken out.
    Remove { id: u64 },
    /// The review's text.
    Body { body: String },
    /// « Abandonner ma relecture », confirmed: all of it gone.
    Discard,
}

/// The review waiting for the view: its remarks placed against the diff read last, and whether the pull request is the
/// user's own (a comment alone, then).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingRead {
    pub review: PendingReview,
    pub own: bool,
}

/// « Publier », as the user clicked it: the verdict, the text as it stands in the panel, and whether the ticket's agent
/// gets the changes asked.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewAsk {
    pub verdict: ReviewState,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub to_agent: bool,
}

/// What a publication did.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Submitted {
    /// The remarks published on their lines.
    pub lines: usize,
    /// The remarks whose line was gone, published in the review's text with their place.
    pub stale: usize,
    /// The ticket whose agent got the changes asked (its key).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_ticket: Option<String>,
    /// Why its agent did not get them: the review is published all the same.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_failure: Option<String>,
}

/// The user's review as the ticket's agent reads it: their words, never a third party's.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OwnReview {
    pub body: String,
    /// Each remark: its place (`path:line`, `path:10-12`) and its text.
    pub notes: Vec<(String, String)>,
    /// The review's text as it went to the host (its stale remarks in it).
    pub sent: String,
}

impl OwnReview {
    /// A text the host now has of it: what its feedback does not hand the agent twice.
    pub fn holds(&self, text: &str) -> bool {
        let text = text.trim();
        !text.is_empty()
            && (self.body.trim() == text
                || self.sent.trim() == text
                || self.notes.iter().any(|(_, b)| b.trim() == text))
    }
}

/// The agent's message of « L'agent corrige les retours » (`reviews`) with the user's own review in it, after its first
/// paragraph: the user's words, said as theirs (their text and each remark with its place), not between the tags of a
/// third party's text. Cut at the message's bound.
pub fn with_own(lang: Lang, reviews: &str, own: &OwnReview) -> String {
    let mut mine = tr_in!(
        lang,
        "Ma relecture, que je viens de publier sur l'hébergeur : ce sont mes demandes, à traiter en premier.",
        "My review, which I just published on the code host: these are my requests, to address first."
    );
    if !own.body.trim().is_empty() {
        mine.push_str("\n\n");
        mine.push_str(own.body.trim());
    }
    if !own.notes.is_empty() {
        mine.push_str("\n\n");
        mine.push_str(&tr_in!(
            lang,
            "Mes remarques sur les lignes :",
            "My remarks on the lines:"
        ));
        for (place, body) in &own.notes {
            mine.push_str(&format!("\n- {place} : {}", body.trim()));
        }
    }
    let (head, rest) = reviews.split_once("\n\n").unwrap_or((reviews, ""));
    let out = if rest.is_empty() {
        format!("{head}\n\n{mine}")
    } else {
        format!("{head}\n\n{mine}\n\n{rest}")
    };
    truncate(&out, FIX_MAX)
}

/// The number of a line of the diff on `side`, when a comment may be on it there: a line removed on the base's, an
/// added or kept one on the tip's (never a hunk's head).
fn number_on(l: &PrLine, side: DiffSide) -> Option<u32> {
    match side {
        DiffSide::Left => l.old.filter(|_| l.kind == '-'),
        DiffSide::Right => l.new.filter(|_| l.kind == '+' || l.kind == ' '),
    }
}

/// The text of the lines `from..=to` of a file's diff on `side`; none when one of them is not there.
pub fn code_at(f: &PrFileRead, side: DiffSide, from: u32, to: u32) -> Option<Vec<String>> {
    (from..=to)
        .map(|n| {
            f.lines
                .iter()
                .find(|l| number_on(l, side) == Some(n))
                .map(|l| l.text.clone())
        })
        .collect()
}

/// Where a remark stands in the diff read: on its line when its text is still there, on the nearest line that has it
/// when a commit moved it (its range with it), none when its line is gone (stale). A remark without the text of its
/// lines stays where its line is.
pub fn place(n: &PendingNote, files: &[PrFileRead]) -> Option<(u32, Option<u32>)> {
    let f = files.iter().find(|f| f.file.path == n.path)?;
    let here = f
        .lines
        .iter()
        .find(|l| number_on(l, n.side) == Some(n.line));
    let Some(last) = n.code.last() else {
        return here.map(|_| (n.line, n.start_line));
    };
    if here.is_some_and(|l| &l.text == last) {
        return Some((n.line, n.start_line));
    }
    let moved = f
        .lines
        .iter()
        .filter(|l| &l.text == last)
        .filter_map(|l| number_on(l, n.side))
        .min_by_key(|k| (i64::from(*k) - i64::from(n.line)).abs())?;
    let delta = i64::from(moved) - i64::from(n.line);
    let start = n
        .start_line
        .map(|s| u32::try_from((i64::from(s) + delta).max(1)).unwrap_or(1))
        .filter(|s| *s < moved);
    Some((moved, start))
}

/// The diff of a reading is there to place remarks against: its lines read by git.
fn placeable(c: &PrContents) -> bool {
    c.diff && c.diff_failure.is_none()
}

/// Each remark placed against the diff read: moved with its code, stale without its line. True when one changed.
pub fn replace(review: &mut PendingReview, files: &[PrFileRead]) -> bool {
    let mut changed = false;
    for n in &mut review.notes {
        let was = (n.line, n.start_line, n.stale);
        match place(n, files) {
            Some((line, start)) => {
                n.line = line;
                n.start_line = start;
                n.stale = false;
            }
            None => n.stale = true,
        }
        changed |= was != (n.line, n.start_line, n.stale);
    }
    changed
}

/// Where a remark is, in words: `path:line`, `path:10-12`.
pub fn place_of(n: &PendingNote) -> String {
    match n.start_line {
        Some(s) => format!("{}:{s}-{}", n.path, n.line),
        None => format!("{}:{}", n.path, n.line),
    }
}

/// The review's text as it goes: the user's, then each stale remark with its place, as a comment of the pull request
/// (« `lien.ts:12` : … », as an agent's remark outside the diff goes).
pub fn review_text(body: &str, notes: &[PendingNote]) -> String {
    let mut out = body.trim().to_string();
    for n in notes.iter().filter(|n| n.stale) {
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(&format!("`{}` : {}", place_of(n), n.body.trim()));
    }
    out
}

/// The review as it goes to the host: its verdict, its text, the remarks on their lines, on the commit read.
pub fn draft_of(review: &PendingReview, verdict: ReviewState, commit: &str) -> ReviewDraft {
    ReviewDraft {
        state: verdict,
        body: review_text(&review.body, &review.notes),
        lines: review
            .notes
            .iter()
            .filter(|n| !n.stale)
            .map(|n| ReviewLine {
                path: n.path.clone(),
                line: n.line,
                body: n.body.trim().to_string(),
                start_line: n.start_line.filter(|s| *s < n.line),
                side: n.side,
            })
            .collect(),
        commit: commit.to_string(),
    }
}

/// Whether the host gives this verdict, as it declares it.
pub fn verdict_allowed(w: &Writing, verdict: ReviewState) -> bool {
    w.review
        && match verdict {
            ReviewState::Commented => true,
            ReviewState::Approved => w.approve,
            ReviewState::ChangesRequested => w.request_changes,
        }
}

/// Why the review cannot go as it stands: a verdict the host or the user's own pull request refuses, a text needed, too
/// long, nothing to say. None when it can.
pub fn refusal(
    review: &PendingReview,
    verdict: ReviewState,
    w: &Writing,
    own: bool,
    host: &str,
) -> Option<String> {
    if !w.review {
        return Some(tr!(
            "{host} ne publie pas de relecture depuis Escouade.",
            "{host} does not publish a review from Escouade."
        ));
    }
    if own && verdict != ReviewState::Commented {
        return Some(tr!(
            "{host} ne permet que de commenter sa propre PR.",
            "{host} only lets you comment on your own pull request."
        ));
    }
    if !verdict_allowed(w, verdict) {
        return Some(tr!(
            "{host} ne donne pas ce verdict depuis Escouade.",
            "{host} does not give this verdict from Escouade."
        ));
    }
    let blank = review.body.trim().is_empty();
    if verdict == ReviewState::ChangesRequested && blank {
        return Some(tr!(
            "Écris le texte de ta relecture pour demander des changements.",
            "Write your review’s text to ask for changes."
        ));
    }
    if verdict == ReviewState::Commented && blank && review.notes.is_empty() {
        return Some(tr!("Rien à publier.", "Nothing to publish."));
    }
    let long = review.body.chars().count() > BODY_MAX
        || review
            .notes
            .iter()
            .any(|n| n.body.chars().count() > BODY_MAX);
    if long {
        return Some(tr!(
            "Un texte dépasse {BODY_MAX} caractères.",
            "A text is longer than {BODY_MAX} characters."
        ));
    }
    None
}

/// A remark's text: not empty, not too long.
fn checked(body: &str) -> Result<String> {
    let body = body.trim();
    if body.is_empty() {
        bail!(tr!("La remarque est vide.", "The remark is empty."));
    }
    if body.chars().count() > BODY_MAX {
        bail!(tr!(
            "La remarque dépasse {BODY_MAX} caractères.",
            "The remark is longer than {BODY_MAX} characters."
        ));
    }
    Ok(body.to_string())
}

/// A host's refusal of a comment on a line it no longer has in its diff (GitHub's 422 « Line could not be resolved »,
/// « … must be part of the diff »).
pub fn lines_refused(e: &anyhow::Error) -> bool {
    let status = e.downcast_ref::<HttpError>().map(|h| h.status);
    let m = format!("{e:#}").to_lowercase();
    status == Some(422)
        && [
            "could not be resolved",
            "part of the diff",
            "same hunk",
            "line must",
            "position",
        ]
        .iter()
        .any(|w| m.contains(w))
}

fn here<T>(message: String) -> Read<T> {
    Read {
        data: None,
        failure: Some(Failure {
            kind: FailureKind::Other,
            message,
            reset: None,
            settings: false,
        }),
    }
}

fn key(project_id: &str, number: u64, host_repo: &str) -> String {
    format!("{project_id}|{}|{number}", host_repo.to_lowercase())
}

/// The reviews a start finds again: those of the projects still here, with something in them.
pub fn kept_pending(all: &[PendingReview], known: impl Fn(&str) -> bool) -> Vec<PendingReview> {
    all.iter()
        .filter(|r| known(&r.project_id) && !r.is_empty())
        .cloned()
        .collect()
}

impl Core {
    /// The review waiting on that pull request, else an empty one.
    fn pending_of(&self, project_id: &str, number: u64, host_repo: &str) -> PendingReview {
        self.pending_reviews
            .read()
            .iter()
            .find(|r| r.is_of(project_id, number, host_repo))
            .cloned()
            .unwrap_or_else(|| PendingReview {
                project_id: project_id.to_string(),
                host_repo: host_repo.to_string(),
                number,
                ..Default::default()
            })
    }

    /// The review kept as it is now (gone when empty), the state saved.
    fn pending_keep(&self, review: &PendingReview) {
        {
            let mut all = self.pending_reviews.write();
            all.retain(|r| !r.is_of(&review.project_id, review.number, &review.host_repo));
            if !review.is_empty() {
                all.push(review.clone());
            }
        }
        self.request_save();
    }

    /// The pull request is the user's own: its author is the login the host is read with.
    fn pr_is_own(&self, project_id: &str, author: &str) -> bool {
        let me = self.pr_me(project_id);
        !me.is_empty() && !author.is_empty() && me.eq_ignore_ascii_case(author)
    }

    /// The review placed against `contents`, kept when a remark moved or went stale; and whether it is the user's own.
    fn pending_placed(
        &self,
        mut review: PendingReview,
        contents: Option<&PrContents>,
    ) -> PendingRead {
        let own = contents.is_some_and(|c| self.pr_is_own(&review.project_id, &c.pr.author));
        if let Some(c) = contents.filter(|c| placeable(c)) {
            if replace(&mut review, &c.files) {
                review.updated_at = now_ms();
                self.pending_keep(&review);
            }
        }
        PendingRead { review, own }
    }

    /// The user's review waiting on the pull request `number`, its remarks placed against the diff read last.
    pub fn pr_pending(&self, project_id: &str, number: u64, host_repo: &str) -> PendingRead {
        let review = self.pending_of(project_id, number, host_repo);
        let kept = self.pr_contents_kept(project_id, number, host_repo);
        self.pending_placed(review, kept.as_ref())
    }

    /// A change of the user's review waiting, on their click: a remark added on lines of the diff read, written again or
    /// taken out, its text, or all of it given up.
    pub fn pr_pending_change(
        &self,
        project_id: &str,
        number: u64,
        host_repo: &str,
        change: PendingChange,
    ) -> Result<PendingRead> {
        let mut review = self.pending_of(project_id, number, host_repo);
        let kept = self.pr_contents_kept(project_id, number, host_repo);
        match change {
            PendingChange::Add {
                path,
                line,
                start_line,
                side,
                body,
            } => {
                let body = checked(&body)?;
                let start_line = start_line.filter(|s| *s > 0 && *s < line);
                if path.is_empty() || line == 0 {
                    bail!(tr!(
                        "Rien à quoi écrire : la ligne manque.",
                        "Nothing to write to: the line is missing."
                    ));
                }
                if review.notes.len() >= NOTES_MAX {
                    bail!(tr!(
                        "Ta relecture a déjà {NOTES_MAX} remarques : publie-la d'abord.",
                        "Your review already has {NOTES_MAX} remarks: publish it first."
                    ));
                }
                // The text of its lines in the diff the user read, to follow them through the commits to come.
                let code = match kept.as_ref().filter(|c| placeable(c)) {
                    Some(c) => {
                        let f = c.files.iter().find(|f| f.file.path == path);
                        match f.and_then(|f| code_at(f, side, start_line.unwrap_or(line), line)) {
                            Some(code) => code,
                            None => bail!(tr!(
                                "Cette ligne n'est plus dans le diff lu : relis la PR.",
                                "This line is no longer in the diff read: read the pull request again."
                            )),
                        }
                    }
                    None => Vec::new(),
                };
                let id = review.next_id();
                review.notes.push(PendingNote {
                    id,
                    path,
                    line,
                    start_line,
                    side,
                    body,
                    code,
                    created: now_ms(),
                    stale: false,
                });
            }
            PendingChange::Edit { id, body } => {
                let body = checked(&body)?;
                let n = review
                    .notes
                    .iter_mut()
                    .find(|n| n.id == id)
                    .ok_or_else(gone_note)?;
                n.body = body;
            }
            PendingChange::Remove { id } => {
                let before = review.notes.len();
                review.notes.retain(|n| n.id != id);
                if review.notes.len() == before {
                    return Err(gone_note());
                }
            }
            PendingChange::Body { body } => {
                if body.chars().count() > BODY_MAX {
                    bail!(tr!(
                        "Le texte dépasse {BODY_MAX} caractères.",
                        "The text is longer than {BODY_MAX} characters."
                    ));
                }
                review.body = body;
            }
            PendingChange::Discard => {
                review.body.clear();
                review.notes.clear();
            }
        }
        review.updated_at = now_ms();
        self.pending_keep(&review);
        Ok(self.pending_placed(review, kept.as_ref()))
    }

    /// « Publier » (ESC-494): the user's review, its text, its verdict and its remarks on their lines, in one review of
    /// the host, under their account; on their click and from nowhere else (the window's command). Published, it is gone
    /// from here, the pull request is read again everywhere (`PrChanged`), and the ticket's agent gets the changes asked
    /// when the user said so. Refused, all of it stays waiting and the failure says why; a line the host no longer has
    /// reads the diff again, its remark then said stale.
    pub async fn pr_review_submit(
        self: &Arc<Self>,
        project_id: &str,
        number: u64,
        host_repo: &str,
        ask: ReviewAsk,
    ) -> Read<Submitted> {
        let k = key(project_id, number, host_repo);
        if !SUBMITTING
            .lock()
            .get_or_insert_with(HashSet::new)
            .insert(k.clone())
        {
            return here(tr!(
                "Ta relecture part déjà chez l'hébergeur.",
                "Your review is already on its way to the host."
            ));
        }
        let out = self.submit(project_id, number, host_repo, ask).await;
        if let Some(s) = SUBMITTING.lock().as_mut() {
            s.remove(&k);
        }
        out
    }

    async fn submit(
        self: &Arc<Self>,
        project_id: &str,
        number: u64,
        host_repo: &str,
        ask: ReviewAsk,
    ) -> Read<Submitted> {
        // The text as it stands in the panel, kept even if the host refuses.
        let mut review = self.pending_of(project_id, number, host_repo);
        if ask.body.chars().count() <= BODY_MAX {
            review.body = ask.body.clone();
            review.updated_at = now_ms();
            self.pending_keep(&review);
        }
        let (client, repo, can) = match self.pr_place(project_id, number, host_repo).await {
            Ok(t) => t,
            Err(e) => return Read::failed(&e, None),
        };
        let host = self
            .project_code_host(project_id)
            .await
            .map(|h| h.name)
            .unwrap_or_else(|| tr!("L'hébergeur", "The host"));
        // The diff the user read (kept while fresh, else read again): the remarks placed against it, on its commit.
        let read = self.pr_contents(project_id, number, host_repo, true).await;
        let contents = read
            .data
            .or_else(|| self.pr_contents_kept(project_id, number, host_repo));
        let placed = self.pending_placed(review, contents.as_ref());
        let (review, own) = (placed.review, placed.own);
        if let Some(why) = refusal(&review, ask.verdict, &can.writing, own, &host) {
            return here(why);
        }
        let tip = contents.as_ref().map(|c| c.tip.clone()).unwrap_or_default();
        let draft = draft_of(&review, ask.verdict, &tip);
        let lines = draft.lines.len();
        let stale = review.notes.iter().filter(|n| n.stale).count();
        if let Err(e) = client.review(&repo, number, &draft).await {
            let mut f = write_failure(&e, client.rate_limit());
            if lines_refused(&e) {
                // The host lost a line since: the diff read again says which remark.
                self.pr_contents_stale(project_id, number, host_repo);
                let again = self.pr_contents(project_id, number, host_repo, true).await;
                let now = self.pending_placed(
                    self.pending_of(project_id, number, host_repo),
                    again.data.as_ref(),
                );
                let gone = now.review.notes.iter().filter(|n| n.stale).count();
                f.kind = FailureKind::Lines;
                if gone > stale {
                    f.message = tr!(
                        "{host} n'a plus la ligne d'une remarque : elle est marquée dépassée. Publie-la comme commentaire de la PR ou retire-la. ({m})",
                        "{host} no longer has the line of a remark: it is marked stale. Publish it as a comment of the pull request or take it out. ({m})",
                        m = f.message
                    );
                }
            }
            return Read {
                data: None,
                failure: Some(f),
            };
        }
        // Published: gone from here, read again everywhere.
        self.pending_keep(&PendingReview {
            project_id: project_id.to_string(),
            host_repo: host_repo.to_string(),
            number,
            ..Default::default()
        });
        self.pr_contents_stale(project_id, number, host_repo);
        self.pr_written(project_id, number);
        let mut done = Submitted {
            lines,
            stale,
            ..Default::default()
        };
        if ask.to_agent && ask.verdict != ReviewState::Approved {
            match self.ticket_of_pr(project_id, number, host_repo) {
                Some(t) => {
                    let own = OwnReview {
                        body: review.body.trim().to_string(),
                        notes: review
                            .notes
                            .iter()
                            .map(|n| (place_of(n), n.body.trim().to_string()))
                            .collect(),
                        sent: draft.body.clone(),
                    };
                    match self.ticket_pr_fix_reviews_with(&t.id, Some(own)).await {
                        Ok(()) => done.agent_ticket = Some(t.key.clone()),
                        Err(e) => done.agent_failure = Some(format!("{e:#}")),
                    }
                }
                None => {
                    done.agent_failure = Some(tr!(
                        "Cette PR n'est celle d'aucun ticket.",
                        "This pull request is no ticket's."
                    ))
                }
            }
        }
        Read::ok(done)
    }
}

fn gone_note() -> anyhow::Error {
    anyhow!(tr!(
        "Cette remarque n'est plus dans ta relecture.",
        "This remark is no longer in your review."
    ))
}
