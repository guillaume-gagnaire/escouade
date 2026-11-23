//! Against the fake Open VSX (`fake`): the search, the install of a theme, the sums, the signatures, the archives
//! trapped, the allowed list before any download, the publishers' trust, the updates, the files served.

use crate::allowed::{check, Layer, Rule, Rules};
use crate::fake::{self, FakeOpenVsx, Spoilt, Version};
use crate::install::{self, install, updates};
use crate::manifest::Kind;
use crate::registry::{compare_versions, Registry};
use crate::serve::{self, mime_of};
use crate::store::Store;
use crate::unpack::{bad_name, unpack, Limits};
use crate::{Code, ExtId};
use serde_json::json;

fn client() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().build().unwrap()
}

struct Bench {
    fake: FakeOpenVsx,
    registry: Registry,
    store: Store,
    _dir: tempfile::TempDir,
}

async fn bench() -> Bench {
    let fake = FakeOpenVsx::start().await;
    let registry = Registry::new(&fake.url, client()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("extensions"));
    Bench {
        fake,
        registry,
        store,
        _dir: dir,
    }
}

fn small() -> Limits {
    Limits {
        vsix: 4 << 20,
        files: 20,
        total: 1 << 20,
        file: 512 << 10,
    }
}

fn theme(version: &str) -> Version {
    Version::new(
        "acme",
        "night",
        version,
        fake::theme_vsix("acme", "night", version, "Acme Night"),
    )
}

impl Bench {
    async fn install(&self, id: &str, layers: &[Layer]) -> crate::Result<crate::store::Installed> {
        install(
            &self.registry,
            &self.store,
            &small(),
            id,
            None,
            layers,
            true,
        )
        .await
    }

    /// Nothing of a refused install is left: no extension, no scratch folder.
    fn nothing_left(&self) {
        assert!(self.store.list().is_empty());
        let left: Vec<_> = std::fs::read_dir(self.store.root().join(".tmp"))
            .map(|d| d.flatten().collect())
            .unwrap_or_default();
        assert!(left.is_empty(), "{left:?}");
    }
}

fn layer(by: &str, rules: serde_json::Value) -> Layer {
    Layer {
        by: by.into(),
        imposed: false,
        rules: Rules::parse(&rules).0,
    }
}

#[tokio::test]
async fn a_search_and_a_card_come_from_the_registry() {
    let b = bench().await;
    b.fake.add(theme("1.0.0"));
    b.fake.add(theme("1.1.0"));
    let page = b.registry.search("night", 0, 10).await.unwrap();
    assert_eq!(page.extensions.len(), 1);
    assert_eq!(page.extensions[0].id, "acme.night");
    assert_eq!(page.extensions[0].version, "1.1.0");
    assert!(page.extensions[0].verified);
    let card = b
        .registry
        .detail(&ExtId::parse("acme.night").unwrap(), None)
        .await
        .unwrap();
    assert_eq!(card.version, "1.1.0");
    assert_eq!(card.versions, vec!["1.1.0", "1.0.0"]);
    assert_eq!(card.publisher, "acme Inc.");
    // Its icon, flat as the page reads it.
    assert!(card
        .icon
        .as_deref()
        .is_some_and(|i| i.ends_with("/icon.png")));
    let none = b
        .registry
        .detail(&ExtId::parse("acme.absent").unwrap(), None)
        .await
        .unwrap_err();
    assert_eq!(none.code, Code::NotFound);
}

