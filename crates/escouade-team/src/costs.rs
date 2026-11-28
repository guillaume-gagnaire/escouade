//! The costs and the quotas of the team (TS-16, `docs/TEAM-SERVER.md` §10): what each device sends of what its agents
//! spent, for the administrators alone.
//!
//! - **What goes** ([`Totals`]): totals, never a turn. For each day, by (project, harness, model, plan): the tokens of
//!   input, of output and of cache, the cost estimated, the highest percentage of quota reached; and the total of each
//!   ticket, by its key. Never a prompt, a reply, a file's name, a ticket's title or a turn: the types hold nothing
//!   else (`deny_unknown_fields` both ways), and the tests read back the exact bytes that go. The member is named only
//!   when the team chose « par membre » (C15); « par projet », the totals do not carry it.
//! - **Sealed to the administrators' key** ([`escouade_team_crypto::CostKey`]), an HPKE pair apart, whose seed is
//!   wrapped for each administrator's device. A member's device keeps its own totals in the clear (the core reads them
//!   from its statistics); it cannot open another's. The server sees that an entry was written, its size and its hour.
//! - **The streams**: [`KEY_STREAM`], written by an administrator alone (the server refuses a member there, and each
//!   device refuses at the reading an entry not sealed by an administrator): each administrator device's own key
//!   ([`KeyEntry::Device`]), the administrators' key of each epoch with its wraps ([`KeyEntry::Key`]), wraps added for a
//!   device come after ([`KeyEntry::Wrap`]), and the totals of each project and of the team for a month
//!   ([`KeyEntry::Summary`]), which every member reads to weigh a budget before a start. [`TOTALS_STREAM`], open to
//!   every device: the totals, sealed.
//! - **The rotation**: an administrator's device draws a new pair as soon as a device the current one was wrapped for
//!   is no longer an administrator's device of the team (a member removed, an administrator made a member again, a
//!   device revoked). A member's device does not seal to a key in that state: its totals wait for the new one, so a
//!   former administrator never reads what is sealed after. At a new epoch every device sends its month again.
//! - **Offline, without double**: what waits is the last totals of each day ([`State::pending`]), in the team's
//!   encrypted base; each entry holds whole days, and an administrator keeps, for each device and day, the latest one
//!   (by its hour, then its number in the stream): an entry sent twice (a reply lost) or late counts once.

use std::collections::{BTreeMap, BTreeSet};

use escouade_team_crypto::costkey::Costs;
use escouade_team_crypto::recovery::Sealed;
use escouade_team_crypto::{CostKey, Identity, MemberId, ProjectId, Role};
use openmls_traits::OpenMlsProvider;
use serde::{Deserialize, Serialize};

pub use crate::keyring::Alarm;
use crate::keyring::{self, is_stale, open, provider, seal_entry, Opened, Ring, Step, Wrap};
use crate::team::Team;
use crate::{wire, Error};

/// The administrators' key of the costs, its wraps and the summaries: a project id no client draws (16 bytes).
pub const KEY_STREAM: ProjectId = ProjectId(*b"escouade/costkey");
/// The totals, sealed to the administrators' key.
pub const TOTALS_STREAM: ProjectId = ProjectId(*b"escouade/costsum");

const STATE: &str = "costs";

/// The days an administrator's device keeps of each device (a little more than a year).
pub const KEPT_DAYS: usize = 400;

// ---------- the bounds (SR3-03, SR3-08) ----------

/// The most a device may say it spent in a day, all its rows together (dollars, or their equivalent in API): past it,
/// its totals are refused whole (one member cannot fill the team's budgets and stop everyone's autopilot).
pub const MAX_DAY_COST: f64 = 10_000.0;
/// The most tokens a device may say it used in a day, all its rows together.
pub const MAX_DAY_TOKENS: u64 = 10_000_000_000;
/// The days of one entry: the month sent again whole, and a little more.
pub const MAX_DAYS: usize = 62;
/// The rows (project, harness, model) of a day, and its tickets.
pub const MAX_ROWS: usize = 256;
pub const MAX_TICKETS: usize = 1_024;
/// The length of a name in a row (project, harness, model, ticket).
const MAX_NAME: usize = 256;

