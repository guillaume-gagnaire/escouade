//! Saves of the state at the same time (ESC-535): at the app's exit for an update, `Core::shutdown` saves while the
//! saver of the board (every 400 ms, when an agent that stopped asked for it) saves too. None may fail, and the file
//! holds the last snapshot taken.

use crate::core::{save_twice, ExitSave};
use crate::core_tests::harness;
use crate::model::PersistedState;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};

/// Writers of the state at once, on threads and on the runtime's blocking tasks, as at the exit.
const WRITERS: usize = 8;
const ROUNDS: usize = 30;

fn view_on_disk(h: &crate::core_tests::Harness) -> String {
    let bytes = std::fs::read(h.core.data.state_file()).unwrap();
    serde_json::from_slice::<PersistedState>(&bytes)
        .unwrap()
        .ui
        .view
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn saves_of_the_state_at_once_all_succeed() {
    let h = harness("save-at-once");
    let failures = Arc::new(AtomicUsize::new(0));
    let first = Arc::new(parking_lot::Mutex::new(None::<String>));
    for _ in 0..ROUNDS {
        let start = Arc::new(Barrier::new(WRITERS));
        let mut threads = Vec::new();
        let mut tasks = Vec::new();
        for i in 0..WRITERS {
            let (core, start, failures, first) = (
                h.core.clone(),
                start.clone(),
                failures.clone(),
                first.clone(),
            );
            let save = move || {
                start.wait();
                if let Err(e) = core.save_state() {
                    first.lock().get_or_insert(format!("{e:#}"));
                    failures.fetch_add(1, Ordering::Relaxed);
                }
            };
            if i % 2 == 0 {
                threads.push(std::thread::spawn(save));
            } else {
                tasks.push(tokio::task::spawn_blocking(save));
            }
        }
        for t in threads {
            t.join().unwrap();
        }
        for t in tasks {
            t.await.unwrap();
        }
    }
    assert_eq!(
        failures.load(Ordering::Relaxed),
        0,
        "saves of the state failed, the first: {:?}",
        first.lock()
    );
    // Nothing left beside it.
    let left: Vec<_> = std::fs::read_dir(h.core.data.root())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".tmp"))
        .collect();
    assert!(left.is_empty(), "{left:?}");
}

/// Each writer changes the state then saves it: once all are done, the file holds the last change, whichever save
/// wrote last.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_state_on_the_disk_is_the_last_one_taken() {
    let h = harness("save-last-snapshot");
    let stale = Arc::new(AtomicUsize::new(0));
    for round in 0..ROUNDS {
        let start = Arc::new(Barrier::new(WRITERS));
        let next = Arc::new(AtomicUsize::new(0));
        let mut threads = Vec::new();
        for _ in 0..WRITERS {
            let (core, start, next) = (h.core.clone(), start.clone(), next.clone());
            threads.push(std::thread::spawn(move || {
                start.wait();
                {
                    let mut ui = core.ui.write();
                    ui.view = format!("{round}-{}", next.fetch_add(1, Ordering::AcqRel));
                }
                let _ = core.save_state();
            }));
        }
        for t in threads {
            t.join().unwrap();
        }
        let last = h.core.ui.read().view.clone();
        if view_on_disk(&h) != last {
            stale.fetch_add(1, Ordering::Relaxed);
        }
    }
    assert_eq!(
        stale.load(Ordering::Relaxed),
        0,
        "rounds whose state on the disk was not the last one taken"
    );
}

/// The last save as the app stops is tried once more when it fails (ESC-535), each failure in the journal.
#[test]
fn the_last_save_is_tried_again_once_when_it_fails() {
    let mut calls = 0;
    assert_eq!(
        save_twice(|| {
            calls += 1;
            Ok(())
        }),
        ExitSave::Saved
    );
    assert_eq!(calls, 1);
    let mut calls = 0;
    assert_eq!(
        save_twice(|| {
            calls += 1;
            if calls == 1 {
                anyhow::bail!("cannot save state: os error 2")
            }
            Ok(())
        }),
        ExitSave::SavedAgain
    );
    assert_eq!(calls, 2);
    let mut calls = 0;
    assert_eq!(
        save_twice(|| {
            calls += 1;
            anyhow::bail!("cannot save state: disk full")
        }),
        ExitSave::Lost
    );
    assert_eq!(calls, 2, "tried twice, no more");
}

/// A state that cannot be written as the app stops: said lost, after a second try; written again once it can.
#[tokio::test]
async fn a_state_that_cannot_be_saved_at_the_exit_is_said_lost() {
    let h = harness("save-at-exit-fails");
    h.core.ui.write().view = "stats".into();
    let place = h.core.data.state_file();
    let _ = std::fs::remove_file(&place);
    std::fs::create_dir_all(place.join("inside")).unwrap();
    assert_eq!(h.core.save_at_exit(), ExitSave::Lost);
    std::fs::remove_dir_all(&place).unwrap();
    assert_eq!(h.core.save_at_exit(), ExitSave::Saved);
    assert_eq!(view_on_disk(&h), "stats");
}
