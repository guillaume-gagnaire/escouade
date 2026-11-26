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
ALTER TABLE teams ADD COLUMN IF NOT EXISTS reserved BIGINT NOT NULL DEFAULT 0;
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
ALTER TABLE team_entries ADD COLUMN IF NOT EXISTS voided BIGINT NOT NULL DEFAULT 0;
DROP INDEX IF EXISTS team_commits;
CREATE UNIQUE INDEX IF NOT EXISTS team_live_commits ON team_entries(team_id, epoch) WHERE kind = 1 AND voided = 0;
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
ALTER TABLE projects ADD COLUMN IF NOT EXISTS reserved BIGINT NOT NULL DEFAULT 0;
ALTER TABLE project_entries ADD COLUMN IF NOT EXISTS voided BIGINT NOT NULL DEFAULT 0;
DROP INDEX IF EXISTS project_commits;
CREATE UNIQUE INDEX IF NOT EXISTS project_live_commits ON project_entries(team_id, project_id, epoch)
    WHERE kind = 1 AND voided = 0;
ALTER TABLE projects ADD COLUMN IF NOT EXISTS pruned BIGINT NOT NULL DEFAULT 0;
CREATE TABLE IF NOT EXISTS snapshots (
    team_id BYTEA NOT NULL,
    project_id BYTEA NOT NULL,
    upto BIGINT NOT NULL,
    device_id BYTEA NOT NULL,
    at BIGINT NOT NULL,
    payload BYTEA NOT NULL,
    flagged_by BYTEA,
    flagged_at BIGINT,
    PRIMARY KEY (team_id, project_id, upto)
);
CREATE TABLE IF NOT EXISTS files (
    team_id BYTEA NOT NULL,
    id BYTEA NOT NULL,
    size BIGINT NOT NULL,
    device_id BYTEA NOT NULL,
    at BIGINT NOT NULL,
    PRIMARY KEY (team_id, id)
);
-- A base made before the files were attached to a project (SR2-03): its files attached to nothing, their author's and
-- the administrators' alone until one of them attaches each.
ALTER TABLE files ADD COLUMN IF NOT EXISTS project_id BYTEA;
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
CREATE TABLE IF NOT EXISTS dropped_entries (
    id BIGSERIAL PRIMARY KEY,
    team_id BYTEA NOT NULL,
    project_id BYTEA,
    seq BIGINT NOT NULL,
    kind BIGINT NOT NULL,
    epoch BIGINT,
    device_id BYTEA NOT NULL,
    written_at BIGINT NOT NULL,
    size BIGINT NOT NULL,
    digest BYTEA NOT NULL,
    operator TEXT NOT NULL,
    at BIGINT NOT NULL
);
CREATE INDEX IF NOT EXISTS dropped_team ON dropped_entries(team_id);
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

/// Makes `project` known to the server at its first write of any sort (an entry, its access, a claim, a number),
/// counted in the team's projects (SR2-04): beyond them, an unknown project is refused. The team's row is locked by the
/// caller.
async fn register_project(
    c: &impl tokio_postgres::GenericClient,
    team: &Id,
    project: &Id,
    quotas: &Quotas,
) -> Result<()> {
    let known = c
        .query_opt(
            "SELECT 1 FROM projects WHERE team_id = $1 AND id = $2",
            &[&team.as_slice(), &project.as_slice()],
        )
        .await?
        .is_some();
    if known {
        return Ok(());
    }
    let n: i64 = c
        .query_one(
            "SELECT COUNT(*) FROM projects WHERE team_id = $1",
            &[&team.as_slice()],
        )
        .await?
        .get(0);
    if n >= quotas.projects as i64 {
        return Err(StoreError::TooMany("projects"));
    }
    c.execute(
        "INSERT INTO projects (team_id, id) VALUES ($1, $2)",
        &[&team.as_slice(), &project.as_slice()],
    )
    .await?;
    Ok(())
}

