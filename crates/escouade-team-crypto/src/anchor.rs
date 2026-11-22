//! The anchors of the recovery and of the escrow (SR2-09, `docs/TEAM-SERVER.md` §3.5): what proves that a wrap was
//! made by an administrator of the team, and not by a member, a former member or the server.
//!
//! A wrap holds the GroupInfo the holder of the 24 words joins by. Whoever can write in the team's stream could seal
//! one of their own (the recovery key's public half is in the roster, HPKE in base mode names no sender), and the
//! fingerprint of the team does not tell a copy apart: it is made of the group's id and its genesis, which every
//! member knows and can put in a group of their own, with themselves as its administrator. So each wrap is also
//! signed by a key no member holds:
//!
//! - **The recovery anchor** is drawn from the 24 words (HKDF, its own label): the holder of the words makes it
//!   again, and checks the wrap with it before opening anything. Its public half is in the roster
//!   (`RecoveryPublic::anchor_key`), so every device can tell a signed wrap from another.
//! - **The escrow anchor** is drawn from the escrow key's seed: the organisation checks its wraps the same way.
//! - **Only administrators' devices hold them.** The device that makes the words (or the escrow pair) keeps the
//!   anchor, never the words; it gives it to each other administrator's device, sealed by HPKE to the X25519 form of
//!   that device's leaf key ([`AnchorGrant`]), and a device keeps an anchor only when it is the one whose public half
//!   the roster holds. A member never receives it, the server only sees ciphertexts.
//!
//! What is left: a former administrator keeps the anchor of the words of their time. Removed, their device is
//! revoked and steering a recovery needs the server with them; made a member again, they still write in the stream.
//! So the client tells the administrators when someone who was one under the current recovery key no longer is
//! (`Team::recovery_stale`), and the team makes a new recovery key (so a new anchor) and prints the new rescue sheet
//! (`docs/TEAM-SERVER.md` §3.5).
//!
//! The leaf key also serves as an X25519 key here (its Montgomery form, as `age` and libsodium do for Ed25519 keys):
//! it saves publishing a key of each device, and HPKE in base mode keeps the two uses apart by its labels.

use ed25519_dalek::{Signer as _, SigningKey, VerifyingKey};
use openmls_traits::crypto::OpenMlsCrypto;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::content::verify_ed25519;
use crate::identity::DeviceId;
use crate::recovery::{hpke_open, hpke_seal, Sealed};
use crate::Error;

const GRANT_INFO: &[u8] = b"escouade/anchor-grant/v1";

/// A signature key that only the administrators' devices hold (and the holder of the words, or of the escrow seed,
/// who makes it again).
pub struct Anchor {
    seed: Zeroizing<[u8; 32]>,
}

impl std::fmt::Debug for Anchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Anchor")
            .field("public", &hex::encode(self.public()))
            .finish_non_exhaustive()
    }
}

impl Clone for Anchor {
    fn clone(&self) -> Self {
        Self::from_seed(*self.seed)
    }
}

impl Anchor {
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            seed: Zeroizing::new(seed),
        }
    }

    /// The seed, for the device's encrypted state (never shown, never sent in the clear).
    pub fn seed(&self) -> &[u8; 32] {
        &self.seed
    }

    fn key(&self) -> SigningKey {
        SigningKey::from_bytes(&self.seed)
    }

    /// The public half, Ed25519: what the roster holds.
    pub fn public(&self) -> Vec<u8> {
        self.key().verifying_key().to_bytes().to_vec()
    }

    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.key().sign(message).to_bytes().to_vec()
    }

    pub fn verify(public: &[u8], message: &[u8], signature: &[u8]) -> Result<(), Error> {
        verify_ed25519(public, message, signature)
    }
}

/// An anchor given to an administrator's device: its seed, sealed to the X25519 form of the device's leaf key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorGrant {
    /// The device it is for (the device id of its leaf).
    #[serde(with = "hex::serde")]
    pub device: [u8; 16],
    /// The anchor's public half: the device keeps the seed only when it gives this key back.
    #[serde(with = "hex::serde")]
    pub anchor: Vec<u8>,
    pub sealed: Sealed,
}

