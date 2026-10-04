//! The embedded editor's access to files: a source's file list, reading and writing a file, and
//! the version a file is compared with.

use crate::git;
use crate::paths::contained;
use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use std::path::{Component, Path};
use std::sync::atomic::{AtomicU64, Ordering};

/// Validate that `rel` is a normal file path component (not `.`, `sub/.`, or empty).
fn validate_rel(rel: &str) -> Result<()> {
    if rel.is_empty() {
        bail!("chemin invalide : ");
    }
    // The last component must be normal (not current dir, parent, etc).
    // Rust drops trailing `.` from components(), so also check the suffix.
    let path = Path::new(rel);
    match path.components().next_back() {
        Some(Component::Normal(_)) => {}
        _ => bail!("chemin invalide : {rel}"),
    }
    // Additional check: refuse if ends with `.` preceded by separator.
    if rel.ends_with("/.") || rel.ends_with("\\.") || rel == "." {
        bail!("chemin invalide : {rel}");
    }
    Ok(())
}

/// Create a temporary file with a unique name. Only succeeds if the file doesn't exist.
/// This prevents following symlinks/junctions at that name.
fn create_temp(tmp: &Path) -> std::io::Result<std::fs::File> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(tmp)
}

/// Larger files are not opened in the editor.
pub const MAX_EDIT_BYTES: u64 = 2 * 1024 * 1024;
/// Errors of `write` when the file is no longer what the editor read.
pub const CHANGED: &str = "changed";
pub const DELETED: &str = "deleted";

// Counter for unique temporary file names to prevent following symlinks.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// What the editor gets of a file: its text with LF line endings, and how to write it back.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileText {
    /// "text", "binary" or "tooLarge" (no text for the last two).
    pub kind: &'static str,
    pub text: Option<String>,
    pub size: u64,
    /// Of the bytes on disk: tells whether the file changed since it was read.
    pub hash: String,
    /// "crlf" when most lines end so, else "lf".
    pub eol: &'static str,
    pub bom: bool,
}

/// FNV-1a of the bytes, with their length.
pub fn hash(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}-{}", bytes.len())
}

pub fn decode(bytes: &[u8]) -> FileText {
    let (bom, body) = match bytes.strip_prefix(b"\xEF\xBB\xBF") {
        Some(rest) => (true, rest),
        None => (false, bytes),
    };
    let (size, h) = (bytes.len() as u64, hash(bytes));
    let binary = body[..body.len().min(8192)].contains(&0);
    let text = if binary {
        None
    } else {
        std::str::from_utf8(body).ok()
    };
    let Some(text) = text else {
        return FileText {
            kind: "binary",
            text: None,
            size,
            hash: h,
            eol: "lf",
            bom,
        };
    };
    let crlf = text.matches("\r\n").count();
    let lf = text.matches('\n').count() - crlf;
    FileText {
        kind: "text",
        text: Some(text.replace("\r\n", "\n")),
        size,
        hash: h,
        eol: if crlf > lf { "crlf" } else { "lf" },
        bom,
    }
}

pub fn encode(text: &str, eol: &str, bom: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len() + 3);
    if bom {
        out.extend_from_slice(b"\xEF\xBB\xBF");
    }
    if eol == "crlf" {
        out.extend_from_slice(text.replace("\r\n", "\n").replace('\n', "\r\n").as_bytes());
    } else {
        out.extend_from_slice(text.as_bytes());
    }
    out
}

pub fn read(root: &Path, rel: &str) -> Result<FileText> {
    validate_rel(rel)?;
    let path = contained(root, rel)?;
    let meta = match std::fs::metadata(&path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => bail!("{rel} introuvable"),
        Err(e) => return Err(e.into()),
    };
    if meta.is_dir() {
        bail!("{rel} est un dossier");
    }
    if meta.len() > MAX_EDIT_BYTES {
        return Ok(FileText {
            kind: "tooLarge",
            text: None,
            size: meta.len(),
            hash: String::new(),
            eol: "lf",
            bom: false,
        });
    }
    Ok(decode(&std::fs::read(&path)?))
}

