//! What the server is started with: its base, its files, its bounds and its rates.

use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Debug)]
pub enum DatabaseConfig {
    /// A SQLite file, in WAL.
    Sqlite(PathBuf),
    /// A PostgreSQL address (`postgres://user:pass@host/db`), and a schema to work in (the tests use one each).
    Postgres { url: String, schema: Option<String> },
}

#[derive(Clone, Debug)]
pub enum FilesConfig {
    /// A folder on the disk.
    Disk(PathBuf),
    /// A bucket of an S3-compatible store (MinIO, Garage, AWS), path-style.
    S3(S3Config),
}

#[derive(Clone, Debug)]
pub struct S3Config {
    /// `https://s3.example.org` or `http://127.0.0.1:9000`.
    pub endpoint: String,
    pub bucket: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    /// Put before every key (`team-server/`), empty for none.
    pub prefix: String,
}

/// The bounds: every size the server accepts, every count it keeps.
#[derive(Clone, Debug)]
pub struct Limits {
    /// The payload of one entry of a stream (an MLS message, an encrypted operation).
    pub max_entry: usize,
    /// One encrypted file.
    pub max_file: usize,
    /// What a team keeps in all: its entries and its files.
    pub team_quota: u64,
    /// One page of a stream read: so many entries at most, and so many bytes (one entry always goes out).
    pub page_entries: u32,
    pub page_bytes: usize,
    pub max_devices: u32,
    pub max_projects: u32,
    /// Live connections of one device.
    pub max_live_per_device: u32,
    pub challenge_ttl: Duration,
    pub session_ttl: Duration,
    /// The longest life of an enrolment token an administrator asks for.
    pub enrollment_ttl_max: Duration,
    /// The lease of a ticket's claim (TS-6, §4.2): 10 minutes, renewed every 3 while its agent runs.
    pub claim_lease: Duration,
    /// The lease of a ticket handed to a teammate (TS-17, §11): 14 days, while they have not taken it.
    pub hand_lease: Duration,
    /// How long the messages a snapshot holds stay after it (SR2-01): 30 days. A device that finds the snapshot
    /// unreadable reads them instead, and flags it: a flagged snapshot never takes their place.
    pub snapshot_retention: Duration,
    /// The time any body has to come whole (SR2-07), plus what its length takes at `body_min_rate` bytes a second.
    pub body_timeout: Duration,
    pub body_min_rate: u64,
    /// A connection where nothing passes for so long is closed (SR2-07).
    pub idle_timeout: Duration,
    /// Connections open at once, in all and from one address (an IPv6 address by its /64). Behind a trusted proxy
    /// (`trust_proxy`), only the bound in all holds: every connection comes from the proxy.
    pub max_connections: usize,
    pub max_connections_per_address: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_entry: 1 << 20,
            max_file: 25 << 20,
            team_quota: 2 << 30,
            page_entries: 500,
            page_bytes: 8 << 20,
            max_devices: 500,
            max_projects: 10_000,
            max_live_per_device: 4,
            challenge_ttl: Duration::from_secs(60),
            session_ttl: Duration::from_secs(15 * 60),
            enrollment_ttl_max: Duration::from_secs(7 * 24 * 3600),
            claim_lease: Duration::from_secs(10 * 60),
            hand_lease: Duration::from_secs(14 * 24 * 3600),
            snapshot_retention: Duration::from_secs(30 * 24 * 3600),
            body_timeout: Duration::from_secs(30),
            body_min_rate: 64 << 10,
            idle_timeout: Duration::from_secs(60),
            max_connections: 8192,
            max_connections_per_address: 512,
        }
    }
}

/// A token bucket: so many requests a second on average, so many at once.
pub use crate::transport::rate::Rate;

#[derive(Clone, Copy, Debug)]
pub struct Rates {
    pub per_address: Rate,
    pub per_device: Rate,
    /// The steps of the pairing relay without a session (reading a slot's message, its try), by address (SR2-05):
    /// a new device makes one of each.
    pub pairing: Rate,
}

impl Default for Rates {
    fn default() -> Self {
        Rates {
            per_address: Rate {
                per_second: 30.0,
                burst: 120.0,
            },
            per_device: Rate {
                per_second: 20.0,
                burst: 80.0,
            },
            pairing: Rate {
                per_second: 0.2,
                burst: 10.0,
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub database: DatabaseConfig,
    pub files: FilesConfig,
    pub limits: Limits,
    pub rates: Rates,
    /// Behind a reverse proxy (Caddy, nginx): the client's address is the last of `X-Forwarded-For`.
    pub trust_proxy: bool,
    /// The data folder: the base and the files by default, and always the server's audit key and the audit's buffers
    /// (AU-2).
    pub data: PathBuf,
    /// The server's own audit (AU-2), off by default.
    pub audit: crate::audit::AuditConfig,
}

impl Config {
    /// SQLite and the files in one folder: `team.db` and `files/`.
    pub fn in_folder(folder: impl Into<PathBuf>) -> Self {
        let folder = folder.into();
        Config {
            database: DatabaseConfig::Sqlite(folder.join("team.db")),
            files: FilesConfig::Disk(folder.join("files")),
            data: folder.clone(),
            limits: Limits::default(),
            rates: Rates::default(),
            trust_proxy: false,
            audit: Default::default(),
        }
    }
}
