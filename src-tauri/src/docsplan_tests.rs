//! « Planifier » (ESC-262): what the planner is given, what is read of its answer, nothing made
//! twice, and a planning to its end with the fake claude, then cancelled.

use crate::core_tests::harness;
use crate::docs::{Decision, Page, Ref};
use crate::docsplan::*;
use crate::i18n::Lang;
use crate::model::{Column, Ticket};

fn ticket(id: &str, key: &str, title: &str, column: Column) -> Ticket {
    Ticket {
        id: id.into(),
        key: key.into(),
        title: title.into(),
        column,
        ..Default::default()
    }
}

fn proposed(title: &str, after: &[&str], pages: &[&str]) -> Proposed {
    Proposed {
        title: title.into(),
        after: after.iter().map(|s| s.to_string()).collect(),
        pages: pages.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    }
}

#[test]
fn the_tickets_are_read_from_the_answer_and_a_prose_is_none() {
    let answer = "Voici.\n\n```json\n{\"tickets\": [{\"title\": \" Relances \", \"description\": \"Par e-mail.\", \"criteria\": [\"Trois relances\", \"\"], \"after\": [1, \"POR-2\"], \"pages\": [\"a/b\"], \"decisions\": [3, \"D-4\", \"tarif-famille\"]}, {\"title\": \"\"}]}\n```";
    let got = proposed_from_answer(answer).unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].title, "Relances");
    assert_eq!(got[0].criteria, vec!["Trois relances"]);
    assert_eq!(got[0].after, vec!["1", "POR-2"]);
    assert_eq!(
        got[0].decisions,
        vec![
            Ref::Number(3),
            Ref::Number(4),
            Ref::Id("tarif-famille".into())
        ]
    );
    assert!(proposed_from_answer("Je ne sais pas.").is_none());
    assert_eq!(proposed_from_answer("{\"tickets\": []}"), Some(vec![]));
}

#[test]
fn nothing_is_made_twice() {
    let board = vec![
        ticket("t1", "POR-1", "Relances par e-mail", Column::Done),
        ticket("t2", "POR-2", "Reçu fiscal", Column::Todo),
    ];
    let pages = vec!["adherent/relances".to_string()];
    let (make, skipped) = sift(
        vec![
            // The board has it, even done, whatever its case and accents.
            proposed("RELANCES par e‑mail !", &[], &[]),
            proposed("Écran des relances", &["1"], &["adherent/relances", "x/y"]),
            // Twice in the list.
            proposed("ecran des relances", &[], &[]),
            proposed("Reçu fiscal", &[], &[]),
            proposed("Envoi du reçu", &["4", "2", "POR-1", "POR-9"], &[]),
        ],
        &board,
        &pages,
        &[],
    );
    assert_eq!(
        make.iter()
            .map(|m| m.ticket.title.as_str())
            .collect::<Vec<_>>(),
        vec!["Écran des relances", "Envoi du reçu"]
    );
    assert_eq!(
        skipped.iter().map(|s| s.key.as_str()).collect::<Vec<_>>(),
        vec!["POR-1", "POR-2"]
    );
    // After one left out: the ticket of the board it repeats.
    assert_eq!(make[0].after, vec![After::Board("t1".into())]);
    // Only the pages given.
    assert_eq!(make[0].ticket.pages, vec!["adherent/relances"]);
    // After a ticket made before it, one of the board by key; an unknown key is dropped.
    assert_eq!(
        make[1].after,
        vec![
            After::Board("t2".into()),
            After::Made(0),
            After::Board("t1".into())
        ]
    );
}

#[test]
fn at_most_max_tickets_are_made() {
    let list = (0..MAX_TICKETS + 3)
        .map(|i| proposed(&format!("Ticket {i}"), &[], &[]))
        .collect();
    let (make, _) = sift(list, &[], &[], &[]);
    assert_eq!(make.len(), MAX_TICKETS);
}

#[test]
fn the_planner_is_given_the_changes_the_decisions_and_the_board() {
    let page = Page {
        path: "adherent/relances".into(),
        title: "Relances \"e-mail\"".into(),
        body: "Trois relances.".into(),
        ..Default::default()
    };
    let d = Decision {
        number: Some(3),
        title: "E-mail seulement".into(),
        decision: "Pas de SMS.".into(),
        ..Default::default()
    };
    let board = vec![Ticket {
        description: "Le reçu part par e-mail.\nDétail.".into(),
        ..ticket("t1", "POR-1", "Reçu fiscal", Column::Done)
    }];
    let p = prompt(Lang::Fr, &[page], &[d], &board, &["src/mail.rs".into()]);
    assert!(p.contains(
        "<changed-page path=\"adherent/relances\" title=\"Relances 'e-mail'\">\nTrois relances.\n</changed-page>"
    ));
    assert!(p.contains("<decision ref=\"0003\" title=\"E-mail seulement\">\nPas de SMS."));
    assert!(p.contains("POR-1 [done] Reçu fiscal — Le reçu part par e-mail."));
    assert!(!p.contains("Détail"));
    assert!(p.contains("<code-hints>\nsrc/mail.rs\n</code-hints>"));
    assert!(system(Lang::En).contains("\"tickets\""));
}

