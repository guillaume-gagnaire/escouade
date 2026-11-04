//! Tests of `escouade-plugin` on the `run.js` plugins (ESC-322, WASM-3): the bench plays them in WASM, in the app's
//! host, without Node (copies of plugins/team-rules and plugins/slack, their manifest on `run.js`, their committed
//! bundle unchanged); `build` writes the same module twice; a bundle that imports what a plugin in WASM has not is
//! refused with the reason, in French and in English; the pinned Javy is checked, its archive and its program.
//!
//! As the tests of plugins_wasm_tests.rs, they need escouade-js built (scripts/escouade-js.sh).

use crate::i18n::Lang;
use crate::paths::DataDir;
use crate::plugins::bench::{cli, run, Options, Report, Scenario, Verdict};
use crate::plugins::manifest::Hand;
use crate::plugins::wasm;
use crate::plugins_wasm_tests::ready;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Where the tests keep the pinned Javy and the modules (as plugins_wasm_tests.rs).
fn shared_data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/wasm-tests")
}

/// A copy of plugins/<dir> (its manifest, its bundle, the JSON files of its root), its manifest on `run.js`.
fn as_run_js(dir: &str) -> PathBuf {
    let src = root().join("plugins").join(dir);
    let to = std::env::temp_dir()
        .join(format!(
            "plugin-bench-wasm-{dir}-{}",
            uuid::Uuid::new_v4().simple()
        ))
        .join(dir);
    std::fs::create_dir_all(to.join("dist")).unwrap();
    std::fs::copy(src.join("dist/plugin.mjs"), to.join("dist/plugin.mjs")).unwrap();
    for e in std::fs::read_dir(&src).unwrap().flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.ends_with(".json") && name != "package.json" && name != "tsconfig.json" {
            std::fs::copy(e.path(), to.join(&name)).unwrap();
        }
    }
    let path = to.join("escouade-plugin.json");
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    m["run"] = serde_json::json!({ "js": "dist/plugin.mjs" });
    // These tests play the bench, not the version check (the repository's plugins name the release that serves WASM,
    // which this build is not yet): the copy asks for any version.
    m["escouade"]["min"] = serde_json::json!("2.1.0");
    std::fs::write(&path, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    to
}

fn options(lang: Lang) -> Options {
    Options {
        lang,
        hand: Hand::First,
        data: shared_data(),
        initialize: Duration::from_secs(5),
        grace: Duration::from_secs(1),
        ..Options::default()
    }
}

fn said(r: &Report, s: Scenario) -> String {
    r.outcome(s).said.clone()
}

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn team_rules_and_slack_on_run_js_pass_the_bench_in_wasm() {
    if !ready() {
        return;
    }
    let rules = run(&as_run_js("team-rules"), &options(Lang::En)).await;
    assert!(rules.passed(), "{}", rules.render(Lang::En));
    assert!(said(&rules, Scenario::Decision).contains("\"allow\""));
    assert_eq!(rules.outcome(Scenario::Crash).verdict, Verdict::Pass);
    assert!(
        said(&rules, Scenario::Shutdown).contains("then ends"),
        "{}",
        rules.render(Lang::En)
    );

    let slack = run(&as_run_js("slack"), &options(Lang::En)).await;
    assert!(slack.passed(), "{}", slack.render(Lang::En));
    assert_eq!(slack.outcome(Scenario::Fact).verdict, Verdict::Pass);
    assert!(said(&slack, Scenario::Secret).contains("token"));
    assert_eq!(slack.outcome(Scenario::Decision).verdict, Verdict::Skip);
}

#[test]
fn build_writes_the_same_module_twice_and_its_sha256() {
    if !ready() {
        return;
    }
    let dir = as_run_js("team-rules");
    let d = dir.to_string_lossy().into_owned();
    let (one, two) = (dir.join("one"), dir.join("two"));
    for out in [&one, &two] {
        let out = out.to_string_lossy().into_owned();
        assert_eq!(
            cli::main(args(&["build", &d, "--first-party", "--out", &out])),
            0
        );
    }
    let name = "dev.escouade.team-rules-2.0.0.wasm";
    let (a, b) = (
        std::fs::read(one.join(name)).unwrap(),
        std::fs::read(two.join(name)).unwrap(),
    );
    assert!(a.starts_with(b"\0asm"));
    assert_eq!(a, b);
    let noted = std::fs::read_to_string(one.join(format!("{name}.sha256"))).unwrap();
    assert_eq!(noted, format!("{}  {name}\n", wasm::sha256(&a)));
    // The validation of a run.js plugin reads its bundle too.
    assert_eq!(cli::main(args(&["validate", &d, "--first-party"])), 0);
}

