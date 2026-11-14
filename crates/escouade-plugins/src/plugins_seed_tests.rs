//! Tests of the app's seed (ESC-338, docs/PLUGINS-DESIGN.md §8.7), on what the fake store published, its bytes and its signatures:
//! installed with no network and by the store's path, a newer version offered and never installed by itself, never an
//! older one, a plugin the user uninstalled left out and proposed, a false signature or a changed file refused, a version
//! withdrawn stopped with its reason, the range of Escouade, a build of development without signatures, and the passage of
//! an id from the origin `builtin` to the seed.

use crate::i18n::Lang;
use crate::paths::DataDir;
use crate::plugins_store_tests::{http, temp_dir, Fake};
use crate::seed::{self, Entry, Seed};
use crate::store::{self, Store};
use crate::wasm::{Made, Modules};
use crate::{builtin, Origin, Plugins, State};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const TRELLO: &str = "dev.escouade.trello";

/// A seed of the versions `plugins`, as the release makes it from what the store published.
async fn seed_of(fake: &Fake, name: &str, plugins: &[(&str, &str)]) -> Seed {
    let dir = temp_dir(name);
    let list: Vec<Value> = plugins
        .iter()
        .map(|(id, version)| json!({ "id": id, "version": version }))
        .collect();
    let answer = fake
        .post("/_fake/seed", json!({ "dir": dir, "plugins": list }))
        .await;
    let entries: Vec<Entry> = serde_json::from_value(answer["plugins"].clone()).unwrap();
    Seed {
        dir: Some(dir),
        entries,
        unsigned: false,
        counts: true,
    }
}

fn data_of(name: &str) -> DataDir {
    let data = DataDir::new(temp_dir(name).join("data"));
    data.ensure().unwrap();
    data
}

/// The app started on `data` with `seed`, at Escouade 2.2.0, as `core` does it: offline, no list but the seed's.
fn started(data: &DataDir, seed: Seed) -> std::sync::Arc<Plugins> {
    let list = seed.revocations();
    let plugins = Plugins::load_seeded(data.clone(), "2.2.0", seed);
    plugins.apply_store(list, Default::default());
    plugins.sync_seed(Lang::Fr, &|_| true);
    plugins
}

fn requests(state: &Value) -> usize {
    let all = state["requests"].as_array().unwrap();
    all.iter()
        .filter(|r| !r.as_str().unwrap().contains("/_fake/"))
        .count()
}

#[tokio::test]
async fn the_seed_installs_with_no_network_then_a_newer_version_is_offered_and_not_installed() {
    let fake = Fake::start().trusted();
    let seed = seed_of(&fake, "first", &[(TRELLO, "2.2.0")]).await;
    let before = requests(&fake.state().await);

    let data = data_of("first");
    let plugins = started(&data, seed.clone());
    assert_eq!(
        requests(&fake.state().await),
        before,
        "no request: the seed reads its files"
    );

    let view = plugins.get(TRELLO, Lang::Fr).expect("installed");
    assert_eq!(view.version, "2.2.0");
    assert_eq!(view.state, State::Ready, "{:?}", view.state);
    assert!(view.verified, "its seal holds with the app's keys");
    assert!(matches!(
        &view.origin,
        Origin::Store { version, verified: true, seed: true, path, .. }
            if version == "2.2.0" && path == "plugins/trello"
    ));
    assert_eq!(
        view.fingerprint,
        format!("store:{}", seed.entries[0].sha256)
    );
    assert!(
        Path::new(&view.folder).join("plugin.wasm").is_file(),
        "the store's module is in its folder"
    );
    assert!(view.update.is_none());

    // The store comes back: 2.3.0 is offered, verified; nothing changes by itself.
    let store = fake.store(&data);
    store.refresh(&http(), Lang::Fr).await;
    plugins.apply_store(store.revocations(), store.offers());
    let view = plugins.get(TRELLO, Lang::Fr).unwrap();
    assert_eq!(view.version, "2.2.0", "offered, not installed");
    let update = view.update.expect("offered");
    assert_eq!(update.version, "2.3.0");
    assert_eq!(update.verified, Some(true));
    assert!(!update.seed, "it comes from the store");

    // And a start offline changes nothing: the plugin stays, its files as they are, once and for all.
    drop(plugins);
    let again = started(&data, seed);
    let view = again.get(TRELLO, Lang::Fr).unwrap();
    assert_eq!(view.version, "2.2.0");
    assert_eq!(view.state, State::Ready);
}

