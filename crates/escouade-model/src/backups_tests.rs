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

// ESC-487: the copies of what else is not made again, their bound, and a copy taken back by hand.

/// A closed project's history that reads, of the project `id`, with `tickets` tickets.
fn history(id: &str, tickets: usize) -> Vec<u8> {
    let tickets: Vec<_> = (0..tickets)
        .map(|i| serde_json::json!({ "id": format!("t{i}"), "key": format!("ATL-{i}"), "title": "x" }))
        .collect();
    serde_json::to_vec(&serde_json::json!({
        "project": { "id": id, "name": "atelier", "path": "/tmp", "color": "#888" },
        "agents": [], "tickets": tickets, "closedAt": 1
    }))
    .unwrap()
}

#[test]
fn the_closed_projects_histories_and_the_workflow_tracks_are_copied_with_their_own_rotation() {
    let data = test_dir("backups-more");
    std::fs::create_dir_all(data.join("closed-projects")).unwrap();
    std::fs::write(data.join("closed-projects/abc.json"), history("abc", 2)).unwrap();
    // Set aside or not named by a plain id: never copied.
    std::fs::write(
        data.join("closed-projects/abc.before-restore-20261116T100000Z.json"),
        history("abc", 1),
    )
    .unwrap();
    std::fs::create_dir_all(data.join("projects/p1")).unwrap();
    std::fs::write(data.join("projects/p1/workflow-tracks.json"), b"{}").unwrap();
    std::fs::write(data.join("sync-queue.json"), b"[]").unwrap();
    let files = copied(&data);
    assert!(files.contains(&"closed-projects/abc.json".to_string()));
    assert!(files.contains(&"projects/p1/workflow-tracks.json".to_string()));
    assert_eq!(
        files
            .iter()
            .filter(|f| f.starts_with("closed-projects/"))
            .count(),
        1
    );
    let now = at("2026-11-16T10:00:00Z");
    back_up_all(&data, now);
    let copies = list(&data, "closed-projects/abc.json");
    assert_eq!(copies.len(), 1);
    assert_eq!(
        copies[0].0,
        data.join("backups")
            .join("closed-projects")
            .join("abc")
            .join("abc-20261116T100000Z.json.gz")
    );
    assert_eq!(read_copy(&copies[0].0).unwrap(), history("abc", 2));
    assert_eq!(list(&data, "projects/p1/workflow-tracks.json").len(), 1);
    assert_eq!(list(&data, "sync-queue.json").len(), 1);
    // A history that does not read as its project's is not copied.
    std::fs::write(data.join("closed-projects/abc.json"), history("other", 3)).unwrap();
    assert_eq!(
        back_up(
            &data,
            "closed-projects/abc.json",
            now + Duration::minutes(10)
        )
        .unwrap(),
        BackedUp::Unreadable
    );
    // The same rotation as the four others, 32 MB at most per file.
    assert_eq!(policy("closed-projects/abc.json").max_bytes, 32 << 20);
    assert_eq!(policy("state.json"), POLICY);
    assert!(!is_copied("closed-projects/../state.json"));
    assert!(!is_copied("projects/../../x/workflow-tracks.json"));
    assert!(!is_copied("conversations/a.jsonl"));
}

#[test]
fn the_copies_of_a_history_gone_stay_thirty_days_and_are_known_meanwhile() {
    let data = test_dir("backups-orphans");
    let now = at("2026-11-16T10:00:00Z");
    put_copy(
        &data,
        "closed-projects/lost.json",
        now - Duration::days(2),
        &history("lost", 4),
    );
    put_copy(
        &data,
        "closed-projects/old.json",
        now - Duration::days(31),
        &history("old", 1),
    );
    let found = known(&data);
    assert!(found.contains(&"closed-projects/lost.json".to_string()));
    assert!(found.contains(&"closed-projects/old.json".to_string()));
    prune_orphans(&data, now);
    assert_eq!(list(&data, "closed-projects/lost.json").len(), 1);
    assert!(list(&data, "closed-projects/old.json").is_empty());
    assert!(!copies_dir(&data, "closed-projects/old.json").exists());
    // The four files' copies always stay, their file gone or not.
    put_copy(&data, "state.json", now - Duration::days(40), &state(1));
    prune_orphans(&data, now);
    assert_eq!(list(&data, "state.json").len(), 1);
}

