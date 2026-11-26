//! The team's `.escouade.yml` (TS-13, `docs/TEAM-SERVER.md` §6.1, §6.2, §6.5 and §6.6): signed, chained, encrypted
//! under the team's key, never in a repository.
//!
//! - **Where it lives**: a stream of the server of its own ([`SETTINGS_STREAM`], a project id no client draws), each
//!   entry an object sealed under the team's key of its epoch and signed by the leaf of its author (the device key of
//!   a member, `escouade-team-crypto::content`). The server reads nothing of it, and can forge nothing: an entry no
//!   leaf signed does not open.
//! - **A version** ([`Version`]) names its number and the digest of the one before: the versions are a chain, and a
//!   device never goes back on it (a version older than the one it applies is refused, one it saw already is a replay).
//!   A **cosignature** names a version's digest: a second administrator's agreement.
//! - **Who signs**: an administrator of the roster at the epoch the entry was sealed at (when this device lived that
//!   epoch, `Team::admins_at`), and of the roster now: a version of an administrator removed since does not count,
//!   and never applies on a device that sees it after the removal. The administrators still there cosign the versions
//!   of the chain whose signers left ([`serve`]), so that a device that joins later reads the whole chain.
//! - **The quorum** (C12-B): a version that changes a sensitive key, the list of the signers or the quorum itself needs
//!   two administrators (two members: two devices of one administrator count once). It is the team's option
//!   (`quorum`), on by default as soon as the team has two administrators. Until its second signature, a version is
//!   proposed and nothing of it applies. With one administrator, the default is off: one signature is enough, and the
//!   members are told the team has a single administrator.
//! - **What never enters, even signed** ([`never`]): a list written here, that no team changes. A version that holds
//!   one of them is refused at the reading, whole.
//! - **The sorts** (§6.5, point 3) come from the core ([`Classify`]): what restricts and what is a value apply; what is
//!   sensitive waits for the consent of this device; what runs is never imposed, only proposed, and applies once this
//!   device accepted it at its fingerprint (a change asks again).
//! - **A lying server**: the device reads again the last entry it read before reading on. Gone, it is a rollback;
//!   changed, a rewritten entry: the reading stops and says it ([`Alarm`]).

use std::collections::{BTreeMap, BTreeSet};

use escouade_team_crypto::content::SealedObject;
use escouade_team_crypto::{DeviceId, MemberId, ObjectId, ProjectId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::team::Team;
use crate::{wire, CryptoError, Error};

/// The stream of the team's settings: a project id no client draws (16 bytes).
pub const SETTINGS_STREAM: ProjectId = ProjectId(*b"escouade/setting");

/// The format of the file this client reads.
pub const FORMAT: u32 = 1;

/// Where this device keeps what it read of the stream, in the team's encrypted base.
const STATE: &str = "settings";

/// What a key may do when a team sets it (§6.5): the core's single list says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Sort {
    Restrict,
    Value,
    Sensitive,
    Executable,
}

/// The core's list of the keys: the sort of each, None for a key it does not know (left out, said).
pub trait Classify: Sync {
    fn sort(&self, key: &str) -> Option<Sort>;

    /// Whether `value` of `key` waits for each device's consent, and is under the quorum: a sensitive key whatever
    /// its value; the core's list may name a value of another key that asks too (the mode « auto » imposed, SR3-07).
    fn asks(&self, key: &str, value: &Value) -> bool {
        let _ = value;
        self.sort(key) == Some(Sort::Sensitive)
    }
}

/// The team's file, as read from its YAML by the core: each part a map of the list's keys (dotted) to their value.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TeamFile {
    pub version: u32,
    /// Rank 2: recommended, the repository and the device change it.
    pub defaults: BTreeMap<String, Value>,
    /// Rank 5: imposed.
    pub enforced: BTreeMap<String, Value>,
    /// What runs: proposed, read and accepted on each device.
    pub proposed: BTreeMap<String, Value>,
    /// Why, by key: shown under the lock.
    pub reasons: BTreeMap<String, String>,
    /// The quorum of two administrators for what is sensitive (C12-B); None: on as soon as there are two.
    pub quorum: Option<bool>,
    /// The members who may sign the settings; empty: every administrator.
    pub signers: Vec<MemberId>,
}

/// A part of the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Part {
    Defaults,
    Enforced,
    Proposed,
}

impl Part {
    pub fn name(self) -> &'static str {
        match self {
            Part::Defaults => "defaults",
            Part::Enforced => "enforced",
            Part::Proposed => "proposed",
        }
    }
}

impl TeamFile {
    pub fn part(&self, part: Part) -> &BTreeMap<String, Value> {
        match part {
            Part::Defaults => &self.defaults,
            Part::Enforced => &self.enforced,
            Part::Proposed => &self.proposed,
        }
    }