#[tokio::test]
async fn a_new_app_s_seed_is_offered_never_installed_by_itself_and_a_version_never_goes_back() {
    let fake = Fake::start().trusted();
    let old = seed_of(&fake, "old", &[(TRELLO, "2.2.0")]).await;
    let new = seed_of(&fake, "new", &[(TRELLO, "2.3.0")]).await;
    let data = data_of("update");
    drop(started(&data, old.clone()));

    // The app was updated: its seed holds 2.3.0, offered, with no network.
    let plugins = started(&data, new.clone());
    let view = plugins.get(TRELLO, Lang::Fr).unwrap();
    assert_eq!(view.version, "2.2.0", "never installed by itself");
    let update = view.update.expect("offered");
    assert_eq!(update.version, "2.3.0");
    assert!(update.seed);

    // The user's click: the usual window, then the install.
    plugins
        .choose(TRELLO, |c| {
            c.settings
                .insert("kept".into(), json!(true))
                .map(drop)
                .unwrap_or(())
        })
        .unwrap();
    let preview = plugins.inspect_seed(TRELLO, Lang::Fr).expect("read");
    assert_eq!(preview.previous.as_ref().unwrap().version, "2.2.0");
    let view = plugins.install(&preview.token, Lang::Fr).unwrap();
    assert_eq!(view.version, "2.3.0");
    assert!(
        view.choices.settings.contains_key("kept"),
        "its choices kept"
    );
    assert!(view.update.is_none());
    drop(plugins);

    // An older app's seed changes nothing: no way down, by itself or by a click.
    let plugins = started(&data, old);
    let view = plugins.get(TRELLO, Lang::Fr).unwrap();
    assert_eq!(view.version, "2.3.0");
    assert!(view.update.is_none());
    let e = plugins.inspect_seed(TRELLO, Lang::Fr).unwrap_err();
    assert!(e.message.contains("on ne redescend pas"), "{e}");
}

#[tokio::test]
async fn the_newest_of_the_store_and_the_seed_is_offered() {
    let fake = Fake::start().trusted();
    let old = seed_of(&fake, "old", &[(TRELLO, "2.2.0")]).await;
    let new = seed_of(&fake, "new", &[(TRELLO, "2.3.0")]).await;
    let data = data_of("newest");
    drop(started(&data, old));
    let plugins = started(&data, new);
    let offer = |version: &str| {
        [(
            TRELLO.to_string(),
            store::Offer {
                version: version.to_string(),
                verified: true,
            },
        )]
        .into_iter()
        .collect()
    };
    plugins.apply_store(None, offer("2.2.5"));
    let u = plugins.get(TRELLO, Lang::Fr).unwrap().update.unwrap();
    assert_eq!((u.version.as_str(), u.seed), ("2.3.0", true));
    plugins.apply_store(None, offer("2.4.0"));
    let u = plugins.get(TRELLO, Lang::Fr).unwrap().update.unwrap();
    assert_eq!((u.version.as_str(), u.seed), ("2.4.0", false));
}

#[tokio::test]
async fn a_plugin_the_user_uninstalled_is_noted_proposed_and_not_put_back() {
    let fake = Fake::start().trusted();
    let seed = seed_of(&fake, "removed", &[(TRELLO, "2.2.0")]).await;
    let data = data_of("removed");
    let plugins = started(&data, seed.clone());
    assert!(
        plugins.seed_offers(Lang::Fr).is_empty(),
        "nothing to propose"
    );
    plugins.choose(TRELLO, |c| c.enabled = false).unwrap();

    // It switches off and uninstalls like any other, its choices kept.
    assert!(plugins.remove(TRELLO, true).unwrap());
    assert!(plugins.get(TRELLO, Lang::Fr).is_none());
    assert!(plugins.uninstalled(TRELLO));
    drop(plugins);

    let plugins = started(&data, seed);
    assert!(plugins.get(TRELLO, Lang::Fr).is_none(), "not put back");
    let offers = plugins.seed_offers(Lang::Fr);
    assert_eq!(
        offers,
        vec![seed::Offer {
            id: TRELLO.into(),
            name: "Trello".into(),
            version: "2.2.0".into(),
            removed: Some("2.2.0".into()),
            refused: None,
        }]
    );

    // On the user's click it comes back, their choices with it, and the note goes.
    let preview = plugins.inspect_seed(TRELLO, Lang::Fr).unwrap();
    let view = plugins.install(&preview.token, Lang::Fr).unwrap();
    assert!(!view.choices.enabled, "the choices an uninstall kept");
    assert!(plugins.seed_offers(Lang::Fr).is_empty());
    assert!(!plugins.uninstalled(TRELLO));
}