fn amount_ok(v: f64) -> bool {
    v.is_finite() && (0.0..=MAX_DAY_COST).contains(&v)
}

/// A name of a row: bounded, no control character; empty only for a model the harness did not say.
fn name_ok(s: &str, empty: bool) -> bool {
    (empty || !s.is_empty()) && s.len() <= MAX_NAME && !s.chars().any(char::is_control)
}

/// `2026-11-20`.
fn day_ok(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

/// Why `totals` cannot be taken, when they cannot. `author`: the member (hex) whose device signed them, None for this
/// device's own (the member is then checked by the caller). Every amount finite, positive and within its bounds; each
/// day's sum within [`MAX_DAY_COST`] and [`MAX_DAY_TOKENS`]; the member named, when it is, the author.
pub fn check(totals: &Totals, author: Option<&str>) -> Result<(), String> {
    if let (Some(m), Some(a)) = (&totals.member, author) {
        if m != a {
            return Err("the totals name another member than the one who signed them".into());
        }
    }
    if totals.days.len() > MAX_DAYS {
        return Err(format!("more than {MAX_DAYS} days in one entry"));
    }
    for d in &totals.days {
        if !day_ok(&d.day) {
            return Err("a day that is not a date".into());
        }
        if d.rows.len() > MAX_ROWS || d.tickets.len() > MAX_TICKETS {
            return Err(format!("too many rows on {}", d.day));
        }
        let (mut cost, mut tokens) = (0.0_f64, 0_u64);
        for r in &d.rows {
            if !name_ok(&r.project, false)
                || !name_ok(&r.harness, false)
                || !name_ok(&r.model, true)
            {
                return Err(format!("a row of {} without a proper name", d.day));
            }
            if !amount_ok(r.cost)
                || r.quota
                    .is_some_and(|q| !q.is_finite() || !(0.0..=1_000.0).contains(&q))
            {
                return Err(format!(
                    "a cost or a quota on {} that is not finite, negative, or too high",
                    d.day
                ));
            }
            cost += r.cost;
            tokens = tokens
                .saturating_add(r.input)
                .saturating_add(r.output)
                .saturating_add(r.cache);
        }
        if cost > MAX_DAY_COST || tokens > MAX_DAY_TOKENS {
            return Err(format!(
                "more than a device can spend in a day on {}",
                d.day
            ));
        }
        for t in &d.tickets {
            let tokens = t.input.saturating_add(t.output).saturating_add(t.cache);
            if !name_ok(&t.project, false)
                || !name_ok(&t.ticket, false)
                || !amount_ok(t.cost)
                || tokens > MAX_DAY_TOKENS
            {
                return Err(format!("a ticket's total on {} out of bounds", d.day));
            }
        }
    }
    Ok(())
}

fn spent_ok(s: &Spent) -> bool {
    s.cost.is_finite() && s.cost >= 0.0
}

/// A summary an administrator published: every amount finite and positive (a NaN would no longer read back here).
fn summary_ok(s: &Summary) -> bool {
    spent_ok(&s.team) && s.projects.values().all(spent_ok)
}

// ---------- what goes ----------

/// What a device sends: the content of an entry of [`TOTALS_STREAM`] once opened, exactly.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Totals {
    /// The member, hex: only when the team chose « par membre » (C15-A).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<String>,
    /// When they were made (ms): the latest wins, for a day.
    pub hour: i64,
    /// Whole days, each replacing what this device sent before for it.
    pub days: Vec<Day>,
}

/// A day of a device.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Day {
    /// `2026-11-20`, in the device's local time.
    pub day: String,
    pub rows: Vec<Row>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tickets: Vec<TicketRow>,
}

/// The totals of a day for a project, a harness and a model.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Row {
    /// The project of the team: the id of its journal (hex), the same on every device.
    pub project: String,
    pub harness: String,
    pub model: String,
    /// On a plan (Claude Max, ChatGPT Pro…): `cost` is an equivalent in API, not a bill.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub plan: bool,
    pub input: u64,
    pub output: u64,
    pub cache: u64,
    pub cost: f64,
    /// The highest percentage of quota its account reached that day, when it has a quota.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quota: Option<f64>,
}

