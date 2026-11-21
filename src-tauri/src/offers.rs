//! The proposals at the right moment (F-3, ESC-558, `docs/FEATURES.md` § 2, spec « Fonctionnalités »): when Escouade
//! sees that a feature switched off would serve, it proposes it in a small window at the bottom right, one at a time:
//! what it noticed, what the feature would do, and « Activer », « Pas maintenant », « Ne plus proposer ».
//!
//! **The signals** (choices of the chef, thresholds fixed here and said in the decision):
//!
//! | Feature off | Signal | Threshold |
//! | --- | --- | --- |
//! | Kanban | calls of `create_ticket` refused by the MCP server | [`MCP_TICKETS`] in [`MCP_TICKETS_WITHIN`] |
//! | Kanban | agents of the user's started by hand on a long first message | [`LONG_PROMPTS`] of [`LONG_PROMPT_CHARS`]+ characters in [`LONG_PROMPTS_WITHIN`] |
//! | Espace Docs | `SPEC.md` or `docs/` changed in commits of a project | [`SPEC_DAYS`] different days in [`SPEC_WITHIN`] |
//! | Espace Docs | documents dropped in the conversations | [`DOCUMENTS`] in [`DOCUMENTS_WITHIN`] |
//! | Coût | a usage limit met | [`LIMITS`] in [`LIMITS_WITHIN`] |
//! | Workflows | skills, commands or agents of a project (`.claude/`, `.agents/`) | [`AGENT_FILES`] in one project |
//! | Tâches programmées | the same message sent by hand at about the same hour | [`SAME_MESSAGE_DAYS`] days in a row, within [`SAME_MESSAGE_SPAN_MIN`] minutes |
//! | Overseer | agents waiting for an answer on several projects at once | [`WAITING_PROJECTS`] projects, seen in [`WAITING_WITHIN`] |
//!
//! **The rules**: one proposal at a time, one a day at most ([`ONE_A_DAY_MS`]); never during a turn on screen nor
//! while an agent waits for a permission (the core's, [`Moment`]), nor while the user types (the window's); « Pas
//! maintenant » not the same one again for a week ([`LATER_MS`]); « Ne plus proposer » for good, given back by Réglages
//! › Fonctionnalités › « Propositions écartées ». A feature imposed (`features.<id>` of a managed device or a team) is
//! never proposed. With the three bubbles of the discovery (ESC-506), one thing at the right moment per session: the
//! window's (`src/lib/offers.svelte.ts`).
//!
//! **Nothing leaves the machine**: what is noticed is kept in `state.json` (`Onboarding::offers`), two weeks at most,
//! without a text (a message is kept as its fingerprint alone), and counted in no statistic.

use crate::core::Core;
use crate::features::{Feature, Features};
use crate::model::{now_ms, AgentStatus, Settings, UiEvent};
pub use escouade_model::features::{FeatureOffers, MessageSeen, WaitingSeen};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::Arc;

pub const MINUTE_MS: i64 = 60_000;
pub const DAY_MS: i64 = 86_400_000;
/// One proposal a day at most: none in the 24 hours after the last one shown.
pub const ONE_A_DAY_MS: i64 = DAY_MS;
/// « Pas maintenant »: not the same one again for a week.
pub const LATER_MS: i64 = 7 * DAY_MS;
/// What is noticed is kept two weeks, no more.
pub const KEPT_MS: i64 = 14 * DAY_MS;