#[tokio::test]
async fn a_false_signature_or_a_changed_file_is_refused_and_says_why() {
    let fake = Fake::start().trusted();
    let seed = seed_of(
        &fake,
        "false",
        &[(TRELLO, "2.2.0"), ("dev.escouade.discord", "1.1.0")],
    )
    .await;
    let dir = seed.dir.clone().unwrap();
    // The signature of another plugin.
    std::fs::copy(
        dir.join("dev.escouade.discord/version.json.minisig"),
        dir.join(TRELLO).join("version.json.minisig"),
    )
    .unwrap();
    let data = data_of("false");
    let plugins = started(&data, seed.clone());
    assert!(plugins.get(TRELLO, Lang::Fr).is_none(), "nothing installed");
    assert!(
        plugins.get("dev.escouade.discord", Lang::Fr).is_some(),
        "the others are not held back"
    );
    let offers = plugins.seed_offers(Lang::Fr);
    assert_eq!(offers.len(), 1);
    assert!(
        offers[0].refused.as_deref().unwrap().contains("signature"),
        "{offers:?}"
    );
    assert!(!data.plugins_dir().join(TRELLO).exists());

    // version.json that is not the fingerprint the app embeds.
    let seed = seed_of(&fake, "changed", &[(TRELLO, "2.2.0")]).await;
    let dir = seed.dir.clone().unwrap().join(TRELLO);
    let json = std::fs::read_to_string(dir.join("version.json")).unwrap();
    std::fs::write(dir.join("version.json"), format!("{json} ")).unwrap();
    let plugins = started(&data_of("changed"), seed);
    assert!(plugins.get(TRELLO, Lang::Fr).is_none());
    assert!(plugins.seed_offers(Lang::Fr)[0]
        .refused
        .as_deref()
        .unwrap()
        .contains("version.json"));

    // An archive or a module changed after version.json.
    for (name, says) in [
        ("archive.tar.gz", "archive"),
        ("plugin.wasm", "module WASM"),
    ] {
        let seed = seed_of(&fake, "file", &[(TRELLO, "2.2.0")]).await;
        std::fs::write(seed.dir.clone().unwrap().join(TRELLO).join(name), "changed").unwrap();
        let plugins = started(&data_of("file"), seed);
        assert!(plugins.get(TRELLO, Lang::Fr).is_none());
        let refused = plugins.seed_offers(Lang::Fr)[0].refused.clone().unwrap();
        assert!(refused.contains(says), "{refused}");
    }

    // Nothing in the app's folder: the seed cannot do anything, and says so.
    let seed = Seed {
        dir: Some(temp_dir("empty")),
        ..seed_of(&fake, "listed", &[(TRELLO, "2.2.0")]).await
    };
    let plugins = started(&data_of("empty"), seed);
    assert!(plugins.get(TRELLO, Lang::Fr).is_none());
    assert!(plugins.seed_offers(Lang::Fr)[0]
        .refused
        .as_deref()
        .unwrap()
        .contains("amorce"));
}

#[tokio::test]
async fn a_release_refuses_what_is_not_signed_and_a_build_of_development_takes_it() {
    let fake = Fake::start().trusted();
    let mut seed = seed_of(&fake, "unsigned", &[(TRELLO, "2.2.0")]).await;
    std::fs::remove_file(
        seed.dir
            .clone()
            .unwrap()
            .join(TRELLO)
            .join("version.json.minisig"),
    )
    .unwrap();

    // A release.
    let plugins = started(&data_of("release"), seed.clone());
    assert!(plugins.get(TRELLO, Lang::Fr).is_none());
    assert!(plugins.seed_offers(Lang::Fr)[0]
        .refused
        .as_deref()
        .unwrap()
        .contains("signé"));

    // A build of development (`debug_assertions`): its module of scripts/seed-dev.sh.
    seed.unsigned = true;
    let plugins = started(&data_of("dev"), seed);
    let view = plugins.get(TRELLO, Lang::Fr).expect("installed");
    assert_eq!(view.state, State::Ready, "{:?}", view.state);
    assert!(!view.verified, "it shows no seal");
    assert!(matches!(
        view.origin,
        Origin::Store {
            verified: false,
            seed: true,
            ..
        }
    ));
}

