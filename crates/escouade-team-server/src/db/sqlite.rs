//! SQLite, in WAL, by rusqlite: one connection, its calls on tokio's blocking threads. Writes take the base's lock
//! at their start (`BEGIN IMMEDIATE`), so two of them never interleave.

use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

use super::*;
use crate::util::random;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS teams (
    id BLOB PRIMARY KEY,
    created_at INTEGER NOT NULL,
    seq INTEGER NOT NULL DEFAULT 0,
    commits INTEGER NOT NULL DEFAULT 0,
    stored INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS devices (
    id BLOB PRIMARY KEY,
    team_id BLOB NOT NULL REFERENCES teams(id),
    public_key BLOB NOT NULL UNIQUE,
    role TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    revoked_at INTEGER
);
CREATE INDEX IF NOT EXISTS devices_team ON devices(team_id);
CREATE TABLE IF NOT EXISTS tokens (
    hash BLOB PRIMARY KEY,
    kind INTEGER NOT NULL,
    team_id BLOB,
    created_at INTEGER NOT NULL,
    expires_at INTEGER,
    used_at INTEGER,
    reused_at INTEGER
);
CREATE TABLE IF NOT EXISTS team_entries (
    team_id BLOB NOT NULL,
    seq INTEGER NOT NULL,
    kind INTEGER NOT NULL,
    epoch INTEGER,
    device_id BLOB NOT NULL,
    at INTEGER NOT NULL,
    payload BLOB NOT NULL,
    PRIMARY KEY (team_id, seq)
);
CREATE UNIQUE INDEX IF NOT EXISTS team_commits ON team_entries(team_id, epoch) WHERE kind = 1;
CREATE TABLE IF NOT EXISTS projects (
    team_id BLOB NOT NULL,
    id BLOB NOT NULL,
    seq INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (team_id, id)
);
CREATE TABLE IF NOT EXISTS project_entries (
    team_id BLOB NOT NULL,
    project_id BLOB NOT NULL,
    seq INTEGER NOT NULL,
    device_id BLOB NOT NULL,
    at INTEGER NOT NULL,
    payload BLOB NOT NULL,
    PRIMARY KEY (team_id, project_id, seq)
);
CREATE TABLE IF NOT EXISTS files (
    team_id BLOB NOT NULL,
    id BLOB NOT NULL,
    size INTEGER NOT NULL,
    device_id BLOB NOT NULL,
    at INTEGER NOT NULL,
    PRIMARY KEY (team_id, id)
);
";

pub struct Sqlite {
    conn: Arc<Mutex<Connection>>,
}

fn backend(e: impl std::fmt::Display) -> StoreError {
    StoreError::Backend(e.to_string())
}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        backend(e)
    }
}

fn id_of(v: Vec<u8>) -> Result<Id> {
    v.try_into()
        .map_err(|_| backend("an id of the wrong length"))
}

/// A row of `devices`: id, team, key, role, created, revoked.
type DeviceRow = (Vec<u8>, Vec<u8>, Vec<u8>, String, i64, Option<i64>);

fn device_row(row: &rusqlite::Row) -> rusqlite::Result<DeviceRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

fn to_device(r: DeviceRow) -> Result<Device> {
    Ok(Device {
        id: id_of(r.0)?,
        team: id_of(r.1)?,
        public_key: r
            .2
            .try_into()
            .map_err(|_| backend("a key of the wrong length"))?,
        role: Role::parse(&r.3),
        created_at: r.4,
        revoked_at: r.5,
    })
}

const DEVICE_COLUMNS: &str = "id, team_id, public_key, role, created_at, revoked_at";

impl Sqlite {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(backend)?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(Sqlite {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// The journal mode the base is in (`wal`).
    pub fn journal_mode(&self) -> Result<String> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        Ok(conn.query_row("PRAGMA journal_mode", [], |r| r.get(0))?)
    }

    async fn run<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut Connection) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = conn.lock().unwrap_or_else(|e| e.into_inner());
            f(&mut conn)
        })
        .await
        .map_err(backend)?
    }
}

