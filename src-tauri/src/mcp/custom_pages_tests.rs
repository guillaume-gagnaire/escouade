//! The page of one's own (ESC-677, AP-7; docs/AGENT-PAGES.md § 7.4), by a real MCP client on a test
//! core: refused, saying what to take, without one; the machine's (« Page à soi » of Réglages ›
//! Projets › Lancement) with its files and `window.escouade`; `pages.custom` of `.escouade.yml`, which
//! waits for the user's trust and comes after the machine's; a path that leaves the repository refused.
//! Shown in a real webview: the e2e's (agent-pages-custom.spec.ts).

use super::tools_tests::{as_agent, project, put_agent, read, refused};
use crate::core_tests::{harness, Harness};
use crate::model::*;
use crate::projectfile as pf;
use serde_json::{json, Value};
use std::path::Path;

async fn world(h: &Harness) -> (Project, AgentMeta) {
    let p = project(h).await;
    let a = put_agent(h, &p, "pagination", 1);
    (p, a)
}

fn index(h: &Harness, p: &Project, page: &str, n: u32) -> String {
    let f = crate::pages::version_dir(h.core.data.root(), &p.id, page, n).join("index.html");
    std::fs::read_to_string(f).unwrap()
}

/// The machine's page of one's own set as the window sets it.
fn set_mine(h: &Harness, id: &str, page: &str) {
    let mut p = h.core.project(id).unwrap();
    p.custom_page = page.to_string();
    h.core.update_project(p).unwrap();
}

/// What `window.escouade` holds, read back from the script written into the document.
fn escouade(html: &str) -> Value {
    let start = html.find("value:f(").expect("no window.escouade") + "value:f(".len();
    let end = html[start..].find("),writable").unwrap() + start;
    serde_json::from_str(&html[start..end]).unwrap()
}