    fn parts(&self) -> impl Iterator<Item = (Part, &String, &Value)> {
        [Part::Defaults, Part::Enforced, Part::Proposed]
            .into_iter()
            .flat_map(move |p| self.part(p).iter().map(move |(k, v)| (p, k, v)))
    }
}

// === What never enters, even signed ===

/// A line of the list of what never enters (§6.5), whatever the team signs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Never {
    /// `bypassPermissions`, named or allowed.
    Bypass,
    /// A command that would run without this device's consent: anything that runs, outside `proposed`; or a
    /// proposal marked as accepted already.
    CommandWithoutConsent,
    /// A plugin installed without the member's gesture.
    PluginWithoutGesture,
    /// A `dev.escouade.*` signed by the team (its private store).
    FirstParty,
    /// A project made public by the file.
    MadePublic,
}

/// Where the file holds a line of the list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeverHit {
    pub rule: Never,
    pub part: Part,
    pub key: String,
}

/// The keys that run, whatever the core's list says (a key added to the list without its sort still never runs).
const RUNS: &[&str] = &[
    "worktrees.setup",
    "worktrees.teardown",
    "launch",
    "board.test_command",
    "hooks",
    "mcp.servers",
    "agents.env",
    "env",
    "plugins.install",
];

/// The keys that install a plugin.
const INSTALLS: &[&str] = &["plugins.install", "plugins.required", "plugins.autoInstall"];

/// What would say a proposal accepted on the devices without them.
const PRE_ACCEPTED: &[&str] = &["trusted", "approved", "autoRun", "autoApprove", "accepted"];

fn under(key: &str, group: &str) -> bool {
    key == group
        || key
            .strip_prefix(group)
            .is_some_and(|rest| rest.starts_with('.'))
}

fn strings_in(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| strings_in(x, out)),
        Value::Object(m) => m.values().for_each(|x| strings_in(x, out)),
        _ => {}
    }
}

fn first_party(s: &str) -> bool {
    let s = s.trim().to_ascii_lowercase();
    s == "dev.escouade" || s.starts_with("dev.escouade.")
}

/// A plugin of the team's store named in an install: `{id, origin: team-store}` or `team-store:<id>`.
fn team_store_first_party(v: &Value) -> bool {
    match v {
        Value::Array(a) => a.iter().any(team_store_first_party),
        Value::Object(m) => {
            let origin = m.get("origin").and_then(Value::as_str).unwrap_or_default();
            let id = m.get("id").and_then(Value::as_str).unwrap_or_default();
            origin == "team-store" && first_party(id)
        }
        Value::String(s) => s.strip_prefix("team-store:").is_some_and(first_party),
        _ => false,
    }
}

/// The first line of the list the file holds, if any: such a version is refused whole, at the reading as at the
/// signing. One test per line (`tests/settings.rs`).
pub fn never(file: &TeamFile, classify: &dyn Classify) -> Option<NeverHit> {
    let hit = |rule, part, key: &str| {
        Some(NeverHit {
            rule,
            part,
            key: key.to_owned(),
        })
    };
    for (part, key, v) in file.parts() {
        let mut texts = Vec::new();
        strings_in(v, &mut texts);
        if texts.iter().any(|s| s.trim() == "bypassPermissions")
            || key == "permissions.allowBypass" && v == &Value::Bool(true)
        {
            return hit(Never::Bypass, part, key);
        }
    }
    for (part, key, v) in file.parts() {
        let public = (key == "private" || key == "projects.forcePrivate")
            && v == &Value::Bool(false)
            || key.rsplit('.').next() == Some("public") && v == &Value::Bool(true);
        if public {
            return hit(Never::MadePublic, part, key);
        }
    }
    for (part, key, v) in file.parts() {
        let namespace = key.rsplit('.').next() == Some("namespace") && {
            let mut texts = Vec::new();
            strings_in(v, &mut texts);
            texts.iter().any(|s| first_party(s))
        };
        if namespace || INSTALLS.iter().any(|g| under(key, g)) && team_store_first_party(v) {
            return hit(Never::FirstParty, part, key);
        }
    }
    for (part, key, _) in file.parts() {
        if part != Part::Proposed && INSTALLS.iter().any(|g| under(key, g)) {
            return hit(Never::PluginWithoutGesture, part, key);
        }
    }
    for (part, key, _) in file.parts() {
        let runs =
            RUNS.iter().any(|g| under(key, g)) || classify.sort(key) == Some(Sort::Executable);
        let pre_accepted = key
            .rsplit('.')
            .next()
            .is_some_and(|last| PRE_ACCEPTED.contains(&last));
        if runs && part != Part::Proposed || pre_accepted {
            return hit(Never::CommandWithoutConsent, part, key);
        }
    }
    None
}

