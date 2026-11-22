//! The team's recovery (C7-B): the recovery key of 24 words, mandatory, and the organisation's escrow key, optional.
//!
//! **The recovery key** is an administrator device, offline. Its 24 words (BIP-39: 256 bits and a checksum) give,
//! by HKDF, an Ed25519 signature key and an X25519 HPKE key, whose public halves are in the roster. At each epoch
//! the committer seals to the HPKE key a [`RecoveryWrap`]: the epoch's record and its GroupInfo (with the ratchet
//! tree and the `external_pub` key). The day all the administrators have lost their devices, the words open the
//! last wrap, and their holder joins the group by an **external commit** (RFC 9420 §12.4.3.2), as the recovery
//! leaf; the members admit it because its signature key is the roster's (`policy.rs`). It is then an administrator:
//! it invites the new devices, then gives way to a new recovery key.
//!
//! The GroupInfo is never given in the clear: whoever holds it could join the group by an external commit too (and
//! be refused by the members, `policy.rs`, but the wrap does not lean on that).
//!
//! **A wrap is anchored** (SR2-09): it names the team's fingerprint, and it is signed twice, over the group's id,
//! the epoch, the fingerprint and the ciphertext: by the leaf of the administrator's device that made it, and by the
//! team's anchor (`anchor.rs`), a key drawn from the words that only administrators' devices hold. The holder of
//! the words checks the anchor's signature before opening anything, and the six words of the fingerprint they typed
//! from the rescue sheet; then, on the GroupInfo (same group, same epoch), that its roster gives that fingerprint,
//! holds this recovery key and its anchor, and that the leaf which signed is an administrator's in it. Only then does
//! it join. A member, or the server, who seals a wrap of a group of their own is refused at the first check.
//!
//! **The escrow key** is a key pair held by the organisation (its CISO). Each epoch's record is sealed to it (an
//! [`EscrowWrap`]): with the history links, it reads all the content. It never joins the group and cannot write.
//! It is in the roster, so every member sees it, and the client says so: "This team has an escrow key: the
//! organisation can read its content." Its wraps are anchored as the recovery's, by an anchor drawn from its seed:
//! the organisation reads only the records an administrator of the team sealed ([`EscrowKey::open`]).

use bip39::{Language, Mnemonic};
use ed25519_dalek::SigningKey;
use openmls::prelude::SignatureScheme;
use openmls_basic_credential::SignatureKeyPair;
use openmls_traits::crypto::OpenMlsCrypto;
use openmls_traits::signatures::Signer;
use openmls_traits::types::{HpkeCiphertext, HpkeKeyPair};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

use crate::anchor::Anchor;
use crate::content::verify_ed25519;
use crate::identity::{DeviceId, DeviceKind, Identity, MemberId};
use crate::keys::{expand, EpochRecord};
use crate::roster::{EscrowPublic, RecoveryPublic, Role, Roster, TeamFingerprint};
use crate::{mls, Error, CIPHERSUITE};

/// The recovery key: 256 bits, shown once as 24 words.
pub struct RecoveryKey {
    entropy: Zeroizing<[u8; 32]>,
}

impl RecoveryKey {
    pub fn from_entropy(entropy: [u8; 32]) -> Self {
        Self {
            entropy: Zeroizing::new(entropy),
        }
    }

    pub fn generate(rand: &impl openmls_traits::random::OpenMlsRand) -> Result<Self, Error> {
        Ok(Self::from_entropy(
            rand.random_array().map_err(|_| Error::Random)?,
        ))
    }

    /// The 24 words, as typed back: case and spaces do not matter, the checksum does.
    pub fn from_words(words: &str) -> Result<Self, Error> {
        let normalized = words
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        let mnemonic = Mnemonic::parse_in_normalized(Language::English, &normalized)
            .map_err(|_| Error::Words)?;
        let (bytes, len) = mnemonic.to_entropy_array();
        if len != 32 {
            return Err(Error::Words);
        }
        let mut entropy = [0u8; 32];
        entropy.copy_from_slice(&bytes[..32]);
        Ok(Self::from_entropy(entropy))
    }

