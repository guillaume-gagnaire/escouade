//! What changes machines in server mode (ESC-538, `docs/REMOTE.md` « Ce qui change de machine », `docs/SERVER-UX.md`
//! § 4, S12, S13): the folders of the server for « Nouveau projet » (a repository and its branch, a project already, the
//! room left), « Cloner un dépôt » (https or ssh only), a logo sent as bytes, the files of the agents received on this
//! computer (their kind read from their bytes, only an image, a PDF or a text opened, ESC-629), the notifications
//! whose click a client asks, and the events a client missed during a cut.

use crate::host::{HeadlessHost, Host, HostEvent, KEPT_CLICKS};
use crate::serverfolders::{self, branch_of, folder_of, same_folder};
use crate::servers::received::{self, kept_name, kind_of};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn git(dir: &Path, args: &[&str]) {
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
}

fn dir(path: &Path) -> PathBuf {
    std::fs::create_dir_all(path).unwrap();
    path.to_path_buf()
}

// ---------- the folders of the server ----------

#[test]
fn a_folder_of_the_server_lists_its_folders_with_their_repositories_branches_and_projects() {
    let root = crate::paths::test_dir("moves-browse");
    let projets = dir(&root.join("projets"));
    // A repository on its branch, one detached, a worktree (a `.git` file), a plain folder, a hidden one, a file.
    let facturation = dir(&projets.join("facturation"));
    git(&facturation, &["init", "-q", "-b", "main"]);
    let outils = dir(&projets.join("outils-internes"));
    git(&outils, &["init", "-q", "-b", "dev"]);
    let detached = dir(&projets.join("Detache"));
    dir(&detached.join(".git"));
    std::fs::write(
        detached.join(".git").join("HEAD"),
        "0123456789abcdef0123456789abcdef01234567\n",
    )
    .unwrap();
    let wt = dir(&projets.join("wt"));
    let wt_git = dir(&root.join("gitdirs").join("wt"));
    std::fs::write(wt_git.join("HEAD"), "ref: refs/heads/feature/x\n").unwrap();
    std::fs::write(
        wt.join(".git"),
        format!("gitdir: {}\n", wt_git.to_string_lossy()),
    )
    .unwrap();
    dir(&projets.join("scripts"));
    dir(&projets.join(".cache"));
    std::fs::write(projets.join("notes.txt"), "pas un dossier").unwrap();

    let projects = vec![(
        // As a project may have been written: another separator, a last slash.
        format!("{}/", facturation.to_string_lossy().replace('\\', "/")),
        "Facturation".to_string(),
    )];
    let l = serverfolders::list(&projets, &root, &projects).unwrap();
    let names: Vec<&str> = l.entries.iter().map(|e| e.name.as_str()).collect();
    // Folders only, the hidden one left out, in the order of their names whatever their case.
    assert_eq!(
        names,
        ["Detache", "facturation", "outils-internes", "scripts", "wt"]
    );
    let get = |n: &str| l.entries.iter().find(|e| e.name == n).unwrap();
    assert!(get("facturation").repo);
    assert_eq!(get("facturation").branch.as_deref(), Some("main"));
    assert_eq!(get("facturation").project.as_deref(), Some("Facturation"));
    assert_eq!(get("outils-internes").branch.as_deref(), Some("dev"));
    assert_eq!(get("outils-internes").project, None);
    assert_eq!(get("Detache").branch.as_deref(), Some("0123456"));
    assert_eq!(get("wt").branch.as_deref(), Some("feature/x"));
    assert!(!get("scripts").repo && get("scripts").branch.is_none());
    assert_eq!(l.more, 0);
    assert_eq!(l.parent.as_deref(), Some(&*root.to_string_lossy()));
    assert!(!l.repo);
    // The room of its disk, said by the system.
    let (free, total) = (l.free.unwrap(), l.total.unwrap());
    assert!(free > 0 && total >= free, "{free} / {total}");
    // The folder itself a repository and a project.
    let inside = serverfolders::list(&facturation, &root, &projects).unwrap();
    assert!(inside.repo);
    assert_eq!(inside.branch.as_deref(), Some("main"));
    assert_eq!(inside.project.as_deref(), Some("Facturation"));
    // It opens on ~/projets.
    assert_eq!(serverfolders::start(&root), projets);
    assert_eq!(
        serverfolders::start(&projets.join("scripts")),
        projets.join("scripts")
    );
}

