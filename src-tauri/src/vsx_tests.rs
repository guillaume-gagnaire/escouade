//! The extensions in the app (ESC-481): the layers of the allowed list (managed device, team, settings, `.escouade.yml`)
//! and who refuses, a private project that asks nothing of Open VSX, the app's authorities trusted, the protocol.

use super::*;
use crate::core_tests::harness;
use crate::locks::{Managed, EXTENSIONS_ALLOWED as POLICY};
use crate::projectfile::EXTENSIONS_ALLOWED as FILE_KEY;
use escouade_vsx::fake::{self, FakeOpenVsx, Version};
use escouade_vsx::ExtId;
use serde_json::json;

fn managed(v: serde_json::Value) -> Managed {
    Managed {
        values: [(POLICY.to_owned(), (v, "the machine's policy".to_owned()))]
            .into_iter()
            .collect(),
        warnings: Vec::new(),
    }
}

fn set_file(core: &Core, id: &str, v: serde_json::Value) {
    let mut projects = core.projects.write();
    let p = projects.iter_mut().find(|p| p.id == id).unwrap();
    let mut file = p.shared.file.clone().unwrap_or_default();
    file.values.insert(FILE_KEY.to_owned(), v);
    p.shared.file = Some(file);
}

fn theme() -> Version {
    Version::new(
        "acme",
        "night",
        "1.0.0",
        fake::theme_vsix("acme", "night", "1.0.0", "Acme Night"),
    )
}

#[test]
fn the_file_holds_the_allowed_list_as_a_mapping() {
    let r = crate::projectfile::read(
        "version: 1\neditor:\n  extensions_allowed:\n    \"*\": false\n    dracula-theme: true\n    acme.pinned: [1.2.3]\n    acme: stable\n    \"a b\": true\n",
    )
    .unwrap();
    assert_eq!(
        r.values[FILE_KEY],
        json!({ "*": false, "dracula-theme": true, "acme.pinned": ["1.2.3"], "acme": "stable" })
    );
    assert_eq!(r.warnings.len(), 1, "{:?}", r.warnings);
    // The file's alone: never applied to the project nor exported from it.
    assert!(crate::projectfile::file_only(FILE_KEY));
    assert!(
        crate::projectfile::schema()["properties"]["editor"]["properties"]["extensions_allowed"]
            .is_object()
    );
}

#[tokio::test]
async fn each_layer_must_allow_an_extension_and_the_strongest_says_it() {
    let h = harness("vsx-layers");
    let (p, _) = h.project(false).await;
    let id = ExtId::parse("acme.night").unwrap();
    // Nothing set: everything allowed.
    let ctx = context(&h.core, std::slice::from_ref(&p.id));
    assert!(ctx.layers.is_empty());
    h.core.settings.write().extensions_allowed = json!({ "*": true });
    set_file(&h.core, &p.id, json!({ "acme": false, "*": true }));
    let ctx = context(&h.core, std::slice::from_ref(&p.id));
    assert_eq!(ctx.layers.len(), 2);
    let r = escouade_vsx::allowed::check(&ctx.layers, &id, None).unwrap_err();
    assert!(r.message.contains(".escouade.yml"), "{r}");
    // The managed device's comes first, imposed: it is the one that says it.
    h.core
        .set_managed(managed(json!({ "*": false, "dracula-theme": true })));
    let ctx = context(&h.core, std::slice::from_ref(&p.id));
    assert_eq!(ctx.layers.len(), 3);
    assert!(ctx.layers[0].imposed);
    let r = escouade_vsx::allowed::check(&ctx.layers, &id, None).unwrap_err();
    assert!(
        r.message.contains("stratégie") || r.message.contains("policy"),
        "{r}"
    );
    // Shown greyed in the settings: the row is locked by the key.
    assert_eq!(
        h.core.locks.read().rows.get("extensionsAllowed"),
        Some(&vec![POLICY.to_owned()])
    );
    // An installed extension the list now leaves out says so (and is not loaded by the view).
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeOpenVsx::start().await;
    fake.add(theme());
    let ext = Extensions::at(dir.path(), &fake.url);
    h.core.set_managed(Managed::default());
    h.core.settings.write().extensions_allowed = serde_json::Value::Null;
    set_file(&h.core, &p.id, serde_json::Value::Null);
    let ctx = context(&h.core, std::slice::from_ref(&p.id));
    install(&ext, &h.core, &ctx, "acme.night", None, true)
        .await
        .unwrap();
    h.core
        .set_managed(managed(json!({ "acme": false, "*": true })));
    let ctx = context(&h.core, std::slice::from_ref(&p.id));
    let list = installed(&ext, &ctx);
    assert!(list.extensions[0].refused.is_some());
    // And it is not downloaded again: refused before Open VSX is asked.
    let before = fake.asked().len();
    let r = install(&ext, &h.core, &ctx, "acme.night", None, true)
        .await
        .unwrap_err();
    assert_eq!(r.code, Code::NotAllowed);
    assert_eq!(fake.asked().len(), before);
}

