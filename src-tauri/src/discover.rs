//! The discovery of the features (ESC-506, ON-3, `docs/ONBOARDING.md` § 7, direction C « La ligne »): after the first
//! launch, what Escouade can do is found by doing it, one thing at a time. Each discovery is one sentence and one real
//! gesture, and it is ticked by the real state of the app, never by a click on its card.
//!
//! What is kept, by installation, in `state.json` (`model::Onboarding`): each discovery done (by the real state, kept
//! done once it was) or dismissed (« Écarter »: never offered again by itself), « Tout masquer », the bubbles already
//! shown (never twice), and the last release whose band « Nouveau dans la … » was seen.
//!
//! A discovery shows only where it can be done: no voice without a microphone or when a policy cuts the dictation, no
//! team without a team server, nothing a policy denies, nothing this version does not have yet.

use crate::core::Core;
use crate::model::{DiscoverOutcome, Discovery, Onboarding, UiEvent};
use crate::telemetry::Event;
use serde::{Deserialize, Serialize};

/// The moments of the cycle, the map's lanes (Planifier, Coder, Relire, Livrer, Piloter), each with its discoveries in
/// their order.
pub const LANES: [(Lane, &[Discovery]); 5] = [
    (
        Lane::Plan,
        &[
            Discovery::Ticket,
            Discovery::Autopilot,
            Discovery::Docs,
            Discovery::Sources,
        ],
    ),
    (
        Lane::Code,
        &[
            Discovery::Dictation,
            Discovery::Voice,
            Discovery::Terminal,
            Discovery::Editor,
            Discovery::SecondAgent,
        ],
    ),
    (
        Lane::Review,
        &[
            Discovery::Launch,
            Discovery::Forge,
            Discovery::PrView,
            Discovery::PrReview,
        ],
    ),
    (Lane::Ship, &[Discovery::Approve, Discovery::OpenPr]),
    (
        Lane::Drive,
        &[
            Discovery::Plugin,
            Discovery::Mcp,
            Discovery::Account,
            Discovery::Remote,
            Discovery::Team,
        ],
    ),
];

/// The release whose novelties the band « Nouveau dans la 2.3 » tells, and those novelties.
pub const NEW_RELEASE: &str = "2.3";
pub const NEW_IN_RELEASE: [Discovery; 5] = [
    Discovery::Editor,
    Discovery::PrView,
    Discovery::PrReview,
    Discovery::Team,
    Discovery::Remote,
];

/// Escouade remote has no setting of its own in this version (the remote API opens for the tests alone, until the server
/// mode has its setting): its discovery is not shown.
pub const REMOTE_IN_VERSION: bool = false;

/// The discoveries the window tells itself, from what it shows (no state of the core says them): the tab Docs opened, a
/// file opened in the editor, a pull request opened in the app.
pub const SEEN_BY_WINDOW: [Discovery; 3] = [Discovery::Docs, Discovery::Editor, Discovery::PrView];

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Lane {
    Plan,
    Code,
    Review,
    Ship,
    Drive,
}

/// What the discovery reads of the app (`Core::discover_facts`): the real state, the policies, the hardware.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    pub fresh: bool,
    pub projects: usize,
    pub tickets: usize,
    /// A ticket in « Terminé »: one was validated.
    pub ticket_done: bool,
    /// A ticket with its pull request.
    pub ticket_pr: bool,
    /// The most agents of the user's own in one project (not a ticket's, nor a discussion's).
    pub agents_in_a_project: usize,
    pub accounts: usize,
    /// A code host connected (an account of GitHub, GitLab…).
    pub forge_connected: bool,
    /// An account of a source of tickets (ESC-512): its own, or the code host's that serves GitLab Issues, Azure Boards.
    pub sources_connected: bool,
    pub pr_reviews: usize,
    pub mcp_on: bool,
    /// A project of a team.
    pub team_project: bool,
    /// A team server this device knows (`team.server`, `servers.allowed`).
    pub team_server_known: bool,
    /// The microphones of this machine; None: not read (a client without the voice).
    pub microphones: Option<usize>,
    /// Why a policy denies a discovery, by discovery (its words, said by the policy).
    pub denied: Vec<(Discovery, String)>,
    /// The discoveries of the features switched off (F-1, `features::discoveries`): gone from the map.
    pub off: Vec<Discovery>,
}

