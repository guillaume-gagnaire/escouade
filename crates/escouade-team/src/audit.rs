//! The audit of the stations (AU-3, `docs/TEAM-AUDIT.md` §3.3, S3): what a station says of itself (a refusal of the
//! core, a permission given, a command approved, a plugin installed…), sealed for the key of the audit and dropped on
//! the server, for the devices that emit the audit alone. On the model of the costs (TS-16, `costs.rs`), with which it
//! shares its stream of keys (`keyring.rs`) and its pair by epoch ([`escouade_team_crypto::AuditKey`]).
//!
//! - **What goes** ([`StationFact`]): an event of [`STATION_EVENTS`] and its closed target and detail
//!   (`escouade_audit::Fact`, AU-1), its id and the station's hour (`at`). Never its actor: the emitter takes it from
//!   the entry's signature (the member and the device), so a station never speaks for another. The types hold nothing
//!   else (`deny_unknown_fields`), the parts are read again as an event of the list at the queue and at the opening.
//! - **The emitters** ([`emitters`]): the devices of the team's machines (the role of service, OE-1) when it has one;
//!   the administrators' devices otherwise (the recourse of A2-A). The key of an epoch is wrapped for them alone, and
//!   drawn again as soon as one it was wrapped for is no longer an emitter (a machine removed, an administrator gone,
//!   or a machine come: the administrators then hand over). A station seals nothing to a key in that state.
//! - **The streams**: [`KEY_STREAM`], written by the emitters alone (the server refuses a member there; each device
//!   refuses at the reading an entry whose signer was not an emitter at its epoch): each emitter's own key, the pair of
//!   each epoch with its wraps, the wraps added after. [`FACTS_STREAM`], written by every device, read by the
//!   administrators and the machines alone (the server refuses it to a member): the facts, sealed. The server sees an
//!   entry, its size, its device and its hour.
//! - **Offline** ([`queue`], [`send`]): the facts wait in the team's encrypted base, in their order, and go at the
//!   return in batches; [`MAX_QUEUE`] at most, the oldest fall past it and the next batch says how many. A batch sent
//!   twice (a reply lost) is told by the facts' ids: the emitter takes each once ([`take`]), with the hour the server
//!   received it (`seen`) beside the station's (`at`).
//! - **No tool of the MCP** reads or drops anything here.

use std::collections::{BTreeMap, VecDeque};

use escouade_audit::event::STATION_EVENTS;
use escouade_audit::{Actor, Fact, Id};
use escouade_team_crypto::auditkey::Audit;
use escouade_team_crypto::recovery::Sealed;
use escouade_team_crypto::{AuditKey, DeviceKind, Identity, MemberId, ProjectId, Role};
use openmls_traits::OpenMlsProvider;
use serde::{Deserialize, Serialize};

pub use crate::keyring::Alarm;
use crate::keyring::{self, is_stale, open, provider, seal_entry, Opened, Ring, Step, Wrap};
use crate::team::Team;
use crate::{wire, Error};

/// The key of the audit, its wraps: a project id no client draws (16 bytes).
pub const KEY_STREAM: ProjectId = ProjectId(*b"escouade/audit/k");
/// The facts of the stations, sealed to the key of the audit: the stream `escouade/audit` of the design.
pub const FACTS_STREAM: ProjectId = ProjectId(*b"escouade/audit/f");

const STATE: &str = "audit";

/// The facts that wait at most; past it, the oldest fall (and the next batch says how many).
pub const MAX_QUEUE: usize = 10_000;
/// The facts of one entry at most, and its bytes before it is sealed (well under the server's 1 MiB an entry).
pub const MAX_BATCH: usize = 64;
const MAX_BATCH_BYTES: usize = 256 * 1024;
/// The bytes of one fact at most.
pub const MAX_FACT_BYTES: usize = 16 * 1024;
/// The last facts that went, kept for the member to see (AU-4: « ce que l'audit reçoit de ce poste »).
const KEPT_SENT: usize = 50;
/// The ids an emitter remembers to take a fact once.
const KEPT_IDS: usize = 8_192;

