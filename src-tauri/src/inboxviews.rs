//! What Escouade puts in the box « À toi » of its own (ESC-513, docs/INBOX.md § 5): views on what
//! already waits for the user elsewhere, never copies. The requests an agent stopped for
//! (permission, question, plan), the tickets « À tester » and the blocked ones, the decisions
//! proposed, the docs to share and the reviews a code host asks of the user are read from the
//! state each time the box is read: settled elsewhere (in the conversation, on the board, in Docs),
//! a view is gone at once. Settled in the box (`inbox_view_act`), it is the same gesture as at its
//! source, through the same functions, on the view the user read: one settled meanwhile is refused,
//! never acted on again.
//!
//! « Plus tard » puts a view out of the box until a time without touching its source; a blocking
//! request is never put off (its agent waits). Like the box, the views exist only while the MCP
//! server is on: off, nothing of them shows, and every source keeps its place of today.

use crate::asks::{MarkKind, ViewMark};
use crate::core::Core;
use crate::model::{now_ms, Column, PendingView, Rhythm, UiEvent};
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Weak};

/// Where a view comes from.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ViewSource {
    /// A permission an agent stopped for.
    Permission,
    /// A question (`AskUserQuestion`).
    Question,
    /// A plan to approve (`ExitPlanMode`).
    Plan,
    /// A ticket « À tester »: « Ticket à valider ».
    TicketReview,
    /// A ticket that no longer moves on by itself (a conflict, failed tests, a refusal).
    TicketBlocked,
    /// A decision proposed, waiting to be accepted.
    Decision,
    /// The changes of a project's docs not shared yet.
    Docs,
    /// A pull request whose review the code host asks of the user (« Ta relecture »).
    Review,
    /// An agent a teammate passes to this machine (TS-17): « Alice te passe DEM-41 · Prendre ».
    Handoff,
    /// A ticket of the team assigned to the user with a teammate's text (ESC-645): « Bob t’assigne DEM-4 · Accepter ».
    TeamAccept,
    /// A scheduled task an agent made or changed, waiting for the user's go-ahead (ESC-647): « Claude programme
    /// Maintenance · Accepter ».
    Scheduled,
}

/// The group a view goes in (§ 10.2): « Tes agents se sont arrêtés », « Aujourd'hui », « Plus tôt ».
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ViewGroup {
    Stopped,
    Today,
    Earlier,
}

/// A view, as the window shows it: one line of the list, and what its gestures name.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InboxItem {
    /// What names it, for its gestures and « Plus tard »: `req:<agent>:<request>`, `ticket:<id>`,
    /// `blocked:<id>`, `decision:<project>:<id>`, `docs:<project>`, `review:<project>:<repo>:<n>`.
    pub key: String,
    pub source: ViewSource,
    pub group: ViewGroup,
    pub project_id: String,
    pub project_name: String,
    pub private: bool,
    /// What it is about, in one line: the command or the file asked for, the question, the ticket's
    /// title, the decision's, the pull request's; empty for the docs (the window says how many).
    pub title: String,
    /// Since when it waits (epoch ms); 0 when its source does not say.
    pub since: i64,
    pub agent_id: Option<String>,
    pub agent_name: Option<String>,
    pub ticket_id: Option<String>,
    pub ticket_key: Option<String>,
    /// Put off until then (« Plus tard »): out of the counts until it comes back.
    pub snoozed_until: Option<i64>,
    /// A request an agent stopped for, as its card in the conversation has it.
    pub request: Option<PendingView>,
    /// A ticket: its column, why it is blocked, whether by a conflict, sent without all its
    /// criteria.
    pub column: Option<Column>,
    pub blocked: Option<String>,
    pub conflict: bool,
    pub partial: bool,
    /// A ticket of its own target (ESC-552), when it is not the board's: « vers main » on its line.
    pub target: Option<String>,
    /// Its validation stopped for another line (ESC-552): the branch it came from, which
    /// « Choisir la cible » offers first.
    pub other_line: Option<String>,
    /// A decision: its id (how a proposal is cited).
    pub decision: Option<String>,
    /// The docs: how many files to share.
    pub files: u32,
    /// A pull request: its number, its link and the host's word (PR, MR).
    pub number: Option<u64>,
    pub url: Option<String>,
    pub word: Option<String>,
    /// A passage: who passes the agent, the harness it ran on (its id), and the branch pushed.
    pub by: Option<String>,
    pub engine: Option<String>,
    pub branch: Option<String>,
    /// What the squad lead said of it (ESC-557): « Le chef s'en occupe » (`claimed`) or « Réglé
    /// par … » (`settled`), with its note. Marked, it no longer counts as waiting for the user.
    pub mark: Option<ViewMark>,
    /// A scheduled task to accept (ESC-647): what the user reads before the go-ahead.
    pub scheduled: Option<ScheduledAsk>,
    /// What the Overseer recommends the user answers to a request it left to them (ESC-646): never an answer.
    pub advice: Option<crate::overseer::Advice>,
    /// The state of its source, that a mark holds (`ViewMark::sign`): changed, the mark falls.
    #[serde(skip)]
    pub(crate) sign: String,
}

