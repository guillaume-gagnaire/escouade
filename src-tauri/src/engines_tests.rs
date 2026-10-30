//! The harness of the agents, the tickets and the projects, and of each turn (ESC-27): what was
//! saved before 1.8 runs on Claude Code, a new agent takes its project's, a ticket's agent its
//! ticket's, a copy the one its original has, and each turn keeps the one that ran it.

use crate::core::{AgentOptions, Core};
use crate::core_tests::{harness, Harness};
use crate::model::*;
use crate::paths::{test_dir, DataDir};
use crate::stats::{EngineShare, Labels};
use crate::tickets_tests::draft;
use serde_json::{json, Value};
use tauri::test::mock_app;

/// A state file as 1.7 wrote it: a project with its settings, an agent on a second account with
/// its session, and a ticket done by it.
fn state_of_1_7(dir: &std::path::Path) -> Value {
    let cwd = dir.to_string_lossy();
    json!({
        "projects": [{
            "id": "p1", "name": "demo", "path": cwd, "color": "red", "worktreePerAgent": true,
            "createdAt": 5, "runCommands": [], "worktreeCopy": [".env*", "secrets/*"],
            "commitMode": "direct", "account": "", "worktreeDir": ".worktrees",
            "agentBranchPrefix": "agents/", "ticketBranchPrefix": "tickets/", "remoteControl": false,
            "board": { "action": "pr", "target": "main", "strategy": "rebase", "prefix": "DEM",
                       "nextNumber": 8, "autopilot": false, "model": "opus", "effort": "high" }
        }],
        "agents": [{
            "id": "a1", "projectId": "p1", "name": "ancien", "named": true, "model": "sonnet",
            "effort": "medium", "mode": "auto", "cwd": cwd, "sessionId": "s1", "lastEntry": "e1",
            "tokens": 1200, "cost": 0.75, "prompts": 3, "ticketId": "t1",
            "touchedFiles": ["src/main.rs"], "createdAt": 10, "status": "done"
        }],
        "tickets": [{
            "id": "t1", "projectId": "p1", "key": "DEM-7", "title": "Ajouter le fichier",
            "description": "Le détail", "criteria": [{ "text": "Le fichier existe", "ok": true, "note": "vu" }],
            "progress": ["Le fichier"], "maxLoops": 5, "column": "done", "rank": 1, "agentId": "a1",
            "iteration": 2, "loops": 2, "cost": 0.75, "model": "opus", "effort": "max",
            "outcome": "⤵ Mergé dans main · squash", "createdAt": 11, "doneAt": 20
        }]
    })
}

