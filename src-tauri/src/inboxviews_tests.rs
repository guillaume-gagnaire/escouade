//! What Escouade puts in the box « À toi » of its own (ESC-513, docs/INBOX.md § 5): each view is the
//! state of its source, settled elsewhere it is gone, settled in the box its source knows it; « Plus
//! tard » leaves the source alone; the server off, nothing shows.

use crate::core_tests::{harness, Harness};
use crate::inboxviews::*;
use crate::model::*;
use serde_json::json;

fn on(h: &Harness) {
    h.core.settings.write().mcp_enabled = true;
}

async fn views(h: &Harness) -> InboxViews {
    h.core.inbox_views(None).await
}

async fn keys(h: &Harness) -> Vec<String> {
    views(h).await.views.into_iter().map(|v| v.key).collect()
}

/// An agent of `p` that asked for `what` (« permission », « exit plan », « question ») and waits.
async fn asking(h: &Harness, p: &Project, what: &str) -> String {
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.send_message(&id, what.into(), vec![]).await.unwrap();
    h.wait("its request", |h| {
        h.agent(&id).status == AgentStatus::Waiting
    })
    .await;
    id
}

fn said(h: &Harness, id: &str, text: &str) -> bool {
    h.items(id).iter().any(|i| i["text"] == text)
}

fn decision(title: &str) -> crate::docs::DecisionPatch {
    crate::docs::DecisionPatch {
        title: Some(title.into()),
        context: Some("Le contexte.".into()),
        decision: Some("Ce qui est décidé.".into()),
        ..Default::default()
    }
}

#[tokio::test]
async fn a_permission_is_a_view_gone_once_answered_in_the_conversation() {
    let h = harness("inbox-views-permission-elsewhere");
    on(&h);
    let (p, _) = h.project(false).await;
    let id = asking(&h, &p, "permission").await;
    let v = views(&h).await;
    assert_eq!(v.blocking, 1);
    assert_eq!(v.total, 1);
    let view = &v.views[0];
    assert_eq!(view.key, format!("req:{id}:req_perm"));
    assert_eq!(view.source, ViewSource::Permission);
    assert_eq!(view.group, ViewGroup::Stopped);
    assert_eq!(view.title, "rm -rf build");
    assert_eq!(view.agent_id.as_deref(), Some(id.as_str()));
    assert_eq!(view.project_name, p.name);
    assert!(view.request.as_ref().is_some_and(|r| r.tool == "Bash"));

    // Answered in the conversation: gone from the box at once, and the box's gesture on what the
    // user read is refused, never sent again.
    h.core
        .answer_permission(&id, "req_perm", "allow", None)
        .unwrap();
    assert!(keys(&h).await.is_empty());
    let late = h
        .core
        .inbox_view_act(&view.key, ViewAct::Allow)
        .await
        .unwrap_err();
    assert_eq!(
        late.to_string(),
        "C’est déjà réglé ailleurs : la boîte se met à jour."
    );
}

#[tokio::test]
async fn a_permission_allowed_in_the_box_frees_its_agent() {
    let h = harness("inbox-views-permission-box");
    on(&h);
    let (p, _) = h.project(false).await;
    let id = asking(&h, &p, "permission").await;
    let key = format!("req:{id}:req_perm");
    h.core.inbox_view_act(&key, ViewAct::Allow).await.unwrap();
    h.wait("the agent going on", |h| said(h, &id, "Commande exécutée."))
        .await;
    assert!(h.agent(&id).status != AgentStatus::Waiting);
    assert!(keys(&h).await.is_empty());
    // Its card in the conversation says it was answered.
    assert!(!h.core.agent(&id).unwrap().lock().has_pending("req_perm"));
}

#[tokio::test]
async fn a_permission_refused_in_the_box_says_why_to_its_agent() {
    let h = harness("inbox-views-permission-deny");
    on(&h);
    let (p, _) = h.project(false).await;
    let id = asking(&h, &p, "permission").await;
    let key = format!("req:{id}:req_perm");
    h.core
        .inbox_view_act(
            &key,
            ViewAct::Deny {
                message: Some("pas maintenant".into()),
            },
        )
        .await
        .unwrap();
    h.wait("the agent told", |h| {
        said(h, &id, "Compris : pas maintenant")
    })
    .await;
}

