//! The parts of a project switched on or off (ESC-658), in the core: who says it (this device, the repository's
//! `.escouade.yml`, the team, else on), what its agents are told, and what stops in the background, nothing deleted.
//! Its MCP tools are `mcp::parts_tests`'.

use super::*;
use crate::core_tests::harness;
use crate::i18n::Lang;
use crate::model::{Column, Criterion, Ticket};

fn write_file(p: &Project, text: &str) {
    std::fs::write(std::path::Path::new(&p.path).join(".escouade.yml"), text).unwrap();
}

fn ticket(p: &Project) -> Ticket {
    Ticket {
        id: "t1".into(),
        project_id: p.id.clone(),
        key: "DEMO-1".into(),
        title: "Un ticket".into(),
        criteria: vec![Criterion {
            text: "il marche".into(),
            ..Default::default()
        }],
        max_loops: 3,
        column: Column::Todo,
        ..Default::default()
    }
}

#[tokio::test]
async fn the_device_goes_before_the_repository_which_goes_before_everything_on() {
    let h = harness("parts-order");
    let (p, _) = h.project(false).await;
    // Nothing said: every part on, as before.
    for part in Part::ALL {
        assert!(h.core.part_on(&p.id, part), "{part:?}");
    }
    // The repository proposes the Docs and the launch off: off while the device chose nothing.
    write_file(&p, "version: 1\nparts:\n  docs: false\n  launch: false\n");
    h.core.read_shared(&p.id, true);
    assert!(!h.core.part_on(&p.id, Part::Docs));
    assert!(!h.core.part_on(&p.id, Part::Launch));
    assert!(h.core.part_on(&p.id, Part::Kanban));
    let now = h.core.project(&p.id).unwrap();
    assert_eq!(state(&now, Part::Docs).by, PartBy::Repo);
    // No warning: the keys are known to the file.
    assert!(now.shared.file.as_ref().unwrap().warnings.is_empty());
    // Nor are they applied to the project or kept on this machine: the file's alone.
    assert!(now.parts.is_empty());
    assert!(now.shared.local.is_empty());

    // The device turns the Docs on: it wins, and the file read again does not change it.
    h.core
        .set_project_part(&p.id, Part::Docs, Some(true))
        .unwrap();
    assert!(h.core.part_on(&p.id, Part::Docs));
    write_file(
        &p,
        "version: 1\nparts:\n  docs: false\n  launch: false\n  kanban: false\n",
    );
    h.core.read_shared(&p.id, false);
    assert!(h.core.part_on(&p.id, Part::Docs));
    assert!(!h.core.part_on(&p.id, Part::Kanban));
    // Saved with the project, read back after a restart.
    let saved = serde_json::to_value(h.core.project(&p.id).unwrap()).unwrap();
    assert_eq!(saved["parts"], serde_json::json!({ "docs": true }));

    // A choice made with no file wins over a file that comes after it.
    h.core
        .set_project_part(&p.id, Part::Memory, Some(true))
        .unwrap();
    write_file(&p, "version: 1\nparts:\n  memory: false\n");
    h.core.read_shared(&p.id, false);
    assert!(h.core.part_on(&p.id, Part::Memory));

    // « Reprendre le défaut »: the repository's again.
    h.core.set_project_part(&p.id, Part::Memory, None).unwrap();
    assert!(!h.core.part_on(&p.id, Part::Memory));
    // The file gone: on, as before.
    std::fs::remove_file(std::path::Path::new(&p.path).join(".escouade.yml")).unwrap();
    h.core.read_shared(&p.id, false);
    assert!(h.core.part_on(&p.id, Part::Memory));
}

#[tokio::test]
async fn a_repository_that_says_true_turns_nothing_on_the_device_turned_off_nor_lifts_a_protection()
{
    let h = harness("parts-repo-true");
    let (p, _) = h.project(false).await;
    h.core
        .set_project_part(&p.id, Part::Kanban, Some(false))
        .unwrap();
    // A repository that proposes everything on, and commands to run.
    write_file(
        &p,
        "version: 1\nparts:\n  docs: true\n  memory: true\n  kanban: true\n  launch: true\nlaunch:\n  - name: web\n    command: npm run dev\n",
    );
    h.core.read_shared(&p.id, true);
    assert!(!h.core.part_on(&p.id, Part::Kanban));
    let now = h.core.project(&p.id).unwrap();
    // Its commands still wait for the user's trust: the launch on is no reason to run them.
    assert!(now.shared.file.as_ref().unwrap().pending.is_some());
    assert!(now.run_commands.is_empty());
    // Its parts on change none of the project's own switches: the autopilot is still the user's.
    assert_eq!(now.board.autopilot, p.board.autopilot);
}

