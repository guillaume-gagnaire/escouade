//! Tests of the tracking of agents in their project's workflow (ESC-193): the facts of a
//! conversation, each kind of fact against the steps of the frozen copies of GSD and Ralph, several
//! agents in one workflow, the history kept after a restart, an agent outside the workflow, a
//! workflow that changes under a step, and what each harness lets be followed. `Fake` is the fake
//! agent: it plays the facts of a run, the way `fake-claude.mjs` plays them for the e2e.

use crate::board::TurnEnd;
use crate::model::{ConvOp, EngineId};
use crate::workflow::*;
use crate::workflow_track::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures/workflows")
        .join(name)
}

fn roots(name: &str) -> Roots {
    let home = fixture(name).join("home");
    Roots {
        claude: Some(home.join(".claude")),
        codex: Some(home.join(".codex")),
        home,
    }
}

fn gsd() -> Scan {
    read(&fixture("gsd/project"), &[], &roots("gsd"), &[])
}

fn ralph() -> Scan {
    read(&fixture("ralph/project"), &[], &roots("none"), &[])
}

/// The fake agent: plays the facts of a run on a track, a clock of its own ticking by seconds.
struct Fake {
    scan: Scan,
    track: AgentTrack,
    now: i64,
}

impl Fake {
    fn new(scan: Scan) -> Self {
        Fake::on(scan, "a1", EngineId::CLAUDE, false)
    }

    fn on(scan: Scan, id: &str, engine: EngineId, view: bool) -> Self {
        Fake {
            scan,
            track: AgentTrack::new(id, engine, view),
            now: 1_000_000,
        }
    }

    fn play(&mut self, fact: Fact) -> bool {
        self.now += 1000;
        self.track.apply(&self.scan, "", &fact, self.now)
    }

    fn says(&mut self, text: &str) -> bool {
        self.play(Fact::Message {
            text: text.to_string(),
        })
    }

    fn tool(&mut self, name: &str, input: Value) -> bool {
        self.play(Fact::Tool {
            name: name.to_string(),
            input,
        })
    }

    fn launches(&mut self, subagent: &str) -> bool {
        self.tool("Agent", json!({"subagent_type": subagent, "prompt": "go"}))
    }

    fn writes(&mut self, file: &str) -> bool {
        self.tool("Write", json!({"file_path": file}))
    }

    fn ends(&mut self, cost: f64, text: &str) {
        self.play(Fact::Cost(cost));
        self.play(Fact::TurnEnd {
            end: End::Finished,
            text: text.to_string(),
        });
    }

    fn step(&self) -> Option<&str> {
        self.track.step.as_deref()
    }

    fn last(&self) -> &Passage {
        self.track.passages.last().unwrap()
    }
}

// ---------- the facts of a conversation ----------

fn append(item: Value) -> ConvOp {
    ConvOp::Append { item }
}

#[test]
fn the_facts_of_a_conversation_are_the_messages_and_the_tools_of_the_main_thread() {
    let mut pending = HashMap::new();
    let facts = facts_of_ops(
        &[
            append(json!({"kind": "user", "id": "u1", "text": "/gsd:plan-phase 2"})),
            append(json!({"kind": "text", "id": "t1", "text": "ok"})),
            append(
                json!({"kind": "tool", "id": "c1", "name": "Skill", "input": {"skill": "brainstorming"}, "parent": null}),
            ),
            // A tool of a sub-agent is not the agent's own.
            append(
                json!({"kind": "tool", "id": "c2", "name": "Write", "input": {"file_path": "x.md"}, "parent": "c9"}),
            ),
            ConvOp::Delta {
                id: "t1".into(),
                text: "…".into(),
            },
            // A notification is no message.
            append(
                json!({"kind": "user", "id": "u2", "text": "done", "origin": "task-notification"}),
            ),
            append(json!({"kind": "user", "id": "u3", "text": "   "})),
        ],
        &mut pending,
    );
    assert_eq!(
        facts,
        [
            Fact::Message {
                text: "/gsd:plan-phase 2".into()
            },
            Fact::Tool {
                name: "Skill".into(),
                input: json!({"skill": "brainstorming"})
            },
        ]
    );
}

#[test]
fn a_streaming_tool_is_a_fact_once_its_input_is_there_and_a_subagent_when_it_comes_back() {
    let mut pending = HashMap::new();
    let first = facts_of_ops(
        &[append(
            json!({"kind": "tool", "id": "c1", "name": "Agent", "input": {}, "status": "running", "parent": null}),
        )],
        &mut pending,
    );
    assert!(first.is_empty());
    let second = facts_of_ops(
        &[ConvOp::Patch {
            id: "c1".into(),
            patch: json!({"input": {"subagent_type": "gsd-planner", "prompt": "x"}}),
        }],
        &mut pending,
    );
    assert_eq!(
        second,
        [Fact::Tool {
            name: "Agent".into(),
            input: json!({"subagent_type": "gsd-planner", "prompt": "x"})
        }]
    );
    // Still running: nothing. Over: the sub-agent is back.
    assert!(facts_of_ops(
        &[ConvOp::Patch {
            id: "c1".into(),
            patch: json!({"status": "running"})
        }],
        &mut pending
    )
    .is_empty());
    let third = facts_of_ops(
        &[ConvOp::Patch {
            id: "c1".into(),
            patch: json!({"status": "ok"}),
        }],
        &mut pending,
    );
    assert_eq!(
        third,
        [Fact::SubagentDone {
            name: "gsd-planner".into()
        }]
    );
    // Told once.
    assert!(facts_of_ops(
        &[ConvOp::Patch {
            id: "c1".into(),
            patch: json!({"status": "ok"})
        }],
        &mut pending
    )
    .is_empty());
}

