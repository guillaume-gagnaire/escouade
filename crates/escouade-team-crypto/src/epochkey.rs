//! What the key of the costs (TS-16, [`crate::CostKey`]) and the key of the audit (AU-3, [`crate::AuditKey`]) share:
//! an HPKE key pair (RFC 9180, the suite's KEM, KDF and AEAD) apart from the team's MLS keys, one by epoch, whose
//! secret half (the seed of the pair) is wrapped, by HPKE too, for a set of devices (each to its own key of the same
//! use, a pair drawn on the device and kept in its encrypted state).
//!
//! - Every device seals to its public half ([`EpochKey::seal`]); only the devices it is wrapped for open
//!   ([`EpochKey::open`]): not the server, which keeps the ciphertexts, nor the other devices.
//! - A [`Use`] names what the key is for: its two HPKE `info`, one for what is sealed and one for the wraps. A
//!   ciphertext of one use never opens under another (the costs' wrap does not give the audit's key).
//! - The associated data bind a ciphertext to its team, to the epoch of the key and to a device (its author, for what
//!   is sealed; the device it is for, for a wrap): the server cannot move a ciphertext to another team or epoch, nor
//!   pass one device's for another's.

use std::marker::PhantomData;

use openmls_traits::crypto::OpenMlsCrypto;
use openmls_traits::random::OpenMlsRand;
use openmls_traits::types::HpkeKeyPair;
use zeroize::Zeroizing;

use crate::recovery::{hpke_open, hpke_seal, Sealed};
use crate::{Error, CIPHERSUITE};

/// What an [`EpochKey`] is for: its name (in errors) and its two HPKE `info`.
pub trait Use {
    /// Its name in errors (`cost`, `audit`).
    const NAME: &'static str;
    /// The type's name in `Debug` (`CostKey`).
    const TYPE: &'static str;
    /// The `info` of what is sealed to the key.
    const SEAL_INFO: &'static [u8];
    /// The `info` of the wraps of its seed.
    const WRAP_INFO: &'static [u8];
}

/// An HPKE key pair of a use: the one of an epoch (whose seed is wrapped), or a device's own (to receive the wraps).
pub struct EpochKey<U: Use> {
    keys: HpkeKeyPair,
    seed: Zeroizing<[u8; 32]>,
    use_: PhantomData<U>,
}

impl<U: Use> std::fmt::Debug for EpochKey<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(U::TYPE)
            .field("public", &hex::encode(&self.keys.public))
            .finish_non_exhaustive()
    }
}

fn aad(team: &[u8; 16], epoch: u64, device: &[u8; 16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(40);
    out.extend_from_slice(team);
    out.extend_from_slice(&epoch.to_be_bytes());
    out.extend_from_slice(device);
    out
}

impl<U: Use> EpochKey<U> {
    pub fn generate(crypto: &impl OpenMlsCrypto, rand: &impl OpenMlsRand) -> Result<Self, Error> {
        let seed: [u8; 32] = rand.random_array().map_err(|_| Error::Random)?;
        Self::from_seed(crypto, seed)
    }

    /// The pair again from its seed (what a wrap carries, what the device's state keeps).
    pub fn from_seed(crypto: &impl OpenMlsCrypto, seed: [u8; 32]) -> Result<Self, Error> {
        let seed = Zeroizing::new(seed);
        let keys = crypto
            .derive_hpke_keypair(CIPHERSUITE.hpke_config(), seed.as_slice())
            .map_err(|e| Error::Mls(format!("{} HPKE key: {e:?}", U::NAME)))?;
        Ok(Self {
            keys,
            seed,
            use_: PhantomData,
        })
    }

    pub fn seed(&self) -> &[u8; 32] {
        &self.seed
    }

    pub fn public(&self) -> Vec<u8> {
        self.keys.public.clone()
    }

    /// `plain` of `device`, sealed to the key `public` of `epoch`.
    pub fn seal(
        crypto: &impl OpenMlsCrypto,
        public: &[u8],
        team: &[u8; 16],
        epoch: u64,
        device: &[u8; 16],
        plain: &[u8],
    ) -> Result<Sealed, Error> {
        hpke_seal(
            crypto,
            public,
            U::SEAL_INFO,
            &aad(team, epoch, device),
            plain,
        )
    }

    /// What `device` sealed to this key (the one of `epoch`).
    pub fn open(
        &self,
        crypto: &impl OpenMlsCrypto,
        team: &[u8; 16],
        epoch: u64,
        device: &[u8; 16],
        sealed: &Sealed,
    ) -> Result<Vec<u8>, Error> {
        hpke_open(
            crypto,
            &self.keys.private,
            U::SEAL_INFO,
            &aad(team, epoch, device),
            sealed,
        )
    }

    /// This pair's seed (the one of `epoch`), wrapped for the device `device`, whose own key of this use is `public`.
    pub fn wrap_seed(
        &self,
        crypto: &impl OpenMlsCrypto,
        public: &[u8],
        team: &[u8; 16],
        epoch: u64,
        device: &[u8; 16],
    ) -> Result<Sealed, Error> {
        hpke_seal(
            crypto,
            public,
            U::WRAP_INFO,
            &aad(team, epoch, device),
            self.seed.as_slice(),
        )
    }

    /// The pair of `epoch`, from its wrap for this device (`self` being the device's own key of this use).
    pub fn unwrap_seed(
        &self,
        crypto: &impl OpenMlsCrypto,
        team: &[u8; 16],
        epoch: u64,
        device: &[u8; 16],
        sealed: &Sealed,
    ) -> Result<EpochKey<U>, Error> {
        let seed = Zeroizing::new(hpke_open(
            crypto,
            &self.keys.private,
            U::WRAP_INFO,
            &aad(team, epoch, device),
            sealed,
        )?);
        let seed: [u8; 32] = seed
            .as_slice()
            .try_into()
            .map_err(|_| Error::Encoding(format!("a {} key's seed is 32 bytes", U::NAME)))?;
        EpochKey::from_seed(crypto, seed)
    }
}