/// Writes `text` back with the file's line endings and BOM, through a temporary file renamed
/// over it. With `expected`, refused (`changed` / `deleted`) when the file is no longer the one
/// read; without, written anyway (created if need be, parent folders included).
pub fn write(
    root: &Path,
    rel: &str,
    text: &str,
    eol: &str,
    bom: bool,
    expected: Option<&str>,
) -> Result<String> {
    validate_rel(rel)?;
    let mut path = contained(root, rel)?;

    // Resolve symlinks on all platforms, ensuring target is within root.
    if std::fs::symlink_metadata(&path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        let real = std::fs::canonicalize(&path)?;
        let real_root = std::fs::canonicalize(root)?;
        if !real.starts_with(&real_root) {
            bail!("chemin hors du dossier : {rel}");
        }
        path = real;
    }

    if let Some(exp) = expected {
        match std::fs::read(&path) {
            Ok(bytes) if hash(&bytes) != exp => bail!(CHANGED),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => bail!(DELETED),
            Err(e) => return Err(e.into()),
        }
    }

    // Refuse to write to read-only files.
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.permissions().readonly() {
            bail!("{rel} est en lecture seule");
        }
    }

    let bytes = encode(text, eol, bom);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let name = path
        .file_name()
        .ok_or_else(|| anyhow!("chemin invalide : {rel}"))?
        .to_string_lossy()
        .into_owned();

    // Create unique temp file name with PID and counter to avoid symlink attacks.
    let pid = std::process::id();
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp = path.with_file_name(format!(".{name}.{pid}-{counter}.escouade-tmp"));

    write_via(&path, &tmp, &bytes)?;
    Ok(hash(&bytes))
}

/// Creates the empty file `rel`, its missing folders with it. Refused when something is already
/// there: a file is never created over another.
pub fn create(root: &Path, rel: &str) -> Result<()> {
    validate_rel(rel)?;
    #[cfg(windows)]
    for part in rel.split(['/', '\\']) {
        windows_name(part).map_err(|_| anyhow!("nom invalide : {rel}"))?;
    }
    let path = contained(root, rel)?;
    if std::fs::symlink_metadata(&path).is_ok() {
        bail!("{rel} existe déjà");
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| anyhow!("{rel} : {e}"))?;
    }
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => bail!("{rel} existe déjà"),
        Err(e) => Err(anyhow!("{rel} : {e}")),
    }
}

/// A name Windows keeps as typed and gives a file: not ending with a dot or a space (it would drop
/// them), without the characters it refuses, and not a device (`CON`, `NUL.txt`, `COM1`…).
#[cfg(windows)]
fn windows_name(name: &str) -> Result<()> {
    if name.ends_with(['.', ' ']) || name.chars().any(|c| c < ' ' || "<>:\"|?*".contains(c)) {
        bail!("nom invalide");
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_ascii_uppercase();
    let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit()
            && stem.as_bytes()[3] != b'0');
    if device {
        bail!("nom invalide");
    }
    Ok(())
}

/// The one way bytes reach a file: written to the temporary path `tmp` (created only if nothing
/// is there, so a planted link is never followed), then renamed over `path`. The temporary file
/// is removed only when this call created it and the write or the rename failed. `write()` calls
/// it with a unique name; the planted-link test calls it with a name it chose.
fn write_via(path: &Path, tmp: &Path, bytes: &[u8]) -> Result<()> {
    // Create temp file; only clean up if we successfully created it.
    let mut temp_file = create_temp(tmp)?;
    use std::io::Write;
    if let Err(e) = temp_file.write_all(bytes) {
        let _ = std::fs::remove_file(tmp);
        return Err(e.into());
    }
    drop(temp_file);

    #[cfg(unix)]
    if let Ok(meta) = std::fs::metadata(path) {
        let _ = std::fs::set_permissions(tmp, meta.permissions());
    }

    if let Err(e) = std::fs::rename(tmp, path) {
        let _ = std::fs::remove_file(tmp);
        return Err(e.into());
    }
    Ok(())
}

/// More files than that and the tree is cut.
pub const MAX_TREE_FILES: usize = 50_000;

/// The files of a source, relative to `root` with forward slashes.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tree {
    pub root: String,
    pub files: Vec<String>,
    pub truncated: bool,
}

/// What git lists and is still on disk (tracked and untracked, ignored files and the agents'
/// worktrees left out), or the folder walked when it is not a repository or git cannot list it.
pub async fn tree(root: &str) -> Tree {
    tree_up_to(root, MAX_TREE_FILES).await
}