// ---------- what goes ----------

/// A fact of a station: the content of a batch of [`FACTS_STREAM`] once opened, exactly.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StationFact {
    /// Drawn by the station (32 hexadecimal digits): the emitter takes a fact once.
    pub id: String,
    /// The station's hour of the fact (ms).
    pub at: i64,
    /// The event, one of [`STATION_EVENTS`].
    pub event: String,
    /// Its target and its detail, the closed structures of the event (`escouade_audit::event`).
    pub target: serde_json::Value,
    pub detail: serde_json::Value,
}

impl StationFact {
    /// `fact` as a station drops it, at `at` (ms): its actor is left out (the emitter takes it from the signature).
    pub fn of(fact: &Fact, at: i64) -> Result<StationFact, Error> {
        if !fact.of_station() {
            return Err(Error::Refused(format!(
                "« {} » is not an event a station says",
                fact.name()
            )));
        }
        let mut id = [0u8; 16];
        getrandom::fill(&mut id).map_err(|e| Error::Encoding(e.to_string()))?;
        let (_, target, detail) = fact.parts();
        let out = StationFact {
            id: hex::encode(id),
            at,
            event: fact.name().to_owned(),
            target,
            detail,
        };
        if wire::encode(&out).len() > MAX_FACT_BYTES {
            return Err(Error::Refused(format!(
                "a fact of more than {MAX_FACT_BYTES} bytes"
            )));
        }
        Ok(out)
    }

    /// The event again, closed, with `actor`: an event that is not a station's, or a part it does not know, is refused.
    pub fn fact(&self, actor: Actor) -> Result<Fact, Error> {
        if !STATION_EVENTS.contains(&self.event.as_str()) {
            return Err(Error::Refused(format!(
                "« {} » is not an event a station says",
                self.event
            )));
        }
        let ok_id = self.id.len() == 32 && self.id.bytes().all(|c| c.is_ascii_hexdigit());
        if !ok_id {
            return Err(Error::Refused(
                "a fact's id is 32 hexadecimal digits".into(),
            ));
        }
        let actor = serde_json::to_value(actor).map_err(|e| Error::Encoding(e.to_string()))?;
        Fact::from_parts(&self.event, actor, self.target.clone(), self.detail.clone())
            .map_err(|e| Error::Refused(e.to_string()))
    }
}

/// What a batch holds once opened, exactly.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Batch {
    facts: Vec<StationFact>,
    /// The facts that fell from the queue before these (past [`MAX_QUEUE`]).
    #[serde(default, skip_serializing_if = "is_zero")]
    dropped: u64,
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

// ---------- the streams ----------

/// An entry of [`KEY_STREAM`], sealed under the team's key and signed by its author's leaf.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
enum KeyEntry {
    /// The key of the audit of the emitter that signs it, to receive the wraps.
    Device { public: Vec<u8> },
    /// The emitters' key of `epoch` and its wraps.
    Key {
        epoch: u64,
        public: Vec<u8>,
        wraps: Vec<Wrap>,
    },
    /// Wraps of the key of `epoch` for emitters that came after it.
    Wrap { epoch: u64, wraps: Vec<Wrap> },
}