/// The total of a day for a ticket (its key, as the team's Kanban gives it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TicketRow {
    pub project: String,
    pub ticket: String,
    pub input: u64,
    pub output: u64,
    pub cache: u64,
    pub cost: f64,
}

/// What a project or the team spent over a month, as an administrator's device publishes it for every member.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Spent {
    pub tokens: u64,
    pub cost: f64,
}

/// The totals of a month, for every member: of each project (by its journal), and of the team. Never a member's.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Summary {
    /// `2026-11`.
    pub month: String,
    /// When it was made (ms).
    pub at: i64,
    pub projects: BTreeMap<String, Spent>,
    pub team: Spent,
}

// ---------- the streams ----------

/// An entry of [`KEY_STREAM`], sealed under the team's key and signed by its author's leaf.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
enum KeyEntry {
    /// The key of the costs of the device that signs it, to receive the wraps.
    Device {
        public: Vec<u8>,
    },
    /// The administrators' key of `epoch` and its wraps.
    Key {
        epoch: u64,
        public: Vec<u8>,
        wraps: Vec<Wrap>,
    },
    /// Wraps of the key of `epoch` for devices that came after it.
    Wrap {
        epoch: u64,
        wraps: Vec<Wrap>,
    },
    Summary {
        summary: Summary,
    },
}

impl keyring::Entry for KeyEntry {
    fn step(self) -> Result<Step, Self> {
        match self {
            KeyEntry::Device { public } => Ok(Step::Device { public }),
            KeyEntry::Key {
                epoch,
                public,
                wraps,
            } => Ok(Step::Key {
                epoch,
                public,
                wraps,
            }),
            KeyEntry::Wrap { epoch, wraps } => Ok(Step::Wrap { epoch, wraps }),
            own @ KeyEntry::Summary { .. } => Err(own),
        }
    }

    fn of(step: Step) -> Self {
        match step {
            Step::Device { public } => KeyEntry::Device { public },
            Step::Key {
                epoch,
                public,
                wraps,
            } => KeyEntry::Key {
                epoch,
                public,
                wraps,
            },
            Step::Wrap { epoch, wraps } => KeyEntry::Wrap { epoch, wraps },
        }
    }
}

/// An entry of [`TOTALS_STREAM`]: the totals sealed to the administrators' key of `epoch`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TotalsEntry {
    epoch: u64,
    sealed: Sealed,
}

/// What a device read of another's totals (an administrator's).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DeviceTotals {
    /// The device (hex), and its member (hex) as its signature says: the core shows the member only « par membre ».
    pub device: String,
    pub author: String,
    /// The member as the totals name it (« par membre »), None « par projet ».
    pub member: Option<String>,
    /// Each day, as last sent, with its hour and number.
    pub days: BTreeMap<String, ReceivedDay>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReceivedDay {
    pub hour: i64,
    pub seq: u64,
    pub member: Option<String>,
    pub rows: Vec<Row>,
    pub tickets: Vec<TicketRow>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct State {
    /// The key's stream as this device read it (its own key, the devices', the pairs and the seeds it holds, the
    /// cursor, the alarm, the entries refused): flattened, under the names TS-16 wrote.
    #[serde(flatten)]
    ring: Ring,
    totals_cursor: u64,
    /// What waits to go, day by day (the last totals of each day).
    pending: BTreeMap<String, Day>,
    pending_member: Option<String>,
    pending_hour: i64,
    /// The last totals that went, as they went (shown to the member: « ce qui remonte de toi »).
    sent: Option<Totals>,
    /// The last of each day that went, and the member it named: a day the same is not sent again.
    went: BTreeMap<String, (Option<String>, Day)>,
    /// The epoch the month was sent again for.
    resent_for: u64,
    /// An administrator's: what each device sent.
    received: BTreeMap<String, DeviceTotals>,
    summary: Option<Summary>,
    /// Totals sealed to a key this device does not hold (before it became an administrator).
    unreadable: usize,
}

/// What this device knows of the costs of the team.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    /// The epoch of the administrators' key, when there is one.
    pub epoch: Option<u64>,
    /// This device holds the key (an administrator's).
    pub holds: bool,
    /// A key a former administrator may hold: nothing is sealed to it until an administrator's device draws a new one.
    pub stale: bool,
    pub alarm: Option<Alarm>,
    /// The days waiting to go.
    pub pending: Vec<String>,
    pub sent: Option<Totals>,
    /// An administrator's: what each device sent.
    pub received: Vec<DeviceTotals>,
    /// The last totals of the projects and of the team, for every member.
    pub summary: Option<Summary>,
    pub refused: usize,
    pub unreadable: usize,
}