#[tokio::test]
async fn a_plan_and_a_question_are_answered_in_the_box() {
    let h = harness("inbox-views-plan-question");
    on(&h);
    let (p, _) = h.project(false).await;
    let plan = asking(&h, &p, "exit plan").await;
    let question = asking(&h, &p, "question").await;
    let v = views(&h).await;
    assert_eq!(v.blocking, 2);
    let sources: Vec<ViewSource> = v.views.iter().map(|v| v.source).collect();
    assert_eq!(sources, [ViewSource::Plan, ViewSource::Question]);
    assert_eq!(v.views[0].title, "Plan");
    assert_eq!(v.views[1].title, "Quelle base de données ?");
    // A question is not « allowed »: its gesture is its answers.
    let q = format!("req:{question}:req_question");
    assert!(h.core.inbox_view_act(&q, ViewAct::Allow).await.is_err());
    h.core
        .inbox_view_act(
            &q,
            ViewAct::Answer {
                answers: json!({"Quelle base de données ?": "SQLite"}),
            },
        )
        .await
        .unwrap();
    h.wait("the choice", |h| {
        said(h, &question, "Choix retenu : SQLite")
    })
    .await;
    h.core
        .inbox_view_act(&format!("req:{plan}:req_plan"), ViewAct::Allow)
        .await
        .unwrap();
    h.wait("the plan", |h| said(h, &plan, "Plan approuvé."))
        .await;
    assert!(keys(&h).await.is_empty());
}

#[tokio::test]
async fn a_ticket_to_test_is_approved_in_the_box_and_is_done() {
    let h = harness("inbox-views-ticket");
    on(&h);
    let (p, _) = h.project(false).await;
    let (t, _) = crate::tickets_tests::reviewed(&h, &p.id, "Fichier [ok]").await;
    let v = views(&h).await;
    let view = v
        .views
        .iter()
        .find(|v| v.source == ViewSource::TicketReview)
        .expect("the ticket to approve");
    assert_eq!(view.key, format!("ticket:{}", t.id));
    assert_eq!(view.group, ViewGroup::Today);
    assert_eq!(view.ticket_key.as_deref(), Some(t.key.as_str()));
    assert_eq!(view.title, "Fichier [ok]");
    assert!(view.agent_name.is_some());
    // Its gestures are a ticket's: a permission's is refused.
    assert!(h
        .core
        .inbox_view_act(&view.key, ViewAct::Allow)
        .await
        .is_err());
    h.core
        .inbox_view_act(&view.key, ViewAct::Approve)
        .await
        .unwrap();
    assert_eq!(h.ticket(&t.id).column, Column::Done);
    assert!(!keys(&h).await.contains(&view.key));
}

#[tokio::test]
async fn a_blocked_ticket_is_a_view_with_its_cards_gestures() {
    let h = harness("inbox-views-blocked");
    on(&h);
    let (p, _) = h.project(false).await;
    let (t, _) = crate::tickets_tests::reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core
        .edit_ticket(&t.id, |x| {
            x.blocked = Some("Conflit avec main".into());
            x.conflict = true;
            Ok(())
        })
        .unwrap();
    let v = views(&h).await;
    let view = v
        .views
        .iter()
        .find(|v| v.source == ViewSource::TicketBlocked)
        .expect("the blocked ticket");
    assert_eq!(view.key, format!("blocked:{}", t.id));
    assert_eq!(view.blocked.as_deref(), Some("Conflit avec main"));
    assert!(view.conflict);
    // « Reprendre » is for one « En cours »: a conflict has « L'agent résout » and « Annuler ».
    assert!(h
        .core
        .inbox_view_act(&view.key, ViewAct::Resume)
        .await
        .is_err());
    h.core
        .inbox_view_act(&view.key, ViewAct::Dismiss)
        .await
        .unwrap();
    // Its block gone, it is a ticket to approve again.
    assert!(h.ticket(&t.id).blocked.is_none());
    let k = keys(&h).await;
    assert!(!k.contains(&view.key));
    assert!(k.contains(&format!("ticket:{}", t.id)));
}

#[tokio::test]
async fn a_decision_proposed_is_accepted_in_the_box_or_in_docs() {
    let h = harness("inbox-views-decision");
    on(&h);
    let (p, _) = h.project(false).await;
    let space = h.core.docs_space(&p.id, None).await.unwrap();
    let a = space
        .decision_revise(None, decision("Trois échéances"))
        .unwrap();
    let b = space
        .decision_revise(None, decision("Relances par e-mail"))
        .unwrap();
    let v = views(&h).await;
    let ours: Vec<&InboxItem> = v
        .views
        .iter()
        .filter(|v| v.source == ViewSource::Decision)
        .collect();
    assert_eq!(ours.len(), 2);
    assert!(ours.iter().all(|v| v.group == ViewGroup::Earlier));
    let key_a = format!("decision:{}:{}", p.id, a.id);
    let key_b = format!("decision:{}:{}", p.id, b.id);
    assert!(ours
        .iter()
        .any(|v| v.key == key_a && v.title == "Trois échéances"));

    // Accepted in the box: the register knows it.
    h.core
        .inbox_view_act(&key_a, ViewAct::Accept)
        .await
        .unwrap();
    let after = h.core.docs_space(&p.id, None).await.unwrap();
    let got = after.decision(&crate::docs::Ref::Id(a.id.clone())).unwrap();
    assert_eq!(got.state, crate::docs::State::Accepted);
    assert!(got.number.is_some());
    // Accepted in Docs: gone from the box.
    h.core
        .docs_accept_decision(&p.id, &crate::docs::Ref::Id(b.id.clone()))
        .await
        .unwrap();
    let k = keys(&h).await;
    assert!(!k.contains(&key_a) && !k.contains(&key_b), "{k:?}");
}