// ---------- each kind of fact ----------

#[test]
fn a_door_typed_begins_the_step_it_names_with_its_time_and_the_next_door_ends_it() {
    let mut a = Fake::new(gsd());
    assert!(a.track.passages.is_empty());
    assert!(a.says("/gsd:discuss-phase 2"));
    assert_eq!(a.step(), Some("gsd:discuss-phase"));
    assert_eq!(
        a.track.workflow.as_deref(),
        Some(a.scan.workflows[0].id.as_str())
    );
    let began = a.last().at;
    a.ends(0.10, "Context written");
    assert!(a.says("/gsd:plan-phase 2"));
    assert_eq!(a.step(), Some("gsd:plan-phase"));
    let first = &a.track.passages[0];
    assert_eq!(first.outcome, Outcome::Done);
    assert_eq!(first.at, began);
    assert_eq!(first.ended, Some(a.last().at));
    assert!(first.ms >= 2000, "{}", first.ms);
    assert!((first.cost - 0.10).abs() < 1e-9);
    assert_eq!(first.door.as_deref(), Some("/gsd:discuss-phase"));
    assert_eq!(a.last().outcome, Outcome::Running);
    assert_eq!(a.last().pass, 1);
}

#[test]
fn a_subagent_launched_signals_its_step_and_runs_until_it_comes_back() {
    let mut a = Fake::new(gsd());
    a.says("/gsd:discuss-phase");
    // The planner belongs to the planning step only: the agent is there without a door.
    assert!(a.launches("gsd-planner"));
    assert_eq!(a.step(), Some("gsd:plan-phase"));
    assert_eq!(a.track.subagents, ["gsd-planner"]);
    assert!(a.last().did.contains(&"agent:gsd-planner".to_string()));
    a.play(Fact::SubagentDone {
        name: "gsd-planner".into(),
    });
    assert!(a.track.subagents.is_empty());
    // A sub-agent no step has changes no step.
    a.launches("Explore");
    assert_eq!(a.step(), Some("gsd:plan-phase"));
    assert_eq!(a.track.passages.len(), 2);
}

#[test]
fn a_skill_called_and_a_state_file_written_signal_their_steps() {
    let scan = read(&fixture("superpowers/project"), &[], &roots("none"), &[]);
    let mut a = Fake::new(scan);
    assert!(a.tool("Skill", json!({"skill": "superpowers:brainstorming"})));
    assert_eq!(a.step(), Some("brainstorming"));
    assert!(a.tool("Skill", json!({"skill": "writing-plans"})));
    assert_eq!(a.step(), Some("writing-plans"));
    assert!(a.last().did.is_empty() || a.last().did.contains(&"skill:writing-plans".to_string()));
    assert_eq!(a.track.passages.len(), 2);

    // A file written: the step that writes it.
    let mut g = Fake::new(gsd());
    g.says("/gsd:discuss-phase");
    let wf = &g.scan.workflows[0];
    let (name, file) = wf
        .steps
        .iter()
        .find(|s| s.name == "gsd:plan-phase")
        .map(|s| (s.name.clone(), s.writes.first().cloned()))
        .unwrap();
    let file = file.expect("the planning step writes a file");
    assert!(g.writes(&format!("C:\\work\\{}", file.replace('/', "\\"))));
    assert_eq!(g.step(), Some(name.as_str()));
    assert!(g.last().did.iter().any(|d| d.starts_with("file:")));
}

#[test]
fn the_cost_and_the_time_of_a_step_are_those_of_the_turns_between() {
    let mut a = Fake::new(gsd());
    a.says("/gsd:plan-phase");
    a.ends(0.25, "Planned");
    a.ends(0.05, "Checked");
    a.says("/gsd:execute-phase");
    a.ends(1.0, "Done");
    let plan = &a.track.passages[0];
    assert!((plan.cost - 0.30).abs() < 1e-9);
    assert!((a.track.cost - 1.30).abs() < 1e-9);
    assert!((a.last().cost - 1.0).abs() < 1e-9);
    assert!(a.track.ms > 0);
}

