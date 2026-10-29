//! Keeps the main window inside the app's interface. It has no title bar and no back button, so a
//! page loaded in it (a link clicked in a conversation, issue #9) left the user stuck, the app
//! gone, with no way out but to kill it. Every navigation of the window, and every request for a
//! new window, is judged here (`destination`): the app's own pages go through; web and mail links
//! are refused and opened outside, in the system's browser or mail client; anything else is
//! refused and opened nowhere. `reload` is the way back should the window still end up elsewhere;
//! it asks first when the window is still on the app and the editor holds unsaved files.

use crate::core::Core;
use crate::i18n::{self, Lang};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::webview::NewWindowResponse;
use tauri::{AppHandle, Manager, Runtime, Url, WebviewWindowBuilder};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

/// Where an address the window is asked to load leads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    /// The app's own pages (and the dev server's, when the app runs from it): the window loads them.
    App,
    /// A web or mail link: the window refuses it, the system's browser or mail client opens it.
    Outside,
    /// Anything else (a file, a script, a data page, another scheme, another host of the app's
    /// scheme): refused, and opened nowhere.
    Nowhere,
}

/// What a request of the page is: it moves the window to an address, or it asks for a new window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Request {
    Navigation,
    NewWindow,
}

/// What `reload` needs of the window's setup, kept once the window is made.
pub struct Home {
    /// The page the window comes back to.
    pub url: Url,
    /// The dev server's address, when the app runs from it: its pages are the app's too.
    pub dev: Option<Url>,
}

/// Where `url` leads. The app's pages are served from `tauri://localhost` (macOS, Linux) and
/// `http(s)://tauri.localhost` (Windows only), and from the origin of `dev`, the dev server, when
/// the app runs from it. The origin is compared whole (scheme, host and port):
/// `tauri.localhost.evil.com` is not the app.
pub fn destination(url: &Url, dev: Option<&Url>) -> Destination {
    destination_on(url, dev, cfg!(windows))
}

/// `destination` on Windows (`windows`) or on another system. Only WebView2 serves the app from
/// `tauri.localhost`; elsewhere that name may resolve to the loopback, where whatever answers
/// would be taken for the app, so it is a web address like any other.
fn destination_on(url: &Url, dev: Option<&Url>, windows: bool) -> Destination {
    let own = matches!(
        (url.scheme(), url.host_str(), url.port()),
        ("tauri", Some("localhost"), None)
    ) || windows
        && matches!(
            (url.scheme(), url.host_str(), url.port()),
            ("http" | "https", Some("tauri.localhost"), None)
        );
    if own || dev.is_some_and(|dev| url.origin() == dev.origin()) {
        return Destination::App;
    }
    match url.scheme() {
        "http" | "https" | "mailto" => Destination::Outside,
        _ => Destination::Nowhere,
    }
}

/// What the log says of `url`: its scheme and host, never its path, query or credentials, which
/// may carry a token.
fn shown(url: &Url) -> String {
    match url.host_str() {
        Some(host) if !host.is_empty() => format!("{}://{host}", url.scheme()),
        _ => format!("{}:", url.scheme()),
    }
}

/// The line the log gets when a request is refused (`outside`: and the address opened outside).
/// End-to-end tests look for these words.
fn refusal(request: Request, url: &Url, outside: bool) -> String {
    let what = match request {
        Request::Navigation => "navigation",
        Request::NewWindow => "new window",
    };
    let opened = if outside { ", opened outside" } else { "" };
    format!("{what} refused: {}{opened}", shown(url))
}

/// Judges a request of the page: whether it is refused. A navigation to the app goes through; the
/// rest is refused, a new window always (the app never opens a second one). A refused address that
/// leads outside is handed to `open`.
fn judge(request: Request, url: &Url, dev: Option<&Url>, open: impl FnOnce(&Url)) -> bool {
    let destination = destination(url, dev);
    if request == Request::Navigation && destination == Destination::App {
        return false;
    }
    let outside = destination == Destination::Outside;
    log::info!("{}", refusal(request, url, outside));
    if outside {
        open(url);
    }
    true
}

