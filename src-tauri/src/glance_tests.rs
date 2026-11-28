//! What waits in a core at a glance (ESC-539): what the piles and « Partout » read of another space.

use crate::core_tests::harness;
use crate::locks::{Locks, Managed, AGGREGATE_VIEW, CONTENTLESS, KEYS};
use crate::model::*;
use crate::tickets::TicketDraft;
use serde_json::{json, Value};

fn policy(pairs: &[(&str, Value)]) -> Locks {
    Locks::compute(
        KEYS,
        None,
        &Managed {
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
        },
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn a_glance_names_what_waits_and_a_policy_against_the_aggregate_view_leaves_the_numbers_alone(
) {
    let h = harness("glance");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    // Nothing waits yet.
    let g = h.core.glance(false);
    assert_eq!((g.waiting, g.running), (0, 0));
    assert!(g.agents.is_empty());
    assert_eq!(g.colors, vec![p.color.clone()]);
    // A file outside the folders it reads without asking: the fake `claude` asks to read it.
    let outside = h.dir.join("ailleurs").join("budget.txt");
    std::fs::create_dir_all(outside.parent().unwrap()).unwrap();
    std::fs::write(&outside, "secret").unwrap();
    h.core
        .send_message_files(
            &id,
            format!("lire-fichier {}", outside.display()),
            vec![],
            vec![],
        )
        .await
        .unwrap();
    h.wait("the request", |h| {
        h.agent(&id).status == AgentStatus::Waiting
    })
    .await;
    let g = h.core.glance(false);
    assert_eq!(g.waiting, 1);
    assert!(!g.counts_only);
    let a = &g.agents[0];
    assert_eq!(a.id, id);
    assert_eq!(a.status, AgentStatus::Waiting);
    let r = a.request.as_ref().expect("its request");
    assert_eq!(r.kind, "permission");
    assert_eq!(r.tool, "Read");
    assert_eq!(g.projects.len(), 1);
    assert_eq!(g.projects[0].name, p.name);
    // In JSON as the window reads it.
    let v = serde_json::to_value(&g).unwrap();
    assert_eq!(v["countsOnly"], json!(false));
    assert_eq!(v["agents"][0]["projectId"], json!(p.id));
    // The window's policy asks the numbers alone: no text, no request.
    let g = h.core.glance(true);
    assert_eq!(g.waiting, 1);
    assert!(g.counts_only && g.agents.is_empty() && g.projects.is_empty() && g.asks.is_empty());
    // The core's own policy denies the aggregate view (a work server's): the same, whatever is asked; and its
    // notifications without content are said.
    *h.core.locks.write() = policy(&[(AGGREGATE_VIEW, json!(false)), (CONTENTLESS, json!(true))]);
    let g = h.core.glance(false);
    assert!(g.counts_only && g.contentless);
    assert!(g.agents.is_empty());
    assert_eq!(g.waiting, 1);
    let text = serde_json::to_string(&g).unwrap();
    assert!(!text.contains("budget.txt"), "{text}");
}

fn draft(title: &str) -> TicketDraft {
    TicketDraft {
        title: title.into(),
        criteria: vec!["Un".into(), "Deux".into()],
        max_loops: 5,
        ..Default::default()
    }
}

/// The phone's Kanban (MO-7): every project, the one asked (the first else), the counts of its columns, its tickets in
/// the board's order, the latest done alone.
#[tokio::test(flavor = "multi_thread")]
async fn a_board_glance_gives_a_project_s_columns_in_the_board_s_order() {
    let h = harness("board-glance");
    let (p, _) = h.project(false).await;
    let q = h.second_project().await;
    // Nothing starts by itself: the tickets stay where the test puts them.
    h.core
        .board_set(
            &p.id,
            BoardSettings {
                autopilot: false,
                ..p.board.clone()
            },
        )
        .unwrap();
    let first = h.core.ticket_create(&p.id, draft("Premier")).await.unwrap();
    let second = h.core.ticket_create(&p.id, draft("Second")).await.unwrap();
    let tested = h
        .core
        .ticket_create(&p.id, draft("À tester"))
        .await
        .unwrap();
    h.core
        .edit_ticket(&tested.id, |t| {
            t.column = Column::Review;
            t.iteration = 3;
            t.criteria[0].ok = true;
            t.review_at = Some(10);
            Ok(())
        })
        .unwrap();
    let conflict = h
        .core
        .ticket_create(&p.id, draft("En conflit"))
        .await
        .unwrap();
    h.core
        .edit_ticket(&conflict.id, |t| {
            t.column = Column::Review;
            t.conflict = true;
            t.blocked = Some("Conflit de merge".into());
            t.review_at = Some(5);
            Ok(())
        })
        .unwrap();
    for i in 0..(crate::glance::MAX_DONE + 2) {
        let t = h
            .core
            .ticket_create(&p.id, draft(&format!("Fini {i}")))
            .await
            .unwrap();
        h.core
            .edit_ticket(&t.id, |t| {
                t.column = Column::Done;
                t.done_at = Some(100 + i as i64);
                Ok(())
            })
            .unwrap();
    }
    h.core
        .ticket_create(&q.id, draft("Ailleurs"))
        .await
        .unwrap();

    let g = h.core.board_glance(None);
    assert_eq!(
        g.projects.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(),
        vec![p.id.as_str(), q.id.as_str()]
    );
    assert_eq!(g.project_id.as_deref(), Some(p.id.as_str()));
    assert_eq!(
        (
            g.counts.todo,
            g.counts.doing,
            g.counts.review,
            g.counts.pr,
            g.counts.done
        ),
        (2, 0, 2, 0, crate::glance::MAX_DONE + 2)
    );
    let keys: Vec<&str> = g.tickets.iter().map(|t| t.id.as_str()).collect();
    // « À faire » by priority, « À tester » by arrival, then the latest done.
    assert_eq!(
        &keys[..4],
        &[
            first.id.as_str(),
            second.id.as_str(),
            conflict.id.as_str(),
            tested.id.as_str()
        ]
    );
    assert_eq!(g.tickets.len(), 4 + crate::glance::MAX_DONE);
    let latest = g.tickets.iter().find(|t| t.column == Column::Done).unwrap();
    assert_eq!(
        latest.title,
        format!("Fini {}", crate::glance::MAX_DONE + 1)
    );
    let t = g.tickets.iter().find(|t| t.id == tested.id).unwrap();
    assert_eq!((t.iteration, t.max_loops), (3, 5));
    assert_eq!(t.criteria, vec![true, false]);
    assert!(
        g.tickets
            .iter()
            .find(|t| t.id == conflict.id)
            .unwrap()
            .conflict
    );
    // In JSON as the phone reads it.
    let v = serde_json::to_value(&g).unwrap();
    assert_eq!(v["projectId"], json!(p.id));
    assert_eq!(v["tickets"][2]["column"], json!("review"));
    assert_eq!(v["tickets"][2]["maxLoops"], json!(5));

    // Another project asked: its own; an unknown one: the first.
    let g = h.core.board_glance(Some(&q.id));
    assert_eq!(g.project_id.as_deref(), Some(q.id.as_str()));
    assert_eq!(g.tickets.len(), 1);
    assert_eq!(
        h.core.board_glance(Some("nope")).project_id.as_deref(),
        Some(p.id.as_str())
    );
}

/// A core with no project: the list empty, no project shown.
#[tokio::test(flavor = "multi_thread")]
async fn a_board_glance_of_a_core_with_no_project_shows_none() {
    let h = harness("board-glance-empty");
    let g = h.core.board_glance(None);
    assert!(g.projects.is_empty() && g.project_id.is_none() && g.tickets.is_empty());
}