    pub fn words(&self) -> Vec<&'static str> {
        Mnemonic::from_entropy_in(Language::English, self.entropy.as_slice())
            .expect("32 bytes make 24 words")
            .words()
            .collect()
    }

    /// The proof that the owner wrote the words down: three of them, typed again (positions from 0).
    pub fn confirms(&self, typed: &[(usize, &str)]) -> bool {
        let words = self.words();
        typed.len() >= 3
            && typed.iter().all(|(i, w)| {
                words
                    .get(*i)
                    .is_some_and(|x| x.eq_ignore_ascii_case(w.trim()))
            })
    }

    /// The seed of the recovery leaf's signature key, for the client to keep it in the keychain while the leaf is
    /// in the group (the day of a recovery), as it keeps a device's.
    pub fn signing_seed(&self) -> Zeroizing<[u8; 32]> {
        Zeroizing::new(expand(
            &self.entropy,
            b"escouade/recovery/signature/v1",
            b"",
        ))
    }

    pub fn signature_keys(&self) -> SignatureKeyPair {
        let seed = self.signing_seed();
        let public = SigningKey::from_bytes(&seed)
            .verifying_key()
            .to_bytes()
            .to_vec();
        SignatureKeyPair::from_raw(SignatureScheme::ED25519, seed.to_vec(), public)
    }

    pub fn hpke_keys(&self, crypto: &impl OpenMlsCrypto) -> Result<HpkeKeyPair, Error> {
        let ikm = Zeroizing::new(expand(&self.entropy, b"escouade/recovery/hpke/v1", b""));
        crypto
            .derive_hpke_keypair(CIPHERSUITE.hpke_config(), ikm.as_slice())
            .map_err(|e| Error::Mls(format!("recovery HPKE key: {e:?}")))
    }

    /// The recovery anchor (`anchor.rs`): what signs every recovery wrap. The device that makes the words keeps it
    /// (never the words) and gives it to the other administrators' devices.
    pub fn anchor(&self) -> Anchor {
        Anchor::from_seed(expand(&self.entropy, b"escouade/recovery/anchor/v1", b""))
    }

    pub fn public(&self, crypto: &impl OpenMlsCrypto) -> Result<RecoveryPublic, Error> {
        Ok(RecoveryPublic {
            signature_key: self.signature_keys().to_public_vec(),
            hpke_key: self.hpke_keys(crypto)?.public,
            anchor_key: self.anchor().public(),
        })
    }

    /// The identity of the recovery leaf: its member and device ids are made of its signature key.
    pub fn identity(&self) -> Identity {
        let public = self.signature_keys().to_public_vec();
        let mut public32 = [0u8; 32];
        public32.copy_from_slice(&public);
        let id = expand(&public32, b"escouade/recovery/identity/v1", b"");
        let mut member = [0u8; 16];
        member.copy_from_slice(&id[..16]);
        let mut device = [0u8; 16];
        device.copy_from_slice(&id[16..]);
        Identity {
            member: MemberId(member),
            device: DeviceId(device),
            kind: DeviceKind::Recovery,
        }
    }
}

/// The escrow key, on the organisation's side.
pub struct EscrowKey {
    keys: HpkeKeyPair,
    holder: String,
    /// What the key pair is derived from: what the organisation keeps (written down, in its safe).
    seed: Zeroizing<[u8; 32]>,
}

impl EscrowKey {
    pub fn generate(
        crypto: &impl OpenMlsCrypto,
        rand: &impl openmls_traits::random::OpenMlsRand,
        holder: &str,
    ) -> Result<Self, Error> {
        let seed: [u8; 32] = rand.random_array().map_err(|_| Error::Random)?;
        Self::from_seed(crypto, seed, holder)
    }

