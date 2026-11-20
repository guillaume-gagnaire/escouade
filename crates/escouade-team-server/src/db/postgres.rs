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
ALTER TABLE tokens ADD COLUMN IF NOT EXISTS used_at BIGINT;
ALTER TABLE tokens ADD COLUMN IF NOT EXISTS reused_at BIGINT;
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
ALTER TABLE projects ADD COLUMN IF NOT EXISTS commits BIGINT NOT NULL DEFAULT 0;
ALTER TABLE project_entries ADD COLUMN IF NOT EXISTS kind BIGINT NOT NULL DEFAULT 0;
ALTER TABLE project_entries ADD COLUMN IF NOT EXISTS epoch BIGINT;
CREATE UNIQUE INDEX IF NOT EXISTS project_commits ON project_entries(team_id, project_id, epoch) WHERE kind = 1;
CREATE TABLE IF NOT EXISTS project_snapshots (
    team_id BYTEA NOT NULL,
    project_id BYTEA NOT NULL,
    upto BIGINT NOT NULL,
    device_id BYTEA NOT NULL,
    at BIGINT NOT NULL,
    payload BYTEA NOT NULL,
    PRIMARY KEY (team_id, project_id)
);
CREATE TABLE IF NOT EXISTS files (
    team_id BYTEA NOT NULL,
    id BYTEA NOT NULL,
    size BIGINT NOT NULL,
    device_id BYTEA NOT NULL,
    at BIGINT NOT NULL,
    PRIMARY KEY (team_id, id)
);
CREATE TABLE IF NOT EXISTS project_access (
    team_id BYTEA NOT NULL,
    project_id BYTEA NOT NULL,
    everyone BOOLEAN NOT NULL,
    devices BYTEA NOT NULL,
    PRIMARY KEY (team_id, project_id)
);
CREATE TABLE IF NOT EXISTS claims (
    team_id BYTEA NOT NULL,
    project_id BYTEA NOT NULL,
    ticket_id BYTEA NOT NULL,
    holder BYTEA,
    until BIGINT NOT NULL,
    token BIGINT NOT NULL,
    version BIGINT NOT NULL,
    claimed_at BIGINT NOT NULL,
    renewed_at BIGINT NOT NULL,
    last_holder BYTEA,
    handed_by BYTEA,
    PRIMARY KEY (team_id, project_id, ticket_id)
);
CREATE TABLE IF NOT EXISTS project_keys (
    team_id BYTEA NOT NULL,
    project_id BYTEA NOT NULL,
    n BIGINT NOT NULL,
    PRIMARY KEY (team_id, project_id)
);
";

const CLAIM_COLUMNS: &str =
    "ticket_id, holder, until, token, version, claimed_at, renewed_at, last_holder, handed_by";

fn claim_row(r: &tokio_postgres::Row) -> Claim {
    let id = |v: Option<Vec<u8>>| v.and_then(|v| <Id>::try_from(v).ok());
    Claim {
        ticket: r.get(0),
        holder: id(r.get(1)),
        until: r.get(2),
        token: r.get::<_, i64>(3) as u64,
        version: r.get::<_, i64>(4) as u64,
        claimed_at: r.get(5),
        renewed_at: r.get(6),
        last_holder: id(r.get(7)),
        handed_by: id(r.get(8)),
    }
}

async fn claim_of(
    c: &impl tokio_postgres::GenericClient,
    team: &Id,
    project: &Id,
    ticket: &[u8],
) -> Result<Option<Claim>> {
    Ok(c
        .query_opt(
            &format!(
                "SELECT {CLAIM_COLUMNS} FROM claims WHERE team_id = $1 AND project_id = $2 AND ticket_id = $3"
            ),
            &[&team.as_slice(), &project.as_slice(), &ticket],
        )
        .await?
        .map(|r| claim_row(&r)))
}