/// Opens `url` in the system's browser or mail client, off the WebView's thread (the system may
/// take its time to answer).
fn open_outside(url: &Url) {
    let (address, shown) = (url.to_string(), shown(url));
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(e) = crate::outside::open(crate::outside::What::Url(&address)) {
            log::error!("{shown} not opened outside: {e}");
        }
    });
}

/// Puts the guard on the main window's `builder` (the handlers can only be set as the window is
/// made): its navigations and new windows are judged by `destination`. `dev` is the dev server's
/// address, when the app runs from it.
pub fn guard<'a, R: Runtime, M: Manager<R>>(
    builder: WebviewWindowBuilder<'a, R, M>,
    dev: Option<Url>,
) -> WebviewWindowBuilder<'a, R, M> {
    let navigating = dev.clone();
    builder
        .on_navigation(move |url| {
            !judge(Request::Navigation, url, navigating.as_ref(), open_outside)
        })
        .on_new_window(move |url, _features| {
            judge(Request::NewWindow, &url, dev.as_ref(), open_outside);
            NewWindowResponse::Deny
        })
}

/// The address of the app's first page: the dev server's, or the one the packaged app serves its
/// files from (`https` is the window's `useHttpsScheme`, which only Windows reads).
pub fn home(dev: Option<&Url>, https: bool) -> Url {
    let address = match dev {
        Some(dev) => return dev.clone(),
        None if cfg!(windows) && https => "https://tauri.localhost/",
        None if cfg!(windows) => "http://tauri.localhost/",
        None => "tauri://localhost/",
    };
    Url::parse(address).expect("an address written here")
}

/// Whether `reload` asks first: the window is on the app, so it holds files the editor may not
/// have saved. A window that left the app lost them already.
fn asks_before_reloading(on_app: bool, unsaved: usize) -> bool {
    on_app && unsaved > 0
}

/// The question, its « Reload » and its « Cancel », when `unsaved` files of the editor would be lost.
fn reload_dialog(lang: Lang, unsaved: usize) -> [String; 3] {
    [
        tr_n_in!(
            lang,
            unsaved,
            "L’éditeur a {n} fichier non enregistré : recharger l’interface le perd.",
            "L’éditeur a {n} fichiers non enregistrés : recharger l’interface les perd.",
            "The editor has {n} unsaved file: reloading the interface loses it.",
            "The editor has {n} unsaved files: reloading the interface loses them.",
            n = unsaved
        ),
        tr_in!(lang, "Recharger", "Reload"),
        tr_in!(lang, "Annuler", "Cancel"),
    ]
}

/// How many files the editor holds unsaved, as the window last said (none without a core).
fn unsaved_files<R: Runtime>(app: &AppHandle<R>) -> usize {
    app.try_state::<Arc<Core>>()
        .map_or(0, |core| core.unsaved.load(Ordering::Acquire))
}

/// Whether the main window is on one of the app's pages.
fn on_the_app<R: Runtime>(app: &AppHandle<R>) -> bool {
    let (Some(window), Some(home)) = (app.get_webview_window("main"), app.try_state::<Home>())
    else {
        return false;
    };
    window
        .url()
        .is_ok_and(|url| destination(&url, home.dev.as_ref()) == Destination::App)
}

/// `go_home`, at once: the page's process ended, nothing is left to ask it (`pagecrash`).
#[cfg_attr(not(any(target_os = "linux", windows)), allow(dead_code))]
pub fn load_home<R: Runtime>(app: &AppHandle<R>) {
    go_home(app);
}

/// Takes the window to its home page and shows it, whatever state it was in (hidden in the tray,
/// minimized).
fn go_home<R: Runtime>(app: &AppHandle<R>) {
    if let (Some(window), Some(home)) = (app.get_webview_window("main"), app.try_state::<Home>()) {
        log::info!("reloading the interface");
        if let Err(e) = window.navigate(home.url.clone()) {
            log::error!("interface not reloaded: {e}");
        }
    }
    crate::notify::show_main(app);
}

