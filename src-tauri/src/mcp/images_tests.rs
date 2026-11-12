//! The images of a ticket through the server (ESC-102), called by a real MCP client (rmcp's) on a
//! test core: `create_ticket` and `update_ticket` attach and take away, `get_ticket` lists,
//! `get_ticket_image` gives one back, `split_ticket` hands copies down. The limits and the refusals
//! are the window's (`ticketimages::check`), so a refused call leaves nothing behind: no ticket, no
//! key used up, no file, no folder.

use super::private_tests::make_private;
#[cfg(windows)]
use super::tools_tests::junction;
use super::tools_tests::{
    as_agent, call, external, last_entry, link, other_project, project, put_agent, read, refused,
    Client,
};
use crate::core::MB;
use crate::core_tests::{harness, Harness};
use crate::model::*;
use crate::tickets::{DraftImage, TicketDraft};
use base64::Engine;
use rmcp::model::CallToolRequestParams;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// The start of a PNG, of a JPEG and of a GIF: what makes a content one.
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n-capture";
const JPEG: &[u8] = b"\xff\xd8\xff\xe0-photo";
const GIF: &[u8] = b"GIF89a-animation";

/// A PNG of `size` bytes.
fn png_of(size: usize) -> Vec<u8> {
    let mut bytes = PNG.to_vec();
    bytes.resize(size, 7);
    bytes
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// An image as the tools take it, by its content.
fn shot(name: &str, bytes: &[u8]) -> Value {
    json!({ "name": name, "mediaType": "image/png", "data": b64(bytes) })
}

/// The files of a folder, by name (none when it is not there).
fn files_of(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|d| {
            d.map(|e| e.unwrap().file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// A ticket « À faire » made as the window makes one, with these images (name, content).
async fn made(h: &Harness, p: &Project, title: &str, images: &[(&str, &[u8])]) -> Ticket {
    h.core
        .ticket_create(
            &p.id,
            TicketDraft {
                title: title.into(),
                images: Some(
                    images
                        .iter()
                        .map(|(name, bytes)| DraftImage {
                            name: (*name).into(),
                            media_type: "image/png".into(),
                            data: b64(bytes),
                            ..Default::default()
                        })
                        .collect(),
                ),
                ..Default::default()
            },
        )
        .await
        .unwrap()
}

fn stored(h: &Harness, id: &str) -> Ticket {
    h.core.ticket(id).unwrap()
}

fn dir_of(h: &Harness, id: &str) -> PathBuf {
    h.core.data.ticket_dir(id)
}

/// The ticket put in `column`, as its agent's work would have.
fn moved(h: &Harness, id: &str, column: Column) {
    h.core
        .edit_ticket(id, |t| {
            t.column = column;
            Ok(())
        })
        .unwrap();
}

/// What a refused call must leave as it was: the tickets, the next key, the folders of `tickets/`.
#[derive(Debug, PartialEq)]
struct Board {
    tickets: usize,
    next: u32,
    folders: Vec<String>,
}

fn board(h: &Harness, p: &Project) -> Board {
    Board {
        tickets: h.core.tickets.read().len(),
        next: h.core.project(&p.id).unwrap().board.next_number,
        folders: files_of(&h.core.data.tickets()),
    }
}

/// The images `get_ticket` lists for `key`.
async fn listed(c: &Client, key: &str) -> Value {
    read(c, "get_ticket", json!({ "ticket": key })).await["images"].clone()
}

/// What `get_ticket_image` answers to `args`: its blocks, and whether it is an error the model reads.
async fn blocks(c: &Client, args: Value) -> (Vec<Value>, bool) {
    let mut params = CallToolRequestParams::new("get_ticket_image");
    if let Value::Object(args) = args {
        params = params.with_arguments(args);
    }
    let answer = serde_json::to_value(c.call_tool(params).await.unwrap()).unwrap();
    (
        answer["content"].as_array().unwrap().clone(),
        answer["isError"] == true,
    )
}

/// The activity log's last entry: its summary and its message.
fn logged(h: &Harness) -> (String, Option<String>) {
    let e = h.core.mcp.activity.entries().pop().unwrap();
    (e.summary, e.message)
}

/// What the core says when a ticket’s images weigh more than 18 MB, and what a file outside an
/// agent’s folder, there or not, is answered.
const TOTAL_REFUSED: &str = "Les images d’un ticket sont limitées à 18 Mo en tout.";
const OUTSIDE: &str = "Un agent d’Escouade ne joint que des fichiers de son dossier de travail : ce fichier n’en fait pas partie, ou il n’existe pas.";

// ---------- create_ticket ----------

#[tokio::test]
async fn images_given_by_content_are_attached_to_a_new_ticket_and_listed_by_get_ticket() {
    let h = harness("mcp-images-create-data");
    let p = project(&h).await;
    let c = external(&h).await;
    let created = read(
        &c,
        "create_ticket",
        json!({
            "project": "demo",
            "title": "Corriger le bouton",
            "images": [
                shot("capture.png", PNG),
                // Declared a PNG, it is a JPEG: the content says what it is, as in the window.
                shot("maquette.png", JPEG),
            ],
        }),
    )
    .await;
    let images = created["images"].as_array().unwrap().clone();
    assert_eq!(images.len(), 2, "{created}");
    let (f0, f1) = (
        images[0]["file"].as_str().unwrap(),
        images[1]["file"].as_str().unwrap(),
    );
    // The file is one of its own, never the name given.
    assert!(f0.ends_with(".png") && f1.ends_with(".jpg"), "{f0} {f1}");
    assert_eq!(
        created["images"],
        json!([
            { "index": 0, "file": f0, "name": "capture.png", "mediaType": "image/png", "size": PNG.len() },
            { "index": 1, "file": f1, "name": "maquette.png", "mediaType": "image/jpeg", "size": JPEG.len() },
        ])
    );
    // The files are in the ticket's folder, with the content they were given.
    let dir = dir_of(&h, created["id"].as_str().unwrap());
    assert_eq!(files_of(&dir), {
        let mut both = vec![f0.to_string(), f1.to_string()];
        both.sort();
        both
    });
    assert_eq!(std::fs::read(dir.join(f0)).unwrap(), PNG);
    assert_eq!(std::fs::read(dir.join(f1)).unwrap(), JPEG);
    // The journal says how many, never what.
    let (summary, message) = logged(&h);
    assert!(summary.contains("images: 2"), "{summary}");
    assert!(!summary.contains(&b64(PNG)) && !summary.contains(&b64(JPEG)));
    assert_eq!(message, None);
    // `get_ticket` lists the same.
    assert_eq!(listed(&c, "DEM-1").await, created["images"]);
    assert_eq!(stored(&h, created["id"].as_str().unwrap()).images.len(), 2);
    assert_eq!(h.core.project(&p.id).unwrap().board.next_number, 2);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_made_without_images_is_as_it_was() {
    let h = harness("mcp-images-create-none");
    let _p = project(&h).await;
    let c = external(&h).await;
    let plain = read(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "Sans image" }),
    )
    .await;
    assert_eq!(plain["images"], json!([]));
    assert!(!logged(&h).0.contains("images"), "{}", logged(&h).0);
    // An empty list is none, and no folder is made for it.
    let empty = read(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "Liste vide", "images": [] }),
    )
    .await;
    assert_eq!(empty["images"], json!([]));
    assert_eq!(files_of(&h.core.data.tickets()), Vec::<String>::new());
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn images_given_by_path_are_read_from_their_file_and_the_path_is_not_kept() {
    let h = harness("mcp-images-create-path");
    let _p = project(&h).await;
    let c = external(&h).await;
    let shots = h.dir.join("captures");
    std::fs::create_dir_all(&shots).unwrap();
    let one = shots.join("écran 1.jpeg");
    let two = shots.join("sans-extension");
    let three = shots.join("animation.png");
    std::fs::write(&one, JPEG).unwrap();
    std::fs::write(&two, PNG).unwrap();
    std::fs::write(&three, GIF).unwrap();
    let created = read(
        &c,
        "create_ticket",
        json!({
            "project": "demo",
            "title": "Par chemin",
            "images": [
                { "path": one },
                { "path": two, "name": "Le bouton.png" },
                // Its extension says PNG: it is a GIF.
                { "path": three, "mediaType": "image/png" },
            ],
        }),
    )
    .await;
    let images = created["images"].as_array().unwrap().clone();
    assert_eq!(images.len(), 3, "{created}");
    let describe = |i: &Value| {
        (
            i["name"].as_str().unwrap().to_string(),
            i["mediaType"].as_str().unwrap().to_string(),
            i["size"].as_u64().unwrap(),
        )
    };
    let expected = |name: &str, media_type: &str, size: usize| {
        (name.to_string(), media_type.to_string(), size as u64)
    };
    assert_eq!(
        images.iter().map(describe).collect::<Vec<_>>(),
        [
            expected("écran 1.jpeg", "image/jpeg", JPEG.len()),
            expected("Le bouton.png", "image/png", PNG.len()),
            expected("animation.png", "image/gif", GIF.len()),
        ]
    );
    let dir = dir_of(&h, created["id"].as_str().unwrap());
    assert_eq!(
        std::fs::read(dir.join(images[0]["file"].as_str().unwrap())).unwrap(),
        JPEG
    );
    // A copy: the ticket keeps nothing of the path, and neither does the journal.
    assert!(
        !serde_json::to_string(&stored(&h, created["id"].as_str().unwrap()))
            .unwrap()
            .contains("captures")
    );
    let (summary, message) = logged(&h);
    assert!(summary.contains("images: 3"), "{summary}");
    assert!(!summary.contains("captures") && !summary.contains("sans-extension"));
    assert_eq!(message, None);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_new_ticket_refuses_what_the_window_refuses_and_nothing_is_made() {
    let h = harness("mcp-images-create-refused");
    let p = project(&h).await;
    let c = external(&h).await;
    let before = board(&h, &p);
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>";
    let cases: Vec<(&str, Vec<Value>, &str)> = vec![
        (
            "a BMP",
            vec![shot("plan.bmp", b"BM6\0\0\0")],
            "« plan.bmp » n’est pas une image : un ticket prend les images PNG, JPEG, GIF et WebP.",
        ),
        (
            "a text",
            vec![shot("notes.png", b"Une note, pas une image.")],
            "« notes.png » n’est pas une image : un ticket prend les images PNG, JPEG, GIF et WebP.",
        ),
        (
            "an SVG",
            vec![shot("logo.svg", svg)],
            "« logo.svg » n’est pas une image : un ticket prend les images PNG, JPEG, GIF et WebP.",
        ),
        (
            "an image over 5 MB",
            vec![shot("photo.png", &png_of(5 * MB + 1))],
            "photo.png dépasse 5 Mo",
        ),
        (
            "images over 18 MB in all",
            vec![
                shot("a.png", &png_of(5 * MB)),
                shot("b.png", &png_of(5 * MB)),
                shot("c.png", &png_of(5 * MB)),
                shot("d.png", &png_of(4 * MB)),
            ],
            "Les images d’un ticket sont limitées à 18 Mo en tout.",
        ),
        (
            "21 images",
            (0..21).map(|i| shot(&format!("{i}.png"), PNG)).collect(),
            "Un ticket prend 20 images au plus.",
        ),
        (
            "a content that is not base64",
            vec![json!({ "name": "abime.png", "data": "pas du base64 !" })],
            "abime.png n’a pas pu être lu",
        ),
    ];
    for (what, images, said) in cases {
        let got = refused(
            &c,
            "create_ticket",
            json!({ "project": "demo", "title": "Refusé", "images": images }),
        )
        .await;
        assert_eq!(got, said, "{what}");
        assert_eq!(board(&h, &p), before, "{what}");
        let (_, _, _, outcome) = last_entry(&h);
        assert_eq!(outcome, "error", "{what}");
    }
    // 20 are taken: the limit is a count, not a refusal of many.
    let twenty = read(
        &c,
        "create_ticket",
        json!({
            "project": "demo",
            "title": "Vingt",
            "images": (0..20).map(|i| shot(&format!("{i}.png"), PNG)).collect::<Vec<_>>(),
        }),
    )
    .await;
    assert_eq!(twenty["images"].as_array().unwrap().len(), 20);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_image_is_one_of_a_content_or_a_path_with_a_name_that_is_a_label() {
    let h = harness("mcp-images-create-args");
    let p = project(&h).await;
    let c = external(&h).await;
    let before = board(&h, &p);
    let long = "x".repeat(101);
    // The second image is the bad one: the answer says which.
    let bad = |image: Value| json!({ "project": "demo", "title": "Refusé", "images": [shot("ok.png", PNG), image] });
    let cases: Vec<(&str, Value, String)> = vec![
        (
            "no name with a content",
            json!({ "data": b64(PNG) }),
            "Image 2 de la liste : Une image donnée par son contenu (data) a besoin d’un nom (name).".into(),
        ),
        (
            "a content and a path",
            json!({ "name": "a.png", "data": b64(PNG), "path": "C:\\a.png" }),
            "Image 2 de la liste : Donne le contenu de l’image (data) ou le chemin de son fichier (path), pas les deux.".into(),
        ),
        (
            "neither",
            json!({ "name": "a.png" }),
            "Image 2 de la liste : Donne le contenu de l’image (data) ou le chemin de son fichier (path).".into(),
        ),
        (
            "an empty name",
            json!({ "name": "  ", "data": b64(PNG) }),
            "Image 2 de la liste : Le nom de l’image est vide.".into(),
        ),
        (
            "a name that is only a folder",
            json!({ "name": "dossier/", "data": b64(PNG) }),
            "Image 2 de la liste : Le nom de l’image est vide.".into(),
        ),
        (
            "a name of 101 characters",
            json!({ "name": long, "data": b64(PNG) }),
            "Image 2 de la liste : Le nom de l’image fait 101 caractères : 100 au plus.".into(),
        ),
    ];
    for (what, image, said) in cases {
        let got = refused(&c, "create_ticket", bad(image)).await;
        assert_eq!(got, said, "{what}");
        assert_eq!(board(&h, &p), before, "{what}");
    }
    // The name is a label: only what follows the last slash, on one line.
    let created = read(
        &c,
        "create_ticket",
        json!({
            "project": "demo",
            "title": "Noms",
            "images": [
                shot("../../etc/passwd.png", PNG),
                shot(r"C:\Users\moi\écran.png", PNG),
                shot("ligne 1\nligne 2.png", PNG),
                shot(&"é".repeat(100), PNG),
            ],
        }),
    )
    .await;
    let names: Vec<&str> = created["images"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "passwd.png",
            "écran.png",
            "ligne 1 ligne 2.png",
            &"é".repeat(100)
        ]
    );
    // And so it is in the first message of the agent: no way up in it.
    let ticket = stored(&h, created["id"].as_str().unwrap());
    assert!(ticket
        .images
        .iter()
        .all(|i| !i.name.contains(['/', '\\', '\n'])));
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_path_that_leads_nowhere_good_is_refused_and_nothing_is_made() {
    let h = harness("mcp-images-create-paths");
    let p = project(&h).await;
    let c = external(&h).await;
    let before = board(&h, &p);
    let work = h.dir.join("captures");
    std::fs::create_dir_all(&work).unwrap();
    let text = work.join("notes.txt");
    std::fs::write(&text, "Une note.").unwrap();
    let big = work.join("grosse.png");
    std::fs::write(&big, png_of(5 * MB + 1)).unwrap();
    let gone = work.join("disparue.png");
    let path = |p: &Path| p.to_string_lossy().to_string();
    let cases: Vec<(&str, String, &str)> = vec![
        ("a relative path", "capture.png".into(), "absolu"),
        ("a relative path up", "../capture.png".into(), "absolu"),
        ("a UNC path", r"\\serveur\partage\a.png".into(), "réseau"),
        ("a UNC path with slashes", "//x/y.png".into(), "réseau"),
        ("a device path", r"\\?\C:\a.png".into(), "réseau"),
        ("a mixed UNC path", r"/\x\y.png".into(), "réseau"),
        ("a file that is not there", path(&gone), "introuvable"),
        ("a folder", path(&work), "pas un fichier"),
        (
            "a file that is no image",
            path(&text),
            "« notes.txt » n’est pas une image",
        ),
        ("a file over 5 MB", path(&big), "grosse.png dépasse 5 Mo"),
    ];
    for (what, path, said) in cases {
        let got = refused(
            &c,
            "create_ticket",
            json!({ "project": "demo", "title": "Refusé", "images": [shot("ok.png", PNG), { "path": path }] }),
        )
        .await;
        // The window’s own refusals come as they are, the others say which image of the list.
        let from_the_core = said.contains("n’est pas une image");
        assert!(
            got.contains(said) && (from_the_core || got.starts_with("Image 2 de la liste : ")),
            "{what}: {got}"
        );
        assert_eq!(board(&h, &p), before, "{what}");
        // The journal says why in the same words, and has no path in them.
        let (summary, message) = logged(&h);
        assert!(!summary.contains(&path), "{what}: {summary}");
        assert!(message.is_some_and(|m| !m.contains(&path)), "{what}");
    }
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn paths_past_the_count_a_ticket_takes_are_refused_without_reading_them() {
    let h = harness("mcp-images-create-many-paths");
    let p = project(&h).await;
    let c = external(&h).await;
    let before = board(&h, &p);
    let file = h.dir.join("capture.png");
    std::fs::write(&file, PNG).unwrap();
    let mut images: Vec<Value> = (0..20).map(|_| json!({ "path": file })).collect();
    // The 21st file is not there: had it been read, that is what would be refused.
    images.push(json!({ "path": h.dir.join("disparue.png") }));
    let got = refused(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "Trop", "images": images }),
    )
    .await;
    assert_eq!(got, "Un ticket prend 20 images au plus.");
    assert_eq!(board(&h, &p), before);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agent_of_escouade_gives_only_the_files_of_its_working_folder() {
    let h = harness("mcp-images-create-agent");
    let p = project(&h).await;
    let work = h.dir.join("travail");
    std::fs::create_dir_all(work.join("sous")).unwrap();
    std::fs::write(work.join("sous").join("dedans.png"), PNG).unwrap();
    std::fs::write(h.dir.join("dehors.png"), PNG).unwrap();
    let agent = put_agent(&h, &p, "chef", 1);
    h.core.agent(&agent.id).unwrap().lock().meta.cwd = work.to_string_lossy().to_string();
    let c = as_agent(&h, &agent.id).await;
    let inside = work.join("sous").join("dedans.png");
    let created = read(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "Dedans", "images": [{ "path": inside }] }),
    )
    .await;
    assert_eq!(created["images"][0]["name"], "dedans.png");
    // Outside, or out and back by a way up: refused, and the tool says what it is about.
    let before = board(&h, &p);
    // A file that is not there says the same as one that is: nothing is learnt of the disk beyond.
    for outside in [
        h.dir.join("dehors.png"),
        work.join("sous").join("..").join("..").join("dehors.png"),
        h.dir.join("absent.png"),
        work.join("..").join("absent.png"),
        h.dir.join("nulle part").join("absent.png"),
    ] {
        let got = refused(
            &c,
            "create_ticket",
            json!({ "project": "demo", "title": "Dehors", "images": [{ "path": outside }] }),
        )
        .await;
        assert_eq!(
            got,
            format!("Image 1 de la liste : {OUTSIDE}"),
            "{outside:?}"
        );
        assert_eq!(last_entry(&h).3, "refused");
        assert_eq!(board(&h, &p), before);
    }
    // Under its folder and not there: it is not found, which tells nothing.
    let got = refused(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "Absent", "images": [{ "path": work.join("sous").join("absent.png") }] }),
    )
    .await;
    assert!(
        got.starts_with("Image 1 de la liste : ") && got.contains("introuvable"),
        "{got}"
    );
    assert_eq!(last_entry(&h).3, "error");
    assert_eq!(board(&h, &p), before);
    // What an agent gives by content is its own, wherever it comes from.
    read(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "Contenu", "images": [shot("a.png", PNG)] }),
    )
    .await;
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[cfg(unix)]
#[tokio::test]
async fn a_link_out_of_its_working_folder_does_not_lead_an_agent_out() {
    let h = harness("mcp-images-create-link");
    let p = project(&h).await;
    let work = h.dir.join("travail");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::write(h.dir.join("secret.png"), PNG).unwrap();
    std::os::unix::fs::symlink(h.dir.join("secret.png"), work.join("lien.png")).unwrap();
    let agent = put_agent(&h, &p, "chef", 1);
    h.core.agent(&agent.id).unwrap().lock().meta.cwd = work.to_string_lossy().to_string();
    let c = as_agent(&h, &agent.id).await;
    let before = board(&h, &p);
    let got = refused(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "Lien", "images": [{ "path": work.join("lien.png") }] }),
    )
    .await;
    assert!(got.contains("dossier de travail"), "{got}");
    assert_eq!(board(&h, &p), before);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[cfg(windows)]