fn read_entries(
    conn: &Connection,
    team: &Id,
    stream: Stream,
    after: u64,
    page: Page,
    from_seq: bool,
) -> Result<Vec<Entry>> {
    // `from_seq`: from `after` included (the conflict's preceding entries), else after it.
    let op = if from_seq { ">=" } else { ">" };
    let limit = page.entries as i64 + 1;
    let mut out = Vec::new();
    match stream {
        Stream::Team => {
            let mut stmt = conn.prepare_cached(&format!(
                "SELECT seq, kind, epoch, device_id, at, payload FROM team_entries
                 WHERE team_id = ?1 AND seq {op} ?2 ORDER BY seq LIMIT ?3"
            ))?;
            let rows = stmt.query_map(params![team.as_slice(), after as i64, limit], |r| {
                Ok(Entry {
                    seq: r.get::<_, i64>(0)? as u64,
                    kind: Kind::from_code(r.get(1)?),
                    epoch: r.get::<_, Option<i64>>(2)?.map(|e| e as u64),
                    device: r.get(3)?,
                    at: r.get(4)?,
                    payload: r.get(5)?,
                })
            })?;
            for row in rows {
                out.push(row?);
            }
        }
        Stream::Project(project) => {
            let mut stmt = conn.prepare_cached(&format!(
                "SELECT seq, device_id, at, payload FROM project_entries
                 WHERE team_id = ?1 AND project_id = ?2 AND seq {op} ?3 ORDER BY seq LIMIT ?4"
            ))?;
            let rows = stmt.query_map(
                params![team.as_slice(), project.as_slice(), after as i64, limit],
                |r| {
                    Ok(Entry {
                        seq: r.get::<_, i64>(0)? as u64,
                        kind: Kind::Message,
                        epoch: None,
                        device: r.get(1)?,
                        at: r.get(2)?,
                        payload: r.get(3)?,
                    })
                },
            )?;
            for row in rows {
                out.push(row?);
            }
        }
    }
    Ok(out)
}

#[async_trait]
impl Store for Sqlite {
    async fn migrate(&self) -> Result<()> {
        self.run(|c| {
            c.execute_batch(SCHEMA)?;
            // A base made before the used tokens were kept (TS-3): their columns added.
            for column in ["used_at", "reused_at"] {
                let has: bool = c
                    .prepare("SELECT 1 FROM pragma_table_info('tokens') WHERE name = ?1")?
                    .exists([column])?;
                if !has {
                    c.execute_batch(&format!("ALTER TABLE tokens ADD COLUMN {column} INTEGER"))?;
                }
            }
            Ok(())
        })
        .await
    }

    async fn ping(&self) -> Result<()> {
        self.run(|c| Ok(c.query_row("SELECT 1", [], |_| Ok(()))?))
            .await
    }

    async fn add_token(
        &self,
        hash: [u8; 32],
        kind: TokenKind,
        team: Option<Id>,
        expires_at: Option<i64>,
        now: i64,
    ) -> Result<()> {
        let kind = kind.code();
        self.run(move |c| {
            c.execute(
                "INSERT INTO tokens (hash, kind, team_id, created_at, expires_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![hash.as_slice(), kind, team.as_ref().map(|t| t.as_slice()), now, expires_at],
            )?;
            Ok(())
        })
        .await
    }

