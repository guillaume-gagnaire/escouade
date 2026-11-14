//! The history: how a new device reads the epochs before its arrival (`docs/TEAM-SERVER.md` §3.9).
//!
//! MLS gives a newcomer the secrets of the epoch it joins, never those of the epochs before. Above it, each epoch
//! `e > 0` has a **history link**: the record of epoch `e - 1` (`keys::EpochRecord`: its content secret, its history
//! key, its head and its leaves), encrypted under a key made of the history key of epoch `e`. Whoever holds epoch
//! `e` walks back the chain to the team's first epoch: one symmetric decryption per epoch, nothing to encrypt again
//! for each new device, and the server only stores the links.
//!
//! - The device that moves the group to epoch `e` (the committer) writes its link; any other member of `e` can write
//!   it if it is missing. Only a member of `e` can: the server cannot forge a link.
//! - A member removed before epoch `e` never has its history key: it reads no link from `e` on, so nothing new.
//! - It is a deliberate choice against MLS's forward secrecy for the content: whoever holds a later epoch reads the
//!   earlier ones, as the persistent project keys of the first design did (C8-A: the history is not encrypted again
//!   at a rotation). A point for the external audit.

use serde::{Deserialize, Serialize};

use crate::keys::{aead_open, aead_seal, expand, EpochRecord};
use crate::Error;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryLink {
    /// The epoch whose history key opens it; it holds the record of `epoch - 1`.
    pub epoch: u64,
    #[serde(with = "hex::serde")]
    pub nonce: [u8; 24],
    #[serde(with = "hex::serde")]
    pub ciphertext: Vec<u8>,
}

fn link_key(group_id: &[u8], record: &EpochRecord) -> [u8; 32] {
    expand(&record.history_key, b"escouade/history-link/v1", group_id)
}

fn link_aad(group_id: &[u8], epoch: u64) -> Vec<u8> {
    let mut out = b"escouade/history-link/v1\0".to_vec();
    out.extend_from_slice(&(group_id.len() as u32).to_be_bytes());
    out.extend_from_slice(group_id);
    out.extend_from_slice(&epoch.to_be_bytes());
    out
}

impl HistoryLink {
    /// The link of `current.epoch`, holding `previous`.
    pub fn make(
        group_id: &[u8],
        current: &EpochRecord,
        previous: &EpochRecord,
        nonce: [u8; 24],
    ) -> Result<Self, Error> {
        if current.epoch == 0 || previous.epoch + 1 != current.epoch {
            return Err(Error::Encoding(
                "a history link joins an epoch to the one before".into(),
            ));
        }
        let ciphertext = aead_seal(
            &link_key(group_id, current),
            &nonce,
            &link_aad(group_id, current.epoch),
            &previous.to_bytes(),
        );
        Ok(Self {
            epoch: current.epoch,
            nonce,
            ciphertext,
        })
    }

    /// The record of `current.epoch - 1`.
    pub fn open(&self, group_id: &[u8], current: &EpochRecord) -> Result<EpochRecord, Error> {
        if self.epoch != current.epoch {
            return Err(Error::UnknownEpoch(self.epoch));
        }
        let plain = aead_open(
            &link_key(group_id, current),
            &self.nonce,
            &link_aad(group_id, self.epoch),
            &self.ciphertext,
        )?;
        let previous = EpochRecord::from_bytes(&plain)?;
        if previous.epoch + 1 != self.epoch {
            return Err(Error::Encoding(
                "a history link holds the epoch before its own".into(),
            ));
        }
        Ok(previous)
    }
}

/// Walks back from `from` to epoch `stop` (included), asking `fetch` for each link: the records, newest first,
/// without `from`. A missing or forged link stops the walk with its error: the history before it stays unread.
pub fn walk_back(
    group_id: &[u8],
    from: &EpochRecord,
    stop: u64,
    mut fetch: impl FnMut(u64) -> Option<HistoryLink>,
) -> Result<Vec<EpochRecord>, Error> {
    let mut out = Vec::new();
    let mut current = from.clone();
    while current.epoch > stop {
        let link = fetch(current.epoch).ok_or(Error::MissingHistory(current.epoch))?;
        let previous = link.open(group_id, &current)?;
        out.push(previous.clone());
        current = previous;
    }
    Ok(out)
}
