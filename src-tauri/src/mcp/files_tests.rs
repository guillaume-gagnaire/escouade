//! The other files of a ticket through the server (ESC-108), called by a real MCP client (rmcp's)
//! on a test core: `create_ticket` and `update_ticket` attach and take away, `get_ticket` lists,
//! `get_ticket_file` reads a part back, `split_ticket` hands copies down. A new file goes the way
//! the window's goes (a copy of the drafts, then moved into the ticket's folder by the core), so the
//! limits and the refusals are the window's (`ticketfiles::check`, the size of the settings), and a
//! refused call leaves nothing behind: no ticket, no key used up, no file, no copy in the drafts.

use super::private_tests::make_private;
#[cfg(windows)]
use super::tools_tests::junction;
use super::tools_tests::{
    as_agent, call, external, link, other_project, project, put_agent, read, refused, Client,
};
use crate::core_tests::{harness, Harness};
use crate::model::*;
use crate::tickets::{DraftFile, TicketDraft};
use base64::Engine;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// A file as the tools take it, by its content.
fn given(name: &str, bytes: &[u8]) -> Value {
    json!({ "name": name, "data": b64(bytes) })
}

/// The bytes of a small archive: not text.
const ZIP: &[u8] = b"PK\x03\x04\x14\x00\x00\x00\x08\x00\xff\xfe-archive";

