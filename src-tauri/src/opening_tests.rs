//! The system opens only inside the folders of the projects, their worktrees and the data folder (`opening`, audit
//! SEC-12, ESC-593): each opening let through and refused, links and twisted paths included. No test here opens
//! anything: the ones let through are judged by `allow` alone, the system is asked only of what is refused.

use crate::core_tests::harness;
use crate::opening::allow;
use crate::paths::test_dir;
use std::path::{Path, PathBuf};

/// `path` canonical, as a refusal names it: its long names (`C:\Users\RUNNER~1` is `runneradmin` on the CI), links
/// resolved, without the verbatim prefix of Windows.
fn real(path: &Path) -> String {
    let p = std::fs::canonicalize(path).unwrap().display().to_string();
    p.strip_prefix(r"\\?\").map(str::to_string).unwrap_or(p)
}

/// A link to the folder `target` at `link`: a junction under Windows (it needs no privilege), a symbolic link
/// elsewhere. It goes when it is dropped, and only it.
struct DirLink(PathBuf);

fn dir_link(link: &Path, target: &Path) -> DirLink {
    #[cfg(windows)]
    {
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .unwrap();
        assert!(made.status.success(), "mklink: {made:?}");
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
    DirLink(link.to_path_buf())
}

impl Drop for DirLink {
    fn drop(&mut self) {
        // A junction is removed as a folder, a symbolic link as a file: never what they lead to.
        let _ = std::fs::remove_dir(&self.0).or_else(|_| std::fs::remove_file(&self.0));
    }
}

/// A project, a worktree of it elsewhere, the data folder and a folder outside them all, with a file in each.
struct Ground {
    _dir: PathBuf,
    project: PathBuf,
    worktree: PathBuf,
    data: PathBuf,
    outside: PathBuf,
}

impl Ground {
    fn new(name: &str) -> Self {
        let dir = test_dir(name);
        let g = Ground {
            project: dir.join("proj"),
            worktree: dir.join("proj-worktrees").join("ada"),
            data: dir.join("data"),
            outside: dir.join("elsewhere"),
            _dir: dir,
        };
        for d in [&g.project, &g.worktree, &g.data, &g.outside] {
            std::fs::create_dir_all(d.join("sub")).unwrap();
            std::fs::write(d.join("notes.txt"), "x").unwrap();
        }
        g
    }

    fn roots(&self) -> Vec<PathBuf> {
        vec![
            self.data.clone(),
            self.project.clone(),
            self.worktree.clone(),
        ]
    }

    fn allowed(&self, path: &Path, reveal: bool) -> bool {
        allow(&self.roots(), path, reveal).is_ok()
    }

    fn refusal(&self, path: &Path, reveal: bool) -> String {
        format!("{:#}", allow(&self.roots(), path, reveal).unwrap_err())
    }
}

#[test]
fn the_folders_of_the_projects_their_worktrees_and_the_data_folder_open_and_what_is_in_them() {
    let g = Ground::new("opening-inside");
    for root in [&g.project, &g.worktree, &g.data] {
        for path in [root.clone(), root.join("sub"), root.join("notes.txt")] {
            assert!(g.allowed(&path, false), "{path:?}");
            assert!(g.allowed(&path, true), "{path:?}");
        }
    }
    let a = allow(&g.roots(), &g.project.join("notes.txt"), false).unwrap();
    // The system is asked the path as it was given.
    assert_eq!(
        a.what(),
        crate::outside::What::Path(&g.project.join("notes.txt"))
    );
    let a = allow(&g.roots(), &g.project.join("notes.txt"), true).unwrap();
    assert_eq!(
        a.what(),
        crate::outside::What::Reveal(&g.project.join("notes.txt"))
    );
}

#[test]
fn a_folder_outside_them_is_refused_with_its_reason() {
    let g = Ground::new("opening-outside");
    for path in [
        g.outside.clone(),
        g.outside.join("notes.txt"),
        g.outside.join("sub"),
    ] {
        let e = g.refusal(&path, false);
        assert!(e.contains("hors des dossiers des projets"), "{e}");
        assert!(e.contains(&path.display().to_string()), "{e}");
        // Shown in its folder, refused just as much: the file manager would open the folder.
        let e = g.refusal(&path, true);
        assert!(e.contains("hors des dossiers des projets"), "{e}");
    }
    // The parent of a project is not in it, nor a folder whose name only starts as the project's does.
    let parent = g.project.parent().unwrap().to_path_buf();
    assert!(!g.allowed(&parent, false));
    let twin = parent.join("proj-evil");
    std::fs::create_dir_all(&twin).unwrap();
    assert!(!g.allowed(&twin, false));
    // Nothing allowed at all: nothing opens.
    assert!(allow(&[], &g.project, false).is_err());
}

#[test]
fn dot_dot_is_judged_where_it_leads() {
    let g = Ground::new("opening-dotdot");
    // Out by `..`: refused, and the refusal says where it leads.
    let out = g.project.join("..").join("elsewhere").join("notes.txt");
    let e = g.refusal(&out, false);
    assert!(e.contains("mène à"), "{e}");
    assert!(e.contains(&real(&g.outside.join("notes.txt"))), "{e}");
    assert!(!g.allowed(&g.project.join(".."), false));
    assert!(!g.allowed(&g.project.join("sub").join("..").join(".."), false));
    // In and back in by `..`: let through.
    assert!(g.allowed(&g.project.join("sub").join("..").join("notes.txt"), false));
    // Shown in its folder: the folder is judged, `..` resolved.
    assert!(!g.allowed(
        &g.project.join("..").join("elsewhere").join("notes.txt"),
        true
    ));
    assert!(g.allowed(&g.project.join("sub").join("..").join("notes.txt"), true));
    // A path ending in `..` names no item to show.
    let e = g.refusal(&g.project.join("sub").join(".."), true);
    assert!(e.contains("pas de dossier où le montrer"), "{e}");
}

#[test]
fn a_link_of_a_project_to_elsewhere_is_not_opened_but_shown_as_the_link() {
    let g = Ground::new("opening-link-out");
    let _link = dir_link(&g.project.join("out"), &g.outside);
    let e = g.refusal(&g.project.join("out"), false);
    assert!(e.contains("mène à"), "{e}");
    assert!(e.contains(&real(&g.outside)), "{e}");
    let e = g.refusal(&g.project.join("out").join("notes.txt"), false);
    assert!(e.contains("mène à"), "{e}");
    // Shown, the link itself is selected in the project's folder: it is not followed.
    assert!(g.allowed(&g.project.join("out"), true));
    // What is through it, shown, is in the folder it leads to: refused.
    assert!(!g.allowed(&g.project.join("out").join("notes.txt"), true));
}

#[test]
fn a_project_reached_through_a_link_or_holding_a_link_to_itself_opens() {
    let g = Ground::new("opening-link-in");
    // The project's folder as the projects name it is a link (a junction to another disk): what it leads to is in.
    let real = g.outside.join("real-project");
    std::fs::create_dir_all(&real).unwrap();
    std::fs::write(real.join("a.txt"), "x").unwrap();
    let linked = g.project.parent().unwrap().join("linked-project");
    let _l = dir_link(&linked, &real);
    let roots = vec![linked.clone()];
    assert!(allow(&roots, &linked.join("a.txt"), false).is_ok());
    assert!(allow(&roots, &real.join("a.txt"), false).is_ok());
    assert!(allow(&roots, &g.outside.join("notes.txt"), false).is_err());
    // A link inside a project to another folder of it: in.
    let _s = dir_link(&g.project.join("again"), &g.project.join("sub"));
    assert!(g.allowed(&g.project.join("again"), false));
}

#[cfg(unix)]
#[test]
fn a_file_link_to_elsewhere_is_judged_where_it_leads() {
    let g = Ground::new("opening-file-link");
    std::os::unix::fs::symlink(g.outside.join("notes.txt"), g.project.join("innocent.txt"))
        .unwrap();
    let e = g.refusal(&g.project.join("innocent.txt"), false);
    assert!(e.contains("mène à"), "{e}");
    // A broken link is no file to open.
    std::os::unix::fs::symlink(g.project.join("nowhere"), g.project.join("broken")).unwrap();
    let e = g.refusal(&g.project.join("broken"), false);
    assert!(e.contains("n’existe pas"), "{e}");
}

#[test]
fn a_relative_or_missing_path_is_refused() {
    let g = Ground::new("opening-relative");
    let e = g.refusal(Path::new("notes.txt"), false);
    assert!(e.contains("pas un chemin complet"), "{e}");
    let e = g.refusal(Path::new("proj/../elsewhere"), false);
    assert!(e.contains("pas un chemin complet"), "{e}");
    let e = g.refusal(&g.project.join("gone.txt"), false);
    assert!(e.contains("n’existe pas"), "{e}");
    let e = g.refusal(&g.project.join("gone.txt"), true);
    assert!(e.contains("n’existe pas"), "{e}");
}

#[cfg(windows)]
#[test]
fn verbatim_unc_and_device_paths_of_windows_are_judged_or_refused() {
    let g = Ground::new("opening-windows");
    let verbatim = |p: &Path| PathBuf::from(format!(r"\\?\{}", p.display()));
    // `\\?\C:\…`: the same path, judged as it.
    assert!(g.allowed(&verbatim(&g.project.join("notes.txt")), false));
    assert!(!g.allowed(&verbatim(&g.outside.join("notes.txt")), false));
    // Its `..` is no `..` for Windows, but a name: no such file, refused.
    assert!(!g.allowed(&verbatim(&g.project.join("..").join("elsewhere")), false));
    // `\\.\C:\…` too.
    assert!(g.allowed(
        &PathBuf::from(format!(r"\\.\{}", g.project.display())),
        false
    ));
    // Case is ignored, as the disk ignores it.
    let upper = PathBuf::from(
        g.project
            .join("NOTES.TXT")
            .display()
            .to_string()
            .to_uppercase(),
    );
    assert!(g.allowed(&upper, false), "{upper:?}");
    // A network path outside the folders is refused before the disk is asked: asking would reach the server.
    for p in [
        r"\\attacker.example\share\run.exe",
        r"//attacker.example/share/run.exe",
        r"\\?\UNC\attacker.example\share\run.exe",
        r"\\127.0.0.1\c$\Windows",
    ] {
        let started = std::time::Instant::now();
        let e = g.refusal(Path::new(p), false);
        assert!(e.contains("hors des dossiers des projets"), "{p}: {e}");
        assert!(started.elapsed() < std::time::Duration::from_secs(2), "{p}");
    }
    // A device is never opened.
    for p in [
        r"\\.\pipe\escouade",
        r"\\.\PhysicalDrive0",
        r"\\?\GLOBALROOT\Device\HarddiskVolume1",
    ] {
        let e = g.refusal(Path::new(p), false);
        assert!(e.contains("périphérique"), "{p}: {e}");
    }
    // A drive-relative path (`C:notes.txt`) or one from the root of the current drive (`\Windows`) is not full.
    for p in [r"C:notes.txt", r"\Windows"] {
        let e = g.refusal(Path::new(p), false);
        assert!(e.contains("pas un chemin complet"), "{p}: {e}");
    }
    // A project under WSL, named by its network path: the path as written is in it, so the disk is asked (and here
    // has no such distribution: refused for that, not for being outside).
    let wsl = PathBuf::from(r"\\wsl.localhost\no-such-distribution-escouade\home\ada\proj");
    let e = format!(
        "{:#}",
        allow(std::slice::from_ref(&wsl), &wsl.join("a.txt"), false).unwrap_err()
    );
    assert!(e.contains("n’existe pas"), "{e}");
}

#[tokio::test]
async fn the_folders_are_the_data_folder_the_projects_their_worktrees_and_their_agents() {
    let h = harness("opening-roots");
    let (p, r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = PathBuf::from(a.meta.worktree.clone().unwrap().path);
    let roots = h.core.open_roots();
    assert!(
        roots.contains(&h.core.data.root().to_path_buf()),
        "{roots:?}"
    );
    assert!(roots.contains(&PathBuf::from(&p.path)), "{roots:?}");
    assert!(roots.contains(&p.worktrees_root()), "{roots:?}");
    assert!(roots.contains(&wt), "{roots:?}");
    assert!(h.core.allow_open(&r, false).is_ok());
    assert!(h.core.allow_open(&wt, false).is_ok());
    assert!(h.core.allow_open(h.core.data.root(), false).is_ok());
    let outside = test_dir("opening-roots-outside");
    let e = format!("{:#}", h.core.allow_open(&outside, false).unwrap_err());
    assert!(e.contains("hors des dossiers des projets"), "{e}");
}

#[tokio::test]
async fn the_window_never_has_a_program_of_a_project_opened_nor_a_path_outside() {
    let h = harness("opening-window");
    let (_p, r) = h.project(false).await;
    for name in ["run.bat", "setup.EXE", "x.ps1", "go.sh"] {
        std::fs::write(r.join(name), "x").unwrap();
        let e = format!(
            "{:#}",
            crate::commands::open_path_in(&h.core, &r.join(name)).unwrap_err()
        );
        assert!(e.contains("programme ou un script"), "{name}: {e}");
    }
    let outside = test_dir("opening-window-outside");
    let e = format!(
        "{:#}",
        crate::commands::open_path_in(&h.core, &outside).unwrap_err()
    );
    assert!(e.contains("hors des dossiers des projets"), "{e}");
}

#[tokio::test]
async fn shown_in_its_folder_outside_them_is_refused_too() {
    let outside = test_dir("opening-reveal-outside");
    std::fs::write(outside.join("a.txt"), "x").unwrap();
    let roots = vec![test_dir("opening-reveal-root")];
    // Selected, then its folder opened as a fallback: both refused, the selection's reason told.
    let e = format!(
        "{:#}",
        crate::fsedit::reveal(
            crate::fsedit::Reveal::Select(outside.join("a.txt")),
            roots.clone()
        )
        .unwrap_err()
    );
    assert!(e.contains("hors des dossiers des projets"), "{e}");
    let e = format!(
        "{:#}",
        crate::fsedit::reveal(crate::fsedit::Reveal::Open(outside.clone()), roots.clone())
            .unwrap_err()
    );
    assert!(e.contains("hors des dossiers des projets"), "{e}");
    // A file an agent sent, or of a request of « À toi », the same, by the desktop's host as by a server's, which would
    // tell its client of it.
    let headless = crate::host::HeadlessHost::new(None);
    let desk = harness("opening-reveal-host");
    for host in [&headless as &dyn crate::host::Host, &*desk.core.host] {
        for reveal in [false, true] {
            let e = format!(
                "{:#}",
                crate::sentfiles::open_or_reveal(
                    host,
                    roots.clone(),
                    outside.join("a.txt"),
                    reveal
                )
                .unwrap_err()
            );
            assert!(e.contains("hors des dossiers des projets"), "{reveal}: {e}");
        }
    }
}
