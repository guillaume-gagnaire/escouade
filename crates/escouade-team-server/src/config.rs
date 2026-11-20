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
        }
    }
}

/// A token bucket: so many requests a second on average, so many at once.
pub use crate::transport::rate::Rate;

#[derive(Clone, Copy, Debug)]
pub struct Rates {
    pub per_address: Rate,
    pub per_device: Rate,
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
}

impl Config {
    /// SQLite and the files in one folder: `team.db` and `files/`.
    pub fn in_folder(folder: impl Into<PathBuf>) -> Self {
        let folder = folder.into();
        Config {
            database: DatabaseConfig::Sqlite(folder.join("team.db")),
            files: FilesConfig::Disk(folder.join("files")),
            limits: Limits::default(),
            rates: Rates::default(),
            trust_proxy: false,
        }
    }
}
