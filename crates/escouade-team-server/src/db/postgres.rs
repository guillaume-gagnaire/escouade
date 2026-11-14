//! PostgreSQL, by tokio-postgres: one connection, its writes in transactions that lock the team's row
//! (`SELECT … FOR UPDATE`), so the numbering and the first commit of an epoch hold with several servers too.
//! The connection is in clear (`NoTls`): the base sits beside the server (the compose's network, a socket, a VPN).

use async_trait::async_trait;
use tokio::sync::Mutex;
use tokio_postgres::{Client, NoTls};

use super::*;
use crate::util::random;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS teams (
    id BYTEA PRIMARY KEY,
    created_at BIGINT NOT NULL,
    seq BIGINT NOT NULL DEFAULT 0,
    commits BIGINT NOT NULL DEFAULT 0,
    stored BIGINT NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS devices (
    id BYTEA PRIMARY KEY,
    team_id BYTEA NOT NULL REFERENCES teams(id),
    public_key BYTEA NOT NULL UNIQUE,
    role TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    revoked_at BIGINT
);
CREATE INDEX IF NOT EXISTS devices_team ON devices(team_id);
CREATE TABLE IF NOT EXISTS tokens (
    hash BYTEA PRIMARY KEY,
    kind BIGINT NOT NULL,
    team_id BYTEA,
    created_at BIGINT NOT NULL,
    expires_at BIGINT
);
CREATE TABLE IF NOT EXISTS team_entries (
    team_id BYTEA NOT NULL,
    seq BIGINT NOT NULL,
    kind BIGINT NOT NULL,
    epoch BIGINT,
    device_id BYTEA NOT NULL,
    at BIGINT NOT NULL,
    payload BYTEA NOT NULL,
    PRIMARY KEY (team_id, seq)
);
CREATE UNIQUE INDEX IF NOT EXISTS team_commits ON team_entries(team_id, epoch) WHERE kind = 1;
CREATE TABLE IF NOT EXISTS projects (
    team_id BYTEA NOT NULL,
    id BYTEA NOT NULL,
    seq BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (team_id, id)
);
CREATE TABLE IF NOT EXISTS project_entries (
    team_id BYTEA NOT NULL,
    project_id BYTEA NOT NULL,
    seq BIGINT NOT NULL,
    device_id BYTEA NOT NULL,
    at BIGINT NOT NULL,
    payload BYTEA NOT NULL,
    PRIMARY KEY (team_id, project_id, seq)
);
CREATE TABLE IF NOT EXISTS files (
    team_id BYTEA NOT NULL,
    id BYTEA NOT NULL,
    size BIGINT NOT NULL,
    device_id BYTEA NOT NULL,
    at BIGINT NOT NULL,
    PRIMARY KEY (team_id, id)
);
";

pub struct Postgres {
    client: Mutex<Client>,
}

fn backend(e: impl std::fmt::Display) -> StoreError {
    StoreError::Backend(e.to_string())
}

impl From<tokio_postgres::Error> for StoreError {
    fn from(e: tokio_postgres::Error) -> Self {
        backend(e)
    }
}

fn id_of(v: Vec<u8>) -> Result<Id> {
    v.try_into()
        .map_err(|_| backend("an id of the wrong length"))
}

fn to_device(r: &tokio_postgres::Row) -> Result<Device> {
    Ok(Device {
        id: id_of(r.get(0))?,
        team: id_of(r.get(1))?,
        public_key: r
            .get::<_, Vec<u8>>(2)
            .try_into()
            .map_err(|_| backend("a key of the wrong length"))?,
        role: Role::parse(r.get(3)),
        created_at: r.get(4),
        revoked_at: r.get(5),
    })
}

const DEVICE_COLUMNS: &str = "id, team_id, public_key, role, created_at, revoked_at";

impl Postgres {
    /// Connects, and works in `schema` if one is given (made if missing).
    pub async fn connect(url: &str, schema: Option<&str>) -> Result<Self> {
        let (client, connection) = tokio_postgres::connect(url, NoTls).await?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                tracing::error!("postgres connection: {e}");
            }
        });
        if let Some(schema) = schema {
            if schema.is_empty()
                || !schema
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                return Err(backend("a schema is letters, digits and _"));
            }
            client
                .batch_execute(&format!(
                    "CREATE SCHEMA IF NOT EXISTS {schema}; SET search_path TO {schema}"
                ))
                .await?;
        }
        Ok(Postgres {
            client: Mutex::new(client),
        })
    }

    /// Drops a schema (the tests' cleanup).
    pub async fn drop_schema(&self, schema: &str) -> Result<()> {
        if !schema
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(backend("a schema is letters, digits and _"));
        }
        self.client
            .lock()
            .await
            .batch_execute(&format!("DROP SCHEMA IF EXISTS {schema} CASCADE"))
            .await?;
        Ok(())
    }
}