#[test]
fn the_changes_to_plan_are_kept_until_planned() {
    let mut p = Planning::default();
    let (two, three) = (Ref::Number(2), Ref::Id("trois".into()));
    p.note(&["a/b".into(), "a/c".into()], std::slice::from_ref(&two));
    p.note(&["a/b".into()], &[two.clone(), three.clone()]);
    assert_eq!(p.pages, vec!["a/b", "a/c"]);
    assert_eq!(p.decisions, vec![two.clone(), three.clone()]);
    p.planned(&["a/b".into()], &[three]);
    assert_eq!(
        (p.pages.clone(), p.decisions.clone()),
        (vec!["a/c".to_string()], vec![two])
    );
    for i in 0..25 {
        p.push(Plan {
            id: i.to_string(),
            ..Default::default()
        });
    }
    assert_eq!(p.plans.len(), 20);
    assert_eq!(p.plans.last().unwrap().id, "24");
    // The view: only the pages still there and the decisions not superseded; no plan cancelled.
    p.plan_mut("24").unwrap().cancelled = true;
    let v = view(
        &p,
        &[Page {
            path: "a/c".into(),
            ..Default::default()
        }],
        &[Decision {
            number: Some(2),
            state: crate::docs::State::Superseded,
            ..Default::default()
        }],
    );
    assert_eq!(v.pages, vec!["a/c"]);
    assert!(v.decisions.is_empty());
    assert!(v.last.is_none());
}

#[test]
fn a_ticket_says_the_pages_and_decisions_it_implements() {
    assert_eq!(
        implements(
            Lang::Fr,
            &["a/b".into()],
            &[Ref::Number(3), Ref::Id("tarif-famille".into())]
        ),
        "Implémente ce que dit la spec : pages `a/b` (read_spec_page) ; décisions 0003, tarif-famille (read_decision)."
    );
    assert!(implements(Lang::En, &[], &[]).is_empty());
}

#[tokio::test]
async fn a_planning_makes_the_tickets_links_them_and_is_cancelled() {
    let h = harness("docs-plan");
    let (p, repo) = h.project(false).await;
    // The planner starts nothing; with the autopilot off, nothing starts at all.
    let mut board = h.core.project(&p.id).unwrap().board;
    board.autopilot = false;
    h.core.board_set(&p.id, board).unwrap();
    let space = h.core.docs_space(&p.id, None).await.unwrap();
    space
        .spec_write("adherent/relances", |x| {
            x.title = "Relances".into();
            x.body = "Trois relances par e-mail.".into();
        })
        .unwrap();
    space
        .spec_write("adherent/recu", |x| {
            x.title = "Reçu".into();
            x.body = "Le reçu part par e-mail.".into();
        })
        .unwrap();
    let d = space
        .decision_propose(
            None,
            Decision {
                title: "E-mail seulement".into(),
                ..Default::default()
            },
        )
        .unwrap();
    // A ticket the board has already: proposed again, left out.
    let had = h
        .core
        .ticket_create(
            &p.id,
            crate::tickets::TicketDraft {
                title: "Archive [doublon]".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    // Nothing to plan yet.
    assert!(h.core.docs_plan(&p.id, None).await.is_err());
    h.core.docs_to_plan(
        &p.id,
        &["adherent/relances".into(), "adherent/recu".into()],
        &[d.reference()],
    );
    assert_eq!(h.core.docs_planning(&p.id).await.unwrap().pages.len(), 2);
    let plan = h.core.docs_plan(&p.id, None).await.unwrap();
    assert_eq!(plan.made.len(), 2, "{plan:?}");
    // The first proposed twice, and the board's: left out.
    assert_eq!(plan.skipped.len(), 1, "{plan:?}");
    assert_eq!(plan.skipped[0].key, had.key);
    let made: Vec<Ticket> = plan
        .made
        .iter()
        .map(|m| h.core.ticket(&m.id).unwrap())
        .collect();
    for t in &made {
        assert_eq!(t.column, Column::Todo);
        assert!(t.agent_id.is_none() && t.started_at.is_none());
        assert_eq!(t.criteria.len(), 2);
        assert_eq!(t.pages.len(), 1, "{:?}", t.pages);
        assert!(t.description.contains("read_spec_page"));
        let page = space.spec_read(&t.pages[0]).unwrap();
        assert!(page.tickets.contains(&t.key), "{page:?}");
    }
    assert_eq!(made[0].decisions, vec![d.reference()]);
    assert_eq!(made[1].after, vec![made[0].id.clone()]);
    // Planned: nothing left to plan, and planning again makes nothing twice.
    let v = h.core.docs_planning(&p.id).await.unwrap();
    assert!(v.pages.is_empty() && v.decisions.is_empty());
    assert_eq!(v.last.as_ref().unwrap().id, plan.id);
    assert!(h.core.docs_plan(&p.id, None).await.is_err());
    h.core
        .docs_to_plan(&p.id, &["adherent/relances".into()], &[]);
    let again = h.core.docs_plan(&p.id, None).await.unwrap();
    assert!(again.made.is_empty(), "{again:?}");
    let relances = made
        .iter()
        .find(|t| t.pages == vec!["adherent/relances".to_string()])
        .unwrap();
    assert!(
        again.skipped.iter().any(|s| s.key == relances.key),
        "{again:?}"
    );
    // « Annuler » of the first: its tickets go, their pages no longer name them, the changes are
    // to plan again.
    let c = h.core.docs_plan_cancel(&p.id, &plan.id).await.unwrap();
    assert_eq!(
        c.removed.len(),
        2,
        "{c:?} {:?}",
        made.iter()
            .map(|t| h
                .core
                .ticket(&t.id)
                .map(|x| (x.column, x.agent_id, x.started_at)))
            .collect::<Vec<_>>()
    );
    assert!(h.core.ticket(&made[0].id).is_err());
    assert!(space
        .spec_read("adherent/relances")
        .unwrap()
        .tickets
        .is_empty());
    let v = h.core.docs_planning(&p.id).await.unwrap();
    assert_eq!(v.pages.len(), 2);
    assert_eq!(v.decisions, vec![d.reference()]);
    // Nothing committed by a planning.
    assert!(crate::core_tests::git(&repo, &["status", "--porcelain"]).contains(".escouade/"));
}