#[test]
fn a_listing_is_bounded_and_refuses_what_is_no_folder_of_the_server() {
    let root = crate::paths::test_dir("moves-browse-bounds");
    let many = dir(&root.join("many"));
    for i in 0..serverfolders::MAX_ENTRIES + 7 {
        dir(&many.join(format!("d{i:04}")));
    }
    let l = serverfolders::list(&many, &root, &[]).unwrap();
    assert_eq!(l.entries.len(), serverfolders::MAX_ENTRIES);
    assert_eq!(l.more, 7);
    // A relative path, a file, a folder that is not there.
    for bad in [PathBuf::from("projets"), root.join("nowhere"), {
        let f = root.join("f.txt");
        std::fs::write(&f, "x").unwrap();
        f
    }] {
        assert!(serverfolders::list(&bad, &root, &[]).is_err(), "{bad:?}");
    }
    assert!(branch_of(&root).is_none());
}

#[test]
fn folders_compare_as_written_and_a_clone_is_named_as_git_names_it() {
    assert!(same_folder(
        "/home/escouade/projets/api",
        "/home/escouade/projets/api/"
    ));
    assert!(!same_folder(
        "/home/escouade/projets/api",
        "/home/escouade/projets/api2"
    ));
    assert!(!same_folder("", ""));
    if cfg!(windows) {
        assert!(same_folder(r"C:\dev\Api", "c:/dev/api"));
    }
    assert_eq!(folder_of("https://github.com/acme/api.git"), "api");
    assert_eq!(
        folder_of("git@github.com:acme/facturation.git"),
        "facturation"
    );
    assert_eq!(
        folder_of("ssh://git@git.acme.fr:2222/acme/outils/"),
        "outils"
    );
}

#[tokio::test]
async fn cloner_un_depot_clones_https_or_ssh_into_a_folder_of_the_server_and_refuses_the_rest() {
    let root = crate::paths::test_dir("moves-clone");
    let work = dir(&root.join("work"));
    git(&work, &["init", "-q", "-b", "main"]);
    git(&work, &["config", "user.email", "t@example.com"]);
    git(&work, &["config", "user.name", "T"]);
    std::fs::write(work.join("README.md"), "# api\n").unwrap();
    git(&work, &["add", "."]);
    git(&work, &["commit", "-q", "-m", "init"]);
    let bare = root.join("api.git");
    git(
        &root,
        &[
            "clone",
            "-q",
            "--bare",
            &work.to_string_lossy(),
            &bare.to_string_lossy(),
        ],
    );
    let url = "https://git.example.test/acme/api.git";
    let config = format!(
        "url.{}.insteadOf={url}",
        bare.to_string_lossy().replace('\\', "/")
    );
    let parent = dir(&root.join("projets"));
    let made = serverfolders::clone(url, &parent, "api", std::slice::from_ref(&config))
        .await
        .unwrap();
    assert_eq!(PathBuf::from(&made), parent.join("api"));
    assert!(parent.join("api").join("README.md").is_file());
    assert_eq!(branch_of(&parent.join("api")).as_deref(), Some("main"));
    // The same folder again: not over what is there.
    let again = serverfolders::clone(url, &parent, "api", std::slice::from_ref(&config)).await;
    assert!(again.unwrap_err().to_string().contains("existe déjà"));
    // Never a local path, file://, another transport, an option of git, a credential; never a folder out of the parent.
    for bad in [
        "/srv/git/api.git",
        "file:///srv/git/api.git",
        "git://git.example.test/api.git",
        "ext::sh -c touch% /tmp/pwned",
        "--upload-pack=touch /tmp/pwned",
        "https://user:secret@git.example.test/api.git",
    ] {
        let e = serverfolders::clone(bad, &parent, "x", &[])
            .await
            .unwrap_err();
        assert!(e.to_string().contains("ne se clone pas"), "{bad}: {e}");
    }
    for folder in ["..", "../x", "a/b", "-x"] {
        assert!(
            serverfolders::clone(url, &parent, folder, &[])
                .await
                .is_err(),
            "{folder}"
        );
    }
    assert!(serverfolders::clone(url, &root.join("nowhere"), "api", &[])
        .await
        .is_err());
    // No folder said: the one git would name.
    let named = serverfolders::clone(
        url,
        &dir(&root.join("ailleurs")),
        " ",
        std::slice::from_ref(&config),
    )
    .await
    .unwrap();
    assert!(named.ends_with("api"), "{named}");
}