/// `build` gives the modules noted in goldens.json, to the byte (ESC-328): what the store compiles, and the local
/// review compares. They hold for one compiler and one bundle; another escouade-js (one built under Windows, or a
/// change of it) or another bundle leaves them out, and the note is made again with the Linux escouade-js of CI.
#[test]
fn build_gives_the_modules_of_the_goldens_on_slack_and_team_rules() {
    if !ready() {
        return;
    }
    let repo = root();
    let goldens: Value = serde_json::from_slice(
        &std::fs::read(repo.join("crates/escouade-plugin-cli/goldens.json")).unwrap(),
    )
    .unwrap();
    if goldens["engine"] != wasm::escouade_js_sha256()
        || goldens["prelude"] != wasm::sha256(wasm::PRELUDE.as_bytes())
    {
        eprintln!("skipped: goldens.json speaks of another compiler");
        return;
    }
    let out = crate::paths::test_dir("plugin-goldens");
    for name in ["team-rules", "slack"] {
        let folder = repo.join("plugins").join(name);
        let bundle = std::fs::read(folder.join("dist/plugin.mjs")).unwrap();
        if goldens["plugins"][name]["bundle"] != wasm::sha256(&bundle) {
            eprintln!("skipped {name}: goldens.json speaks of another bundle");
            continue;
        }
        let (d, o) = (folder.to_string_lossy(), out.join(name));
        let o = o.to_string_lossy();
        assert_eq!(
            cli::main(args(&["build", &d, "--first-party", "--out", &o])),
            0
        );
        let file = format!("dev.escouade.{name}-2.0.0.wasm");
        let module = std::fs::read(out.join(name).join(file)).unwrap();
        assert_eq!(goldens["plugins"][name]["module"], wasm::sha256(&module));
    }
}

/// The program the store's image builds says the app's version: the local review compares them (`compiler.prelude`).
#[test]
fn the_crate_of_escouade_plugin_has_the_version_of_the_app() {
    assert_eq!(escouade_plugin_cli::VERSION, env!("CARGO_PKG_VERSION"));
}

/// A copy of team-rules whose bundle imports node:child_process.
fn child_process() -> PathBuf {
    let dir = as_run_js("team-rules");
    std::fs::write(
        dir.join("dist/plugin.mjs"),
        "import { spawn } from \"node:child_process\";\nspawn('calc');\n",
    )
    .unwrap();
    dir
}

#[tokio::test(flavor = "multi_thread")]
async fn a_bundle_that_imports_node_child_process_is_refused_with_the_reason_in_french_and_english()
{
    let dir = child_process();
    let refusal = wasm::script_of("import { spawn } from \"node:child_process\";\n").unwrap_err();
    let (fr, en) = (refusal.message(Lang::Fr), refusal.message(Lang::En));
    assert_eq!(
        fr,
        "le bundle importe node:child_process, qu'un plugin en WASM n'a pas (seulement node:fs, node:readline)"
    );
    assert_eq!(
        en,
        "the bundle imports node:child_process, which a plugin in WASM does not have (only node:fs, node:readline)"
    );
    // The bench says it on the initialization, before anything is compiled; validate and build refuse it.
    for (lang, text) in [(Lang::Fr, &fr), (Lang::En, &en)] {
        let r = run(&dir, &options(lang)).await;
        assert_eq!(r.outcome(Scenario::Initialize).verdict, Verdict::Fail);
        assert_eq!(&said(&r, Scenario::Initialize), text);
        assert!(r
            .outcome(Scenario::Initialize)
            .missing
            .as_deref()
            .unwrap()
            .contains("node:fs"));
    }
    // And exports.
    let r = wasm::script_of("export const x = 1;\n").unwrap_err();
    assert!(r.message(Lang::Fr).contains("sans export"));
}

/// `validate` and `build` refuse it too, in either language, and write nothing.
#[test]
fn validate_and_build_refuse_a_bundle_that_imports_node_child_process() {
    let dir = child_process();
    let d = dir.to_string_lossy().into_owned();
    for lang in ["fr", "en"] {
        assert_eq!(
            cli::main(args(&["validate", &d, "--first-party", "--lang", lang])),
            1
        );
        assert_eq!(
            cli::main(args(&["build", &d, "--first-party", "--lang", lang])),
            1
        );
    }
    assert!(!dir.join("packs").exists());
}

#[test]
fn a_javy_archive_changed_is_refused() {
    let (asset, sum) = wasm::javy_asset().unwrap();
    let e = wasm::unpack_javy(b"\x1f\x8b not the pinned Javy", asset, sum).unwrap_err();
    assert!(e.contains("not the pinned Javy, refused"), "{e}");
}

