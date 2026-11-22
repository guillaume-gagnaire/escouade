//! The administrator's circuit breaker (TS-15, `docs/TEAM-SERVER.md` §9): an entry signed by an administrator that
//! takes something out of the team's space (a plugin, a harness, a model, or every agent), until an administrator
//! lifts it.
//!
//! - **Where it lives**: a stream of the server of its own ([`BREAKER_STREAM`], a project id no client draws), as the
//!   team's list of projects (TS-8). Each entry is an object sealed under the team's key of its epoch and signed by the
//!   leaf of its author: [`Entry::Set`] (a breaker set, or set again whole) or [`Entry::Lift`]. The server sees that an
//!   entry was written, its size and its hour; never what it takes out, nor why.
//! - **Who writes**: an administrator. The client refuses a member's write before it leaves; the server, which knows
//!   the roles, refuses a member's entry in this stream (defense in depth); every device refuses, at the reading, an
//!   entry sealed by a member who was not an administrator at its epoch (or, for an epoch this device did not live,
//!   is not one now), and an entry whose seal or signature does not hold (forged).
//! - **The `serial`**: each entry carries a number above every one this device read before (the team's highest,
//!   kept with the state). An entry whose serial is not above it is refused (`Refused::Replayed`): a server that
//!   plays an old breaker again, or an old lifting, changes nothing. Two administrators who write at once: the second
//!   entry read is refused, and its author writes it again above (`set`, `lift`).
//! - **It only restricts**: it says what is taken out; the core applies it through its locks (TS-11). Nothing here
//!   starts, installs or writes anything.
//! - **A lying server**: the device reads again the last entry it read before reading on. Gone, it is a rollback;
//!   changed, a rewritten entry: the reading stops and says it (`settings::Alarm`). What was in force stays in force.

use std::collections::{BTreeMap, BTreeSet};

use escouade_team_crypto::content::SealedObject;
use escouade_team_crypto::{MemberId, ObjectId, ProjectId, Role};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub use crate::settings::Alarm;
use crate::team::Team;
use crate::{wire, CryptoError, Error};

/// The stream of the team's circuit breakers: a project id no client draws (16 bytes). The server knows it, to keep
/// its writes to the administrators.
pub const BREAKER_STREAM: ProjectId = ProjectId(*b"escouade/breaker");

/// Where this device keeps what it read of the stream, in the team's encrypted base.
const STATE: &str = "breaker";

/// The longest reason.
pub const MAX_REASON: usize = 500;
/// The longest name of an author, or of a project.
pub const MAX_NAME: usize = 120;
/// The longest id of what is taken out (a plugin, a harness, a model).
pub const MAX_ID: usize = 128;
/// The breakers lifted kept for the window (the most recent).
const KEPT_LIFTED: usize = 20;
/// The refusals kept (the most recent).
const KEPT_REFUSALS: usize = 50;

/// What a breaker holds for: the whole team's space, or one project of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Scope {
    Team,
    /// A project of the team, by the id of its journal (hex, the Kanban's: `Project::team_project` on each device), and
    /// its name as the administrator saw it.
    Project {
        project: String,
        name: String,
    },
}

/// What a breaker takes out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum What {
    /// A plugin: switched off on each device, then uninstalled when `uninstall` says so.
    Plugin { id: String, uninstall: bool },
    /// A harness: no agent of the team starts on it; those that run stop between two turns.
    Harness { id: String },
    /// A model: the same.
    Model { id: String },
    /// Every agent of the team's projects: as quitting the app, the turn under way lost.
    Agents,
}

/// An entry of the stream, as it is sealed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Entry {
    Set {
        id: [u8; 16],
        serial: u64,
        scope: Scope,
        what: What,
        reason: String,
        /// The name the administrator signs with (« Claire Martin »).
        by: String,
        at: u64,
    },
    Lift {
        id: [u8; 16],
        serial: u64,
        by: String,
        at: u64,
    },
}

