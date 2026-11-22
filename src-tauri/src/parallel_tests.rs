//! ESC-620: the one bound of « En parallèle » in the core: 16 on a desktop, 64 on a server, lowered by a ceiling the
//! managed device or the team imposes (the lower holds); the Kanban starts by it, the window and the remote API set it
//! by it (`board_set`, `board_capacity`).

use crate::core_tests::harness;
use crate::locks::{LockedBy, Managed, TeamPolicies, MAX_PARALLEL};
use crate::model::{BoardSettings, Column, Project, Ticket};
use serde_json::json;

fn device(n: u32) -> Managed {
    Managed {
        values: [(
            MAX_PARALLEL.to_owned(),
            (json!(n), "the machine's policy".to_owned()),
        )]
        .into_iter()
        .collect(),
        warnings: Vec::new(),
    }
}

/// The project `p` made one of the team « Acme », which imposes `n` at most.
fn team(h: &crate::core_tests::Harness, p: &Project, n: u32) {
    h.core
        .projects
        .write()
        .iter_mut()
        .find(|x| x.id == p.id)
        .unwrap()
        .team = Some("acme".into());
    h.core.set_team_policies(Some(TeamPolicies {
        id: "acme".into(),
        team: "Acme".into(),
        signed: None,
        recommended: Default::default(),
        enforced: [(MAX_PARALLEL.to_owned(), json!(n))].into_iter().collect(),
    }));
}

fn set(h: &crate::core_tests::Harness, p: &Project, n: u32) -> u32 {
    let board = BoardSettings {
        max_parallel: n,
        ..h.core.project(&p.id).unwrap().board
    };
    h.core.board_set(&p.id, board).unwrap().board.max_parallel
}

#[tokio::test]
async fn sixteen_on_a_desktop_sixty_four_on_a_server() {
    let h = harness("parallel-bound");
    let (p, _) = h.project(false).await;
    assert_eq!(h.core.parallel_max(&p.id), 16);
    let cap = h.core.board_capacity(Some(&p.id)).unwrap();
    assert_eq!(
        (cap.server, cap.limit, cap.max, cap.warn_above),
        (false, 16, 16, 6)
    );
    assert!(cap.ceiling.is_none());
    assert!(cap.machine.cores >= 1 && cap.fits >= 1 && cap.agent_mb > 0);
    // The window (and the remote API, the same command): a value beyond brought within, one within kept.
    assert_eq!(set(&h, &p, 12), 12);
    assert_eq!(set(&h, &p, 30), 16);
    assert_eq!(set(&h, &p, 0), 1);
    assert!(h.core.check_parallel(&p.id, 16).is_ok());
    let beyond = h.core.check_parallel(&p.id, 17).unwrap_err().to_string();
    assert!(
        beyond.contains("16") && beyond.contains("un poste"),
        "{beyond}"
    );

    h.core.mark_server();
    assert_eq!(h.core.parallel_max(&p.id), 64);
    let cap = h.core.board_capacity(Some(&p.id)).unwrap();
    assert_eq!((cap.server, cap.limit, cap.max), (true, 64, 64));
    assert_eq!(set(&h, &p, 40), 40);
    assert_eq!(set(&h, &p, 100), 64);
    let beyond = h.core.check_parallel(&p.id, 65).unwrap_err().to_string();
    assert!(
        beyond.contains("64") && beyond.contains("un serveur"),
        "{beyond}"
    );
    // A project to come (the assistant): the machine's bound.
    assert_eq!(h.core.board_capacity(None).unwrap().max, 64);
}

