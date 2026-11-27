//! The read bridge of the browsers (ESC-668): its closed list of the DevTools protocol, its
//! buffers, its masking, its addresses, its selectors and its captures' plans. What it does in a
//! real webview is `agentbrowser_probe`'s (examples).

use crate::browserread::*;
use serde_json::json;

fn console(text: &str, level: Level) -> ConsoleEntry {
    ConsoleEntry {
        seq: 0,
        t: 0,
        level,
        text: text.into(),
        source: "console".into(),
        url: String::new(),
        line: None,
        from: Teller::Browser,
    }
}

fn failure(url: &str) -> NetEntry {
    NetEntry {
        seq: 0,
        t: 0,
        method: "GET".into(),
        url: url.into(),
        status: Some(404),
        error: None,
        kind: "fetch".into(),
        from: Teller::Browser,
    }
}

// ---------- the closed list ----------

#[test]
fn the_closed_list_takes_its_methods_and_refuses_every_other() {
    for m in Method::ALL {
        assert_eq!(Method::parse(m.name()), Ok(m));
    }
    // What the probe of ESC-663 read the cookies and the storage by, and their kin.
    for name in [
        "Network.getCookies",
        "Network.getAllCookies",
        "Storage.getCookies",
        "Network.getResponseBody",
        "Network.setExtraHTTPHeaders",
        "DOMStorage.getDOMStorageItems",
        "IndexedDB.requestData",
        "Runtime.callFunctionOn",
        "Runtime.compileScript",
        "Page.addScriptToEvaluateOnNewDocument",
        "Page.setDownloadBehavior",
        "Browser.close",
        "Target.attachToTarget",
        "Fetch.enable",
        "DOM.setFileInputFiles",
        "runtime.evaluate",
        "",
    ] {
        assert!(Method::parse(name).is_err(), "{name} must be refused");
    }
}

#[test]
fn evaluate_runs_escouade_s_library_in_escouade_s_world_alone() {
    let ours = expression(&Script::Read {
        selector: Some("#tarifs"),
        what: What::Html,
    });
    let call = |expr: &str, ctx: i64| json!({ "expression": expr, "contextId": ctx, "returnByValue": true, "silent": true });
    assert!(guard(Method::RuntimeEvaluate, &call(&ours, 7), Some(7)).is_ok());
    // Another world (the page's), none made yet, no context at all.
    assert!(guard(Method::RuntimeEvaluate, &call(&ours, 1), Some(7)).is_err());
    assert!(guard(Method::RuntimeEvaluate, &call(&ours, 7), None).is_err());
    assert!(guard(
        Method::RuntimeEvaluate,
        &json!({ "expression": ours }),
        Some(7)
    )
    .is_err());
    // A script that is not the library: what an agent would write.
    assert!(guard(
        Method::RuntimeEvaluate,
        &call("localStorage.getItem('token')", 7),
        Some(7)
    )
    .is_err());
    assert!(guard(
        Method::RuntimeEvaluate,
        &call(&format!("1;{ours}"), 7),
        Some(7)
    )
    .is_err());
    // The console's API of the page's world, and anything else the method may take.
    let mut more = call(&ours, 7);
    more["includeCommandLineAPI"] = json!(true);
    assert!(guard(Method::RuntimeEvaluate, &more, Some(7)).is_err());
    assert!(guard(Method::RuntimeEvaluate, &json!("text"), Some(7)).is_err());
}

#[test]
fn a_navigation_goes_to_a_web_address_alone() {
    assert!(guard(
        Method::PageNavigate,
        &json!({ "url": "http://localhost:5173/a" }),
        None
    )
    .is_ok());
    for url in [
        "file:///C:/secret",
        "javascript:alert(1)",
        "data:text/html,x",
        "escouade://x",
        "nothing",
    ] {
        assert!(
            guard(Method::PageNavigate, &json!({ "url": url }), None).is_err(),
            "{url}"
        );
    }
    assert!(guard(Method::PageNavigate, &json!({}), None).is_err());
}

