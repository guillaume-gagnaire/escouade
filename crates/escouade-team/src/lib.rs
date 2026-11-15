//! Escouade's team client (TS-2, `docs/TEAM-SERVER.md` §2.4, §2.6, §3 and §16).
//!
//! The team is an MLS group; its cryptography is `escouade-team-crypto` (the roster and its rules, the project keys
//! by epoch, the history, the invitations, the recovery, the forks and the rollbacks), pure and without I/O. This
//! crate puts around it what a device needs to live in a team:
//! - its **keys in the keychain** (`keychain.rs`), behind a trait the core gives with the app's `SecretStore`;
//! - its **state on the disk** (`storage.rs`): openmls's storage in SQLite, encrypted, the past epochs' keys erased;
//!   the **checkpoint** in the keychain, apart from what a backup brings back;
//! - the **client of the server** (`client.rs`): enrolment, signed challenge and session, streams, files, live;
//! - the **team** (`team.rs`): reading the stream, committing (and playing again after a 409), the verified journal
//!   of the members, the rotation, the revocation, the missing history links, the head beacons.
//!
//! Wired into nothing yet: TS-3 and TS-4 show it, TS-5 syncs the projects' operations on it. `README.md` says what
//! is done and what is left.

pub mod client;
#[cfg(any(test, feature = "fuzzing"))]
pub mod fuzzing;
pub mod keychain;
pub mod storage;
pub mod team;
pub mod wire;

pub use client::{
    ClientError, HttpTransport, LiveEvent, LiveFeed, LiveOptions, ServerClient, Transport,
};
pub use escouade_team_crypto as crypto;
pub use escouade_team_crypto::Error as CryptoError;
pub use keychain::{KeyPlace, Keychain, MemoryKeychain, Vault};
pub use storage::{SqliteStorage, StorageError, TeamProvider};
pub use team::{Applied, Options, Setup, Synced, Team};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Crypto(#[from] CryptoError),
    #[error(transparent)]
    Client(#[from] ClientError),
    #[error("the team's state: {0}")]
    Storage(String),
    #[error("the keychain: {0}")]
    Keychain(String),
    #[error("encoding: {0}")]
    Encoding(String),
    #[error("this device has no keys for this team")]
    NoKeys,
    #[error("this device has no state for this team")]
    NoState,
    #[error(
        "the server no longer shows entry {seen} of the team's stream, which this device read: it went back, refused"
    )]
    ServerRollback { seen: u64 },
    #[error("the server shows entry {seq} of the team's stream changed since this device read it: refused")]
    Rewritten { seq: u64 },
    #[error("the server shows what cannot be: {0}")]
    Server(String),
    #[error("{0}")]
    Refused(String),
    #[error("other commits keep taking the epoch: try again later")]
    Busy,
}

impl From<StorageError> for Error {
    fn from(e: StorageError) -> Self {
        Error::Storage(e.to_string())
    }
}