#[tokio::test]
async fn the_docs_to_share_and_the_reviews_asked_are_views_as_their_places_last_knew_them() {
    let h = harness("inbox-views-docs-reviews");
    on(&h);
    let (p, _) = h.project(false).await;
    // Not known yet (u32::MAX: told again), none: no view.
    h.core.docs_shares.lock().insert(
        p.id.clone(),
        crate::docsshare::Memo {
            pending: u32::MAX,
            conflict: None,
        },
    );
    assert!(keys(&h).await.is_empty());
    h.core.docs_shares.lock().get_mut(&p.id).unwrap().pending = 3;
    let v = views(&h).await;
    assert_eq!(v.views.len(), 1);
    assert_eq!(v.views[0].key, format!("docs:{}", p.id));
    assert_eq!(v.views[0].files, 3);
    h.core.docs_shares.lock().get_mut(&p.id).unwrap().pending = 0;
    assert!(keys(&h).await.is_empty());

    // The reviews the host asks of the user, as the list was last read: not a draft, not one of
    // the user's own, not one asked of another.
    let pr = |n: u64, author: &str, reviewers: &[&str], draft: bool| crate::forge::PullRequest {
        number: n,
        url: format!("https://example.com/pr/{n}"),
        title: format!("PR {n}"),
        state: crate::forge::PrState::Open,
        draft,
        author: author.into(),
        reviewers: reviewers.iter().map(|r| r.to_string()).collect(),
        ..Default::default()
    };
    let list = crate::prview::ProjectPrs {
        word: "PR".into(),
        prs: vec![
            pr(1, "ada", &["Lea"], false),
            pr(2, "ada", &["lea"], true),
            pr(3, "lea", &["lea"], false),
            pr(4, "ada", &["bob"], false),
        ],
        me: "lea".into(),
    };
    h.core.pr_lists.lock().entry(p.id.clone()).or_default().read =
        Some((std::time::Instant::now(), now_ms(), list));
    let v = views(&h).await;
    assert_eq!(v.views.len(), 1, "{:?}", v.views);
    let r = &v.views[0];
    assert_eq!(r.source, ViewSource::Review);
    assert_eq!(r.number, Some(1));
    assert_eq!(r.url.as_deref(), Some("https://example.com/pr/1"));
    assert_eq!(r.word.as_deref(), Some("PR"));
    // No gesture of the box acts on a pull request: « Ouvrir la PR » is a link.
    assert!(h
        .core
        .inbox_view_act(&r.key, ViewAct::Accept)
        .await
        .is_err());
}

#[tokio::test]
async fn put_off_a_view_leaves_its_source_and_comes_back_never_a_blocking_one() {
    let h = harness("inbox-views-snooze");
    on(&h);
    let (p, _) = h.project(false).await;
    let space = h.core.docs_space(&p.id, None).await.unwrap();
    let d = space
        .decision_revise(None, decision("Tarif famille"))
        .unwrap();
    let key = format!("decision:{}:{}", p.id, d.id);
    let id = asking(&h, &p, "permission").await;
    assert_eq!(views(&h).await.total, 2);

    let later = now_ms() + 3_600_000;
    h.core.inbox_view_snooze(&key, Some(later)).await.unwrap();
    let v = views(&h).await;
    assert_eq!(v.total, 1);
    // Put off, last, with its time; the decision itself still waits.
    let last = v.views.last().unwrap();
    assert_eq!(last.key, key);
    assert_eq!(last.snoozed_until, Some(later));
    let still = h.core.docs_space(&p.id, None).await.unwrap();
    assert_eq!(
        still
            .decision(&crate::docs::Ref::Id(d.id.clone()))
            .unwrap()
            .state,
        crate::docs::State::Proposed
    );
    // Kept across a restart.
    h.core.save_state().unwrap();
    let app = tauri::test::mock_app();
    let data = crate::paths::DataDir::new(h.dir.join("data"));
    let (again, _rx): (
        std::sync::Arc<crate::core::Core<tauri::test::MockRuntime>>,
        _,
    ) = crate::core::Core::load(app.handle().clone(), data);
    assert_eq!(
        again.inbox_snoozes.lock().until.get(&key).copied(),
        Some(later)
    );
    drop(again);
    // A request its agent stopped for is never put off.
    let refused = h
        .core
        .inbox_view_snooze(&format!("req:{id}:req_perm"), Some(later))
        .await
        .unwrap_err();
    assert!(refused.to_string().contains("ne se reporte pas"));
    // Its time come, it is back (and forgotten by the snoozes).
    h.core
        .inbox_snoozes
        .lock()
        .until
        .insert(key.clone(), now_ms() - 1);
    let v = views(&h).await;
    assert_eq!(v.total, 2);
    assert!(v.views.iter().all(|v| v.snoozed_until.is_none()));
    assert!(h.core.inbox_snoozes.lock().until.is_empty());
    // Brought back by hand.
    h.core.inbox_view_snooze(&key, Some(later)).await.unwrap();
    h.core.inbox_view_snooze(&key, None).await.unwrap();
    assert_eq!(views(&h).await.total, 2);
}

