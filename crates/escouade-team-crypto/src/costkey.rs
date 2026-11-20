//! The administrators' key of the costs (TS-16, `docs/TEAM-SERVER.md` §10): an HPKE key pair (RFC 9180, the suite's
//! KEM, KDF and AEAD) apart from the team's MLS keys.
//!
//! - Every member seals the totals of its device to its public half ([`CostKey::seal_totals`]): the server, which
//!   keeps them, and the other members, who can read the stream, cannot open them; the member keeps its own totals in
//!   the clear on its device.
//! - Its secret half (the seed of the pair) is wrapped, by HPKE too, for each administrator's device: to the device's
//!   own key of the costs ([`CostKey::wrap_seed`]), a pair drawn on the device and kept in its encrypted state.
//! - A new pair is drawn when the administrators change (one leaves, or loses the role): the one who left keeps the
//!   totals sealed before, never those after. A new administrator's device gets the wrap of the current pair.
//!
//! The associated data bind a ciphertext to its team, to the epoch of the key and to its author (the device, for
//! the totals; the device it is for, for a wrap): the server cannot move a ciphertext to another team or epoch, nor
//! pass one device's totals for another's.

use openmls_traits::crypto::OpenMlsCrypto;
use openmls_traits::random::OpenMlsRand;
use openmls_traits::types::HpkeKeyPair;
use zeroize::Zeroizing;

use crate::recovery::{hpke_open, hpke_seal, Sealed};
use crate::{Error, CIPHERSUITE};

const TOTALS_INFO: &[u8] = b"escouade/cost-totals/v1";
const WRAP_INFO: &[u8] = b"escouade/cost-key-wrap/v1";

/// An HPKE key pair of the costs: the administrators' (its epoch), or a device's own (to receive the wraps).
pub struct CostKey {
    keys: HpkeKeyPair,
    seed: Zeroizing<[u8; 32]>,
}

impl std::fmt::Debug for CostKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CostKey")
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

impl CostKey {
    pub fn generate(crypto: &impl OpenMlsCrypto, rand: &impl OpenMlsRand) -> Result<Self, Error> {
        let seed: [u8; 32] = rand.random_array().map_err(|_| Error::Random)?;
        Self::from_seed(crypto, seed)
    }

    /// The pair again from its seed (what a wrap carries, what the device's state keeps).
    pub fn from_seed(crypto: &impl OpenMlsCrypto, seed: [u8; 32]) -> Result<Self, Error> {
        let seed = Zeroizing::new(seed);
        let keys = crypto
            .derive_hpke_keypair(CIPHERSUITE.hpke_config(), seed.as_slice())
            .map_err(|e| Error::Mls(format!("cost HPKE key: {e:?}")))?;
        Ok(Self { keys, seed })
    }

    pub fn seed(&self) -> &[u8; 32] {
        &self.seed
    }

    pub fn public(&self) -> Vec<u8> {
        self.keys.public.clone()
    }

    /// The totals of `device`, sealed to the administrators' key `public` of `epoch`.
    pub fn seal_totals(
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
            TOTALS_INFO,
            &aad(team, epoch, device),
            plain,
        )
    }

    /// Totals sealed to this key (the administrators' of `epoch`) by `device`.
    pub fn open_totals(
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
            TOTALS_INFO,
            &aad(team, epoch, device),
            sealed,
        )
    }

    /// This pair's seed (the administrators' of `epoch`), wrapped for the device `device`, whose own key of the costs
    /// is `public`.
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
            WRAP_INFO,
            &aad(team, epoch, device),
            self.seed.as_slice(),
        )
    }

    /// The administrators' pair of `epoch`, from its wrap for this device (`self` being the device's own key).
    pub fn unwrap_seed(
        &self,
        crypto: &impl OpenMlsCrypto,
        team: &[u8; 16],
        epoch: u64,
        device: &[u8; 16],
        sealed: &Sealed,
    ) -> Result<CostKey, Error> {
        let seed = Zeroizing::new(hpke_open(
            crypto,
            &self.keys.private,
            WRAP_INFO,
            &aad(team, epoch, device),
            sealed,
        )?);
        let seed: [u8; 32] = seed
            .as_slice()
            .try_into()
            .map_err(|_| Error::Encoding("a cost key's seed is 32 bytes".into()))?;
        CostKey::from_seed(crypto, seed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openmls_rust_crypto::OpenMlsRustCrypto;
    use openmls_traits::OpenMlsProvider;

    const TEAM: [u8; 16] = [1; 16];
    const ALICE: [u8; 16] = [2; 16];
    const BOB: [u8; 16] = [3; 16];

    #[test]
    fn totals_open_with_the_administrators_key_alone_and_only_as_sealed() {
        let p = OpenMlsRustCrypto::default();
        let admins = CostKey::generate(p.crypto(), p.rand()).unwrap();
        let other = CostKey::generate(p.crypto(), p.rand()).unwrap();
        let sealed =
            CostKey::seal_totals(p.crypto(), &admins.public(), &TEAM, 1, &BOB, b"totals").unwrap();
        assert_eq!(
            admins
                .open_totals(p.crypto(), &TEAM, 1, &BOB, &sealed)
                .unwrap(),
            b"totals"
        );
        // Another key, another epoch, another author, another team: nothing opens.
        assert!(other
            .open_totals(p.crypto(), &TEAM, 1, &BOB, &sealed)
            .is_err());
        assert!(admins
            .open_totals(p.crypto(), &TEAM, 2, &BOB, &sealed)
            .is_err());
        assert!(admins
            .open_totals(p.crypto(), &TEAM, 1, &ALICE, &sealed)
            .is_err());
        assert!(admins
            .open_totals(p.crypto(), &[9; 16], 1, &BOB, &sealed)
            .is_err());
    }

    #[test]
    fn a_wrap_gives_the_pair_back_to_its_device_alone() {
        let p = OpenMlsRustCrypto::default();
        let admins = CostKey::generate(p.crypto(), p.rand()).unwrap();
        let alice = CostKey::generate(p.crypto(), p.rand()).unwrap();
        let bob = CostKey::generate(p.crypto(), p.rand()).unwrap();
        let wrap = admins
            .wrap_seed(p.crypto(), &alice.public(), &TEAM, 3, &ALICE)
            .unwrap();
        let back = alice
            .unwrap_seed(p.crypto(), &TEAM, 3, &ALICE, &wrap)
            .unwrap();
        assert_eq!(back.public(), admins.public());
        assert!(bob
            .unwrap_seed(p.crypto(), &TEAM, 3, &ALICE, &wrap)
            .is_err());
        assert!(alice
            .unwrap_seed(p.crypto(), &TEAM, 3, &BOB, &wrap)
            .is_err());
        // The seed gives the same pair again.
        let again = CostKey::from_seed(p.crypto(), *admins.seed()).unwrap();
        assert_eq!(again.public(), admins.public());
    }
}