impl keyring::Entry for KeyEntry {
    fn step(self) -> Result<Step, Self> {
        Ok(match self {
            KeyEntry::Device { public } => Step::Device { public },
            KeyEntry::Key {
                epoch,
                public,
                wraps,
            } => Step::Key {
                epoch,
                public,
                wraps,
            },
            KeyEntry::Wrap { epoch, wraps } => Step::Wrap { epoch, wraps },
        })
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

/// An entry of [`FACTS_STREAM`]: a batch sealed to the emitters' key of `epoch`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FactsEntry {
    epoch: u64,
    sealed: Sealed,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct State {
    #[serde(flatten)]
    ring: Ring,
    /// What waits to go, in its order.
    pending: VecDeque<StationFact>,
    /// The facts that fell from the queue since the last batch went.
    dropped: u64,
    /// The last facts that went, the newest last.
    sent: VecDeque<StationFact>,
    /// An emitter's: where it stopped reading the facts, and the ids it took.
    facts_cursor: u64,
    taken: VecDeque<String>,
    /// Batches sealed to a key this device does not hold (before it emitted, or for others).
    unreadable: usize,
}

/// What this device knows of the audit of the stations.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    /// The epoch of the emitters' key, when there is one.
    pub epoch: Option<u64>,
    /// This device holds the key (an emitter's).
    pub holds: bool,
    /// This device emits the audit (a machine of the team, or an administrator's when the team has none).
    pub emitter: bool,
    /// A key wrapped for a device that no longer emits: nothing is sealed to it until an emitter draws a new one.
    pub stale: bool,
    pub alarm: Option<Alarm>,
    /// The facts waiting to go.
    pub pending: usize,
    /// The facts that fell from the queue and are not said yet.
    pub dropped: u64,
    /// The last facts that went, the newest last.
    pub sent: Vec<StationFact>,
    pub refused: usize,
    pub unreadable: usize,
}

/// A fact an emitter took: who said it (its signature), when (the station's hour and the server's), and the event.
#[derive(Clone, Debug, PartialEq)]
pub struct Received {
    pub id: String,
    /// The device and its member (hex), as the entry's signature says.
    pub device: String,
    pub member: String,
    /// The entry's number in [`FACTS_STREAM`].
    pub seq: u64,
    /// The station's hour of the fact (ms).
    pub at: i64,
    /// The hour the server received it (ms).
    pub seen: i64,
    /// The event, closed; its actor is the member and the device of the signature.
    pub fact: Fact,
}

/// Facts a station let fall from its queue before a batch (it was offline too long).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dropped {
    pub device: String,
    pub member: String,
    pub count: u64,
    pub seen: i64,
}

/// What an emitter took from the stream at a reading.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Taken {
    pub facts: Vec<Received>,
    pub dropped: Vec<Dropped>,
}

fn load(team: &Team) -> Result<State, Error> {
    Ok(team.load_extra(STATE)?.unwrap_or_default())
}

/// The emitters among `leaves` (device kind only): the machines' devices when there is one, else the administrators'.
fn emitters_among(
    leaves: impl Iterator<Item = (MemberId, [u8; 16])> + Clone,
    machine: impl Fn(&MemberId) -> bool,
    admin: impl Fn(&MemberId) -> bool,
) -> BTreeMap<String, String> {
    let pick = |keep: &dyn Fn(&MemberId) -> bool| -> BTreeMap<String, String> {
        leaves
            .clone()
            .filter(|(m, _)| keep(m))
            .map(|(m, d)| (hex::encode(d), hex::encode(m.0)))
            .collect()
    };
    let machines = pick(&machine);
    if machines.is_empty() {
        pick(&admin)
    } else {
        machines
    }
}

/// The devices that emit the audit now (device hex → member hex): the team's machines' (the role of service) when it
/// has one, else its administrators'.
pub fn emitters(team: &Team) -> Result<BTreeMap<String, String>, Error> {
    let members = team.members()?;
    let leaves: Vec<(MemberId, [u8; 16])> = members
        .iter()
        .flat_map(|(m, _, devices)| devices.iter().map(move |d| (*m, d.0)))
        .collect();
    let role = |m: &MemberId| members.iter().find(|(x, _, _)| x == m).map(|(_, r, _)| *r);
    Ok(emitters_among(
        leaves.iter().copied(),
        |m| role(m) == Some(Role::Service),
        |m| role(m) == Some(Role::Admin),
    ))
}