/// `reload` with `unsaved` files in the editor. When it has to ask, the window is shown and `ask`
/// gets the count and the way to go on, which it calls on a « yes ».
fn reload_with<R: Runtime>(
    app: &AppHandle<R>,
    unsaved: usize,
    ask: impl FnOnce(usize, Box<dyn FnOnce() + Send>),
) {
    if asks_before_reloading(on_the_app(app), unsaved) {
        crate::notify::show_main(app);
        let handle = app.clone();
        ask(unsaved, Box::new(move || go_home(&handle)));
    } else {
        go_home(app);
    }
}

/// Asks with a dialog of the system, over the window: a page that froze cannot ask itself.
fn ask_to_reload<R: Runtime>(app: &AppHandle<R>, unsaved: usize, go: Box<dyn FnOnce() + Send>) {
    let [question, reload, cancel] = reload_dialog(i18n::ui(), unsaved);
    app.dialog()
        .message(question)
        .title("Escouade")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(reload, cancel))
        .show(move |confirmed| {
            if confirmed {
                go();
            }
        });
}

/// The way back to the app should the window be anywhere else: it goes to its home page and is
/// shown, whatever state it was in (hidden in the tray, minimized). The tray icon's « Recharger
/// l’interface ». A window still on the app is reloaded only once the user agrees, when the editor
/// holds unsaved files that the reload loses; one that left the app holds nothing, it goes at once.
pub fn reload<R: Runtime>(app: &AppHandle<R>) {
    let handle = app.clone();
    reload_with(app, unsaved_files(app), move |unsaved, go| {
        ask_to_reload(&handle, unsaved, go)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::WebviewUrl;
    use Destination::{App, Nowhere, Outside};

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    fn dev() -> Url {
        url("http://localhost:1420/")
    }

    #[test]
    fn the_app_s_own_pages_stay_in_the_window() {
        for page in [
            "tauri://localhost/",
            "tauri://localhost/index.html",
            "tauri://localhost/#/settings",
        ] {
            assert_eq!(destination(&url(page), None), App, "{page}");
        }
    }

    #[test]
    fn tauri_localhost_is_the_app_on_windows_only() {
        // WebView2 serves the app from there; elsewhere the name may well resolve to the loopback,
        // where anything may answer: it is a web address like any other.
        let there = if cfg!(windows) { App } else { Outside };
        for page in [
            "http://tauri.localhost/",
            "https://tauri.localhost/x?y#z",
            "https://tauri.localhost/#/settings",
        ] {
            assert_eq!(destination(&url(page), None), there, "{page}");
            assert_eq!(destination(&url(page), Some(&dev())), there, "{page}");
        }
    }

    #[test]
    fn each_system_judges_the_loopback_name_by_its_own_rule() {
        // Both systems on any machine, whatever `destination` reads of this one.
        for page in ["http://tauri.localhost/", "https://tauri.localhost/x"] {
            assert_eq!(destination_on(&url(page), None, true), App, "{page}");
            assert_eq!(destination_on(&url(page), None, false), Outside, "{page}");
        }
        // The app's own scheme is the app's on both.
        for windows in [true, false] {
            assert_eq!(
                destination_on(&url("tauri://localhost/"), None, windows),
                App
            );
        }
        // And what is not the app's stays out on both.
        for windows in [true, false] {
            for (link, expected) in [
                ("http://tauri.localhost.evil.com/", Outside),
                ("https://tauri.localhost:8443/", Outside),
                ("tauri://evil/", Nowhere),
            ] {
                assert_eq!(
                    destination_on(&url(link), None, windows),
                    expected,
                    "{link}"
                );
            }
        }
        assert_eq!(
            destination(&url("http://tauri.localhost/"), None),
            destination_on(&url("http://tauri.localhost/"), None, cfg!(windows))
        );
    }

    #[test]
    fn the_dev_server_is_the_app_only_when_it_is_given_and_on_the_same_port() {
        let page = url("http://localhost:1420/src/main.ts");
        assert_eq!(destination(&page, Some(&dev())), App);
        assert_eq!(
            destination(&url("http://localhost:1421/"), Some(&dev())),
            Outside
        );
        assert_eq!(
            destination(&url("https://localhost:1420/"), Some(&dev())),
            Outside
        );
        assert_eq!(
            destination(&url("http://127.0.0.1:1420/"), Some(&dev())),
            Outside
        );
        assert_eq!(destination(&page, None), Outside);
        // The app's own origins stay the app's with a dev server given.
        assert_eq!(destination(&url("tauri://localhost/"), Some(&dev())), App);
    }

    #[test]
    fn the_web_and_mail_links_go_outside() {
        for link in [
            "https://example.com/a?token=x",
            "http://example.com",
            "http://127.0.0.1:4260/x",
            "http://localhost:1420/",
            "mailto:someone@example.com",
            "MAILTO:someone@example.com",
            // Looks like the app's host, is not.
            "http://tauri.localhost.evil.com/",
            "https://tauri.localhost:8443/",
            "http://tauri.localhost@evil.com/",
        ] {
            assert_eq!(destination(&url(link), None), Outside, "{link}");
        }
    }

    #[test]
    fn what_a_system_could_not_open_safely_goes_nowhere() {
        for link in [
            "about:blank",
            "file:///C:/Windows/win.ini",
            "data:text/html,x",
            "javascript:alert(1)",
            "blob:http://tauri.localhost/1234",
            "tel:+33100000000",
            "ftp://example.com/",
            "tauri://evil/",
            "tauri://localhost:8080/",
            "ws://localhost:1420/",
        ] {
            assert_eq!(destination(&url(link), Some(&dev())), Nowhere, "{link}");
        }
    }

    #[test]
    fn a_url_is_shown_in_the_log_by_its_scheme_and_host_only() {
        for (link, shown_in_log) in [
            ("https://example.com/a?token=x#frag", "https://example.com"),
            (
                "https://user:pw@example.com:8443/a?token=x",
                "https://example.com",
            ),
            ("http://127.0.0.1:4260/x", "http://127.0.0.1"),
            ("tauri://evil/path", "tauri://evil"),
            ("about:blank", "about:"),
            ("file:///C:/Windows/win.ini", "file:"),
            ("mailto:someone@example.com", "mailto:"),
            ("javascript:alert(1)", "javascript:"),
            ("data:text/html,secret", "data:"),
        ] {
            assert_eq!(shown(&url(link)), shown_in_log, "{link}");
        }
    }

    #[test]
    fn a_refusal_is_logged_with_what_end_to_end_tests_look_for() {
        let line = |request, link, outside| refusal(request, &url(link), outside);
        assert_eq!(
            line(Request::Navigation, "https://example.com/a?t=1", true),
            "navigation refused: https://example.com, opened outside"
        );
        assert_eq!(
            line(Request::Navigation, "about:blank", false),
            "navigation refused: about:"
        );
        assert_eq!(
            line(Request::NewWindow, "about:blank", false),
            "new window refused: about:"
        );
        assert_eq!(
            line(Request::NewWindow, "mailto:a@b.c", true),
            "new window refused: mailto:, opened outside"
        );
    }

    /// What `judge` opened outside, and whether it refused.
    fn judged(request: Request, link: &str, dev: Option<&Url>) -> (bool, Vec<String>) {
        let opened = std::cell::RefCell::new(Vec::new());
        let refused = judge(request, &url(link), dev, |u| {
            opened.borrow_mut().push(u.to_string())
        });
        (refused, opened.into_inner())
    }

    #[test]
    fn a_navigation_to_the_app_goes_through_and_opens_nothing() {
        assert_eq!(
            judged(Request::Navigation, "tauri://localhost/", None),
            (false, vec![])
        );
        // On Windows, where WebView2 serves the app from it; elsewhere it is the web.
        let link = "http://tauri.localhost/x#y";
        let expected = if cfg!(windows) {
            (false, vec![])
        } else {
            (true, vec![link.to_string()])
        };
        assert_eq!(judged(Request::Navigation, link, None), expected);
        let dev = dev();
        assert_eq!(
            judged(Request::Navigation, "http://localhost:1420/a", Some(&dev)),
            (false, vec![])
        );
    }

    #[test]
    fn a_navigation_to_the_web_or_mail_is_refused_and_opened_outside() {
        for link in [
            "https://example.com/a?token=x",
            "mailto:someone@example.com",
        ] {
            assert_eq!(
                judged(Request::Navigation, link, None),
                (true, vec![link.to_string()])
            );
        }
    }

    #[test]
    fn a_navigation_anywhere_else_is_refused_and_opened_nowhere() {
        for link in [
            "about:blank",
            "file:///C:/Windows/win.ini",
            "javascript:alert(1)",
        ] {
            assert_eq!(judged(Request::Navigation, link, None), (true, vec![]));
        }
        // Another host of the app's scheme is not the app.
        assert_eq!(
            judged(Request::Navigation, "tauri://evil/", Some(&dev())),
            (true, vec![])
        );
    }

    #[test]
    fn a_new_window_is_always_refused_and_only_the_outside_ones_are_opened() {
        let dev = dev();
        for (link, opened) in [
            ("https://example.com/", true),
            ("mailto:someone@example.com", true),
            // The app never opens a second window, not even for itself.
            ("tauri://localhost/", false),
            ("http://localhost:1420/", false),
            ("about:blank", false),
            ("file:///C:/Windows/win.ini", false),
        ] {
            let expected = if opened {
                vec![link.to_string()]
            } else {
                vec![]
            };
            assert_eq!(
                judged(Request::NewWindow, link, Some(&dev)),
                (true, expected),
                "{link}"
            );
        }
    }

    #[test]
    fn the_home_page_is_the_dev_server_in_dev_and_the_app_s_own_address_otherwise() {
        let dev = dev();
        assert_eq!(home(Some(&dev), false), dev);
        let packaged = if cfg!(windows) {
            "http://tauri.localhost/"
        } else {
            "tauri://localhost/"
        };
        assert_eq!(home(None, false), url(packaged));
        if cfg!(windows) {
            assert_eq!(home(None, true), url("https://tauri.localhost/"));
        }
        // Whatever the way it is made, the home page is one the window may load.
        for (dev, https) in [(Some(&dev), false), (None, false), (None, true)] {
            assert_eq!(destination(&home(dev, https), dev), App);
        }
    }

    #[test]
    fn reloading_brings_a_window_that_left_the_app_back_to_its_home_page() {
        let app = tauri::test::mock_app();
        let home_page = url("tauri://localhost/");
        app.manage(Home {
            url: home_page.clone(),
            dev: None,
        });
        let stranded = url("https://example.com/");
        let window = guard(
            WebviewWindowBuilder::new(app.handle(), "main", WebviewUrl::External(stranded.clone())),
            None,
        )
        .build()
        .unwrap();
        assert_eq!(window.url().unwrap(), stranded);
        reload(app.handle());
        assert_eq!(window.url().unwrap(), home_page);
    }

    #[test]
    fn reloading_without_a_window_or_a_home_page_does_nothing() {
        let app = tauri::test::mock_app();
        reload(app.handle());
        app.manage(Home {
            url: url("tauri://localhost/"),
            dev: None,
        });
        reload(app.handle());
    }

    /// A mock app whose main window is at `at`, home being `tauri://localhost/`, the dev server
    /// `dev`. The window is returned with it.
    fn app_at(
        at: &str,
        dev: Option<Url>,
    ) -> (
        tauri::App<tauri::test::MockRuntime>,
        tauri::WebviewWindow<tauri::test::MockRuntime>,
    ) {
        let app = tauri::test::mock_app();
        app.manage(Home {
            url: url("tauri://localhost/"),
            dev: dev.clone(),
        });
        let window = guard(
            WebviewWindowBuilder::new(app.handle(), "main", WebviewUrl::External(url(at))),
            dev,
        )
        .build()
        .unwrap();
        (app, window)
    }

    /// What `reload_with` asked: how many files were unsaved, and the way to go on with the reload,
    /// if it asked at all.
    type Asked = std::cell::RefCell<Option<(usize, Box<dyn FnOnce() + Send>)>>;

    fn reloaded(app: &tauri::App<tauri::test::MockRuntime>, unsaved: usize) -> Asked {
        let asked = Asked::default();
        reload_with(app.handle(), unsaved, |n, go| {
            *asked.borrow_mut() = Some((n, go))
        });
        asked
    }

    #[test]
    fn it_asks_before_reloading_only_a_window_on_the_app_with_files_unsaved() {
        assert!(asks_before_reloading(true, 1));
        assert!(asks_before_reloading(true, 7));
        assert!(!asks_before_reloading(true, 0));
        // A window that left the app holds nothing any more.
        assert!(!asks_before_reloading(false, 3));
        assert!(!asks_before_reloading(false, 0));
    }

    #[test]
    fn a_window_on_the_app_with_files_unsaved_is_asked_about_before_it_is_reloaded() {
        let (app, window) = app_at("tauri://localhost/#/editor", None);
        let asked = reloaded(&app, 2);
        let (n, go) = asked.into_inner().expect("it asked");
        assert_eq!(n, 2);
        // Nothing moved yet: the answer decides.
        assert_eq!(window.url().unwrap(), url("tauri://localhost/#/editor"));
        go();
        assert_eq!(window.url().unwrap(), url("tauri://localhost/"));
    }

    #[test]
    fn a_refused_reload_leaves_the_window_where_it_was() {
        let (app, window) = app_at("tauri://localhost/#/editor", None);
        let asked = reloaded(&app, 1);
        // « Annuler »: the way on is dropped unused.
        drop(asked.into_inner());
        assert_eq!(window.url().unwrap(), url("tauri://localhost/#/editor"));
    }

    #[test]
    fn a_window_on_the_dev_server_is_on_the_app_too() {
        let (app, window) = app_at("http://localhost:1420/#/editor", Some(dev()));
        let (n, go) = reloaded(&app, 3).into_inner().expect("it asked");
        assert_eq!(n, 3);
        go();
        assert_eq!(window.url().unwrap(), url("tauri://localhost/"));
    }

    #[test]
    fn a_window_that_left_the_app_is_reloaded_without_a_question_whatever_it_held() {
        let (app, window) = app_at("https://example.com/", None);
        assert!(reloaded(&app, 4).into_inner().is_none());
        assert_eq!(window.url().unwrap(), url("tauri://localhost/"));
        // The dev server's address is not the app's when the app does not run from it.
        let (app, window) = app_at("http://localhost:1420/", None);
        assert!(reloaded(&app, 4).into_inner().is_none());
        assert_eq!(window.url().unwrap(), url("tauri://localhost/"));
    }

    #[test]
    fn a_window_on_the_app_with_nothing_unsaved_is_reloaded_without_a_question() {
        let (app, window) = app_at("tauri://localhost/#/editor", None);
        assert!(reloaded(&app, 0).into_inner().is_none());
        assert_eq!(window.url().unwrap(), url("tauri://localhost/"));
    }

    #[test]
    fn without_a_core_nothing_is_unsaved() {
        let (app, window) = app_at("tauri://localhost/#/editor", None);
        assert_eq!(unsaved_files(app.handle()), 0);
        reload(app.handle());
        assert_eq!(window.url().unwrap(), url("tauri://localhost/"));
    }

    #[test]
    fn the_question_is_written_in_both_languages_with_the_count() {
        use crate::i18n::Lang::{En, Fr};
        assert_eq!(
            reload_dialog(Fr, 1),
            [
                "L’éditeur a 1 fichier non enregistré : recharger l’interface le perd.",
                "Recharger",
                "Annuler"
            ]
        );
        assert_eq!(
            reload_dialog(Fr, 3)[0],
            "L’éditeur a 3 fichiers non enregistrés : recharger l’interface les perd."
        );
        assert_eq!(
            reload_dialog(En, 1),
            [
                "The editor has 1 unsaved file: reloading the interface loses it.",
                "Reload",
                "Cancel"
            ]
        );
        assert_eq!(
            reload_dialog(En, 2)[0],
            "The editor has 2 unsaved files: reloading the interface loses them."
        );
    }
}
