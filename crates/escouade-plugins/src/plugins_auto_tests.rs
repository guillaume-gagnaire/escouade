//! « Mettre à jour tout seuls » (ESC-700), against the fake store of the end-to-end tests: what installs by itself is
//! what the window for all of « Tout mettre à jour » would agree to (a version of the store, verified, that asks for
//! nothing more); each refusal leaves the update offered; a plugin whose harness is in a turn waits; an error is said
//! once and not tried again.

use crate::auto::{self, Held};
use crate::i18n::Lang;
use crate::paths::DataDir;
use crate::plugins_store_tests::{http, temp_dir, Fake};
use crate::store::Store;
use crate::{Origin, PluginView, Plugins, Refusal};
use escouade_model::model::PluginsAutoUpdate;
use parking_lot::Mutex;
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;

const AUBE: &str = "dev.exemple.aube";
const DISCORD: &str = "dev.escouade.discord";

async fn ready(fake: &Fake, name: &str) -> (Store, Arc<Plugins>) {
    let data = DataDir::new(temp_dir(name).join("data"));
    let store = fake.store(&data);
    store.refresh(&http(), Lang::Fr).await;
    let plugins = Plugins::load(data, "2.1.0");
    plugins.apply_store(store.revocations(), store.offers());
    (store, plugins)
}

async fn install(store: &Store, plugins: &Plugins, id: &str, version: Option<&str>) -> PluginView {
    let f = store
        .fetch(&http(), id, version, false, Lang::Fr)
        .await
        .unwrap();
    let p = plugins.inspect_store(f, Lang::Fr).unwrap();
    plugins.install(&p.token, Lang::Fr).unwrap()
}

/// A version held back published, then the updates looked for as « Chercher maintenant » does.
async fn release(fake: &Fake, store: &Store, plugins: &Plugins, id: &str, version: &str) {
    fake.post("/_fake/release", json!({ "id": id, "version": version }))
        .await;
    plugins
        .look_for_updates(store, Ok(http()), Lang::Fr, std::time::Duration::ZERO)
        .await;
}

struct Pass {
    tried: Mutex<HashSet<(String, String)>>,
}

impl Pass {
    fn new() -> Self {
        Self {
            tried: Mutex::new(HashSet::new()),
        }
    }

    async fn run(
        &self,
        store: &Store,
        plugins: &Arc<Plugins>,
        scope: PluginsAutoUpdate,
        busy: bool,
    ) -> auto::AutoReport {
        let p = plugins.clone();
        let install = move |token: &str| p.install(token, Lang::Fr);
        plugins
            .auto_update(
                store,
                &http(),
                scope,
                &move |_| busy,
                &self.tried,
                &install,
                Lang::Fr,
            )
            .await
    }
}

fn held_of(report: &auto::AutoReport, id: &str) -> Option<Held> {
    report
        .held
        .iter()
        .find(|(i, _)| i == id)
        .map(|(_, h)| h.clone())
}

#[tokio::test]
async fn a_verified_version_that_asks_for_nothing_more_installs_by_itself_and_says_it() {
    let fake = Fake::start().trusted();
    let (store, plugins) = ready(&fake, "auto-ok").await;
    install(&store, &plugins, AUBE, None).await;
    release(&fake, &store, &plugins, AUBE, "1.1.0").await;
    assert_eq!(
        plugins.get(AUBE, Lang::Fr).unwrap().update.unwrap().version,
        "1.1.0"
    );
    let report = Pass::new()
        .run(&store, &plugins, PluginsAutoUpdate::Verified, false)
        .await;
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    assert_eq!(report.installed.len(), 1);
    let (view, from) = &report.installed[0];
    assert_eq!((view.version.as_str(), from.as_str()), ("1.1.0", "1.0.0"));
    let now = plugins.get(AUBE, Lang::Fr).unwrap();
    assert_eq!(now.version, "1.1.0");
    assert!(now.verified, "its seal holds");
    assert!(now.update.is_none());
    let mark = now
        .choices
        .auto_updated
        .expect("the update by itself is noted");
    assert_eq!((mark.from.as_str(), mark.to.as_str()), ("1.0.0", "1.1.0"));
    // An install by a gesture takes the note off.
    let f = store
        .fetch(&http(), AUBE, Some("1.1.0"), false, Lang::Fr)
        .await
        .unwrap();
    let p = plugins.inspect_store(f, Lang::Fr).unwrap();
    let again = plugins.install(&p.token, Lang::Fr).unwrap();
    assert!(again.choices.auto_updated.is_none());
}

