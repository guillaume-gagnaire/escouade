//! The model of the agents' pages (ESC-673, AP-2; docs/AGENT-PAGES.md § 3): the registry and its
//! numbers, the versions and their copies, the bounds, the durable state, the purge, the switch; the
//! copy of a page's files, which never leaves its folder. The tools are `mcp::pages_tests`.

use crate::agent::AgentRt;
use crate::agentpage::Shelter;
use crate::core::Core;
use crate::core_tests::{harness, Harness};
use crate::devpage::Placement;
use crate::i18n::Lang;
use crate::model::*;
use crate::pages::{self, AgentPage, Author, Bundle, PageVersion, Resolved, Show};
use crate::paths::DataDir;
use std::path::Path;
use std::sync::Arc;
use tauri::test::{mock_app, MockRuntime};
use tauri::Manager;

const DAY_MS: i64 = 24 * 3600 * 1000;

/// An agent of `p` named `name`, as the app keeps one (no process).
fn put_agent(h: &Harness, p: &Project, name: &str) -> AgentMeta {
    let meta = AgentMeta {
        id: format!("a-{}-{name}", p.id),
        project_id: p.id.clone(),
        name: name.into(),
        cwd: p.path.clone(),
        account: crate::accounts::PRINCIPAL.into(),
        ..Default::default()
    };
    let rt = AgentRt::new(meta.clone(), &h.core.data.conversations());
    h.core
        .agents
        .write()
        .insert(meta.id.clone(), Arc::new(parking_lot::Mutex::new(rt)));
    meta
}

fn author(a: &AgentMeta) -> Author {
    Author {
        agent: a.id.clone(),
        name: a.name.clone(),
        project_id: a.project_id.clone(),
        ticket_id: None,
    }
}

fn html(text: &str) -> Bundle {
    pages::from_html(
        Lang::Fr,
        &format!("<!doctype html><title>t</title><p>{text}</p>"),
    )
    .unwrap()
}

fn show(title: &str, page: Option<&str>, b: Bundle) -> Show {
    Show {
        title: title.into(),
        template: "html",
        page: page.map(str::to_string),
        bundle: b,
        quiet: false,
        escouade: None,
    }
}

/// A harness, its project and an agent of it.
async fn world(name: &str) -> (Harness, Project, AgentMeta) {
    let h = harness(name);
    let (p, _) = h.project(false).await;
    let a = put_agent(&h, &p, "pagination");
    (h, p, a)
}

fn root(h: &Harness) -> std::path::PathBuf {
    h.core.data.root().to_path_buf()
}

#[tokio::test]
async fn a_page_is_numbered_copied_and_versioned_and_keeps_its_ten_last_versions() {
    let (h, p, a) = world("pages-versions").await;
    let first = h
        .core
        .page_show(&author(&a), show("Revue POR-41", None, html("une")))
        .unwrap();
    assert_eq!((first.page.as_str(), first.version), ("p-1", 1));
    let v1 = pages::version_dir(&root(&h), &p.id, "p-1", 1);
    let served = std::fs::read_to_string(v1.join("index.html")).unwrap();
    assert!(served.contains("<p>une</p>"), "{served}");
    // A new version of the same page, not a new page.
    let second = h
        .core
        .page_show(
            &author(&a),
            show("Revue POR-41 (2)", Some("p-1"), html("deux")),
        )
        .unwrap();
    assert_eq!((second.page.as_str(), second.version), ("p-1", 2));
    let seen = h.core.page_get(&a.id, "p-1").unwrap();
    assert_eq!(seen.page.title, "Revue POR-41 (2)");
    assert_eq!(seen.page.versions.len(), 2);
    assert_eq!(seen.row, Some(Placement::Hidden));
    // Ten kept: the eleventh takes the first away, its folder with it.
    for k in 3..=11 {
        h.core
            .page_show(
                &author(&a),
                show("Revue", Some("p-1"), html(&format!("v{k}"))),
            )
            .unwrap();
    }
    let seen = h.core.page_get(&a.id, "p-1").unwrap();
    let ns: Vec<u32> = seen.page.versions.iter().map(|v| v.n).collect();
    assert_eq!(ns, (2..=11).collect::<Vec<_>>());
    assert!(!v1.exists());
    assert!(pages::version_dir(&root(&h), &p.id, "p-1", 11)
        .join("index.html")
        .is_file());
    // A new page has the next number.
    let other = h
        .core
        .page_show(&author(&a), show("Schéma", None, html("s")))
        .unwrap();
    assert_eq!(other.page, "p-2");
}