#[test]
fn a_way_back_keeps_the_reason_the_step_gave_and_the_file_that_decided() {
    let mut a = Fake::new(gsd());
    a.says("/gsd:verify-work 3");
    a.ends(
        0.1,
        "2 gaps: the login test is missing, the logout is not wired",
    );
    a.says("/gsd:plan-phase 3 --gaps");
    let left = &a.track.passages[0];
    assert_eq!(left.outcome, Outcome::Back);
    assert_eq!(left.back_to.as_deref(), Some("gsd:plan-phase"));
    assert_eq!(
        left.reason.as_deref(),
        Some("2 gaps: the login test is missing, the logout is not wired")
    );
    assert!(left.decided_by.as_deref().unwrap().contains("verify-work"));
    // Again to verify, and back again: the planning step is on its second pass.
    a.says("/gsd:verify-work 3");
    a.says("/gsd:plan-phase 3 --gaps");
    assert_eq!(a.last().pass, 3 - 1);
    assert_eq!(a.last().step.as_deref(), Some("gsd:plan-phase"));
}

#[test]
fn a_way_back_without_a_turn_says_the_condition_the_step_wrote() {
    let mut a = Fake::new(gsd());
    a.says("/gsd:verify-work");
    a.says("/gsd:plan-phase");
    let reason = a.track.passages[0].reason.clone().unwrap();
    assert!(reason.contains("--gaps"), "{reason}");
}

#[test]
fn the_same_door_typed_again_is_a_new_pass() {
    let mut a = Fake::new(gsd());
    a.says("/gsd:plan-phase 1");
    a.says("/gsd:plan-phase 2");
    assert_eq!(a.track.passages.len(), 2);
    assert_eq!(a.track.passages[1].pass, 2);
    // GSD revises its plan up to three times: planning twice is a way back to itself.
    assert_eq!(a.track.passages[0].outcome, Outcome::Back);
}

#[test]
fn a_failed_or_stopped_turn_is_the_outcome_of_the_passage() {
    let mut a = Fake::new(gsd());
    a.says("/gsd:plan-phase");
    a.play(Fact::TurnEnd {
        end: End::Failed,
        text: "API error".into(),
    });
    assert_eq!(a.last().outcome, Outcome::Failed);
    a.says("/gsd:execute-phase");
    // Failed stays failed once it is left.
    assert_eq!(a.track.passages[0].outcome, Outcome::Failed);
    a.play(Fact::TurnEnd {
        end: End::Stopped,
        text: String::new(),
    });
    assert_eq!(a.last().outcome, Outcome::Stopped);
}

#[test]
fn the_ticket_loop_is_kept_beside_the_step() {
    let mut a = Fake::new(gsd());
    a.says("/gsd:plan-phase");
    assert!(a.play(Fact::TicketLoop { n: 2, max: 5 }));
    assert_eq!(a.track.ticket_loop, Some((2, 5)));
    assert!(!a.play(Fact::TicketLoop { n: 2, max: 5 }));
}

// ---------- several agents ----------

#[test]
fn several_agents_in_the_same_workflow_each_have_their_step_and_their_history() {
    let scan = gsd();
    let mut a = Fake::on(scan.clone(), "a1", EngineId::CLAUDE, false);
    let mut b = Fake::on(scan.clone(), "a2", EngineId::CLAUDE, false);
    let mut c = Fake::on(scan, "a3", EngineId::OPENCODE, false);
    a.says("/gsd:plan-phase");
    b.says("/gsd:plan-phase");
    c.says("/gsd:execute-phase");
    b.ends(0.5, "ok");
    b.says("/gsd:execute-phase");
    // Two on the same step stack; nothing locks a step.
    assert_eq!(a.step(), Some("gsd:plan-phase"));
    assert_eq!(b.step(), Some("gsd:execute-phase"));
    assert_eq!(c.step(), Some("gsd:execute-phase"));
    assert_eq!(a.track.passages.len(), 1);
    assert_eq!(b.track.passages.len(), 2);
    assert_eq!(c.track.passages.len(), 1);
    assert_eq!(a.track.cost, 0.0);
    assert!((b.track.cost - 0.5).abs() < 1e-9);
}

// ---------- outside the workflow ----------

#[test]
fn an_agent_whose_turn_does_something_else_is_outside_until_the_next_door() {
    let mut a = Fake::new(gsd());
    a.says("/gsd:execute-phase 3");
    // A message from no door, and a turn of tools none of which signals a step.
    assert!(!a.says("lance la suite e2e, s'il te plaît"));
    a.tool("Bash", json!({"command": "npm run e2e"}));
    a.tool("Read", json!({"file_path": "tests/e2e/a.spec.ts"}));
    assert_eq!(a.step(), Some("gsd:execute-phase"));
    a.ends(0.2, "e2e green");
    assert_eq!(a.step(), None);
    assert_eq!(
        a.track.outside.as_deref(),
        Some("lance la suite e2e, s'il te plaît")
    );
    let out = a.last();
    assert!(out.step.is_none());
    assert_eq!(
        out.doing.as_deref(),
        Some("lance la suite e2e, s'il te plaît")
    );
    assert_eq!(a.track.passages[0].outcome, Outcome::Left);
    // It comes back by the next door.
    a.says("/gsd:verify-work 3");
    assert_eq!(a.step(), Some("gsd:verify-work"));
    assert!(a.track.outside.is_none());
    assert_eq!(a.track.passages.len(), 3);
    assert!(a.track.passages[1].ms > 0);
}

