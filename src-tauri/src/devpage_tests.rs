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

/// The ports of this machine the project's launch declares, in these tests: 4100 to 4109 (its
/// commands and its agents' blocks), Escouade's server's never among them.
fn local(port: u16) -> bool {
    (4100..4110).contains(&port)
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
        assert_eq!(judge(&url(to), &home, own, local), Verdict::Here, "{to}");
    }
}

#[test]
fn another_web_origin_stays_in_the_tab() {
    // ESC-695: a tab is a real browser. A sign-in goes through its provider and comes back by a
    // redirection: links, redirections, forms and the bar stay in the tab, on any web origin.
    let home = url("http://localhost:41731/");
    for to in [
        "https://claude.ai/login",
        "https://accounts.google.com/o/oauth2/v2/auth?client_id=x&redirect_uri=y",
        "https://github.com/login/oauth/authorize?scope=read",
        "http://example.com/",
        "http://localhost.evil.com:41731/",
        "https://xn--80ak6aa92e.com/",
    ] {
        assert_eq!(judge(&url(to), &home, own, local), Verdict::Here, "{to}");
    }
    // From a site of the internet too, back to the launch included.
    let home = url("https://claude.ai/");
    for to in [
        "https://accounts.google.com/",
        "http://localhost:4105/callback",
    ] {
        assert_eq!(judge(&url(to), &home, own, local), Verdict::Here, "{to}");
    }
}

#[test]
fn this_machine_at_a_port_the_project_does_not_declare_is_refused() {
    // A site that stays in the tab could reach a service of this machine (Escouade's own server,
    // a database's console): the loopback at another port than the tab's own and the project's
    // declared ones goes nowhere, in every spelling of it.
    let home = url("http://localhost:41731/");
    for to in [
        "http://localhost:41732/",
        "http://127.0.0.1:22/",
        "http://127.1.2.3:8080/",
        "http://[::1]:5000/",
        "http://[::ffff:127.0.0.1]:5000/",
        "http://0.0.0.0:6379/",
        "http://app.localhost:9000/",
        "http://2130706433:5000/",
        "https://localhost/",
    ] {
        assert_eq!(judge(&url(to), &home, own, local), Verdict::Nowhere, "{to}");
    }
    // Its own port, on any name of the loopback, and the declared ones.
    for to in [
        "http://127.0.0.1:41731/",
        "https://localhost:41731/",
        "http://localhost:4100/",
        "http://[::1]:4109/x",
    ] {
        assert_eq!(judge(&url(to), &home, own, local), Verdict::Here, "{to}");
    }
    // A site of the internet has no port of this machine of its own: its 443 is not the loopback's.
    let home = url("https://claude.ai/");
    for to in ["https://localhost/", "http://127.0.0.1:443/"] {
        assert_eq!(judge(&url(to), &home, own, local), Verdict::Nowhere, "{to}");
    }
}

#[test]
fn mail_and_phone_open_in_the_system() {
    let home = url("http://localhost:41731/");
    for to in ["mailto:someone@example.com", "tel:+33123456789"] {
        assert_eq!(judge(&url(to), &home, own, local), Verdict::Outside, "{to}");
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
        assert_eq!(judge(&url(to), &home, own, local), Verdict::Nowhere, "{to}");
    }
    // Nor from another site.
    let home = url("https://example.com/");
    for to in ["http://localhost:1420/", "http://tauri.localhost/"] {
        assert_eq!(
            judge(&url(to), &home, own, |_| true),
            Verdict::Nowhere,
            "{to}"
        );
    }
}

#[test]
fn files_scripts_and_other_schemes_go_nowhere() {
    let home = url("http://localhost:41731/");
    for to in [
        "file:///C:/Users/me/.escouade/settings.json",
        "javascript:alert(1)",
        "data:text/html,<p>x</p>",
        "blob:http://localhost:41731/0f1e-2d3c",
        "ipc://localhost/open_url",
        "http://ipc.localhost/open_url",
        "asset://localhost/x",
        "vscode://file/x",
        "ms-settings:privacy",
        "ftp://example.com/",
        "ws://example.com/",
    ] {
        assert_eq!(judge(&url(to), &home, own, local), Verdict::Nowhere, "{to}");
    }
}

