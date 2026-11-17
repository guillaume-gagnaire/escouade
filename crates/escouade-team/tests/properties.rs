//! Properties where the fuzzing serves less (TS-18), by proptest: the hybrid clock (`escouade-team-crypto/src/hlc.rs`)
//! and the three-way merge of a text (`merge.rs`). The convergence of the copies, field by field, is `replica.rs`'s
//! proptest and the target `convergence` of `fuzz/`.

use escouade_team::merge::merge3;
use escouade_team_crypto::hlc::{Clock, Hlc, HLC_BYTES, MAX_DRIFT_MS};
use escouade_team_crypto::DeviceId;
use proptest::prelude::*;

/// What a device's clock does: a write at a wall clock, or a stamp seen from another device.
#[derive(Clone, Debug)]
enum Step {
    Tick { now: u64 },
    Observe { now: u64, wall: u64, count: u32 },
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        (0u64..10_000_000).prop_map(|now| Step::Tick { now }),
        (0u64..10_000_000, 0u64..20_000_000, any::<u32>())
            .prop_map(|(now, wall, count)| Step::Observe { now, wall, count }),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// A device's stamps always go up, whatever its wall clock does; a write made after seeing a stamp comes after
    /// it (when that stamp is within the drift's bound); and a stamp from far ahead moves the clock at most the
    /// drift's bound ahead of the wall clock.
    #[test]
    fn a_clock_never_goes_back_and_passes_what_it_saw(steps in prop::collection::vec(step(), 1..60)) {
        let me = DeviceId([1; 16]);
        let other = DeviceId([2; 16]);
        let mut clock = Clock::default();
        let mut last: Option<Hlc> = None;
        let mut seen: Option<Hlc> = None;
        let mut latest_now = 0u64;
        for s in steps {
            match s {
                Step::Tick { now } => {
                    latest_now = latest_now.max(now);
                    let h = clock.tick(now, me);
                    if let Some(l) = last {
                        prop_assert!(h > l, "{h:?} after {l:?}");
                    }
                    if let Some(o) = seen.take() {
                        prop_assert!(h > o, "{h:?} does not pass {o:?}, which it saw");
                    }
                    last = Some(h);
                }
                Step::Observe { now, wall, count } => {
                    latest_now = latest_now.max(now);
                    let before = clock;
                    let o = Hlc { wall, count, device: other };
                    clock.observe(&o, now);
                    prop_assert!(clock.wall >= before.wall, "observing moved the clock back");
                    prop_assert!(
                        clock.wall <= before.wall.max(now.saturating_add(MAX_DRIFT_MS)),
                        "a stamp from far ahead moved the clock past the drift"
                    );
                    // A stamp within the drift's bound must be passed by the next write; one beyond it is kept as it
                    // is by the copies, but this clock does not jump to it.
                    if wall <= now.saturating_add(MAX_DRIFT_MS) {
                        seen = Some(seen.map_or(o, |s: Hlc| s.max(o)));
                    }
                }
            }
        }
        let _ = latest_now;
    }

    /// The bytes of a stamp sort as the stamps do, and read back the same.
    #[test]
    fn the_bytes_of_a_stamp_sort_as_the_stamps(a in (any::<u64>(), any::<u32>(), any::<[u8; 16]>()), b in (any::<u64>(), any::<u32>(), any::<[u8; 16]>())) {
        let a = Hlc { wall: a.0, count: a.1, device: DeviceId(a.2) };
        let b = Hlc { wall: b.0, count: b.1, device: DeviceId(b.2) };
        prop_assert_eq!(a.cmp(&b), a.to_bytes().cmp(&b.to_bytes()));
        prop_assert_eq!(Hlc::from_bytes(&a.to_bytes()), Some(a));
        prop_assert_eq!(a.to_bytes().len(), HLC_BYTES);
    }
}

/// A text of lines drawn from a small alphabet (so that sides share lines), each ending with a newline.
fn text() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec("[a-d]{1,3}", 0..12)
}

fn joined(lines: &[String]) -> String {
    lines.iter().map(|l| format!("{l}\n")).collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// One side unchanged, or both the same: the other side whole. Never a panic, whatever the three texts.
    #[test]
    fn one_side_unchanged_gives_the_other(base in text(), side in text(), other in text()) {
        let (b, s, o) = (joined(&base), joined(&side), joined(&other));
        prop_assert_eq!(merge3(&b, &b, &s), Some(s.clone()));
        prop_assert_eq!(merge3(&b, &s, &b), Some(s.clone()));
        prop_assert_eq!(merge3(&b, &s, &s), Some(s.clone()));
        let _ = merge3(&b, &s, &o);
    }

    /// Two sides that change two lines of the base far apart: both changes are kept, the rest as it was.
    #[test]
    fn changes_far_apart_are_both_kept(
        base in prop::collection::vec("[a-d]{1,3}", 6..14),
        i in 0usize..3,
        gap in 3usize..6,
    ) {
        let j = (i + gap).min(base.len() - 1);
        prop_assume!(j >= i + 3);
        let mut ours = base.clone();
        ours[i] = "OURS".into();
        let mut theirs = base.clone();
        theirs[j] = "THEIRS".into();
        let mut both = base.clone();
        both[i] = "OURS".into();
        both[j] = "THEIRS".into();
        prop_assert_eq!(merge3(&joined(&base), &joined(&ours), &joined(&theirs)), Some(joined(&both)));
    }

    /// The merged text, when there is one, holds every line both sides added, and none that both removed.
    #[test]
    fn a_merge_keeps_what_both_added(base in text(), ours in text(), theirs in text()) {
        let (b, o, t) = (joined(&base), joined(&ours), joined(&theirs));
        if let Some(m) = merge3(&b, &o, &t) {
            let lines: Vec<&str> = m.lines().collect();
            for l in ours.iter().filter(|l| theirs.contains(l) && !base.contains(l)) {
                prop_assert!(lines.contains(&l.as_str()), "{l} added on both sides, lost in {m:?}");
            }
            for l in base.iter().filter(|l| !ours.contains(l) && !theirs.contains(l)) {
                prop_assert!(!lines.contains(&l.as_str()), "{l} removed on both sides, back in {m:?}");
            }
        }
    }
}