#[tokio::test]
async fn the_setting_off_installs_nothing() {
    let fake = Fake::start().trusted();
    let (store, plugins) = ready(&fake, "auto-off").await;
    install(&store, &plugins, AUBE, None).await;
    release(&fake, &store, &plugins, AUBE, "1.1.0").await;
    let report = Pass::new()
        .run(&store, &plugins, PluginsAutoUpdate::Never, false)
        .await;
    assert!(report.installed.is_empty());
    let now = plugins.get(AUBE, Lang::Fr).unwrap();
    assert_eq!(now.version, "1.0.0");
    assert_eq!(now.update.unwrap().version, "1.1.0", "still offered");
    assert_eq!(
        auto::offered(
            PluginsAutoUpdate::Never,
            &plugins.get(AUBE, Lang::Fr).unwrap(),
            None
        ),
        Err(Held::Off)
    );
}

#[tokio::test]
async fn escouade_s_own_alone_leaves_a_third_party_offered() {
    let fake = Fake::start().trusted();
    let (store, plugins) = ready(&fake, "auto-own").await;
    install(&store, &plugins, AUBE, None).await;
    install(&store, &plugins, DISCORD, None).await;
    fake.post("/_fake/release", json!({ "id": AUBE, "version": "1.1.0" }))
        .await;
    release(&fake, &store, &plugins, DISCORD, "1.2.0").await;
    let report = Pass::new()
        .run(&store, &plugins, PluginsAutoUpdate::Escouade, false)
        .await;
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    let ids: Vec<&str> = report
        .installed
        .iter()
        .map(|(v, _)| v.id.as_str())
        .collect();
    assert_eq!(ids, [DISCORD]);
    assert_eq!(held_of(&report, AUBE), Some(Held::ThirdParty));
    assert_eq!(plugins.get(DISCORD, Lang::Fr).unwrap().version, "1.2.0");
    let aube = plugins.get(AUBE, Lang::Fr).unwrap();
    assert_eq!(aube.version, "1.0.0");
    assert_eq!(aube.update.unwrap().version, "1.1.0", "still offered");
}

#[tokio::test]
async fn a_version_that_asks_for_more_stays_offered() {
    let fake = Fake::start().trusted();
    let (store, plugins) = ready(&fake, "auto-more").await;
    install(&store, &plugins, AUBE, None).await;
    fake.post("/_fake/release", json!({ "id": AUBE, "version": "1.1.0" }))
        .await;
    release(&fake, &store, &plugins, AUBE, "1.2.0").await;
    let report = Pass::new()
        .run(&store, &plugins, PluginsAutoUpdate::Verified, false)
        .await;
    assert!(report.installed.is_empty());
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    assert_eq!(held_of(&report, AUBE), Some(Held::AsksMore));
    let now = plugins.get(AUBE, Lang::Fr).unwrap();
    assert_eq!(now.version, "1.0.0");
    assert_eq!(now.update.unwrap().version, "1.2.0", "still offered");
}

#[tokio::test]
async fn a_version_not_verified_or_whose_seal_is_lost_stays_offered() {
    let fake = Fake::start().trusted();
    let (store, plugins) = ready(&fake, "auto-unverified").await;
    // Nuit 1.0.0 is verified, its 1.1.0 is not: the seal would be lost.
    install(&store, &plugins, "dev.exemple.nuit", None).await;
    release(&fake, &store, &plugins, "dev.exemple.nuit", "1.1.0").await;
    let report = Pass::new()
        .run(&store, &plugins, PluginsAutoUpdate::Verified, false)
        .await;
    assert!(report.installed.is_empty());
    assert_eq!(
        held_of(&report, "dev.exemple.nuit"),
        Some(Held::NotVerified)
    );
    let now = plugins.get("dev.exemple.nuit", Lang::Fr).unwrap();
    assert_eq!(now.version, "1.0.0");
    assert!(now.update.is_some(), "still offered");
}