// ---------- a logo sent ----------

#[test]
fn a_logo_sent_as_bytes_is_checked_as_a_file_is() {
    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x01\0\0\0\x01\x08\x06\0\0\0";
    let lang = crate::i18n::Lang::Fr;
    let logo = crate::projectlogo::checked(lang, "logo.png", PNG.to_vec()).unwrap();
    assert_eq!(logo.media_type, "image/png");
    let e = crate::projectlogo::checked(lang, "rapport.pdf", b"%PDF-1.7".to_vec()).unwrap_err();
    assert!(e.to_string().contains("n’est pas une image"), "{e}");
    let heavy = vec![0u8; crate::projectlogo::MAX_LOGO + 1];
    let e = crate::projectlogo::checked(lang, "lourd.png", heavy).unwrap_err();
    assert!(e.to_string().contains("pèse"), "{e}");
    // Kept under the project, its previous logo gone.
    let root = crate::paths::test_dir("moves-logo");
    let data = crate::paths::DataDir::new(root.clone());
    let first = crate::projectlogo::keep(&data, "p1", &logo).unwrap();
    let second = crate::projectlogo::keep(&data, "p1", &logo).unwrap();
    assert_ne!(first, second);
    assert!(!data.project_files("p1").join(&first).exists());
    assert!(data.project_files("p1").join(&second).is_file());
    assert!(crate::projectlogo::keep(&data, "../x", &logo).is_err());
}

// ---------- the files received ----------

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
const PDF: &[u8] = b"%PDF-1.7\n%\xE2\xE3";

#[test]
fn a_file_received_is_known_by_its_bytes_and_keeps_the_type_they_say() {
    assert_eq!(kind_of(b"MZ\x90\0\x03"), "program");
    assert_eq!(kind_of(b"\x7fELF\x02\x01"), "program");
    assert_eq!(kind_of(&[0xCF, 0xFA, 0xED, 0xFE, 7]), "program");
    assert_eq!(kind_of(b"#!/bin/sh\nrm -rf ~\n"), "script");
    assert_eq!(kind_of(b"\xEF\xBB\xBF#!/usr/bin/env node\n"), "script");
    assert_eq!(kind_of(PDF), "pdf");
    assert_eq!(kind_of(PNG), "image");
    assert_eq!(kind_of(b"col1;col2\n"), "file");
    // The extension of the type its bytes say, when its name says another or none.
    assert_eq!(kept_name("rapport.pdf", PDF), "rapport.pdf");
    assert_eq!(kept_name("rapport", PDF), "rapport.pdf");
    assert_eq!(kept_name("capture.txt", PNG), "capture.txt.png");
    assert_eq!(kept_name("photo.JPG", b"\xFF\xD8\xFF\xE0"), "photo.JPG");
    assert_eq!(kept_name("export.csv", b"a;b\n"), "export.csv");
    // A name with a path or reserved characters: its last part, made safe.
    assert_eq!(kept_name("../../etc/passwd", b"x"), "passwd");
    assert_eq!(kept_name(r"..\..\a:b.txt", b"x"), "a_b.txt");
    // A program or a script, by its bytes or by its name.
    assert!(received::runnable("facture.pdf", b"MZ\x90"));
    assert!(received::runnable("rejouer.sh", b"echo"));
    assert!(received::runnable("deploy.ps1", b"Remove-Item"));
    assert!(!received::runnable("rapport.pdf", PDF));
}