#[tokio::test]
async fn a_version_of_the_seed_withdrawn_is_stopped_with_its_reason() {
    let fake = Fake::start().trusted();
    fake.post(
        "/_fake/revocations",
        json!({ "entries": [{
            "id": TRELLO,
            "versions": ["2.2.0"],
            "action": "disable",
            "reason": { "fr": "Il fuit les jetons.", "en": "It leaks tokens." },
            "date": "2026-11-05",
        }] }),
    )
    .await;
    let seed = seed_of(&fake, "revoked", &[(TRELLO, "2.2.0")]).await;
    assert_eq!(
        seed.revocations().unwrap().serial,
        1,
        "the list of the seed"
    );

    // Nothing from the store: the seed's list holds, and the plugin is there, stopped, its reason said.
    let data = data_of("revoked");
    let plugins = started(&data, seed);
    let view = plugins.get(TRELLO, Lang::Fr).expect("there, stopped");
    match &view.state {
        State::Disabled { reason } => assert!(reason.contains("Il fuit les jetons."), "{reason}"),
        s => panic!("not stopped: {s:?}"),
    }
    assert_eq!(view.withdrawn.unwrap().action, "disable");
    let rt = crate::runtime::Runtime::new(
        plugins.clone(),
        std::sync::Arc::new(crate::secrets::MemoryStore::default()),
        &data,
        "2.2.0",
        crate::runtime::Limits::default(),
    );
    assert!(
        rt.call(TRELLO, "x", json!({})).await.is_err(),
        "nothing runs"
    );
    // It does not install on a click either.
    plugins.remove(TRELLO, false).unwrap();
    let e = plugins.inspect_seed(TRELLO, Lang::Fr).unwrap_err();
    assert!(e.message.contains("retirée par Escouade"), "{e}");
}

#[tokio::test]
async fn a_version_of_the_seed_whose_seal_was_taken_back_does_not_install() {
    let fake = Fake::start().trusted();
    fake.post(
        "/_fake/revocations",
        json!({ "entries": [{
            "id": TRELLO,
            "versions": "*",
            "action": "unverify",
            "reason": { "fr": "Relecture à reprendre.", "en": "Review again." },
            "date": "2026-11-05",
        }] }),
    )
    .await;
    let seed = seed_of(&fake, "unverify", &[(TRELLO, "2.2.0")]).await;
    let plugins = started(&data_of("unverify"), seed);
    assert!(plugins.get(TRELLO, Lang::Fr).is_none());
    let refused = plugins.seed_offers(Lang::Fr)[0].refused.clone().unwrap();
    assert!(refused.contains("Relecture à reprendre."), "{refused}");
}

#[tokio::test]
async fn the_store_s_list_holds_when_it_is_the_newest_the_seed_s_when_it_is() {
    let fake = Fake::start().trusted();
    let entries = |id: &str| {
        json!({ "entries": [{
            "id": id, "versions": "*", "action": "disable",
            "reason": { "fr": "non", "en": "no" }, "date": "2026-11-05",
        }] })
    };
    fake.post("/_fake/revocations", entries("dev.exemple.un"))
        .await;
    let data = data_of("lists");
    let store = fake.store(&data);
    store.refresh(&http(), Lang::Fr).await;
    assert_eq!(store.revocations().unwrap().serial, 1);

    // The seed of a newer app carries no. 2: it holds, offline, over what was kept.
    fake.post("/_fake/revocations", entries("dev.exemple.deux"))
        .await;
    let seed = seed_of(&fake, "lists", &[(TRELLO, "2.2.0")]).await;
    let store = Store::load(&data, fake.url.clone(), fake.url.clone(), "2.2.0")
        .with_seed_list(seed.revocations());
    assert_eq!(store.revocations().unwrap().serial, 2);
    assert_eq!(
        store.view().revocations.unwrap().from.as_deref(),
        Some("seed")
    );

    // The store's no. 3, read since: an older seed changes nothing.
    fake.post("/_fake/revocations", entries("dev.exemple.trois"))
        .await;
    let store = fake.store(&data);
    store.refresh(&http(), Lang::Fr).await;
    let store = Store::load(&data, fake.url.clone(), fake.url.clone(), "2.2.0")
        .with_seed_list(seed.revocations());
    assert_eq!(store.revocations().unwrap().serial, 3);

    // A list of the seed that was changed is not taken.
    let dir = seed.dir.clone().unwrap();
    std::fs::write(dir.join("revocations.json"), "{\"serial\":9}").unwrap();
    assert!(seed.revocations().is_none());
}