#[tokio::test]
async fn without_a_page_of_ones_own_custom_is_refused_saying_what_to_take() {
    let h = harness("mcp-custom-none");
    let (_, a) = world(&h).await;
    let c = as_agent(&h, &a.id).await;
    let e = refused(
        &c,
        "show_page",
        json!({ "title": "Ma revue", "template": "custom", "data": {} }),
    )
    .await;
    assert!(
        e.contains("pas réglé de page à soi") && e.contains("review, diagram, compare ou html"),
        "{e}"
    );
    assert!(!h.core.data.root().join("pages").exists());
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn the_machines_page_is_shown_with_its_files_and_window_escouade() {
    let h = harness("mcp-custom-mine");
    let (p, a) = world(&h).await;
    let home = Path::new(&p.path);
    std::fs::create_dir_all(home.join("mine")).unwrap();
    std::fs::write(
        home.join("mine").join("revue.html"),
        "<!doctype html><html><head><script src=app.js></script></head><body><p>MINE-PAGE</p></body></html>",
    )
    .unwrap();
    std::fs::write(
        home.join("mine").join("app.js"),
        "document.title = window.escouade.title",
    )
    .unwrap();
    std::fs::write(home.join("mine").join(".env"), "TOKEN=TOP-SECRET").unwrap();
    // Not an .html document: refused when saved.
    let mut bad = h.core.project(&p.id).unwrap();
    bad.custom_page = "mine/app.js".into();
    let e = h.core.update_project(bad).unwrap_err().to_string();
    assert!(e.contains(".html"), "{e}");
    set_mine(&h, &p.id, "mine/revue.html");

    let c = as_agent(&h, &a.id).await;
    // Its document is the user's: neither html nor file.
    for extra in [json!({ "html": "<p>x</p>" }), json!({ "file": "x.html" })] {
        let mut args = json!({ "title": "R", "template": "custom" });
        args.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        let e = refused(&c, "show_page", args).await;
        assert!(e.contains("ses données (data)"), "{e}");
    }
    // Data that tries to close the script.
    let data = json!({ "note": "</script><img src=x onerror=alert(1)>", "n": [1, 2] });
    let v = read(
        &c,
        "show_page",
        json!({ "title": "Ma revue", "template": "custom", "data": data }),
    )
    .await;
    assert_eq!(
        (v["page"].as_str(), v["version"].as_u64()),
        (Some("p-1"), Some(1))
    );
    assert_eq!(v["files"], 2, "{v}");
    let html = index(&h, &p, "p-1", 1);
    // Right after the doctype, before any script of the page's.
    assert!(html.starts_with("<!doctype html><script>(()=>"), "{html}");
    assert!(html.find("window,\"escouade\"").unwrap() < html.find("src=app.js").unwrap());
    assert!(html.contains("MINE-PAGE"));
    assert!(!html.contains("</script><img"), "{html}");
    assert!(html.contains("Object.freeze") && html.contains("configurable:false"));
    let e = escouade(&html);
    assert_eq!(e["page"], "p-1");
    assert_eq!(e["version"], 1);
    assert_eq!(e["template"], "custom");
    assert_eq!(e["title"], "Ma revue");
    assert_eq!(e["agent"], json!({ "name": "pagination" }));
    assert_eq!(e["project"], json!({ "name": p.name }));
    assert!(e["ticket"].is_null() && e["ask"].is_null());
    assert_eq!(e["data"], data);
    assert!(matches!(e["theme"].as_str(), Some("dark" | "light")));
    assert!(matches!(e["lang"].as_str(), Some("fr" | "en")));
    // What it holds, no more: no path, no id of Escouade.
    let keys: Vec<&str> = e.as_object().unwrap().keys().map(String::as_str).collect();
    let mut keys = keys;
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "agent", "ask", "data", "lang", "page", "project", "template", "theme", "ticket",
            "title", "version"
        ]
    );
    let home_text = p.path.replace('\\', "\\\\");
    assert!(
        !html.contains(&home_text) && !html.contains(&a.id),
        "{html}"
    );
    // Its files came with it; the hidden one never did.
    let dir = crate::pages::version_dir(h.core.data.root(), &p.id, "p-1", 1);
    assert!(dir.join("app.js").exists());
    assert!(!dir.join(".env").exists());
    // A new version: its number follows.
    let v = read(
        &c,
        "show_page",
        json!({ "title": "Ma revue", "template": "custom", "data": { "n": 3 }, "page": "p-1" }),
    )
    .await;
    assert_eq!(v["version"], 2);
    assert_eq!(escouade(&index(&h, &p, "p-1", 2))["version"], 2);
    let g = read(&c, "get_page", json!({ "page": "p-1" })).await;
    assert_eq!(g["template"], "custom");
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn the_files_page_waits_for_trust_and_the_machines_comes_first() {
    let h = harness("mcp-custom-file");
    let (p, a) = world(&h).await;
    let home = Path::new(&p.path);
    std::fs::create_dir_all(home.join("design")).unwrap();
    std::fs::write(
        home.join("design").join("revue.html"),
        "<!doctype html><p>REPO-PAGE</p>",
    )
    .unwrap();
    std::fs::write(home.join("mine.html"), "<!doctype html><p>MINE-PAGE</p>").unwrap();
    std::fs::write(
        home.join(pf::FILE),
        "version: 1\npages:\n  custom: design/revue.html\n",
    )
    .unwrap();
    let p = h.core.read_shared(&p.id, true).unwrap();
    // Read, not applied: it counts with the commands, waiting for the user's trust.
    assert!(p.file_custom_page.is_empty());
    let pending = p.shared.file.clone().unwrap().pending.unwrap();
    assert_eq!(pending.commands[pf::CUSTOM_PAGE], "design/revue.html");
    let c = as_agent(&h, &a.id).await;
    let args = json!({ "title": "Revue", "template": "custom", "data": { "x": 1 } });
    let e = refused(&c, "show_page", args.clone()).await;
    assert!(
        e.contains("attend l’accord") && e.contains("pages.custom"),
        "{e}"
    );

    // Trusted: the repository's page.
    let p = h.core.trust_shared(&p.id, &pending.hash).unwrap();
    assert_eq!(p.file_custom_page, "design/revue.html");
    let v = read(&c, "show_page", args.clone()).await;
    assert!(index(&h, &p, v["page"].as_str().unwrap(), 1).contains("REPO-PAGE"));

    // The machine's comes first.
    set_mine(&h, &p.id, "mine.html");
    let v = read(&c, "show_page", args.clone()).await;
    let html = index(&h, &p, v["page"].as_str().unwrap(), 1);
    assert!(
        html.contains("MINE-PAGE") && !html.contains("REPO-PAGE"),
        "{html}"
    );
    // Read again, the file never takes its place.
    std::fs::write(
        home.join(pf::FILE),
        "version: 1\npages:\n  custom: design/autre.html\n",
    )
    .unwrap();
    let p = h
        .core
        .read_shared(&p.id, false)
        .unwrap_or_else(|| h.core.project(&p.id).unwrap());
    assert_eq!(p.custom_page, "mine.html");
    // The new value waits; the trusted one stays.
    assert_eq!(p.file_custom_page, "design/revue.html");
    // The machine's taken away: the file's trusted one again.
    set_mine(&h, &p.id, "");
    let v = read(&c, "show_page", args).await;
    assert!(index(&h, &p, v["page"].as_str().unwrap(), 1).contains("REPO-PAGE"));
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn a_page_out_of_the_repository_is_refused() {
    let h = harness("mcp-custom-out");
    let (p, a) = world(&h).await;
    let home = Path::new(&p.path);
    std::fs::write(h.dir.join("dehors.html"), "<p>TOP-SECRET-OUT</p>").unwrap();
    for bad in [
        "../dehors.html",
        "design/../../dehors.html",
        "/etc/x.html",
        "C:/x.html",
    ] {
        std::fs::write(
            home.join(pf::FILE),
            format!("version: 1\npages:\n  custom: '{bad}'\n"),
        )
        .unwrap();
        let p = h
            .core
            .read_shared(&p.id, true)
            .unwrap_or_else(|| h.core.project(&p.id).unwrap());
        let file = p.shared.file.clone().unwrap();
        assert!(!file.values.contains_key(pf::CUSTOM_PAGE), "{bad}");
        assert!(file.pending.is_none(), "{bad}");
        assert!(
            file.warnings
                .iter()
                .any(|w| w.text.contains(pf::CUSTOM_PAGE)),
            "{bad}: {:?}",
            file.warnings
        );
    }
    let c = as_agent(&h, &a.id).await;
    let e = refused(
        &c,
        "show_page",
        json!({ "title": "R", "template": "custom", "data": {} }),
    )
    .await;
    assert!(e.contains("pas réglé de page à soi"), "{e}");

    // A link of the repository that leads out of it is not followed, even trusted.
    #[cfg(windows)]
    {
        use super::tools_tests::junction;
        let away = h.dir.join("away");
        std::fs::create_dir_all(&away).unwrap();
        std::fs::write(away.join("page.html"), "<p>TOP-SECRET-AWAY</p>").unwrap();
        let _j = junction(&home.join("lien"), &away);
        std::fs::write(
            home.join(pf::FILE),
            "version: 1\npages:\n  custom: lien/page.html\n",
        )
        .unwrap();
        let p = h.core.read_shared(&p.id, true).unwrap();
        let hash = p.shared.file.unwrap().pending.unwrap().hash;
        h.core.trust_shared(&p.id, &hash).unwrap();
        let e = refused(
            &c,
            "show_page",
            json!({ "title": "R", "template": "custom", "data": {} }),
        )
        .await;
        assert!(
            e.contains("sort du dépôt") && !e.contains("TOP-SECRET"),
            "{e}"
        );
        assert!(!h.core.data.root().join("pages").exists());
    }
    c.cancel().await.unwrap();
}
