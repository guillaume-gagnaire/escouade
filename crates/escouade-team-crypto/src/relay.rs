//! The server, as the mock-up plays it: blind, in memory. It orders the commits (MLS's delivery service, RFC 9420
//! §3), keeps the history links, the wraps, the join requests, the Welcomes and the objects, and reads none of them.
//! Its fields are public: the tests play a server that lies (a fork, a rollback, a key put in place of another).

use std::collections::BTreeMap;

use crate::content::SealedObject;
use crate::device::Publication;
use crate::history::HistoryLink;
use crate::invite::{JoinRequest, WelcomeDelivery};
use crate::recovery::{EscrowWrap, RecoveryWrap};
use crate::Error;

#[derive(Debug, Default, Clone)]
pub struct Relay {
    /// The commit log: `commits[e]` moves the group from epoch `e` to `e + 1`.
    pub commits: Vec<Vec<u8>>,
    pub history: BTreeMap<u64, HistoryLink>,
    pub recovery: BTreeMap<u64, RecoveryWrap>,
    pub escrow: BTreeMap<u64, EscrowWrap>,
    pub join_requests: Vec<JoinRequest>,
    pub welcomes: BTreeMap<[u8; 16], WelcomeDelivery>,
    pub objects: Vec<SealedObject>,
}

impl Relay {
    /// Takes what a device publishes. The first commit for an epoch wins; a second one is refused, and its device
    /// starts again from the log.
    pub fn publish(&mut self, publication: Publication) -> Result<(), Error> {
        if let Some((epoch, commit)) = publication.commit {
            if epoch as usize != self.commits.len() {
                return Err(Error::Conflict);
            }
            self.commits.push(commit);
        }
        if let Some((id, welcome)) = publication.welcome {
            self.welcomes.insert(id, welcome);
        }
        if let Some(link) = publication.history {
            self.history.insert(link.epoch, link);
        }
        if let Some(wrap) = publication.recovery {
            self.recovery.insert(wrap.epoch, wrap);
        }
        if let Some(wrap) = publication.escrow {
            self.escrow.insert(wrap.epoch, wrap);
        }
        Ok(())
    }

    pub fn history_link(&self, epoch: u64) -> Option<HistoryLink> {
        self.history.get(&epoch).cloned()
    }

    pub fn latest_recovery(&self) -> Option<&RecoveryWrap> {
        self.recovery.values().next_back()
    }

    pub fn latest_escrow(&self) -> Option<&EscrowWrap> {
        self.escrow.values().next_back()
    }

    /// Every blob the server holds, for the tests that try to read it without the keys.
    pub fn blobs(&self) -> Vec<Vec<u8>> {
        let mut out: Vec<Vec<u8>> = self.commits.clone();
        out.extend(self.history.values().map(|l| l.ciphertext.clone()));
        out.extend(self.recovery.values().map(|w| w.sealed.ciphertext.clone()));
        out.extend(self.escrow.values().map(|w| w.sealed.ciphertext.clone()));
        out.extend(self.welcomes.values().map(|w| w.welcome.clone()));
        out.extend(self.objects.iter().map(|o| o.ciphertext.clone()));
        out
    }
}