#[tokio::test]
async fn a_team_imposes_its_list_too() {
    let h = harness("vsx-team");
    let (p, _) = h.project(false).await;
    let team = crate::locks::TeamPolicies {
        id: "acme".into(),
        team: "Acme".into(),
        signed: None,
        recommended: Default::default(),
        enforced: [(POLICY.to_owned(), json!({ "*": false }))]
            .into_iter()
            .collect(),
    };
    let locks = crate::locks::Locks::compute(crate::locks::KEYS, Some(&team), &Managed::default());
    let layers = locks.extension_layers();
    assert_eq!(layers.len(), 1);
    assert!(layers[0].by.by().contains("Acme"));
    let _ = p;
}

#[tokio::test]
async fn a_private_project_asks_nothing_of_open_vsx() {
    let h = harness("vsx-private");
    let (p, _) = h.project(false).await;
    h.core
        .projects
        .write()
        .iter_mut()
        .find(|x| x.id == p.id)
        .unwrap()
        .private = true;
    let fake = FakeOpenVsx::start().await;
    fake.add(theme());
    let dir = tempfile::tempdir().unwrap();
    let ext = Extensions::at(dir.path(), &fake.url);
    let ctx = context(&h.core, std::slice::from_ref(&p.id));
    assert!(ctx.private.is_some());
    for r in [
        search(&ext, &h.core, &ctx, "night", 0).await.err(),
        card(&ext, &h.core, &ctx, "acme.night", None).await.err(),
        install(&ext, &h.core, &ctx, "acme.night", None, true)
            .await
            .err(),
        updates(&ext, &h.core, &ctx).await.err(),
        image(&ext, &h.core, &ctx, &format!("{}/x.png", fake.url))
            .await
            .err(),
    ] {
        assert_eq!(r.map(|r| r.code), Some(Code::Private));
    }
    assert!(fake.asked().is_empty(), "{:?}", fake.asked());
    // A view that names no project of the device, while it holds a private one: refused as private too (ESC-648).
    let ctx = context(&h.core, &[]);
    assert!(ctx.private.is_some());
    let ctx = context(&h.core, &["no-such-project".to_owned()]);
    assert!(ctx.private.is_some());
    assert!(fake.asked().is_empty(), "{:?}", fake.asked());
    // Without the private project in the view, the search goes.
    let other = h.second_project().await;
    let ctx = context(&h.core, std::slice::from_ref(&other.id));
    assert!(ctx.private.is_none());
    assert_eq!(
        search(&ext, &h.core, &ctx, "night", 0)
            .await
            .unwrap()
            .extensions
            .len(),
        1
    );
}

