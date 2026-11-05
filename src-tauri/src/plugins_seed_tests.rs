//! Tests of the app's seed (ESC-338, docs/PLUGINS.md §8.7), on what the fake store published, its bytes and its signatures:
//! installed with no network and by the store's path, a newer version offered and never installed by itself, never an
//! older one, a plugin the user uninstalled left out and proposed, a false signature or a changed file refused, a version
//! withdrawn stopped with its reason, the range of Escouade, a build of development without signatures, and the passage of
//! an id from the origin `builtin` to the seed.

use crate::i18n::Lang;
use crate::paths::DataDir;
use crate::plugins::seed::{self, Entry, Seed};
use crate::plugins::store::{self, Store};
use crate::plugins::wasm::{Made, Modules};
use crate::plugins::{builtin, Origin, Plugins, State};
use crate::plugins_store_tests::{http, temp_dir, Fake};
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
    plugins.sync_seed(Lang::Fr);
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
    let rt = crate::plugins::runtime::Runtime::new(
        plugins.clone(),
        std::sync::Arc::new(crate::integrations::secrets::MemoryStore::default()),
        &data,
        "2.2.0",
        crate::plugins::runtime::Limits::default(),
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
    let rt = crate::plugins::runtime::Runtime::new(
        plugins.clone(),
        std::sync::Arc::new(crate::integrations::secrets::MemoryStore::default()),
        &data,
        "2.2.0",
        crate::plugins::runtime::Limits::default(),
    );
    rt.prepare_all();
    let folder = PathBuf::from(plugins.get(TRELLO, Lang::Fr).unwrap().folder);
    let (_, made) = Modules::new(&data, "2.2.0")
        .prepare_reactor(&folder)
        .expect("precompiled");
    assert_eq!(made, Made::Precompiled, "the start did not do it");
}

/// Until Z2 the registry gives these plugins; the day `plugins/seed.json` names one, it leaves the origin `builtin`, the
/// user's choices with it, and becomes a plugin like any other.
#[tokio::test]
async fn an_id_the_seed_names_leaves_the_origin_builtin_with_its_choices() {
    let fake = Fake::start().trusted();
    let data = data_of("passage");
    // A user of a version before: the plugin given, switched off.
    let plugins = Plugins::load_for_core(data.clone(), "2.2.0");
    let given = plugins.get(TRELLO, Lang::Fr).unwrap();
    assert_eq!(given.origin, Origin::Builtin);
    assert!(plugins.remove(TRELLO, false).is_err(), "given, not removed");
    plugins.choose(TRELLO, |c| c.enabled = false).unwrap();
    drop(plugins);

    let seed = seed_of(&fake, "passage", &[(TRELLO, "2.2.0")]).await;
    let plugins = started(&data, seed.clone());
    let view = plugins.get(TRELLO, Lang::Fr).unwrap();
    assert!(matches!(view.origin, Origin::Store { seed: true, .. }));
    assert!(!view.choices.enabled, "the user's choice");
    assert_eq!(
        plugins.get(builtin::GITHUB, Lang::Fr).unwrap().origin,
        Origin::Builtin,
        "the others are given as before"
    );

    // Its install is no longer refused, and it goes as any other's.
    assert!(plugins.remove(TRELLO, false).unwrap());
    drop(plugins);
    let plugins = started(&data, seed);
    assert!(
        plugins.get(TRELLO, Lang::Fr).is_none(),
        "the registry does not put it back"
    );
    assert_eq!(plugins.seed_offers(Lang::Fr).len(), 1);
    let e = plugins.remove(builtin::GITHUB, false).unwrap_err();
    assert!(e.to_string().contains("given with Escouade"));
}

/// A plugin the seed refused for a user of the origin `builtin` keeps running as it did: the app's own program.
#[tokio::test]
async fn an_id_the_seed_cannot_give_stays_builtin_and_is_never_removed_with_the_app_s_folder() {
    let fake = Fake::start().trusted();
    let data = data_of("stays");
    let given = Plugins::load_for_core(data.clone(), "2.2.0");
    given.choose(TRELLO, |c| c.enabled = false).unwrap();
    drop(given);
    let seed = Seed {
        dir: Some(temp_dir("stays")),
        ..seed_of(&fake, "listed", &[(TRELLO, "2.2.0")]).await
    };
    let plugins = started(&data, seed);
    assert_eq!(
        plugins.get(TRELLO, Lang::Fr).unwrap().origin,
        Origin::Builtin
    );
    assert!(plugins.remove(TRELLO, false).is_err());
    assert!(plugins.app_dir().exists());
}

#[test]
fn the_committed_list_reads_and_names_ids_of_the_registry() {
    let list =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../plugins/seed.json"))
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