    /// The key pair again from its seed (what the organisation kept).
    pub fn from_seed(
        crypto: &impl OpenMlsCrypto,
        seed: [u8; 32],
        holder: &str,
    ) -> Result<Self, Error> {
        let seed = Zeroizing::new(seed);
        let keys = crypto
            .derive_hpke_keypair(CIPHERSUITE.hpke_config(), seed.as_slice())
            .map_err(|e| Error::Mls(format!("escrow HPKE key: {e:?}")))?;
        Ok(Self {
            keys,
            holder: holder.to_string(),
            seed,
        })
    }

    /// The seed, for the organisation to keep: it gives the key pair back (`from_seed`), and reads all the content.
    pub fn seed(&self) -> &[u8; 32] {
        &self.seed
    }

    /// The escrow anchor (`anchor.rs`), drawn from the seed: what signs every escrow wrap. The administrators'
    /// devices hold it (it comes with the team's half, [`Self::team_half`]); the organisation checks with it.
    pub fn anchor(&self) -> Anchor {
        Anchor::from_seed(expand(&self.seed, b"escouade/escrow/anchor/v1", b""))
    }

    pub fn public(&self) -> EscrowPublic {
        EscrowPublic {
            hpke_key: self.keys.public.clone(),
            holder: self.holder.clone(),
            anchor_key: self.anchor().public(),
        }
    }

    /// What the team gets of the pair: its public half, and the anchor its administrators sign the wraps with.
    pub fn team_half(&self) -> EscrowTeamHalf {
        EscrowTeamHalf {
            public: self.public(),
            anchor: self.anchor(),
        }
    }

    /// The record of an epoch, from its wrap: only when the team's anchor signed it, for the team of `team` (the
    /// fingerprint the organisation noted), and when the roster it holds says the leaf that signed is an
    /// administrator's.
    pub fn open(
        &self,
        crypto: &impl OpenMlsCrypto,
        team: &TeamFingerprint,
        group_id: &[u8],
        wrap: &EscrowWrap,
    ) -> Result<EpochRecord, Error> {
        if wrap.signed.team != team.0 {
            return Err(Error::OtherTeam);
        }
        wrap.signed.check(
            ESCROW_KIND,
            group_id,
            wrap.epoch,
            &wrap.sealed,
            &self.anchor().public(),
        )?;
        let plain = Zeroizing::new(hpke_open(
            crypto,
            &self.keys.private,
            ESCROW_INFO,
            &wrap_aad(group_id, wrap.epoch),
            &wrap.sealed,
        )?);
        let payload: EscrowPayload = serde_json::from_slice(&plain)
            .map_err(|e| Error::Encoding(format!("escrow wrap: {e}")))?;
        let record = payload.record.clone();
        if record.epoch != wrap.epoch {
            return Err(Error::Encoding("escrow wrap of another epoch".into()));
        }
        let roster = &payload.roster;
        if TeamFingerprint::of(group_id, &roster.genesis) != *team
            || roster.escrow.as_ref().is_none_or(|e| {
                e.hpke_key != self.keys.public || e.anchor_key != self.anchor().public()
            })
        {
            return Err(Error::OtherTeam);
        }
        let leaves: Vec<(Identity, Vec<u8>)> = record
            .members
            .iter()
            .filter_map(|m| {
                Identity::from_bytes(&m.identity)
                    .ok()
                    .map(|i| (i, m.signature_key.clone()))
            })
            .collect();
        admin_signed(roster, &leaves, &wrap.signed.signer)?;
        Ok(record)
    }
}

/// The team's half of an escrow pair: its public half for the roster, and the anchor of its wraps, which the
/// administrators' devices keep (the organisation keeps the secret, the seed).
#[derive(Clone, Debug)]
pub struct EscrowTeamHalf {
    pub public: EscrowPublic,
    pub anchor: Anchor,
}