#[tokio::test]
async fn an_agent_keeps_twenty_pages_and_is_told_to_update_one() {
    let (h, p, a) = world("pages-twenty").await;
    for k in 1..=pages::PER_AGENT {
        h.core
            .page_show(&author(&a), show(&format!("Page {k}"), None, html("x")))
            .unwrap();
    }
    let e = h
        .core
        .page_show(&author(&a), show("Une de trop", None, html("x")))
        .unwrap_err();
    assert!(e.contains("20 pages"), "{e}");
    assert!(e.contains("page: \"p-"), "{e}");
    // Another agent has its own twenty.
    let b = put_agent(&h, &p, "autre");
    assert_eq!(
        h.core
            .page_show(&author(&b), show("La sienne", None, html("x")))
            .unwrap()
            .page,
        "p-21"
    );
    // And a version is no new page.
    assert!(h
        .core
        .page_show(&author(&a), show("Page 1", Some("p-1"), html("y")))
        .is_ok());
}

#[tokio::test]
async fn a_page_is_its_agents_alone() {
    let (h, p, a) = world("pages-own").await;
    let b = put_agent(&h, &p, "voisin");
    h.core
        .page_show(&author(&a), show("La mienne", None, html("x")))
        .unwrap();
    // Another agent of the same project: not found, in the words of a page that does not exist.
    let e = h.core.page_get(&b.id, "p-1").unwrap_err();
    let none = h.core.page_get(&b.id, "p-99").unwrap_err();
    assert!(e.contains("Aucune page « p-1 »"), "{e}");
    assert_eq!(
        e.replace("p-1", "p-?"),
        none.replace("p-99", "p-?"),
        "the same words"
    );
    assert!(h
        .core
        .page_show(&author(&b), show("Volée", Some("p-1"), html("x")))
        .is_err());
    assert!(h.core.page_close(&b.id, "p-1").is_err());
    // Its own agent is told its pages.
    let e = h.core.page_get(&a.id, "p-7").unwrap_err();
    assert!(e.contains("Tes pages : p-1"), "{e}");
}

#[tokio::test]
async fn the_bounds_of_a_version_are_held_before_anything_is_written() {
    let (h, p, a) = world("pages-bounds").await;
    let big = "x".repeat(pages::HTML_MAX as usize + 1);
    assert!(pages::from_html(Lang::Fr, &big)
        .unwrap_err()
        .contains("2 Mo"));
    assert!(pages::from_html(Lang::Fr, "  ").is_err());
    let mut b = Bundle::default();
    for k in 0..pages::FILES_MAX {
        b.add(Lang::Fr, format!("f{k}.txt"), vec![b'x']).unwrap();
    }
    assert!(b
        .add(Lang::Fr, "one-more.txt".into(), vec![b'x'])
        .unwrap_err()
        .contains("64 fichiers"));
    let mut heavy = Bundle::default();
    heavy
        .add(
            Lang::Fr,
            "a.bin".into(),
            vec![0; pages::VERSION_MAX as usize],
        )
        .unwrap();
    assert!(heavy
        .add(Lang::Fr, "b.bin".into(), vec![0])
        .unwrap_err()
        .contains("10 Mo"));
    // A title of one line, 120 characters at most.
    assert!(pages::checked_title(Lang::Fr, "").is_err());
    assert!(pages::checked_title(Lang::Fr, "a\nb").is_err());
    assert!(pages::checked_title(Lang::Fr, &"é".repeat(121)).is_err());
    assert_eq!(
        pages::checked_title(Lang::Fr, "  Revue  ").unwrap(),
        "Revue"
    );
    // Nothing of a refused page was written.
    let _ = (p, a);
    assert!(!root(&h).join("pages").exists());
}

