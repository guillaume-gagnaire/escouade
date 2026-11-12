//! A harness project by project (ESC-404): the switch of its plugin in a project holds for its agents. Off there, none
//! starts, resumes or is sent a message, the menus leave it out, the Kanban blocks its tickets with why; a private
//! project asks first (`Asks`) and « Activer » turns it on for it; cutting it stops its agents of the project; the
//! private projects with agents on a harness have it turned on once, at the update.

use crate::core::Core;
use crate::core_tests::{harness, Harness};
use crate::harness_scope::InProject;
use crate::model::*;
use crate::paths::{test_dir, DataDir};
use crate::tickets_tests::draft;
use serde_json::json;
use tauri::test::mock_app;

const CLAUDE_PLUGIN: &str = crate::plugins::builtin::CLAUDE;

fn make_private(h: &Harness, p: &Project) {
    h.core
        .projects
        .write()
        .iter_mut()
        .find(|x| x.id == p.id)
        .unwrap()
        .private = true;
}

/// Every harness of the registry turned on for the private project `project`, as the user does it with « Activer »:
/// for the tests whose agents run there.
pub(crate) fn turn_on_every_harness(h: &Harness, project: &str) {
    for e in crate::engine::registry::all() {
        // A harness that goes through no plugin (a test's own) has nothing to turn on.
        let _ = h.core.enable_harness_in(project, e.id());
    }
}

fn switch_of(h: &Harness, project: &str) -> Option<bool> {
    h.core
        .plugins
        .choices(CLAUDE_PLUGIN)
        .unwrap()
        .projects
        .get(project)
        .copied()
}

/// A project that is not private: Claude Code on, then turned off for it. Its agent stops, takes no message, does not
/// start again; no new agent is made there; the scope and the menus say so. The other project is not touched.
#[tokio::test]
async fn a_harness_turned_off_for_a_project_runs_no_agent_there() {
    let h = harness("esc404-off");
    let (p, _) = h.project(false).await;
    assert_eq!(h.core.harness_in(EngineId::CLAUDE, &p.id), InProject::On);
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    h.core
        .send_message(&a.id, "Bonjour".into(), Vec::new())
        .await
        .unwrap();
    h.wait("its process", |h| h.alive(&a.id)).await;
    h.wait("its turn done", |h| !h.agent(&a.id).status.is_active())
        .await;

    h.core
        .set_plugin_project(CLAUDE_PLUGIN, &p.id, false)
        .await
        .unwrap();
    assert_eq!(switch_of(&h, &p.id), Some(false));
    assert_eq!(h.core.harness_in(EngineId::CLAUDE, &p.id), InProject::Off);
    // Cut: its process stopped, as turning the plugin off for the app stops them.
    assert!(!h.alive(&a.id), "the agent's process still runs");
    // Nothing more reaches it, nor starts it again.
    let sent = h
        .core
        .send_message(&a.id, "Encore".into(), Vec::new())
        .await
        .unwrap_err()
        .to_string();
    assert!(sent.contains("Claude Code"), "{sent}");
    assert!(!h.alive(&a.id));
    assert!(h.core.ensure_process(&a.id).await.is_err());
    assert!(!h.alive(&a.id));
    // No new agent there.
    assert!(h.core.create_agent(&p.id, None).await.is_err());
    // The window's lists: off, not offered.
    let scope = h.core.harness_scope().remove(&p.id).unwrap_or_default();
    assert!(scope.off.contains(&EngineId::CLAUDE), "{scope:?}");
    assert!(!scope.asks.contains(&EngineId::CLAUDE), "{scope:?}");
    assert!(!h.core.engines_usable_in(&p.id).contains(&EngineId::CLAUDE));
    assert!(h.core.engines_usable().contains(&EngineId::CLAUDE));
    // Turned on again by the user: it goes on.
    h.core
        .set_plugin_project(CLAUDE_PLUGIN, &p.id, true)
        .await
        .unwrap();
    h.core
        .send_message(&a.id, "Reprends".into(), Vec::new())
        .await
        .unwrap();
    h.wait("its process again", |h| h.alive(&a.id)).await;
    let scope = h.core.harness_scope().remove(&p.id).unwrap_or_default();
    assert!(
        !scope.off.contains(&EngineId::CLAUDE) && !scope.asks.contains(&EngineId::CLAUDE),
        "{scope:?}"
    );
}

