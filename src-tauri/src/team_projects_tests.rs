//! « Projets de l'équipe » in the core (TS-8): a project here known by the address of its repository whatever its form,
//! the box of TS-13 kept for the projects outside the list, « Cloner » from a local repository served as the hosts'
//! tests serve theirs (`insteadOf`), a project of several repositories that says which one is missing, git's refusals
//! said as « Demande l'accès », and what is refused before git sees it.

use super::*;
use crate::teams::{CatalogOf, MemberView};
use escouade_team::catalog::{Repo, Scheme};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn fresh(name: &str) -> PathBuf {
    let d = crate::paths::test_dir(&format!("team-projects-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A bare repository with one commit on `main`, and the `-c` that makes `url` reach it.
fn served(root: &Path, name: &str, url: &str) -> (PathBuf, String) {
    let work = root.join(format!("{name}-work"));
    std::fs::create_dir_all(&work).unwrap();
    git(&work, &["init", "-q", "-b", "main"]);
    git(&work, &["config", "user.email", "t@example.com"]);
    git(&work, &["config", "user.name", "T"]);
    std::fs::write(work.join("README.md"), format!("# {name}\n")).unwrap();
    git(&work, &["add", "."]);
    git(&work, &["commit", "-q", "-m", "init"]);
    let bare = root.join(format!("{name}.git"));
    git(
        root,
        &[
            "clone",
            "-q",
            "--bare",
            &work.to_string_lossy(),
            &bare.to_string_lossy(),
        ],
    );
    let config = format!(
        "url.{}.insteadOf={url}",
        bare.to_string_lossy().replace('\\', "/")
    );
    (bare, config)
}

fn listed(urls: &[&str]) -> Listed {
    Listed {
        name: "Acme API".into(),
        repos: urls
            .iter()
            .map(|u| Repo {
                url: (*u).into(),
                folder: String::new(),
            })
            .collect(),
        ..Listed::default()
    }
}

fn of(items: Vec<(&str, Listed)>) -> CatalogOf {
    CatalogOf {
        name: "ACME".into(),
        admin: false,
        me: "02".repeat(16),
        members: vec![MemberView {
            id: "02".repeat(16),
            admin: false,
            me: true,
        }],
        catalog: Catalog {
            projects: items
                .into_iter()
                .map(|(id, project)| Item {
                    id: id.into(),
                    project,
                    by: "01".repeat(16),
                    at: 0,
                    seq: 1,
                })
                .collect(),
            ..Catalog::default()
        },
        proposals: Default::default(),
    }
}

#[test]
fn a_project_here_is_known_by_its_repository_and_the_box_stays_for_the_others() {
    let locals = vec![
        Local {
            id: "p-api".into(),
            name: "api (local)".into(),
            team: None,
            keys: vec![catalog::key("git@GitHub.com:acme/api").unwrap()],
            board: None,
        },
        Local {
            id: "p-site".into(),
            name: "site".into(),
            team: Some("acme".into()),
            keys: vec![catalog::key("https://github.com/acme/site").unwrap()],
            board: None,
        },
        Local {
            id: "p-mine".into(),
            name: "mine".into(),
            team: None,
            keys: vec![],
            board: None,
        },
    ];
    let mut web = listed(&["https://github.com/acme/web.git"]);
    web.name = "Web".into();
    let view = combine(
        of(vec![
            ("a1", listed(&["https://github.com/acme/api.git"])),
            ("b2", web),
        ]),
        "acme",
        &locals,
    );
    let api = &view.projects[0];
    let here = api.here.as_ref().expect("the clone here is found");
    assert_eq!(here.project, "p-api");
    assert!(!here.attached, "attached by the member's gesture only");
    assert!(here.name_differs, "its own name stays, said");
    assert_eq!(here.repos, vec![0]);
    assert!(api.open_to_me);
    assert_eq!(api.repos[0].url, "https://github.com/acme/api.git");
    assert_eq!(api.repos[0].forge.as_deref(), Some("GitHub"));
    assert_eq!(api.repos[0].scheme, "https");
    assert!(
        view.projects[1].here.is_none(),
        "Web is not here: « Cloner »"
    );
    // A project of the team here whose repository is not in the list keeps its box.
    assert_eq!(view.outside, vec!["p-site".to_string()]);
    // Kept to other members: not open to this one.
    let mut kept = listed(&["https://github.com/acme/secret"]);
    kept.access = Access::Members {
        members: vec!["01".repeat(16)],
    };
    let view = combine(of(vec![("c3", kept)]), "acme", &[]);
    assert!(!view.projects[0].open_to_me);
}

#[test]
fn gits_refusals_of_access_are_told_apart() {
    for refusal in [
        "remote: Repository not found.\nfatal: repository 'https://github.com/acme/api.git/' not found",
        "fatal: Authentication failed for 'https://gitlab.com/acme/api.git/'",
        "git@github.com: Permission denied (publickey).\nfatal: Could not read from remote repository.",
        "fatal: could not read Username for 'https://github.com': terminal prompts disabled",
        "remote: HTTP Basic: Access denied",
        "fatal: unable to access 'https://dev.azure.com/acme/_git/api/': The requested URL returned error: 403",
    ] {
        assert!(no_access_in(refusal), "{refusal}");
    }
    for other in [
        "fatal: unable to access 'https://git.acme.fr/api/': Could not resolve host: git.acme.fr",
        "fatal: destination path 'api' already exists and is not an empty directory.",
    ] {
        assert!(!no_access_in(other), "{other}");
    }
    let a = catalog::address("git@github.com:acme/api.git").unwrap();
    assert_eq!(a.scheme, Scheme::Ssh);
    let said = ask_access(&a);
    assert!(
        said.contains("acme/api") && said.contains("GitHub"),
        "{said}"
    );
    let a = catalog::address("https://git.acme.fr/acme/api").unwrap();
    assert!(ask_access(&a).contains("git.acme.fr"));
}

#[tokio::test]
async fn cloner_clones_by_the_members_git_and_the_clone_is_known_afterwards() {
    let root = fresh("clone");
    let url = "https://git.example.test/acme/api.git";
    let (_bare, config) = served(&root, "api", url);
    let parent = root.join("dev");
    std::fs::create_dir_all(&parent).unwrap();
    let p = listed(&[url]);
    let done = clone_listed(&p, &parent, "api", std::slice::from_ref(&config))
        .await
        .expect("cloned");
    assert!(done.complete);
    let dest = parent.join("api");
    assert_eq!(done.path, dest.to_string_lossy());
    assert!(dest.join("README.md").exists());
    // The remote keeps the list's address (before insteadOf): the project is known by it afterwards.
    assert_eq!(git(&dest, &["config", "--get", "remote.origin.url"]), url);
    assert_eq!(keys_of(&dest).await, vec![catalog::key(url).unwrap()]);
    // Again: the clone there is kept, not cloned twice.
    let again = clone_listed(&p, &parent, "api", &[config]).await.unwrap();
    assert!(again.complete && again.repos[0].already);
    // Another folder there already is not overwritten.
    std::fs::create_dir_all(parent.join("busy")).unwrap();
    std::fs::write(parent.join("busy").join("x"), "x").unwrap();
    assert!(clone_listed(&p, &parent, "busy", &[]).await.is_err());
    // A name that is not a folder, a parent that is not one.
    assert!(clone_listed(&p, &parent, "../out", &[]).await.is_err());
    assert!(clone_listed(&p, &parent.join("nope"), "api2", &[])
        .await
        .is_err());
}

#[tokio::test]
async fn a_project_of_several_repositories_clones_them_all_and_says_which_is_missing() {
    let root = fresh("several");
    let front = "https://git.example.test/acme/front";
    let gone = "https://git.example.test/acme/gone";
    let (_b, config) = served(&root, "front", front);
    // `gone` reaches a folder that is no repository.
    let missing = root.join("missing.git");
    let config_gone = format!(
        "url.{}.insteadOf={gone}",
        missing.to_string_lossy().replace('\\', "/")
    );
    let parent = root.join("dev");
    std::fs::create_dir_all(&parent).unwrap();
    let p = listed(&[front, gone]);
    let view = clone_listed(&p, &parent, "plateforme", &[config.clone(), config_gone])
        .await
        .unwrap();
    assert!(!view.complete);
    assert!(view.repos[0].done);
    assert!(parent
        .join("plateforme")
        .join("front")
        .join("README.md")
        .exists());
    let miss = &view.repos[1];
    assert!(!miss.done);
    assert_eq!(miss.url, gone);
    assert!(miss.error.is_some() || miss.no_access.is_some());
    assert!(
        !parent.join("plateforme").join("gone").exists(),
        "a clone stopped halfway leaves no folder"
    );
    // « Réessayer » once the access is given: only the missing one is cloned.
    let (_b2, config_now) = served(&root, "gone", gone);
    let view = clone_listed(&p, &parent, "plateforme", &[config, config_now])
        .await
        .unwrap();
    assert!(view.complete);
    assert!(view.repos[0].already && !view.repos[1].already);
}

#[tokio::test]
async fn a_dangerous_address_never_reaches_git() {
    let root = fresh("danger");
    for bad in [
        "file:///c/src/api",
        "--upload-pack=calc.exe",
        "ext::sh%20-c%20id",
        "C:/src/api",
        "https://token@github.com/acme/api",
    ] {
        let err = clone_listed(&listed(&[bad]), &root, "x", &[])
            .await
            .expect_err(bad);
        assert!(!err.is_empty());
    }
    assert!(
        std::fs::read_dir(&root).unwrap().next().is_none(),
        "nothing made"
    );
}

#[test]
fn the_logo_of_the_list_becomes_a_file_for_the_wizard() {
    let dir = fresh("logo");
    let f = logo_file(&dir, "ab12", "data:image/png;base64,iVBORw0KGgo=").unwrap();
    assert_eq!(std::fs::read(&f).unwrap(), b"\x89PNG\r\n\x1a\n");
    assert!(logo_file(&dir, "ab12", "data:image/svg+xml;base64,PHN2Zz4=").is_none());
    // The id goes in a file name: only its hex digits.
    let f = logo_file(&dir, "../../x", "data:image/png;base64,iVBORw0KGgo=").unwrap();
    assert_eq!(f.parent().unwrap(), dir);
}