#[tokio::test]
async fn the_pages_of_the_machine_make_room_by_the_oldest_page_whose_row_is_closed() {
    let (h, p, a) = world("pages-room").await;
    let first = h
        .core
        .page_show(&author(&a), show("Ancienne", None, html("old")))
        .unwrap();
    let open = h
        .core
        .page_show(&author(&a), show("Ouverte", None, html("open")))
        .unwrap();
    h.core.page_close(&a.id, &first.page).unwrap();
    // Weighed as if they filled the machine.
    {
        let mut b = h.core.pages.lock();
        for pg in b.list.iter_mut() {
            pg.versions[0].bytes = pages::ALL_MAX / 2;
        }
    }
    let third = h
        .core
        .page_show(&author(&a), show("Nouvelle", None, html("new")))
        .unwrap();
    assert_eq!(third.page, "p-3");
    let ids: Vec<String> = h
        .core
        .pages
        .lock()
        .list
        .iter()
        .map(|p| p.id.clone())
        .collect();
    assert_eq!(ids, [open.page.clone(), third.page.clone()]);
    assert!(!pages::folder(&root(&h), &p.id, &first.page).exists());
    // With every page's row open, there is no room: said, nothing erased.
    {
        let mut b = h.core.pages.lock();
        for pg in b.list.iter_mut() {
            pg.versions[0].bytes = pages::ALL_MAX / 2;
        }
    }
    let e = h
        .core
        .page_show(&author(&a), show("Trop", None, html("x")))
        .unwrap_err();
    assert!(e.contains("500 Mo"), "{e}");
    assert_eq!(h.core.pages.lock().list.len(), 2);
}

#[tokio::test]
async fn the_registry_is_in_the_durable_state_and_a_number_is_never_given_twice() {
    let (h, p, a) = world("pages-durable").await;
    h.core
        .page_show(&author(&a), show("Revue", None, html("une")))
        .unwrap();
    h.core
        .page_show(&author(&a), show("Revue", Some("p-1"), html("deux")))
        .unwrap();
    h.core
        .page_show(&author(&a), show("Schéma", None, html("s")))
        .unwrap();
    h.core.save_state().unwrap();
    let saved: PersistedState =
        serde_json::from_slice(&std::fs::read(h.core.data.state_file()).unwrap()).unwrap();
    assert_eq!(saved.page_seq, 2);
    assert_eq!(saved.pages.len(), 2);
    assert_eq!(saved.pages[0].versions.len(), 2);
    assert_eq!(saved.pages[0].agent_name, "pagination");
    // The rows are of the run: none comes back.
    let app = mock_app();
    let data = DataDir::new(h.dir.join("data"));
    let (again, _rx): (Arc<Core<MockRuntime>>, _) = Core::load(app.handle().clone(), data);
    put_agent_in(&again, &a);
    let seen = again.page_get(&a.id, "p-1").unwrap();
    assert_eq!(seen.page.versions.len(), 2);
    assert_eq!(seen.row, None);
    assert!(again.page_rows(&p.id).is_empty());
    // A page erased leaves its number taken.
    again.pages.lock().list.clear();
    let next = again
        .page_show(&author(&a), show("Encore", None, html("x")))
        .unwrap();
    assert_eq!(next.page, "p-3");
}

fn put_agent_in(core: &Arc<Core<MockRuntime>>, meta: &AgentMeta) {
    if core.agent(&meta.id).is_ok() {
        return;
    }
    let rt = AgentRt::new(meta.clone(), &core.data.conversations());
    core.agents
        .write()
        .insert(meta.id.clone(), Arc::new(parking_lot::Mutex::new(rt)));
}