/// Whether `who` emitted the audit at `epoch`, as the leaves and the roles of that epoch say (the same answer on every
/// device that lived through it; a device that did not cannot tell, and refuses).
fn emitter_at(team: &Team, epoch: u64, who: &Identity) -> bool {
    let Some(leaves) = team.leaves_at(epoch) else {
        return false;
    };
    let devices: Vec<(MemberId, [u8; 16])> = leaves
        .iter()
        .filter(|i| i.kind == DeviceKind::Device)
        .map(|i| (i.member, i.device.0))
        .collect();
    let admins = team.admins_at(epoch);
    emitters_among(
        devices.iter().copied(),
        |m| team.is_machine(m),
        |m| admins.is_some_and(|a| a.contains(m)),
    )
    .get(&hex::encode(who.device.0))
        == Some(&hex::encode(who.member.0))
}

async fn read_keys(team: &Team, state: &mut State) -> Result<(), Error> {
    keyring::read_keys::<Audit, KeyEntry>(
        team,
        KEY_STREAM,
        &mut state.ring,
        &emitter_at,
        &mut |_| false,
    )
    .await
}

fn view_of(state: &State, emitters: &BTreeMap<String, String>, me: &str) -> View {
    let cur = state.ring.current();
    View {
        epoch: cur.map(|(e, _)| e),
        holds: state.ring.holds(),
        emitter: emitters.contains_key(me),
        stale: cur.is_some_and(|(_, k)| is_stale(k, emitters)),
        alarm: state.ring.alarm,
        pending: state.pending.len(),
        dropped: state.dropped,
        sent: state.sent.iter().cloned().collect(),
        refused: state.ring.refused,
        unreadable: state.unreadable,
    }
}

fn me(team: &Team) -> String {
    hex::encode(team.identity().device.0)
}

/// Puts a fact of this station in the queue, at `at` (the station's hour, ms): it goes at the next [`send`]. An event
/// that is not a station's ([`STATION_EVENTS`]) is refused; its actor is not kept. Gives the fact's id.
pub fn queue(team: &Team, fact: &Fact, at: i64) -> Result<String, Error> {
    let fact = StationFact::of(fact, at)?;
    let id = fact.id.clone();
    let mut state = load(team)?;
    push(&mut state, fact);
    team.save_extra(STATE, &state)?;
    Ok(id)
}

/// A fact at the end of the queue; past [`MAX_QUEUE`], the oldest falls and is counted.
fn push(state: &mut State, fact: StationFact) {
    state.pending.push_back(fact);
    while state.pending.len() > MAX_QUEUE {
        state.pending.pop_front();
        state.dropped += 1;
    }
}

/// Reads the key's stream, and does what an emitter's device does (its own key written, a key drawn when there is
/// none or the current one is stale, the key wrapped for the emitters that lack it). Nothing is sent: [`send`].
pub async fn read(team: &mut Team) -> Result<View, Error> {
    let mut state = load(team)?;
    let emitters = emitters(team)?;
    let me = me(team);
    let mut result = read_keys(team, &mut state).await;
    if result.is_ok() && emitters.contains_key(&me) {
        result =
            match keyring::serve::<Audit, KeyEntry>(team, KEY_STREAM, &mut state.ring, &emitters)
                .await
            {
                Ok(true) => read_keys(team, &mut state).await,
                Ok(false) => Ok(()),
                Err(e) => Err(e),
            };
    }
    team.save_extra(STATE, &state)?;
    result.map(|()| view_of(&state, &emitters, &me))
}

/// The next batch of the queue: up to [`MAX_BATCH`] facts and [`MAX_BATCH_BYTES`], one fact at least.
fn next_batch(state: &State) -> Batch {
    let mut batch = Batch {
        facts: Vec::new(),
        dropped: state.dropped,
    };
    for f in state.pending.iter().take(MAX_BATCH) {
        batch.facts.push(f.clone());
        if batch.facts.len() > 1 && wire::encode(&batch).len() > MAX_BATCH_BYTES {
            batch.facts.pop();
            break;
        }
    }
    batch
}