fn load(team: &Team) -> Result<State, Error> {
    Ok(team.load_extra(STATE)?.unwrap_or_default())
}

/// The administrators' devices of the team now: (device hex, member hex).
fn admin_devices(team: &Team) -> Result<BTreeMap<String, String>, Error> {
    Ok(team
        .members()?
        .into_iter()
        .filter(|(_, role, _)| *role == Role::Admin)
        .flat_map(|(m, _, devices)| {
            devices
                .into_iter()
                .map(move |d| (hex::encode(d.0), hex::encode(m.0)))
        })
        .collect())
}

fn admins_now(team: &Team) -> BTreeSet<MemberId> {
    team.members()
        .map(|ms| {
            ms.into_iter()
                .filter(|(_, r, _)| *r == Role::Admin)
                .map(|(m, _, _)| m)
                .collect()
        })
        .unwrap_or_default()
}

/// Who writes the key's stream: an administrator of the entry's epoch (of now, for an epoch this device did not live).
fn by_an_administrator(team: &Team, epoch: u64, who: &Identity) -> bool {
    match team.admins_at(epoch) {
        Some(then) => then.contains(&who.member),
        None => admins_now(team).contains(&who.member),
    }
}

async fn read_keys(team: &Team, state: &mut State) -> Result<(), Error> {
    let summary = &mut state.summary;
    keyring::read_keys::<Costs, KeyEntry>(
        team,
        KEY_STREAM,
        &mut state.ring,
        &by_an_administrator,
        &mut |entry| match entry {
            KeyEntry::Summary { summary: s } if !summary_ok(&s) => false,
            KeyEntry::Summary { summary: s } => {
                if summary
                    .as_ref()
                    .is_none_or(|was| (was.month.as_str(), was.at) <= (s.month.as_str(), s.at))
                {
                    *summary = Some(s);
                }
                true
            }
            _ => false,
        },
    )
    .await
}

async fn read_totals(team: &Team, state: &mut State) -> Result<(), Error> {
    let client = team.client().clone();
    let id = team.id();
    let p = provider();
    'read: loop {
        let page = client
            .read_project(&TOTALS_STREAM.0, state.totals_cursor, None)
            .await?;
        for e in &page.entries {
            if e.seq <= state.totals_cursor {
                break 'read;
            }
            match open::<TotalsEntry>(team, TOTALS_STREAM, &e.payload, &|_, _, _| true) {
                Opened::Ok(member, device, entry) => match state.ring.seeds.get(&entry.epoch) {
                    Some(seed) => {
                        let opened = CostKey::from_seed(p.crypto(), *seed)
                            .and_then(|k| {
                                k.open_totals(p.crypto(), &id, entry.epoch, &device, &entry.sealed)
                            })
                            .ok()
                            .and_then(|plain| wire::decode::<Totals>(&plain).ok());
                        let taken = opened.is_some_and(|t| take(state, member, device, e.seq, t));
                        if !taken {
                            state.ring.refused += 1;
                        }
                    }
                    None => state.unreadable += 1,
                },
                Opened::Unallowed(_) | Opened::Refused => state.ring.refused += 1,
                Opened::Later => break 'read,
            }
            state.totals_cursor = e.seq;
        }
        if !page.more || page.entries.is_empty() {
            break;
        }
    }
    Ok(())
}

