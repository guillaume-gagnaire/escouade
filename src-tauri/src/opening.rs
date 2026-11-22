//! What the system may open for the window (audit SEC-12, ESC-593): a file in its program, a folder in the file manager,
//! an item shown in its folder, only inside the folders of the projects (their worktrees, their agents' folders) and
//! Escouade's data folder. The path is judged where it really is: a link, a junction, `..`, a verbatim spelling
//! (`\\?\C:\…`) are resolved before; a network path (`\\server\share`) not inside one of these folders as written is
//! refused before the disk is asked, as asking would already reach the server. Elsewhere, refused with its reason.
//!
//! A link (http, https…) keeps its own way (`commands::open_url`). The refusal of programs and scripts sent by an agent
//! (ESC-171) stays with those who open them (`sentfiles`, `asks`), before this one.
//!
//! Only an `Allowed` is opened (`outside::open_allowed`), and only `allow` makes one: no path reaches the system's opener
//! without passing here. The window has no permission of Tauri's opener for paths (`capabilities/default.json`).

use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Result};

use crate::outside::What;

/// A path `allow` let through: opened in its program or the file manager, or shown in its folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allowed {
    path: PathBuf,
    /// Where it really is (its folder resolved and its name kept, when shown in it).
    real: PathBuf,
    reveal: bool,
}

impl Allowed {
    /// What the system is asked: the path as it was given (a link of a project shown as the link).
    pub fn what(&self) -> What<'_> {
        if self.reveal {
            What::Reveal(&self.path)
        } else {
            What::Path(&self.path)
        }
    }

    /// Where the path really is: what it leads to when opened, its folder resolved when shown in it.
    pub fn real(&self) -> &Path {
        &self.real
    }

    /// Opens it (`outside::open_allowed`).
    pub fn open(&self) -> Result<()> {
        crate::outside::open_allowed(self)
    }
}

/// `path` let through when it is inside one of `roots` (the folders of the projects and their worktrees, the data
/// folder: `Core::open_roots`), where it really is. `reveal`: shown selected in its folder, which is then the one
/// judged (a link of a project is shown, not followed); else opened, the file or folder it leads to judged.
pub fn allow(roots: &[PathBuf], path: &Path, reveal: bool) -> Result<Allowed> {
    let said = shown(path);
    if !path.is_absolute() {
        bail!(tr!(
            "« {said} » n’est pas un chemin complet : Escouade ne le fait pas ouvrir par le système.",
            "“{said}” is not a full path: Escouade doesn’t have the system open it."
        ));
    }
    let plain = unverbatim(path);
    if cfg!(windows) && device(&plain) {
        bail!(tr!(
            "« {said} » est un périphérique de Windows : Escouade ne le fait pas ouvrir par le système.",
            "“{said}” is a device of Windows: Escouade doesn’t have the system open it."
        ));
    }
    // A network path is asked of the disk only inside a folder of the projects as written (a project under WSL,
    // `\\wsl.localhost\…`): asking another one would already reach its server, with the user's credentials.
    if cfg!(windows) && network(&plain) && !roots.iter().any(|r| under(&plain, &unverbatim(r))) {
        return Err(refused(&said));
    }
    let missing = || {
        anyhow::anyhow!(tr!(
            "« {said} » n’existe pas ou ne se lit pas : Escouade ne le fait pas ouvrir par le système.",
            "“{said}” doesn’t exist or can’t be read: Escouade doesn’t have the system open it."
        ))
    };
    // Shown in its folder, the item is not followed (a link of a project is shown as the link): its folder is resolved,
    // its name kept. Opened, it is what the path leads to that is judged.
    let real = if reveal {
        let (Some(dir), Some(name)) = (path.parent(), path.file_name()) else {
            bail!(tr!(
                "« {said} » n’a pas de dossier où le montrer.",
                "“{said}” has no folder to be shown in."
            ));
        };
        if std::fs::symlink_metadata(path).is_err() {
            return Err(missing());
        }
        std::fs::canonicalize(dir)
            .map_err(|_| missing())?
            .join(name)
    } else {
        std::fs::canonicalize(path).map_err(|_| missing())?
    };
    // The roots that hold the path as written first: the one that holds it is usually among them, and the others
    // (a project under WSL wakes its distribution up) are asked only when it is not.
    let mut order: Vec<&PathBuf> = roots.iter().collect();
    order.sort_by_key(|r| !under(&plain, &unverbatim(r)));
    for root in order {
        if std::fs::canonicalize(root).is_ok_and(|r| under(&real, &r)) {
            return Ok(Allowed {
                path: path.to_path_buf(),
                real: real.clone(),
                reveal,
            });
        }
    }
    let real = shown(&real);
    if !same(&real, &said) {
        bail!(tr!(
            "« {said} » mène à « {real} », hors des dossiers des projets, de leurs worktrees et du dossier de données d’Escouade : Escouade ne le fait pas ouvrir par le système.",
            "“{said}” leads to “{real}”, outside the folders of the projects, their worktrees and Escouade’s data folder: Escouade doesn’t have the system open it."
        ));
    }
    Err(refused(&said))
}