#[tokio::test]
async fn the_server_off_nothing_of_the_box_shows_and_every_source_keeps_its_place() {
    let h = harness("inbox-views-off");
    let (p, _) = h.project(false).await;
    let id = asking(&h, &p, "permission").await;
    let v = views(&h).await;
    assert!(!v.enabled);
    assert!(v.views.is_empty());
    assert_eq!((v.blocking, v.total), (0, 0));
    let refused = h
        .core
        .inbox_view_act(&format!("req:{id}:req_perm"), ViewAct::Allow)
        .await
        .unwrap_err();
    assert!(refused.to_string().contains("fermée avec le serveur MCP"));
    // The request still waits where it is today.
    assert_eq!(h.agent(&id).status, AgentStatus::Waiting);
    assert!(h.core.agent(&id).unwrap().lock().has_pending("req_perm"));
    // On again: there it is.
    on(&h);
    assert_eq!(views(&h).await.blocking, 1);
}

#[tokio::test]
async fn a_view_of_a_private_project_is_marked_so_and_the_counts_are_of_every_project() {
    let h = harness("inbox-views-private");
    on(&h);
    let (p, _) = h.project(false).await;
    // Made private once its agent asks (a harness is activated for a private project by hand).
    asking(&h, &p, "permission").await;
    {
        let mut projects = h.core.projects.write();
        projects.iter_mut().find(|x| x.id == p.id).unwrap().private = true;
    }
    let v = h.core.inbox_views(Some("another")).await;
    // None of this project, but counted: « à toi » is of all.
    assert!(v.views.is_empty());
    assert_eq!((v.blocking, v.total), (1, 1));
    let v = h.core.inbox_views(Some(&p.id)).await;
    assert!(v.views[0].private);
}

#[test]
fn a_title_is_a_first_line_that_says_something() {
    assert_eq!(first_line("\n# Plan\n\n1. Écrire"), "Plan");
    assert_eq!(first_line("  rm -rf build  \nsuite"), "rm -rf build");
    let long = "x".repeat(200);
    let cut = first_line(&long);
    assert_eq!(cut.chars().count(), 140);
    assert!(cut.ends_with('…'));
    assert_eq!(first_line(""), "");
}

#[test]
fn a_gesture_comes_as_the_window_sends_it() {
    let a: ViewAct = serde_json::from_value(json!({"act": "allow"})).unwrap();
    assert_eq!(a, ViewAct::Allow);
    let r: ViewAct =
        serde_json::from_value(json!({"act": "reject", "comment": "le test"})).unwrap();
    assert_eq!(
        r,
        ViewAct::Reject {
            comment: "le test".into()
        }
    );
    let q: ViewAct =
        serde_json::from_value(json!({"act": "answer", "answers": {"Q ?": "A"}})).unwrap();
    assert!(matches!(q, ViewAct::Answer { .. }));
    assert!(serde_json::from_value::<ViewAct>(json!({"act": "publish"})).is_err());
}

#[tokio::test]
async fn a_fact_that_moves_a_view_has_the_window_read_the_box_again() {
    let h = harness("inbox-views-told");
    on(&h);
    let (p, _) = h.project(false).await;
    h.events.lock().clear();
    asking(&h, &p, "permission").await;
    h.core.facts.flush();
    let told = h
        .events
        .lock()
        .iter()
        .any(|e| e["type"] == "inbox" && e["projectId"] == p.id.as_str());
    assert!(told);
}
