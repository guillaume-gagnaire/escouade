//! `show_page`, `get_page` and `close_page` (ESC-673, AP-2), called by a real MCP client (rmcp's) on a
//! test core, without a webview: a template, the agent's own HTML and its file with its relative files,
//! a version, the row in `list_browsers` (and `read_browser` refused on it), who may (Claude outside
//! Escouade, another agent, a private project, a discussion), the copy that never leaves the worktree,
//! « Pages des agents » off (out of the list and of the instructions). Shown in a real webview: the
//! e2e's (agent-pages-mcp.spec.ts).

use super::private_tests::make_private;
use super::tests::names;
use super::tools_tests::{
    as_agent, call, external, last_entry, other_project, project, put_agent, read, refused,
};
use crate::core_tests::{harness, Harness};
use crate::model::*;
use serde_json::{json, Value};
use std::path::Path;

fn review() -> Value {
    json!({
        "subject": { "ticket": "POR-41", "branch": "ticket/por-41", "target": "main" },
        "criteria": [{ "text": "La liste montre 25 adhérents par page", "state": "met" }],
    })
}

/// The project, an agent of it working in its folder.
async fn world(h: &Harness) -> (Project, AgentMeta) {
    let p = project(h).await;
    let a = put_agent(h, &p, "pagination", 1);
    (p, a)
}

fn version_file(h: &Harness, p: &Project, page: &str, n: u32, rel: &str) -> std::path::PathBuf {
    crate::pages::version_dir(h.core.data.root(), &p.id, page, n).join(rel)
}