impl InboxItem {
    fn new(key: String, source: ViewSource, group: ViewGroup, project: (&str, &str, bool)) -> Self {
        Self {
            key,
            source,
            group,
            project_id: project.0.to_string(),
            project_name: project.1.to_string(),
            private: project.2,
            title: String::new(),
            since: 0,
            agent_id: None,
            agent_name: None,
            ticket_id: None,
            ticket_key: None,
            snoozed_until: None,
            request: None,
            column: None,
            blocked: None,
            conflict: false,
            partial: false,
            target: None,
            other_line: None,
            decision: None,
            files: 0,
            number: None,
            url: None,
            word: None,
            by: None,
            engine: None,
            branch: None,
            mark: None,
            scheduled: None,
            advice: None,
            sign: String::new(),
        }
    }

    pub fn blocking(&self) -> bool {
        self.group == ViewGroup::Stopped
    }

    /// It waits for the user: not put off, not marked by the squad lead.
    pub fn waiting(&self) -> bool {
        self.snoozed_until.is_none() && self.mark.is_none()
    }
}

/// A scheduled task waiting for the user's go-ahead (ESC-647), as its « À toi » line describes it: its project is the
/// view's, its author the view's `by` and `ticket_key`.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledAsk {
    pub task_id: String,
    pub rhythm: Rhythm,
    /// Its rhythm as a cron expression.
    pub cron: String,
    pub description: String,
    pub criteria: Vec<String>,
    pub catch_up: bool,
    /// Its next due time once accepted (epoch ms).
    pub next: Option<i64>,
}

/// The views as the window reads them.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InboxViews {
    /// The MCP server is wanted: off, no box (and no view).
    pub enabled: bool,
    /// Of the project asked, or of all: the blocking first (the oldest first), then the others,
    /// the oldest first; those put off last.
    pub views: Vec<InboxItem>,
    /// The requests agents stopped for, every project (« N en attente »).
    pub blocking: usize,
    /// All that waits for the user in the box, every project: the views not put off and the
    /// requests open (not put off): what « à toi » counts.
    pub total: usize,
}

/// A gesture on a view: the same as at its source.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "act", rename_all = "camelCase")]
pub enum ViewAct {
    /// « Autoriser » a permission, or approve a plan.
    Allow,
    /// « Toujours autoriser », when the card offers it.
    Always,
    /// « Refuser », with a word for the agent.
    Deny { message: Option<String> },
    /// The answers to a question, by question.
    Answer { answers: serde_json::Value },
    /// « Valider » a ticket « À tester ».
    Approve,
    /// « Renvoyer à l'agent », with what to change.
    Reject { comment: String },
    /// « Reprendre » a blocked ticket « En cours ».
    Resume,
    /// « L'agent résout » a conflict.
    Resolve,
    /// « Annuler » a conflict.
    Dismiss,
    /// « Viser <branche> »: the ticket's target set to the branch its validation stopped for
    /// (ESC-552), or to another.
    Retarget { target: String },
    /// « Accepter » a decision proposed.
    Accept,
    /// « Partager » the docs.
    Share,
    /// « Prendre » an agent a teammate passes, on `engine` (its own when none).
    Take {
        engine: Option<crate::model::EngineId>,
    },
}

impl ViewAct {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Always => "always",
            Self::Deny { .. } => "deny",
            Self::Answer { .. } => "answer",
            Self::Approve => "approve",
            Self::Reject { .. } => "reject",
            Self::Resume => "resume",
            Self::Resolve => "resolve",
            Self::Dismiss => "dismiss",
            Self::Retarget { .. } => "retarget",
            Self::Accept => "accept",
            Self::Share => "share",
            Self::Take { .. } => "take",
        }
    }

    /// Whether this gesture is one of the view's source.
    fn fits(&self, v: &InboxItem) -> bool {
        use ViewSource as S;
        match (self, v.source) {
            (Self::Allow | Self::Deny { .. }, S::Permission | S::Plan) => true,
            (Self::Always, S::Permission) => v.request.as_ref().is_some_and(|r| r.can_always),
            (Self::Answer { .. }, S::Question) => true,
            (Self::Approve | Self::Reject { .. }, S::TicketReview) => true,
            (Self::Resume, S::TicketBlocked) => v.column == Some(Column::Doing) && !v.conflict,
            (Self::Resolve | Self::Dismiss, S::TicketBlocked) => v.conflict,
            (Self::Retarget { .. }, S::TicketBlocked) => v.other_line.is_some(),
            (Self::Accept, S::Decision | S::TeamAccept | S::Scheduled) => true,
            (Self::Share, S::Docs) => true,
            (Self::Take { .. }, S::Handoff) => true,
            _ => false,
        }
    }
}

/// What the box keeps of its views: their « Plus tard », and the squad lead's marks (ESC-557).
#[derive(Debug, Default)]
pub struct ViewSnoozes {
    pub until: BTreeMap<String, i64>,
    pub marks: BTreeMap<String, ViewMark>,
}

