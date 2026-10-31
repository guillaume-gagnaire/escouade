//! Tests of the launches' pages (`devpage`, ESC-311): where a page may go, its labels, its storage
//! apart, the calls it makes refused, and its registry (one page per agent). The webviews
//! themselves (a child of the main window, the « Pages » window, the devtools) are tried by the
//! end-to-end test `e2e/devpage.spec.ts` and by hand.

use crate::devpage::*;
use std::path::Path;
use tauri::Url;

fn url(s: &str) -> Url {
    Url::parse(s).unwrap()
}

/// Escouade's own addresses as `navigation::destination` sees them on Windows, with a dev server.
fn own(u: &Url) -> bool {
    crate::navigation::destination(u, Some(&url("http://localhost:1420/")))
        == crate::navigation::Destination::App
        || matches!(u.host_str(), Some("tauri.localhost"))
        || u.scheme() == "tauri"
}

#[test]
fn a_page_stays_on_its_launchs_origin() {
    let home = url("http://localhost:41731/users?search=marie@");
    for to in [
        "http://localhost:41731/",
        "http://localhost:41731/users?cursor=abc",
        "http://localhost:41731/a/b#c",
        "about:blank",
    ] {
        assert_eq!(judge(&url(to), &home, own), Verdict::Here, "{to}");
    }
}

#[test]
fn another_origin_opens_in_the_systems_browser() {
    let home = url("http://localhost:41731/");
    for to in [
        // Another port, another host, another scheme of the same host: other origins.
        "http://localhost:41732/",
        "http://127.0.0.1:41731/",
        "https://localhost:41731/",
        "https://github.com/gagnaire/escouade",
        "http://localhost.evil.com:41731/",
        "mailto:someone@example.com",
    ] {
        assert_eq!(judge(&url(to), &home, own), Verdict::Outside, "{to}");
    }
}

#[test]
fn escouades_own_addresses_are_refused_even_on_the_launchs_origin() {
    // The launch is Escouade's own dev server: its pages are still refused, never loaded in a page.
    let home = url("http://localhost:1420/");
    for to in [
        "http://localhost:1420/",
        "http://localhost:1420/index.html",
        "http://tauri.localhost/",
        "tauri://localhost/",
    ] {
        assert_eq!(judge(&url(to), &home, own), Verdict::Nowhere, "{to}");
    }
}

#[test]
fn files_scripts_and_other_schemes_go_nowhere() {
    let home = url("http://localhost:41731/");
    for to in [
        "file:///C:/Users/me/.escouade/settings.json",
        "javascript:alert(1)",
        "data:text/html,<p>x</p>",
        "ipc://localhost/open_url",
        "http://ipc.localhost/open_url",
        "asset://localhost/x",
        "vscode://file/x",
    ] {
        let verdict = judge(&url(to), &home, own);
        assert!(
            verdict == Verdict::Nowhere || (verdict == Verdict::Outside && to.starts_with("http")),
            "{to}: {verdict:?}"
        );
        assert_ne!(verdict, Verdict::Here, "{to}");
    }
}

#[test]
fn the_devtools_shortcut_asks_for_the_devtools() {
    let home = url("http://localhost:41731/");
    assert_eq!(judge(&url(DEVTOOLS_URL), &home, own), Verdict::Devtools);
    // Its script sends F12, Cmd+Opt+I and Ctrl+Shift+I there, and nothing else is in it.
    let script = shortcut_script();
    assert!(script.contains("'F12'"));
    assert!(script.contains("e.metaKey && e.altKey"));
    assert!(script.contains("e.ctrlKey && e.shiftKey"));
    assert!(script.contains(DEVTOOLS_URL));
    assert!(!script.contains("__TAURI"));
}

#[test]
fn a_launch_opens_on_a_web_address_that_is_not_escouades() {
    assert_eq!(
        launch_url(" http://localhost:41731/users ", own).unwrap(),
        url("http://localhost:41731/users")
    );
    assert!(launch_url("https://app.localhost:8443/", own).is_ok());
    for text in [
        "",
        "localhost:41731",
        "file:///etc/passwd",
        "javascript:alert(1)",
        "http://localhost:1420/",
        "http://tauri.localhost/",
        "tauri://localhost/",
    ] {
        assert!(launch_url(text, own).is_err(), "{text:?}");
    }
}

