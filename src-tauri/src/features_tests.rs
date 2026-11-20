//! The features switched on or off (F-1, ESC-550): the register, the reprise of an update, the policies that impose
//! them, and what each takes with it in the core (its MCP tools are `mcp::features_tests`', its scheduled tasks
//! `scheduled_tests`').

use super::*;
use crate::core_tests::harness;
use crate::locks::{Managed, KEYS};
use crate::model::{DiscoverOutcome, Discovery, Settings, Ticket};
use crate::paths::{test_dir, DataDir};
use serde_json::{json, Value};
use tauri::test::mock_app;

fn load(data: &DataDir) -> Arc<Core> {
    let app = mock_app();
    let (core, _rx) = Core::load(app.handle().clone(), data.clone());
    std::mem::forget(app);
    core
}

fn saved(data: &DataDir) -> Value {
    serde_json::from_str(&std::fs::read_to_string(data.settings_file()).unwrap()).unwrap()
}

fn managed(pairs: &[(&str, Value)]) -> Managed {
    Managed {
        values: pairs
            .iter()
            .map(|(k, v)| {
                (
                    (*k).to_owned(),
                    (v.clone(), "the machine's policy".to_owned()),
                )
            })
            .collect(),
        warnings: Vec::new(),
    }
}

/// Sets the switch of `f` as Réglages › Fonctionnalités does (through the save).
fn switch(core: &Arc<Core>, f: Feature, on: bool) {
    let mut s = core.settings.read().clone();
    s.features.get_or_insert_with(Default::default).set(f, on);
    core.save_settings(s).unwrap();
}

#[test]
fn the_register_holds_the_nine_features_and_the_essential_is_none_of_them() {
    let ids: Vec<&str> = Feature::ALL.iter().map(|f| f.id()).collect();
    assert_eq!(
        ids,
        [
            "kanban",
            "editor",
            "docs",
            "cost",
            "stats",
            "workflows",
            "scheduled",
            "overseer",
            "voice"
        ]
    );
    // Each is imposable by the lock: a policy of the single list, on the row of its switch.
    for f in Feature::ALL {
        let k = crate::locks::key_of(KEYS, f.policy())
            .unwrap_or_else(|| panic!("{} has no policy", f.id()));
        assert_eq!(
            k.rows,
            [format!(
                "feature{}{}",
                f.id()[..1].to_uppercase(),
                &f.id()[1..]
            )]
        );
    }
    // The essential has no switch: the agents, the conversation, the Git panel, the terminals and the launches. Their
    // tools are of no feature.
    for tool in [
        "list_projects",
        "list_agents",
        "create_agent",
        "send_message",
        "stop_agent",
        "list_pull_requests",
        "ask_user",
        "search_memory",
        "write_memory",
        "get_usage",
        "report_progress",
    ] {
        assert_eq!(of_tool(tool), None, "{tool}");
    }
}

#[test]
fn every_tool_of_a_feature_is_a_tool_the_server_has() {
    let shipped: Vec<String> = crate::mcp::tools::Tools::tool_router()
        .list_all()
        .iter()
        .map(|t| t.name.to_string())
        .collect();
    for (f, tools) in TOOLS {
        for t in *tools {
            assert!(shipped.iter().any(|s| s == t), "{}: {t}", f.id());
        }
    }
}

#[test]
fn a_new_installation_has_every_feature_on_and_nothing_noted_as_used() {
    let dir = test_dir("features-new");
    let data = DataDir::new(dir.join("data"));
    let core = load(&data);
    let s = core.settings.read().clone();
    assert_eq!(s.features, Some(Features::all(true)));
    assert!(s.features_used.is_empty());
    assert_eq!(saved(&data)["features"]["kanban"], json!(true));
}