async fn put_claim(
    c: &impl tokio_postgres::GenericClient,
    team: &Id,
    project: &Id,
    claim: &Claim,
) -> Result<()> {
    c.execute(
        "INSERT INTO claims (team_id, project_id, ticket_id, holder, until, token, version, claimed_at, renewed_at, last_holder, handed_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
         ON CONFLICT (team_id, project_id, ticket_id) DO UPDATE SET holder = excluded.holder, until = excluded.until,
           token = excluded.token, version = excluded.version, claimed_at = excluded.claimed_at,
           renewed_at = excluded.renewed_at, last_holder = excluded.last_holder, handed_by = excluded.handed_by",
        &[
            &team.as_slice(),
            &project.as_slice(),
            &claim.ticket.as_slice(),
            &claim.holder.as_ref().map(|h| h.as_slice()),
            &claim.until,
            &(claim.token as i64),
            &(claim.version as i64),
            &claim.claimed_at,
            &claim.renewed_at,
            &claim.last_holder.as_ref().map(|h| h.as_slice()),
            &claim.handed_by.as_ref().map(|h| h.as_slice()),
        ],
    )
    .await?;
    Ok(())
}

async fn access_of(
    c: &impl tokio_postgres::GenericClient,
    team: &Id,
    project: &Id,
) -> Result<Option<Access>> {
    Ok(c.query_opt(
        "SELECT everyone, devices FROM project_access WHERE team_id = $1 AND project_id = $2",
        &[&team.as_slice(), &project.as_slice()],
    )
    .await?
    .map(|r| Access::from_parts(r.get(0), &r.get::<_, Vec<u8>>(1))))
}

/// Whether a holder still holds: a device of the team not revoked, with access to the project.
async fn holder_ok(
    c: &impl tokio_postgres::GenericClient,
    team: &Id,
    project: &Id,
    holder: Option<Id>,
) -> Result<bool> {
    let Some(holder) = holder else {
        return Ok(true);
    };
    let revoked = c
        .query_opt(
            "SELECT revoked_at FROM devices WHERE id = $1 AND team_id = $2",
            &[&holder.as_slice(), &team.as_slice()],
        )
        .await?
        .map(|r| r.get::<_, Option<i64>>(0));
    if !matches!(revoked, Some(None)) {
        return Ok(false);
    }
    Ok(access_of(c, team, project)
        .await?
        .is_none_or(|a| a.admits(&holder)))
}

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

