//! Gemini CLI on the bench, through the fake `gemini --acp` (tests/fixtures/fake-gemini.mjs), whose
//! message picks the scenario, its conversation through its plugin (INT-10.7). The streams of the
//! study of Gemini CLI 0.66.0 (tests/fixtures/gemini/) are replayed by the plugin's client against
//! tests/fixtures/replays/gemini/ (`plugins_acp_tests`).

use super::GEMINI;
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

/// The fake `gemini` of the platform: its `.cmd` under Windows, its shell script elsewhere.
pub(crate) fn fake_gemini() -> String {
    let name = if cfg!(windows) {
        "fake-gemini.cmd"
    } else {
        "fake-gemini"
    };
    fixtures().join(name).to_string_lossy().into_owned()
}

/// The recorded stream `file` of tests/fixtures/gemini/, whole (`{"dir", …the message}` a line).
pub(crate) fn stream(file: &str) -> Vec<Value> {
    let text = std::fs::read_to_string(fixtures().join("gemini").join(file)).unwrap();
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .collect()
}

pub(crate) struct OnBench;

impl Target for OnBench {
    fn engine(&self) -> &'static dyn Engine {
        &GEMINI
    }

    fn program(&self) -> Program {
        GEMINI.locate(&fake_gemini(), "").expect("the fake gemini")
    }

    /// Each agent's own `GEMINI_CLI_HOME`, as the core gives it (`agent_home_var`).
    fn env(&self, dir: &Path) -> Vec<(String, String)> {
        vec![
            (
                "GEMINI_CLI_HOME".into(),
                dir.join("gemini").to_string_lossy().into(),
            ),
            (
                "FAKE_GEMINI_LOG".into(),
                dir.join("fake-gemini.jsonl").to_string_lossy().into(),
            ),
        ]
    }

    fn prompt(&self, scenario: Scenario) -> Option<String> {
        Some(
            match scenario {
                Scenario::Text | Scenario::Resume => "bonjour",
                Scenario::Thinking => "raisonnement",
                Scenario::Approval => "commande",
                Scenario::Edit => "fichier",
                Scenario::Question => "question",
                Scenario::Interrupt => "lent",
                Scenario::PlanMode => "propose un plan",
                Scenario::Subagents => "sous-agent",
                // No copy nor quota in ACP: never played.
                Scenario::Fork | Scenario::Limit | Scenario::Quota => return None,
            }
            .to_string(),
        )
    }

    fn model(&self) -> Option<String> {
        Some(super::DEFAULT.into())
    }

    /// Its plan in its mode plan; its subagents in its mode yolo only (refused by its rules
    /// otherwise, as Gemini's).
    fn mode(&self, scenario: Scenario) -> Option<&'static str> {
        match scenario {
            Scenario::PlanMode => Some("plan"),
            Scenario::Subagents => Some("bypassPermissions"),
            _ => None,
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_fake_gemini_passes_the_bench() {
    // Its conversation goes through its plugin (INT-10.7): the plugins the app gives, in a runtime of their own.
    let (rt, plugins, _) = crate::plugins_acp_tests::runtime("int10-gemini-bench");
    crate::plugins::harness::sync(&rt, &plugins);
    assert!(super::plugged().is_some(), "the plugin Gemini CLI is on");
    let dir = bench_dir("esc274-bench-gemini");
    let played = run(&OnBench, &dir).await;
    // No copy nor quota in ACP: neither asked of it.
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
    rt.shutdown();
}
