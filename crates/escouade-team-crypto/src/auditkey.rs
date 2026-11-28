//! The key of the audit of the stations (AU-3, `docs/TEAM-AUDIT.md` §3.3, S3): an HPKE key pair apart from the team's
//! MLS keys and from the key of the costs, an [`EpochKey`] of the use [`Audit`], on the model of the costs' (TS-16).
//!
//! - Every device seals the facts of its station to its public half ([`AuditKey::seal_facts`]): the server, which keeps
//!   them, and the members, the device's own member included, cannot open them.
//! - Its seed is wrapped for the devices that emit the audit alone (the machine of the team under the role of service,
//!   OE-1, when the team has one; the administrators' devices otherwise, the recourse of A2-A), each to its own key of
//!   the audit ([`AuditKey::wrap_seed`]). A new pair is drawn when one of them leaves.
//! - Its labels are its own: a wrap or a ciphertext of the costs never opens as the audit's, nor the reverse.

use openmls_traits::crypto::OpenMlsCrypto;

use crate::epochkey::{EpochKey, Use};
use crate::recovery::Sealed;
use crate::Error;

/// The use of the audit of the stations.
pub struct Audit;

impl Use for Audit {
    const NAME: &'static str = "audit";
    const TYPE: &'static str = "AuditKey";
    const SEAL_INFO: &'static [u8] = b"escouade/audit-facts/v1";
    const WRAP_INFO: &'static [u8] = b"escouade/audit-key-wrap/v1";
}

/// An HPKE key pair of the audit: the emitters' (its epoch), or a device's own (to receive the wraps).
pub type AuditKey = EpochKey<Audit>;

impl EpochKey<Audit> {
    /// The facts of `device`'s station, sealed to the emitters' key `public` of `epoch`.
    pub fn seal_facts(
        crypto: &impl OpenMlsCrypto,
        public: &[u8],
        team: &[u8; 16],
        epoch: u64,
        device: &[u8; 16],
        plain: &[u8],
    ) -> Result<Sealed, Error> {
        Self::seal(crypto, public, team, epoch, device, plain)
    }

    /// Facts sealed to this key (the emitters' of `epoch`) by `device`.
    pub fn open_facts(
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
    use crate::CostKey;
    use openmls_rust_crypto::OpenMlsRustCrypto;
    use openmls_traits::OpenMlsProvider;

    const TEAM: [u8; 16] = [1; 16];
    const STATION: [u8; 16] = [2; 16];
    const MACHINE: [u8; 16] = [3; 16];

    #[test]
    fn facts_open_with_the_emitters_key_alone_and_only_as_sealed() {
        let p = OpenMlsRustCrypto::default();
        let emitters = AuditKey::generate(p.crypto(), p.rand()).unwrap();
        let other = AuditKey::generate(p.crypto(), p.rand()).unwrap();
        let sealed =
            AuditKey::seal_facts(p.crypto(), &emitters.public(), &TEAM, 1, &STATION, b"facts")
                .unwrap();
        assert_eq!(
            emitters
                .open_facts(p.crypto(), &TEAM, 1, &STATION, &sealed)
                .unwrap(),
            b"facts"
        );
        assert!(other
            .open_facts(p.crypto(), &TEAM, 1, &STATION, &sealed)
            .is_err());
        assert!(emitters
            .open_facts(p.crypto(), &TEAM, 2, &STATION, &sealed)
            .is_err());
        assert!(emitters
            .open_facts(p.crypto(), &TEAM, 1, &MACHINE, &sealed)
            .is_err());
        assert!(emitters
            .open_facts(p.crypto(), &[9; 16], 1, &STATION, &sealed)
            .is_err());
    }

    #[test]
    fn a_wrap_gives_the_pair_back_to_its_emitter_alone() {
        let p = OpenMlsRustCrypto::default();
        let emitters = AuditKey::generate(p.crypto(), p.rand()).unwrap();
        let machine = AuditKey::generate(p.crypto(), p.rand()).unwrap();
        let station = AuditKey::generate(p.crypto(), p.rand()).unwrap();
        let wrap = emitters
            .wrap_seed(p.crypto(), &machine.public(), &TEAM, 4, &MACHINE)
            .unwrap();
        let back = machine
            .unwrap_seed(p.crypto(), &TEAM, 4, &MACHINE, &wrap)
            .unwrap();
        assert_eq!(back.public(), emitters.public());
        assert!(station
            .unwrap_seed(p.crypto(), &TEAM, 4, &MACHINE, &wrap)
            .is_err());
        assert!(machine
            .unwrap_seed(p.crypto(), &TEAM, 5, &MACHINE, &wrap)
            .is_err());
    }

    /// The two uses never cross: what is sealed for the costs does not open as facts, a wrap of the costs' key does
    /// not give the audit's (the same seed, the same pair, other labels).
    #[test]
    fn the_audit_and_the_costs_never_open_each_others() {
        let p = OpenMlsRustCrypto::default();
        let costs = CostKey::generate(p.crypto(), p.rand()).unwrap();
        let audit = AuditKey::from_seed(p.crypto(), *costs.seed()).unwrap();
        assert_eq!(audit.public(), costs.public());
        let totals =
            CostKey::seal_totals(p.crypto(), &costs.public(), &TEAM, 1, &STATION, b"x").unwrap();
        assert!(audit
            .open_facts(p.crypto(), &TEAM, 1, &STATION, &totals)
            .is_err());
        let facts =
            AuditKey::seal_facts(p.crypto(), &audit.public(), &TEAM, 1, &STATION, b"x").unwrap();
        assert!(costs
            .open_totals(p.crypto(), &TEAM, 1, &STATION, &facts)
            .is_err());
        let own_costs = CostKey::generate(p.crypto(), p.rand()).unwrap();
        let own_audit = AuditKey::from_seed(p.crypto(), *own_costs.seed()).unwrap();
        let wrap = costs
            .wrap_seed(p.crypto(), &own_costs.public(), &TEAM, 1, &MACHINE)
            .unwrap();
        assert!(own_audit
            .unwrap_seed(p.crypto(), &TEAM, 1, &MACHINE, &wrap)
            .is_err());
        assert!(format!("{audit:?}").starts_with("AuditKey"));
    }
}