impl Entry {
    pub fn serial(&self) -> u64 {
        match self {
            Entry::Set { serial, .. } | Entry::Lift { serial, .. } => *serial,
        }
    }
}

/// A breaker in force.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Breaker {
    /// Its id (hex).
    pub id: String,
    pub scope: Scope,
    pub what: What,
    pub reason: String,
    /// The name its administrator signed with.
    pub by: String,
    /// The member who signed it (their id, hex).
    pub member: String,
    pub at: u64,
    pub serial: u64,
    /// The number of its entry in the stream.
    pub seq: u64,
}

impl Breaker {
    /// Whether it holds for the project of the journal `journal` (hex; None: a project of no journal of the team).
    pub fn holds_for(&self, journal: Option<&str>) -> bool {
        match &self.scope {
            Scope::Team => true,
            Scope::Project { project, .. } => {
                journal.is_some_and(|j| j.eq_ignore_ascii_case(project))
            }
        }
    }
}

/// A breaker lifted, with who lifted it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lifted {
    pub breaker: Breaker,
    pub by: String,
    pub member: String,
    pub at: u64,
}

/// Why an entry of the stream was refused.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Refused {
    /// Sealed by a member who was not an administrator.
    NotAdmin,
    /// Its seal or its signature does not hold: forged, or of no member of the team.
    Unsigned,
    /// Its serial is not above the highest this device read.
    Replayed { serial: u64, highest: u64 },
    /// What it says does not pass (`check`).
    Bad { why: String },
    /// Its bytes do not decode.
    Unreadable { why: String },
    /// A lifting of no breaker in force.
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Refusal {
    pub seq: u64,
    pub reason: Refused,
}

/// What this device keeps of the stream.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct State {
    active: BTreeMap<String, Breaker>,
    lifted: Vec<Lifted>,
    /// The highest serial read.
    serial: u64,
    refusals: Vec<Refusal>,
    cursor: u64,
    last: Option<(u64, [u8; 32])>,
    alarm: Option<Alarm>,
}

/// The breakers as this device read them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    /// In force, the most recent first.
    pub active: Vec<Breaker>,
    /// Lifted, the most recent first.
    pub lifted: Vec<Lifted>,
    pub refusals: Vec<Refusal>,
    pub alarm: Option<Alarm>,
    /// The highest serial read.
    pub serial: u64,
}

fn view_of(state: &State) -> View {
    let mut active: Vec<Breaker> = state.active.values().cloned().collect();
    active.sort_by_key(|b| std::cmp::Reverse(b.serial));
    let mut lifted = state.lifted.clone();
    lifted.reverse();
    View {
        active,
        lifted,
        refusals: state.refusals.clone(),
        alarm: state.alarm,
        serial: state.serial,
    }
}

// === What an entry may say ===

fn line(text: &str, max: usize, what: &str) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err(format!("{what}: empty"));
    }
    if text.chars().count() > max {
        return Err(format!("{what}: longer than {max} characters"));
    }
    if text.chars().any(|c| c.is_control() && c != '\n') {
        return Err(format!("{what}: a control character"));
    }
    Ok(())
}

fn ident(id: &str, what: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > MAX_ID {
        return Err(format!("{what}: not an id"));
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "._:/@-+".contains(c))
        || id.starts_with('-')
    {
        return Err(format!("{what}: « {id} » is not an id"));
    }
    Ok(())
}

/// What a breaker says, checked: a reason and a name, an id of what it takes out, a project by the id of a journal.
pub fn check(scope: &Scope, what: &What, reason: &str, by: &str) -> Result<(), String> {
    line(reason, MAX_REASON, "the reason")?;
    line(by, MAX_NAME, "the name")?;
    if by.contains('\n') {
        return Err("the name: on one line".into());
    }
    if let Scope::Project { project, name } = scope {
        if project.len() != 32 || !project.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("the project: not the id of a journal".into());
        }
        if name.chars().count() > MAX_NAME || name.chars().any(char::is_control) {
            return Err("the project's name".into());
        }
    }
    match what {
        What::Plugin { id, .. } => ident(id, "the plugin"),
        What::Harness { id } => ident(id, "the harness"),
        What::Model { id } => ident(id, "the model"),
        What::Agents => Ok(()),
    }
}