async fn flush(
    team: &Team,
    state: &mut State,
    emitters: &BTreeMap<String, String>,
) -> Result<(), Error> {
    let p = provider();
    let me = team.identity().device.0;
    while !state.pending.is_empty() && state.ring.alarm.is_none() {
        let Some((epoch, key)) = state.ring.current() else {
            return Ok(());
        };
        // A device that no longer emits may hold this key: the facts wait for the next one.
        if is_stale(key, emitters) {
            return Ok(());
        }
        let batch = next_batch(state);
        let sealed = AuditKey::seal_facts(
            p.crypto(),
            &key.public,
            &team.id(),
            epoch,
            &me,
            &wire::encode(&batch),
        )?;
        let payload = seal_entry(team, FACTS_STREAM, &FactsEntry { epoch, sealed })?;
        team.client()
            .append_project(&FACTS_STREAM.0, &payload)
            .await?;
        state.dropped = 0;
        for _ in 0..batch.facts.len() {
            if let Some(f) = state.pending.pop_front() {
                state.sent.push_back(f);
            }
        }
        while state.sent.len() > KEPT_SENT {
            state.sent.pop_front();
        }
    }
    Ok(())
}

/// Sends what waits in the queue, sealed to the current key (nothing while the key is stale, or there is none). What
/// went before the server stopped answering is not sent again.
pub async fn send(team: &mut Team) -> Result<View, Error> {
    let mut state = load(team)?;
    let emitters = emitters(team)?;
    let result = flush(team, &mut state, &emitters).await;
    team.save_extra(STATE, &state)?;
    result.map(|()| view_of(&state, &emitters, &me(team)))
}

/// [`read`], then [`send`].
pub async fn sync(team: &mut Team) -> Result<View, Error> {
    read(team).await?;
    send(team).await
}

/// An emitter's: the facts dropped since its last reading, each once, opened. A batch sealed to the current key that
/// this device does not hold yet (its wrap is not written) stops the reading: it is read again once the key is here.
/// The server refuses the stream to a member (an error).
pub async fn take(team: &mut Team) -> Result<Taken, Error> {
    let mut state = load(team)?;
    let mut out = Taken::default();
    let result = take_into(team, &mut state, &mut out).await;
    team.save_extra(STATE, &state)?;
    result.map(|()| out)
}

async fn take_into(team: &Team, state: &mut State, out: &mut Taken) -> Result<(), Error> {
    let client = team.client().clone();
    let id = team.id();
    let p = provider();
    // An emitter waits for the wrap of the current key; another device (an administrator, when a machine emits) never
    // gets one.
    let emitting = emitters(team)?.contains_key(&me(team));
    'read: loop {
        let page = client
            .read_project(&FACTS_STREAM.0, state.facts_cursor, None)
            .await?;
        for e in &page.entries {
            if e.seq <= state.facts_cursor {
                break 'read;
            }
            match open::<FactsEntry>(team, FACTS_STREAM, &e.payload, &|_, _, _| true) {
                Opened::Ok(member, device, entry) => match state.ring.seeds.get(&entry.epoch) {
                    Some(seed) => {
                        let batch = AuditKey::from_seed(p.crypto(), *seed)
                            .and_then(|k| {
                                k.open_facts(p.crypto(), &id, entry.epoch, &device, &entry.sealed)
                            })
                            .ok()
                            .and_then(|plain| wire::decode::<Batch>(&plain).ok());
                        match batch {
                            Some(b) => {
                                take_batch(state, out, member, device, e.seq, seen_ms(e.at), b)
                            }
                            None => state.ring.refused += 1,
                        }
                    }
                    None if emitting
                        && state.ring.current().is_some_and(|(c, _)| c == entry.epoch) =>
                    {
                        break 'read
                    }
                    None => state.unreadable += 1,
                },
                Opened::Unallowed(_) | Opened::Refused => state.ring.refused += 1,
                Opened::Later => break 'read,
            }
            state.facts_cursor = e.seq;
        }
        if !page.more || page.entries.is_empty() {
            break;
        }
    }
    Ok(())
}