/// The entries of a folder, by name (none when it is not there).
fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|d| {
            d.map(|e| e.unwrap().file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// What is left in the drafts: nothing, once a call is over, whatever it did.
fn drafts_left(h: &Harness) -> Vec<String> {
    entries(&h.core.data.drafts_files())
}

/// Where the ticket's file is.
fn path_of(h: &Harness, ticket_id: &str, file: &str) -> PathBuf {
    let (uniq, name) = file.split_once('/').unwrap();
    h.core.data.ticket_files(ticket_id).join(uniq).join(name)
}

/// A ticket « À faire » made as the window makes one, with these files (name, content).
async fn made(h: &Harness, p: &Project, title: &str, files: &[(&str, &[u8])]) -> Ticket {
    let joined = files
        .iter()
        .map(|(name, bytes)| {
            let copy = crate::attachfiles_tests::copy(h, "draft", name, bytes);
            DraftFile {
                name: copy.name,
                path: copy.path,
                ..Default::default()
            }
        })
        .collect();
    h.core
        .ticket_create(
            &p.id,
            TicketDraft {
                title: title.into(),
                files: Some(joined),
                ..Default::default()
            },
        )
        .await
        .unwrap()
}

fn stored(h: &Harness, id: &str) -> Ticket {
    h.core.ticket(id).unwrap()
}

fn moved(h: &Harness, id: &str, column: Column) {
    h.core
        .edit_ticket(id, |t| {
            t.column = column;
            Ok(())
        })
        .unwrap();
}

/// What a refused call must leave as it was: the tickets, the next key, the folders of `tickets/`,
/// and the drafts.
#[derive(Debug, PartialEq)]
struct Board {
    tickets: usize,
    next: u32,
    folders: Vec<String>,
    drafts: Vec<String>,
}

fn board(h: &Harness, p: &Project) -> Board {
    Board {
        tickets: h.core.tickets.read().len(),
        next: h.core.project(&p.id).unwrap().board.next_number,
        folders: entries(&h.core.data.tickets()),
        drafts: drafts_left(h),
    }
}

/// The activity log's last entry: its summary and its message.
fn logged(h: &Harness) -> (String, Option<String>) {
    let e = h.core.mcp.activity.entries().pop().unwrap();
    (e.summary, e.message)
}

const OUTSIDE: &str = "Un agent d’Escouade ne joint que des fichiers de son dossier de travail : ce fichier n’en fait pas partie, ou il n’existe pas.";

// ---------- create_ticket ----------

#[tokio::test]
async fn files_of_any_format_are_attached_to_a_new_ticket_by_content_or_path_and_listed() {
    let h = harness("mcp-files-create");
    let _p = project(&h).await;
    let c = external(&h).await;
    let docs = h.dir.join("documents");
    std::fs::create_dir_all(&docs).unwrap();
    let sheet = docs.join("budget 2026.xlsx");
    std::fs::write(&sheet, b"xlsx-content").unwrap();
    let log = docs.join("trace.log");
    std::fs::write(&log, b"ligne 1\nligne 2\n").unwrap();
    let created = read(
        &c,
        "create_ticket",
        json!({
            "project": "demo",
            "title": "Avec des fichiers",
            "files": [
                given("archive.zip", ZIP),
                { "path": sheet },
                { "path": log, "name": "journal du serveur.log" },
            ],
        }),
    )
    .await;
    let files = created["files"].as_array().unwrap().clone();
    assert_eq!(files.len(), 3, "{created}");
    let described: Vec<(u64, String, u64)> = files
        .iter()
        .map(|f| {
            (
                f["index"].as_u64().unwrap(),
                f["name"].as_str().unwrap().to_string(),
                f["size"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        described,
        [
            (0, "archive.zip".into(), ZIP.len() as u64),
            (1, "budget 2026.xlsx".into(), 12),
            (2, "journal du serveur.log".into(), 16),
        ]
    );
    // In the ticket's folder, `<uniq>/<name>`, with the content given; the originals stay.
    let id = created["id"].as_str().unwrap();
    for (f, bytes) in files
        .iter()
        .zip([ZIP, b"xlsx-content", b"ligne 1\nligne 2\n"])
    {
        let file = f["file"].as_str().unwrap();
        assert!(
            file.ends_with(&format!("/{}", f["name"].as_str().unwrap())),
            "{file}"
        );
        assert_eq!(std::fs::read(path_of(&h, id, file)).unwrap(), bytes);
    }
    assert!(sheet.exists() && log.exists());
    // Nothing is left in the drafts, and the ticket keeps nothing of the paths.
    assert_eq!(drafts_left(&h), Vec::<String>::new());
    assert!(!serde_json::to_string(&stored(&h, id))
        .unwrap()
        .contains("documents"));
    // The journal says how many, never what.
    let (summary, message) = logged(&h);
    assert!(summary.contains("files: 3"), "{summary}");
    assert!(!summary.contains(&b64(ZIP)) && !summary.contains("documents"));
    assert_eq!(message, None);
    // `get_ticket` lists the same.
    let got = read(&c, "get_ticket", json!({ "ticket": "DEM-1" })).await;
    assert_eq!(got["files"], created["files"]);
    // Its agent is given their paths with its first message, as for the window's.
    let t = stored(&h, id);
    let note = crate::ticketfiles::message(
        crate::i18n::Lang::Fr,
        None,
        &h.core.data.ticket_dir(id),
        &t.files,
    );
    assert_eq!(note.matches("Fichier joint : ").count(), 3, "{note}");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_new_ticket_refuses_what_the_window_refuses_and_nothing_is_made_or_left() {
    let h = harness("mcp-files-create-refused");
    let p = project(&h).await;
    let c = external(&h).await;
    let before = board(&h, &p);
    let big = h.dir.join("gros.iso");
    std::fs::write(&big, vec![0u8; (1 << 20) + 1]).unwrap();
    h.core.settings.write().attach_max_mb = 1;
    let secret = b64(b"contenu-secret");
    let cases: Vec<(Value, &str)> = vec![
        // Over the size the settings allow, by content and by path: refused as the window does.
        (
            json!([given("ok.txt", b"ok"), given("gros.bin", &vec![0u8; (1 << 20) + 1])]),
            "Fichier 2 de la liste : gros.bin dépasse 1 Mo",
        ),
        (json!([{ "path": big }]), "Fichier 1 de la liste : gros.iso dépasse 1 Mo"),
        (
            json!([{ "name": "a.txt", "data": secret, "path": big }]),
            "Fichier 1 de la liste : Donne le contenu du fichier (data) ou son chemin (path), pas les deux.",
        ),
        (
            json!([{ "name": "a.txt" }]),
            "Fichier 1 de la liste : Donne le contenu du fichier (data) ou son chemin (path).",
        ),
        (
            json!([{ "data": secret }]),
            "Fichier 1 de la liste : Un fichier donné par son contenu (data) a besoin d’un nom (name).",
        ),
        (
            json!([{ "name": "  ", "data": secret }]),
            "Fichier 1 de la liste : Le nom du fichier est vide.",
        ),
        // A content that is not base64 is not repeated.
        (
            json!([{ "name": "a.txt", "data": "pas-du-base64-secret!!" }]),
            "Fichier 1 de la liste : Le contenu de a.txt n’est pas du base64 valide.",
        ),
        (
            json!([{ "path": "relatif/a.txt" }]),
            "Fichier 1 de la liste : Le chemin doit être absolu : donne le chemin complet du fichier.",
        ),
        (
            json!([{ "path": h.dir.join("absent.txt") }]),
            "Fichier 1 de la liste : Le fichier est introuvable ou illisible",
        ),
        (
            json!([{ "path": r"\\serveur\partage\a.txt" }]),
            "Fichier 1 de la liste : Un chemin réseau",
        ),
    ];
    for (files, said) in cases {
        let why = refused(
            &c,
            "create_ticket",
            json!({ "project": "demo", "title": "Refusé", "files": files }),
        )
        .await;
        assert!(why.starts_with(said), "{files}: {why}");
        assert!(
            !why.contains("pas-du-base64") && !why.contains(&secret),
            "{why}"
        );
        assert_eq!(board(&h, &p), before, "{files}");
        // The journal keeps the count, never a content nor a path.
        let (summary, _) = logged(&h);
        assert!(
            !summary.contains(&secret) && !summary.contains("gros.iso"),
            "{summary}"
        );
    }
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn more_than_twenty_files_are_refused_before_any_is_read_or_copied() {
    let h = harness("mcp-files-create-count");
    let p = project(&h).await;
    let c = external(&h).await;
    let before = board(&h, &p);
    // Paths that lead nowhere: had one been looked at, the refusal would say so.
    let many: Vec<Value> = (0..21)
        .map(|i| json!({ "path": h.dir.join(format!("absent-{i}.bin")) }))
        .collect();
    let why = refused(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "Trop", "files": many }),
    )
    .await;
    assert_eq!(why, "Un ticket prend 20 fichiers au plus.");
    assert_eq!(board(&h, &p), before);
    // Twenty go through.
    let twenty: Vec<Value> = (0..20).map(|i| given(&format!("{i}.txt"), b"x")).collect();
    let ok = read(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "Vingt", "files": twenty }),
    )
    .await;
    assert_eq!(ok["files"].as_array().unwrap().len(), 20);
    assert_eq!(drafts_left(&h), Vec::<String>::new());
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_failure_after_some_files_were_copied_leaves_no_copy_in_the_drafts() {
    let h = harness("mcp-files-create-cleanup");
    let p = project(&h).await;
    let c = external(&h).await;
    let before = board(&h, &p);
    // The third fails its check: the list is checked whole first, so none is copied.
    let why = refused(
        &c,
        "create_ticket",
        json!({
            "project": "demo",
            "title": "x",
            "files": [given("a.txt", b"a"), given("b.txt", b"b"), { "path": h.dir.join("absent.txt") }],
        }),
    )
    .await;
    assert!(why.starts_with("Fichier 3 de la liste"), "{why}");
    assert_eq!(board(&h, &p), before);
    // Another argument refused: no copy is even made.
    let why = refused(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "x", "after": ["DEM-99"], "files": [given("a.txt", b"a")] }),
    )
    .await;
    assert!(!why.is_empty());
    assert_eq!(board(&h, &p), before);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_copies_a_call_made_in_the_drafts_go_once_it_is_over_whatever_the_core_did() {
    use super::files::{drafts, Drafted};
    use super::tools::FileArg;
    let h = harness("mcp-files-drafted");
    let arg = |name: &str| FileArg {
        name: Some(name.into()),
        data: Some(b64(b"contenu")),
        path: None,
    };
    let mut drafted = Drafted::new(&h.core);
    let copies = drafts(
        &h.core,
        &super::Caller::External,
        vec![arg("a.txt"), arg("b.txt")],
        0,
        &mut drafted,
    )
    .await
    .unwrap();
    // Copies of the drafts, as the form makes them, the core to move them.
    assert_eq!(drafts_left(&h).len(), 2);
    for c in &copies {
        assert_eq!(std::fs::read(&c.path).unwrap(), b"contenu");
        assert!(c.file.is_empty());
    }
    // Not moved (the core refused, the call stopped): they go with the call.
    drop(drafted);
    assert_eq!(drafts_left(&h), Vec::<String>::new());
}

#[tokio::test]
async fn an_update_the_core_refuses_after_the_files_are_moved_leaves_nothing_of_them() {
    let h = harness("mcp-files-update-cycle");
    let p = project(&h).await;
    let a = made(&h, &p, "A", &[]).await;
    let b = made(&h, &p, "B", &[("b.txt", b"b")]).await;
    let c = external(&h).await;
    read(
        &c,
        "update_ticket",
        json!({ "ticket": b.key, "after": [a.key] }),
    )
    .await;
    // A waits for B, which waits for A: refused once the new file is in B's folder.
    let why = refused(
        &c,
        "update_ticket",
        json!({ "ticket": a.key, "after": [b.key], "addFiles": [given("n.txt", b"n")] }),
    )
    .await;
    assert!(!why.is_empty());
    assert!(stored(&h, &a.id).files.is_empty());
    assert!(!h.core.data.ticket_files(&a.id).exists());
    assert_eq!(drafts_left(&h), Vec::<String>::new());
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agent_of_escouade_gives_only_the_files_of_its_working_folder() {
    let h = harness("mcp-files-create-agent");
    let p = project(&h).await;
    let work = h.dir.join("travail");
    std::fs::create_dir_all(work.join("sous")).unwrap();
    std::fs::write(work.join("sous").join("dedans.csv"), b"a;b").unwrap();
    std::fs::write(h.dir.join("dehors.csv"), b"secret").unwrap();
    let agent = put_agent(&h, &p, "chef", 1);
    h.core.agent(&agent.id).unwrap().lock().meta.cwd = work.to_string_lossy().to_string();
    let c = as_agent(&h, &agent.id).await;
    let created = read(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "Dedans", "files": [{ "path": work.join("sous").join("dedans.csv") }] }),
    )
    .await;
    assert_eq!(created["files"][0]["name"], "dedans.csv");
    let before = board(&h, &p);
    for outside in [
        h.dir.join("dehors.csv"),
        work.join("sous").join("..").join("..").join("dehors.csv"),
        h.dir.join("absent.csv"),
    ] {
        let why = refused(
            &c,
            "create_ticket",
            json!({ "project": "demo", "title": "Dehors", "files": [{ "path": outside }] }),
        )
        .await;
        assert_eq!(
            why,
            format!("Fichier 1 de la liste : {OUTSIDE}"),
            "{outside:?}"
        );
        assert_eq!(board(&h, &p), before);
    }
    // Claude outside Escouade gives any file of the machine.
    let outside = external(&h).await;
    let ok = read(
        &outside,
        "create_ticket",
        json!({ "project": "demo", "title": "Dehors", "files": [{ "path": h.dir.join("dehors.csv") }] }),
    )
    .await;
    assert_eq!(ok["files"][0]["size"], 6);
    c.cancel().await.unwrap();
    outside.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[cfg(windows)]
#[tokio::test]
async fn a_junction_out_of_its_working_folder_does_not_lead_an_agent_out() {
    let h = harness("mcp-files-create-junction");
    let p = project(&h).await;
    let (work, outside) = (h.dir.join("travail"), h.dir.join("dehors"));
    std::fs::create_dir_all(&work).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.txt"), b"secret").unwrap();
    let _link = junction(&work.join("lien"), &outside);
    let agent = put_agent(&h, &p, "chef", 1);
    h.core.agent(&agent.id).unwrap().lock().meta.cwd = work.to_string_lossy().to_string();
    let c = as_agent(&h, &agent.id).await;
    let before = board(&h, &p);
    let why = refused(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "x", "files": [{ "path": work.join("lien").join("secret.txt") }] }),
    )
    .await;
    assert_eq!(why, format!("Fichier 1 de la liste : {OUTSIDE}"));
    assert_eq!(board(&h, &p), before);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

// ---------- update_ticket ----------

#[tokio::test]
async fn files_are_added_after_the_ones_a_ticket_has_and_taken_away_by_file_or_rank() {
    let h = harness("mcp-files-update");
    let p = project(&h).await;
    let t = made(
        &h,
        &p,
        "Ticket",
        &[("a.zip", b"a"), ("b.pdf", b"bb"), ("c.log", b"ccc")],
    )
    .await;
    let c = external(&h).await;
    let (a, b, cc) = (t.files[0].clone(), t.files[1].clone(), t.files[2].clone());
    let updated = read(
        &c,
        "update_ticket",
        json!({
            "ticket": t.key,
            "addFiles": [given("d.txt", b"dddd")],
            "removeFiles": [{ "file": b.file }, { "index": 0 }],
        }),
    )
    .await;
    let names: Vec<&str> = updated["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["c.log", "d.txt"]);
    // The ones kept are as they were; the ones taken away are gone from the disk.
    let now = stored(&h, &t.id);
    assert_eq!(now.files[0], cc);
    assert!(!path_of(&h, &t.id, &a.file).exists() && !path_of(&h, &t.id, &b.file).exists());
    assert_eq!(
        std::fs::read(path_of(&h, &t.id, &now.files[1].file)).unwrap(),
        b"dddd"
    );
    // Files alone leave the words as they were: no line saying who edited the ticket.
    assert_eq!(now.title, t.title);
    assert_eq!(now.description, t.description);
    assert_eq!(drafts_left(&h), Vec::<String>::new());
    let (summary, _) = logged(&h);
    assert!(
        summary.contains("addFiles: 1") && summary.contains("removeFiles: 2"),
        "{summary}"
    );
    // A change of title keeps them.
    let renamed = read(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "title": "Renommé" }),
    )
    .await;
    assert_eq!(renamed["files"], updated["files"]);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_update_refuses_what_the_window_refuses_and_changes_nothing() {
    let h = harness("mcp-files-update-refused");
    let p = project(&h).await;
    let t = made(&h, &p, "Ticket", &[("a.zip", b"a"), ("b.pdf", b"b")]).await;
    let c = external(&h).await;
    let check = |h: &Harness| {
        assert_eq!(stored(h, &t.id).files, t.files);
        assert_eq!(drafts_left(h), Vec::<String>::new());
        for f in &t.files {
            assert!(path_of(h, &t.id, &f.file).exists());
        }
    };
    // A reference that fits no file of the ticket: nothing is removed, nothing copied.
    for reference in [
        json!({ "file": "../state.json" }),
        json!({ "index": 2 }),
        json!({ "file": t.files[0].file, "index": 0 }),
        json!({}),
    ] {
        let why = refused(
            &c,
            "update_ticket",
            json!({ "ticket": t.key, "addFiles": [given("c.txt", b"c")], "removeFiles": [reference] }),
        )
        .await;
        assert!(why.contains("get_ticket"), "{why}");
        check(&h);
    }
    // The files the ticket keeps count: 2 kept and 19 added make 21.
    let many: Vec<Value> = (0..19).map(|i| given(&format!("{i}.txt"), b"x")).collect();
    let why = refused(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "addFiles": many }),
    )
    .await;
    assert_eq!(why, "Un ticket prend 20 fichiers au plus.");
    check(&h);
    // Over the size of the settings.
    h.core.settings.write().attach_max_mb = 1;
    let why = refused(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "addFiles": [given("gros.bin", &vec![0u8; (1 << 20) + 1])] }),
    )
    .await;
    assert_eq!(why, "Fichier 1 de la liste : gros.bin dépasse 1 Mo");
    check(&h);
    // Empty lists alone are nothing to change.
    let why = refused(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "addFiles": [], "removeFiles": [] }),
    )
    .await;
    assert!(
        why.contains("addFiles") && why.contains("removeFiles"),
        "{why}"
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_that_has_started_is_refused_before_any_file_is_read_or_reference_looked_at() {
    let h = harness("mcp-files-update-started");
    let p = project(&h).await;
    let t = made(&h, &p, "Ticket", &[("a.zip", b"a")]).await;
    moved(&h, &t.id, Column::Doing);
    let c = external(&h).await;
    for args in [
        json!({ "ticket": t.key, "addFiles": [{ "path": h.dir.join("absent.txt") }] }),
        json!({ "ticket": t.key, "removeFiles": [{ "index": 9 }] }),
    ] {
        let why = refused(&c, "update_ticket", args).await;
        assert_eq!(why, "Seul un ticket « À faire » se modifie.");
    }
    assert_eq!(stored(&h, &t.id).files, t.files);
    assert_eq!(drafts_left(&h), Vec::<String>::new());
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agent_changes_the_files_of_a_ticket_of_another_project_unless_it_is_private() {
    let h = harness("mcp-files-update-other");
    let p = project(&h).await;
    let other = other_project(&h, "autre").await;
    let theirs = made(&h, &other, "Le leur", &[("a.zip", b"a"), ("b.zip", b"b")]).await;
    let agent = put_agent(&h, &p, "chef", 1);
    let c = as_agent(&h, &agent.id).await;
    // Another project that is not private: as in its own.
    read(
        &c,
        "update_ticket",
        json!({ "ticket": theirs.key, "removeFiles": [{ "index": 0 }] }),
    )
    .await;
    assert_eq!(stored(&h, &theirs.id).files, theirs.files[1..]);
    // A private one: not found, and its files stay.
    let theirs = stored(&h, &theirs.id);
    make_private(&h, &other);
    let why = refused(
        &c,
        "update_ticket",
        json!({ "ticket": theirs.key, "removeFiles": [{ "index": 0 }] }),
    )
    .await;
    assert!(
        why.starts_with(&format!("Aucun ticket « {} »", theirs.key)),
        "{why}"
    );
    assert!(!why.contains("autre") && !why.contains("Le leur"), "{why}");
    assert_eq!(stored(&h, &theirs.id).files, theirs.files);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

// ---------- get_ticket and get_ticket_file ----------

#[tokio::test]
async fn get_ticket_lists_every_file_and_says_no_size_for_one_that_is_gone() {
    let h = harness("mcp-files-listed");
    let p = project(&h).await;
    let t = made(&h, &p, "Ticket", &[("a.zip", b"a"), ("b.pdf", b"bb")]).await;
    // The purge of the settings took the first.
    std::fs::remove_file(path_of(&h, &t.id, &t.files[0].file)).unwrap();
    let c = external(&h).await;
    let got = read(&c, "get_ticket", json!({ "ticket": t.key })).await;
    assert_eq!(
        got["files"],
        json!([
            { "index": 0, "file": t.files[0].file, "name": "a.zip", "size": null },
            { "index": 1, "file": t.files[1].file, "name": "b.pdf", "size": 2 },
        ])
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_file_is_read_by_its_end_or_a_part_as_text_or_base64_and_bounded() {
    let h = harness("mcp-files-read");
    let p = project(&h).await;
    let text: String = (0..8000).map(|i| format!("ligne {i} é\n")).collect();
    let t = made(
        &h,
        &p,
        "Ticket",
        &[("trace.log", text.as_bytes()), ("a.zip", ZIP)],
    )
    .await;
    let c = external(&h).await;
    // Its end, 8 KiB by default, cut at a character.
    let end = read(
        &c,
        "get_ticket_file",
        json!({ "ticket": t.key, "index": 0 }),
    )
    .await;
    let size = text.len() as u64;
    assert_eq!(end["encoding"], "text");
    assert_eq!(end["size"], size);
    assert!(text.ends_with(end["content"].as_str().unwrap()));
    let length = end["length"].as_u64().unwrap();
    assert!((8189..=8192).contains(&length), "{length}");
    assert_eq!(end["offset"].as_u64().unwrap() + length, size);
    assert_eq!(
        (&end["ticket"], &end["index"], &end["file"], &end["name"]),
        (
            &json!(t.key),
            &json!(0),
            &json!(t.files[0].file),
            &json!("trace.log")
        )
    );
    // A part, from its offset, by its file.
    let part = read(
        &c,
        "get_ticket_file",
        json!({ "ticket": t.key, "file": t.files[0].file, "offset": 0, "bytes": 20 }),
    )
    .await;
    // Byte 20 falls in « é »: the part ends before it.
    assert_eq!(part["content"], &text[..19]);
    assert_eq!(
        (part["offset"].as_u64(), part["length"].as_u64()),
        (Some(0), Some(19))
    );
    // 64 KiB at most, whatever is asked.
    let most = read(
        &c,
        "get_ticket_file",
        json!({ "ticket": t.key, "index": 0, "offset": 0, "bytes": 10_000_000 }),
    )
    .await;
    assert!(most["length"].as_u64().unwrap() <= 65_536);
    assert!(most["length"].as_u64().unwrap() >= 65_533);
    // Not text: base64, whole when it is small.
    let zip = read(
        &c,
        "get_ticket_file",
        json!({ "ticket": t.key, "index": 1 }),
    )
    .await;
    assert_eq!(zip["encoding"], "base64");
    assert_eq!(zip["content"], b64(ZIP));
    // Claude outside Escouade is told where it is, to read it whole by its own means.
    let path = PathBuf::from(zip["path"].as_str().unwrap());
    assert_eq!(std::fs::read(&path).unwrap(), ZIP);
    // The path its agent is given, not the canonical form Windows makes of it.
    assert_eq!(path, path_of(&h, &t.id, &t.files[1].file));
    // The journal has the call, never the content.
    let (summary, message) = logged(&h);
    assert!(
        summary.contains("index: 1") && !summary.contains(&b64(ZIP)),
        "{summary}"
    );
    assert_eq!(message, None);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agent_reads_the_files_of_the_projects_it_sees_and_is_given_no_path() {
    let h = harness("mcp-files-read-agent");
    let p = project(&h).await;
    let other = other_project(&h, "autre").await;
    let own = made(&h, &p, "Le sien", &[("a.txt", b"le sien")]).await;
    let theirs = made(&h, &other, "Le leur", &[("b.txt", b"le leur")]).await;
    moved(&h, &own.id, Column::Doing);
    let agent = put_agent(&h, &p, "chef", 1);
    link(&h, &agent.id, &own.id);
    let c = as_agent(&h, &agent.id).await;
    // Its own ticket, without naming it.
    let got = read(&c, "get_ticket_file", json!({ "index": 0 })).await;
    assert_eq!(got["content"], "le sien");
    assert!(got.get("path").is_none(), "{got}");
    // Another project's that is not private, without a path either.
    let got = read(
        &c,
        "get_ticket_file",
        json!({ "ticket": theirs.key, "index": 0 }),
    )
    .await;
    assert_eq!(got["content"], "le leur");
    assert!(got.get("path").is_none(), "{got}");
    // A private project's: not found, as with get_ticket.
    make_private(&h, &other);
    for (tool, args) in [
        (
            "get_ticket_file",
            json!({ "ticket": theirs.key, "index": 0 }),
        ),
        ("get_ticket", json!({ "ticket": theirs.key })),
    ] {
        let why = refused(&c, tool, args).await;
        assert!(
            why.starts_with(&format!("Aucun ticket « {} »", theirs.key)),
            "{tool}: {why}"
        );
        assert!(
            !why.contains("le leur") && !why.contains("autre"),
            "{tool}: {why}"
        );
    }
    // Claude outside Escouade must name the ticket.
    let outside = external(&h).await;
    let why = refused(&outside, "get_ticket_file", json!({ "index": 0 })).await;
    assert!(why.contains("clé ou l’id"), "{why}");
    c.cancel().await.unwrap();
    outside.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_file_that_is_not_the_tickets_or_is_gone_is_refused_with_a_text() {
    let h = harness("mcp-files-read-refused");
    let p = project(&h).await;
    let t = made(&h, &p, "Ticket", &[("a.txt", b"a"), ("b.txt", b"b")]).await;
    // A file of the ticket's folder that is not in its list is no file of it.
    let stray = h.core.data.ticket_files(&t.id).join("99999999");
    std::fs::create_dir_all(&stray).unwrap();
    std::fs::write(stray.join("x.txt"), b"x").unwrap();
    let c = external(&h).await;
    for (args, said) in [
        (
            json!({ "ticket": t.key, "file": "99999999/x.txt" }),
            "get_ticket",
        ),
        (
            json!({ "ticket": t.key, "file": "../../state.json" }),
            "get_ticket",
        ),
        (json!({ "ticket": t.key, "index": 5 }), "0 à 1"),
        (json!({ "ticket": t.key }), "index"),
    ] {
        let why = refused(&c, "get_ticket_file", args.clone()).await;
        assert!(why.contains(said), "{args}: {why}");
    }
    std::fs::remove_file(path_of(&h, &t.id, &t.files[1].file)).unwrap();
    let why = refused(
        &c,
        "get_ticket_file",
        json!({ "ticket": t.key, "index": 1 }),
    )
    .await;
    assert_eq!(
        why,
        "Le fichier « b.txt » ne peut pas être lu : il a disparu du dossier du ticket ou n’est pas lisible."
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

// ---------- split_ticket ----------

/// A ticket in progress with these files, and its agent's client.
async fn splitting(h: &Harness, p: &Project, files: &[(&str, &[u8])]) -> (Ticket, Client) {
    let own = made(h, p, "Gros ticket", files).await;
    moved(h, &own.id, Column::Doing);
    let agent = put_agent(h, p, "gros", 1);
    link(h, &agent.id, &own.id);
    (stored(h, &own.id), as_agent(h, &agent.id).await)
}

fn ticket_of(h: &Harness, key: &str) -> Ticket {
    h.core
        .tickets
        .read()
        .iter()
        .find(|t| t.key == key)
        .cloned()
        .unwrap()
}

/// The names and contents of a ticket's files.
fn contents(h: &Harness, t: &Ticket) -> Vec<(String, Vec<u8>)> {
    t.files
        .iter()
        .map(|f| {
            (
                f.name.clone(),
                std::fs::read(path_of(h, &t.id, &f.file)).unwrap(),
            )
        })
        .collect()
}

#[tokio::test]
async fn the_tickets_of_a_split_get_copies_of_all_the_files_of_their_origin_by_default() {
    let h = harness("mcp-files-split-default");
    let p = project(&h).await;
    let (own, c) = splitting(&h, &p, &[("a.zip", ZIP), ("b.csv", b"x;y")]).await;
    let keys = read(
        &c,
        "split_ticket",
        json!({ "tickets": [{ "title": "Étape 1" }, { "title": "Étape 2" }] }),
    )
    .await;
    assert_eq!(keys, json!(["DEM-2", "DEM-3"]));
    let expected = vec![
        ("a.zip".to_string(), ZIP.to_vec()),
        ("b.csv".into(), b"x;y".to_vec()),
    ];
    let mut all = own.files.iter().map(|f| f.file.clone()).collect::<Vec<_>>();
    for key in ["DEM-2", "DEM-3"] {
        let t = ticket_of(&h, key);
        assert_eq!(contents(&h, &t), expected);
        all.extend(t.files.iter().map(|f| f.file.clone()));
    }
    // Copies, not the same files; the origin keeps its own.
    let distinct: std::collections::HashSet<_> = all.iter().collect();
    assert_eq!(distinct.len(), 6, "{all:?}");
    assert_eq!(contents(&h, &stored(&h, &own.id)), expected);
    assert_eq!(drafts_left(&h), Vec::<String>::new());
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn each_ticket_of_a_split_takes_the_files_it_names_or_none() {
    let h = harness("mcp-files-split-select");
    let p = project(&h).await;
    let (own, c) = splitting(&h, &p, &[("a.txt", b"a"), ("b.txt", b"b"), ("c.txt", b"c")]).await;
    read(
        &c,
        "split_ticket",
        json!({ "tickets": [
            { "title": "Un", "files": [{ "index": 2 }, { "file": own.files[0].file }] },
            { "title": "Deux", "files": [] },
        ] }),
    )
    .await;
    let names = |key: &str| -> Vec<String> {
        ticket_of(&h, key)
            .files
            .iter()
            .map(|f| f.name.clone())
            .collect()
    };
    assert_eq!(names("DEM-2"), ["c.txt", "a.txt"]);
    assert!(names("DEM-3").is_empty());
    let (summary, _) = logged(&h);
    assert!(summary.contains("files: 2"), "{summary}");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_split_by_default_leaves_out_a_file_that_is_gone_or_too_big_and_says_so() {
    let h = harness("mcp-files-split-skipped");
    let p = project(&h).await;
    let (own, c) = splitting(
        &h,
        &p,
        &[
            ("a.txt", b"a"),
            ("gone.txt", b"g"),
            ("gros.bin", &vec![0u8; (1 << 20) + 1]),
        ],
    )
    .await;
    std::fs::remove_file(path_of(&h, &own.id, &own.files[1].file)).unwrap();
    // The settings now allow less than when it was joined.
    h.core.settings.write().attach_max_mb = 1;
    let answer = read(
        &c,
        "split_ticket",
        json!({ "tickets": [{ "title": "Suite" }] }),
    )
    .await;
    assert_eq!(answer["tickets"], json!(["DEM-2"]));
    assert_eq!(
        answer["skipped"],
        json!([
            "DEM-2 : le fichier « gone.txt » n’a pas été copié, son fichier a disparu ou n’est pas lisible.",
            "DEM-2 : le fichier « gros.bin » n’a pas été copié, il dépasse 1 Mo, la taille maximale d’un fichier joint des réglages.",
        ])
    );
    let t = ticket_of(&h, "DEM-2");
    assert_eq!(contents(&h, &t), [("a.txt".to_string(), b"a".to_vec())]);
    assert_eq!(drafts_left(&h), Vec::<String>::new());
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_split_that_names_a_file_it_cannot_hand_down_makes_no_ticket_at_all() {
    let h = harness("mcp-files-split-named");
    let p = project(&h).await;
    let (own, c) = splitting(&h, &p, &[("a.txt", b"a"), ("gone.txt", b"g")]).await;
    std::fs::remove_file(path_of(&h, &own.id, &own.files[1].file)).unwrap();
    let before = board(&h, &p);
    for files in [
        json!([{ "index": 1 }]),
        json!([{ "index": 7 }]),
        json!([{ "file": "../x" }]),
    ] {
        let why = refused(
            &c,
            "split_ticket",
            json!({ "tickets": [{ "title": "Un" }, { "title": "Deux", "files": files }] }),
        )
        .await;
        assert!(why.starts_with("Ticket 2 de la liste : "), "{why}");
        assert_eq!(board(&h, &p), before);
    }
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

// ---------- the schemas ----------

#[tokio::test]
async fn the_schemas_say_what_a_file_is_and_how_one_is_named_and_read() {
    let h = harness("mcp-files-schemas");
    let c = external(&h).await;
    let tools = c.list_all_tools().await.unwrap();
    let tool =
        |name: &str| serde_json::to_value(tools.iter().find(|t| t.name == name).unwrap()).unwrap();
    let keys = |schema: &Value| -> Vec<String> {
        let mut keys: Vec<String> = schema["properties"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        keys.sort();
        keys
    };
    let described = |schema: &Value| {
        for (field, spec) in schema["properties"].as_object().unwrap() {
            assert!(
                spec["description"]
                    .as_str()
                    .is_some_and(|d| d.len() > 10 && !d.contains('\n')),
                "{field}: {spec}"
            );
        }
    };
    let create = tool("create_ticket")["inputSchema"].clone();
    let update = tool("update_ticket")["inputSchema"].clone();
    let split = tool("split_ticket")["inputSchema"].clone();
    let read_one = tool("get_ticket_file")["inputSchema"].clone();
    for file in [
        &create["properties"]["files"]["items"],
        &update["properties"]["addFiles"]["items"],
    ] {
        assert_eq!(keys(file), ["data", "name", "path"], "{file}");
        described(file);
    }
    for reference in [
        &update["properties"]["removeFiles"]["items"],
        &split["properties"]["tickets"]["items"]["properties"]["files"]["items"],
    ] {
        assert_eq!(keys(reference), ["file", "index"], "{reference}");
        described(reference);
    }
    assert_eq!(
        keys(&read_one),
        ["bytes", "file", "index", "offset", "ticket"]
    );
    described(&read_one);
    for (name, said) in [
        (
            "create_ticket",
            ["files", "any format", "Settings › Advanced › Attachments"].as_slice(),
        ),
        ("update_ticket", &["addFiles", "removeFiles"]),
        ("get_ticket", &["files", "get_ticket_file"]),
        ("split_ticket", &["files", "skipped"]),
        (
            "get_ticket_file",
            &["offset", "base64", "path", "own project", "65536"],
        ),
    ] {
        let description = tool(name)["description"].as_str().unwrap().to_string();
        for word in said {
            assert!(
                description.contains(word),
                "{name} lacks {word}: {description}"
            );
        }
    }
    let read_one = tool("get_ticket_file");
    assert_eq!(read_one["annotations"]["readOnlyHint"], true);
    // A call whose arguments do not fit is answered without the value given.
    let (text, error) = call(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "x", "files": ["C:/Users/me/secret.xlsx"] }),
    )
    .await;
    assert!(error && !text.contains("secret.xlsx"), "{text}");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}