// === Reading ===

enum Opened {
    Refused(Refused),
    Fork,
    /// An epoch this device has not reached yet: read again later.
    Later,
}

fn entry_digest(e: &wire::Entry) -> [u8; 32] {
    Sha256::digest(wire::encode(e)).into()
}

fn admins_now(team: &Team) -> BTreeSet<MemberId> {
    team.roster()
        .map(|r| {
            r.members
                .iter()
                .filter(|(_, role)| **role == Role::Admin)
                .map(|(m, _)| *m)
                .collect()
        })
        .unwrap_or_default()
}

/// Opens an entry: its seal, its signer, an administrator at its epoch (or now, for an epoch not lived here), and what
/// it says checked.
fn open(team: &Team, e: &wire::Entry) -> Result<(MemberId, Entry), Opened> {
    let unreadable = |why: String| Opened::Refused(Refused::Unreadable { why });
    let sealed: SealedObject = wire::decode(&e.payload).map_err(unreadable)?;
    if sealed.header.project != BREAKER_STREAM {
        return Err(unreadable("an object of another stream".into()));
    }
    let plain = match team.device().open(&sealed) {
        Ok(p) => p,
        Err(CryptoError::Fork { .. }) => return Err(Opened::Fork),
        Err(CryptoError::Behind(_)) => return Err(Opened::Later),
        Err(_) => return Err(Opened::Refused(Refused::Unsigned)),
    };
    let who = team
        .signer_at(sealed.header.epoch, &sealed.header.author)
        .ok_or(Opened::Refused(Refused::Unsigned))?;
    let admin = match team.admins_at(sealed.header.epoch) {
        Some(then) => then.contains(&who.member),
        None => admins_now(team).contains(&who.member),
    };
    if !admin {
        return Err(Opened::Refused(Refused::NotAdmin));
    }
    let entry: Entry = wire::decode(&plain).map_err(unreadable)?;
    let checked = match &entry {
        Entry::Set {
            scope,
            what,
            reason,
            by,
            ..
        } => check(scope, what, reason, by),
        Entry::Lift { by, .. } => line(by, MAX_NAME, "the name"),
    };
    checked.map_err(|why| Opened::Refused(Refused::Bad { why }))?;
    Ok((who.member, entry))
}

fn refuse(state: &mut State, seq: u64, reason: Refused) {
    state.refusals.push(Refusal { seq, reason });
    let over = state.refusals.len().saturating_sub(KEPT_REFUSALS);
    state.refusals.drain(..over);
}

/// One entry that opened, applied (or refused: a serial not above the highest, a lifting of nothing).
fn apply(state: &mut State, seq: u64, member: MemberId, entry: Entry) {
    let serial = entry.serial();
    if serial <= state.serial {
        let highest = state.serial;
        refuse(state, seq, Refused::Replayed { serial, highest });
        return;
    }
    match entry {
        Entry::Set {
            id,
            serial,
            scope,
            what,
            reason,
            by,
            at,
        } => {
            state.serial = serial;
            let id = hex::encode(id);
            state.active.insert(
                id.clone(),
                Breaker {
                    id,
                    scope,
                    what,
                    reason,
                    by,
                    member: member.to_string(),
                    at,
                    serial,
                    seq,
                },
            );
        }
        Entry::Lift { id, serial, by, at } => {
            let Some(breaker) = state.active.remove(&hex::encode(id)) else {
                refuse(state, seq, Refused::Unknown);
                return;
            };
            state.serial = serial;
            state.lifted.push(Lifted {
                breaker,
                by,
                member: member.to_string(),
                at,
            });
            let over = state.lifted.len().saturating_sub(KEPT_LIFTED);
            state.lifted.drain(..over);
        }
    }
}