/// Why a discovery is not on this device.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Absent {
    /// No microphone on this machine.
    NoMicrophone,
    /// No team server known: an invitation brings one.
    NoTeamServer,
    /// A policy denies it, in its words.
    Policy { why: String },
}

/// Where a discovery stands.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DiscoverStatus {
    /// To try.
    Todo,
    Done,
    Dismissed,
    /// Not on this device (`absent` says why).
    Absent,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverItem {
    pub id: Discovery,
    pub lane: Lane,
    pub status: DiscoverStatus,
    pub absent: Option<Absent>,
    /// What it needs first, another discovery not done yet (« après GitHub »): its gesture goes there first.
    pub after: Option<Discovery>,
    /// New in the release of the band.
    pub new: bool,
}

/// The discovery as the window shows it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverView {
    /// Every discovery this version has, in the order of the map.
    pub items: Vec<DiscoverItem>,
    pub hidden: bool,
    /// Done, among those on this device.
    pub done: usize,
    pub total: usize,
    /// The dot on « Aide »: a discovery on this device neither done nor dismissed, and not « Tout masquer ».
    pub dot: bool,
    /// The bubbles at the right moment still allowed: on this device, to try, never shown, not hidden.
    pub tips: Vec<Discovery>,
    /// The band « Nouveau dans la 2.3 » to show: the release it tells.
    pub band: Option<String>,
}

/// A gesture of the window on the discovery.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Act {
    /// What the window saw done (`SEEN_BY_WINDOW` only: the core sees the rest by itself).
    Seen { id: Discovery },
    /// « Écarter ».
    Dismiss { id: Discovery },
    /// « Remettre », on the map: a discovery dismissed offered again.
    Restore { id: Discovery },
    /// « Tout masquer », or « Réafficher ».
    Hide { hidden: bool },
    /// A bubble at the right moment was shown: never again.
    Tipped { id: Discovery },
    /// The band « Nouveau dans la … » seen or closed: never again for this release.
    BandSeen,
}

/// The lane of `id`.
pub fn lane_of(id: Discovery) -> Lane {
    LANES
        .iter()
        .find(|(_, ids)| ids.contains(&id))
        .map(|(l, _)| *l)
        .unwrap_or(Lane::Drive)
}

/// Whether this version has `id` at all: one it has not is never shown.
pub fn in_version(id: Discovery) -> bool {
    id != Discovery::Remote || REMOTE_IN_VERSION
}

/// Whether the real state says `id` is done, wherever it was done.
pub fn real(id: Discovery, f: &Facts) -> bool {
    match id {
        Discovery::Ticket => f.tickets > 0,
        Discovery::SecondAgent => f.agents_in_a_project >= 2,
        Discovery::Forge => f.forge_connected,
        Discovery::Sources => f.sources_connected,
        Discovery::PrReview => f.pr_reviews > 0,
        Discovery::Approve => f.ticket_done,
        Discovery::OpenPr => f.ticket_pr,
        Discovery::Mcp => f.mcp_on,
        Discovery::Account => f.accounts >= 2,
        Discovery::Team => f.team_project,
        // Moments, not states: the core ticks them as they happen (`Core::discovered`), the window those it alone sees.
        Discovery::Docs
        | Discovery::Dictation
        | Discovery::Voice
        | Discovery::Terminal
        | Discovery::Editor
        | Discovery::Launch
        | Discovery::PrView
        | Discovery::Plugin
        // On by default: the autopilot is discovered once it started a ticket by itself, not by its switch.
        | Discovery::Autopilot
        | Discovery::Remote => false,
    }
}

