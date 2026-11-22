//! The local copy of a team's project (TS-5, `docs/TEAM-SERVER.md` §2.4 and §2.5): SQLite, in the data folder, one
//! file per project of each team (`<teams>/<team>/projects/<project>.sqlite3`). The interface reads the copy, never
//! the server; everything reads offline, and everything but taking a ticket writes offline.
//!
//! - **The journal replayed**: each operation read from the project's stream (opened, its chain and its signature
//!   checked by `team.rs`) is kept in `ops` and applied to the state; [`Replica::rebuild`] replays the state from the
//!   last snapshot and the journal.
//! - **The fusion, field by field** (C2-A): every write of a field is kept in `writes` with its stamp (`Hlc`, whose
//!   bytes sort as the stamps); the field's value is the write of the highest stamp: the last writer wins, whatever
//!   order the copies applied them in. Two writes from the same base (the value their authors saw) are concurrent:
//!   the one that lost is kept, as a conflict to show (« Bob a changé le titre en même temps · Reprendre le mien »).
//! - **A deletion is final**: an object deleted stays deleted, whatever writes its fields afterwards.
//! - **A document (a spec page) goes in whole versions**: the heads (the versions no other names as its base) are
//!   the current one (the highest stamp) and the conflicts, kept for the three-way merge on the device (TS-9).
//! - **The outbox** (§2.5): what this device wrote, applied to its copy at once, waits in `outbox` in its order, and
//!   leaves when the server is reached again.
//!
//! The copy is in the clear on the disk, as the local Kanban of `.escouade/` is: it is what the interface reads. The
//! state of the team (keys, MLS) stays in its encrypted storage (`storage.rs`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use escouade_team_crypto::hlc::{Clock, Hlc};
use escouade_team_crypto::ops::{Chain, OpBody, Operation, Scope, Value};
use escouade_team_crypto::{DeviceId, ObjectId};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::Error;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS meta (k TEXT PRIMARY KEY, v TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS base (id INTEGER PRIMARY KEY CHECK (id = 1), state BLOB NOT NULL);
CREATE TABLE IF NOT EXISTS ops (
    digest BLOB PRIMARY KEY,
    seq INTEGER,
    author BLOB NOT NULL,
    dseq INTEGER NOT NULL,
    op BLOB NOT NULL
);
CREATE TABLE IF NOT EXISTS objects (
    id BLOB PRIMARY KEY,
    kind TEXT,
    created BLOB,
    deleted BLOB
);
CREATE TABLE IF NOT EXISTS writes (
    object BLOB NOT NULL,
    field TEXT NOT NULL,
    stamp BLOB NOT NULL,
    base BLOB,
    value TEXT NOT NULL,
    PRIMARY KEY (object, field, stamp)
);
CREATE TABLE IF NOT EXISTS versions (
    object BLOB NOT NULL,
    id BLOB NOT NULL,
    bases TEXT NOT NULL,
    stamp BLOB NOT NULL,
    content TEXT NOT NULL,
    PRIMARY KEY (object, id)
);
CREATE TABLE IF NOT EXISTS counts (
    digest BLOB PRIMARY KEY,
    counter TEXT NOT NULL,
    object BLOB NOT NULL,
    at INTEGER
);
CREATE TABLE IF NOT EXISTS numbers (
    counter TEXT NOT NULL,
    object BLOB NOT NULL,
    number INTEGER NOT NULL,
    PRIMARY KEY (counter, object),
    UNIQUE (counter, number)
);
CREATE TABLE IF NOT EXISTS outbox (
    n INTEGER PRIMARY KEY AUTOINCREMENT,
    digest BLOB NOT NULL,
    op BLOB NOT NULL,
    fence BLOB
);
";

fn db(e: rusqlite::Error) -> Error {
    Error::Storage(format!("the project's copy: {e}"))
}

/// What the copy keeps of the project's stream and of this device's place in it.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectMeta {
    /// Whose key seals the project: the team's, or its own group's. Known when the project is made, joined, or its
    /// first operation read.
    pub scope: Option<Scope>,
    /// The last number of the stream read.
    pub cursor: u64,
    /// The last entry read, its number and digest: read again before reading on.
    pub last: Option<(u64, [u8; 32])>,
    /// Each device's chain as read: its last `dseq` and that operation's digest.
    pub chains: BTreeMap<DeviceId, Chain>,
    /// This device's last operation, sent or not.
    pub own: Option<Chain>,
    /// This device's operations the server took, by their number, until they are read back: one the server drops
    /// in silence shows.
    pub sent: BTreeMap<u64, Chain>,
    /// The number the snapshot this copy holds goes up to (loaded, made here, or checked once this device was past it).
    pub snapshot: Option<u64>,
    /// The snapshots of the stream this device found unreadable and put aside (SR2-01), by their number.
    pub discarded: BTreeSet<u64>,
    pub clock: Clock,
}

/// A write of a ticket's holder: the claim's opaque id and the token it holds it under (TS-6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fence {
    pub claim: [u8; 16],
    pub token: u64,
}

impl Fence {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = self.claim.to_vec();
        out.extend_from_slice(&self.token.to_be_bytes());
        out
    }

    pub fn from_bytes(b: &[u8]) -> Option<Fence> {
        if b.len() != 24 {
            return None;
        }
        Some(Fence {
            claim: b[..16].try_into().ok()?,
            token: u64::from_be_bytes(b[16..].try_into().ok()?),
        })
    }
}

/// An object as the interface reads it: its fields, each the last write.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ObjectView {
    pub id: ObjectId,
    pub kind: String,
    pub fields: BTreeMap<String, Value>,
}

/// A write that lost to a concurrent one: kept to show, and to take back.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Conflict {
    pub field: String,
    /// The value that lost, and whose it was.
    pub value: Value,
    pub stamp: Hlc,
    /// The value that won.
    pub winner: Value,
    pub winner_stamp: Hlc,
    /// The value both writes were made from (their base), when the copy has it: the three-way merge's base.
    pub base: Option<Value>,
}