#[test]
fn one_command_alone_or_a_message_without_tools_never_takes_an_agent_out() {
    let mut a = Fake::new(gsd());
    a.says("/gsd:execute-phase");
    // No tool at all: a question in the conversation.
    a.says("c'est où, la config ?");
    a.ends(0.01, "Là.");
    assert_eq!(a.step(), Some("gsd:execute-phase"));
    // A tool that signals the step it is in.
    a.says("continue");
    a.launches("gsd-executor");
    a.tool("Bash", json!({"command": "ls"}));
    a.ends(0.01, "ok");
    assert_eq!(a.step(), Some("gsd:execute-phase"));
    assert!(a.track.outside.is_none());
}

#[test]
fn an_agent_that_never_met_a_door_is_in_no_workflow() {
    let mut a = Fake::new(gsd());
    a.says("fais ceci");
    a.tool("Bash", json!({"command": "ls"}));
    a.ends(0.1, "fait");
    assert!(a.track.workflow.is_none());
    assert!(a.track.passages.is_empty());
    assert!(a.track.outside.is_none());
}

// ---------- a workflow that changes ----------

#[test]
fn a_workflow_that_changes_is_behind_for_the_step_in_course_and_taken_at_the_next_step() {
    let mut a = Fake::new(gsd());
    a.says("/gsd:plan-phase");
    let old = a.track.version.clone();
    assert!(!a.track.behind(&a.scan));
    // The reading finds a changed workflow: one step gained an agent.
    let mut changed = a.scan.clone();
    changed.workflows[0].steps[1].agents.push(StepAgent {
        name: "gsd-new".into(),
        path: None,
        description: String::new(),
    });
    assert_ne!(version_of(&changed.workflows[0]), old);
    assert!(a.track.behind(&changed));
    assert!(a.track.view("p1", &changed).behind);
    // The agent keeps its version for the step in course, then takes the new one.
    a.scan = changed;
    a.ends(0.1, "ok");
    assert_eq!(a.last().version, old);
    a.says("/gsd:execute-phase");
    assert_ne!(a.last().version, old);
    assert_eq!(a.track.passages[0].version, old);
    assert!(!a.track.behind(&a.scan));
}

#[test]
fn a_step_that_is_gone_from_the_workflow_is_left_by_the_next_fact() {
    let mut a = Fake::new(gsd());
    a.says("/gsd:execute-phase");
    a.scan.workflows[0]
        .steps
        .retain(|s| s.name != "gsd:execute-phase");
    assert!(a.track.behind(&a.scan));
    a.says("/gsd:verify-work");
    assert_eq!(a.step(), Some("gsd:verify-work"));
    assert_eq!(
        a.track.passages[0].step.as_deref(),
        Some("gsd:execute-phase")
    );
    assert_eq!(a.track.passages[0].outcome, Outcome::Done);
}

// ---------- what each harness lets be followed ----------

#[test]
fn the_tracking_is_partial_for_the_harnesses_that_do_not_tell_everything() {
    let full = |e, v| support_of(e, v);
    assert!(!full(EngineId::CLAUDE, false).partial);
    assert!(!full(EngineId::CLAUDE, true).partial);
    assert!(!full(EngineId::OPENCODE, false).partial);
    assert!(!full(EngineId::COPILOT, false).partial);
    let codex = full(EngineId::CODEX, false);
    assert!(codex.partial);
    assert_eq!(codex.missing, ["subagents"]);
    let cursor = full(EngineId::CURSOR, false);
    assert!(cursor.partial);
    assert_eq!(cursor.missing, ["cost"]);
    let view = full(EngineId::CURSOR, true);
    assert_eq!(view.missing, ["subagents", "cost", "timing"]);
    let a = Fake::on(gsd(), "x", EngineId::CODEX, false);
    assert!(a.track.support.partial);
}

// ---------- a prompt run over and over ----------

#[test]
fn a_prompt_run_over_and_over_is_followed_by_its_turns_and_its_list() {
    let mut a = Fake::new(ralph());
    assert!(a.says("Read PROMPT.md and do the first item of fix_plan.md"));
    assert_eq!(a.track.turns.len(), 1);
    assert!(a.track.step.is_none());
    assert_eq!(
        a.track.workflow.as_deref(),
        Some(a.scan.workflows[0].id.as_str())
    );
    a.ends(0.3, "Item done");
    a.play(Fact::List(Progress { done: 4, total: 6 }));
    // The next turn checked nothing.
    a.says("Read PROMPT.md and do the first item of fix_plan.md");
    a.ends(0.2, "Red tests");
    a.play(Fact::List(Progress { done: 4, total: 6 }));
    a.says("/loop PROMPT.md");
    a.ends(0.1, "Item done");
    a.play(Fact::List(Progress { done: 5, total: 6 }));
    let t = &a.track.turns;
    assert_eq!(t.len(), 3);
    assert_eq!(t.iter().map(|x| x.n).collect::<Vec<_>>(), [1, 2, 3]);
    assert!(t.iter().all(|x| x.ended.is_some() && x.ms > 0));
    assert_eq!(t[0].checked, Some(Progress { done: 4, total: 6 }));
    assert_eq!(t[0].ticked, Some(true));
    assert_eq!(t[1].ticked, Some(false));
    assert_eq!(t[2].ticked, Some(true));
    assert!((t[0].cost - 0.3).abs() < 1e-9);
    assert!((a.track.cost - 0.6).abs() < 1e-9);
    // A turn is not a passage in a step.
    assert!(a.track.passages.is_empty());
}

