//! The embedded editor's access to files: a source's file list, reading and writing a file, and
//! the version a file is compared with.

use crate::git;
use crate::paths::contained;
use crate::testlaunch;
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

/// Refuses on Windows a path with a part it would not keep as typed or keeps for a device.
fn valid_names(rel: &str) -> Result<()> {
    #[cfg(windows)]
    for part in rel.split(['/', '\\']) {
        windows_name(part).map_err(|_| anyhow!("nom invalide : {rel}"))?;
    }
    #[cfg(not(windows))]
    let _ = rel;
    Ok(())
}

/// Creates the empty file `rel`, its missing folders with it. Refused when something is already
/// there: a file is never created over another.
pub fn create(root: &Path, rel: &str) -> Result<()> {
    validate_rel(rel)?;
    valid_names(rel)?;
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

/// Creates the empty folder `rel`, its missing parents with it. Refused when something is already
/// there, and in the agents' worktrees.
pub fn mkdir(root: &Path, rel: &str) -> Result<()> {
    validate_rel(rel)?;
    valid_names(rel)?;
    in_worktrees(root, rel)?;
    let path = contained(root, rel)?;
    if std::fs::symlink_metadata(&path).is_ok() {
        bail!("{rel} existe déjà");
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| anyhow!("{rel} : {e}"))?;
    }
    match std::fs::create_dir(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => bail!("{rel} existe déjà"),
        Err(e) => Err(anyhow!("{rel} : {e}")),
    }
}

/// Renames the file or folder `from` to `to`, into other folders when `to` names some (made if
/// need be). Refused when something is at `to` already, unless it is `from` itself, spelled in
/// another case on a disk that ignores it: changing only the case is a rename like any other. A
/// file written at `to` meanwhile (by an agent) makes it fail rather than be replaced.
pub fn rename(root: &Path, from: &str, to: &str, kept: &[String]) -> Result<()> {
    validate_rel(from)?;
    validate_rel(to)?;
    valid_names(to)?;
    in_worktrees(root, from)?;
    holds_worktrees(root, from, kept)?;
    // Named as a folder that would hold worktrees, `to` holds none: nothing is there.
    in_worktrees(root, to)?;
    let src = native(root, from)?;
    let dst = native(root, to)?;
    if std::fs::symlink_metadata(&src).is_err() {
        bail!("{from} introuvable");
    }
    let slash = |s: &str| s.replace('\\', "/");
    let (a, b) = (slash(from), slash(to));
    if a == b {
        return Ok(());
    }
    let (lower_a, lower_b) = (a.to_lowercase(), b.to_lowercase());
    if lower_b.starts_with(&format!("{lower_a}/")) {
        bail!("{to} est dans {from}");
    }
    if std::fs::symlink_metadata(&dst).is_ok() {
        // On a disk that ignores case, `to` is found because it is `from`: its name, spelled as
        // typed, is then not one its folder lists.
        let same_folder = Path::new(&a).parent() == Path::new(&b).parent();
        if !(same_folder && lower_a == lower_b && !listed(&dst)) {
            bail!("{to} existe déjà");
        }
        return std::fs::rename(&src, &dst).map_err(|e| anyhow!("{from} : {e}"));
    }
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(|e| anyhow!("{to} : {e}"))?;
    }
    move_new(&src, &dst).map_err(|e| match e.kind() {
        std::io::ErrorKind::AlreadyExists => anyhow!("{to} existe déjà"),
        _ => anyhow!("{from} : {e}"),
    })
}

/// Sends the file or folder `rel` (with all it holds) to the trash with `send` (`to_trash`, or
/// what a test gives). Never the root, nor the agents' worktrees or a folder holding them; a link
/// goes, not what it points to (which is never outside the root anyway).
pub fn delete(
    root: &Path,
    rel: &str,
    kept: &[String],
    send: impl FnOnce(&Path) -> Result<()>,
) -> Result<()> {
    validate_rel(rel)?;
    in_worktrees(root, rel)?;
    holds_worktrees(root, rel, kept)?;
    let path = native(root, rel)?;
    if std::fs::symlink_metadata(&path).is_err() {
        bail!("{rel} introuvable");
    }
    // A link to the root would take its place in the trash's eyes, or in the user's.
    if std::fs::canonicalize(&path)? == std::fs::canonicalize(root)? {
        bail!("{rel} est la racine de la source");
    }
    send(&path)
}