#[test]
fn a_file_received_goes_in_the_folder_of_its_space_never_opened_when_it_runs() {
    let downloads = crate::paths::test_dir("moves-received");
    let a = received::save(&downloads, "Travail", "rapport-relances.pdf", PDF).unwrap();
    let folder = downloads.join("Escouade").join("Travail");
    assert_eq!(PathBuf::from(&a.folder), folder);
    assert_eq!(PathBuf::from(&a.path), folder.join("rapport-relances.pdf"));
    assert_eq!((a.kind, a.runnable, a.opened), ("pdf", false, false));
    assert_eq!(std::fs::read(&a.path).unwrap(), PDF);
    // The same name again: a copy of its own.
    let b = received::save(&downloads, "Travail", "rapport-relances.pdf", PDF).unwrap();
    assert_eq!(b.name, "rapport-relances (2).pdf");
    // A script: kept, never runnable from its copy.
    let s = received::save(
        &downloads,
        "Travail",
        "rejouer-relances.sh",
        b"#!/bin/sh\necho\n",
    )
    .unwrap();
    assert!(s.runnable);
    assert_eq!(s.kind, "script");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&s.path).unwrap().permissions().mode();
        assert_eq!(mode & 0o111, 0, "{mode:o}");
    }
    // Under Windows, the copy is marked as come from another machine.
    #[cfg(windows)]
    {
        let zone = std::fs::read_to_string(format!("{}:Zone.Identifier", s.path)).unwrap();
        assert!(zone.contains("ZoneId=3"), "{zone}");
    }
    // A space's name never leads out of the downloads.
    let odd = received::save(&downloads, "../..", "x.txt", b"x").unwrap();
    assert!(PathBuf::from(&odd.path).starts_with(downloads.join("Escouade")));
    assert!(received::save(&downloads, " ", "x.txt", b"x").is_err());
    // Opened again: a copy of the downloads only, a script only shown in its folder.
    assert!(received::shown(&downloads, Path::new(&a.path), false).is_ok());
    let e = received::shown(&downloads, Path::new(&s.path), false).unwrap_err();
    assert!(e.to_string().contains("programme ou un script"), "{e}");
    assert!(received::shown(&downloads, Path::new(&s.path), true).is_ok());
    let elsewhere = downloads.join("autre.pdf");
    std::fs::write(&elsewhere, PDF).unwrap();
    assert!(received::shown(&downloads, &elsewhere, false).is_err());
    assert!(received::shown(&downloads, &folder.join("nowhere.pdf"), false).is_err());
    // A program renamed as a document is one all the same.
    let p = received::save(&downloads, "Travail", "facture.pdf", b"MZ\x90\0").unwrap();
    assert!(p.runnable);
    assert!(received::shown(&downloads, Path::new(&p.path), false).is_err());
}

// ---------- the notifications ----------

#[test]
fn a_notification_of_a_host_without_a_window_is_clicked_once_by_its_id() {
    let host = HeadlessHost::default();
    let mut events = host.subscribe();
    let clicks = Arc::new(AtomicUsize::new(0));
    let c = clicks.clone();
    host.notify(
        "facturation · relances",
        "Tour fini",
        Box::new(move || {
            c.fetch_add(1, Ordering::SeqCst);
        }),
    );
    let Ok(HostEvent::Notify { id, title, .. }) = events.try_recv() else {
        panic!("no notification published");
    };
    assert_eq!(title, "facturation · relances");
    assert!(host.clicked(id));
    assert_eq!(clicks.load(Ordering::SeqCst), 1);
    // Once: a second click (another client) does nothing more; an id never given, nothing.
    assert!(!host.clicked(id));
    assert!(!host.clicked(id + 1000));
    assert_eq!(clicks.load(Ordering::SeqCst), 1);
    // Only the last ones are kept.
    let mut first = None;
    for i in 0..KEPT_CLICKS + 1 {
        host.notify(&format!("n{i}"), "", Box::new(|| {}));
        if let Ok(HostEvent::Notify { id, .. }) = events.try_recv() {
            first.get_or_insert(id);
        }
    }
    assert!(!host.clicked(first.unwrap()));
}

