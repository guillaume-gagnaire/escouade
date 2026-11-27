//! Tests of the agents' pages' shelter (`agentpage`, ESC-667): its labels refused by the window's
//! guard, the scheme serving a page to its own webview alone, its paths, its CSP in a header and
//! a `<meta>`, its prefetches gone, its navigation. What a real webview does with it is told by
//! `agentpage_probe` (Linux, macOS in the CI; Windows by hand) and `e2e/agentpage.spec.ts`.

use crate::agentpage::*;
use std::path::Path;
use std::sync::Arc;
use tauri::Url;

fn url(s: &str) -> Url {
    Url::parse(s).unwrap()
}

fn not_own(_: &Url) -> bool {
    false
}

/// A folder of its own, gone with it.
struct Tmp(std::path::PathBuf);

impl Tmp {
    fn new() -> Tmp {
        let dir = std::env::temp_dir().join(format!("agentpage-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A page's folder with `index.html`, `other.txt` and `img/x.png`; and a neighbour's.
fn folders() -> (Tmp, Shelter) {
    let tmp = Tmp::new();
    let one = tmp.path().join("p-12");
    std::fs::create_dir_all(one.join("img")).unwrap();
    std::fs::write(
        one.join("index.html"),
        "<!doctype html><title>t</title><link rel=\"preconnect\" href=\"https://x.example\"><p>page</p>",
    )
    .unwrap();
    std::fs::write(one.join("other.txt"), "own").unwrap();
    std::fs::write(one.join("img/x.png"), [137u8, 80, 78, 71]).unwrap();
    std::fs::write(tmp.path().join("secret.txt"), "out of the page").unwrap();
    write_neighbour(&tmp.path().join("p-13")).unwrap();
    let shelter = Shelter::default();
    shelter.put("p-12", one).unwrap();
    shelter.put("p-13", tmp.path().join("p-13")).unwrap();
    (tmp, shelter)
}

fn body(r: &tauri::http::Response<Vec<u8>>) -> String {
    String::from_utf8_lossy(r.body()).into_owned()
}

#[test]
fn a_pages_number_and_label() {
    for ok in ["p-1", "p-12", "p-999999999"] {
        assert!(is_page(ok), "{ok}");
    }
    for no in [
        "",
        "p-",
        "p-0",
        "p-012",
        "p-1a",
        "p-1234567890",
        "P-1",
        "p-1/..",
        "x-1",
        "p--1",
    ] {
        assert!(!is_page(no), "{no}");
    }
    assert_eq!(label("p-12"), "agentpage-p-12");
    assert_eq!(page_of("agentpage-p-12"), Some("p-12"));
    assert_eq!(page_of("agentpage-x"), None);
    assert_eq!(page_of("devpage-p-12"), None);
    assert_eq!(page_of("main"), None);
}

#[test]
fn no_command_answers_an_agents_page() {
    // The window's thread wrapper refuses every call of such a label (`windowthread::off_the_window`).
    assert!(refuses("agentpage-p-12"));
    assert!(refuses("agentpage-"));
    assert!(refuses(&label("p-1")));
    for label in ["main", "devpage-a1", "space-s1", ""] {
        assert!(!refuses(label), "{label}");
    }
    // And no capability names it: no plugin answers it either.
    let text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities/default.json"),
    )
    .unwrap();
    let cap: serde_json::Value = serde_json::from_str(&text).unwrap();
    let webviews: Vec<&str> = cap["webviews"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|w| w.as_str())
        .collect();
    assert_eq!(webviews, ["main"]);
    assert!(cap.get("windows").is_none() && cap.get("remote").is_none());
    for dir in ["capabilities"] {
        for entry in std::fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join(dir)).unwrap() {
            let text = std::fs::read_to_string(entry.unwrap().path()).unwrap();
            assert!(
                !text.contains("agentpage"),
                "a capability names an agent's page"
            );
        }
    }
}

#[test]
fn the_window_thread_guard_reads_the_agents_pages_label() {
    // `refuses` must be asked where every command passes: the guard of the app's handler.
    let src =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/windowthread.rs"))
            .unwrap();
    assert!(src.contains("crate::agentpage::refuses(&label)"));
}

#[test]
fn a_page_is_served_to_its_own_webview_alone() {
    let (_tmp, s) = folders();
    let ok = s.serve(
        "agentpage-p-12",
        "GET",
        "escouade-page://p-12.localhost/",
        false,
    );
    assert_eq!(ok.status(), 200);
    assert!(body(&ok).contains("<p>page</p>"));
    // The window, a launch's page, another page, a label that is no page's: nothing, and the same
    // nothing as a missing file (a 404 that says nothing).
    for label in [
        "main",
        "devpage-a1",
        "agentpage-p-13",
        "agentpage-",
        "agentpage-p-12x",
        "",
    ] {
        let r = s.serve(
            label,
            "GET",
            "escouade-page://p-12.localhost/index.html",
            false,
        );
        assert_eq!(r.status(), 404, "{label}");
        assert!(r.body().is_empty(), "{label}");
    }
    // p-12 asking for p-13's files, in each form.
    for uri in [
        "escouade-page://p-13.localhost/index.html",
        "escouade-page://localhost/p-13/index.html",
        "escouade-page://p-12.localhost/../p-13/index.html",
        "escouade-page://p-12.localhost/%2e%2e/p-13/index.html",
        "escouade-page://p-12.localhost/..%2fp-13/index.html",
        "escouade-page://p-12.localhost/..%5c..%5csecret.txt",
        "escouade-page://p-12.localhost/%2e%2e/secret.txt",
    ] {
        let r = s.serve("agentpage-p-12", "GET", uri, false);
        assert_eq!(r.status(), 404, "{uri}");
        assert!(!body(&r).contains("secret"), "{uri}");
    }
    // Its own neighbour reads its own page.
    let r = s.serve(
        "agentpage-p-13",
        "GET",
        "escouade-page://p-13.localhost/",
        false,
    );
    assert!(body(&r).contains("secret of p-13"));
    // A page not served (closed, never put).
    assert_eq!(
        s.serve(
            "agentpage-p-14",
            "GET",
            "escouade-page://p-14.localhost/",
            false
        )
        .status(),
        404
    );
    s.remove("p-12");
    assert_eq!(
        s.serve(
            "agentpage-p-12",
            "GET",
            "escouade-page://p-12.localhost/",
            false
        )
        .status(),
        404
    );
}

#[test]
fn its_files_by_a_clean_path_of_its_folder() {
    let (_tmp, s) = folders();
    let get = |uri: &str| s.serve("agentpage-p-12", "GET", uri, false);
    let r = get("escouade-page://p-12.localhost/other.txt?x=1#y");
    assert_eq!(r.status(), 200);
    assert_eq!(body(&r), "own");
    assert_eq!(r.headers()["Content-Type"], "text/plain; charset=utf-8");
    let r = get("escouade-page://p-12.localhost/img/x.png");
    assert_eq!(r.headers()["Content-Type"], "image/png");
    assert_eq!(r.headers()["X-Content-Type-Options"], "nosniff");
    assert_eq!(get("escouade-page://p-12.localhost/img/").status(), 404);
    assert_eq!(
        get("escouade-page://p-12.localhost/missing.js").status(),
        404
    );
    for uri in [
        // A port, a user, another scheme, another host, `.`, `//`, a drive.
        "escouade-page://p-12.localhost:8080/",
        "escouade-page://u@p-12.localhost/",
        "http://p-12.localhost/",
        "escouade-page://p-12/",
        "escouade-page://p-12.localhost/./other.txt",
        "escouade-page://p-12.localhost//other.txt",
        "escouade-page://p-12.localhost/C:/Windows/win.ini",
        "escouade-page://p-12.localhost/%00",
        "escouade-page://p-12.localhost/%zz",
    ] {
        assert_eq!(get(uri).status(), 404, "{uri}");
    }
    // HEAD says it, GET alone gives it, nothing else.
    let head = s.serve(
        "agentpage-p-12",
        "HEAD",
        "escouade-page://p-12.localhost/other.txt",
        false,
    );
    assert_eq!(head.status(), 200);
    assert!(head.body().is_empty());
    for method in ["POST", "PUT", "DELETE", "OPTIONS"] {
        let r = s.serve(
            "agentpage-p-12",
            method,
            "escouade-page://p-12.localhost/other.txt",
            false,
        );
        assert_eq!(r.status(), 404, "{method}");
    }
}

#[cfg(unix)]
#[test]
fn a_link_in_its_folder_is_never_followed() {
    let (tmp, s) = folders();
    let one = tmp.path().join("p-12");
    std::os::unix::fs::symlink(tmp.path().join("secret.txt"), one.join("leak.txt")).unwrap();
    std::os::unix::fs::symlink(tmp.path(), one.join("up")).unwrap();
    for uri in [
        "escouade-page://p-12.localhost/leak.txt",
        "escouade-page://p-12.localhost/up/secret.txt",
    ] {
        assert_eq!(
            s.serve("agentpage-p-12", "GET", uri, false).status(),
            404,
            "{uri}"
        );
    }
}

#[test]
fn without_the_core_no_channel_answers() {
    let (_tmp, s) = folders();
    assert_eq!(
        locate("escouade-page://p-12.localhost/_/choose"),
        Some(("p-12".into(), Route::Channel("choose".into())))
    );
    for (method, uri) in [
        ("POST", "escouade-page://p-12.localhost/_/choose"),
        ("GET", "escouade-page://p-12.localhost/_/choose"),
        ("GET", "escouade-page://p-12.localhost/_"),
    ] {
        assert_eq!(
            s.serve_body(
                "agentpage-p-12",
                method,
                uri,
                br#"{"options":["a"]}"#,
                false
            )
            .status(),
            404,
            "{uri}"
        );
    }
}

#[test]
fn the_body_of_choose_is_closed_and_bounded() {
    assert_eq!(
        parse_choose(br#"{"options":["a"],"note":"garde la courbe"}"#),
        Some(Proposal {
            options: vec!["a".into()],
            note: Some("garde la courbe".into())
        })
    );
    assert!(parse_choose(br#"{"options":["a","b"]}"#).is_some());
    let long_note = format!(
        r#"{{"options":["a"],"note":"{}"}}"#,
        "é".repeat(NOTE_MAX + 1)
    );
    let big = format!(r#"{{"options":["a"],"note":"{}"}}"#, "x".repeat(CHOOSE_MAX));
    for wrong in [
        &b""[..],
        b"null",
        b"[]",
        br#"{"options":[]}"#,
        br#"{"options":"a"}"#,
        br#"{"options":[""]}"#,
        br#"{"options":["a"],"ask":"a-2"}"#,
        br#"{"options":["a"],"approve":true}"#,
        br#"{"options":["abcdefghijklmnopq"]}"#,
        br#"{"options":["a","b","c","d","e","f","g"]}"#,
        long_note.as_bytes(),
        big.as_bytes(),
    ] {
        assert_eq!(
            parse_choose(wrong),
            None,
            "{}",
            String::from_utf8_lossy(wrong)
        );
    }
}

#[test]
fn choose_only_proposes_from_its_own_page_once_a_second() {
    let (_tmp, s) = folders();
    let seen: Arc<parking_lot::Mutex<Vec<(String, Proposal)>>> = Arc::default();
    let kept = seen.clone();
    s.set_propose(Arc::new(move |page, p| {
        let ok = p.options != ["z"];
        kept.lock().push((page.to_string(), p));
        ok
    }));
    let uri = "escouade-page://p-12.localhost/_/choose";
    let body = br#"{"options":["a"]}"#;
    // Another page's webview, the window, a GET, another channel: a 404, the core never told.
    assert_eq!(
        s.serve_body("agentpage-p-13", "POST", uri, body, false)
            .status(),
        404
    );
    assert_eq!(s.serve_body("main", "POST", uri, body, false).status(), 404);
    assert_eq!(
        s.serve_body("agentpage-p-12", "GET", uri, body, false)
            .status(),
        404
    );
    assert_eq!(
        s.serve_body(
            "agentpage-p-12",
            "POST",
            "escouade-page://p-12.localhost/_/answer",
            body,
            false
        )
        .status(),
        404
    );
    assert_eq!(
        s.serve_body(
            "agentpage-p-12",
            "POST",
            uri,
            br#"{"options":["a"],"x":1}"#,
            false
        )
        .status(),
        404
    );
    assert!(seen.lock().is_empty());
    // Its own: handed to the core, a 204 that says nothing more.
    let ok = s.serve_body("agentpage-p-12", "POST", uri, body, false);
    assert_eq!(ok.status(), 204);
    assert!(ok.body().is_empty());
    assert_eq!(seen.lock().len(), 1);
    assert_eq!(seen.lock()[0].0, "p-12");
    // A second within the second: refused before the core.
    assert_eq!(
        s.serve_body("agentpage-p-12", "POST", uri, body, false)
            .status(),
        404
    );
    assert_eq!(seen.lock().len(), 1);
    // The neighbour has its own second; what the core refuses is a 404.
    assert_eq!(
        s.serve_body(
            "agentpage-p-13",
            "POST",
            "escouade-page://p-13.localhost/_/choose",
            br#"{"options":["z"]}"#,
            false
        )
        .status(),
        404
    );
    assert_eq!(seen.lock().len(), 2);
}

#[test]
fn the_csp_has_no_network_and_no_eval() {
    for windows in [false, true] {
        let policy = csp("p-12", windows);
        let me = origin("p-12", windows);
        assert!(policy.starts_with("default-src 'none';"));
        assert!(!policy.contains("unsafe-eval") && !policy.contains("wasm-unsafe-eval"));
        // Its own origin, written out: `'self'` would let a WebSocket out on Windows.
        assert!(!policy.contains("'self'"));
        assert!(!policy.contains('*'));
        // No scheme as a source: the web, whole.
        for token in policy.split([' ', ';']) {
            assert!(
                !["http:", "https:", "ws:", "wss:"].contains(&token),
                "{token} in {policy}"
            );
        }
        for directive in [
            format!("connect-src {me};"),
            format!("img-src {me} data: blob:;"),
            format!("media-src {me} blob:;"),
            format!("script-src {me} 'unsafe-inline';"),
            "frame-src 'none';".into(),
            "child-src 'none';".into(),
            "worker-src 'none';".into(),
            "object-src 'none';".into(),
            "form-action 'none';".into(),
            "base-uri 'none';".into(),
        ] {
            assert!(policy.contains(&directive), "{directive} in {policy}");
        }
    }
    assert_eq!(origin("p-12", false), "escouade-page://p-12.localhost");
    assert_eq!(origin("p-12", true), "http://escouade-page.p-12.localhost");
    assert_eq!(
        crate::agentpage::url("p-12").as_str(),
        "escouade-page://p-12.localhost/"
    );
}

#[test]
fn every_answer_carries_the_csp_in_its_header_and_its_document_in_a_meta() {
    let (_tmp, s) = folders();
    for windows in [false, true] {
        let r = s.serve(
            "agentpage-p-12",
            "GET",
            "escouade-page://p-12.localhost/",
            windows,
        );
        let h = r.headers();
        assert_eq!(h["Content-Security-Policy"], csp("p-12", windows).as_str());
        assert_eq!(h["X-DNS-Prefetch-Control"], "off");
        assert_eq!(h["Cache-Control"], "no-store");
        assert_eq!(h["Referrer-Policy"], "no-referrer");
        assert!(h["Permissions-Policy"]
            .to_str()
            .unwrap()
            .contains("camera=()"));
        let html = body(&r);
        // The doctype first, then the CSP before anything of the page.
        let meta = format!(
            "<!doctype html><meta http-equiv=\"Content-Security-Policy\" content=\"{}\">",
            csp("p-12", windows)
        );
        assert!(html.starts_with(&meta), "{html}");
        assert!(html.contains("x-dns-prefetch-control"));
        // The page's preconnection is gone.
        assert!(!html.contains("preconnect"), "{html}");
        let file = s.serve(
            "agentpage-p-12",
            "GET",
            "escouade-page://p-12.localhost/other.txt",
            windows,
        );
        assert_eq!(
            file.headers()["Content-Security-Policy"],
            csp("p-12", windows).as_str()
        );
        assert_eq!(body(&file), "own");
    }
    // The probe's alone: one of the two taken away.
    let tmp = Tmp::new();
    std::fs::write(tmp.path().join("index.html"), "<p>x</p>").unwrap();
    s.put_hardened(
        "p-2",
        tmp.path().to_path_buf(),
        Harden {
            header: false,
            meta: true,
        },
    )
    .unwrap();
    let r = s.serve(
        "agentpage-p-2",
        "GET",
        "escouade-page://p-2.localhost/",
        false,
    );
    assert!(r.headers().get("Content-Security-Policy").is_none());
    assert!(body(&r).starts_with("<meta http-equiv=\"Content-Security-Policy\""));
}

#[test]
fn the_meta_goes_after_the_doctype_whatever_comes_before_it() {
    let tag = "<meta http-equiv=\"Content-Security-Policy\"";
    for (html, before) in [
        ("<!DOCTYPE html>\n<p>x", "<!DOCTYPE html>"),
        ("\u{feff}  <!doctype html><p>x", "\u{feff}  <!doctype html>"),
        ("<p>x", ""),
        ("<html><head></head>", ""),
    ] {
        let out = harden_html(html, "p-1", false, true);
        assert!(out.starts_with(&format!("{before}{tag}")), "{out}");
    }
    assert_eq!(harden_html("<p>x", "p-1", false, false), "<p>x");
}

#[test]
fn prefetches_are_taken_out_of_what_is_served() {
    let html = "<head><LINK REL=\"dns-prefetch\" href=\"//x.example\">\
        <link rel=preconnect href=https://x.example><link href=\"/a\" rel=\"prefetch\"/>\
        <link rel=\"prerender\" href=\"https://x.example/p\">\
        <link rel=\"stylesheet\" href=\"a.css\"><linkx rel=prefetch>\
        <script type=\"speculationrules\">{\"prefetch\":[{\"urls\":[\"https://x.example\"]}]}</script>\
        <script>var keep = 'prefetch';</script></head>";
    let out = strip_prefetches(html);
    assert!(!out.to_ascii_lowercase().contains("dns-prefetch"));
    assert!(!out.contains("preconnect") && !out.contains("prerender"));
    assert!(!out.contains("speculationrules") && !out.contains("x.example"));
    assert!(out.contains("<link rel=\"stylesheet\" href=\"a.css\">"));
    // What is not a `<link>` stays as it was.
    assert!(out.contains("<linkx rel=prefetch>"));
    assert!(out.contains("<script>var keep = 'prefetch';</script>"));
    // An unclosed tag at the end is left alone, not lost.
    assert_eq!(strip_prefetches("<p>a <b"), "<p>a <b");
}

#[test]
fn the_guard_takes_away_webrtc_code_from_text_and_the_prefetches_a_script_adds() {
    let s = init_script();
    for name in [
        "RTCPeerConnection",
        "webkitRTCPeerConnection",
        "RTCDataChannel",
    ] {
        assert!(s.contains(&format!("'{name}'")), "{name}");
    }
    assert!(s.contains("configurable: false"));
    // `eval` and every kind of `Function`, refused (WebView2 lets `eval` through the CSP).
    assert!(s.contains("lock(W, 'eval'"));
    assert!(s.contains("lock(W, 'Function'"));
    assert!(s.contains("async function* () {}"));
    // The prefetches, by each way a page has to add one.
    for rel in [
        "dns-prefetch",
        "preconnect",
        "prefetch",
        "prerender",
        "speculationrules",
    ] {
        assert!(s.contains(rel), "{rel}");
    }
    for sink in [
        "'appendChild'",
        "'insertAdjacentElement'",
        "'innerHTML'",
        "'outerHTML'",
        "'insertAdjacentHTML'",
        "'write'",
        "'createContextualFragment'",
        "'setAttribute'",
        "'setAttributeNS'",
        "'rel'",
        "'type'",
    ] {
        assert!(s.contains(sink), "{sink}");
    }
    // Nothing of Escouade in it, and no code made from text of its own.
    assert!(!s.contains("__TAURI"));
    assert!(!s.contains("new Function("));
}

#[test]
fn webview2_answers_every_request_but_the_pages_own() {
    for uri in [
        "http://escouade-page.p-12.localhost/",
        "http://escouade-page.p-12.localhost/img/x.png?v=1",
        "data:image/png;base64,AAAA",
        "blob:http://escouade-page.p-12.localhost/1f2e",
        "about:blank",
    ] {
        assert!(own_request(uri, "p-12"), "{uri}");
    }
    for uri in [
        "http://escouade-page.p-13.localhost/",
        "http://escouade-page.p-12.localhost.evil.example/",
        "http://escouade-page.p-12.localhost:8080/",
        "https://escouade-page.p-12.localhost/",
        "http://127.0.0.1:4130/fetch",
        "http://ipc.localhost/open_url",
        "http://tauri.localhost/index.html",
        "ws://escouade-page.p-12.localhost/",
        "escouade-page://p-12.localhost/",
    ] {
        assert!(!own_request(uri, "p-12"), "{uri}");
    }
}

#[test]
fn webview2s_browser_of_the_pages_goes_to_the_network_through_the_dead_end_alone() {
    let args = browser_args(4130);
    // wry's own switches, kept (they are replaced when others are given).
    assert!(args.contains("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection"));
    assert!(args.contains("--proxy-server=http://127.0.0.1:4130"));
    // The loopback too, which Chromium otherwise lets by.
    assert!(args.contains("--proxy-bypass-list=<-loopback>"));
    assert!(args.contains("--force-webrtc-ip-handling-policy=disable_non_proxied_udp"));
    assert!(args.contains("--dns-prefetch-disable"));
    assert!(!args.contains("remote-debugging"));
}

#[cfg(not(target_os = "macos"))]
#[test]
fn the_dead_end_drops_every_connection_and_counts_it() {
    use std::io::Read;
    let port = dead_end().expect("a dead end");
    assert_eq!(dead_end(), Some(port), "one for the whole run");
    let before = cut();
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    let _ = std::io::Write::write_all(&mut s, b"GET http://example.com/ HTTP/1.1\r\n\r\n");
    let mut answer = Vec::new();
    let _ = s.read_to_end(&mut answer);
    assert!(answer.is_empty(), "the dead end answers nothing");
    let end = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while cut() == before && std::time::Instant::now() < end {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(cut() > before);
}

#[test]
fn a_page_stays_on_its_own_address() {
    for (to, windows) in [
        ("escouade-page://p-12.localhost/", false),
        ("escouade-page://p-12.localhost/b.html?x#y", false),
        ("escouade-page://p-12.localhost/", true),
        ("http://escouade-page.p-12.localhost/b.html", true),
    ] {
        assert_eq!(
            judge(&url(to), "p-12", windows, not_own),
            Verdict::Here,
            "{to}"
        );
    }
    for (to, windows) in [
        // Another page, under each form; the page's `http` form where it is the loopback's.
        ("escouade-page://p-13.localhost/", false),
        ("escouade-page://localhost/p-12/", false),
        ("escouade-page://p-12.localhost:81/", false),
        ("http://escouade-page.p-13.localhost/", true),
        ("http://escouade-page.p-12.localhost/", false),
        ("http://escouade-page.p-12.localhost:8080/", true),
        // A file, a script, Escouade's own schemes, a blank page.
        ("file:///etc/passwd", false),
        ("javascript:alert(1)", false),
        ("tauri://localhost/", false),
        ("about:blank", false),
        ("data:text/html,x", false),
    ] {
        assert_eq!(
            judge(&url(to), "p-12", windows, not_own),
            Verdict::Nowhere,
            "{to}"
        );
    }
    // The web: refused in the page, kept for the window to offer.
    for to in [
        "https://example.com/?secret=1",
        "http://127.0.0.1:9/x",
        "mailto:a@b.c",
    ] {
        assert_eq!(
            judge(&url(to), "p-12", false, not_own),
            Verdict::Outside,
            "{to}"
        );
    }
    // Escouade's own web address (its dev server): nowhere, never offered.
    assert_eq!(
        judge(&url("http://localhost:1420/"), "p-12", false, |u| u.port()
            == Some(1420)),
        Verdict::Nowhere
    );
}

#[test]
fn the_web_address_it_asked_for_is_taken_once() {
    let s = Shelter::default();
    assert_eq!(take_wanted(&s, "p-12"), None);
}

#[test]
fn the_trap_has_its_listeners_ports() {
    let t = trap(4130, 4131);
    assert!(!t.contains("__PORT__") && !t.contains("__UDP__"));
    assert!(t.contains("http://127.0.0.1:4130/"));
    assert!(t.contains("stun:127.0.0.1:4131"));
    // It says what it got in its title, the one thing it may give.
    assert!(t.contains("document.title = 'escouade-probe:'"));
}

#[test]
fn an_agents_page_has_no_reading_buffers_of_the_browsers() {
    // The read bridge of the browsers (`browserread`, ESC-668) keeps a launch's page's console and
    // network from its creation: an agent's page has none until AP-2 decides it.
    let src =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/agentpage.rs"))
            .unwrap();
    assert!(!src.contains("browserread"));
}