#[test]
fn an_update_keeps_on_what_it_uses_and_notes_it_the_rest_as_a_new_installation() {
    let dir = test_dir("features-update");
    let data = DataDir::new(dir.join("data"));
    data.ensure().unwrap();
    // Settings of before the switches: the voice set, the Overseer turned on.
    let mut settings = json!({ "claudePath": crate::core_tests::fake_cli(), "sound": false });
    settings["voice"] = json!({ "transcription": "online" });
    settings["overseer"] = json!({ "enabled": true });
    std::fs::write(data.settings_file(), settings.to_string()).unwrap();
    // A project with tickets and a spec, another with neither; a scheduled task.
    let with = dir.join("with");
    std::fs::create_dir_all(with.join(".escouade").join("spec").join("general")).unwrap();
    std::fs::write(with.join(".escouade/spec/general/intro.md"), "# Intro\n").unwrap();
    let without = dir.join("without");
    std::fs::create_dir_all(&without).unwrap();
    let project = |id: &str, path: &std::path::Path| json!({ "id": id, "name": id, "path": path.to_string_lossy(), "color": "" });
    let ticket = Ticket {
        id: "t1".into(),
        project_id: "p1".into(),
        key: "P-1".into(),
        title: "Un ticket".into(),
        ..Default::default()
    };
    let state = json!({
        "projects": [project("p1", &with), project("p2", &without)],
        "tickets": [ticket],
        "scheduled": [{ "id": "s1", "projectId": "p1", "title": "Revue", "rhythm": { "kind": "cron", "cron": "0 9 * * *" } }],
        "ui": { "view": "stats" },
    });
    std::fs::write(data.state_file(), state.to_string()).unwrap();
    let core = load(&data);
    let s = core.settings.read().clone();
    let used: Vec<(Feature, u32)> = s
        .features_used
        .iter()
        .map(|u| (u.feature, u.projects))
        .collect();
    assert_eq!(
        used,
        [
            (Feature::Kanban, 1),
            (Feature::Docs, 1),
            (Feature::Stats, 0),
            (Feature::Scheduled, 1),
            (Feature::Overseer, 0),
            (Feature::Voice, 0),
        ]
    );
    // What it uses is on; the rest as a new installation has it: on.
    assert_eq!(s.features, Some(Features::all(true)));
    let file = saved(&data);
    assert_eq!(file["features"]["docs"], json!(true));
    assert_eq!(
        file["featuresUsed"][0],
        json!({ "feature": "kanban", "projects": 1 })
    );
    // Settled once: the user's choice is never guessed again.
    switch(&core, Feature::Docs, false);
    drop(core);
    let again = load(&data);
    assert!(!again.feature_on(Feature::Docs));
    // Were a new installation to have them off, the rest would be off and what is used on.
    let rest_off = escouade_model::features::settle(&s.features_used, false);
    let on: Vec<Feature> = Feature::ALL
        .into_iter()
        .filter(|f| rest_off.get(*f))
        .collect();
    assert_eq!(
        on,
        [
            Feature::Kanban,
            Feature::Docs,
            Feature::Stats,
            Feature::Scheduled,
            Feature::Overseer,
            Feature::Voice
        ]
    );
}