#[test]
fn a_message_in_a_prompt_loop_is_a_turn_and_never_takes_the_agent_out() {
    let mut a = Fake::new(ralph());
    a.says("PROMPT.md");
    a.ends(0.1, "ok");
    a.says("fais autre chose");
    a.tool("Bash", json!({"command": "ls"}));
    a.ends(0.1, "ok");
    assert!(a.track.outside.is_none());
    assert_eq!(a.track.turns.len(), 2);
}

// ---------- the history after a restart, and the window ----------

#[test]
fn a_track_is_written_and_read_again_as_it_was() {
    let mut a = Fake::new(gsd());
    a.says("/gsd:verify-work");
    a.ends(0.4, "1 gap: x");
    a.says("/gsd:plan-phase");
    a.says("lance les tests");
    a.tool("Bash", json!({"command": "npm test"}));
    a.ends(0.1, "ok");
    let json = serde_json::to_vec(&a.track).unwrap();
    let back: AgentTrack = serde_json::from_slice(&json).unwrap();
    assert_eq!(back, a.track);
    // A track written by an older version, with fewer fields, still reads.
    let old: AgentTrack = serde_json::from_value(json!({"agentId": "x", "passages": []})).unwrap();
    assert_eq!(old.agent_id, "x");
}

#[test]
fn the_history_keeps_the_most_recent_passages_only() {
    let mut a = Fake::new(gsd());
    for i in 0..(MAX_HISTORY + 20) {
        a.says(if i % 2 == 0 {
            "/gsd:plan-phase"
        } else {
            "/gsd:execute-phase"
        });
    }
    assert_eq!(a.track.passages.len(), MAX_HISTORY);
    // Counted from the first ever, not from the first kept.
    assert_eq!(
        a.track.passages.last().unwrap().n as usize,
        MAX_HISTORY + 20
    );
    assert_eq!(a.track.passages[0].n as usize, 21);
}

// ---------- in the core ----------

fn ops_user(text: &str) -> Vec<ConvOp> {
    vec![append(
        json!({"kind": "user", "id": format!("u{text}"), "text": text}),
    )]
}

fn ops_agent(sub: &str, id: &str) -> Vec<ConvOp> {
    vec![append(
        json!({"kind": "tool", "id": id, "name": "Agent", "input": {"subagent_type": sub, "prompt": "p"}, "parent": null}),
    )]
}