#[test]
fn one_label_per_agent_and_no_command_answers_a_page() {
    assert_eq!(page_label("a1b2"), "devpage-a1b2");
    // What Tauri refuses in a label becomes an `_`: two agents never share one by accident of a
    // character, and a page's label can never be the window's.
    assert_eq!(page_label("a/b c:é"), "devpage-a_b_c__");
    assert_ne!(page_label("a"), page_label("b"));
    assert!(refuses(&page_label("anything")));
    for label in ["main", "pagesbar-p1", "pages-p1", ""] {
        assert!(!refuses(label), "{label}");
    }
    assert_eq!(window_label("p1"), "pages-p1");
    assert_eq!(bar_label("p1"), "pagesbar-p1");
}

#[test]
fn the_capability_names_the_windows_webviews_never_a_page() {
    let text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities/default.json"),
    )
    .unwrap();
    let cap: serde_json::Value = serde_json::from_str(&text).unwrap();
    // By window, a capability would also cover the pages, children of the main window.
    assert!(cap.get("windows").is_none(), "{cap}");
    let webviews: Vec<String> = cap["webviews"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(webviews, ["main", "pagesbar-*"]);
    for w in &webviews {
        let pattern = glob::Pattern::new(w).unwrap();
        assert!(!pattern.matches(&page_label("a1")), "{w}");
    }
    assert!(cap.get("remote").is_none(), "{cap}");
}

#[test]
fn each_project_keeps_its_pages_storage_apart_from_the_window() {
    let root = Path::new("/data/.escouade");
    let a = data_dir(root, "p1");
    assert_eq!(a, root.join("pages").join("p1"));
    assert_ne!(a, data_dir(root, "p2"));
    // Never the window's own folder, nor out of the pages' folder.
    assert!(data_dir(root, "../x").starts_with(root.join("pages")));
    assert_eq!(data_dir(root, "../x"), root.join("pages").join("___x"));
    // macOS: a data store per project, the same for its pages.
    assert_eq!(store_id("p1"), store_id("p1"));
    assert_ne!(store_id("p1"), store_id("p2"));
    assert_ne!(store_id("p1"), [0u8; 16]);
}

#[test]
fn the_window_never_has_devtools_in_a_release() {
    // Tauri's `devtools` gives every webview its devtools unless told otherwise: the main window is.
    let lib =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs")).unwrap();
    assert!(lib.contains("window.devtools = Some(cfg!(debug_assertions));"));
    let page =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/devpage.rs"))
            .unwrap();
    assert!(
        page.contains(".devtools(cfg!(debug_assertions))"),
        "the bar of « Pages »"
    );
}

/// A page made in a mock app: its label, its registry entry; opening it again keeps one page.
#[test]
fn launching_again_keeps_one_page_per_agent() {
    use std::sync::Arc;
    use tauri::Manager;
    let app = tauri::test::mock_app();
    let _main = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let pages = Arc::new(Pages::default());
    let told = Arc::new(parking_lot::Mutex::new(Vec::<PageEvent>::new()));
    let tell: Tell = {
        let told = told.clone();
        Arc::new(move |e| told.lock().push(e))
    };
    let own: Own = Arc::new(own);
    let dir = std::env::temp_dir().join(format!("escouade-devpage-{}", std::process::id()));
    let open_at = |agent: &str, u: &str| {
        open(
            app.handle(),
            &pages,
            Open {
                agent_id: agent,
                project_id: "p1",
                title: "pagination-users · web",
                url: u,
                detached: false,
                bounds: Some(Bounds {
                    x: 10.0,
                    y: 60.0,
                    width: 800.0,
                    height: 600.0,
                }),
                data_root: &dir,
                visible: true,
            },
            own.clone(),
            tell.clone(),
        )
    };
    open_at("a1", "http://localhost:41731/").unwrap();
    open_at("a1", "http://localhost:41731/users").unwrap();
    open_at("a2", "http://localhost:41742/").unwrap();
    assert_eq!(pages.of_project("p1").len(), 2);
    assert_eq!(pages.get("a1").unwrap().url, "http://localhost:41731/users");
    assert!(app.get_webview(&page_label("a1")).is_some());
    assert!(app.get_webview(&page_label("a2")).is_some());
    // Escouade's own address is never opened.
    assert!(open_at("a3", "tauri://localhost/").is_err());
    assert!(pages.get("a3").is_none());
    close(app.handle(), &pages, "a1").unwrap();
    assert!(pages.get("a1").is_none());
    assert_eq!(pages.of_project("p1").len(), 1);
}