#[tokio::test]
async fn the_feature_of_the_app_off_takes_the_part_with_it_whatever_its_switch() {
    let h = harness("parts-feature");
    let (p, _) = h.project(false).await;
    h.core
        .settings
        .write()
        .features
        .get_or_insert_with(Default::default)
        .set(crate::features::Feature::Docs, false);
    assert!(!h.core.part_on(&p.id, Part::Docs));
    let why = h.core.part_check(&p.id, Part::Docs).unwrap_err();
    assert!(why.contains("Réglages › Fonctionnalités"), "{why}");
    // The memory and the launch are of no feature.
    assert!(h.core.part_on(&p.id, Part::Memory));
}

#[tokio::test]
async fn the_team_imposes_over_the_device_and_its_switch_is_refused() {
    let h = harness("parts-team");
    let (p, _) = h.project(false).await;
    {
        let mut projects = h.core.projects.write();
        let x = projects.iter_mut().find(|x| x.id == p.id).unwrap();
        x.parts.set(Part::Kanban, Some(true));
        x.shared.team_parts = Some(escouade_model::projectfile::TeamPlugins {
            team: "Acme".into(),
            defaults: [("parts.docs".to_string(), serde_json::json!(false))].into(),
            enforced: [("parts.kanban".to_string(), serde_json::json!(false))].into(),
        });
    }
    assert!(!h.core.part_on(&p.id, Part::Kanban));
    let why = h
        .core
        .set_project_part(&p.id, Part::Kanban, Some(true))
        .unwrap_err();
    assert!(why.to_string().contains("Acme"), "{why}");
    let why = h.core.part_check(&p.id, Part::Kanban).unwrap_err();
    assert!(why.contains("par l’équipe Acme"), "{why}");
    // What the team recommends, the device changes.
    assert!(!h.core.part_on(&p.id, Part::Docs));
    h.core
        .set_project_part(&p.id, Part::Docs, Some(true))
        .unwrap();
    assert!(h.core.part_on(&p.id, Part::Docs));
    // Read from the team's layers: the keys `parts.*` alone, booleans alone.
    let layers = crate::team_settings::Layers {
        defaults: [
            ("parts.launch".to_string(), serde_json::json!(false)),
            ("board.draft_pr".to_string(), serde_json::json!(true)),
        ]
        .into(),
        enforced: [("parts.memory".to_string(), serde_json::json!("off"))].into(),
        accepted: Default::default(),
    };
    let t = crate::projectfile::team_parts("Acme", &layers).unwrap();
    assert_eq!(t.defaults.len(), 1);
    assert!(t.enforced.is_empty());
}

#[tokio::test]
async fn its_agents_are_told_nothing_of_a_part_off() {
    let h = harness("parts-told");
    let (p, _) = h.project(false).await;
    let t = ticket(&p);
    let told = |h: &crate::core_tests::Harness| {
        let p = h.core.project(&p.id).unwrap();
        let protocol = h
            .core
            .ticket_instructions(&p, Lang::Fr, &t, Some(4130), false, None);
        let memory =
            crate::core::with_memory_of(Lang::Fr, Some(protocol.clone()), h.core.told_of(&p))
                .unwrap();
        (protocol, memory)
    };
    let (protocol, all) = told(&h);
    assert!(protocol.contains("search_spec") && protocol.contains("\"lancement\""));
    assert!(all.contains("search_memory") && all.contains("search_documents"));

    h.core
        .set_project_part(&p.id, Part::Docs, Some(false))
        .unwrap();
    let (protocol, all) = told(&h);
    assert!(
        !protocol.contains("search_spec") && !protocol.contains("write_spec_page"),
        "{protocol}"
    );
    assert!(!all.contains("search_documents"), "{all}");
    assert!(all.contains("search_memory"));
    assert!(h
        .core
        .docs_follow(&h.core.project(&p.id).unwrap(), Lang::Fr)
        .await
        .is_none());

    h.core
        .set_project_part(&p.id, Part::Memory, Some(false))
        .unwrap();
    let (_, all) = told(&h);
    assert!(
        !all.contains("search_memory") && !all.contains("mémoire partagée"),
        "{all}"
    );
    // Nothing left to tell: no instructions at all.
    assert_eq!(
        crate::core::with_memory_of(
            Lang::Fr,
            None,
            h.core.told_of(&h.core.project(&p.id).unwrap())
        ),
        None
    );

    h.core
        .set_project_part(&p.id, Part::Launch, Some(false))
        .unwrap();
    let (protocol, _) = told(&h);
    // Its ports still said, no recipe asked.
    assert!(
        protocol.contains("4130") && !protocol.contains("\"lancement\""),
        "{protocol}"
    );

    // A brainstorm of the Docs proposes no ticket while the Kanban is off.
    use crate::discussion::{instructions_of, Kind};
    assert!(instructions_of(Lang::Fr, Kind::Brainstorm, "demo", true).contains("tickets"));
    assert!(!instructions_of(Lang::Fr, Kind::Brainstorm, "demo", false).contains("ticket"));

    // On again: all of it back, word for word.
    for part in Part::ALL {
        h.core.set_project_part(&p.id, part, Some(true)).unwrap();
    }
    let (protocol, all) = told(&h);
    assert_eq!(
        all,
        crate::core::with_memory(Lang::Fr, Some(protocol.clone())).unwrap()
    );
    assert!(protocol.contains("search_spec") && protocol.contains("\"lancement\""));
}

