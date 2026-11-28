//! The administrators' key of the costs (TS-16, `docs/TEAM-SERVER.md` §10): an HPKE key pair (RFC 9180, the suite's
//! KEM, KDF and AEAD) apart from the team's MLS keys, an [`EpochKey`] of the use [`Costs`] (what it shares with the key
//! of the audit, AU-3, is in `epochkey.rs`; its labels and its bytes are those of TS-16, unchanged).
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

use crate::epochkey::{EpochKey, Use};
use crate::recovery::Sealed;
use crate::Error;

/// The use of the costs: its labels are TS-16's.
pub struct Costs;

impl Use for Costs {
    const NAME: &'static str = "cost";
    const TYPE: &'static str = "CostKey";
    const SEAL_INFO: &'static [u8] = b"escouade/cost-totals/v1";
    const WRAP_INFO: &'static [u8] = b"escouade/cost-key-wrap/v1";
}

/// An HPKE key pair of the costs: the administrators' (its epoch), or a device's own (to receive the wraps).
pub type CostKey = EpochKey<Costs>;

impl EpochKey<Costs> {
    /// The totals of `device`, sealed to the administrators' key `public` of `epoch`.
    pub fn seal_totals(
        crypto: &impl OpenMlsCrypto,
        public: &[u8],
        team: &[u8; 16],
        epoch: u64,
        device: &[u8; 16],
        plain: &[u8],
    ) -> Result<Sealed, Error> {
        Self::seal(crypto, public, team, epoch, device, plain)
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
        self.open(crypto, team, epoch, device, sealed)
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

    /// AU-3 made the code common: the bytes of the costs did not move. Their labels and associated data are TS-16's,
    /// opened here by hand, as a client of before would.
    #[test]
    fn the_labels_and_the_associated_data_of_the_costs_are_those_of_ts_16() {
        use crate::recovery::hpke_open;
        let p = OpenMlsRustCrypto::default();
        let admins = CostKey::generate(p.crypto(), p.rand()).unwrap();
        let mut aad = Vec::new();
        aad.extend_from_slice(&TEAM);
        aad.extend_from_slice(&5u64.to_be_bytes());
        aad.extend_from_slice(&BOB);
        let pair = p
            .crypto()
            .derive_hpke_keypair(crate::CIPHERSUITE.hpke_config(), admins.seed())
            .unwrap();
        let sealed =
            CostKey::seal_totals(p.crypto(), &admins.public(), &TEAM, 5, &BOB, b"totals").unwrap();
        let plain = hpke_open(
            p.crypto(),
            &pair.private,
            b"escouade/cost-totals/v1",
            &aad,
            &sealed,
        )
        .unwrap();
        assert_eq!(plain, b"totals");
        let device = CostKey::generate(p.crypto(), p.rand()).unwrap();
        let wrap = admins
            .wrap_seed(p.crypto(), &device.public(), &TEAM, 5, &BOB)
            .unwrap();
        let own = p
            .crypto()
            .derive_hpke_keypair(crate::CIPHERSUITE.hpke_config(), device.seed())
            .unwrap();
        let seed = hpke_open(
            p.crypto(),
            &own.private,
            b"escouade/cost-key-wrap/v1",
            &aad,
            &wrap,
        )
        .unwrap();
        assert_eq!(seed.as_slice(), admins.seed());
        assert_eq!(format!("{admins:?}").split(' ').next(), Some("CostKey"));
    }
}