/// `tree`, cut at `max` files (a smaller `max` lets the cut be tested).
async fn tree_up_to(root: &str, max: usize) -> Tree {
    let listed = match git::toplevel(root).await {
        Some(_) => git_files(root).await.ok(),
        None => None,
    };
    // Not a repository, or git cannot list it: the folder is walked.
    let mut files = match listed {
        Some(f) => f,
        None => walk(root, max).await,
    };
    let truncated = files.len() > max;
    files.truncate(max);
    Tree {
        root: root.to_string(),
        files,
        truncated,
    }
}

/// The files git lists that are still on disk: tracked files deleted from the working tree
/// (not committed yet) are left out.
async fn git_files(root: &str) -> Result<Vec<String>> {
    let mut files = git::list_files(root).await?;
    let gone = git::run(root, &["ls-files", "-z", "--deleted"]).await?;
    let gone: std::collections::HashSet<String> = String::from_utf8_lossy(&gone)
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    files.retain(|f| !gone.contains(f));
    Ok(files)
}

/// The folder walked, sorted, off the async workers (it can be 50 000 files).
async fn walk(root: &str, max: usize) -> Vec<String> {
    let root = root.to_string();
    tokio::task::spawn_blocking(move || {
        let mut f = git::walk_files(&root, max + 1);
        f.sort();
        f
    })
    .await
    .unwrap_or_default()
}

/// The version a file is compared with, and what it is called.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Base {
    /// "HEAD", or the branch a worktree left.
    pub reference: String,
    /// None when the file is not there (new), is not a file or is not text.
    pub text: Option<String>,
}