#[tokio::test]
async fn a_state_saved_before_the_harnesses_loads_on_claude_code_and_loses_nothing() {
    let dir = test_dir("esc27-state-before-engines");
    let data = DataDir::new(dir.join("data"));
    data.ensure().unwrap();
    std::fs::write(data.settings_file(), r#"{"claudePath":"","sound":false}"#).unwrap();
    std::fs::write(data.state_file(), state_of_1_7(&dir).to_string()).unwrap();
    let app = mock_app();
    let (core, _rx) = Core::load(app.handle().clone(), data.clone());
    let a = core.agent("a1").unwrap().lock().meta.clone();
    assert_eq!(a.engine, EngineId::Claude);
    assert_eq!(
        (
            a.name.as_str(),
            a.model.as_str(),
            a.session_id.as_deref(),
            a.last_entry.as_deref(),
            a.tokens,
            a.prompts,
            a.ticket_id.as_deref()
        ),
        (
            "ancien",
            "sonnet",
            Some("s1"),
            Some("e1"),
            1200,
            3,
            Some("t1")
        )
    );
    assert_eq!(a.touched_files, ["src/main.rs"]);
    let t = core.ticket("t1").unwrap();
    assert_eq!(t.engine, None);
    assert_eq!(
        (
            t.key.as_str(),
            t.column,
            t.model.as_str(),
            t.effort.as_str(),
            t.loops
        ),
        ("DEM-7", Column::Done, "opus", "max", 2)
    );
    assert_eq!(t.criteria[0].note, "vu");
    let p = core.project("p1").unwrap();
    assert_eq!((p.engine, p.engine()), (None, EngineId::Claude));
    assert_eq!(
        (
            p.board.action.as_str(),
            p.board.next_number,
            p.worktree_dir.as_str(),
            p.commit_mode
        ),
        ("pr", 8, ".worktrees", CommitMode::Direct)
    );
    // Saved again, the file says the harnesses, and keeps the rest as it was.
    core.save_state().unwrap();
    let saved: Value = serde_json::from_slice(&std::fs::read(data.state_file()).unwrap()).unwrap();
    assert_eq!(saved["agents"][0]["engine"], "claude");
    assert_eq!(saved["tickets"][0]["engine"], Value::Null);
    assert_eq!(saved["projects"][0]["engine"], Value::Null);
    let before = state_of_1_7(&dir);
    for (list, keys) in [
        (
            "agents",
            &[
                "sessionId",
                "lastEntry",
                "tokens",
                "cost",
                "touchedFiles",
                "ticketId",
            ][..],
        ),
        (
            "tickets",
            &[
                "key", "criteria", "progress", "outcome", "agentId", "doneAt",
            ][..],
        ),
        (
            "projects",
            &["worktreeCopy", "agentBranchPrefix", "ticketBranchPrefix"][..],
        ),
    ] {
        for key in keys {
            assert_eq!(saved[list][0][key], before[list][0][key], "{list}.{key}");
        }
    }
    core.stop_for_good();
}

/// An account for every harness beside Principal: an agent of a harness goes to its accounts
/// alone (ESC-30). Each runs its fake.
pub(crate) fn accounts_for_every_harness(h: &Harness) {
    h.set_settings(|s| {
        for engine in [
            EngineId::Codex,
            EngineId::Copilot,
            EngineId::Cursor,
            EngineId::Opencode,
        ] {
            s.accounts.push(crate::model::Account {
                id: format!("{engine}-work"),
                name: format!("{engine} work"),
                engine,
                config_dir: h.dir.join(engine.as_str()).to_string_lossy().into(),
                claude_path: match engine {
                    EngineId::Codex => crate::engine::fake_codex(),
                    EngineId::Opencode => crate::engine::fake_opencode(),
                    EngineId::Copilot => crate::engine::fake_copilot(),
                    EngineId::Cursor => crate::engine::fake_cursor(),
                    _ => String::new(),
                },
                ..Default::default()
            });
        }
    });
}

#[tokio::test]
async fn a_new_agent_runs_on_its_projects_harness_unless_told_another() {
    let h = harness("esc27-new-agent");
    let (p, _) = h.project(false).await;
    accounts_for_every_harness(&h);
    // A project saved before 1.8, or whose harness is the default: Claude Code.
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    assert_eq!(a.engine, EngineId::Claude);
    h.core
        .projects
        .write()
        .iter_mut()
        .find(|x| x.id == p.id)
        .unwrap()
        .engine = Some(EngineId::Codex);
    let b = h.core.create_agent(&p.id, None).await.unwrap().meta;
    assert_eq!(b.engine, EngineId::Codex);
    let c = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::Copilot),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta;
    assert_eq!(c.engine, EngineId::Copilot);
    // Kept with the agent.
    h.core.save_state().unwrap();
    let saved: Value =
        serde_json::from_slice(&std::fs::read(h.core.data.state_file()).unwrap()).unwrap();
    let engine_of = |id: &str| {
        saved["agents"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["id"] == id)
            .unwrap()["engine"]
            .clone()
    };
    assert_eq!(
        [engine_of(&a.id), engine_of(&b.id), engine_of(&c.id)],
        [json!("claude"), json!("codex"), json!("copilot")]
    );
    assert_eq!(saved["projects"][0]["engine"], "codex");
}

#[tokio::test]
async fn each_turn_keeps_the_harness_that_ran_it_and_a_copy_takes_the_one_its_original_has() {
    let h = harness("esc27-turns");
    let (p, _) = h.project(false).await;
    accounts_for_every_harness(&h);
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&id, "Premier").await;
    h.wait("the first turn counted", |h| {
        h.core.stats.today_cost() > 0.0
    })
    .await;
    let first_cost = h.core.stats.today_cost();
    // A handover (ESC-88) puts it on another harness: its turns from then on are that one's.
    // Its process (Claude Code's) gone, as a handover stops it.
    {
        let h = h.core.agent(&id).unwrap();
        let mut rt = h.lock();
        if let Some(p) = rt.detach() {
            p.kill();
        }
        rt.meta.engine = EngineId::Codex;
        rt.meta.account = "codex-work".into();
        rt.meta.session_id = None;
    }
    h.turn(&id, "Second").await;
    // Its Claude model is none of Codex's: it goes on the one its account lists by default
    // (`settle_model`, read by an app-server of its own since ESC-186), which OpenAI's grid prices
    // (`pricing::estimate_openai`, Codex telling no cost).
    let codex_counted = |h: &Harness| {
        h.core
            .stats
            .query("day", None, &Labels::default())
            .by_engine
            .iter()
            .any(|s| s.engine == "codex")
    };
    h.wait("the second turn counted", codex_counted).await;
    assert_eq!(h.agent(&id).model, "gpt-6-luna");
    assert!(h.core.stats.today_cost() > first_cost);
    let turns: Vec<Value> = h
        .items(&id)
        .into_iter()
        .filter(|i| i["kind"] == "turn")
        .map(|i| i["engine"].clone())
        .collect();
    assert_eq!(turns, [json!("claude"), json!("codex")]);
    let by_engine: Vec<String> = h
        .core
        .stats
        .query("day", None, &Labels::default())
        .by_engine
        .into_iter()
        .map(|s: EngineShare| s.engine)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    assert_eq!(by_engine, ["claude", "codex"]);
    // Its copy takes the harness it has now, not the one it began on.
    let copy = h.core.duplicate_agent(&id).await.unwrap().meta;
    assert_eq!(copy.engine, EngineId::Codex);
}

