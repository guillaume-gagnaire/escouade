//! « Un dossier du serveur » and « Cloner un dépôt » (ESC-538, `docs/SERVER-UX.md` § 4 and 9): the folders of the
//! core's machine, for the assistant « Nouveau projet » of a client on a server, whose system picker would show this
//! computer's folders, not the server's. A listing says, for each folder, whether it is a repository and its branch,
//! whether it is a project already, and the room left on its disk.
//!
//! Bounded to what the server allows: folders only (never a file's name nor its content), the hidden ones left out, at
//! most [`MAX_ENTRIES`] of them, an absolute path that is a folder the server's account can read. A paired device has
//! the server's shell already (its terminals): the listing gives it nothing more, only a picker.

use anyhow::{bail, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// The most folders a listing gives: what is past is counted (`more`), not listed.
pub const MAX_ENTRIES: usize = 500;

/// A folder listed.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub name: String,
    pub path: String,
    /// A git repository (`.git` there, a folder or a worktree's file).
    pub repo: bool,
    /// Its branch (or the start of its commit, detached), when it is a repository.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// The name of the project it is already (or that it holds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
}

/// A folder of the server, as « Un dossier du serveur » shows it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    pub path: String,
    /// The folder above, none at the root.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// The home folder of the server's account.
    pub home: String,
    /// Whether this folder is a repository itself, its branch, a project already.
    pub repo: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    pub entries: Vec<Entry>,
    /// The folders past `MAX_ENTRIES`, not listed.
    pub more: usize,
    /// The room left and the size of its disk, in bytes, when the system says.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub free: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
}

/// Where the browser opens without a path: `~/projets` (or another usual folder of projects), else the home folder.
pub fn start(home: &Path) -> PathBuf {
    for c in ["projets", "projects", "dev", "src", "code"] {
        let p = home.join(c);
        if p.is_dir() {
            return p;
        }
    }
    home.to_path_buf()
}

/// Two paths of the same folder, as written by a user or a project: separators and a last slash aside, and the case
/// under Windows.
pub fn same_folder(a: &str, b: &str) -> bool {
    let norm = |s: &str| {
        let s = s.trim().replace('\\', "/");
        let s = s.trim_end_matches('/').to_string();
        if cfg!(windows) {
            s.to_lowercase()
        } else {
            s
        }
    };
    let (a, b) = (norm(a), norm(b));
    !a.is_empty() && a == b
}

/// The branch of the repository `dir` (`.git` a folder, or the file of a worktree), or the start of its commit when
/// detached; none when it is no repository or its `HEAD` cannot be read.
pub fn branch_of(dir: &Path) -> Option<String> {
    let dot = dir.join(".git");
    let git = if dot.is_file() {
        let text = std::fs::read_to_string(&dot).ok()?;
        let to = text.lines().find_map(|l| l.strip_prefix("gitdir:"))?.trim();
        let to = PathBuf::from(to);
        if to.is_absolute() {
            to
        } else {
            dir.join(to)
        }
    } else {
        dot
    };
    let head = std::fs::read_to_string(git.join("HEAD")).ok()?;
    let head = head.trim();
    match head.strip_prefix("ref:") {
        Some(r) => Some(r.trim().trim_start_matches("refs/heads/").to_string()),
        None if head.len() >= 7 && head.chars().all(|c| c.is_ascii_hexdigit()) => {
            Some(head[..7].to_string())
        }
        None => None,
    }
}

fn is_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
}

/// The folder `path` of the server listed. `projects`: the paths and names of the core's projects.
pub fn list(path: &Path, home: &Path, projects: &[(String, String)]) -> Result<Listing> {
    let lang = crate::i18n::ui();
    let shown = path.display().to_string();
    if !path.is_absolute() {
        bail!(tr_in!(
            lang,
            "« {shown} » n’est pas un chemin absolu.",
            "“{shown}” isn’t an absolute path."
        ));
    }
    if !path.is_dir() {
        bail!(tr_in!(
            lang,
            "« {shown} » n’est pas un dossier du serveur.",
            "“{shown}” isn’t a folder of the server."
        ));
    }
    let project_at = |p: &str| {
        projects
            .iter()
            .find(|(at, _)| same_folder(at, p))
            .map(|(_, name)| name.clone())
    };
    let read = std::fs::read_dir(path).map_err(|e| {
        anyhow::anyhow!(tr_in!(
            lang,
            "Impossible de lire « {shown} » : {e}",
            "Can’t read “{shown}”: {e}"
        ))
    })?;
    let mut entries: Vec<Entry> = Vec::new();
    let mut seen = 0usize;
    for e in read.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let p = e.path();
        // A link to a folder is one (`is_dir` follows it).
        if !p.is_dir() {
            continue;
        }
        seen += 1;
        if entries.len() >= MAX_ENTRIES {
            continue;
        }
        let repo = is_repo(&p);
        let ps = p.to_string_lossy().to_string();
        entries.push(Entry {
            branch: repo.then(|| branch_of(&p)).flatten(),
            project: project_at(&ps),
            name,
            path: ps,
            repo,
        });
    }
    entries.sort_by_key(|e| e.name.to_lowercase());
    let repo = is_repo(path);
    let (free, total) = disk_space(path).unzip();
    Ok(Listing {
        path: shown.clone(),
        parent: path.parent().map(|p| p.display().to_string()),
        home: home.display().to_string(),
        repo,
        branch: repo.then(|| branch_of(path)).flatten(),
        project: project_at(&shown),
        more: seen.saturating_sub(entries.len()),
        entries,
        free,
        total,
    })
}