impl ViewSnoozes {
    /// A ticket moved (its column, a conflict): the marks on its views fall, whether the box was read between or not.
    /// Called by the move itself, under the tickets' lock (`Core::edit_ticket_with`), never by its fact: the bus
    /// hands a fact to its sinks later, on its own thread, and the fact of the move into « À tester » would drop the
    /// mark the squad lead left after it (ESC-621). Whether one fell.
    pub fn forget_ticket(&mut self, ticket_id: &str) -> bool {
        let ticket = self.marks.remove(&format!("ticket:{ticket_id}")).is_some();
        let blocked = self.marks.remove(&format!("blocked:{ticket_id}")).is_some();
        ticket || blocked
    }
}

/// The longest note of a mark, in characters.
pub const MARK_NOTE_MAX: usize = 200;

/// Who leaves a mark on a view: an agent of Escouade (its id), or Claude outside Escouade.
#[derive(Debug, Clone)]
pub struct Marker {
    pub agent: Option<String>,
    pub name: String,
}

/// Why the squad lead does not settle a view itself (`settle_inbox_view`), or does not take it in
/// hand (`claim_inbox_view`): what stays the user's gesture, or what is answered, not ticked.
pub(crate) fn not_markable(source: ViewSource, from_ticket: bool, settle: bool) -> Option<String> {
    match source {
        ViewSource::Docs => Some(tr!(
            "Partager les docs reste le geste de l’utilisateur (ou share_docs) : cette ligne ne se coche pas à sa place.",
            "Sharing the docs stays the user’s gesture (or share_docs): this line is not ticked for them."
        )),
        ViewSource::TeamAccept => Some(tr!(
            "Accepter le texte qu’un collègue a écrit reste le geste de l’utilisateur : cette ligne n’est ni prise en charge ni cochée par un agent.",
            "Accepting a text a teammate wrote stays the user’s gesture: this line is neither taken in hand nor ticked by an agent."
        )),
        ViewSource::Scheduled => Some(tr!(
            "Accepter une tâche programmée par un agent reste le geste de l’utilisateur : cette ligne n’est ni prise en charge ni cochée par un agent.",
            "Accepting a task an agent scheduled stays the user’s gesture: this line is neither taken in hand nor ticked by an agent."
        )),
        ViewSource::Review => Some(tr!(
            "Cette relecture, l’hébergeur la demande à l’utilisateur : elle reste la sienne, ni prise en charge ni cochée par un agent.",
            "The code host asks this review of the user: it stays theirs, neither taken in hand nor ticked by an agent."
        )),
        ViewSource::Permission | ViewSource::Question | ViewSource::Plan if settle => Some(tr!(
            "Un agent arrêté attend une réponse, pas une coche : réponds-y (answer_permission, answer_question) ou laisse-la à l’utilisateur.",
            "A stopped agent waits for an answer, not a tick: answer it (answer_permission, answer_question) or leave it to the user."
        )),
        ViewSource::Decision if settle && !from_ticket => Some(tr!(
            "Seule la décision d’un ticket que tu as fusionné se coche comme réglée : celle-ci ne vient d’aucun ticket, l’utilisateur l’accepte.",
            "Only the decision of a ticket you merged is ticked as settled: this one comes from no ticket, the user accepts it."
        )),
        _ => None,
    }
}

/// A mark's note, checked: one line, said, short, no secret.
fn mark_note(note: &str) -> Result<String> {
    let note = crate::asks::one_line(note);
    if note.is_empty() {
        bail!(tr!(
            "Dis en une phrase ce que tu fais ou comment c’est réglé (note) : l’utilisateur la lit dans sa boîte.",
            "Say in one sentence what you are doing or how it was settled (note): the user reads it in their box."
        ));
    }
    if note.chars().count() > MARK_NOTE_MAX {
        bail!(tr!(
            "La note tient en {MARK_NOTE_MAX} caractères.",
            "The note fits in {MARK_NOTE_MAX} characters."
        ));
    }
    if crate::asks::looks_secret(&note) {
        bail!(tr!(
            "Ce texte ressemble à un secret (une clé ou un jeton) : ne le mets pas dans la boîte.",
            "This text looks like a secret (a key or a token): don’t put it in the box."
        ));
    }
    Ok(note)
}

