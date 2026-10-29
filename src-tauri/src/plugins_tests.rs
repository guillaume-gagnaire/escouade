//! Tests of the plugins (ESC-173): the manifest's reader on the cases it shares with the store's
//! library, the range of Escouade, the published schema; installing from a folder and from a fake
//! git repository of the test's folder, a folder in development read again when it changes.

use crate::i18n::Lang;
use crate::paths::DataDir;
use crate::plugins::manifest::{self, Code};
use crate::plugins::{Origin, Plugins, State};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// A folder of its own in the test's temporary folder.
fn temp_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("plugins-{name}-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v
}

/// The bytes of a case's file: a PNG's header of its width and height, padded to its size; or
/// its text.
fn bytes_of(f: &Value) -> Vec<u8> {
    if let Some(t) = f["text"].as_str() {
        return t.chars().map(|c| c as u8).collect();
    }
    let size = f["size"].as_u64().unwrap_or(64).max(33) as usize;
    let side = |i: usize| f["png"][i].as_u64().unwrap() as u32;
    png_header(side(0), side(1), size)
}

/// A PNG's header of `width` × `height`, padded to `size` bytes: what the readers look at.
fn png_header(width: u32, height: u32, size: usize) -> Vec<u8> {
    let mut b = vec![0u8; size.max(33)];
    b[..16].copy_from_slice(b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR");
    b[16..20].copy_from_slice(&width.to_be_bytes());
    b[20..24].copy_from_slice(&height.to_be_bytes());
    b[24..29].copy_from_slice(&[8, 6, 0, 0, 0]);
    b
}

/// Every case of tests/fixtures/plugins/manifests.json, read as the store's library reads it.
#[test]
fn the_reader_says_of_each_shared_case_what_the_store_says() {
    let text =
        std::fs::read_to_string(root().join("tests/fixtures/plugins/manifests.json")).unwrap();
    let cases: Value = serde_json::from_str(&text).unwrap();
    let cases = cases["cases"].as_array().unwrap();
    assert!(cases.len() >= 10);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let builtin = case["builtin"].as_bool().unwrap_or(false);
        let files = case["files"].as_object();
        let file = |p: &str| files.and_then(|f| f.get(p)).map(bytes_of);
        let read = manifest::check_with(
            &case["manifest"],
            builtin,
            files.map(|_| &file as manifest::FileOf),
        );
        let errors: Vec<String> = match &read.manifest {
            Ok(_) => Vec::new(),
            Err(p) => p
                .iter()
                .map(|p| {
                    let code = serde_json::to_value(p.code).unwrap();
                    format!("{} {}", p.path, code.as_str().unwrap())
                })
                .collect(),
        };
        let want: Vec<String> = case["errors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.as_str().unwrap().to_string())
            .collect();
        assert_eq!(sorted(errors), sorted(want), "{name}");
        let ignored: Vec<String> = case["ignored"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.as_str().unwrap().to_string())
            .collect();
        assert_eq!(sorted(read.ignored), sorted(ignored), "{name}");
    }
}

fn valid() -> Value {
    json!({
        "id": "dev.exemple.slack",
        "name": { "fr": "Slack fr", "en": "Slack" },
        "version": "1.2.0",
        "description": { "fr": "Un salon", "en": "A channel" },
        "escouade": { "api": "1", "min": "2.1.0" },
        "run": { "command": "bin/slack", "windows": "bin/slack.exe" },
        "permissions": { "events": ["ticket.*"], "decide": true },
        "contributes": { "themes": [{ "id": "t" }], "hologram": [] }
    })
}

#[test]
fn a_valid_manifest_gives_what_the_app_reads_without_running_anything() {
    let m = manifest::check(&valid(), false).manifest.unwrap();
    assert_eq!(m.id, "dev.exemple.slack");
    assert_eq!(m.name.get(Lang::Fr), "Slack fr");
    assert_eq!(m.name.get(Lang::En), "Slack");
    assert_eq!(m.escouade.api, 1);
    let run = m.run.as_ref().unwrap();
    assert_eq!(run.command, "bin/slack");
    assert_eq!(run.by_system["windows"], "bin/slack.exe");
    assert!(!m.declarative());
    assert!(m.permissions.decide);
    // The points Escouade knows are kept as written; another one is not.
    assert!(m.contributes.contains_key("themes"));
    assert!(!m.contributes.contains_key("hologram"));
    // A text in one language is read in the other.
    let mut v = valid();
    v["name"] = json!({ "fr": "Seulement" });
    let m = manifest::check(&v, false).manifest.unwrap();
    assert_eq!(m.name.get(Lang::En), "Seulement");
    // Not JSON at all: the whole file says so.
    let r = manifest::read("{ id: ", false);
    let p = r.manifest.unwrap_err();
    assert_eq!((p[0].path.as_str(), p[0].code), ("", Code::Type));
}

#[test]
fn each_problem_says_its_field_and_why_in_both_languages() {
    let mut v = valid();
    v["id"] = json!("Slack");
    v["escouade"]["api"] = json!("v1");
    v["contributes"]["settings"] = json!([{ "key": "a", "type": "colour", "title": "A" }]);
    let problems = manifest::check(&v, false).manifest.unwrap_err();
    let fr: Vec<String> = problems.iter().map(|p| p.message(Lang::Fr)).collect();
    let en: Vec<String> = problems.iter().map(|p| p.message(Lang::En)).collect();
    assert_eq!(
        fr,
        [
            "`id` n'est pas un nom de domaine inversé en minuscules (`dev.exemple.slack`).",
            "`escouade.api` n'est pas une version majeure de l'API des plugins (`\"1\"`).",
            "`contributes.settings[0].type` vaut l'un de : string, number, boolean, select.",
        ]
    );
    assert_eq!(
        en[0],
        "`id` is no reversed domain name in lower case (`dev.exemple.slack`)."
    );
    let missing = manifest::check(&json!({}), false).manifest.unwrap_err();
    assert_eq!(missing[0].message(Lang::Fr), "`id` manque.");
    assert_eq!(missing[0].message(Lang::En), "`id` is missing.");
    let not_object = manifest::read("[]", false).manifest.unwrap_err();
    assert_eq!(
        not_object[0].message(Lang::Fr),
        "Le manifeste devrait être un objet."
    );
}

fn with_range(api: &str, min: &str, max: Option<&str>) -> manifest::Manifest {
    let mut v = valid();
    v["escouade"] = json!({ "api": api, "min": min });
    if let Some(max) = max {
        v["escouade"]["max"] = json!(max);
    }
    manifest::check(&v, false).manifest.unwrap()
}

/// As a harness's `Tested`: an API this Escouade does not serve, or an Escouade out of the range,
/// disables the plugin, and says why.
#[test]
fn an_api_or_an_escouade_out_of_its_range_disables_the_plugin_with_the_reason() {
    assert_eq!(
        with_range("1", "2.1.0", None).refusal("2.1.0", Lang::Fr),
        None
    );
    assert_eq!(
        with_range("1", "2", Some("2.4")).refusal("2.4.9", Lang::Fr),
        None
    );
    assert_eq!(
        with_range("2", "2.1.0", None).refusal("2.1.0", Lang::Fr).unwrap(),
        "Ce plugin parle la version 2 de l'API des plugins ; cette version d'Escouade sert la version 1."
    );
    assert_eq!(
        with_range("1", "2.3.0", None)
            .refusal("2.1.0", Lang::En)
            .unwrap(),
        "This plugin needs Escouade 2.3.0 or newer; you have 2.1.0. Update Escouade."
    );
    assert_eq!(
        with_range("1", "2.0", Some("2.0")).refusal("2.1.0", Lang::Fr).unwrap(),
        "Ce plugin a été essayé jusqu'à Escouade 2.0 ; tu as la 2.1.0. Cherche une version plus récente du plugin."
    );
}

/// The schema the repository publishes (`schemas/plugin-1.json`, served by the site) is the one
/// the reader makes: `ESCOUADE_WRITE_SCHEMA=1` writes it again.
#[test]
fn the_published_schema_is_up_to_date() {
    let schema = manifest::schema();
    assert_eq!(schema["$id"], manifest::SCHEMA_URL);
    let made = serde_json::to_string_pretty(&schema).unwrap() + "\n";
    let files = [
        root().join("schemas/plugin-1.json"),
        root().join("website/public/schemas/plugin-1.json"),
    ];
    if std::env::var_os("ESCOUADE_WRITE_SCHEMA").is_some() {
        for f in &files {
            std::fs::write(f, &made).unwrap();
        }
    }
    for f in &files {
        let published: Value =
            serde_json::from_str(&std::fs::read_to_string(f).unwrap_or_default())
                .unwrap_or_default();
        assert_eq!(published, schema, "{} is not up to date", f.display());
    }
}

/// A plugin's folder: its manifest (`manifest`), and a file of its own.
fn plugin_folder(dir: &Path, manifest: &Value) {
    std::fs::create_dir_all(dir.join("themes")).unwrap();
    std::fs::write(
        dir.join(manifest::FILE),
        serde_json::to_string_pretty(manifest).unwrap(),
    )
    .unwrap();
    std::fs::write(dir.join("themes/dark.json"), "{}").unwrap();
}

fn declarative(id: &str, version: &str) -> Value {
    json!({
        "id": id,
        "name": "Contraste",
        "version": version,
        "escouade": { "api": "1", "min": "2.1.0" },
        "contributes": { "themes": [{ "id": "contrast" }] },
        "later": true
    })
}

#[test]
fn a_folder_is_copied_into_the_data_pinned_by_its_files_and_found_again() {
    let base = temp_dir("folder");
    let data = DataDir::new(base.join("data"));
    let src = base.join("src");
    plugin_folder(&src, &declarative("dev.exemple.contraste", "1.0.0"));
    std::fs::create_dir_all(src.join(".git")).unwrap();
    std::fs::write(src.join(".git/HEAD"), "ref").unwrap();

    let plugins = Plugins::load(data.clone(), "2.1.0");
    let view = plugins.install_folder(&src, Lang::Fr).unwrap();
    assert_eq!(view.id, "dev.exemple.contraste");
    assert_eq!(view.state, State::Ready);
    assert!(view.declarative);
    assert_eq!(view.ignored, ["later"]);
    assert!(
        view.fingerprint.starts_with("sha256:"),
        "{}",
        view.fingerprint
    );
    let copy = data.plugins_dir().join("dev.exemple.contraste");
    assert!(copy.join("themes/dark.json").is_file());
    // Its history is not copied.
    assert!(!copy.join(".git").exists());
    assert!(matches!(view.origin, Origin::Folder { .. }));

    // The app started again finds it as it was.
    let again = Plugins::load(data.clone(), "2.1.0");
    let list = again.list(Lang::Fr);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].state, State::Ready);
    assert_eq!(list[0].fingerprint, view.fingerprint);

    // A copy whose files changed loads disabled, and says so.
    std::fs::write(copy.join("themes/dark.json"), "{\"x\":1}").unwrap();
    let changed = Plugins::load(data.clone(), "2.1.0");
    let State::Disabled { reason } = &changed.list(Lang::Fr)[0].state else {
        panic!("{:?}", changed.list(Lang::Fr)[0].state)
    };
    assert!(reason.starts_with("Ses fichiers ont changé"), "{reason}");

    // An older Escouade than its range: loaded disabled, with the reason.
    plugins.install_folder(&src, Lang::Fr).unwrap();
    let old = Plugins::load(data.clone(), "2.0.5");
    let State::Disabled { reason } = &old.list(Lang::En)[0].state else {
        panic!()
    };
    assert_eq!(
        reason,
        "This plugin needs Escouade 2.1.0 or newer; you have 2.0.5. Update Escouade."
    );

    // Uninstalled: its copy goes, the source folder stays.
    assert!(plugins.remove("dev.exemple.contraste").unwrap());
    assert!(!copy.exists());
    assert!(src.join(manifest::FILE).is_file());
    assert!(Plugins::load(data, "2.1.0").list(Lang::Fr).is_empty());
}