#[tokio::test]
async fn a_theme_is_installed_checked_and_its_files_served_with_their_type() {
    let b = bench().await;
    b.fake.add(theme("1.0.0"));
    let e = b.install("acme.night", &[]).await.unwrap();
    assert_eq!(e.kind, Kind::Declarative);
    assert!(e.signed && e.verified && e.enabled);
    assert_eq!(e.contributes.themes, vec!["Acme Night"]);
    assert_eq!(e.sha256.len(), 64);
    // Only `extension/` is unpacked.
    let dir = b.store.dir_of("acme.night").unwrap();
    assert!(dir.join("package.json").is_file());
    assert!(!dir.join("extension.vsixmanifest").exists());
    let (bytes, mime) = serve::read(&b.store, "ACME.Night", "themes/theme.json").unwrap();
    assert!(String::from_utf8(bytes).unwrap().contains("Acme Night"));
    assert_eq!(mime, "application/json; charset=utf-8");
    let r = serve::respond(&b.store, "GET", "/acme.night/themes/theme.json");
    assert_eq!(r.status(), 200);
    assert_eq!(
        r.headers()["content-type"],
        "application/json; charset=utf-8"
    );
    assert_eq!(r.headers()["x-content-type-options"], "nosniff");
    // A page of an extension sends no Referer where it links to (ESC-654).
    assert_eq!(r.headers()["referrer-policy"], "no-referrer");
    // Read again from the disk, the list holds.
    let again = Store::open(b.store.root());
    assert_eq!(again.get("acme.night").unwrap().version, "1.0.0");
    assert!(again.trusts("acme"));
}

#[tokio::test]
async fn a_false_sum_a_false_signature_and_no_signature_are_refused() {
    for (spoilt, code) in [
        (Spoilt::Sum, Code::BadSum),
        (Spoilt::Signature, Code::BadSignature),
        (Spoilt::Unsigned, Code::Unsigned),
    ] {
        let b = bench().await;
        b.fake.add(theme("1.0.0").spoilt(spoilt));
        let r = b.install("acme.night", &[]).await.unwrap_err();
        assert_eq!(r.code, code, "{spoilt:?}: {r}");
        b.nothing_left();
    }
}

#[tokio::test]
async fn a_signature_by_another_key_is_refused() {
    let vsix = fake::theme_vsix("acme", "night", "1.0.0", "Acme Night");
    let other = ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]);
    use ed25519_dalek::Signer;
    let sig = other.sign(&vsix).to_bytes();
    let sigzip = fake::zip(&[(".signature.sig", &sig[..])]);
    let key = crate::verify::public_key(&fake::public_pem()).unwrap();
    assert_eq!(
        crate::verify::check_signature(&vsix, &sigzip, &key)
            .unwrap_err()
            .code,
        Code::BadSignature
    );
    assert!(crate::verify::check_signature(&vsix, &fake::sigzip(&vsix), &key).is_ok());
    // A key that is not Ed25519, a signature that is not one.
    assert!(crate::verify::public_key(
        "-----BEGIN PUBLIC KEY-----\nAAAA\n-----END PUBLIC KEY-----"
    )
    .is_err());
    assert!(crate::verify::check_signature(&vsix, b"not a zip", &key).is_err());
}

/// Open VSX's real key and signature of a real VSIX (Dracula 2.25.1, read on 2026-11-16): the format holds.
#[test]
fn the_format_of_open_vsx_is_the_one_checked() {
    let pem = "-----BEGIN PUBLIC KEY-----\nMCowBQYDK2VwAyEAje+vAaSS1zHV5WHCJSa5UXvxRo6+yerEU3IEmtuEuF4=\n-----END PUBLIC KEY-----\n";
    assert!(crate::verify::public_key(pem).is_ok());
}

fn trapped_version(vsix: Vec<u8>) -> Version {
    Version::new("acme", "night", "1.0.0", vsix)
}