/// The day of a decision (`YYYY-MM-DD`), as epoch ms at midnight UTC; 0 when it says none.
fn day_ms(date: &str) -> i64 {
    chrono::NaiveDate::parse_from_str(date.trim(), "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|d| d.and_utc().timestamp_millis())
        .unwrap_or(0)
}

/// The first line that says something, cut to the 140 characters of a request's title.
pub(crate) fn first_line(s: &str) -> String {
    let line = s
        .lines()
        .map(|l| l.trim().trim_start_matches('#').trim())
        .find(|l| !l.is_empty())
        .unwrap_or("");
    let mut out: String = line.chars().take(140).collect();
    if line.chars().count() > 140 {
        out.pop();
        out.push('…');
    }
    out
}

fn kind_of(view: &PendingView) -> ViewSource {
    match (view.kind.as_str(), view.tool.as_str()) {
        ("question", _) => ViewSource::Question,
        (_, "ExitPlanMode") => ViewSource::Plan,
        _ => ViewSource::Permission,
    }
}

fn same_login(a: &str, b: &str) -> bool {
    !a.is_empty() && a.eq_ignore_ascii_case(b)
}

/// The order of the list: the blocking first, then the others, the oldest first; put off last.
fn ordered(views: &mut [InboxItem]) {
    views.sort_by(|a, b| {
        (
            a.snoozed_until.is_some(),
            a.mark.is_some(),
            !a.blocking(),
            a.since,
            &a.key,
        )
            .cmp(&(
                b.snoozed_until.is_some(),
                b.mark.is_some(),
                !b.blocking(),
                b.since,
                &b.key,
            ))
    });
}

impl Core {
    /// The views of every project, as the state has them now; their « Plus tard » applied, those
    /// whose time came back taken out of it.
    pub async fn inbox_view_items(self: &Arc<Self>) -> Vec<InboxItem> {
        let projects = self.projects.read().clone();
        let known: HashSet<&str> = projects.iter().map(|p| p.id.as_str()).collect();
        let of = |id: &str| {
            projects
                .iter()
                .find(|p| p.id == id)
                .map(|p| (p.id.clone(), p.name.clone(), p.private))
        };
        // The board's target branch of a project (ESC-552: a ticket's own is said when it differs).
        let board_of = |id: &str| {
            projects
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.board.target.trim())
        };
        let mut views = Vec::new();

        // The requests agents stopped for, as their cards in the conversation have them: those of a
        // process that is still there (in the Claude Code view, its terminal asks).
        let keys: BTreeMap<String, String> = self
            .tickets
            .read()
            .iter()
            .map(|t| (t.id.clone(), t.key.clone()))
            .collect();
        let handles: Vec<_> = self.agents.read().values().cloned().collect();
        for h in handles {
            let rt = h.lock();
            let m = &rt.meta;
            if m.archived || m.terminal || rt.proc.is_none() {
                continue;
            }
            let Some((pid, pname, private)) = of(&m.project_id) else {
                continue;
            };
            for w in rt.waiting() {
                let source = kind_of(&w.view);
                let mut v = InboxItem::new(
                    format!("req:{}:{}", m.id, w.view.id),
                    source,
                    ViewGroup::Stopped,
                    (&pid, &pname, private),
                );
                v.title = match source {
                    ViewSource::Question => w
                        .view
                        .questions
                        .first()
                        .map(|q| first_line(&q.question))
                        .unwrap_or_default(),
                    _ => first_line(&w.view.arg),
                };
                v.since = w.since;
                v.agent_id = Some(m.id.clone());
                v.agent_name = Some(m.name.clone());
                v.ticket_id = m.ticket_id.clone();
                v.ticket_key = m.ticket_id.as_ref().and_then(|t| keys.get(t).cloned());
                v.request = Some(w.view);
                views.push(v);
            }
        }

        // The tickets « À tester » and the blocked ones, as the board shows them.
        let tickets = self.tickets.read().clone();
        // The agents a teammate passes to this machine (TS-17).
        for t in &tickets {
            let Some(h) = t
                .team
                .as_ref()
                .and_then(|x| x.handoff.as_ref())
                .filter(|h| h.incoming)
            else {
                continue;
            };
            let Some((pid, pname, private)) = of(&t.project_id) else {
                continue;
            };
            let mut v = InboxItem::new(
                format!("handoff:{}", t.id),
                ViewSource::Handoff,
                ViewGroup::Today,
                (&pid, &pname, private),
            );
            v.title = first_line(&t.title);
            v.since = h.at;
            v.ticket_id = Some(t.id.clone());
            v.ticket_key = Some(t.key.clone());
            v.column = Some(t.column);
            v.agent_name = Some(h.agent.clone());
            v.by = Some(h.from_name.clone());
            v.engine = Some(h.engine.clone());
            v.branch = Some(h.branch.clone());
            views.push(v);
        }
        // The tickets of the team assigned to the user with a teammate's text not accepted (ESC-645): its key names the
        // writes read, so that one changed meanwhile is refused, never accepted unread.
        for t in &tickets {
            let Some(tt) = t
                .team
                .as_ref()
                .filter(|x| t.column == Column::Todo && x.awaits_accept())
            else {
                continue;
            };
            let Some((pid, pname, private)) = of(&t.project_id) else {
                continue;
            };
            let sign = tt.foreign.join(",");
            let mut v = InboxItem::new(
                format!(
                    "accept:{}:{}",
                    t.id,
                    crate::teamboard::text_print(&tt.foreign)
                ),
                ViewSource::TeamAccept,
                ViewGroup::Today,
                (&pid, &pname, private),
            );
            v.title = first_line(&t.title);
            v.since = t.created_at;
            v.ticket_id = Some(t.id.clone());
            v.ticket_key = Some(t.key.clone());
            v.column = Some(t.column);
            v.by = Some(if tt.written_by.is_empty() {
                tr!("un collègue", "a teammate")
            } else {
                tt.written_by.join(", ")
            });
            v.sign = sign;
            views.push(v);
        }
        // The scheduled tasks an agent made or changed (ESC-647): their key holds what the user reads, so that one
        // changed meanwhile is refused, never accepted unread.
        for sv in self.scheduled_views() {
            let st = &sv.task;
            if !st.awaiting {
                continue;
            }
            let Some((pid, pname, private)) = of(&st.project_id) else {
                continue;
            };
            let print = crate::scheduled::scheduled_print(st);
            let mut v = InboxItem::new(
                format!("scheduled:{}:{print}", st.id),
                ViewSource::Scheduled,
                ViewGroup::Today,
                (&pid, &pname, private),
            );
            v.title = first_line(&st.title);
            v.since = st.author.as_ref().map_or(st.created_at, |a| a.at);
            v.agent_id = st.author.as_ref().map(|a| a.agent_id.clone());
            v.agent_name = st.author.as_ref().map(|a| a.agent_name.clone());
            v.by = v.agent_name.clone();
            v.ticket_key = st.author.as_ref().and_then(|a| a.ticket_key.clone());
            let next = crate::scheduled::parse(&st.rhythm).ok().and_then(|c| {
                crate::scheduled::next_after(&c, self.schedule_clock.now(), &chrono::Local)
            });
            v.scheduled = Some(ScheduledAsk {
                task_id: st.id.clone(),
                rhythm: st.rhythm.clone(),
                cron: crate::scheduled::cron_of(&st.rhythm)
                    .unwrap_or_else(|_| st.rhythm.cron.clone()),
                description: st.description.clone(),
                criteria: st.criteria.clone(),
                catch_up: st.catch_up,
                next,
            });
            v.sign = print;
            views.push(v);
        }
        for t in &tickets {
            if t.shared.as_ref().is_some_and(|s| s.hidden) || t.column == Column::Done {
                continue;
            }
            let Some((pid, pname, private)) = of(&t.project_id) else {
                continue;
            };
            let (source, key) = if t.blocked.is_some() {
                (ViewSource::TicketBlocked, format!("blocked:{}", t.id))
            } else if t.column == Column::Review {
                (ViewSource::TicketReview, format!("ticket:{}", t.id))
            } else {
                continue;
            };
            let mut v = InboxItem::new(key, source, ViewGroup::Today, (&pid, &pname, private));
            v.title = first_line(&t.title);
            v.since = t.review_at.or(t.started_at).unwrap_or(t.created_at);
            v.ticket_id = Some(t.id.clone());
            v.ticket_key = Some(t.key.clone());
            v.column = Some(t.column);
            v.blocked = t.blocked.clone();
            v.conflict = t.conflict;
            v.partial = t.partial;
            v.target = Some(t.target.trim().to_string())
                .filter(|own| !own.is_empty() && Some(own.as_str()) != board_of(&t.project_id));
            v.other_line = t.other_line.clone();
            // What a mark holds of it: its time « À tester » (it comes back with another), or its
            // block as it is.
            v.sign = match source {
                ViewSource::TicketReview => format!("review:{}", t.review_at.unwrap_or(0)),
                _ => format!(
                    "blocked:{:?}:{}:{}",
                    t.column,
                    t.conflict,
                    t.blocked.as_deref().unwrap_or_default()
                ),
            };
            if let Some(a) = t.agent_id.as_deref().and_then(|a| self.agent(a).ok()) {
                let rt = a.lock();
                v.agent_id = Some(rt.meta.id.clone());
                v.agent_name = Some(rt.meta.name.clone());
            }
            views.push(v);
        }

        // The decisions proposed, of every project whose docs can be read.
        for (pid, pname, private) in projects.iter().map(|p| (&p.id, &p.name, p.private)) {
            let Ok(space) = self.docs_space(pid, None).await else {
                continue;
            };
            for d in space.decisions() {
                if d.state != crate::docs::State::Proposed || d.id.is_empty() {
                    continue;
                }
                let mut v = InboxItem::new(
                    format!("decision:{pid}:{}", d.id),
                    ViewSource::Decision,
                    ViewGroup::Earlier,
                    (pid, pname, private),
                );
                v.title = first_line(&d.title);
                v.since = day_ms(&d.date);
                v.decision = Some(d.id.clone());
                v.sign = format!("decision:{}:{}", d.date, d.title);
                if d.origin == "ticket" && !d.origin_ref.is_empty() {
                    v.ticket_key = Some(d.origin_ref.clone());
                    v.ticket_id = tickets
                        .iter()
                        .find(|t| t.project_id == *pid && t.key == d.origin_ref)
                        .map(|t| t.id.clone());
                }
                views.push(v);
            }
        }

        // The docs to share, as the Docs tab was last told of them.
        let shares: Vec<(String, u32)> = self
            .docs_shares
            .lock()
            .iter()
            .filter(|(_, m)| m.pending > 0 && m.pending != u32::MAX)
            .map(|(p, m)| (p.clone(), m.pending))
            .collect();
        for (p, n) in shares {
            let Some((pid, pname, private)) = of(&p) else {
                continue;
            };
            let mut v = InboxItem::new(
                format!("docs:{pid}"),
                ViewSource::Docs,
                ViewGroup::Earlier,
                (&pid, &pname, private),
            );
            v.files = n;
            views.push(v);
        }

        // The reviews the host asks of the user, in the lists as they were last read (never read
        // for the box: it says what the views « Pull requests » and « Vue d’ensemble » last saw).
        let lists: Vec<(String, crate::prview::ProjectPrs)> = self
            .pr_lists
            .lock()
            .iter()
            .filter(|(p, _)| known.contains(p.as_str()))
            .filter_map(|(p, l)| l.read.as_ref().map(|r| (p.clone(), r.2.clone())))
            .collect();
        for (p, list) in lists {
            let Some((pid, pname, private)) = of(&p) else {
                continue;
            };
            for pr in &list.prs {
                let open = pr.state == crate::forge::PrState::Open && !pr.draft;
                let asked = pr.reviewers.iter().any(|r| same_login(r, &list.me));
                if !open || !asked || same_login(&pr.author, &list.me) {
                    continue;
                }
                let mut v = InboxItem::new(
                    format!("review:{pid}:{}:{}", pr.host_repo.to_lowercase(), pr.number),
                    ViewSource::Review,
                    ViewGroup::Today,
                    (&pid, &pname, private),
                );
                v.title = first_line(&pr.title);
                v.since = (pr.created as i64).saturating_mul(1000);
                v.number = Some(pr.number);
                v.url = Some(pr.url.clone());
                v.word = Some(list.word.clone());
                views.push(v);
            }
        }

        // « Plus tard »: those whose time came back return; those whose view went are forgotten.
        // The squad lead's marks (ESC-557) fall with their view, or once its source changed.
        let now = now_ms();
        let present: HashSet<String> = views.iter().map(|v| v.key.clone()).collect();
        let signs: HashMap<String, String> = views
            .iter()
            .map(|v| (v.key.clone(), v.sign.clone()))
            .collect();
        let changed = {
            let mut s = self.inbox_snoozes.lock();
            let before = (s.until.len(), s.marks.len());
            s.until.retain(|k, t| *t > now && present.contains(k));
            s.marks
                .retain(|k, m| signs.get(k).is_some_and(|sign| *sign == m.sign));
            for v in views.iter_mut() {
                if !v.blocking() {
                    v.snoozed_until = s.until.get(&v.key).copied();
                }
                v.mark = s.marks.get(&v.key).cloned();
            }
            before != (s.until.len(), s.marks.len())
        };
        if changed {
            self.request_save();
        }
        for v in views.iter_mut().filter(|v| v.request.is_some()) {
            v.advice = self.overseer_advice(&v.key);
        }
        ordered(&mut views);
        views
    }

    /// `inbox_views`: the views of a project or of all, and the counts of every project. Nothing
    /// while the MCP server is off.
    pub async fn inbox_views(self: &Arc<Self>, project_id: Option<&str>) -> InboxViews {
        if !self.inbox_enabled() {
            return InboxViews {
                enabled: false,
                views: Vec::new(),
                blocking: 0,
                total: 0,
            };
        }
        let all = self.inbox_view_items().await;
        let blocking = all.iter().filter(|v| v.blocking()).count();
        let asks = self
            .asks
            .lock()
            .list
            .iter()
            .filter(|a| a.state == crate::asks::AskState::Open)
            .count();
        let total = asks + all.iter().filter(|v| v.waiting()).count();
        InboxViews {
            enabled: true,
            views: all
                .into_iter()
                .filter(|v| project_id.is_none_or(|p| v.project_id == p))
                .collect(),
            blocking,
            total,
        }
    }

    /// The view `key` as it is now; an error that says it was settled elsewhere when it is gone.
    pub(crate) async fn inbox_view(self: &Arc<Self>, key: &str) -> Result<InboxItem> {
        if !self.inbox_enabled() {
            bail!(tr!(
                "La boîte « À toi » est fermée avec le serveur MCP : allume-le dans Réglages › Agents › Piloter Escouade.",
                "The “Your turn” box is closed with the MCP server: turn it on in Settings › Agents › Drive Escouade."
            ));
        }
        self.inbox_view_items()
            .await
            .into_iter()
            .find(|v| v.key == key)
            .ok_or_else(|| {
                anyhow!(tr!(
                    "C’est déjà réglé ailleurs : la boîte se met à jour.",
                    "It was already settled elsewhere: the box is updating."
                ))
            })
    }

    /// `inbox_view_act`: a gesture on a view, the one of its source, on the view the user read
    /// (one settled meanwhile is refused). Its source then knows it: the agent goes on, the ticket
    /// moves, the decision is accepted, the docs are shared.
    pub async fn inbox_view_act(self: &Arc<Self>, key: &str, act: ViewAct) -> Result<()> {
        let v = self.inbox_view(key).await?;
        if !act.fits(&v) {
            bail!(tr!(
                "Ce geste ne va pas avec cette ligne de la boîte.",
                "This gesture doesn’t go with this line of the box."
            ));
        }
        let name = act.name();
        let ticket = || v.ticket_id.clone().unwrap_or_default();
        let request = || {
            (
                v.agent_id.clone().unwrap_or_default(),
                v.request.as_ref().map(|r| r.id.clone()).unwrap_or_default(),
            )
        };
        match act {
            ViewAct::Allow | ViewAct::Always => {
                let (agent, req) = request();
                let how = if act == ViewAct::Always {
                    "always"
                } else {
                    "allow"
                };
                self.answer_permission(&agent, &req, how, None)?;
            }
            ViewAct::Deny { message } => {
                let (agent, req) = request();
                self.answer_permission(&agent, &req, "deny", message)?;
            }
            ViewAct::Answer { answers } => {
                let (agent, req) = request();
                self.answer_question(&agent, &req, answers)?;
            }
            ViewAct::Approve => {
                self.ticket_approve(&ticket()).await?;
                self.telemetry
                    .track(crate::telemetry::Event::TicketApproved);
            }
            ViewAct::Reject { comment } => self.ticket_reject(&ticket(), &comment).await?,
            ViewAct::Resume => self.ticket_resume(&ticket()).await?,
            ViewAct::Resolve => self.ticket_resolve_conflict(&ticket()).await?,
            ViewAct::Dismiss => self.ticket_dismiss(&ticket())?,
            ViewAct::Retarget { target } => {
                self.ticket_set_target(&ticket(), &target).await?;
            }
            // A teammate's text accepted (ESC-645): the writes of this view, the ones the user read; changed since,
            // its key changed too and this one was refused above.
            ViewAct::Accept if v.source == ViewSource::TeamAccept => {
                let seen: Vec<String> = v
                    .sign
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect();
                self.team_accept(&ticket(), &seen)?;
            }
            // A task an agent scheduled (ESC-647), as the user read it: changed since, its key changed and this one
            // was refused above; the print is checked again under the tasks' lock.
            ViewAct::Accept if v.source == ViewSource::Scheduled => {
                let id = v
                    .scheduled
                    .as_ref()
                    .map(|s| s.task_id.clone())
                    .unwrap_or_default();
                self.scheduled_accept(&id, Some(&v.sign))?;
            }
            ViewAct::Accept => {
                let id = v.decision.clone().unwrap_or_default();
                self.docs_accept_decision(&v.project_id, &crate::docs::Ref::Id(id))
                    .await?;
                self.docs_changed(&v.project_id).await;
            }
            ViewAct::Share => {
                self.docs_share(&v.project_id, None, false).await?;
            }
            ViewAct::Take { engine } => {
                self.team_handoff_take(&ticket(), engine).await?;
            }
        }
        log::info!("inbox view {} {name}", v.source_name());
        self.telemetry
            .track(crate::telemetry::Event::InboxViewSettled {
                source: v.source_name(),
                act: name,
            });
        self.inbox_view_told(&v.project_id);
        Ok(())
    }

    /// « Plus tard » on a view: out of the box until `until` (ms), its source untouched; None
    /// brings it back now. Never a request an agent stopped for: its agent waits.
    pub async fn inbox_view_snooze(self: &Arc<Self>, key: &str, until: Option<i64>) -> Result<()> {
        let v = self.inbox_view(key).await?;
        if v.blocking() {
            bail!(tr!(
                "Ton agent s’est arrêté pour cette demande : elle ne se reporte pas, elle attend ta réponse.",
                "Your agent stopped for this request: it is not put off, it waits for your answer."
            ));
        }
        {
            let mut s = self.inbox_snoozes.lock();
            match until {
                Some(t) if t > now_ms() => {
                    s.until.insert(key.to_string(), t);
                }
                _ => {
                    s.until.remove(key);
                }
            }
        }
        self.request_save();
        self.inbox_view_told(&v.project_id);
        Ok(())
    }

    /// `claim_inbox_view` (ESC-557): the squad lead takes the view in hand, with what it does (« vérification en
    /// cours, chaîne de tests »). It no longer counts as waiting for the user and shows apart, its note and its time;
    /// the user may still act on it. It falls once its source changes, or given back (`inbox_view_release`).
    pub async fn inbox_view_claim(
        self: &Arc<Self>,
        key: &str,
        by: &Marker,
        note: &str,
    ) -> Result<InboxItem> {
        let v = self.inbox_view(key).await?;
        if let Some(why) = not_markable(v.source, v.ticket_key.is_some(), false) {
            bail!(why);
        }
        if v.mark.as_ref().is_some_and(|m| m.kind == MarkKind::Settled) {
            bail!(tr!(
                "Cette ligne est déjà cochée comme réglée : elle n’attend plus personne.",
                "This line is already ticked as settled: it no longer waits for anyone."
            ));
        }
        self.inbox_mark(v, by, note, MarkKind::Claimed)
    }

    /// `settle_inbox_view` (ESC-557): the squad lead ticks as settled a view it settled another way (a ticket merged by
    /// hand, the decision of such a ticket), with how. Out of the box, into its trace, whence the user puts it back
    /// (`inbox_view_reopen`); its source untouched. Never what stays the user's gesture.
    pub async fn inbox_view_settle(
        self: &Arc<Self>,
        key: &str,
        by: &Marker,
        note: &str,
    ) -> Result<InboxItem> {
        let v = self.inbox_view(key).await?;
        if let Some(why) = not_markable(v.source, v.ticket_key.is_some(), true) {
            bail!(why);
        }
        self.inbox_mark(v, by, note, MarkKind::Settled)
    }

    fn inbox_mark(
        self: &Arc<Self>,
        mut v: InboxItem,
        by: &Marker,
        note: &str,
        kind: MarkKind,
    ) -> Result<InboxItem> {
        let mark = ViewMark {
            kind,
            note: mark_note(note)?,
            by: by.agent.clone(),
            by_name: by.name.clone(),
            at: now_ms(),
            sign: v.sign.clone(),
        };
        self.inbox_snoozes
            .lock()
            .marks
            .insert(v.key.clone(), mark.clone());
        v.mark = Some(mark);
        self.request_save();
        log::info!(
            "inbox view {} {}",
            v.source_name(),
            match kind {
                MarkKind::Claimed => "claimed",
                MarkKind::Settled => "settled",
            }
        );
        self.inbox_view_told(&v.project_id);
        Ok(v)
    }

    /// The squad lead gives back a view it took in hand: it waits for the user again.
    pub async fn inbox_view_release(self: &Arc<Self>, key: &str) -> Result<InboxItem> {
        let mut v = self.inbox_view(key).await?;
        if v.mark.as_ref().is_none_or(|m| m.kind != MarkKind::Claimed) {
            bail!(tr!(
                "Personne ne s’occupe de cette ligne : il n’y a rien à rendre.",
                "Nobody is on this line: there is nothing to give back."
            ));
        }
        self.inbox_unmark(&v);
        v.mark = None;
        log::info!("inbox view {} released", v.source_name());
        Ok(v)
    }

    /// `inbox_view_reopen`: the user puts back in the box a view the squad lead marked (« Remettre dans la boîte »).
    pub async fn inbox_view_reopen(self: &Arc<Self>, key: &str) -> Result<()> {
        let v = self.inbox_view(key).await?;
        if v.mark.is_none() {
            return Ok(());
        }
        self.inbox_unmark(&v);
        log::info!("inbox view {} put back", v.source_name());
        Ok(())
    }

    fn inbox_unmark(&self, v: &InboxItem) {
        self.inbox_snoozes.lock().marks.remove(&v.key);
        self.request_save();
        self.inbox_view_told(&v.project_id);
    }

    /// The window reads the box again: a view of this project changed.
    pub(crate) fn inbox_view_told(&self, project_id: &str) {
        self.hub.emit(UiEvent::Inbox {
            project_id: project_id.to_string(),
        });
    }
}