#[tokio::test]
async fn a_page_goes_the_days_of_the_box_after_its_agent_and_at_once_with_its_project() {
    let (h, p, a) = world("pages-purge").await;
    h.core
        .page_show(&author(&a), show("Revue", None, html("x")))
        .unwrap();
    let dir = pages::folder(&root(&h), &p.id, "p-1");
    assert!(dir.is_dir());
    // Its agent there: kept.
    h.core.pages_sweep();
    assert!(h.core.pages.lock().list[0].orphaned_at.is_none());
    // Archived: marked, kept for the days of the box's closed files (30 by default).
    h.core.agent(&a.id).unwrap().lock().meta.archived = true;
    h.core.pages_sweep();
    let since = h.core.pages.lock().list[0].orphaned_at.unwrap();
    // Back: no longer marked.
    h.core.agent(&a.id).unwrap().lock().meta.archived = false;
    h.core.pages_sweep();
    assert!(h.core.pages.lock().list[0].orphaned_at.is_none());
    // Deleted, 29 days ago: kept; 30 days ago: gone, with its files.
    h.core.agents.write().remove(&a.id);
    h.core.pages_sweep();
    h.core.pages.lock().list[0].orphaned_at = Some(since - 29 * DAY_MS);
    h.core.pages_sweep();
    assert_eq!(h.core.pages.lock().list.len(), 1);
    h.core.pages.lock().list[0].orphaned_at = Some(since - 31 * DAY_MS);
    h.core.pages_sweep();
    assert!(h.core.pages.lock().list.is_empty());
    assert!(!dir.exists());
    // « Garder les fichiers des demandes closes » at 7 days: a week after.
    h.core.settings.write().inbox_keep_days = 7;
    let b = put_agent(&h, &p, "autre");
    h.core
        .page_show(&author(&b), show("Autre", None, html("x")))
        .unwrap();
    h.core.agents.write().remove(&b.id);
    h.core.pages_sweep();
    h.core.pages.lock().list[0].orphaned_at = Some(since - 8 * DAY_MS);
    h.core.pages_sweep();
    assert!(h.core.pages.lock().list.is_empty());
    // A project gone takes its pages at once.
    let c = put_agent(&h, &p, "troisieme");
    h.core
        .page_show(&author(&c), show("Dernière", None, html("x")))
        .unwrap();
    h.core.projects.write().retain(|x| x.id != p.id);
    h.core.pages_sweep();
    assert!(h.core.pages.lock().list.is_empty());
    assert!(!pages::folder(&root(&h), &p.id, "p-3").exists());
}

#[tokio::test]
async fn off_the_pages_refuse_and_their_rows_close_with_nothing_erased() {
    let (h, p, a) = world("pages-switch").await;
    h.core
        .page_show(&author(&a), show("Revue", None, html("x")))
        .unwrap();
    assert_eq!(h.core.page_rows(&p.id).len(), 1);
    let mut s = h.core.settings.read().clone();
    s.agent_pages = false;
    h.core.save_settings(s).unwrap();
    assert!(!h.core.pages_on());
    assert!(h.core.page_rows(&p.id).is_empty());
    let e = h
        .core
        .page_show(&author(&a), show("Autre", None, html("x")))
        .unwrap_err();
    assert!(e.contains("Pages des agents"), "{e}");
    assert!(h.core.page_get(&a.id, "p-1").is_err());
    // Nothing erased: back on, the page is there, its row closed.
    let mut s = h.core.settings.read().clone();
    s.agent_pages = true;
    h.core.save_settings(s).unwrap();
    let seen = h.core.page_get(&a.id, "p-1").unwrap();
    assert_eq!(seen.row, None);
    assert!(pages::version_dir(&root(&h), &p.id, "p-1", 1).is_dir());
}

#[tokio::test]
async fn a_page_shown_is_served_to_its_own_webview_from_its_last_version() {
    let (h, p, a) = world("pages-served").await;
    h.core.app.manage(Arc::new(Shelter::default()));
    let shelter = h.core.app.state::<Arc<Shelter>>().inner().clone();
    h.core
        .page_show(&author(&a), show("Revue", None, html("première")))
        .unwrap();
    let body = |label: &str| {
        let r = shelter.serve(label, "GET", "escouade-page://p-1.localhost/", false);
        (
            r.status().as_u16(),
            String::from_utf8_lossy(r.body()).into_owned(),
        )
    };
    let (status, text) = body("agentpage-p-1");
    assert_eq!(status, 200);
    assert!(text.contains("première"), "{text}");
    // Its CSP put at its head.
    assert!(text.contains("Content-Security-Policy"), "{text}");
    // Another webview is served nothing of it.
    assert_eq!(body("agentpage-p-2").0, 404);
    assert_eq!(body("main").0, 404);
    h.core
        .page_show(&author(&a), show("Revue", Some("p-1"), html("seconde")))
        .unwrap();
    assert!(body("agentpage-p-1").1.contains("seconde"));
    // Its row closed: still served (the card opens it again); erased with its project: no more.
    h.core.page_close(&a.id, "p-1").unwrap();
    assert_eq!(body("agentpage-p-1").0, 200);
    h.core.projects.write().retain(|x| x.id != p.id);
    h.core.pages_sweep();
    assert_eq!(body("agentpage-p-1").0, 404);
}

