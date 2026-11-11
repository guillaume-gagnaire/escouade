//! Tests of the registry of the first hand (docs/PLUGINS-DESIGN.md §8.7) and of the workspace a build of development runs
//! (`Origin::Workspace`, ESC-355): the ids `dev.escouade.*` kept for Escouade's own hand, the switches for the app and
//! for a project, a plugin of the same id from elsewhere left as it is, and the demonstration plugin, a module, started
//! by the runtime.

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

/// The folder of the build, beside the tests' own (`deps/`): the workspace's modules are beside it (`plugins-wasm/`).
fn app_dir() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    exe.parent().unwrap().parent().unwrap().to_path_buf()
}

fn plugins_in(data: &DataDir) -> Arc<Plugins> {
    Plugins::load_in(data.clone(), VERSION, app_dir())
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn the_workspace_runs_the_demonstration_plugin_a_module_at_the_version_of_the_app() {
    let data = DataDir::new(base("registry").join("data"));
    let plugins = plugins_in(&data);
    let all = plugins.list(Lang::En);
    assert_eq!(all.len(), REGISTRY.len());
    let v = plugins.get(ID, Lang::En).expect("the workspace runs it");
    assert_eq!(v.origin, Origin::Workspace);
    assert_eq!(v.state, State::Ready);
    assert_eq!(v.version, VERSION, "the build's version");
    assert_eq!(v.fingerprint, "workspace");
    assert!(v.choices.enabled);
    assert_eq!(
        v.manifest.unwrap().run.unwrap().wasm.as_deref(),
        Some("Cargo.toml")
    );
    assert!(v.update.is_none() && v.withdrawn.is_none() && v.in_store.is_none());
    // The origin `builtin` reads no more: an entry of 2.1 is kept aside (`plugins_seed_tests`).
    assert!(serde_json::from_value::<Origin>(json!({ "kind": "builtin" })).is_err());
}

/// A plugin ported for the store says the version of Escouade it was ported for: a build of development runs the
/// workspace's all the same, its version not yet there.
#[test]
fn a_plugin_of_the_workspace_runs_on_the_build_that_runs_it() {
    let dir = base("min");
    let plugins = Plugins::load_in(DataDir::new(dir.join("data")), "2.1.2", dir.join("app"));
    let v = plugins
        .get("dev.escouade.linear", Lang::En)
        .expect("the workspace runs it");
    assert!(v.manifest.is_some(), "{:?}", v.state);
    assert_eq!(v.manifest.unwrap().escouade.min, "2.1.2");
}

/// A pre-release (2.2.0-beta.1) runs the workspace all the same: their version is the build's, pre-release and all,
/// their `escouade.min` its numbers alone, which is all a manifest takes there.
#[test]
fn a_pre_release_runs_the_workspace() {
    let dir = base("beta");
    let plugins = Plugins::load_in(
        DataDir::new(dir.join("data")),
        "2.2.0-beta.1",
        dir.join("app"),
    );
    for id in [ID, "dev.escouade.linear"] {
        let v = plugins.get(id, Lang::En).expect("the workspace runs it");
        assert!(v.manifest.is_some(), "{id}: {:?}", v.state);
        assert_eq!(v.version, "2.2.0-beta.1", "{id}");
        assert_eq!(v.manifest.unwrap().escouade.min, "2.2.0", "{id}");
    }
}

/// The sorting is by the origin and the hand, never by the id: a reserved id of anyone else's hand is refused.
#[test]
fn a_reserved_id_is_refused_to_a_plugin_of_anyone_else_s_hand() {
    let text = builtin::manifest_text(ID, VERSION).unwrap();
    assert!(manifest::read(&text, Hand::First).manifest.is_ok());
    let third = manifest::read(&text, Hand::Third).manifest.unwrap_err();
    assert!(third.iter().any(|p| p.path == "id"), "{third:?}");
    // Nor does a folder copied by hand with the id give it a place: the id is refused to the hand of anyone's.
    let dir = base("reserved");
    let mut m: serde_json::Value = serde_json::from_str(&text).unwrap();
    m["run"] = json!({ "js": "plugin.mjs" });
    std::fs::write(dir.join(manifest::FILE), m.to_string()).unwrap();
    std::fs::write(dir.join("plugin.mjs"), "").unwrap();
    let plugins = plugins_in(&DataDir::new(dir.join("data")));
    assert!(plugins.inspect_folder(&dir, Lang::En).is_err());
}

/// No plugin wins by its id (ESC-355): in development, Escouade's own hand may take the id of one of the registry, and
/// it takes the place of the workspace's.
#[test]
fn a_folder_in_development_with_an_id_of_the_registry_takes_its_place() {
    let dir = base("same-id");
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let mut m: serde_json::Value =
        serde_json::from_str(&builtin::manifest_text(ID, VERSION).unwrap()).unwrap();
    m["version"] = json!("9.9.9");
    m["run"] = json!({ "js": "plugin.mjs" });
    std::fs::write(src.join(manifest::FILE), m.to_string()).unwrap();
    std::fs::write(src.join("plugin.mjs"), "").unwrap();
    let plugins = plugins_in(&DataDir::new(dir.join("data")));
    let preview = plugins.inspect_dev(&src, Lang::En).unwrap();
    assert!(preview.refusal.is_none(), "{:?}", preview.refusal);
    plugins.install(&preview.token, Lang::En).unwrap();
    let v = plugins.get(ID, Lang::En).unwrap();
    assert!(matches!(v.origin, Origin::Dev { .. }), "{:?}", v.origin);
    assert_eq!(v.version, "9.9.9");
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
    assert!(
        plugins.remove(ID, false).is_err(),
        "a module of the workspace cannot be removed"
    );
    // Read again from the files, as a new start does: still one, still off, no copy of it written.
    let again = plugins_in(&data);
    assert_eq!(again.list(Lang::En).len(), REGISTRY.len());
    let v = again.get(ID, Lang::En).unwrap();
    assert!(!v.choices.enabled);
    assert_eq!(v.choices.projects.get("p1"), Some(&false));
    assert!(!data.plugins_dir().join(ID).exists());
}

/// The registry is the folders of `plugins/` whose program is a module (`plugins/<id-court>/`, docs/PLUGINS-DESIGN.md §8.6.1) and
/// the demonstration, a fixture: each with its manifest and its README (the rule of INT-4: readable alone), each
/// manifest holding as one of the first hand. A plugin to publish has its LICENSE, the root's, and its CHANGELOG too.
#[test]
fn every_folder_of_the_app_s_plugins_has_what_it_needs_and_is_in_the_registry() {
    let of_the_app = |dir: &PathBuf| {
        std::fs::read_to_string(dir.join(manifest::FILE))
            .is_ok_and(|text| text.contains("\"wasm\""))
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
        // Its page: README.md, or one by language (README.fr.md and README.en.md), as the store takes them.
        let pages: Vec<String> = match std::fs::read_to_string(dir.join("README.md")) {
            Ok(one) => vec![one],
            Err(_) => ["README.fr.md", "README.en.md"]
                .iter()
                .map(|f| {
                    std::fs::read_to_string(dir.join(f))
                        .unwrap_or_else(|_| panic!("{name}: no README.md, nor {f}"))
                })
                .collect(),
        };
        for readme in &pages {
            assert!(
                readme.lines().count() > 5,
                "{name}: a README that says something"
            );
        }
        let m = manifest::read(&text, Hand::First)
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

/// The demonstration plugin runs: started by the first fact it hears, its module in WASM, the handshake held, and off
/// for the app, nothing of it starts.
#[tokio::test(flavor = "multi_thread")]
async fn the_demonstration_plugin_runs_as_a_module_unless_it_is_off() {
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
        let view = plugins.get(id, Lang::En).expect("the workspace runs it");
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