/// A private project: Claude Code is off there by default (`Asks`, still offered for the question of the window); no
/// agent is made, nor started, before « Activer », which turns the plugin on for it alone.
#[tokio::test]
async fn a_private_project_asks_once_then_its_agent_starts() {
    let h = harness("esc404-private");
    let (p, _) = h.project(false).await;
    make_private(&h, &p);
    assert_eq!(h.core.harness_in(EngineId::CLAUDE, &p.id), InProject::Asks);
    let scope = h.core.harness_scope().remove(&p.id).unwrap_or_default();
    assert!(scope.asks.contains(&EngineId::CLAUDE), "{scope:?}");
    assert!(scope.off.is_empty(), "{scope:?}");
    // Offered by the window, behind its question; the core picks it for nothing of the project by itself.
    assert!(!h.core.engines_usable_in(&p.id).contains(&EngineId::CLAUDE));
    let refused = h
        .core
        .create_agent(&p.id, None)
        .await
        .unwrap_err()
        .to_string();
    assert!(
        refused.contains("privé") || refused.contains("private"),
        "{refused}"
    );
    assert!(h.core.project_agents(&p.id).is_empty());
    // « Annuler » leaves nothing set; « Activer » turns it on for this project.
    assert_eq!(switch_of(&h, &p.id), None);
    h.core.enable_harness_in(&p.id, EngineId::CLAUDE).unwrap();
    assert_eq!(switch_of(&h, &p.id), Some(true));
    assert_eq!(h.core.harness_in(EngineId::CLAUDE, &p.id), InProject::On);
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    h.core
        .send_message(&a.id, "Bonjour".into(), Vec::new())
        .await
        .unwrap();
    h.wait("its process", |h| h.alive(&a.id)).await;
    let scope = h.core.harness_scope().remove(&p.id).unwrap_or_default();
    assert!(
        !scope.off.contains(&EngineId::CLAUDE) && !scope.asks.contains(&EngineId::CLAUDE),
        "{scope:?}"
    );
    assert!(h.core.engines_usable_in(&p.id).contains(&EngineId::CLAUDE));
}

/// An agent already in a private project whose harness was never turned on for it (the project made private since):
/// it does not start, nor resume, nor take a message, until « Activer ».
#[tokio::test]
async fn an_agent_of_a_private_project_waits_for_its_harness_to_be_turned_on() {
    let h = harness("esc404-existing");
    let (p, _) = h.project(false).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    make_private(&h, &p);
    assert!(h
        .core
        .send_message(&a.id, "Bonjour".into(), Vec::new())
        .await
        .is_err());
    assert!(h.core.ensure_process(&a.id).await.is_err());
    assert!(!h.alive(&a.id));
    h.core.enable_harness_in(&p.id, EngineId::CLAUDE).unwrap();
    h.core
        .send_message(&a.id, "Bonjour".into(), Vec::new())
        .await
        .unwrap();
    h.wait("its process", |h| h.alive(&a.id)).await;
}

/// The autopilot does not ask in the user's place: a ticket of a private project whose harness is not on stays blocked,
/// with why, and no agent; « Activer » then « Reprendre » starts it.
#[tokio::test]
async fn the_autopilot_blocks_a_ticket_whose_harness_is_off_for_its_project() {
    let h = harness("esc404-autopilot");
    let (p, _) = h.project(false).await;
    make_private(&h, &p);
    h.set_board(&p.id, |s| s.autopilot = false);
    let t = h
        .core
        .ticket_create(&p.id, draft("Privé [ok]", &[], 5))
        .await
        .unwrap();
    h.set_board(&p.id, |s| s.autopilot = true);
    h.wait_ticket(&t.id, "blocked", |t| t.blocked.is_some())
        .await;
    let blocked = h.ticket(&t.id);
    assert!(blocked.agent_id.is_none(), "{blocked:?}");
    let why = blocked.blocked.unwrap();
    assert!(why.contains("Claude Code"), "{why}");
    // Not turned on by the Kanban.
    assert_eq!(switch_of(&h, &p.id), None);
    assert!(h.core.project_agents(&p.id).is_empty());
    // The user's « Activer », then the ticket taken up.
    h.core.enable_harness_in(&p.id, EngineId::CLAUDE).unwrap();
    h.core.ticket_resume(&t.id).await.unwrap();
    h.wait_ticket(&t.id, "to test", |t| t.column == Column::Review)
        .await;
}