/// The system's trash: the Recycle Bin on Windows, the Trash on macOS, where the user can put the
/// file back. On a thread of its own: Windows' shell wants COM set up its way on the thread that
/// calls it, and `trash` panics when something else set it up otherwise on one of the pool's.
pub fn to_trash(path: &Path) -> Result<()> {
    let path = path.to_path_buf();
    std::thread::spawn(move || {
        #[cfg(target_os = "macos")]
        let ctx = {
            use trash::macos::{DeleteMethod, TrashContextExtMacos};
            let mut c = trash::TrashContext::default();
            // Through the Finder, macOS would first ask to let the app control it.
            c.set_delete_method(DeleteMethod::NsFileManager);
            c
        };
        #[cfg(not(target_os = "macos"))]
        let ctx = trash::TrashContext::default();
        ctx.delete(&path)
    })
    .join()
    .map_err(|_| anyhow!("la corbeille n’a pas répondu"))?
    .map_err(|e| anyhow!("{e}"))
}

/// Where the agents' worktrees are, from the folder holding them (the project's, or the root).
pub const WORKTREES: &str = ".claude/worktrees";

/// The names of a path, lowercase: case is ignored on Windows and macOS.
fn lower_parts(s: &str) -> Vec<String> {
    s.split(['/', '\\'])
        .filter(|p| !p.is_empty() && *p != ".")
        .map(str::to_lowercase)
        .collect()
}

/// Refuses `rel` when it is in the agents' worktrees: below a `.claude/worktrees` at any depth (as
/// the search leaves them out), spelled so or reached through a link on the way.
fn in_worktrees(root: &Path, rel: &str) -> Result<()> {
    for path in [Some(rel.to_string()), real_rel(root, rel)]
        .into_iter()
        .flatten()
    {
        let parts = lower_parts(&path);
        if parts
            .windows(2)
            .any(|w| w[0] == ".claude" && w[1] == "worktrees")
        {
            bail!("{rel} est dans les worktrees des agents");
        }
    }
    Ok(())
}

/// Refuses `rel` when renaming or deleting it would take worktrees with it: it holds one of the
/// `kept` folders (relative to the root) that is there.
fn holds_worktrees(root: &Path, rel: &str, kept: &[String]) -> Result<()> {
    let path = lower_parts(rel);
    let holds = |k: &String| lower_parts(k).starts_with(&path);
    if kept
        .iter()
        .any(|k| holds(k) && std::fs::symlink_metadata(root.join(k)).is_ok())
    {
        bail!("{rel} contient les worktrees des agents");
    }
    Ok(())
}

/// `rel` from the root as the disk has it: the links on the way to it followed (from the deepest
/// part of it that is there), not the one it may be itself. None outside the root, which
/// `contained` refuses anyway.
fn real_rel(root: &Path, rel: &str) -> Option<String> {
    let full = root.join(rel);
    let mut rest = vec![full.file_name()?.to_os_string()];
    let mut dir = full.parent()?.to_path_buf();
    let real = loop {
        if let Ok(real) = std::fs::canonicalize(&dir) {
            break real;
        }
        rest.push(dir.file_name()?.to_os_string());
        dir = dir.parent()?.to_path_buf();
    };
    let mut path = real
        .strip_prefix(std::fs::canonicalize(root).ok()?)
        .ok()?
        .to_path_buf();
    path.extend(rest.iter().rev());
    Some(path.to_string_lossy().replace('\\', "/"))
}

/// `rel` inside `root` (see `contained`), joined part by part: the tree's paths use `/`, which
/// the shell taking a file to the trash on Windows does not read as its `\`.
fn native(root: &Path, rel: &str) -> Result<std::path::PathBuf> {
    contained(root, rel)?;
    let mut path = root.to_path_buf();
    path.extend(Path::new(rel).components().filter_map(|c| match c {
        Component::Normal(name) => Some(name),
        _ => None,
    }));
    Ok(path)
}

/// Whether the folder of `path` lists its name as written, case included.
fn listed(path: &Path) -> bool {
    let (Some(dir), Some(name)) = (path.parent(), path.file_name()) else {
        return false;
    };
    std::fs::read_dir(dir)
        .map(|entries| entries.flatten().any(|e| e.file_name() == name))
        .unwrap_or(false)
}

