//! The base, behind `Store`: SQLite in WAL by default (rusqlite), PostgreSQL as an option (tokio-postgres).
//!
//! **Why not sqlx** (`docs/TEAM-SERVER.md` §14 proposed it for both): `libsqlite3-sys` declares `links = "sqlite3"`,
//! so a workspace holds one version of it. The app's lock holds 0.39.1 (rusqlite 0.41); sqlx 0.8.6 asks for 0.30.1
//! and sqlx 0.9.0 for `>=0.30.1, <0.38`. The server therefore takes rusqlite at the app's version for SQLite, and
//! tokio-postgres for PostgreSQL, both behind this trait.
//!
//! What it keeps is what `docs/TEAM-SERVER.md` §3.8 lets the server see, and nothing more: opaque ids, public keys,
//! roles, sizes, hours, which device wrote, the encrypted bytes. Tokens are kept as their SHA-256.

pub mod postgres;
pub mod sqlite;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::util::Id;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Admin,
    Member,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Member => "member",
        }
    }

    pub fn parse(s: &str) -> Role {
        if s == "admin" {
            Role::Admin
        } else {
            Role::Member
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    pub id: Id,
    pub team: Id,
    pub public_key: [u8; 32],
    pub role: Role,
    pub created_at: i64,
    pub revoked_at: Option<i64>,
}

/// What a stream entry is, for the server: an MLS commit (ordered by epoch, the first one wins: the team's group in the
/// team's stream, a project's own group in its stream) or anything else.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Commit,
    Message,
}

impl Kind {
    pub fn code(self) -> i64 {
        match self {
            Kind::Commit => 1,
            Kind::Message => 0,
        }
    }

    pub fn from_code(c: i64) -> Kind {
        if c == 1 {
            Kind::Commit
        } else {
            Kind::Message
        }
    }
}

/// One entry of a stream, as kept and as served.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub seq: u64,
    pub kind: Kind,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub epoch: Option<u64>,
    #[serde(with = "serde_bytes")]
    pub device: Vec<u8>,
    pub at: i64,
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
}

/// A project's snapshot (TS-5): the encrypted state of the project at `upto`, made by a device of the team.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub upto: u64,
    pub device: Id,
    pub at: i64,
    pub payload: Vec<u8>,
}

/// Where an entry goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stream {
    Team,
    Project(Id),
}

#[derive(Clone, Debug)]
pub struct NewEntry {
    pub kind: Kind,
    pub epoch: Option<u64>,
    pub device: Id,
    pub at: i64,
    pub payload: Vec<u8>,
    /// A write of a ticket's holder (TS-6): refused when its claim token is no longer the current one.
    pub fence: Option<Fence>,
}

/// Who reads and writes a project on the server (TS-6, `docs/TEAM-SERVER.md` §2.6): every device of the team, or
/// those listed. A project the server never heard of is open to every device of the team until its first write,
/// which makes it so (an administrator restricts it).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Access {
    pub everyone: bool,
    pub devices: Vec<Id>,
}

impl Access {
    pub fn admits(&self, device: &Id) -> bool {
        self.everyone || self.devices.contains(device)
    }

    pub(crate) fn devices_bytes(&self) -> Vec<u8> {
        self.devices.concat()
    }

    pub(crate) fn from_parts(everyone: bool, bytes: &[u8]) -> Access {
        Access {
            everyone,
            devices: bytes.as_chunks::<16>().0.to_vec(),
        }
    }
}

