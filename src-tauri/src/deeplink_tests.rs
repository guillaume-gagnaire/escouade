//! The deep link `escouade://plugin/<id>` (STORE-11, ESC-287): what is read, what is refused, what a launch's arguments hold.

use crate::deeplink::{links_in, parse, DeepLink, Inbox};

fn plugin(id: &str) -> DeepLink {
    DeepLink::Plugin { id: id.into() }
}

fn bad(url: &str) -> DeepLink {
    DeepLink::Bad { url: url.into() }
}

#[test]
fn reads_the_page_of_a_plugin() {
    assert_eq!(
        parse("escouade://plugin/dev.exemple.slack"),
        plugin("dev.exemple.slack")
    );
    // A `/` after the id, the scheme and the word in any case, `%2E` as the store's encodeURIComponent never writes it.
    assert_eq!(
        parse("escouade://plugin/dev.exemple.slack/"),
        plugin("dev.exemple.slack")
    );
    assert_eq!(
        parse("Escouade://Plugin/dev.exemple.team-rules"),
        plugin("dev.exemple.team-rules")
    );
    assert_eq!(
        parse("escouade://plugin/dev%2Eexemple%2Eslack"),
        plugin("dev.exemple.slack")
    );
    assert_eq!(
        parse("escouade:plugin/dev.escouade.github"),
        plugin("dev.escouade.github")
    );
    assert_eq!(
        parse("  escouade://plugin/dev.exemple.slack\n"),
        plugin("dev.exemple.slack")
    );
}

#[test]
fn refuses_what_is_no_plugin_page() {
    for url in [
        "escouade://",
        "escouade://plugin",
        "escouade://plugin/",
        "escouade://plugins/dev.exemple.slack",
        "escouade://install/dev.exemple.slack",
        "escouade://plugin/dev.exemple.slack/install",
        "escouade://plugin/dev.exemple.slack?install=1",
        "escouade://plugin/dev.exemple.slack#versions",
        // No reversed domain name in lower case.
        "escouade://plugin/slack",
        "escouade://plugin/Dev.Exemple.Slack",
        "escouade://plugin/dev..slack",
        "escouade://plugin/../../etc/passwd",
        "escouade://plugin/dev.exemple.slack%2Finstall",
        "escouade://plugin/dev.exemple.%zz",
        "escouade://plugin/dev.exemple.%ff",
        "https://store.escouade.dev/plugins/dev.exemple.slack",
        "",
    ] {
        assert_eq!(parse(url), bad(url.trim()), "{url:?}");
    }
    // Too long: refused, shown cut short.
    let long = format!("escouade://plugin/dev.{}", "a".repeat(400));
    match parse(&long) {
        DeepLink::Bad { url } => assert!(url.ends_with('…') && url.chars().count() == 121),
        other => panic!("{other:?}"),
    }
    // Nothing of a control character reaches the window.
    assert_eq!(
        parse("escouade://plugin/\u{1b}[31m"),
        bad("escouade://plugin/[31m")
    );
}

#[test]
fn finds_the_links_among_a_launchs_arguments() {
    // The program's name is never one, nor the arguments of another kind.
    assert_eq!(
        links_in([
            "escouade://plugin/x.y",
            "--flag",
            "escouade://plugin/dev.exemple.slack",
            "ESCOUADE:plugin/a.b",
            "c:\\escouade"
        ]),
        vec!["escouade://plugin/dev.exemple.slack", "ESCOUADE:plugin/a.b"]
    );
    assert!(links_in(["escouade.exe"]).is_empty());
    assert!(links_in(Vec::<String>::new()).is_empty());
    // A bad link of the scheme is kept, to be said.
    assert_eq!(
        links_in(["escouade", "escouade://nope"]),
        vec!["escouade://nope"]
    );
}

#[test]
fn the_inbox_gives_each_link_once() {
    let inbox = Inbox::default();
    inbox.put(plugin("dev.exemple.slack"));
    inbox.put(bad("escouade://nope"));
    assert_eq!(
        inbox.take(),
        vec![plugin("dev.exemple.slack"), bad("escouade://nope")]
    );
    assert!(inbox.take().is_empty());
}

#[test]
fn says_to_the_window_what_a_link_asks() {
    assert_eq!(
        serde_json::to_value(plugin("dev.exemple.slack")).unwrap(),
        serde_json::json!({ "kind": "plugin", "id": "dev.exemple.slack" })
    );
    assert_eq!(
        serde_json::to_value(bad("escouade://nope")).unwrap(),
        serde_json::json!({ "kind": "bad", "url": "escouade://nope" })
    );
}

fn read(file: &str) -> String {
    std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(file)).unwrap()
}

fn json(file: &str) -> serde_json::Value {
    serde_json::from_str(&read(file)).unwrap()
}

#[test]
fn the_installers_register_the_scheme() {
    // Read by the bundler: the NSIS and MSI installers write it in the registry, the .app in its Info.plist
    // (CFBundleURLTypes), the .deb, .rpm and AppImage in their `.desktop` (MimeType=x-scheme-handler/escouade).
    let conf = json("tauri.conf.json");
    assert_eq!(
        conf["plugins"]["deep-link"]["desktop"]["schemes"],
        serde_json::json!([crate::deeplink::SCHEME])
    );
    // The packages of Linux write their `.desktop` from the template, which must hand the link over (`%u`) and keep the
    // MimeType the bundler fills.
    let linux = json("tauri.linux.conf.json");
    for package in ["deb", "rpm"] {
        let template = linux["bundle"]["linux"][package]["desktopTemplate"]
            .as_str()
            .unwrap_or_else(|| panic!("{package}"));
        let desktop = read(template);
        assert!(
            desktop.lines().any(|l| l == "Exec={{exec}} %u"),
            "{desktop}"
        );
        assert!(desktop.contains("MimeType={{mime_type}}"), "{desktop}");
    }
}
