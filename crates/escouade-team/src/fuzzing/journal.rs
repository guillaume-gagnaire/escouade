//! The members' journal as a device checks it, against a server that lies (`docs/TEAM-SERVER.md` §3.7).
//!
//! Bob holds epochs 1 to 3 of the team of [`World`]. The bytes are a list of claims, each read from a kind and its
//! bytes: a head cited for an epoch (by an object, a member's beacon, the server), the state the server shows, a
//! checkpoint put back, and Alice's beacon, object and history link changed by the bytes. The properties:
//! - a head that is not Bob's for an epoch he holds is a fork, never taken for his;
//! - an epoch older than his checkpoint, shown by the server or by a state put back, is a rollback;
//! - a beacon, an object or a link changed opens as the one Alice signed, or not at all: a false signature is never
//!   taken.

use escouade_team_crypto::beacon::Beacon;
use escouade_team_crypto::content::SealedObject;
use escouade_team_crypto::history::HistoryLink;
use escouade_team_crypto::{Checkpoint, Error, HeadCheck};

use super::{alter, World};
use crate::wire;

fn head_of(bytes: &[u8]) -> [u8; 32] {
    let mut h = [0u8; 32];
    for (i, b) in bytes.iter().take(32).enumerate() {
        h[i] = *b;
    }
    h
}

pub fn journal(data: &[u8]) {
    World::with(|w| claims(w, data));
}

fn claims(w: &World, data: &[u8]) {
    let bob = &w.bob;
    let seen = bob.checkpoint().expect("Bob has a checkpoint");
    let mut rest = data;
    while let Some((&kind, tail)) = rest.split_first() {
        let (&epoch, tail) = tail.split_first().unwrap_or((&0, &[]));
        let epoch = u64::from(epoch % 8);
        let n = tail.len().min(40);
        let (bytes, tail) = tail.split_at(n);
        rest = tail;
        // A head: Bob's own for that epoch, xored with the bytes (none: his very head).
        let own = bob.record(epoch).map(|r| r.head);
        let mut cited = own.unwrap_or_else(|| head_of(bytes));
        for (i, b) in bytes.iter().take(32).enumerate() {
            cited[i] ^= b;
        }
        match kind % 6 {
            0 => match bob.check_head(epoch, &cited) {
                Ok(HeadCheck::Same) => assert_eq!(Some(cited), own, "a forged head taken"),
                Ok(HeadCheck::Behind) => assert!(epoch > seen.epoch, "behind an epoch he holds"),
                Ok(HeadCheck::Unknown) => assert!(own.is_none(), "an epoch he holds unknown"),
                Err(Error::Fork { epoch: e }) => {
                    assert_eq!(e, epoch);
                    assert!(own.is_some_and(|h| h != cited), "a fork without a cause");
                }
                Err(e) => panic!("check_head: {e:?}"),
            },
            1 => {
                let shown = bob.check_server_state(epoch, (bytes.len() % 2 == 0).then_some(&cited));
                if epoch < seen.epoch {
                    assert!(
                        matches!(shown, Err(Error::Rollback { .. })),
                        "an older state shown and taken: {shown:?}"
                    );
                } else if let Ok(HeadCheck::Same) = shown {
                    assert!(
                        bytes.len() % 2 == 1 || Some(cited) == own,
                        "a forged state taken"
                    );
                }
            }
            2 => {
                let saved = Checkpoint { epoch, head: cited };
                let restored = bob.check_restored(&saved);
                let now = bob.epoch().expect("an epoch");
                if now < epoch {
                    assert!(matches!(restored, Err(Error::Rollback { .. })));
                } else if now == epoch && cited != seen.head {
                    assert!(matches!(restored, Err(Error::Fork { .. })));
                } else {
                    assert!(restored.is_ok(), "{restored:?}");
                }
            }
            3 => {
                let changed = alter(&wire::encode(&w.beacon), bytes);
                if let Ok(b) = wire::decode::<Beacon>(&changed) {
                    if let Ok(check) = bob.check_beacon(&b) {
                        // Signed: what it says is what Alice signed.
                        assert_eq!(
                            (b.epoch, b.head, b.at, &b.author),
                            (w.beacon.epoch, w.beacon.head, w.beacon.at, &w.beacon.author),
                            "a beacon changed and taken ({check:?})"
                        );
                    }
                }
            }
            4 => {
                let changed = alter(&wire::encode(&w.object), bytes);
                if let Ok(o) = wire::decode::<SealedObject>(&changed) {
                    if let Ok(plain) = bob.open(&o) {
                        assert_eq!(plain, w.plain, "an object changed opens as another");
                        assert_eq!(o.header, w.object.header, "a header changed and taken");
                    }
                }
            }
            _ => {
                let changed = alter(&wire::encode(&w.link), bytes);
                if let Ok(l) = wire::decode::<HistoryLink>(&changed) {
                    if let Some(current) = bob.record(l.epoch) {
                        if let Ok(previous) = l.open(&w.group, current) {
                            assert_eq!(
                                Some(&previous),
                                bob.record(l.epoch - 1),
                                "a history link changed opens as another past"
                            );
                        }
                    }
                }
            }
        }
    }
    // The raw bytes, as the server would serve them.
    let _ = wire::decode::<Beacon>(data);
    let _ = wire::decode::<SealedObject>(data);
    let _ = wire::decode::<HistoryLink>(data);
    let _ = wire::decode::<Checkpoint>(data);
    let _ = wire::decode::<wire::TeamMessage>(data);
}