/// A ticket's agent whose harness is cut for its project: stopped, its ticket blocked with why.
#[tokio::test]
async fn cutting_a_harness_for_a_project_blocks_the_ticket_of_its_agent() {
    let h = harness("esc404-cut-ticket");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let t = h
        .core
        .ticket_create(&p.id, draft("Long travail [lent]", &[], 5))
        .await
        .unwrap();
    h.core.ticket_start(&t.id).unwrap();
    let agent = crate::tickets_tests::wait_running(&h, &t.id).await;
    h.core
        .set_plugin_project(CLAUDE_PLUGIN, &p.id, false)
        .await
        .unwrap();
    assert!(!h.alive(&agent));
    h.wait_ticket(&t.id, "blocked", |t| t.blocked.is_some())
        .await;
    assert!(h.ticket(&t.id).blocked.unwrap().contains("Claude Code"));
}

/// A state as the app saved it before ESC-404: a private project with an agent on Claude Code, another private one with
/// none, an archived agent's private project, and a project that is not private.
fn state_before(dir: &std::path::Path) -> serde_json::Value {
    let cwd = dir.to_string_lossy();
    let project = |id: &str, private: bool| json!({ "id": id, "name": id, "path": cwd, "color": "red", "createdAt": 1, "private": private });
    let agent = |id: &str, project: &str, archived: bool| {
        json!({ "id": id, "projectId": project, "name": id, "model": "sonnet", "cwd": cwd, "createdAt": 2,
                "engine": "claude", "archived": archived })
    };
    json!({
        "projects": [project("used", true), project("unused", true), project("archived", true), project("open", false)],
        "agents": [agent("a1", "used", false), agent("a2", "archived", true), agent("a3", "open", false)],
        "tickets": []
    })
}

fn load(data: &DataDir) -> std::sync::Arc<Core<tauri::test::MockRuntime>> {
    let app = mock_app();
    let (core, _rx) = Core::load(app.handle().clone(), data.clone());
    // The mock app lives as long as the test (its handle is the core's).
    std::mem::forget(app);
    core
}

/// The update turns Claude Code on for the private project that has an agent on it, and for that one alone; once: a
/// project the user turns off afterwards stays off at the next start.
#[tokio::test]
async fn the_update_turns_on_a_harness_for_the_private_projects_that_use_it_once() {
    let dir = test_dir("esc404-migration");
    let data = DataDir::new(dir.join("data"));
    data.ensure().unwrap();
    std::fs::write(
        data.settings_file(),
        json!({ "claudePath": crate::core_tests::fake_cli(), "sound": false }).to_string(),
    )
    .unwrap();
    std::fs::write(data.state_file(), state_before(&dir).to_string()).unwrap();
    let core = load(&data);
    let projects = core.plugins.choices(CLAUDE_PLUGIN).unwrap().projects;
    assert_eq!(projects.get("used"), Some(&true), "{projects:?}");
    for untouched in ["unused", "archived", "open"] {
        assert_eq!(projects.get(untouched), None, "{untouched}: {projects:?}");
    }
    assert_eq!(core.harness_in(EngineId::CLAUDE, "used"), InProject::On);
    assert_eq!(core.harness_in(EngineId::CLAUDE, "unused"), InProject::Asks);
    assert_eq!(core.harness_in(EngineId::CLAUDE, "open"), InProject::On);
    // The user turns it off for « used », and removes its choice of « unused »: the next start changes nothing.
    core.plugins
        .choose(CLAUDE_PLUGIN, |c| {
            c.projects.insert("used".into(), false);
        })
        .unwrap();

    drop(core);
    let again = load(&data);
    let projects = again.plugins.choices(CLAUDE_PLUGIN).unwrap().projects;
    assert_eq!(projects.get("used"), Some(&false), "{projects:?}");
}

