//! Mistral Vibe on the bench, through the fake `vibe-acp` (tests/fixtures/fake-vibe.mjs), whose
//! message picks the scenario, its conversation through its plugin (INT-10.6). The real streams of
//! Mistral Vibe 3.0.3 (tests/fixtures/vibe/) are replayed by the plugin's client against
//! tests/fixtures/replays/vibe/ (`plugins_acp_tests`).

use super::VIBE;
use crate::engine::bench::{bench_dir, run, Scenario, Target};
use crate::engine::{Engine, Program};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
}

/// The fake `vibe-acp` of the platform: its `.cmd` under Windows, its shell script elsewhere.
pub(crate) fn fake_vibe() -> String {
    let name = if cfg!(windows) {
        "fake-vibe.cmd"
    } else {
        "fake-vibe"
    };
    fixtures().join(name).to_string_lossy().into_owned()
}

/// The recorded stream `file` of tests/fixtures/vibe/, whole (`{"ms", "dir", "msg"}` a line).
pub(crate) fn stream(file: &str) -> Vec<Value> {
    let text = std::fs::read_to_string(fixtures().join("vibe").join(file)).unwrap();
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .collect()
}

/// What Vibe said (`in`) in the recorded stream `file`.
pub(crate) fn recorded(file: &str) -> Vec<Value> {
    stream(file)
        .into_iter()
        .filter(|l| l["dir"] == "in")
        .map(|l| l["msg"].clone())
        .collect()
}

pub(crate) struct OnBench;

impl Target for OnBench {
    fn engine(&self) -> &'static dyn Engine {
        &VIBE
    }

    fn program(&self) -> Program {
        VIBE.locate(&fake_vibe(), "").expect("the fake vibe-acp")
    }

    /// Each agent's own `VIBE_HOME`, as the core gives it (`agent_home_var`).
    fn env(&self, dir: &Path) -> Vec<(String, String)> {
        vec![
            (
                "VIBE_HOME".into(),
                dir.join("vibe").to_string_lossy().into(),
            ),
            (
                "FAKE_VIBE_LOG".into(),
                dir.join("fake-vibe.jsonl").to_string_lossy().into(),
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
                // No quota in ACP: never played.
                Scenario::Limit => return None,
            }
            .to_string(),
        )
    }

    fn model(&self) -> Option<String> {
        Some(super::DEFAULT.into())
    }

    /// Its plan is asked in Escouade's mode Plan only (Vibe has none of its own).
    fn mode(&self, scenario: Scenario) -> Option<&'static str> {
        (scenario == Scenario::PlanMode).then_some("plan")
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_fake_vibe_passes_the_bench() {
    // Its conversation goes through its plugin (INT-10.6): the plugins the app gives, in a runtime of their own.
    let (rt, plugins, _) = crate::plugins_acp_tests::runtime("int10-vibe-bench");
    crate::plugins::harness::sync(&rt, &plugins);
    assert!(super::plugged().is_some(), "the plugin Mistral Vibe is on");
    let dir = bench_dir("esc266-bench-vibe");
    let played = run(&OnBench, &dir).await;
    // No quota in ACP: neither its limit nor its reading asked of it.
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
            Scenario::Fork,
        ]
    );
    rt.shutdown();
}