// === The entries ===

/// A version of the file, as its author signed it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Version {
    pub number: u64,
    /// The digest of the version before (None for the first).
    #[serde(with = "opt_digest")]
    pub previous: Option<[u8; 32]>,
    /// The file, as JSON (its maps sorted).
    pub file: String,
    /// The author's name, as they gave it (« Claire Martin »); who signed is the leaf.
    pub by: String,
    /// Seconds since the epoch, on the author's device.
    pub at: u64,
    /// What the author says of the change.
    pub note: String,
}

mod opt_digest {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &Option<[u8; 32]>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(d) => s.serialize_some(&hex::encode(d)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<[u8; 32]>, D::Error> {
        let text: Option<String> = Option::deserialize(d)?;
        text.map(|t| {
            let mut out = [0u8; 32];
            hex::decode_to_slice(&t, &mut out).map_err(serde::de::Error::custom)?;
            Ok(out)
        })
        .transpose()
    }
}

/// What an entry of the stream carries.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Entry {
    /// A version: its encoding, whose digest the next version and the cosignatures name.
    Version {
        #[serde(with = "serde_bytes")]
        body: Vec<u8>,
    },
    /// An administrator's agreement to a version.
    Cosign {
        number: u64,
        #[serde(with = "hex::serde")]
        digest: [u8; 32],
    },
}

/// The digest of a version: what the chain and the cosignatures name.
pub fn digest(body: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"escouade/team-settings/v1\0");
    h.update(body);
    h.finalize().into()
}

/// The fingerprint of a value of a key, which this device's consent names: a change asks again.
pub fn fingerprint(part: Part, key: &str, value: &Value) -> String {
    let mut h = Sha256::new();
    h.update(part.name().as_bytes());
    h.update(b"\0");
    h.update(key.as_bytes());
    h.update(b"\0");
    h.update(serde_json::to_string(value).unwrap_or_default().as_bytes());
    hex::encode(h.finalize())
}

fn consent_name(part: Part, key: &str) -> String {
    format!("{}.{key}", part.name())
}

// === What a device keeps ===

/// Why an entry, or a version, is refused.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Refused {
    /// Signed by no leaf of the team (unsigned, or forged by the server), or not readable.
    Unsigned,
    /// Signed by a member who was not an administrator at the epoch it was sealed at, or is not among the signers.
    NotAdmin,
    /// Signed by an administrator removed (or no longer one) since.
    RemovedAdmin,
    /// A line of what never enters.
    Never { hit: NeverHit },
    /// It does not follow the chain this device holds: another version before, a number skipped.
    BadChain,
    /// Older than the version applied, or of a number already taken: it goes back.
    Behind,
    /// Seen already: the server served it again.
    Replayed,
    /// Another version of its number came first.
    Lost,
    /// A format this client does not read, or an entry that does not decode.
    Unreadable { why: String },
}

/// What the server showed that cannot be: the reading stops on this device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Alarm {
    /// The last entry read is gone.
    Rollback,
    /// The last entry read changed.
    Rewritten,
    /// The numbering went back.
    Server,
    /// An entry cites another head of an epoch this device holds (a fork of the team).
    Fork,
}

/// An entry that opened: who sealed it, at which epoch.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Seen {
    seq: u64,
    epoch: u64,
    member: MemberId,
    device: DeviceId,
    entry: Entry,
}

/// An entry refused at the reading, before any version could be made of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Refusal {
    pub seq: u64,
    pub reason: Refused,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct State {
    cursor: u64,
    /// The last entry read: its number and the digest of its bytes.
    last: Option<(u64, [u8; 32])>,
    seen: Vec<Seen>,
    refusals: Vec<Refusal>,
    /// The chain this device chose, version by version: never undone.
    chosen: Vec<[u8; 32]>,
    /// This device's consents: `<part>.<key>` to the fingerprint of the value accepted.
    consents: BTreeMap<String, String>,
    alarm: Option<Alarm>,
}