#[tokio::test]
async fn the_window_says_where_a_row_is_shown() {
    let (h, p, a) = world("pages-placed").await;
    h.core
        .page_show(&author(&a), show("Revue", None, html("x")))
        .unwrap();
    h.core.page_placed("p-1", Placement::Beside);
    assert_eq!(h.core.page_rows(&p.id)[0].1, Placement::Beside);
    // A page with no row is not given one.
    h.core.page_placed("p-9", Placement::Full);
    assert_eq!(h.core.page_rows(&p.id).len(), 1);
}

#[tokio::test]
async fn the_window_draws_a_row_goes_back_to_a_version_and_opens_a_closed_page_again() {
    let (h, p, a) = world("pages-window").await;
    h.core.app.manage(Arc::new(Shelter::default()));
    let shelter = h.core.app.state::<Arc<Shelter>>().inner().clone();
    let body = || {
        let r = shelter.serve(
            "agentpage-p-1",
            "GET",
            "escouade-page://p-1.localhost/",
            false,
        );
        String::from_utf8_lossy(r.body()).into_owned()
    };
    h.core
        .page_show(&author(&a), show("Revue", None, html("première")))
        .unwrap();
    h.core
        .page_show(&author(&a), show("Revue v2", Some("p-1"), html("seconde")))
        .unwrap();
    let rows = h.core.page_open_rows();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(
        (row.page.as_str(), row.project_id.as_str()),
        ("p-1", p.id.as_str())
    );
    assert_eq!(
        (row.title.as_str(), row.agent_name.as_str()),
        ("Revue v2", "pagination")
    );
    assert_eq!(row.versions.iter().map(|v| v.n).collect::<Vec<_>>(), [1, 2]);
    assert_eq!((row.shown, row.open, row.quiet), (2, true, false));
    // « v1 ▾ »: the row shows its first version, served again; a version not kept is refused.
    let back = h.core.page_show_version("p-1", 1).unwrap();
    assert_eq!(back.shown, 1);
    assert!(body().contains("première"));
    let e = h.core.page_show_version("p-1", 7).unwrap_err();
    assert!(e.contains("plus gardée"), "{e}");
    // The agent's next version shows the last again.
    h.core
        .page_show(
            &author(&a),
            show("Revue v3", Some("p-1"), html("troisième")),
        )
        .unwrap();
    assert_eq!(h.core.page_row("p-1").unwrap().shown, 3);
    assert!(body().contains("troisième"));
    // Closed by its ×: kept, its card opens it again on its last version.
    assert!(h.core.page_close_row("p-1"));
    assert!(!h.core.page_close_row("p-1"));
    let closed = h.core.page_row("p-1").unwrap();
    assert!(!closed.open);
    assert!(h.core.page_open_rows().is_empty());
    assert!(h.core.page_show_version("p-1", 2).is_err());
    let again = h.core.page_reopen("p-1").unwrap();
    assert_eq!((again.open, again.shown), (true, 3));
    assert_eq!(h.core.page_rows(&p.id)[0].1, Placement::Hidden);
    // `open: quiet`: its row waits.
    let mut quiet = show("Calme", None, html("c"));
    quiet.quiet = true;
    h.core.page_show(&author(&a), quiet).unwrap();
    assert!(h.core.page_row("p-2").unwrap().quiet);
    // Off: nothing opens again; its project gone, the page is no longer kept.
    let mut s = h.core.settings.read().clone();
    s.agent_pages = false;
    h.core.save_settings(s).unwrap();
    assert!(h
        .core
        .page_reopen("p-1")
        .unwrap_err()
        .contains("Pages des agents"));
    let mut s = h.core.settings.read().clone();
    s.agent_pages = true;
    h.core.save_settings(s).unwrap();
    h.core.projects.write().retain(|x| x.id != p.id);
    h.core.pages_sweep();
    assert!(h.core.page_row("p-1").is_none());
    assert!(h
        .core
        .page_reopen("p-1")
        .unwrap_err()
        .contains("plus gardée"));
}

