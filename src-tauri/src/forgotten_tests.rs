//! An agent its harness's plugin forgot (ESC-562): the plugin of Claude Code started anew without it, its next instance
//! started first by another caller, and the fake `claude` streaming meanwhile; and the end of a process whose why was
//! said already, not said twice. The plugin forgetting it by itself, or refusing it, is played on the demonstration
//! plugin (`plugins_harness_tests.rs`).

use crate::agent::{AgentRt, Effects};
use crate::board::TurnEnd;
use crate::core_tests::harness;
use crate::model::{AgentMeta, AgentStatus};
use crate::paths::test_dir;
use serde_json::Value;

/// What the core said for each batch its plugin refused before ESC-562.
const UNTRANSLATED: &str = "n’a pas traduit";

fn notices(items: &[Value]) -> Vec<String> {
    items
        .iter()
        .filter(|i| i["kind"] == "notice")
        .filter_map(|i| i["text"].as_str().map(str::to_string))
        .collect()
}

/// The capture of 2026-11-20: the plugin of Claude Code forgotten (updated, the language changed) while its agent's
/// harness is quiet in the middle of a turn, and its next instance started by someone else before the agent's next
/// batch (another agent, a reading of the quota). Before ESC-562 each batch said « n’a pas traduit … it was not
/// attached », the end of the turn among them, and the agent never ended its turn; now the next instance knows the
/// agent, and the turn ends as it would have. (A batch under way when the plugin is forgotten fails as a crash, which
/// gave the agent again before ESC-562 too.)
#[tokio::test(flavor = "multi_thread")]
async fn a_claude_code_agent_whose_plugin_started_anew_without_it_goes_on_with_its_turn() {
    let h = harness("esc562-claude-forgotten");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&id, "slow 6s".into(), vec![])
        .await
        .unwrap();
    h.wait("its first line", |h| {
        h.agent(&id).status == AgentStatus::Running
            && h.items(&id).iter().any(|i| {
                i["text"]
                    .as_str()
                    .is_some_and(|t| t.contains("Je commence"))
            })
    })
    .await;
    let plugin = crate::plugins::builtin::CLAUDE;
    h.core.plugin_runtime.forget(plugin);
    // Started again by another caller first, not by the agent.
    h.core.plugin_runtime.start(plugin).unwrap();
    h.wait("the end of the turn", |h| !h.agent(&id).status.is_active())
        .await;
    let items = h.items(&id);
    let said = notices(&items);
    assert!(!said.iter().any(|t| t.contains(UNTRANSLATED)), "{said:#?}");
    let turn = items.iter().rev().find(|i| i["kind"] == "turn").unwrap();
    assert_eq!(turn["isError"], false, "{turn}");
    assert_eq!(h.agent(&id).status, AgentStatus::Done);
    // And its next message goes.
    h.core
        .send_message(&id, "bonjour".into(), vec![])
        .await
        .unwrap();
    h.wait("its next turn", |h| {
        h.items(&id).iter().filter(|i| i["kind"] == "turn").count() == 2
            && !h.agent(&id).status.is_active()
    })
    .await;
}

/// A process ends on why its core stopped it, the line its conversation said just before (its plugin forgot it): the
/// turn ends on that line, not on a second one.
#[test]
fn an_end_whose_why_was_said_already_is_not_said_twice() {
    let dir = test_dir("esc562-said-once");
    let meta = AgentMeta {
        id: crate::model::new_id(),
        status: AgentStatus::Running,
        ..Default::default()
    };
    let mut rt = AgentRt::new(meta, &dir.join("conv"));
    rt.saw_init = true;
    rt.meta.status = AgentStatus::Running;
    let gen = rt.gen;
    let mut fx = Effects::default();
    let line =
        "Le plugin de Claude Code a oublié cet agent et ne l’a pas repris (x) : l’agent s’arrête.";
    rt.notice("error", line, &mut fx);
    let mut fx = Effects::default();
    rt.on_exit(gen, None, line, &mut fx);
    let said = notices(&rt.conv.items());
    assert_eq!(said, [line], "once");
    assert_eq!(rt.meta.status, AgentStatus::Error);
    assert!(
        matches!(&fx.turn_end, Some(TurnEnd::Error(why)) if why == line),
        "{:?}",
        fx.turn_end
    );

    // Another why than the line before: said as always.
    let meta = AgentMeta {
        id: crate::model::new_id(),
        status: AgentStatus::Running,
        ..Default::default()
    };
    let mut rt = AgentRt::new(meta, &dir.join("conv2"));
    rt.saw_init = true;
    rt.meta.status = AgentStatus::Running;
    let gen = rt.gen;
    let mut fx = Effects::default();
    rt.notice("error", line, &mut fx);
    rt.on_exit(gen, Some(3), "autre chose", &mut fx);
    let said = notices(&rt.conv.items());
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(said[1].contains("autre chose"), "{said:?}");
}