#[tokio::test]
async fn the_kanban_starts_as_many_tickets_as_its_bound_beyond_six() {
    let h = harness("parallel-start");
    let (p, _) = h.project(false).await;
    h.core.mark_server();
    set(&h, &p, 20);
    let tickets: Vec<Ticket> = (0..30)
        .map(|n| Ticket {
            id: format!("t{n}"),
            project_id: p.id.clone(),
            column: Column::Todo,
            rank: n,
            ..Default::default()
        })
        .collect();
    let mut board = h.core.board_within(&h.core.project(&p.id).unwrap());
    board.autopilot = true;
    assert_eq!(
        crate::board::to_start(&tickets, &p.id, &board, |_| false).len(),
        20
    );
    // Under a ceiling of 8: 8, the setting kept as it is.
    h.core.set_managed(device(8));
    let mut board = h.core.board_within(&h.core.project(&p.id).unwrap());
    board.autopilot = true;
    assert_eq!(board.max_parallel, 8);
    assert_eq!(
        crate::board::to_start(&tickets, &p.id, &board, |_| false).len(),
        8
    );
    assert_eq!(h.core.project(&p.id).unwrap().board.max_parallel, 20);
}

#[tokio::test]
async fn a_ceiling_imposed_by_the_device_or_the_team_lowers_the_bound_and_the_lower_holds() {
    let h = harness("parallel-ceiling");
    let (p, _) = h.project(false).await;
    h.core.mark_server();
    // The device's.
    h.core.set_managed(device(10));
    let cap = h.core.board_capacity(Some(&p.id)).unwrap();
    assert_eq!((cap.limit, cap.max), (64, 10));
    let c = cap.ceiling.unwrap();
    assert_eq!(c.value, 10);
    assert!(matches!(c.by, LockedBy::Device { .. }));
    assert_eq!(set(&h, &p, 12), 10);
    let over = h.core.check_parallel(&p.id, 11).unwrap_err().to_string();
    assert!(
        over.contains("10") && over.contains("stratégie de ce poste"),
        "{over}"
    );
    // The team's, lower: it holds, and is said the team's.
    team(&h, &p, 4);
    let cap = h.core.board_capacity(Some(&p.id)).unwrap();
    assert_eq!(cap.max, 4);
    assert_eq!(
        cap.ceiling.unwrap().by,
        LockedBy::Team {
            team: "Acme".into()
        }
    );
    let over = h.core.check_parallel(&p.id, 5).unwrap_err().to_string();
    assert!(over.contains("l’équipe Acme"), "{over}");
    // The team's, higher than the device's: the device's holds.
    team(&h, &p, 30);
    let cap = h.core.board_capacity(Some(&p.id)).unwrap();
    assert_eq!(cap.max, 10);
    assert!(matches!(cap.ceiling.unwrap().by, LockedBy::Device { .. }));
    // A ceiling above the machine's bound does not raise it.
    h.core.set_managed(Managed::default());
    h.core.set_team_policies(None);
    h.core
        .server
        .store(false, std::sync::atomic::Ordering::Relaxed);
    h.core.set_managed(device(40));
    assert_eq!(h.core.parallel_max(&p.id), 16);
    let over = h.core.check_parallel(&p.id, 17).unwrap_err().to_string();
    assert!(over.contains("un poste"), "{over}");
}

#[test]
fn a_ceiling_out_of_its_range_is_left_out_and_said() {
    use crate::locks::{Locks, KEYS};
    let bad = |v: serde_json::Value| {
        let managed = Managed {
            values: [(
                MAX_PARALLEL.to_owned(),
                (v, "the machine's policy".to_owned()),
            )]
            .into_iter()
            .collect(),
            warnings: Vec::new(),
        };
        Locks::compute(KEYS, None, &managed)
    };
    for v in [json!(0), json!(65), json!("huit")] {
        let l = bad(v.clone());
        assert!(l.max_parallel().is_none(), "{v}");
        assert!(
            l.warnings.iter().any(|w| w.contains(MAX_PARALLEL)),
            "{v}: {:?}",
            l.warnings
        );
    }
    // A registry's DWORD, a string of a number: taken.
    assert_eq!(bad(json!("12")).max_parallel().map(|(n, _)| n), Some(12));
}
