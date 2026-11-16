//! The screen of the copies (ESC-487) on a real core: a closed project's history lost and found again, a copy of the
//! state compared then chosen for the next start, refused while an agent works, and nothing on the user's own folder.

use crate::closed_tests::{add_again, close};
use crate::core_tests::harness;
use crate::model::*;
use crate::tickets::TicketDraft;
use escouade_model::backups;

fn draft(title: &str) -> TicketDraft {
    TicketDraft {
        title: title.into(),
        max_loops: 5,
        ..Default::default()
    }
}

fn not_the_users_folder(h: &crate::core_tests::Harness) {
    let own = dirs::home_dir().unwrap().join(".escouade");
    assert!(!h.core.data.root().starts_with(own));
}

#[tokio::test]
async fn a_closed_projects_history_lost_is_found_again_from_its_copy_and_reopened() {
    let h = harness("esc487-lost-history");
    not_the_users_folder(&h);
    let (p, repo) = h.project(false).await;
    {
        let mut board = h.core.project(&p.id).unwrap().board;
        board.autopilot = false;
        h.core.board_set(&p.id, board).unwrap();
    }
    h.core
        .ticket_create(&p.id, draft("Relances par e-mail"))
        .await
        .unwrap();
    close(&h, &p.id).await;
    let file = format!("closed-projects/{}.json", p.id);
    // The copies of the hour: the history among them.
    h.core.back_up_now();
    let view = h.core.backups_view();
    let copies = &view.files.iter().find(|f| f.file == file).unwrap().copies;
    assert_eq!(copies.len(), 1);
    assert_eq!(copies[0].summary.tickets, Some(1));
    assert!(view.lost.is_empty(), "its file is there");
    // The history lost (as the cut of 2026-11-15 lost the state): its file gone, the index not citing it.
    std::fs::remove_file(h.core.data.closed_project_file(&p.id)).unwrap();
    h.core.closed_index.lock().remove(&p.id);
    let view = h.core.backups_view();
    assert_eq!(view.lost.len(), 1);
    let lost = &view.lost[0];
    assert_eq!((lost.id.as_str(), lost.tickets), (p.id.as_str(), 1));
    assert_eq!(lost.name, p.name);
    // Found again: at once, nothing to restart; among the closed projects.
    let done = h.core.restore_backup(&file, &lost.copy).unwrap();
    assert!(done.done && !done.restart);
    assert_eq!(done.closed_id.as_deref(), Some(p.id.as_str()));
    assert!(done.set_aside.is_none(), "no file of now to set aside");
    assert!(h.core.closed_projects().iter().any(|c| c.id == p.id));
    assert!(h.core.backups_view().lost.is_empty());
    // « Rouvrir »: the project and its ticket back.
    let back = add_again(&h, &repo).await;
    assert_eq!(back.id, p.id);
    assert!(h
        .core
        .tickets
        .read()
        .iter()
        .any(|t| t.project_id == p.id && t.title == "Relances par e-mail"));
    // Open now: its history's copies are not taken back over it.
    let name = h
        .core
        .backups_view()
        .files
        .iter()
        .find(|f| f.file == file)
        .unwrap()
        .copies[0]
        .name
        .clone();
    let e = h.core.restore_backup(&file, &name).unwrap_err();
    assert!(e.to_string().contains("Ce projet est ouvert"), "{e}");
}

#[tokio::test]
async fn a_copy_of_the_state_is_compared_then_chosen_for_the_next_start_never_while_an_agent_works()
{
    let h = harness("esc487-state");
    not_the_users_folder(&h);
    let (p, _) = h.project(false).await;
    {
        let mut board = h.core.project(&p.id).unwrap().board;
        board.autopilot = false;
        h.core.board_set(&p.id, board).unwrap();
    }
    h.core.ticket_create(&p.id, draft("Facture")).await.unwrap();
    h.core.save_now();
    h.core.back_up_now();
    // A ticket more today than in the copy.
    h.core.ticket_create(&p.id, draft("Panier")).await.unwrap();
    h.core.save_now();
    let view = h.core.backups_view();
    let state = view.files.iter().find(|f| f.file == "state.json").unwrap();
    assert!(state.exists && state.today.readable);
    assert_eq!(state.today.tickets, Some(2));
    let copy = state.copies[0].clone();
    assert_eq!(copy.summary.tickets, Some(1));
    assert_eq!(copy.summary.projects, Some(1));
    // Comparing writes nothing.
    let before = std::fs::read(h.core.data.state_file()).unwrap();
    let d = h.core.compare_backup("state.json", &copy.name).unwrap();
    let tickets = d.kinds.iter().find(|k| k.kind == "tickets").unwrap();
    assert!(tickets.gained.is_empty());
    assert_eq!(tickets.lost.len(), 1);
    assert!(tickets.lost[0].ends_with("Panier"));
    assert_eq!(std::fs::read(h.core.data.state_file()).unwrap(), before);
    assert!(h
        .core
        .compare_backup("state.json", "../state.json")
        .is_err());
    // An agent at work: refused, nothing chosen.
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.agent(&a).unwrap().lock().meta.status = AgentStatus::Running;
    let e = h.core.restore_backup("state.json", &copy.name).unwrap_err();
    assert!(e.to_string().starts_with("1 agent(s) au travail"), "{e}");
    assert!(backups::pending(h.core.data.root()).is_none());
    assert_eq!(h.core.backups_view().at_work, 1);
    h.core.agent(&a).unwrap().lock().meta.status = AgentStatus::Done;
    // Idle: chosen for the next start, the file of now untouched until then.
    let out = h.core.restore_backup("state.json", &copy.name).unwrap();
    assert!(!out.done && out.restart);
    let pending = h.core.backups_view().pending.unwrap();
    assert_eq!(
        (pending.file.as_str(), pending.copy.as_str()),
        ("state.json", copy.name.as_str())
    );
    assert_eq!(std::fs::read(h.core.data.state_file()).unwrap(), before);
    h.core.cancel_restore().unwrap();
    assert!(h.core.backups_view().pending.is_none());
}