#[test]
fn a_session_taken_again_says_how_many_events_its_client_missed() {
    let sessions = crate::remoteapi::sessions::Sessions::default();
    let now = std::time::Instant::now();
    let (id, resumed) = sessions.open("dev1", None, now);
    assert!(!resumed);
    // Never left: nothing missed.
    assert_eq!(sessions.missed(&id, 10), None);
    sessions.note_seen(&id, 40);
    sessions.leave(&id, now);
    let (again, resumed) = sessions.open("dev1", Some(&id), now);
    assert!(resumed && again == id);
    assert_eq!(sessions.missed(&id, 52), Some(12));
}

/// What a file received from a server becomes, type by type (ESC-629): only an image or a PDF is opened by the system,
/// a plain text, Markdown, CSV or JSON is read in Escouade's editor, anything else is only shown in its folder, and
/// the refusal says why.
#[test]
fn a_file_received_from_a_server_opens_only_when_its_type_is_on_the_list() {
    use received::Opening::{Editor, Folder, System};
    let downloads = crate::paths::test_dir("moves-received-list");
    let gif: &[u8] = b"GIF89a\x01\0\x01\0";
    let jpeg: &[u8] = b"\xFF\xD8\xFF\xE0\0\x10JFIF";
    let webp: &[u8] = b"RIFF\x24\0\0\0WEBPVP8 ";
    let xml: &[u8] = b"<?xml version=\"1.0\"?><x/>";
    // Opened with what reads its type: its bytes and its name agree.
    for (name, bytes) in [
        ("capture.png", PNG),
        ("photo.jpg", jpeg),
        ("photo.jpeg", jpeg),
        ("anim.gif", gif),
        ("image.webp", webp),
        ("rapport.pdf", PDF),
        ("RAPPORT.PDF", PDF),
    ] {
        let r = received::save(&downloads, "Travail", name, bytes).unwrap();
        assert_eq!(received::opening(&r.name, bytes), System, "{name}");
        assert_eq!(r.opens, "system", "{name}");
        assert!(
            received::shown(&downloads, Path::new(&r.path), false).is_ok(),
            "{name}"
        );
    }
    // Read in Escouade's editor, never handed to the system (a spreadsheet would compute a CSV).
    for (name, bytes) in [
        ("notes.txt", &b"bonjour\n"[..]),
        ("lisez-moi.md", &b"# Titre\n"[..]),
        ("journal.markdown", &b"*x*"[..]),
        ("export.csv", &b"a;b\n=1+1;2\n"[..]),
        ("donnees.json", &b"{\"a\": 1}"[..]),
        ("accents.txt", "étés et « guillemets »".as_bytes()),
    ] {
        let r = received::save(&downloads, "Travail", name, bytes).unwrap();
        assert_eq!(r.opens, "editor", "{name}");
        let e = received::shown(&downloads, Path::new(&r.path), false).unwrap_err();
        assert!(e.to_string().contains("éditeur d’Escouade"), "{name}: {e}");
        assert!(received::shown(&downloads, Path::new(&r.path), true).is_ok());
    }
    // Anything else: only shown in its folder, with why. Consoles, help files, disk images, libraries and searches,
    // installers, add-ins, pages and drawings of the web, shortcuts and links of every system, scripts and workflows.
    for (name, bytes) in [
        ("rapport.msc", xml),
        ("aide.chm", &b"ITSF\x03\0\0\0"[..]),
        ("disque.iso", &b"\0\0\0\0CD001"[..]),
        ("disque.vhdx", &b"vhdxfile"[..]),
        ("disque.img", &b"\0\0"[..]),
        ("dossier.library-ms", xml),
        ("recherche.search-ms", xml),
        ("app.appinstaller", xml),
        ("paquet.msix", &b"PK\x03\x04"[..]),
        ("addin.xll", &b"PK\x03\x04"[..]),
        ("page.html", &b"<html><script>x</script>"[..]),
        ("page.htm", &b"<html>"[..]),
        (
            "image.svg",
            &b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>"[..],
        ),
        ("explorer.scf", &b"[Shell]\r\nCommand=2\r\n"[..]),
        ("module.psm1", &b"Get-Date"[..]),
        ("ouvrir.terminal", xml),
        ("lien.webloc", xml),
        ("lien.fileloc", xml),
        ("script.scpt", &b"FasdUAS"[..]),
        ("flux.workflow", &b"x"[..]),
        ("etude.docx", &b"PK\x03\x04"[..]),
        ("tableau.xlsm", &b"PK\x03\x04"[..]),
        ("archive.zip", &b"PK\x03\x04"[..]),
        ("sans-extension", &b"bonjour"[..]),
        ("deploy.ps1", &b"Remove-Item"[..]),
        ("rejouer.sh", &b"#!/bin/sh\n"[..]),
        ("programme.exe", &b"MZ\x90\0"[..]),
    ] {
        let r = received::save(&downloads, "Travail", name, bytes).unwrap();
        assert_eq!(r.opens, "folder", "{name}");
        let e = received::shown(&downloads, Path::new(&r.path), false).unwrap_err();
        let why = e.to_string();
        assert!(
            why.contains("programme ou un script")
                || why.contains("pas d’un type qu’Escouade ouvre"),
            "{name}: {why}"
        );
        assert!(received::shown(&downloads, Path::new(&r.path), true).is_ok());
    }
    // A name of the list whose bytes say another thing is not on it: a text that holds a program, a « PDF » of XML.
    assert_eq!(received::opening("notes.txt", b"MZ\x90\0"), Folder);
    assert_eq!(received::opening("notes.txt", b"#!/bin/sh\n"), Folder);
    assert_eq!(received::opening("notes.txt", b"a\0b"), Folder);
    assert_eq!(received::opening("notes.txt", b"\xFF\xFE\xFD"), Folder);
    assert_eq!(received::opening("rapport.pdf", xml), Folder);
    assert_eq!(received::opening("capture.png", xml), Folder);
    // A text cut in the middle of a character (its start only is read) stays a text; a name ending in dots too.
    assert_eq!(
        received::opening("notes.txt", &"été".as_bytes()[..3]),
        Editor
    );
    assert_eq!(received::opening("notes.txt. ", b"x"), Editor);
    // The extension of the type its bytes say makes a renamed image one again, opened as such.
    let r = received::save(&downloads, "Travail", "ouvrir.terminal", PNG).unwrap();
    assert_eq!(
        (r.name.as_str(), r.opens),
        ("ouvrir.terminal.png", "system")
    );
}