#[tokio::test]
async fn a_version_for_another_escouade_is_refused() {
    let fake = Fake::start().trusted();
    let seed = seed_of(&fake, "range", &[("dev.escouade.seed-later", "1.0.0")]).await;
    let plugins = started(&data_of("range"), seed);
    assert!(plugins.get("dev.escouade.seed-later", Lang::Fr).is_none());
    let refused = plugins.seed_offers(Lang::Fr)[0].refused.clone().unwrap();
    assert!(refused.contains("99.0.0"), "{refused}");
}

#[tokio::test]
async fn a_module_of_the_seed_is_precompiled_at_its_activation_not_at_the_start() {
    let fake = Fake::start().trusted();
    let seed = seed_of(&fake, "precompile", &[(TRELLO, "2.2.0")]).await;
    let data = data_of("precompile");
    let plugins = started(&data, seed);
    let rt = crate::runtime::Runtime::new(
        plugins.clone(),
        std::sync::Arc::new(crate::secrets::MemoryStore::default()),
        &data,
        "2.2.0",
        crate::runtime::Limits::default(),
    );
    rt.prepare_all();
    let folder = PathBuf::from(plugins.get(TRELLO, Lang::Fr).unwrap().folder);
    let (_, made) = Modules::new(&data, "2.2.0")
        .prepare_reactor(&folder)
        .expect("precompiled");
    assert_eq!(made, Made::Precompiled, "the start did not do it");
}

/// The list a user of 2.1 has (`plugins.json`): each of `ids` given by the app, its origin `builtin` (gone with ESC-355),
/// those of `off` switched off, and `others` beside them as they are.
pub fn list_of_2_1(data: &DataDir, ids: &[&str], off: &[&str], others: Vec<Value>) {
    let mut plugins: Vec<Value> = ids
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "origin": { "kind": "builtin" },
                "fingerprint": "builtin:2.1.4",
                "digest": "",
                "installedAt": "2026-10-20T10:00:00Z",
                "enabled": !off.contains(id),
                "projects": {},
                "revoked": [],
                "settings": {},
            })
        })
        .collect();
    plugins.extend(others);
    std::fs::create_dir_all(data.root()).unwrap();
    std::fs::write(
        data.plugins_file(),
        serde_json::to_vec_pretty(&json!({ "version": 1, "plugins": plugins })).unwrap(),
    )
    .unwrap();
}

/// A plugin 2.1 gave (its origin `builtin`) is installed from the seed with the user's choices, a plugin like any other,
/// which uninstalls; no plugin takes its place by its id, and the list keeps nothing of the origin `builtin`.
#[tokio::test]
async fn a_plugin_2_1_gave_is_installed_from_the_seed_with_its_choices_and_uninstalls() {
    let fake = Fake::start().trusted();
    let data = data_of("passage");
    list_of_2_1(&data, &[TRELLO, builtin::GITHUB], &[TRELLO], Vec::new());

    let seed = seed_of(&fake, "passage", &[(TRELLO, "2.2.0")]).await;
    let plugins = started(&data, seed.clone());
    let view = plugins.get(TRELLO, Lang::Fr).unwrap();
    assert!(matches!(view.origin, Origin::Store { seed: true, .. }));
    assert!(!view.choices.enabled, "the user's choice");
    let list = std::fs::read_to_string(data.plugins_file()).unwrap();
    assert!(!list.contains("\"builtin\""), "{list}");

    // It goes as any other's.
    assert!(plugins.remove(TRELLO, false).unwrap());
    drop(plugins);
    let plugins = started(&data, seed);
    assert!(
        plugins.get(TRELLO, Lang::Fr).is_none(),
        "nothing puts it back"
    );
    assert_eq!(plugins.seed_offers(Lang::Fr).len(), 1);
}

/// A plugin 2.1 gave that the seed cannot give (its files refused) is not run from the app: the view says why, and
/// proposes it; its choices wait for its install.
#[tokio::test]
async fn a_plugin_2_1_gave_that_the_seed_cannot_give_waits_with_its_choices() {
    let fake = Fake::start().trusted();
    let data = data_of("stays");
    list_of_2_1(&data, &[TRELLO], &[TRELLO], Vec::new());
    let seed = Seed {
        dir: Some(temp_dir("stays")),
        ..seed_of(&fake, "listed", &[(TRELLO, "2.2.0")]).await
    };
    let plugins = started(&data, seed);
    assert!(plugins.get(TRELLO, Lang::Fr).is_none(), "not installed");
    let offer = plugins
        .seed_offers(Lang::Fr)
        .into_iter()
        .find(|o| o.id == TRELLO)
        .unwrap();
    assert!(offer.refused.is_some(), "{offer:?}");
    assert!(plugins.choices(TRELLO).is_none());
    let kept = data
        .plugins_dir()
        .join(".kept")
        .join(format!("{TRELLO}.json"));
    let kept: Value = serde_json::from_slice(&std::fs::read(kept).unwrap()).unwrap();
    assert_eq!(kept["enabled"], false, "{kept}");
}

