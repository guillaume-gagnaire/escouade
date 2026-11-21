//! The long secrets in pieces (ESC-527), on the memory store: cut, glued back, checked, written whole or not at all,
//! forgotten whole.

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const NAME: &str = "plugin:dev.test.vpn:config";

/// A configuration of `n` bytes, of lines like an OpenVPN one's.
fn config(n: usize, seed: char) -> String {
    let line = format!("{seed}{seed}MIIDdzCCAl+gAwIBAgIUP1c2xQ8\n");
    line.repeat(n / line.len() + 1)[..n].to_string()
}

fn chunked() -> (Arc<MemoryStore>, Chunked) {
    let mem = Arc::new(MemoryStore::default());
    (mem.clone(), Chunked::new(mem))
}

#[test]
fn a_short_secret_is_one_entry_as_before_and_one_kept_before_reads_as_it_was() {
    let (mem, s) = chunked();
    s.set(NAME, "hunter2").unwrap();
    assert_eq!(mem.all().len(), 1);
    assert_eq!(mem.entry(NAME).as_deref(), Some("hunter2"));
    // Kept by a version before, at the limit of a piece.
    let before = "x".repeat(PIECE);
    mem.set("plugin:dev.test.vpn:old", &before).unwrap();
    assert_eq!(
        s.get("plugin:dev.test.vpn:old").unwrap(),
        Some(before.clone())
    );
    s.set("plugin:dev.test.vpn:edge", &before).unwrap();
    assert_eq!(mem.entry("plugin:dev.test.vpn:edge"), Some(before));
    assert_eq!(s.get("plugin:dev.test.missing").unwrap(), None);
}

#[test]
fn a_long_secret_is_cut_into_pieces_under_a_header_and_glued_back() {
    let (mem, s) = chunked();
    let secret = config(6 * 1024, 'k');
    s.set(NAME, &secret).unwrap();
    assert_eq!(
        mem.entry(NAME).unwrap(),
        format!("escouade-pieces:v1:a:6:{}", sha256(&secret))
    );
    for i in 1..=6 {
        let piece = mem.entry(&format!("{NAME}#a{i}")).unwrap();
        assert!(piece.encode_utf16().count() <= PIECE);
    }
    assert_eq!(mem.entry(&format!("{NAME}#a7")), None);
    assert_eq!(mem.all().len(), 7);
    assert_eq!(s.get(NAME).unwrap(), Some(secret));
    // Every entry stays under Windows' 1,280 characters.
    assert!(mem.all().values().all(|v| v.encode_utf16().count() <= 1280));
}

#[test]
fn a_piece_is_never_cut_inside_a_character_and_counts_utf16_units() {
    let (mem, s) = chunked();
    // Three UTF-16 units each: 1,200 characters would be 3,600 units.
    let secret = "😀é".repeat(900);
    s.set(NAME, &secret).unwrap();
    assert!(mem
        .all()
        .values()
        .all(|v| v.encode_utf16().count() <= PIECE));
    assert_eq!(s.get(NAME).unwrap(), Some(secret));
}

#[test]
fn a_secret_that_looks_like_a_header_is_kept_in_pieces() {
    let (mem, s) = chunked();
    let sly = format!("{HEADER}a:1:{}", "0".repeat(64));
    s.set(NAME, &sly).unwrap();
    assert_ne!(mem.entry(NAME).as_deref(), Some(sly.as_str()));
    assert_eq!(s.get(NAME).unwrap(), Some(sly));
}

#[test]
fn a_wrong_sha256_a_missing_piece_or_one_too_many_is_an_error_never_a_secret_cut_short() {
    let secret = config(5000, 'w');
    // A wrong SHA-256: a piece changed.
    let (mem, s) = chunked();
    s.set(NAME, &secret).unwrap();
    mem.set(&format!("{NAME}#a2"), "tampered").unwrap();
    let e = s.get(NAME).unwrap_err();
    assert!(format!("{e:#}").contains("SHA-256"), "{e:#}");
    // A piece missing.
    let (mem, s) = chunked();
    s.set(NAME, &secret).unwrap();
    mem.delete(&format!("{NAME}#a3")).unwrap();
    let e = s.get(NAME).unwrap_err();
    assert!(format!("{e:#}").contains("missing"), "{e:#}");
    // The last piece missing: the rest would read as a shorter secret.
    let (mem, s) = chunked();
    s.set(NAME, &secret).unwrap();
    mem.delete(&format!("{NAME}#a5")).unwrap();
    assert!(s.get(NAME).is_err());
    // One too many.
    let (mem, s) = chunked();
    s.set(NAME, &secret).unwrap();
    mem.set(&format!("{NAME}#a6"), "more").unwrap();
    let e = s.get(NAME).unwrap_err();
    assert!(format!("{e:#}").contains("more pieces"), "{e:#}");
    // A header that cannot be read.
    for bad in [
        format!("{HEADER}c:2:{}", "0".repeat(64)),
        format!("{HEADER}a:0:{}", "0".repeat(64)),
        format!("{HEADER}a:65:{}", "0".repeat(64)),
        format!("{HEADER}a:2:{}:more", "0".repeat(64)),
        format!("{HEADER}a:2:abc"),
        format!("{HEADER}a:2"),
    ] {
        let (mem, s) = chunked();
        mem.set(NAME, &bad).unwrap();
        assert!(s.get(NAME).is_err(), "{bad}");
    }
    // A keychain out of reach.
    let (mem, s) = chunked();
    s.set(NAME, &secret).unwrap();
    mem.refuse.store(true, Ordering::SeqCst);
    assert!(s.get(NAME).is_err());
}

