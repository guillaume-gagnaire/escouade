//! The envelopes (HPKE) the server keeps for the team of [`World`]: the recovery wrap and the escrow wrap of epoch 3.
//! Changed by the bytes, each opens as the one sealed (the same record, the same GroupInfo) or not at all; the raw
//! bytes as wraps, as an epoch's record, and as the escrow key's texts (`escrow.rs`) are read without a panic, and a
//! team's half of an escrow key that reads is a key and its anchor.

use escouade_team_crypto::keys::EpochRecord;
use escouade_team_crypto::recovery::{EscrowWrap, RecoveryWrap};
use openmls_traits::OpenMlsProvider as _;

use super::{alter, World};
use crate::{escrow, wire};

pub fn envelope(data: &[u8]) {
    World::with(|w| {
        let crypto = w.alice.provider().crypto();
        let team = w.alice.fingerprint().expect("a team");
        let sealed = w.alice.record(3).expect("epoch 3");
        let changed = alter(&wire::encode(&w.escrow_wrap), data);
        if let Ok(wrap) = wire::decode::<EscrowWrap>(&changed) {
            if let Ok(record) = w.escrow.open(crypto, &team, &w.group, &wrap) {
                assert_eq!(
                    &record, sealed,
                    "an escrow wrap changed opens as another epoch"
                );
            }
        }
        let changed = alter(&wire::encode(&w.recovery_wrap), data);
        if let Ok(wrap) = wire::decode::<RecoveryWrap>(&changed) {
            if let Ok(opened) = wrap.open(crypto, &w.recovery, &w.group) {
                let first = w
                    .recovery_wrap
                    .open(crypto, &w.recovery, &w.group)
                    .expect("the wrap opens");
                assert_eq!(
                    &opened.record, sealed,
                    "a recovery wrap changed opens as another epoch"
                );
                assert_eq!(
                    opened.group_info, first.group_info,
                    "a recovery wrap changed gives another GroupInfo"
                );
            }
        }
        // As they come, raw.
        if let Ok(wrap) = wire::decode::<EscrowWrap>(data) {
            let _ = w.escrow.open(crypto, &team, &w.group, &wrap);
        }
        if let Ok(wrap) = wire::decode::<RecoveryWrap>(data) {
            let _ = wrap.open(crypto, &w.recovery, &w.group);
        }
    });
    if let Ok(r) = EpochRecord::from_bytes(data) {
        assert_eq!(EpochRecord::from_bytes(&r.to_bytes()).expect("again"), r);
    }
    if let Ok(text) = std::str::from_utf8(data) {
        if let Ok(half) = escrow::parse_team(text, "Org") {
            assert_eq!(
                half.public.hpke_key.len(),
                32,
                "a public escrow key of another size"
            );
            assert_eq!(
                half.public.anchor_key,
                half.anchor.public(),
                "an escrow anchor that is not its own"
            );
        }
        let _ = escrow::open_secret(text, "Org");
    }
}