async fn read(team: &Team, state: &mut State) -> Result<(), Error> {
    let client = team.client().clone();
    if let Some((seq, d)) = state.last {
        let page = client
            .read_project(&BREAKER_STREAM.0, seq - 1, Some(1))
            .await?;
        match page.entries.first() {
            Some(e) if e.seq == seq => {
                if entry_digest(e) != d {
                    state.alarm = Some(Alarm::Rewritten);
                    return Ok(());
                }
            }
            _ => {
                state.alarm = Some(Alarm::Rollback);
                return Ok(());
            }
        }
    }
    loop {
        let page = client
            .read_project(&BREAKER_STREAM.0, state.cursor, None)
            .await?;
        for e in &page.entries {
            if e.seq <= state.cursor {
                state.alarm = Some(Alarm::Server);
                return Ok(());
            }
            match open(team, e) {
                Ok((member, entry)) => apply(state, e.seq, member, entry),
                Err(Opened::Refused(reason)) => refuse(state, e.seq, reason),
                Err(Opened::Fork) => {
                    state.alarm = Some(Alarm::Fork);
                    return Ok(());
                }
                Err(Opened::Later) => return Ok(()),
            }
            state.cursor = e.seq;
            state.last = Some((e.seq, entry_digest(e)));
        }
        if !page.more || page.entries.is_empty() {
            return Ok(());
        }
    }
}

fn load(team: &Team) -> Result<State, Error> {
    Ok(team.load_extra(STATE)?.unwrap_or_default())
}

fn save(team: &Team, state: &State) -> Result<(), Error> {
    team.save_extra(STATE, state)
}

/// Reads what is new in the stream. Under an alarm, nothing is read until [`retry`]: what was in force stays so.
pub async fn sync(team: &mut Team) -> Result<View, Error> {
    let mut state = load(team)?;
    if state.alarm.is_none() {
        read(team, &mut state).await?;
    }
    save(team, &state)?;
    Ok(view_of(&state))
}

/// The breakers as this device read them last, without reading the server (offline, at the start).
pub fn view(team: &Team) -> Result<View, Error> {
    Ok(view_of(&load(team)?))
}

/// « Réessayer »: the alarm lifted, the stream read again from where it was.
pub async fn retry(team: &mut Team) -> Result<View, Error> {
    let mut state = load(team)?;
    state.alarm = None;
    save(team, &state)?;
    sync(team).await
}

// === Writing ===

fn admin(team: &Team) -> Result<(), Error> {
    if team.is_admin()? {
        Ok(())
    } else {
        Err(CryptoError::NotAdmin.into())
    }
}

/// The entry sealed by this device, as it goes to the server: a fresh object id each time.
pub fn seal(team: &Team, entry: &Entry) -> Result<Vec<u8>, Error> {
    let mut object = [0u8; 16];
    getrandom::fill(&mut object).map_err(|e| Error::Encoding(e.to_string()))?;
    let sealed = team
        .device()
        .seal(BREAKER_STREAM, ObjectId(object), 0, &wire::encode(entry))?;
    Ok(wire::encode(&sealed))
}

/// Sends the entry `make(serial)` above the highest serial read; when another administrator's entry took that serial
/// first (it is refused as replayed at the reading), made again above it, three times at most.
async fn append(team: &mut Team, make: impl Fn(u64) -> Entry) -> Result<View, Error> {
    for _ in 0..3 {
        let view = sync(team).await?;
        if view.alarm.is_some() {
            return Err(Error::Refused(
                "the server shows what cannot be in the team's circuit breakers: nothing is sent"
                    .into(),
            ));
        }
        let payload = seal(team, &make(view.serial + 1))?;
        let seq = team
            .client()
            .append_project(&BREAKER_STREAM.0, &payload)
            .await?;
        let after = sync(team).await?;
        let lost = after
            .refusals
            .iter()
            .any(|r| r.seq == seq && matches!(r.reason, Refused::Replayed { .. }));
        if !lost {
            return Ok(after);
        }
    }
    Err(Error::Refused(
        "other administrators write the circuit breakers at the same time: try again".into(),
    ))
}