#[test]
fn the_arguments_of_a_script_are_values_never_code() {
    let e = expression(&Script::Read {
        selector: Some("a'); fetch('//evil'); ('"),
        what: What::Text,
    });
    // The selector stays a string of the JSON the library is called with.
    assert!(
        e.ends_with(r#"({"run":"read","selector":"a'); fetch('//evil'); ('","what":"text"})"#),
        "{e}"
    );
    let e = expression(&Script::Rect {
        selector: Some("\"}); alert(1); ({\""),
    });
    assert!(e.contains(r#""selector":"\"}); alert(1); ({\"""#), "{e}");
}

#[test]
fn the_drain_and_the_world_are_calls_without_arguments() {
    // The drain asks the relay inside its own call; the world's count is the probe's (ESC-692).
    assert!(expression(&Script::Drain).ends_with(r#"({"run":"drain"})"#));
    assert!(expression(&Script::World).ends_with(r#"({"run":"world"})"#));
}

// ---------- the selectors ----------

#[test]
fn a_selector_is_short_simple_and_closed() {
    for ok in [
        "#tarifs",
        "#tarifs .carte-pro",
        "main > section:nth-of-type(2) .card a",
        "[role=tab]:nth-of-type(2)",
        "a[href=\"x y > z\"], .b .c .d .e",
        "input:not([type=password])",
    ] {
        assert!(check_selector(ok).is_ok(), "{ok}");
    }
    for bad in [
        "",
        "  ",
        "main:HAS(.x)",
        "a b c d e",
        "a > b > c > d > e",
        "x, a b c d e",
        "[role=tab",
        "a[title=\"open",
    ] {
        assert!(check_selector(bad).is_err(), "{bad}");
    }
    assert!(check_selector(&"a".repeat(201)).is_err());
    assert!(check_selector(&"a".repeat(200)).is_ok());
}

// ---------- the buffers ----------

#[test]
fn a_buffer_keeps_its_last_500_entries_and_says_what_it_lost() {
    let b = Buffers::default();
    for i in 0..1200 {
        b.push_console(console(&format!("line {i}"), Level::Log));
    }
    let all = b.read(&Query {
        limit: 500,
        ..Default::default()
    });
    assert_eq!(all.entries.len(), CAP);
    assert_eq!(all.entries[0].text, "line 700");
    assert_eq!(all.entries[CAP - 1].text, "line 1199");
    assert_eq!(all.dropped, 700);
    assert_eq!(all.cursor, "1200.0");
    // From a cursor: what came after it, and what of that went.
    let after: Cursor = "1000.0".parse().unwrap();
    let next = b.read(&Query {
        since: Since::Cursor(after),
        ..Default::default()
    });
    assert_eq!(next.entries.len(), 200);
    assert_eq!(next.entries[0].text, "line 1000");
    assert_eq!(next.dropped, 0);
    let old = b.read(&Query {
        since: Since::Cursor("600.0".parse().unwrap()),
        limit: 500,
        ..Default::default()
    });
    assert_eq!(old.dropped, 100);
    // A limit leaves the cursor at the last entry given: the next read goes on from there.
    let first = b.read(&Query {
        limit: 10,
        ..Default::default()
    });
    assert_eq!(first.entries.len(), 10);
    assert_eq!(first.cursor, "710.0");
    let then = b.read(&Query {
        since: Since::Cursor(first.cursor.parse().unwrap()),
        limit: 1,
        ..Default::default()
    });
    assert_eq!(then.entries[0].text, "line 710");
    // 500 at most, 200 by default.
    assert_eq!(
        b.read(&Query {
            limit: 10_000,
            ..Default::default()
        })
        .entries
        .len(),
        500
    );
    assert_eq!(b.read(&Query::default()).entries.len(), 200);
}

#[test]
fn a_read_takes_the_levels_it_asks_and_the_network_apart() {
    let b = Buffers::default();
    b.push_console(console("a", Level::Log));
    b.push_console(console("b", Level::Error));
    b.push_network(failure("http://localhost:5173/api?token=abc"));
    b.push_console(console("c", Level::Warn));
    let errors = b.read(&Query {
        levels: vec![Level::Error, Level::Warn],
        ..Default::default()
    });
    assert_eq!(
        errors
            .entries
            .iter()
            .map(|e| e.text.as_str())
            .collect::<Vec<_>>(),
        ["b", "c"]
    );
    assert!(errors.network.is_empty());
    assert_eq!(errors.cursor, "3.1");
    let both = b.read(&Query {
        network: true,
        ..Default::default()
    });
    assert_eq!(both.network.len(), 1);
    assert_eq!(both.network[0].url, "http://localhost:5173/api");
    // The network's buffer is bounded too.
    for i in 0..700 {
        b.push_network(failure(&format!("http://localhost/{i}")));
    }
    let net = b.read(&Query {
        network: true,
        limit: 500,
        ..Default::default()
    });
    assert_eq!(net.network.len(), CAP);
    assert_eq!(net.network[0].url, "http://localhost/200");
    assert_eq!(net.dropped, 201);
}

#[test]
fn a_read_from_a_time_takes_what_came_since() {
    let b = Buffers::default();
    b.push_console(console("before", Level::Log));
    std::thread::sleep(std::time::Duration::from_millis(20));
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    std::thread::sleep(std::time::Duration::from_millis(5));
    b.push_console(console("after", Level::Log));
    let r = b.read(&Query {
        since: Since::Time(at),
        ..Default::default()
    });
    assert_eq!(r.entries.len(), 1);
    assert_eq!(r.entries[0].text, "after");
    assert_eq!(r.cursor, "2.0");
}

#[test]
fn an_entry_is_cut_its_address_stripped_and_a_closed_buffer_takes_nothing() {
    let b = Buffers::default();
    let mut e = console(&"é".repeat(2000), Level::Info);
    e.url = "https://ada:pw@app.test/main.js?v=1&key=secret#L3".into();
    b.push_console(e);
    let got = b.read(&Query::default());
    assert_eq!(got.entries[0].text.chars().count(), TEXT_MAX);
    assert_eq!(got.entries[0].url, "https://app.test/main.js");
    b.close();
    b.push_console(console("late", Level::Log));
    b.push_network(failure("http://x/"));
    let after = b.read(&Query {
        network: true,
        ..Default::default()
    });
    assert_eq!(after.entries.len(), 1);
    assert!(after.network.is_empty());
}

#[test]
fn the_protocol_s_events_become_entries_told_by_the_browser() {
    let b = Buffers::default();
    b.take_cdp(
        "Runtime.consoleAPICalled",
        &json!({ "type": "error", "args": [{ "type": "string", "value": "probe-error-663" }, { "type": "number", "value": 3 },
                 { "type": "object", "description": "Object" }],
                 "stackTrace": { "callFrames": [{ "url": "http://127.0.0.1:5/?k=v", "lineNumber": 41 }] } }),
    );
    b.take_cdp(
        "Runtime.exceptionThrown",
        &json!({ "exceptionDetails": { "text": "Uncaught", "exception": { "description": "TypeError: x is undefined" },
                 "url": "http://127.0.0.1:5/app.js", "lineNumber": 9 } }),
    );
    b.take_cdp(
        "Log.entryAdded",
        &json!({ "entry": { "level": "warning", "source": "network", "text": "Failed to load resource", "url": "http://127.0.0.1:5/missing?x=1" } }),
    );
    b.take_cdp(
        "Network.requestWillBeSent",
        &json!({ "requestId": "1", "request": { "method": "POST", "url": "http://127.0.0.1:5/api?token=t" } }),
    );
    b.take_cdp(
        "Network.responseReceived",
        &json!({ "requestId": "1", "type": "Fetch", "response": { "status": 500, "url": "http://127.0.0.1:5/api?token=t" } }),
    );
    b.take_cdp(
        "Network.responseReceived",
        &json!({ "requestId": "2", "type": "Document", "response": { "status": 200, "url": "http://127.0.0.1:5/" } }),
    );
    b.take_cdp(
        "Network.requestWillBeSent",
        &json!({ "requestId": "3", "request": { "method": "GET", "url": "http://127.0.0.1:9/refused-663?s=1" } }),
    );
    b.take_cdp(
        "Network.loadingFailed",
        &json!({ "requestId": "3", "type": "Fetch", "errorText": "net::ERR_UNSAFE_PORT" }),
    );
    b.take_cdp("Network.loadingFinished", &json!({ "requestId": "1" }));
    let r = b.read(&Query {
        network: true,
        ..Default::default()
    });
    let e = &r.entries;
    assert_eq!(e.len(), 3);
    assert_eq!(
        (e[0].level, e[0].text.as_str()),
        (Level::Error, "probe-error-663 3 Object")
    );
    assert_eq!(
        (e[0].url.as_str(), e[0].line),
        ("http://127.0.0.1:5/", Some(42))
    );
    assert_eq!(
        (e[1].source.as_str(), e[1].text.as_str()),
        ("exception", "TypeError: x is undefined")
    );
    assert_eq!((e[2].level, e[2].source.as_str()), (Level::Warn, "network"));
    assert_eq!(e[2].url, "http://127.0.0.1:5/missing");
    assert!(e.iter().all(|x| x.from == Teller::Browser));
    let n = &r.network;
    assert_eq!(n.len(), 2);
    assert_eq!(
        (
            n[0].method.as_str(),
            n[0].status,
            n[0].url.as_str(),
            n[0].kind.as_str()
        ),
        ("POST", Some(500), "http://127.0.0.1:5/api", "fetch")
    );
    assert_eq!(
        (n[1].error.as_deref(), n[1].url.as_str()),
        (
            Some("net::ERR_UNSAFE_PORT"),
            "http://127.0.0.1:9/refused-663"
        )
    );
    // What it serializes to, for `browser_console` (AB-4).
    let text = serde_json::to_string(&r).unwrap();
    assert!(
        text.contains(r#""from":"browser""#) && text.contains(r#""type":"fetch""#),
        "{text}"
    );
    assert!(!text.contains("token=t") && !text.contains("seq"), "{text}");
}

#[test]
fn what_the_relay_of_webkit_told_is_marked_as_the_page_s() {
    let b = Buffers::default();
    let told = |v: serde_json::Value| v.to_string();
    b.take_relay(&json!({ "entries": [
        told(json!({ "kind": "console", "level": "warn", "text": "probe-warn", "t": 1 })),
        told(json!({ "kind": "console", "level": "error", "source": "exception", "text": "boom", "url": "http://x/a.js?k=1", "line": 3 })),
        told(json!({ "kind": "network", "method": "post", "url": "http://x/api?token=1", "status": 404, "type": "fetch" })),
        told(json!({ "kind": "network", "method": "GET", "url": "data:text/plain,secret", "error": "failed", "type": "img" })),
        told(json!({ "kind": "cookie", "text": "nothing" })),
        "not json".to_string(),
    ], "dropped": 0 }));
    b.take_relay(&json!({ "nothing": true }));
    let r = b.read(&Query {
        network: true,
        ..Default::default()
    });
    assert_eq!(r.entries.len(), 2);
    assert!(r.entries.iter().all(|e| e.from == Teller::Page));
    assert_eq!(
        (
            r.entries[1].source.as_str(),
            r.entries[1].url.as_str(),
            r.entries[1].line
        ),
        ("exception", "http://x/a.js", Some(3))
    );
    assert_eq!(r.network.len(), 2);
    assert_eq!(
        (r.network[0].method.as_str(), r.network[0].url.as_str()),
        ("POST", "http://x/api")
    );
    assert_eq!(r.network[1].url, "data:");
    assert!(r.network.iter().all(|e| e.from == Teller::Page));
}

// ---------- addresses ----------

#[test]
fn an_address_loses_its_query_its_fragment_and_its_credentials() {
    assert_eq!(
        strip_url("https://app.test/a/b?token=1#x"),
        "https://app.test/a/b"
    );
    assert_eq!(
        strip_url("http://user:pw@localhost:5173/"),
        "http://localhost:5173/"
    );
    assert_eq!(
        strip_url("wss://app.test/socket?auth=1"),
        "wss://app.test/socket"
    );
    assert_eq!(strip_url("data:text/html;base64,PHNjcmlwdD4="), "data:");
    assert_eq!(strip_url("blob:https://app.test/1234"), "blob:");
    assert_eq!(strip_url("/api/users?id=3"), "/api/users");
    assert_eq!(strip_url("  "), "");
    assert_eq!(
        strip_url(&format!("https://app.test/{}", "a".repeat(900)))
            .chars()
            .count(),
        TEXT_MAX
    );
}

// ---------- masking ----------

#[test]
fn the_html_masks_passwords_hidden_fields_token_metas_and_inline_scripts() {
    let page = concat!(
        r#"<html><head><meta name="csrf-token" content="abc123"><meta name="description" content="Tarifs">"#,
        r#"<meta property="og:nonce" content='n0nce'><script src="/app.js"></script>"#,
        r#"<script>window.__CONFIG__ = {"apiKey":"k"}</script><SCRIPT type="module">import x</SCRIPT></head>"#,
        r#"<body><input id="pw" type="password" value="s3cret-663"><input type="PASSWORD" value=bare>"#,
        r#"<input type="hidden" name="_csrf" value="tok"><input name="q" value="visible"><input type="hidden">"#,
        r#"<p title="a > b">x</p></body></html>"#
    );
    let (out, n) = mask_html(page);
    for gone in [
        "abc123",
        "n0nce",
        "__CONFIG__",
        "import x",
        "s3cret-663",
        "bare",
        "\"tok\"",
    ] {
        assert!(!out.contains(gone), "{gone} left in {out}");
    }
    for kept in [
        r#"content="Tarifs""#,
        r#"<script src="/app.js"></script>"#,
        r#"value="visible""#,
        r#"<input type="hidden">"#,
        r#"title="a > b""#,
        r#"<input id="pw" type="password" value="«masqué»">"#,
        r#"<input type="PASSWORD" value="«masqué»">"#,
        r#"<SCRIPT type="module">«masqué»</SCRIPT>"#,
    ] {
        assert!(out.contains(kept), "{kept} missing in {out}");
    }
    assert_eq!(n, 7, "{out}");
    // Masked again, nothing more: what the library already masked is not counted twice.
    let (again, m) = mask_html(&out);
    assert_eq!((again.as_str(), m), (out.as_str(), 0));
}

#[test]
fn the_tokens_that_show_are_masked_wherever_they_are() {
    let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
    let page = format!(
        "<p>Ta clé : ghp_{a}. Jeton : {jwt}</p><code data-k=\"sk-ant-api03-{a}\">Authorization: Bearer {b}</code>\
         <pre>-----BEGIN RSA PRIVATE KEY-----\nMIIEow\n-----END RSA PRIVATE KEY-----</pre><p>AKIAABCDEFGHIJKLMNOP et une phrase-normale.</p>",
        a = "A".repeat(36),
        b = "z".repeat(32)
    );
    let (out, n) = mask_html(&page);
    assert_eq!(n, 6, "{out}");
    for gone in ["ghp_", "eyJ", "sk-ant", "zzzzzzzz", "MIIEow", "AKIA"] {
        assert!(!out.contains(gone), "{gone} left in {out}");
    }
    assert!(
        out.contains("Ta clé : «masqué». Jeton : «masqué»</p>"),
        "{out}"
    );
    assert!(
        out.contains("Bearer «masqué»") && out.contains("phrase-normale"),
        "{out}"
    );
    // Ordinary words, ids and short values stay.
    let (plain, none) = mask_tokens("Bearer court, eyJ.a.b, sk-court, section-2 et ghp_trop-court");
    assert_eq!(none, 0, "{plain}");
}

#[test]
fn a_read_is_cut_at_a_tag_s_border() {
    let html = "<div><p>un texte</p><p class=\"long\">deux</p></div>";
    let (cut, truncated, total) = clip(html, 25, What::Html);
    assert!(truncated);
    assert_eq!(total, html.chars().count());
    assert_eq!(cut, "<div><p>un texte</p>");
    // A text cut, or a tag: after the last whole tag.
    let (cut, ..) = clip(html, 33, What::Html);
    assert_eq!(cut, "<div><p>un texte</p>");
    let (cut, ..) = clip(html, 40, What::Html);
    assert_eq!(cut, "<div><p>un texte</p><p class=\"long\">");
    let (cut, ..) = clip("<p>a &amp; b</p>", 7, What::Html);
    assert_eq!(cut, "<p>");
    let (cut, ..) = clip("a &amp; b", 4, What::Html);
    assert_eq!(cut, "a ");
    let (cut, truncated, _) = clip("é".repeat(10).as_str(), 4, What::Text);
    assert_eq!((cut.as_str(), truncated), ("éééé", true));
    assert_eq!(
        clip("court", 50, What::Html),
        ("court".to_string(), false, 5)
    );
}

#[test]
fn a_read_puts_its_parts_together_masked_and_cut() {
    let answer = json!({
        "url": "http://127.0.0.1:5/", "title": "Sonde\u{200b} ESC-668", "matched": 2, "masked": 1,
        "parts": ["<form><input type=\"password\" value=\"«masqué»\"></form>", "<p>ghp_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA</p>"],
    });
    let r = assemble(&answer, Some("form, p"), What::Html, 50_000).unwrap();
    assert_eq!(r.matched, 2);
    assert_eq!(r.title, "Sonde ESC-668");
    assert_eq!(r.masked, 2);
    assert!(r.content.contains("<p>«masqué»</p>") && !r.truncated);
    assert_eq!(r.chars, r.content.chars().count());
    let cut = assemble(&answer, None, What::Html, 30).unwrap();
    assert!(cut.truncated && cut.chars <= 30 && cut.total_chars > 30);
    let bad = assemble(&json!({ "error": "selector" }), Some("a["), What::Html, 10);
    assert!(bad.unwrap_err().contains("a["));
}

// ---------- captures ----------

fn view() -> View {
    View {
        inner_width: 900.0,
        inner_height: 700.0,
        scroll_x: 0.0,
        scroll_y: 400.0,
        dpr: 1.0,
        doc_width: 900.0,
        doc_height: 2138.0,
    }
}

#[test]
fn a_capture_takes_what_of_an_element_shows_or_the_view_or_the_document() {
    let el = Rect {
        x: 20.0,
        y: 650.0,
        width: 100.0,
        height: 120.0,
    };
    let p = plan(view(), Some(el), false).unwrap();
    assert_eq!(
        p.viewport,
        Rect {
            x: 20.0,
            y: 650.0,
            width: 100.0,
            height: 50.0
        }
    );
    // The protocol of WebView2 clips in the document's pixels.
    assert_eq!(
        p.document,
        Rect {
            x: 20.0,
            y: 1050.0,
            width: 100.0,
            height: 50.0
        }
    );
    assert_eq!(p.scale, 1.0);
    let outside = Rect { y: 800.0, ..el };
    assert!(plan(view(), Some(outside), false).is_err());
    let v = plan(view(), None, false).unwrap();
    assert_eq!(
        (v.viewport.width, v.viewport.height, v.document.y),
        (900.0, 700.0, 400.0)
    );
    let d = plan(view(), None, true).unwrap();
    assert!(d.full_page);
    assert_eq!(
        d.document,
        Rect {
            x: 0.0,
            y: 0.0,
            width: 900.0,
            height: 2138.0
        }
    );
    assert!((d.scale - 1600.0 / 2138.0).abs() < 1e-9);
    // A document higher than 8 000 pixels is cut there; a screen of density 2 is scaled for it.
    let tall = View {
        doc_height: 30_000.0,
        dpr: 2.0,
        ..view()
    };
    let t = plan(tall, None, true).unwrap();
    assert_eq!(t.document.height, SHOT_DOC_HEIGHT);
    assert!((t.scale - 1600.0 / 16_000.0).abs() < 1e-9);
    let dense = plan(View { dpr: 2.0, ..view() }, None, false).unwrap();
    assert!((dense.scale - 1600.0 / 1800.0).abs() < 1e-9);
}

#[test]
fn a_cursor_of_an_older_webview_reads_the_new_buffers_from_their_start() {
    // The browser opened again: its new buffers start over, behind the cursor the old ones gave.
    let b = Buffers::default();
    b.push_console(console("après", Level::Error));
    b.push_network(failure("http://localhost:4100/manque"));
    let read = b.read(&Query {
        since: Since::Cursor("40.7".parse().unwrap()),
        network: true,
        ..Default::default()
    });
    assert_eq!(read.entries.len(), 1);
    assert_eq!(read.network.len(), 1);
    assert_eq!(read.cursor, "1.1");
    assert_eq!(read.dropped, 0);
    // A cursor of these buffers reads after it, as before.
    let next = b.read(&Query {
        since: Since::Cursor(read.cursor.parse().unwrap()),
        network: true,
        ..Default::default()
    });
    assert!(next.entries.is_empty() && next.network.is_empty());
}

// ---------- the bounds of a capture (ESC-670) ----------

/// An image of noise, `w` × `h`, in `format`: what compresses worst.
fn noise(w: u32, h: u32, format: image::ImageFormat) -> Vec<u8> {
    let mut seed = 0x2545_f491_u32;
    let img = image::RgbImage::from_fn(w, h, |_, _| {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        image::Rgb([seed as u8, (seed >> 8) as u8, (seed >> 16) as u8])
    });
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut out, format)
        .unwrap();
    out.into_inner()
}

fn shot(bytes: Vec<u8>, format: Format) -> Shot {
    Shot {
        bytes,
        format,
        width: 0.0,
        height: 0.0,
        scale: 1.0,
    }
}

#[test]
fn a_jpeg_that_fits_is_kept_as_it_is() {
    let bytes = noise(120, 80, image::ImageFormat::Jpeg);
    let f = fit(&shot(bytes.clone(), Format::Jpeg)).unwrap();
    assert_eq!(f.bytes, bytes);
    assert_eq!((f.width, f.height, f.factor), (120, 80, 1.0));
}

#[test]
fn a_png_becomes_a_jpeg_1600_pixels_at_most_on_its_longest_side() {
    let f = fit(&shot(
        noise(2400, 600, image::ImageFormat::Png),
        Format::Png,
    ))
    .unwrap();
    assert_eq!(&f.bytes[..2], &[0xFF, 0xD8], "a JPEG");
    assert_eq!((f.width, f.height), (1600, 400));
    assert!((f.factor - 1600.0 / 2400.0).abs() < 1e-9);
    assert!(f.bytes.len() <= SHOT_BYTES);
    let back = image::load_from_memory(&f.bytes).unwrap();
    assert_eq!((back.width(), back.height()), (1600, 400));
}

#[test]
fn too_many_bytes_lower_the_quality_then_the_size() {
    let bytes = noise(1200, 900, image::ImageFormat::Jpeg);
    assert!(bytes.len() > 200_000, "{}", bytes.len());
    let f = fit_within(&shot(bytes, Format::Jpeg), 200_000).unwrap();
    assert!(f.bytes.len() <= 200_000, "{}", f.bytes.len());
    assert!(f.width <= 1200 && f.factor <= 1.0);
    // Under the real bound, a capture of 1 600 pixels of noise fits too.
    let f = fit(&shot(
        noise(1600, 1600, image::ImageFormat::Png),
        Format::Png,
    ))
    .unwrap();
    assert!(f.bytes.len() <= SHOT_BYTES, "{}", f.bytes.len());
}

#[test]
fn what_is_no_image_is_refused() {
    assert!(fit(&shot(b"not an image".to_vec(), Format::Png)).is_err());
}

#[test]
fn an_agent_reads_the_console_of_the_origins_it_may_read_alone() {
    // ESC-695: a tab goes to any web origin, a sign-in's included. What the console kept while it
    // was on a site the agent may not read is not given, even once the tab is back on the launch.
    let b = Buffers::default();
    b.set_origin("http://localhost:4211/tarifs");
    b.push_console(console("launch 1", Level::Log));
    b.push_network(failure("http://localhost:4211/api"));
    b.set_origin("https://accounts.google.com/signin?continue=x");
    b.push_console(console("provider", Level::Error));
    b.push_network(failure("https://accounts.google.com/x"));
    b.set_origin("data:text/html,x");
    b.push_console(console("no origin", Level::Log));
    b.set_origin("http://localhost:4211/callback");
    b.push_console(console("launch 2", Level::Log));
    let launch = |o: &str| o == "http://localhost:4211";
    let q = Query {
        network: true,
        ..Default::default()
    };
    let read = b.read_of(&q, &launch);
    let texts: Vec<_> = read.entries.iter().map(|e| e.text.as_str()).collect();
    assert_eq!(texts, vec!["launch 1", "launch 2"]);
    assert_eq!(read.network.len(), 1);
    assert_eq!(read.network[0].url, "http://localhost:4211/api");
    // The cursor still goes past what it did not give: the next read starts after it.
    assert_eq!(read.cursor, "4.2");
    // With the user's leave for the provider (C4), its entries too.
    let both = |o: &str| launch(o) || o == "https://accounts.google.com";
    assert_eq!(b.read_of(&q, &both).entries.len(), 3);
    // Escouade's own reading (no agent) is of every origin.
    assert_eq!(b.read(&q).entries.len(), 4);
}
