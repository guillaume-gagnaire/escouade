//! The keys of an epoch: what a device keeps of each epoch of the team's group, and the project keys made of it.
//!
//! MLS forgets an epoch's secrets when the next one starts (forward secrecy). A Kanban and a spec are a state that
//! lasts: their content must stay readable. So at each epoch, a device exports two secrets from MLS (RFC 9420
//! §8.5, the exporter) and keeps them in its [`EpochRecord`]:
//! - the **content secret**, of which each project's key for the epoch is derived (HKDF), then each object's key;
//! - the **history key**, which opens the history link of the epoch (`history.rs`): the record of the epoch before.
//!
//! A member removed in a commit never has the secrets of the epochs after it: they come from the commit's path
//! secrets, encrypted to the remaining leaves only.

use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::identity::{ObjectId, ProjectId};
use crate::Error;

/// The exporter's labels (RFC 9420 §8.5): what is ours of the epoch's secrets.
pub const CONTENT_LABEL: &str = "escouade/content/v1";
pub const HISTORY_LABEL: &str = "escouade/history/v1";

/// A leaf of an epoch: its identity and its signature key, to check who wrote an object of that epoch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Zeroize)]
pub struct EpochMember {
    #[serde(with = "hex::serde")]
    pub identity: Vec<u8>,
    #[serde(with = "hex::serde")]
    pub signature_key: Vec<u8>,
}

/// What a device keeps of an epoch, wiped from memory when dropped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct EpochRecord {
    pub epoch: u64,
    #[serde(with = "hex::serde")]
    pub content_secret: [u8; 32],
    #[serde(with = "hex::serde")]
    pub history_key: [u8; 32],
    /// The epoch's head ([`head`]): what an author cites, to tell a fork.
    #[serde(with = "hex::serde")]
    pub head: [u8; 32],
    pub members: Vec<EpochMember>,
}

impl EpochRecord {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("a record always serializes")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        serde_json::from_slice(bytes).map_err(|e| Error::Encoding(format!("epoch record: {e}")))
    }

    /// The key of a project for this epoch.
    pub fn project_key(&self, project: &ProjectId) -> [u8; 32] {
        project_key(&self.content_secret, project)
    }

    pub fn has_author(&self, signature_key: &[u8]) -> bool {
        self.members
            .iter()
            .any(|m| m.signature_key == signature_key)
    }
}

/// The head of an epoch: SHA-256 of the group, the epoch and its epoch authenticator (RFC 9420 §8.7), which all the
/// members of an epoch share and nobody else can make. Two members shown two different commits for one epoch have
/// two different heads from then on: the fork shows as soon as one cites a head the other does not know. A hash, so
/// that it can travel in the clear beside an object without giving the authenticator away.
pub fn head(group_id: &[u8], epoch: u64, epoch_authenticator: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"escouade/head/v1\0");
    h.update((group_id.len() as u32).to_be_bytes());
    h.update(group_id);
    h.update(epoch.to_be_bytes());
    h.update(epoch_authenticator);
    h.finalize().into()
}

/// HKDF-Expand (SHA-256) of a uniform secret (an MLS export, or a key made of one), under a label and an id.
pub fn expand(secret: &[u8; 32], label: &[u8], id: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::from_prk(secret).expect("32 bytes is a valid PRK for SHA-256");
    let mut out = [0u8; 32];
    let mut info = Vec::with_capacity(label.len() + 1 + id.len());
    info.extend_from_slice(label);
    info.push(0);
    info.extend_from_slice(id);
    hk.expand(&info, &mut out)
        .expect("32 bytes is a valid HKDF length");
    out
}

pub fn project_key(content_secret: &[u8; 32], project: &ProjectId) -> [u8; 32] {
    expand(content_secret, b"escouade/project/v1", &project.0)
}

pub fn object_key(project_key: &[u8; 32], object: &ObjectId) -> [u8; 32] {
    expand(project_key, b"escouade/object/v1", &object.0)
}

/// XChaCha20-Poly1305: a 192-bit random nonce never repeats in practice, without a counter to keep.
pub fn aead_seal(key: &[u8; 32], nonce: &[u8; 24], aad: &[u8], plaintext: &[u8]) -> Vec<u8> {
    XChaCha20Poly1305::new(key.into())
        .encrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .expect("XChaCha20-Poly1305 encrypts any message that fits in memory")
}

pub fn aead_open(
    key: &[u8; 32],
    nonce: &[u8; 24],
    aad: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, Error> {
    XChaCha20Poly1305::new(key.into())
        .decrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| Error::Decrypt)
}
