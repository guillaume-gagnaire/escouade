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

/// The app's data folder as `escouade-plugin` reads it (the pinned Javy, the modules of the `run.js` plugins), moving
/// nothing: the sandboxed folder, else `~/.escouade`.
pub fn tools_data_dir() -> PathBuf {
    sandbox_dir().unwrap_or_else(|| {
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
/// The temporary file is `<file>.tmp`, one name for every writer: two writers of a same file at once must be
/// serialized (ESC-440). It is removed when the writing fails.
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
    let tmp = path.with_extension("tmp");
    let written = (|| {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        if private {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut f = options.open(&tmp)?;
        #[cfg(unix)]
        if private {
            use std::os::unix::fs::PermissionsExt;
            // The mode only applies to a new file: one left by a crash keeps its own, emptied.
            f.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        #[cfg(not(unix))]
        let _ = private;
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
        assert!(!d.join("catalog.tmp").exists() && !d.join("integrations.tmp").exists());
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
        assert!(
            !d.join("state.tmp").exists(),
            "the temporary file is removed"
        );
        assert!(write_cache(&f, b"{}").is_err());
        assert!(!d.join("state.tmp").exists());
        // A folder that does not exist: the error, nothing written.
        let missing = d.join("nope").join("state.json");
        assert!(write_atomic(&missing, b"{}").is_err());
        assert!(!missing.exists());
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
