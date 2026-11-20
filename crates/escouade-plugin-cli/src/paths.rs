//! The folders `escouade-plugin` reads, and how it writes a file; the app's `paths` takes them from here.

use std::path::{Path, PathBuf};

/// The name of the app's data folder in the home.
pub const DATA_DIR_NAME: &str = ".escouade";

/// A sandboxed data folder (end-to-end tests, demos): `$ESCOUADE_DATA_DIR`, or the former
/// `$CCM_DATA_DIR`.
pub fn sandbox_dir() -> Option<PathBuf> {
    sandbox_dir_from(|k| std::env::var_os(k))
}

pub fn sandbox_dir_from(var: impl Fn(&str) -> Option<std::ffi::OsString>) -> Option<PathBuf> {
    ["ESCOUADE_DATA_DIR", "CCM_DATA_DIR"]
        .into_iter()
        .find_map(|k| var(k).filter(|d| !d.is_empty()))
        .map(PathBuf::from)
}

/// The data folder of a test given none of its own (ESC-523): never the user's. Under nextest (`NEXTEST_RUN_ID`, which
/// it gives each test and every program a test starts), `$ESCOUADE_TEST_DATA_DIR`, else [`shared_test_data_dir`]:
/// shared by the tests and kept from one run to the next, so the pinned toolchain and Javy are downloaded once, while
/// each test that compiles gives its own `ESCOUADE_DATA_DIR` for its copies.
pub fn test_data_dir() -> Option<PathBuf> {
    test_data_dir_from(|k| std::env::var_os(k))
}

pub fn test_data_dir_from(var: impl Fn(&str) -> Option<std::ffi::OsString>) -> Option<PathBuf> {
    var("NEXTEST_RUN_ID")?;
    Some(
        var("ESCOUADE_TEST_DATA_DIR")
            .filter(|d| !d.is_empty())
            .map_or_else(shared_test_data_dir, PathBuf::from),
    )
}

/// `escouade-test-data` in the system's cache folder (`%LOCALAPPDATA%`, `~/.cache`, `~/Library/Caches`). Not in the
/// temporary folder: a test process of the app's crates has one of its own, removed after it (`test_tmp.rs`), and the
/// toolchain would be downloaded again by each test.
pub fn shared_test_data_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("escouade-test-data")
}

/// The app's data folder as `escouade-plugin` reads it (the pinned Javy, the modules of the `run.js` plugins), moving
/// nothing: the sandboxed folder, else that of a test, else `~/.escouade`.
pub fn tools_data_dir() -> PathBuf {
    sandbox_dir().or_else(test_data_dir).unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(DATA_DIR_NAME)
    })
}

/// The cache of the modules of the `run.js` plugins, in the data folder `data`.
pub fn plugin_wasm_dir(data: &Path) -> PathBuf {
    data.join("cache").join("plugins-wasm")
}

/// Where the `run.wasm` plugins compile (copies of their folders, cargo's `target`), in the data folder `data`.
pub fn plugin_rust_dir(data: &Path) -> PathBuf {
    data.join("cache").join("plugins-rust")
}

/// The programs Escouade downloads for itself (the pinned Javy of the `run.js` plugins, the toolchain of the `run.wasm`
/// ones), in the data folder `data`.
pub fn tools_dir(data: &Path) -> PathBuf {
    data.join("tools")
}

/// Writes a file so that a crash or a power cut leaves either its former content or the new one, whole (ESC-469):
/// a temporary file beside it, its data forced onto the disk (`sync_all`), then put in its place, the folder forced
/// too (Unix: its own `sync_all`; Windows: a « write-through » replacement, `MOVEFILE_WRITE_THROUGH`). For whatever is
/// not made again: the state, the settings, the plugins' list, the histories, the memory… A cache writes by
/// `write_cache`. Without the forcing, NTFS journals the renaming before the data reaches the disk: after a power cut,
/// the file has its size and holds zeros (2026-11-15).
///
/// The temporary file is the write's own (`temp_of`): two writers of a same file at once, threads or processes, never
/// share one, and neither fails (ESC-535; with one name for all, the second found its file renamed away, « os error
/// 2 »). The last one put in place wins: a writer whose content must follow the order of its changes takes the
/// snapshot and writes it under one lock of its own (the state: `Core::save_state`). The temporary file is removed when
/// the writing fails; one a crash left is removed by the first write in its folder (`sweep_leftovers`).
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    write_through_temp(path, bytes, Durability::Forced, false)
}