/// Counts `bytes` more in the team's quota (fewer when negative), refused beyond it (SR2-04). The team's row is locked
/// by the caller.
async fn charge(
    c: &impl tokio_postgres::GenericClient,
    team: &Id,
    bytes: i64,
    quotas: &Quotas,
) -> Result<()> {
    let stored: i64 = c
        .query_one(
            "SELECT stored FROM teams WHERE id = $1",
            &[&team.as_slice()],
        )
        .await?
        .get(0);
    if bytes > 0 && (stored + bytes) as u64 > quotas.team_bytes {
        return Err(StoreError::Quota);
    }
    c.execute(
        "UPDATE teams SET stored = GREATEST(stored + $2, 0) WHERE id = $1",
        &[&team.as_slice(), &bytes],
    )
    .await?;
    Ok(())
}

pub struct Postgres {
    client: Mutex<Client>,
}

fn backend(e: impl std::fmt::Display) -> StoreError {
    StoreError::Backend(e.to_string())
}

impl From<tokio_postgres::Error> for StoreError {
    /// Its code alone, never its text (SR2-10): a `DETAIL` of PostgreSQL can quote the value of a key.
    fn from(e: tokio_postgres::Error) -> Self {
        StoreError::Backend(match e.code() {
            Some(code) => format!("postgres: SQLSTATE {}", code.code()),
            None if e.is_closed() => "postgres: the connection is closed".into(),
            None => "postgres: a client error".into(),
        })
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
        // A base made before the team tokens expired (SR2-11): one not used yet lasts its bound from its making.
        client
            .execute(
                "UPDATE tokens SET expires_at = created_at + $1 WHERE kind = 0 AND expires_at IS NULL",
                &[&TEAM_TOKEN_TTL],
            )
            .await?;
        // A base made before the retention (SR2-01): one snapshot per project, its messages already forgotten. The
        // schema of the search path (a test's own, or the base's).
        let old = client
            .query_one("SELECT to_regclass('project_snapshots') IS NOT NULL", &[])
            .await?
            .get::<_, bool>(0);
        if old {
            client
                .batch_execute(
                    "BEGIN;
                     INSERT INTO snapshots (team_id, project_id, upto, device_id, at, payload)
                       SELECT team_id, project_id, upto, device_id, at, payload FROM project_snapshots
                       ON CONFLICT DO NOTHING;
                     UPDATE projects p SET pruned = s.upto FROM project_snapshots s
                       WHERE s.team_id = p.team_id AND s.project_id = p.id;
                     DROP TABLE project_snapshots;
                     COMMIT;",
                )
                .await?;
        }
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
            // The revoked devices are kept for the history, out of the count (SR2-12).
            let count: i64 = tx
                .query_one(
                    "SELECT COUNT(*) FROM devices WHERE team_id = $1 AND revoked_at IS NULL",
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
        let mut client = self.client.lock().await;
        let tx = client.transaction().await?;
        // The team's row, locked: two revocations count one after the other.
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
        let role: String = row.get(0);
        let revoked: Option<i64> = row.get(1);
        // The team's last administrator on the server stays (SR2-13): another is named first.
        if Role::parse(&role) == Role::Admin && revoked.is_none() {
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
            "UPDATE devices SET revoked_at = COALESCE(revoked_at, $3) WHERE id = $1 AND team_id = $2",
            &[&device.as_slice(), &team.as_slice(), &now],
        )
        .await?;
        tx.commit().await?;
        Ok(true)
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
        if Role::parse(&current) == Role::Admin && role != Role::Admin && revoked.is_none() {
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
                "SELECT seq, commits, stored, reserved FROM teams WHERE id = $1 FOR UPDATE",
                &[&team.as_slice()],
            )
            .await?;
        let (seq, commits, stored): (i64, i64, i64) = (row.get(0), row.get(1), row.get(2));
        let team_reserved = row.get::<_, i64>(3) != 0;
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
                        return Err(StoreError::Void(
                            "it is not for the epoch of the last commit",
                        ));
                    }
                    let n = tx
                        .execute(
                            "UPDATE team_entries SET voided = 1
                             WHERE team_id = $1 AND kind = 1 AND epoch = $2 AND voided = 0",
                            &[&team.as_slice(), &(asked as i64)],
                        )
                        .await?;
                    if n == 0 {
                        return Err(StoreError::Void("the last commit is declared null already"));
                    }
                }
                if is_commit {
                    let asked = entry.epoch.unwrap_or(u64::MAX);
                    if asked == commits as u64 && team_reserved && !entry.by_admin {
                        return Err(StoreError::Reserved);
                    }
                    if asked != commits as u64 {
                        let from = tx
                            .query_opt(
                                "SELECT seq FROM team_entries WHERE team_id = $1 AND kind = 1 AND epoch = $2 AND voided = 0",
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
                let epoch = entry
                    .epoch
                    .filter(|_| entry.kind != Kind::Message)
                    .map(|e| e as i64);
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
                let (step, reserved): (i64, bool) = match entry.kind {
                    Kind::Commit => (1, false),
                    Kind::Void => (-1, true),
                    Kind::Message => (0, team_reserved),
                };
                tx.execute(
                    "UPDATE teams SET seq = $2, commits = commits + $3, stored = stored + $4, reserved = $5 WHERE id = $1",
                    &[&team.as_slice(), &seq, &step, &size, &(reserved as i64)],
                )
                .await?;
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
                let current = tx
                    .query_opt(
                        "SELECT seq, commits, reserved FROM projects WHERE team_id = $1 AND id = $2 FOR UPDATE",
                        &[&team.as_slice(), &project.as_slice()],
                    )
                    .await?
                    .map(|r| (r.get::<_, i64>(0), r.get::<_, i64>(1), r.get::<_, i64>(2) != 0));
                let is_commit = entry.kind == Kind::Commit;
                // A holder's write passes under the claim's current token alone (TS-6); the team's row is locked.
                if let Some(fence) = &entry.fence {
                    let claim = claim_of(&tx, &team, &project, &fence.ticket).await?;
                    let moved = fenced(claim, fence, entry.device)?;
                    put_claim(&tx, &team, &project, &moved).await?;
                }
                let project_commits = current.map_or(0, |c| c.1);
                let reserved = current.is_some_and(|c| c.2);
                if entry.kind == Kind::Void {
                    let asked = entry.epoch.unwrap_or(u64::MAX);
                    if project_commits == 0 || asked != project_commits as u64 - 1 {
                        return Err(StoreError::Void(
                            "it is not for the epoch of the last commit",
                        ));
                    }
                    let n = tx
                        .execute(
                            "UPDATE project_entries SET voided = 1
                             WHERE team_id = $1 AND project_id = $2 AND kind = 1 AND epoch = $3 AND voided = 0",
                            &[&team.as_slice(), &project.as_slice(), &(asked as i64)],
                        )
                        .await?;
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
                        let from = tx
                            .query_opt(
                                "SELECT seq FROM project_entries
                                 WHERE team_id = $1 AND project_id = $2 AND kind = 1 AND epoch = $3 AND voided = 0",
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
                        register_project(&tx, &team, &project, &quotas).await?;
                        0
                    }
                };
                let seq = current + 1;
                let epoch = entry.epoch.filter(|_| entry.kind != Kind::Message);
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
                let (step, reserved): (i64, bool) = match entry.kind {
                    Kind::Commit => (1, false),
                    Kind::Void => (-1, true),
                    Kind::Message => (0, reserved),
                };
                tx.execute(
                    "UPDATE projects SET seq = $3, commits = commits + $4, reserved = $5 WHERE team_id = $1 AND id = $2",
                    &[&team.as_slice(), &project.as_slice(), &seq, &step, &(reserved as i64)],
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

    async fn drop_entry(
        &self,
        team: Id,
        stream: Stream,
        seq: u64,
        operator: String,
        now: i64,
    ) -> Result<Option<Dropped>> {
        let mut client = self.client.lock().await;
        let tx = client.transaction().await?;
        let Some(team_commits) = tx
            .query_opt(
                "SELECT commits FROM teams WHERE id = $1 FOR UPDATE",
                &[&team.as_slice()],
            )
            .await?
            .map(|r| r.get::<_, i64>(0))
        else {
            return Err(StoreError::Drop("the base holds no such team"));
        };
        let page = Page {
            entries: 1,
            bytes: usize::MAX,
        };
        let Some(entry) = read_entries(&tx, &team, stream, seq, page, true)
            .await?
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
            return Err(StoreError::Drop(
                "a void stays: the devices read on past it already",
            ));
        }
        if is_commit {
            let voided: i64 = match project {
                None => tx
                    .query_one(
                        "SELECT voided FROM team_entries WHERE team_id = $1 AND seq = $2",
                        &[&team.as_slice(), &(seq as i64)],
                    )
                    .await?
                    .get(0),
                Some(p) => tx
                    .query_one(
                        "SELECT voided FROM project_entries WHERE team_id = $1 AND project_id = $2 AND seq = $3",
                        &[&team.as_slice(), &p.as_slice(), &(seq as i64)],
                    )
                    .await?
                    .get(0),
            };
            if voided != 0 {
                return Err(StoreError::Drop(
                    "a commit declared null stays: the devices read on past it already",
                ));
            }
            let stream_commits = match project {
                None => team_commits,
                Some(p) => tx
                    .query_one(
                        "SELECT commits FROM projects WHERE team_id = $1 AND id = $2 FOR UPDATE",
                        &[&team.as_slice(), &p.as_slice()],
                    )
                    .await?
                    .get::<_, i64>(0),
            };
            if entry.epoch.map(|e| e as i64 + 1) != Some(stream_commits) {
                return Err(StoreError::Drop(
                    "a later commit follows this one: remove the later ones first",
                ));
            }
        }
        let size = entry.payload.len() as i64;
        let commit = is_commit as i64;
        match project {
            None => {
                tx.execute(
                    "DELETE FROM team_entries WHERE team_id = $1 AND seq = $2",
                    &[&team.as_slice(), &(seq as i64)],
                )
                .await?;
                tx.execute(
                    "UPDATE teams SET commits = commits - $2 WHERE id = $1",
                    &[&team.as_slice(), &commit],
                )
                .await?;
            }
            Some(p) => {
                tx.execute(
                    "DELETE FROM project_entries WHERE team_id = $1 AND project_id = $2 AND seq = $3",
                    &[&team.as_slice(), &p.as_slice(), &(seq as i64)],
                )
                .await?;
                tx.execute(
                    "UPDATE projects SET commits = commits - $3 WHERE team_id = $1 AND id = $2",
                    &[&team.as_slice(), &p.as_slice(), &commit],
                )
                .await?;
            }
        }
        tx.execute(
            "UPDATE teams SET stored = GREATEST(stored - $2, 0) WHERE id = $1",
            &[&team.as_slice(), &size],
        )
        .await?;
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
        let project_bytes = dropped.project.map(|p| p.to_vec());
        tx.execute(
            "INSERT INTO dropped_entries
             (team_id, project_id, seq, kind, epoch, device_id, written_at, size, digest, operator, at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            &[
                &team.as_slice(),
                &project_bytes,
                &(seq as i64),
                &dropped.kind.code(),
                &dropped.epoch.map(|e| e as i64),
                &dropped.device.as_slice(),
                &dropped.written_at,
                &size,
                &dropped.digest.as_slice(),
                &dropped.operator,
                &now,
            ],
        )
        .await?;
        tx.commit().await?;
        Ok(Some(dropped))
    }

    async fn dropped(&self, team: Id) -> Result<Vec<Dropped>> {
        let rows = self
            .client
            .lock()
            .await
            .query(
                "SELECT project_id, seq, kind, epoch, device_id, written_at, size, digest, operator, at
                 FROM dropped_entries WHERE team_id = $1 ORDER BY at, id",
                &[&team.as_slice()],
            )
            .await?;
        rows.iter()
            .map(|r| {
                Ok(Dropped {
                    project: r.get::<_, Option<Vec<u8>>>(0).map(id_of).transpose()?,
                    seq: r.get::<_, i64>(1) as u64,
                    kind: Kind::from_code(r.get(2)),
                    epoch: r.get::<_, Option<i64>>(3).map(|e| e as u64),
                    device: id_of(r.get(4))?,
                    written_at: r.get(5),
                    size: r.get::<_, i64>(6) as u64,
                    digest: r
                        .get::<_, Vec<u8>>(7)
                        .try_into()
                        .map_err(|_| backend("a digest of the wrong length"))?,
                    operator: r.get(8),
                    at: r.get(9),
                })
            })
            .collect()
    }

    async fn put_snapshot(
        &self,
        team: Id,
        project: Id,
        snapshot: Snapshot,
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
        let last: Option<i64> = tx
            .query_one(
                "SELECT MAX(upto) FROM snapshots WHERE team_id = $1 AND project_id = $2",
                &[&team.as_slice(), &project.as_slice()],
            )
            .await?
            .get(0);
        if last.is_some_and(|upto| upto as u64 >= snapshot.upto) {
            return Err(StoreError::Snapshot("not after the one kept"));
        }
        let size = snapshot.payload.len() as i64;
        if (stored + size) as u64 > quotas.team_bytes {
            return Err(StoreError::Quota);
        }
        tx.execute(
            "INSERT INTO snapshots (team_id, project_id, upto, device_id, at, payload) VALUES ($1, $2, $3, $4, $5, $6)",
            &[
                &team.as_slice(),
                &project.as_slice(),
                &(snapshot.upto as i64),
                &snapshot.device.as_slice(),
                &snapshot.at,
                &snapshot.payload,
            ],
        )
        .await?;
        tx.execute(
            "UPDATE teams SET stored = stored + $2 WHERE id = $1",
            &[&team.as_slice(), &size],
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn snapshot(
        &self,
        team: Id,
        project: Id,
        before: Option<u64>,
    ) -> Result<Option<Snapshot>> {
        let before = before.map_or(i64::MAX, |b| b.min(i64::MAX as u64) as i64);
        let client = self.client.lock().await;
        let row = client
            .query_opt(
                "SELECT upto, device_id, at, payload FROM snapshots
                 WHERE team_id = $1 AND project_id = $2 AND upto < $3 ORDER BY upto DESC LIMIT 1",
                &[&team.as_slice(), &project.as_slice(), &before],
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
            .query_one(
                "SELECT MAX(upto) FROM snapshots WHERE team_id = $1 AND project_id = $2",
                &[&team.as_slice(), &project.as_slice()],
            )
            .await?
            .get::<_, Option<i64>>(0)
            .map(|u| u as u64))
    }

    async fn flag_snapshot(
        &self,
        team: Id,
        project: Id,
        upto: u64,
        device: Id,
        now: i64,
    ) -> Result<bool> {
        let upto = upto.min(i64::MAX as u64) as i64;
        let found = self
            .client
            .lock()
            .await
            .execute(
                "UPDATE snapshots SET flagged_by = COALESCE(flagged_by, $4), flagged_at = COALESCE(flagged_at, $5)
                 WHERE team_id = $1 AND project_id = $2 AND upto = $3",
                &[&team.as_slice(), &project.as_slice(), &upto, &device.as_slice(), &now],
            )
            .await?;
        Ok(found > 0)
    }

    async fn remove_snapshot(&self, team: Id, project: Id, upto: u64) -> Result<bool> {
        let upto = upto.min(i64::MAX as u64) as i64;
        let mut client = self.client.lock().await;
        let tx = client.transaction().await?;
        tx.query_one(
            "SELECT stored FROM teams WHERE id = $1 FOR UPDATE",
            &[&team.as_slice()],
        )
        .await?;
        let size = tx
            .query_opt(
                "SELECT length(payload)::BIGINT FROM snapshots WHERE team_id = $1 AND project_id = $2 AND upto = $3",
                &[&team.as_slice(), &project.as_slice(), &upto],
            )
            .await?
            .map(|r| r.get::<_, i64>(0));
        let Some(size) = size else {
            return Ok(false);
        };
        let pruned = tx
            .query_opt(
                "SELECT pruned FROM projects WHERE team_id = $1 AND id = $2",
                &[&team.as_slice(), &project.as_slice()],
            )
            .await?
            .map_or(0, |r| r.get::<_, i64>(0));
        if upto <= pruned {
            return Err(StoreError::Snapshot(
                "the stream was pruned to it: its messages are gone",
            ));
        }
        tx.execute(
            "DELETE FROM snapshots WHERE team_id = $1 AND project_id = $2 AND upto = $3",
            &[&team.as_slice(), &project.as_slice(), &upto],
        )
        .await?;
        tx.execute(
            "UPDATE teams SET stored = stored - $2 WHERE id = $1",
            &[&team.as_slice(), &size],
        )
        .await?;
        tx.commit().await?;
        Ok(true)
    }

    async fn unreadable_snapshots(&self, team: Id) -> Result<Vec<Unreadable>> {
        let rows = self
            .client
            .lock()
            .await
            .query(
                "SELECT project_id, upto, device_id, at, flagged_by, flagged_at FROM snapshots
                 WHERE team_id = $1 AND flagged_at IS NOT NULL ORDER BY flagged_at, project_id, upto",
                &[&team.as_slice()],
            )
            .await?;
        rows.iter()
            .map(|r| {
                Ok(Unreadable {
                    project: id_of(r.get(0))?,
                    upto: r.get::<_, i64>(1) as u64,
                    device: id_of(r.get(2))?,
                    at: r.get(3),
                    flagged_by: id_of(r.get(4))?,
                    flagged_at: r.get(5),
                })
            })
            .collect()
    }

    async fn prune_snapshots(&self, cut: i64) -> Result<u64> {
        let mut client = self.client.lock().await;
        // For each project, its last snapshot old enough that nobody flagged, when it is past what is pruned.
        let due = client
            .query(
                "SELECT s.team_id, s.project_id, MAX(s.upto) FROM snapshots s
                 JOIN projects p ON p.team_id = s.team_id AND p.id = s.project_id
                 WHERE s.at <= $1 AND s.flagged_at IS NULL AND s.upto > p.pruned
                 GROUP BY s.team_id, s.project_id",
                &[&cut],
            )
            .await?;
        let mut forgotten = 0u64;
        for row in due {
            let (team, project, upto): (Vec<u8>, Vec<u8>, i64) =
                (row.get(0), row.get(1), row.get(2));
            let tx = client.transaction().await?;
            tx.query_one(
                "SELECT stored FROM teams WHERE id = $1 FOR UPDATE",
                &[&team],
            )
            .await?;
            let r = tx
                .query_one(
                    "SELECT COUNT(*), COALESCE(SUM(length(payload)), 0)::BIGINT FROM project_entries
                     WHERE team_id = $1 AND project_id = $2 AND kind = 0 AND seq <= $3",
                    &[&team, &project, &upto],
                )
                .await?;
            let (messages, freed): (i64, i64) = (r.get(0), r.get(1));
            let older: i64 = tx
                .query_one(
                    "SELECT COALESCE(SUM(length(payload)), 0)::BIGINT FROM snapshots
                     WHERE team_id = $1 AND project_id = $2 AND upto < $3",
                    &[&team, &project, &upto],
                )
                .await?
                .get(0);
            tx.execute(
                "DELETE FROM project_entries WHERE team_id = $1 AND project_id = $2 AND kind = 0 AND seq <= $3",
                &[&team, &project, &upto],
            )
            .await?;
            tx.execute(
                "DELETE FROM snapshots WHERE team_id = $1 AND project_id = $2 AND upto < $3",
                &[&team, &project, &upto],
            )
            .await?;
            tx.execute(
                "UPDATE projects SET pruned = $3 WHERE team_id = $1 AND id = $2",
                &[&team, &project, &upto],
            )
            .await?;
            tx.execute(
                "UPDATE teams SET stored = stored - $2 WHERE id = $1",
                &[&team, &(freed + older)],
            )
            .await?;
            tx.commit().await?;
            forgotten += messages as u64;
        }
        Ok(forgotten)
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
            "INSERT INTO files (team_id, id, size, device_id, at, project_id) VALUES ($1, $2, $3, $4, $5, $6)",
            &[
                &team.as_slice(),
                &file.as_slice(),
                &(size as i64),
                &device.as_slice(),
                &now,
                &project.as_ref().map(|p| p.as_slice()),
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

    async fn file(&self, team: Id, file: Id) -> Result<Option<StoredFile>> {
        let row = self
            .client
            .lock()
            .await
            .query_opt(
                "SELECT size, device_id, project_id FROM files WHERE team_id = $1 AND id = $2",
                &[&team.as_slice(), &file.as_slice()],
            )
            .await?;
        row.map(|r| {
            Ok(StoredFile {
                size: r.get::<_, i64>(0) as u64,
                device: id_of(r.get(1))?,
                project: r.get::<_, Option<Vec<u8>>>(2).map(id_of).transpose()?,
            })
        })
        .transpose()
    }

    async fn attach_file(&self, team: Id, file: Id, project: Id) -> Result<bool> {
        Ok(self
            .client
            .lock()
            .await
            .execute(
                "UPDATE files SET project_id = $3 WHERE team_id = $1 AND id = $2 AND project_id IS NULL",
                &[&team.as_slice(), &file.as_slice(), &project.as_slice()],
            )
            .await?
            == 1)
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

    async fn set_access(
        &self,
        team: Id,
        project: Id,
        access: Access,
        admin: bool,
        quotas: Quotas,
    ) -> Result<()> {
        let mut client = self.client.lock().await;
        let tx = client.transaction().await?;
        tx.query_one(
            "SELECT 1 FROM teams WHERE id = $1 FOR UPDATE",
            &[&team.as_slice()],
        )
        .await?;
        let before = access_of(&tx, &team, &project).await?;
        if before.is_some() && !admin {
            return Err(StoreError::NotAdmin);
        }
        register_project(&tx, &team, &project, &quotas).await?;
        let was = before.map_or(0, |a| a.devices_bytes().len() as i64);
        charge(
            &tx,
            &team,
            access.devices_bytes().len() as i64 - was,
            &quotas,
        )
        .await?;
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
        quotas: Quotas,
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
            register_project(&tx, &team, &project, &quotas).await?;
            charge(&tx, &team, ROW_BYTES as i64, &quotas).await?;
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

    async fn next_key(&self, team: Id, project: Id, quotas: Quotas) -> Result<u64> {
        let mut client = self.client.lock().await;
        let tx = client.transaction().await?;
        tx.query_one(
            "SELECT 1 FROM teams WHERE id = $1 FOR UPDATE",
            &[&team.as_slice()],
        )
        .await?;
        register_project(&tx, &team, &project, &quotas).await?;
        let n: i64 = tx
            .query_one(
                "INSERT INTO project_keys (team_id, project_id, n) VALUES ($1, $2, 1)
                 ON CONFLICT (team_id, project_id) DO UPDATE SET n = project_keys.n + 1 RETURNING n",
                &[&team.as_slice(), &project.as_slice()],
            )
            .await?
            .get(0);
        tx.commit().await?;
        Ok(n as u64)
    }
}