// === What the window shows ===

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Status {
    /// The version in force.
    Applied,
    /// In force before a later one.
    Superseded,
    /// Waits for its second signature (or a signature of an administrator of now): nothing of it applies.
    Proposed,
    Refused {
        reason: Refused,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionView {
    pub number: u64,
    pub digest: String,
    pub by: String,
    /// The author's member id.
    pub author: String,
    pub at: u64,
    pub note: String,
    pub file: TeamFile,
    /// The members whose signature counts, now.
    pub signers: Vec<String>,
    /// The signatures it needs: 2 under the quorum.
    pub need: usize,
    /// This device's member signed it.
    pub signed_by_me: bool,
    pub status: Status,
}

/// A key of the version in force that waits for this device.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pending {
    pub part: Part,
    pub key: String,
    pub value: Value,
    pub sort: Sort,
    /// What the consent names.
    pub fingerprint: String,
}

/// What applies on this device, from the version in force.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Effective {
    /// Rank 2.
    pub defaults: BTreeMap<String, Value>,
    /// Rank 5: what restricts, the values, the sensitive keys this device accepted.
    pub enforced: BTreeMap<String, Value>,
    /// What runs, accepted on this device at its fingerprint.
    pub accepted: BTreeMap<String, Value>,
    /// The sensitive keys and the proposals that wait for this device.
    pub pending: Vec<Pending>,
    /// The keys the core does not know: left out.
    pub unknown: Vec<String>,
    pub reasons: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    /// The number of the version in force.
    pub applied: Option<u64>,
    /// Every version, the newest first.
    pub versions: Vec<VersionView>,
    /// The entries refused before being versions (unsigned, forged).
    pub refusals: Vec<Refusal>,
    pub alarm: Option<Alarm>,
    /// The quorum holds for what is sensitive.
    pub quorum: bool,
    pub admins: usize,
    pub effective: Effective,
}

// === The rules ===

/// What the rules read of the team, now.
struct Context<'a> {
    now_admins: BTreeSet<MemberId>,
    admins_at: &'a dyn Fn(u64) -> Option<BTreeSet<MemberId>>,
    classify: &'a dyn Classify,
}

struct Candidate {
    digest: [u8; 32],
    version: Version,
    file: Option<TeamFile>,
    author: MemberId,
    epoch: u64,
    refused: Option<Refused>,
}

/// Whether two versions differ on what the quorum guards: a sensitive key, the signers, the quorum.
fn guarded(prev: Option<&TeamFile>, next: &TeamFile, classify: &dyn Classify) -> bool {
    let empty = TeamFile::default();
    let prev = prev.unwrap_or(&empty);
    if prev.signers != next.signers || prev.quorum != next.quorum {
        return true;
    }
    [Part::Defaults, Part::Enforced, Part::Proposed]
        .into_iter()
        .any(|part| {
            let (a, b) = (prev.part(part), next.part(part));
            a.keys().chain(b.keys()).any(|k| {
                let (x, y) = (a.get(k), b.get(k));
                x != y
                    && (x.is_some_and(|v| classify.asks(k, v))
                        || y.is_some_and(|v| classify.asks(k, v)))
            })
        })
}

/// Whether `next`, after `prev` (None: the first version), needs the quorum's two signatures when the quorum holds.
pub fn needs_quorum(prev: Option<&TeamFile>, next: &TeamFile, classify: &dyn Classify) -> bool {
    guarded(prev, next, classify)
}

struct Evaluated {
    chain: Vec<usize>,
    statuses: BTreeMap<usize, Status>,
    counted: BTreeMap<usize, BTreeSet<MemberId>>,
    need: BTreeMap<usize, usize>,
    quorum: bool,
}