// ---------- the copy ----------

#[test]
fn a_name_a_file_gives_is_taken_within_its_folder_alone() {
    use Resolved::*;
    let r = pages::resolve;
    assert_eq!(r("", "app.js"), In("app.js".into()));
    assert_eq!(r("", "./img/x.png"), In("img/x.png".into()));
    assert_eq!(r("css", "../img/x.png?v=2#a"), In("img/x.png".into()));
    assert_eq!(r("", "img/a%20b.png"), In("img/a b.png".into()));
    assert_eq!(r("", "../secret.txt"), Out);
    assert_eq!(r("css", "../../x.png"), Out);
    assert_eq!(r("", ".env"), Out);
    assert_eq!(r("", "conf/.git/config"), Out);
    assert_eq!(r("", "_/choose"), Out);
    assert_eq!(r("", "_t/page.js"), Out);
    assert_eq!(r("", "index.html"), Out);
    assert_eq!(r("", "a\\b.png"), Out);
    assert_eq!(r("", "img/%2e%2e/%2e%2e/x"), Out);
    for ignored in [
        "",
        "#crit-3",
        "/etc/passwd",
        "\\\\server\\share",
        "https://example.com/x.js",
        "//example.com/x.js",
        "data:image/png;base64,AAAA",
        "mailto:a@b",
        "C:/Windows/win.ini",
        "javascript:alert(1)",
    ] {
        assert_eq!(r("", ignored), Ignored, "{ignored}");
    }
}

/// A page's folder in a worktree, and a secret beside it.
fn worktree(dir: &Path) -> std::path::PathBuf {
    let home = dir.join("wt");
    let page = home.join("design").join("revue");
    std::fs::create_dir_all(page.join("img")).unwrap();
    std::fs::create_dir_all(page.join("js")).unwrap();
    std::fs::write(home.join("secret.txt"), "TOP-SECRET-OUTSIDE").unwrap();
    std::fs::write(page.join(".env"), "TOKEN=TOP-SECRET-ENV").unwrap();
    std::fs::write(
        page.join("revue.html"),
        r##"<!doctype html><html><head>
<link rel="stylesheet" href="./style.css">
<link rel="icon" href="/favicon.ico">
<script type="module" src="js/app.js"></script>
</head><body>
<img src='img/a.png' srcset="img/a.png 1x, img/a@2x.png 2x">
<img src=img/missing.png>
<a href="#crit-3">ici</a> <a href="https://example.com/">web</a>
<img src="../../secret.txt"> <script src=".env"></script>
<a href="revue.html">moi</a>
</body></html>"##,
    )
    .unwrap();
    std::fs::write(
        page.join("style.css"),
        "@import 'theme.css'; body { background: url(\"img/bg.png\") } .x { background: url(../../secret.txt) }",
    )
    .unwrap();
    std::fs::write(page.join("theme.css"), ":root{--a:1}").unwrap();
    std::fs::write(
        page.join("js").join("app.js"),
        "import { m } from './mod.js'; import('../js/lazy.js'); const s = 'from \"../../../secret.txt\"';",
    )
    .unwrap();
    std::fs::write(page.join("js").join("mod.js"), "export const m = 1;").unwrap();
    std::fs::write(page.join("js").join("lazy.js"), "export default 2;").unwrap();
    std::fs::write(page.join("img").join("a.png"), b"\x89PNG\r\n\x1a\nA").unwrap();
    std::fs::write(page.join("img").join("a@2x.png"), b"\x89PNG\r\n\x1a\nB").unwrap();
    std::fs::write(page.join("img").join("bg.png"), b"\x89PNG\r\n\x1a\nC").unwrap();
    home
}

