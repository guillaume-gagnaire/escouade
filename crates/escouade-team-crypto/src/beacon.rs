//! The head beacons (point 7 of the audit in `README.md`): at regular hours, and at each new epoch, every device
//! says in the team's stream the epoch it is at and its head, signed by its leaf.
//!
//! A fork shows as soon as something crosses from a branch to the other (an object, a history link, a commit). A
//! beacon is such a crossing, made on purpose and often: a member on the other branch who reads it sees a head it
//! does not have for that epoch, signed by a leaf it knows, and says "the server shows two versions of the team".
//! The server can still hide the beacons of one half from the other for good: what is left then is to compare the
//! six words of the head out of band (`TeamDevice::head_words`).
//!
//! A beacon carries nothing secret: the epoch the server knows already, and the head, a hash.

use serde::{Deserialize, Serialize};

use crate::content::verify_ed25519;
use crate::Error;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Beacon {
    pub epoch: u64,
    #[serde(with = "hex::serde")]
    pub head: [u8; 32],
    /// Seconds since the Unix epoch, on the device that signs.
    pub at: u64,
    /// The signature key of the device's leaf.
    #[serde(with = "hex::serde")]
    pub author: Vec<u8>,
    #[serde(with = "hex::serde")]
    pub signature: Vec<u8>,
}

impl Beacon {
    /// What the leaf signs: a label, the group, the epoch, the head, the hour and the author's key.
    pub fn signed_bytes(
        group_id: &[u8],
        epoch: u64,
        head: &[u8; 32],
        at: u64,
        author: &[u8],
    ) -> Vec<u8> {
        let mut out = b"escouade/beacon/v1\0".to_vec();
        out.extend_from_slice(&(group_id.len() as u32).to_be_bytes());
        out.extend_from_slice(group_id);
        out.extend_from_slice(&epoch.to_be_bytes());
        out.extend_from_slice(head);
        out.extend_from_slice(&at.to_be_bytes());
        out.extend_from_slice(&(author.len() as u16).to_be_bytes());
        out.extend_from_slice(author);
        out
    }

    /// Signs with `sign`, the author's leaf key (`author` its public half).
    pub fn sign(
        group_id: &[u8],
        epoch: u64,
        head: [u8; 32],
        at: u64,
        author: Vec<u8>,
        sign: impl FnOnce(&[u8]) -> Result<Vec<u8>, Error>,
    ) -> Result<Self, Error> {
        let signature = sign(&Self::signed_bytes(group_id, epoch, &head, at, &author))?;
        Ok(Self {
            epoch,
            head,
            at,
            author,
            signature,
        })
    }

    pub fn verify(&self, group_id: &[u8]) -> Result<(), Error> {
        verify_ed25519(
            &self.author,
            &Self::signed_bytes(group_id, self.epoch, &self.head, self.at, &self.author),
            &self.signature,
        )
    }
}
