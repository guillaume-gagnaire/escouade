//! The repositories of a project's folder (ESC-169): what `repos::detect` finds, and the paths
//! said from the project's folder.

use crate::core_tests::git;
use crate::paths::test_dir;
use crate::repos::{detect, join, label, prefixed, split};
use std::path::Path;

fn init(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    git(dir, &["init", "-q", "-b", "main"]);
}

#[tokio::test]
async fn a_folder_of_repositories_gives_them_two_levels_down_and_skips_the_rest() {
    let d = test_dir("repos-detect");
    for r in ["front", "api", "services/billing", "deep/er/still"] {
        init(&d.join(r));
    }
    // Never looked into: hidden, build and package folders, the worktrees folder.
    for r in [".hidden/x", "node_modules/pkg", "target/r", "wt/agent"] {
        init(&d.join(r));
    }
    std::fs::create_dir_all(d.join("docs")).unwrap();
    // Inside a repository found: its own business.
    init(&d.join("api").join("inner"));
    let found = detect(&d, &d.join("wt")).await;
    assert_eq!(found, ["api", "front", "services/billing"]);
}

#[tokio::test]
async fn a_folder_of_one_repository_or_none_is_no_project_of_several() {
    let d = test_dir("repos-one");
    assert!(detect(&d, &d.join(".claude/worktrees")).await.is_empty());
    init(&d);
    std::fs::create_dir_all(d.join("src")).unwrap();
    assert!(detect(&d, &d.join(".claude/worktrees")).await.is_empty());
}

#[tokio::test]
async fn a_root_repository_holding_others_comes_first_its_submodules_and_ignored_folders_left_out()
{
    let d = test_dir("repos-root");
    init(&d);
    init(&d.join("front"));
    init(&d.join("lib"));
    std::fs::write(
        d.join(".gitmodules"),
        "[submodule \"lib\"]\n\tpath = lib\n\turl = ../lib\n",
    )
    .unwrap();
    // Ignored by the root: not looked into, unless it is a repository itself.
    std::fs::write(d.join(".gitignore"), "cache/\nback/\n").unwrap();
    init(&d.join("cache").join("x"));
    init(&d.join("back"));
    let found = detect(&d, &d.join(".claude/worktrees")).await;
    assert_eq!(found, ["", "back", "front"]);
}

#[test]
fn paths_go_to_their_repository_and_back() {
    let repos: Vec<String> = ["", "api", "services/billing"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(split(&repos, "api/src/a.ts"), Some(("api", "src/a.ts")));
    assert_eq!(
        split(&repos, "services/billing/x"),
        Some(("services/billing", "x"))
    );
    assert_eq!(split(&repos, "README.md"), Some(("", "README.md")));
    assert_eq!(split(&repos[1..], "README.md"), None);
    assert_eq!(split(&repos[1..], "apiary/x"), None);
    assert_eq!(prefixed("api", "src/a.ts"), "api/src/a.ts");
    assert_eq!(prefixed("", "a.ts"), "a.ts");
    assert_eq!(label(""), tr!("racine", "root"));
    assert_eq!(
        Path::new(&join("base", "services/billing")),
        Path::new("base").join("services").join("billing")
    );
}

/// ESC-653: `.gitmodules` comes with the repository: a link there (to `/dev/zero` under Unix, a
/// junction under Windows) is not read, and the folder is found all the same.
#[tokio::test]
async fn a_gitmodules_that_is_a_link_is_not_read() {
    let d = test_dir("repos-gitmodules-link");
    init(&d);
    init(&d.join("lib"));
    let outside = test_dir("repos-gitmodules-outside");
    std::fs::write(
        outside.join("modules"),
        "[submodule \"lib\"]\n\tpath = lib\n\turl = ../lib\n",
    )
    .unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("/dev/zero", d.join(".gitmodules")).unwrap();
    #[cfg(windows)]
    assert!(crate::paths::make_dir_link(
        &outside,
        &d.join(".gitmodules")
    ));
    assert_eq!(detect(&d, &d.join("wt")).await, ["", "lib"]);
    #[cfg(windows)]
    std::fs::remove_dir(d.join(".gitmodules")).unwrap();
    #[cfg(unix)]
    std::fs::remove_file(d.join(".gitmodules")).unwrap();
    if crate::paths::make_file_link(&outside.join("modules"), &d.join(".gitmodules")) {
        assert_eq!(detect(&d, &d.join("wt")).await, ["", "lib"]);
    }
}