/// The room left and the size of the disk of `path`, in bytes.
#[cfg(unix)]
pub fn disk_space(path: &Path) -> Option<(u64, u64)> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    // SAFETY: a NUL-terminated path and a zeroed struct the call fills.
    unsafe {
        let mut s: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(c.as_ptr(), &mut s) != 0 {
            return None;
        }
        let unit = s.f_frsize as u64;
        Some((s.f_bavail as u64 * unit, s.f_blocks as u64 * unit))
    }
}

#[cfg(windows)]
pub fn disk_space(path: &Path) -> Option<(u64, u64)> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let (mut free, mut total, mut all_free) = (0u64, 0u64, 0u64);
    // SAFETY: a NUL-terminated path and three numbers the call fills.
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, &mut total, &mut all_free) };
    (ok != 0).then_some((free, total))
}

#[cfg(not(any(unix, windows)))]
pub fn disk_space(_path: &Path) -> Option<(u64, u64)> {
    None
}

/// The folder a clone of `url` goes in, as git names it: the last part of its path, without `.git`.
pub fn folder_of(url: &str) -> String {
    let url = url.trim().trim_end_matches('/');
    let last = url.rsplit(['/', ':']).next().unwrap_or("");
    last.strip_suffix(".git").unwrap_or(last).to_string()
}

/// « Cloner un dépôt »: `url` (https or ssh only, checked as a team's list is: never a local path, `file://`, an option
/// of git or a credential) cloned into `parent/folder` (none: the one git would name) on the core's machine, by its
/// account's git. The folder made.
/// `config` is given to git with `-c` (the tests' `insteadOf`).
pub async fn clone(url: &str, parent: &Path, folder: &str, config: &[String]) -> Result<String> {
    let lang = crate::i18n::ui();
    let address = escouade_team::catalog::address(url).map_err(|e| {
        anyhow::anyhow!(tr_in!(
            lang,
            "Cette adresse ne se clone pas : {e}",
            "This address can’t be cloned: {e}"
        ))
    })?;
    // No folder said: the one git would make.
    let named = folder_of(url);
    let folder = match folder.trim() {
        "" => named.as_str(),
        f => f,
    };
    if !crate::team_projects::folder_ok(folder) {
        bail!(tr_in!(
            lang,
            "« {folder} » n’est pas un nom de dossier.",
            "“{folder}” is not a folder name."
        ));
    }
    if !parent.is_dir() {
        let p = parent.display();
        bail!(tr_in!(
            lang,
            "{p} n’est pas un dossier.",
            "{p} is not a folder."
        ));
    }
    let dest = parent.join(folder);
    if dest.exists() && std::fs::read_dir(&dest).is_ok_and(|mut d| d.next().is_some()) {
        let p = dest.display();
        bail!(tr_in!(
            lang,
            "{p} existe déjà : choisis un autre dossier.",
            "{p} already exists: choose another folder."
        ));
    }
    match crate::team_projects::clone_one(&address, "", &dest, config).await {
        Ok(()) => Ok(dest.to_string_lossy().to_string()),
        Err((no_access, why)) if no_access => bail!(tr_in!(
            lang,
            "Le serveur n’a pas accès à ce dépôt (ses clés, ses identifiants git) : {why}",
            "The server has no access to this repository (its keys, its git credentials): {why}"
        )),
        Err((_, why)) => bail!(tr_in!(
            lang,
            "Le clone a échoué : {why}",
            "The clone failed: {why}"
        )),
    }
}
