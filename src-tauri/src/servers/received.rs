//! The files a server's agents send, received on this computer (ESC-538, S12, `docs/SERVER-UX.md` § 4): downloaded when
//! the user opens one (never ahead), into `Téléchargements/Escouade/<espace>`, then opened from there — only what is on
//! the list of what Escouade opens (ESC-629, [`opening`]): an image or a PDF with what reads that type on this computer,
//! plain text, Markdown, CSV or JSON in Escouade's editor; anything else (a program, a script, a console, a link, a disk
//! image, a page of the web…) is only downloaded and shown in its folder, and the window says why.
//!
//! What the server sends is not trusted to say what it is:
//! - its kind is read from its bytes: a program (PE, ELF, Mach-O) or a script (`#!`) is one whatever its name; a name
//!   of a program or a script (`sentfiles::runnable_name`) is one whatever its bytes;
//! - an image or a PDF keeps the type its bytes say: a name that says another (or none) gets the right extension, so
//!   that the system opens it with what reads that type;
//! - the copy is never made executable, and it is marked as come from elsewhere: under Windows `Zone.Identifier`
//!   (zone 3, as a download of the web), under macOS `com.apple.quarantine` (Gatekeeper asks before anything runs it);
//! - nothing is written out of the folder of the space, and nothing is opened without the user's gesture.

use anyhow::{bail, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// A file received, as the window says it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Received {
    /// The copy on this computer.
    pub path: String,
    /// Its folder (`…/Téléchargements/Escouade/Travail`).
    pub folder: String,
    /// Its name there (the extension of its type added when its name said another).
    pub name: String,
    /// What its bytes say: `program`, `script`, `image`, `pdf`, `file`.
    pub kind: &'static str,
    /// A program or a script: never opened from Escouade.
    pub runnable: bool,
    /// How it opens here ([`Opening`]): `system`, `editor` or `folder`.
    pub opens: &'static str,
    /// It was opened (asked, and no program).
    pub opened: bool,
}

/// What the bytes of a file are, as far as opening it goes.
pub fn kind_of(bytes: &[u8]) -> &'static str {
    const MACHO: [[u8; 4]; 5] = [
        [0xFE, 0xED, 0xFA, 0xCE],
        [0xFE, 0xED, 0xFA, 0xCF],
        [0xCE, 0xFA, 0xED, 0xFE],
        [0xCF, 0xFA, 0xED, 0xFE],
        [0xCA, 0xFE, 0xBA, 0xBE],
    ];
    if bytes.starts_with(b"MZ")
        || bytes.starts_with(b"\x7fELF")
        || MACHO.iter().any(|m| bytes.starts_with(m))
    {
        return "program";
    }
    let text = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    if text.starts_with(b"#!") {
        return "script";
    }
    if bytes.starts_with(b"%PDF-") {
        return "pdf";
    }
    if crate::sentfiles::image_kind(bytes).is_some_and(|k| k != "image/svg+xml") {
        return "image";
    }
    "file"
}

/// The extensions of a type its bytes say (the first is the one added).
fn extensions_of(bytes: &[u8]) -> &'static [&'static str] {
    if bytes.starts_with(b"%PDF-") {
        return &["pdf"];
    }
    match crate::sentfiles::image_kind(bytes) {
        Some("image/png") => &["png"],
        Some("image/jpeg") => &["jpg", "jpeg", "jfif"],
        Some("image/gif") => &["gif"],
        Some("image/webp") => &["webp"],
        _ => &[],
    }
}

/// The name a received file is kept under: made safe for this computer's system, with the extension of the type its
/// bytes say when its own says another.
pub fn kept_name(name: &str, bytes: &[u8]) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let safe = crate::attachfiles::safe_name(base);
    let exts = extensions_of(bytes);
    if exts.is_empty() {
        return safe;
    }
    let ext = safe
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    if exts.contains(&ext.as_str()) {
        safe
    } else {
        format!("{safe}.{}", exts[0])
    }
}

/// The folder of the space `space` in `downloads`: `Escouade/<espace>`, its name made safe.
pub fn folder_of(downloads: &Path, space: &str) -> PathBuf {
    downloads
        .join("Escouade")
        .join(crate::attachfiles::safe_name(space.trim()))
}