#[tokio::test]
async fn an_agent_shows_a_template_updates_it_reads_its_state_and_closes_its_row() {
    let h = harness("mcp-pages-show");
    let (p, a) = world(&h).await;
    let c = as_agent(&h, &a.id).await;
    let said = c
        .peer_info()
        .and_then(|i| i.instructions.clone())
        .unwrap_or_default();
    assert!(
        said.contains("show_page") && said.contains("no secret"),
        "{said}"
    );
    let tools = names(&c.list_all_tools().await.unwrap());
    for t in ["show_page", "get_page", "close_page"] {
        assert!(tools.contains(&t.to_string()), "{t}: {tools:?}");
    }
    let v = read(
        &c,
        "show_page",
        json!({ "title": "Revue de POR-41", "template": "review", "data": review() }),
    )
    .await;
    assert_eq!(v["page"], "p-1");
    assert_eq!(v["browser"], "page:p-1");
    assert_eq!(v["version"], 1);
    assert_eq!(v["opened"], "waiting");
    assert!(v["ask"].is_null());
    assert_eq!(v["files"], 1);
    let html = std::fs::read_to_string(version_file(&h, &p, "p-1", 1, "index.html")).unwrap();
    assert!(
        html.contains("La liste montre 25 adhérents par page"),
        "{html}"
    );
    // The log keeps the template and the number, never the title.
    let (_, tool, summary, outcome) = last_entry(&h);
    assert_eq!((tool.as_str(), outcome), ("show_page", "ok"));
    assert!(
        summary.contains("review") && !summary.contains("POR-41"),
        "{summary}"
    );
    // A new version.
    let mut data = review();
    data["criteria"][0]["state"] = json!("partial");
    let v = read(
        &c,
        "show_page",
        json!({ "title": "Revue de POR-41", "template": "review", "data": data, "page": "p-1" }),
    )
    .await;
    assert_eq!(
        (v["page"].as_str(), v["version"].as_u64()),
        (Some("p-1"), Some(2))
    );
    let g = read(&c, "get_page", json!({ "page": "p-1" })).await;
    assert_eq!(g["title"], "Revue de POR-41");
    assert_eq!(g["template"], "review");
    assert_eq!(g["version"], 2);
    assert_eq!(g["versions"].as_array().unwrap().len(), 2);
    assert_eq!(g["row"], "open");
    assert_eq!(g["placement"], "hidden");
    assert_eq!(g["on_screen"], false);
    // Its row, in list_browsers; read_browser refuses it.
    let rows = read(&c, "list_browsers", json!({})).await;
    let row = rows["browsers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["browser"] == "page:p-1")
        .cloned()
        .unwrap_or_else(|| panic!("{rows}"));
    assert_eq!(row["name"], "Revue de POR-41");
    assert_eq!(row["by"], "agent");
    assert_eq!(row["agent"], "pagination");
    assert_eq!(row["launch"], false);
    assert_eq!(row["placement"], "hidden");
    assert_eq!(row["project"], p.name.as_str());
    assert_eq!(row["url"], "escouade-page://p-1.localhost/");
    let e = refused(&c, "read_browser", json!({ "browser": "page:p-1" })).await;
    assert!(e.contains("get_page"), "{e}");
    let e = refused(&c, "capture_browser", json!({ "browser": "page:p-1" })).await;
    assert!(e.contains("ne se lit ni ne se capture"), "{e}");
    let e = refused(&c, "browser_console", json!({ "browser": "page:p-1" })).await;
    assert!(e.contains("console d’une page d’agent"), "{e}");
    // Closed: out of the list, kept.
    let v = read(&c, "close_page", json!({ "page": "p-1" })).await;
    assert_eq!(v["was_open"], true);
    let rows = read(&c, "list_browsers", json!({})).await;
    assert!(!rows.to_string().contains("page:p-1"), "{rows}");
    let g = read(&c, "get_page", json!({ "page": "p-1" })).await;
    assert_eq!(g["row"], "closed");
    // Shown again, its row opens again.
    read(
        &c,
        "show_page",
        json!({ "title": "Revue de POR-41", "template": "review", "data": review(), "page": "p-1" }),
    )
    .await;
    assert_eq!(
        read(&c, "get_page", json!({ "page": "p-1" })).await["row"],
        "open"
    );
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn what_does_not_fit_is_refused_and_said() {
    let h = harness("mcp-pages-refused");
    let (_, a) = world(&h).await;
    let c = as_agent(&h, &a.id).await;
    let e = refused(
        &c,
        "show_page",
        json!({ "title": "Revue", "template": "review", "data": { "criteria": [{ "text": "x", "state": "peut-être" }] } }),
    )
    .await;
    assert!(e.contains("criteria.0.state"), "{e}");
    for (args, said) in [
        (json!({ "title": "R", "template": "review" }), "data"),
        (
            json!({ "title": "R", "template": "review", "data": review(), "html": "<p>x</p>" }),
            "html",
        ),
        (json!({ "title": "R", "template": "html" }), "l’un des deux"),
        (
            json!({ "title": "R", "template": "html", "html": "<p>x</p>", "file": "a.html" }),
            "l’un des deux",
        ),
        (
            json!({ "title": "R", "template": "html", "html": "<p>x</p>", "data": {} }),
            "data",
        ),
        (
            json!({ "title": "R", "template": "custom", "data": {} }),
            "page à soi",
        ),
        (
            json!({ "title": "", "template": "html", "html": "<p>x</p>" }),
            "titre",
        ),
        (
            json!({ "title": "R", "template": "html", "html": "<p>x</p>", "page": "p-9" }),
            "Aucune page",
        ),
    ] {
        let e = refused(&c, "show_page", args.clone()).await;
        assert!(e.contains(said), "{args}: {e}");
    }
    let e = refused(&c, "get_page", json!({ "page": "p-1" })).await;
    assert!(e.contains("tu n’en as montré aucune"), "{e}");
    assert!(!h.core.data.root().join("pages").exists());
    c.cancel().await.unwrap();
}

/// A page and its files in the agent's folder, a secret beside it.
fn page_files(home: &Path) {
    let page = home.join("design");
    std::fs::create_dir_all(page.join("img")).unwrap();
    std::fs::write(home.join("secret.txt"), "TOP-SECRET").unwrap();
    std::fs::write(page.join(".env"), "TOKEN=TOP-SECRET-ENV").unwrap();
    std::fs::write(
        page.join("revue.html"),
        "<!doctype html><link rel=stylesheet href=style.css><img src=img/a.png><img src=../secret.txt><script src=.env></script><p>Ma revue</p>",
    )
    .unwrap();
    std::fs::write(page.join("style.css"), "p{color:red}").unwrap();
    std::fs::write(page.join("img").join("a.png"), b"\x89PNG\r\n\x1a\nA").unwrap();
}

#[tokio::test]
async fn a_page_file_comes_with_its_files_and_never_with_what_is_outside_its_folder() {
    let h = harness("mcp-pages-file");
    let (p, a) = world(&h).await;
    let home = Path::new(&p.path);
    page_files(home);
    let c = as_agent(&h, &a.id).await;
    let v = read(
        &c,
        "show_page",
        json!({ "title": "Ma revue", "template": "html", "file": "design/revue.html" }),
    )
    .await;
    assert_eq!(v["files"], 3);
    let mut out: Vec<String> = serde_json::from_value(v["left_out"].clone()).unwrap();
    out.sort();
    assert_eq!(out, ["../secret.txt", ".env"]);
    assert!(version_file(&h, &p, "p-1", 1, "style.css").is_file());
    assert!(version_file(&h, &p, "p-1", 1, "img/a.png").is_file());
    let dir = crate::pages::version_dir(h.core.data.root(), &p.id, "p-1", 1);
    let all: Vec<_> = walk(&dir);
    for f in &all {
        let text = String::from_utf8_lossy(&std::fs::read(f).unwrap()).into_owned();
        assert!(!text.contains("TOP-SECRET"), "{}", f.display());
    }
    // Absolute within the folder: taken; out of it: refused, whatever it is.
    let abs = home.join("design").join("revue.html");
    read(
        &c,
        "show_page",
        json!({ "title": "Ma revue", "template": "html", "file": abs, "page": "p-1" }),
    )
    .await;
    let outside = h.dir.join("dehors.html");
    std::fs::write(&outside, "<p>dehors</p>").unwrap();
    for file in [
        json!(outside),
        json!("../dehors.html"),
        json!("secret.txt"),
        json!("design/absente.html"),
    ] {
        let e = refused(
            &c,
            "show_page",
            json!({ "title": "Dehors", "template": "html", "file": file }),
        )
        .await;
        assert!(
            !e.contains("TOP-SECRET") && !e.contains("dehors</p>"),
            "{e}"
        );
    }
    // The page of the agent's own HTML, given whole.
    let v = read(
        &c,
        "show_page",
        json!({ "title": "Libre", "template": "html", "html": "<!doctype html><p>libre</p><img src=img/a.png>" }),
    )
    .await;
    assert_eq!(v["files"], 1);
    c.cancel().await.unwrap();
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let path = e.path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else {
            out.push(path);
        }
    }
    out
}

#[cfg(windows)]
#[tokio::test]
async fn a_link_out_of_the_folder_is_never_followed() {
    use super::tools_tests::junction;
    let h = harness("mcp-pages-junction");
    let (p, a) = world(&h).await;
    let home = Path::new(&p.path);
    page_files(home);
    let away = h.dir.join("away");
    std::fs::create_dir_all(&away).unwrap();
    std::fs::write(away.join("x.css"), "TOP-SECRET-AWAY").unwrap();
    std::fs::write(away.join("page.html"), "<p>TOP-SECRET-AWAY</p>").unwrap();
    let _j = junction(&home.join("design").join("lien"), &away);
    std::fs::write(
        home.join("design").join("par-lien.html"),
        "<link rel=stylesheet href=lien/x.css><p>ok</p>",
    )
    .unwrap();
    let c = as_agent(&h, &a.id).await;
    let v = read(
        &c,
        "show_page",
        json!({ "title": "Lien", "template": "html", "file": "design/par-lien.html" }),
    )
    .await;
    assert_eq!(v["left_out"], json!(["lien/x.css"]));
    assert_eq!(v["files"], 1);
    let e = refused(
        &c,
        "show_page",
        json!({ "title": "Lien", "template": "html", "file": "design/lien/page.html" }),
    )
    .await;
    assert!(!e.contains("TOP-SECRET"), "{e}");
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn a_page_is_known_to_its_agent_alone_and_a_private_project_keeps_its_own() {
    let h = harness("mcp-pages-private");
    let (p, a) = world(&h).await;
    let p = make_private(&h, &p);
    let c = as_agent(&h, &a.id).await;
    read(
        &c,
        "show_page",
        json!({ "title": "Coffre", "template": "html", "html": "<p>secret du coffre</p>" }),
    )
    .await;
    // Another agent of the same project: not its page.
    let b = put_agent(&h, &p, "voisin", 2);
    let cb = as_agent(&h, &b.id).await;
    let e = refused(&cb, "get_page", json!({ "page": "p-1" })).await;
    assert!(e.contains("Aucune page « p-1 »"), "{e}");
    let e = refused(&cb, "close_page", json!({ "page": "p-1" })).await;
    assert!(e.contains("Aucune page"), "{e}");
    // It lists the row (its project's), never reads it.
    let rows = read(&cb, "list_browsers", json!({})).await;
    assert!(rows.to_string().contains("page:p-1"), "{rows}");
    // An agent of another project knows nothing of it, nor does Claude outside Escouade.
    let other = other_project(&h, "Voisin").await;
    let o = put_agent(&h, &other, "dehors", 3);
    let co = as_agent(&h, &o.id).await;
    let e = refused(&co, "get_page", json!({ "page": "p-1" })).await;
    assert!(
        e.contains("Aucune page « p-1 »") && !e.contains("Coffre"),
        "{e}"
    );
    let rows = read(&co, "list_browsers", json!({})).await;
    assert!(!rows.to_string().contains("page:"), "{rows}");
    let x = external(&h).await;
    let e = refused(&x, "list_browsers", json!({ "project": p.id })).await;
    assert!(!e.contains("Coffre"), "{e}");
    for (tool, args) in [
        (
            "show_page",
            json!({ "title": "x", "template": "html", "html": "<p>x</p>" }),
        ),
        ("get_page", json!({ "page": "p-1" })),
        ("close_page", json!({ "page": "p-1" })),
    ] {
        let e = refused(&x, tool, args).await;
        assert!(
            e.contains("Claude hors d’Escouade ne montre pas de page"),
            "{tool}: {e}"
        );
    }
    for c in [c, cb, co, x] {
        c.cancel().await.unwrap();
    }
}

#[tokio::test]
async fn a_discussion_shows_a_page_without_a_file_and_lists_its_own_rows_alone() {
    let h = harness("mcp-pages-discussion");
    let (p, a) = world(&h).await;
    page_files(Path::new(&p.path));
    let d = put_agent(&h, &p, "discussion", 2);
    h.core.agent(&d.id).unwrap().lock().meta.discussion = Some(Default::default());
    let ca = as_agent(&h, &a.id).await;
    read(
        &ca,
        "show_page",
        json!({ "title": "De l’agent", "template": "html", "html": "<p>a</p>" }),
    )
    .await;
    let cd = as_agent(&h, &d.id).await;
    let e = refused(
        &cd,
        "show_page",
        json!({ "title": "x", "template": "html", "file": "design/revue.html" }),
    )
    .await;
    assert!(e.contains("Une discussion n’a pas de dossier"), "{e}");
    let v = read(
        &cd,
        "show_page",
        json!({ "title": "De la discussion", "template": "html", "html": "<p>d</p>" }),
    )
    .await;
    assert_eq!(v["page"], "p-2");
    let rows = read(&cd, "list_browsers", json!({})).await;
    let keys: Vec<&str> = rows["browsers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["browser"].as_str().unwrap())
        .collect();
    assert_eq!(keys, ["page:p-2"]);
    ca.cancel().await.unwrap();
    cd.cancel().await.unwrap();
}

#[tokio::test]
async fn off_the_tools_leave_the_list_and_the_instructions_and_refuse() {
    let h = harness("mcp-pages-switch");
    let (p, a) = world(&h).await;
    let c = as_agent(&h, &a.id).await;
    read(
        &c,
        "show_page",
        json!({ "title": "Revue", "template": "html", "html": "<p>x</p>" }),
    )
    .await;
    c.cancel().await.unwrap();
    let mut s = h.core.settings.read().clone();
    s.agent_pages = false;
    h.core.save_settings(s).unwrap();
    let c = as_agent(&h, &a.id).await;
    let said = c
        .peer_info()
        .and_then(|i| i.instructions.clone())
        .unwrap_or_default();
    assert!(!said.contains("show_page"), "{said}");
    assert!(said.contains("Escouade runs Claude Code agents"), "{said}");
    let tools = names(&c.list_all_tools().await.unwrap());
    for t in ["show_page", "get_page", "close_page"] {
        assert!(!tools.contains(&t.to_string()), "{t}");
    }
    assert!(tools.contains(&"list_browsers".to_string()));
    // A harness that kept the list is refused.
    for (tool, args) in [
        (
            "show_page",
            json!({ "title": "x", "template": "html", "html": "<p>x</p>" }),
        ),
        ("get_page", json!({ "page": "p-1" })),
        ("close_page", json!({ "page": "p-1" })),
    ] {
        let (e, error) = call(&c, tool, args).await;
        assert!(error, "{tool}: {e}");
    }
    // Its rows closed, nothing erased.
    let rows = read(&c, "list_browsers", json!({})).await;
    assert!(!rows.to_string().contains("page:"), "{rows}");
    assert!(crate::pages::version_dir(h.core.data.root(), &p.id, "p-1", 1).is_dir());
    assert_eq!(h.core.pages.lock().list.len(), 1);
    c.cancel().await.unwrap();
}