#[test]
fn the_windows_a_page_opens_are_bounded_and_close_with_their_tab() {
    // ESC-695: a sign-in's window opens in Escouade, on the tab's session; a few at a time.
    let pages = Pages::default();
    for n in 0..POPUPS_MAX {
        assert!(pages.add_popup("browser:b1", &format!("w{n}")), "{n}");
    }
    assert!(!pages.add_popup("browser:b1", "w9"), "beyond the bound");
    assert_eq!(pages.popups("browser:b1"), POPUPS_MAX);
    assert_eq!(pages.popups("browser:b2"), 0);
    // Another tab has its own.
    assert!(pages.add_popup("browser:b2", "x0"));
    // One closed by its page (`window.close`) makes room again.
    pages.remove_popup("w1");
    assert!(pages.add_popup("browser:b1", "w3"));
    // The tab closes: its windows with it, the other tab's kept.
    let mut gone = pages.take_popups("browser:b1");
    gone.sort();
    assert_eq!(gone, vec!["w0", "w2", "w3"]);
    assert!(pages.take_popups("browser:b1").is_empty());
    assert_eq!(pages.take_popups("browser:b2"), vec!["x0"]);
}

#[test]
fn a_pages_window_is_refused_every_command_and_says_its_origin_first() {
    // Its label is its tab's, with a `:` that no tab's label holds: no command answers it.
    let label = popup_label("browser:b1", 3);
    assert_eq!(label, "devpage-browser_b1:w3");
    assert!(refuses(&label));
    assert!(refuses(&popup_label("agent-1", 1)));
    assert_ne!(popup_label("browser:b1", 1), page_label("browser:b1-w1"));
    // Its title: the origin, which the page does not choose, before the document's title.
    let at = url("https://accounts.google.com/o/oauth2/v2/auth?client_id=x");
    assert_eq!(popup_title(&at, ""), "https://accounts.google.com");
    assert_eq!(
        popup_title(&at, "Sign in – Google\u{202e}\nAccounts"),
        "https://accounts.google.com — Sign in – Google Accounts"
    );
    assert_eq!(
        popup_title(&url("https://evil.example/"), "https://accounts.google.com"),
        "https://evil.example — https://accounts.google.com"
    );
    assert_eq!(popup_title(&url("about:blank"), ""), "");
}

#[test]
fn a_pages_window_closes_by_its_own_address_a_tab_refuses_it() {
    // The window's `window.close` goes to `CLOSE_URL`, which its own navigation closes it by; in a
    // tab, that address is refused as any other of Escouade's schemes.
    let script = close_script();
    assert!(script.contains(CLOSE_URL));
    assert!(script.contains("defineProperty(window, 'close'"));
    let home = url("http://localhost:41731/");
    assert_eq!(judge(&url(CLOSE_URL), &home, own, local), Verdict::Nowhere);
}

#[test]
fn a_pages_window_has_a_size_of_its_own_whatever_the_page_asks() {
    // A page asks any number (WebView2 gives them unsigned: a negative one comes huge); the window's
    // place is Escouade's, never the page's.
    assert_eq!(popup_size(None), (520.0, 680.0));
    assert_eq!(popup_size(Some((480.0, 600.0))), (480.0, 600.0));
    assert_eq!(popup_size(Some((4_294_935_296.0, 1.0))), (1200.0, 320.0));
    assert_eq!(
        popup_size(Some((f64::NAN, f64::INFINITY))),
        (1200.0, 1000.0)
    );
}

#[test]
fn a_page_cannot_open_its_devtools_by_itself() {
    // ESC-657: the devtools open on a gesture of the user alone. The address the page's shortcut went
    // to before (`escouade-devtools://open/`) is now refused like any other scheme: a script of the
    // page that goes there opens nothing.
    let home = url("http://localhost:41731/");
    assert_eq!(
        judge(&url("escouade-devtools://open/"), &home, own, local),
        Verdict::Nowhere
    );
    // Its script holds the width's shortcut alone: no F12, no Ctrl+Shift+I, nothing of Tauri's.
    let script = shortcut_script();
    assert!(!script.contains("F12"));
    assert!(!script.contains("KeyI"));
    assert!(!script.contains("escouade-devtools://open/"));
    assert!(!script.contains("__TAURI"));
}