/// Kanban off: two calls of `create_ticket` refused by the MCP server in a week.
pub const MCP_TICKETS: usize = 2;
pub const MCP_TICKETS_WITHIN: i64 = 7 * DAY_MS;
/// Kanban off: three agents started by hand on a first message of 300 characters or more, in a week.
pub const LONG_PROMPTS: usize = 3;
pub const LONG_PROMPT_CHARS: usize = 300;
pub const LONG_PROMPTS_WITHIN: i64 = 7 * DAY_MS;
/// Espace Docs off: `SPEC.md` or `docs/` changed by commits on three different days of the last two weeks.
pub const SPEC_DAYS: usize = 3;
pub const SPEC_WITHIN: i64 = 14 * DAY_MS;
/// Espace Docs off: three documents dropped in the conversations in two weeks.
pub const DOCUMENTS: usize = 3;
pub const DOCUMENTS_WITHIN: i64 = 14 * DAY_MS;
/// Coût off: one usage limit met in a week.
pub const LIMITS: usize = 1;
pub const LIMITS_WITHIN: i64 = 7 * DAY_MS;
/// Workflows off: two skills, commands or agents in one project.
pub const AGENT_FILES: usize = 2;
/// Tâches programmées off: the same message (20 characters or more) sent by hand three days in a row, all within an
/// hour of the day.
pub const SAME_MESSAGE_DAYS: usize = 3;
pub const SAME_MESSAGE_SPAN_MIN: i64 = 60;
pub const SAME_MESSAGE_CHARS: usize = 20;
/// Overseer off: agents waiting for an answer on three projects at once, seen in the last week.
pub const WAITING_PROJECTS: u32 = 3;
pub const WAITING_WITHIN: i64 = 7 * DAY_MS;
/// How long a reading of a project's commits serves before it is read again.
const SPEC_READ_MS: i64 = 60 * MINUTE_MS;

/// The files that count as documents dropped in a conversation (an image does not).
pub const DOCUMENT_EXTENSIONS: &[&str] = &[
    "pdf", "doc", "docx", "odt", "rtf", "md", "markdown", "txt", "ppt", "pptx", "odp", "xls",
    "xlsx", "ods", "csv",
];

/// Where a project keeps its skills, commands and agents.
pub const AGENT_DIRS: &[&str] = &[".claude", ".agents"];

/// What Escouade noticed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Signal {
    McpTickets,
    LongPrompts,
    SpecEdits,
    Documents,
    UsageLimit,
    AgentFiles,
    SameMessage,
    WaitingProjects,
}

impl Signal {
    /// The feature it proposes.
    pub fn feature(self) -> Feature {
        match self {
            Signal::McpTickets | Signal::LongPrompts => Feature::Kanban,
            Signal::SpecEdits | Signal::Documents => Feature::Docs,
            Signal::UsageLimit => Feature::Cost,
            Signal::AgentFiles => Feature::Workflows,
            Signal::SameMessage => Feature::Scheduled,
            Signal::WaitingProjects => Feature::Overseer,
        }
    }
}

/// A proposal: the feature, what was noticed and how much (`count`), and the project it was noticed in when it is one's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Offer {
    pub feature: Feature,
    pub signal: Signal,
    pub count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
}

/// The proposals as a client shows them: the one to show now, if any, and those set apart for good.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferView {
    pub offer: Option<Offer>,
    /// « Ne plus proposer », in the order of the register: Réglages › Fonctionnalités › « Propositions écartées ».
    pub never: Vec<Feature>,
    /// The settings as « Activer » left them: the client takes them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Box<Settings>>,
}

/// What a client tells of a proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum OfferAct {
    /// It is on screen: the day's one.
    Shown { feature: Feature },
    /// « Activer »: the feature on.
    Enable { feature: Feature },
    /// « Pas maintenant »: not this one again for a week.
    Later { feature: Feature },
    /// « Ne plus proposer »: never again.
    Never { feature: Feature },
    /// « Reproposer » of « Propositions écartées »: it may come again.
    Restore { feature: Feature },
}

/// What is read of the projects now, not kept: the most a project has.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Live {
    /// The days its spec changed in the last two weeks, and the project.
    pub spec_days: Option<(u32, String)>,
    /// Its skills, commands and agents, and the project.
    pub agent_files: Option<(u32, String)>,
}

/// The moment: never a proposal while a turn is on screen or an agent waits for a permission.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Moment {
    pub turn: bool,
    pub permission: bool,
}

/// How many of `at` are in the `span` before `now`.
fn within(at: &[i64], now: i64, span: i64) -> usize {
    at.iter()
        .filter(|t| **t <= now && now - **t <= span)
        .count()
}

/// The local day of `at` (days since the epoch), `offset_min` the local time's offset from UTC.
fn day_of(at: i64, offset_min: i64) -> i64 {
    (at + offset_min * MINUTE_MS).div_euclid(DAY_MS)
}

/// The local minute of the day of `at`.
fn minute_of(at: i64, offset_min: i64) -> i64 {
    (at + offset_min * MINUTE_MS).rem_euclid(DAY_MS) / MINUTE_MS
}