fn draw_id() -> Result<[u8; 16], Error> {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).map_err(|e| Error::Encoding(e.to_string()))?;
    Ok(b)
}

fn id_of(hex_id: &str) -> Result<[u8; 16], Error> {
    let mut b = [0u8; 16];
    hex::decode_to_slice(hex_id, &mut b).map_err(|e| Error::Encoding(e.to_string()))?;
    Ok(b)
}

/// An administrator sets a breaker: checked before it leaves. Its id and the view after.
pub async fn set(
    team: &mut Team,
    scope: Scope,
    what: What,
    reason: &str,
    by: &str,
    at: u64,
) -> Result<(String, View), Error> {
    admin(team)?;
    let reason = reason.trim().to_owned();
    let by = by.trim().to_owned();
    check(&scope, &what, &reason, &by).map_err(Error::Refused)?;
    let id = draw_id()?;
    let view = append(team, |serial| Entry::Set {
        id,
        serial,
        scope: scope.clone(),
        what: what.clone(),
        reason: reason.clone(),
        by: by.clone(),
        at,
    })
    .await?;
    Ok((hex::encode(id), view))
}

/// An administrator lifts a breaker in force.
pub async fn lift(team: &mut Team, id: &str, by: &str, at: u64) -> Result<View, Error> {
    admin(team)?;
    let by = by.trim().to_owned();
    line(&by, MAX_NAME, "the name").map_err(Error::Refused)?;
    if !view(team)?.active.iter().any(|b| b.id == id) {
        return Err(Error::Refused(
            "no circuit breaker of this id is in force".into(),
        ));
    }
    let raw = id_of(id)?;
    append(team, |serial| Entry::Lift {
        id: raw,
        serial,
        by: by.clone(),
        at,
    })
    .await
}

/// The other devices of the team with a live connection open now: what the confirmation of a breaker says will stop
/// « sur N postes en ligne ». Those offline apply it when they come back.
pub async fn online(team: &Team) -> Result<usize, Error> {
    let me = team.client().device().to_vec();
    Ok(team
        .client()
        .devices()
        .await?
        .iter()
        .filter(|d| d.online && !d.revoked && d.id != me)
        .count())
}