#[test]
fn a_page_without_an_agent_has_a_key_of_its_own() {
    // A browser opened alone (ESC-657), and the page of a launch command of this project (ESC-560).
    assert!(agentless_key("browser:b1", "p1").is_ok());
    assert!(agentless_key("browser:k3x-9_a", "p1").is_ok());
    assert!(agentless_key("run:p1:c1", "p1").is_ok());
    for key in [
        "",
        "browser:",
        "a1",
        "browser:a/b",
        "browser:a b",
        "browser:é",
        // Another project's command, or none.
        "run:p2:c1",
        "run:p1:",
        "run:p1",
        "devpage-a1",
    ] {
        assert!(agentless_key(key, "p1").is_err(), "{key:?}");
    }
    assert!(agentless_key(&format!("browser:{}", "x".repeat(41)), "p1").is_err());
    // Its webview's label is a page's: no command answers it either.
    assert!(refuses(&page_label("browser:b1")));
    assert_eq!(page_label("browser:b1"), "devpage-browser_b1");
}

#[test]
fn a_documents_title_is_one_clean_line() {
    assert_eq!(
        clip_title("  Utilisateurs \n — demo  "),
        "Utilisateurs — demo"
    );
    assert_eq!(clip_title("a\u{200b}b\u{202e}c\u{feff}"), "abc");
    assert_eq!(clip_title("\u{7}bell"), "bell");
    assert_eq!(clip_title(&"é".repeat(200)).chars().count(), 120);
    assert_eq!(clip_title(""), "");
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
    for label in ["main", "space-s1", ""] {
        assert!(!refuses(label), "{label}");
    }
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
    // The « Pages » windows' bars are gone (ESC-657).
    assert_eq!(webviews, ["main"]);
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
    // The « Pages » window and its bar, Escouade's own interface with devtools in a debug build,
    // are gone (ESC-657): devpage.rs makes no webview of the app's own pages any more.
    assert!(
        !page.contains("WebviewUrl::App"),
        "an app's page made in devpage.rs"
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
                bounds: Some(Bounds {
                    x: 10.0,
                    y: 60.0,
                    width: 800.0,
                    height: 600.0,
                }),
                data_root: &dir,
                visible: true,
                local: Arc::new(local),
            },
            own.clone(),
            tell.clone(),
        )
    };
    open_at("a1", "http://localhost:41731/").unwrap();
    open_at("a1", "http://localhost:41731/users").unwrap();
    open_at("a2", "http://localhost:41742/").unwrap();
    assert!(pages.get("a1").is_some() && pages.get("a2").is_some());
    assert_eq!(pages.get("a1").unwrap().url, "http://localhost:41731/users");
    assert!(app.get_webview(&page_label("a1")).is_some());
    assert!(app.get_webview(&page_label("a2")).is_some());
    // Escouade's own address is never opened.
    assert!(open_at("a3", "tauri://localhost/").is_err());
    assert!(pages.get("a3").is_none());
    close(app.handle(), &pages, "a1").unwrap();
    assert!(pages.get("a1").is_none());
    assert!(pages.get("a2").is_some());
}

#[test]
fn the_widths_are_said_to_the_window_as_it_says_them() {
    // The window places the page at its width (`deviceFit`, devpage-key.ts): the backend keeps the
    // mode chosen, said as the window says it (the « Pages » window that laid it out itself is gone,
    // ESC-657).
    assert_eq!(
        serde_json::to_string(&PageMode::Mobile).unwrap(),
        "\"mobile\""
    );
    assert_eq!(
        serde_json::from_str::<PageMode>("\"tablet\"").unwrap(),
        PageMode::Tablet
    );
}

#[test]
fn the_width_shortcut_asks_the_window_for_the_next_width() {
    let home = url("http://localhost:41731/");
    assert_eq!(
        judge(&url(RESPONSIVE_URL), &home, own, local),
        Verdict::Responsive
    );
    let script = shortcut_script();
    assert!(script.contains(RESPONSIVE_URL));
    assert!(script.contains("e.code === 'KeyM'"));
}

#[test]
fn the_width_chosen_is_kept_on_the_page() {
    let pages = Pages::default();
    assert!(!pages.set_mode("a1", PageMode::Mobile), "no page yet");
}
