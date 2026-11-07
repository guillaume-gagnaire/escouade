//! Tests of the plugins the app gives (INT-4, ESC-210): the registry and the origin `builtin`, the ids `dev.escouade.*`
//! kept for Escouade's own hand, the switches for the app and for a project, the same id from the store refused, the
//! program of the app checked before it runs, and the demonstration plugin started by the runtime from
//! `escouade-plugins` (the one the build made beside the tests).

use crate::i18n::Lang;
use crate::integrations::secrets::MemoryStore;
use crate::paths::DataDir;
use crate::plugins::builtin::{self, REGISTRY};
use crate::plugins::manifest::{self, Hand};
use crate::plugins::runtime::{Limits, PhaseName, Runtime};
use crate::plugins::{Origin, Plugins, State};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

const ID: &str = "dev.escouade.minimal";
const VERSION: &str = "2.2.0";

fn base(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "plugins-builtin-{name}-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Where the build put its programs: the folder of the tests' own, `deps/`, is beside it.
fn app_dir() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let dir = exe.parent().unwrap().parent().unwrap().to_path_buf();
    assert!(
        dir.join(builtin::program_name()).is_file(),
        "{} is not built beside the tests (cargo build --bin escouade-plugins)",
        dir.join(builtin::program_name()).display()
    );
    dir
}

