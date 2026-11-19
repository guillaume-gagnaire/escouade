//! The cache of the `run.wasm` compilations, kept within bounds (ESC-523, docs/PLUGINS-DESIGN.md §8.17).
//!
//! Each compilation works in a copy of the plugin's folder, with its `target`, under
//! `<data>/cache/plugins-rust/<rustc>-<canon>/<key of the folder's path>/` (a copy per folder and per compiler: a copy
//! reused makes the next compilation of that folder quick). Without bounds, every folder compiled once stays there:
//! 926 copies, 83 GB in eleven days on a machine that ran the tests of the repository in many worktrees.
//!
//! - **A copy** holds `lock` while a compilation works in it ([`crate::rust`] takes it), `used` (its date is that of the
//!   last compilation's start) and `size` (what it weighed at the last compilation's end, in bytes).
//! - **The bounds**: a copy unused for [`DEFAULT_DAYS`] days goes, then the least recently used ones until the cache is
//!   under [`DEFAULT_GB`] GB; `ESCOUADE_RUST_CACHE_DAYS` and `ESCOUADE_RUST_CACHE_GB` change them (decimals allowed, 0
//!   keeps nothing but what is in use). Done at the end of each compilation, and by `escouade-plugin cache clear`.
//! - **Never a copy in use**: a copy is removed under its own lock, taken without waiting; one whose lock is held is
//!   left, as is the copy of the compilation that prunes. It is first renamed to `.trash-<id>` beside the compilers'
//!   folders, then removed: a removal cut short (a crash, a file held open) leaves a `.trash-*` that the next pruning
//!   finishes, never a half-removed copy where a compilation would look for it.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Days a copy may stay unused.
pub const DEFAULT_DAYS: f64 = 7.0;
/// The cache's size, in GB (1024³ bytes), beyond which the least recently used copies go.
pub const DEFAULT_GB: f64 = 5.0;
/// The variable that changes [`DEFAULT_DAYS`].
pub const DAYS_VAR: &str = "ESCOUADE_RUST_CACHE_DAYS";
/// The variable that changes [`DEFAULT_GB`].
pub const GB_VAR: &str = "ESCOUADE_RUST_CACHE_GB";

/// The lock a compilation holds in its copy.
pub const LOCK: &str = "lock";
/// The file whose date says when a copy last served.
pub const USED: &str = "used";
/// What a copy weighed at its last compilation's end, in bytes.
pub const SIZE: &str = "size";
/// The prefix of a copy being removed.
const TRASH: &str = ".trash-";
/// A lock older than this was left by a process that died: a compilation never takes a day.
const DEAD: Duration = Duration::from_secs(24 * 3600);
const GB: f64 = 1024.0 * 1024.0 * 1024.0;

/// How long a copy may stay unused, and how much the cache may weigh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub age: Duration,
    pub bytes: u64,
}

impl Bounds {
    /// The bounds of the environment, else the defaults.
    pub fn from_env() -> Self {
        Self::of(|k| std::env::var(k).ok())
    }

    pub fn of(var: impl Fn(&str) -> Option<String>) -> Self {
        let read = |k: &str, default: f64| {
            var(k)
                .and_then(|v| v.trim().replace(',', ".").parse::<f64>().ok())
                .filter(|v| v.is_finite() && *v >= 0.0)
                .unwrap_or(default)
        };
        Self {
            age: Duration::from_secs_f64(read(DAYS_VAR, DEFAULT_DAYS) * 86_400.0),
            bytes: (read(GB_VAR, DEFAULT_GB) * GB) as u64,
        }
    }

    /// Nothing kept but what is in use: `cache clear`.
    pub const NOTHING: Self = Self {
        age: Duration::ZERO,
        bytes: 0,
    };
}

/// What a pruning removed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Pruned {
    pub copies: usize,
    pub bytes: u64,
}

/// What the cache holds.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Usage {
    pub copies: usize,
    /// Of them, those a compilation holds now.
    pub busy: usize,
    pub bytes: u64,
}

/// A copy of the cache, as a pruning sees it.
struct Entry {
    path: PathBuf,
    used: SystemTime,
    busy: bool,
}