/// HEAD's version of `rel`; for a worktree (`base_branch` given), the version where its branch
/// left that base, so that the agent's commits count as changes. None outside a repository.
pub async fn base(root: &str, base_branch: Option<&str>, rel: &str) -> Result<Option<Base>> {
    validate_rel(rel)?;
    contained(Path::new(root), rel)?;
    if git::toplevel(root).await.is_none() {
        return Ok(None);
    }
    let head = || ("HEAD".to_string(), "HEAD".to_string());
    let (rev, reference) = match base_branch {
        Some(b) => match git::text(root, &["merge-base", "HEAD", b]).await {
            Ok(sha) if !sha.is_empty() => (sha, b.to_string()),
            _ => head(),
        },
        None => head(),
    };
    let spec = format!("{rev}:./{}", rel.replace('\\', "/"));
    let text = match git::run(root, &["cat-file", "blob", &spec]).await {
        Ok(bytes) => decode(&bytes).text,
        Err(_) => None,
    };
    Ok(Some(Base { reference, text }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::test_dir;

    #[test]
    fn decodes_text_line_endings_bom_and_binary() {
        let t = decode(b"\xEF\xBB\xBFa\r\nb\r\nc\n");
        assert_eq!(
            (t.kind, t.text.as_deref(), t.eol, t.bom),
            ("text", Some("a\nb\nc\n"), "crlf", true)
        );
        let t = decode(b"a\nb\n");
        assert_eq!((t.kind, t.eol, t.bom), ("text", "lf", false));
        assert_eq!(decode(b"PNG\0\x01").kind, "binary");
        assert_eq!(decode(&[0xff, 0xfe, 0x41]).kind, "binary");
        assert_eq!(decode(b"").text.as_deref(), Some(""));
        assert_eq!(decode(b"x\r\n").hash, hash(b"x\r\n"));
    }

    #[test]
    fn encodes_back_with_the_line_endings_and_bom_of_the_file() {
        assert_eq!(
            encode("a\nb\n", "crlf", true),
            b"\xEF\xBB\xBFa\r\nb\r\n".to_vec()
        );
        assert_eq!(encode("a\nb", "lf", false), b"a\nb".to_vec());
        // A stray CRLF typed or pasted in does not become CRCRLF.
        assert_eq!(encode("a\r\nb\n", "crlf", false), b"a\r\nb\r\n".to_vec());
    }

    #[test]
    fn reads_a_file_of_its_root_and_leaves_a_big_one_out() {
        let dir = test_dir("fsedit-read");
        std::fs::write(dir.join("a.ts"), "x\r\n").unwrap();
        let t = read(&dir, "a.ts").unwrap();
        assert_eq!((t.text.as_deref(), t.eol), (Some("x\n"), "crlf"));
        assert_eq!(t.hash, hash(b"x\r\n"));
        std::fs::write(
            dir.join("big.txt"),
            vec![b'a'; (MAX_EDIT_BYTES + 1) as usize],
        )
        .unwrap();
        let t = read(&dir, "big.txt").unwrap();
        assert_eq!((t.kind, t.text), ("tooLarge", None));
        assert_eq!(t.size, MAX_EDIT_BYTES + 1);
        let missing = read(&dir, "missing.ts").unwrap_err().to_string();
        assert!(missing.contains("introuvable"), "{missing}");
        assert!(read(&dir, "../x").is_err());
    }

    #[test]
    fn writes_atomically_keeping_crlf_and_refuses_a_file_changed_on_disk() {
        let dir = test_dir("fsedit-write");
        let p = dir.join("a.ts");
        std::fs::write(&p, "a\r\n").unwrap();
        let first = read(&dir, "a.ts").unwrap();
        let h = write(&dir, "a.ts", "a\nb\n", "crlf", false, Some(&first.hash)).unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"a\r\nb\r\n");
        assert_eq!(h, hash(b"a\r\nb\r\n"));
        // Changed behind the editor's back: refused, unless forced.
        std::fs::write(&p, "agent\n").unwrap();
        let err = write(&dir, "a.ts", "mine\n", "lf", false, Some(&h)).unwrap_err();
        assert_eq!(err.to_string(), CHANGED);
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "agent\n");
        write(&dir, "a.ts", "mine\n", "lf", false, None).unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "mine\n");
        // Deleted behind its back.
        let h = hash(b"mine\n");
        std::fs::remove_file(&p).unwrap();
        let err = write(&dir, "a.ts", "x\n", "lf", false, Some(&h)).unwrap_err();
        assert_eq!(err.to_string(), DELETED);
        // Forced: created again, parent folders included, no temporary file left behind.
        write(&dir, "new/deep/b.ts", "b\n", "lf", false, None).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("new/deep/b.ts")).unwrap(),
            "b\n"
        );
        let left: Vec<_> = std::fs::read_dir(dir.join("new/deep"))
            .unwrap()
            .flatten()
            .map(|e| e.file_name())
            .collect();
        assert_eq!(left, vec![std::ffi::OsString::from("b.ts")]);
        assert!(write(&dir, "../escape.ts", "x", "lf", false, None).is_err());
    }

    #[test]
    fn creates_an_empty_file_with_its_folders_but_never_over_one_that_is_there() {
        let dir = test_dir("fsedit-create");
        create(&dir, "a.ts").unwrap();
        assert_eq!(std::fs::read(dir.join("a.ts")).unwrap(), b"");
        create(&dir, "new/deep/b.ts").unwrap();
        assert!(dir.join("new/deep/b.ts").is_file());
        // There already, in any case on a disk that ignores it: refused, left as it was.
        std::fs::write(dir.join("c.ts"), "keep\n").unwrap();
        let err = create(&dir, "c.ts").unwrap_err().to_string();
        assert_eq!(err, "c.ts existe déjà");
        assert_eq!(std::fs::read_to_string(dir.join("c.ts")).unwrap(), "keep\n");
        assert!(create(&dir, "new")
            .unwrap_err()
            .to_string()
            .contains("existe déjà"));
        #[cfg(any(windows, target_os = "macos"))]
        {
            assert_eq!(
                create(&dir, "C.TS").unwrap_err().to_string(),
                "C.TS existe déjà"
            );
            assert_eq!(std::fs::read_to_string(dir.join("c.ts")).unwrap(), "keep\n");
        }
        // A file where a folder is asked for.
        assert!(create(&dir, "c.ts/d.ts").is_err());
        assert!(create(&dir, "../escape.ts").is_err());
        assert!(!dir.parent().unwrap().join("escape.ts").exists());
        for rel in ["", ".", "sub/."] {
            assert!(create(&dir, rel).is_err(), "{rel:?}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn creates_no_file_under_a_name_windows_would_change_or_keeps_for_a_device() {
        let dir = test_dir("fsedit-create-win");
        for rel in [
            "a.",
            "b ",
            "sub./c.ts",
            "d?.ts",
            "e:f",
            "CON",
            "nul.txt",
            "x/com1",
        ] {
            let err = create(&dir, rel).unwrap_err().to_string();
            assert!(err.contains("nom invalide"), "{rel:?}: {err}");
        }
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn keeps_the_permissions_of_the_file() {
        use std::os::unix::fs::PermissionsExt;
        let dir = test_dir("fsedit-perms");
        let p = dir.join("run.sh");
        std::fs::write(&p, "echo a\n").unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        write(&dir, "run.sh", "echo b\n", "lf", false, None).unwrap();
        assert_eq!(
            std::fs::metadata(&p).unwrap().permissions().mode() & 0o777,
            0o755
        );
    }

    #[test]
    fn refuses_rel_ending_with_dot() {
        let base = test_dir("fsedit-dot");
        let dir = base.join("root");
        std::fs::create_dir_all(&dir).unwrap();
        let before: std::collections::HashSet<_> = std::fs::read_dir(&base)
            .unwrap()
            .flatten()
            .map(|e| e.file_name())
            .collect();

        for rel in [".", "./", ".//", "././", "sub/."] {
            let err = write(&dir, rel, "x", "lf", false, None).unwrap_err();
            assert!(err.to_string().contains("chemin invalide"), "{rel}: {err}");
            let err = read(&dir, rel).unwrap_err();
            assert!(err.to_string().contains("chemin invalide"), "{rel}: {err}");
        }

        // Verify nothing was created in base - same set of entries
        let after: std::collections::HashSet<_> = std::fs::read_dir(&base)
            .unwrap()
            .flatten()
            .map(|e| e.file_name())
            .collect();
        assert_eq!(
            before, after,
            "nothing should be created in base, before {:?}, after {:?}",
            before, after
        );
    }

    #[test]
    fn refuses_read_only_file() {
        let dir = test_dir("fsedit-readonly");
        let p = dir.join("readonly.txt");
        std::fs::write(&p, "original\n").unwrap();
        let original_perms = std::fs::metadata(&p).unwrap().permissions();
        let mut perms = original_perms.clone();
        perms.set_readonly(true);
        std::fs::set_permissions(&p, perms).unwrap();
        let err = write(&dir, "readonly.txt", "new\n", "lf", false, None).unwrap_err();
        assert_eq!(err.to_string(), "readonly.txt est en lecture seule");
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "original\n");
        // Restore original permissions for test dir cleanup
        std::fs::set_permissions(&p, original_perms).unwrap();
    }

    #[test]
    fn saving_through_a_link_writes_its_target_and_keeps_the_link() {
        let dir = test_dir("fsedit-link");
        std::fs::write(dir.join("real.md"), "old\n").unwrap();
        if !crate::paths::make_file_link(&dir.join("real.md"), &dir.join("link.md")) {
            eprintln!("skipped: cannot create file links here");
            return;
        }
        write(&dir, "link.md", "new\n", "lf", false, None).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("real.md")).unwrap(),
            "new\n"
        );
        assert!(std::fs::symlink_metadata(dir.join("link.md"))
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[test]
    fn saving_through_a_chain_of_links_writes_the_last_target() {
        let dir = test_dir("fsedit-link-chain");
        std::fs::write(dir.join("c.md"), "old\n").unwrap();
        let made = crate::paths::make_file_link(&dir.join("c.md"), &dir.join("b.md"))
            && crate::paths::make_file_link(&dir.join("b.md"), &dir.join("a.md"));
        if !made {
            eprintln!("skipped: cannot create file links here");
            return;
        }
        write(&dir, "a.md", "new\n", "lf", false, None).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("c.md")).unwrap(), "new\n");
        for l in ["a.md", "b.md"] {
            assert!(
                std::fs::symlink_metadata(dir.join(l))
                    .unwrap()
                    .file_type()
                    .is_symlink(),
                "{l}"
            );
        }
    }

    #[test]
    fn a_planted_link_at_the_temporary_name_is_neither_followed_nor_removed() {
        let base = test_dir("fsedit-planted");
        let root = base.join("root");
        let outside = base.join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let target = root.join("a.txt");

        // A plain file already at the temporary name is neither overwritten nor removed. This
        // part runs on every system and tells `create_new` from `File::create`.
        let tmp = root.join(".a.txt.file.escouade-tmp");
        std::fs::write(&tmp, "foreign").unwrap();
        assert!(write_via(&target, &tmp, b"x").is_err());
        assert_eq!(std::fs::read_to_string(&tmp).unwrap(), "foreign");
        assert!(!target.exists());

        // A link to a file outside the folder is not written through. (A link to a folder cannot
        // be opened for writing whatever the flags, so it would not tell them apart.)
        let secret = outside.join("secret.txt");
        std::fs::write(&secret, "secret").unwrap();
        let tmp = root.join(".a.txt.filelink.escouade-tmp");
        if crate::paths::make_file_link(&secret, &tmp) {
            assert!(write_via(&target, &tmp, b"x").is_err());
            assert_eq!(std::fs::read_to_string(&secret).unwrap(), "secret");
            assert!(std::fs::symlink_metadata(&tmp).is_ok());
            assert!(!target.exists());
        } else {
            eprintln!("skipped: cannot create file links here");
        }

        // A link to a folder outside: nothing is created in it, the link stays.
        let tmp = root.join(".a.txt.dirlink.escouade-tmp");
        if crate::paths::make_dir_link(&outside, &tmp) {
            assert!(write_via(&target, &tmp, b"x").is_err());
            assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 1);
            assert!(std::fs::symlink_metadata(&tmp).is_ok());
            assert!(!target.exists());
        } else {
            eprintln!("skipped: cannot create a directory link here");
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_file_that_cannot_be_read_is_not_overwritten_by_a_guarded_save() {
        use std::os::unix::fs::PermissionsExt;
        if unsafe { libc::geteuid() } == 0 {
            eprintln!("skipped: root reads any file");
            return;
        }
        let dir = test_dir("fsedit-unreadable");
        let p = dir.join("secret.txt");
        std::fs::write(&p, "secret\n").unwrap();
        let h = hash(b"secret\n");
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o200)).unwrap();
        let err = write(&dir, "secret.txt", "mine\n", "lf", false, Some(&h));
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644)).unwrap();
        let err = err.unwrap_err().to_string();
        assert!(
            err != CHANGED && err != DELETED && !err.contains("lecture seule"),
            "{err}"
        );
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "secret\n");
    }

    fn git(dir: &Path, args: &[&str]) {
        let ok = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?}");
    }

    fn repo(name: &str) -> std::path::PathBuf {
        let r = test_dir(name);
        git(&r, &["init", "-q", "-b", "main"]);
        git(&r, &["config", "user.email", "t@t"]);
        git(&r, &["config", "user.name", "t"]);
        git(&r, &["config", "core.autocrlf", "false"]);
        std::fs::create_dir_all(r.join("src")).unwrap();
        std::fs::write(r.join("src/app.ts"), "const a = 1;\n").unwrap();
        std::fs::write(r.join(".gitignore"), "dist/\n").unwrap();
        git(&r, &["add", "-A"]);
        git(&r, &["commit", "-qm", "init"]);
        r
    }

    #[tokio::test]
    async fn lists_the_files_git_knows_without_the_ignored_ones() {
        let r = repo("fsedit-tree");
        std::fs::create_dir_all(r.join("dist")).unwrap();
        std::fs::write(r.join("dist/out.js"), "x").unwrap();
        std::fs::write(r.join("notes.md"), "n").unwrap();
        let t = tree(&r.to_string_lossy()).await;
        assert_eq!(t.files, vec![".gitignore", "notes.md", "src/app.ts"]);
        assert!(!t.truncated);
        assert_eq!(t.root, r.to_string_lossy());
    }

    #[tokio::test]
    async fn lists_a_plain_folder_by_walking_it() {
        let d = test_dir("fsedit-tree-plain");
        std::fs::create_dir_all(d.join("node_modules/x")).unwrap();
        std::fs::write(d.join("node_modules/x/i.js"), "").unwrap();
        std::fs::write(d.join("a.txt"), "").unwrap();
        assert_eq!(tree(&d.to_string_lossy()).await.files, vec!["a.txt"]);
    }

    #[tokio::test]
    async fn compares_with_head_or_with_where_a_branch_left_its_base() {
        let r = repo("fsedit-base");
        let root = r.to_string_lossy().to_string();
        std::fs::write(r.join("src/app.ts"), "const a = 2;\n").unwrap();
        let b = base(&root, None, "src/app.ts").await.unwrap().unwrap();
        assert_eq!(
            (b.reference.as_str(), b.text.as_deref()),
            ("HEAD", Some("const a = 1;\n"))
        );
        let new = base(&root, None, "src/new.ts").await.unwrap().unwrap();
        assert_eq!(new.text, None);
        // A branch with commits of its own: compared with where it left main.
        git(&r, &["checkout", "-qb", "feature"]);
        git(&r, &["commit", "-qam", "two"]);
        let b = base(&root, Some("main"), "src/app.ts")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            (b.reference.as_str(), b.text.as_deref()),
            ("main", Some("const a = 1;\n"))
        );
        assert!(base(&root, None, "../x").await.is_err());
        let plain = test_dir("fsedit-base-plain");
        assert_eq!(
            base(&plain.to_string_lossy(), None, "a.txt").await.unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn base_refuses_a_path_that_does_not_name_a_file() {
        let r = repo("fsedit-base-invalid");
        let root = r.to_string_lossy().to_string();
        for rel in [".", "./", "src/.", ""] {
            let err = base(&root, None, rel).await.unwrap_err().to_string();
            assert!(err.contains("chemin invalide"), "{rel:?}: {err}");
        }
    }

    #[tokio::test]
    async fn the_base_of_a_folder_is_not_a_listing_taken_for_text() {
        let r = repo("fsedit-base-folder");
        let b = base(&r.to_string_lossy(), None, "src")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(b.text, None);
    }

    #[tokio::test]
    async fn a_tree_too_big_is_cut_and_says_so() {
        let r = repo("fsedit-tree-cut");
        for n in 0..3 {
            std::fs::write(r.join(format!("f{n}.txt")), "").unwrap();
        }
        let root = r.to_string_lossy();
        // .gitignore, f0..f2 and src/app.ts: five files.
        let t = tree_up_to(&root, 4).await;
        assert_eq!((t.files.len(), t.truncated), (4, true));
        let t = tree_up_to(&root, 5).await;
        assert_eq!((t.files.len(), t.truncated), (5, false));

        let d = test_dir("fsedit-tree-cut-plain");
        for n in 0..3 {
            std::fs::write(d.join(format!("p{n}.txt")), "").unwrap();
        }
        let t = tree_up_to(&d.to_string_lossy(), 2).await;
        assert_eq!((t.files.len(), t.truncated), (2, true));
        let t = tree_up_to(&d.to_string_lossy(), 3).await;
        assert_eq!((t.files.len(), t.truncated), (3, false));
    }

    #[tokio::test]
    async fn the_tree_leaves_out_the_files_deleted_on_disk() {
        let r = repo("fsedit-tree-deleted");
        std::fs::write(r.join("gone.md"), "g").unwrap();
        git(&r, &["add", "gone.md"]);
        git(&r, &["commit", "-qm", "gone"]);
        // Deleted in the working tree, or removed from the index too: not on disk either way.
        std::fs::remove_file(r.join("src/app.ts")).unwrap();
        git(&r, &["rm", "-q", "gone.md"]);
        // Still there: a file added to the index, and one git does not know.
        std::fs::write(r.join("staged.md"), "s").unwrap();
        git(&r, &["add", "staged.md"]);
        std::fs::write(r.join("untracked.md"), "u").unwrap();
        let t = tree(&r.to_string_lossy()).await;
        assert_eq!(t.files, vec![".gitignore", "staged.md", "untracked.md"]);
    }

    #[tokio::test]
    async fn the_tree_walks_the_folder_when_git_cannot_list_it() {
        let r = repo("fsedit-tree-broken");
        std::fs::write(r.join("notes.md"), "n").unwrap();
        // A repository whose index is unreadable: git answers for the folder, not for the files.
        std::fs::write(r.join(".git/index"), "not an index").unwrap();
        let t = tree(&r.to_string_lossy()).await;
        assert_eq!(t.files, vec![".gitignore", "notes.md", "src/app.ts"]);
    }
}