/// A build of development runs the workspace's modules of the plugins the seed has not the files of; what is installed
/// of an id stays (no plugin wins by its id), and a module of the workspace is not removed.
#[test]
fn the_workspace_fills_what_is_not_installed_and_wins_nothing() {
    let data = data_of("workspace");
    let plugins = Plugins::load_for_core(data.clone(), "2.2.0");
    assert_eq!(
        plugins.get(builtin::GITHUB, Lang::Fr).unwrap().origin,
        Origin::Workspace
    );
    let e = plugins.remove(builtin::GITHUB, false).unwrap_err();
    assert!(e.to_string().contains("workspace"), "{e}");
    // A folder in development with an id of the registry is of anyone's hand (audit SEC-27): refused, the workspace's
    // keeps its place.
    let src = temp_dir("workspace-dev");
    std::fs::write(
        src.join(crate::manifest::FILE),
        json!({
            "id": builtin::GITHUB,
            "name": "GitHub du développeur",
            "version": "9.0.0",
            "escouade": { "api": "1", "min": "2.1.0" },
            "contributes": { "themes": [{ "id": "mine" }] }
        })
        .to_string(),
    )
    .unwrap();
    std::fs::create_dir_all(src.join("themes")).unwrap();
    std::fs::write(src.join("themes/mine.json"), "{}").unwrap();
    assert!(plugins.load_dev(&src, Lang::Fr).is_err());
    drop(plugins);
    let plugins = Plugins::load_for_core(data, "2.2.0");
    let v = plugins.get(builtin::GITHUB, Lang::Fr).unwrap();
    assert_eq!(v.origin, Origin::Workspace, "{:?}", v.origin);
    assert_ne!(v.version, "9.0.0");
}

#[test]
fn the_committed_list_reads_and_names_ids_of_the_registry() {
    let list = std::fs::read_to_string(
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../src-tauri"))
            .join("../plugins/seed.json"),
    )
    .unwrap();
    for e in seed::parse(&list).expect("plugins/seed.json reads") {
        assert!(builtin::seeds(&e.id), "{} is not an id of the seed", e.id);
        assert_eq!(e.sha256.len(), 64, "{}", e.id);
    }
    assert!(
        !builtin::seeds("dev.escouade.minimal"),
        "the demonstration is no product"
    );
    assert!(builtin::seeds(builtin::GITHUB));
    assert!(seed::parse("{\"format\":2,\"plugins\":[]}").is_err());
}

#[test]
fn a_seed_with_nothing_changes_nothing() {
    let seed = Seed::of_app(None);
    assert!(!seed.has(TRELLO));
    assert!(seed.revocations().is_none());
    let plugins = started(&data_of("nothing"), Seed::default());
    assert!(plugins.seed_offers(Lang::Fr).is_empty());
}

#[test]
fn the_committed_list_holds_the_first_hand_the_store_signed() {
    let list = std::fs::read_to_string(
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../src-tauri"))
            .join("../plugins/seed.json"),
    )
    .unwrap();
    let ids: Vec<String> = seed::parse(&list)
        .unwrap()
        .into_iter()
        .map(|e| e.id)
        .collect();
    // Z2 (ESC-354): the harnesses, the sources and code hosts, the voices; Goose and those on request beside the registry.
    for id in [
        builtin::CLAUDE,
        builtin::CODEX,
        builtin::TRELLO,
        builtin::GITHUB,
        builtin::ACP,
        crate::sort::GOOSE,
        "dev.escouade.discord",
        "dev.escouade.team-rules",
    ] {
        assert!(ids.iter().any(|i| i == id), "{id} is in the seed");
        assert!(builtin::seeds(id), "{id} may be in the seed");
    }
    assert!(!builtin::seeds("dev.escouade.une-autre"));
}

#[test]
fn a_build_of_development_without_the_seeds_files_runs_the_registrys_plugins() {
    // The list names Trello; a folder without its files leaves it to the registry, as before the seed (ESC-354).
    let empty = temp_dir("dev-no-files");
    let seed = Seed::of_app(Some(empty.clone()));
    assert!(!seed.has(TRELLO));
    let plugins = started(&data_of("dev-no-files-data"), seed);
    assert_eq!(
        plugins.get(TRELLO, Lang::Fr).unwrap().origin,
        Origin::Workspace
    );
    // Its files there: the seed takes it.
    std::fs::create_dir_all(empty.join(TRELLO)).unwrap();
    assert!(Seed::of_app(Some(empty)).has(TRELLO));
}

