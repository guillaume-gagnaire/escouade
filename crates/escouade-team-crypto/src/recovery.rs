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
//! **The escrow key** is a key pair held by the organisation (its CISO). Each epoch's record is sealed to it (an
//! [`EscrowWrap`]): with the history links, it reads all the content. It never joins the group and cannot write.
//! It is in the roster, so every member sees it, and the client says so: "This team has an escrow key: the
//! organisation can read its content."

use bip39::{Language, Mnemonic};
use ed25519_dalek::SigningKey;
use openmls::prelude::SignatureScheme;
use openmls_basic_credential::SignatureKeyPair;
use openmls_traits::crypto::OpenMlsCrypto;
use openmls_traits::types::{HpkeCiphertext, HpkeKeyPair};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

use crate::identity::{DeviceId, DeviceKind, Identity, MemberId};
use crate::keys::{expand, EpochRecord};
use crate::roster::{EscrowPublic, RecoveryPublic};
use crate::{Error, CIPHERSUITE};

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

    fn signing_seed(&self) -> Zeroizing<[u8; 32]> {
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

    pub fn public(&self, crypto: &impl OpenMlsCrypto) -> Result<RecoveryPublic, Error> {
        Ok(RecoveryPublic {
            signature_key: self.signature_keys().to_public_vec(),
            hpke_key: self.hpke_keys(crypto)?.public,
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
}

impl EscrowKey {
    pub fn generate(
        crypto: &impl OpenMlsCrypto,
        rand: &impl openmls_traits::random::OpenMlsRand,
        holder: &str,
    ) -> Result<Self, Error> {
        let ikm: Zeroizing<[u8; 32]> =
            Zeroizing::new(rand.random_array().map_err(|_| Error::Random)?);
        let keys = crypto
            .derive_hpke_keypair(CIPHERSUITE.hpke_config(), ikm.as_slice())
            .map_err(|e| Error::Mls(format!("escrow HPKE key: {e:?}")))?;
        Ok(Self {
            keys,
            holder: holder.to_string(),
        })
    }

    pub fn public(&self) -> EscrowPublic {
        EscrowPublic {
            hpke_key: self.keys.public.clone(),
            holder: self.holder.clone(),
        }
    }

    pub fn open(
        &self,
        crypto: &impl OpenMlsCrypto,
        group_id: &[u8],
        wrap: &EscrowWrap,
    ) -> Result<EpochRecord, Error> {
        let plain = Zeroizing::new(hpke_open(
            crypto,
            &self.keys.private,
            ESCROW_INFO,
            &wrap_aad(group_id, wrap.epoch),
            &wrap.sealed,
        )?);
        let record = EpochRecord::from_bytes(&plain)?;
        if record.epoch != wrap.epoch {
            return Err(Error::Encoding("escrow wrap of another epoch".into()));
        }
        Ok(record)
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

fn hpke_seal(
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

fn hpke_open(
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

/// What the recovery key needs to come back at an epoch.
#[derive(Serialize, Deserialize)]
struct RecoveryPayload {
    record: EpochRecord,
    #[serde(with = "hex::serde")]
    group_info: Vec<u8>,
}

impl Drop for RecoveryPayload {
    fn drop(&mut self) {
        self.group_info.zeroize();
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryWrap {
    pub epoch: u64,
    pub sealed: Sealed,
}

impl RecoveryWrap {
    pub fn seal(
        crypto: &impl OpenMlsCrypto,
        recovery: &RecoveryPublic,
        group_id: &[u8],
        record: &EpochRecord,
        group_info: Vec<u8>,
    ) -> Result<Self, Error> {
        let payload = RecoveryPayload {
            record: record.clone(),
            group_info,
        };
        let plain =
            Zeroizing::new(serde_json::to_vec(&payload).expect("a payload always serializes"));
        Ok(Self {
            epoch: record.epoch,
            sealed: hpke_seal(
                crypto,
                &recovery.hpke_key,
                RECOVERY_INFO,
                &wrap_aad(group_id, record.epoch),
                &plain,
            )?,
        })
    }

    /// The record of the epoch and its GroupInfo (TLS-encoded `MlsMessageOut`).
    pub fn open(
        &self,
        crypto: &impl OpenMlsCrypto,
        key: &RecoveryKey,
        group_id: &[u8],
    ) -> Result<(EpochRecord, Vec<u8>), Error> {
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
        Ok((payload.record.clone(), payload.group_info.clone()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscrowWrap {
    pub epoch: u64,
    pub sealed: Sealed,
}

impl EscrowWrap {
    pub fn seal(
        crypto: &impl OpenMlsCrypto,
        escrow: &EscrowPublic,
        group_id: &[u8],
        record: &EpochRecord,
    ) -> Result<Self, Error> {
        let plain = Zeroizing::new(record.to_bytes());
        Ok(Self {
            epoch: record.epoch,
            sealed: hpke_seal(
                crypto,
                &escrow.hpke_key,
                ESCROW_INFO,
                &wrap_aad(group_id, record.epoch),
                &plain,
            )?,
        })
    }
}
