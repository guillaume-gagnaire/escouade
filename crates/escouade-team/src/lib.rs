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
//!   of the members, the rotation, the revocation, the missing history links, the head beacons;
//! - the **projects** (`replica.rs`, `team/sync.rs`, TS-5): the journal of operations encrypted and signed, chained by
//!   device, the local copy in SQLite, the outbox offline, the snapshots, a project's own MLS group when the team keeps
//!   it to some members.
//!
//! The core wires it in (`src-tauri/src/teams.rs`). `README.md` says what is done and what is left.

pub mod audit;
pub mod breaker;
pub mod catalog;
pub mod client;
pub mod costs;
pub mod escrow;
#[cfg(any(test, feature = "fuzzing"))]
pub mod fuzzing;
pub mod handoff;
pub mod keychain;
mod keyring;
pub mod lease;
pub mod merge;
pub mod pairing;
pub mod plugins;
pub mod proposals;
pub mod replica;
pub mod settings;
pub mod signing;
pub mod storage;
pub mod team;
pub mod wire;

pub use client::{
    ClientError, HttpTransport, LiveEvent, LiveFeed, LiveOptions, ServerClient, Transport,
};
pub use escouade_team_crypto as crypto;
pub use escouade_team_crypto::Error as CryptoError;
pub use keychain::{KeyPlace, Keychain, MemoryKeychain, Vault};
pub use lease::{LeaseError, LeaseKeeper, LeaseState};
pub use pairing::{PairCode, PairState};
pub use storage::{SqliteStorage, StorageError, TeamProvider};
pub use team::{
    Applied, DropAlert, Invited, NullAlert, Options, RefusalAlert, Served, Setup, SnapshotAlert,
    Synced, Team,
};

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
    #[error("this invitation was already used: ask its administrator for a new one")]
    InvitationUsed,
    #[error("this invitation is unknown to the server, expired, or not registered yet by an administrator")]
    InvitationUnknown,
    #[error("the code does not match: ask for a new one (a code has one try)")]
    WrongCode,
    #[error("this code had its one try already: ask for a new one")]
    CodeTried,
    #[error("no such code: its time is over, or it served")]
    CodeGone,
    #[error("the server holds no recovery wrap of this team")]
    NoRecoveryWrap,
    // === A project's journal (TS-5) ===
    #[error("the server plays again an operation of this project (entry {seq}): refused")]
    Replayed { seq: u64 },
    #[error("the server reorders or drops the operations of a device of this project (entry {seq}): refused")]
    Reordered { seq: u64 },
    #[error("the server dropped an operation this device sent (number {seq}): refused")]
    Withheld { seq: u64 },
    #[error(
        "two operations of one device take the same place in its chain (entry {seq}): refused"
    )]
    Equivocation { seq: u64 },
    #[error("an operation of this project that no member signed (entry {seq}): refused")]
    Forged { seq: u64 },
    #[error("this device has no access to this project")]
    NoAccess,
    #[error("this project is not on this device")]
    NoProject,
}

/// What an error says of the server, for the interface: a fork, a rollback, a rewritten entry are the server showing
/// what it should not, and the device stops reading the team (« Le serveur montre deux versions de l'équipe »).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alarm {
    /// Two versions of the team.
    Fork { epoch: u64 },
    /// The server went back, or showed an older state than this device saw.
    Rollback,
    /// An entry read changed.
    Rewritten,
    /// A project's journal played again, reordered, cut, forged (TS-5).
    Journal,
}

impl Error {
    pub fn alarm(&self) -> Option<Alarm> {
        match self {
            Error::Crypto(CryptoError::Fork { epoch }) => Some(Alarm::Fork { epoch: *epoch }),
            Error::Crypto(CryptoError::Rollback { .. }) | Error::ServerRollback { .. } => {
                Some(Alarm::Rollback)
            }
            Error::Rewritten { .. } => Some(Alarm::Rewritten),
            Error::Replayed { .. }
            | Error::Reordered { .. }
            | Error::Withheld { .. }
            | Error::Equivocation { .. }
            | Error::Forged { .. } => Some(Alarm::Journal),
            _ => None,
        }
    }
}

impl From<StorageError> for Error {
    fn from(e: StorageError) -> Self {
        Error::Storage(e.to_string())
    }
}