    async fn enroll(
        &self,
        hash: [u8; 32],
        public_key: [u8; 32],
        now: i64,
        quotas: Quotas,
    ) -> Result<Device> {
        self.run(move |c| {
            let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let token: Option<(i64, Option<Vec<u8>>, Option<i64>, Option<i64>)> = tx
                .query_row(
                    "SELECT kind, team_id, expires_at, used_at FROM tokens WHERE hash = ?1",
                    [hash.as_slice()],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .optional()?;
            let Some((kind, team, expires_at, used_at)) = token else {
                return Err(StoreError::BadToken);
            };
            if used_at.is_some() {
                tx.execute("UPDATE tokens SET reused_at = ?2 WHERE hash = ?1", params![hash.as_slice(), now])?;
                tx.commit()?;
                return Err(StoreError::TokenUsed);
            }
            if expires_at.is_some_and(|e| e <= now) {
                tx.execute("DELETE FROM tokens WHERE hash = ?1", [hash.as_slice()])?;
                tx.commit()?;
                return Err(StoreError::BadToken);
            }
            let taken: bool = tx
                .query_row("SELECT 1 FROM devices WHERE public_key = ?1", [public_key.as_slice()], |_| Ok(()))
                .optional()?
                .is_some();
            if taken {
                return Err(StoreError::KeyTaken);
            }
            let (team, role) = if kind == 0 {
                let team: Id = random();
                tx.execute("INSERT INTO teams (id, created_at) VALUES (?1, ?2)", params![team.as_slice(), now])?;
                (team, Role::Admin)
            } else {
                let team = id_of(team.ok_or(StoreError::BadToken)?)?;
                let count: i64 =
                    tx.query_row("SELECT COUNT(*) FROM devices WHERE team_id = ?1", [team.as_slice()], |r| r.get(0))?;
                if count >= quotas.devices as i64 {
                    return Err(StoreError::TooMany("devices"));
                }
                (team, if kind == TokenKind::Recovery.code() { Role::Admin } else { Role::Member })
            };
            let id: Id = random();
            tx.execute(
                "INSERT INTO devices (id, team_id, public_key, role, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id.as_slice(), team.as_slice(), public_key.as_slice(), role.as_str(), now],
            )?;
            // Kept used, not deleted: a second use is told apart from an unknown token.
            tx.execute("UPDATE tokens SET used_at = ?2 WHERE hash = ?1", params![hash.as_slice(), now])?;
            tx.commit()?;
            Ok(Device {
                id,
                team,
                public_key,
                role,
                created_at: now,
                revoked_at: None,
            })
        })
        .await
    }

    async fn device(&self, id: Id) -> Result<Option<Device>> {
        self.run(move |c| {
            let row = c
                .query_row(
                    &format!("SELECT {DEVICE_COLUMNS} FROM devices WHERE id = ?1"),
                    [id.as_slice()],
                    device_row,
                )
                .optional()?;
            row.map(to_device).transpose()
        })
        .await
    }

    async fn devices(&self, team: Id) -> Result<Vec<Device>> {
        self.run(move |c| {
            let mut stmt = c.prepare_cached(&format!(
                "SELECT {DEVICE_COLUMNS} FROM devices WHERE team_id = ?1 ORDER BY created_at, id"
            ))?;
            let rows = stmt.query_map([team.as_slice()], device_row)?;
            let mut out = Vec::new();
            for r in rows {
                out.push(to_device(r?)?);
            }
            Ok(out)
        })
        .await
    }

    async fn revoke(&self, team: Id, device: Id, now: i64) -> Result<bool> {
        self.run(move |c| {
            let n = c.execute(
                "UPDATE devices SET revoked_at = COALESCE(revoked_at, ?3) WHERE id = ?1 AND team_id = ?2",
                params![device.as_slice(), team.as_slice(), now],
            )?;
            Ok(n == 1)
        })
        .await
    }

    async fn set_role(&self, team: Id, device: Id, role: Role) -> Result<bool> {
        self.run(move |c| {
            let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let current: Option<(String, Option<i64>)> = tx
                .query_row(
                    "SELECT role, revoked_at FROM devices WHERE id = ?1 AND team_id = ?2",
                    params![device.as_slice(), team.as_slice()],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            let Some((current, revoked)) = current else {
                return Ok(false);
            };
            if Role::parse(&current) == Role::Admin && role == Role::Member && revoked.is_none() {
                let admins: i64 = tx.query_row(
                    "SELECT COUNT(*) FROM devices WHERE team_id = ?1 AND role = 'admin' AND revoked_at IS NULL",
                    [team.as_slice()],
                    |r| r.get(0),
                )?;
                if admins <= 1 {
                    return Err(StoreError::LastAdmin);
                }
            }
            tx.execute(
                "UPDATE devices SET role = ?3 WHERE id = ?1 AND team_id = ?2",
                params![device.as_slice(), team.as_slice(), role.as_str()],
            )?;
            tx.commit()?;
            Ok(true)
        })
        .await
    }

    async fn token_alerts(&self, team: Id) -> Result<Vec<TokenAlert>> {
        self.run(move |c| {
            let mut stmt = c.prepare_cached(
                "SELECT hash, used_at, reused_at FROM tokens WHERE team_id = ?1 AND reused_at IS NOT NULL ORDER BY reused_at",
            )?;
            let rows = stmt.query_map([team.as_slice()], |r| {
                Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))
            })?;
            let mut out = Vec::new();
            for row in rows {
                let (hash, used_at, reused_at) = row?;
                out.push(TokenAlert {
                    hash: hash.try_into().map_err(|_| backend("a hash of the wrong length"))?,
                    used_at,
                    reused_at,
                });
            }
            Ok(out)
        })
        .await
    }