/// A version of a document.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Version {
    pub id: ObjectId,
    pub bases: Vec<ObjectId>,
    pub stamp: Hlc,
    pub content: Value,
}

/// The versions of a document that count: the current one, and the others no version merged (the conflicts).
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Versions {
    pub current: Option<Version>,
    pub conflicts: Vec<Version>,
}

/// A row of `objects`: id, kind, created, deleted.
pub type ObjectRow = (Vec<u8>, Option<String>, Option<Vec<u8>>, Option<Vec<u8>>);
/// A row of `writes`: object, field, stamp, base, value.
pub type WriteRow = (Vec<u8>, String, Vec<u8>, Option<Vec<u8>>, String);
/// A row of `versions`: object, id, bases, stamp, content.
pub type VersionRow = (Vec<u8>, Vec<u8>, String, Vec<u8>, String);
/// A row of `numbers`: counter, object, number.
pub type NumberRow = (String, Vec<u8>, i64);
/// An operation of the outbox: its place, its digest, its bytes.
pub type Waiting = (i64, [u8; 32], Vec<u8>);
/// An operation of the outbox taken back: its bytes, its fence.
pub type Unsent = (Vec<u8>, Option<Fence>);
/// A row of the outbox read whole: digest, bytes, fence.
type OutboxRow = (Vec<u8>, Vec<u8>, Option<Vec<u8>>);

/// The state, as a snapshot holds it and as two copies compare: every row of the state, in order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub objects: Vec<ObjectRow>,
    pub writes: Vec<WriteRow>,
    pub versions: Vec<VersionRow>,
    /// The numbers given (TS-9): a snapshot keeps them, as the operations that gave them leave the journal.
    #[serde(default)]
    pub numbers: Vec<NumberRow>,
}

fn stamp_of(bytes: &[u8]) -> Result<Hlc, Error> {
    Hlc::from_bytes(bytes).ok_or_else(|| Error::Storage("a stamp of the copy".into()))
}

fn object_of(bytes: Vec<u8>) -> Result<ObjectId, Error> {
    bytes
        .try_into()
        .map(ObjectId)
        .map_err(|_| Error::Storage("an object id of the copy".into()))
}

fn json(v: &Value) -> String {
    serde_json::to_string(v).expect("a value always serializes")
}

fn value_of(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or(Value::Null)
}

pub struct Replica {
    /// One connection, behind a lock: a team (and its copies) is shared by the core's tasks.
    inner: Mutex<Connection>,
}