/// Whether the leaf of key `signer` is an administrator's in `roster`, among `leaves` (identity, signature key):
/// a device of a member the roster makes an administrator, or the recovery leaf of this roster's key.
pub(crate) fn admin_signed(
    roster: &Roster,
    leaves: &[(Identity, Vec<u8>)],
    signer: &[u8],
) -> Result<(), Error> {
    let admin = leaves.iter().any(|(identity, key)| {
        key.as_slice() == signer
            && match identity.kind {
                DeviceKind::Device => roster.role(&identity.member) == Some(Role::Admin),
                DeviceKind::Recovery => roster.recovery.signature_key.as_slice() == signer,
            }
    });
    if admin {
        Ok(())
    } else {
        Err(Error::Unanchored)
    }
}

const RECOVERY_INFO: &[u8] = b"escouade/recovery-wrap/v1";
const ESCROW_INFO: &[u8] = b"escouade/escrow-wrap/v1";

fn wrap_aad(group_id: &[u8], epoch: u64) -> Vec<u8> {
    let mut out = (group_id.len() as u32).to_be_bytes().to_vec();
    out.extend_from_slice(group_id);
    out.extend_from_slice(&epoch.to_be_bytes());
    out
}

/// An HPKE ciphertext (RFC 9180, base mode, the suite's KEM, KDF and AEAD), as stored by the server.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sealed {
    #[serde(with = "hex::serde")]
    pub kem_output: Vec<u8>,
    #[serde(with = "hex::serde")]
    pub ciphertext: Vec<u8>,
}

pub(crate) fn hpke_seal(
    crypto: &impl OpenMlsCrypto,
    public: &[u8],
    info: &[u8],
    aad: &[u8],
    plain: &[u8],
) -> Result<Sealed, Error> {
    let ct = crypto
        .hpke_seal(CIPHERSUITE.hpke_config(), public, info, aad, plain)
        .map_err(|e| Error::Mls(format!("HPKE seal: {e:?}")))?;
    Ok(Sealed {
        kem_output: ct.kem_output.as_slice().to_vec(),
        ciphertext: ct.ciphertext.as_slice().to_vec(),
    })
}

pub(crate) fn hpke_open(
    crypto: &impl OpenMlsCrypto,
    private: &[u8],
    info: &[u8],
    aad: &[u8],
    sealed: &Sealed,
) -> Result<Vec<u8>, Error> {
    let ct = HpkeCiphertext {
        kem_output: sealed.kem_output.clone().into(),
        ciphertext: sealed.ciphertext.clone().into(),
    };
    crypto
        .hpke_open(CIPHERSUITE.hpke_config(), &ct, private, info, aad)
        .map_err(|_| Error::Decrypt)
}

const RECOVERY_KIND: &[u8] = b"recovery";
const ESCROW_KIND: &[u8] = b"escrow";

/// Who makes a wrap: the team it is for, the leaf of the administrator's device that signs it, and the anchor.
pub struct WrapAuthor<'a> {
    pub group_id: &'a [u8],
    pub team: TeamFingerprint,
    pub leaf: &'a SignatureKeyPair,
    pub anchor: &'a Anchor,
}

/// The signatures of a wrap (SR2-09): the team it is for, the administrator's leaf that made it, and the anchor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrapSignatures {
    /// The team's fingerprint (`TeamFingerprint`: the group's id and its genesis).
    #[serde(with = "hex::serde")]
    pub team: [u8; 32],
    /// The signature key of the leaf that signed.
    #[serde(with = "hex::serde")]
    pub signer: Vec<u8>,
    /// The leaf's signature.
    #[serde(with = "hex::serde")]
    pub signature: Vec<u8>,
    /// The anchor's signature, over the same bytes.
    #[serde(with = "hex::serde")]
    pub anchor: Vec<u8>,
}