/// The size of what is under `path`, links not followed; what cannot be read counts for nothing.
pub fn size_of(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if !meta.is_dir() {
        return meta.len();
    }
    std::fs::read_dir(path)
        .map(|entries| entries.flatten().map(|e| size_of(&e.path())).sum::<u64>())
        .unwrap_or(0)
}

/// The age of the file `path`, when it has one.
fn age_of(path: &Path, now: SystemTime) -> Option<Duration> {
    let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok()?;
    Some(now.duration_since(modified).unwrap_or_default())
}

/// Says the copy `work` serves now: its date of use is now.
pub fn mark_used(work: &Path) {
    let _ = std::fs::write(work.join(USED), b"");
}

/// Notes what the copy `work` weighs, at the end of a compilation.
pub fn note_size(work: &Path) {
    let bytes = size_of(work);
    let _ = std::fs::write(work.join(SIZE), bytes.to_string());
}

/// What a copy weighs: what its last compilation noted, else measured (a copy of an older version, a damaged note).
fn weight_of(work: &Path) -> u64 {
    std::fs::read_to_string(work.join(SIZE))
        .ok()
        .and_then(|t| t.trim().parse().ok())
        .unwrap_or_else(|| size_of(work))
}

/// The copies under `root`, the cache of the compilations, of every compiler.
fn entries(root: &Path, now: SystemTime) -> Vec<Entry> {
    let mut out = Vec::new();
    let Ok(compilers) = std::fs::read_dir(root) else {
        return out;
    };
    for compiler in compilers.flatten() {
        let name = compiler.file_name();
        if name.to_string_lossy().starts_with(TRASH) || !compiler.path().is_dir() {
            continue;
        }
        let Ok(copies) = std::fs::read_dir(compiler.path()) else {
            continue;
        };
        for copy in copies.flatten() {
            let path = copy.path();
            if !path.is_dir() {
                continue;
            }
            let busy = age_of(&path.join(LOCK), now).is_some_and(|age| age < DEAD);
            let used = [path.join(USED), path.clone()]
                .iter()
                .find_map(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            out.push(Entry { path, used, busy });
        }
    }
    out
}

/// The lock of a copy, taken without waiting; `None` while a compilation holds it. One left by a process that died
/// ([`DEAD`]) is taken.
fn try_lock(work: &Path, now: SystemTime) -> Option<PathBuf> {
    let lock = work.join(LOCK);
    for _ in 0..2 {
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock)
        {
            Ok(_) => return Some(lock),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if !age_of(&lock, now).is_some_and(|age| age >= DEAD) {
                    return None;
                }
                let _ = std::fs::remove_file(&lock);
            }
            Err(_) => return None,
        }
    }
    None
}

/// Removes the copy `work` unless a compilation holds it; whether it went.
fn remove(root: &Path, work: &Path, now: SystemTime) -> bool {
    let Some(lock) = try_lock(work, now) else {
        return false;
    };
    let trash = root.join(format!("{TRASH}{}", uuid::Uuid::new_v4().simple()));
    if std::fs::rename(work, &trash).is_err() {
        // A file of it held open (Windows): left for a later pruning.
        let _ = std::fs::remove_file(&lock);
        return false;
    }
    let _ = std::fs::remove_dir_all(&trash);
    true
}

