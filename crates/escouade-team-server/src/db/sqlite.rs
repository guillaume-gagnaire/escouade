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
    stored INTEGER NOT NULL DEFAULT 0,
    reserved INTEGER NOT NULL DEFAULT 0
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
    voided INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (team_id, seq)
);
CREATE TABLE IF NOT EXISTS projects (
    team_id BLOB NOT NULL,
    id BLOB NOT NULL,
    seq INTEGER NOT NULL DEFAULT 0,
    commits INTEGER NOT NULL DEFAULT 0,
    reserved INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (team_id, id)
);
CREATE TABLE IF NOT EXISTS project_entries (
    team_id BLOB NOT NULL,
    project_id BLOB NOT NULL,
    seq INTEGER NOT NULL,
    kind INTEGER NOT NULL DEFAULT 0,
    epoch INTEGER,
    device_id BLOB NOT NULL,
    at INTEGER NOT NULL,
    payload BLOB NOT NULL,
    voided INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (team_id, project_id, seq)
);
CREATE TABLE IF NOT EXISTS snapshots (
    team_id BLOB NOT NULL,
    project_id BLOB NOT NULL,
    upto INTEGER NOT NULL,
    device_id BLOB NOT NULL,
    at INTEGER NOT NULL,
    payload BLOB NOT NULL,
    flagged_by BLOB,
    flagged_at INTEGER,
    PRIMARY KEY (team_id, project_id, upto)
);
CREATE TABLE IF NOT EXISTS files (
    team_id BLOB NOT NULL,
    id BLOB NOT NULL,
    size INTEGER NOT NULL,
    device_id BLOB NOT NULL,
    at INTEGER NOT NULL,
    project_id BLOB,
    PRIMARY KEY (team_id, id)
);
CREATE TABLE IF NOT EXISTS project_access (
    team_id BLOB NOT NULL,
    project_id BLOB NOT NULL,
    everyone INTEGER NOT NULL,
    devices BLOB NOT NULL,
    PRIMARY KEY (team_id, project_id)
);
CREATE TABLE IF NOT EXISTS claims (
    team_id BLOB NOT NULL,
    project_id BLOB NOT NULL,
    ticket_id BLOB NOT NULL,
    holder BLOB,
    until INTEGER NOT NULL,
    token INTEGER NOT NULL,
    version INTEGER NOT NULL,
    claimed_at INTEGER NOT NULL,
    renewed_at INTEGER NOT NULL,
    last_holder BLOB,
    handed_by BLOB,
    PRIMARY KEY (team_id, project_id, ticket_id)
);
CREATE TABLE IF NOT EXISTS project_keys (
    team_id BLOB NOT NULL,
    project_id BLOB NOT NULL,
    n INTEGER NOT NULL,
    PRIMARY KEY (team_id, project_id)
);
CREATE TABLE IF NOT EXISTS dropped_entries (
    team_id BLOB NOT NULL,
    project_id BLOB,
    seq INTEGER NOT NULL,
    kind INTEGER NOT NULL,
    epoch INTEGER,
    device_id BLOB NOT NULL,
    written_at INTEGER NOT NULL,
    size INTEGER NOT NULL,
    digest BLOB NOT NULL,
    operator TEXT NOT NULL,
    at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS dropped_team ON dropped_entries(team_id);
";

const CLAIM_COLUMNS: &str =
    "ticket_id, holder, until, token, version, claimed_at, renewed_at, last_holder, handed_by";

fn claim_row(r: &rusqlite::Row) -> rusqlite::Result<Claim> {
    let id = |v: Option<Vec<u8>>| v.and_then(|v| <Id>::try_from(v).ok());
    Ok(Claim {
        ticket: r.get(0)?,
        holder: id(r.get(1)?),
        until: r.get(2)?,
        token: r.get::<_, i64>(3)? as u64,
        version: r.get::<_, i64>(4)? as u64,
        claimed_at: r.get(5)?,
        renewed_at: r.get(6)?,
        last_holder: id(r.get(7)?),
        handed_by: id(r.get(8)?),
    })
}

fn claim_of(c: &Connection, team: &Id, project: &Id, ticket: &[u8]) -> Result<Option<Claim>> {
    Ok(c.query_row(
        &format!(
            "SELECT {CLAIM_COLUMNS} FROM claims WHERE team_id = ?1 AND project_id = ?2 AND ticket_id = ?3"
        ),
        params![team.as_slice(), project.as_slice(), ticket],
        claim_row,
    )
    .optional()?)
}

fn put_claim(c: &Connection, team: &Id, project: &Id, claim: &Claim) -> Result<()> {
    c.execute(
        "INSERT INTO claims (team_id, project_id, ticket_id, holder, until, token, version, claimed_at, renewed_at, last_holder, handed_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT (team_id, project_id, ticket_id) DO UPDATE SET holder = excluded.holder, until = excluded.until,
           token = excluded.token, version = excluded.version, claimed_at = excluded.claimed_at,
           renewed_at = excluded.renewed_at, last_holder = excluded.last_holder, handed_by = excluded.handed_by",
        params![
            team.as_slice(),
            project.as_slice(),
            claim.ticket,
            claim.holder.as_ref().map(|h| h.as_slice()),
            claim.until,
            claim.token as i64,
            claim.version as i64,
            claim.claimed_at,
            claim.renewed_at,
            claim.last_holder.as_ref().map(|h| h.as_slice()),
            claim.handed_by.as_ref().map(|h| h.as_slice())
        ],
    )?;
    Ok(())
}

fn access_of(c: &Connection, team: &Id, project: &Id) -> Result<Option<Access>> {
    Ok(c.query_row(
        "SELECT everyone, devices FROM project_access WHERE team_id = ?1 AND project_id = ?2",
        params![team.as_slice(), project.as_slice()],
        |r| {
            Ok(Access::from_parts(
                r.get::<_, i64>(0)? != 0,
                &r.get::<_, Vec<u8>>(1)?,
            ))
        },
    )
    .optional()?)
}

/// Whether a holder still holds: a device of the team not revoked, with access to the project.
fn holder_ok(c: &Connection, team: &Id, project: &Id, holder: Option<Id>) -> Result<bool> {
    let Some(holder) = holder else {
        return Ok(true);
    };
    let revoked: Option<Option<i64>> = c
        .query_row(
            "SELECT revoked_at FROM devices WHERE id = ?1 AND team_id = ?2",
            params![holder.as_slice(), team.as_slice()],
            |r| r.get(0),
        )
        .optional()?;
    if !matches!(revoked, Some(None)) {
        return Ok(false);
    }
    Ok(access_of(c, team, project)?.is_none_or(|a| a.admits(&holder)))
}

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
                "SELECT seq, kind, epoch, device_id, at, payload FROM project_entries
                 WHERE team_id = ?1 AND project_id = ?2 AND seq {op} ?3 ORDER BY seq LIMIT ?4"
            ))?;
            let rows = stmt.query_map(
                params![team.as_slice(), project.as_slice(), after as i64, limit],
                |r| {
                    Ok(Entry {
                        seq: r.get::<_, i64>(0)? as u64,
                        kind: Kind::from_code(r.get(1)?),
                        epoch: r.get::<_, Option<i64>>(2)?.map(|e| e as u64),
                        device: r.get(3)?,
                        at: r.get(4)?,
                        payload: r.get(5)?,
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
            // A base made before the projects' own groups (TS-5): their commits' columns added.
            for (table, column, kind) in [
                ("projects", "commits", "INTEGER NOT NULL DEFAULT 0"),
                ("project_entries", "kind", "INTEGER NOT NULL DEFAULT 0"),
                ("project_entries", "epoch", "INTEGER"),
                // A base made before a ticket could be handed (TS-17).
                ("claims", "handed_by", "BLOB"),
                // A base made before the retention of the snapshots (SR2-01): the number the messages are gone up to.
                ("projects", "pruned", "INTEGER NOT NULL DEFAULT 0"),
                // A base made before the files were attached to a project (SR2-03): its files attached to nothing,
                // their author's and the administrators' alone until one of them attaches each.
                ("files", "project_id", "BLOB"),
                // A base made before the voids (SR2-02): the commits declared null, the epochs kept to the administrators.
                ("teams", "reserved", "INTEGER NOT NULL DEFAULT 0"),
                ("team_entries", "voided", "INTEGER NOT NULL DEFAULT 0"),
                ("projects", "reserved", "INTEGER NOT NULL DEFAULT 0"),
                ("project_entries", "voided", "INTEGER NOT NULL DEFAULT 0"),
            ] {
                let has: bool = c
                    .prepare(&format!("SELECT 1 FROM pragma_table_info('{table}') WHERE name = ?1"))?
                    .exists([column])?;
                if !has {
                    c.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {kind}"))?;
                }
            }
            // One commit holds an epoch, the ones declared null aside (SR2-02).
            c.execute_batch(
                "DROP INDEX IF EXISTS team_commits;
                 DROP INDEX IF EXISTS project_commits;
                 CREATE UNIQUE INDEX IF NOT EXISTS team_live_commits ON team_entries(team_id, epoch)
                   WHERE kind = 1 AND voided = 0;
                 CREATE UNIQUE INDEX IF NOT EXISTS project_live_commits ON project_entries(team_id, project_id, epoch)
                   WHERE kind = 1 AND voided = 0;",
            )?;
            // A base made before the retention (SR2-01): one snapshot per project, its messages already forgotten.
            let old: bool = c
                .prepare("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'project_snapshots'")?
                .exists([])?;
            if old {
                let tx = c.transaction()?;
                tx.execute_batch(
                    "INSERT OR IGNORE INTO snapshots (team_id, project_id, upto, device_id, at, payload)
                       SELECT team_id, project_id, upto, device_id, at, payload FROM project_snapshots;
                     UPDATE projects SET pruned = (SELECT s.upto FROM project_snapshots s
                       WHERE s.team_id = projects.team_id AND s.project_id = projects.id)
                     WHERE EXISTS (SELECT 1 FROM project_snapshots s
                       WHERE s.team_id = projects.team_id AND s.project_id = projects.id);
                     DROP TABLE project_snapshots;",
                )?;
                tx.commit()?;
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
            if Role::parse(&current) == Role::Admin && role != Role::Admin && revoked.is_none() {
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
            let (seq, commits, stored, reserved): (i64, i64, i64, bool) = tx.query_row(
                "SELECT seq, commits, stored, reserved FROM teams WHERE id = ?1",
                [team.as_slice()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get::<_, i64>(3)? != 0)),
            )?;
            let size = entry.payload.len() as i64;
            if (stored + size) as u64 > quotas.team_bytes {
                return Err(StoreError::Quota);
            }
            let out = match stream {
                Stream::Team => {
                    let is_commit = entry.kind == Kind::Commit;
                    if entry.kind == Kind::Void {
                        let asked = entry.epoch.unwrap_or(u64::MAX);
                        if commits == 0 || asked != commits as u64 - 1 {
                            return Err(StoreError::Void("it is not for the epoch of the last commit"));
                        }
                        let n = tx.execute(
                            "UPDATE team_entries SET voided = 1 WHERE team_id = ?1 AND kind = 1 AND epoch = ?2 AND voided = 0",
                            params![team.as_slice(), asked as i64],
                        )?;
                        if n == 0 {
                            return Err(StoreError::Void("the last commit is declared null already"));
                        }
                    }
                    if is_commit {
                        let asked = entry.epoch.unwrap_or(u64::MAX);
                        if asked == commits as u64 && reserved && !entry.by_admin {
                            return Err(StoreError::Reserved);
                        }
                        if asked != commits as u64 {
                            let from: Option<i64> = tx
                                .query_row(
                                    "SELECT seq FROM team_entries WHERE team_id = ?1 AND kind = 1 AND epoch = ?2 AND voided = 0",
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
                            entry.epoch.filter(|_| entry.kind != Kind::Message).map(|e| e as i64),
                            entry.device.as_slice(),
                            entry.at,
                            entry.payload
                        ],
                    )?;
                    let (step, reserved) = match entry.kind {
                        Kind::Commit => (1, false),
                        Kind::Void => (-1, true),
                        Kind::Message => (0, reserved),
                    };
                    tx.execute(
                        "UPDATE teams SET seq = ?2, commits = commits + ?3, stored = stored + ?4, reserved = ?5 WHERE id = ?1",
                        params![team.as_slice(), seq, step, size, reserved as i64],
                    )?;
                    Entry {
                        seq: seq as u64,
                        kind: entry.kind,
                        epoch: entry.epoch.filter(|_| entry.kind != Kind::Message),
                        device: entry.device.to_vec(),
                        at: entry.at,
                        payload: entry.payload,
                    }
                }
                Stream::Project(project) => {
                    let current: Option<(i64, i64, bool)> = tx
                        .query_row(
                            "SELECT seq, commits, reserved FROM projects WHERE team_id = ?1 AND id = ?2",
                            params![team.as_slice(), project.as_slice()],
                            |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? != 0)),
                        )
                        .optional()?;
                    let is_commit = entry.kind == Kind::Commit;
                    // A holder's write passes under the claim's current token alone (TS-6).
                    if let Some(fence) = &entry.fence {
                        let claim = claim_of(&tx, &team, &project, &fence.ticket)?;
                        let moved = fenced(claim, fence, entry.device)?;
                        put_claim(&tx, &team, &project, &moved)?;
                    }
                    let project_commits = current.map_or(0, |c| c.1);
                    let reserved = current.is_some_and(|c| c.2);
                    if entry.kind == Kind::Void {
                        let asked = entry.epoch.unwrap_or(u64::MAX);
                        if project_commits == 0 || asked != project_commits as u64 - 1 {
                            return Err(StoreError::Void("it is not for the epoch of the last commit"));
                        }
                        let n = tx.execute(
                            "UPDATE project_entries SET voided = 1
                             WHERE team_id = ?1 AND project_id = ?2 AND kind = 1 AND epoch = ?3 AND voided = 0",
                            params![team.as_slice(), project.as_slice(), asked as i64],
                        )?;
                        if n == 0 {
                            return Err(StoreError::Void("the last commit is declared null already"));
                        }
                    }
                    if is_commit {
                        let asked = entry.epoch.unwrap_or(u64::MAX);
                        if asked == project_commits as u64 && reserved && !entry.by_admin {
                            return Err(StoreError::Reserved);
                        }
                        if asked != project_commits as u64 {
                            let from: Option<i64> = tx
                                .query_row(
                                    "SELECT seq FROM project_entries
                                     WHERE team_id = ?1 AND project_id = ?2 AND kind = 1 AND epoch = ?3 AND voided = 0",
                                    params![team.as_slice(), project.as_slice(), asked.min(i64::MAX as u64) as i64],
                                    |r| r.get(0),
                                )
                                .optional()?;
                            let preceding = match from {
                                Some(from) => paginate(read_entries(&tx, &team, stream, from as u64, page, true)?, page).0,
                                None => Vec::new(),
                            };
                            return Err(StoreError::EpochTaken {
                                asked,
                                expected: project_commits as u64,
                                preceding,
                            });
                        }
                    }
                    let current = match current.map(|c| c.0) {
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
                    let epoch = entry.epoch.filter(|_| entry.kind != Kind::Message);
                    tx.execute(
                        "INSERT INTO project_entries (team_id, project_id, seq, kind, epoch, device_id, at, payload)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                        params![
                            team.as_slice(),
                            project.as_slice(),
                            seq,
                            entry.kind.code(),
                            epoch.map(|e| e as i64),
                            entry.device.as_slice(),
                            entry.at,
                            entry.payload
                        ],
                    )?;
                    let (step, reserved) = match entry.kind {
                        Kind::Commit => (1, false),
                        Kind::Void => (-1, true),
                        Kind::Message => (0, reserved),
                    };
                    tx.execute(
                        "UPDATE projects SET seq = ?3, commits = commits + ?4, reserved = ?5 WHERE team_id = ?1 AND id = ?2",
                        params![team.as_slice(), project.as_slice(), seq, step, reserved as i64],
                    )?;
                    tx.execute("UPDATE teams SET stored = stored + ?2 WHERE id = ?1", params![team.as_slice(), size])?;
                    Entry {
                        seq: seq as u64,
                        kind: entry.kind,
                        epoch,
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

    async fn drop_entry(
        &self,
        team: Id,
        stream: Stream,
        seq: u64,
        operator: String,
        now: i64,
    ) -> Result<Option<Dropped>> {
        self.run(move |c| {
            let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let commits: Option<i64> = tx
                .query_row("SELECT commits FROM teams WHERE id = ?1", [team.as_slice()], |r| r.get(0))
                .optional()?;
            let Some(team_commits) = commits else {
                return Err(StoreError::Drop("the base holds no such team"));
            };
            let page = Page { entries: 1, bytes: usize::MAX };
            let Some(entry) = read_entries(&tx, &team, stream, seq, page, true)?
                .into_iter()
                .next()
                .filter(|e| e.seq == seq)
            else {
                return Ok(None);
            };
            let is_commit = entry.kind == Kind::Commit;
            let project = match stream {
                Stream::Team => None,
                Stream::Project(p) => Some(p),
            };
            // A void and the commit it declared null stay together (SR2-02): the devices read on past them already.
            if entry.kind == Kind::Void {
                return Err(StoreError::Drop("a void stays: the devices read on past it already"));
            }
            if is_commit {
                let voided: i64 = match project {
                    None => tx.query_row(
                        "SELECT voided FROM team_entries WHERE team_id = ?1 AND seq = ?2",
                        params![team.as_slice(), seq as i64],
                        |r| r.get(0),
                    )?,
                    Some(p) => tx.query_row(
                        "SELECT voided FROM project_entries WHERE team_id = ?1 AND project_id = ?2 AND seq = ?3",
                        params![team.as_slice(), p.as_slice(), seq as i64],
                        |r| r.get(0),
                    )?,
                };
                if voided != 0 {
                    return Err(StoreError::Drop("a commit declared null stays: the devices read on past it already"));
                }
                let stream_commits = match project {
                    None => team_commits,
                    Some(p) => tx.query_row(
                        "SELECT commits FROM projects WHERE team_id = ?1 AND id = ?2",
                        params![team.as_slice(), p.as_slice()],
                        |r| r.get(0),
                    )?,
                };
                if entry.epoch.map(|e| e as i64 + 1) != Some(stream_commits) {
                    return Err(StoreError::Drop(
                        "a later commit follows this one: remove the later ones first",
                    ));
                }
            }
            let size = entry.payload.len() as i64;
            match project {
                None => {
                    tx.execute(
                        "DELETE FROM team_entries WHERE team_id = ?1 AND seq = ?2",
                        params![team.as_slice(), seq as i64],
                    )?;
                    tx.execute(
                        "UPDATE teams SET commits = commits - ?2 WHERE id = ?1",
                        params![team.as_slice(), is_commit as i64],
                    )?;
                }
                Some(p) => {
                    tx.execute(
                        "DELETE FROM project_entries WHERE team_id = ?1 AND project_id = ?2 AND seq = ?3",
                        params![team.as_slice(), p.as_slice(), seq as i64],
                    )?;
                    tx.execute(
                        "UPDATE projects SET commits = commits - ?3 WHERE team_id = ?1 AND id = ?2",
                        params![team.as_slice(), p.as_slice(), is_commit as i64],
                    )?;
                }
            }
            tx.execute(
                "UPDATE teams SET stored = MAX(stored - ?2, 0) WHERE id = ?1",
                params![team.as_slice(), size],
            )?;
            let dropped = Dropped {
                project,
                seq,
                kind: entry.kind,
                epoch: entry.epoch,
                device: id_of(entry.device.clone())?,
                written_at: entry.at,
                size: size as u64,
                digest: entry_digest(&entry),
                operator,
                at: now,
            };
            tx.execute(
                "INSERT INTO dropped_entries
                 (team_id, project_id, seq, kind, epoch, device_id, written_at, size, digest, operator, at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    team.as_slice(),
                    dropped.project.as_ref().map(|p| p.as_slice()),
                    seq as i64,
                    dropped.kind.code(),
                    dropped.epoch.map(|e| e as i64),
                    dropped.device.as_slice(),
                    dropped.written_at,
                    size,
                    dropped.digest.as_slice(),
                    dropped.operator,
                    now
                ],
            )?;
            tx.commit()?;
            Ok(Some(dropped))
        })
        .await
    }

    async fn dropped(&self, team: Id) -> Result<Vec<Dropped>> {
        self.run(move |c| {
            let mut stmt = c.prepare_cached(
                "SELECT project_id, seq, kind, epoch, device_id, written_at, size, digest, operator, at
                 FROM dropped_entries WHERE team_id = ?1 ORDER BY at, rowid",
            )?;
            type Row = (Option<Vec<u8>>, i64, i64, Option<i64>, Vec<u8>, i64, i64, Vec<u8>, String, i64);
            let rows = stmt.query_map([team.as_slice()], |r| -> rusqlite::Result<Row> {
                Ok((
                    r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?,
                    r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?,
                ))
            })?;
            let mut out = Vec::new();
            for row in rows {
                let (project, seq, kind, epoch, device, written_at, size, digest, operator, at) = row?;
                out.push(Dropped {
                    project: project.map(id_of).transpose()?,
                    seq: seq as u64,
                    kind: Kind::from_code(kind),
                    epoch: epoch.map(|e| e as u64),
                    device: id_of(device)?,
                    written_at,
                    size: size as u64,
                    digest: digest.try_into().map_err(|_| backend("a digest of the wrong length"))?,
                    operator,
                    at,
                });
            }
            Ok(out)
        })
        .await
    }

    async fn put_snapshot(
        &self,
        team: Id,
        project: Id,
        snapshot: Snapshot,
        quotas: Quotas,
    ) -> Result<()> {
        self.run(move |c| {
            let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let stored: i64 = tx.query_row(
                "SELECT stored FROM teams WHERE id = ?1",
                [team.as_slice()],
                |r| r.get(0),
            )?;
            let seq: Option<i64> = tx
                .query_row(
                    "SELECT seq FROM projects WHERE team_id = ?1 AND id = ?2",
                    params![team.as_slice(), project.as_slice()],
                    |r| r.get(0),
                )
                .optional()?;
            if seq.is_none_or(|s| (s as u64) < snapshot.upto) || snapshot.upto == 0 {
                return Err(StoreError::Snapshot("beyond the project's stream"));
            }
            let last: Option<i64> = tx.query_row(
                "SELECT MAX(upto) FROM snapshots WHERE team_id = ?1 AND project_id = ?2",
                params![team.as_slice(), project.as_slice()],
                |r| r.get(0),
            )?;
            if last.is_some_and(|upto| upto as u64 >= snapshot.upto) {
                return Err(StoreError::Snapshot("not after the one kept"));
            }
            let size = snapshot.payload.len() as i64;
            if (stored + size) as u64 > quotas.team_bytes {
                return Err(StoreError::Quota);
            }
            tx.execute(
                "INSERT INTO snapshots (team_id, project_id, upto, device_id, at, payload) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    team.as_slice(),
                    project.as_slice(),
                    snapshot.upto as i64,
                    snapshot.device.as_slice(),
                    snapshot.at,
                    snapshot.payload
                ],
            )?;
            tx.execute(
                "UPDATE teams SET stored = stored + ?2 WHERE id = ?1",
                params![team.as_slice(), size],
            )?;
            tx.commit()?;
            Ok(())
        })
        .await
    }

    async fn snapshot(
        &self,
        team: Id,
        project: Id,
        before: Option<u64>,
    ) -> Result<Option<Snapshot>> {
        self.run(move |c| {
            let before = before.map_or(i64::MAX, |b| b.min(i64::MAX as u64) as i64);
            let row: Option<(i64, Vec<u8>, i64, Vec<u8>)> = c
                .query_row(
                    "SELECT upto, device_id, at, payload FROM snapshots
                     WHERE team_id = ?1 AND project_id = ?2 AND upto < ?3 ORDER BY upto DESC LIMIT 1",
                    params![team.as_slice(), project.as_slice(), before],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .optional()?;
            row.map(|(upto, device, at, payload)| {
                Ok(Snapshot {
                    upto: upto as u64,
                    device: id_of(device)?,
                    at,
                    payload,
                })
            })
            .transpose()
        })
        .await
    }

    async fn snapshot_upto(&self, team: Id, project: Id) -> Result<Option<u64>> {
        self.run(move |c| {
            Ok(c.query_row(
                "SELECT MAX(upto) FROM snapshots WHERE team_id = ?1 AND project_id = ?2",
                params![team.as_slice(), project.as_slice()],
                |r| r.get::<_, Option<i64>>(0),
            )?
            .map(|u| u as u64))
        })
        .await
    }

    async fn flag_snapshot(
        &self,
        team: Id,
        project: Id,
        upto: u64,
        device: Id,
        now: i64,
    ) -> Result<bool> {
        self.run(move |c| {
            let upto = upto.min(i64::MAX as u64) as i64;
            let found = c.execute(
                "UPDATE snapshots SET flagged_by = COALESCE(flagged_by, ?4), flagged_at = COALESCE(flagged_at, ?5)
                 WHERE team_id = ?1 AND project_id = ?2 AND upto = ?3",
                params![team.as_slice(), project.as_slice(), upto, device.as_slice(), now],
            )?;
            Ok(found > 0)
        })
        .await
    }

    async fn remove_snapshot(&self, team: Id, project: Id, upto: u64) -> Result<bool> {
        self.run(move |c| {
            let upto = upto.min(i64::MAX as u64) as i64;
            let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let size: Option<i64> = tx
                .query_row(
                    "SELECT length(payload) FROM snapshots WHERE team_id = ?1 AND project_id = ?2 AND upto = ?3",
                    params![team.as_slice(), project.as_slice(), upto],
                    |r| r.get(0),
                )
                .optional()?;
            let Some(size) = size else {
                return Ok(false);
            };
            let pruned: i64 = tx
                .query_row(
                    "SELECT pruned FROM projects WHERE team_id = ?1 AND id = ?2",
                    params![team.as_slice(), project.as_slice()],
                    |r| r.get(0),
                )
                .optional()?
                .unwrap_or(0);
            if upto <= pruned {
                return Err(StoreError::Snapshot(
                    "the stream was pruned to it: its messages are gone",
                ));
            }
            tx.execute(
                "DELETE FROM snapshots WHERE team_id = ?1 AND project_id = ?2 AND upto = ?3",
                params![team.as_slice(), project.as_slice(), upto],
            )?;
            tx.execute(
                "UPDATE teams SET stored = stored - ?2 WHERE id = ?1",
                params![team.as_slice(), size],
            )?;
            tx.commit()?;
            Ok(true)
        })
        .await
    }

    async fn unreadable_snapshots(&self, team: Id) -> Result<Vec<Unreadable>> {
        self.run(move |c| {
            let mut stmt = c.prepare_cached(
                "SELECT project_id, upto, device_id, at, flagged_by, flagged_at FROM snapshots
                 WHERE team_id = ?1 AND flagged_at IS NOT NULL ORDER BY flagged_at, project_id, upto",
            )?;
            let rows = stmt.query_map([team.as_slice()], |r| {
                Ok((
                    r.get::<_, Vec<u8>>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, Vec<u8>>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, Vec<u8>>(4)?,
                    r.get::<_, i64>(5)?,
                ))
            })?;
            let mut out = Vec::new();
            for row in rows {
                let (project, upto, device, at, flagged_by, flagged_at) = row?;
                out.push(Unreadable {
                    project: id_of(project)?,
                    upto: upto as u64,
                    device: id_of(device)?,
                    at,
                    flagged_by: id_of(flagged_by)?,
                    flagged_at,
                });
            }
            Ok(out)
        })
        .await
    }

    async fn prune_snapshots(&self, cut: i64) -> Result<u64> {
        self.run(move |c| {
            // For each project, its last snapshot old enough that nobody flagged, when it is past what is pruned.
            let due: Vec<(Vec<u8>, Vec<u8>, i64)> = {
                let mut stmt = c.prepare_cached(
                    "SELECT s.team_id, s.project_id, MAX(s.upto) FROM snapshots s
                     JOIN projects p ON p.team_id = s.team_id AND p.id = s.project_id
                     WHERE s.at <= ?1 AND s.flagged_at IS NULL AND s.upto > p.pruned
                     GROUP BY s.team_id, s.project_id",
                )?;
                let rows = stmt.query_map([cut], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
                rows.collect::<rusqlite::Result<_>>()?
            };
            let mut forgotten = 0u64;
            for (team, project, upto) in due {
                let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
                let (messages, freed): (i64, i64) = tx.query_row(
                    "SELECT COUNT(*), COALESCE(SUM(length(payload)), 0) FROM project_entries
                     WHERE team_id = ?1 AND project_id = ?2 AND kind = 0 AND seq <= ?3",
                    params![team, project, upto],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )?;
                let older: i64 = tx.query_row(
                    "SELECT COALESCE(SUM(length(payload)), 0) FROM snapshots
                     WHERE team_id = ?1 AND project_id = ?2 AND upto < ?3",
                    params![team, project, upto],
                    |r| r.get(0),
                )?;
                tx.execute(
                    "DELETE FROM project_entries WHERE team_id = ?1 AND project_id = ?2 AND kind = 0 AND seq <= ?3",
                    params![team, project, upto],
                )?;
                tx.execute(
                    "DELETE FROM snapshots WHERE team_id = ?1 AND project_id = ?2 AND upto < ?3",
                    params![team, project, upto],
                )?;
                tx.execute(
                    "UPDATE projects SET pruned = ?3 WHERE team_id = ?1 AND id = ?2",
                    params![team, project, upto],
                )?;
                tx.execute(
                    "UPDATE teams SET stored = stored - ?2 WHERE id = ?1",
                    params![team, freed + older],
                )?;
                tx.commit()?;
                forgotten += messages as u64;
            }
            Ok(forgotten)
        })
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn reserve_file(
        &self,
        team: Id,
        file: Id,
        project: Option<Id>,
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
                "INSERT INTO files (team_id, id, size, device_id, at, project_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    team.as_slice(),
                    file.as_slice(),
                    size as i64,
                    device.as_slice(),
                    now,
                    project.as_ref().map(|p| p.as_slice())
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

    async fn file(&self, team: Id, file: Id) -> Result<Option<StoredFile>> {
        self.run(move |c| {
            let row = c
                .query_row(
                    "SELECT size, device_id, project_id FROM files WHERE team_id = ?1 AND id = ?2",
                    params![team.as_slice(), file.as_slice()],
                    |r| {
                        Ok((
                            r.get::<_, i64>(0)?,
                            r.get::<_, Vec<u8>>(1)?,
                            r.get::<_, Option<Vec<u8>>>(2)?,
                        ))
                    },
                )
                .optional()?;
            row.map(|(size, device, project)| {
                Ok(StoredFile {
                    size: size as u64,
                    device: id_of(device)?,
                    project: project.map(id_of).transpose()?,
                })
            })
            .transpose()
        })
        .await
    }

    async fn attach_file(&self, team: Id, file: Id, project: Id) -> Result<bool> {
        self.run(move |c| {
            Ok(c.execute(
                "UPDATE files SET project_id = ?3 WHERE team_id = ?1 AND id = ?2 AND project_id IS NULL",
                params![team.as_slice(), file.as_slice(), project.as_slice()],
            )? == 1)
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

    async fn access(&self, team: Id, project: Id) -> Result<Option<Access>> {
        self.run(move |c| access_of(c, &team, &project)).await
    }

    async fn set_access(&self, team: Id, project: Id, access: Access, admin: bool) -> Result<()> {
        self.run(move |c| {
            let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
            if access_of(&tx, &team, &project)?.is_some() && !admin {
                return Err(StoreError::NotAdmin);
            }
            tx.execute(
                "INSERT INTO project_access (team_id, project_id, everyone, devices) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (team_id, project_id) DO UPDATE SET everyone = excluded.everyone, devices = excluded.devices",
                params![team.as_slice(), project.as_slice(), access.everyone as i64, access.devices_bytes()],
            )?;
            tx.commit()?;
            Ok(())
        })
        .await
    }

    async fn open_access(&self, team: Id, project: Id) -> Result<()> {
        self.run(move |c| {
            c.execute(
                "INSERT OR IGNORE INTO project_access (team_id, project_id, everyone, devices) VALUES (?1, ?2, 1, x'')",
                params![team.as_slice(), project.as_slice()],
            )?;
            Ok(())
        })
        .await
    }

    async fn claim(
        &self,
        team: Id,
        project: Id,
        ticket: Id,
        device: Id,
        ask: ClaimAsk,
        now: i64,
        lease: i64,
    ) -> Result<Claim> {
        self.run(move |c| {
            let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let current = claim_of(&tx, &team, &project, &ticket)?;
            if current.is_none() {
                let n: i64 = tx.query_row(
                    "SELECT COUNT(*) FROM claims WHERE team_id = ?1 AND project_id = ?2",
                    params![team.as_slice(), project.as_slice()],
                    |r| r.get(0),
                )?;
                if n >= MAX_CLAIMS {
                    return Err(StoreError::TooMany("claims"));
                }
            }
            let ok = holder_ok(
                &tx,
                &team,
                &project,
                current.as_ref().and_then(|c| c.holder),
            )?;
            let to_ok = match ask {
                ClaimAsk::Hand { to, .. } => holder_ok(&tx, &team, &project, Some(to))?,
                _ => true,
            };
            let claim = decide_with(current, ticket, device, ask, now, lease, ok, to_ok)?;
            put_claim(&tx, &team, &project, &claim)?;
            tx.commit()?;
            Ok(claim)
        })
        .await
    }

    async fn claims(&self, team: Id, project: Id) -> Result<Vec<Claim>> {
        self.run(move |c| {
            let mut stmt = c.prepare_cached(&format!(
                "SELECT {CLAIM_COLUMNS} FROM claims WHERE team_id = ?1 AND project_id = ?2 ORDER BY ticket_id"
            ))?;
            let rows = stmt.query_map(params![team.as_slice(), project.as_slice()], claim_row)?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })
        .await
    }

    async fn next_key(&self, team: Id, project: Id) -> Result<u64> {
        self.run(move |c| {
            let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute(
                "INSERT INTO project_keys (team_id, project_id, n) VALUES (?1, ?2, 1)
                 ON CONFLICT (team_id, project_id) DO UPDATE SET n = n + 1",
                params![team.as_slice(), project.as_slice()],
            )?;
            let n: i64 = tx.query_row(
                "SELECT n FROM project_keys WHERE team_id = ?1 AND project_id = ?2",
                params![team.as_slice(), project.as_slice()],
                |r| r.get(0),
            )?;
            tx.commit()?;
            Ok(n as u64)
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{ClaimAsk, Store, StoreError, MAX_CLAIMS};

    /// A base from before the retention (SR2-01) kept one snapshot per project, its messages already forgotten: the
    /// snapshot moves to the new table, the project says its stream is pruned to it (an administrator cannot remove
    /// it: nothing would take its place), and the old table goes.
    #[tokio::test]
    async fn a_base_from_before_the_retention_keeps_its_snapshots() {
        let dir = tempfile::tempdir().unwrap();
        let store = Sqlite::open(&dir.path().join("team.db")).unwrap();
        store.migrate().await.unwrap();
        let (team, project) = ([1u8; 16], [2u8; 16]);
        store
            .run(move |c| {
                c.execute_batch(
                    "CREATE TABLE project_snapshots (
                        team_id BLOB NOT NULL, project_id BLOB NOT NULL, upto INTEGER NOT NULL,
                        device_id BLOB NOT NULL, at INTEGER NOT NULL, payload BLOB NOT NULL,
                        PRIMARY KEY (team_id, project_id))",
                )?;
                c.execute(
                    "INSERT INTO teams (id, created_at, stored) VALUES (?1, 0, 3)",
                    [team.as_slice()],
                )?;
                c.execute(
                    "INSERT INTO projects (team_id, id, seq) VALUES (?1, ?2, 9)",
                    params![team.as_slice(), project.as_slice()],
                )?;
                c.execute(
                    "INSERT INTO project_snapshots VALUES (?1, ?2, 7, ?3, 5, X'010203')",
                    params![team.as_slice(), project.as_slice(), [3u8; 16].as_slice()],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        store.migrate().await.unwrap();
        let s = store.snapshot(team, project, None).await.unwrap().unwrap();
        assert_eq!(
            (s.upto, s.device, s.at, s.payload),
            (7, [3u8; 16], 5, vec![1, 2, 3])
        );
        assert!(matches!(
            store.remove_snapshot(team, project, 7).await,
            Err(StoreError::Snapshot(_))
        ));
        let gone: bool = store
            .run(|c| {
                Ok(
                    c.prepare("SELECT 1 FROM sqlite_master WHERE name = 'project_snapshots'")?
                        .exists([])?,
                )
            })
            .await
            .unwrap();
        assert!(!gone);
        // Again: nothing to do.
        store.migrate().await.unwrap();
    }

    /// A project holds `MAX_CLAIMS` claims at most (TS-18): one more ticket is refused, one held already is still
    /// taken again. The claims are written straight into the base, as the routes would take long to make so many.
    #[tokio::test]
    async fn a_project_holds_a_bounded_number_of_claims() {
        let dir = tempfile::tempdir().unwrap();
        let store = Sqlite::open(&dir.path().join("team.db")).unwrap();
        store.migrate().await.unwrap();
        let (team, project, device) = ([1u8; 16], [2u8; 16], [3u8; 16]);
        store
            .run(move |c| {
                let tx = c.transaction()?;
                {
                    let mut insert = tx.prepare(
                        "INSERT INTO claims (team_id, project_id, ticket_id, holder, until, token, version, claimed_at, renewed_at, last_holder)
                         VALUES (?1, ?2, ?3, NULL, 0, 1, 1, 0, 0, NULL)",
                    )?;
                    for i in 0..MAX_CLAIMS {
                        let mut ticket = [0u8; 16];
                        ticket[..8].copy_from_slice(&i.to_be_bytes());
                        insert.execute(params![team.as_slice(), project.as_slice(), ticket.as_slice()])?;
                    }
                }
                tx.commit()?;
                Ok(())
            })
            .await
            .unwrap();
        let take = ClaimAsk::Take { expected: None };
        let refused = store
            .claim(team, project, [9; 16], device, take, 10, 600)
            .await;
        assert!(
            matches!(refused, Err(StoreError::TooMany("claims"))),
            "{refused:?}"
        );
        // Another project has its own count.
        assert!(store
            .claim(team, [5; 16], [9; 16], device, take, 10, 600)
            .await
            .is_ok());
    }
}