#[tokio::test]
async fn a_junction_out_of_its_working_folder_does_not_lead_an_agent_out() {
    let h = harness("mcp-images-create-junction");
    let p = project(&h).await;
    let (work, outside) = (h.dir.join("travail"), h.dir.join("dehors"));
    std::fs::create_dir_all(&work).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.png"), PNG).unwrap();
    let _link = junction(&work.join("lien"), &outside);
    let agent = put_agent(&h, &p, "chef", 1);
    h.core.agent(&agent.id).unwrap().lock().meta.cwd = work.to_string_lossy().to_string();
    let c = as_agent(&h, &agent.id).await;
    let before = board(&h, &p);
    let got = refused(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "Jonction", "images": [{ "path": work.join("lien").join("secret.png") }] }),
    )
    .await;
    assert_eq!(got, format!("Image 1 de la liste : {OUTSIDE}"));
    assert_eq!(last_entry(&h).3, "refused");
    assert_eq!(board(&h, &p), before);
    // Claude outside Escouade follows it: the file is one of the machine.
    c.cancel().await.unwrap();
    let outside_client = external(&h).await;
    let created = read(
        &outside_client,
        "create_ticket",
        json!({ "project": "demo", "title": "Suivie", "images": [{ "path": work.join("lien").join("secret.png") }] }),
    )
    .await;
    assert_eq!(created["images"][0]["name"], "secret.png");
    outside_client.cancel().await.unwrap();
    h.core.mcp.stop();
}