/// The claim of a ticket (TS-6, §4.2), by its opaque id: which device holds it, until when, under which token. The
/// server knows nothing else of the ticket: neither its title, nor its key, nor its column.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    #[serde(with = "serde_bytes")]
    pub ticket: Vec<u8>,
    /// The device that holds it; still there once its lease is over, until another one takes it (« interrompu »).
    #[serde(default, with = "opt_id", skip_serializing_if = "Option::is_none")]
    pub holder: Option<Id>,
    /// The end of the lease (seconds); 0 when it was given back.
    pub until: i64,
    /// Grows at each claim: a write of the holder carries it, and one under an older token is refused.
    pub token: u64,
    /// Grows at each claim, release and write of the holder: what a claim expects, to take a ticket as it saw it.
    pub version: u64,
    pub claimed_at: i64,
    /// The holder's last renewal: its device no longer answers when it is long past.
    pub renewed_at: i64,
    /// The device that held it last, after a release or a take-over.
    #[serde(default, with = "opt_id", skip_serializing_if = "Option::is_none")]
    pub last_holder: Option<Id>,
    /// The device that handed it to its holder (TS-17, §11), while the holder has not taken it yet: it may take it back.
    #[serde(default, with = "opt_id", skip_serializing_if = "Option::is_none")]
    pub handed_by: Option<Id>,
}

mod opt_id {
    pub fn serialize<S: serde::Serializer>(v: &Option<[u8; 16]>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(b) => s.serialize_bytes(b),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        d: D,
    ) -> Result<Option<[u8; 16]>, D::Error> {
        let v: Option<serde_bytes::ByteBuf> = serde::Deserialize::deserialize(d)?;
        v.map(|b| {
            b.into_vec()
                .try_into()
                .map_err(|_| serde::de::Error::custom("an id is 16 bytes"))
        })
        .transpose()
    }
}

impl Claim {
    pub fn free(ticket: Id) -> Claim {
        Claim {
            ticket: ticket.to_vec(),
            holder: None,
            until: 0,
            token: 0,
            version: 0,
            claimed_at: 0,
            renewed_at: 0,
            last_holder: None,
            handed_by: None,
        }
    }
}

/// A holder's write: the ticket's claim and the token it holds it under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fence {
    pub ticket: Id,
    pub token: u64,
}

/// What a device asks of a ticket's claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimAsk {
    /// Take it (free, its lease over, its holder gone, or already this device's: a new token), as it was at
    /// `expected` (its version) when given.
    Take { expected: Option<u64> },
    /// Keep it a lease longer, under its token (also once its lease is over, while nobody took it).
    Renew { token: u64 },
    /// Give it back.
    Release { token: u64 },
    /// Hand it to another device of the project (TS-17, §11), under its token: that device holds it at once, under a new
    /// token, for a longer lease, until it takes it (a new token again) or gives it back.
    Hand { token: u64, to: Id },
    /// Take back a ticket this device handed, under the token of the hand, while its new holder has not taken it.
    Withdraw { token: u64 },
}

/// The most claims a project keeps (one per ticket ever taken).
pub const MAX_CLAIMS: i64 = 100_000;

/// What a claim becomes (the same for both bases): `holder_ok` says its holder is still a device of the project (not
/// revoked, still with access); a holder gone holds nothing.
#[cfg(test)]
pub(crate) fn decide(
    current: Option<Claim>,
    ticket: Id,
    device: Id,
    ask: ClaimAsk,
    now: i64,
    lease: i64,
    holder_ok: bool,
) -> Result<Claim> {
    decide_with(current, ticket, device, ask, now, lease, holder_ok, true)
}

