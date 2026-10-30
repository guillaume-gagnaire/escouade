//! Tests of the store in the app (STORE-9), against the fake store of the end-to-end tests
//! (`tests/fixtures/store/fake-store.mjs`, run by node, its key made for the run): the catalogue
//! and the last compatible version, a verified version installed with its seal checked here (and
//! again offline), an unverified one, « N'installer que des versions vérifiées », a changed archive
//! and a forged seal refused, the updates of the store, a plugin from git offered the store's
//! version, the revocation list (disable, unverify, a `serial` that goes back, the copy on
//! escouade.dev), and the count of installs.

use crate::i18n::Lang;
use crate::paths::DataDir;
use crate::plugins::store::{self, Store};
use crate::plugins::{Origin, Plugins, State};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("store-{name}-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The fake store, run by node; stopped when dropped.
struct Fake {
    url: String,
    key: String,
    child: Child,
}

impl Drop for Fake {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Fake {
    fn start() -> Self {
        let mut child = Command::new("node")
            .arg(root().join("tests/fixtures/store/fake-store.mjs"))
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("node runs the fake store");
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let v: Value = serde_json::from_str(&line).expect("the fake store says its address");
        Self {
            url: v["url"].as_str().unwrap().to_string(),
            key: v["key"].as_str().unwrap().to_string(),
            child,
        }
    }

    /// Trusts its key as the app trusts the store's (this test's process alone: nextest runs each
    /// test in its own).
    fn trusted(self) -> Self {
        std::env::set_var("ESCOUADE_STORE_TEST_KEY", &self.key);
        self
    }

    fn store(&self, data: &DataDir) -> Store {
        Store::load(
            data,
            self.url.clone(),
            format!("{}/mirror", self.url),
            "2.1.0",
        )
    }

    async fn post(&self, path: &str, body: Value) -> Value {
        reqwest::Client::new()
            .post(format!("{}{path}", self.url))
            .json(&body)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap()
    }

    async fn state(&self) -> Value {
        reqwest::get(format!("{}/_fake/state", self.url))
            .await
            .unwrap()
            .json()
            .await
            .unwrap()
    }
}

fn http() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().build().unwrap()
}

/// A store read, and plugins of the same data.
async fn ready(fake: &Fake, name: &str) -> (DataDir, Store, Arc<Plugins>) {
    let data = DataDir::new(temp_dir(name).join("data"));
    let store = fake.store(&data);
    store.refresh(&http(), Lang::Fr).await;
    let plugins = Plugins::load(data.clone(), "2.1.0");
    plugins.apply_store(store.revocations(), store.offers());
    (data, store, plugins)
}

async fn install(
    store: &Store,
    plugins: &Plugins,
    id: &str,
    version: Option<&str>,
    verified_only: bool,
) -> Result<crate::plugins::PluginView, String> {
    let f = store
        .fetch(&http(), id, version, verified_only, Lang::Fr)
        .await?;
    let p = plugins.inspect_store(f, Lang::Fr).map_err(|r| r.message)?;
    plugins.install(&p.token, Lang::Fr).map_err(|r| r.message)
}

#[tokio::test]
async fn the_catalogue_is_kept_and_the_last_compatible_version_is_offered() {
    let fake = Fake::start().trusted();
    let (data, store, _) = ready(&fake, "catalogue").await;
    let view = store.view();
    let cat = view.catalog.expect("the catalogue was read");
    assert!(cat.plugins.iter().any(|p| p.id == "dev.exemple.slack"));
    assert!(view.read_at.is_some() && view.error.is_none());
    // 2.0.0 asks for Escouade 99: the 1.0.0 is the one this Escouade runs.
    assert_eq!(view.best["dev.exemple.later"], "1.0.0");
    assert_eq!(view.best["dev.exemple.high-contrast"], "1.1.0");

    // Read again: the ETag answers 304, the catalogue kept is the same.
    store.refresh(&http(), Lang::Fr).await;
    assert!(store.view().catalog.is_some());
    // Without the store, a new start shows what was kept, and says the store does not answer.
    drop(fake);
    let again = Store::load(
        &data,
        "http://127.0.0.1:9".into(),
        "http://127.0.0.1:9".into(),
        "2.1.0",
    );
    assert!(again.view().catalog.is_some());
    again.refresh(&http(), Lang::Fr).await;
    let v = again.view();
    assert!(v.catalog.is_some());
    assert!(v.error.unwrap().contains("Le store ne répond pas"));
}

#[tokio::test]
async fn a_verified_version_installs_with_its_seal_checked_here_and_again_offline() {
    let fake = Fake::start().trusted();
    let (data, store, plugins) = ready(&fake, "verified").await;
    let f = store
        .fetch(
            &http(),
            "dev.exemple.high-contrast",
            Some("1.0.0"),
            false,
            Lang::Fr,
        )
        .await
        .unwrap();
    assert!(f.minisig.is_some(), "the seal holds with the app's keys");
    let preview = plugins.inspect_store(f, Lang::Fr).unwrap();
    assert!(preview.verified);
    assert!(preview.icon.is_some(), "the icon is read from the archive");
    assert!(
        matches!(&preview.origin, Origin::Store { version, verified: true, tag, .. } if version == "1.0.0" && tag == "v1.0.0")
    );
    let view = plugins.install(&preview.token, Lang::Fr).unwrap();
    assert_eq!(view.state, State::Ready);
    assert!(view.verified);
    assert!(view.fingerprint.starts_with("store:"));

    // Read again from the disk, the store gone: the seal is checked against the files kept.
    drop(store);
    drop(fake);
    let again = Plugins::load(data.clone(), "2.1.0");
    assert!(
        again
            .get("dev.exemple.high-contrast", Lang::Fr)
            .unwrap()
            .verified
    );
    // A version.json changed on the disk: no seal any more.
    let kept = data
        .plugins_dir()
        .join(".store/dev.exemple.high-contrast/version.json");
    let text = std::fs::read_to_string(&kept).unwrap();
    std::fs::write(&kept, text.replace("1.0.0", "1.0.1")).unwrap();
    let changed = Plugins::load(data.clone(), "2.1.0");
    assert!(
        !changed
            .get("dev.exemple.high-contrast", Lang::Fr)
            .unwrap()
            .verified
    );
    // Uninstalled: its proofs go with it.
    changed.remove("dev.exemple.high-contrast", false).unwrap();
    assert!(!kept.exists());
}

#[tokio::test]
async fn without_the_store_key_a_verified_version_installs_without_a_seal() {
    // No ESCOUADE_STORE_TEST_KEY: as a release before the user gives the keys (STORE_PUBLIC_KEYS empty).
    let fake = Fake::start();
    if !crate::plugins::store_keys::STORE_PUBLIC_KEYS.is_empty() {
        return;
    }
    let (_, store, plugins) = ready(&fake, "nokey").await;
    let view = install(&store, &plugins, "dev.exemple.team-rules", None, false)
        .await
        .unwrap();
    assert!(!view.verified);
    // « N'installer que des versions vérifiées » then refuses it.
    let refused = install(
        &store,
        &plugins,
        "dev.exemple.high-contrast",
        Some("1.0.0"),
        true,
    )
    .await
    .unwrap_err();
    assert!(refused.contains("n'est pas vérifiée"), "{refused}");
}

#[tokio::test]
async fn an_unverified_version_installs_with_its_program_unless_only_verified_ones_are_wanted() {
    let fake = Fake::start().trusted();
    let (_, store, plugins) = ready(&fake, "unverified").await;
    let refused = install(&store, &plugins, "dev.exemple.slack", None, true)
        .await
        .unwrap_err();
    assert!(
        refused.contains("La v1.0.0 n'est pas vérifiée"),
        "{refused}"
    );
    assert!(plugins.get("dev.exemple.slack", Lang::Fr).is_none());

    let view = install(&store, &plugins, "dev.exemple.slack", None, false)
        .await
        .unwrap();
    assert!(!view.verified);
    // This system's program from the release, where the manifest names it, not the repository's.
    let program = if cfg!(windows) { "slack.cmd" } else { "slack" };
    let written = std::fs::read_to_string(Path::new(&view.folder).join(program)).unwrap();
    assert!(written.contains(&store::platform()), "{written}");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(Path::new(&view.folder).join(program))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111);
    }
}

