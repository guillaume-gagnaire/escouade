use super::*;
use escouade_base::testing::test_dir;

fn at(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

fn copy(at: DateTime<Utc>) -> Copy {
    Copy { at, size: 1000 }
}

/// A state that reads, with `n` projects.
fn state(n: usize) -> Vec<u8> {
    let projects: Vec<_> = (0..n)
        .map(|i| serde_json::json!({ "id": format!("p{i}"), "name": format!("P{i}"), "path": "/tmp", "color": "#888" }))
        .collect();
    let v = serde_json::json!({ "projects": projects, "tickets": [] });
    let bytes = serde_json::to_vec_pretty(&v).unwrap();
    assert!(reads("state.json", &bytes), "the test's state reads");
    bytes
}

/// A copy of `file` with `bytes`, made at `at`, as `back_up` makes them.
fn put_copy(data: &Path, file: &str, at: DateTime<Utc>, bytes: &[u8]) -> PathBuf {
    let dir = copies_dir(data, file);
    std::fs::create_dir_all(&dir).unwrap();
    let mut z = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    z.write_all(bytes).unwrap();
    let path = dir.join(copy_name(file, at));
    std::fs::write(&path, z.finish().unwrap()).unwrap();
    path
}

#[test]
fn the_rotation_keeps_the_latest_then_one_per_hour_then_one_per_day() {
    let now = at("2026-11-15T10:00:00Z");
    // A copy every 10 minutes for 40 days.
    let copies: Vec<Copy> = (0..40 * 24 * 6)
        .map(|i| copy(now - Duration::minutes(10 * i)))
        .collect();
    let keep = kept(&copies, now, POLICY);
    let kept_at: Vec<DateTime<Utc>> = copies
        .iter()
        .zip(&keep)
        .filter(|(_, k)| **k)
        .map(|(c, _)| c.at)
        .collect();
    // The 6 newest.
    for c in &copies[..6] {
        assert!(kept_at.contains(&c.at));
    }
    // The first of each of the last 48 hours: on the hour, as the copies fall.
    for h in 1..48 {
        assert!(kept_at.contains(&(now - Duration::hours(h))), "hour {h}");
    }
    // The first of each of the last 30 days: midnight.
    for d in 1..30 {
        let midnight = at("2026-11-15T00:00:00Z") - Duration::days(d);
        assert!(kept_at.contains(&midnight), "day {d}");
    }
    // Nothing older than 30 days, nothing in between.
    assert!(kept_at.iter().all(|t| now - *t <= Duration::days(30)));
    assert!(kept_at.len() <= 6 + 48 + 30, "{} kept", kept_at.len());
    assert!(kept_at.len() >= 48 + 29);
}

#[test]
fn copies_made_again_and_again_push_out_the_latest_ones_only() {
    let now = at("2026-11-15T10:59:00Z");
    // One good copy per hour over 3 days, then 200 copies of an emptied state in the last hour.
    let mut copies: Vec<Copy> = (1..72).map(|h| copy(now - Duration::hours(h))).collect();
    let good = copies.len();
    copies.extend((0..200).map(|i| Copy {
        at: now - Duration::seconds(i),
        size: 2,
    }));
    let keep = kept(&copies, now, POLICY);
    // Every good copy of the last 48 hours stays, and those of the days before.
    for (i, c) in copies[..good].iter().enumerate() {
        if now - c.at < Duration::hours(48) {
            assert!(keep[i], "the good copy of {} stays", c.at);
        }
    }
    assert!(keep[..good].iter().filter(|k| **k).count() >= 47);
    // Of the 200, the 6 newest and the first of their hour.
    assert_eq!(keep[good..].iter().filter(|k| **k).count(), 7);
}

#[test]
fn the_rotation_holds_its_size_and_always_keeps_the_newest_copy() {
    let now = at("2026-11-15T10:00:00Z");
    let policy = Policy {
        max_bytes: 2500,
        ..POLICY
    };
    let copies: Vec<Copy> = (0..10).map(|i| copy(now - Duration::hours(i))).collect();
    let keep = kept(&copies, now, policy);
    // 1000 bytes each: the two newest.
    assert_eq!(
        keep,
        [true, true, false, false, false, false, false, false, false, false]
    );
    // One copy bigger than everything allowed: kept all the same, alone.
    let big = [
        Copy {
            at: now,
            size: 10_000,
        },
        copy(now - Duration::hours(1)),
    ];
    assert_eq!(kept(&big, now, policy), [true, false]);
    assert!(kept(&[], now, policy).is_empty());
}

#[test]
fn a_copy_is_made_of_a_file_that_reads_and_differs_only() {
    let data = test_dir("esc469-back-up");
    let now = at("2026-11-15T10:00:00Z");
    assert_eq!(
        back_up(&data, "state.json", now).unwrap(),
        BackedUp::Missing
    );
    std::fs::write(data.join("state.json"), state(2)).unwrap();
    let BackedUp::Made(first) = back_up(&data, "state.json", now).unwrap() else {
        panic!("a copy")
    };
    assert_eq!(read_copy(&first).unwrap(), state(2));
    assert!(first.ends_with("backups/state/state-20261115T100000Z.json.gz"));
    // The same content: no copy.
    let later = now + Duration::minutes(10);
    assert_eq!(back_up(&data, "state.json", later).unwrap(), BackedUp::Same);
    // A file of zeros: no copy, the good one stays.
    std::fs::write(data.join("state.json"), vec![0u8; 4096]).unwrap();
    assert_eq!(
        back_up(&data, "state.json", later).unwrap(),
        BackedUp::Unreadable
    );
    assert_eq!(list(&data, "state.json").len(), 1);
    // Changed: a second copy.
    std::fs::write(data.join("state.json"), state(3)).unwrap();
    assert!(matches!(
        back_up(&data, "state.json", later).unwrap(),
        BackedUp::Made(_)
    ));
    assert_eq!(list(&data, "state.json").len(), 2);
    // The other files are copied as JSON objects.
    std::fs::write(data.join("plugins.json"), br#"{"plugins":[]}"#).unwrap();
    assert!(matches!(
        back_up(&data, "plugins.json", now).unwrap(),
        BackedUp::Made(_)
    ));
    std::fs::write(data.join("integrations.json"), b"[1]").unwrap();
    assert_eq!(
        back_up(&data, "integrations.json", now).unwrap(),
        BackedUp::Unreadable
    );
}

#[test]
fn damages_are_told_apart() {
    assert_eq!(damage(&[0; 100]), Damage::Zeros);
    assert_eq!(damage(b""), Damage::Zeros);
    assert_eq!(damage(br#"{"projects": [{"id": "p"#), Damage::Cut);
    assert_eq!(damage(b"hello"), Damage::Invalid);
    let mut tail = state(1);
    tail.extend_from_slice(&[0; 64]);
    assert_eq!(damage(&tail), Damage::Invalid);
}

#[test]
fn a_state_of_zeros_takes_the_newest_copy_that_reads_and_is_kept_under_a_dated_name() {
    let data = test_dir("esc469-recover-zeros");
    let now = at("2026-11-15T10:40:00Z");
    put_copy(&data, "state.json", at("2026-11-15T09:00:00Z"), &state(1));
    let good = put_copy(&data, "state.json", at("2026-11-15T10:20:00Z"), &state(2));
    // The newest copy does not read: passed over.
    put_copy(&data, "state.json", at("2026-11-15T10:30:00Z"), &[0; 32]);
    let zeros = vec![0u8; 8 << 10];
    std::fs::write(data.join("state.json"), &zeros).unwrap();
    let r = recover(&data, "state.json", now).expect("a recovery");
    assert_eq!(r.damage, Damage::Zeros);
    assert_eq!(r.restored.as_deref(), Some(good.as_path()));
    assert_eq!(r.restored_at, Some(at("2026-11-15T10:20:00Z")));
    assert_eq!(r.skipped, 1);
    assert_eq!(std::fs::read(data.join("state.json")).unwrap(), state(2));
    let kept = r.kept_as.clone().unwrap();
    assert_eq!(kept, data.join("state.unreadable-20261115T104000Z.json"));
    assert_eq!(std::fs::read(&kept).unwrap(), zeros);
    assert!(r.written_at.is_some());
    // Read again, it reads: nothing more to do.
    assert_eq!(recover(&data, "state.json", now), None);
}

#[test]
fn a_cut_or_false_file_is_recovered_the_same_and_never_set_aside_over_another() {
    let data = test_dir("esc469-recover-cut");
    let now = at("2026-11-15T10:40:00Z");
    put_copy(
        &data,
        "settings.json",
        at("2026-11-15T09:00:00Z"),
        br#"{"language":"en"}"#,
    );
    std::fs::write(data.join("settings.json"), br#"{"language": "e"#).unwrap();
    let r = recover(&data, "settings.json", now).unwrap();
    assert_eq!(r.damage, Damage::Cut);
    assert_eq!(
        std::fs::read(data.join("settings.json")).unwrap(),
        br#"{"language":"en"}"#
    );
    // Damaged again within the same second: a name of its own.
    std::fs::write(data.join("settings.json"), b"not json").unwrap();
    let again = recover(&data, "settings.json", now).unwrap();
    assert_eq!(again.damage, Damage::Invalid);
    assert_eq!(
        again.kept_as.unwrap(),
        data.join("settings.unreadable-20261115T104000Z-2.json")
    );
    assert_eq!(
        std::fs::read(r.kept_as.unwrap()).unwrap(),
        br#"{"language": "e"#,
        "the first one stays"
    );
    // plugins.json the same.
    put_copy(
        &data,
        "plugins.json",
        at("2026-11-15T09:00:00Z"),
        br#"{"plugins":[]}"#,
    );
    std::fs::write(data.join("plugins.json"), [0u8; 10]).unwrap();
    assert!(recover(&data, "plugins.json", now)
        .unwrap()
        .restored
        .is_some());
}

#[test]
fn without_a_copy_that_reads_the_file_is_kept_and_the_app_starts_without_it() {
    let data = test_dir("esc469-recover-none");
    let now = at("2026-11-15T10:40:00Z");
    put_copy(&data, "state.json", at("2026-11-15T10:30:00Z"), b"zz");
    std::fs::write(data.join("state.json"), [0u8; 64]).unwrap();
    let r = recover(&data, "state.json", now).unwrap();
    assert_eq!((r.restored, r.restored_at, r.skipped), (None, None, 1));
    assert!(!data.join("state.json").exists());
    assert_eq!(std::fs::read(r.kept_as.unwrap()).unwrap(), [0u8; 64]);
}

#[test]
fn a_missing_or_readable_file_is_nothing_to_tell() {
    let data = test_dir("esc469-recover-fresh");
    let now = at("2026-11-15T10:40:00Z");
    assert!(recover_all(&data, now).is_empty());
    assert!(notices(&data).is_empty());
    std::fs::write(data.join("state.json"), state(1)).unwrap();
    std::fs::write(data.join("settings.json"), b"{}").unwrap();
    assert!(recover_all(&data, now).is_empty());
    assert!(!data.join(NOTICES).exists());
}

#[test]
fn the_recoveries_stay_noted_until_the_notice_is_closed() {
    let data = test_dir("esc469-notices");
    let now = at("2026-11-15T10:40:00Z");
    put_copy(&data, "state.json", at("2026-11-15T10:20:00Z"), &state(2));
    std::fs::write(data.join("state.json"), [0u8; 64]).unwrap();
    std::fs::write(data.join("plugins.json"), b"{").unwrap();
    let found = recover_all(&data, now);
    assert_eq!(found.len(), 2);
    assert_eq!(notices(&data), found);
    // Another start: they are still there, with the new ones.
    std::fs::write(data.join("settings.json"), [0u8; 8]).unwrap();
    assert_eq!(recover_all(&data, now).len(), 1);
    assert_eq!(notices(&data).len(), 3);
    dismiss(&data).unwrap();
    assert!(notices(&data).is_empty());
    dismiss(&data).unwrap();
}

#[test]
fn a_state_saved_after_a_recovery_pushes_out_no_copy() {
    let data = test_dir("esc469-after-recovery");
    let start = at("2026-11-15T10:40:00Z");
    // One good copy per hour over the 2 days before.
    for h in 1..48 {
        put_copy(&data, "state.json", start - Duration::hours(h), &state(5));
    }
    std::fs::write(data.join("state.json"), [0u8; 64]).unwrap();
    recover(&data, "state.json", start).unwrap();
    // The app runs on, its state saved and copied every 10 minutes for two hours: emptied by the user, say.
    for i in 0..12 {
        std::fs::write(data.join("state.json"), state(i % 2)).unwrap();
        back_up(
            &data,
            "state.json",
            start + Duration::minutes(10 * i as i64),
        )
        .unwrap();
    }
    let now = start + Duration::minutes(110);
    let left: Vec<DateTime<Utc>> = list(&data, "state.json")
        .iter()
        .map(|(_, c)| c.at)
        .collect();
    for h in 1..46 {
        assert!(
            left.contains(&(start - Duration::hours(h))),
            "the copy of {h} h before stays"
        );
    }
    assert!(left.iter().all(|t| now - *t <= Duration::days(30)));
}