/// The different local days of `at` in the `span` before `now`.
pub fn days_within(at: &[i64], now: i64, span: i64, offset_min: i64) -> usize {
    at.iter()
        .filter(|t| **t <= now && now - **t <= span)
        .map(|t| day_of(*t, offset_min))
        .collect::<BTreeSet<_>>()
        .len()
}

/// The longest run of days in a row on which the same message was sent by hand, all within
/// [`SAME_MESSAGE_SPAN_MIN`] minutes of the day (the first sending of each day counts).
pub fn same_message_days(seen: &[MessageSeen], now: i64, offset_min: i64) -> usize {
    let mut by_hash: HashMap<&str, BTreeMap<i64, i64>> = HashMap::new();
    for m in seen.iter().filter(|m| m.at <= now && now - m.at <= KEPT_MS) {
        let days = by_hash.entry(&m.hash).or_default();
        let (day, minute) = (day_of(m.at, offset_min), minute_of(m.at, offset_min));
        let first = days.entry(day).or_insert(minute);
        *first = (*first).min(minute);
    }
    let mut best = 0;
    for days in by_hash.values() {
        let days: Vec<(i64, i64)> = days.iter().map(|(d, m)| (*d, *m)).collect();
        // Every run of days in a row whose minutes stay within the span.
        for start in 0..days.len() {
            let (mut lo, mut hi) = (days[start].1, days[start].1);
            let mut len = 1;
            for i in start + 1..days.len() {
                if days[i].0 != days[i - 1].0 + 1 {
                    break;
                }
                let (l, h) = (lo.min(days[i].1), hi.max(days[i].1));
                if h - l > SAME_MESSAGE_SPAN_MIN {
                    break;
                }
                (lo, hi, len) = (l, h, len + 1);
            }
            best = best.max(len);
        }
    }
    best
}

/// A message's fingerprint: its words, in lower case, hashed (FNV-1a); None for a message too short to be a
/// consigne (« continue », « ok »). The text is never kept.
pub fn fingerprint(text: &str) -> Option<String> {
    let words = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    if words.chars().count() < SAME_MESSAGE_CHARS {
        return None;
    }
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in words.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    Some(format!("{h:016x}"))
}