/// What both keys sign: a label, the kind of wrap, the group, the epoch, the team, the ciphertext's hash and the
/// leaf's key.
fn statement(
    kind: &[u8],
    group_id: &[u8],
    epoch: u64,
    team: &[u8; 32],
    sealed: &Sealed,
    signer: &[u8],
) -> Vec<u8> {
    let mut out = b"escouade/wrap/v1\0".to_vec();
    out.extend_from_slice(kind);
    out.push(0);
    out.extend_from_slice(&(group_id.len() as u32).to_be_bytes());
    out.extend_from_slice(group_id);
    out.extend_from_slice(&epoch.to_be_bytes());
    out.extend_from_slice(team);
    let mut h = Sha256::new();
    h.update((sealed.kem_output.len() as u32).to_be_bytes());
    h.update(&sealed.kem_output);
    h.update(&sealed.ciphertext);
    out.extend_from_slice(&h.finalize());
    out.extend_from_slice(&(signer.len() as u16).to_be_bytes());
    out.extend_from_slice(signer);
    out
}

impl WrapSignatures {
    fn make(kind: &[u8], by: &WrapAuthor, epoch: u64, sealed: &Sealed) -> Result<Self, Error> {
        let signer = by.leaf.to_public_vec();
        let message = statement(kind, by.group_id, epoch, &by.team.0, sealed, &signer);
        Ok(Self {
            team: by.team.0,
            signature: by.leaf.sign(&message).map_err(mls)?,
            anchor: by.anchor.sign(&message),
            signer,
        })
    }

    /// The anchor's signature (by `anchor_key`), then the leaf's.
    fn check(
        &self,
        kind: &[u8],
        group_id: &[u8],
        epoch: u64,
        sealed: &Sealed,
        anchor_key: &[u8],
    ) -> Result<(), Error> {
        let message = statement(kind, group_id, epoch, &self.team, sealed, &self.signer);
        Anchor::verify(anchor_key, &message, &self.anchor).map_err(|_| Error::Unanchored)?;
        verify_ed25519(&self.signer, &message, &self.signature).map_err(|_| Error::Unanchored)
    }
}

/// What the recovery key needs to come back at an epoch.
#[derive(Serialize, Deserialize)]
struct RecoveryPayload {
    record: EpochRecord,
    #[serde(with = "hex::serde")]
    group_info: Vec<u8>,
    /// The escrow anchor, when the team has an escrow key: the recovered team keeps signing its wraps.
    #[serde(default, with = "hex::serde", skip_serializing_if = "Vec::is_empty")]
    escrow_anchor: Vec<u8>,
}