#[tokio::test]
async fn the_registry_is_asked_with_the_apps_authorities() {
    // A registry over https whose certificate only the settings' file of authorities makes trusted (a proxy's root).
    let server = escouade_forge::integrations::fake::FakeServer::start_tls().await;
    server.on(
        "GET",
        "/api/-/search",
        200,
        json!({ "extensions": [], "totalSize": 0 }),
    );
    let dir = tempfile::tempdir().unwrap();
    let ext = Extensions::at(dir.path(), &server.url);
    let mut settings = crate::model::Settings {
        no_proxy: String::new(),
        ..Default::default()
    };
    let r = ext
        .registry(&settings)
        .unwrap()
        .search("x", 0, 5)
        .await
        .unwrap_err();
    assert_eq!(r.code, Code::Network);
    let pem = dir.path().join("root.pem");
    std::fs::write(&pem, &server.cert_pem).unwrap();
    settings.extra_ca_file = pem.to_string_lossy().into_owned();
    let page = ext
        .registry(&settings)
        .unwrap()
        .search("x", 0, 5)
        .await
        .unwrap();
    assert_eq!(page.total, 0);
}

#[tokio::test]
async fn the_protocol_serves_an_installed_extension_and_nothing_else() {
    let h = harness("vsx-protocol");
    let fake = FakeOpenVsx::start().await;
    fake.add(theme());
    let dir = tempfile::tempdir().unwrap();
    let ext = Extensions::at(dir.path(), &fake.url);
    let ctx = context(&h.core, &[]);
    install(&ext, &h.core, &ctx, "acme.night", None, true)
        .await
        .unwrap();
    let ask = |method: &str, uri: &str| {
        let req = tauri::http::Request::builder()
            .method(method)
            .uri(uri)
            .body(Vec::new())
            .unwrap();
        protocol(Some(&h.core), Some(&ext), &req)
    };
    // Windows' form and the others'.
    for uri in [
        "http://escouade-ext.localhost/acme.night/themes/theme.json",
        "escouade-ext://localhost/acme.night/themes/theme.json",
    ] {
        let r = ask("GET", uri);
        assert_eq!(r.status(), 200, "{uri}");
        assert_eq!(
            r.headers()["content-type"],
            "application/json; charset=utf-8"
        );
    }
    assert_eq!(
        ask(
            "GET",
            "http://escouade-ext.localhost/acme.night/../installed.json"
        )
        .status(),
        403
    );
    assert_eq!(
        ask(
            "GET",
            "http://escouade-ext.localhost/acme.night/%2e%2e/installed.json"
        )
        .status(),
        403
    );
    assert_eq!(
        ask(
            "DELETE",
            "http://escouade-ext.localhost/acme.night/package.json"
        )
        .status(),
        405
    );
    assert_eq!(
        ask(
            "GET",
            "http://escouade-ext.localhost/other.ext/package.json"
        )
        .status(),
        404
    );
    // The page's listing and command go through the same guard.
    let list = files(&ext, &[], "acme.night").unwrap();
    assert!(list
        .iter()
        .any(|f| f.path == "themes/theme.json" && f.mime.starts_with("application/json")));
    assert!(escouade_vsx::serve::read(&ext.store, "acme.night", "../installed.json").is_err());
    assert_eq!(
        protocol(Some(&h.core), None, &tauri::http::Request::new(Vec::new())).status(),
        503
    );
}

#[test]
fn an_image_is_typed_by_its_bytes() {
    assert_eq!(image_type(b"\x89PNG\r\n\x1a\nxx"), Some("image/png"));
    assert_eq!(
        image_type(b"<?xml version='1.0'?><svg></svg>"),
        Some("image/svg+xml")
    );
    assert_eq!(image_type(b"<html><script>"), None);
}

// ---------- the policy judged by the core, an extension turned off not served (ESC-648) ----------

