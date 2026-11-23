//! The data folder and the paths, in `escouade-model` (docs/CRATES.md), re-exported here; the folders and links of the
//! app's tests stay.

pub use escouade_model::paths::*;
#[cfg(test)]
use std::path::{Path, PathBuf};

/// The bytes of a regular file of at most `max` bytes, the one way a file of a project tree is
/// read inline on an async worker (fsedit, git's untracked files). A FIFO, socket or device
/// planted there (by an agent) would block that worker at open: opened non-blocking, its type
/// checked on the handle itself (no swap after a check), and its read capped whatever it grew to.
pub fn read_regular(path: &std::path::Path, max: u64) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut opts = std::fs::OpenOptions::new();
    opts.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.custom_flags(libc::O_NONBLOCK);
    }
    let file = opts.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(std::io::Error::other("not a regular file"));
    }
    let mut bytes = Vec::new();
    file.take(max.saturating_add(1)).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max {
        return Err(std::io::Error::other("file too large"));
    }
    Ok(bytes)
}

/// A regular file opened to be read, never through a link at its last part nor waited on: what
/// `O_NOFOLLOW | O_NONBLOCK` do under Unix, `FILE_FLAG_OPEN_REPARSE_POINT` does under Windows,
/// where `O_NOFOLLOW` does not exist (the link itself is opened, a symbolic link or a junction,
/// and its kind on the handle refuses it). For a file a repository decides, checked before by
/// `symlink_metadata` and that may have been swapped since.
pub fn open_no_follow(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    let mut opts = std::fs::OpenOptions::new();
    opts.read(true);
    regular_no_follow(opts, path)
}

/// `path` opened to be written whole, made when it is not there, as `open_no_follow` opens one to
/// be read: a link put there (a symbolic link, a junction under Windows) or a pipe is refused
/// before a byte is written. It is emptied only once known to be a regular file, never at open.
pub fn create_no_follow(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true);
    let file = regular_no_follow(opts, path)?;
    file.set_len(0)?;
    Ok(file)
}

fn regular_no_follow(
    mut opts: std::fs::OpenOptions,
    path: &std::path::Path,
) -> std::io::Result<std::fs::File> {
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::custom_flags(&mut opts, libc::O_NOFOLLOW | libc::O_NONBLOCK);
    #[cfg(windows)]
    std::os::windows::fs::OpenOptionsExt::custom_flags(
        &mut opts,
        windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT,
    );
    let file = opts.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "not a regular file",
        ));
    }
    Ok(file)
}

/// Cargo puts in the `PATH` of a test every folder a build script gives the linker, and that of
/// whisper-rs-sys gives each folder of whisper.cpp's cmake tree (46 of them): the `PATH` then
/// passes the 8191 characters cmd.exe reads, and the fake `claude.cmd` no longer finds `node`.
/// Those folders hold no program: they go, once, before a test starts anything.
#[cfg(all(test, windows))]
pub(crate) fn trim_test_path() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let Some(path) = std::env::var_os("PATH") else {
            return;
        };
        let kept: Vec<PathBuf> = std::env::split_paths(&path)
            .filter(|p| !p.to_string_lossy().contains("whisper-rs-sys-"))
            .collect();
        if let Ok(joined) = std::env::join_paths(kept) {
            // Edition 2021: safe; done once, before the test spawns anything.
            std::env::set_var("PATH", joined);
        }
    });
}

#[cfg(test)]
pub fn test_dir(name: &str) -> PathBuf {
    #[cfg(windows)]
    trim_test_path();
    let dir = std::env::temp_dir().join(format!("ccm-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // The real path (macOS: /private/var, not /var), as the processes started in it report it.
    if cfg!(windows) {
        dir
    } else {
        dir.canonicalize().unwrap()
    }
}

#[cfg(test)]
pub fn make_dir_link(target: &Path, link: &Path) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
    #[cfg(windows)]
    {
        // Try symlink first (requires admin/developer mode)
        if std::os::windows::fs::symlink_dir(target, link).is_ok() {
            return true;
        }
        // Fall back to junction (no special privileges needed)
        std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link.to_string_lossy().replace('/', "\\"))
            .arg(target.to_string_lossy().replace('/', "\\"))
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
pub fn make_file_link(target: &Path, link: &Path) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(target, link).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ESC-653: `open_no_follow` opens a plain file and refuses a link at its last part (under
    /// Windows a junction, which needs no privilege, or a symbolic link where one is allowed),
    /// and under Unix a FIFO without waiting for a writer.
    #[test]
    fn open_no_follow_refuses_a_link_and_never_waits() {
        let root = test_dir("paths-no-follow");
        let plain = root.join("plain.txt");
        std::fs::write(&plain, "x").unwrap();
        assert!(open_no_follow(&plain).is_ok());
        let dir = root.join("dir");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(open_no_follow(&dir).is_err());
        let linked = root.join("linked");
        assert!(make_dir_link(&dir, &linked));
        assert!(open_no_follow(&linked).is_err());
        let file_link = root.join("file-link.txt");
        if make_file_link(&plain, &file_link) {
            assert!(open_no_follow(&file_link).is_err());
        } else {
            eprintln!("no symbolic link to a file allowed here: that part skipped");
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            let fifo = root.join("p");
            let c = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
            assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
            assert!(open_no_follow(&fifo).is_err());
            // No reader: opened blocking to be written, this would hang here.
            assert!(create_no_follow(&fifo).is_err());
        }
        // Written: made when missing, emptied once known to be a plain file, never through a link.
        let made = root.join("made.txt");
        std::io::Write::write_all(&mut create_no_follow(&made).unwrap(), b"long").unwrap();
        std::io::Write::write_all(&mut create_no_follow(&made).unwrap(), b"ab").unwrap();
        assert_eq!(std::fs::read_to_string(&made).unwrap(), "ab");
        assert!(create_no_follow(&linked).is_err());
        assert!(create_no_follow(&dir).is_err());
        #[cfg(windows)]
        let _ = std::fs::remove_dir(&linked);
    }
}