/// This computer's folder of downloads (in a build of development, `ESCOUADE_DOWNLOADS_DIR` for the tests).
pub fn downloads() -> PathBuf {
    if cfg!(debug_assertions) {
        if let Some(d) = std::env::var_os("ESCOUADE_DOWNLOADS_DIR").filter(|v| !v.is_empty()) {
            return PathBuf::from(d);
        }
    }
    dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
        .unwrap_or_else(std::env::temp_dir)
}

/// A path of `dir` for `name` no file holds yet: `rapport.pdf`, then `rapport (2).pdf`…
fn free_path(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (name.to_string(), String::new()),
    };
    (2..10_000)
        .map(|n| dir.join(format!("{stem} ({n}){ext}")))
        .find(|p| !p.exists())
        .unwrap_or(first)
}

/// Whether a received file is one never opened: its bytes or its name say a program or a script.
pub fn runnable(name: &str, bytes: &[u8]) -> bool {
    matches!(kind_of(bytes), "program" | "script") || crate::sentfiles::runnable_name(name)
}

/// How a file received from a server opens on this computer (ESC-629): a list of what opens, everything else shown in
/// its folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opening {
    /// An image (PNG, JPEG, GIF, WebP) or a PDF, its bytes and its name agreeing: opened with what reads that type.
    System,
    /// Plain text, Markdown, CSV or JSON (`.txt`, `.md`, `.markdown`, `.csv`, `.json`), its bytes text: read in
    /// Escouade's editor, never handed to the system (a spreadsheet would compute a CSV's formulas).
    Editor,
    /// Anything else: only shown in its folder.
    Folder,
}

impl Opening {
    /// Its name for the window.
    pub fn as_str(self) -> &'static str {
        match self {
            Opening::System => "system",
            Opening::Editor => "editor",
            Opening::Folder => "folder",
        }
    }
}

/// The extensions of the texts read in Escouade's editor.
const TEXTS: &[&str] = &["txt", "md", "markdown", "csv", "json"];

/// How the file `name` (its name kept here), of the bytes `bytes` (its start is enough), opens: on the list only when
/// its name and its bytes say the same type, never a program or a script.
pub fn opening(name: &str, bytes: &[u8]) -> Opening {
    if runnable(name, bytes) {
        return Opening::Folder;
    }
    let name = name.trim().trim_end_matches(['.', ' ']);
    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    if matches!(kind_of(bytes), "image" | "pdf") && extensions_of(bytes).contains(&ext.as_str()) {
        return Opening::System;
    }
    if TEXTS.contains(&ext.as_str()) && is_text(bytes) {
        return Opening::Editor;
    }
    Opening::Folder
}

/// Whether `bytes` read as UTF-8 text (a character cut at their end allowed: they may be a file's start), with no NUL.
fn is_text(bytes: &[u8]) -> bool {
    if bytes.contains(&0) {
        return false;
    }
    match std::str::from_utf8(bytes) {
        Ok(_) => true,
        Err(e) => e.error_len().is_none(),
    }
}

/// The bytes `bytes` of the file `name` sent from the space `space`, kept in its folder of `downloads`.
pub fn save(downloads: &Path, space: &str, name: &str, bytes: &[u8]) -> Result<Received> {
    if space.trim().is_empty() {
        bail!("no space said");
    }
    let dir = folder_of(downloads, space);
    std::fs::create_dir_all(&dir)?;
    let kept = kept_name(name, bytes);
    let path = free_path(&dir, &kept);
    std::fs::write(&path, bytes)?;
    never_runnable(&path);
    mark_from_elsewhere(&path);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or(kept);
    Ok(Received {
        runnable: runnable(&name, bytes),
        opens: opening(&name, bytes).as_str(),
        kind: kind_of(bytes),
        folder: dir.to_string_lossy().to_string(),
        path: path.to_string_lossy().to_string(),
        name,
        opened: false,
    })
}

/// Under macOS and Linux, the copy without the right to run (`rw-r--r--`), whatever the umask said.
fn never_runnable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)) {
            log::debug!("received: the copy's rights not set: {e}");
        }
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// The value of `com.apple.quarantine` macOS gives a download: its flags (`0081`: downloaded, the user asked before it
/// opens), the time in hexadecimal seconds, the app that downloaded it.
#[cfg(any(target_os = "macos", test))]
pub(crate) fn quarantine_value(now: std::time::SystemTime) -> String {
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("0081;{secs:08x};Escouade;")
}

