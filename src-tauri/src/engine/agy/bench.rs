//! Antigravity CLI on the bench, through the fake `agy` (tests/fixtures/fake-agy.mjs), whose message
//! picks the scenario, its conversation through its plugin (INT-10.8); and the real streams of `agy` 1.4.9 (tests/fixtures/agy/), replayed from
//! tests/fixtures/replays/agy/.

use super::AGY;
use crate::engine::bench::{bench_dir, replay, run, Scenario, Target};
use crate::engine::{Engine, Program};
use std::path::{Path, PathBuf};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
}

/// The fake `agy` of the platform: its `.cmd` under Windows, its shell script elsewhere.
pub(crate) fn fake_agy() -> String {
    let name = if cfg!(windows) {
        "fake-agy.cmd"
    } else {
        "fake-agy"
    };
    fixtures().join(name).to_string_lossy().into_owned()
}

pub(crate) struct OnBench;

impl Target for OnBench {
    fn engine(&self) -> &'static dyn Engine {
        &AGY
    }

    fn program(&self) -> Program {
        AGY.locate(&fake_agy(), "").expect("the fake agy")
    }

    /// Each agent's own home, as the core gives it (`agent_home_var`).
    fn env(&self, dir: &Path) -> Vec<(String, String)> {
        vec![
            (
                super::HOME_VAR.into(),
                dir.join("agy-home").to_string_lossy().into(),
            ),
            (
                "FAKE_AGY_LOG".into(),
                dir.join("fake-agy.jsonl").to_string_lossy().into(),
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
                // No quota in its stream: never played.
                Scenario::Limit => return None,
            }
            .to_string(),
        )
    }

    fn model(&self) -> Option<String> {
        Some(super::DEFAULT.into())
    }

    /// Its plan is written in its mode `plan` alone.
    fn mode(&self, scenario: Scenario) -> Option<&'static str> {
        (scenario == Scenario::PlanMode).then_some("plan")
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_fake_agy_passes_the_bench() {
    // Its conversation goes through its plugin (INT-10.8): the plugins the app gives, in a runtime of their own.
    let (rt, plugins, _) = crate::plugins_acp_tests::runtime("int10-agy-bench");
    crate::plugins::harness::sync(&rt, &plugins);
    let dir = bench_dir("esc270-bench-agy");
    let played = run(&OnBench, &dir).await;
    // No quota nor copy: neither its limit, its reading nor a fork asked of it.
    assert_eq!(
        played,
        [
            Scenario::Text,
            Scenario::Thinking,
            Scenario::Approval,
            Scenario::Edit,
            Scenario::Question,
            Scenario::Interrupt,
            Scenario::PlanMode,
            Scenario::Subagents,
            Scenario::Resume,
        ]
    );
}

#[test]
fn the_recorded_streams_of_agy_make_the_items_expected() {
    replay::check_all(&AGY, "agy");
}