/// [`decide`], `to_ok` saying whether the device a hand goes to is one of the project (TS-17).
#[allow(clippy::too_many_arguments)]
pub(crate) fn decide_with(
    current: Option<Claim>,
    ticket: Id,
    device: Id,
    ask: ClaimAsk,
    now: i64,
    lease: i64,
    holder_ok: bool,
    to_ok: bool,
) -> Result<Claim> {
    let c = current.unwrap_or_else(|| Claim::free(ticket));
    let held_by_other = c.holder.is_some_and(|h| h != device) && c.until > now && holder_ok;
    match ask {
        ClaimAsk::Take { expected } => {
            if held_by_other {
                return Err(StoreError::Taken(c));
            }
            if expected.is_some_and(|v| v != c.version) {
                return Err(StoreError::StaleVersion(c));
            }
            Ok(Claim {
                holder: Some(device),
                until: now + lease,
                token: c.token + 1,
                version: c.version + 1,
                claimed_at: now,
                renewed_at: now,
                last_holder: c.holder.filter(|h| *h != device).or(c.last_holder),
                handed_by: None,
                ..c
            })
        }
        ClaimAsk::Renew { token } => {
            if c.holder != Some(device) || c.token != token {
                return Err(StoreError::ClaimLost(c));
            }
            Ok(Claim {
                until: now + lease,
                renewed_at: now,
                ..c
            })
        }
        ClaimAsk::Release { token } => {
            if c.holder != Some(device) || c.token != token {
                return Err(StoreError::ClaimLost(c));
            }
            Ok(Claim {
                holder: None,
                until: 0,
                version: c.version + 1,
                renewed_at: now,
                last_holder: Some(device),
                handed_by: None,
                ..c
            })
        }
        ClaimAsk::Hand { token, to } => {
            if c.holder != Some(device) || c.token != token {
                return Err(StoreError::ClaimLost(c));
            }
            if to == device || !to_ok {
                return Err(StoreError::NotInProject(c));
            }
            Ok(Claim {
                holder: Some(to),
                until: now + lease,
                token: c.token + 1,
                version: c.version + 1,
                claimed_at: now,
                renewed_at: now,
                last_holder: Some(device),
                handed_by: Some(device),
                ..c
            })
        }
        ClaimAsk::Withdraw { token } => {
            if c.handed_by != Some(device) || c.token != token || c.holder.is_none() {
                return Err(StoreError::ClaimLost(c));
            }
            Ok(Claim {
                holder: Some(device),
                until: now + lease,
                token: c.token + 1,
                version: c.version + 1,
                claimed_at: now,
                renewed_at: now,
                last_holder: c.holder,
                handed_by: None,
                ..c
            })
        }
    }
}

/// Whether a holder's write passes: its token is the claim's current one, and it is its holder's. It moves the
/// claim's version on.
pub(crate) fn fenced(current: Option<Claim>, fence: &Fence, device: Id) -> Result<Claim> {
    let c = current.unwrap_or_else(|| Claim::free(fence.ticket));
    if c.holder != Some(device) || c.token != fence.token {
        return Err(StoreError::StaleToken(c));
    }
    Ok(Claim {
        version: c.version + 1,
        ..c
    })
}

/// The bounds a write is checked against, in its transaction.
#[derive(Clone, Copy, Debug)]
pub struct Quotas {
    pub team_bytes: u64,
    pub devices: u32,
    pub projects: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    /// From `init`: creates a team, and its first device as administrator.
    Team,
    /// From an administrator: adds a device to the team (the entry point of TS-3's invitations).
    Device,
    /// From the operator (`escouade-team-server recovery-token`): adds a device to a team as an administrator of the
    /// server, for the holder of the recovery key the day every administrator lost their devices (TS-3, §3.5).
    Recovery,
}

impl TokenKind {
    pub fn code(self) -> i64 {
        match self {
            TokenKind::Team => 0,
            TokenKind::Device => 1,
            TokenKind::Recovery => 2,
        }
    }
}

/// A project's snapshot a device found unreadable (SR2-01): its messages are never pruned while it is flagged, and an
/// administrator is told (`unreadable_snapshots`), with the device that posted it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unreadable {
    pub project: Id,
    pub upto: u64,
    /// The device that posted it, and when.
    pub device: Id,
    pub at: i64,
    /// The first device that found it unreadable, and when.
    pub flagged_by: Id,
    pub flagged_at: i64,
}