fn plugins_in(data: &DataDir) -> Arc<Plugins> {
    Plugins::load_in(data.clone(), VERSION, app_dir())
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn the_registry_gives_the_demonstration_plugin_with_the_origin_builtin_and_the_version_of_the_app()
{
    let data = DataDir::new(base("registry").join("data"));
    let plugins = plugins_in(&data);
    let all = plugins.list(Lang::En);
    assert_eq!(all.len(), REGISTRY.len());
    let v = plugins.get(ID, Lang::En).expect("the registry gives it");
    assert_eq!(v.origin, Origin::Builtin);
    assert_eq!(v.state, State::Ready);
    assert_eq!(v.version, VERSION, "their version is Escouade's");
    assert_eq!(v.fingerprint, "builtin:2.2.0");
    assert!(v.choices.enabled);
    assert_eq!(
        v.manifest.unwrap().run.unwrap().command,
        "{escouade}/escouade-plugins"
    );
    assert!(v.update.is_none() && v.withdrawn.is_none() && v.in_store.is_none());
    assert_eq!(
        v.origin,
        serde_json::from_value(json!({ "kind": "builtin" })).unwrap()
    );
}

/// A plugin ported for the store says the version of Escouade it was ported for: the app that gives it runs it all the
/// same, its version not yet there.
#[test]
fn a_plugin_of_the_registry_runs_on_the_app_that_gives_it() {
    let dir = base("min");
    let plugins = Plugins::load_in(DataDir::new(dir.join("data")), "2.1.2", dir.join("app"));
    let v = plugins
        .get("dev.escouade.linear", Lang::En)
        .expect("the registry gives it");
    assert!(v.manifest.is_some(), "{:?}", v.state);
    assert_eq!(v.manifest.unwrap().escouade.min, "2.1.2");
}

/// The sorting is by the origin and the hand, never by the id: a reserved id of anyone else's hand is refused, and
/// the app's programs are the app's alone.
#[test]
fn a_reserved_id_or_a_program_of_the_app_is_refused_to_a_plugin_that_is_not_the_apps() {
    let text = builtin::manifest_text(ID, VERSION).unwrap();
    assert!(manifest::read(&text, Hand::Builtin).manifest.is_ok());
    for hand in [Hand::Third, Hand::First] {
        let read = manifest::read(&text, hand);
        let problems = read.manifest.expect_err("refused");
        assert!(
            problems.iter().any(|p| p.path == "run.command"),
            "{hand:?}: {problems:?}"
        );
    }
    let third = manifest::read(&text, Hand::Third).manifest.unwrap_err();
    assert!(third.iter().any(|p| p.path == "id"), "{third:?}");
    // Nor does a folder copied by hand with the id give it a place: the id is refused to the hand of anyone's.
    let dir = base("reserved");
    std::fs::write(
        dir.join(manifest::FILE),
        text.replace("{escouade}/escouade-plugins", "run.mjs"),
    )
    .unwrap();
    let plugins = plugins_in(&DataDir::new(dir.join("data")));
    assert!(plugins.inspect_folder(&dir, Lang::En).is_err());
}

#[test]
fn a_plugin_of_the_store_or_a_folder_with_the_id_of_one_the_app_gives_is_refused() {
    let dir = base("same-id");
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let mut m: serde_json::Value =
        serde_json::from_str(&builtin::manifest_text(ID, "9.9.9").unwrap()).unwrap();
    m["run"] = json!({ "command": "run.mjs" });
    std::fs::write(src.join(manifest::FILE), m.to_string()).unwrap();
    let plugins = plugins_in(&DataDir::new(dir.join("data")));
    // In development, Escouade's own hand may take the id (ESC-314): the one the app gives still wins.
    let preview = plugins.inspect_dev(&src, Lang::En).unwrap();
    assert!(preview
        .refusal
        .as_deref()
        .unwrap()
        .contains("shipped with Escouade"));
    let err = plugins.install(&preview.token, Lang::En).unwrap_err();
    assert!(err.to_string().contains("shipped with Escouade"), "{err}");
    let v = plugins.get(ID, Lang::En).unwrap();
    assert_eq!(v.origin, Origin::Builtin);
    assert_eq!(v.version, VERSION);
}

#[test]
fn the_switches_hold_across_a_restart_and_the_plugin_cannot_be_removed() {
    let data = DataDir::new(base("choices").join("data"));
    let plugins = plugins_in(&data);
    assert!(plugins.reaches(ID, Some("p1"), false));
    plugins
        .choose(ID, |c| {
            c.projects.insert("p1".into(), false);
        })
        .unwrap();
    assert!(!plugins.reaches(ID, Some("p1"), false), "off for a project");
    assert!(plugins.reaches(ID, Some("p2"), false), "on for the others");
    plugins.choose(ID, |c| c.enabled = false).unwrap();
    assert!(!plugins.reaches(ID, Some("p2"), false), "off for the app");
    assert!(plugins.remove(ID, false).is_err(), "it cannot be removed");
    // Read again from the files, as a new start does: still one, still off, no copy of it written.
    let again = plugins_in(&data);
    assert_eq!(again.list(Lang::En).len(), REGISTRY.len());
    let v = again.get(ID, Lang::En).unwrap();
    assert!(!v.choices.enabled);
    assert_eq!(v.choices.projects.get("p1"), Some(&false));
    assert!(!data.plugins_dir().join(ID).exists());
}

#[test]
fn a_copy_of_its_id_left_by_an_older_install_gives_way_and_keeps_the_choices() {
    let dir = base("older");
    let data = DataDir::new(dir.join("data"));
    let copy = data.plugins_dir().join(ID);
    std::fs::create_dir_all(&copy).unwrap();
    std::fs::write(copy.join("junk"), "x").unwrap();
    std::fs::create_dir_all(data.root()).unwrap();
    std::fs::write(
        data.plugins_file(),
        json!({ "version": 1, "plugins": [{
            "id": ID,
            "origin": { "kind": "store", "version": "1.0.0", "sha256": "abc", "verified": true },
            "fingerprint": "sha256:abc",
            "installedAt": "2026-10-01T00:00:00Z",
            "enabled": false
        }] })
        .to_string(),
    )
    .unwrap();
    let plugins = plugins_in(&data);
    let all = plugins.list(Lang::En);
    assert_eq!(all.len(), REGISTRY.len());
    assert_eq!(all[0].origin, Origin::Builtin);
    assert!(!all[0].choices.enabled, "the user's choice is kept");
    assert!(!copy.exists(), "the stale copy goes");
}

/// The registry is the folders of `plugins/` whose program is `escouade-plugins` or a module (`plugins/<id-court>/`, docs/PLUGINS.md
/// §8.6.1) and the demonstration, a fixture: each with its manifest and its README (the rule of INT-4: readable alone),
/// each manifest holding as the app's own. A plugin to publish has its LICENSE, the root's, and its CHANGELOG too.
#[test]
fn every_folder_of_the_app_s_plugins_has_what_it_needs_and_is_in_the_registry() {
    let of_the_app = |dir: &PathBuf| {
        std::fs::read_to_string(dir.join(manifest::FILE)).is_ok_and(|text| {
            text.contains("{escouade}/escouade-plugins") || text.contains("\"wasm\"")
        })
    };
    let folders: Vec<PathBuf> = std::fs::read_dir(repo().join("plugins"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(of_the_app)
        .collect();
    let license = std::fs::read_to_string(repo().join("LICENSE")).unwrap();
    let fixture = repo().join("tests/fixtures/plugins/minimal");
    let mut seen = Vec::new();
    for dir in folders.iter().chain([&fixture]) {
        let name = dir.display();
        let text = std::fs::read_to_string(dir.join(manifest::FILE))
            .unwrap_or_else(|_| panic!("{name}: no {}", manifest::FILE));
        let readme = std::fs::read_to_string(dir.join("README.md"))
            .unwrap_or_else(|_| panic!("{name}: no README.md"));
        assert!(
            readme.lines().count() > 5,
            "{name}: a README that says something"
        );
        let m = manifest::read(&text, Hand::Builtin)
            .manifest
            .unwrap_or_else(|p| panic!("{name}: {p:?}"));
        assert!(m.id.starts_with(manifest::RESERVED_PREFIX));
        seen.push(m.id);
        if dir != &fixture {
            let own = std::fs::read_to_string(dir.join("LICENSE"))
                .unwrap_or_else(|_| panic!("{name}: no LICENSE"));
            assert_eq!(own, license, "{name}: the GPL of the code it came from");
            assert!(
                dir.join("CHANGELOG.md").is_file(),
                "{name}: no CHANGELOG.md"
            );
        }
    }
    seen.sort();
    let mut ids: Vec<&str> = REGISTRY.iter().map(|e| e.id).collect();
    ids.sort();
    assert_eq!(
        seen, ids,
        "the registry is the folders of the app's plugins"
    );
}

#[test]
fn only_the_app_s_own_program_is_started_for_a_program_of_the_app() {
    let app = app_dir();
    let program = app.join(builtin::program_name());
    builtin::check_program(&program, &app).expect("the app's own");
    // The same file, anywhere else: not the app's.
    let elsewhere = base("copy");
    let copy = elsewhere.join(builtin::program_name());
    std::fs::copy(&program, &copy).unwrap();
    let err = builtin::check_program(&copy, &app).unwrap_err();
    assert!(err.contains("is not the"), "{err}");
    // Another file of the app's folder.
    let other = app.join(if cfg!(windows) {
        "escouade-hook.exe"
    } else {
        "escouade-hook"
    });
    if other.exists() {
        assert!(builtin::check_program(&other, &app).is_err());
    }
    // A program that is not there.
    assert!(builtin::check_program(&app.join("nope"), &app).is_err());
}

async fn until(what: &str, f: impl Fn() -> bool) {
    let started = Instant::now();
    while !f() {
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "{what}: not in time"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn limits() -> Limits {
    Limits {
        initialize: Duration::from_secs(20),
        shutdown: Duration::from_secs(2),
        ..Limits::default()
    }
}

/// The demonstration plugin runs: started by the first fact it hears, from the app's `escouade-plugins` (checked
/// before it runs), the handshake held, and off for the app, nothing of it starts.
#[tokio::test(flavor = "multi_thread")]
async fn the_demonstration_plugin_runs_from_the_program_of_the_app_unless_it_is_off() {
    let data = DataDir::new(base("runs").join("data"));
    let plugins = plugins_in(&data);
    let rt = Runtime::new(
        plugins.clone(),
        Arc::new(MemoryStore::default()),
        &data,
        VERSION,
        limits(),
    );
    plugins.choose(ID, |c| c.enabled = false).unwrap();
    assert!(
        rt.activate("event:ticket.column").is_empty(),
        "off for the app: nothing runs"
    );
    plugins.choose(ID, |c| c.enabled = true).unwrap();
    assert_eq!(rt.activate("event:ticket.column"), vec![ID.to_string()]);
    until("ready", || {
        let s = rt.status(ID);
        if s.stop.is_some() {
            panic!("{:?} {:#?}", s.stop, rt.journal(ID));
        }
        s.phase == PhaseName::Running
    })
    .await;
    let hello = rt
        .call(ID, "minimal/hello", json!({}))
        .await
        .unwrap_or_else(|e| panic!("{e}: {:#?}", rt.journal(ID)));
    assert_eq!(hello["escouade"], VERSION);
    let echo = rt
        .call(ID, "minimal/echo", json!({ "a": [1, 2] }))
        .await
        .unwrap();
    assert_eq!(echo, json!({ "a": [1, 2] }));
    rt.shutdown();
}

/// A program of the app that is not the app's own is not run.
#[tokio::test(flavor = "multi_thread")]
async fn a_program_of_the_app_that_is_not_the_app_s_own_is_not_started() {
    let data = DataDir::new(base("refused").join("data"));
    // The app's folder holds no escouade-plugins here: the registry's plugin names a program that is not there.
    let empty = base("no-program");
    let plugins = Plugins::load_in(data.clone(), VERSION, empty);
    let rt = Runtime::new(
        plugins,
        Arc::new(MemoryStore::default()),
        &data,
        VERSION,
        limits(),
    );
    rt.activate("event:ticket.column");
    until("refused", || rt.status(ID).stop.is_some()).await;
    let stop = format!("{:?}", rt.status(ID).stop);
    assert!(stop.contains("not the app's own program"), "{stop}");
    rt.shutdown();
}

/// The isola plugin is the launch environment of a worktree with an `.isola.toml` while it is on for the app and for
/// the project, and its program is installed; else the worktree is nobody's. Its process answers `launch/ls`, `down`
/// and `destroy` (INT-14).
#[tokio::test(flavor = "multi_thread")]
async fn isola_is_the_launch_environment_of_its_worktrees_unless_it_is_off_or_missing_its_program()
{
    use crate::launchenv::Launch;
    // The test double is a script of node's.
    #[cfg(windows)]
    crate::paths::trim_test_path();
    let id = "dev.escouade.isola";
    let data = DataDir::new(base("isola").join("data"));
    let plugins = plugins_in(&data);
    let rt = Runtime::new(
        plugins.clone(),
        Arc::new(MemoryStore::default()),
        &data,
        VERSION,
        limits(),
    );
    let launch = Launch::new(plugins.clone(), rt.clone());
    let wt = base("isola-worktree");
    let dir = wt.to_string_lossy().to_string();
    // No fake isola, no program: nobody's, with the file or without.
    *crate::isola::TEST_CLI.lock().unwrap() = None;
    std::fs::write(wt.join(".isola.toml"), "").unwrap();
    assert!(!launch.manages(&dir, "p1"));
    assert!(
        launch.configured(&dir, "p1"),
        "the file alone says what the recipe should leave"
    );
    crate::isola::tests::use_fake();
    assert!(launch.manages(&dir, "p1"));
    let env = launch.for_dir(&dir, "p1").unwrap();
    assert_eq!(
        env.up_command(),
        crate::isola::UP,
        "the command is the manifest's, the input closed where the system has no terminal for it"
    );
    std::fs::write(wt.join(".isola-branch"), "feat").unwrap();
    assert!(env.services(&dir, "feat").await.unwrap().is_empty());
    crate::isola::run(&dir, &["up"], crate::isola::LIMIT)
        .await
        .unwrap();
    let services = env.services(&dir, "feat").await.unwrap();
    assert_eq!(services[0].name, "web");
    assert_eq!(services[0].url, "http://feat.demo.localhost:3000");
    env.down(&dir).await.unwrap();
    assert!(env.services(&dir, "feat").await.unwrap().is_empty());
    env.destroy(&dir).await.unwrap();
    assert_eq!(
        crate::isola::tests::calls(&wt),
        [
            "ls --json",
            "up",
            "ls --json",
            "down",
            "ls --json",
            "destroy"
        ]
    );
    // Its failure says why, in the language of the interface (the fake fails when it is told to).
    std::fs::write(wt.join(".isola-fail"), "").unwrap();
    let e = env.down(&dir).await.unwrap_err().to_string();
    assert!(e.contains("the fake isola fails"), "{e}");
    std::fs::remove_file(wt.join(".isola-fail")).unwrap();
    // Off for one project: nobody's there, still isola's elsewhere; off for the app: nobody's anywhere, and the
    // process is not asked.
    plugins
        .choose(id, |c| {
            c.projects.insert("p2".into(), false);
        })
        .unwrap();
    assert!(!launch.manages(&dir, "p2") && launch.manages(&dir, "p1"));
    plugins.choose(id, |c| c.enabled = false).unwrap();
    assert!(!launch.manages(&dir, "p1") && !launch.configured(&dir, "p1"));
    // Without the file, it is not isola's.
    plugins.choose(id, |c| c.enabled = true).unwrap();
    std::fs::remove_file(wt.join(".isola.toml")).unwrap();
    assert!(!launch.manages(&dir, "p1"));
    // A path or an option is no file name: the manifest of a plugin cannot reach outside the worktree.
    rt.shutdown();
}

/// The code hosts are modules (ESC-345): what they reach is their manifest's, and the hosts of their accounts for those
/// that have instances; a call that may be long has the time and the room a process had.
#[test]
fn the_code_hosts_are_modules_that_reach_their_hosts_and_their_instances() {
    let dir = base("hosts");
    let plugins = Plugins::load_in(DataDir::new(dir.join("data")), "2.2.0", dir.join("app"));
    for (id, instances) in [
        ("dev.escouade.github", true),
        ("dev.escouade.gitlab", true),
        ("dev.escouade.bitbucket", false),
        ("dev.escouade.gitea", true),
        ("dev.escouade.azure-devops", true),
    ] {
        let view = plugins.get(id, Lang::En).expect("the registry gives it");
        let m = view.manifest.expect("its manifest");
        assert_eq!(
            m.run.as_ref().and_then(|r| r.wasm.as_deref()),
            Some("Cargo.toml"),
            "{id}"
        );
        let declared = m.permissions.network.iter().any(|h| h == "{account}");
        assert_eq!(declared, instances, "{id}");
        assert_eq!(m.limits.fetch_seconds, Some(15), "{id}");
        assert_eq!(m.limits.fetch_bytes, Some(8 << 20), "{id}");
    }
}