impl Replica {
    /// The copy in `path` (made with its folder if needed).
    pub fn open(path: &Path) -> Result<Self, Error> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| Error::Storage(e.to_string()))?;
        }
        let conn = Connection::open(path).map_err(db)?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")
            .map_err(db)?;
        Self::with(conn)
    }

    pub fn in_memory() -> Result<Self, Error> {
        Self::with(Connection::open_in_memory().map_err(db)?)
    }

    fn with(conn: Connection) -> Result<Self, Error> {
        conn.execute_batch(SCHEMA).map_err(db)?;
        // A copy made before the claims (TS-6): the outbox's fence added.
        let has: bool = conn
            .prepare("SELECT 1 FROM pragma_table_info('outbox') WHERE name = 'fence'")
            .and_then(|mut s| s.exists([]))
            .map_err(db)?;
        if !has {
            conn.execute_batch("ALTER TABLE outbox ADD COLUMN fence BLOB")
                .map_err(db)?;
        }
        Ok(Self {
            inner: Mutex::new(conn),
        })
    }

    fn conn(&self) -> MutexGuard<'_, Connection> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    // === Transactions ===

    /// Runs `f` in a transaction: kept, or rolled back on its error.
    pub fn atomically<T>(&self, f: impl FnOnce(&Self) -> Result<T, Error>) -> Result<T, Error> {
        self.conn().execute_batch("BEGIN IMMEDIATE").map_err(db)?;
        match f(self) {
            Ok(t) => {
                self.conn().execute_batch("COMMIT").map_err(db)?;
                Ok(t)
            }
            Err(e) => {
                if !self.conn().is_autocommit() {
                    let _ = self.conn().execute_batch("ROLLBACK");
                }
                Err(e)
            }
        }
    }

    // === The meta ===

    pub fn meta(&self) -> Result<ProjectMeta, Error> {
        let text: Option<String> = self
            .conn()
            .query_row("SELECT v FROM meta WHERE k = 'meta'", [], |r| r.get(0))
            .optional()
            .map_err(db)?;
        match text {
            Some(t) => serde_json::from_str(&t).map_err(|e| Error::Storage(e.to_string())),
            None => Ok(ProjectMeta::default()),
        }
    }

    pub fn set_meta(&self, meta: &ProjectMeta) -> Result<(), Error> {
        let text = serde_json::to_string(meta).map_err(|e| Error::Storage(e.to_string()))?;
        self.conn()
            .execute(
                "INSERT INTO meta (k, v) VALUES ('meta', ?1) ON CONFLICT (k) DO UPDATE SET v = excluded.v",
                [text],
            )
            .map_err(db)?;
        Ok(())
    }

    // === The journal and the state ===

    /// Applies an operation (its digest given: that of its bytes as signed), once: false when the copy had it. `seq`
    /// is its number in the stream, None for one of this device not sent yet.
    pub fn apply(
        &self,
        op: &Operation,
        digest: &[u8; 32],
        seq: Option<u64>,
    ) -> Result<bool, Error> {
        let new = self
            .conn()
            .execute(
                "INSERT OR IGNORE INTO ops (digest, seq, author, dseq, op) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    digest.as_slice(),
                    seq.map(|s| s as i64),
                    op.author.device.0.as_slice(),
                    op.dseq as i64,
                    op.to_bytes()
                ],
            )
            .map_err(db)?
            == 1;
        if !new {
            if let Some(s) = seq {
                self.conn()
                    .execute(
                        "UPDATE ops SET seq = ?2 WHERE digest = ?1 AND seq IS NULL",
                        params![digest.as_slice(), s as i64],
                    )
                    .map_err(db)?;
            }
            return Ok(false);
        }
        if let OpBody::Number {
            object,
            counter,
            at,
        } = &op.body
        {
            self.conn()
                .execute(
                    "INSERT OR IGNORE INTO counts (digest, counter, object, at) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        digest.as_slice(),
                        counter,
                        object.0.as_slice(),
                        at.map(i64::from)
                    ],
                )
                .map_err(db)?;
        }
        self.apply_body(op)?;
        Ok(true)
    }

    /// Gives the numbers of the counters (TS-9), in the server's order, to the operations `Number` read up to `upto`
    /// (the stream's number this copy read every entry before): each object its first number, after the highest one
    /// given; one given as it is (`at`, an import's) when it is free. The same on every copy, as they read the same
    /// order; a number given is never taken back. This device's operations not read back yet wait.
    pub fn assign_numbers(&self, upto: u64) -> Result<usize, Error> {
        type Row = (String, Vec<u8>, Option<i64>);
        let waiting: Vec<Row> = {
            let c = self.conn();
            let mut stmt = c
                .prepare(
                    "SELECT c.counter, c.object, c.at FROM counts c JOIN ops o ON o.digest = c.digest
                     WHERE o.seq IS NOT NULL AND o.seq <= ?1
                     AND NOT EXISTS (SELECT 1 FROM numbers n WHERE n.counter = c.counter AND n.object = c.object)
                     ORDER BY o.seq",
                )
                .map_err(db)?;
            let rows = stmt
                .query_map([upto as i64], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                .map_err(db)?;
            rows.collect::<Result<_, _>>().map_err(db)?
        };
        let mut given = 0;
        let c = self.conn();
        for (counter, object, at) in waiting {
            let has = c
                .query_row(
                    "SELECT 1 FROM numbers WHERE counter = ?1 AND object = ?2",
                    params![counter, object],
                    |_| Ok(()),
                )
                .optional()
                .map_err(db)?
                .is_some();
            if has {
                continue;
            }
            let taken = |n: i64| -> Result<bool, Error> {
                Ok(c.query_row(
                    "SELECT 1 FROM numbers WHERE counter = ?1 AND number = ?2",
                    params![counter, n],
                    |_| Ok(()),
                )
                .optional()
                .map_err(db)?
                .is_some())
            };
            let number = match at {
                Some(n) if n > 0 && !taken(n)? => n,
                _ => {
                    let max: Option<i64> = c
                        .query_row(
                            "SELECT MAX(number) FROM numbers WHERE counter = ?1",
                            [&counter],
                            |r| r.get(0),
                        )
                        .map_err(db)?;
                    max.unwrap_or(0) + 1
                }
            };
            c.execute(
                "INSERT INTO numbers (counter, object, number) VALUES (?1, ?2, ?3)",
                params![counter, object, number],
            )
            .map_err(db)?;
            given += 1;
        }
        // The counts of operations gone from the journal (a snapshot) were given before it.
        c.execute(
            "DELETE FROM counts WHERE digest NOT IN (SELECT digest FROM ops)",
            [],
        )
        .map_err(db)?;
        Ok(given)
    }

    /// The number an object has of a counter, once the server's order gave it.
    pub fn number(&self, counter: &str, object: &ObjectId) -> Result<Option<u32>, Error> {
        self.conn()
            .query_row(
                "SELECT number FROM numbers WHERE counter = ?1 AND object = ?2",
                params![counter, object.0.as_slice()],
                |r| r.get::<_, i64>(0),
            )
            .optional()
            .map(|n| n.map(|n| n as u32))
            .map_err(db)
    }

    /// Every number of a counter: object and number, by number.
    pub fn numbers(&self, counter: &str) -> Result<Vec<(ObjectId, u32)>, Error> {
        let rows: Vec<(Vec<u8>, i64)> = {
            let c = self.conn();
            let mut stmt = c
                .prepare("SELECT object, number FROM numbers WHERE counter = ?1 ORDER BY number")
                .map_err(db)?;
            let rows = stmt
                .query_map([counter], |r| Ok((r.get(0)?, r.get(1)?)))
                .map_err(db)?;
            rows.collect::<Result<_, _>>().map_err(db)?
        };
        rows.into_iter()
            .map(|(o, n)| Ok((object_of(o)?, n as u32)))
            .collect()
    }

    /// Whether an object asked a number of a counter that the server's order has not given yet (this device's, not
    /// sent or not read back).
    pub fn number_waits(&self, counter: &str, object: &ObjectId) -> Result<bool, Error> {
        if self.number(counter, object)?.is_some() {
            return Ok(false);
        }
        Ok(self
            .conn()
            .query_row(
                "SELECT 1 FROM counts WHERE counter = ?1 AND object = ?2",
                params![counter, object.0.as_slice()],
                |_| Ok(()),
            )
            .optional()
            .map_err(db)?
            .is_some())
    }

    /// What an operation does to the state. Every statement is the same whatever the order of the operations: a
    /// write is kept by its stamp, an object's kind is its latest creation's, a deletion stays.
    fn apply_body(&self, op: &Operation) -> Result<(), Error> {
        let stamp = op.hlc.to_bytes();
        let write = |object: &ObjectId, field: &str, value: &Value, base: Option<&Hlc>| {
            self.conn()
                .execute(
                    "INSERT OR IGNORE INTO writes (object, field, stamp, base, value) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        object.0.as_slice(),
                        field,
                        stamp.as_slice(),
                        base.map(|b| b.to_bytes().to_vec()),
                        json(value)
                    ],
                )
                .map(|_| ())
                .map_err(db)
        };
        match &op.body {
            OpBody::Create {
                object,
                kind,
                fields,
            } => {
                self.conn()
                    .execute(
                        "INSERT INTO objects (id, kind, created) VALUES (?1, ?2, ?3)
                         ON CONFLICT (id) DO UPDATE SET kind = excluded.kind, created = excluded.created
                         WHERE objects.created IS NULL OR excluded.created > objects.created",
                        params![object.0.as_slice(), kind, stamp.as_slice()],
                    )
                    .map_err(db)?;
                for (field, value) in fields {
                    write(object, field, value, None)?;
                }
            }
            OpBody::Set {
                object,
                field,
                value,
                base,
            } => write(object, field, value, base.as_ref())?,
            OpBody::Move {
                object,
                column,
                rank,
                base,
            } => {
                write(
                    object,
                    "column",
                    &Value::String(column.clone()),
                    base.as_ref(),
                )?;
                if let Some(rank) = rank {
                    write(object, "rank", &Value::String(rank.clone()), None)?;
                }
            }
            OpBody::Delete { object } => {
                self.conn()
                    .execute(
                        "INSERT INTO objects (id, deleted) VALUES (?1, ?2)
                         ON CONFLICT (id) DO UPDATE SET deleted = excluded.deleted
                         WHERE objects.deleted IS NULL OR excluded.deleted > objects.deleted",
                        params![object.0.as_slice(), stamp.as_slice()],
                    )
                    .map_err(db)?;
            }
            OpBody::Version {
                object,
                version,
                bases,
                content,
            } => {
                let bases: Vec<String> = bases.iter().map(|b| hex::encode(b.0)).collect();
                self.conn()
                    .execute(
                        "INSERT OR IGNORE INTO versions (object, id, bases, stamp, content) VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![
                            object.0.as_slice(),
                            version.0.as_slice(),
                            serde_json::to_string(&bases).expect("strings serialize"),
                            stamp.as_slice(),
                            json(content)
                        ],
                    )
                    .map_err(db)?;
            }
            // Its number comes from the server's order (`assign_numbers`), not from the operation alone.
            OpBody::Number { .. } => {}
        }
        Ok(())
    }

    /// The member a device wrote as, as an operation of the journal says it (its author, whose signature was checked
    /// when it was read: TS-10, the provenance of a memory). None when the journal holds none of that device (none
    /// yet, or all before the snapshot).
    pub fn author_member(
        &self,
        device: &DeviceId,
    ) -> Result<Option<escouade_team_crypto::MemberId>, Error> {
        let op: Option<Vec<u8>> = self
            .conn()
            .query_row(
                "SELECT op FROM ops WHERE author = ?1 LIMIT 1",
                [device.0.as_slice()],
                |r| r.get(0),
            )
            .optional()
            .map_err(db)?;
        Ok(op
            .and_then(|b| Operation::from_bytes(&b).ok())
            .filter(|o| o.author.device == *device)
            .map(|o| o.author.member))
    }

    /// How many operations the journal holds (after the snapshot).
    pub fn journal_len(&self) -> Result<u64, Error> {
        self.conn()
            .query_row("SELECT COUNT(*) FROM ops", [], |r| r.get::<_, i64>(0))
            .map(|n| n as u64)
            .map_err(db)
    }

    /// The state again from the snapshot and the journal: the same, whatever order the journal is replayed in.
    pub fn rebuild(&self) -> Result<(), Error> {
        self.atomically(|r| {
            r.clear_state()?;
            if let Some(state) = r.base()? {
                r.put_state(&state)?;
            }
            let ops: Vec<Vec<u8>> = {
                let c = r.conn();
                let mut stmt = c
                    .prepare("SELECT op FROM ops ORDER BY digest")
                    .map_err(db)?;
                let rows = stmt.query_map([], |row| row.get(0)).map_err(db)?;
                rows.collect::<Result<_, _>>().map_err(db)?
            };
            for bytes in ops {
                r.apply_body(&Operation::from_bytes(&bytes)?)?;
            }
            let cursor = r.meta()?.cursor;
            r.assign_numbers(cursor)?;
            Ok(())
        })
    }

    fn clear_state(&self) -> Result<(), Error> {
        self.conn()
            .execute_batch(
                "DELETE FROM objects; DELETE FROM writes; DELETE FROM versions; DELETE FROM numbers;",
            )
            .map_err(db)
    }

    fn base(&self) -> Result<Option<State>, Error> {
        let bytes: Option<Vec<u8>> = self
            .conn()
            .query_row("SELECT state FROM base WHERE id = 1", [], |r| r.get(0))
            .optional()
            .map_err(db)?;
        bytes
            .map(|b| serde_json::from_slice(&b).map_err(|e| Error::Encoding(e.to_string())))
            .transpose()
    }

    fn put_state(&self, state: &State) -> Result<(), Error> {
        for (id, kind, created, deleted) in &state.objects {
            self.conn()
                .execute(
                    "INSERT OR REPLACE INTO objects (id, kind, created, deleted) VALUES (?1, ?2, ?3, ?4)",
                    params![id, kind, created, deleted],
                )
                .map_err(db)?;
        }
        for (object, field, stamp, base, value) in &state.writes {
            self.conn()
                .execute(
                    "INSERT OR IGNORE INTO writes (object, field, stamp, base, value) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![object, field, stamp, base, value],
                )
                .map_err(db)?;
        }
        for (object, id, bases, stamp, content) in &state.versions {
            self.conn()
                .execute(
                    "INSERT OR IGNORE INTO versions (object, id, bases, stamp, content) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![object, id, bases, stamp, content],
                )
                .map_err(db)?;
        }
        for (counter, object, number) in &state.numbers {
            self.conn()
                .execute(
                    "INSERT OR IGNORE INTO numbers (counter, object, number) VALUES (?1, ?2, ?3)",
                    params![counter, object, number],
                )
                .map_err(db)?;
        }
        Ok(())
    }

    /// Every row of the state, in order: what a snapshot holds, and what two copies compare.
    pub fn state(&self) -> Result<State, Error> {
        let mut out = State::default();
        let c = self.conn();
        let mut stmt = c
            .prepare("SELECT id, kind, created, deleted FROM objects ORDER BY id")
            .map_err(db)?;
        out.objects = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .map_err(db)?
            .collect::<Result<_, _>>()
            .map_err(db)?;
        let mut stmt = c
            .prepare("SELECT object, field, stamp, base, value FROM writes ORDER BY object, field, stamp")
            .map_err(db)?;
        out.writes = stmt
            .query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .map_err(db)?
            .collect::<Result<_, _>>()
            .map_err(db)?;
        let mut stmt = c
            .prepare("SELECT object, id, bases, stamp, content FROM versions ORDER BY object, id")
            .map_err(db)?;
        out.versions = stmt
            .query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .map_err(db)?
            .collect::<Result<_, _>>()
            .map_err(db)?;
        let mut stmt = c
            .prepare("SELECT counter, object, number FROM numbers ORDER BY counter, number")
            .map_err(db)?;
        out.numbers = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(db)?
            .collect::<Result<_, _>>()
            .map_err(db)?;
        Ok(out)
    }

    pub fn dump(&self) -> Result<Vec<u8>, Error> {
        serde_json::to_vec(&self.state()?).map_err(|e| Error::Encoding(e.to_string()))
    }

    /// A snapshot's state takes the place of the start of the journal: the state is the snapshot's, then the
    /// operations after it (and this device's not sent yet) applied again.
    pub fn load_snapshot(&self, state: &[u8], upto: u64) -> Result<(), Error> {
        let state: State =
            serde_json::from_slice(state).map_err(|e| Error::Encoding(format!("snapshot: {e}")))?;
        self.atomically(|r| {
            r.set_base(&state)?;
            r.conn()
                .execute(
                    "DELETE FROM ops WHERE seq IS NOT NULL AND seq <= ?1",
                    [upto as i64],
                )
                .map_err(db)?;
            Ok(())
        })?;
        self.rebuild()
    }

    /// After this device made a snapshot at `upto`: its state is the base, the journal before it forgotten.
    pub fn compact(&self, upto: u64) -> Result<(), Error> {
        let state = self.state()?;
        self.atomically(|r| {
            r.set_base(&state)?;
            r.conn()
                .execute(
                    "DELETE FROM ops WHERE seq IS NOT NULL AND seq <= ?1",
                    [upto as i64],
                )
                .map_err(db)?;
            Ok(())
        })?;
        // The base holds everything applied, the operations kept included: replaying them changes nothing.
        self.rebuild()
    }

    fn set_base(&self, state: &State) -> Result<(), Error> {
        let bytes = serde_json::to_vec(state).map_err(|e| Error::Encoding(e.to_string()))?;
        self.conn()
            .execute(
                "INSERT INTO base (id, state) VALUES (1, ?1) ON CONFLICT (id) DO UPDATE SET state = excluded.state",
                [bytes],
            )
            .map_err(db)?;
        Ok(())
    }

    // === Reading ===

    fn live_objects(&self, kind: Option<&str>) -> Result<Vec<(ObjectId, String)>, Error> {
        let c = self.conn();
        let mut stmt = c
            .prepare(
                "SELECT id, kind FROM objects WHERE created IS NOT NULL AND deleted IS NULL
                 AND (?1 IS NULL OR kind = ?1) ORDER BY created, id",
            )
            .map_err(db)?;
        let rows = stmt
            .query_map([kind], |r| {
                Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(db)?;
        let mut out = Vec::new();
        for row in rows {
            let (id, kind) = row.map_err(db)?;
            out.push((object_of(id)?, kind));
        }
        Ok(out)
    }

    fn fields(&self, id: &ObjectId) -> Result<BTreeMap<String, Value>, Error> {
        let c = self.conn();
        let mut stmt = c
            .prepare(
                "SELECT field, value FROM writes w WHERE object = ?1 AND stamp =
                 (SELECT MAX(stamp) FROM writes v WHERE v.object = w.object AND v.field = w.field)",
            )
            .map_err(db)?;
        let rows = stmt
            .query_map([id.0.as_slice()], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(db)?;
        let mut out = BTreeMap::new();
        for row in rows {
            let (field, value) = row.map_err(db)?;
            out.insert(field, value_of(&value));
        }
        Ok(out)
    }

    /// The objects created and not deleted, oldest first; of one kind (`ticket`) when given.
    pub fn objects(&self, kind: Option<&str>) -> Result<Vec<ObjectView>, Error> {
        self.live_objects(kind)?
            .into_iter()
            .map(|(id, kind)| {
                Ok(ObjectView {
                    id,
                    kind,
                    fields: self.fields(&id)?,
                })
            })
            .collect()
    }

    /// An object, None when it is not created (yet) or deleted.
    pub fn object(&self, id: &ObjectId) -> Result<Option<ObjectView>, Error> {
        let kind: Option<String> = self
            .conn()
            .query_row(
                "SELECT kind FROM objects WHERE id = ?1 AND created IS NOT NULL AND deleted IS NULL",
                [id.0.as_slice()],
                |r| r.get(0),
            )
            .optional()
            .map_err(db)?;
        kind.map(|kind| {
            Ok(ObjectView {
                id: *id,
                kind,
                fields: self.fields(id)?,
            })
        })
        .transpose()
    }

    /// Whether an object was deleted.
    pub fn is_deleted(&self, id: &ObjectId) -> Result<bool, Error> {
        Ok(self
            .conn()
            .query_row(
                "SELECT 1 FROM objects WHERE id = ?1 AND deleted IS NOT NULL",
                [id.0.as_slice()],
                |_| Ok(()),
            )
            .optional()
            .map_err(db)?
            .is_some())
    }

    /// The stamp of a field's value now: the base of the next write of it.
    pub fn field_stamp(&self, id: &ObjectId, field: &str) -> Result<Option<Hlc>, Error> {
        let bytes: Option<Vec<u8>> = self
            .conn()
            .query_row(
                "SELECT MAX(stamp) FROM writes WHERE object = ?1 AND field = ?2",
                params![id.0.as_slice(), field],
                |r| r.get(0),
            )
            .map_err(db)?;
        bytes.map(|b| stamp_of(&b)).transpose()
    }

    /// The writes of an object that lost to a concurrent one (the same base as the winner's).
    pub fn conflicts(&self, id: &ObjectId) -> Result<Vec<Conflict>, Error> {
        type Row = (String, Vec<u8>, Option<Vec<u8>>, String);
        let c = self.conn();
        let mut stmt = c
            .prepare(
                "SELECT field, stamp, base, value FROM writes w WHERE object = ?1 AND stamp =
                 (SELECT MAX(stamp) FROM writes v WHERE v.object = w.object AND v.field = w.field) ORDER BY field",
            )
            .map_err(db)?;
        let winners: Vec<Row> = stmt
            .query_map([id.0.as_slice()], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })
            .map_err(db)?
            .collect::<Result<_, _>>()
            .map_err(db)?;
        let mut out = Vec::new();
        for (field, stamp, base, value) in winners {
            let mut stmt = c
                .prepare(
                    "SELECT stamp, value FROM writes WHERE object = ?1 AND field = ?2 AND base IS ?3 AND stamp <> ?4
                     ORDER BY stamp DESC",
                )
                .map_err(db)?;
            let losers: Vec<(Vec<u8>, String)> = stmt
                .query_map(params![id.0.as_slice(), field, base, stamp], |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })
                .map_err(db)?
                .collect::<Result<_, _>>()
                .map_err(db)?;
            let base_value: Option<String> = match &base {
                Some(b) => c
                    .query_row(
                        "SELECT value FROM writes WHERE object = ?1 AND field = ?2 AND stamp = ?3",
                        params![id.0.as_slice(), field, b],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(db)?,
                None => None,
            };
            for (s, v) in losers {
                out.push(Conflict {
                    field: field.clone(),
                    value: value_of(&v),
                    stamp: stamp_of(&s)?,
                    winner: value_of(&value),
                    winner_stamp: stamp_of(&stamp)?,
                    base: base_value.as_deref().map(value_of),
                });
            }
        }
        Ok(out)
    }

    /// The versions of a document that count: the heads, the highest stamp current.
    pub fn versions(&self, id: &ObjectId) -> Result<Versions, Error> {
        let c = self.conn();
        let mut stmt = c
            .prepare("SELECT id, bases, stamp, content FROM versions WHERE object = ?1")
            .map_err(db)?;
        let rows: Vec<(Vec<u8>, String, Vec<u8>, String)> = stmt
            .query_map([id.0.as_slice()], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })
            .map_err(db)?
            .collect::<Result<_, _>>()
            .map_err(db)?;
        let mut all = Vec::new();
        let mut named: BTreeSet<ObjectId> = BTreeSet::new();
        for (vid, bases, stamp, content) in rows {
            let bases: Vec<String> = serde_json::from_str(&bases).unwrap_or_default();
            let bases: Vec<ObjectId> = bases
                .iter()
                .filter_map(|b| {
                    let mut out = [0u8; 16];
                    hex::decode_to_slice(b, &mut out)
                        .ok()
                        .map(|_| ObjectId(out))
                })
                .collect();
            named.extend(bases.iter().copied());
            all.push(Version {
                id: object_of(vid)?,
                bases,
                stamp: stamp_of(&stamp)?,
                content: value_of(&content),
            });
        }
        let mut heads: Vec<Version> = all.into_iter().filter(|v| !named.contains(&v.id)).collect();
        heads.sort_by_key(|v| std::cmp::Reverse(v.stamp));
        let mut heads = heads.into_iter();
        Ok(Versions {
            current: heads.next(),
            conflicts: heads.collect(),
        })
    }

    /// The documents that have versions (the spec's pages, TS-9).
    pub fn versioned(&self) -> Result<Vec<ObjectId>, Error> {
        let rows: Vec<Vec<u8>> = {
            let c = self.conn();
            let mut stmt = c
                .prepare("SELECT DISTINCT object FROM versions ORDER BY object")
                .map_err(db)?;
            let rows = stmt.query_map([], |r| r.get(0)).map_err(db)?;
            rows.collect::<Result<_, _>>().map_err(db)?
        };
        rows.into_iter().map(object_of).collect()
    }

    /// Every version of a document, oldest first: what the three-way merge looks for a base in (TS-9).
    pub fn history(&self, id: &ObjectId) -> Result<Vec<Version>, Error> {
        let rows: Vec<(Vec<u8>, String, Vec<u8>, String)> = {
            let c = self.conn();
            let mut stmt = c
                .prepare("SELECT id, bases, stamp, content FROM versions WHERE object = ?1 ORDER BY stamp")
                .map_err(db)?;
            let rows = stmt
                .query_map([id.0.as_slice()], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
                })
                .map_err(db)?;
            rows.collect::<Result<_, _>>().map_err(db)?
        };
        rows.into_iter()
            .map(|(vid, bases, stamp, content)| {
                let bases: Vec<String> = serde_json::from_str(&bases).unwrap_or_default();
                Ok(Version {
                    id: object_of(vid)?,
                    bases: bases
                        .iter()
                        .filter_map(|b| {
                            let mut out = [0u8; 16];
                            hex::decode_to_slice(b, &mut out)
                                .ok()
                                .map(|_| ObjectId(out))
                        })
                        .collect(),
                    stamp: stamp_of(&stamp)?,
                    content: value_of(&content),
                })
            })
            .collect()
    }

    /// The version two versions of a document were both made after, the latest of them (by stamp): the base of
    /// their three-way merge. None when they have none (two pages made apart under the same path).
    pub fn common_base(
        &self,
        id: &ObjectId,
        a: &ObjectId,
        b: &ObjectId,
    ) -> Result<Option<Version>, Error> {
        let all = self.history(id)?;
        let by: BTreeMap<ObjectId, &Version> = all.iter().map(|v| (v.id, v)).collect();
        let ancestors = |from: &ObjectId| {
            let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
            let mut todo = vec![*from];
            while let Some(v) = todo.pop() {
                if seen.insert(v) {
                    if let Some(x) = by.get(&v) {
                        todo.extend(x.bases.iter().copied());
                    }
                }
            }
            seen
        };
        let (of_a, of_b) = (ancestors(a), ancestors(b));
        Ok(all
            .iter()
            .filter(|v| of_a.contains(&v.id) && of_b.contains(&v.id))
            .max_by_key(|v| v.stamp)
            .cloned())
    }

    // === The outbox ===

    /// Puts an operation of this device in the outbox, after it.
    pub fn queue(&self, digest: &[u8; 32], op: &[u8]) -> Result<(), Error> {
        self.queue_fenced(digest, op, None)
    }

    /// The same, for a write of a ticket's holder (TS-6): it goes with the claim's id and token, and the server
    /// refuses it once another device took the ticket over.
    pub fn queue_fenced(
        &self,
        digest: &[u8; 32],
        op: &[u8],
        fence: Option<&Fence>,
    ) -> Result<(), Error> {
        self.conn()
            .execute(
                "INSERT INTO outbox (digest, op, fence) VALUES (?1, ?2, ?3)",
                params![digest.as_slice(), op, fence.map(Fence::to_bytes)],
            )
            .map_err(db)?;
        Ok(())
    }

    /// The fence of an operation of the outbox, if it has one.
    pub fn outbox_fence(&self, n: i64) -> Result<Option<Fence>, Error> {
        let bytes: Option<Option<Vec<u8>>> = self
            .conn()
            .query_row("SELECT fence FROM outbox WHERE n = ?1", [n], |r| r.get(0))
            .optional()
            .map_err(db)?;
        Ok(bytes.flatten().and_then(|b| Fence::from_bytes(&b)))
    }

    /// Takes every operation of the outbox back, in order, with its fence: they leave the journal too (not sent, they
    /// are this device's alone), and the state is replayed without them. The caller writes again those it keeps.
    pub fn take_unsent(&self) -> Result<Vec<Unsent>, Error> {
        self.atomically(|r| {
            let rows: Vec<OutboxRow> = {
                let c = r.conn();
                let mut stmt = c
                    .prepare("SELECT digest, op, fence FROM outbox ORDER BY n")
                    .map_err(db)?;
                let rows = stmt
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                    .map_err(db)?;
                rows.collect::<Result<_, _>>().map_err(db)?
            };
            for (digest, _, _) in &rows {
                r.conn()
                    .execute(
                        "DELETE FROM ops WHERE digest = ?1 AND seq IS NULL",
                        [digest.as_slice()],
                    )
                    .map_err(db)?;
            }
            r.conn().execute("DELETE FROM outbox", []).map_err(db)?;
            Ok(rows
                .into_iter()
                .map(|(_, op, fence)| (op, fence.and_then(|f| Fence::from_bytes(&f))))
                .collect())
        })
        .and_then(|taken| {
            self.rebuild()?;
            Ok(taken)
        })
    }

    /// The first operation waiting: its place, its digest and its bytes.
    pub fn outbox_first(&self) -> Result<Option<Waiting>, Error> {
        let row: Option<(i64, Vec<u8>, Vec<u8>)> = self
            .conn()
            .query_row(
                "SELECT n, digest, op FROM outbox ORDER BY n LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(db)?;
        row.map(|(n, d, op)| {
            let d: [u8; 32] = d
                .try_into()
                .map_err(|_| Error::Storage("a digest of the outbox".into()))?;
            Ok((n, d, op))
        })
        .transpose()
    }

    pub fn outbox_remove(&self, n: i64) -> Result<(), Error> {
        self.conn()
            .execute("DELETE FROM outbox WHERE n = ?1", [n])
            .map_err(db)?;
        Ok(())
    }

    /// How many operations wait to go.
    pub fn pending(&self) -> Result<usize, Error> {
        self.conn()
            .query_row("SELECT COUNT(*) FROM outbox", [], |r| r.get::<_, i64>(0))
            .map(|n| n as usize)
            .map_err(db)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use escouade_team_crypto::ops::Author;
    use escouade_team_crypto::{MemberId, ProjectId};
    use proptest::prelude::*;

    const P: ProjectId = ProjectId([7; 16]);

    fn device(n: u8) -> DeviceId {
        DeviceId([n + 1; 16])
    }

    fn object(n: u8) -> ObjectId {
        ObjectId([n + 1; 16])
    }

    /// One step of the generated history: a device, an action, an object, a field, a value, how far its clock goes.
    type Step = (u8, u8, u8, u8, u8, u8);

    /// The operations of a history, as each device writes them (its own clock, its last write of a field as base).
    fn operations(steps: &[Step]) -> Vec<(Operation, [u8; 32])> {
        let mut clocks = [Clock::default(); 3];
        let mut walls = [0u64; 3];
        let mut seen: BTreeMap<(u8, u8, u8), Hlc> = BTreeMap::new();
        let mut versions: BTreeMap<u8, Vec<ObjectId>> = BTreeMap::new();
        let mut dseq = [0u64; 3];
        let mut out = Vec::new();
        for (i, &(d, action, o, f, v, advance)) in steps.iter().enumerate() {
            let d = d % 3;
            let di = d as usize;
            walls[di] += u64::from(advance % 3);
            let hlc = clocks[di].tick(walls[di], device(d));
            let object = object(o % 3);
            let field = ["title", "description", "column"][(f % 3) as usize].to_string();
            let value = Value::from(format!("v{v}"));
            let body = match action % 6 {
                0 => OpBody::Create {
                    object,
                    kind: "ticket".into(),
                    fields: [(field.clone(), value)].into_iter().collect(),
                },
                1 | 2 => OpBody::Set {
                    object,
                    field: field.clone(),
                    value,
                    base: seen.get(&(d, o % 3, f % 3)).copied(),
                },
                3 => OpBody::Move {
                    object,
                    column: format!("c{v}"),
                    rank: (v % 2 == 0).then(|| format!("r{v}")),
                    base: seen.get(&(d, o % 3, 2)).copied(),
                },
                4 => OpBody::Delete { object },
                _ => {
                    let known = versions.entry(o % 3).or_default();
                    let bases = known.iter().rev().take((v % 2) as usize).copied().collect();
                    let version = ObjectId([i as u8; 16]);
                    known.push(version);
                    OpBody::Version {
                        object,
                        version,
                        bases,
                        content: value,
                    }
                }
            };
            seen.insert((d, o % 3, f % 3), hlc);
            dseq[di] += 1;
            let op = Operation {
                project: P,
                author: Author {
                    member: MemberId([d; 16]),
                    device: device(d),
                    key: vec![d; 32],
                },
                dseq: dseq[di],
                prev: [0; 32],
                team_epoch: 0,
                team_head: [0; 32],
                hlc,
                body,
            };
            let digest = Operation::digest_of(&op.to_bytes());
            out.push((op, digest));
        }
        out
    }

    type Read = (State, Vec<ObjectView>, Vec<Vec<Conflict>>, Vec<Versions>);

    /// Everything the interface reads of a copy.
    fn read(r: &Replica) -> Read {
        let ids: Vec<ObjectId> = (0..3).map(object).collect();
        (
            r.state().unwrap(),
            r.objects(None).unwrap(),
            ids.iter().map(|i| r.conflicts(i).unwrap()).collect(),
            ids.iter().map(|i| r.versions(i).unwrap()).collect(),
        )
    }

    fn history() -> impl Strategy<Value = (Vec<Step>, Vec<usize>, Vec<usize>)> {
        prop::collection::vec(any::<Step>(), 1..40).prop_flat_map(|steps| {
            let n = steps.len();
            let order: Vec<usize> = (0..n).collect();
            (
                Just(steps),
                Just(order).prop_shuffle(),
                prop::collection::vec(0..n, 0..10),
            )
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// Whatever order the operations come in, and played again, every copy ends the same.
        #[test]
        fn every_copy_ends_the_same(input in history()) {
            let (steps, order, again) = input;
            let ops = operations(&steps);
            let a = Replica::in_memory().unwrap();
            for (op, d) in &ops {
                a.apply(op, d, None).unwrap();
            }
            let b = Replica::in_memory().unwrap();
            for i in &order {
                b.apply(&ops[*i].0, &ops[*i].1, Some(*i as u64 + 1)).unwrap();
            }
            for i in &again {
                prop_assert!(!b.apply(&ops[*i].0, &ops[*i].1, None).unwrap());
            }
            let c = Replica::in_memory().unwrap();
            for (op, d) in ops.iter().rev() {
                c.apply(op, d, None).unwrap();
            }
            prop_assert_eq!(read(&a), read(&b));
            prop_assert_eq!(read(&a), read(&c));
            // Replayed from the journal, the same again.
            b.rebuild().unwrap();
            prop_assert_eq!(read(&a), read(&b));
        }

        /// A snapshot of a copy, then the rest of the operations: the same as the whole journal.
        #[test]
        fn a_snapshot_then_the_rest_is_the_whole(input in history(), cut in 0usize..40) {
            let (steps, order, _) = input;
            let ops = operations(&steps);
            let cut = cut.min(ops.len());
            let whole = Replica::in_memory().unwrap();
            for (op, d) in &ops {
                whole.apply(op, d, None).unwrap();
            }
            let first = Replica::in_memory().unwrap();
            for (i, j) in order.iter().take(cut).enumerate() {
                first.apply(&ops[*j].0, &ops[*j].1, Some(i as u64 + 1)).unwrap();
            }
            let later = Replica::in_memory().unwrap();
            later.load_snapshot(&first.dump().unwrap(), cut as u64).unwrap();
            for (i, j) in order.iter().enumerate().skip(cut) {
                later.apply(&ops[*j].0, &ops[*j].1, Some(i as u64 + 1)).unwrap();
            }
            prop_assert_eq!(read(&whole), read(&later));
        }
    }

    #[test]
    fn the_last_writer_wins_and_the_loser_is_kept() {
        let r = Replica::in_memory().unwrap();
        let steps: Vec<Step> = vec![
            // Device 0 creates object 0 with a title.
            (0, 0, 0, 0, 1, 1),
            // Devices 1 and 2 set the title from no base they saw, at the same wall: the device id breaks the tie.
            (1, 1, 0, 0, 2, 2),
            (2, 1, 0, 0, 3, 2),
            // Device 0 deletes object 1; device 1 writes it afterwards: it stays deleted.
            (0, 4, 1, 0, 0, 1),
            (1, 1, 1, 0, 4, 1),
        ];
        for (op, d) in operations(&steps) {
            r.apply(&op, &d, None).unwrap();
        }
        let o = r.object(&object(0)).unwrap().unwrap();
        assert_eq!(o.fields["title"], Value::from("v3"));
        // Neither saw the title of the creation: both it and the other write are kept as conflicts, newest first.
        let conflicts = r.conflicts(&object(0)).unwrap();
        let lost: Vec<&Value> = conflicts.iter().map(|c| &c.value).collect();
        assert_eq!(lost, [&Value::from("v2"), &Value::from("v1")]);
        assert!(conflicts.iter().all(|c| c.winner == "v3"));
        assert!(r.object(&object(1)).unwrap().is_none());
        assert!(r.is_deleted(&object(1)).unwrap());
        assert_eq!(r.objects(Some("ticket")).unwrap().len(), 1);
    }

    #[test]
    fn the_outbox_keeps_its_order() {
        let r = Replica::in_memory().unwrap();
        for i in 0..3u8 {
            r.queue(&[i; 32], &[i]).unwrap();
        }
        assert_eq!(r.pending().unwrap(), 3);
        let (n, d, op) = r.outbox_first().unwrap().unwrap();
        assert_eq!((d, op), ([0; 32], vec![0]));
        r.outbox_remove(n).unwrap();
        assert_eq!(r.outbox_first().unwrap().unwrap().1, [1; 32]);
    }
}
