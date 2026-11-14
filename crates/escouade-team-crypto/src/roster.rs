//! The team's roster: who the members are, their role, the recovery key and the escrow key.
//!
//! It is a private-use extension of the MLS group context (RFC 9420 §13.3, type 0xff00 in the private range
//! 0xF000–0xFFFF), so that every member holds the same one for an epoch, bound to it by the key schedule, and that
//! it only changes by a commit (a GroupContextExtensions proposal) that the rules of `policy.rs` admit: the signed
//! journal of the members of `docs/TEAM-SERVER.md` §3.7 is the group and its commits. Commits go as MLS private
//! messages: the server does not read the roster.

use std::collections::BTreeMap;

use openmls::prelude::{
    Extension, ExtensionType, Extensions, GroupContext, RequiredCapabilitiesExtension,
    UnknownExtension,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::identity::MemberId;
use crate::Error;

/// The roster's extension type, in the private-use range.
pub const ROSTER_EXTENSION: u16 = 0xff00;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// Adds and removes members, changes roles, the recovery and escrow keys, the team's settings.
    Admin,
    /// Reads and writes the content; adds and removes their own devices only.
    Member,
}

/// What made the team, never changed afterwards: the team's fingerprint is made of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Genesis {
    pub owner: MemberId,
    /// The signature key of the owner's first device.
    #[serde(with = "hex::serde")]
    pub owner_key: Vec<u8>,
    /// Seconds since the epoch, on the owner's device.
    pub created_at: u64,
}

/// The public half of the recovery key (`recovery.rs`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryPublic {
    /// The Ed25519 key its leaf signs with, the day it joins.
    #[serde(with = "hex::serde")]
    pub signature_key: Vec<u8>,
    /// The X25519 key the epochs are sealed to (HPKE), for it to join again.
    #[serde(with = "hex::serde")]
    pub hpke_key: Vec<u8>,
}

/// The public half of the organisation's escrow key: every epoch's secrets are sealed to it, so the organisation can
/// read all the content. Shown to every member while it is set (C7-B).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscrowPublic {
    #[serde(with = "hex::serde")]
    pub hpke_key: Vec<u8>,
    /// Who holds it, as the administrator said it ("the CISO of ACME").
    pub holder: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Roster {
    pub genesis: Genesis,
    pub members: BTreeMap<MemberId, Role>,
    pub recovery: RecoveryPublic,
    pub escrow: Option<EscrowPublic>,
}

impl Roster {
    pub fn role(&self, member: &MemberId) -> Option<Role> {
        self.members.get(member).copied()
    }

    pub fn admins(&self) -> usize {
        self.members.values().filter(|r| **r == Role::Admin).count()
    }

    /// JSON, its maps sorted (BTreeMap) and its fields in a fixed order: the same roster gives the same bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("a roster always serializes")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        serde_json::from_slice(bytes).map_err(|e| Error::Encoding(format!("roster: {e}")))
    }

    /// The group context's extensions: the roster, and the capability every leaf must have to hold it.
    pub fn to_extensions(&self) -> Result<Extensions<GroupContext>, Error> {
        Extensions::from_vec(vec![
            Extension::Unknown(ROSTER_EXTENSION, UnknownExtension(self.to_bytes())),
            Extension::RequiredCapabilities(RequiredCapabilitiesExtension::new(
                &[ExtensionType::Unknown(ROSTER_EXTENSION)],
                &[],
                &[],
            )),
        ])
        .map_err(|e| Error::Mls(format!("roster extension: {e:?}")))
    }

    pub fn from_context(context: &GroupContext) -> Result<Self, Error> {
        Self::from_extensions(context.extensions())
    }

    pub fn from_extensions(extensions: &Extensions<GroupContext>) -> Result<Self, Error> {
        let ext = extensions
            .unknown(ROSTER_EXTENSION)
            .ok_or_else(|| Error::Policy("the group context has no roster".into()))?;
        Self::from_bytes(&ext.0)
    }
}

/// The team's fingerprint: SHA-256 of the group id and the genesis, in every invitation link, and shown as six words
/// to compare by voice (`docs/TEAM-SERVER.md` §3.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TeamFingerprint(pub [u8; 32]);

impl TeamFingerprint {
    pub fn of(group_id: &[u8], genesis: &Genesis) -> Self {
        let mut h = Sha256::new();
        h.update(b"escouade/team-fingerprint/v1\0");
        h.update((group_id.len() as u32).to_be_bytes());
        h.update(group_id);
        h.update(serde_json::to_vec(genesis).expect("a genesis always serializes"));
        Self(h.finalize().into())
    }

    /// The first 66 bits, as six words of the BIP-39 English list (11 bits each).
    pub fn words(&self) -> [&'static str; 6] {
        let list = bip39::Language::English.word_list();
        let mut bits: u128 = 0;
        for b in &self.0[..9] {
            bits = (bits << 8) | u128::from(*b);
        }
        // 72 bits read, the first 66 kept.
        bits >>= 6;
        let mut out = [""; 6];
        for (i, word) in out.iter_mut().enumerate() {
            let shift = 11 * (5 - i);
            *word = list[((bits >> shift) & 0x7ff) as usize];
        }
        out
    }
}