#[tokio::test]
async fn a_changed_archive_or_a_forged_seal_stops_everything() {
    let fake = Fake::start().trusted();
    let (data, store, plugins) = ready(&fake, "tampered").await;
    let e = install(&store, &plugins, "dev.exemple.tampered", None, false)
        .await
        .unwrap_err();
    assert!(e.contains("L'archive reçue ne correspond pas"), "{e}");
    let e = install(&store, &plugins, "dev.exemple.forged", None, false)
        .await
        .unwrap_err();
    assert!(
        e.contains("La signature de cette version ne tient pas"),
        "{e}"
    );
    let e = install(&store, &plugins, "dev.exemple.nowhere", None, false)
        .await
        .unwrap_err();
    assert!(e.contains("n'est pas dans le catalogue"), "{e}");
    assert!(plugins.list(Lang::Fr).is_empty());
    // Nothing was set aside.
    let left: Vec<_> = std::fs::read_dir(data.plugins_dir())
        .map(|d| d.filter_map(|e| e.ok()).map(|e| e.file_name()).collect())
        .unwrap_or_default();
    assert!(left.is_empty(), "{left:?}");
}

#[tokio::test]
async fn the_updates_of_a_store_plugin_come_from_the_catalogue_and_say_if_verified() {
    let fake = Fake::start().trusted();
    let (_, store, plugins) = ready(&fake, "update").await;
    install(
        &store,
        &plugins,
        "dev.exemple.high-contrast",
        Some("1.0.0"),
        false,
    )
    .await
    .unwrap();
    plugins.apply_store(store.revocations(), store.offers());
    let view = plugins.get("dev.exemple.high-contrast", Lang::Fr).unwrap();
    let update = view.update.expect("1.1.0 is offered");
    assert_eq!(update.version, "1.1.0");
    assert_eq!(update.verified, Some(false));
    assert!(view.in_store.is_none());

    // Updated: the choices kept, the seal gone with the unverified version.
    plugins
        .choose("dev.exemple.high-contrast", |c| {
            c.projects.insert("p1".into(), false);
        })
        .unwrap();
    let f = store
        .fetch(
            &http(),
            "dev.exemple.high-contrast",
            Some("1.1.0"),
            false,
            Lang::Fr,
        )
        .await
        .unwrap();
    let p = plugins.inspect_store(f, Lang::Fr).unwrap();
    assert_eq!(p.previous.as_ref().unwrap().version, "1.0.0");
    let view = plugins.install(&p.token, Lang::Fr).unwrap();
    assert_eq!(view.version, "1.1.0");
    assert!(!view.verified);
    assert_eq!(view.choices.projects.get("p1"), Some(&false));
    assert!(view.update.is_none());
}