// ---------- the weight of a list ----------

#[tokio::test]
async fn the_files_of_a_new_ticket_are_not_read_once_they_weigh_more_than_it_takes() {
    let h = harness("mcp-images-total-create");
    let p = project(&h).await;
    let c = external(&h).await;
    let before = board(&h, &p);
    let shots = h.dir.join("captures");
    std::fs::create_dir_all(&shots).unwrap();
    let big = shots.join("grosse.png");
    std::fs::write(&big, png_of(5 * MB)).unwrap();
    let gone = shots.join("disparue.png");
    // The fourth takes the list past 18 MB. The 16 after it are not there: had they been looked at,
    // that is what would be said.
    let mut images: Vec<Value> = (0..4).map(|_| json!({ "path": big })).collect();
    images.extend((0..16).map(|_| json!({ "path": gone })));
    let got = refused(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "Lourd", "images": images }),
    )
    .await;
    assert_eq!(got, TOTAL_REFUSED);
    assert_eq!(board(&h, &p), before);
    let (summary, message) = logged(&h);
    assert!(!summary.contains("captures"), "{summary}");
    assert_eq!(message.as_deref(), Some(TOTAL_REFUSED));
    // Exactly 18 MB is still taken: 5 + 5 + 5 + 3.
    let three = shots.join("trois.png");
    std::fs::write(&three, png_of(3 * MB)).unwrap();
    let created = read(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "Pile", "images": [{ "path": big }, { "path": big }, { "path": big }, { "path": three }] }),
    )
    .await;
    let sizes: u64 = created["images"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["size"].as_u64().unwrap())
        .sum();
    assert_eq!(sizes, 18 * MB as u64);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_images_a_ticket_keeps_weigh_in_what_an_update_reads() {
    let h = harness("mcp-images-total-update");
    let p = project(&h).await;
    let five = png_of(5 * MB);
    let t = made(
        &h,
        &p,
        "Lourd",
        &[("a.png", &five), ("b.png", &five), ("c.png", &five)],
    )
    .await;
    let dir = dir_of(&h, &t.id);
    let c = external(&h).await;
    let shots = h.dir.join("captures");
    std::fs::create_dir_all(&shots).unwrap();
    let big = shots.join("grosse.png");
    std::fs::write(&big, &five).unwrap();
    let gone = shots.join("disparue.png");
    let (before_files, before_images) = (files_of(&dir), stored(&h, &t.id).images);
    // 15 MB are kept: the first file takes it past 18, and the next ones are never looked at.
    let got = refused(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "addImages": [{ "path": big }, { "path": gone }, { "path": gone }] }),
    )
    .await;
    assert_eq!(got, TOTAL_REFUSED);
    assert_eq!(stored(&h, &t.id).images, before_images);
    assert_eq!(files_of(&dir), before_files);
    // One taken away makes room for one: 10 kept and 5 added make 15 ...
    let got = refused(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "removeImages": [{ "index": 0 }], "addImages": [{ "path": big }, { "path": big }, { "path": gone }] }),
    )
    .await;
    // ... and the second takes it to 20, before the file that is not there is looked at.
    assert_eq!(got, TOTAL_REFUSED);
    assert_eq!(stored(&h, &t.id).images, before_images);
    assert_eq!(files_of(&dir), before_files);
    let updated = read(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "removeImages": [{ "index": 0 }], "addImages": [{ "path": big }] }),
    )
    .await;
    let names: Vec<&str> = updated["images"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["b.png", "c.png", "grosse.png"]);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

// ---------- update_ticket ----------