#[test]
fn a_javy_changed_on_the_disk_is_never_run_and_is_downloaded_again() {
    if !ready() {
        return;
    }
    // The pinned Javy of the tests, checked (its SHA-256 noted beside it), copied into tools of the test's own.
    let shared = wasm::Modules::new(&DataDir::new(shared_data()), "tests");
    let exe = shared.pinned_javy().unwrap();
    let base = std::env::temp_dir().join(format!("javy-check-{}", uuid::Uuid::new_v4().simple()));
    let data = DataDir::new(base.clone());
    let dir = data
        .tools_dir()
        .join(format!("javy-{}", wasm::JAVY_VERSION));
    std::fs::create_dir_all(&dir).unwrap();
    let name = exe.file_name().unwrap();
    std::fs::copy(&exe, dir.join(name)).unwrap();
    let noted = format!("javy-{}.sha256", wasm::JAVY_VERSION);
    std::fs::copy(exe.with_file_name(&noted), dir.join(&noted)).unwrap();
    let sum = std::fs::read_to_string(dir.join(&noted)).unwrap();
    assert_eq!(sum, wasm::sha256(&std::fs::read(&exe).unwrap()));

    let mine = wasm::Modules::new(&data, "tests");
    assert_eq!(mine.pinned_javy().unwrap(), dir.join(name));

    // Changed: not run, downloaded again and checked.
    let mut bytes = std::fs::read(dir.join(name)).unwrap();
    bytes.extend_from_slice(b"changed");
    std::fs::write(dir.join(name), &bytes).unwrap();
    let again = mine.pinned_javy().unwrap();
    assert_eq!(wasm::sha256(&std::fs::read(&again).unwrap()), sum);
    let _ = std::fs::remove_dir_all(base);
}

/// The module of the plugin in `dir` (a copy on `run.js`), compiled as `escouade-plugin build` compiles it.
fn built(dir: &Path) -> PathBuf {
    let bundle = std::fs::read_to_string(dir.join("dist/plugin.mjs")).unwrap();
    let bytes = wasm::Modules::new(&DataDir::new(shared_data()), "tests")
        .build(&bundle)
        .unwrap();
    let at = dir.join("built.wasm");
    std::fs::write(&at, bytes).unwrap();
    at
}

#[tokio::test(flavor = "multi_thread")]
async fn the_bench_plays_the_module_it_is_given_as_it_is() {
    if !ready() {
        return;
    }
    // The review plays the module the store published (ESC-324): team-rules' own passes…
    let rules = as_run_js("team-rules");
    let module = built(&rules);
    let r = run(
        &rules,
        &Options {
            module: Some(module.clone()),
            offline: true,
            ..options(Lang::En)
        },
    )
    .await;
    assert!(r.passed(), "{}", r.render(Lang::En));
    assert!(said(&r, Scenario::Decision).contains("\"allow\""));

    // …and another's, given in its place, is the one that runs: Slack's module does not decide.
    let slack = as_run_js("slack");
    let other = built(&slack);
    let r = run(
        &rules,
        &Options {
            module: Some(other),
            offline: true,
            ..options(Lang::En)
        },
    )
    .await;
    assert!(
        !said(&r, Scenario::Decision).contains("\"allow\""),
        "{}",
        r.render(Lang::En)
    );

    // A file that is no module is said, never compiled from the bundle instead.
    let junk = rules.join("junk.wasm");
    std::fs::write(&junk, b"not a module").unwrap();
    let r = run(
        &rules,
        &Options {
            module: Some(junk),
            ..options(Lang::Fr)
        },
    )
    .await;
    assert_eq!(r.outcome(Scenario::Initialize).verdict, Verdict::Fail);
    assert!(
        said(&r, Scenario::Initialize).contains("n'est pas un module WASM"),
        "{}",
        r.render(Lang::Fr)
    );
}

#[test]
fn the_cli_reads_wasm_and_offline() {
    if !ready() {
        return;
    }
    let rules = as_run_js("team-rules");
    let module = built(&rules);
    let (d, m) = (
        rules.to_string_lossy().into_owned(),
        module.to_string_lossy().into_owned(),
    );
    assert_eq!(
        cli::main(args(&[
            "test",
            &d,
            "--first-party",
            "--wasm",
            &m,
            "--offline",
            "--json"
        ])),
        0
    );
    assert_eq!(
        cli::main(args(&["test", &d, "--first-party", "--wasm", &d, "--json"])),
        1
    );
}

/// A bundle that tries slack.com, which its permissions.network names, when it is initialized, and says what came of it.
const FETCHER: &str = r#"
const out = (m) => __esc.write(JSON.stringify(m));
__esc.listen((line) => {
  const m = JSON.parse(line);
  if (m.method === "initialize")
    fetch("https://slack.com/api/api.test", { method: "POST", body: "{}" })
      .then((r) => console.error(`fetched ${r.status}`), (e) => console.error(`fetch: ${e.message}`))
      .then(() => out({ jsonrpc: "2.0", id: m.id, result: { protocolVersion: 1, capabilities: {} } }));
  if (m.method === "shutdown") out({ jsonrpc: "2.0", id: m.id, result: null });
});
"#;

#[tokio::test(flavor = "multi_thread")]
async fn offline_the_bench_refuses_every_fetch_even_to_the_hosts_of_permissions_network() {
    if !ready() {
        return;
    }
    let dir = as_run_js("slack");
    std::fs::write(dir.join("dist/plugin.mjs"), FETCHER).unwrap();
    let r = run(
        &dir,
        &Options {
            module: Some(built(&dir)),
            offline: true,
            ..options(Lang::En)
        },
    )
    .await;
    let seen = serde_json::to_string(&r.seen).unwrap();
    assert!(
        seen.contains("slack.com is not in permissions.network"),
        "{seen}\n{}",
        r.render(Lang::En)
    );
    assert!(!seen.contains("fetched"), "{seen}");
}
