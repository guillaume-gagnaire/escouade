//! The void of a commit (SR2-02): an administrator's word that the commit the server kept for an epoch is null.
//!
//! The server keeps the first commit of an epoch without reading it. One that MLS or the team's rules refuse (bytes of
//! nothing, or a member's commit that does what only an administrator may) changes nothing on a device: it is kept as
//! **null** (its digest, who posted it), the group stays at its epoch, and the reading goes on. An administrator's
//! device then signs a void of it with its leaf: the server, which takes a void from an administrator's device only,
//! frees the epoch again, kept to the administrators until their next commit. Every device checks the void (signed by
//! a leaf of an administrator of that epoch, for that group): one that had merged the commit the void names is told it
//! shows another version of the team than the administrators' (a fork). A void that does not hold changes nothing.
//!
//! A void carries nothing secret: the epoch and the number the server knows already, and the digest of bytes it holds.

use serde::{Deserialize, Serialize};

use crate::content::verify_ed25519;
use crate::Error;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Void {
    /// The epoch the null commit was for (the group is still at it).
    pub epoch: u64,
    /// The number the server gave it in the stream.
    pub seq: u64,
    /// The SHA-256 of its bytes.
    #[serde(with = "hex::serde")]
    pub digest: [u8; 32],
    /// The signature key of the administrator's leaf.
    #[serde(with = "hex::serde")]
    pub author: Vec<u8>,
    #[serde(with = "hex::serde")]
    pub signature: Vec<u8>,
}

impl Void {
    /// What the leaf signs: a label, the group, the epoch, the number, the digest and the author's key.
    pub fn signed_bytes(
        group_id: &[u8],
        epoch: u64,
        seq: u64,
        digest: &[u8; 32],
        author: &[u8],
    ) -> Vec<u8> {
        let mut out = b"escouade/void/v1\0".to_vec();
        out.extend_from_slice(&(group_id.len() as u32).to_be_bytes());
        out.extend_from_slice(group_id);
        out.extend_from_slice(&epoch.to_be_bytes());
        out.extend_from_slice(&seq.to_be_bytes());
        out.extend_from_slice(digest);
        out.extend_from_slice(&(author.len() as u16).to_be_bytes());
        out.extend_from_slice(author);
        out
    }

    /// Signs with `sign`, the author's leaf key (`author` its public half).
    pub fn sign(
        group_id: &[u8],
        epoch: u64,
        seq: u64,
        digest: [u8; 32],
        author: Vec<u8>,
        sign: impl FnOnce(&[u8]) -> Result<Vec<u8>, Error>,
    ) -> Result<Self, Error> {
        let signature = sign(&Self::signed_bytes(group_id, epoch, seq, &digest, &author))?;
        Ok(Self {
            epoch,
            seq,
            digest,
            author,
            signature,
        })
    }

    pub fn verify(&self, group_id: &[u8]) -> Result<(), Error> {
        verify_ed25519(
            &self.author,
            &Self::signed_bytes(group_id, self.epoch, self.seq, &self.digest, &self.author),
            &self.signature,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer as _, SigningKey};

    fn signed(group: &[u8]) -> (Void, SigningKey) {
        let key = SigningKey::from_bytes(&[7; 32]);
        let void = Void::sign(
            group,
            4,
            12,
            [9; 32],
            key.verifying_key().to_bytes().to_vec(),
            |b| Ok(key.sign(b).to_bytes().to_vec()),
        )
        .unwrap();
        (void, key)
    }

    #[test]
    fn a_void_holds_for_its_group_alone() {
        let (void, _) = signed(b"group");
        void.verify(b"group").unwrap();
        assert!(matches!(void.verify(b"other"), Err(Error::BadSignature)));
    }

    #[test]
    fn a_void_changed_in_any_field_no_longer_holds() {
        let (void, _) = signed(b"group");
        for changed in [
            Void {
                epoch: 5,
                ..void.clone()
            },
            Void {
                seq: 13,
                ..void.clone()
            },
            Void {
                digest: [8; 32],
                ..void.clone()
            },
        ] {
            assert!(matches!(changed.verify(b"group"), Err(Error::BadSignature)));
        }
    }
}