#[test]
fn all_the_copies_together_stay_under_their_bound_the_newest_of_each_file_kept() {
    let data = test_dir("backups-total");
    let now = at("2026-11-16T10:00:00Z");
    for h in 0..5 {
        put_copy(
            &data,
            "state.json",
            now - Duration::hours(h),
            &state(h as usize + 1),
        );
        put_copy(
            &data,
            "closed-projects/abc.json",
            now - Duration::hours(h),
            &history("abc", h as usize),
        );
    }
    let total = total_size(&data);
    // A bound under the newest two: only the newest of each file stays.
    prune_total(&data, 1);
    assert_eq!(list(&data, "state.json").len(), 1);
    assert_eq!(list(&data, "closed-projects/abc.json").len(), 1);
    assert_eq!(list(&data, "state.json")[0].1.at, now);
    assert!(total_size(&data) < total);
    // Under the bound, nothing goes.
    put_copy(&data, "state.json", now - Duration::hours(9), &state(9));
    let before = all_copies(&data).len();
    prune_total(&data, TOTAL_MAX);
    assert_eq!(all_copies(&data).len(), before);
}

#[test]
fn a_copy_taken_back_sets_the_file_of_now_aside_and_one_that_does_not_read_touches_nothing() {
    let data = test_dir("backups-restore");
    let now = at("2026-11-16T10:00:00Z");
    std::fs::write(data.join("state.json"), state(1)).unwrap();
    let good = put_copy(&data, "state.json", now - Duration::hours(2), &state(3));
    let bad = put_copy(&data, "state.json", now - Duration::hours(1), b"\0\0\0");
    let good_name = good.file_name().unwrap().to_str().unwrap().to_string();
    let bad_name = bad.file_name().unwrap().to_str().unwrap().to_string();
    // A copy that does not read: refused, the file as it was.
    assert!(matches!(
        restore(&data, "state.json", &bad_name, now),
        Err(RestoreError::Unreadable)
    ));
    assert!(matches!(
        plan(&data, "state.json", &bad_name, now),
        Err(RestoreError::Unreadable)
    ));
    assert_eq!(std::fs::read(data.join("state.json")).unwrap(), state(1));
    // Not a copy of the file (another name, a path out of the folder): refused.
    assert!(matches!(
        restore(&data, "state.json", "../state.json", now),
        Err(RestoreError::Unknown)
    ));
    assert!(matches!(
        restore(&data, "../x.json", &good_name, now),
        Err(RestoreError::Unknown)
    ));
    // The good one: the file of now beside it under a dated name, the copy in its place.
    let aside = restore(&data, "state.json", &good_name, now)
        .unwrap()
        .unwrap();
    assert_eq!(
        aside.file_name().unwrap(),
        "state.before-restore-20261116T100000Z.json"
    );
    assert_eq!(std::fs::read(&aside).unwrap(), state(1));
    assert_eq!(std::fs::read(data.join("state.json")).unwrap(), state(3));
    // Again in the same second: never over the one before.
    let again = restore(&data, "state.json", &good_name, now)
        .unwrap()
        .unwrap();
    assert_eq!(
        again.file_name().unwrap(),
        "state.before-restore-20261116T100000Z-2.json"
    );
    let set = set_aside_files(&data);
    assert_eq!(set.len(), 2);
    assert!(set
        .iter()
        .all(|s| s.file == "state.json" && s.why == BEFORE_RESTORE));
}

#[test]
fn a_copy_chosen_is_put_in_place_at_the_next_start_once_and_told() {
    let data = test_dir("backups-pending");
    let now = at("2026-11-16T10:00:00Z");
    std::fs::write(data.join("settings.json"), b"{}").unwrap();
    let copy = put_copy(
        &data,
        "settings.json",
        now - Duration::days(1),
        br#"{"accounts":[{"id":"principal","name":"Principal"}]}"#,
    );
    let name = copy.file_name().unwrap().to_str().unwrap();
    assert!(apply_pending(&data, now).is_none(), "nothing chosen");
    let p = plan(&data, "settings.json", name, now).unwrap();
    assert_eq!(pending(&data), Some(p.clone()));
    // Called off, then chosen again.
    cancel(&data).unwrap();
    assert_eq!(pending(&data), None);
    plan(&data, "settings.json", name, now).unwrap();
    let r = apply_pending(&data, now + Duration::minutes(1)).unwrap();
    assert!(r.error.is_none());
    assert_eq!(r.copy_at, now - Duration::days(1));
    assert!(r.set_aside.as_ref().unwrap().exists());
    assert!(std::fs::read_to_string(data.join("settings.json"))
        .unwrap()
        .contains("Principal"));
    assert_eq!(pending(&data), None, "once");
    assert_eq!(restored(&data), vec![r]);
    dismiss_restored(&data).unwrap();
    assert!(restored(&data).is_empty());
    // A copy gone meanwhile: told, the file left as it is.
    plan(&data, "settings.json", name, now).unwrap();
    std::fs::remove_file(&copy).unwrap();
    let r = apply_pending(&data, now).unwrap();
    assert!(r.error.is_some());
    assert!(std::fs::read_to_string(data.join("settings.json"))
        .unwrap()
        .contains("Principal"));
}
