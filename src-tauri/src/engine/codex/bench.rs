//! Codex on the bench, through the fake `codex app-server` (tests/fixtures/fake-codex.mjs), whose
//! message picks the scenario, its conversation through its plugin (INT-10.4); and the real streams of Codex 0.163.0 (tests/fixtures/codex/),
//! replayed from tests/fixtures/replays/codex/.

use super::CODEX;
use crate::engine::bench::{bench_dir, replay, run, Scenario, Target};
use crate::engine::{Engine, Program};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
}

/// The fake `codex` of the platform: its `.cmd` under Windows, its shell script elsewhere.
pub(crate) fn fake_codex() -> String {
    let name = if cfg!(windows) {
        "fake-codex.cmd"
    } else {
        "fake-codex"
    };
    fixtures().join(name).to_string_lossy().into_owned()
}

/// What Codex said (`s2c`) in the recorded stream `file` of tests/fixtures/codex/.
pub(crate) fn recorded(file: &str) -> Vec<Value> {
    let text = std::fs::read_to_string(fixtures().join("codex").join(file)).unwrap();
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .filter(|l| l["dir"] == "s2c")
        .map(|l| l["msg"].clone())
        .collect()
}

pub(crate) struct OnBench;

impl Target for OnBench {
    fn engine(&self) -> &'static dyn Engine {
        &CODEX
    }

    fn program(&self) -> Program {
        CODEX.locate(&fake_codex(), "").expect("the fake codex")
    }

    fn env(&self, dir: &Path) -> Vec<(String, String)> {
        vec![
            (
                "CODEX_HOME".into(),
                dir.join("codex").to_string_lossy().into(),
            ),
            (
                "FAKE_CODEX_LOG".into(),
                dir.join("fake-codex.jsonl").to_string_lossy().into(),
            ),
        ]
    }

    fn prompt(&self, scenario: Scenario) -> Option<String> {
        Some(
            match scenario {
                Scenario::Text | Scenario::Resume | Scenario::Fork | Scenario::Quota => "bonjour",
                Scenario::Thinking => "raisonnement",
                Scenario::Approval => "commande",
                Scenario::Edit => "fichier",
                Scenario::Question => "question",
                Scenario::Interrupt => "lent",
                Scenario::PlanMode => "propose un plan",
                Scenario::Subagents => "sous-agent",
                Scenario::Limit => "limite",
            }
            .to_string(),
        )
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_fake_codex_passes_the_bench() {
    // Its conversation goes through its plugin (INT-10.4): the plugins the app gives, in a runtime of their own.
    let (rt, plugins, _) = crate::plugins_acp_tests::runtime("int10-codex-bench");
    crate::plugins::harness::sync(&rt, &plugins);
    let dir = bench_dir("esc37-bench-codex");
    let played = run(&OnBench, &dir).await;
    assert_eq!(played, Scenario::ALL);
}

#[test]
fn the_recorded_streams_of_codex_make_the_items_expected() {
    replay::check_all(&CODEX, "codex");
}