/// The server keeps an entry's hour to the second: `seen` is in ms, as `at`.
fn seen_ms(at: i64) -> i64 {
    at.saturating_mul(1_000)
}

fn take_batch(
    state: &mut State,
    out: &mut Taken,
    member: MemberId,
    device: [u8; 16],
    seq: u64,
    seen: i64,
    batch: Batch,
) {
    let (device, member) = (hex::encode(device), hex::encode(member.0));
    // A batch sent twice (a reply lost) says its fallen facts once: with the facts it carries, already taken.
    let again = !batch.facts.is_empty() && batch.facts.iter().all(|f| state.taken.contains(&f.id));
    if batch.dropped > 0 && !again {
        out.dropped.push(Dropped {
            device: device.clone(),
            member: member.clone(),
            count: batch.dropped,
            seen,
        });
    }
    let actor = Actor {
        member: Id::new(member.clone()).ok(),
        device: Id::new(device.clone()).ok(),
        ..Default::default()
    };
    for f in batch.facts {
        if state.taken.contains(&f.id) {
            continue;
        }
        match f.fact(actor.clone()) {
            Ok(fact) => {
                state.taken.push_back(f.id.clone());
                out.facts.push(Received {
                    id: f.id,
                    device: device.clone(),
                    member: member.clone(),
                    seq,
                    at: f.at,
                    seen,
                    fact,
                });
            }
            Err(_) => state.ring.refused += 1,
        }
    }
    while state.taken.len() > KEPT_IDS {
        state.taken.pop_front();
    }
}

/// What this device knows of the audit of the stations, without reading the server.
pub fn view(team: &Team) -> Result<View, Error> {
    Ok(view_of(&load(team)?, &emitters(team)?, &me(team)))
}

/// After an alarm, once a human looked: read the key's stream from the start again.
pub fn retry(team: &Team) -> Result<(), Error> {
    let mut state = load(team)?;
    state.ring.retry();
    team.save_extra(STATE, &state)?;
    Ok(())
}