#[tokio::test]
async fn a_plugin_from_a_folder_is_offered_the_store_version_of_its_id() {
    let fake = Fake::start().trusted();
    let (_, store, plugins) = ready(&fake, "fromgit").await;
    let src = root().join("tests/fixtures/plugins/examples/high-contrast");
    let view = plugins.install_folder(&src, Lang::Fr).unwrap();
    assert_eq!(view.id, "dev.exemple.high-contrast");
    let offer = view.in_store.expect("the store has this id");
    assert_eq!(offer.version, "1.1.0");
    assert!(!view.verified);
    // Moved to the store's version: the same id, now from the store.
    let moved = install(&store, &plugins, "dev.exemple.high-contrast", None, false)
        .await
        .unwrap();
    assert!(matches!(moved.origin, Origin::Store { .. }));
    assert!(moved.in_store.is_none());
    assert_eq!(plugins.list(Lang::Fr).len(), 1);
}

#[tokio::test]
async fn the_revocation_list_withdraws_unverifies_and_its_serial_never_goes_back() {
    let fake = Fake::start().trusted();
    let (data, store, plugins) = ready(&fake, "revoke").await;
    install(&store, &plugins, "dev.exemple.team-rules", None, false)
        .await
        .unwrap();
    install(
        &store,
        &plugins,
        "dev.exemple.high-contrast",
        Some("1.0.0"),
        false,
    )
    .await
    .unwrap();
    let stopped = Arc::new(AtomicUsize::new(0));
    let s = stopped.clone();
    plugins.on_reload(move |id| {
        if id == "dev.exemple.team-rules" {
            s.fetch_add(1, Ordering::SeqCst);
        }
    });
    let reason = |fr: &str| json!({ "fr": fr, "en": "why" });
    fake.post(
        "/_fake/revocations",
        json!({ "entries": [
            { "id": "dev.exemple.team-rules", "versions": "*", "action": "disable", "reason": reason("Envoyait les commandes ailleurs."), "date": "2026-10-30" },
            { "id": "dev.exemple.high-contrast", "versions": ["1.0.0"], "action": "unverify", "reason": reason("Relecture à refaire."), "date": "2026-10-30" },
        ] }),
    )
    .await;
    store.refresh(&http(), Lang::Fr).await;
    plugins.apply_store(store.revocations(), store.offers());
    let list = store.view().revocations.unwrap();
    assert_eq!(list.serial, Some(1));
    assert_eq!(list.from.as_deref(), Some("store"));

    let rules = plugins.get("dev.exemple.team-rules", Lang::Fr).unwrap();
    assert!(
        matches!(&rules.state, State::Disabled { reason } if reason.contains("Envoyait les commandes ailleurs"))
    );
    assert_eq!(rules.withdrawn.as_ref().unwrap().action, "disable");
    assert_eq!(stopped.load(Ordering::SeqCst), 1, "its process is stopped");
    let hc = plugins.get("dev.exemple.high-contrast", Lang::Fr).unwrap();
    assert_eq!(hc.state, State::Ready);
    assert!(!hc.verified, "the seal is taken off");
    assert_eq!(hc.withdrawn.unwrap().reason, "Relecture à refaire.");
    // A withdrawn version does not install again.
    let e = install(&store, &plugins, "dev.exemple.team-rules", None, false)
        .await
        .unwrap_err();
    assert!(e.contains("retirée par Escouade"), "{e}");

    // The list kept applies at the next start, store or not.
    let again = Plugins::load(data.clone(), "2.1.0");
    let kept = fake.store(&data);
    again.apply_store(kept.revocations(), kept.offers());
    assert!(matches!(
        again.get("dev.exemple.team-rules", Lang::Fr).unwrap().state,
        State::Disabled { .. }
    ));

    // An older list played again is refused; the one taken stays.
    fake.post("/_fake/revocations", json!({ "entries": [], "serial": 3 }))
        .await;
    store.refresh(&http(), Lang::Fr).await;
    assert_eq!(store.view().revocations.unwrap().serial, Some(3));
    fake.post("/_fake/revocations", json!({ "entries": [], "serial": 2 }))
        .await;
    store.refresh(&http(), Lang::Fr).await;
    let list = store.view().revocations.unwrap();
    assert_eq!(list.serial, Some(3));
    assert!(list.error.unwrap().contains("plus ancienne"));

    // A list signed by another key is refused.
    let json = reqwest::get(format!("{}/api/v1/revocations.json", fake.url))
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let sig = reqwest::get(format!("{}/api/v1/revocations.json.minisig", fake.url))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let stranger = vec!["RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3".to_string()];
    assert!(matches!(
        store::check_list(&stranger, &json, &sig, 0),
        Err(store::ListRefused::Signature(_))
    ));
    assert!(store::check_list(std::slice::from_ref(&fake.key), &json, &sig, 0).is_ok());
}

