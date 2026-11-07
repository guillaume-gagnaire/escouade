//! Cursor on the bench, through the fake `agent acp` (tests/fixtures/fake-cursor.mjs), whose message
//! picks the scenario, its conversation through its plugin (INT-10.1). The real streams of Cursor CLI
//! 2026.10.22 (tests/fixtures/cursor/) are replayed by the plugin's client against
//! tests/fixtures/replays/cursor/ (`plugins_acp_tests`).

use super::CURSOR;
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

/// The fake `agent` of the platform: its `.cmd` under Windows, its shell script elsewhere.
pub(crate) fn fake_cursor() -> String {
    let name = if cfg!(windows) {
        "fake-cursor.cmd"
    } else {
        "fake-cursor"
    };
    fixtures().join(name).to_string_lossy().into_owned()
}

/// The recorded stream `file` of tests/fixtures/cursor/, whole (`{"dir", "t", "msg"}` a line).
pub(crate) fn stream(file: &str) -> Vec<Value> {
    let text = std::fs::read_to_string(fixtures().join("cursor").join(file)).unwrap();
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .collect()
}

/// What Cursor said (`s2c`) in the recorded stream `file`.
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
        &CURSOR
    }

    fn program(&self) -> Program {
        CURSOR.locate(&fake_cursor(), "").expect("the fake agent")
    }

    /// Each agent's own configuration folder, as the core gives it (`agent_home_var`).
    fn env(&self, dir: &Path) -> Vec<(String, String)> {
        vec![
            (
                "CURSOR_CONFIG_DIR".into(),
                dir.join("cursor").to_string_lossy().into(),
            ),
            (
                "FAKE_CURSOR_LOG".into(),
                dir.join("fake-cursor.jsonl").to_string_lossy().into(),
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
        Some(super::AUTO.into())
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_fake_cursor_passes_the_bench() {
    // Its conversation goes through its plugin (INT-10.1): the plugins the app gives, in a runtime of their own.
    let (rt, plugins, _) = crate::plugins_acp_tests::runtime("int10-cursor-bench");
    crate::plugins::harness::sync(&rt, &plugins);
    assert!(super::plugged().is_some(), "the plugin Cursor CLI is on");
    let dir = bench_dir("esc42-bench-cursor");
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
