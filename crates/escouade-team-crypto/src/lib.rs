//! Escouade's team server: a mock-up of its cryptography (TS-0a, `docs/TEAM-SERVER.md` §3 and §16).
//!
//! The team is an MLS group (RFC 9420, by `openmls`); what MLS does not do is ours, above it: the roster and its
//! rules, the project keys by epoch, the history for a new device, the invitation by an authentication secret, the
//! recovery key of 24 words and the optional escrow key, the detection of a fork or a rollback. `README.md` says
//! what each part does, its choices, and the points for the external audit (TS-20).
//!
//! Wired into nothing yet: TS-2 makes of it the `escouade-team` crate of the client.

pub mod content;
pub mod device;
pub mod history;
pub mod identity;
pub mod invite;
pub mod keys;
pub mod policy;
pub mod recovery;
pub mod relay;
pub mod roster;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod vectors;

use openmls::prelude::Ciphersuite;

pub use device::{Checkpoint, HeadCheck, Processed, Publication, TeamDevice};
pub use identity::{DeviceId, DeviceKind, Identity, MemberId, ObjectId, ProjectId};
pub use invite::{InvitationError, InviteLink, Purpose};
pub use recovery::{EscrowKey, RecoveryKey};
pub use roster::{Role, Roster, TeamFingerprint};

/// The suite: `MLS_128_DHKEMX25519_CHACHA20POLY1305_SHA256_Ed25519` (0x0003, RFC 9420 §17.1). X25519 and Ed25519
/// as the first design chose (§3.2), ChaCha20-Poly1305 as the content's XChaCha20-Poly1305: constant time without
/// AES instructions, on every target (mobile included), and all of it in RustCrypto (`openmls_rust_crypto`).
pub const CIPHERSUITE: Ciphersuite =
    Ciphersuite::MLS_128_DHKEMX25519_CHACHA20POLY1305_SHA256_Ed25519;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("MLS: {0}")]
    Mls(String),
    #[error("refused by the team's rules: {0}")]
    Policy(String),
    #[error("the server shows two versions of the team (epoch {epoch}): contact an administrator")]
    Fork { epoch: u64 },
    #[error(
        "the server shows epoch {shown}, older than epoch {seen} this device has seen: refused"
    )]
    Rollback { seen: u64, shown: u64 },
    #[error("a commit of epoch {epoch}, older than this device's {current}: a replay, refused")]
    Stale { epoch: u64, current: u64 },
    #[error("epoch {0} is unknown to this device")]
    UnknownEpoch(u64),
    #[error("epoch {0} is ahead of this device: process the commits first")]
    Behind(u64),
    #[error("the history link of epoch {0} is missing")]
    MissingHistory(u64),
    #[error("decryption failed")]
    Decrypt,
    #[error("bad signature, or an author who was not a member of the epoch")]
    BadSignature,
    #[error(transparent)]
    Invitation(#[from] InvitationError),
    #[error("these words are not a recovery key")]
    Words,
    #[error("only an administrator can do this")]
    NotAdmin,
    #[error("this device is not in the team")]
    NotInTeam,
    #[error("the server already holds a commit for this epoch")]
    Conflict,
    #[error("encoding: {0}")]
    Encoding(String),
    #[error("no randomness")]
    Random,
}

pub(crate) fn mls(e: impl std::fmt::Debug) -> Error {
    Error::Mls(format!("{e:?}"))
}