fn events(h: &crate::core_tests::Harness) -> Vec<Value> {
    h.events
        .lock()
        .iter()
        .filter(|e| e["type"] == "workflowTrack")
        .cloned()
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_core_tracks_an_agent_keeps_its_history_after_a_restart_and_forgets_it_with_it() {
    let h = crate::core_tests::harness("workflow-track-core");
    let (p, dir) = h.project(false).await;
    let cmds = dir.join(".claude/commands");
    std::fs::create_dir_all(&cmds).unwrap();
    std::fs::write(
        cmds.join("one.md"),
        "Do one with subagent_type=\"worker\", then /two.\n",
    )
    .unwrap();
    std::fs::write(
        cmds.join("two.md"),
        "Do two. Back to /one when red, up to 3 times.\n",
    )
    .unwrap();
    let scan = h.core.refresh_workflows(&p.id, true).await.unwrap();
    assert_eq!(scan.workflows.len(), 1, "{scan:#?}");
    let agent = crate::core_tests::agent_on(&h, &p, "principal").await;
    let id = agent.id.clone();

    // An agent that follows no door: nothing is said, nothing is kept.
    h.core
        .track_workflow(&id, &p.id, &ops_user("bonjour"), 0.0, None);
    assert!(events(&h).is_empty());
    assert!(h.core.workflow_tracks(&p.id).is_empty());

    h.core
        .track_workflow(&id, &p.id, &ops_user("/one"), 0.0, None);
    h.core
        .track_workflow(&id, &p.id, &ops_agent("worker", "c1"), 0.0, None);
    h.core.track_workflow(
        &id,
        &p.id,
        &[],
        0.42,
        Some(&TurnEnd::Finished("fini".into())),
    );
    h.core
        .track_workflow(&id, &p.id, &ops_user("/two"), 0.0, None);

    let tracks = h.core.workflow_tracks(&p.id);
    assert_eq!(tracks.len(), 1);
    let t = &tracks[0].track;
    assert_eq!(t.agent_id, id);
    assert_eq!(t.step.as_deref(), Some("two"));
    assert_eq!(t.passages.len(), 2);
    assert!((t.passages[0].cost - 0.42).abs() < 1e-9);
    assert_eq!(t.passages[0].did, ["agent:worker"]);
    assert!(!tracks[0].behind);

    // What the window is told: only when something showed changed (the door, the sub-agent, the
    // cost of the turn, the second door), not once per fact.
    let told = events(&h);
    assert!(!told.is_empty() && told.len() <= 5, "{}", told.len());
    let last = told.last().unwrap();
    assert_eq!(last["track"]["agentId"], json!(id));
    assert_eq!(last["track"]["step"], "two");
    assert_eq!(last["track"]["projectId"], json!(p.id));

    // The file is the data folder's, never the project's.
    let file = h.core.data.workflow_tracks_file(&p.id);
    assert!(file.is_file());
    assert!(file.starts_with(h.core.data.root()));
    assert!(!file.starts_with(&dir));

    // A restart: nothing in memory, the history is read again from the file.
    *h.core.wf_tracks.lock() = Default::default();
    let again = h.core.workflow_tracks(&p.id);
    assert_eq!(again.len(), 1);
    assert_eq!(again[0].track.passages, t.passages);
    assert_eq!(again[0].track.step.as_deref(), Some("two"));
    // And it goes on from there.
    h.core
        .track_workflow(&id, &p.id, &ops_user("/one"), 0.0, None);
    let more = h.core.workflow_tracks(&p.id);
    assert_eq!(more[0].track.passages.len(), 3);
    assert_eq!(more[0].track.passages[1].outcome, Outcome::Back);

    // A workflow that changes: told to the agent that follows it, which keeps its version.
    let before = events(&h).len();
    std::fs::write(
        cmds.join("two.md"),
        "Do two with subagent_type=\"other\". Back to /one.\n",
    )
    .unwrap();
    h.core.refresh_workflows(&p.id, true).await.unwrap();
    let after = events(&h);
    assert!(after.len() > before);
    assert_eq!(after.last().unwrap()["track"]["behind"], true);

    // Deleted with its agent.
    h.core.delete_agent(&id, true).await.unwrap();
    assert!(h.core.workflow_tracks(&p.id).is_empty());
    let kept: HashMap<String, AgentTrack> =
        serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert!(kept.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_fake_claude_plays_the_facts_of_a_workflow_through_the_whole_path() {
    let h = crate::core_tests::harness("workflow-track-fake");
    let (p, dir) = h.project(false).await;
    let cmds = dir.join(".claude/commands");
    std::fs::create_dir_all(&cmds).unwrap();
    std::fs::write(
        cmds.join("plan.md"),
        "Plan it with subagent_type=\"planner\", write `.planning/STATE.md`, then /build.\n",
    )
    .unwrap();
    std::fs::write(
        cmds.join("build.md"),
        "Build it with subagent_type=\"builder\".\n",
    )
    .unwrap();
    let scan = h.core.refresh_workflows(&p.id, true).await.unwrap();
    assert_eq!(scan.workflows.len(), 1, "{scan:#?}");
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let done = |h: &crate::core_tests::Harness, n: usize| {
        h.core
            .workflow_tracks(&p.id)
            .first()
            .is_some_and(|t| t.track.passages.len() >= n)
            && !h.agent(&id).status.is_active()
    };
    h.core
        .send_message(
            &id,
            "/plan 1 faits-workflow: [{\"agent\":\"planner\"},{\"write\":\".planning/STATE.md\"}]"
                .into(),
            vec![],
        )
        .await
        .unwrap();
    h.wait("the planning step", |h| done(h, 1)).await;
    h.core
        .send_message(
            &id,
            "/build faits-workflow: [{\"agent\":\"builder\"},{\"bash\":\"npm test\"}]".into(),
            vec![],
        )
        .await
        .unwrap();
    h.wait("the building step", |h| done(h, 2)).await;
    // Something else, with tools and no door: outside.
    h.core
        .send_message(
            &id,
            "lance autre chose faits-workflow: [{\"bash\":\"ls\"}]".into(),
            vec![],
        )
        .await
        .unwrap();
    for _ in 0..300 {
        if done(&h, 3) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(
        done(&h, 3),
        "{}",
        serde_json::to_string_pretty(&h.core.workflow_tracks(&p.id)[0].track).unwrap()
    );

    let t = &h.core.workflow_tracks(&p.id)[0].track;
    assert_eq!(t.passages[0].step.as_deref(), Some("plan"));
    assert!(t.passages[0]
        .did
        .iter()
        .any(|d| d.starts_with("file:") && d.ends_with(".planning/STATE.md")));
    assert!(t.passages[0].did.contains(&"agent:planner".to_string()));
    assert_eq!(t.passages[0].outcome, Outcome::Done);
    assert!(t.passages[0].cost > 0.0);
    assert_eq!(t.passages[1].step.as_deref(), Some("build"));
    assert!(t.passages[1].did.contains(&"agent:builder".to_string()));
    assert!(t.passages[2].step.is_none());
    assert!(t.passages[2]
        .doing
        .as_deref()
        .unwrap()
        .starts_with("lance autre chose"));
    assert!(t.subagents.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_project_never_read_is_read_at_the_first_fact_of_its_agents() {
    // A project added while the app runs: nothing has read its workflows yet when its agent types
    // its first door (ESC-194). That door is not lost.
    let h = crate::core_tests::harness("workflow-track-unread");
    let (p, dir) = h.project(false).await;
    let cmds = dir.join(".claude/commands");
    std::fs::create_dir_all(&cmds).unwrap();
    std::fs::write(cmds.join("one.md"), "Do one, then /two.\n").unwrap();
    std::fs::write(cmds.join("two.md"), "Do two. Back to /one when red.\n").unwrap();
    assert!(!h.core.data.workflows_file(&p.id).exists());
    let agent = crate::core_tests::agent_on(&h, &p, "principal").await;

    h.core
        .track_workflow(&agent.id, &p.id, &ops_user("/one"), 0.0, None);

    let tracks = h.core.workflow_tracks(&p.id);
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].track.step.as_deref(), Some("one"));
    // The reading is kept, as « Relire » would.
    assert!(h.core.data.workflows_file(&p.id).is_file());
}

// ---------- the workflows as the user corrected them (ESC-195) ----------

use crate::workflowfix::{corrected, Fix, FixStep, Signal};

fn fix_step(name: &str, from: &[&str]) -> FixStep {
    FixStep {
        name: name.into(),
        from: from.iter().map(|s| s.to_string()).collect(),
        signals: Vec::new(),
    }
}

/// GSD as read, with the corrections `f` makes of it.
fn gsd_fixed(f: impl FnOnce(&Workflow) -> Fix) -> Scan {
    let scan = gsd();
    let fix = f(&scan.workflows[0]);
    corrected(&scan, &[fix])
}

/// The four steps of GSD, as read, named as they are.
fn as_read(w: &Workflow) -> Vec<FixStep> {
    w.steps
        .iter()
        .map(|s| fix_step(&s.name, &[&s.name]))
        .collect()
}

#[test]
fn a_step_renamed_is_followed_under_its_new_name_by_its_command_and_its_subagents() {
    let mut a = Fake::new(gsd_fixed(|w| {
        let mut steps = as_read(w);
        steps[1].name = "Planifier".into();
        Fix {
            workflow: w.id.clone(),
            steps,
            ..Default::default()
        }
    }));
    assert!(a.says("/gsd:plan-phase 2"));
    assert_eq!(a.step(), Some("Planifier"));
    assert_eq!(a.last().door.as_deref(), Some("/gsd:plan-phase"));
    // From a step elsewhere, its subagent brings the agent back into it.
    assert!(a.says("/gsd:execute-phase 2"));
    assert!(a.launches("gsd-planner"));
    assert_eq!(a.step(), Some("Planifier"));
}

#[test]
fn two_steps_merged_are_one_step_that_both_commands_enter() {
    let mut a = Fake::new(gsd_fixed(|w| {
        let mut steps = as_read(w);
        let plan = steps.remove(1);
        steps[0] = fix_step("Préparer", &["gsd:discuss-phase", &plan.name]);
        Fix {
            workflow: w.id.clone(),
            steps,
            ..Default::default()
        }
    }));
    assert_eq!(a.scan.workflows[0].steps.len(), 3);
    assert!(a.says("/gsd:discuss-phase 2"));
    assert_eq!(a.step(), Some("Préparer"));
    assert!(a.says("/gsd:execute-phase 2"));
    assert!(a.says("/gsd:plan-phase 2"));
    assert_eq!(a.step(), Some("Préparer"));
    // Its subagents are those of both.
    assert!(a.says("/gsd:execute-phase 2"));
    assert!(a.launches("gsd-plan-checker"));
    assert_eq!(a.step(), Some("Préparer"));
    assert!(a
        .track
        .passages
        .iter()
        .all(|p| p.step.as_deref() != Some("gsd:plan-phase")));
}

#[test]
fn a_step_removed_is_never_entered() {
    let mut a = Fake::new(gsd_fixed(|w| Fix {
        workflow: w.id.clone(),
        removed: vec!["gsd:verify-work".into()],
        ..Default::default()
    }));
    assert!(a.says("/gsd:execute-phase 2"));
    a.says("/gsd:verify-work 2");
    a.launches("gsd-verifier");
    assert_ne!(a.step(), Some("gsd:verify-work"));
    assert!(a
        .track
        .passages
        .iter()
        .all(|p| p.step.as_deref() != Some("gsd:verify-work")));
}

#[test]
fn a_workflow_set_apart_is_no_longer_followed() {
    let mut a = Fake::new(gsd_fixed(|w| Fix {
        workflow: w.id.clone(),
        dismissed: true,
        ..Default::default()
    }));
    assert!(a.scan.workflows.is_empty());
    a.says("/gsd:discuss-phase 2");
    assert!(a.track.workflow.is_none());
    assert!(a.track.passages.is_empty());
}

#[test]
fn a_step_added_by_hand_is_entered_by_its_command_its_subagent_or_its_file() {
    let scan = gsd_fixed(|w| {
        let mut steps = as_read(w);
        let mut ship = fix_step("Livrer", &[]);
        ship.signals = vec![
            Signal::Command("/gsd:ship".into()),
            Signal::Agent("gsd-shipper".into()),
            Signal::File(".planning/SHIPPED.md".into()),
        ];
        steps.push(ship);
        Fix {
            workflow: w.id.clone(),
            steps,
            ..Default::default()
        }
    });
    // A command typed.
    let mut a = Fake::new(scan.clone());
    assert!(a.says("/gsd:ship v1"));
    assert_eq!(a.step(), Some("Livrer"));
    assert_eq!(a.last().door.as_deref(), Some("/gsd:ship"));
    // A subagent launched, from another step.
    let mut b = Fake::new(scan.clone());
    assert!(b.says("/gsd:verify-work 2"));
    assert!(b.launches("gsd-shipper"));
    assert_eq!(b.step(), Some("Livrer"));
    // A file written.
    let mut c = Fake::new(scan);
    assert!(c.says("/gsd:verify-work 2"));
    assert!(c.writes("C:/repo/.planning/SHIPPED.md"));
    assert_eq!(c.step(), Some("Livrer"));
}

#[tokio::test(flavor = "multi_thread")]
async fn corrections_changed_empty_the_tracking_s_reading_and_keep_the_history_as_it_was() {
    let h = crate::core_tests::harness("workflow-track-fixes");
    let (p, dir) = h.project(false).await;
    let cmds = dir.join(".claude/commands");
    std::fs::create_dir_all(&cmds).unwrap();
    std::fs::write(
        cmds.join("one.md"),
        "Do one with subagent_type=\"worker\", then /two.\n",
    )
    .unwrap();
    std::fs::write(cmds.join("two.md"), "Do two. Back to /one when red.\n").unwrap();
    let scan = h.core.refresh_workflows(&p.id, true).await.unwrap();
    let w = scan.workflows[0].clone();
    let agent = crate::core_tests::agent_on(&h, &p, "principal").await;
    let id = agent.id.clone();
    h.core
        .track_workflow(&id, &p.id, &ops_user("/one"), 0.0, None);
    assert_eq!(
        h.core.workflow_tracks(&p.id)[0].track.step.as_deref(),
        Some("one")
    );

    // « one » renamed in the settings: the tracking takes it at once, no new reading needed.
    let before = events(&h).len();
    h.core
        .update_project(crate::model::Project {
            workflow_fixes: vec![Fix {
                workflow: w.id.clone(),
                steps: vec![fix_step("Premier", &["one"]), fix_step("two", &["two"])],
                ..Default::default()
            }],
            ..h.core.project(&p.id).unwrap()
        })
        .unwrap();
    assert!(events(&h).len() > before, "the agent is told");
    h.core
        .track_workflow(&id, &p.id, &ops_user("/two"), 0.0, None);
    h.core
        .track_workflow(&id, &p.id, &ops_user("/one"), 0.0, None);
    let t = h.core.workflow_tracks(&p.id)[0].track.clone();
    assert_eq!(t.step.as_deref(), Some("Premier"));
    // The passage kept keeps the name its step had.
    assert_eq!(t.passages[0].step.as_deref(), Some("one"));
    assert_eq!(t.passages[2].step.as_deref(), Some("Premier"));

    // « Relire »: still corrected.
    h.core.refresh_workflows(&p.id, true).await.unwrap();
    h.core
        .track_workflow(&id, &p.id, &ops_user("/two"), 0.0, None);
    h.core
        .track_workflow(&id, &p.id, &ops_user("/one"), 0.0, None);
    assert_eq!(
        h.core.workflow_tracks(&p.id)[0].track.step.as_deref(),
        Some("Premier")
    );

    // Set apart: no longer followed.
    h.core
        .update_project(crate::model::Project {
            workflow_fixes: vec![Fix {
                workflow: w.id.clone(),
                dismissed: true,
                ..Default::default()
            }],
            ..h.core.project(&p.id).unwrap()
        })
        .unwrap();
    let n = h.core.workflow_tracks(&p.id)[0].track.passages.len();
    h.core
        .track_workflow(&id, &p.id, &ops_user("/two"), 0.0, None);
    assert_eq!(h.core.workflow_tracks(&p.id)[0].track.passages.len(), n);

    // From the file of the workflow, changed by a pull: read again, the tracking follows it.
    let file = dir.join(".escouade/workflows/claude-one.yml");
    let mut doc = crate::workflowfiles::read(&std::fs::read_to_string(&file).unwrap()).unwrap();
    doc.fixes = Fix {
        workflow: w.id.clone(),
        steps: vec![fix_step("Deux", &["two"])],
        ..Default::default()
    };
    std::fs::write(&file, crate::workflowfiles::write(&doc)).unwrap();
    h.core.sync_workflow_files(&p.id);
    assert_eq!(h.core.project(&p.id).unwrap().workflow_fixes.len(), 1);
    h.core
        .track_workflow(&id, &p.id, &ops_user("/two"), 0.0, None);
    assert_eq!(
        h.core.workflow_tracks(&p.id)[0].track.step.as_deref(),
        Some("Deux")
    );
}
