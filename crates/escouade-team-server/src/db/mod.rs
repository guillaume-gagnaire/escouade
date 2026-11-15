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

/// What a stream entry is, for the server: an MLS commit (ordered by epoch, the first one wins) or anything else.
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
    #[error("too many {0}")]
    TooMany(&'static str),
    #[error("it exists already")]
    Exists,
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