    async fn sweep_tokens(&self, now: i64) -> Result<()> {
        self.run(move |c| {
            c.execute(
                "DELETE FROM tokens WHERE expires_at IS NOT NULL AND expires_at + 604800 <= ?1",
                [now],
            )?;
            Ok(())
        })
        .await
    }

    async fn append(
        &self,
        team: Id,
        stream: Stream,
        entry: NewEntry,
        quotas: Quotas,
        page: Page,
    ) -> Result<Entry> {
        self.run(move |c| {
            let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let (seq, commits, stored): (i64, i64, i64) = tx
                .query_row("SELECT seq, commits, stored FROM teams WHERE id = ?1", [team.as_slice()], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?))
                })?;
            let size = entry.payload.len() as i64;
            if (stored + size) as u64 > quotas.team_bytes {
                return Err(StoreError::Quota);
            }
            let out = match stream {
                Stream::Team => {
                    let is_commit = entry.kind == Kind::Commit;
                    if is_commit {
                        let asked = entry.epoch.unwrap_or(u64::MAX);
                        if asked != commits as u64 {
                            let from: Option<i64> = tx
                                .query_row(
                                    "SELECT seq FROM team_entries WHERE team_id = ?1 AND kind = 1 AND epoch = ?2",
                                    params![team.as_slice(), asked as i64],
                                    |r| r.get(0),
                                )
                                .optional()?;
                            let preceding = match from {
                                Some(from) => paginate(read_entries(&tx, &team, stream, from as u64, page, true)?, page).0,
                                None => Vec::new(),
                            };
                            return Err(StoreError::EpochTaken {
                                asked,
                                expected: commits as u64,
                                preceding,
                            });
                        }
                    }
                    let seq = seq + 1;
                    tx.execute(
                        "INSERT INTO team_entries (team_id, seq, kind, epoch, device_id, at, payload)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![
                            team.as_slice(),
                            seq,
                            entry.kind.code(),
                            entry.epoch.filter(|_| is_commit).map(|e| e as i64),
                            entry.device.as_slice(),
                            entry.at,
                            entry.payload
                        ],
                    )?;
                    tx.execute(
                        "UPDATE teams SET seq = ?2, commits = commits + ?3, stored = stored + ?4 WHERE id = ?1",
                        params![team.as_slice(), seq, is_commit as i64, size],
                    )?;
                    Entry {
                        seq: seq as u64,
                        kind: entry.kind,
                        epoch: entry.epoch.filter(|_| is_commit),
                        device: entry.device.to_vec(),
                        at: entry.at,
                        payload: entry.payload,
                    }
                }
                Stream::Project(project) => {
                    let current: Option<i64> = tx
                        .query_row(
                            "SELECT seq FROM projects WHERE team_id = ?1 AND id = ?2",
                            params![team.as_slice(), project.as_slice()],
                            |r| r.get(0),
                        )
                        .optional()?;
                    let current = match current {
                        Some(s) => s,
                        None => {
                            let n: i64 = tx.query_row(
                                "SELECT COUNT(*) FROM projects WHERE team_id = ?1",
                                [team.as_slice()],
                                |r| r.get(0),
                            )?;
                            if n >= quotas.projects as i64 {
                                return Err(StoreError::TooMany("projects"));
                            }
                            tx.execute(
                                "INSERT INTO projects (team_id, id) VALUES (?1, ?2)",
                                params![team.as_slice(), project.as_slice()],
                            )?;
                            0
                        }
                    };
                    let seq = current + 1;
                    tx.execute(
                        "INSERT INTO project_entries (team_id, project_id, seq, device_id, at, payload)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![team.as_slice(), project.as_slice(), seq, entry.device.as_slice(), entry.at, entry.payload],
                    )?;
                    tx.execute(
                        "UPDATE projects SET seq = ?3 WHERE team_id = ?1 AND id = ?2",
                        params![team.as_slice(), project.as_slice(), seq],
                    )?;
                    tx.execute("UPDATE teams SET stored = stored + ?2 WHERE id = ?1", params![team.as_slice(), size])?;
                    Entry {
                        seq: seq as u64,
                        kind: Kind::Message,
                        epoch: None,
                        device: entry.device.to_vec(),
                        at: entry.at,
                        payload: entry.payload,
                    }
                }
            };
            tx.commit()?;
            Ok(out)
        })
        .await
    }

    async fn read(
        &self,
        team: Id,
        stream: Stream,
        since: u64,
        page: Page,
    ) -> Result<(Vec<Entry>, bool)> {
        self.run(move |c| {
            Ok(paginate(
                read_entries(c, &team, stream, since, page, false)?,
                page,
            ))
        })
        .await
    }

    async fn reserve_file(
        &self,
        team: Id,
        file: Id,
        size: u64,
        device: Id,
        now: i64,
        quotas: Quotas,
    ) -> Result<()> {
        self.run(move |c| {
            let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let exists = tx
                .query_row(
                    "SELECT 1 FROM files WHERE team_id = ?1 AND id = ?2",
                    params![team.as_slice(), file.as_slice()],
                    |_| Ok(()),
                )
                .optional()?
                .is_some();
            if exists {
                return Err(StoreError::Exists);
            }
            let stored: i64 = tx.query_row(
                "SELECT stored FROM teams WHERE id = ?1",
                [team.as_slice()],
                |r| r.get(0),
            )?;
            if stored as u64 + size > quotas.team_bytes {
                return Err(StoreError::Quota);
            }
            tx.execute(
                "INSERT INTO files (team_id, id, size, device_id, at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    team.as_slice(),
                    file.as_slice(),
                    size as i64,
                    device.as_slice(),
                    now
                ],
            )?;
            tx.execute(
                "UPDATE teams SET stored = stored + ?2 WHERE id = ?1",
                params![team.as_slice(), size as i64],
            )?;
            tx.commit()?;
            Ok(())
        })
        .await
    }

    async fn file_size(&self, team: Id, file: Id) -> Result<Option<u64>> {
        self.run(move |c| {
            Ok(c.query_row(
                "SELECT size FROM files WHERE team_id = ?1 AND id = ?2",
                params![team.as_slice(), file.as_slice()],
                |r| r.get::<_, i64>(0),
            )
            .optional()?
            .map(|s| s as u64))
        })
        .await
    }

    async fn delete_file(&self, team: Id, file: Id) -> Result<Option<u64>> {
        self.run(move |c| {
            let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let size: Option<i64> = tx
                .query_row(
                    "SELECT size FROM files WHERE team_id = ?1 AND id = ?2",
                    params![team.as_slice(), file.as_slice()],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(size) = size {
                tx.execute(
                    "DELETE FROM files WHERE team_id = ?1 AND id = ?2",
                    params![team.as_slice(), file.as_slice()],
                )?;
                tx.execute(
                    "UPDATE teams SET stored = stored - ?2 WHERE id = ?1",
                    params![team.as_slice(), size],
                )?;
            }
            tx.commit()?;
            Ok(size.map(|s| s as u64))
        })
        .await
    }

    async fn used_bytes(&self, team: Id) -> Result<u64> {
        self.run(move |c| {
            Ok(c.query_row(
                "SELECT stored FROM teams WHERE id = ?1",
                [team.as_slice()],
                |r| r.get::<_, i64>(0),
            )
            .optional()?
            .unwrap_or(0) as u64)
        })
        .await
    }

    async fn counts(&self) -> Result<Counts> {
        self.run(|c| {
            let teams: i64 = c.query_row("SELECT COUNT(*) FROM teams", [], |r| r.get(0))?;
            let (devices, revoked): (i64, i64) =
                c.query_row("SELECT COUNT(*), COUNT(revoked_at) FROM devices", [], |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })?;
            Ok(Counts {
                teams: teams as u64,
                devices: devices as u64,
                revoked_devices: revoked as u64,
            })
        })
        .await
    }
}
