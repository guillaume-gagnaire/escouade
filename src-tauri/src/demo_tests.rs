use super::*;
use crate::paths::test_dir;

/// Every path under `dir`, relative, sorted: what is on the disk.
fn tree(dir: &Path) -> Vec<String> {
    fn walk(base: &Path, at: &Path, out: &mut Vec<String>) {
        for e in std::fs::read_dir(at).into_iter().flatten().flatten() {
            let p = e.path();
            out.push(
                p.strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
            if p.is_dir() {
                walk(base, &p, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

fn git_out(dir: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[tokio::test]
async fn the_demo_is_a_repository_of_its_own_under_the_data_folder_and_nothing_else() {
    let base = test_dir("demo-create");
    let root = base.join("data");
    std::fs::create_dir_all(&root).unwrap();
    // Something of the user's beside it, which must not move.
    std::fs::write(root.join("state.json"), "{}").unwrap();
    std::fs::write(base.join("voisin.txt"), "à moi").unwrap();
    let before_outside = tree(&base)
        .into_iter()
        .filter(|p| !p.starts_with("data/demo"))
        .collect::<Vec<_>>();

    let dir = create(&root).await.unwrap();
    assert_eq!(dir, root.join("demo").join(FOLDER));
    assert!(written(&dir));
    // Its files, its tests that fail on purpose, run without anything to install.
    for (rel, body) in files() {
        assert_eq!(std::fs::read_to_string(dir.join(rel)).unwrap(), body);
    }
    let package: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("package.json")).unwrap()).unwrap();
    assert_eq!(package["scripts"]["test"], "node --test test/todo.test.ts");
    assert!(package.get("dependencies").is_none());
    assert!(package.get("devDependencies").is_none());
    // A repository of its own, one commit on main, by Escouade, nothing left to commit.
    assert!(demo_toplevel_is(&dir));
    assert_eq!(git_out(&dir, &["branch", "--show-current"]), "main");
    assert_eq!(git_out(&dir, &["rev-list", "--count", "HEAD"]), "1");
    assert_eq!(git_out(&dir, &["log", "-1", "--format=%an"]), "Escouade");
    assert_eq!(git_out(&dir, &["status", "--porcelain"]), "");
    // No remote: nothing can leave it.
    assert_eq!(git_out(&dir, &["remote"]), "");
    // Nothing written outside its folder.
    let after_outside = tree(&base)
        .into_iter()
        .filter(|p| !p.starts_with("data/demo"))
        .collect::<Vec<_>>();
    assert_eq!(before_outside, after_outside);

    // Made again: kept as it is (a file changed by an agent stays).
    std::fs::write(dir.join("NOTES.md"), "gardé").unwrap();
    assert_eq!(create(&root).await.unwrap(), dir);
    assert!(dir.join("NOTES.md").is_file());
}

/// `dir` is the top of its own repository, not inside another one's.
fn demo_toplevel_is(dir: &Path) -> bool {
    let top = git_out(dir, &["rev-parse", "--show-toplevel"]);
    same(Path::new(&top), dir)
        || std::fs::canonicalize(&top).ok() == std::fs::canonicalize(dir).ok()
}

#[tokio::test]
async fn removing_the_demo_takes_its_folder_and_worktrees_and_touches_nothing_else() {
    let base = test_dir("demo-remove");
    let root = base.join("data");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("state.json"), "{}").unwrap();
    let dir = create(&root).await.unwrap();
    // An agent's worktree, where Escouade makes them (inside the project).
    let wt = dir.join(".claude").join("worktrees").join("agent-1");
    git_out(
        &dir,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "escouade/agent-1",
            &wt.to_string_lossy(),
        ],
    );
    assert!(wt.join("package.json").is_file());

    // Any other folder is refused, and stays.
    let other = base.join("autre");
    std::fs::create_dir_all(&other).unwrap();
    assert!(remove(&root, &other).is_err());
    assert!(remove(&root, &root).is_err());
    assert!(other.is_dir());

    // Its own spelling, with a trailing separator: it goes, all of it.
    let spelled = format!("{}/", dir.to_string_lossy().replace('\\', "/"));
    remove(&root, Path::new(&spelled)).unwrap();
    assert!(!dir.exists());
    assert!(!root.join("demo").exists());
    assert!(root.join("state.json").is_file());
    assert!(other.is_dir());
    // Removed already: nothing to do, no error.
    remove(&root, &dir).unwrap();
}

#[tokio::test]
async fn a_folder_there_that_is_no_demo_is_refused_and_left_alone() {
    let base = test_dir("demo-foreign");
    let root = base.join("data");
    let dir = super::dir(&root);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("mien.txt"), "à moi").unwrap();
    let err = create(&root).await.unwrap_err().to_string();
    assert!(
        err.contains("n’est pas le projet de démonstration"),
        "{err}"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("mien.txt")).unwrap(),
        "à moi"
    );
    assert!(!dir.join(".git").exists());
}

#[test]
fn the_demo_is_known_by_its_folder_whatever_the_spelling() {
    let root = Path::new("C:/Users/ada/.escouade");
    assert!(is_demo(
        root,
        Path::new("C:/Users/ada/.escouade/demo/bac-a-sable")
    ));
    assert!(is_demo(
        root,
        Path::new("C:\\Users\\ada\\.escouade\\demo\\bac-a-sable\\")
    ));
    assert!(!is_demo(root, Path::new("C:/Users/ada/.escouade/demo")));
    assert!(!is_demo(root, Path::new("C:/Users/ada/dev/bac-a-sable")));
}