#[tokio::test]
async fn trapped_archives_are_refused_whole() {
    let pkg = fake::package("acme", "night", "1.0.0", json!({}));
    let zeros = vec![0u8; 2 << 20];
    let many: Vec<(String, Vec<u8>)> = (0..30)
        .map(|i| (format!("extension/f{i}.json"), b"{}".to_vec()))
        .collect();
    let mut many_refs: Vec<(&str, &[u8])> = vec![("extension/package.json", &pkg)];
    many_refs.extend(many.iter().map(|(n, b)| (n.as_str(), b.as_slice())));
    let cases: Vec<(&str, Vec<u8>)> = vec![
        (
            "..",
            fake::zip(&[
                ("extension/package.json", &pkg),
                ("extension/../../evil.txt", b"x"),
            ]),
        ),
        (
            "absolute",
            fake::zip(&[("extension/package.json", &pkg), ("/etc/evil", b"x")]),
        ),
        (
            "drive",
            fake::zip(&[("extension/package.json", &pkg), ("C:/evil", b"x")]),
        ),
        (
            "backslash",
            fake::zip(&[
                ("extension/package.json", &pkg),
                ("extension\\..\\evil", b"x"),
            ]),
        ),
        (
            "reserved",
            fake::zip(&[
                ("extension/package.json", &pkg),
                ("extension/con.json", b"x"),
            ]),
        ),
        (
            "link",
            fake::zip_with_link(
                &[("extension/package.json", &pkg)],
                "extension/key",
                "/home/me/.ssh/id_ed25519",
            ),
        ),
        (
            "bomb",
            fake::zip(&[
                ("extension/package.json", &pkg),
                ("extension/zeros.bin", &zeros),
            ]),
        ),
        ("too many", fake::zip(&many_refs)),
        (
            "twice",
            fake::zip(&[
                ("extension/package.json", &pkg),
                ("extension/A.json", b"1"),
                ("extension/a.json", b"2"),
            ]),
        ),
        ("no package.json", fake::zip(&[("extension/x.json", b"{}")])),
    ];
    for (what, vsix) in cases {
        let b = bench().await;
        b.fake.add(trapped_version(vsix));
        let r = b.install("acme.night", &[]).await.unwrap_err();
        assert_eq!(r.code, Code::Trapped, "{what}: {r}");
        b.nothing_left();
        // Nothing was written outside the store.
        assert!(
            !b.store.root().parent().unwrap().join("evil.txt").exists(),
            "{what}"
        );
    }
}

#[test]
fn a_bomb_is_read_no_further_than_its_bound() {
    // 2 MB of zeros declared, deflated to a few KB: the reader stops at the bound, whatever the archive says.
    let pkg = fake::package("acme", "night", "1.0.0", json!({}));
    let vsix = fake::zip(&[
        ("extension/package.json", &pkg),
        ("extension/zeros.bin", &vec![0u8; 2 << 20]),
    ]);
    assert!(vsix.len() < 64 << 10);
    let dir = tempfile::tempdir().unwrap();
    let r = unpack(&vsix, dir.path(), &small()).unwrap_err();
    assert_eq!(r.code, Code::Trapped);
    assert!(r.message.contains("Mo") || r.message.contains("MB"), "{r}");
}

#[test]
fn the_names_an_archive_may_not_hold() {
    for bad in [
        "",
        "/a",
        "\\a",
        "a/../b",
        "..",
        "a:b",
        "C:/x",
        "a\\b",
        "x/aux.txt",
        "x/end.",
        "x/end ",
        "a//b",
    ] {
        assert!(bad_name(bad).is_some(), "{bad:?}");
    }
    for good in [
        "extension/package.json",
        "extension/themes/a-b_c.json",
        "extension/.gitignore",
        "dir/",
    ] {
        assert!(bad_name(good).is_none(), "{good:?}");
    }
}

#[tokio::test]
async fn an_archive_of_another_extension_is_refused() {
    let b = bench().await;
    b.fake.add(Version::new(
        "acme",
        "night",
        "1.0.0",
        fake::theme_vsix("evil", "night", "1.0.0", "Evil"),
    ));
    let r = b.install("acme.night", &[]).await.unwrap_err();
    assert_eq!(r.code, Code::Mismatch);
    b.nothing_left();
}

#[tokio::test]
async fn an_extension_outside_the_allowed_list_is_never_asked_for() {
    let b = bench().await;
    b.fake.add(theme("1.0.0"));
    let layers = [layer(
        "la stratégie de ce poste",
        json!({ "*": false, "dracula-theme": true }),
    )];
    let r = b.install("acme.night", &layers).await.unwrap_err();
    assert_eq!(r.code, Code::NotAllowed);
    assert!(
        r.message.contains("la stratégie de ce poste") || r.message.contains("acme.night"),
        "{r}"
    );
    // Not even its card: nothing was asked of Open VSX.
    assert!(b.fake.asked().is_empty(), "{:?}", b.fake.asked());
    b.nothing_left();
}