fn evaluate(
    cands: &[Candidate],
    cosigns: &BTreeMap<[u8; 32], Vec<(MemberId, u64)>>,
    chosen: &mut Vec<[u8; 32]>,
    cx: &Context,
) -> Evaluated {
    let mut out = Evaluated {
        chain: Vec::new(),
        statuses: BTreeMap::new(),
        counted: BTreeMap::new(),
        need: BTreeMap::new(),
        quorum: cx.now_admins.len() >= 2,
    };
    // Who counts for a version, after `prev`: an administrator then (when this device lived the epoch) and now,
    // among the signers of `prev` when it names some; each member once.
    let count = |i: usize, prev: Option<&TeamFile>| -> BTreeSet<MemberId> {
        let c = &cands[i];
        let mut all = vec![(c.author, c.epoch)];
        all.extend(cosigns.get(&c.digest).into_iter().flatten().copied());
        all.into_iter()
            .filter(|(m, e)| {
                (cx.admins_at)(*e).is_none_or(|s| s.contains(m))
                    && cx.now_admins.contains(m)
                    && prev.is_none_or(|p| p.signers.is_empty() || p.signers.contains(m))
            })
            .map(|(m, _)| m)
            .collect()
    };
    let mut prev_file: Option<TeamFile> = None;
    let mut prev_digest: Option<[u8; 32]> = None;
    loop {
        let k = out.chain.len() as u64 + 1;
        let quorum = prev_file
            .as_ref()
            .and_then(|f| f.quorum)
            .unwrap_or(cx.now_admins.len() >= 2);
        out.quorum = quorum;
        let here: Vec<usize> = (0..cands.len())
            .filter(|&i| {
                let c = &cands[i];
                c.refused.is_none() && c.version.number == k && c.version.previous == prev_digest
            })
            .collect();
        if here.is_empty() {
            break;
        }
        for &i in &here {
            let file = cands[i]
                .file
                .as_ref()
                .expect("a candidate not refused has its file");
            let need = if quorum && guarded(prev_file.as_ref(), file, cx.classify) {
                2
            } else {
                1
            };
            out.need.insert(i, need);
            out.counted.insert(i, count(i, prev_file.as_ref()));
        }
        let sticky = chosen
            .get(k as usize - 1)
            .and_then(|d| here.iter().copied().find(|&i| cands[i].digest == *d));
        let pick = sticky.or_else(|| {
            here.iter()
                .copied()
                .find(|i| out.counted[i].len() >= out.need[i])
        });
        let Some(pick) = pick else {
            // Nothing complete at this number: proposals, or refused when no administrator of now signed them.
            for &i in &here {
                let c = &cands[i];
                let then = (cx.admins_at)(c.epoch);
                let status = if !out.counted[&i].is_empty() {
                    Status::Proposed
                } else if then.as_ref().is_some_and(|s| s.contains(&c.author))
                    && !cx.now_admins.contains(&c.author)
                {
                    Status::Refused {
                        reason: Refused::RemovedAdmin,
                    }
                } else {
                    Status::Refused {
                        reason: Refused::NotAdmin,
                    }
                };
                out.statuses.insert(i, status);
            }
            break;
        };
        if chosen.len() < k as usize {
            chosen.push(cands[pick].digest);
        }
        for &i in &here {
            if i != pick {
                out.statuses.insert(
                    i,
                    Status::Refused {
                        reason: Refused::Lost,
                    },
                );
            }
        }
        out.chain.push(pick);
        prev_file = cands[pick].file.clone();
        prev_digest = Some(cands[pick].digest);
    }
    for (n, &i) in out.chain.iter().enumerate() {
        let status = if n + 1 == out.chain.len() {
            Status::Applied
        } else {
            Status::Superseded
        };
        out.statuses.insert(i, status);
    }
    let held = out.chain.len() as u64;
    for (i, c) in cands.iter().enumerate() {
        if out.statuses.contains_key(&i) {
            continue;
        }
        let reason = c.refused.clone().unwrap_or(if c.version.number <= held {
            Refused::Behind
        } else {
            Refused::BadChain
        });
        out.statuses.insert(i, Status::Refused { reason });
    }
    out
}

/// A version as the rules judged it ([`judge`]).
#[cfg(any(test, feature = "fuzzing"))]
pub(crate) struct Judged {
    pub version: Version,
    pub file: Option<TeamFile>,
    pub digest: [u8; 32],
    pub status: Status,
    /// The members whose signature counts.
    pub counted: BTreeSet<MemberId>,
    pub need: usize,
}

/// The rules of the versions without a team, for the fuzzing (`fuzzing/settings.rs`): the entries as a device opened
/// them (their number, epoch, signer and what they carry), the administrators now and at each epoch, and the chain
/// this device chose before (kept, and grown). Gives each version judged, the chain's indices, and the quorum.
#[cfg(any(test, feature = "fuzzing"))]
pub(crate) fn judge(
    seen: Vec<(u64, u64, MemberId, Entry)>,
    now_admins: BTreeSet<MemberId>,
    admins_at: &dyn Fn(u64) -> Option<BTreeSet<MemberId>>,
    chosen: &mut Vec<[u8; 32]>,
    classify: &dyn Classify,
) -> (Vec<Judged>, Vec<usize>, bool) {
    let state = State {
        seen: seen
            .into_iter()
            .map(|(seq, epoch, member, entry)| Seen {
                seq,
                epoch,
                member,
                device: DeviceId([0; 16]),
                entry,
            })
            .collect(),
        ..Default::default()
    };
    let cx = Context {
        now_admins,
        admins_at,
        classify,
    };
    let (cands, cosigns, _) = candidates(&state, classify);
    let ev = evaluate(&cands, &cosigns, chosen, &cx);
    let judged = cands
        .into_iter()
        .enumerate()
        .map(|(i, c)| Judged {
            status: ev.statuses[&i].clone(),
            counted: ev.counted.get(&i).cloned().unwrap_or_default(),
            need: ev.need.get(&i).copied().unwrap_or(1),
            version: c.version,
            file: c.file,
            digest: c.digest,
        })
        .collect();
    (judged, ev.chain, ev.quorum)
}