#[tokio::test]
async fn images_are_added_to_a_ticket_after_the_ones_it_has() {
    let h = harness("mcp-images-update-add");
    let p = project(&h).await;
    let t = made(&h, &p, "Avec une image", &[("une.png", PNG)]).await;
    let c = external(&h).await;
    let updated = read(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "addImages": [shot("deux.png", &png_of(40)), { "name": "trois.gif", "data": b64(GIF) }] }),
    )
    .await;
    let images = updated["images"].as_array().unwrap().clone();
    let names: Vec<&str> = images.iter().map(|i| i["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["une.png", "deux.png", "trois.gif"], "{updated}");
    assert_eq!(images[2]["mediaType"], "image/gif");
    // The one it had is the same file; the others are new ones, all in its folder.
    assert_eq!(images[0]["file"], t.images[0].file.as_str());
    assert_eq!(
        images
            .iter()
            .map(|i| i["index"].clone())
            .collect::<Vec<_>>(),
        [json!(0), json!(1), json!(2)]
    );
    let dir = dir_of(&h, &t.id);
    assert_eq!(files_of(&dir).len(), 3);
    assert_eq!(
        std::fs::read(dir.join(images[2]["file"].as_str().unwrap())).unwrap(),
        GIF
    );
    assert_eq!(stored(&h, &t.id).images.len(), 3);
    // Images alone: the words of the ticket are not touched, and no line says it was edited.
    assert_eq!(updated["title"], "Avec une image");
    assert_eq!(updated["description"], t.description.as_str());
    assert!(!updated["description"]
        .as_str()
        .unwrap()
        .contains("Modifié par"));
    let (summary, message) = logged(&h);
    assert!(summary.contains("addImages: 2"), "{summary}");
    assert!(!summary.contains("removeImages"), "{summary}");
    assert!(!summary.contains(&b64(GIF)));
    assert_eq!(message, None);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn images_are_taken_away_by_file_or_by_rank_and_the_others_stay() {
    let h = harness("mcp-images-update-remove");
    let p = project(&h).await;
    let t = made(
        &h,
        &p,
        "Cinq images",
        &[
            ("a.png", PNG),
            ("b.png", &png_of(30)),
            ("c.png", &png_of(31)),
            ("d.png", &png_of(32)),
            ("e.png", &png_of(33)),
        ],
    )
    .await;
    let dir = dir_of(&h, &t.id);
    let file = |i: usize| t.images[i].file.clone();
    let c = external(&h).await;
    // By file.
    let by_file = read(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "removeImages": [{ "file": file(1) }] }),
    )
    .await;
    let names = |v: &Value| -> Vec<String> {
        v["images"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["name"].as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(names(&by_file), ["a.png", "c.png", "d.png", "e.png"]);
    // The ranks are those of the list now: they follow it.
    assert_eq!(by_file["images"][1]["index"], 1);
    assert!(!dir.join(file(1)).exists());
    assert!(dir.join(file(0)).exists() && dir.join(file(2)).exists());
    let (summary, _) = logged(&h);
    assert!(
        summary.contains("removeImages: 1") && !summary.contains("addImages"),
        "{summary}"
    );
    // By rank, from 0, and two at once.
    let by_index = read(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "removeImages": [{ "index": 0 }, { "index": 3 }] }),
    )
    .await;
    assert_eq!(names(&by_index), ["c.png", "d.png"]);
    assert_eq!(files_of(&dir), {
        let mut left = vec![file(2), file(3)];
        left.sort();
        left
    });
    assert!(logged(&h).0.contains("removeImages: 2"));
    // The one left alone at last: the folder goes with its last file.
    read(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "removeImages": [{ "file": file(2) }, { "index": 1 }] }),
    )
    .await;
    assert_eq!(stored(&h, &t.id).images, vec![]);
    assert!(!dir.exists());
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn images_are_taken_away_and_added_in_one_call_and_the_title_keeps_them() {
    let h = harness("mcp-images-update-both");
    let p = project(&h).await;
    let t = made(
        &h,
        &p,
        "Deux images",
        &[("a.png", PNG), ("b.png", &png_of(30))],
    )
    .await;
    let dir = dir_of(&h, &t.id);
    let c = external(&h).await;
    let updated = read(
        &c,
        "update_ticket",
        json!({
            "ticket": t.key,
            "removeImages": [{ "index": 0 }],
            "addImages": [shot("c.png", &png_of(31))],
        }),
    )
    .await;
    let names: Vec<&str> = updated["images"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap())
        .collect();
    // What stays comes first, what is added last.
    assert_eq!(names, ["b.png", "c.png"]);
    assert!(!dir.join(&t.images[0].file).exists());
    assert!(dir.join(&t.images[1].file).exists());
    assert_eq!(files_of(&dir).len(), 2);
    let (summary, _) = logged(&h);
    assert!(
        summary.contains("addImages: 1 · removeImages: 1"),
        "{summary}"
    );
    // A ticket edited in its words keeps its images.
    let retitled = read(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "title": "Nouveau titre", "description": "Un texte" }),
    )
    .await;
    assert_eq!(retitled["title"], "Nouveau titre");
    assert!(retitled["description"]
        .as_str()
        .unwrap()
        .contains("Modifié par"));
    assert_eq!(retitled["images"], updated["images"]);
    assert_eq!(files_of(&dir).len(), 2);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn images_alone_leave_the_title_the_description_and_the_criteria_as_they_were() {
    let h = harness("mcp-images-update-alone");
    let _p = project(&h).await;
    let c = external(&h).await;
    let created = read(
        &c,
        "create_ticket",
        json!({
            "project": "demo",
            "title": "Un titre",
            "description": "Un texte.",
            "criteria": ["Un critère", "Un autre"],
            "images": [shot("a.png", PNG)],
        }),
    )
    .await;
    let updated = read(
        &c,
        "update_ticket",
        json!({ "ticket": "DEM-1", "addImages": [shot("b.png", PNG)] }),
    )
    .await;
    for field in [
        "title",
        "description",
        "criteria",
        "after",
        "loops",
        "column",
    ] {
        assert_eq!(updated[field], created[field], "{field}");
    }
    assert!(updated["description"]
        .as_str()
        .unwrap()
        .ends_with("Créé par Claude (hors Escouade) via Escouade"));
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_reference_that_fits_no_image_of_the_ticket_fails_and_nothing_is_removed() {
    let h = harness("mcp-images-update-refs");
    let p = project(&h).await;
    let t = made(
        &h,
        &p,
        "Deux images",
        &[("a.png", PNG), ("b.png", &png_of(30))],
    )
    .await;
    let dir = dir_of(&h, &t.id);
    // A file outside the ticket's folder, one level up: a name that leads there removes nothing.
    let outside = h.core.data.tickets().join("dehors.png");
    std::fs::write(&outside, PNG).unwrap();
    let state = h.core.data.tickets().join("state.json");
    std::fs::write(&state, "{}").unwrap();
    let before_files = files_of(&dir);
    let before_images = stored(&h, &t.id).images;
    let absolute = dir.join(&t.images[0].file).to_string_lossy().to_string();
    let c = external(&h).await;
    let cases: Vec<(&str, Value)> = vec![
        ("an unknown file", json!({ "file": "inconnue.png" })),
        ("a way up", json!({ "file": "../dehors.png" })),
        ("a way up to a state", json!({ "file": "../state.json" })),
        (
            "a way up with backslashes",
            json!({ "file": r"..\dehors.png" }),
        ),
        ("an absolute path", json!({ "file": absolute })),
        ("a rank out of the list", json!({ "index": 2 })),
        (
            "a file and a rank",
            json!({ "file": t.images[0].file, "index": 0 }),
        ),
        ("neither", json!({})),
    ];
    for (what, reference) in cases {
        // The good one comes first: it is not removed either.
        let got = refused(
            &c,
            "update_ticket",
            json!({ "ticket": t.key, "removeImages": [{ "index": 0 }, reference] }),
        )
        .await;
        assert!(got.contains("get_ticket"), "{what}: {got}");
        assert_eq!(last_entry(&h).3, "error", "{what}");
        assert_eq!(stored(&h, &t.id).images, before_images, "{what}");
        assert_eq!(files_of(&dir), before_files, "{what}");
        assert_eq!(std::fs::read(&outside).unwrap(), PNG, "{what}");
        assert_eq!(std::fs::read_to_string(&state).unwrap(), "{}", "{what}");
    }
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_images_a_ticket_has_count_in_the_limits_of_an_update() {
    let h = harness("mcp-images-update-limits");
    let p = project(&h).await;
    let fifteen: Vec<(String, Vec<u8>)> = (0..15)
        .map(|i| (format!("{i}.png"), png_of(20 + i)))
        .collect();
    let refs: Vec<(&str, &[u8])> = fifteen
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    let t = made(&h, &p, "Quinze images", &refs).await;
    let dir = dir_of(&h, &t.id);
    let c = external(&h).await;
    let before_files = files_of(&dir);
    let before_images = stored(&h, &t.id).images;
    let six: Vec<Value> = (0..6).map(|i| shot(&format!("n{i}.png"), PNG)).collect();
    let got = refused(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "addImages": six }),
    )
    .await;
    assert_eq!(got, "Un ticket prend 20 images au plus.");
    // The state is as it was, and no file was written for the ones refused.
    assert_eq!(stored(&h, &t.id).images, before_images);
    assert_eq!(files_of(&dir), before_files);
    // One taken away makes room: 14 kept and 6 added make 20.
    let six: Vec<Value> = (0..6).map(|i| shot(&format!("n{i}.png"), PNG)).collect();
    let twenty = read(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "removeImages": [{ "index": 0 }], "addImages": six }),
    )
    .await;
    assert_eq!(twenty["images"].as_array().unwrap().len(), 20);
    assert_eq!(files_of(&dir).len(), 20);
    // And an image over the limit is refused as it is for a new ticket, the others kept.
    let got = refused(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "removeImages": [{ "index": 0 }], "addImages": [shot("grosse.png", &png_of(5 * MB + 1))] }),
    )
    .await;
    assert_eq!(got, "grosse.png dépasse 5 Mo");
    assert_eq!(stored(&h, &t.id).images.len(), 20);
    assert_eq!(files_of(&dir).len(), 20);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_that_has_started_is_refused_its_images_as_the_window_refuses_it() {
    let h = harness("mcp-images-update-started");
    let p = project(&h).await;
    let t = made(&h, &p, "Parti", &[("a.png", PNG), ("b.png", &png_of(30))]).await;
    moved(&h, &t.id, Column::Doing);
    let dir = dir_of(&h, &t.id);
    let before_files = files_of(&dir);
    let before_images = stored(&h, &t.id).images;
    let c = external(&h).await;
    for args in [
        json!({ "ticket": t.key, "addImages": [shot("c.png", PNG)] }),
        json!({ "ticket": t.key, "removeImages": [{ "index": 0 }] }),
        json!({ "ticket": t.key, "removeImages": [{ "index": 0 }], "addImages": [shot("c.png", PNG)] }),
    ] {
        let got = refused(&c, "update_ticket", args).await;
        assert_eq!(got, "Seul un ticket « À faire » se modifie.");
        assert_eq!(last_entry(&h).3, "refused");
        // No file left over, none gone; the images are still read.
        assert_eq!(files_of(&dir), before_files);
        assert_eq!(stored(&h, &t.id).images, before_images);
    }
    let (shown, error) = blocks(&c, json!({ "ticket": t.key, "index": 1 })).await;
    assert!(!error, "{shown:?}");
    assert_eq!(shown[0]["data"], b64(&png_of(30)));
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_that_has_started_is_refused_before_any_file_is_read_or_reference_looked_at() {
    let h = harness("mcp-images-update-started-first");
    let p = project(&h).await;
    let t = made(&h, &p, "Parti", &[("a.png", PNG)]).await;
    moved(&h, &t.id, Column::Doing);
    let dir = dir_of(&h, &t.id);
    let before_files = files_of(&dir);
    let before_images = stored(&h, &t.id).images;
    let gone = h.dir.join("disparue.png");
    let c = external(&h).await;
    // What would fail first on a ticket that is open: a file that is not there, a reference that
    // fits no image, an image with no name. The refusal is the column’s each time.
    for args in [
        json!({ "ticket": t.key, "addImages": [{ "path": gone }] }),
        json!({ "ticket": t.key, "removeImages": [{ "file": "../state.json" }] }),
        json!({ "ticket": t.key, "removeImages": [{ "index": 9 }, { "file": "a.png", "index": 0 }] }),
        json!({ "ticket": t.key, "addImages": [{ "data": b64(PNG) }] }),
        json!({ "ticket": t.key, "removeImages": [{ "file": "../state.json" }], "addImages": [{ "path": gone }] }),
    ] {
        let got = refused(&c, "update_ticket", args.clone()).await;
        assert_eq!(got, "Seul un ticket « À faire » se modifie.", "{args}");
        assert_eq!(last_entry(&h).3, "refused", "{args}");
        assert_eq!(files_of(&dir), before_files);
        assert_eq!(stored(&h, &t.id).images, before_images);
    }
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn empty_lists_alone_are_nothing_to_change() {
    let h = harness("mcp-images-update-empty");
    let p = project(&h).await;
    let t = made(&h, &p, "Une image", &[("a.png", PNG)]).await;
    let c = external(&h).await;
    for args in [
        json!({ "ticket": t.key, "addImages": [], "removeImages": [] }),
        json!({ "ticket": t.key, "addImages": [] }),
        json!({ "ticket": t.key, "removeImages": [] }),
    ] {
        let got = refused(&c, "update_ticket", args).await;
        assert!(
            got.contains("Rien à modifier")
                && got.contains("addImages")
                && got.contains("removeImages"),
            "{got}"
        );
        assert_eq!(last_entry(&h).3, "error");
    }
    // An empty list beside a field that changes is no obstacle, and takes nothing away.
    let updated = read(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "title": "Autre", "addImages": [], "removeImages": [] }),
    )
    .await;
    assert_eq!(updated["images"].as_array().unwrap().len(), 1);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agent_changes_the_images_of_a_ticket_of_another_project_unless_it_is_private() {
    let h = harness("mcp-images-update-other");
    let p = project(&h).await;
    let other = other_project(&h, "autre").await;
    let t = made(&h, &p, "Chez nous", &[("a.png", PNG)]).await;
    let agent = put_agent(&h, &other, "voisin", 1);
    let c = as_agent(&h, &agent.id).await;
    // Another project that is not private: as in its own.
    for args in [
        json!({ "ticket": t.key, "addImages": [shot("b.png", PNG)] }),
        json!({ "ticket": t.key, "removeImages": [{ "index": 0 }] }),
    ] {
        read(&c, "update_ticket", args).await;
        assert_eq!(last_entry(&h).3, "ok");
    }
    let names: Vec<String> = stored(&h, &t.id)
        .images
        .iter()
        .map(|i| i.name.clone())
        .collect();
    assert_eq!(names, ["b.png"]);
    // A private one: not found, and nothing changes.
    make_private(&h, &p);
    let before_files = files_of(&dir_of(&h, &t.id));
    let before = stored(&h, &t.id).images;
    for args in [
        json!({ "ticket": t.key, "addImages": [shot("c.png", PNG)] }),
        json!({ "ticket": t.key, "removeImages": [{ "index": 0 }] }),
    ] {
        let got = refused(&c, "update_ticket", args).await;
        assert!(
            got.starts_with(&format!("Aucun ticket « {} »", t.key)),
            "{got}"
        );
        assert!(!got.contains("demo") && !got.contains("Chez nous"), "{got}");
        assert_eq!(last_entry(&h).3, "error");
    }
    assert_eq!(stored(&h, &t.id).images, before);
    assert_eq!(files_of(&dir_of(&h, &t.id)), before_files);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

// ---------- get_ticket ----------

#[tokio::test]
async fn get_ticket_lists_every_image_and_says_no_size_for_a_file_that_is_gone() {
    let h = harness("mcp-images-get-ticket");
    let p = project(&h).await;
    let t = made(
        &h,
        &p,
        "Trois images",
        &[
            ("a.png", PNG),
            ("b.png", &png_of(30)),
            ("c.png", &png_of(31)),
        ],
    )
    .await;
    let dir = dir_of(&h, &t.id);
    let plain = made(&h, &p, "Sans image", &[]).await;
    // One file is gone; one image of a state edited by hand has a name that leads out of the folder,
    // with a real file where it leads.
    std::fs::remove_file(dir.join(&t.images[1].file)).unwrap();
    std::fs::write(h.core.data.tickets().join("x.png"), PNG).unwrap();
    h.core
        .edit_ticket(&t.id, |t| {
            t.images.push(TicketImage {
                name: "x.png".into(),
                media_type: "image/png".into(),
                file: "../x.png".into(),
            });
            Ok(())
        })
        .unwrap();
    let c = external(&h).await;
    let got = listed(&c, &t.key).await;
    let file = |i: usize| t.images[i].file.clone();
    assert_eq!(
        got,
        json!([
            { "index": 0, "file": file(0), "name": "a.png", "mediaType": "image/png", "size": PNG.len() },
            { "index": 1, "file": file(1), "name": "b.png", "mediaType": "image/png", "size": null },
            { "index": 2, "file": file(2), "name": "c.png", "mediaType": "image/png", "size": 31 },
            { "index": 3, "file": "../x.png", "name": "x.png", "mediaType": "image/png", "size": null },
        ])
    );
    assert_eq!(listed(&c, &plain.key).await, json!([]));
    // A ticket that has started or is done keeps its images, and says so.
    for column in [Column::Doing, Column::Review, Column::Done] {
        moved(&h, &t.id, column);
        assert_eq!(listed(&c, &t.key).await, got, "{column:?}");
    }
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

// ---------- get_ticket_image ----------

#[tokio::test]
async fn an_image_is_read_by_its_file_or_its_rank_as_an_image_block_then_its_description() {
    let h = harness("mcp-images-get-image");
    let p = project(&h).await;
    let jpeg = [JPEG, &[7u8; 40]].concat();
    let t = made(&h, &p, "Deux images", &[("a.png", PNG)]).await;
    let c = external(&h).await;
    // The second is added by the tools, a JPEG.
    let updated = read(
        &c,
        "update_ticket",
        json!({ "ticket": t.key, "addImages": [shot("photo.jpg", &jpeg)] }),
    )
    .await;
    let second = updated["images"][1]["file"].as_str().unwrap().to_string();
    for (what, args, index, content, mime, name) in [
        (
            "by file",
            json!({ "ticket": t.key, "file": second }),
            1,
            jpeg.as_slice(),
            "image/jpeg",
            "photo.jpg",
        ),
        (
            "by rank",
            json!({ "ticket": "dem-1", "index": 0 }),
            0,
            PNG,
            "image/png",
            "a.png",
        ),
        (
            "by id",
            json!({ "ticket": t.id, "index": 1 }),
            1,
            jpeg.as_slice(),
            "image/jpeg",
            "photo.jpg",
        ),
    ] {
        let (got, error) = blocks(&c, args).await;
        assert!(!error, "{what}: {got:?}");
        assert_eq!(got.len(), 2, "{what}: {got:?}");
        assert_eq!(got[0]["type"], "image", "{what}");
        assert_eq!(got[0]["mimeType"], mime, "{what}");
        assert_eq!(got[0]["data"], b64(content), "{what}");
        assert_eq!(got[1]["type"], "text", "{what}");
        let info: Value = serde_json::from_str(got[1]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            info,
            json!({
                "ticket": "DEM-1",
                "index": index,
                "file": updated["images"][index]["file"],
                "name": name,
                "mediaType": mime,
                "size": content.len(),
            }),
            "{what}"
        );
    }
    // The journal has what was asked, never the image.
    let (_, error) = blocks(&c, json!({ "ticket": t.key, "index": 1 })).await;
    assert!(!error);
    let (summary, message) = logged(&h);
    assert!(
        summary.contains("DEM-1") && summary.contains("index: 1"),
        "{summary}"
    );
    assert!(!summary.contains(&b64(&jpeg)) && message.is_none());
    assert_eq!(last_entry(&h).1, "get_ticket_image");
    assert_eq!(last_entry(&h).3, "ok");
    let (_, error) = blocks(&c, json!({ "ticket": t.key, "file": second })).await;
    assert!(!error);
    assert!(logged(&h).0.contains(&format!("file: {second}")));
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agent_reads_an_image_of_its_own_ticket_without_naming_it_and_claude_outside_must() {
    let h = harness("mcp-images-get-image-own");
    let p = project(&h).await;
    let own = made(&h, &p, "Le sien", &[("a.png", PNG), ("b.png", &png_of(30))]).await;
    let agent = put_agent(&h, &p, "chef", 1);
    link(&h, &agent.id, &own.id);
    let c = as_agent(&h, &agent.id).await;
    let (got, error) = blocks(&c, json!({ "index": 1 })).await;
    assert!(!error, "{got:?}");
    assert_eq!(got[0]["data"], b64(&png_of(30)));
    // The others' tickets are read too, as get_ticket reads them.
    let theirs = made(&h, &p, "Un autre", &[("z.png", &png_of(50))]).await;
    let (got, error) = blocks(&c, json!({ "ticket": theirs.key, "index": 0 })).await;
    assert!(!error, "{got:?}");
    assert_eq!(got[0]["data"], b64(&png_of(50)));
    c.cancel().await.unwrap();
    // Outside Escouade, without a ticket there is none to read.
    let c = external(&h).await;
    let (got, error) = blocks(&c, json!({ "index": 0 })).await;
    assert!(error);
    assert!(
        got[0]["text"].as_str().unwrap().contains("Donne la clé"),
        "{got:?}"
    );
    assert_eq!(last_entry(&h).3, "refused");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_image_that_is_not_the_tickets_or_is_gone_is_refused_with_a_text() {
    let h = harness("mcp-images-get-image-refused");
    let p = project(&h).await;
    let t = made(
        &h,
        &p,
        "Deux images",
        &[("a.png", PNG), ("b.png", &png_of(30))],
    )
    .await;
    // A file the state names beyond the folder, there for real: never read.
    std::fs::write(h.core.data.tickets().join("x.png"), PNG).unwrap();
    h.core
        .edit_ticket(&t.id, |t| {
            t.images.push(TicketImage {
                name: "x.png".into(),
                media_type: "image/png".into(),
                file: "../x.png".into(),
            });
            Ok(())
        })
        .unwrap();
    std::fs::remove_file(dir_of(&h, &t.id).join(&t.images[1].file)).unwrap();
    let absolute = dir_of(&h, &t.id)
        .join(&t.images[0].file)
        .to_string_lossy()
        .to_string();
    let c = external(&h).await;
    for (what, args, said) in [
        (
            "a rank out of the list",
            json!({ "ticket": t.key, "index": 9 }),
            "get_ticket",
        ),
        (
            "an unknown file",
            json!({ "ticket": t.key, "file": "inconnue.png" }),
            "get_ticket",
        ),
        (
            "a way up",
            json!({ "ticket": t.key, "file": "../dehors.png" }),
            "get_ticket",
        ),
        (
            "an absolute path",
            json!({ "ticket": t.key, "file": absolute }),
            "get_ticket",
        ),
        (
            "a file and a rank",
            json!({ "ticket": t.key, "file": t.images[0].file, "index": 0 }),
            "get_ticket",
        ),
        ("neither", json!({ "ticket": t.key }), "get_ticket"),
        (
            "a file that is gone",
            json!({ "ticket": t.key, "index": 1 }),
            "disparu",
        ),
        (
            "a name that leads out, by rank",
            json!({ "ticket": t.key, "index": 2 }),
            "disparu",
        ),
        (
            "a ticket that is not there",
            json!({ "ticket": "ZZZ-9", "index": 0 }),
            "ZZZ-9",
        ),
    ] {
        let (got, error) = blocks(&c, args).await;
        assert!(error, "{what}: {got:?}");
        assert_eq!(got.len(), 1, "{what}: {got:?}");
        let text = got[0]["text"].as_str().unwrap();
        assert!(text.contains(said), "{what}: {text}");
        assert_eq!(last_entry(&h).3, "error", "{what}");
        assert!(!text.contains(&b64(PNG)));
    }
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agent_reads_the_images_of_the_tickets_of_the_projects_it_sees() {
    let h = harness("mcp-images-get-image-project");
    let p = project(&h).await;
    let other = other_project(&h, "autre").await;
    let t = made(&h, &p, "Chez nous", &[("a.png", PNG)]).await;
    let neighbour = put_agent(&h, &other, "voisin", 1);
    let c = as_agent(&h, &neighbour.id).await;
    // Another project that is not private: its sheet and its images.
    assert_eq!(listed(&c, &t.key).await[0]["name"], "a.png");
    for args in [
        json!({ "ticket": t.key, "index": 0 }),
        json!({ "ticket": t.key, "file": t.images[0].file }),
    ] {
        let (got, error) = blocks(&c, args).await;
        assert!(!error, "{got:?}");
        assert_eq!(got[0]["data"], b64(PNG));
        assert_eq!(last_entry(&h).3, "ok");
    }
    // A private one: not found, and no image.
    make_private(&h, &p);
    for args in [
        json!({ "ticket": t.key, "index": 0 }),
        json!({ "ticket": t.key, "file": t.images[0].file }),
    ] {
        let (got, error) = blocks(&c, args).await;
        assert!(error, "{got:?}");
        assert_eq!(got.len(), 1, "{got:?}");
        let text = got[0]["text"].as_str().unwrap();
        assert!(
            text.starts_with(&format!("Aucun ticket « {} »", t.key)),
            "{text}"
        );
        assert!(
            !text.contains(&b64(PNG)) && !text.contains("demo"),
            "{text}"
        );
        assert_eq!(last_entry(&h).3, "error");
    }
    c.cancel().await.unwrap();
    // An agent of the same project reads it, and so does Claude outside Escouade, whatever the project.
    let mate = put_agent(&h, &p, "collegue", 2);
    let c = as_agent(&h, &mate.id).await;
    let (got, error) = blocks(&c, json!({ "ticket": t.key, "index": 0 })).await;
    assert!(!error, "{got:?}");
    assert_eq!(got[0]["data"], b64(PNG));
    c.cancel().await.unwrap();
    let c = external(&h).await;
    let (got, error) = blocks(&c, json!({ "ticket": t.key, "index": 0 })).await;
    assert!(!error, "{got:?}");
    assert_eq!(got[0]["data"], b64(PNG));
    assert_eq!(last_entry(&h).3, "ok");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_biggest_image_goes_through_the_transport_both_ways() {
    let h = harness("mcp-images-get-image-big");
    let _p = project(&h).await;
    let c = external(&h).await;
    let big = png_of(5 * MB);
    let created = read(
        &c,
        "create_ticket",
        json!({ "project": "demo", "title": "Grosse", "images": [shot("grosse.png", &big)] }),
    )
    .await;
    assert_eq!(created["images"][0]["size"], 5 * MB);
    let (got, error) = blocks(&c, json!({ "ticket": "DEM-1", "index": 0 })).await;
    assert!(!error, "{}", got[0]["text"]);
    assert_eq!(got[0]["data"].as_str().unwrap(), b64(&big));
    let info: Value = serde_json::from_str(got[1]["text"].as_str().unwrap()).unwrap();
    assert_eq!(info["size"], 5 * MB);
    // The log is short whatever the image weighs.
    let (summary, message) = logged(&h);
    assert!(summary.len() < 100 && message.is_none(), "{summary}");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

// ---------- split_ticket ----------

/// A ticket in progress with these images, and its agent's client.
async fn splitting(h: &Harness, p: &Project, images: &[(&str, &[u8])]) -> (Ticket, Client) {
    let own = made(h, p, "Gros ticket", images).await;
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

#[tokio::test]
async fn the_tickets_of_a_split_get_copies_of_all_the_images_of_their_origin_by_default() {
    let h = harness("mcp-images-split-default");
    let p = project(&h).await;
    let jpeg = [JPEG, &[7u8; 40]].concat();
    let (own, c) = splitting(&h, &p, &[("a.png", PNG), ("b.jpg", &jpeg)]).await;
    let made_keys = read(
        &c,
        "split_ticket",
        json!({ "tickets": [{ "title": "Étape 1" }, { "title": "Étape 2", "description": "Suite" }] }),
    )
    .await;
    assert_eq!(made_keys, json!(["DEM-2", "DEM-3"]));
    let mut all_files = own
        .images
        .iter()
        .map(|i| i.file.clone())
        .collect::<Vec<_>>();
    for key in ["DEM-2", "DEM-3"] {
        let t = ticket_of(&h, key);
        // Same names and types as the origin's (the type a content has), and the same content.
        assert_eq!(
            t.images
                .iter()
                .map(|i| (i.name.as_str(), i.media_type.as_str()))
                .collect::<Vec<_>>(),
            [("a.png", "image/png"), ("b.jpg", "image/jpeg")]
        );
        let dir = dir_of(&h, &t.id);
        assert_eq!(std::fs::read(dir.join(&t.images[0].file)).unwrap(), PNG);
        assert_eq!(std::fs::read(dir.join(&t.images[1].file)).unwrap(), jpeg);
        assert_eq!(files_of(&dir).len(), 2);
        all_files.extend(t.images.iter().map(|i| i.file.clone()));
    }
    // Copies, not the same files: six files, all different, in three folders.
    let distinct: std::collections::HashSet<_> = all_files.iter().collect();
    assert_eq!(all_files.len(), 6);
    assert_eq!(distinct.len(), 6, "{all_files:?}");
    assert_eq!(files_of(&h.core.data.tickets()).len(), 3);
    // The origin keeps its own, untouched.
    assert_eq!(stored(&h, &own.id).images, own.images);
    assert_eq!(files_of(&dir_of(&h, &own.id)).len(), 2);
    // Nothing was said of images: the journal has no `images`.
    let (summary, _) = logged(&h);
    assert!(!summary.contains("images"), "{summary}");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn each_ticket_of_a_split_takes_the_images_it_names_or_none() {
    let h = harness("mcp-images-split-select");
    let p = project(&h).await;
    let (own, c) = splitting(
        &h,
        &p,
        &[
            ("a.png", PNG),
            ("b.png", &png_of(30)),
            ("c.png", &png_of(31)),
        ],
    )
    .await;
    let made_keys = read(
        &c,
        "split_ticket",
        json!({ "tickets": [
            { "title": "Par fichier", "images": [{ "file": own.images[2].file }] },
            { "title": "Par rang", "images": [{ "index": 0 }, { "index": 1 }] },
            { "title": "Aucune", "images": [] },
            { "title": "Sans doublon", "images": [{ "index": 1 }, { "file": own.images[1].file }, { "index": 1 }] },
            { "title": "Toutes" },
        ] }),
    )
    .await;
    assert_eq!(
        made_keys,
        json!(["DEM-2", "DEM-3", "DEM-4", "DEM-5", "DEM-6"])
    );
    let names = |key: &str| -> Vec<String> {
        ticket_of(&h, key)
            .images
            .iter()
            .map(|i| i.name.clone())
            .collect()
    };
    assert_eq!(names("DEM-2"), ["c.png"]);
    assert_eq!(names("DEM-3"), ["a.png", "b.png"]);
    assert_eq!(names("DEM-4"), Vec::<String>::new());
    assert_eq!(names("DEM-5"), ["b.png"]);
    assert_eq!(names("DEM-6"), ["a.png", "b.png", "c.png"]);
    // Their folder is there only when they have files.
    assert!(!dir_of(&h, &ticket_of(&h, "DEM-4").id).exists());
    assert_eq!(files_of(&dir_of(&h, &ticket_of(&h, "DEM-5").id)).len(), 1);
    let t = ticket_of(&h, "DEM-2");
    assert_eq!(
        std::fs::read(dir_of(&h, &t.id).join(&t.images[0].file)).unwrap(),
        png_of(31)
    );
    // The journal says how many references were given in all: 1 + 2 + 0 + 3.
    let (summary, message) = logged(&h);
    assert!(summary.contains("images: 6"), "{summary}");
    assert_eq!(message, None);
    assert_eq!(stored(&h, &own.id).images, own.images);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_split_with_an_image_it_cannot_hand_down_makes_no_ticket_at_all() {
    let h = harness("mcp-images-split-refused");
    let p = project(&h).await;
    let (own, c) = splitting(&h, &p, &[("a.png", PNG), ("b.png", &png_of(30))]).await;
    let before = board(&h, &p);
    let got = refused(
        &c,
        "split_ticket",
        json!({ "tickets": [
            { "title": "Bon", "images": [{ "index": 0 }] },
            { "title": "Mauvais", "images": [{ "index": 0 }, { "index": 5 }] },
        ] }),
    )
    .await;
    assert!(
        got.starts_with("Ticket 2 de la liste : ") && got.contains("get_ticket"),
        "{got}"
    );
    assert_eq!(last_entry(&h).3, "error");
    assert_eq!(board(&h, &p), before);
    for hostile in [
        json!({ "file": "../state.json" }),
        json!({ "file": "inconnue.png" }),
        json!({ "file": own.images[0].file, "index": 0 }),
        json!({}),
    ] {
        let got = refused(
            &c,
            "split_ticket",
            json!({ "tickets": [{ "title": "Bon" }, { "title": "Mauvais", "images": [hostile] }] }),
        )
        .await;
        assert!(got.starts_with("Ticket 2 de la liste : "), "{got}");
        assert_eq!(board(&h, &p), before);
    }
    // A file of the origin that is gone cannot be copied when it is named: the call cannot be
    // honoured, and nothing is made. (By default it is left out, said by `a_split_by_default_...`.)
    std::fs::remove_file(dir_of(&h, &own.id).join(&own.images[1].file)).unwrap();
    for named in [json!({ "index": 1 }), json!({ "file": own.images[1].file })] {
        let got = refused(
            &c,
            "split_ticket",
            json!({ "tickets": [{ "title": "Bon", "images": [{ "index": 0 }] }, { "title": "Le disparu", "images": [named] }] }),
        )
        .await;
        assert!(
            got.starts_with("Ticket 2 de la liste : ") && got.contains("b.png"),
            "{got}"
        );
        assert_eq!(board(&h, &p), before);
    }
    // Without it they go through: the one that is there, or none.
    let ok = read(
        &c,
        "split_ticket",
        json!({ "tickets": [{ "title": "Le bon", "images": [{ "index": 0 }] }, { "title": "Aucune", "images": [] }] }),
    )
    .await;
    assert_eq!(ok, json!(["DEM-2", "DEM-3"]));
    assert_eq!(ticket_of(&h, "DEM-2").images.len(), 1);
    // The origin is as it was.
    assert_eq!(stored(&h, &own.id).images, own.images);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_split_by_default_leaves_out_an_image_whose_file_is_gone_and_says_so() {
    let h = harness("mcp-images-split-skipped");
    let p = project(&h).await;
    let (own, c) = splitting(
        &h,
        &p,
        &[
            ("a.png", PNG),
            ("plan.png", &png_of(30)),
            ("vue.png", &png_of(31)),
        ],
    )
    .await;
    let dir = dir_of(&h, &own.id);
    // Nothing missing: the keys alone, as before.
    let ok = read(
        &c,
        "split_ticket",
        json!({ "tickets": [{ "title": "Entier" }] }),
    )
    .await;
    assert_eq!(ok, json!(["DEM-2"]));
    assert_eq!(ticket_of(&h, "DEM-2").images.len(), 3);
    std::fs::remove_file(dir.join(&own.images[1].file)).unwrap();
    std::fs::remove_file(dir.join(&own.images[2].file)).unwrap();
    // One by default, one that names what is left, one that names nothing: the call is honoured,
    // the images that cannot be copied are said, one line for each ticket and each image.
    let got = read(
        &c,
        "split_ticket",
        json!({ "tickets": [
            { "title": "Par défaut" },
            { "title": "Choisie", "images": [{ "index": 0 }] },
            { "title": "Aucune", "images": [] },
            { "title": "Autre défaut", "description": "Suite" },
        ] }),
    )
    .await;
    assert_eq!(
        got["tickets"],
        json!(["DEM-3", "DEM-4", "DEM-5", "DEM-6"]),
        "{got}"
    );
    assert_eq!(
        got["skipped"],
        json!([
            "DEM-3 : l’image « plan.png » n’a pas été copiée, son fichier a disparu ou n’est pas lisible.",
            "DEM-3 : l’image « vue.png » n’a pas été copiée, son fichier a disparu ou n’est pas lisible.",
            "DEM-6 : l’image « plan.png » n’a pas été copiée, son fichier a disparu ou n’est pas lisible.",
            "DEM-6 : l’image « vue.png » n’a pas été copiée, son fichier a disparu ou n’est pas lisible.",
        ]),
        "{got}"
    );
    let names = |key: &str| -> Vec<String> {
        ticket_of(&h, key)
            .images
            .iter()
            .map(|i| i.name.clone())
            .collect()
    };
    assert_eq!(names("DEM-3"), ["a.png"]);
    assert_eq!(names("DEM-4"), ["a.png"]);
    assert_eq!(names("DEM-5"), Vec::<String>::new());
    assert_eq!(names("DEM-6"), ["a.png"]);
    let t = ticket_of(&h, "DEM-3");
    assert_eq!(
        std::fs::read(dir_of(&h, &t.id).join(&t.images[0].file)).unwrap(),
        PNG
    );
    // The log has the call as it was asked: one image named in all.
    let (summary, message) = logged(&h);
    assert!(summary.contains("images: 1"), "{summary}");
    assert_eq!(message, None);
    // The origin is as it was: it keeps the images whose files are gone.
    assert_eq!(stored(&h, &own.id).images, own.images);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_split_that_names_the_image_whose_file_is_gone_makes_nothing() {
    let h = harness("mcp-images-split-named-gone");
    let p = project(&h).await;
    let (own, c) = splitting(&h, &p, &[("a.png", PNG), ("plan.png", &png_of(30))]).await;
    std::fs::remove_file(dir_of(&h, &own.id).join(&own.images[1].file)).unwrap();
    let before = board(&h, &p);
    // The first ticket leaves it out by default, the second names it: the call cannot be honoured.
    let got = refused(
        &c,
        "split_ticket",
        json!({ "tickets": [{ "title": "Par défaut" }, { "title": "Le plan", "images": [{ "file": own.images[1].file }] }] }),
    )
    .await;
    assert!(
        got.starts_with("Ticket 2 de la liste : ") && got.contains("plan.png"),
        "{got}"
    );
    assert_eq!(board(&h, &p), before);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_ticket_without_images_has_none_to_hand_down_and_the_tool_stays_the_agents() {
    let h = harness("mcp-images-split-none");
    let p = project(&h).await;
    let (_own, c) = splitting(&h, &p, &[]).await;
    let before = board(&h, &p);
    let got = refused(
        &c,
        "split_ticket",
        json!({ "tickets": [{ "title": "Un", "images": [{ "index": 0 }] }] }),
    )
    .await;
    assert!(
        got.starts_with("Ticket 1 de la liste : ") && got.contains("aucune"),
        "{got}"
    );
    assert_eq!(board(&h, &p), before);
    let ok = read(
        &c,
        "split_ticket",
        json!({ "tickets": [{ "title": "Un" }] }),
    )
    .await;
    assert_eq!(ok, json!(["DEM-2"]));
    assert_eq!(ticket_of(&h, "DEM-2").images, vec![]);
    c.cancel().await.unwrap();
    // Claude outside Escouade is still refused, images or not.
    let outside = external(&h).await;
    let got = refused(
        &outside,
        "split_ticket",
        json!({ "tickets": [{ "title": "Un", "images": [] }] }),
    )
    .await;
    assert!(got.contains("réservé aux agents d’Escouade"), "{got}");
    assert_eq!(last_entry(&h).3, "refused");
    outside.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_new_tool_is_called_like_the_others_and_a_call_it_cannot_parse_is_logged() {
    let h = harness("mcp-images-tool");
    let _p = project(&h).await;
    let c = external(&h).await;
    // An index that is no number fits no schema: the router answers, the journal notes it.
    let (text, error) = call(
        &c,
        "get_ticket_image",
        json!({ "ticket": "DEM-1", "index": "zéro" }),
    )
    .await;
    assert!(error, "{text}");
    let entry = h.core.mcp.activity.entries().pop().unwrap();
    assert_eq!(
        (entry.tool.as_str(), entry.outcome),
        ("get_ticket_image", "error")
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

/// What the log says of a call whose arguments the schema refuses.
const DO_NOT_FIT: &str =
    "Appel refusé par le serveur : les arguments ne correspondent pas au schéma de l’outil";

#[tokio::test]
async fn arguments_the_schema_refuses_are_neither_logged_nor_answered_with_the_value_given() {
    let h = harness("mcp-images-router-values");
    let _p = project(&h).await;
    let c = external(&h).await;
    // As big as the biggest image a ticket takes, and then some: what serde would quote whole.
    let blob = format!("iVBORw0KGgoDEBUTDUBASE64{}", "QUJD".repeat(500_000));
    let cases: Vec<(&str, &'static str, Value, &str)> = vec![
        (
            "a path where an object goes",
            "create_ticket",
            json!({ "project": "demo", "title": "x", "images": ["C:/Users/me/secret.png"] }),
            "secret.png",
        ),
        (
            "a content where an object goes",
            "create_ticket",
            json!({ "project": "demo", "title": "x", "images": [blob] }),
            "DEBUTDUBASE64",
        ),
        (
            "a negative rank",
            "get_ticket_image",
            json!({ "ticket": "DEM-1", "index": -5 }),
            "-5",
        ),
        (
            "a column that is none",
            "list_tickets",
            json!({ "project": "demo", "column": "colonne-secrete" }),
            "colonne-secrete",
        ),
        (
            "a decision that is none",
            "answer_permission",
            json!({ "agent": "a", "request": "r", "decision": "decision-secrete" }),
            "decision-secrete",
        ),
        (
            "a number for a text",
            "create_ticket",
            json!({ "project": 123456789, "title": "x" }),
            "123456789",
        ),
        (
            "a text for a number",
            "get_ticket_image",
            json!({ "index": "zéro-secret" }),
            "zéro-secret",
        ),
        (
            "a boolean for a text",
            "create_ticket",
            json!({ "project": "demo", "title": true }),
            "true",
        ),
        (
            "a field that is missing",
            "create_ticket",
            json!({ "title": "x" }),
            "project",
        ),
    ];
    for (what, tool, args, value) in cases {
        let (text, error) = call(&c, tool, args).await;
        assert!(error, "{what}: {text}");
        // Short whatever the request weighed, and nothing quoted from it.
        assert!(
            text.chars().count() < 300,
            "{what}: {} characters",
            text.chars().count()
        );
        assert!(
            !text.contains(value) && !text.contains(['"', '`']),
            "{what}: {text}"
        );
        // Still something the model can act on: it is told to read the schema again.
        assert!(text.contains("schéma"), "{what}: {text}");
        let e = h.core.mcp.activity.entries().pop().unwrap();
        assert_eq!(
            (e.tool.as_str(), e.outcome, e.summary.as_str()),
            (tool, "error", ""),
            "{what}"
        );
        assert_eq!(e.message.as_deref(), Some(DO_NOT_FIT), "{what}");
    }
    // A tool the server does not have: the name is the client’s. The gate refuses it before rmcp,
    // repeats nothing of it, and the log does not keep it.
    let name = "outil-secret-".repeat(100);
    let err = c
        .call_tool(CallToolRequestParams::new(name))
        .await
        .unwrap_err();
    assert!(!err.to_string().contains("outil-secret"), "{err}");
    let e = h.core.mcp.activity.entries().pop().unwrap();
    assert_eq!((e.outcome, e.tool.as_str()), ("refused", ""));
    assert_eq!(
        e.message.as_deref(),
        Some("Requête refusée : outil inconnu")
    );
    let shown = serde_json::to_string(&e).unwrap();
    assert!(!shown.contains("outil-secret"), "{shown}");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_schemas_say_what_an_image_is_and_how_one_is_named() {
    let h = harness("mcp-images-schemas");
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
    // Every field says what it takes, in English, on one line.
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
    let read_one = tool("get_ticket_image")["inputSchema"].clone();
    for image in [
        &create["properties"]["images"]["items"],
        &update["properties"]["addImages"]["items"],
    ] {
        assert_eq!(
            keys(image),
            ["data", "mediaType", "name", "path"],
            "{image}"
        );
        described(image);
    }
    for reference in [
        &update["properties"]["removeImages"]["items"],
        &split["properties"]["tickets"]["items"]["properties"]["images"]["items"],
    ] {
        assert_eq!(keys(reference), ["file", "index"], "{reference}");
        described(reference);
        assert!(reference["properties"]["index"]
            .to_string()
            .contains("integer"));
    }
    assert_eq!(keys(&read_one), ["file", "index", "ticket"]);
    described(&read_one);
    // None of them is required, and the tools say it in their own words.
    assert_eq!(update["required"], json!(["ticket"]));
    for (name, said) in [
        ("create_ticket", ["images", "base64", "path"].as_slice()),
        ("update_ticket", &["addImages", "removeImages", "stay"]),
        ("get_ticket", &["images", "index", "file", "size"]),
        (
            "split_ticket",
            &["images", "copies", "all", "skipped", "gone"],
        ),
        (
            "get_ticket_image",
            &["image", "text", "file", "index", "own project"],
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
    // The working folder is a safeguard of the server, and the schema says no more than that.
    let path = create["properties"]["images"]["items"]["properties"]["path"]["description"]
        .as_str()
        .unwrap();
    assert!(
        path.contains("working folder") && path.contains("not a security boundary"),
        "{path}"
    );
    let read_one = tool("get_ticket_image");
    assert_eq!(read_one["annotations"]["readOnlyHint"], true);
    assert_eq!(read_one["annotations"]["openWorldHint"], false);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}