/// Whether the file `name` is a document (`DOCUMENT_EXTENSIONS`).
pub fn is_document(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| DOCUMENT_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// The skills (a folder with its `SKILL.md`), commands and agents (a `.md`) of the project in `dir`, under `.claude/`
/// and `.agents/`; the settings and the rest do not count.
pub fn agent_files(dir: &Path) -> u32 {
    let mut n = 0;
    for root in AGENT_DIRS {
        let root = dir.join(root);
        if let Ok(skills) = std::fs::read_dir(root.join("skills")) {
            n += skills
                .flatten()
                .filter(|e| e.path().join("SKILL.md").is_file())
                .count() as u32;
        }
        for kind in ["commands", "agents"] {
            if let Ok(files) = std::fs::read_dir(root.join(kind)) {
                n += files
                    .flatten()
                    .filter(|e| {
                        let p = e.path();
                        p.is_file() && p.extension().is_some_and(|x| x == "md")
                    })
                    .count() as u32;
            }
        }
    }
    n
}

/// What was noticed of `f`, switched off, when it reaches its threshold; the first signal of the feature that does.
pub fn noticed(
    f: Feature,
    saved: &FeatureOffers,
    live: &Live,
    now: i64,
    offset_min: i64,
) -> Option<Offer> {
    let offer = |signal: Signal, count: usize, project: Option<&String>| Offer {
        feature: f,
        signal,
        count: count as u32,
        project: project.cloned(),
    };
    match f {
        Feature::Kanban => {
            let tickets = within(&saved.mcp_tickets, now, MCP_TICKETS_WITHIN);
            if tickets >= MCP_TICKETS {
                return Some(offer(Signal::McpTickets, tickets, None));
            }
            let prompts = within(&saved.long_prompts, now, LONG_PROMPTS_WITHIN);
            (prompts >= LONG_PROMPTS).then(|| offer(Signal::LongPrompts, prompts, None))
        }
        Feature::Docs => {
            if let Some((days, project)) = &live.spec_days {
                if *days as usize >= SPEC_DAYS {
                    return Some(offer(Signal::SpecEdits, *days as usize, Some(project)));
                }
            }
            let documents = within(&saved.documents, now, DOCUMENTS_WITHIN);
            (documents >= DOCUMENTS).then(|| offer(Signal::Documents, documents, None))
        }
        Feature::Cost => {
            let limits = within(&saved.limits, now, LIMITS_WITHIN);
            (limits >= LIMITS).then(|| offer(Signal::UsageLimit, limits, None))
        }
        Feature::Workflows => live
            .agent_files
            .as_ref()
            .filter(|(n, _)| *n as usize >= AGENT_FILES)
            .map(|(n, p)| offer(Signal::AgentFiles, *n as usize, Some(p))),
        Feature::Scheduled => {
            let days = same_message_days(&saved.messages, now, offset_min);
            (days >= SAME_MESSAGE_DAYS).then(|| offer(Signal::SameMessage, days, None))
        }
        Feature::Overseer => saved
            .waiting
            .iter()
            .filter(|w| {
                w.at <= now && now - w.at <= WAITING_WITHIN && w.projects >= WAITING_PROJECTS
            })
            .map(|w| w.projects)
            .max()
            .map(|n| offer(Signal::WaitingProjects, n as usize, None)),
        // No signal of their own (yet): never proposed.
        Feature::Editor | Feature::Stats | Feature::Voice => None,
    }
}

/// Whether `f` may be proposed at all: off, not imposed, not set apart for good nor for the week. Tâches programmées
/// is proposed only with the Kanban on (its runs are tickets): off with it, the Kanban comes first.
pub fn proposable(
    f: Feature,
    saved: &FeatureOffers,
    features: &Features,
    imposed: &dyn Fn(Feature) -> bool,
    now: i64,
) -> bool {
    if features.get(f) || f.needs().is_some_and(|n| !features.on(n)) {
        return false;
    }
    if imposed(f) || f.needs().is_some_and(imposed) {
        return false;
    }
    if saved.never.contains(&f) {
        return false;
    }
    !saved.later.get(&f).is_some_and(|t| now - *t < LATER_MS)
}

/// The proposal to show now, if any: one at a time, the first of the register whose signal holds; none the day after
/// one was shown, nor while a turn is on screen or an agent waits for a permission.
pub fn candidate(
    saved: &FeatureOffers,
    features: &Features,
    imposed: &dyn Fn(Feature) -> bool,
    live: &Live,
    moment: Moment,
    now: i64,
    offset_min: i64,
) -> Option<Offer> {
    if saved.last_shown.is_some_and(|t| now - t < ONE_A_DAY_MS) {
        return None;
    }
    if moment.turn || moment.permission {
        return None;
    }
    Feature::ALL
        .into_iter()
        .filter(|f| proposable(*f, saved, features, imposed, now))
        .find_map(|f| noticed(f, saved, live, now, offset_min))
}

/// What is older than two weeks goes.
pub fn prune(saved: &mut FeatureOffers, now: i64) {
    let keep = |t: &i64| now - *t <= KEPT_MS;
    saved.long_prompts.retain(keep);
    saved.mcp_tickets.retain(keep);
    saved.documents.retain(keep);
    saved.limits.retain(keep);
    saved.waiting.retain(|w| keep(&w.at));
    saved.messages.retain(|m| keep(&m.at));
}

/// `act` on `saved` (« Activer » turns the feature on apart, `Core::offer_act`). Whether anything changed.
pub fn apply(saved: &mut FeatureOffers, act: OfferAct, now: i64) -> bool {
    let before = saved.clone();
    match act {
        OfferAct::Shown { .. } => saved.last_shown = Some(now),
        OfferAct::Enable { feature } => {
            saved.later.remove(&feature);
        }
        OfferAct::Later { feature } => {
            saved.later.insert(feature, now);
        }
        OfferAct::Never { feature } => {
            saved.later.remove(&feature);
            saved.never.insert(feature);
        }
        OfferAct::Restore { feature } => {
            saved.never.remove(&feature);
        }
    }
    *saved != before
}

/// The local time's offset from UTC, in minutes.
fn local_offset_min() -> i64 {
    i64::from(chrono::Local::now().offset().local_minus_utc()) / 60
}

/// The commits of the projects' spec read last: by folder, when, and their times (ms).
static SPEC_READS: parking_lot::Mutex<BTreeMap<String, (i64, Vec<i64>)>> =
    parking_lot::Mutex::new(BTreeMap::new());

/// When the commits of `dir` changed `SPEC.md` or `docs/` in the last two weeks (ms); read again once an hour.
async fn spec_commits(dir: &str, now: i64) -> Vec<i64> {
    if let Some((at, times)) = SPEC_READS.lock().get(dir) {
        if now - at < SPEC_READ_MS {
            return times.clone();
        }
    }
    let since = format!("--max-age={}", (now - SPEC_WITHIN) / 1000);
    let times: Vec<i64> = crate::git::text(
        dir,
        &[
            "log",
            &since,
            "--format=%ct",
            "--",
            ":(icase)SPEC.md",
            "docs",
        ],
    )
    .await
    .unwrap_or_default()
    .lines()
    .filter_map(|l| l.trim().parse::<i64>().ok())
    .map(|s| s * 1000)
    .collect();
    SPEC_READS
        .lock()
        .insert(dir.to_owned(), (now, times.clone()));
    times
}

impl Core {
    /// Whether the policies (the device's, a team's) impose `f`, on or off.
    fn feature_imposed(&self, f: Feature) -> bool {
        self.locks.read().get(f.policy()).is_some()
    }

    /// `change` on what the proposals keep, then saved and told to the window when it changed.
    fn offers_change(&self, change: impl FnOnce(&mut FeatureOffers, i64) -> bool) -> bool {
        let now = now_ms();
        let changed = {
            let mut saved = self.onboarding.write();
            prune(&mut saved.offers, now);
            change(&mut saved.offers, now)
        };
        if changed {
            self.request_save();
            self.hub.emit(UiEvent::FeatureOffer {});
        }
        changed
    }

    /// `signal` noticed now, while its feature is off (nothing is kept of a feature that is on).
    pub fn offer_note(&self, signal: Signal) {
        if self.feature_on(signal.feature()) {
            return;
        }
        self.offers_change(|o, now| {
            match signal {
                Signal::McpTickets => o.mcp_tickets.push(now),
                Signal::LongPrompts => o.long_prompts.push(now),
                Signal::Documents => o.documents.push(now),
                Signal::UsageLimit => o.limits.push(now),
                // Read from the projects, or with what they need (`offer_note_message`, `offer_note_waiting`).
                Signal::SpecEdits
                | Signal::AgentFiles
                | Signal::SameMessage
                | Signal::WaitingProjects => return false,
            }
            true
        });
    }

    /// A message the user sends by hand to the agent `id` (the window, a client of the remote API), before it goes: a
    /// long first message to an agent of the user's (Kanban), the documents dropped with it (Espace Docs), and its
    /// fingerprint (Tâches programmées), for the features that are off.
    pub fn offer_note_message(&self, id: &str, text: &str, files: &[String]) {
        let Ok(h) = self.agent(id) else {
            return;
        };
        let (users, first) = {
            let rt = h.lock();
            let m = &rt.meta;
            (
                m.ticket_id.is_none() && m.discussion.is_none() && m.created_by.is_none(),
                m.prompts == 0,
            )
        };
        let long = !self.feature_on(Feature::Kanban)
            && users
            && first
            && text.trim().chars().count() >= LONG_PROMPT_CHARS;
        let documents = if self.feature_on(Feature::Docs) {
            0
        } else {
            files.iter().filter(|f| is_document(f)).count()
        };
        let print = (!self.feature_on(Feature::Scheduled))
            .then(|| fingerprint(text))
            .flatten();
        if !long && documents == 0 && print.is_none() {
            return;
        }
        self.offers_change(|o, now| {
            if long {
                o.long_prompts.push(now);
            }
            o.documents.extend(std::iter::repeat_n(now, documents));
            if let Some(hash) = print {
                o.messages.push(MessageSeen { hash, at: now });
            }
            true
        });
    }

    /// Agents waiting for an answer on several projects at once (Overseer off): seen now, kept when they reach the
    /// threshold, once in ten minutes at most.
    pub fn offer_note_waiting(&self) {
        if self.feature_on(Feature::Overseer) {
            return;
        }
        let projects = self
            .agents
            .read()
            .values()
            .filter_map(|h| {
                let rt = h.lock();
                (!rt.meta.archived && rt.waiting_views().next().is_some())
                    .then(|| rt.meta.project_id.clone())
            })
            .collect::<BTreeSet<_>>()
            .len() as u32;
        if projects < WAITING_PROJECTS {
            return;
        }
        self.offers_change(|o, now| {
            if o.waiting
                .last()
                .is_some_and(|w| now - w.at < 10 * MINUTE_MS && w.projects >= projects)
            {
                return false;
            }
            o.waiting.push(WaitingSeen { at: now, projects });
            true
        });
    }

    /// What is read of the projects now, for the features that are off: their spec's commits (Espace Docs) and their
    /// skills, commands and agents (Workflows).
    async fn offers_live(&self, now: i64, offset_min: i64) -> Live {
        let projects = self.projects.read().clone();
        let mut live = Live::default();
        if !self.feature_on(Feature::Docs) {
            for p in &projects {
                let days = days_within(
                    &spec_commits(&p.path, now).await,
                    now,
                    SPEC_WITHIN,
                    offset_min,
                ) as u32;
                if days > 0 && live.spec_days.as_ref().is_none_or(|(d, _)| days > *d) {
                    live.spec_days = Some((days, p.name.clone()));
                }
            }
        }
        if !self.feature_on(Feature::Workflows) {
            for p in &projects {
                let n = agent_files(Path::new(&p.path));
                if n > 0 && live.agent_files.as_ref().is_none_or(|(m, _)| n > *m) {
                    live.agent_files = Some((n, p.name.clone()));
                }
            }
        }
        live
    }

    /// The moment as the core sees it: a turn of an agent on screen (`visible`, the client's), an agent waiting for a
    /// permission.
    fn offer_moment(&self, visible: &[String]) -> Moment {
        let agents = self.agents.read();
        let permission = agents
            .values()
            .any(|h| h.lock().waiting_views().any(|v| v.kind == "permission"));
        let turn = visible
            .iter()
            .filter_map(|id| agents.get(id))
            .any(|h| h.lock().meta.status == AgentStatus::Running);
        Moment { turn, permission }
    }

    fn offer_view(&self, offer: Option<Offer>, settings: Option<Box<Settings>>) -> OfferView {
        let never = self.onboarding.read().offers.never.clone();
        OfferView {
            offer,
            never: Feature::ALL
                .into_iter()
                .filter(|f| never.contains(f))
                .collect(),
            settings,
        }
    }

    /// The proposal to show now, if any, for a client whose screen shows the agents `visible`.
    pub async fn feature_offer(&self, visible: &[String]) -> OfferView {
        let (now, offset) = (now_ms(), local_offset_min());
        self.offer_note_waiting();
        let live = self.offers_live(now, offset).await;
        let moment = self.offer_moment(visible);
        let features = self.settings.read().features.clone().unwrap_or_default();
        // Read before the proposals' lock: one lock at a time.
        let imposed: BTreeSet<Feature> = Feature::ALL
            .into_iter()
            .filter(|f| self.feature_imposed(*f))
            .collect();
        let offer = {
            let mut saved = self.onboarding.write();
            prune(&mut saved.offers, now);
            candidate(
                &saved.offers,
                &features,
                &|f| imposed.contains(&f),
                &live,
                moment,
                now,
                offset,
            )
        };
        self.offer_view(offer, None)
    }

    /// `act` on a proposal. « Activer » turns the feature on as Réglages › Fonctionnalités does (refused when a policy
    /// imposes it), with the Kanban for the scheduled tasks, and gives the settings back.
    pub fn feature_offer_act(self: &Arc<Self>, act: OfferAct) -> anyhow::Result<OfferView> {
        let mut settings = None;
        if let OfferAct::Enable { feature } = act {
            let mut s = self.settings.read().clone();
            let switches = s.features.get_or_insert_with(Default::default);
            switches.set(feature, true);
            if let Some(need) = feature.needs() {
                switches.set(need, true);
            }
            self.save_settings(s)?;
            settings = Some(Box::new(self.settings.read().clone()));
        }
        self.offers_change(|o, now| apply(o, act, now));
        Ok(self.offer_view(None, settings))
    }

    /// The proposals as they are, without one to show (Réglages › Fonctionnalités).
    pub fn feature_offers(&self) -> OfferView {
        self.offer_view(None, None)
    }
}

#[cfg(test)]
#[path = "offers_tests.rs"]
mod tests;