// === Building what the window shows ===

type Cosigns = BTreeMap<[u8; 32], Vec<(MemberId, u64)>>;

fn candidates(state: &State, classify: &dyn Classify) -> (Vec<Candidate>, Cosigns, Vec<Refusal>) {
    let mut cands: Vec<Candidate> = Vec::new();
    let mut cosigns: Cosigns = BTreeMap::new();
    let mut refusals = Vec::new();
    for s in &state.seen {
        match &s.entry {
            Entry::Version { body } => {
                let d = digest(body);
                if cands.iter().any(|c| c.digest == d) {
                    refusals.push(Refusal {
                        seq: s.seq,
                        reason: Refused::Replayed,
                    });
                    continue;
                }
                let version: Version = match wire::decode(body) {
                    Ok(v) => v,
                    Err(why) => {
                        refusals.push(Refusal {
                            seq: s.seq,
                            reason: Refused::Unreadable { why },
                        });
                        continue;
                    }
                };
                let (file, refused) = match serde_json::from_str::<TeamFile>(&version.file) {
                    Ok(f) if f.version != FORMAT => {
                        let why = format!("format {} (this client reads {FORMAT})", f.version);
                        (Some(f), Some(Refused::Unreadable { why }))
                    }
                    Ok(f) => {
                        let hit = never(&f, classify).map(|hit| Refused::Never { hit });
                        (Some(f), hit)
                    }
                    Err(e) => (None, Some(Refused::Unreadable { why: e.to_string() })),
                };
                cands.push(Candidate {
                    digest: d,
                    version,
                    file,
                    author: s.member,
                    epoch: s.epoch,
                    refused,
                });
            }
            Entry::Cosign { digest, .. } => {
                let list = cosigns.entry(*digest).or_default();
                if !list.iter().any(|(m, _)| *m == s.member) {
                    list.push((s.member, s.epoch));
                }
            }
        }
    }
    (cands, cosigns, refusals)
}

fn effective(
    file: &TeamFile,
    consents: &BTreeMap<String, String>,
    classify: &dyn Classify,
) -> Effective {
    let mut out = Effective {
        reasons: file.reasons.clone(),
        ..Default::default()
    };
    for (part, key, value) in file.parts() {
        let Some(sort) = classify.sort(key) else {
            out.unknown.push(format!("{}.{key}", part.name()));
            continue;
        };
        let print = fingerprint(part, key, value);
        let accepted = consents.get(&consent_name(part, key)) == Some(&print);
        let asks = part == Part::Proposed || classify.asks(key, value);
        if asks && !accepted {
            out.pending.push(Pending {
                part,
                key: key.clone(),
                value: value.clone(),
                sort,
                fingerprint: print,
            });
            continue;
        }
        let into = match part {
            Part::Defaults => &mut out.defaults,
            Part::Enforced => &mut out.enforced,
            Part::Proposed => &mut out.accepted,
        };
        into.insert(key.clone(), value.clone());
    }
    out
}

fn build(team: &Team, state: &mut State, classify: &dyn Classify) -> Result<View, Error> {
    let Ok(roster) = team.roster() else {
        return Ok(View::default());
    };
    let now_admins: BTreeSet<MemberId> = roster
        .members
        .iter()
        .filter(|(_, r)| **r == escouade_team_crypto::Role::Admin)
        .map(|(m, _)| *m)
        .collect();
    let me = team.identity().member;
    let admins_at = |e: u64| team.admins_at(e).cloned();
    let cx = Context {
        now_admins: now_admins.clone(),
        admins_at: &admins_at,
        classify,
    };
    let (cands, cosigns, mut refusals) = candidates(state, classify);
    let ev = evaluate(&cands, &cosigns, &mut state.chosen, &cx);
    refusals.extend(state.refusals.iter().cloned());
    refusals.sort_by_key(|r| r.seq);
    let mut versions: Vec<VersionView> = cands
        .iter()
        .enumerate()
        .map(|(i, c)| VersionView {
            number: c.version.number,
            digest: hex::encode(c.digest),
            by: c.version.by.clone(),
            author: c.author.to_string(),
            at: c.version.at,
            note: c.version.note.clone(),
            file: c.file.clone().unwrap_or_default(),
            signers: ev
                .counted
                .get(&i)
                .map(|s| s.iter().map(|m| m.to_string()).collect())
                .unwrap_or_default(),
            need: ev.need.get(&i).copied().unwrap_or(1),
            signed_by_me: c.author == me
                || cosigns
                    .get(&c.digest)
                    .is_some_and(|l| l.iter().any(|(m, _)| *m == me)),
            status: ev.statuses[&i].clone(),
        })
        .collect();
    versions.sort_by_key(|v| std::cmp::Reverse(v.number));
    let applied = ev.chain.last().map(|&i| &cands[i]);
    Ok(View {
        applied: applied.map(|c| c.version.number),
        effective: applied
            .and_then(|c| c.file.as_ref())
            .map(|f| effective(f, &state.consents, classify))
            .unwrap_or_default(),
        versions,
        refusals,
        alarm: state.alarm,
        quorum: ev.quorum,
        admins: now_admins.len(),
    })
}