#[tokio::test]
async fn a_version_the_revocation_list_takes_off_stays_offered() {
    let fake = Fake::start().trusted();
    let (store, plugins) = ready(&fake, "auto-revoked").await;
    install(&store, &plugins, AUBE, None).await;
    fake.post(
        "/_fake/revocations",
        json!({ "entries": [
            { "id": AUBE, "versions": ["1.1.0"], "action": "unverify", "reason": { "fr": "Relecture à refaire.", "en": "why" }, "date": "2026-11-27" },
        ] }),
    )
    .await;
    release(&fake, &store, &plugins, AUBE, "1.1.0").await;
    let report = Pass::new()
        .run(&store, &plugins, PluginsAutoUpdate::Verified, false)
        .await;
    assert!(report.installed.is_empty());
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    let why = held_of(&report, AUBE).unwrap();
    assert!(why == Held::Revoked || why == Held::NotVerified, "{why:?}");
    assert_eq!(plugins.get(AUBE, Lang::Fr).unwrap().version, "1.0.0");
}

#[tokio::test]
async fn a_plugin_whose_harness_is_in_a_turn_waits_for_its_end() {
    let fake = Fake::start().trusted();
    let (store, plugins) = ready(&fake, "auto-turn").await;
    install(&store, &plugins, AUBE, None).await;
    release(&fake, &store, &plugins, AUBE, "1.1.0").await;
    let pass = Pass::new();
    let report = pass
        .run(&store, &plugins, PluginsAutoUpdate::Verified, true)
        .await;
    assert_eq!(report.waiting, [AUBE]);
    assert!(report.installed.is_empty());
    assert_eq!(plugins.get(AUBE, Lang::Fr).unwrap().version, "1.0.0");
    // The turn over: the same pass installs it, it was not counted as tried.
    let report = pass
        .run(&store, &plugins, PluginsAutoUpdate::Verified, false)
        .await;
    assert_eq!(report.installed.len(), 1);
    assert_eq!(plugins.get(AUBE, Lang::Fr).unwrap().version, "1.1.0");
}

#[tokio::test]
async fn an_error_leaves_the_version_installed_is_said_once_and_is_not_tried_again() {
    let fake = Fake::start().trusted();
    let (store, plugins) = ready(&fake, "auto-error").await;
    install(&store, &plugins, AUBE, None).await;
    release(&fake, &store, &plugins, AUBE, "1.1.0").await;
    let pass = Pass::new();
    let p = plugins.clone();
    let failing = move |token: &str| -> Result<PluginView, Refusal> {
        p.discard(token);
        Err(Refusal::other("le dossier est pris".to_string()))
    };
    let report = plugins
        .auto_update(
            &store,
            &http(),
            PluginsAutoUpdate::Verified,
            &|_| false,
            &pass.tried,
            &failing,
            Lang::Fr,
        )
        .await;
    assert_eq!(report.failed.len(), 1);
    let f = &report.failed[0];
    assert_eq!((f.id.as_str(), f.version.as_str()), (AUBE, "1.1.0"));
    assert!(f.message.contains("le dossier est pris"));
    let now = plugins.get(AUBE, Lang::Fr).unwrap();
    assert_eq!(now.version, "1.0.0", "the version installed stays");
    assert!(now.update.is_some(), "still offered, for a gesture");
    // The next pass (6 hours later, « Chercher maintenant ») does not try it again: no loop.
    let report = pass
        .run(&store, &plugins, PluginsAutoUpdate::Verified, false)
        .await;
    assert!(report.installed.is_empty() && report.failed.is_empty());
    assert_eq!(plugins.get(AUBE, Lang::Fr).unwrap().version, "1.0.0");
    // A store that does not answer: an error of its own, said once too.
    let fake2 = Fake::start().trusted();
    let (store2, plugins2) = ready(&fake2, "auto-down").await;
    install(&store2, &plugins2, AUBE, None).await;
    release(&fake2, &store2, &plugins2, AUBE, "1.1.0").await;
    fake2.post("/_fake/down", json!({ "down": true })).await;
    let pass2 = Pass::new();
    let report = pass2
        .run(&store2, &plugins2, PluginsAutoUpdate::Verified, false)
        .await;
    assert_eq!(report.failed.len(), 1);
    let report = pass2
        .run(&store2, &plugins2, PluginsAutoUpdate::Verified, false)
        .await;
    assert!(report.failed.is_empty());
    assert_eq!(plugins2.get(AUBE, Lang::Fr).unwrap().version, "1.0.0");
}