/// Finishes the removals cut short: the `.trash-*` under `root`.
fn empty_trash(root: &Path) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for e in entries.flatten() {
        if e.file_name().to_string_lossy().starts_with(TRASH) {
            let path = e.path();
            if path.is_dir() {
                let _ = std::fs::remove_dir_all(&path);
            } else {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

/// Keeps the cache `root` within `bounds`: the copies unused for longer than `bounds.age`, then the least recently used
/// ones while it weighs more than `bounds.bytes`. Neither `keep` (the copy of the compilation that prunes) nor a copy
/// whose lock is held goes; they count in the weight all the same.
pub fn prune(root: &Path, bounds: &Bounds, keep: Option<&Path>) -> Pruned {
    prune_at(root, bounds, keep, SystemTime::now())
}

/// [`prune`] as at `now`.
pub fn prune_at(root: &Path, bounds: &Bounds, keep: Option<&Path>, now: SystemTime) -> Pruned {
    empty_trash(root);
    let mut entries = entries(root, now);
    entries.sort_by_key(|e| e.used);
    let mut pruned = Pruned::default();
    let kept = |e: &Entry| e.busy || keep.is_some_and(|k| same(k, &e.path));
    let mut left = Vec::new();
    for e in entries {
        let old = now.duration_since(e.used).unwrap_or_default() > bounds.age;
        if old && !kept(&e) {
            let bytes = weight_of(&e.path);
            if remove(root, &e.path, now) {
                pruned.copies += 1;
                pruned.bytes += bytes;
                continue;
            }
        }
        left.push(e);
    }
    let weights: Vec<u64> = left.iter().map(|e| weight_of(&e.path)).collect();
    let mut total: u64 = weights.iter().sum();
    for (e, bytes) in left.iter().zip(weights) {
        if total <= bounds.bytes {
            break;
        }
        if !kept(e) && remove(root, &e.path, now) {
            pruned.copies += 1;
            pruned.bytes += bytes;
            total = total.saturating_sub(bytes);
        }
    }
    // The folders of the compilers left empty (another rustc, another wrapper).
    if let Ok(compilers) = std::fs::read_dir(root) {
        for c in compilers.flatten() {
            let _ = std::fs::remove_dir(c.path());
        }
    }
    if pruned.copies > 0 {
        log::info!(
            "{}: {} copies removed, {} bytes",
            root.display(),
            pruned.copies,
            pruned.bytes
        );
    }
    pruned
}

/// Whether two paths name the same folder, as far as their words go.
fn same(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf());
    norm(a) == norm(b)
}

/// What the cache `root` holds.
pub fn usage(root: &Path) -> Usage {
    let now = SystemTime::now();
    let entries = entries(root, now);
    Usage {
        copies: entries.len(),
        busy: entries.iter().filter(|e| e.busy).count(),
        bytes: entries.iter().map(|e| weight_of(&e.path)).sum(),
    }
}

/// Removes every copy of the cache `root` that no compilation holds: `escouade-plugin cache clear`.
pub fn clear(root: &Path) -> Pruned {
    prune(root, &Bounds::NOTHING, None)
}

/// Removes the toolchains of another version than `version` from `tools` (`rust-<version>/`), each under its own
/// lock, taken without waiting: a download or a check of it under way keeps it. A download cut short
/// (`.rust-<id>/`) older than a day goes too.
pub fn old_toolchains(tools: &Path, version: &str) -> Vec<PathBuf> {
    let now = SystemTime::now();
    let mut removed = Vec::new();
    let Ok(entries) = std::fs::read_dir(tools) else {
        return removed;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let path = e.path();
        if !path.is_dir() {
            continue;
        }
        if name.starts_with(".rust-") {
            if age_of(&path, now).is_some_and(|age| age >= DEAD) {
                let _ = std::fs::remove_dir_all(&path);
                removed.push(path);
            }
            continue;
        }
        let Some(other) = name.strip_prefix("rust-") else {
            continue;
        };
        if other == version {
            continue;
        }
        let lock = tools.join(format!("rust-{other}.lock"));
        let taken = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock)
            .is_ok();
        if !taken {
            continue;
        }
        if std::fs::remove_dir_all(&path).is_ok() {
            removed.push(path);
        }
        let _ = std::fs::remove_file(&lock);
    }
    if !removed.is_empty() {
        log::info!("toolchains of another version removed: {removed:?}");
    }
    removed
}

/// A size said for a person: `1,5 Go` / `1.5 GB`.
pub fn human(bytes: u64, lang: crate::i18n::Lang) -> String {
    let fr = matches!(lang, crate::i18n::Lang::Fr);
    let units: [&str; 4] = if fr {
        ["o", "Ko", "Mo", "Go"]
    } else {
        ["B", "KB", "MB", "GB"]
    };
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < units.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    let text = if unit == 0 {
        format!("{bytes}")
    } else {
        format!("{value:.1}")
    };
    let text = if fr { text.replace('.', ",") } else { text };
    format!("{text} {}", units[unit])
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: Duration = Duration::from_secs(86_400);

    fn root(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("escouade-rust-cache-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A copy of `bytes` bytes in `root`, last used `ago`.
    fn copy(root: &Path, name: &str, bytes: usize, ago: Duration) -> PathBuf {
        let work = root.join("1.98.0-abc").join(name);
        std::fs::create_dir_all(work.join("target")).unwrap();
        std::fs::write(work.join("target").join("lib.rlib"), vec![0u8; bytes]).unwrap();
        mark_used(&work);
        note_size(&work);
        let when = SystemTime::now() - ago;
        std::fs::File::options()
            .write(true)
            .open(work.join(USED))
            .unwrap()
            .set_modified(when)
            .unwrap();
        work
    }

    fn bounds(days: f64, bytes: u64) -> Bounds {
        Bounds {
            age: Duration::from_secs_f64(days * 86_400.0),
            bytes,
        }
    }

    #[test]
    fn the_bounds_come_from_the_environment_or_the_defaults() {
        let none = Bounds::of(|_| None);
        assert_eq!(none.age, Duration::from_secs(7 * 86_400));
        assert_eq!(none.bytes, 5 * 1024 * 1024 * 1024);
        let set = Bounds::of(|k| {
            Some(match k {
                DAYS_VAR => "0,5".into(),
                _ => "0.25".into(),
            })
        });
        assert_eq!(set.age, Duration::from_secs(43_200));
        assert_eq!(set.bytes, 256 * 1024 * 1024);
        // What does not read is the default, never "no bound".
        let wrong = Bounds::of(|_| Some("-3".into()));
        assert_eq!(wrong, none);
        assert_eq!(Bounds::of(|_| Some("beaucoup".into())), none);
    }

    #[test]
    fn a_copy_unused_for_longer_than_the_bound_goes() {
        let r = root("age");
        let old = copy(&r, "old", 10, 8 * DAY);
        let recent = copy(&r, "recent", 10, DAY);
        let pruned = prune(&r, &bounds(7.0, u64::MAX), None);
        assert_eq!(pruned.copies, 1);
        assert!(!old.exists() && recent.is_dir());
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn over_the_size_the_least_recently_used_go_first() {
        let r = root("size");
        let a = copy(&r, "a", 1000, 3 * DAY);
        let b = copy(&r, "b", 1000, 2 * DAY);
        let c = copy(&r, "c", 1000, DAY);
        // Room for two copies: the oldest goes.
        let two = weight_of(&b) + weight_of(&c);
        let pruned = prune(&r, &bounds(30.0, two), None);
        assert_eq!(pruned.copies, 1);
        assert!(pruned.bytes >= 1000);
        assert!(!a.exists() && b.is_dir() && c.is_dir());
        assert_eq!(usage(&r).copies, 2);
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn a_copy_in_use_or_kept_never_goes() {
        let r = root("busy");
        let busy = copy(&r, "busy", 1000, 30 * DAY);
        std::fs::write(busy.join(LOCK), b"").unwrap();
        let own = copy(&r, "own", 1000, 30 * DAY);
        let other = copy(&r, "other", 1000, 30 * DAY);
        let pruned = prune(&r, &Bounds::NOTHING, Some(&own));
        assert_eq!(pruned.copies, 1);
        assert!(busy.is_dir() && own.is_dir() && !other.exists());
        assert_eq!(usage(&r).busy, 1);
        // `clear` keeps the one in use too, and takes the other once its lock is gone.
        assert_eq!(clear(&r).copies, 1);
        assert!(busy.is_dir());
        std::fs::remove_file(busy.join(LOCK)).unwrap();
        assert_eq!(clear(&r).copies, 1);
        assert_eq!(usage(&r), Usage::default());
        // The compiler's folder, left empty, goes too.
        assert_eq!(std::fs::read_dir(&r).unwrap().count(), 0);
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn a_lock_left_by_a_dead_process_does_not_keep_a_copy_forever() {
        let r = root("dead");
        let work = copy(&r, "dead", 10, 30 * DAY);
        let lock = work.join(LOCK);
        std::fs::write(&lock, b"").unwrap();
        std::fs::File::options()
            .write(true)
            .open(&lock)
            .unwrap()
            .set_modified(SystemTime::now() - 2 * DAY)
            .unwrap();
        assert_eq!(prune(&r, &bounds(7.0, u64::MAX), None).copies, 1);
        assert!(!work.exists());
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn a_damaged_cache_is_pruned_without_failing() {
        let r = root("damaged");
        // A copy of a former version: no `used`, no `size` (its folder's date, its weight measured).
        let bare = r.join("1.97.0-old").join("bare");
        std::fs::create_dir_all(bare.join("target")).unwrap();
        std::fs::write(bare.join("target").join("x"), vec![0u8; 500]).unwrap();
        // A size that does not read, a stray file among the copies and among the compilers, a removal cut short.
        let garbled = copy(&r, "garbled", 500, DAY);
        std::fs::write(garbled.join(SIZE), "pas un nombre").unwrap();
        std::fs::write(r.join("1.98.0-abc").join("stray"), b"x").unwrap();
        std::fs::write(r.join("stray"), b"x").unwrap();
        let trash = r.join(".trash-cut");
        std::fs::create_dir_all(trash.join("target")).unwrap();
        std::fs::write(trash.join("target").join("y"), b"y").unwrap();
        let u = usage(&r);
        assert_eq!(u.copies, 2);
        assert!(u.bytes >= 1000, "{u:?}");
        let pruned = prune(&r, &bounds(30.0, 0), None);
        assert_eq!(pruned.copies, 2);
        assert!(!trash.exists() && !bare.exists() && !garbled.exists());
        // A cache that is not there, or is a file, is nothing to prune.
        assert_eq!(
            prune(&r.join("nope"), &Bounds::NOTHING, None),
            Pruned::default()
        );
        assert_eq!(
            prune(&r.join("stray"), &Bounds::NOTHING, None),
            Pruned::default()
        );
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn two_prunings_at_once_remove_each_copy_once() {
        let r = root("two");
        for i in 0..20 {
            copy(&r, &format!("c{i}"), 100, 30 * DAY);
        }
        let (a, b) = std::thread::scope(|s| {
            let one = s.spawn(|| prune(&r, &bounds(7.0, u64::MAX), None));
            let two = s.spawn(|| prune(&r, &bounds(7.0, u64::MAX), None));
            (one.join().unwrap(), two.join().unwrap())
        });
        assert_eq!(a.copies + b.copies, 20);
        assert_eq!(usage(&r).copies, 0);
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn a_toolchain_of_another_version_goes_and_this_one_stays() {
        let tools = root("tools");
        for name in ["rust-1.97.0", "rust-1.98.0", "javy-9.1.0", ".rust-cut"] {
            std::fs::create_dir_all(tools.join(name).join("bin")).unwrap();
        }
        // A download of another version under way keeps its toolchain.
        std::fs::create_dir_all(tools.join("rust-1.96.0")).unwrap();
        std::fs::write(tools.join("rust-1.96.0.lock"), b"").unwrap();
        let removed = old_toolchains(&tools, "1.98.0");
        assert_eq!(removed, vec![tools.join("rust-1.97.0")]);
        for kept in ["rust-1.98.0", "javy-9.1.0", ".rust-cut", "rust-1.96.0"] {
            assert!(tools.join(kept).is_dir(), "{kept}");
        }
        assert!(!tools.join("rust-1.97.0.lock").exists());
        let _ = std::fs::remove_dir_all(&tools);
    }

    #[test]
    fn a_size_is_said_in_each_language() {
        use crate::i18n::Lang;
        assert_eq!(human(512, Lang::Fr), "512 o");
        assert_eq!(human(1536 * 1024 * 1024, Lang::Fr), "1,5 Go");
        assert_eq!(human(1536 * 1024 * 1024, Lang::En), "1.5 GB");
    }
}