fn entry_of(r: &tokio_postgres::Row) -> Entry {
    Entry {
        seq: r.get::<_, i64>(0) as u64,
        kind: Kind::from_code(r.get(1)),
        epoch: r.get::<_, Option<i64>>(2).map(|e| e as u64),
        device: r.get(3),
        at: r.get(4),
        payload: r.get(5),
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
                    "SELECT seq, kind, epoch, device_id, at, payload FROM project_entries
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
    Ok(rows.iter().map(entry_of).collect())
}

#[async_trait]
impl Store for Postgres {
    async fn migrate(&self) -> Result<()> {
        let client = self.client.lock().await;
        client.batch_execute(SCHEMA).await?;
        // A base made before a ticket could be handed (TS-17).
        client
            .batch_execute("ALTER TABLE claims ADD COLUMN IF NOT EXISTS handed_by BYTEA")
            .await?;
        Ok(())
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
        let kind: i64 = kind.code();
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
                "SELECT kind, team_id, expires_at, used_at FROM tokens WHERE hash = $1 FOR UPDATE",
                &[&hash.as_slice()],
            )
            .await?
        else {
            return Err(StoreError::BadToken);
        };
        let kind: i64 = token.get(0);
        let team: Option<Vec<u8>> = token.get(1);
        let expires_at: Option<i64> = token.get(2);
        let used_at: Option<i64> = token.get(3);
        if used_at.is_some() {
            tx.execute(
                "UPDATE tokens SET reused_at = $2 WHERE hash = $1",
                &[&hash.as_slice(), &now],
            )
            .await?;
            tx.commit().await?;
            return Err(StoreError::TokenUsed);
        }
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
            let role = if kind == TokenKind::Recovery.code() {
                Role::Admin
            } else {
                Role::Member
            };
            (team, role)
        };
        let id: Id = random();
        tx.execute(
            "INSERT INTO devices (id, team_id, public_key, role, created_at) VALUES ($1, $2, $3, $4, $5)",
            &[&id.as_slice(), &team.as_slice(), &public_key.as_slice(), &role.as_str(), &now],
        )
        .await?;
        // Kept used, not deleted: a second use is told apart from an unknown token.
        tx.execute(
            "UPDATE tokens SET used_at = $2 WHERE hash = $1",
            &[&hash.as_slice(), &now],
        )
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

    async fn set_role(&self, team: Id, device: Id, role: Role) -> Result<bool> {
        let mut client = self.client.lock().await;
        let tx = client.transaction().await?;
        // The team's row, locked: two changes of role count one after the other.
        tx.query_opt(
            "SELECT 1 FROM teams WHERE id = $1 FOR UPDATE",
            &[&team.as_slice()],
        )
        .await?;
        let Some(row) = tx
            .query_opt(
                "SELECT role, revoked_at FROM devices WHERE id = $1 AND team_id = $2",
                &[&device.as_slice(), &team.as_slice()],
            )
            .await?
        else {
            return Ok(false);
        };
        let current: String = row.get(0);
        let revoked: Option<i64> = row.get(1);
        if Role::parse(&current) == Role::Admin && role == Role::Member && revoked.is_none() {
            let admins: i64 = tx
                .query_one(
                    "SELECT COUNT(*) FROM devices WHERE team_id = $1 AND role = 'admin' AND revoked_at IS NULL",
                    &[&team.as_slice()],
                )
                .await?
                .get(0);
            if admins <= 1 {
                return Err(StoreError::LastAdmin);
            }
        }
        tx.execute(
            "UPDATE devices SET role = $3 WHERE id = $1 AND team_id = $2",
            &[&device.as_slice(), &team.as_slice(), &role.as_str()],
        )
        .await?;
        tx.commit().await?;
        Ok(true)
    }

    async fn token_alerts(&self, team: Id) -> Result<Vec<TokenAlert>> {
        let rows = self
            .client
            .lock()
            .await
            .query(
                "SELECT hash, used_at, reused_at FROM tokens WHERE team_id = $1 AND reused_at IS NOT NULL ORDER BY reused_at",
                &[&team.as_slice()],
            )
            .await?;
        rows.iter()
            .map(|r| {
                let hash: Vec<u8> = r.get(0);
                Ok(TokenAlert {
                    hash: hash
                        .try_into()
                        .map_err(|_| StoreError::Backend("a hash of the wrong length".into()))?,
                    used_at: r.get(1),
                    reused_at: r.get(2),
                })
            })
            .collect()
    }

    async fn sweep_tokens(&self, now: i64) -> Result<()> {
        self.client
            .lock()
            .await
            .execute(
                "DELETE FROM tokens WHERE expires_at IS NOT NULL AND expires_at + 604800 <= $1",
                &[&now],
            )
            .await?;
        Ok(())
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
                        "SELECT seq, commits FROM projects WHERE team_id = $1 AND id = $2 FOR UPDATE",
                        &[&team.as_slice(), &project.as_slice()],
                    )
                    .await?
                    .map(|r| (r.get::<_, i64>(0), r.get::<_, i64>(1)));
                let is_commit = entry.kind == Kind::Commit;
                // A holder's write passes under the claim's current token alone (TS-6); the team's row is locked.
                if let Some(fence) = &entry.fence {
                    let claim = claim_of(&tx, &team, &project, &fence.ticket).await?;
                    let moved = fenced(claim, fence, entry.device)?;
                    put_claim(&tx, &team, &project, &moved).await?;
                }
                let project_commits = current.map_or(0, |c| c.1);
                if is_commit {
                    let asked = entry.epoch.unwrap_or(u64::MAX);
                    if asked != project_commits as u64 {
                        let from = tx
                            .query_opt(
                                "SELECT seq FROM project_entries WHERE team_id = $1 AND project_id = $2 AND kind = 1 AND epoch = $3",
                                &[&team.as_slice(), &project.as_slice(), &(asked.min(i64::MAX as u64) as i64)],
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
                            expected: project_commits as u64,
                            preceding,
                        });
                    }
                }
                let current = match current.map(|c| c.0) {
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
                let epoch = entry.epoch.filter(|_| is_commit);
                tx.execute(
                    "INSERT INTO project_entries (team_id, project_id, seq, kind, epoch, device_id, at, payload)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                    &[
                        &team.as_slice(),
                        &project.as_slice(),
                        &seq,
                        &entry.kind.code(),
                        &epoch.map(|e| e as i64),
                        &entry.device.as_slice(),
                        &entry.at,
                        &entry.payload,
                    ],
                )
                .await?;
                tx.execute(
                    "UPDATE projects SET seq = $3, commits = commits + $4 WHERE team_id = $1 AND id = $2",
                    &[&team.as_slice(), &project.as_slice(), &seq, &(is_commit as i64)],
                )
                .await?;
                tx.execute(
                    "UPDATE teams SET stored = stored + $2 WHERE id = $1",
                    &[&team.as_slice(), &size],
                )
                .await?;
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
        tx.commit().await?;
        Ok(out)
    }

    async fn put_snapshot(
        &self,
        team: Id,
        project: Id,
        snapshot: Snapshot,
        quotas: Quotas,
    ) -> Result<u64> {
        let mut client = self.client.lock().await;
        let tx = client.transaction().await?;
        let stored: i64 = tx
            .query_one(
                "SELECT stored FROM teams WHERE id = $1 FOR UPDATE",
                &[&team.as_slice()],
            )
            .await?
            .get(0);
        let seq = tx
            .query_opt(
                "SELECT seq FROM projects WHERE team_id = $1 AND id = $2 FOR UPDATE",
                &[&team.as_slice(), &project.as_slice()],
            )
            .await?
            .map(|r| r.get::<_, i64>(0));
        if seq.is_none_or(|s| (s as u64) < snapshot.upto) || snapshot.upto == 0 {
            return Err(StoreError::Snapshot("beyond the project's stream"));
        }
        let old = tx
            .query_opt(
                "SELECT upto, length(payload)::BIGINT FROM project_snapshots WHERE team_id = $1 AND project_id = $2",
                &[&team.as_slice(), &project.as_slice()],
            )
            .await?
            .map(|r| (r.get::<_, i64>(0), r.get::<_, i64>(1)));
        if old.is_some_and(|(upto, _)| upto as u64 >= snapshot.upto) {
            return Err(StoreError::Snapshot("not after the one kept"));
        }
        let upto = snapshot.upto as i64;
        let row = tx
            .query_one(
                "SELECT COUNT(*), COALESCE(SUM(length(payload)), 0)::BIGINT FROM project_entries
                 WHERE team_id = $1 AND project_id = $2 AND kind = 0 AND seq <= $3",
                &[&team.as_slice(), &project.as_slice(), &upto],
            )
            .await?;
        let (pruned, freed): (i64, i64) = (row.get(0), row.get(1));
        let size = snapshot.payload.len() as i64;
        let after = stored - freed - old.map_or(0, |o| o.1) + size;
        if after as u64 > quotas.team_bytes {
            return Err(StoreError::Quota);
        }
        tx.execute(
            "DELETE FROM project_entries WHERE team_id = $1 AND project_id = $2 AND kind = 0 AND seq <= $3",
            &[&team.as_slice(), &project.as_slice(), &upto],
        )
        .await?;
        tx.execute(
            "INSERT INTO project_snapshots (team_id, project_id, upto, device_id, at, payload)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (team_id, project_id) DO UPDATE SET
               upto = EXCLUDED.upto, device_id = EXCLUDED.device_id, at = EXCLUDED.at, payload = EXCLUDED.payload",
            &[
                &team.as_slice(),
                &project.as_slice(),
                &upto,
                &snapshot.device.as_slice(),
                &snapshot.at,
                &snapshot.payload,
            ],
        )
        .await?;
        tx.execute(
            "UPDATE teams SET stored = $2 WHERE id = $1",
            &[&team.as_slice(), &after],
        )
        .await?;
        tx.commit().await?;
        Ok(pruned as u64)
    }

    async fn snapshot(&self, team: Id, project: Id) -> Result<Option<Snapshot>> {
        let client = self.client.lock().await;
        let row = client
            .query_opt(
                "SELECT upto, device_id, at, payload FROM project_snapshots WHERE team_id = $1 AND project_id = $2",
                &[&team.as_slice(), &project.as_slice()],
            )
            .await?;
        row.map(|r| {
            Ok(Snapshot {
                upto: r.get::<_, i64>(0) as u64,
                device: id_of(r.get(1))?,
                at: r.get(2),
                payload: r.get(3),
            })
        })
        .transpose()
    }

    async fn snapshot_upto(&self, team: Id, project: Id) -> Result<Option<u64>> {
        let client = self.client.lock().await;
        Ok(client
            .query_opt(
                "SELECT upto FROM project_snapshots WHERE team_id = $1 AND project_id = $2",
                &[&team.as_slice(), &project.as_slice()],
            )
            .await?
            .map(|r| r.get::<_, i64>(0) as u64))
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

    async fn access(&self, team: Id, project: Id) -> Result<Option<Access>> {
        let client = self.client.lock().await;
        access_of(&*client, &team, &project).await
    }

    async fn set_access(&self, team: Id, project: Id, access: Access, admin: bool) -> Result<()> {
        let mut client = self.client.lock().await;
        let tx = client.transaction().await?;
        tx.query_one(
            "SELECT 1 FROM teams WHERE id = $1 FOR UPDATE",
            &[&team.as_slice()],
        )
        .await?;
        if access_of(&tx, &team, &project).await?.is_some() && !admin {
            return Err(StoreError::NotAdmin);
        }
        tx.execute(
            "INSERT INTO project_access (team_id, project_id, everyone, devices) VALUES ($1, $2, $3, $4)
             ON CONFLICT (team_id, project_id) DO UPDATE SET everyone = excluded.everyone, devices = excluded.devices",
            &[&team.as_slice(), &project.as_slice(), &access.everyone, &access.devices_bytes()],
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn open_access(&self, team: Id, project: Id) -> Result<()> {
        let client = self.client.lock().await;
        client
            .execute(
                "INSERT INTO project_access (team_id, project_id, everyone, devices) VALUES ($1, $2, TRUE, '')
                 ON CONFLICT (team_id, project_id) DO NOTHING",
                &[&team.as_slice(), &project.as_slice()],
            )
            .await?;
        Ok(())
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
        let mut client = self.client.lock().await;
        let tx = client.transaction().await?;
        // The team's row locked: claims and holders' writes go one at a time, as on SQLite.
        tx.query_one(
            "SELECT 1 FROM teams WHERE id = $1 FOR UPDATE",
            &[&team.as_slice()],
        )
        .await?;
        let current = claim_of(&tx, &team, &project, &ticket).await?;
        if current.is_none() {
            let n: i64 = tx
                .query_one(
                    "SELECT COUNT(*) FROM claims WHERE team_id = $1 AND project_id = $2",
                    &[&team.as_slice(), &project.as_slice()],
                )
                .await?
                .get(0);
            if n >= MAX_CLAIMS {
                return Err(StoreError::TooMany("claims"));
            }
        }
        let ok = holder_ok(
            &tx,
            &team,
            &project,
            current.as_ref().and_then(|c| c.holder),
        )
        .await?;
        let to_ok = match ask {
            ClaimAsk::Hand { to, .. } => holder_ok(&tx, &team, &project, Some(to)).await?,
            _ => true,
        };
        let claim = decide_with(current, ticket, device, ask, now, lease, ok, to_ok)?;
        put_claim(&tx, &team, &project, &claim).await?;
        tx.commit().await?;
        Ok(claim)
    }

    async fn claims(&self, team: Id, project: Id) -> Result<Vec<Claim>> {
        let client = self.client.lock().await;
        Ok(client
            .query(
                &format!(
                    "SELECT {CLAIM_COLUMNS} FROM claims WHERE team_id = $1 AND project_id = $2 ORDER BY ticket_id"
                ),
                &[&team.as_slice(), &project.as_slice()],
            )
            .await?
            .iter()
            .map(claim_row)
            .collect())
    }

    async fn next_key(&self, team: Id, project: Id) -> Result<u64> {
        let client = self.client.lock().await;
        let n: i64 = client
            .query_one(
                "INSERT INTO project_keys (team_id, project_id, n) VALUES ($1, $2, 1)
                 ON CONFLICT (team_id, project_id) DO UPDATE SET n = project_keys.n + 1 RETURNING n",
                &[&team.as_slice(), &project.as_slice()],
            )
            .await?
            .get(0);
        Ok(n as u64)
    }
}