#[tokio::test]
async fn the_copy_on_escouade_dev_answers_when_the_store_is_down() {
    let fake = Fake::start().trusted();
    let (_, store, _) = ready(&fake, "mirror").await;
    let entry = json!({ "id": "dev.exemple.slack", "versions": "*", "action": "disable", "reason": { "fr": "x", "en": "x" }, "date": "2026-10-30" });
    fake.post("/_fake/revocations", json!({ "entries": [entry] }))
        .await;
    fake.post("/_fake/down", json!({ "down": true })).await;
    store.refresh(&http(), Lang::Fr).await;
    let v = store.view();
    assert!(v.catalog.is_some(), "the catalogue kept is shown");
    assert!(v.error.is_some());
    let list = v.revocations.unwrap();
    assert_eq!(list.serial, Some(1));
    assert_eq!(list.from.as_deref(), Some("mirror"));
}

#[tokio::test]
async fn an_install_is_counted_by_its_id_and_version_alone() {
    let fake = Fake::start().trusted();
    let (_, store, _) = ready(&fake, "count").await;
    store
        .count(&http(), "dev.exemple.slack", "1.0.0", false)
        .await;
    store
        .count(&http(), "dev.exemple.slack", "1.0.1", true)
        .await;
    let state = fake.state().await;
    assert_eq!(
        state["installs"],
        json!([
            { "id": "dev.exemple.slack", "version": "1.0.0", "event": "install" },
            { "id": "dev.exemple.slack", "version": "1.0.1", "event": "update" },
        ])
    );
}
