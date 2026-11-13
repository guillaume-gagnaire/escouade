//! Whether two paths name the same file (`attachfiles::same_file`), as an agent's granted files are kept.

/// `path` with its separators as `/` and no `\\?\` prefix.
///
/// The separators are changed only in a path of Windows (a drive, or a share): in a path of Linux
/// a backslash is a letter of a name, and `/home/me/a\b.txt` is not `/home/me/a/b.txt`.
fn slashed(path: &str) -> String {
    let path = path.trim_start_matches(r"\\?\");
    let b = path.as_bytes();
    let windows =
        path.starts_with(r"\\") || (b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':');
    if windows {
        path.replace('\\', "/")
    } else {
        path.to_string()
    }
}

/// A path of Windows, as `slashed` wrote it: a drive (`C:/…`), a share (`//server/…`), or a drive
/// seen from WSL (`/mnt/c/…`).
fn windows_like(path: &str) -> bool {
    let b = path.as_bytes();
    let drive = b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':';
    let mounted = path.strip_prefix("/mnt/").is_some_and(|rest| {
        let r = rest.as_bytes();
        r.first().is_some_and(u8::is_ascii_alphabetic) && (r.len() == 1 || r[1] == b'/')
    });
    drive || mounted || path.starts_with("//")
}

/// Both paths are the same file: the separators and a `\\?\` prefix do not count, nor the case for a
/// path of Windows (or one of its drives seen from WSL, `/mnt/c/…`). Nothing is read on the disk
/// and no `..` is resolved: a way round to a file is not that file.
pub fn same_file(a: &str, b: &str) -> bool {
    let (a, b) = (slashed(a), slashed(b));
    if windows_like(&a) || windows_like(&b) {
        a.to_lowercase() == b.to_lowercase()
    } else {
        a == b
    }
}