/// The refusal of `said`, outside the folders.
fn refused(said: &str) -> anyhow::Error {
    anyhow::anyhow!(tr!(
        "« {said} » est hors des dossiers des projets, de leurs worktrees et du dossier de données d’Escouade : Escouade ne le fait pas ouvrir par le système.",
        "“{said}” is outside the folders of the projects, their worktrees and Escouade’s data folder: Escouade doesn’t have the system open it."
    ))
}

/// `path` without its verbatim prefix of Windows: `\\?\C:\a` is `C:\a`, `\\?\UNC\server\share` is
/// `\\server\share`; `\\.\C:\a` is `C:\a` too. The rest as it is.
fn unverbatim(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    let rest = |p: &str| {
        s.get(p.len()..)
            .filter(|_| s.starts_with(p))
            .map(str::to_string)
    };
    if let Some(r) = rest(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{r}"));
    }
    for prefix in [r"\\?\", r"\\.\"] {
        if let Some(r) = rest(prefix).filter(|r| drive(r)) {
            return PathBuf::from(r);
        }
    }
    path.to_path_buf()
}

/// Whether `s` starts with a drive (`C:`).
fn drive(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':'
}

/// A path of Windows' devices (`\\.\pipe\…`, `\\.\PhysicalDrive0`, `\\?\GLOBALROOT\…`) once `unverbatim`
/// has turned the drives' back into plain paths.
fn device(plain: &Path) -> bool {
    let s = plain.to_string_lossy();
    s.starts_with(r"\\.\")
        || s.starts_with(r"\\?\")
        || s.starts_with("//./")
        || s.starts_with("//?/")
}

/// A network path (`\\server\share`, `//server/share`).
fn network(plain: &Path) -> bool {
    !escouade_base::paths::safe_origin(&plain.to_string_lossy())
}

/// Whether `path` is `root` or inside it, name by name: case ignored under Windows and macOS (as their disks do), `.`
/// and a last separator ignored. Never resolves anything: `..` is a name like any other here, which the canonical
/// paths compared do not hold.
fn under(path: &Path, root: &Path) -> bool {
    let names = |p: &Path| -> Vec<String> {
        p.components()
            .filter(|c| !matches!(c, Component::CurDir))
            .map(|c| {
                let s = c.as_os_str().to_string_lossy().replace('/', "\\");
                if cfg!(any(windows, target_os = "macos")) {
                    s.to_lowercase()
                } else {
                    s
                }
            })
            .collect()
    };
    let (p, r) = (names(path), names(root));
    !r.is_empty() && p.len() >= r.len() && p[..r.len()] == r[..]
}

/// Whether two paths shown are the same, case ignored where the disk ignores it.
fn same(a: &str, b: &str) -> bool {
    if cfg!(any(windows, target_os = "macos")) {
        a.to_lowercase() == b.to_lowercase()
    } else {
        a == b
    }
}

/// `path` as the user reads it: without the verbatim prefix `canonicalize` gives under Windows.
fn shown(path: &Path) -> String {
    unverbatim(path).display().to_string()
}

impl<R: tauri::Runtime> crate::core::Core<R> {
    /// The folders the system may open in (`allow`): Escouade's data folder, and of every project its folder, the folder
    /// of its new worktrees, and the folder and worktree of each of its agents (a worktree made before the « Dossier
    /// des worktrees » changed stays where it is).
    pub(crate) fn open_roots(&self) -> Vec<PathBuf> {
        let mut roots = vec![self.data.root().to_path_buf()];
        for p in self.projects.read().iter() {
            roots.push(PathBuf::from(&p.path));
            roots.push(p.worktrees_root());
        }
        // Read apart: an agent's lock is never held with the list's.
        let handles: Vec<crate::agent::AgentHandle> =
            self.agents.read().values().cloned().collect();
        for h in handles {
            let m = &h.lock().meta;
            if !m.cwd.is_empty() {
                roots.push(PathBuf::from(&m.cwd));
            }
            if let Some(w) = &m.worktree {
                roots.push(PathBuf::from(&w.path));
            }
        }
        roots.retain(|r| r.is_absolute());
        roots.dedup();
        roots
    }

    /// `path` let through for the system (`allow`), against the folders as they are now (`open_roots`).
    pub(crate) fn allow_open(&self, path: &Path, reveal: bool) -> Result<Allowed> {
        allow(&self.open_roots(), path, reveal)
    }
}
