//! Where a language server of the catalog is (ESC-446): the path set by hand first, then the one the project's
//! `.escouade.yml` gives, then for each of its programs the `node_modules/.bin` of the worktree and of the project, the
//! `PATH`, and the place where its tool installs it (`Standard`). The places are given (`Places`), so that the tests
//! lay out their own.

use std::path::{Component, Path, PathBuf};

use serde::Serialize;

use super::catalog::{Server, Standard};

/// Where the program found comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Origin {
    /// The path set in the app's settings.
    Manual,
    /// The path `language_servers.<id>.path` of the project's `.escouade.yml`: a file of the repository.
    ProjectFile,
    /// A `node_modules/.bin` of the worktree or of the project: a file of the repository, or that its install put.
    NodeModules,
    Path,
    Standard,
}

impl Origin {
    /// A program the repository chose: it runs only once the user trusted the project (a cloned repository can hold a
    /// false server).
    pub fn of_project(self) -> bool {
        matches!(self, Origin::ProjectFile | Origin::NodeModules)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    pub program: PathBuf,
    pub origin: Origin,
}

/// Why no program is found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum NotFound {
    /// The path set by hand names no program: said as such, never another found in its place.
    Manual { path: String },
    /// The path of `.escouade.yml` is absolute, leaves the repository, or names no file.
    ProjectFile { path: String },
    /// Nowhere: the command that installs it.
    Absent { install: String },
}

/// The places to look in.
pub struct Places<'a> {
    /// The folders of the `PATH`, in order.
    pub path: Vec<PathBuf>,
    /// The root of the worktree, then the project's: their `node_modules/.bin`, and what `.escouade.yml` names.
    pub roots: Vec<PathBuf>,
    pub home: Option<PathBuf>,
    /// Windows' `%APPDATA%`.
    pub appdata: Option<PathBuf>,
    /// The output of a tool (`rustup which rust-analyzer`, `go env GOPATH`), trimmed; None when it fails.
    pub run: &'a (dyn Fn(&str, &[&str]) -> Option<String> + Sync),
}

impl Places<'static> {
    /// The machine's places, for `roots`.
    pub fn system(roots: Vec<PathBuf>) -> Self {
        Places {
            path: std::env::var_os("PATH")
                .map(|p| std::env::split_paths(&p).collect())
                .unwrap_or_default(),
            roots,
            home: dirs::home_dir(),
            appdata: std::env::var_os("APPDATA").map(PathBuf::from),
            run: &run_tool,
        }
    }
}

/// Runs `program args`, its standard output trimmed, without a window; None when it fails or says nothing.
fn run_tool(program: &str, args: &[&str]) -> Option<String> {
    let mut cmd = std::process::Command::new(program);
    cmd.args(args)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
    }
    let out = cmd.output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !text.is_empty()).then_some(text)
}

/// The endings a program has on this system, tried in order (`PATHEXT`'s usual ones on Windows, where a
/// `node_modules/.bin` holds `.cmd` shims).
fn endings() -> &'static [&'static str] {
    if cfg!(windows) {
        &[".exe", ".cmd", ".bat", ".com"]
    } else {
        &[""]
    }
}

/// The program `name` of `dir`, when there is one.
pub fn program_in(dir: &Path, name: &str) -> Option<PathBuf> {
    endings()
        .iter()
        .map(|e| dir.join(format!("{name}{e}")))
        .find(|p| p.is_file())
}

/// The program `name` on `path`.
fn on_path(path: &[PathBuf], name: &str) -> Option<PathBuf> {
    path.iter().find_map(|d| program_in(d, name))
}

/// A path set by hand: an absolute path to a file, or the name of a program of the `PATH`.
fn manual(path: &str, places: &Places) -> Option<PathBuf> {
    let p = Path::new(path);
    if p.is_absolute() {
        return p.is_file().then(|| p.to_path_buf());
    }
    if p.components().count() == 1 {
        return on_path(&places.path, path);
    }
    None
}

/// A path of `.escouade.yml`: relative to a root, never out of it.
fn of_file(path: &str, places: &Places) -> Option<PathBuf> {
    let p = Path::new(path);
    if p.is_absolute()
        || p.components()
            .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
    {
        return None;
    }
    places.roots.iter().find_map(|r| {
        let full = r.join(p);
        if full.is_file() {
            return Some(full);
        }
        // Without its ending (`node_modules/.bin/x`), as a file written for all the systems names it.
        let (dir, name) = (full.parent()?, full.file_name()?.to_str()?);
        program_in(dir, name)
    })
}

/// The program `name` where `standard` puts it.
fn standard(standard: Standard, name: &str, places: &Places) -> Option<PathBuf> {
    match standard {
        Standard::Rustup => (places.run)("rustup", &["which", name])
            .map(PathBuf::from)
            .filter(|p| p.is_file()),
        Standard::CargoBin => program_in(&places.home.as_ref()?.join(".cargo").join("bin"), name),
        Standard::GoBin => {
            let gopath = (places.run)("go", &["env", "GOPATH"])
                .and_then(|g| std::env::split_paths(&g).next())
                .or_else(|| places.home.as_ref().map(|h| h.join("go")))?;
            program_in(&gopath.join("bin"), name)
        }
        Standard::PythonUser => {
            let local = places
                .home
                .as_ref()
                .and_then(|h| program_in(&h.join(".local").join("bin"), name));
            local.or_else(|| {
                // `%APPDATA%\Python\Python313\Scripts`, the newest first.
                let base = places.appdata.as_ref()?.join("Python");
                let mut dirs: Vec<PathBuf> = std::fs::read_dir(base)
                    .ok()?
                    .flatten()
                    .map(|e| e.path().join("Scripts"))
                    .collect();
                dirs.sort();
                dirs.iter().rev().find_map(|d| program_in(d, name))
            })
        }
    }
}

/// Finds `server`: `manual` (the app's settings, empty for none) wins, then `file` (`.escouade.yml`'s), then its
/// programs, each in the `node_modules/.bin` of the roots (an npm server), on the `PATH`, then where its tool puts it.
pub fn find(
    server: &Server,
    manual_path: &str,
    file: Option<&str>,
    places: &Places,
) -> Result<Found, NotFound> {
    let manual_path = manual_path.trim();
    if !manual_path.is_empty() {
        return manual(manual_path, places)
            .map(|program| Found {
                program,
                origin: Origin::Manual,
            })
            .ok_or_else(|| NotFound::Manual {
                path: manual_path.to_string(),
            });
    }
    if let Some(path) = file.map(str::trim).filter(|p| !p.is_empty()) {
        return of_file(path, places)
            .map(|program| Found {
                program,
                origin: Origin::ProjectFile,
            })
            .ok_or_else(|| NotFound::ProjectFile {
                path: path.to_string(),
            });
    }
    for name in server.programs {
        if server.node {
            let local = places
                .roots
                .iter()
                .find_map(|r| program_in(&r.join("node_modules").join(".bin"), name));
            if let Some(program) = local {
                return Ok(Found {
                    program,
                    origin: Origin::NodeModules,
                });
            }
        }
        if let Some(program) = on_path(&places.path, name) {
            return Ok(Found {
                program,
                origin: Origin::Path,
            });
        }
        if let Some(program) = server
            .standard
            .iter()
            .find_map(|s| standard(*s, name, places))
        {
            return Ok(Found {
                program,
                origin: Origin::Standard,
            });
        }
    }
    Err(NotFound::Absent {
        install: server.install.to_string(),
    })
}