/// A token used again after its first use (a leaked invitation link): what an administrator is told.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenAlert {
    pub hash: [u8; 32],
    /// When it was used first, and again last.
    pub used_at: i64,
    pub reused_at: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("this token is unknown, used or expired")]
    BadToken,
    /// The token served already: kept used until it expires, so its second user is told so (and its team's
    /// administrators too).
    #[error("this token was already used")]
    TokenUsed,
    #[error("a team keeps one administrator on the server at least")]
    LastAdmin,
    #[error("this key already has a device")]
    KeyTaken,
    #[error("a commit already holds epoch {asked}; the next is {expected}")]
    EpochTaken {
        asked: u64,
        expected: u64,
        /// The entries from the commit that won the epoch on, a page at most.
        preceding: Vec<Entry>,
    },
    #[error("the team's quota is reached")]
    Quota,
    /// A snapshot older than the one kept, or beyond the stream.
    #[error("this snapshot is refused: {0}")]
    Snapshot(&'static str),
    #[error("too many {0}")]
    TooMany(&'static str),
    #[error("it exists already")]
    Exists,
    /// The device has no access to the project (TS-6).
    #[error("no access to this project")]
    NoAccess,
    /// Only an administrator changes who has access to a project.
    #[error("an administrator's")]
    NotAdmin,
    /// Another device holds the ticket.
    #[error("the ticket is taken")]
    Taken(Claim),
    /// The ticket moved on since the version the claim expected.
    #[error("the ticket changed meanwhile")]
    StaleVersion(Claim),
    /// The claim is no longer this device's (renewed or given back).
    #[error("the claim was lost")]
    ClaimLost(Claim),
    /// A holder's write under a token that is no longer the current one.
    #[error("the claim token is stale")]
    StaleToken(Claim),
    /// A hand to a device that is not one of the project's (TS-17): revoked, without access, or the holder itself.
    #[error("that device is not one of this project")]
    NotInProject(Claim),
    #[error("the base: {0}")]
    Backend(String),
}

pub type Result<T> = std::result::Result<T, StoreError>;

/// Counts for the metrics, with no id.
#[derive(Clone, Copy, Debug, Default)]
pub struct Counts {
    pub teams: u64,
    pub devices: u64,
    pub revoked_devices: u64,
}

#[async_trait]
pub trait Store: Send + Sync {
    /// Creates the tables if needed.
    async fn migrate(&self) -> Result<()>;
    async fn ping(&self) -> Result<()>;

    async fn add_token(
        &self,
        hash: [u8; 32],
        kind: TokenKind,
        team: Option<Id>,
        expires_at: Option<i64>,
        now: i64,
    ) -> Result<()>;
    /// Uses a token once: a team token makes a team and its administrator, a device token a member of its team, a
    /// recovery token an administrator of its team. A used token stays, used, until its time is over (then a week):
    /// a second use is `TokenUsed`, and noted for the team's administrators (`token_alerts`).
    async fn enroll(
        &self,
        hash: [u8; 32],
        public_key: [u8; 32],
        now: i64,
        quotas: Quotas,
    ) -> Result<Device>;

    async fn device(&self, id: Id) -> Result<Option<Device>>;
    async fn devices(&self, team: Id) -> Result<Vec<Device>>;
    /// Revokes a device of `team`; false if it is not one.
    async fn revoke(&self, team: Id, device: Id, now: i64) -> Result<bool>;
    /// Sets the role of a device of `team` on the server; false if it is not one. The last administrator not revoked
    /// is never made a member (`LastAdmin`).
    async fn set_role(&self, team: Id, device: Id, role: Role) -> Result<bool>;
    /// The team's tokens used again after their first use.
    async fn token_alerts(&self, team: Id) -> Result<Vec<TokenAlert>>;
    /// Forgets the used tokens whose time is over.
    async fn sweep_tokens(&self, now: i64) -> Result<()>;

    /// Appends an entry and gives it its number. A commit must be for the next epoch.
    async fn append(
        &self,
        team: Id,
        stream: Stream,
        entry: NewEntry,
        quotas: Quotas,
        page: Page,
    ) -> Result<Entry>;
    /// The entries after `since`, a page at most, and whether more follow.
    async fn read(
        &self,
        team: Id,
        stream: Stream,
        since: u64,
        page: Page,
    ) -> Result<(Vec<Entry>, bool)>;

    /// Keeps a project's snapshot beside the ones before (SR2-01): nothing is forgotten when it comes. Its bytes count
    /// in the quota. Refused when it is not after the last one kept, or beyond the stream.
    async fn put_snapshot(
        &self,
        team: Id,
        project: Id,
        snapshot: Snapshot,
        quotas: Quotas,
    ) -> Result<()>;
    /// A project's last snapshot, or with `before` its last one before that number (a device that found the last one
    /// unreadable goes back to the one before).
    async fn snapshot(
        &self,
        team: Id,
        project: Id,
        before: Option<u64>,
    ) -> Result<Option<Snapshot>>;
    /// The number the last snapshot of a project holds up to, if any.
    async fn snapshot_upto(&self, team: Id, project: Id) -> Result<Option<u64>>;
    /// Notes that `device` found the snapshot of `project` at `upto` unreadable (the first device and time are kept).
    /// False when there is no such snapshot.
    async fn flag_snapshot(
        &self,
        team: Id,
        project: Id,
        upto: u64,
        device: Id,
        now: i64,
    ) -> Result<bool>;
    /// Removes the snapshot of `project` at `upto` (an administrator's): its bytes go back to the quota. False when
    /// there is no such snapshot; refused (`Snapshot`) for the one the stream was pruned to, its messages gone.
    async fn remove_snapshot(&self, team: Id, project: Id, upto: u64) -> Result<bool>;
    /// The team's snapshots a device found unreadable, for its administrators.
    async fn unreadable_snapshots(&self, team: Id) -> Result<Vec<Unreadable>>;
    /// The retention of SR2-01: for each project, its last snapshot older than `cut` (seconds since the epoch) that no
    /// device flagged takes the place of the messages it holds: they and the snapshots before it are forgotten, their
    /// bytes given back to the quota (the commits stay: a device follows the project's group by them). Gives how many
    /// messages were forgotten.
    async fn prune_snapshots(&self, cut: i64) -> Result<u64>;

    /// Reserves a file's place before its bytes are written.
    async fn reserve_file(
        &self,
        team: Id,
        file: Id,
        size: u64,
        device: Id,
        now: i64,
        quotas: Quotas,
    ) -> Result<()>;
    async fn file_size(&self, team: Id, file: Id) -> Result<Option<u64>>;
    /// Forgets a file and gives its place back; its size, if it was there.
    async fn delete_file(&self, team: Id, file: Id) -> Result<Option<u64>>;
    async fn used_bytes(&self, team: Id) -> Result<u64>;

    async fn counts(&self) -> Result<Counts>;

    /// Who has access to a project, if the server knows it (TS-6).
    async fn access(&self, team: Id, project: Id) -> Result<Option<Access>>;
    /// Sets who has access to a project: anyone of the team the first time, an administrator afterwards
    /// (`NotAdmin`).
    async fn set_access(&self, team: Id, project: Id, access: Access, admin: bool) -> Result<()>;
    /// Makes a project open to the whole team if the server never heard of it (its first write).
    async fn open_access(&self, team: Id, project: Id) -> Result<()>;
    /// Takes, renews or gives back a ticket's claim, in one transaction (§4.2).
    #[allow(clippy::too_many_arguments)]
    async fn claim(
        &self,
        team: Id,
        project: Id,
        ticket: Id,
        device: Id,
        ask: ClaimAsk,
        now: i64,
        lease: i64,
    ) -> Result<Claim>;
    /// A project's claims.
    async fn claims(&self, team: Id, project: Id) -> Result<Vec<Claim>>;
    /// The next number of a project's tickets (§4.1): 1, 2, 3…, one at a time, never twice.
    async fn next_key(&self, team: Id, project: Id) -> Result<u64>;
}

/// The bound of a page of entries.
#[derive(Clone, Copy, Debug)]
pub struct Page {
    pub entries: u32,
    pub bytes: usize,
}

/// Cuts `rows` (one more than asked was read, to know if more follow) to the page: always one entry, then as many as
/// fit in its bytes.
pub(crate) fn paginate(mut rows: Vec<Entry>, page: Page) -> (Vec<Entry>, bool) {
    let mut more = rows.len() > page.entries as usize;
    rows.truncate(page.entries as usize);
    let mut total = 0usize;
    let mut keep = 0usize;
    for (i, e) in rows.iter().enumerate() {
        total += e.payload.len();
        if i > 0 && total > page.bytes {
            break;
        }
        keep = i + 1;
    }
    if keep < rows.len() {
        more = true;
        rows.truncate(keep);
    }
    (rows, more)
}

pub async fn open(
    config: &crate::config::DatabaseConfig,
) -> std::result::Result<Box<dyn Store>, StoreError> {
    let store: Box<dyn Store> = match config {
        crate::config::DatabaseConfig::Sqlite(path) => Box::new(sqlite::Sqlite::open(path)?),
        crate::config::DatabaseConfig::Postgres { url, schema } => {
            Box::new(postgres::Postgres::connect(url, schema.as_deref()).await?)
        }
    };
    store.migrate().await?;
    Ok(store)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(seq: u64, size: usize) -> Entry {
        Entry {
            seq,
            kind: Kind::Message,
            epoch: None,
            device: vec![0; 16],
            at: 0,
            payload: vec![0; size],
        }
    }

    const T: Id = [1; 16];
    const A: Id = [2; 16];
    const B: Id = [3; 16];

    #[test]
    fn a_claim_passes_once_and_its_token_grows() {
        let take = ClaimAsk::Take { expected: None };
        let a = decide(None, T, A, take, 100, 600, true).unwrap();
        assert_eq!((a.holder, a.token, a.until), (Some(A), 1, 700));
        assert!(matches!(
            decide(Some(a.clone()), T, B, take, 200, 600, true),
            Err(StoreError::Taken(_))
        ));
        // Its lease over: B takes it, under a new token; A's write under the old one is refused.
        let b = decide(Some(a.clone()), T, B, take, 701, 600, true).unwrap();
        assert_eq!((b.holder, b.token, b.last_holder), (Some(B), 2, Some(A)));
        let f = Fence {
            ticket: T,
            token: 1,
        };
        assert!(matches!(
            fenced(Some(b.clone()), &f, A),
            Err(StoreError::StaleToken(_))
        ));
        assert!(matches!(
            decide(
                Some(b.clone()),
                T,
                A,
                ClaimAsk::Renew { token: 1 },
                702,
                600,
                true
            ),
            Err(StoreError::ClaimLost(_))
        ));
        let f = Fence {
            ticket: T,
            token: 2,
        };
        assert_eq!(
            fenced(Some(b.clone()), &f, B).unwrap().version,
            b.version + 1
        );
        // A holder gone (revoked, out of the project) holds nothing.
        assert!(decide(Some(b.clone()), T, A, take, 702, 600, false).is_ok());
    }

    #[test]
    fn a_claim_expects_the_version_it_saw_and_is_given_back() {
        let a = decide(
            None,
            T,
            A,
            ClaimAsk::Take { expected: Some(0) },
            0,
            600,
            true,
        )
        .unwrap();
        let back = decide(
            Some(a.clone()),
            T,
            A,
            ClaimAsk::Release { token: 1 },
            10,
            600,
            true,
        )
        .unwrap();
        assert_eq!(
            (back.holder, back.until, back.last_holder),
            (None, 0, Some(A))
        );
        assert!(matches!(
            decide(
                Some(back.clone()),
                T,
                B,
                ClaimAsk::Take {
                    expected: Some(a.version)
                },
                11,
                600,
                true
            ),
            Err(StoreError::StaleVersion(_))
        ));
        let take = ClaimAsk::Take {
            expected: Some(back.version),
        };
        let b = decide(Some(back.clone()), T, B, take, 11, 600, true).unwrap();
        assert_eq!(b.token, 2);
        let renewed = decide(Some(b), T, B, ClaimAsk::Renew { token: 2 }, 400, 600, true).unwrap();
        assert_eq!((renewed.until, renewed.renewed_at), (1000, 400));
    }

    /// TS-17: a hand gives the ticket to another device under a new token; it takes it (a new token again), or the one
    /// that handed takes it back while it did not. Nobody else does, and a hand to a device out of the project fails.
    #[test]
    fn a_hand_moves_the_claim_and_its_token_grows() {
        const C: Id = [4; 16];
        let a = decide(None, T, A, ClaimAsk::Take { expected: None }, 0, 600, true).unwrap();
        let hand = |to| ClaimAsk::Hand { token: 1, to };
        let out = decide_with(Some(a.clone()), T, A, hand(B), 10, 9000, true, false);
        assert!(matches!(out, Err(StoreError::NotInProject(_))));
        let out = decide_with(Some(a.clone()), T, A, hand(A), 10, 9000, true, true);
        assert!(matches!(out, Err(StoreError::NotInProject(_))));
        let out = decide_with(Some(a.clone()), T, C, hand(B), 10, 9000, true, true);
        assert!(matches!(out, Err(StoreError::ClaimLost(_))));
        let h = decide_with(Some(a.clone()), T, A, hand(B), 10, 9000, true, true).unwrap();
        assert_eq!(
            (h.holder, h.token, h.until, h.handed_by, h.last_holder),
            (Some(B), 2, 9010, Some(A), Some(A))
        );
        // A's old token writes nothing more; a third device does not take it, nor take it back.
        let f = Fence {
            ticket: T,
            token: 1,
        };
        assert!(matches!(
            fenced(Some(h.clone()), &f, A),
            Err(StoreError::StaleToken(_))
        ));
        let take = ClaimAsk::Take { expected: None };
        assert!(matches!(
            decide(Some(h.clone()), T, C, take, 20, 600, true),
            Err(StoreError::Taken(_))
        ));
        let back = ClaimAsk::Withdraw { token: 2 };
        assert!(matches!(
            decide(Some(h.clone()), T, C, back, 20, 600, true),
            Err(StoreError::ClaimLost(_))
        ));
        // The abandon: A takes it back, under a new token.
        let w = decide(Some(h.clone()), T, A, back, 20, 600, true).unwrap();
        assert_eq!((w.holder, w.token, w.handed_by), (Some(A), 3, None));
        // The take: B takes it, under a new token; A no longer takes it back.
        let b = decide(Some(h.clone()), T, B, take, 20, 600, true).unwrap();
        assert_eq!((b.holder, b.token, b.handed_by), (Some(B), 3, None));
        assert!(matches!(
            decide(Some(b), T, A, back, 30, 600, true),
            Err(StoreError::ClaimLost(_))
        ));
    }

    #[test]
    fn a_page_holds_one_entry_at_least_and_says_when_more_follow() {
        let page = Page {
            entries: 2,
            bytes: 10,
        };
        let (rows, more) = paginate(vec![entry(1, 4), entry(2, 4), entry(3, 4)], page);
        assert_eq!((rows.len(), more), (2, true));
        let (rows, more) = paginate(vec![entry(1, 40), entry(2, 4)], page);
        assert_eq!((rows.len(), more), (1, true));
        let (rows, more) = paginate(vec![entry(1, 4)], page);
        assert_eq!((rows.len(), more), (1, false));
    }
}