/// Moves `from` to `to`, failing with `AlreadyExists` rather than replacing what is at `to` at
/// that very moment, an empty folder included. Never a copy: both are in the same root.
#[cfg(windows)]
fn move_new(from: &Path, to: &Path) -> std::io::Result<()> {
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;
    // No MOVEFILE_REPLACE_EXISTING: what is there stays, and the call fails.
    let ok = unsafe { MoveFileExW(wide(from).as_ptr(), wide(to).as_ptr(), 0) };
    if ok == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// `path` for a wide Win32 call, in its `\\?\` form when long: past 260 characters, a plain
/// path is refused.
#[cfg(windows)]
fn wide(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    let plain = path.to_string_lossy().replace('/', "\\");
    let long = if plain.len() < 248 || plain.starts_with(r"\\?\") {
        plain
    } else if let Some(share) = plain.strip_prefix(r"\\") {
        format!(r"\\?\UNC\{share}")
    } else {
        format!(r"\\?\{plain}")
    };
    std::ffi::OsStr::new(&long)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(target_os = "macos")]
fn move_new(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let c = |p: &Path| {
        std::ffi::CString::new(p.as_os_str().as_bytes())
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))
    };
    let (f, t) = (c(from)?, c(to)?);
    if unsafe { libc::renamex_np(f.as_ptr(), t.as_ptr(), libc::RENAME_EXCL) } == 0 {
        return Ok(());
    }
    let e = std::io::Error::last_os_error();
    // A volume that cannot (ENOTSUP): an ordinary rename, `to` found free just before.
    if e.raw_os_error() == Some(libc::ENOTSUP) && std::fs::symlink_metadata(to).is_err() {
        return std::fs::rename(from, to);
    }
    Err(e)
}

/// Elsewhere (the app is made for Windows and macOS), `to` is found free just before.
#[cfg(not(any(windows, target_os = "macos")))]
fn move_new(from: &Path, to: &Path) -> std::io::Result<()> {
    if std::fs::symlink_metadata(to).is_ok() {
        return Err(std::io::ErrorKind::AlreadyExists.into());
    }
    std::fs::rename(from, to)
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
    /// The files of `files` that git ignores: the ones the project copies into its worktrees.
    pub ignored: Vec<String>,
}

/// What git lists and is still on disk (tracked and untracked, the agents' worktrees left out),
/// or the folder walked when it is not a repository or git cannot list it. Ignored files are not
/// there, but those matching one of the `copied` patterns (the project's « Fichiers copiés dans les
/// worktrees »): the editor shows what the agents get beside the code. A file git tracks is never
/// one of them, and nothing here lets one into a commit. The patterns match from `root`, and also
/// from the folder `below` it (the project's, with a trailing slash, when it is a subfolder of its
/// repository): the copy takes from there, and the commit guard accepts both.
pub async fn tree(root: &str, copied: &[String], below: &str) -> Tree {
    tree_up_to(root, MAX_TREE_FILES, copied, below).await
}

/// `tree`, cut at `max` files (a smaller `max` lets the cut be tested).
async fn tree_up_to(root: &str, max: usize, copied: &[String], below: &str) -> Tree {
    let mut ignored = Vec::new();
    let listed = match git::toplevel(root).await {
        Some(_) => {
            // They all read the working tree: asked together.
            let (listed, shown, shown_below) = tokio::join!(
                git_files(root),
                testlaunch::matching_ignored(root, copied),
                copied_below(root, copied, below)
            );
            if listed.is_ok() {
                ignored = shown;
                ignored.extend(shown_below);
                ignored.sort();
                ignored.dedup();
            }
            listed.ok()
        }
        None => None,
    };
    // Not a repository, or git cannot list it: the folder is walked, and it has no ignored file.
    let mut files = match listed {
        Some(f) => f,
        None => walk(root, max).await,
    };
    let mut truncated = files.len() > max;
    files.truncate(max);
    // The copied files are cut apart from the others: a repository full of ignored logs matching a
    // wide pattern must not push the files of the code out of the tree.
    truncated |= ignored.len() > max;
    ignored.truncate(max);
    if !ignored.is_empty() {
        files.extend(ignored.iter().cloned());
        files.sort();
        files.dedup();
    }
    Tree {
        root: root.to_string(),
        files,
        truncated,
        ignored,
    }
}