/// The totals of a device, day by day: the latest of each day wins (its hour, then its number), so an entry sent twice
/// or late counts once. Totals out of their bounds, or naming another member than the one who signed them, are refused
/// whole (SR3-03): false.
fn take(state: &mut State, member: MemberId, device: [u8; 16], seq: u64, t: Totals) -> bool {
    if check(&t, Some(&hex::encode(member.0))).is_err() {
        return false;
    }
    let device = hex::encode(device);
    let entry = state
        .received
        .entry(device.clone())
        .or_insert_with(|| DeviceTotals {
            device,
            author: hex::encode(member.0),
            ..Default::default()
        });
    for d in t.days {
        let newer = entry
            .days
            .get(&d.day)
            .is_none_or(|was| (was.hour, was.seq) <= (t.hour, seq));
        if newer {
            entry.days.insert(
                d.day.clone(),
                ReceivedDay {
                    hour: t.hour,
                    seq,
                    member: t.member.clone(),
                    rows: d.rows,
                    tickets: d.tickets,
                },
            );
        }
    }
    entry.member = entry
        .days
        .values()
        .next_back()
        .and_then(|d| d.member.clone());
    while entry.days.len() > KEPT_DAYS {
        let first = entry.days.keys().next().cloned();
        if let Some(first) = first {
            entry.days.remove(&first);
        }
    }
    true
}

/// What an administrator's device does: its own key written, a key drawn when there is none or the current one is
/// stale, the key wrapped for the administrators' devices that have none (`keyring::serve`), and the month's summary
/// published when it changed.
async fn serve(team: &Team, state: &mut State, month: &str, now: i64) -> Result<bool, Error> {
    if !team.is_admin()? || state.ring.alarm.is_some() {
        return Ok(false);
    }
    let admins = admin_devices(team)?;
    let mut wrote =
        keyring::serve::<Costs, KeyEntry>(team, KEY_STREAM, &mut state.ring, &admins).await?;
    // The summary of the month, for every member (their budgets).
    let summary = summary_of(state, month, now);
    let same = state.summary.as_ref().is_some_and(|s| {
        s.month == summary.month && s.projects == summary.projects && s.team == summary.team
    });
    if !same && (!summary.projects.is_empty() || state.summary.is_some()) {
        let entry = KeyEntry::Summary { summary };
        team.client()
            .append_project(&KEY_STREAM.0, &seal_entry(team, KEY_STREAM, &entry)?)
            .await?;
        wrote = true;
    }
    Ok(wrote)
}

/// The totals of `month` (`2026-11`) of each project and of the team, from what every device sent.
fn summary_of(state: &State, month: &str, now: i64) -> Summary {
    let mut s = Summary {
        month: month.to_string(),
        at: now,
        ..Default::default()
    };
    for d in state.received.values() {
        for (day, got) in &d.days {
            if !day.starts_with(month) {
                continue;
            }
            for r in &got.rows {
                let tokens = r.input.saturating_add(r.output).saturating_add(r.cache);
                let p = s.projects.entry(r.project.clone()).or_default();
                p.tokens = p.tokens.saturating_add(tokens);
                p.cost += r.cost;
                s.team.tokens = s.team.tokens.saturating_add(tokens);
                s.team.cost += r.cost;
            }
        }
    }
    s
}

async fn flush(
    team: &Team,
    state: &mut State,
    admins: &BTreeMap<String, String>,
) -> Result<(), Error> {
    if state.pending.is_empty() || state.ring.alarm.is_some() {
        return Ok(());
    }
    let Some((epoch, key)) = state.ring.current() else {
        return Ok(());
    };
    // A former administrator may hold this key: the totals wait for the next one.
    if is_stale(key, admins) {
        return Ok(());
    }
    let totals = Totals {
        member: state.pending_member.clone(),
        hour: state.pending_hour,
        days: state.pending.values().cloned().collect(),
    };
    let p = provider();
    let me = team.identity().device.0;
    let sealed = CostKey::seal_totals(
        p.crypto(),
        &key.public,
        &team.id(),
        epoch,
        &me,
        &wire::encode(&totals),
    )?;
    let entry = TotalsEntry { epoch, sealed };
    let payload = seal_entry(team, TOTALS_STREAM, &entry)?;
    team.client()
        .append_project(&TOTALS_STREAM.0, &payload)
        .await?;
    state.pending.clear();
    state.resent_for = epoch;
    for d in &totals.days {
        state
            .went
            .insert(d.day.clone(), (totals.member.clone(), d.clone()));
    }
    while state.went.len() > 62 {
        let first = state.went.keys().next().cloned();
        if let Some(first) = first {
            state.went.remove(&first);
        }
    }
    state.sent = Some(totals);
    Ok(())
}