// The store's count of the seed's installs (ESC-394, docs/STORE.md §5.4): an install from the seed counts as an install,
// with the user's consent to the statistics, once per version, kept while the store does not answer.

fn installs(state: &Value) -> Vec<Value> {
    state["installs"].as_array().unwrap().clone()
}

fn counted(id: &str, version: &str) -> Value {
    json!({ "id": id, "version": version, "event": "install" })
}

fn pending(plugins: &Plugins) -> Vec<(String, String)> {
    plugins
        .seed_counts_pending()
        .into_iter()
        .map(|c| (c.id, c.version))
        .collect()
}

#[tokio::test]
async fn an_install_from_the_seed_waits_for_the_consent_then_counts_once() {
    let fake = Fake::start().trusted();
    let seed = seed_of(&fake, "count", &[(TRELLO, "2.2.0")]).await;
    let data = data_of("count");
    let store = fake.store(&data);
    let plugins = started(&data, seed.clone());
    assert_eq!(
        pending(&plugins),
        [(TRELLO.to_string(), "2.2.0".to_string())]
    );

    // Not answered yet on the statistics: nothing is sent, it waits.
    plugins.send_seed_counts(&store, &http(), None).await;
    assert!(installs(&fake.state().await).is_empty());
    assert_eq!(pending(&plugins).len(), 1);

    // A yes: sent, with its id, its version and `install` alone.
    plugins.send_seed_counts(&store, &http(), Some(true)).await;
    assert_eq!(installs(&fake.state().await), [counted(TRELLO, "2.2.0")]);
    assert!(pending(&plugins).is_empty());

    // Once: not again, nor at the next start.
    plugins.send_seed_counts(&store, &http(), Some(true)).await;
    drop(plugins);
    let plugins = started(&data, seed);
    assert!(pending(&plugins).is_empty());
    plugins.send_seed_counts(&store, &http(), Some(true)).await;
    assert_eq!(installs(&fake.state().await).len(), 1);
}

#[tokio::test]
async fn an_install_from_the_seed_is_forgotten_on_a_refusal() {
    let fake = Fake::start().trusted();
    let seed = seed_of(&fake, "refused", &[(TRELLO, "2.2.0")]).await;
    let data = data_of("refused");
    let store = fake.store(&data);
    let plugins = started(&data, seed.clone());
    plugins.send_seed_counts(&store, &http(), Some(false)).await;
    assert!(pending(&plugins).is_empty());
    // A yes later sends nothing of it, nor does a start.
    plugins.send_seed_counts(&store, &http(), Some(true)).await;
    drop(plugins);
    let plugins = started(&data, seed);
    assert!(pending(&plugins).is_empty());
    plugins.send_seed_counts(&store, &http(), Some(true)).await;
    assert!(installs(&fake.state().await).is_empty());
}

#[tokio::test]
async fn an_install_from_the_seed_for_a_gesture_counts() {
    let fake = Fake::start().trusted();
    let seed = seed_of(&fake, "gesture", &[(TRELLO, "2.2.0")]).await;
    let data = data_of("gesture");
    let store = fake.store(&data);
    let list = seed.revocations();
    let plugins = Plugins::load_seeded(data.clone(), "2.2.0", seed);
    plugins.apply_store(list, Default::default());
    plugins.sync_seed(Lang::Fr, &|_| false);
    assert!(plugins.get(TRELLO, Lang::Fr).is_none(), "not used yet");
    assert!(pending(&plugins).is_empty());

    plugins
        .need(TRELLO, Lang::Fr)
        .unwrap()
        .expect("installed now");
    plugins.send_seed_counts(&store, &http(), Some(true)).await;
    assert_eq!(installs(&fake.state().await), [counted(TRELLO, "2.2.0")]);
}

