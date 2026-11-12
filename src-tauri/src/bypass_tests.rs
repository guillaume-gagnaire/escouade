//! « Autoriser le mode Bypass » and the mode of new agents chosen at the first start (audit SEC-09).

use super::*;
use crate::accounts::{defaults_with, RECOMMENDED_MODE};
use crate::core::AgentOptions;
use crate::core_tests::harness;
use crate::model::{EngineId, Settings};
use crate::paths::{test_dir, DataDir};
use serde_json::json;
use tauri::test::mock_app;

fn load(data: &DataDir) -> std::sync::Arc<Core<tauri::test::MockRuntime>> {
    let app = mock_app();
    let (core, _rx) = Core::load(app.handle().clone(), data.clone());
    std::mem::forget(app);
    core
}

fn saved(data: &DataDir) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(data.settings_file()).unwrap()).unwrap()
}

#[test]
fn the_arguments_that_open_the_mode_of_no_question_leave_a_launch_unless_allowed() {
    let args = || {
        vec![
            "--output-format".to_string(),
            "stream-json".into(),
            "--allow-dangerously-skip-permissions".into(),
            "--dangerously-skip-permissions".into(),
        ]
    };
    let mut held = args();
    hold(&mut held, false);
    assert_eq!(held, ["--output-format", "stream-json"]);
    let mut kept = args();
    hold(&mut kept, true);
    assert_eq!(kept, args());
    assert!(refusal(BYPASS, false)
        .unwrap()
        .contains("Autoriser le mode Bypass"));
    assert!(refusal(BYPASS, true).is_none());
    assert!(refusal("auto", false).is_none());
}

#[test]
fn a_new_installation_asks_the_mode_and_runs_the_recommended_one_until_it_is_answered() {
    let dir = test_dir("sec09-new");
    let data = DataDir::new(dir.join("data"));
    let core = load(&data);
    let s = core.settings.read().clone();
    assert_eq!(s.allow_bypass, Some(false));
    assert_eq!(s.agent_mode, "", "asked by the window");
    assert_eq!(saved(&data)["allowBypass"], json!(false));
    // Until it is answered: the recommended mode, never the mode of no question.
    assert_eq!(RECOMMENDED_MODE, "acceptEdits");
    assert_eq!(
        defaults_with(None, EngineId::CLAUDE, &s.agent_mode).mode,
        "acceptEdits"
    );
    for chosen in ["default", "plan", "acceptEdits", "auto"] {
        assert_eq!(defaults_with(None, EngineId::CLAUDE, chosen).mode, chosen);
    }
    assert_eq!(
        defaults_with(None, EngineId::CLAUDE, BYPASS).mode,
        "auto",
        "the mode of no question is never a default"
    );
}

#[test]
fn an_installation_that_existed_keeps_its_modes_and_bypass_if_it_used_it() {
    let installed = |name: &str, state: serde_json::Value| {
        let dir = test_dir(name);
        let data = DataDir::new(dir.join("data"));
        data.ensure().unwrap();
        std::fs::write(
            data.settings_file(),
            json!({ "claudePath": crate::core_tests::fake_cli(), "sound": false }).to_string(),
        )
        .unwrap();
        std::fs::write(data.state_file(), state.to_string()).unwrap();
        (dir, data)
    };
    let agent = |cwd: &str, mode: &str| {
        json!({ "id": "a1", "projectId": "p", "name": "a1", "model": "sonnet", "cwd": cwd, "createdAt": 2,
                "engine": "claude", "mode": mode })
    };
    let project =
        |cwd: &str| json!({ "id": "p", "name": "p", "path": cwd, "color": "red", "createdAt": 1 });

    // An agent left in Bypass: the setting is found on, its mode stays.
    let (dir, data) = installed("sec09-used", json!({}));
    let cwd = dir.to_string_lossy().into_owned();
    std::fs::write(
        data.state_file(),
        json!({ "projects": [project(&cwd)], "agents": [agent(&cwd, BYPASS)], "tickets": [] })
            .to_string(),
    )
    .unwrap();
    let core = load(&data);
    assert_eq!(core.settings.read().allow_bypass, Some(true));
    assert_eq!(core.settings.read().agent_mode, "auto");
    assert_eq!(core.agent("a1").unwrap().lock().meta.mode, BYPASS);
    drop(core);
    // The user's later choice is kept: off stays off.
    let mut v = saved(&data);
    v["allowBypass"] = json!(false);
    std::fs::write(data.settings_file(), v.to_string()).unwrap();
    assert_eq!(load(&data).settings.read().allow_bypass, Some(false));

    // Without the mode anywhere: off, and its new agents keep `auto`.
    let (dir, data) = installed("sec09-unused", json!({}));
    let cwd = dir.to_string_lossy().into_owned();
    std::fs::write(
        data.state_file(),
        json!({ "projects": [project(&cwd)], "agents": [agent(&cwd, "auto")], "tickets": [] })
            .to_string(),
    )
    .unwrap();
    let core = load(&data);
    assert_eq!(core.settings.read().allow_bypass, Some(false));
    assert_eq!(core.settings.read().agent_mode, "auto");
    assert_eq!(saved(&data)["agentMode"], json!("auto"));
}

/// Without the setting, an agent in Bypass does not start, none goes into it, and Claude Code is launched without
/// `--allow-dangerously-skip-permissions`; with it, both are as before.
#[tokio::test]
async fn the_mode_of_no_question_is_refused_without_the_setting() {
    let h = harness("sec09-refused");
    let (p, r) = h.project(false).await;
    assert!(!h.core.bypass_allowed());
    let flag = "--allow-dangerously-skip-permissions".to_string();

    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&a, "Bonjour").await;
    assert!(!h.launches(&r).last().unwrap().contains(&flag));
    let refused = h
        .core
        .set_agent_options(&a, None, None, Some(BYPASS.into()))
        .await
        .unwrap_err()
        .to_string();
    assert!(refused.contains("Autoriser le mode Bypass"), "{refused}");
    assert_ne!(h.core.agent(&a).unwrap().lock().meta.mode, BYPASS);

    let b = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                mode: Some(BYPASS.into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    let refused = h
        .core
        .send_message(&b, "Bonjour".into(), Vec::new())
        .await
        .unwrap_err()
        .to_string();
    assert!(refused.contains("Autoriser le mode Bypass"), "{refused}");

    // Allowed in the settings: the agent in Bypass starts, with the argument.
    let mut s: Settings = h.core.settings.read().clone();
    s.allow_bypass = Some(true);
    h.core.save_settings(s).unwrap();
    h.turn(&b, "Bonjour").await;
    assert!(h.launches(&r).last().unwrap().contains(&flag));
}