fn entry_of(r: &tokio_postgres::Row, project: bool) -> Entry {
    if project {
        Entry {
            seq: r.get::<_, i64>(0) as u64,
            kind: Kind::Message,
            epoch: None,
            device: r.get(1),
            at: r.get(2),
            payload: r.get(3),
        }
    } else {
        Entry {
            seq: r.get::<_, i64>(0) as u64,
            kind: Kind::from_code(r.get(1)),
            epoch: r.get::<_, Option<i64>>(2).map(|e| e as u64),
            device: r.get(3),
            at: r.get(4),
            payload: r.get(5),
        }
    }
}

async fn read_entries(
    c: &impl tokio_postgres::GenericClient,
    team: &Id,
    stream: Stream,
    after: u64,
    page: Page,
    from_seq: bool,
) -> Result<Vec<Entry>> {
    let op = if from_seq { ">=" } else { ">" };
    let limit = page.entries as i64 + 1;
    let rows = match stream {
        Stream::Team => {
            c.query(
                &format!(
                    "SELECT seq, kind, epoch, device_id, at, payload FROM team_entries
                     WHERE team_id = $1 AND seq {op} $2 ORDER BY seq LIMIT $3"
                ),
                &[&team.as_slice(), &(after as i64), &limit],
            )
            .await?
        }
        Stream::Project(project) => {
            c.query(
                &format!(
                    "SELECT seq, device_id, at, payload FROM project_entries
                     WHERE team_id = $1 AND project_id = $2 AND seq {op} $3 ORDER BY seq LIMIT $4"
                ),
                &[
                    &team.as_slice(),
                    &project.as_slice(),
                    &(after as i64),
                    &limit,
                ],
            )
            .await?
        }
    };
    let project = matches!(stream, Stream::Project(_));
    Ok(rows.iter().map(|r| entry_of(r, project)).collect())
}

#[async_trait]
impl Store for Postgres {
    async fn migrate(&self) -> Result<()> {
        Ok(self.client.lock().await.batch_execute(SCHEMA).await?)
    }

    async fn ping(&self) -> Result<()> {
        self.client.lock().await.simple_query("SELECT 1").await?;
        Ok(())
    }

    async fn add_token(
        &self,
        hash: [u8; 32],
        kind: TokenKind,
        team: Option<Id>,
        expires_at: Option<i64>,
        now: i64,
    ) -> Result<()> {
        let kind: i64 = match kind {
            TokenKind::Team => 0,
            TokenKind::Device => 1,
        };
        let team = team.map(|t| t.to_vec());
        self.client
            .lock()
            .await
            .execute(
                "INSERT INTO tokens (hash, kind, team_id, created_at, expires_at) VALUES ($1, $2, $3, $4, $5)",
                &[&hash.as_slice(), &kind, &team, &now, &expires_at],
            )
            .await?;
        Ok(())
    }

    async fn enroll(
        &self,
        hash: [u8; 32],
        public_key: [u8; 32],
        now: i64,
        quotas: Quotas,
    ) -> Result<Device> {
        let mut client = self.client.lock().await;
        let tx = client.transaction().await?;
        let Some(token) = tx
            .query_opt(
                "SELECT kind, team_id, expires_at FROM tokens WHERE hash = $1 FOR UPDATE",
                &[&hash.as_slice()],
            )
            .await?
        else {
            return Err(StoreError::BadToken);
        };
        let kind: i64 = token.get(0);
        let team: Option<Vec<u8>> = token.get(1);
        let expires_at: Option<i64> = token.get(2);
        if expires_at.is_some_and(|e| e <= now) {
            tx.execute("DELETE FROM tokens WHERE hash = $1", &[&hash.as_slice()])
                .await?;
            tx.commit().await?;
            return Err(StoreError::BadToken);
        }
        if tx
            .query_opt(
                "SELECT 1 FROM devices WHERE public_key = $1",
                &[&public_key.as_slice()],
            )
            .await?
            .is_some()
        {
            return Err(StoreError::KeyTaken);
        }
        let (team, role) = if kind == 0 {
            let team: Id = random();
            tx.execute(
                "INSERT INTO teams (id, created_at) VALUES ($1, $2)",
                &[&team.as_slice(), &now],
            )
            .await?;
            (team, Role::Admin)
        } else {
            let team = id_of(team.ok_or(StoreError::BadToken)?)?;
            // The team's row, locked: two enrolments count one after the other.
            tx.query_one(
                "SELECT 1 FROM teams WHERE id = $1 FOR UPDATE",
                &[&team.as_slice()],
            )
            .await?;
            let count: i64 = tx
                .query_one(
                    "SELECT COUNT(*) FROM devices WHERE team_id = $1",
                    &[&team.as_slice()],
                )
                .await?
                .get(0);
            if count >= quotas.devices as i64 {
                return Err(StoreError::TooMany("devices"));
            }
            (team, Role::Member)
        };
        let id: Id = random();
        tx.execute(
            "INSERT INTO devices (id, team_id, public_key, role, created_at) VALUES ($1, $2, $3, $4, $5)",
            &[&id.as_slice(), &team.as_slice(), &public_key.as_slice(), &role.as_str(), &now],
        )
        .await?;
        tx.execute("DELETE FROM tokens WHERE hash = $1", &[&hash.as_slice()])
            .await?;
        tx.commit().await?;
        Ok(Device {
            id,
            team,
            public_key,
            role,
            created_at: now,
            revoked_at: None,
        })
    }