/// The copy is marked as come from elsewhere, and never executable (ESC-629): `Zone.Identifier` under Windows,
/// `com.apple.quarantine` under macOS, no right to run under macOS and Linux.
#[test]
fn a_file_received_from_a_server_is_marked_as_downloaded_and_never_runs() {
    let downloads = crate::paths::test_dir("moves-received-marks");
    let r = received::save(&downloads, "Travail", "ouvrir.terminal", b"<?xml?>").unwrap();
    #[cfg(windows)]
    {
        let zone = std::fs::read_to_string(format!("{}:Zone.Identifier", r.path)).unwrap();
        assert!(zone.contains("ZoneId=3"), "{zone}");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&r.path).unwrap().permissions().mode();
        assert_eq!(mode & 0o111, 0, "{mode:o}");
    }
    #[cfg(target_os = "macos")]
    {
        let out = Command::new("xattr")
            .args(["-p", "com.apple.quarantine", &r.path])
            .output()
            .unwrap();
        let value = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "no quarantine on {}", r.path);
        assert!(
            value.starts_with("0081;") && value.contains(";Escouade;"),
            "{value}"
        );
    }
    // The value is the one macOS gives a download: its flags, the time in hexadecimal, the app.
    let at = std::time::UNIX_EPOCH + std::time::Duration::from_secs(0x6740_0000);
    assert_eq!(received::quarantine_value(at), "0081;67400000;Escouade;");
    let _ = r;
}