#[test]
fn a_feature_imposed_by_the_device_is_brought_within_it_and_its_switch_refused_with_who() {
    let h = harness("features-locked");
    h.core.set_managed(managed(&[
        ("features.kanban", json!(false)),
        ("features.docs", json!(true)),
    ]));
    // Brought within, with its row locked for the window.
    assert!(!h.core.feature_on(Feature::Kanban));
    let locks = h.core.locks.read().clone();
    assert_eq!(locks.rows["featureKanban"], ["features.kanban"]);
    assert_eq!(locks.rows["featureDocs"], ["features.docs"]);
    // Turning it on again is refused, said with who imposes it; so is turning off one imposed on.
    let mut s = h.core.settings.read().clone();
    s.features.as_mut().unwrap().set(Feature::Kanban, true);
    let e = h.core.save_settings(s).unwrap_err().to_string();
    assert!(
        e.contains("features.kanban") && e.contains("stratégie de ce poste"),
        "{e}"
    );
    let mut s = h.core.settings.read().clone();
    s.features.as_mut().unwrap().set(Feature::Docs, false);
    assert!(h.core.save_settings(s).is_err());
    // What it does is refused with the same reason, the lock named.
    let why = h.core.feature_check(Feature::Kanban).unwrap_err();
    assert!(
        why.contains("Kanban") && why.contains("stratégie de ce poste"),
        "{why}"
    );
    assert!(h
        .core
        .tool_refusal("create_ticket")
        .unwrap()
        .contains("stratégie"));
    // The policy taken off: the switch is the user's again (off, as it was brought).
    h.core.set_managed(Managed::default());
    switch(&h.core, Feature::Kanban, true);
    assert!(h.core.feature_on(Feature::Kanban));
}

#[test]
fn a_team_may_impose_a_feature_too() {
    let h = harness("features-team");
    let team = crate::locks::TeamPolicies {
        id: "acme".into(),
        team: "Acme".into(),
        enforced: [("features.voice".to_string(), json!(false))].into(),
        ..Default::default()
    };
    let locks = crate::locks::Locks::compute(KEYS, Some(&team), &Managed::default());
    let mut s = Settings {
        features: Some(Features::default()),
        ..Default::default()
    };
    assert_eq!(locks.clamp_settings(&mut s), ["features.voice"]);
    assert!(!s.feature(Feature::Voice));
    assert_eq!(
        locks
            .get("features.voice")
            .unwrap()
            .by
            .as_ref()
            .unwrap()
            .by(),
        "par l’équipe Acme"
    );
    drop(h);
}