/// Why `id` cannot be done on this device; None: it can.
pub fn absent(id: Discovery, f: &Facts) -> Option<Absent> {
    if let Some((_, why)) = f.denied.iter().find(|(d, _)| *d == id) {
        return Some(Absent::Policy { why: why.clone() });
    }
    match id {
        Discovery::Dictation | Discovery::Voice if f.microphones == Some(0) => {
            Some(Absent::NoMicrophone)
        }
        Discovery::Team if !f.team_server_known && !f.team_project => Some(Absent::NoTeamServer),
        _ => None,
    }
}

/// What `id` needs done first, when it is not.
pub fn needs(id: Discovery, f: &Facts, saved: &Onboarding) -> Option<Discovery> {
    let first = match id {
        Discovery::Autopilot | Discovery::Approve => Discovery::Ticket,
        Discovery::PrView | Discovery::OpenPr => Discovery::Forge,
        Discovery::PrReview => Discovery::PrView,
        _ => return None,
    };
    (!done(first, f, saved)).then_some(first)
}

fn done(id: Discovery, f: &Facts, saved: &Onboarding) -> bool {
    real(id, f) || saved.discover.get(&id) == Some(&DiscoverOutcome::Done)
}

/// Where `id` stands.
pub fn status(id: Discovery, saved: &Onboarding, f: &Facts) -> DiscoverStatus {
    if done(id, f, saved) {
        return DiscoverStatus::Done;
    }
    if absent(id, f).is_some() {
        return DiscoverStatus::Absent;
    }
    match saved.discover.get(&id) {
        Some(DiscoverOutcome::Dismissed) => DiscoverStatus::Dismissed,
        _ => DiscoverStatus::Todo,
    }
}

/// The discoveries the real state made done since, kept as done (a discovery done stays done). Whether any was.
pub fn settle(saved: &mut Onboarding, f: &Facts) -> bool {
    let mut changed = false;
    for id in all() {
        if real(id, f) && saved.discover.get(&id) != Some(&DiscoverOutcome::Done) {
            saved.discover.insert(id, DiscoverOutcome::Done);
            changed = true;
        }
    }
    changed
}

/// Every discovery of this version, in the order of the map.
pub fn all() -> impl Iterator<Item = Discovery> {
    LANES
        .iter()
        .flat_map(|(_, ids)| ids.iter().copied())
        .filter(|id| in_version(*id))
}

/// Every discovery shown here: those of this version, but those of a feature switched off (F-1).
pub fn shown(f: &Facts) -> impl Iterator<Item = Discovery> + '_ {
    all().filter(|id| !f.off.contains(id))
}

/// « 2.3 » of « 2.3.0-beta.4 ».
pub fn release_of(version: &str) -> String {
    version.split('.').take(2).collect::<Vec<_>>().join(".")
}

/// Whether the band « Nouveau dans la … » shows, `version` running: a device set up before this release, which has
/// not seen it yet. A new installation never sees it (its journey keeps its release as seen), nor a minor version.
pub fn band(saved: &Onboarding, f: &Facts, version: &str, window: bool) -> Option<String> {
    let release = release_of(version);
    let shows = window
        && release == NEW_RELEASE
        && !f.fresh
        && f.projects > 0
        && saved.seen_release.as_deref() != Some(release.as_str());
    shows.then_some(release)
}

/// The discovery as the window shows it.
pub fn view(saved: &Onboarding, f: &Facts, version: &str, window: bool) -> DiscoverView {
    let items: Vec<DiscoverItem> = shown(f)
        .map(|id| {
            let status = status(id, saved, f);
            DiscoverItem {
                id,
                lane: lane_of(id),
                status,
                absent: if status == DiscoverStatus::Absent {
                    absent(id, f)
                } else {
                    None
                },
                after: if status == DiscoverStatus::Todo {
                    needs(id, f, saved)
                } else {
                    None
                },
                new: NEW_IN_RELEASE.contains(&id),
            }
        })
        .collect();
    let here = |i: &&DiscoverItem| i.status != DiscoverStatus::Absent;
    let done = items
        .iter()
        .filter(here)
        .filter(|i| i.status == DiscoverStatus::Done)
        .count();
    let total = items.iter().filter(here).count();
    let open = items.iter().any(|i| i.status == DiscoverStatus::Todo);
    let tips = if saved.discover_hidden {
        Vec::new()
    } else {
        items
            .iter()
            .filter(|i| i.status == DiscoverStatus::Todo && !saved.tipped.contains(&i.id))
            .map(|i| i.id)
            .collect()
    };
    DiscoverView {
        hidden: saved.discover_hidden,
        dot: window && open && !saved.discover_hidden,
        band: band(saved, f, version, window),
        items,
        done,
        total,
        tips,
    }
}