// === The stream ===

enum Opened {
    Refused(Refused),
    Fork,
    /// An epoch this device has not reached yet: read again later.
    Later,
}

fn open(team: &Team, e: &wire::Entry) -> Result<Seen, Opened> {
    let unreadable = |why: String| Opened::Refused(Refused::Unreadable { why });
    let sealed: SealedObject = wire::decode(&e.payload).map_err(unreadable)?;
    if sealed.header.project != SETTINGS_STREAM {
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
    let entry: Entry = wire::decode(&plain).map_err(unreadable)?;
    Ok(Seen {
        seq: e.seq,
        epoch: sealed.header.epoch,
        member: who.member,
        device: who.device,
        entry,
    })
}

fn entry_digest(e: &wire::Entry) -> [u8; 32] {
    Sha256::digest(wire::encode(e)).into()
}

async fn read(team: &Team, state: &mut State) -> Result<(), Error> {
    let client = team.client().clone();
    if let Some((seq, d)) = state.last {
        let page = client
            .read_project(&SETTINGS_STREAM.0, seq - 1, Some(1))
            .await?;
        match page.entries.first() {
            Some(e) if e.seq == seq => {
                if entry_digest(e) != d {
                    state.alarm = Some(Alarm::Rewritten);
                    return Ok(());
                }
            }
            // The very entry removed by the server's operator (SR2-02): a hole, the reading goes on after it.
            _ if client
                .was_dropped(Some(&SETTINGS_STREAM.0), seq, &d)
                .await? =>
            {
                state.last = None
            }
            _ => {
                state.alarm = Some(Alarm::Rollback);
                return Ok(());
            }
        }
    }
    loop {
        let page = client
            .read_project(&SETTINGS_STREAM.0, state.cursor, None)
            .await?;
        for e in &page.entries {
            if e.seq <= state.cursor {
                state.alarm = Some(Alarm::Server);
                return Ok(());
            }
            match open(team, e) {
                Ok(seen) => state.seen.push(seen),
                Err(Opened::Refused(reason)) => state.refusals.push(Refusal { seq: e.seq, reason }),
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

/// Reads what is new in the stream of the settings, and says what applies on this device. Under an alarm, nothing is
/// read until [`retry`].
pub async fn sync(team: &mut Team, classify: &dyn Classify) -> Result<View, Error> {
    let mut state = load(team)?;
    if state.alarm.is_none() {
        read(team, &mut state).await?;
    }
    let view = build(team, &mut state, classify)?;
    save(team, &state)?;
    Ok(view)
}

/// What applies, from what this device read, without reading the server.
pub fn view(team: &Team, classify: &dyn Classify) -> Result<View, Error> {
    let mut state = load(team)?;
    let view = build(team, &mut state, classify)?;
    save(team, &state)?;
    Ok(view)
}

/// « Réessayer »: the alarm lifted, the stream read again from where it was (a server that still lies says it again).
pub async fn retry(team: &mut Team, classify: &dyn Classify) -> Result<View, Error> {
    let mut state = load(team)?;
    state.alarm = None;
    save(team, &state)?;
    sync(team, classify).await
}

async fn append(team: &Team, entry: &Entry, number: u64, d: &[u8; 32]) -> Result<u64, Error> {
    let mut object = [0u8; 16];
    object.copy_from_slice(&d[..16]);
    let sealed = team.device().seal(
        SETTINGS_STREAM,
        ObjectId(object),
        number,
        &wire::encode(entry),
    )?;
    Ok(team
        .client()
        .append_project(&SETTINGS_STREAM.0, &wire::encode(&sealed))
        .await?)
}

fn admin(team: &Team) -> Result<(), Error> {
    if team.is_admin()? {
        Ok(())
    } else {
        Err(CryptoError::NotAdmin.into())
    }
}

/// The file's line of what never enters, said.
pub fn never_text(hit: &NeverHit) -> String {
    let what = match hit.rule {
        Never::Bypass => "bypassPermissions is never imposed nor allowed by a team",
        Never::CommandWithoutConsent => {
            "a command never runs without the consent of each device: it goes in `proposed`, never marked accepted"
        }
        Never::PluginWithoutGesture => "a plugin is never installed without the gesture of the member",
        Never::FirstParty => "a team never signs a dev.escouade.* plugin",
        Never::MadePublic => "the file never makes a project public",
    };
    format!("{}.{}: {what}", hit.part.name(), hit.key)
}

/// An administrator proposes a version: signed by this device, chained to the version in force. It applies at once
/// when it needs one signature; under the quorum it waits for a second administrator ([`cosign`]).
pub async fn propose(
    team: &mut Team,
    classify: &dyn Classify,
    mut file: TeamFile,
    by: &str,
    note: &str,
    at: u64,
) -> Result<View, Error> {
    admin(team)?;
    file.version = FORMAT;
    if let Some(hit) = never(&file, classify) {
        return Err(Error::Refused(never_text(&hit)));
    }
    let view = sync(team, classify).await?;
    if view.alarm.is_some() {
        return Err(Error::Refused(
            "the server shows what cannot be on the settings of the team: nothing is sent".into(),
        ));
    }
    let state = load(team)?;
    let version = Version {
        number: state.chosen.len() as u64 + 1,
        previous: state.chosen.last().copied(),
        file: serde_json::to_string(&file).map_err(|e| Error::Encoding(e.to_string()))?,
        by: by.trim().to_owned(),
        at,
        note: note.trim().to_owned(),
    };
    let body = wire::encode(&version);
    let d = digest(&body);
    append(team, &Entry::Version { body }, version.number, &d).await?;
    sync(team, classify).await
}

/// An administrator signs the version of another (`digest`, hex): the second signature of the quorum.
pub async fn cosign(
    team: &mut Team,
    classify: &dyn Classify,
    digest_hex: &str,
) -> Result<View, Error> {
    admin(team)?;
    let view = sync(team, classify).await?;
    let Some(v) = view.versions.iter().find(|v| v.digest == digest_hex) else {
        return Err(Error::Refused("no such version of the settings".into()));
    };
    if let Status::Refused { reason } = &v.status {
        return Err(Error::Refused(format!(
            "this version is refused: {reason:?}"
        )));
    }
    if v.signed_by_me {
        return Err(Error::Refused("you signed this version already".into()));
    }
    let mut d = [0u8; 32];
    hex::decode_to_slice(digest_hex, &mut d).map_err(|e| Error::Encoding(e.to_string()))?;
    let number = v.number;
    append(team, &Entry::Cosign { number, digest: d }, number, &d).await?;
    sync(team, classify).await
}

/// This device accepts a sensitive key, or a proposal, of the version in force: at its fingerprint, which must still
/// be the one the member read.
pub fn consent(
    team: &Team,
    classify: &dyn Classify,
    part: Part,
    key: &str,
    print: &str,
) -> Result<View, Error> {
    let mut state = load(team)?;
    let now = build(team, &mut state, classify)?;
    let waiting = now
        .effective
        .pending
        .iter()
        .any(|p| p.part == part && p.key == key && p.fingerprint == print);
    if !waiting {
        return Err(Error::Refused(
            "this setting changed since, or waits for nothing: read it again".into(),
        ));
    }
    state
        .consents
        .insert(consent_name(part, key), print.to_owned());
    let view = build(team, &mut state, classify)?;
    save(team, &state)?;
    Ok(view)
}

/// This device takes its consent back: the key waits again.
pub fn withdraw(
    team: &Team,
    classify: &dyn Classify,
    part: Part,
    key: &str,
) -> Result<View, Error> {
    let mut state = load(team)?;
    state.consents.remove(&consent_name(part, key));
    let view = build(team, &mut state, classify)?;
    save(team, &state)?;
    Ok(view)
}

/// The device of an administrator cosigns the versions of the chain whose signatures no longer count (their signer
/// left, or is no longer an administrator): a device that comes later reads the whole chain. The number cosigned.
pub async fn serve(team: &mut Team, classify: &dyn Classify) -> Result<usize, Error> {
    if !team.is_admin().unwrap_or(false) {
        return Ok(0);
    }
    let view = view(team, classify)?;
    if view.alarm.is_some() {
        return Ok(0);
    }
    let mut done = 0;
    for v in &view.versions {
        let in_chain = matches!(v.status, Status::Applied | Status::Superseded);
        if in_chain && !v.signed_by_me && v.signers.len() < v.need.max(1) {
            let mut d = [0u8; 32];
            hex::decode_to_slice(&v.digest, &mut d).map_err(|e| Error::Encoding(e.to_string()))?;
            let number = v.number;
            append(team, &Entry::Cosign { number, digest: d }, number, &d).await?;
            done += 1;
        }
    }
    if done > 0 {
        sync(team, classify).await?;
    }
    Ok(done)
}