fn grant_aad(group_id: &[u8], device: &[u8; 16], anchor: &[u8]) -> Vec<u8> {
    let mut out = (group_id.len() as u32).to_be_bytes().to_vec();
    out.extend_from_slice(group_id);
    out.extend_from_slice(device);
    out.extend_from_slice(anchor);
    out
}

/// The X25519 public key of a leaf's Ed25519 key (its Montgomery form).
fn x25519_public(leaf_key: &[u8]) -> Result<[u8; 32], Error> {
    let bytes: [u8; 32] = leaf_key.try_into().map_err(|_| Error::BadSignature)?;
    let key = VerifyingKey::from_bytes(&bytes).map_err(|_| Error::BadSignature)?;
    Ok(key.to_montgomery().to_bytes())
}

impl AnchorGrant {
    pub fn seal(
        crypto: &impl OpenMlsCrypto,
        group_id: &[u8],
        device: DeviceId,
        leaf_key: &[u8],
        anchor: &Anchor,
    ) -> Result<Self, Error> {
        let public = anchor.public();
        let sealed = hpke_seal(
            crypto,
            &x25519_public(leaf_key)?,
            GRANT_INFO,
            &grant_aad(group_id, &device.0, &public),
            anchor.seed(),
        )?;
        Ok(Self {
            device: device.0,
            anchor: public,
            sealed,
        })
    }

    /// Opens a grant with the seed of this device's leaf key; the anchor must give back the public half it names.
    pub fn open(
        &self,
        crypto: &impl OpenMlsCrypto,
        group_id: &[u8],
        leaf_seed: &[u8; 32],
    ) -> Result<Anchor, Error> {
        let private = Zeroizing::new(SigningKey::from_bytes(leaf_seed).to_scalar_bytes());
        let plain = Zeroizing::new(hpke_open(
            crypto,
            private.as_slice(),
            GRANT_INFO,
            &grant_aad(group_id, &self.device, &self.anchor),
            &self.sealed,
        )?);
        let seed: [u8; 32] = plain
            .as_slice()
            .try_into()
            .map_err(|_| Error::Encoding("an anchor of another length".into()))?;
        let anchor = Anchor::from_seed(seed);
        if anchor.public() != self.anchor {
            return Err(Error::BadSignature);
        }
        Ok(anchor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openmls_rust_crypto::OpenMlsRustCrypto;
    use openmls_traits::OpenMlsProvider;

    #[test]
    fn a_grant_opens_with_the_leaf_key_it_is_sealed_to_only() {
        let provider = OpenMlsRustCrypto::default();
        let crypto = provider.crypto();
        let leaf_seed = [7u8; 32];
        let leaf_key = SigningKey::from_bytes(&leaf_seed)
            .verifying_key()
            .to_bytes()
            .to_vec();
        let anchor = Anchor::from_seed([9u8; 32]);
        let grant =
            AnchorGrant::seal(crypto, b"group", DeviceId([1; 16]), &leaf_key, &anchor).unwrap();
        let opened = grant.open(crypto, b"group", &leaf_seed).unwrap();
        assert_eq!(opened.public(), anchor.public());

        // Another leaf, another group, another device named, another anchor named: refused.
        assert!(grant.open(crypto, b"group", &[8u8; 32]).is_err());
        assert!(grant.open(crypto, b"other", &leaf_seed).is_err());
        let mut moved = grant.clone();
        moved.device = [2; 16];
        assert!(moved.open(crypto, b"group", &leaf_seed).is_err());
        let mut renamed = grant.clone();
        renamed.anchor = Anchor::from_seed([3u8; 32]).public();
        assert!(renamed.open(crypto, b"group", &leaf_seed).is_err());
    }

    #[test]
    fn an_anchor_signs_and_its_public_half_checks() {
        let anchor = Anchor::from_seed([5u8; 32]);
        let signature = anchor.sign(b"a wrap");
        assert!(Anchor::verify(&anchor.public(), b"a wrap", &signature).is_ok());
        assert!(Anchor::verify(&anchor.public(), b"another", &signature).is_err());
        let other = Anchor::from_seed([6u8; 32]);
        assert!(Anchor::verify(&other.public(), b"a wrap", &signature).is_err());
        assert!(!format!("{anchor:?}").contains(&hex::encode([5u8; 32])));
    }
}