impl Drop for RecoveryPayload {
    fn drop(&mut self) {
        self.group_info.zeroize();
        self.escrow_anchor.zeroize();
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryWrap {
    pub epoch: u64,
    pub sealed: Sealed,
    pub signed: WrapSignatures,
}

/// What a recovery wrap holds, once opened.
pub struct RecoveryOpened {
    pub record: EpochRecord,
    /// The GroupInfo (TLS-encoded `MlsMessageOut`).
    pub group_info: Vec<u8>,
    pub escrow_anchor: Option<Anchor>,
}

impl RecoveryWrap {
    /// Seals the epoch's record and GroupInfo to the recovery key, signed by `by`: its anchor must be the roster's.
    pub fn seal(
        crypto: &impl OpenMlsCrypto,
        recovery: &RecoveryPublic,
        by: &WrapAuthor,
        record: &EpochRecord,
        group_info: Vec<u8>,
        escrow_anchor: Option<&Anchor>,
    ) -> Result<Self, Error> {
        if recovery.anchor_key.is_empty() || by.anchor.public() != recovery.anchor_key {
            return Err(Error::Unanchored);
        }
        let payload = RecoveryPayload {
            record: record.clone(),
            group_info,
            escrow_anchor: escrow_anchor.map(|a| a.seed().to_vec()).unwrap_or_default(),
        };
        let plain =
            Zeroizing::new(serde_json::to_vec(&payload).expect("a payload always serializes"));
        let sealed = hpke_seal(
            crypto,
            &recovery.hpke_key,
            RECOVERY_INFO,
            &wrap_aad(by.group_id, record.epoch),
            &plain,
        )?;
        Ok(Self {
            epoch: record.epoch,
            signed: WrapSignatures::make(RECOVERY_KIND, by, record.epoch, &sealed)?,
            sealed,
        })
    }

    /// Whether the team's anchor (`anchor_key`, the roster's) signed this wrap, for the team `team`: what every
    /// device can tell without the words.
    pub fn is_anchored(&self, group_id: &[u8], team: &TeamFingerprint, anchor_key: &[u8]) -> bool {
        self.signed.team == team.0
            && self
                .signed
                .check(
                    RECOVERY_KIND,
                    group_id,
                    self.epoch,
                    &self.sealed,
                    anchor_key,
                )
                .is_ok()
    }

    /// Checks the anchor's signature with the words' own anchor, then opens: the record of the epoch, its
    /// GroupInfo, and the escrow anchor if any. The GroupInfo is still to check (`TeamDevice::recover_with`).
    pub fn open(
        &self,
        crypto: &impl OpenMlsCrypto,
        key: &RecoveryKey,
        group_id: &[u8],
    ) -> Result<RecoveryOpened, Error> {
        self.signed.check(
            RECOVERY_KIND,
            group_id,
            self.epoch,
            &self.sealed,
            &key.anchor().public(),
        )?;
        let keys = key.hpke_keys(crypto)?;
        let plain = Zeroizing::new(hpke_open(
            crypto,
            &keys.private,
            RECOVERY_INFO,
            &wrap_aad(group_id, self.epoch),
            &self.sealed,
        )?);
        let payload: RecoveryPayload = serde_json::from_slice(&plain)
            .map_err(|e| Error::Encoding(format!("recovery wrap: {e}")))?;
        if payload.record.epoch != self.epoch {
            return Err(Error::Encoding("recovery wrap of another epoch".into()));
        }
        let escrow_anchor = match payload.escrow_anchor.as_slice() {
            [] => None,
            seed => Some(Anchor::from_seed(seed.try_into().map_err(|_| {
                Error::Encoding("an escrow anchor of another length".into())
            })?)),
        };
        Ok(RecoveryOpened {
            record: payload.record.clone(),
            group_info: payload.group_info.clone(),
            escrow_anchor,
        })
    }
}

/// What the organisation reads back of an epoch: its record, and the roster that says who signed.
#[derive(Serialize, Deserialize)]
struct EscrowPayload {
    record: EpochRecord,
    roster: Roster,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscrowWrap {
    pub epoch: u64,
    pub sealed: Sealed,
    pub signed: WrapSignatures,
}

impl EscrowWrap {
    /// Seals the epoch's record and roster to the escrow key, signed by `by`: its anchor must be the roster's.
    pub fn seal(
        crypto: &impl OpenMlsCrypto,
        escrow: &EscrowPublic,
        by: &WrapAuthor,
        record: &EpochRecord,
        roster: &Roster,
    ) -> Result<Self, Error> {
        if escrow.anchor_key.is_empty() || by.anchor.public() != escrow.anchor_key {
            return Err(Error::Unanchored);
        }
        let payload = EscrowPayload {
            record: record.clone(),
            roster: roster.clone(),
        };
        let plain =
            Zeroizing::new(serde_json::to_vec(&payload).expect("a payload always serializes"));
        let sealed = hpke_seal(
            crypto,
            &escrow.hpke_key,
            ESCROW_INFO,
            &wrap_aad(by.group_id, record.epoch),
            &plain,
        )?;
        Ok(Self {
            epoch: record.epoch,
            signed: WrapSignatures::make(ESCROW_KIND, by, record.epoch, &sealed)?,
            sealed,
        })
    }

    /// Whether the escrow anchor (`anchor_key`, the roster's) signed this wrap, for the team `team`.
    pub fn is_anchored(&self, group_id: &[u8], team: &TeamFingerprint, anchor_key: &[u8]) -> bool {
        self.signed.team == team.0
            && self
                .signed
                .check(ESCROW_KIND, group_id, self.epoch, &self.sealed, anchor_key)
                .is_ok()
    }
}