/// `p` made a project of the team `acme`, whose policy imposes `allowed`.
fn impose_by_team(core: &Core, p: &str, allowed: serde_json::Value) {
    let team = crate::locks::TeamPolicies {
        id: "acme".into(),
        team: "Acme".into(),
        signed: None,
        recommended: Default::default(),
        enforced: [(POLICY.to_owned(), allowed)].into_iter().collect(),
    };
    let locks = crate::locks::Locks::compute(crate::locks::KEYS, Some(&team), &Managed::default());
    core.team_locks.write().insert("acme".into(), locks);
    core.projects
        .write()
        .iter_mut()
        .find(|x| x.id == p)
        .unwrap()
        .team = Some("acme".into());
}

#[tokio::test]
async fn a_team_list_holds_whatever_projects_the_page_names() {
    let h = harness("vsx-team-any-view");
    let (team_project, _) = h.project(false).await;
    let own = h.second_project().await;
    impose_by_team(&h.core, &team_project.id, json!({ "*": false }));
    let fake = FakeOpenVsx::start().await;
    fake.add(theme());
    let dir = tempfile::tempdir().unwrap();
    let ext = Extensions::at(dir.path(), &fake.url);
    let id = ExtId::parse("acme.night").unwrap();
    // The page names no project, or one outside of the team: the team's list is still imposed.
    for projects in [Vec::new(), vec![own.id.clone()]] {
        let ctx = context(&h.core, &projects);
        assert!(
            ctx.layers
                .iter()
                .any(|l| l.imposed && l.by.contains("Acme")),
            "{projects:?}"
        );
        let r = install(&ext, &h.core, &ctx, "acme.night", None, true)
            .await
            .unwrap_err();
        assert_eq!(r.code, Code::NotAllowed, "{projects:?}");
        assert!(escouade_vsx::allowed::check(&ctx.layers, &id, None).is_err());
    }
    // Refused before Open VSX is asked for the archive.
    assert!(
        !fake.asked().iter().any(|a| a.path.contains(".vsix")),
        "{:?}",
        fake.asked()
    );
}

#[tokio::test]
async fn an_extension_turned_off_or_refused_is_no_longer_served() {
    let h = harness("vsx-served");
    let (p, _) = h.project(false).await;
    let fake = FakeOpenVsx::start().await;
    fake.add(theme());
    let dir = tempfile::tempdir().unwrap();
    let ext = Extensions::at(dir.path(), &fake.url);
    let ctx = context(&h.core, std::slice::from_ref(&p.id));
    install(&ext, &h.core, &ctx, "acme.night", None, true)
        .await
        .unwrap();
    let get = || {
        let req = tauri::http::Request::builder()
            .method("GET")
            .uri("http://escouade-ext.localhost/acme.night/package.json")
            .body(Vec::new())
            .unwrap();
        protocol(Some(&h.core), Some(&ext), &req).status()
    };
    assert_eq!(get(), 200);
    assert!(files(&ext, &served_layers(&h.core), "acme.night").is_ok());
    // Turned off: neither the protocol, nor the page's listing and file.
    ext.store.set_enabled("acme.night", false).unwrap();
    assert_eq!(get(), 403);
    assert_eq!(
        files(&ext, &served_layers(&h.core), "acme.night")
            .unwrap_err()
            .code,
        Code::Off
    );
    let r = escouade_vsx::serve::read_allowed(
        &ext.store,
        &served_layers(&h.core),
        "acme.night",
        "package.json",
    )
    .unwrap_err();
    assert_eq!(r.code, Code::Off);
    // On again, then refused by a team's list (of a project the page never names): not served either.
    ext.store.set_enabled("acme.night", true).unwrap();
    assert_eq!(get(), 200);
    impose_by_team(&h.core, &p.id, json!({ "acme": false, "*": true }));
    assert_eq!(get(), 403);
    let r = escouade_vsx::serve::read_allowed(
        &ext.store,
        &served_layers(&h.core),
        "acme.night",
        "package.json",
    )
    .unwrap_err();
    assert_eq!(r.code, Code::NotAllowed);
    assert!(files(&ext, &served_layers(&h.core), "acme.night").is_err());
}
