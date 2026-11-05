//! Claude Code on the bench, through the fake `claude` (tests/fixtures/fake-claude.mjs), whose
//! message picks the scenario.

use crate::engine::bench::{bench_dir, replay, run, Scenario, Target};
use crate::engine::{Engine, Program, CLAUDE};

pub(crate) struct ClaudeCode;

impl Target for ClaudeCode {
    fn engine(&self) -> &'static dyn Engine {
        &CLAUDE
    }

    fn program(&self) -> Program {
        CLAUDE
            .locate(&crate::core_tests::fake_cli(), "")
            .expect("the fake claude")
    }

    fn prompt(&self, scenario: Scenario) -> Option<String> {
        Some(
            match scenario {
                Scenario::Text | Scenario::Resume | Scenario::Fork | Scenario::Quota => "bonjour",
                Scenario::Thinking => "raisonnement",
                Scenario::Approval => "permission",
                Scenario::Edit => "edit",
                Scenario::Question => "question",
                Scenario::Interrupt => "slow",
                Scenario::PlanMode => "exit plan",
                Scenario::Subagents => "sous-agent",
                Scenario::Limit => "limite",
            }
            .to_string(),
        )
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_fake_claude_passes_the_bench() {
    // Its conversation goes through its plugin (INT-10.5): the plugins the app gives, in a runtime of their own.
    let (rt, plugins, _) = crate::plugins_acp_tests::runtime("int10-claude-bench");
    crate::plugins::harness::sync(&rt, &plugins);
    let dir = bench_dir("esc35-bench-claude");
    let played = run(&ClaudeCode, &dir).await;
    // Claude Code can do everything the bench asks.
    assert_eq!(played, Scenario::ALL);
}

#[test]
fn the_recorded_flux_of_claude_code_make_the_items_expected() {
    replay::check_all(&CLAUDE, "claude");
}