/// `write_atomic` for a file that may hold secrets, maybe their only copy: readable by its owner alone (Unix), forced
/// onto the disk as `write_atomic` does.
pub fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    write_through_temp(path, bytes, Durability::Forced, true)
}

/// Writes a cache, which is made again when it is lost or damaged (the precompiled modules, the store's catalogue, the
/// scan of the workflows): a temporary file and a renaming, never a truncated file in its place while the system runs,
/// nothing forced onto the disk (a power cut may leave it damaged: its reader makes it again).
pub fn write_cache(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    write_through_temp(path, bytes, Durability::Light, false)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Durability {
    Forced,
    Light,
}

fn write_through_temp(
    path: &Path,
    bytes: &[u8],
    durability: Durability,
    private: bool,
) -> std::io::Result<()> {
    use std::io::Write;
    sweep_leftovers(path);
    let tmp = temp_of(path);
    let written = (|| {
        let mut options = std::fs::OpenOptions::new();
        // Always a new file, made with its mode: never one another writer fills, nor one a crash left readable by all.
        options.write(true).create_new(true);
        #[cfg(unix)]
        if private {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        #[cfg(not(unix))]
        let _ = private;
        let mut f = match options.open(&tmp) {
            // Its name is this process's and this write's alone: a file there was left by a crashed process whose
            // number this one has now.
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                std::fs::remove_file(&tmp)?;
                options.open(&tmp)?
            }
            opened => opened?,
        };
        f.write_all(bytes)?;
        if durability == Durability::Forced {
            f.sync_all()?;
        }
        // Closed first: Windows does not move a file still open.
        drop(f);
        match durability {
            Durability::Forced => replace_durably(&tmp, path),
            Durability::Light => std::fs::rename(&tmp, path),
        }
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    written
}

/// The temporary file of one write of `path`: `<file>.<process>-<n>.tmp` beside it, `n` counting this process's
/// writes, so that no two writes at once share it (ESC-535).
fn temp_of(path: &Path) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{}-{n}.tmp", std::process::id()));
    path.with_file_name(name)
}

/// The number of the process whose write `name` was the temporary file of (`temp_of`), if it is one.
fn temp_writer(name: &str) -> Option<u32> {
    let (file, tag) = name.strip_suffix(".tmp")?.rsplit_once('.')?;
    let (pid, n) = tag.split_once('-')?;
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    (!file.is_empty() && digits(pid) && digits(n))
        .then(|| pid.parse().ok())
        .flatten()
}

/// How old a temporary file of another process is when it is taken for a crash's leftover: a write lasts milliseconds.
const LEFTOVER_AGE: std::time::Duration = std::time::Duration::from_secs(60);

/// The temporary files a crash or a power cut left beside `path` (`temp_of`, of another process, a minute old at
/// least) removed, once per folder and process: with one name for all, the next write took the place of such a file;
/// with a name per write, nothing else would.
fn sweep_leftovers(path: &Path) {
    static SWEPT: parking_lot::Mutex<std::collections::BTreeSet<PathBuf>> =
        parking_lot::Mutex::new(std::collections::BTreeSet::new());
    let dir = match path.parent() {
        Some(d) if !d.as_os_str().is_empty() => d,
        _ => Path::new("."),
    };
    {
        if !SWEPT.lock().insert(dir.to_path_buf()) {
            return;
        }
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let me = std::process::id();
    for e in entries.flatten() {
        let name = e.file_name();
        let Some(pid) = name.to_str().and_then(temp_writer) else {
            continue;
        };
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age >= LEFTOVER_AGE);
        if pid != me && old && e.file_type().is_ok_and(|t| t.is_file()) {
            match std::fs::remove_file(e.path()) {
                // Its name alone: the journal names no folder of the disk.
                Ok(()) => log::info!("{}: left by a crash, removed", name.to_string_lossy()),
                Err(err) => log::debug!("{}: not removed: {err}", name.to_string_lossy()),
            }
        }
    }
}

/// `from` put in the place of `to`, the change of the folder on the disk when this returns.
#[cfg(unix)]
fn replace_durably(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::rename(from, to)?;
    sync_dir_of(to)
}

/// The folder of `file` forced onto the disk: its entries (a file made, renamed or removed in it). A folder that cannot
/// be opened (no right to read it) is left: the data of its files is on the disk all the same. Windows has no such
/// forcing for a mere user: nothing there (its renamings go by `MOVEFILE_WRITE_THROUGH`).
pub fn sync_dir_of(file: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let dir = match file.parent() {
            Some(d) if !d.as_os_str().is_empty() => d,
            _ => Path::new("."),
        };
        if let Ok(d) = std::fs::File::open(dir) {
            return d.sync_all();
        }
    }
    let _ = file;
    Ok(())
}

/// `from` put in the place of `to`, the change of the folder on the disk when this returns: `MoveFileExW` with
/// `MOVEFILE_WRITE_THROUGH`, which returns once the move is flushed. When it refuses (the file held open by another
/// program without the right to replace it), `std::fs::rename`, which Windows allows in more cases: the data is on the
/// disk already, only the renaming may wait for the system's next flush.
#[cfg(windows)]
fn replace_durably(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let wide = |p: &Path| -> Vec<u16> {
        p.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    };
    let (f, t) = (wide(from), wide(to));
    // SAFETY: two NUL-terminated wide strings, alive for the call.
    let moved = unsafe {
        MoveFileExW(
            f.as_ptr(),
            t.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved != 0 {
        return Ok(());
    }
    let refused = std::io::Error::last_os_error();
    std::fs::rename(from, to).inspect_err(|e| {
        log::debug!(
            "{}: write-through replacement refused ({refused}), then: {e}",
            to.display()
        );
    })
}

#[cfg(not(any(unix, windows)))]
fn replace_durably(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::rename(from, to)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("escouade-paths-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn temporaries(d: &Path) -> Vec<String> {
        std::fs::read_dir(d)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".tmp"))
            .collect()
    }

    #[test]
    fn a_test_has_a_data_folder_of_its_own_and_a_program_run_by_hand_has_none() {
        use std::ffi::OsString;
        let none = |_: &str| None::<OsString>;
        assert_eq!(test_data_dir_from(none), None);
        let nextest = |k: &str| (k == "NEXTEST_RUN_ID").then(|| OsString::from("run"));
        assert_eq!(test_data_dir_from(nextest), Some(shared_test_data_dir()));
        // Out of the temporary folder, which a test process of the app's crates has to itself.
        assert!(
            !shared_test_data_dir().starts_with(std::env::temp_dir())
                || dirs::cache_dir().is_none()
        );
        let named = |k: &str| match k {
            "NEXTEST_RUN_ID" => Some(OsString::from("run")),
            "ESCOUADE_TEST_DATA_DIR" => Some(OsString::from("D:/tests")),
            _ => None,
        };
        assert_eq!(test_data_dir_from(named), Some(PathBuf::from("D:/tests")));
        // This test itself runs under nextest: it does not read the user's folder.
        if std::env::var_os("NEXTEST_RUN_ID").is_some() {
            let home = dirs::home_dir().unwrap().join(DATA_DIR_NAME);
            assert_ne!(tools_data_dir(), home);
        }
    }

    #[test]
    fn a_forced_write_puts_its_content_in_place_and_leaves_no_temporary_file() {
        let d = dir("forced");
        let f = d.join("state.json");
        write_atomic(&f, br#"{"a":1}"#).unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), br#"{"a":1}"#);
        // Replaces what was there.
        write_atomic(&f, br#"{"a":2}"#).unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), br#"{"a":2}"#);
        let names: Vec<_> = std::fs::read_dir(&d)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from("state.json")]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_light_write_and_a_private_one_put_their_content_in_place() {
        let d = dir("light");
        let cache = d.join("catalog.json");
        write_cache(&cache, b"1").unwrap();
        write_cache(&cache, b"2").unwrap();
        assert_eq!(std::fs::read(&cache).unwrap(), b"2");
        let secret = d.join("integrations.json");
        write_private(&secret, b"s").unwrap();
        write_private(&secret, b"t").unwrap();
        assert_eq!(std::fs::read(&secret).unwrap(), b"t");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&secret).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        assert!(temporaries(&d).is_empty());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_write_that_fails_says_so_and_leaves_the_place_and_no_temporary_file() {
        let d = dir("fails");
        // The file's place is a folder that holds a file: nothing can replace it.
        let f = d.join("state.json");
        std::fs::create_dir_all(f.join("inside")).unwrap();
        assert!(write_atomic(&f, b"{}").is_err());
        assert!(f.join("inside").is_dir());
        assert!(temporaries(&d).is_empty(), "the temporary file is removed");
        assert!(write_cache(&f, b"{}").is_err());
        assert!(temporaries(&d).is_empty());
        // A folder that does not exist: the error, nothing written.
        let missing = d.join("nope").join("state.json");
        assert!(write_atomic(&missing, b"{}").is_err());
        assert!(!missing.exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Writers of one file at once (ESC-535): none fails, the file holds one of their contents whole, and nothing is left
    /// beside it. With one temporary file for all, a writer found it renamed away by another (« os error 2 »).
    #[test]
    fn writers_of_a_same_file_at_once_all_succeed() {
        let d = dir("at-once");
        let f = d.join("state.json");
        const WRITERS: usize = 8;
        for round in 0..20 {
            let start = std::sync::Arc::new(std::sync::Barrier::new(WRITERS));
            let writers: Vec<_> = (0..WRITERS)
                .map(|i| {
                    let (f, start) = (f.clone(), start.clone());
                    std::thread::spawn(move || {
                        let bytes = format!("{{\"round\":{round},\"writer\":{i}}}").repeat(4096);
                        start.wait();
                        let write = [write_atomic, write_private, write_cache][i % 3];
                        write(&f, bytes.as_bytes()).map(|()| bytes)
                    })
                })
                .collect();
            let written: Vec<String> = writers
                .into_iter()
                .map(|w| {
                    w.join()
                        .unwrap()
                        .expect("a write at the same time as others")
                })
                .collect();
            let there = std::fs::read_to_string(&f).unwrap();
            assert!(written.contains(&there), "one writer's content, whole");
        }
        assert!(temporaries(&d).is_empty());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_temporary_file_of_a_write_is_its_own() {
        let a = temp_of(Path::new("/d/state.json"));
        let b = temp_of(Path::new("/d/state.json"));
        assert_ne!(a, b);
        let name = a.file_name().unwrap().to_str().unwrap();
        assert!(
            name.starts_with("state.json.") && name.ends_with(".tmp"),
            "{name}"
        );
        assert_eq!(temp_writer(name), Some(std::process::id()));
        for not_one in [
            "state.tmp",
            "state.json.tmp",
            "notes.12.tmp",
            "notes.a-1.tmp",
            "notes.1-.tmp",
            ".1-2.tmp",
            "state.json.1-2",
        ] {
            assert_eq!(temp_writer(not_one), None, "{not_one}");
        }
        assert_eq!(temp_writer("plugins.json.4242-7.tmp"), Some(4242));
    }

    /// What a crash left (another process's temporary file, a minute old) goes at the first write in its folder; a
    /// fresh one (a write under way in another process) and the user's own files stay.
    #[test]
    fn the_first_write_in_a_folder_removes_what_a_crash_left() {
        let d = dir("leftovers");
        let other = std::process::id().wrapping_add(1);
        let old = d.join(format!("state.json.{other}-3.tmp"));
        let fresh = d.join(format!("settings.json.{other}-4.tmp"));
        let theirs = d.join("notes.tmp");
        for f in [&old, &fresh, &theirs] {
            std::fs::write(f, "{").unwrap();
        }
        let before =
            std::time::SystemTime::now() - LEFTOVER_AGE - std::time::Duration::from_secs(5);
        std::fs::File::options()
            .write(true)
            .open(&old)
            .unwrap()
            .set_modified(before)
            .unwrap();
        write_atomic(&d.join("state.json"), b"{}").unwrap();
        assert!(!old.exists(), "left by a crash");
        assert!(fresh.exists() && theirs.exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// What a forced write costs, measured on the machine (ESC-469): `cargo test -p escouade-plugin-cli --release
    /// what_a_forced_write_costs -- --ignored --nocapture`, with `ESCOUADE_BENCH_DIR` set to a folder of the disk to
    /// measure (the temporary folder otherwise).
    #[test]
    #[ignore]
    fn what_a_forced_write_costs() {
        let base =
            std::env::var_os("ESCOUADE_BENCH_DIR").map_or_else(std::env::temp_dir, PathBuf::from);
        let d = base.join(format!("escouade-bench-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("state.json");
        let pattern = b"{}[],:\"abcdefgh 0123456789\n";
        for size in [9usize << 20, 1 << 20, 64 << 10] {
            let bytes: Vec<u8> = (0..size).map(|i| pattern[i % pattern.len()]).collect();
            type Write = fn(&Path, &[u8]) -> std::io::Result<()>;
            let writes: [(&str, Write); 2] = [("light", write_cache), ("forced", write_atomic)];
            for (name, write) in writes {
                let mut times = Vec::new();
                for _ in 0..15 {
                    let t = std::time::Instant::now();
                    write(&f, &bytes).unwrap();
                    times.push(t.elapsed().as_secs_f64() * 1000.0);
                }
                times.sort_by(f64::total_cmp);
                println!(
                    "{name:>6} {:>5} Ko: median {:.1} ms, max {:.1} ms",
                    size >> 10,
                    times[times.len() / 2],
                    times[times.len() - 1]
                );
            }
        }
        let _ = std::fs::remove_dir_all(&d);
    }
}
