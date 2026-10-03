//! Finds a program on the PATH as Windows does (PATHEXT): the GitHub CLI for pull requests.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// `program` found on the PATH, or None when it is not installed.
pub fn find(program: &str) -> Option<PathBuf> {
    resolve_program(program, std::env::var_os("PATH").as_deref(), &pathext())
}

/// Resolves a bare program name through `path_var` and the `pathext` extensions
/// (`gh` → `…\GitHub CLI\gh.exe`). Paths are returned as is when they exist. With extensions to
/// try, a name without one is never taken bare: that is a shell script next to the launcher, which
/// Windows cannot start.
pub fn resolve_program(program: &str, path_var: Option<&OsStr>, pathext: &str) -> Option<PathBuf> {
    let p = Path::new(program);
    if p.components().count() > 1 || p.is_absolute() {
        return p.is_file().then(|| p.to_path_buf());
    }
    let bare = pathext.split(';').all(|e| e.is_empty()) || p.extension().is_some();
    let exts: Vec<String> = bare
        .then(String::new)
        .into_iter()
        .chain(
            pathext
                .split(';')
                .filter(|e| !e.is_empty())
                .map(|e| e.to_lowercase()),
        )
        .collect();
    for dir in std::env::split_paths(path_var?) {
        for ext in &exts {
            let candidate = dir.join(format!("{program}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// The extensions Windows tries for a bare program name; none elsewhere.
fn pathext() -> String {
    if cfg!(windows) {
        std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into())
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_a_bare_name_through_the_path_and_its_extensions() {
        let dir = crate::paths::test_dir("which-resolve");
        std::fs::write(dir.join("gh.cmd"), "@echo off").unwrap();
        let path = std::env::join_paths([dir.join("nowhere"), dir.clone()]).unwrap();
        let found = resolve_program("gh", Some(path.as_os_str()), ".COM;.EXE;.BAT;.CMD").unwrap();
        assert_eq!(found, dir.join("gh.cmd"));
        assert_eq!(
            resolve_program("missing-tool", Some(path.as_os_str()), ".EXE;.CMD"),
            None
        );
        let explicit = dir.join("gh.cmd");
        assert_eq!(
            resolve_program(&explicit.to_string_lossy(), None, ""),
            Some(explicit)
        );
    }

    #[test]
    fn skips_the_extensionless_scripts_next_to_windows_launchers() {
        let dir = crate::paths::test_dir("which-scripts");
        std::fs::write(dir.join("gh"), "#!/usr/bin/env sh").unwrap();
        std::fs::write(dir.join("gh.cmd"), "@echo off").unwrap();
        let path = std::env::join_paths([dir.clone()]).unwrap();
        assert_eq!(
            resolve_program("gh", Some(path.as_os_str()), ".EXE;.CMD"),
            Some(dir.join("gh.cmd"))
        );
        // Without PATHEXT (Unix), the bare name is the program.
        assert_eq!(
            resolve_program("gh", Some(path.as_os_str()), ""),
            Some(dir.join("gh"))
        );
        // A name given with its extension is taken as is.
        assert_eq!(
            resolve_program("gh.cmd", Some(path.as_os_str()), ".EXE;.CMD"),
            Some(dir.join("gh.cmd"))
        );
    }
}