#[test]
fn an_invalid_manifest_loads_nothing_and_says_why_field_by_field() {
    let base = temp_dir("invalid");
    let data = DataDir::new(base.join("data"));
    let src = base.join("src");
    let mut bad = declarative("Contraste", "1.0");
    bad["escouade"]["api"] = json!(1);
    plugin_folder(&src, &bad);
    let plugins = Plugins::load(data.clone(), "2.1.0");
    let refusal = plugins.install_folder(&src, Lang::Fr).unwrap_err();
    assert_eq!(
        refusal.message,
        "Le manifeste (escouade-plugin.json) n'est pas valide ; rien n'a été chargé :\n\
         - `id` n'est pas un nom de domaine inversé en minuscules (`dev.exemple.slack`).\n\
         - `version` n'est pas un numéro de version (`1.2.0`).\n\
         - `escouade.api` devrait être un texte."
    );
    let paths: Vec<&str> = refusal
        .problems
        .iter()
        .map(|p| p.problem.path.as_str())
        .collect();
    assert_eq!(paths, ["id", "version", "escouade.api"]);
    // Nothing was copied, nothing listed.
    assert!(
        !data.plugins_dir().exists() || std::fs::read_dir(data.plugins_dir()).unwrap().count() == 0
    );
    assert!(plugins.list(Lang::Fr).is_empty());
    // No manifest at all.
    let empty = base.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let refusal = plugins.load_dev(&empty, Lang::En).unwrap_err();
    assert!(
        refusal.message.starts_with("No escouade-plugin.json in "),
        "{}",
        refusal.message
    );
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A git repository of a plugin in the test's folder, two versions tagged.
fn fake_repo(dir: &Path) -> (String, String) {
    std::fs::create_dir_all(dir).unwrap();
    git(dir, &["init", "-q", "-b", "main"]);
    git(dir, &["config", "user.email", "t@test"]);
    git(dir, &["config", "user.name", "t"]);
    git(dir, &["config", "core.autocrlf", "false"]);
    plugin_folder(dir, &declarative("dev.exemple.git", "1.0.0"));
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-qm", "v1"]);
    git(dir, &["tag", "v1.0.0"]);
    let v1 = git(dir, &["rev-parse", "HEAD"]);
    plugin_folder(dir, &declarative("dev.exemple.git", "1.1.0"));
    git(dir, &["commit", "-qam", "v1.1"]);
    git(dir, &["tag", "v1.1.0"]);
    let v11 = git(dir, &["rev-parse", "HEAD"]);
    (v1, v11)
}

#[tokio::test]
async fn a_git_repository_is_installed_at_its_tag_pinned_by_the_commit() {
    let base = temp_dir("git");
    let data = DataDir::new(base.join("data"));
    let repo = base.join("repo");
    let (v1, v11) = fake_repo(&repo);
    let url = repo.display().to_string();
    let plugins = Plugins::load(data.clone(), "2.1.0");

    let view = plugins.install_git(&url, "v1.0.0", Lang::Fr).await.unwrap();
    assert_eq!(view.version, "1.0.0");
    assert_eq!(view.fingerprint, format!("git:{v1}"));
    assert_eq!(view.state, State::Ready);
    assert_eq!(
        view.origin,
        Origin::Git {
            url: url.clone(),
            tag: "v1.0.0".into()
        }
    );
    let copy = data.plugins_dir().join("dev.exemple.git");
    assert!(copy.join(manifest::FILE).is_file());
    assert!(!copy.join(".git").exists());

    // Another tag takes its place, pinned by its own commit; no clone is left behind.
    let view = plugins.install_git(&url, "v1.1.0", Lang::Fr).await.unwrap();
    assert_eq!(view.version, "1.1.0");
    assert_eq!(view.fingerprint, format!("git:{v11}"));
    assert_eq!(plugins.list(Lang::Fr).len(), 1);
    let left: Vec<_> = std::fs::read_dir(data.plugins_dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(left, ["dev.exemple.git"]);

    // A tag that does not exist, or none: refused, and why.
    let refusal = plugins.install_git(&url, "v9", Lang::Fr).await.unwrap_err();
    assert!(
        refusal
            .message
            .contains("n'a pas pu être cloné à l'étiquette v9"),
        "{}",
        refusal.message
    );
    let refusal = plugins.install_git(&url, " ", Lang::En).await.unwrap_err();
    assert!(
        refusal.message.starts_with("Give the tag"),
        "{}",
        refusal.message
    );
    assert_eq!(
        plugins.get("dev.exemple.git", Lang::Fr).unwrap().version,
        "1.1.0"
    );
}

fn wait_for(what: &str, mut ok: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(20);
    while !ok() {
        assert!(Instant::now() < until, "{what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn a_folder_in_development_is_loaded_in_place_and_read_again_when_it_changes() {
    let base = temp_dir("dev");
    let data = DataDir::new(base.join("data"));
    let src = base.join("src");
    plugin_folder(&src, &declarative("dev.exemple.dev", "0.1.0"));
    let plugins = Plugins::load(data.clone(), "2.1.0");
    let view = plugins.load_dev(&src, Lang::Fr).unwrap();
    assert!(matches!(view.origin, Origin::Dev { .. }));
    assert_eq!(view.fingerprint, "");
    // Loaded where it is: nothing copied.
    assert!(!data.plugins_dir().join("dev.exemple.dev").exists());

    let rev = plugins.revision();
    plugin_folder(&src, &declarative("dev.exemple.dev", "0.2.0"));
    wait_for("the new version read", || {
        plugins.revision() > rev
            && plugins.get("dev.exemple.dev", Lang::Fr).unwrap().version == "0.2.0"
    });

    // Broken while it is developed: nothing of it loads, and each field says why.
    let mut bad = declarative("dev.exemple.dev", "0.3");
    bad["escouade"] = json!({ "api": "1" });
    plugin_folder(&src, &bad);
    wait_for("the broken manifest read", || {
        matches!(
            plugins.get("dev.exemple.dev", Lang::Fr).unwrap().state,
            State::Invalid { .. }
        )
    });
    let view = plugins.get("dev.exemple.dev", Lang::Fr).unwrap();
    assert!(view.manifest.is_none());
    let State::Invalid { problems, .. } = view.state else {
        panic!()
    };
    let paths: Vec<&str> = problems.iter().map(|p| p.problem.path.as_str()).collect();
    assert_eq!(paths, ["version", "escouade.min"]);

    // Started again, the app watches it still.
    drop(plugins);
    let again = Plugins::load(data, "2.1.0");
    let rev = again.revision();
    plugin_folder(&src, &declarative("dev.exemple.dev", "0.4.0"));
    wait_for("read again after a restart", || {
        again.revision() > rev && again.get("dev.exemple.dev", Lang::Fr).unwrap().version == "0.4.0"
    });
    // Uninstalled, the folder stays.
    assert!(again.remove("dev.exemple.dev").unwrap());
    assert!(src.join(manifest::FILE).is_file());
}

/// PLG-13: the icon of an installed plugin, by its id alone, read in its folder and never out of
/// it; none for a plugin without one; an id that is not installed is refused.
#[test]
fn the_icon_of_a_plugin_is_given_by_its_id_and_never_read_out_of_its_folder() {
    use base64::Engine as _;
    let base = temp_dir("icon");
    let data = DataDir::new(base.join("data"));
    let src = base.join("src");
    let mut m = declarative("dev.exemple.logo", "1.0.0");
    m["icon"] = json!("./assets/icon.png");
    plugin_folder(&src, &m);
    std::fs::create_dir_all(src.join("assets")).unwrap();
    let png = png_header(512, 512, 600);
    std::fs::write(src.join("assets/icon.png"), &png).unwrap();
    let plugins = Plugins::load(data.clone(), "2.1.0");
    let view = plugins.install_folder(&src, Lang::Fr).unwrap();
    assert_eq!(view.state, State::Ready);
    assert_eq!(
        view.manifest.unwrap().icon.as_deref(),
        Some("./assets/icon.png")
    );
    let url = plugins.icon("dev.exemple.logo").unwrap().unwrap();
    let b64 = url.strip_prefix("data:image/png;base64,").unwrap();
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(b64)
            .unwrap(),
        png
    );

    // An id that is not installed, or a path given as an id: refused, nothing read.
    assert!(plugins.icon("dev.exemple.absent").is_err());
    assert!(plugins.icon("../../src/assets/icon.png").is_err());

    // Without an icon: none, the window draws its letters.
    let plain = base.join("plain");
    plugin_folder(&plain, &declarative("dev.exemple.plain", "1.0.0"));
    plugins.install_folder(&plain, Lang::Fr).unwrap();
    assert_eq!(plugins.icon("dev.exemple.plain").unwrap(), None);

    // An icon that does not hold refuses the plugin, field by field.
    let bad = base.join("bad");
    let mut m = declarative("dev.exemple.bad", "1.0.0");
    m["icon"] = json!("icon.png");
    plugin_folder(&bad, &m);
    std::fs::write(bad.join("icon.png"), png_header(300, 256, 600)).unwrap();
    let refusal = plugins.install_folder(&bad, Lang::En).unwrap_err();
    assert!(
        refusal
            .message
            .ends_with("- `icon` is not square (a square PNG of 256 px at least, 512 advised)."),
        "{}",
        refusal.message
    );
    std::fs::write(bad.join("icon.png"), "<svg/>").unwrap();
    let refusal = plugins.install_folder(&bad, Lang::Fr).unwrap_err();
    assert_eq!(refusal.problems[0].problem.code, Code::Png);

    // The folder's reader never leaves it: no `..`, no absolute path, no drive.
    let outside = base.join("outside.png");
    std::fs::write(&outside, &png).unwrap();
    assert!(manifest::icon_file(&src, "assets/icon.png").is_some());
    assert!(manifest::icon_file(&src, "../outside.png").is_none());
    assert!(manifest::icon_file(&src, "assets/../../outside.png").is_none());
    assert!(manifest::icon_file(&src, &outside.display().to_string()).is_none());
    assert!(manifest::icon_file(&src, "assets").is_none());
    // Nor does a link of the folder to a file out of it (when the system lets the test make one).
    #[cfg(unix)]
    let linked = std::os::unix::fs::symlink(&outside, src.join("link.png")).is_ok();
    #[cfg(windows)]
    let linked = std::os::windows::fs::symlink_file(&outside, src.join("link.png")).is_ok();
    if linked {
        assert!(manifest::icon_file(&src, "link.png").is_none());
    }
    // A file too heavy is read one byte past the limit, no further.
    std::fs::write(src.join("big.png"), png_header(512, 512, 3 << 20)).unwrap();
    let big = manifest::icon_file(&src, "big.png").unwrap();
    assert_eq!(big.len() as u64, manifest::ICON_BYTES + 1);
    assert_eq!(manifest::icon_problems(&big), [Code::Heavy]);

    // A folder in development whose icon became too heavy: none, until it holds again.
    let dev = base.join("dev");
    let mut m = declarative("dev.exemple.devlogo", "0.1.0");
    m["icon"] = json!("icon.png");
    plugin_folder(&dev, &m);
    std::fs::write(dev.join("icon.png"), &png).unwrap();
    plugins.load_dev(&dev, Lang::Fr).unwrap();
    assert!(plugins.icon("dev.exemple.devlogo").unwrap().is_some());
    std::fs::write(dev.join("icon.png"), png_header(512, 512, 2 << 20)).unwrap();
    assert_eq!(plugins.icon("dev.exemple.devlogo").unwrap(), None);
}
