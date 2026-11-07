//! Copilot on the bench, through the fake `copilot --server --stdio` (tests/fixtures/fake-copilot.mjs),
//! whose message picks the scenario; and the real streams of Copilot CLI 1.0.98
//! (tests/fixtures/copilot/), replayed from tests/fixtures/replays/copilot/.

use super::COPILOT;
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

/// The fake `copilot` of the platform: its `.cmd` under Windows, its shell script elsewhere.
pub(crate) fn fake_copilot() -> String {
    let name = if cfg!(windows) {
        "fake-copilot.cmd"
    } else {
        "fake-copilot"
    };
    fixtures().join(name).to_string_lossy().into_owned()
}

/// The recorded stream `file` of tests/fixtures/copilot/, whole (`{"dir", "t", "msg"}` a line).
pub(crate) fn stream(file: &str) -> Vec<Value> {
    let text = std::fs::read_to_string(fixtures().join("copilot").join(file)).unwrap();
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .collect()
}

/// What Copilot said (`s2c`) in the recorded stream `file`.
pub(crate) fn recorded(file: &str) -> Vec<Value> {
    stream(file)
        .into_iter()
        .filter(|l| l["dir"] == "s2c")
        .map(|l| l["msg"].clone())
        .collect()
}

pub(crate) struct OnBench;

impl Target for OnBench {
    fn engine(&self) -> &'static dyn Engine {
        &COPILOT
    }

    fn program(&self) -> Program {
        COPILOT
            .locate(&fake_copilot(), "")
            .expect("the fake copilot")
    }

    fn env(&self, dir: &Path) -> Vec<(String, String)> {
        vec![
            (
                "COPILOT_HOME".into(),
                dir.join("copilot").to_string_lossy().into(),
            ),
            (
                "FAKE_COPILOT_LOG".into(),
                dir.join("fake-copilot.jsonl").to_string_lossy().into(),
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

    fn model(&self) -> Option<String> {
        Some("gpt-6-luna".into())
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_fake_copilot_passes_the_bench() {
    // Its conversation goes through its plugin (INT-10.3): the plugins the app gives, in a runtime of their own.
    let (rt, plugins, _) = crate::plugins_acp_tests::runtime("int10-copilot-bench");
    crate::plugins::harness::sync(&rt, &plugins);
    let dir = bench_dir("esc39-bench-copilot");
    let played = run(&OnBench, &dir).await;
    // The month's credits (ESC-40): the limit and the quota too.
    assert_eq!(played, Scenario::ALL.to_vec());
}

#[test]
fn the_recorded_streams_of_copilot_make_the_items_expected() {
    replay::check_all(&COPILOT, "copilot");
}
