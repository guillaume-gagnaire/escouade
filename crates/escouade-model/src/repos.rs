//! The folder of a project's repository (`repos::join`).

use std::path::PathBuf;

/// The folder `repo` (from the project's, with `/`) of the folder `base` (the project's, or an
/// agent's worktrees folder).
pub fn join(base: &str, repo: &str) -> String {
    if repo.is_empty() {
        return base.to_string();
    }
    let mut p = PathBuf::from(base);
    for part in repo.split('/') {
        p.push(part);
    }
    p.to_string_lossy().into_owned()
}