fn view_of(state: &State, admins: &BTreeMap<String, String>) -> View {
    let cur = state.ring.current();
    View {
        epoch: cur.map(|(e, _)| e),
        holds: state.ring.holds(),
        stale: cur.is_some_and(|(_, k)| is_stale(k, admins)),
        alarm: state.ring.alarm,
        pending: state.pending.keys().cloned().collect(),
        sent: state.sent.clone(),
        received: state.received.values().cloned().collect(),
        summary: state.summary.clone(),
        refused: state.ring.refused,
        unreadable: state.unreadable,
    }
}

/// Whether the month must go again whole: the key changed since it last went.
pub fn wants_month(team: &Team) -> Result<bool, Error> {
    let state = load(team)?;
    Ok(state
        .ring
        .current()
        .is_some_and(|(e, _)| e != state.resent_for))
}

/// Puts the totals of this device in the queue: each day replaces what waited for it. They go at the next [`sync`].
pub fn queue(team: &Team, totals: Totals) -> Result<(), Error> {
    // What the administrators would refuse does not wait here either: a NaN would no longer read back (SR3-08).
    check(&totals, Some(&hex::encode(team.identity().member.0))).map_err(Error::Refused)?;
    let mut state = load(team)?;
    // A day that went already as it is waits no more, unless the key changed (the month goes again to it) or the member
    // is named now: nothing goes for nothing.
    let again = state
        .ring
        .current()
        .is_some_and(|(e, _)| e != state.resent_for);
    let went = |d: &Day| {
        !again
            && state
                .went
                .get(&d.day)
                .is_some_and(|(m, x)| *m == totals.member && x == d)
    };
    let fresh: Vec<Day> = totals.days.into_iter().filter(|d| !went(d)).collect();
    if fresh.is_empty() {
        return Ok(());
    }
    for d in fresh {
        state.pending.insert(d.day.clone(), d);
    }
    state.pending_member = totals.member;
    state.pending_hour = totals.hour;
    team.save_extra(STATE, &state)?;
    Ok(())
}

/// Reads the streams of the costs and does what an administrator's device does (`month`: the month in progress,
/// for the summary). What was read is kept even when the server stops answering midway. Nothing is sent: [`send`].
pub async fn read(team: &mut Team, month: &str, now: i64) -> Result<View, Error> {
    let mut state = load(team)?;
    let admins = admin_devices(team)?;
    let mut result = read_keys(team, &mut state).await;
    if result.is_ok() && team.is_admin()? {
        result = read_totals(team, &mut state).await;
        if result.is_ok() {
            result = match serve(team, &mut state, month, now).await {
                Ok(true) => read_keys(team, &mut state).await,
                Ok(false) => Ok(()),
                Err(e) => Err(e),
            };
        }
    }
    team.save_extra(STATE, &state)?;
    result.map(|()| view_of(&state, &admins))
}

/// Sends what waits in the queue, sealed to the current key (nothing while the key is stale, or there is none).
pub async fn send(team: &mut Team) -> Result<View, Error> {
    let mut state = load(team)?;
    let admins = admin_devices(team)?;
    let result = flush(team, &mut state, &admins).await;
    team.save_extra(STATE, &state)?;
    result.map(|()| view_of(&state, &admins))
}

/// [`read`], then [`send`].
pub async fn sync(team: &mut Team, month: &str, now: i64) -> Result<View, Error> {
    read(team, month, now).await?;
    send(team).await
}

