//! The fence of private projects on the files an agent reads and writes through the client (`fs/read_text_file`,
//! `fs/write_text_file`): the barrier that holds, so compared by the paths' components, never by their text. A path
//! must be absolute (ACP asks it); its `.` and `..` are taken out without the disk, then the longest part of it that
//! exists is canonicalized (links and junctions followed, without the `\\?\` prefix of Windows) and the rest put
//! back; under Windows, the case is ignored. The file is then opened at that very path (`resolve`), never at the text
//! the agent sent: under Unix the kernel follows a link before it takes a `..` (`link/../secret`), so the text could
//! open another file than the one checked.

use std::path::{Component, Path, PathBuf};

/// `path` without its `.` and `..`, by its text alone; None when a `..` climbs over its root.
fn lexical(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() || out.as_os_str().is_empty() {
                    return None;
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    Some(out)
}

/// `path` (normalized) with its longest part that exists canonicalized and the rest put back on it.
fn canonical(path: &Path) -> PathBuf {
    let mut rest: Vec<std::ffi::OsString> = Vec::new();
    let mut at = path.to_path_buf();
    loop {
        if let Ok(real) = dunce::canonicalize(&at) {
            let mut out = real;
            for part in rest.iter().rev() {
                out.push(part);
            }
            return out;
        }
        match (at.file_name().map(|n| n.to_os_string()), at.parent()) {
            (Some(name), Some(parent)) => {
                rest.push(name);
                at = parent.to_path_buf();
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// The key a path is compared by: the case ignored under Windows.
fn key(path: PathBuf) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(path.to_string_lossy().to_lowercase())
    } else {
        path
    }
}

/// The real path of `path`, the one to open: its `.` and `..` taken out, its longest existing part canonicalized.
/// None when it is not absolute or climbs over its root.
pub fn resolve(path: &str) -> Option<PathBuf> {
    let p = Path::new(path);
    if path.is_empty() || !p.is_absolute() {
        return None;
    }
    Some(canonical(&lexical(p)?))
}

/// The fence as it is compared: each path of it resolved, by its key (or, not absolute, left out).
pub fn fence_of(fence: &[String]) -> Vec<PathBuf> {
    fence.iter().filter_map(|p| resolve(p)).map(key).collect()
}

/// Whether the real path `real` (by `resolve`) is one of `fence`'s (by `fence_of`), or under one.
pub fn within(real: &Path, fence: &[PathBuf]) -> bool {
    let k = key(real.to_path_buf());
    fence.iter().any(|f| k.starts_with(f))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> &'static str {
        if cfg!(windows) {
            "C:\\x"
        } else {
            "/x"
        }
    }

    #[test]
    fn dots_are_taken_out_and_a_relative_path_does_not_hold() {
        let sep = std::path::MAIN_SEPARATOR;
        let r = root();
        let fence = fence_of(&[format!("{r}{sep}private")]);
        let climbed = resolve(&format!("{r}{sep}public{sep}..{sep}private{sep}secret")).unwrap();
        assert!(within(&climbed, &fence));
        assert!(!within(
            &resolve(&format!("{r}{sep}privateer")).unwrap(),
            &fence
        ));
        assert!(resolve("private/secret").is_none());
        assert!(resolve("").is_none());
    }
}