#[tokio::test]
async fn the_kanban_off_starts_none_of_its_tickets_reads_no_workflow_and_keeps_its_tickets() {
    let h = harness("parts-kanban");
    let (p, _) = h.project(false).await;
    h.core
        .set_project_part(&p.id, Part::Kanban, Some(false))
        .unwrap();
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
    h.core.schedule_now().await;
    let column = |h: &crate::core_tests::Harness| {
        h.core
            .tickets
            .read()
            .iter()
            .find(|x| x.id == t.id)
            .map(|x| x.column)
    };
    assert_eq!(column(&h), Some(Column::Todo));
    // Its workflows are neither read nor followed.
    assert!(h.core.workflows_of(&p.id).off);
    assert!(h.core.refresh_workflows(&p.id, true).await.unwrap().off);
    assert!(!h.core.project(&p.id).unwrap().workflows_on());
    // No planner: it would make tickets.
    let why = h.core.docs_plan(&p.id, None).await.unwrap_err();
    assert!(why.to_string().contains("Kanban"), "{why}");
    // On again: its ticket is there, its workflows read.
    h.core.set_project_part(&p.id, Part::Kanban, None).unwrap();
    assert!(column(&h).is_some());
    assert!(!h.core.workflows_of(&p.id).off);
}

#[tokio::test]
async fn the_docs_off_compare_nothing_start_no_discussion_and_keep_their_pages() {
    let h = harness("parts-docs");
    let (p, _) = h.project(false).await;
    h.core
        .set_project_part(&p.id, Part::Docs, Some(false))
        .unwrap();
    let why = h.core.docs_compare(&p.id, None).await.unwrap_err();
    assert!(why.to_string().contains("Docs"), "{why}");
    let why = h
        .core
        .discussion_start(
            &p.id,
            crate::discussion::Start {
                subject: "La spec".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(why.to_string().contains("Docs"), "{why}");
    let why = h.core.docs_plan(&p.id, None).await.unwrap_err();
    assert!(why.to_string().contains("Docs"), "{why}");
}

#[tokio::test]
async fn the_launch_off_runs_none_of_its_commands_probes_nothing_and_keeps_them() {
    let h = harness("parts-launch");
    let (p, _) = h.project(false).await;
    h.core
        .set_project_part(&p.id, Part::Launch, Some(false))
        .unwrap();
    let why = h.core.part_check(&p.id, Part::Launch).unwrap_err();
    assert!(
        why.contains("Lancement et navigateurs") && why.contains("demo"),
        "{why}"
    );
    // An agent's test launch running stops with it.
    h.core
        .test_runs
        .lock()
        .insert("ghost".into(), vec!["term".into()]);
    h.core.launch_part_off(&p.id);
    // An agent of another project, or none: untouched.
    assert!(h.core.test_runs.lock().contains_key("ghost"));
    h.core.set_project_part(&p.id, Part::Launch, None).unwrap();
    assert!(h.core.part_check(&p.id, Part::Launch).is_ok());
}
