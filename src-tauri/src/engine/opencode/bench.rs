//! OpenCode on the bench, through the fake `opencode serve` (tests/fixtures/fake-opencode.mjs),
//! whose message picks the scenario; and the real streams of OpenCode 1.19.3
//! (tests/fixtures/opencode/), replayed from tests/fixtures/replays/opencode/.

use super::OPENCODE;
use crate::agent::{AgentRt, Effects};
use crate::engine::bench::{bench_dir, replay, run, Scenario, Target};
use crate::engine::{Engine, Program};
use escouade_plugin_protocol::engine::method;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
}

/// The fake `opencode` of the platform: its `.cmd` under Windows, its shell script elsewhere.
pub(crate) fn fake_opencode() -> String {
    let name = if cfg!(windows) {
        "fake-opencode.cmd"
    } else {
        "fake-opencode"
    };
    fixtures().join(name).to_string_lossy().into_owned()
}

/// What OpenCode said (`s2c`) in the recorded stream `file` of tests/fixtures/opencode/.
pub(crate) fn recorded(file: &str) -> Vec<Value> {
    let text = std::fs::read_to_string(fixtures().join("opencode").join(file)).unwrap();
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .filter(|l| l["dir"] == "s2c")
        .map(|l| l["msg"].clone())
        .collect()
}

/// The translation of its module: a frame of OpenCode's, its items applied to the agent.
#[derive(Default)]
pub(crate) struct Through(Option<String>);

impl Through {
    pub(crate) fn frame(&mut self, rt: &mut AgentRt, frame: &Value, fx: &mut Effects) {
        let plugin = crate::plugins::builtin::OPENCODE;
        let agent = match &self.0 {
            Some(agent) => agent.clone(),
            None => {
                let agent = uuid::Uuid::new_v4().simple().to_string();
                let attach = json!({
                    "agent": agent, "cwd": rt.meta.cwd, "model": rt.meta.model,
                    "session": rt.meta.session_id, "lang": "fr",
                });
                crate::plugins::view::call(plugin, method::ATTACH, &attach).unwrap();
                self.0.insert(agent).clone()
            }
        };
        let frames = json!({ "agent": agent, "frames": [frame.to_string()] });
        let answer = crate::plugins::view::call(plugin, method::FRAMES, &frames).unwrap();
        rt.apply_items(answer["items"].as_array().unwrap(), fx);
    }
}

pub(crate) struct OnBench;

impl Target for OnBench {
    fn engine(&self) -> &'static dyn Engine {
        &OPENCODE
    }

    fn program(&self) -> Program {
        OPENCODE
            .locate(&fake_opencode(), "")
            .expect("the fake opencode")
    }

    /// Each scenario its own server (its folder is in its variables): a fake of its own.
    fn env(&self, dir: &Path) -> Vec<(String, String)> {
        vec![(
            "FAKE_OPENCODE_LOG".into(),
            dir.join("fake-opencode.jsonl").to_string_lossy().into(),
        )]
    }

    fn model(&self) -> Option<String> {
        Some("ollama/qwen3-coder:30b-32k".into())
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
async fn the_fake_opencode_passes_the_bench() {
    // Its conversation goes through its plugin (INT-10.2): the plugins the app gives, in a runtime of their own.
    let (rt, plugins, _) = crate::plugins_acp_tests::runtime("int10-opencode-bench");
    crate::plugins::harness::sync(&rt, &plugins);
    let dir = bench_dir("esc44-bench-opencode");
    let played = run(&OnBench, &dir).await;
    // No quota: neither the limit nor the quota is asked of it.
    let expected: Vec<Scenario> = Scenario::ALL
        .into_iter()
        .filter(|s| !matches!(s, Scenario::Limit | Scenario::Quota))
        .collect();
    assert_eq!(played, expected);
    // Every agent stopped, its server too.
    assert_eq!(crate::plugins::shared::running(), 0);
    rt.shutdown();
}

#[test]
fn the_recorded_streams_of_opencode_make_the_items_expected() {
    replay::check_all_through(&OPENCODE, "opencode", || {
        let mut through = Through::default();
        Box::new(move |rt, frame, fx| through.frame(rt, frame, fx))
    });
}