/// `act` done on `saved`. Whether it changed anything.
pub fn apply(saved: &mut Onboarding, act: &Act, f: &Facts, version: &str) -> bool {
    let before = saved.clone();
    match act {
        Act::Seen { id } => {
            if SEEN_BY_WINDOW.contains(id) {
                saved.discover.insert(*id, DiscoverOutcome::Done);
            }
        }
        // A discovery done is never dismissed: what was done stays done.
        Act::Dismiss { id } => {
            if !done(*id, f, saved) {
                saved.discover.insert(*id, DiscoverOutcome::Dismissed);
            }
        }
        Act::Restore { id } => {
            if saved.discover.get(id) == Some(&DiscoverOutcome::Dismissed) {
                saved.discover.remove(id);
            }
        }
        Act::Hide { hidden } => saved.discover_hidden = *hidden,
        Act::Tipped { id } => {
            saved.tipped.insert(*id);
        }
        Act::BandSeen => saved.seen_release = Some(release_of(version)),
    }
    settle(saved, f);
    *saved != before
}

/// The discovery a fact of the core makes done (the same moments the statistics count, consent or not).
pub fn of_event(e: &Event) -> Option<Discovery> {
    Some(match e {
        Event::TicketCreated => Discovery::Ticket,
        Event::TicketStarted { autopilot: true } => Discovery::Autopilot,
        Event::TicketApproved => Discovery::Approve,
        Event::TerminalOpened => Discovery::Terminal,
        Event::LaunchStarted => Discovery::Launch,
        Event::McpEnabled => Discovery::Mcp,
        Event::DictationUsed { .. } => Discovery::Dictation,
        Event::VoiceConversation { .. } => Discovery::Voice,
        Event::PluginInstalled { from_store: true } => Discovery::Plugin,
        Event::PrOpened { .. } => Discovery::OpenPr,
        Event::PrReviewStarted { .. } => Discovery::PrReview,
        Event::IntegrationConnected { service } if *service == "github" => Discovery::Forge,
        _ => return None,
    })
}

/// The version of this build.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

