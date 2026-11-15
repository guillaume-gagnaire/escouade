//! The start on a data folder a power cut damaged (ESC-469): the state, the settings and the plugins' list that do not
//! read come back from their copies, the window is told, and nothing reaches the user's own data folder.

use crate::core::Core;
use crate::model::{Column, PersistedState, Project, Ticket};
use crate::paths::{test_dir, DataDir};
use escouade_model::backups::{self, Damage};
use tauri::test::mock_app;

fn load(data: &DataDir) -> std::sync::Arc<Core> {
    let app = mock_app();
    let (core, _rx) = Core::load(app.handle().clone(), data.clone());
    std::mem::forget(app);
    core
}

fn project(id: &str) -> Project {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "name": "Atelier",
        "path": "/tmp/atelier",
        "color": "oklch(0.7 0.1 40)",
    }))
    .unwrap()
}

/// A state with a project and its two tickets, as the app saves it.
fn state() -> Vec<u8> {
    let tickets = ["ATL-1", "ATL-2"]
        .iter()
        .enumerate()
        .map(|(i, key)| Ticket {
            id: format!("t{i}"),
            project_id: "p1".into(),
            key: (*key).into(),
            title: format!("Ticket {key}"),
            column: Column::Todo,
            ..Default::default()
        })
        .collect();
    let s = PersistedState {
        projects: vec![project("p1")],
        tickets,
        ..Default::default()
    };
    serde_json::to_vec_pretty(&s).unwrap()
}

fn data(name: &str) -> DataDir {
    let data = DataDir::new(test_dir(name).join("data"));
    data.ensure().unwrap();
    // Never the user's own folder.
    assert!(!data
        .root()
        .starts_with(dirs::home_dir().unwrap().join(".escouade")));
    data
}

fn unreadable(data: &DataDir, stem: &str) -> Vec<std::path::PathBuf> {
    std::fs::read_dir(data.root())
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(&format!("{stem}.unreadable-")))
        })
        .collect()
}

#[test]
fn a_state_of_zeros_comes_back_from_its_newest_copy_and_the_window_is_told() {
    let data = data("esc469-start-zeros");
    // A copy made an hour before the cut, then the cut: the file has its size and holds zeros.
    std::fs::write(data.state_file(), state()).unwrap();
    let copied_at = chrono::Utc::now() - chrono::Duration::hours(1);
    assert!(matches!(
        backups::back_up(data.root(), "state.json", copied_at).unwrap(),
        backups::BackedUp::Made(_)
    ));
    let zeros = vec![0u8; state().len()];
    std::fs::write(data.state_file(), &zeros).unwrap();
    std::fs::write(data.settings_file(), [0u8; 512]).unwrap();

    let core = load(&data);
    assert_eq!(core.projects.read().len(), 1);
    let keys: Vec<String> = core.tickets.read().iter().map(|t| t.key.clone()).collect();
    assert_eq!(keys, ["ATL-1", "ATL-2"]);
    // The file of zeros kept, under a dated name.
    let kept = unreadable(&data, "state");
    assert_eq!(kept.len(), 1);
    assert_eq!(std::fs::read(&kept[0]).unwrap(), zeros);
    // The window is told, both files, until the notice is closed.
    let told = backups::notices(data.root());
    assert_eq!(told.len(), 2);
    assert_eq!(told[0].file, "state.json");
    assert_eq!(told[0].damage, Damage::Zeros);
    assert_eq!(
        told[0].restored_at.map(|t| t.timestamp()),
        Some(copied_at.timestamp())
    );
    // The settings had no copy: the defaults, said all the same.
    assert_eq!(told[1].file, "settings.json");
    assert_eq!(told[1].restored, None);
    assert_eq!(unreadable(&data, "settings").len(), 1);

    // Saved after the recovery, then copied: the copy from before stays.
    core.save_now();
    let now = chrono::Utc::now();
    backups::back_up(data.root(), "state.json", now).unwrap();
    let copies = backups::list(data.root(), "state.json");
    assert!(copies
        .iter()
        .any(|(_, c)| c.at.timestamp() == copied_at.timestamp()));
    core.stop_for_good();
}

#[test]
fn without_a_copy_the_app_starts_empty_and_says_so() {
    let data = data("esc469-start-no-copy");
    std::fs::write(data.state_file(), br#"{"projects": [{"id": "p1", "na"#).unwrap();
    let core = load(&data);
    assert!(core.projects.read().is_empty());
    let told = backups::notices(data.root());
    assert_eq!(told.len(), 1);
    assert_eq!(
        (told[0].damage, told[0].restored.as_ref()),
        (Damage::Cut, None)
    );
    assert_eq!(unreadable(&data, "state").len(), 1);
    // Saved empty: the file kept stays as it was.
    core.save_now();
    assert_eq!(
        std::fs::read(&unreadable(&data, "state")[0]).unwrap(),
        br#"{"projects": [{"id": "p1", "na"#
    );
    core.stop_for_good();
}

#[test]
fn a_new_installation_starts_empty_with_nothing_to_tell() {
    let data = data("esc469-start-fresh");
    let core = load(&data);
    assert!(core.projects.read().is_empty());
    assert!(backups::notices(data.root()).is_empty());
    assert!(unreadable(&data, "state").is_empty());
    core.stop_for_good();
}

#[test]
fn a_plugins_list_that_does_not_read_comes_back_from_its_copy() {
    let data = data("esc469-start-plugins");
    let list = br#"{"version": 1, "plugins": []}"#;
    std::fs::write(data.plugins_file(), list).unwrap();
    backups::back_up(data.root(), "plugins.json", chrono::Utc::now()).unwrap();
    std::fs::write(data.plugins_file(), b"{\"version\": 1, \"plu").unwrap();
    let core = load(&data);
    // Read back whole (the app may have written it again since, as it does).
    let read: serde_json::Value =
        serde_json::from_slice(&std::fs::read(data.plugins_file()).unwrap()).unwrap();
    assert!(read["plugins"].is_array());
    let told = backups::notices(data.root());
    assert_eq!(told.len(), 1);
    assert_eq!(told[0].file, "plugins.json");
    assert!(told[0].restored.is_some());
    core.stop_for_good();
}