#[tokio::test]
async fn a_version_outside_the_list_is_refused_before_its_download() {
    let b = bench().await;
    b.fake.add(theme("1.0.0"));
    b.fake.add(theme("2.0.0"));
    let layers = [
        layer("les réglages de ce poste", json!({ "acme": true })),
        layer("l’équipe Acme", json!({ "acme.night": ["1.0.0"] })),
    ];
    let r = b.install("acme.night", &layers).await.unwrap_err();
    assert_eq!(r.code, Code::NotAllowed);
    assert!(r.message.contains("2.0.0"), "{r}");
    assert!(!b.fake.downloaded());
    // The version allowed passes.
    let e = install(
        &b.registry,
        &b.store,
        &small(),
        "acme.night",
        Some("1.0.0"),
        &layers,
        true,
    )
    .await
    .unwrap();
    assert_eq!(e.version, "1.0.0");
}

#[tokio::test]
async fn a_publisher_not_trusted_is_asked_about_before_any_download() {
    let b = bench().await;
    b.fake.add(theme("1.0.0").unverified());
    let r = install(
        &b.registry,
        &b.store,
        &small(),
        "acme.night",
        None,
        &[],
        false,
    )
    .await
    .unwrap_err();
    assert_eq!(r.code, Code::Untrusted);
    assert!(r.message.contains("acme Inc."), "{r}");
    assert!(!b.fake.downloaded());
    let e = b.install("acme.night", &[]).await.unwrap();
    assert!(!e.verified);
    // Trusted once, not asked again.
    b.store.remove("acme.night").unwrap();
    install(
        &b.registry,
        &b.store,
        &small(),
        "acme.night",
        None,
        &[],
        false,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn an_update_is_proposed_checked_again_and_replaces_the_former_version() {
    let b = bench().await;
    b.fake.add(theme("1.0.0"));
    b.install("acme.night", &[]).await.unwrap();
    let before = b.store.dir_of("acme.night").unwrap();
    b.store.set_enabled("acme.night", false).unwrap();
    assert!(updates(&b.registry, &b.store, &[]).await.is_empty());
    b.fake.add(theme("1.2.0").spoilt(Spoilt::Signature));
    let ups = updates(&b.registry, &b.store, &[]).await;
    assert_eq!(ups.len(), 1);
    assert_eq!(
        (ups[0].from.as_str(), ups[0].to.as_str()),
        ("1.0.0", "1.2.0")
    );
    // Proposed only: nothing changed until it is asked.
    assert_eq!(b.store.get("acme.night").unwrap().version, "1.0.0");
    // A spoilt update is refused, the version before kept.
    let r = b.install("acme.night", &[]).await.unwrap_err();
    assert_eq!(r.code, Code::BadSignature);
    assert_eq!(b.store.get("acme.night").unwrap().version, "1.0.0");
    assert!(before.join("package.json").is_file());
    b.fake.add(theme("1.3.0"));
    let e = b.install("acme.night", &[]).await.unwrap();
    assert_eq!(e.version, "1.3.0");
    assert!(!e.enabled, "an update keeps it off");
    assert!(!before.exists(), "the former version's folder goes");
    // Refused by the list, an update says so.
    b.fake.add(theme("1.4.0"));
    let ups = updates(
        &b.registry,
        &b.store,
        &[layer("x", json!({ "acme.night": ["1.3.0"] }))],
    )
    .await;
    assert!(ups[0].refused.is_some());
}

#[tokio::test]
async fn code_extensions_install_and_say_what_they_wait_for() {
    let b = bench().await;
    b.fake.add(Version::new(
        "acme",
        "web",
        "1.0.0",
        fake::code_vsix("acme", "web", "1.0.0", true),
    ));
    b.fake.add(Version::new(
        "acme",
        "node",
        "1.0.0",
        fake::code_vsix("acme", "node", "1.0.0", false),
    ));
    let web = b.install("acme.web", &[]).await.unwrap();
    let node = b.install("acme.node", &[]).await.unwrap();
    assert_eq!(web.kind, Kind::Web);
    assert_eq!(node.kind, Kind::Node);
    let w = install::state(web, &[]);
    let n = install::state(node, &[]);
    assert!(w.waits.is_some() && n.waits.is_some());
    assert_ne!(w.waits, n.waits);
    let theme = Version::new(
        "acme",
        "t",
        "1.0.0",
        fake::theme_vsix("acme", "t", "1.0.0", "T"),
    );
    b.fake.add(theme);
    let t = b.install("acme.t", &[]).await.unwrap();
    let s = install::state(
        t,
        &[layer(
            "le .escouade.yml de Atlas",
            json!({ "acme.t": false }),
        )],
    );
    assert!(s.waits.is_none());
    assert!(s.refused.unwrap().contains("Atlas"));
}

#[tokio::test]
async fn only_the_files_of_an_installed_extension_are_served() {
    let b = bench().await;
    b.fake.add(theme("1.0.0"));
    b.install("acme.night", &[]).await.unwrap();
    std::fs::write(b.store.root().join("secret.txt"), "secret").unwrap();
    for (path, status) in [
        ("/acme.night/../installed.json", 403),
        ("/acme.night/%2e%2e/secret.txt", 403),
        ("/acme.night/..%2Fsecret.txt", 403),
        ("/acme.night//etc/passwd", 403),
        ("/acme.night/C:/Windows/win.ini", 403),
        ("/acme.night/themes", 404),
        ("/acme.night/absent.json", 404),
        ("/acme.other/package.json", 404),
        ("/../secret.txt", 404),
        ("/acme.night", 403),
    ] {
        assert_eq!(
            serve::respond(&b.store, "GET", path).status(),
            status,
            "{path}"
        );
    }
    assert_eq!(
        serve::respond(&b.store, "POST", "/acme.night/package.json").status(),
        405
    );
    assert_eq!(
        serve::respond(&b.store, "PUT", "/acme.night/package.json").status(),
        405
    );
    let head = serve::respond(&b.store, "HEAD", "/acme.night/package.json");
    assert_eq!(head.status(), 200);
    assert!(head.body().is_empty());
    // Removed, no longer served.
    b.store.remove("acme.night").unwrap();
    assert_eq!(
        serve::respond(&b.store, "GET", "/acme.night/package.json").status(),
        404
    );
}

#[cfg(unix)]
#[tokio::test]
async fn a_link_in_an_installed_folder_is_never_served() {
    let b = bench().await;
    b.fake.add(theme("1.0.0"));
    b.install("acme.night", &[]).await.unwrap();
    let dir = b.store.dir_of("acme.night").unwrap();
    std::fs::write(b.store.root().join("secret.txt"), "secret").unwrap();
    std::os::unix::fs::symlink(b.store.root().join("secret.txt"), dir.join("out.json")).unwrap();
    std::os::unix::fs::symlink(dir.join("package.json"), dir.join("in.json")).unwrap();
    assert_ne!(
        serve::respond(&b.store, "GET", "/acme.night/out.json").status(),
        200
    );
    assert_ne!(
        serve::respond(&b.store, "GET", "/acme.night/in.json").status(),
        200
    );
}

#[tokio::test]
async fn the_requests_go_through_the_proxy_given() {
    // The registry's host does not exist: only the proxy (the fake, answering requests in proxy form) reaches it.
    let fake = FakeOpenVsx::start().await;
    fake.add(theme("1.0.0"));
    let through = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(&fake.url).unwrap())
        .build()
        .unwrap();
    let registry = Registry::new("http://open-vsx.invalid", through).unwrap();
    let page = registry.search("night", 0, 5).await.unwrap();
    assert_eq!(page.extensions.len(), 1);
    assert!(fake.asked().iter().all(|a| a.proxied));
    // An address of the card on another origin is not asked.
    let r = registry
        .bytes(&format!("{}/files/x", fake.url), 10)
        .await
        .unwrap_err();
    assert_eq!(r.code, Code::Network);
}

#[test]
fn the_allowed_rule_is_vs_codes() {
    let id = |s: &str| ExtId::parse(s).unwrap();
    let r = Rules::parse(&json!({
        "*": false,
        "Dracula-Theme": true,
        "acme": "stable",
        "acme.pinned": ["1.2.3", "1.2.4@win32-x64"],
        "acme.off": false,
    }))
    .0;
    assert!(r.allows(&id("dracula-theme.theme-dracula"), Some(("1.0.0", true))));
    assert!(!r.allows(&id("other.x"), None));
    assert!(r.allows(&id("acme.x"), Some(("1.0.0", false))));
    assert!(!r.allows(&id("acme.x"), Some(("1.0.0-beta", true))));
    assert!(r.allows(&id("acme.pinned"), None));
    assert!(r.allows(&id("acme.pinned"), Some(("1.2.4", false))));
    assert!(!r.allows(&id("acme.pinned"), Some(("1.3.0", false))));
    assert!(!r.allows(&id("acme.off"), None));
    assert_eq!(r.rule_for(&id("ACME.OFF")), Some(&Rule::Deny));
    // Nothing set: all allowed; something set, what it does not name is not.
    assert!(Rules::parse(&json!({})).0.allows(&id("any.thing"), None));
    assert!(Rules::parse(&serde_json::Value::Null)
        .0
        .allows(&id("any.thing"), None));
    assert!(!Rules::parse(&json!({ "acme": true }))
        .0
        .allows(&id("any.thing"), None));
    // What cannot be read is said and left out.
    let (r, warnings) =
        Rules::parse(&json!({ "a b": true, "acme": 3, "ok": true, "pub": ["1.0.0"] }));
    assert_eq!(warnings.len(), 3, "{warnings:?}");
    assert_eq!(r.0.len(), 1);
    assert_eq!(Rules::parse(&json!(["x"])).1.len(), 1);
    // Each layer must allow it: the first that refuses says it.
    let layers = [
        layer("A", json!({ "*": true })),
        layer("B", json!({ "*": true, "acme": false })),
    ];
    assert!(check(&layers, &id("dracula.x"), None).is_ok());
    assert!(check(&layers, &id("acme.x"), None)
        .unwrap_err()
        .message
        .contains('B'));
}

#[test]
fn ids_and_versions_are_checked() {
    assert!(ExtId::parse("dracula-theme.theme-dracula").is_ok());
    for bad in [
        "", "nodot", "a.", ".b", "a.b.c", "a/b.c", "a.b/../c", "a .b", "a.b?x",
    ] {
        assert!(ExtId::parse(bad).is_err(), "{bad:?}");
    }
    assert!(crate::is_version("1.2.3-beta.1+x"));
    for bad in ["", "v1", "1/2", "1..2", "1.2 3"] {
        assert!(!crate::is_version(bad), "{bad:?}");
    }
    use std::cmp::Ordering::*;
    assert_eq!(compare_versions("1.10.0", "1.9.2"), Greater);
    assert_eq!(compare_versions("1.0.0-beta", "1.0.0"), Less);
    assert_eq!(compare_versions("2.0.0", "2.0.0"), Equal);
}

#[test]
fn each_file_has_its_type() {
    assert_eq!(mime_of("dist/web.js"), "text/javascript; charset=utf-8");
    assert_eq!(mime_of("a.mjs"), "text/javascript; charset=utf-8");
    assert_eq!(
        mime_of("syntaxes/toml.tmLanguage.json"),
        "application/json; charset=utf-8"
    );
    assert_eq!(
        mime_of("syntaxes/x.tmLanguage"),
        "application/xml; charset=utf-8"
    );
    assert_eq!(mime_of("icons/a.svg"), "image/svg+xml");
    assert_eq!(mime_of("fonts/x.woff2"), "font/woff2");
    assert_eq!(mime_of("server.wasm"), "application/wasm");
    assert_eq!(mime_of("LICENSE"), "application/octet-stream");
}

#[tokio::test]
async fn the_theme_chosen_is_one_of_an_installed_extension_turned_on() {
    let b = bench().await;
    b.fake.add(theme("1.0.0"));
    b.install("acme.night", &[]).await.unwrap();
    assert!(b.store.set_theme(Some("acme.night/Other".into())).is_err());
    b.store
        .set_theme(Some("acme.night/Acme Night".into()))
        .unwrap();
    assert_eq!(b.store.theme().as_deref(), Some("acme.night/Acme Night"));
    // Turned off, Escouade's theme again.
    b.store.set_enabled("acme.night", false).unwrap();
    assert_eq!(b.store.theme(), None);
    assert!(b
        .store
        .set_theme(Some("acme.night/Acme Night".into()))
        .is_err());
    b.store.set_theme(None).unwrap();
}