impl Core {
    /// What the discovery reads of the app now. `microphones`: those of this machine, when the caller has the voice.
    pub fn discover_facts(&self, microphones: Option<usize>) -> Facts {
        use crate::model::Column;
        let (tickets, ticket_done, ticket_pr) = {
            let t = self.tickets.read();
            (
                t.len(),
                t.iter().any(|t| t.column == Column::Done),
                t.iter().any(|t| t.pr.is_some()),
            )
        };
        let (projects, team_project) = {
            let p = self.projects.read();
            (p.len(), p.iter().any(|p| p.team.is_some()))
        };
        let agents_in_a_project = {
            let mut by: std::collections::HashMap<String, usize> = Default::default();
            for h in self.agents.read().values() {
                let a = h.lock();
                // The user's own agents: not a ticket's, nor a discussion of the tab Docs.
                if !a.meta.archived && a.meta.ticket_id.is_none() && a.meta.discussion.is_none() {
                    *by.entry(a.meta.project_id.clone()).or_default() += 1;
                }
            }
            by.into_values().max().unwrap_or(0)
        };
        let (accounts, mcp_on) = {
            let s = self.settings.read();
            (s.accounts.len(), s.mcp_enabled)
        };
        let forge_connected = !crate::forge::accounts::all(&self.accounts.read()).is_empty();
        let sources_connected = self.onboarding_facts().sources_connected;
        let pr_reviews = self.pr_reviews.read().len();
        let locks = self.locks.read();
        let mut denied = Vec::new();
        if let Err(e) = locks.check_dictation() {
            denied.push((Discovery::Dictation, e.to_string()));
            denied.push((Discovery::Voice, e.to_string()));
        }
        if locks.mcp_external() == "denied" {
            denied.push((
                Discovery::Mcp,
                locks.imposed(escouade_policy::list::MCP_EXTERNAL),
            ));
        }
        if let Err(e) = locks.check_remote_api() {
            denied.push((Discovery::Remote, e.to_string()));
        }
        // A plugin of the store: refused when no version of the store may come in.
        let origins = escouade_policy::list::PLUGIN_ORIGINS_KEY;
        if !locks.allows(origins, "store") && !locks.allows(origins, "store-verified") {
            let why = locks
                .denial(origins, "store")
                .unwrap_or_else(|| locks.imposed(origins));
            denied.push((Discovery::Plugin, why));
        }
        let off = crate::features::discoveries_off(&self.settings.read());
        let team_server_known = locks.enrollment().is_some()
            || locks
                .value(escouade_policy::list::SERVERS)
                .and_then(|v| v.as_array())
                .is_some_and(|a| !a.is_empty());
        drop(locks);
        Facts {
            fresh: self.fresh,
            projects,
            tickets,
            ticket_done,
            ticket_pr,
            agents_in_a_project,
            accounts,
            forge_connected,
            sources_connected,
            pr_reviews,
            mcp_on,
            team_project,
            team_server_known,
            microphones,
            denied,
            off,
        }
    }

    /// The discovery as the window shows it; the discoveries the real state made done since are kept as done.
    pub fn discover_view(&self, microphones: Option<usize>) -> DiscoverView {
        let window = crate::remoteapi::current() == crate::remoteapi::Client::Window;
        let facts = self.discover_facts(microphones);
        let mut saved = self.onboarding.write();
        // A new installation starts at its own release: the band « Nouveau dans la … » is never for it.
        let mut changed = settle(&mut saved, &facts);
        if facts.fresh && saved.seen_release.is_none() {
            saved.seen_release = Some(release_of(version()));
            changed = true;
        }
        let shown = view(&saved, &facts, version(), window);
        drop(saved);
        if changed {
            self.request_save();
        }
        shown
    }

    /// `act` on the discovery; the discovery as it then is.
    pub fn discover_act(&self, act: Act, microphones: Option<usize>) -> DiscoverView {
        let window = crate::remoteapi::current() == crate::remoteapi::Client::Window;
        let facts = self.discover_facts(microphones);
        let (changed, shown) = {
            let mut saved = self.onboarding.write();
            let changed = apply(&mut saved, &act, &facts, version());
            (changed, view(&saved, &facts, version(), window))
        };
        if changed {
            self.request_save();
        }
        shown
    }

    /// `e` counted in the usage statistics (with the user's consent), and the discovery it does ticked (always).
    pub fn track(&self, e: Event) {
        self.discovered(&e);
        self.telemetry.track(e);
    }

    /// A fact of the core that does a discovery (`of_event`): it is ticked at once, and the window told.
    pub fn discovered(&self, e: &Event) {
        if let Some(id) = of_event(e) {
            self.discover_done(id);
        }
    }

    /// `id` done for real: ticked at once, and the window told.
    pub fn discover_done(&self, id: Discovery) {
        {
            let mut saved = self.onboarding.write();
            if saved.discover.get(&id) == Some(&DiscoverOutcome::Done) {
                return;
            }
            saved.discover.insert(id, DiscoverOutcome::Done);
        }
        self.request_save();
        self.hub.emit(UiEvent::DiscoverChanged {});
    }
}

#[cfg(test)]
#[path = "discover_tests.rs"]
mod tests;