#[tokio::test]
async fn a_count_the_store_does_not_take_stays_and_goes_later() {
    let fake = Fake::start().trusted();
    let seed = seed_of(&fake, "offline", &[(TRELLO, "2.2.0")]).await;
    let data = data_of("offline");
    let plugins = started(&data, seed.clone());

    // The store in trouble (503): kept.
    let store = fake.store(&data);
    fake.post("/_fake/down", json!({ "down": true })).await;
    plugins.send_seed_counts(&store, &http(), Some(true)).await;
    assert_eq!(pending(&plugins).len(), 1);
    // No network at all: kept.
    let nowhere = Store::load(
        &data,
        "http://127.0.0.1:9".into(),
        "http://127.0.0.1:9/mirror".into(),
        "2.2.0",
    );
    plugins
        .send_seed_counts(&nowhere, &http(), Some(true))
        .await;
    assert_eq!(pending(&plugins).len(), 1);
    assert!(installs(&fake.state().await).is_empty());

    // Back at the next start: sent, once.
    fake.post("/_fake/down", json!({ "down": false })).await;
    drop(plugins);
    let plugins = started(&data, seed);
    plugins.send_seed_counts(&store, &http(), Some(true)).await;
    assert_eq!(installs(&fake.state().await), [counted(TRELLO, "2.2.0")]);
    assert!(pending(&plugins).is_empty());
}

#[tokio::test]
async fn a_beta_s_installs_from_the_seed_never_counted_count_once_with_their_version() {
    let fake = Fake::start().trusted();
    let data = data_of("beta");
    let store = fake.store(&data);
    // A 2.2.0-beta: the seed installed Trello and counted nothing (no notes of counts).
    let beta = Seed {
        counts: false,
        ..seed_of(&fake, "beta", &[(TRELLO, "2.2.0")]).await
    };
    drop(started(&data, beta));
    let notes = std::fs::read_to_string(data.plugins_dir().join(".seed.json")).unwrap_or_default();
    assert!(!notes.contains("pending"), "{notes}");

    // The app with this fix, its seed a newer version: the one installed counts, the newer is offered and not installed.
    let seed = seed_of(&fake, "fixed", &[(TRELLO, "2.3.0")]).await;
    let plugins = started(&data, seed.clone());
    assert_eq!(plugins.get(TRELLO, Lang::Fr).unwrap().version, "2.2.0");
    assert_eq!(
        pending(&plugins),
        [(TRELLO.to_string(), "2.2.0".to_string())]
    );
    plugins.send_seed_counts(&store, &http(), Some(true)).await;
    drop(plugins);
    let plugins = started(&data, seed);
    plugins.send_seed_counts(&store, &http(), Some(true)).await;
    assert_eq!(installs(&fake.state().await), [counted(TRELLO, "2.2.0")]);
}

#[tokio::test]
async fn nothing_counts_of_a_plugin_the_app_gives_a_folder_in_development_or_a_seed_of_development()
{
    let fake = Fake::start().trusted();
    // A seed of development (`scripts/seed-dev.sh`, the end-to-end tests'): installed, never counted.
    let data = data_of("dev-seed");
    let dev = Seed {
        counts: false,
        ..seed_of(&fake, "dev-seed", &[(TRELLO, "2.2.0")]).await
    };
    let plugins = started(&data, dev);
    assert!(plugins.get(TRELLO, Lang::Fr).is_some());
    assert!(pending(&plugins).is_empty());
    assert!(
        !Seed::of_app(None).counts,
        "a build of development's seed never counts"
    );

    // A plugin the app gives (isola while it is given) and a folder in development, beside a seed that counts.
    let data = data_of("given");
    drop(Plugins::load_for_core(data.clone(), "2.2.0"));
    let counting = Seed {
        counts: true,
        ..Seed::default()
    };
    let plugins = started(&data, counting.clone());
    assert!(plugins
        .list(Lang::Fr)
        .iter()
        .any(|v| v.origin == Origin::Workspace));
    let src = temp_dir("in-dev");
    std::fs::write(
        src.join(crate::manifest::FILE),
        json!({
            "id": "dev.exemple.mine",
            "name": "Mine",
            "version": "1.0.0",
            "escouade": { "api": "1", "min": "2.1.0" },
            "contributes": { "themes": [{ "id": "mine" }] },
            "later": true
        })
        .to_string(),
    )
    .unwrap();
    std::fs::create_dir_all(src.join("themes")).unwrap();
    std::fs::write(src.join("themes/mine.json"), "{}").unwrap();
    let preview = plugins.inspect_dev(&src, Lang::Fr).unwrap();
    plugins.install(&preview.token, Lang::Fr).unwrap();
    drop(plugins);
    let plugins = started(&data, counting);
    assert!(plugins.get("dev.exemple.mine", Lang::Fr).is_some());
    assert!(pending(&plugins).is_empty());
    plugins
        .send_seed_counts(&fake.store(&data), &http(), Some(true))
        .await;
    assert!(installs(&fake.state().await).is_empty());
}