#[tokio::test]
async fn each_refusal_of_the_choice() {
    let fake = Fake::start().trusted();
    let (store, plugins) = ready(&fake, "auto-choice").await;
    install(&store, &plugins, AUBE, None).await;
    release(&fake, &store, &plugins, AUBE, "1.1.0").await;
    let installed = plugins.get(AUBE, Lang::Fr).unwrap();
    let all = PluginsAutoUpdate::Verified;
    assert_eq!(auto::offered(all, &installed, None), Ok(()));

    // Before anything is read: the plugins of a git repository, a folder, a folder in development, never.
    for origin in [
        Origin::Git {
            url: "https://example.com/aube.git".into(),
            tag: "v1.0.0".into(),
        },
        Origin::Folder {
            path: "C:/aube".into(),
        },
        Origin::Dev {
            path: "C:/aube".into(),
        },
    ] {
        let mut p = installed.clone();
        p.origin = origin;
        assert_eq!(auto::offered(all, &p, None), Err(Held::NotStore));
    }
    let mut p = installed.clone();
    p.update.as_mut().unwrap().verified = Some(false);
    assert_eq!(auto::offered(all, &p, None), Err(Held::NotVerified));
    let mut p = installed.clone();
    p.update.as_mut().unwrap().seed = true;
    assert_eq!(auto::offered(all, &p, None), Err(Held::Seed));
    assert_eq!(
        auto::offered(PluginsAutoUpdate::Escouade, &installed, None),
        Err(Held::ThirdParty)
    );

    // The version read: what the window for all would hold, and each thing that takes it out.
    let f = store
        .fetch(&http(), AUBE, Some("1.1.0"), true, Lang::Fr)
        .await
        .unwrap();
    let preview = plugins.inspect_store(f, Lang::Fr).unwrap();
    assert_eq!(auto::agreed(&preview, &installed), Ok(()));
    let mut p = preview.clone();
    p.verified = false;
    assert_eq!(auto::agreed(&p, &installed), Err(Held::NotVerified));
    let mut p = preview.clone();
    p.unverified = Some("Relecture à refaire.".into());
    assert_eq!(auto::agreed(&p, &installed), Err(Held::NotVerified));
    let mut p = preview.clone();
    p.grown = vec!["network:aube.example".into()];
    assert_eq!(auto::agreed(&p, &installed), Err(Held::AsksMore));
    // The line of a harness changed (its program, arguments or variables): asks for more.
    let mut p = preview.clone();
    p.grown = vec!["harness:aube --yolo".into()];
    assert_eq!(auto::agreed(&p, &installed), Err(Held::HarnessLine));
    let mut p = preview.clone();
    p.refusal = Some("API 9".into());
    assert_eq!(auto::agreed(&p, &installed), Err(Held::Refused));
    let mut p = preview.clone();
    p.origin = Origin::Folder {
        path: "C:/aube".into(),
    };
    assert_eq!(auto::agreed(&p, &installed), Err(Held::NotStore));
    // A contribution more (a second theme), or no manifest installed to compare with.
    let mut p = preview.clone();
    let themes = p.manifest.contributes["themes"].as_array().unwrap().clone();
    let mut second = themes[0].clone();
    second["id"] = json!("aube-2");
    p.manifest.contributes["themes"] = json!([themes[0].clone(), second]);
    assert_eq!(auto::agreed(&p, &installed), Err(Held::Contributions));
    // Its colours changed alone: nothing more.
    let mut p = preview.clone();
    let mut theme = themes[0].clone();
    theme["dark"]["--text"] = json!("#eeeeee");
    p.manifest.contributes["themes"] = json!([theme]);
    assert_eq!(auto::agreed(&p, &installed), Ok(()));
    let mut bare = installed.clone();
    bare.manifest = None;
    assert_eq!(auto::agreed(&preview, &bare), Err(Held::Unknown));
    plugins.discard(&preview.token);
}

#[tokio::test]
async fn a_plugin_serves_the_harness_it_is_the_plugin_of_or_describes() {
    let fake = Fake::start().trusted();
    let (store, plugins) = ready(&fake, "auto-serves").await;
    let mut p = install(&store, &plugins, AUBE, None).await;
    assert!(!auto::serves(&p, &["claude".into()]));
    p.id = crate::builtin::CLAUDE.into();
    assert!(auto::serves(&p, &["claude".into()]));
    assert!(!auto::serves(&p, &["codex".into()]));
    // A harness its manifest describes, by its id.
    let mut d = install(&store, &plugins, AUBE, None).await;
    d.manifest.as_mut().unwrap().contributes.insert(
        "harnesses".into(),
        json!([{ "id": "aube-cli", "name": "Aube CLI", "locate": { "names": ["aube"] } }]),
    );
    assert!(auto::serves(&d, &["aube-cli".into()]));
    assert!(!auto::serves(&d, &[]));
}