#[test]
fn a_page_file_comes_with_the_files_it_names_and_never_with_anything_outside() {
    let dir = crate::paths::test_dir("pages-copy");
    let home = worktree(&dir);
    let file = home.join("design").join("revue").join("revue.html");
    let b = pages::from_file(Lang::Fr, &file, &home, &pages::open_within).unwrap();
    let mut names: Vec<&str> = b.files.iter().map(|(r, _)| r.as_str()).collect();
    assert_eq!(names[0], "index.html");
    names.sort();
    assert_eq!(
        names,
        [
            "img/a.png",
            "img/a@2x.png",
            "img/bg.png",
            "index.html",
            "js/app.js",
            "js/lazy.js",
            "js/mod.js",
            "style.css",
            "theme.css",
        ]
    );
    for (_, bytes) in &b.files {
        let text = String::from_utf8_lossy(bytes);
        assert!(!text.contains("TOP-SECRET-OUTSIDE") && !text.contains("TOP-SECRET-ENV"));
    }
    let mut out = b.left_out.clone();
    out.sort();
    // Even a name a script only holds in a string is left out, never read.
    assert_eq!(
        out,
        [
            "../../../secret.txt",
            "../../secret.txt",
            ".env",
            "img/missing.png"
        ]
    );
    // A file out of the working folder is refused, whatever its name.
    let elsewhere = dir.join("ailleurs.html");
    std::fs::write(&elsewhere, "<p>x</p>").unwrap();
    assert!(pages::from_file(Lang::Fr, &elsewhere, &home, &pages::open_within).is_err());
    // Not an HTML document.
    assert!(pages::from_file(
        Lang::Fr,
        &home.join("secret.txt"),
        &home,
        &pages::open_within
    )
    .unwrap_err()
    .contains(".html"));
}

#[test]
fn an_image_of_a_template_is_a_file_of_the_working_folder_and_an_image_by_its_content() {
    let dir = crate::paths::test_dir("pages-images");
    let home = worktree(&dir);
    let mut b = Bundle::default();
    b.add(Lang::Fr, "index.html".into(), b"<p>x</p>".to_vec())
        .unwrap();
    pages::add_images(
        Lang::Fr,
        &mut b,
        &["design/revue/img/a.png".into()],
        Some(&home),
        &pages::open_within,
    )
    .unwrap();
    assert!(b.files.iter().any(|(r, _)| r == "design/revue/img/a.png"));
    for bad in [
        "../secret.txt",
        "/etc/passwd",
        "secret.txt",
        "design/revue/.env",
    ] {
        assert!(
            pages::add_images(
                Lang::Fr,
                &mut b,
                &[bad.into()],
                Some(&home),
                &pages::open_within
            )
            .is_err(),
            "{bad}"
        );
    }
    std::fs::write(home.join("faux.png"), "<svg onload=alert(1)></svg>").unwrap();
    let e = pages::add_images(
        Lang::Fr,
        &mut b,
        &["faux.png".into()],
        Some(&home),
        &pages::open_within,
    )
    .unwrap_err();
    assert!(e.contains("n’est pas une image"), "{e}");
    // Without a working folder (a discussion), no image.
    assert!(pages::add_images(
        Lang::Fr,
        &mut Bundle::default(),
        &["design/revue/img/a.png".into()],
        None,
        &pages::open_within
    )
    .is_err());
}

#[test]
fn a_page_of_the_state_reads_and_weighs_its_versions() {
    let p = AgentPage {
        id: "p-4".into(),
        versions: vec![
            PageVersion {
                n: 1,
                bytes: 10,
                ..Default::default()
            },
            PageVersion {
                n: 2,
                bytes: 5,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    assert_eq!(p.weight(), 15);
    assert_eq!(pages::key("p-4"), "page:p-4");
    assert_eq!(pages::of_key("page:p-4"), Some("p-4"));
    assert_eq!(pages::of_key("page:p-04"), None);
    assert_eq!(pages::of_key("browser:p-4"), None);
    assert_eq!(pages::bracket(1_000), "small");
    assert_eq!(pages::bracket(500_000), "medium");
    assert_eq!(pages::bracket(5_000_000), "large");
}