/// The bytes that go for `fact` before they are sealed (alone in its batch): what the tests read back.
pub fn plain_bytes(fact: &StationFact) -> Vec<u8> {
    wire::encode(&Batch {
        facts: vec![fact.clone()],
        dropped: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn lock_refused() -> Fact {
        Fact::from_parts(
            "lock.refused",
            json!({}),
            json!({"keys": ["agents.engines"]}),
            json!({"via": "mcp"}),
        )
        .unwrap()
    }

    #[test]
    fn a_fact_holds_these_fields_and_its_actor_is_left_out() {
        let mut fact = lock_refused();
        if let Fact::LockRefused { actor, .. } = &mut fact {
            actor.member = Some(Id::new("someone-else").unwrap());
        }
        let f = StationFact::of(&fact, 1_700).unwrap();
        let v = serde_json::to_value(&f).unwrap();
        let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, ["at", "detail", "event", "id", "target"]);
        assert!(!serde_json::to_string(&f).unwrap().contains("someone-else"));
        // A field more is refused both ways; a part the event does not know too.
        let mut with = v.clone();
        with["actor"] = json!({"member": "x"});
        assert!(serde_json::from_value::<StationFact>(with).is_err());
        let mut f2 = f.clone();
        f2.detail["prompt"] = json!("rm -rf");
        assert!(f2.fact(Actor::default()).is_err());
        // The emitter's actor is the signer's.
        let actor = Actor {
            member: Some(Id::new("ab").unwrap()),
            ..Default::default()
        };
        let back = f.fact(actor.clone()).unwrap();
        assert_eq!(back.actor(), &actor);
        assert_eq!(back.name(), "lock.refused");
        // The bytes of a batch read back as these fields.
        let b: serde_json::Value = ciborium::from_reader(plain_bytes(&f).as_slice()).unwrap();
        assert_eq!(b.as_object().unwrap().keys().collect::<Vec<_>>(), ["facts"]);
    }

    #[test]
    fn only_the_events_of_a_station_go() {
        let not_ours = Fact::from_parts("session.opened", json!({}), json!({}), json!({})).unwrap();
        assert!(StationFact::of(&not_ours, 1).is_err());
        let mut f = StationFact::of(&lock_refused(), 1).unwrap();
        f.event = "member.added".into();
        assert!(f.fact(Actor::default()).is_err());
        let mut f = StationFact::of(&lock_refused(), 1).unwrap();
        f.id = "../x".into();
        assert!(f.fact(Actor::default()).is_err());
    }

    #[test]
    fn the_emitters_are_the_machines_else_the_administrators() {
        let (a, b, m) = (MemberId([1; 16]), MemberId([2; 16]), MemberId([3; 16]));
        let leaves = [(a, [10; 16]), (b, [20; 16]), (m, [30; 16]), (a, [11; 16])];
        let admin = |x: &MemberId| *x == a;
        let none = emitters_among(leaves.iter().copied(), |_| false, admin);
        assert_eq!(
            none.keys().cloned().collect::<Vec<_>>(),
            [hex::encode([10u8; 16]), hex::encode([11u8; 16])]
        );
        let with = emitters_among(leaves.iter().copied(), |x| *x == m, admin);
        assert_eq!(
            with.keys().cloned().collect::<Vec<_>>(),
            [hex::encode([30u8; 16])]
        );
    }

    /// Past its bound, the queue lets the oldest fall, counts them, and the next batch says how many; the emitter
    /// reports them apart and takes the facts once.
    #[test]
    fn the_queue_is_bounded_and_says_what_fell() {
        let mut state = State::default();
        let first = StationFact::of(&lock_refused(), 1).unwrap();
        push(&mut state, first.clone());
        for i in 0..MAX_QUEUE {
            push(
                &mut state,
                StationFact::of(&lock_refused(), 2 + i as i64).unwrap(),
            );
        }
        assert_eq!(state.pending.len(), MAX_QUEUE);
        assert_eq!(state.dropped, 1);
        assert!(
            state.pending.iter().all(|f| f.id != first.id),
            "the oldest fell"
        );
        let batch = next_batch(&state);
        assert_eq!(batch.dropped, 1);
        let mut emitter = State::default();
        let mut out = Taken::default();
        take_batch(
            &mut emitter,
            &mut out,
            MemberId([2; 16]),
            [7; 16],
            3,
            99,
            batch.clone(),
        );
        take_batch(
            &mut emitter,
            &mut out,
            MemberId([2; 16]),
            [7; 16],
            4,
            100,
            batch,
        );
        assert_eq!(
            out.facts.len(),
            MAX_BATCH,
            "a batch sent twice is taken once"
        );
        assert_eq!(out.dropped.len(), 1, "said once");
        assert_eq!(out.dropped[0].count, 1);
        assert!(out
            .facts
            .iter()
            .all(|f| f.seen == 99 && f.member == hex::encode([2u8; 16])));
    }

    #[test]
    fn a_batch_is_bounded() {
        let mut state = State::default();
        for _ in 0..(MAX_BATCH + 5) {
            state
                .pending
                .push_back(StationFact::of(&lock_refused(), 1).unwrap());
        }
        state.dropped = 3;
        let b = next_batch(&state);
        assert_eq!(b.facts.len(), MAX_BATCH);
        assert_eq!(b.dropped, 3);
        assert!(wire::encode(&b).len() <= MAX_BATCH_BYTES);
    }
}