#[tokio::test]
async fn a_tickets_agent_runs_on_the_tickets_harness_else_on_its_projects() {
    let h = harness("esc27-tickets");
    let (p, _) = h.project(false).await;
    accounts_for_every_harness(&h);
    h.set_board(&p.id, |s| s.autopilot = false);
    // A ticket of its own harness.
    let own = h
        .core
        .ticket_create(&p.id, draft("Le sien [ok]", &[], 5))
        .await
        .unwrap();
    assert_eq!(own.engine, None);
    h.core
        .edit_ticket(&own.id, |t| {
            t.engine = Some(EngineId::Cursor);
            Ok(())
        })
        .unwrap();
    h.core.ticket_start(&own.id).unwrap();
    h.wait("its agent", |h| h.ticket(&own.id).agent_id.is_some())
        .await;
    let agent = h.ticket(&own.id).agent_id.unwrap();
    assert_eq!(h.agent(&agent).engine, EngineId::Cursor);
    // One that says none: its project's.
    h.core
        .projects
        .write()
        .iter_mut()
        .find(|x| x.id == p.id)
        .unwrap()
        .engine = Some(EngineId::Opencode);
    let theirs = h
        .core
        .ticket_create(&p.id, draft("Celui du projet [ok]", &[], 5))
        .await
        .unwrap();
    h.core.ticket_start(&theirs.id).unwrap();
    h.wait("its agent", |h| h.ticket(&theirs.id).agent_id.is_some())
        .await;
    let agent = h.ticket(&theirs.id).agent_id.unwrap();
    assert_eq!(h.agent(&agent).engine, EngineId::Opencode);
}

// ---------- Choosing the harness (ESC-29) ----------

/// The form of a ticket sends its « Harness » field: null for the project's. The tools of the MCP
/// server and the imports send none, and the ticket keeps its own.
#[tokio::test]
async fn a_tickets_form_sets_its_harness_and_a_draft_without_one_keeps_it() {
    let h = harness("esc29-ticket-form");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let form = |engine: Value| -> crate::tickets::TicketDraft {
        let d = json!({
            "title": "Le sien", "description": "", "criteria": [], "maxLoops": 5, "after": [],
            "branch": "", "engine": engine
        });
        serde_json::from_value(d).unwrap()
    };
    // As the window sends it: its harness, or null.
    let t = h
        .core
        .ticket_create(&p.id, form(json!("codex")))
        .await
        .unwrap();
    assert_eq!(t.engine, Some(EngineId::Codex));
    let none = h
        .core
        .ticket_create(&p.id, form(json!(null)))
        .await
        .unwrap();
    assert_eq!(none.engine, None);
    // A draft without the field (the MCP tools): the ticket keeps it.
    let without: crate::tickets::TicketDraft =
        serde_json::from_value(json!({ "title": "Renommé", "maxLoops": 5 })).unwrap();
    assert_eq!(without.engine, None);
    let kept = h.core.ticket_update(&t.id, without).unwrap();
    assert_eq!(
        (kept.title.as_str(), kept.engine),
        ("Renommé", Some(EngineId::Codex))
    );
    // Null: back to the project's.
    let back = h.core.ticket_update(&t.id, form(json!(null))).unwrap();
    assert_eq!(back.engine, None);
    let other = h.core.ticket_update(&t.id, form(json!("cursor"))).unwrap();
    assert_eq!(other.engine, Some(EngineId::Cursor));
}

/// Only a harness whose program is found is offered (`engines_found`): with none, the first start
/// says « Aucun harness trouvé ».
#[tokio::test]
async fn the_harnesses_offered_are_those_whose_program_is_found() {
    let h = harness("esc29-found");
    assert_eq!(h.core.engines_found(), [EngineId::Claude]);
    let absent = h.dir.join("absent.cmd").to_string_lossy().to_string();
    let mut s = h.core.settings.read().clone();
    s.claude_path = absent;
    h.core.save_settings(s).unwrap();
    assert!(h.core.engines_found().is_empty());
    let mut s = h.core.settings.read().clone();
    s.claude_path = crate::core_tests::fake_cli();
    h.core.save_settings(s).unwrap();
    assert_eq!(h.core.engines_found(), [EngineId::Claude]);
}