#[test]
fn more_than_64_kib_is_refused_before_anything_is_written() {
    let (mem, s) = chunked();
    s.set(NAME, &config(SECRET_MAX, 'm')).unwrap();
    assert_eq!(s.get(NAME).unwrap().unwrap().len(), SECRET_MAX);
    let before = mem.all();
    let e = s.set(NAME, &config(SECRET_MAX + 1, 'n')).unwrap_err();
    assert!(is_too_long(&e));
    assert_eq!(e.to_string(), "a secret holds 64 KiB at most");
    assert_eq!(mem.all(), before);
    assert_eq!(s.get(NAME).unwrap(), Some(config(SECRET_MAX, 'm')));
}

#[test]
fn no_name_of_a_secret_is_taken_for_a_piece() {
    for piece in ["k#a1", "plugin:x:token#b12", "#a3"] {
        assert!(is_piece_name(piece), "{piece}");
    }
    for name in [
        "k",
        "k#",
        "k#a",
        "k#c1",
        "k#a1x",
        "git.acme.dev:8443",
        "k#1",
        "k#aa1",
    ] {
        assert!(!is_piece_name(name), "{name}");
    }
    let (mem, s) = chunked();
    s.set(NAME, &config(3000, 'p')).unwrap();
    let piece = format!("{NAME}#a1");
    assert!(s.get(&piece).is_err());
    assert!(s.set(&piece, "mine").is_err());
    assert!(s.delete(&piece).is_err());
    assert_eq!(s.get(NAME).unwrap(), Some(config(3000, 'p')));
    assert!(mem.entry(&piece).is_some());
}

#[test]
fn long_to_short_to_long_again_reads_each_one_and_leaves_no_piece_behind() {
    let (mem, s) = chunked();
    let long1 = config(4000, 'a');
    let long2 = config(2500, 'b');
    let long3 = config(7000, 'c');
    s.set(NAME, &long1).unwrap();
    assert_eq!(s.get(NAME).unwrap(), Some(long1));
    s.set(NAME, &long2).unwrap();
    assert!(mem.entry(NAME).unwrap().contains(":b:3:"));
    assert_eq!(mem.all().len(), 4, "the old slot's pieces are out");
    assert_eq!(s.get(NAME).unwrap(), Some(long2));
    s.set(NAME, "short").unwrap();
    assert_eq!(mem.all().len(), 1);
    assert_eq!(s.get(NAME).unwrap().as_deref(), Some("short"));
    s.set(NAME, &long3).unwrap();
    assert_eq!(mem.all().len(), 7);
    assert_eq!(s.get(NAME).unwrap(), Some(long3));
}

#[test]
fn forgetting_takes_out_the_header_and_every_piece_of_both_slots() {
    let (mem, s) = chunked();
    mem.set("plugin:other:token", "stays").unwrap();
    s.set(NAME, &config(6000, 'f')).unwrap();
    // Pieces a write cut short left in the other slot.
    mem.set(&format!("{NAME}#b1"), "left").unwrap();
    mem.set(&format!("{NAME}#b2"), "left").unwrap();
    s.delete(NAME).unwrap();
    assert_eq!(
        mem.all().into_keys().collect::<Vec<_>>(),
        ["plugin:other:token"]
    );
    assert_eq!(s.get(NAME).unwrap(), None);
    // A short one, and nothing at all.
    s.set(NAME, "short").unwrap();
    s.delete(NAME).unwrap();
    s.delete(NAME).unwrap();
    assert_eq!(mem.all().len(), 1);
    // A keychain out of reach says so.
    mem.refuse.store(true, Ordering::SeqCst);
    assert!(s.delete(NAME).is_err());
}

/// A keychain whose writes stop after `budget` of them, as a machine cut off mid-write: what was not written is not,
/// and nothing is put right after.
struct Cut {
    mem: MemoryStore,
    budget: AtomicUsize,
    writes: AtomicUsize,
}

impl Cut {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            mem: MemoryStore::default(),
            budget: AtomicUsize::new(usize::MAX),
            writes: AtomicUsize::new(0),
        })
    }

    fn spend(&self) -> Result<()> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        self.budget
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |b| b.checked_sub(1))
            .map(|_| ())
            .map_err(|_| anyhow::anyhow!("cut off"))
    }
}