    async fn device(&self, id: Id) -> Result<Option<Device>> {
        let row = self
            .client
            .lock()
            .await
            .query_opt(
                &format!("SELECT {DEVICE_COLUMNS} FROM devices WHERE id = $1"),
                &[&id.as_slice()],
            )
            .await?;
        row.as_ref().map(to_device).transpose()
    }

    async fn devices(&self, team: Id) -> Result<Vec<Device>> {
        let rows = self
            .client
            .lock()
            .await
            .query(
                &format!("SELECT {DEVICE_COLUMNS} FROM devices WHERE team_id = $1 ORDER BY created_at, id"),
                &[&team.as_slice()],
            )
            .await?;
        rows.iter().map(to_device).collect()
    }

    async fn revoke(&self, team: Id, device: Id, now: i64) -> Result<bool> {
        let n = self
            .client
            .lock()
            .await
            .execute(
                "UPDATE devices SET revoked_at = COALESCE(revoked_at, $3) WHERE id = $1 AND team_id = $2",
                &[&device.as_slice(), &team.as_slice(), &now],
            )
            .await?;
        Ok(n == 1)
    }

    async fn append(
        &self,
        team: Id,
        stream: Stream,
        entry: NewEntry,
        quotas: Quotas,
        page: Page,
    ) -> Result<Entry> {
        let mut client = self.client.lock().await;
        let tx = client.transaction().await?;
        let row = tx
            .query_one(
                "SELECT seq, commits, stored FROM teams WHERE id = $1 FOR UPDATE",
                &[&team.as_slice()],
            )
            .await?;
        let (seq, commits, stored): (i64, i64, i64) = (row.get(0), row.get(1), row.get(2));
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
                        let from = tx
                            .query_opt(
                                "SELECT seq FROM team_entries WHERE team_id = $1 AND kind = 1 AND epoch = $2",
                                &[&team.as_slice(), &(asked.min(i64::MAX as u64) as i64)],
                            )
                            .await?
                            .map(|r| r.get::<_, i64>(0));
                        let preceding = match from {
                            Some(from) => {
                                paginate(
                                    read_entries(&tx, &team, stream, from as u64, page, true)
                                        .await?,
                                    page,
                                )
                                .0
                            }
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
                let epoch = entry.epoch.filter(|_| is_commit).map(|e| e as i64);
                tx.execute(
                    "INSERT INTO team_entries (team_id, seq, kind, epoch, device_id, at, payload)
                     VALUES ($1, $2, $3, $4, $5, $6, $7)",
                    &[
                        &team.as_slice(),
                        &seq,
                        &entry.kind.code(),
                        &epoch,
                        &entry.device.as_slice(),
                        &entry.at,
                        &entry.payload,
                    ],
                )
                .await?;
                tx.execute(
                    "UPDATE teams SET seq = $2, commits = commits + $3, stored = stored + $4 WHERE id = $1",
                    &[&team.as_slice(), &seq, &(is_commit as i64), &size],
                )
                .await?;
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
                let current = tx
                    .query_opt(
                        "SELECT seq FROM projects WHERE team_id = $1 AND id = $2 FOR UPDATE",
                        &[&team.as_slice(), &project.as_slice()],
                    )
                    .await?
                    .map(|r| r.get::<_, i64>(0));
                let current = match current {
                    Some(s) => s,
                    None => {
                        let n: i64 = tx
                            .query_one(
                                "SELECT COUNT(*) FROM projects WHERE team_id = $1",
                                &[&team.as_slice()],
                            )
                            .await?
                            .get(0);
                        if n >= quotas.projects as i64 {
                            return Err(StoreError::TooMany("projects"));
                        }
                        tx.execute(
                            "INSERT INTO projects (team_id, id) VALUES ($1, $2)",
                            &[&team.as_slice(), &project.as_slice()],
                        )
                        .await?;
                        0
                    }
                };
                let seq = current + 1;
                tx.execute(
                    "INSERT INTO project_entries (team_id, project_id, seq, device_id, at, payload)
                     VALUES ($1, $2, $3, $4, $5, $6)",
                    &[
                        &team.as_slice(),
                        &project.as_slice(),
                        &seq,
                        &entry.device.as_slice(),
                        &entry.at,
                        &entry.payload,
                    ],
                )
                .await?;
                tx.execute(
                    "UPDATE projects SET seq = $3 WHERE team_id = $1 AND id = $2",
                    &[&team.as_slice(), &project.as_slice(), &seq],
                )
                .await?;
                tx.execute(
                    "UPDATE teams SET stored = stored + $2 WHERE id = $1",
                    &[&team.as_slice(), &size],
                )
                .await?;
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
        tx.commit().await?;
        Ok(out)
    }

    async fn read(
        &self,
        team: Id,
        stream: Stream,
        since: u64,
        page: Page,
    ) -> Result<(Vec<Entry>, bool)> {
        let client = self.client.lock().await;
        let since = since.min(i64::MAX as u64);
        Ok(paginate(
            read_entries(&*client, &team, stream, since, page, false).await?,
            page,
        ))
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
        let mut client = self.client.lock().await;
        let tx = client.transaction().await?;
        let stored: i64 = tx
            .query_one(
                "SELECT stored FROM teams WHERE id = $1 FOR UPDATE",
                &[&team.as_slice()],
            )
            .await?
            .get(0);
        if tx
            .query_opt(
                "SELECT 1 FROM files WHERE team_id = $1 AND id = $2",
                &[&team.as_slice(), &file.as_slice()],
            )
            .await?
            .is_some()
        {
            return Err(StoreError::Exists);
        }
        if stored as u64 + size > quotas.team_bytes {
            return Err(StoreError::Quota);
        }
        tx.execute(
            "INSERT INTO files (team_id, id, size, device_id, at) VALUES ($1, $2, $3, $4, $5)",
            &[
                &team.as_slice(),
                &file.as_slice(),
                &(size as i64),
                &device.as_slice(),
                &now,
            ],
        )
        .await?;
        tx.execute(
            "UPDATE teams SET stored = stored + $2 WHERE id = $1",
            &[&team.as_slice(), &(size as i64)],
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn file_size(&self, team: Id, file: Id) -> Result<Option<u64>> {
        Ok(self
            .client
            .lock()
            .await
            .query_opt(
                "SELECT size FROM files WHERE team_id = $1 AND id = $2",
                &[&team.as_slice(), &file.as_slice()],
            )
            .await?
            .map(|r| r.get::<_, i64>(0) as u64))
    }

    async fn delete_file(&self, team: Id, file: Id) -> Result<Option<u64>> {
        let mut client = self.client.lock().await;
        let tx = client.transaction().await?;
        let size = tx
            .query_opt(
                "DELETE FROM files WHERE team_id = $1 AND id = $2 RETURNING size",
                &[&team.as_slice(), &file.as_slice()],
            )
            .await?
            .map(|r| r.get::<_, i64>(0));
        if let Some(size) = size {
            tx.execute(
                "UPDATE teams SET stored = stored - $2 WHERE id = $1",
                &[&team.as_slice(), &size],
            )
            .await?;
        }
        tx.commit().await?;
        Ok(size.map(|s| s as u64))
    }

    async fn used_bytes(&self, team: Id) -> Result<u64> {
        Ok(self
            .client
            .lock()
            .await
            .query_opt(
                "SELECT stored FROM teams WHERE id = $1",
                &[&team.as_slice()],
            )
            .await?
            .map_or(0, |r| r.get::<_, i64>(0) as u64))
    }

    async fn counts(&self) -> Result<Counts> {
        let client = self.client.lock().await;
        let teams: i64 = client
            .query_one("SELECT COUNT(*) FROM teams", &[])
            .await?
            .get(0);
        let row = client
            .query_one("SELECT COUNT(*), COUNT(revoked_at) FROM devices", &[])
            .await?;
        Ok(Counts {
            teams: teams as u64,
            devices: row.get::<_, i64>(0) as u64,
            revoked_devices: row.get::<_, i64>(1) as u64,
        })
    }
}
