use super::*;
use crate::projectfile::{SharedFile, TeamPlugins};
use serde_json::json;
use std::collections::BTreeMap;

fn project() -> Project {
    serde_json::from_value(json!({ "id": "p1", "name": "demo", "path": "C:/demo", "color": "red" }))
        .unwrap()
}

fn file(values: &[(&str, serde_json::Value)]) -> Option<SharedFile> {
    Some(SharedFile {
        keys: values.iter().map(|(k, _)| k.to_string()).collect(),
        values: values
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
        ..Default::default()
    })
}

fn team(defaults: &[(&str, bool)], enforced: &[(&str, bool)]) -> Option<TeamPlugins> {
    let map = |m: &[(&str, bool)]| -> BTreeMap<String, serde_json::Value> {
        m.iter().map(|(k, v)| (k.to_string(), json!(v))).collect()
    };
    Some(TeamPlugins {
        team: "Acme".into(),
        defaults: map(defaults),
        enforced: map(enforced),
    })
}

#[test]
fn a_project_where_nobody_says_anything_has_every_part() {
    let p = project();
    for s in states(&p) {
        assert!(s.on, "{:?}", s.part);
        assert_eq!(s.by, PartBy::Default);
        assert_eq!(s.team, None);
    }
    // A project saved before the parts reads the same, and saves nothing of them.
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.get("parts").is_none());
}

#[test]
fn the_repository_proposes_and_this_device_goes_first() {
    let mut p = project();
    p.shared.file = file(&[("parts.docs", json!(false)), ("parts.kanban", json!(true))]);
    assert_eq!(
        state(&p, Part::Docs),
        PartState {
            part: Part::Docs,
            on: false,
            by: PartBy::Repo,
            team: None
        }
    );
    assert_eq!(state(&p, Part::Kanban).by, PartBy::Repo);
    assert_eq!(state(&p, Part::Memory).by, PartBy::Default);
    // The device chose: its choice wins, whichever way.
    p.parts.set(Part::Docs, Some(true));
    p.parts.set(Part::Kanban, Some(false));
    assert_eq!(
        state(&p, Part::Docs),
        PartState {
            part: Part::Docs,
            on: true,
            by: PartBy::Device,
            team: None
        }
    );
    assert_eq!(
        state(&p, Part::Kanban),
        PartState {
            part: Part::Kanban,
            on: false,
            by: PartBy::Device,
            team: None
        }
    );
    // A file that changes later does not change it.
    p.shared.file = file(&[("parts.docs", json!(false)), ("parts.kanban", json!(true))]);
    assert!(on(&p, Part::Docs));
    assert!(!on(&p, Part::Kanban));
    // Forgotten (« Reprendre le défaut »): the repository's again.
    p.parts.set(Part::Docs, None);
    assert!(!on(&p, Part::Docs));
    // A value that is not a boolean is no default.
    p.shared.file = file(&[("parts.docs", json!("false"))]);
    assert_eq!(state(&p, Part::Docs).by, PartBy::Default);
}

#[test]
fn a_choice_made_before_the_file_wins_over_the_file_that_comes_after() {
    let mut p = project();
    p.parts.set(Part::Memory, Some(true));
    p.shared.file = file(&[("parts.memory", json!(false))]);
    assert!(on(&p, Part::Memory));
}

#[test]
fn the_team_imposes_over_the_device_and_recommends_under_the_repository() {
    let mut p = project();
    p.shared.team_parts = team(
        &[("parts.launch", false), ("parts.docs", false)],
        &[("parts.kanban", false)],
    );
    p.parts.set(Part::Kanban, Some(true));
    p.shared.file = file(&[("parts.docs", json!(true))]);
    let k = state(&p, Part::Kanban);
    assert_eq!(
        (k.on, k.by, k.team.as_deref()),
        (false, PartBy::TeamEnforced, Some("Acme"))
    );
    // The repository before the team's recommendation.
    assert_eq!(state(&p, Part::Docs).by, PartBy::Repo);
    assert!(on(&p, Part::Docs));
    let l = state(&p, Part::Launch);
    assert_eq!(
        (l.on, l.by, l.team.as_deref()),
        (false, PartBy::TeamDefault, Some("Acme"))
    );
    // The device before the team's recommendation.
    p.parts.set(Part::Launch, Some(true));
    assert_eq!(state(&p, Part::Launch).by, PartBy::Device);
}

#[test]
fn the_choices_are_saved_and_read_back() {
    let mut p = project();
    p.parts.set(Part::Launch, Some(false));
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v["parts"], json!({ "launch": false }));
    let back: Project = serde_json::from_value(v).unwrap();
    assert_eq!(back.parts.get(Part::Launch), Some(false));
    assert_eq!(back.parts.get(Part::Docs), None);
}

#[test]
fn each_part_has_its_id_key_and_feature() {
    for part in Part::ALL {
        assert_eq!(Part::of(part.id()), Some(part));
        assert_eq!(part.key(), format!("parts.{}", part.id()));
    }
    assert_eq!(Part::Docs.feature(), Some(Feature::Docs));
    assert_eq!(Part::Kanban.feature(), Some(Feature::Kanban));
    assert_eq!(Part::Memory.feature(), None);
    assert_eq!(Part::of("editor"), None);
}