impl SecretStore for Cut {
    fn get(&self, name: &str) -> Result<Option<String>> {
        self.mem.get(name)
    }
    fn set(&self, name: &str, secret: &str) -> Result<()> {
        self.spend()?;
        self.mem.set(name, secret)
    }
    fn delete(&self, name: &str) -> Result<()> {
        self.spend()?;
        self.mem.delete(name)
    }
}

/// From `old` to `new`, cut after each write in turn: the store reads `old` until the header turns, `new` after, never
/// anything else nor an error; the write says Ok only once `new` reads; the next write, whole, leaves only its own
/// entries, and « Oublier » none.
fn cut_at_each_step(old: &str, new: &str) {
    let whole = {
        let c = Cut::new();
        let s = Chunked::new(c.clone());
        s.set(NAME, old).unwrap();
        c.writes.store(0, Ordering::SeqCst);
        s.set(NAME, new).unwrap();
        c.writes.load(Ordering::SeqCst)
    };
    assert!(whole > 1);
    let mut turned = false;
    for budget in 0..=whole {
        let c = Cut::new();
        let s = Chunked::new(c.clone());
        s.set(NAME, old).unwrap();
        c.budget.store(budget, Ordering::SeqCst);
        let done = s.set(NAME, new);
        c.budget.store(usize::MAX, Ordering::SeqCst);
        let read = s.get(NAME).unwrap().unwrap();
        assert!(read == old || read == new, "cut after {budget} writes");
        if read == new {
            turned = true;
        } else {
            assert!(!turned, "cut after {budget} writes: back to the old secret");
            assert!(done.is_err(), "cut after {budget} writes");
        }
        if done.is_ok() {
            assert_eq!(read, new);
        }
        let next = config(3100, 'z');
        s.set(NAME, &next).unwrap();
        assert_eq!(s.get(NAME).unwrap(), Some(next));
        assert_eq!(
            c.mem.all().len(),
            4,
            "cut after {budget} writes: {:?}",
            c.mem.all().keys()
        );
        s.delete(NAME).unwrap();
        assert!(
            c.mem.all().is_empty(),
            "cut after {budget} writes: {:?}",
            c.mem.all().keys()
        );
    }
    assert!(turned);
}

#[test]
fn a_write_cut_at_each_step_leaves_the_old_secret_readable() {
    let long1 = config(5000, 'o');
    let long2 = config(3700, 'n');
    cut_at_each_step(&long1, &long2);
    cut_at_each_step(&long2, &long1);
    cut_at_each_step("short-old", &long2);
    cut_at_each_step(&long1, "short-new");
}

/// A keychain that refuses to write one name.
struct Flaky {
    mem: Arc<MemoryStore>,
    fail_on: String,
}

impl SecretStore for Flaky {
    fn get(&self, name: &str) -> Result<Option<String>> {
        self.mem.get(name)
    }
    fn set(&self, name: &str, secret: &str) -> Result<()> {
        if name == self.fail_on {
            anyhow::bail!("refused");
        }
        self.mem.set(name, secret)
    }
    fn delete(&self, name: &str) -> Result<()> {
        self.mem.delete(name)
    }
}

#[test]
fn a_piece_the_keychain_refuses_takes_out_what_was_written_and_the_old_secret_stays() {
    let old = config(2600, 'q');
    for fail_on in [format!("{NAME}#b1"), format!("{NAME}#b3"), NAME.to_string()] {
        let (mem, s) = chunked();
        s.set(NAME, &old).unwrap();
        let before = mem.all();
        let flaky = Chunked::new(Arc::new(Flaky {
            mem: mem.clone(),
            fail_on: fail_on.clone(),
        }));
        let e = flaky.set(NAME, &config(4000, 'r')).unwrap_err();
        assert_eq!(format!("{e:#}"), "refused");
        assert_eq!(mem.all(), before, "{fail_on}");
        assert_eq!(s.get(NAME).unwrap(), Some(old.clone()));
    }
    // A short one before, the first piece refused: it stays, alone.
    let (mem, s) = chunked();
    s.set(NAME, "short").unwrap();
    let flaky = Chunked::new(Arc::new(Flaky {
        mem: mem.clone(),
        fail_on: format!("{NAME}#a2"),
    }));
    assert!(flaky.set(NAME, &config(4000, 'r')).is_err());
    assert_eq!(mem.all().len(), 1);
    assert_eq!(s.get(NAME).unwrap().as_deref(), Some("short"));
}

#[test]
fn no_keychain_stays_no_keychain() {
    let (mem, s) = chunked();
    mem.absent.store(true, Ordering::SeqCst);
    assert!(!s.present());
    assert!(is_no_keychain(
        &s.set(NAME, &config(5000, 'x')).unwrap_err()
    ));
    assert!(is_no_keychain(&s.set(NAME, "short").unwrap_err()));
    assert_eq!(s.get(NAME).unwrap(), None);
    s.delete(NAME).unwrap();
}