/// The attribute `com.apple.quarantine` set on `path`.
#[cfg(target_os = "macos")]
fn quarantine(path: &Path) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let c_path = std::ffi::CString::new(path.as_os_str().as_bytes())?;
    let value = quarantine_value(std::time::SystemTime::now());
    // SAFETY: both strings end with their NUL and live past the call; the value is read for its length only.
    let r = unsafe {
        libc::setxattr(
            c_path.as_ptr(),
            c"com.apple.quarantine".as_ptr(),
            value.as_ptr().cast(),
            value.len(),
            0,
            0,
        )
    };
    if r == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// The copy marked as come from another machine, as a download of the web: under Windows its `Zone.Identifier` (zone
/// 3), under macOS its `com.apple.quarantine` (Gatekeeper asks before anything runs it, Terminal included). Linux has
/// no such mark: the copy is only never executable.
fn mark_from_elsewhere(path: &Path) {
    #[cfg(windows)]
    {
        let stream = format!("{}:Zone.Identifier", path.display());
        if let Err(e) = std::fs::write(stream, "[ZoneTransfer]\r\nZoneId=3\r\n") {
            log::debug!("received: the copy not marked as downloaded: {e}");
        }
    }
    #[cfg(target_os = "macos")]
    if let Err(e) = quarantine(path) {
        log::debug!("received: the copy not put in quarantine: {e}");
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    let _ = path;
}

/// The copy `path` (one of `downloads`' folder of Escouade, never another file) to open or show in its folder: refused
/// out of it, or to open by the system when it is not an image or a PDF ([`Opening::System`]), with why.
pub fn shown(downloads: &Path, path: &Path, reveal: bool) -> Result<PathBuf> {
    let lang = crate::i18n::ui();
    let root = downloads.join("Escouade");
    let (Ok(root), Ok(file)) = (dunce_canon(&root), dunce_canon(path)) else {
        bail!(tr_in!(
            lang,
            "Cette copie n’est plus là.",
            "This copy is no longer there."
        ));
    };
    if !file.starts_with(&root) || !file.is_file() {
        bail!("not a received file: {}", path.display());
    }
    if !reveal {
        let name = file
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let head = head_of(&file);
        if runnable(&name, &head) {
            bail!(tr_in!(
                lang,
                "« {name} » est un programme ou un script : Escouade ne l’ouvre pas, elle l’affiche dans son dossier.",
                "“{name}” is a program or a script: Escouade doesn’t open it, it shows it in its folder."
            ));
        }
        match opening(&name, &head) {
            Opening::System => {}
            Opening::Editor => bail!(tr_in!(
                lang,
                "« {name} » se lit dans l’éditeur d’Escouade : le système ne l’ouvre pas, elle l’affiche dans son dossier.",
                "“{name}” reads in Escouade’s editor: the system doesn’t open it, it is shown in its folder."
            )),
            Opening::Folder => bail!(tr_in!(
                lang,
                "« {name} » n’est pas d’un type qu’Escouade ouvre (images, PDF, texte, Markdown, CSV, JSON) : elle l’affiche dans son dossier.",
                "“{name}” is not of a type Escouade opens (images, PDF, text, Markdown, CSV, JSON): it shows it in its folder."
            )),
        }
    }
    Ok(file)
}

fn head_of(path: &Path) -> Vec<u8> {
    use std::io::Read;
    let mut head = vec![0u8; 4096];
    let n = std::fs::File::open(path)
        .and_then(|mut f| f.read(&mut head))
        .unwrap_or(0);
    head.truncate(n);
    head
}

/// A path made canonical, without Windows' `\\?\` prefix, so that two of them compare.
fn dunce_canon(p: &Path) -> std::io::Result<PathBuf> {
    let c = std::fs::canonicalize(p)?;
    #[cfg(windows)]
    {
        let s = c.to_string_lossy();
        if let Some(rest) = s.strip_prefix(r"\\?\") {
            if !rest.starts_with("UNC\\") {
                return Ok(PathBuf::from(rest));
            }
        }
    }
    Ok(c)
}
