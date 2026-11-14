//! An object of a project (a ticket, a spec page, a memory), encrypted and signed by its author.
//!
//! - Its key: HKDF of the project's key of its epoch, and of its id (`keys.rs`).
//! - XChaCha20-Poly1305; the associated data carry the project, the object, its version, the epoch, the head the
//!   author knew and the author's key: the server can neither move an object from a project to another, nor pass it
//!   for another or for an older version, nor change the head it cites.
//! - The author signs the ciphertext with the signature key of their leaf (Ed25519): the server cannot forge an
//!   object, and a member cannot write in the name of another.

use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::identity::{ObjectId, ProjectId};
use crate::keys::{aead_open, aead_seal, object_key, EpochRecord};
use crate::Error;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectHeader {
    pub project: ProjectId,
    pub object: ObjectId,
    pub version: u64,
    pub epoch: u64,
    /// The head of `epoch` the author knew (`keys::head`).
    #[serde(with = "hex::serde")]
    pub head: [u8; 32],
    /// The author's signature key, a leaf of `epoch`.
    #[serde(with = "hex::serde")]
    pub author: Vec<u8>,
}

impl ObjectHeader {
    /// The associated data: a label, then each field at a fixed length (the author's key, 32 bytes for Ed25519,
    /// prefixed by its length).
    pub fn aad(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(160);
        out.extend_from_slice(b"escouade/object/v1\0");
        out.extend_from_slice(&self.project.0);
        out.extend_from_slice(&self.object.0);
        out.extend_from_slice(&self.version.to_be_bytes());
        out.extend_from_slice(&self.epoch.to_be_bytes());
        out.extend_from_slice(&self.head);
        out.extend_from_slice(&(self.author.len() as u16).to_be_bytes());
        out.extend_from_slice(&self.author);
        out
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedObject {
    pub header: ObjectHeader,
    #[serde(with = "hex::serde")]
    pub nonce: [u8; 24],
    #[serde(with = "hex::serde")]
    pub ciphertext: Vec<u8>,
    #[serde(with = "hex::serde")]
    pub signature: Vec<u8>,
}

impl SealedObject {
    /// What the author signs.
    pub fn signed_bytes(header: &ObjectHeader, nonce: &[u8; 24], ciphertext: &[u8]) -> Vec<u8> {
        let mut out = b"escouade/object-signature/v1\0".to_vec();
        out.extend_from_slice(&header.aad());
        out.extend_from_slice(nonce);
        out.extend_from_slice(ciphertext);
        out
    }

    /// Encrypts under the record's project key, then signs with `sign` (the author's leaf key).
    pub fn seal(
        record: &EpochRecord,
        header: ObjectHeader,
        nonce: [u8; 24],
        plaintext: &[u8],
        sign: impl FnOnce(&[u8]) -> Result<Vec<u8>, Error>,
    ) -> Result<Self, Error> {
        if header.epoch != record.epoch || header.head != record.head {
            return Err(Error::Encoding(
                "an object is sealed under the epoch it names".into(),
            ));
        }
        let key = object_key(&record.project_key(&header.project), &header.object);
        let ciphertext = aead_seal(&key, &nonce, &header.aad(), plaintext);
        let signature = sign(&Self::signed_bytes(&header, &nonce, &ciphertext))?;
        Ok(Self {
            header,
            nonce,
            ciphertext,
            signature,
        })
    }

    /// Checks the signature by a leaf of the record's epoch, then decrypts. The head is checked by the caller against
    /// what it knows of the epoch (`TeamDevice::open`), before this.
    pub fn open(&self, record: &EpochRecord) -> Result<Vec<u8>, Error> {
        if self.header.epoch != record.epoch {
            return Err(Error::UnknownEpoch(self.header.epoch));
        }
        if !record.has_author(&self.header.author) {
            return Err(Error::BadSignature);
        }
        verify_ed25519(
            &self.header.author,
            &Self::signed_bytes(&self.header, &self.nonce, &self.ciphertext),
            &self.signature,
        )?;
        let key = object_key(
            &record.project_key(&self.header.project),
            &self.header.object,
        );
        aead_open(&key, &self.nonce, &self.header.aad(), &self.ciphertext)
    }
}

pub fn verify_ed25519(public: &[u8], message: &[u8], signature: &[u8]) -> Result<(), Error> {
    let public: [u8; 32] = public.try_into().map_err(|_| Error::BadSignature)?;
    let key = VerifyingKey::from_bytes(&public).map_err(|_| Error::BadSignature)?;
    let signature = Signature::from_slice(signature).map_err(|_| Error::BadSignature)?;
    key.verify_strict(message, &signature)
        .map_err(|_| Error::BadSignature)
}
