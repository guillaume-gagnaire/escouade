//! Tests of the conformance bench of the plugins (PLG-3, ESC-175): it passes on the fake plugin
//! (tests/fixtures/fake-plugin.mjs) and on the stand-ins of PLG-10's examples
//! (tests/fixtures/plugins/examples/), and fails, with the right reason, on a plugin that ignores
//! `initialize`, answers past its delay, takes a version it does not speak, asks for another's
//! secret or writes its own out, or does not end at `shutdown`. `escouade-plugin` runs the same.

use crate::i18n::Lang;
use crate::plugins::bench::{cli, run, Options, Report, Scenario, Verdict};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn example(name: &str) -> PathBuf {
    root().join("tests/fixtures/plugins/examples").join(name)
}

/// The fake plugin in a folder of the test's own, its program started with `args`; it asks for
/// `ticket.*`, decides, and keeps the secret `token`.
fn fake(name: &str, args: &[&str]) -> PathBuf {
    #[cfg(windows)]
    crate::paths::trim_test_path();
    let dir = std::env::temp_dir()
        .join(format!(
            "plugin-bench-{name}-{}",
            uuid::Uuid::new_v4().simple()
        ))
        .join("plugin");
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["fake-plugin.mjs", "fake-plugin.cmd", "fake-plugin"] {
        std::fs::copy(root().join("tests/fixtures").join(f), dir.join(f)).unwrap();
    }
    let manifest = json!({
        "id": "dev.test.fake",
        "name": "Faux plugin",
        "version": "1.0.0",
        "escouade": { "api": "1", "min": "2.1.0" },
        "run": { "command": "fake-plugin", "windows": "fake-plugin.cmd", "args": args },
        "activation": ["event:ticket.column"],
        "permissions": { "events": ["ticket.*"], "decide": true, "secrets": ["token"] }
    });
    std::fs::write(
        dir.join("escouade-plugin.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    dir
}

/// In English, with room for a slow machine to start node, and a short wait past a delay.
fn options() -> Options {
    #[cfg(windows)]
    crate::paths::trim_test_path();
    Options {
        lang: Lang::En,
        app_version: "2.1.1".into(),
        initialize: Duration::from_secs(5),
        grace: Duration::from_secs(1),
        ..Options::default()
    }
}

async fn bench(folder: &Path) -> Report {
    run(folder, &options()).await
}

fn verdict(r: &Report, s: Scenario) -> Verdict {
    r.outcome(s).verdict
}

fn said(r: &Report, s: Scenario) -> String {
    r.outcome(s).said.clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_fake_plugin_passes_every_scenario() {
    let r = bench(&fake("pass", &[])).await;
    for o in &r.outcomes {
        assert_eq!(o.verdict, Verdict::Pass, "{}", r.render(Lang::En));
    }
    assert!(r.passed());
    assert_eq!(r.plugin, "dev.test.fake 1.0.0");
    assert!(said(&r, Scenario::Fact).contains("ticket.created received"));
    assert!(said(&r, Scenario::Decision).contains("\"ask\""));
    assert!(said(&r, Scenario::Secret).contains("token"));
    assert!(said(&r, Scenario::Version).contains("its own (1)"));
    // initialize twice (the crash), the decision, the unknown method, shutdown.
    let methods: Vec<&str> = r.timings.iter().map(|t| t.method.as_str()).collect();
    assert_eq!(
        methods,
        [
            "initialize",
            "permissions/decide",
            "bench/unknown",
            "initialize",
            "shutdown"
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_examples_pass_and_what_does_not_concern_them_is_not_played() {
    let slack = bench(&example("slack")).await;
    assert!(slack.passed(), "{}", slack.render(Lang::En));
    assert_eq!(verdict(&slack, Scenario::Fact), Verdict::Pass);
    assert!(said(&slack, Scenario::Secret).contains("webhook"));
    assert_eq!(verdict(&slack, Scenario::Decision), Verdict::Skip);

    let rules = bench(&example("team-rules")).await;
    assert!(rules.passed(), "{}", rules.render(Lang::En));
    assert!(said(&rules, Scenario::Decision).contains("\"allow\""));
    assert_eq!(verdict(&rules, Scenario::Fact), Verdict::Skip);
    assert_eq!(verdict(&rules, Scenario::Secret), Verdict::Skip);

    let theme = bench(&example("high-contrast")).await;
    assert!(theme.passed(), "{}", theme.render(Lang::En));
    assert_eq!(verdict(&theme, Scenario::Manifest), Verdict::Pass);
    for s in &Scenario::ALL[1..] {
        assert_eq!(verdict(&theme, *s), Verdict::Skip, "{s:?}");
    }
    assert!(said(&theme, Scenario::Initialize).contains("declarative"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plugin_that_ignores_initialize_fails_on_it() {
    let r = bench(&fake("ignore", &["--ignore-initialize"])).await;
    assert!(!r.passed());
    assert_eq!(verdict(&r, Scenario::Initialize), Verdict::Fail);
    assert_eq!(
        said(&r, Scenario::Initialize),
        "no answer to initialize within 5 s: Escouade stops the plugin"
    );
    assert!(r
        .outcome(Scenario::Initialize)
        .missing
        .as_deref()
        .unwrap()
        .contains("\"protocolVersion\": 1"));
    for s in &Scenario::ALL[2..] {
        assert_eq!(verdict(&r, *s), Verdict::Skip, "{s:?}");
        assert!(said(&r, *s).contains("initialization must pass first"));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plugin_past_its_delay_fails_on_the_delays() {
    let r = bench(&fake("slow", &["--slow", "800"])).await;
    assert!(!r.passed());
    assert_eq!(
        verdict(&r, Scenario::Delay),
        Verdict::Fail,
        "{}",
        r.render(Lang::En)
    );
    let delay = said(&r, Scenario::Delay);
    assert!(
        delay.starts_with("permissions/decide: answered in "),
        "{delay}"
    );
    assert!(delay.contains("past its delay of 500 ms"), "{delay}");
    assert!(!delay.contains("bench/unknown"), "within 5 s: {delay}");
    // The decision itself holds: only its delay fails.
    assert_eq!(verdict(&r, Scenario::Decision), Verdict::Pass);
    assert_eq!(verdict(&r, Scenario::Initialize), Verdict::Pass);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plugin_that_takes_any_version_fails_and_another_version_fails_initialize() {
    let echo = bench(&fake("echo", &["--protocol", "echo"])).await;
    assert_eq!(verdict(&echo, Scenario::Initialize), Verdict::Pass);
    assert_eq!(verdict(&echo, Scenario::Version), Verdict::Fail);
    assert_eq!(
        said(&echo, Scenario::Version),
        "offered version 999, which does not exist, it says it speaks it"
    );

    let two = bench(&fake("two", &["--protocol", "2"])).await;
    assert_eq!(
        said(&two, Scenario::Initialize),
        "it speaks version 2 of the protocol; this Escouade speaks version 1"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_secret_of_another_or_written_out_fails() {
    let steal = bench(&fake("steal", &["--fact-secret", "plugin:dev.other:token"])).await;
    assert_eq!(verdict(&steal, Scenario::Secret), Verdict::Fail);
    assert!(said(&steal, Scenario::Secret)
        .contains("asked for the secret \"plugin:dev.other:token\", which is not one of its own"));

    let leak = bench(&fake("leak", &["--leak"])).await;
    assert_eq!(verdict(&leak, Scenario::Secret), Verdict::Fail);
    assert_eq!(
        said(&leak, Scenario::Secret),
        "it wrote the value of a secret on its standard error or in a `log`"
    );
    // What it wrote on its own goes to the report (`--json`): the review (STORE-7) reads it there.
    let json = serde_json::to_value(&leak).unwrap();
    assert!(json["seen"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["kind"] == "stderr"
            && s["line"]
                .as_str()
                .unwrap()
                .starts_with("fake-plugin: the secret is")));
    let quiet = serde_json::to_value(bench(&fake("quiet", &[])).await).unwrap();
    assert!(quiet.get("seen").is_none_or(|s| s
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["kind"] != "stderr")));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plugin_that_ignores_shutdown_fails_on_it() {
    let r = bench(&fake("shutdown", &["--ignore-shutdown"])).await;
    assert_eq!(verdict(&r, Scenario::Shutdown), Verdict::Fail);
    assert_eq!(
        said(&r, Scenario::Shutdown),
        "no answer to shutdown within 1000 ms: Escouade ends its job"
    );
    assert_eq!(verdict(&r, Scenario::Crash), Verdict::Pass);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_manifest_that_does_not_hold_plays_nothing() {
    let dir = fake("invalid", &[]);
    std::fs::write(
        dir.join("escouade-plugin.json"),
        r#"{ "id": "Pas un id", "version": "1" }"#,
    )
    .unwrap();
    let r = bench(&dir).await;
    assert_eq!(verdict(&r, Scenario::Manifest), Verdict::Fail);
    assert!(said(&r, Scenario::Manifest).contains("id"));
    for s in &Scenario::ALL[1..] {
        assert_eq!(verdict(&r, *s), Verdict::Skip);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_report_says_what_is_missing_in_both_languages() {
    let dir = fake("render", &["--ignore-shutdown"]);
    let r = run(
        &dir,
        &Options {
            lang: Lang::Fr,
            measure: 50,
            ..options()
        },
    )
    .await;
    let fr = r.render(Lang::Fr);
    assert!(fr.contains("✗ Fermeture"), "{fr}");
    assert!(fr.contains("Ce qui manque : répondre à `shutdown`"), "{fr}");
    assert!(fr.contains("✓ Initialisation"), "{fr}");
    assert!(fr.contains("8 réussis, 1 échoués, 0 sans objet"), "{fr}");
    let m = r.measure.as_ref().expect("measured");
    assert_eq!(m.calls, 50);
    assert!(m.median_us > 0 && m.p99_us >= m.median_us);
    assert!(fr.contains("Allers-retours : 50 appels"), "{fr}");
    let json = serde_json::to_value(&r).unwrap();
    assert_eq!(json["outcomes"][8]["scenario"], "shutdown");
    assert_eq!(json["outcomes"][8]["verdict"], "fail");
}

#[test]
fn the_command_tests_and_validates_a_folder() {
    #[cfg(windows)]
    crate::paths::trim_test_path();
    let path = |p: PathBuf| p.to_string_lossy().into_owned();
    let args = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let theme = path(example("high-contrast"));
    assert_eq!(cli::main(args(&["validate", &theme, "--lang", "en"])), 0);
    assert_eq!(cli::main(args(&["test", &theme, "--lang", "en"])), 0);
    let rules = path(example("team-rules"));
    assert_eq!(cli::main(args(&["test", &rules, "--json"])), 0);

    let broken = fake("cli", &["--ignore-shutdown"]);
    assert_eq!(cli::main(args(&["test", &path(broken.clone())])), 1);
    std::fs::write(broken.join("escouade-plugin.json"), "{}").unwrap();
    assert_eq!(cli::main(args(&["validate", &path(broken)])), 1);
    assert_eq!(cli::main(args(&[])), 2);
    assert_eq!(cli::main(args(&["--help"])), 0);
}

/// A folder of the fake plugin whose icon is a PNG of `width` × 256 px.
fn with_icon(name: &str, width: u32) -> PathBuf {
    let dir = fake(name, &[]);
    let text = std::fs::read_to_string(dir.join("escouade-plugin.json")).unwrap();
    let mut m: serde_json::Value = serde_json::from_str(&text).unwrap();
    m["icon"] = json!("icon.png");
    std::fs::write(dir.join("escouade-plugin.json"), m.to_string()).unwrap();
    let mut png = vec![0u8; 64];
    png[..16].copy_from_slice(b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR");
    png[16..20].copy_from_slice(&width.to_be_bytes());
    png[20..24].copy_from_slice(&256u32.to_be_bytes());
    std::fs::write(dir.join("icon.png"), &png).unwrap();
    dir
}

/// PLG-13: the bench reads the icon's file of the folder, as the app does when it installs it.
#[tokio::test(flavor = "multi_thread")]
async fn an_icon_that_does_not_hold_fails_the_manifest() {
    let r = bench(&with_icon("icon", 300)).await;
    assert_eq!(verdict(&r, Scenario::Manifest), Verdict::Fail);
    let said = said(&r, Scenario::Manifest);
    assert!(said.contains("`icon`"), "{said}");
}

/// And `escouade-plugin validate` too.
#[test]
fn validate_reads_the_icon_of_the_folder() {
    #[cfg(windows)]
    crate::paths::trim_test_path();
    let path = |p: PathBuf| p.to_string_lossy().into_owned();
    let bad = path(with_icon("icon-bad", 300));
    assert_eq!(cli::main(vec!["validate".into(), bad]), 1);
    let good = path(with_icon("icon-good", 256));
    assert_eq!(cli::main(vec!["validate".into(), good]), 0);
}