/// What this device knows of the costs, without reading the server.
pub fn view(team: &Team) -> Result<View, Error> {
    let admins = admin_devices(team)?;
    Ok(view_of(&load(team)?, &admins))
}

/// After an alarm, once a human looked: read from the start again.
pub fn retry(team: &Team) -> Result<(), Error> {
    let mut state = load(team)?;
    state.ring.retry();
    team.save_extra(STATE, &state)?;
    Ok(())
}

/// The bytes that go for `totals` before they are sealed: what the tests and the window read back.
pub fn plain_bytes(totals: &Totals) -> Vec<u8> {
    wire::encode(totals)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOB: MemberId = MemberId([2; 16]);
    const BOARD: &str = "a1b2c3d4e5f60718293a4b5c6d7e8f90";

    fn row(cost: f64) -> Row {
        Row {
            project: BOARD.into(),
            harness: "claude".into(),
            model: "m".into(),
            plan: false,
            input: 10,
            output: 0,
            cache: 0,
            cost,
            quota: None,
        }
    }

    fn totals(member: Option<MemberId>, day: &str, rows: Vec<Row>) -> Totals {
        Totals {
            member: member.map(|m| hex::encode(m.0)),
            hour: 1,
            days: vec![Day {
                day: day.into(),
                rows,
                tickets: vec![],
            }],
        }
    }

    /// The administrator's state after Bob's device sent `t`: whether it was taken, and the month's summary.
    fn taken(t: Totals) -> (bool, Summary, State) {
        let mut state = State::default();
        let ok = take(&mut state, BOB, [9; 16], 1, t);
        let s = summary_of(&state, "2026-11", 0);
        (ok, s, state)
    }

    /// SR3-03: what the report's proof sent is refused at the reading, and the summary does not move.
    #[test]
    fn an_inflated_a_negative_or_a_nan_total_is_refused_at_the_reading() {
        for cost in [
            1.0e12,
            -2.0e12,
            -0.01,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            MAX_DAY_COST + 1.0,
        ] {
            let (ok, s, state) = taken(totals(None, "2026-11-20", vec![row(cost)]));
            assert!(!ok, "{cost} refused");
            assert_eq!(s.team.cost, 0.0);
            assert!(state.received.is_empty());
        }
        // Each row within the bound, but their sum past it: refused whole.
        let (ok, s, _) = taken(totals(
            None,
            "2026-11-20",
            vec![row(MAX_DAY_COST / 2.0 + 1.0), row(MAX_DAY_COST / 2.0 + 1.0)],
        ));
        assert!(!ok);
        assert_eq!(s.team.cost, 0.0);
        // Tokens past the bound of a day (the proof's `u64::MAX / 4`).
        let mut r = row(1.0);
        r.input = u64::MAX / 4;
        assert!(!taken(totals(None, "2026-11-20", vec![r])).0);
        // A quota that is not finite.
        let mut r = row(1.0);
        r.quota = Some(f64::NAN);
        assert!(!taken(totals(None, "2026-11-20", vec![r])).0);
        // Within the bounds: taken, and counted.
        let (ok, s, _) = taken(totals(None, "2026-11-20", vec![row(12.5)]));
        assert!(ok);
        assert_eq!(s.team.cost, 12.5);
        assert_eq!(s.projects[BOARD].cost, 12.5);
    }

    /// SR3-03: the totals are the device's that signs them; a member named that is not its author is refused.
    #[test]
    fn totals_naming_another_member_than_their_signer_are_refused() {
        let carol = MemberId([3; 16]);
        let (ok, s, state) = taken(totals(Some(carol), "2026-11-20", vec![row(1.0)]));
        assert!(!ok, "Bob's spending is not put to Carol");
        assert_eq!(s.team.cost, 0.0);
        assert!(state.received.is_empty());
        let (ok, _, state) = taken(totals(Some(BOB), "2026-11-20", vec![row(1.0)]));
        assert!(ok);
        let got = &state.received[&hex::encode([9u8; 16])];
        assert_eq!(got.author, hex::encode(BOB.0));
        assert_eq!(got.member, Some(hex::encode(BOB.0)));
    }

    /// SR3-03: the number of an entry is bounded (days, rows, tickets), and a day is a date.
    #[test]
    fn the_size_of_an_entry_is_bounded() {
        let mut t = totals(None, "2026-11-20", vec![row(0.0); MAX_ROWS + 1]);
        assert!(check(&t, None).is_err());
        t.days[0].rows.truncate(MAX_ROWS);
        assert!(check(&t, None).is_ok());
        t.days = (0..=MAX_DAYS)
            .map(|i| Day {
                day: format!("2026-{:02}-{:02}", 1 + i / 28, 1 + i % 28),
                rows: vec![],
                tickets: vec![],
            })
            .collect();
        assert!(check(&t, None).is_err());
        assert!(check(&totals(None, "../../x", vec![row(1.0)]), None).is_err());
        let mut t = totals(None, "2026-11-20", vec![row(1.0)]);
        t.days[0].tickets = vec![
            TicketRow {
                project: BOARD.into(),
                ticket: "ESC-1".into(),
                input: 0,
                output: 0,
                cache: 0,
                cost: 1.0,
            };
            MAX_TICKETS + 1
        ];
        assert!(check(&t, None).is_err());
        t.days[0].tickets.truncate(1);
        t.days[0].tickets[0].cost = f64::NAN;
        assert!(check(&t, None).is_err());
    }

    /// AU-3 moved the key's part of the state into `keyring::Ring`, flattened: a state written before reads back whole,
    /// and is written again under the same names.
    #[test]
    fn a_state_written_before_the_ring_reads_back_whole() {
        let seed = [7u8; 32];
        let before = serde_json::json!({
            "own": seed,
            "announced": true,
            "devices": { "aa": ["bb", [1, 2]] },
            "keys": { "1": { "public": [3], "devices": ["aa"], "seq": 4 }, "2": { "public": [5], "devices": [], "seq": 9 } },
            "seeds": { "2": seed },
            "key_cursor": 9,
            "key_last": [9, seed],
            "totals_cursor": 12,
            "alarm": null,
            "pending": {},
            "pending_member": null,
            "pending_hour": 0,
            "sent": null,
            "went": {},
            "resent_for": 2,
            "received": {},
            "summary": null,
            "refused": 3,
            "unreadable": 1
        });
        let state: State = serde_json::from_value(before.clone()).unwrap();
        assert_eq!(state.ring.own, Some(seed));
        assert!(state.ring.announced);
        assert_eq!(state.ring.current().map(|(e, k)| (e, k.seq)), Some((2, 9)));
        assert!(state.ring.holds());
        assert_eq!((state.ring.key_cursor, state.ring.refused), (9, 3));
        assert_eq!(
            (state.totals_cursor, state.resent_for, state.unreadable),
            (12, 2, 1)
        );
        let again = serde_json::to_value(&state).unwrap();
        let names = |v: &serde_json::Value| {
            v.as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>()
        };
        assert_eq!(names(&again), names(&before));
        // Through the bytes the base keeps, too.
        let bytes = serde_json::to_vec(&state).unwrap();
        let back: State = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back.ring.seeds.get(&2), Some(&seed));
    }

    /// SR3-08: a summary with a NaN is refused, and what is kept reads back.
    #[test]
    fn a_summary_that_is_not_finite_is_refused_and_the_state_reads_back() {
        let mut s = Summary {
            month: "2026-11".into(),
            ..Default::default()
        };
        assert!(summary_ok(&s));
        s.team.cost = f64::NAN;
        assert!(!summary_ok(&s));
        s.team.cost = 1.0;
        s.projects.insert(
            BOARD.into(),
            Spent {
                tokens: 1,
                cost: f64::INFINITY,
            },
        );
        assert!(!summary_ok(&s));
        // What an administrator kept after refusing a NaN still reads back as JSON (the state of the costs).
        let (_, _, state) = taken(totals(None, "2026-11-22", vec![row(f64::NAN)]));
        let json = serde_json::to_string(&state).unwrap();
        assert!(serde_json::from_str::<State>(&json).is_ok());
    }
}