#[tokio::test]
async fn the_kanban_off_starts_no_ticket_and_its_discoveries_go_its_tickets_kept() {
    let h = harness("features-kanban");
    let (p, _) = h.project(false).await;
    switch(&h.core, Feature::Kanban, false);
    let t = h
        .core
        .ticket_create(
            &p.id,
            crate::tickets::TicketDraft {
                title: "Un ticket [ok]".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    // The autopilot passes: the ticket stays « À faire ».
    h.core.schedule_now().await;
    assert_eq!(
        h.core
            .tickets
            .read()
            .iter()
            .find(|x| x.id == t.id)
            .unwrap()
            .column,
        crate::model::Column::Todo
    );
    let shown = |core: &Core| -> Vec<Discovery> {
        core.discover_view(Some(1))
            .items
            .iter()
            .map(|i| i.id)
            .collect()
    };
    for d in discoveries(Feature::Kanban) {
        assert!(!shown(&h.core).contains(d), "{d:?}");
    }
    // On again: its ticket is there, its discoveries back, and the autopilot takes it.
    switch(&h.core, Feature::Kanban, true);
    assert!(shown(&h.core).contains(&Discovery::Ticket));
    assert!(h.core.tickets.read().iter().any(|x| x.id == t.id));
}

#[test]
fn the_docs_off_their_discovery_goes_and_their_pages_stay() {
    let h = harness("features-docs");
    h.core
        .onboarding
        .write()
        .discover
        .insert(Discovery::Docs, DiscoverOutcome::Done);
    switch(&h.core, Feature::Docs, false);
    let ids: Vec<Discovery> = h
        .core
        .discover_view(Some(1))
        .items
        .iter()
        .map(|i| i.id)
        .collect();
    assert!(!ids.contains(&Discovery::Docs));
    // What was done stays done: back with the feature.
    switch(&h.core, Feature::Docs, true);
    let back = h.core.discover_view(Some(1));
    let docs = back.items.iter().find(|i| i.id == Discovery::Docs).unwrap();
    assert_eq!(docs.status, crate::discover::DiscoverStatus::Done);
}

#[test]
fn the_editor_off_no_language_server_starts_and_its_settings_stay() {
    let h = harness("features-editor");
    let mut s = h.core.settings.read().clone();
    s.editor_font_size = 15.0;
    h.core.save_settings(s).unwrap();
    switch(&h.core, Feature::Editor, false);
    let why = h.core.feature_check(Feature::Editor).unwrap_err();
    assert!(why.contains("Éditeur"), "{why}");
    let ids: Vec<Discovery> = h
        .core
        .discover_view(Some(1))
        .items
        .iter()
        .map(|i| i.id)
        .collect();
    assert!(!ids.contains(&Discovery::Editor));
    switch(&h.core, Feature::Editor, true);
    assert!(h.core.feature_check(Feature::Editor).is_ok());
    assert_eq!(h.core.settings.read().editor_font_size, 15.0);
}

#[test]
fn the_voice_off_no_dictation_starts_and_its_settings_stay() {
    let h = harness("features-voice");
    let mut s = h.core.settings.read().clone();
    s.voice.language = "fr".into();
    h.core.save_settings(s).unwrap();
    switch(&h.core, Feature::Voice, false);
    assert!(h
        .core
        .feature_check(Feature::Voice)
        .unwrap_err()
        .contains("Voix"));
    let ids: Vec<Discovery> = h
        .core
        .discover_view(Some(1))
        .items
        .iter()
        .map(|i| i.id)
        .collect();
    assert!(!ids.contains(&Discovery::Dictation) && !ids.contains(&Discovery::Voice));
    switch(&h.core, Feature::Voice, true);
    assert_eq!(h.core.settings.read().voice.language, "fr");
    assert!(h
        .core
        .discover_view(Some(1))
        .items
        .iter()
        .any(|i| i.id == Discovery::Dictation));
}

#[test]
fn the_overseer_off_it_does_not_run_though_it_is_turned_on() {
    let h = harness("features-overseer");
    let mut s = h.core.settings.read().clone();
    s.overseer.enabled = true;
    s.mcp_enabled = true;
    *h.core.settings.write() = s;
    assert!(h.core.overseer_wanted());
    {
        let mut s = h.core.settings.write();
        s.features.as_mut().unwrap().set(Feature::Overseer, false);
    }
    assert!(!h.core.overseer_wanted());
    // Its setting stays: it runs again once the feature is on.
    assert!(h.core.settings.read().overseer.enabled);
    h.core
        .settings
        .write()
        .features
        .as_mut()
        .unwrap()
        .set(Feature::Overseer, true);
    assert!(h.core.overseer_wanted());
}

#[test]
fn the_workflows_off_none_is_read_and_they_come_back() {
    let h = harness("features-workflows");
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (p, _) = rt.block_on(h.project(false));
    switch(&h.core, Feature::Workflows, false);
    assert!(h.core.workflows_of(&p.id).off);
    assert!(
        rt.block_on(h.core.refresh_workflows(&p.id, true))
            .unwrap()
            .off
    );
    switch(&h.core, Feature::Workflows, true);
    assert!(!h.core.workflows_of(&p.id).off);
}

#[test]
fn the_cost_and_the_statistics_switch_alone_and_keep_their_data() {
    // Neither runs anything of its own in the background: their views are the window's (src/lib/features.ts), their
    // figures are still counted, to come back whole.
    let h = harness("features-cost-stats");
    for f in [Feature::Cost, Feature::Stats] {
        switch(&h.core, f, false);
        assert!(!h.core.feature_on(f));
        assert!(h.core.feature_check(f).unwrap_err().contains(f.name(true)));
        let off: Vec<Feature> = Feature::ALL
            .into_iter()
            .filter(|x| !h.core.feature_on(*x))
            .collect();
        assert_eq!(off, [f]);
        switch(&h.core, f, true);
    }
    // Saved, read back whole.
    assert_eq!(saved(&h.core.data)["features"]["cost"], json!(true));
}