impl InboxItem {
    /// Its source, as the log and the statistics say it.
    pub fn source_name(&self) -> &'static str {
        match self.source {
            ViewSource::Permission => "permission",
            ViewSource::Question => "question",
            ViewSource::Plan => "plan",
            ViewSource::TicketReview => "ticketReview",
            ViewSource::TicketBlocked => "ticketBlocked",
            ViewSource::Decision => "decision",
            ViewSource::Docs => "docs",
            ViewSource::Review => "review",
            ViewSource::Handoff => "handoff",
            ViewSource::TeamAccept => "teamAccept",
            ViewSource::Scheduled => "scheduled",
        }
    }
}

/// The facts that change a view's source tell the window to read the box again: a request asked
/// or answered, an agent's status, a ticket's column or conflict, a decision proposed or accepted,
/// the docs written. The board's and the conversation's own events still tell their places.
pub struct ViewsSink {
    pub core: Weak<Core>,
}

impl crate::facts::Sink for ViewsSink {
    fn take(&self, fact: &Arc<crate::facts::Fact>) {
        use crate::facts::Body as B;
        let moves = matches!(
            fact.body,
            B::AgentStatus { .. }
                | B::AgentStopped { .. }
                | B::RequestPermission { .. }
                | B::RequestQuestion { .. }
                | B::RequestAnswered { .. }
                | B::TicketColumn { .. }
                | B::TicketApproved {}
                | B::TicketRejected { .. }
                | B::TicketConflict { .. }
                | B::TestsResult { .. }
                | B::DecisionProposed { .. }
                | B::DecisionAccepted { .. }
                | B::DocsWritten { .. }
                | B::DocsFiled { .. }
        );
        let Some(project) = fact.project.as_ref().filter(|_| moves) else {
            return;
        };
        // What the squad lead said of a ticket's views falls with its move itself (`ViewSnoozes::forget_ticket`), not
        // here: this fact comes later, and may come after a mark left on the view the move made (ESC-621).
        if let Some(core) = self.core.upgrade() {
            if core.inbox_enabled() {
                core.inbox_view_told(&project.id);
            }
        }
    }
}