/// The files matching `copied` in the folder `below` the root (empty for the root itself), from the
/// root as every path of the tree is.
async fn copied_below(root: &str, copied: &[String], below: &str) -> Vec<String> {
    if below.is_empty() {
        return Vec::new();
    }
    let dir = Path::new(root).join(below);
    testlaunch::matching_ignored(&dir.to_string_lossy(), copied)
        .await
        .into_iter()
        .map(|f| format!("{below}{f}"))
        .collect()
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

    /// The names in `dir`, sorted.
    fn names(dir: &Path) -> Vec<String> {
        let mut n: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        n.sort();
        n
    }

    #[test]
    fn renames_a_file_and_a_folder_with_all_it_holds() {
        let dir = test_dir("fsedit-rename");
        std::fs::write(dir.join("a.ts"), "a\n").unwrap();
        rename(&dir, "a.ts", "b.ts", &[]).unwrap();
        assert_eq!(names(&dir), vec!["b.ts"]);
        assert_eq!(std::fs::read_to_string(dir.join("b.ts")).unwrap(), "a\n");
        // Into folders made on the way.
        rename(&dir, "b.ts", "lib/deep/b.ts", &[]).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("lib/deep/b.ts")).unwrap(),
            "a\n"
        );
        std::fs::write(dir.join("lib/c.ts"), "c\n").unwrap();
        rename(&dir, "lib", "src", &[]).unwrap();
        assert_eq!(names(&dir), vec!["src"]);
        assert_eq!(names(&dir.join("src")), vec!["c.ts", "deep"]);
        assert!(dir.join("src/deep/b.ts").is_file());
        // Out of a folder, back to the root.
        rename(&dir, "src/deep/b.ts", "b.ts", &[]).unwrap();
        assert!(dir.join("b.ts").is_file());
        assert_eq!(
            rename(&dir, "missing.ts", "x.ts", &[])
                .unwrap_err()
                .to_string(),
            "missing.ts introuvable"
        );
        assert!(!dir.join("x.ts").exists());
    }

    #[test]
    fn renames_over_nothing_that_is_there() {
        let dir = test_dir("fsedit-rename-taken");
        std::fs::write(dir.join("a.ts"), "a\n").unwrap();
        std::fs::write(dir.join("b.ts"), "b\n").unwrap();
        std::fs::create_dir(dir.join("empty")).unwrap();
        let refused = |from: &str, to: &str| rename(&dir, from, to, &[]).unwrap_err().to_string();
        assert_eq!(refused("a.ts", "b.ts"), "b.ts existe déjà");
        // Not even an empty folder, which a rename could take the place of on macOS.
        assert_eq!(refused("a.ts", "empty"), "empty existe déjà");
        assert_eq!(refused("empty", "a.ts"), "a.ts existe déjà");
        #[cfg(any(windows, target_os = "macos"))]
        assert_eq!(refused("a.ts", "B.TS"), "B.TS existe déjà");
        assert_eq!(names(&dir), vec!["a.ts", "b.ts", "empty"]);
        assert_eq!(std::fs::read_to_string(dir.join("a.ts")).unwrap(), "a\n");
        assert_eq!(std::fs::read_to_string(dir.join("b.ts")).unwrap(), "b\n");
        // A folder into itself: refused before any folder is made in it.
        assert_eq!(
            refused("empty", "empty/sub/empty"),
            "empty/sub/empty est dans empty"
        );
        assert_eq!(names(&dir.join("empty")), Vec::<String>::new());
        // A folder where a file is.
        assert!(rename(&dir, "a.ts", "b.ts/a.ts", &[]).is_err());
        assert_eq!(names(&dir), vec!["a.ts", "b.ts", "empty"]);
    }

    #[test]
    fn renames_a_file_or_a_folder_in_another_case_only() {
        let dir = test_dir("fsedit-rename-case");
        std::fs::write(dir.join("a.ts"), "a\n").unwrap();
        rename(&dir, "a.ts", "A.ts", &[]).unwrap();
        assert_eq!(names(&dir), vec!["A.ts"]);
        assert_eq!(std::fs::read_to_string(dir.join("A.ts")).unwrap(), "a\n");
        std::fs::create_dir(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/x.ts"), "x\n").unwrap();
        rename(&dir, "src", "Src", &[]).unwrap();
        assert_eq!(names(&dir), vec!["A.ts", "Src"]);
        assert_eq!(names(&dir.join("Src")), vec!["x.ts"]);
        // The same name: nothing to do.
        rename(&dir, "A.ts", "A.ts", &[]).unwrap();
        assert_eq!(names(&dir), vec!["A.ts", "Src"]);
    }

    #[test]
    fn renames_nothing_out_of_its_root_nor_through_a_link_leaving_it() {
        let base = test_dir("fsedit-rename-out");
        let root = base.join("root");
        let outside = base.join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(root.join("a.ts"), "a\n").unwrap();
        std::fs::write(outside.join("secret.txt"), "s\n").unwrap();
        for (from, to) in [
            ("../outside/secret.txt", "s.txt"),
            ("a.ts", "../escape.ts"),
            ("a.ts", "sub/../../escape.ts"),
            ("", "x.ts"),
            (".", "x"),
            ("a.ts", ""),
            ("a.ts", "."),
            ("a.ts", "sub/."),
        ] {
            assert!(rename(&root, from, to, &[]).is_err(), "{from:?} -> {to:?}");
        }
        let abs = outside.join("moved.ts").to_string_lossy().into_owned();
        assert!(rename(&root, "a.ts", &abs, &[]).is_err());
        if crate::paths::make_dir_link(&outside, &root.join("out")) {
            assert!(rename(&root, "out/secret.txt", "s.txt", &[]).is_err());
            assert!(rename(&root, "a.ts", "out/a.ts", &[]).is_err());
            assert!(rename(&root, "a.ts", "out/new/a.ts", &[]).is_err());
            assert_eq!(names(&outside), vec!["secret.txt"]);
        } else {
            eprintln!("skipped: cannot create a directory link here");
        }
        assert!(root.join("a.ts").is_file());
        assert!(!root.join("s.txt").exists());
        assert_eq!(names(&base), vec!["outside", "root"]);
    }

    #[test]
    fn the_move_itself_fails_rather_than_replace_what_appeared_at_its_target() {
        // What an agent writes at the target between the check and the move is never replaced.
        let dir = test_dir("fsedit-move-new");
        std::fs::write(dir.join("a.ts"), "a\n").unwrap();
        std::fs::write(dir.join("b.ts"), "agent\n").unwrap();
        std::fs::create_dir(dir.join("d")).unwrap();
        let e = move_new(&dir.join("a.ts"), &dir.join("b.ts")).unwrap_err();
        assert_eq!(e.kind(), std::io::ErrorKind::AlreadyExists, "{e}");
        assert_eq!(
            std::fs::read_to_string(dir.join("b.ts")).unwrap(),
            "agent\n"
        );
        std::fs::create_dir(dir.join("empty")).unwrap();
        assert!(move_new(&dir.join("d"), &dir.join("empty")).is_err());
        assert_eq!(names(&dir), vec!["a.ts", "b.ts", "d", "empty"]);
        move_new(&dir.join("a.ts"), &dir.join("c.ts")).unwrap();
        assert_eq!(names(&dir), vec!["b.ts", "c.ts", "d", "empty"]);
    }

    /// What the trash of a test was sent.
    type Sent = std::rc::Rc<std::cell::RefCell<Vec<std::path::PathBuf>>>;

    /// A trash that keeps what it is sent, and leaves it on the disk.
    fn bin() -> (Sent, impl Fn(&Path) -> Result<()>) {
        let sent = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let keep = sent.clone();
        (sent, move |p: &Path| {
            keep.borrow_mut().push(p.to_path_buf());
            Ok(())
        })
    }

    #[test]
    fn deletes_a_file_or_a_folder_by_sending_it_to_the_trash() {
        let dir = test_dir("fsedit-delete");
        std::fs::create_dir_all(dir.join("src/lib")).unwrap();
        std::fs::write(dir.join("src/lib/a.ts"), "a\n").unwrap();
        let (sent, send) = bin();
        delete(&dir, "src/lib/a.ts", &[], &send).unwrap();
        delete(&dir, "src", &[], &send).unwrap();
        // Given with the system's separators, for its trash to read; nothing removed but by it.
        assert_eq!(
            *sent.borrow(),
            vec![dir.join("src").join("lib").join("a.ts"), dir.join("src")]
        );
        assert!(dir.join("src/lib/a.ts").is_file());
        assert_eq!(
            delete(&dir, "missing.ts", &[], &send)
                .unwrap_err()
                .to_string(),
            "missing.ts introuvable"
        );
        let err = delete(&dir, "src", &[], |_| bail!("corbeille pleine")).unwrap_err();
        assert_eq!(err.to_string(), "corbeille pleine");
        assert_eq!(sent.borrow().len(), 2);
    }

    #[test]
    fn deletes_neither_the_root_nor_anything_out_of_it() {
        let base = test_dir("fsedit-delete-root");
        let root = base.join("root");
        let outside = base.join("outside");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("secret.txt"), "s\n").unwrap();
        let (sent, send) = bin();
        for rel in [
            "",
            ".",
            "./",
            "sub/..",
            "sub/.",
            "..",
            "../outside/secret.txt",
        ] {
            assert!(delete(&root, rel, &[], &send).is_err(), "{rel:?}");
        }
        let abs = outside.join("secret.txt").to_string_lossy().into_owned();
        assert!(delete(&root, &abs, &[], &send).is_err());
        if crate::paths::make_dir_link(&outside, &root.join("out")) {
            assert!(delete(&root, "out/secret.txt", &[], &send).is_err());
        }
        // A link to the root itself is the root.
        if crate::paths::make_dir_link(&root, &root.join("self")) {
            let err = delete(&root, "self", &[], &send).unwrap_err().to_string();
            assert!(err.contains("racine"), "{err}");
        }
        assert!(sent.borrow().is_empty());
    }

    #[test]
    fn never_renames_deletes_or_makes_a_folder_in_the_agents_worktrees() {
        let dir = test_dir("fsedit-worktrees");
        for f in [
            ".claude/settings.json",
            ".claude/worktrees/dem-1/x.ts",
            "packages/web/index.ts",
            "packages/web/.claude/worktrees/dem-2/y.ts",
            "sub/.claude/worktrees/z/z.ts",
        ] {
            let p = dir.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, "x").unwrap();
        }
        let kept = vec![
            ".claude/worktrees".to_string(),
            "packages/web/.claude/worktrees".to_string(),
        ];
        let (sent, send) = bin();
        // In them, at any depth, or holding those of the project's folder.
        for rel in [
            ".claude/worktrees/dem-1/x.ts",
            ".claude/worktrees/dem-1",
            ".claude/worktrees",
            ".CLAUDE/Worktrees/dem-1",
            "sub/.claude/worktrees/z",
            ".claude",
            "packages",
            "packages/web",
            "packages/web/.claude",
        ] {
            let err = delete(&dir, rel, &kept, &send).unwrap_err().to_string();
            assert!(err.contains("worktrees des agents"), "{rel}: {err}");
            let err = rename(&dir, rel, "moved", &kept).unwrap_err().to_string();
            assert!(err.contains("worktrees des agents"), "{rel}: {err}");
        }
        assert!(rename(
            &dir,
            ".claude/settings.json",
            ".claude/worktrees/s.json",
            &kept
        )
        .is_err());
        assert!(mkdir(&dir, ".claude/worktrees/new").is_err());
        assert!(mkdir(&dir, "packages/web/.claude/worktrees/new").is_err());
        assert!(sent.borrow().is_empty());
        assert!(!dir.join("moved").exists());
        // What is beside them is the project's like any file.
        rename(&dir, "packages/web/index.ts", "packages/web/main.ts", &kept).unwrap();
        delete(&dir, ".claude/settings.json", &kept, &send).unwrap();
        mkdir(&dir, ".claude/agents").unwrap();
        assert_eq!(sent.borrow().len(), 1);
    }

    #[test]
    fn a_folder_that_would_hold_worktrees_but_holds_none_is_like_any_other() {
        let dir = test_dir("fsedit-worktrees-none");
        std::fs::create_dir_all(dir.join(".claude")).unwrap();
        std::fs::write(dir.join(".claude/settings.json"), "x").unwrap();
        std::fs::create_dir_all(dir.join("docs")).unwrap();
        // The project's folder and the root, where no agent has a worktree yet.
        let kept = vec![
            ".claude/worktrees".to_string(),
            "web/.claude/worktrees".to_string(),
        ];
        let (sent, send) = bin();
        // Named as one of them, or as a folder above one: still none there.
        rename(&dir, "docs", "web", &kept).unwrap();
        rename(&dir, "web", "docs", &kept).unwrap();
        delete(&dir, ".claude", &kept, &send).unwrap();
        assert_eq!(*sent.borrow(), vec![dir.join(".claude")]);
    }

    #[test]
    fn a_link_into_the_agents_worktrees_does_not_lead_into_them() {
        let dir = test_dir("fsedit-worktrees-link");
        // Joined part by part: `mklink /J` takes no `/`.
        let wt = dir.join(".claude").join("worktrees").join("dem-1");
        std::fs::create_dir_all(&wt).unwrap();
        std::fs::write(wt.join("x.ts"), "x").unwrap();
        if !crate::paths::make_dir_link(&wt, &dir.join("wt")) {
            eprintln!("skipped: cannot create a directory link here");
            return;
        }
        let kept = vec![".claude/worktrees".to_string()];
        let (sent, send) = bin();
        let err = delete(&dir, "wt/x.ts", &kept, &send)
            .unwrap_err()
            .to_string();
        assert!(err.contains("worktrees des agents"), "{err}");
        let err = rename(&dir, "wt/x.ts", "x.ts", &kept)
            .unwrap_err()
            .to_string();
        assert!(err.contains("worktrees des agents"), "{err}");
        std::fs::write(dir.join("a.ts"), "a").unwrap();
        let err = rename(&dir, "a.ts", "wt/a.ts", &kept)
            .unwrap_err()
            .to_string();
        assert!(err.contains("worktrees des agents"), "{err}");
        let err = rename(&dir, "a.ts", "wt/new/a.ts", &kept)
            .unwrap_err()
            .to_string();
        assert!(err.contains("worktrees des agents"), "{err}");
        assert!(mkdir(&dir, "wt/new").is_err());
        assert!(wt.join("x.ts").is_file());
        assert!(!wt.join("new").exists());
        // The link itself goes, not what it leads to.
        delete(&dir, "wt", &kept, &send).unwrap();
        assert_eq!(*sent.borrow(), vec![dir.join("wt")]);
    }

    #[test]
    fn makes_an_empty_folder_with_its_parents_but_never_over_what_is_there() {
        let dir = test_dir("fsedit-mkdir");
        mkdir(&dir, "a").unwrap();
        assert!(dir.join("a").is_dir());
        assert_eq!(names(&dir.join("a")), Vec::<String>::new());
        mkdir(&dir, "b/c/d").unwrap();
        assert!(dir.join("b/c/d").is_dir());
        std::fs::write(dir.join("f.ts"), "keep\n").unwrap();
        let refused = |rel: &str| mkdir(&dir, rel).unwrap_err().to_string();
        assert_eq!(refused("a"), "a existe déjà");
        assert_eq!(refused("f.ts"), "f.ts existe déjà");
        #[cfg(any(windows, target_os = "macos"))]
        assert_eq!(refused("A"), "A existe déjà");
        assert!(mkdir(&dir, "f.ts/x").is_err());
        assert_eq!(std::fs::read_to_string(dir.join("f.ts")).unwrap(), "keep\n");
        for rel in ["", ".", "sub/.", "../escape"] {
            assert!(mkdir(&dir, rel).is_err(), "{rel:?}");
        }
        assert!(!dir.parent().unwrap().join("escape").exists());
        #[cfg(windows)]
        for rel in ["x.", "CON", "y/nul", "q?"] {
            assert!(refused(rel).contains("nom invalide"), "{rel}");
        }
        assert_eq!(names(&dir), vec!["a", "b", "f.ts"]);
    }

    #[cfg(windows)]
    #[test]
    fn renames_to_no_name_windows_would_change_or_keeps_for_a_device() {
        let dir = test_dir("fsedit-rename-win");
        std::fs::write(dir.join("a.ts"), "a\n").unwrap();
        for to in ["b.", "CON", "x/nul.txt", "c?.ts"] {
            let err = rename(&dir, "a.ts", to, &[]).unwrap_err().to_string();
            assert!(err.contains("nom invalide"), "{to:?}: {err}");
        }
        assert_eq!(names(&dir), vec!["a.ts"]);
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
        let t = tree(&r.to_string_lossy(), &[], "").await;
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
        assert_eq!(
            tree(&d.to_string_lossy(), &[], "").await.files,
            vec!["a.txt"]
        );
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
        let t = tree_up_to(&root, 4, &[], "").await;
        assert_eq!((t.files.len(), t.truncated), (4, true));
        let t = tree_up_to(&root, 5, &[], "").await;
        assert_eq!((t.files.len(), t.truncated), (5, false));

        let d = test_dir("fsedit-tree-cut-plain");
        for n in 0..3 {
            std::fs::write(d.join(format!("p{n}.txt")), "").unwrap();
        }
        let t = tree_up_to(&d.to_string_lossy(), 2, &[], "").await;
        assert_eq!((t.files.len(), t.truncated), (2, true));
        let t = tree_up_to(&d.to_string_lossy(), 3, &[], "").await;
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
        let t = tree(&r.to_string_lossy(), &[], "").await;
        assert_eq!(t.files, vec![".gitignore", "staged.md", "untracked.md"]);
    }

    #[tokio::test]
    async fn the_tree_walks_the_folder_when_git_cannot_list_it() {
        let r = repo("fsedit-tree-broken");
        std::fs::write(r.join("notes.md"), "n").unwrap();
        // A repository whose index is unreadable: git answers for the folder, not for the files.
        std::fs::write(r.join(".git/index"), "not an index").unwrap();
        let t = tree(&r.to_string_lossy(), &[], "").await;
        assert_eq!(t.files, vec![".gitignore", "notes.md", "src/app.ts"]);
    }

    fn patterns(p: &[&str]) -> Vec<String> {
        p.iter().map(|s| s.to_string()).collect()
    }

    /// A repository ignoring `.env`, `.env.local`, `dist/` and `node_modules/`, with those files
    /// (and a `.env.example` it does not ignore) on disk.
    fn repo_with_env(name: &str) -> std::path::PathBuf {
        let r = repo(name);
        std::fs::write(
            r.join(".gitignore"),
            "dist/\nnode_modules/\n.env\n.env.local\n",
        )
        .unwrap();
        for f in [".env", ".env.local", ".env.example", "node_modules/x.js"] {
            let p = r.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, "x").unwrap();
        }
        r
    }

    #[tokio::test]
    async fn the_tree_shows_the_ignored_files_the_copy_patterns_name_and_no_other() {
        let r = repo_with_env("fsedit-tree-copied");
        let t = tree(&r.to_string_lossy(), &patterns(&[".env*"]), "").await;
        assert_eq!(t.ignored, vec![".env", ".env.local"]);
        // In the list too, in its order; node_modules stays out, and `.env.example`, which git does
        // not ignore, is a file like any other.
        assert_eq!(
            t.files,
            vec![
                ".env",
                ".env.example",
                ".env.local",
                ".gitignore",
                "src/app.ts"
            ]
        );
        assert!(!t.truncated);
    }

    #[tokio::test]
    async fn the_tree_marks_nothing_without_patterns_or_in_a_plain_folder() {
        let r = repo_with_env("fsedit-tree-copied-none");
        let t = tree(&r.to_string_lossy(), &[], "").await;
        assert!(t.ignored.is_empty());
        assert!(!t.files.contains(&".env".to_string()));
        let t = tree(&r.to_string_lossy(), &patterns(&["", "  "]), "").await;
        assert!(t.ignored.is_empty());

        // Not a repository: nothing is ignored, the walk lists `.env` like any file.
        let d = test_dir("fsedit-tree-copied-plain");
        std::fs::write(d.join(".env"), "x").unwrap();
        let t = tree(&d.to_string_lossy(), &patterns(&[".env*"]), "").await;
        assert_eq!(t.files, vec![".env"]);
        assert!(t.ignored.is_empty());
    }

    #[tokio::test]
    async fn a_pattern_with_double_stars_reaches_the_copied_files_of_the_folders() {
        let r = repo_with_env("fsedit-tree-copied-deep");
        // A folder git tracks something in: one holding nothing but ignored files is listed whole by
        // git, as the copy sees it.
        std::fs::create_dir_all(r.join("api")).unwrap();
        std::fs::write(r.join("api/index.ts"), "x").unwrap();
        git(&r, &["add", "api/index.ts"]);
        std::fs::write(r.join("api/.env"), "x").unwrap();
        // The gitignore's `.env` ignores it at any depth; the pattern `.env*` names the root only.
        let root = r.to_string_lossy();
        let t = tree(&root, &patterns(&[".env*"]), "").await;
        assert_eq!(t.ignored, vec![".env", ".env.local"]);
        assert!(!t.files.contains(&"api/.env".to_string()));
        let t = tree(&root, &patterns(&["**/.env*"]), "").await;
        assert_eq!(t.ignored, vec![".env", ".env.local", "api/.env"]);
        assert!(t.files.contains(&"api/.env".to_string()));
    }

    #[tokio::test]
    async fn a_project_below_the_root_of_its_repository_gets_its_own_copied_files_too() {
        let r = repo_with_env("fsedit-tree-copied-below");
        std::fs::create_dir_all(r.join("packages/web")).unwrap();
        std::fs::write(r.join("packages/web/index.ts"), "x").unwrap();
        git(&r, &["add", "packages/web/index.ts"]);
        std::fs::write(r.join("packages/web/.env"), "x").unwrap();
        std::fs::write(r.join("packages/web/.env.local"), "x").unwrap();
        let root = r.to_string_lossy();
        // The copy takes what matches from the project's folder: its `.env` is shown where it is, the
        // checkout's own `.env` too (the commit guard accepts both).
        let t = tree(&root, &patterns(&[".env*"]), "packages/web/").await;
        assert_eq!(
            t.ignored,
            vec![
                ".env",
                ".env.local",
                "packages/web/.env",
                "packages/web/.env.local"
            ]
        );
        assert!(t.files.contains(&"packages/web/.env".to_string()));
        // A project at the root of its repository has nothing below it.
        let t = tree(&root, &patterns(&[".env*"]), "").await;
        assert_eq!(t.ignored, vec![".env", ".env.local"]);
        // Patterns naming folders still match from the project's folder.
        let t = tree(&root, &patterns(&["**/.env"]), "packages/web/").await;
        assert_eq!(t.ignored, vec![".env", "packages/web/.env"]);
    }

    #[tokio::test]
    async fn a_file_git_tracks_is_not_marked_ignored_even_when_a_pattern_names_it() {
        let r = repo_with_env("fsedit-tree-copied-tracked");
        git(&r, &["add", "-f", ".env"]);
        let t = tree(&r.to_string_lossy(), &patterns(&[".env*"]), "").await;
        assert_eq!(t.ignored, vec![".env.local"]);
        assert!(t.files.contains(&".env".to_string()));
    }

    #[tokio::test]
    async fn the_ignored_files_shown_are_cut_too_and_the_tree_says_so() {
        let r = repo("fsedit-tree-copied-cut");
        std::fs::write(r.join(".gitignore"), "*.log\n").unwrap();
        for n in 0..4 {
            std::fs::write(r.join(format!("{n}.log")), "").unwrap();
        }
        // .gitignore and src/app.ts are listed whole; four ignored files, three allowed.
        let t = tree_up_to(&r.to_string_lossy(), 3, &patterns(&["*.log"]), "").await;
        assert_eq!(t.ignored.len(), 3);
        assert!(t.truncated);
        assert!(t.ignored.iter().all(|f| t.files.contains(f)));
        let t = tree_up_to(&r.to_string_lossy(), 4, &patterns(&["*.log"]), "").await;
        assert_eq!((t.ignored.len(), t.truncated), (4, false));
    }
}