/// The device of an administrator writes again the breakers in force whose author is no longer an administrator (they
/// left, or lost the role): a device that comes later, which did not live their epoch, reads them. The number written
/// again.
pub async fn serve(team: &mut Team, at: u64) -> Result<usize, Error> {
    if !team.is_admin().unwrap_or(false) {
        return Ok(0);
    }
    let current = view(team)?;
    if current.alarm.is_some() {
        return Ok(0);
    }
    let admins: BTreeSet<String> = admins_now(team).iter().map(|m| m.to_string()).collect();
    let mut done = 0;
    for b in current
        .active
        .iter()
        .filter(|b| !admins.contains(&b.member))
    {
        let id = id_of(&b.id)?;
        append(team, |serial| Entry::Set {
            id,
            serial,
            scope: b.scope.clone(),
            what: b.what.clone(),
            reason: b.reason.clone(),
            by: b.by.clone(),
            at,
        })
        .await?;
        done += 1;
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(serial: u64, id: u8) -> Entry {
        Entry::Set {
            id: [id; 16],
            serial,
            scope: Scope::Team,
            what: What::Harness { id: "codex".into() },
            reason: "fuite de clé".into(),
            by: "Claire Martin".into(),
            at: 1,
        }
    }

    #[test]
    fn a_serial_never_goes_back() {
        let mut s = State::default();
        apply(&mut s, 1, MemberId([1; 16]), set(1, 1));
        assert_eq!(s.active.len(), 1);
        // The same entry again (a replay), then an older serial: refused, nothing changes.
        apply(&mut s, 2, MemberId([1; 16]), set(1, 1));
        apply(&mut s, 3, MemberId([1; 16]), set(0, 2));
        assert_eq!(s.active.len(), 1);
        assert_eq!(
            s.refusals,
            vec![
                Refusal {
                    seq: 2,
                    reason: Refused::Replayed {
                        serial: 1,
                        highest: 1
                    }
                },
                Refusal {
                    seq: 3,
                    reason: Refused::Replayed {
                        serial: 0,
                        highest: 1
                    }
                },
            ]
        );
        // Lifted above, then the old lifting played again: refused.
        let lift = Entry::Lift {
            id: [1; 16],
            serial: 2,
            by: "Claire Martin".into(),
            at: 2,
        };
        apply(&mut s, 4, MemberId([1; 16]), lift.clone());
        assert!(s.active.is_empty());
        assert_eq!(s.lifted.len(), 1);
        apply(&mut s, 5, MemberId([1; 16]), set(3, 1));
        apply(&mut s, 6, MemberId([1; 16]), lift);
        assert_eq!(s.active.len(), 1, "the old lifting lifts nothing");
        // A lifting of nothing.
        apply(
            &mut s,
            7,
            MemberId([1; 16]),
            Entry::Lift {
                id: [9; 16],
                serial: 9,
                by: "x".into(),
                at: 3,
            },
        );
        assert_eq!(s.refusals.last().unwrap().reason, Refused::Unknown);
        assert_eq!(s.serial, 3, "a refused entry does not take the serial");
    }

    #[test]
    fn what_a_breaker_says_is_checked() {
        let ok = What::Harness { id: "codex".into() };
        assert_eq!(check(&Scope::Team, &ok, "fuite", "Claire"), Ok(()));
        assert!(check(&Scope::Team, &ok, "  ", "Claire").is_err());
        assert!(check(&Scope::Team, &ok, "fuite", "").is_err());
        assert!(check(&Scope::Team, &ok, "fuite", "a\nb").is_err());
        assert!(check(&Scope::Team, &ok, &"x".repeat(MAX_REASON + 1), "c").is_err());
        assert!(check(&Scope::Team, &ok, "a\u{7}b", "c").is_err());
        for bad in ["", "--upload", "a b", "a;b", &"a".repeat(MAX_ID + 1)] {
            assert!(
                check(
                    &Scope::Team,
                    &What::Plugin {
                        id: bad.into(),
                        uninstall: false
                    },
                    "r",
                    "c"
                )
                .is_err(),
                "{bad}"
            );
        }
        assert_eq!(
            check(
                &Scope::Team,
                &What::Model {
                    id: "anthropic/claude-opus-5-5".into()
                },
                "r",
                "c"
            ),
            Ok(())
        );
        let p = |project: &str| Scope::Project {
            project: project.into(),
            name: "API".into(),
        };
        assert!(check(&p(&"ab".repeat(16)), &What::Agents, "r", "c").is_ok());
        assert!(check(&p("ab"), &What::Agents, "r", "c").is_err());
        assert!(check(&p(&"zz".repeat(16)), &What::Agents, "r", "c").is_err());
    }

    #[test]
    fn a_project_breaker_holds_for_its_journal_alone() {
        let b = |scope| Breaker {
            id: String::new(),
            scope,
            what: What::Agents,
            reason: String::new(),
            by: String::new(),
            member: String::new(),
            at: 0,
            serial: 1,
            seq: 1,
        };
        let team = b(Scope::Team);
        assert!(team.holds_for(None));
        assert!(team.holds_for(Some("ab")));
        let one = b(Scope::Project {
            project: "AB".repeat(16),
            name: String::new(),
        });
        assert!(one.holds_for(Some(&"ab".repeat(16))));
        assert!(!one.holds_for(Some(&"cd".repeat(16))));
        assert!(!one.holds_for(None));
    }
}