// ---------- SEC-08: a plugin of a third party in a private project ----------

const TIERS: &str = "com.exemple.tiers";

fn tiers_engine() -> EngineId {
    EngineId::new(&format!("plugin:{TIERS}:outil"))
}

/// A plugin of a third party (not `dev.escouade.*`) installed from a folder, as the user would.
fn install_tiers(plugins: &crate::plugins::Plugins, name: &str) {
    let src = test_dir(name).join("src");
    std::fs::create_dir_all(src.join("themes")).unwrap();
    std::fs::write(
        src.join(crate::plugins::manifest::FILE),
        json!({
            "id": TIERS,
            "name": "Outil tiers",
            "version": "1.0.0",
            "escouade": { "api": "1", "min": "2.1.0" },
            "contributes": { "themes": [{ "id": "t" }] }
        })
        .to_string(),
    )
    .unwrap();
    std::fs::write(src.join("themes/t.json"), "{}").unwrap();
    plugins.install_folder(&src, crate::i18n::Lang::Fr).unwrap();
}

/// Its harness reaches a private project only once the user turned it on there (« Activer »): not before, whatever
/// it says of its fence.
#[tokio::test]
async fn a_third_party_harness_is_not_launched_in_a_private_project_until_turned_on_there() {
    let h = harness("sec08-tiers");
    let (p, _) = h.project(false).await;
    install_tiers(&h.core.plugins, "sec08-tiers-plugin");
    let engine = tiers_engine();
    assert!(!crate::plugins::builtin::first_hand(TIERS));
    assert_eq!(h.core.harness_in(engine, &p.id), InProject::On);
    make_private(&h, &p);
    assert_eq!(h.core.harness_in(engine, &p.id), InProject::Asks);
    let refused = h
        .core
        .check_harness_in(engine, &p.id)
        .unwrap_err()
        .to_string();
    assert!(refused.contains("privé"), "{refused}");
    h.core.enable_harness_in(&p.id, engine).unwrap();
    assert_eq!(h.core.harness_in(engine, &p.id), InProject::On);
    assert!(h.core.check_harness_in(engine, &p.id).is_ok());
}

/// The update that turns on, once, the harnesses a private project already uses turns on Escouade's own alone: one of
/// a third party asks the user first.
#[tokio::test]
async fn the_update_never_turns_on_a_third_party_harness_for_a_private_project() {
    let dir = test_dir("sec08-migration");
    let data = DataDir::new(dir.join("data"));
    data.ensure().unwrap();
    std::fs::write(
        data.settings_file(),
        json!({ "claudePath": crate::core_tests::fake_cli(), "sound": false }).to_string(),
    )
    .unwrap();
    install_tiers(
        &crate::plugins::Plugins::load(data.clone(), env!("CARGO_PKG_VERSION")),
        "sec08-migration-plugin",
    );
    let cwd = dir.to_string_lossy();
    let agent = |id: &str, engine: &str| {
        json!({ "id": id, "projectId": "client", "name": id, "model": "sonnet", "cwd": cwd, "createdAt": 2,
                "engine": engine })
    };
    let state = json!({
        "projects": [{ "id": "client", "name": "client", "path": cwd, "color": "red", "createdAt": 1, "private": true }],
        "agents": [agent("a1", "claude"), agent("a2", tiers_engine().as_str())],
        "tickets": []
    });
    std::fs::write(data.state_file(), state.to_string()).unwrap();
    let core = load(&data);
    assert_eq!(core.harness_in(EngineId::CLAUDE, "client"), InProject::On);
    assert_eq!(core.harness_in(tiers_engine(), "client"), InProject::Asks);
    assert_eq!(
        core.plugins.choices(TIERS).unwrap().projects.get("client"),
        None
    );
}
