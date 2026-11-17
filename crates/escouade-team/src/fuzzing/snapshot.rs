//! A project's snapshot (TS-5) as it comes from the server and from a member: the bytes as a snapshot (its encoding
//! goes back and forth), as the state of a snapshot loaded into a copy (refused, or loaded whole: what loads dumps
//! and loads again the same), and a snapshot sealed by the fixture's device, changed by the bytes: it opens as the one
//! sealed, or not at all.

use escouade_team_crypto::ops::{Sealed, Snapshot};

use super::{alter, sealed_snapshot, PROJECT};
use crate::replica::Replica;
use crate::wire;

pub fn snapshot(data: &[u8]) {
    if let Ok(s) = Snapshot::from_bytes(data) {
        assert_eq!(Snapshot::from_bytes(&s.to_bytes()).expect("again"), s);
    }
    let copy = Replica::in_memory().expect("a copy");
    let upto = data.first().map_or(0, |b| u64::from(*b));
    if copy.load_snapshot(data, upto).is_ok() {
        let state = copy.state().expect("its state");
        let again = Replica::in_memory().expect("a copy");
        again
            .load_snapshot(&copy.dump().expect("a dump"), upto)
            .expect("what loaded loads again");
        assert_eq!(again.state().expect("its state"), state);
        let _ = copy.objects(None);
    }
    let (record, sealed, plain) = sealed_snapshot();
    if let Ok(s) = wire::decode::<Sealed>(&alter(sealed, data)) {
        if let Ok(snap) = s.open_snapshot(record, &PROJECT) {
            assert_eq!(
                snap.to_bytes(),
                *plain,
                "a snapshot changed opens as another"
            );
        }
    }
    if let Ok(s) = wire::decode::<Sealed>(data) {
        let _ = s.open_snapshot(record, &PROJECT);
    }
}
