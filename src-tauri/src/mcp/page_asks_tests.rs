//! The way back of the agents' pages (ESC-676, AP-4; docs/AGENT-PAGES.md § 6 and § 8): `ask` of
//! `show_page` leaves a request in the box by the same call, the page draws its options; what the
//! page proposes (`_/choose`, `Core::page_propose`) is checked against that request (its options
//! alone, `multi`, still waiting) and **answers nothing**: the band of the window sends it on the
//! user's gesture, by the box's own command. A go-ahead is never drawn in a page nor proposed from it.
//! Its answer is told to the page; a new request replaces the one the page carried.

use super::tools_tests::{as_agent, project, put_agent, read, refused};
use crate::agentpage::Proposal;
use crate::asks::{AnswerIn, AskKind, AskState};
use crate::core_tests::{harness, Harness};
use crate::model::*;
use serde_json::{json, Value};

fn review() -> Value {
    json!({
        "subject": { "ticket": "POR-41", "branch": "ticket/por-41", "target": "main" },
        "criteria": [{ "text": "La liste montre 25 adhérents par page", "state": "met" }],
    })
}

fn verdict() -> Value {
    json!({
        "kind": "choice",
        "title": "Fusionner POR-41 ?",
        "options": [
            { "label": "Fusionner dans main" },
            { "label": "Renvoyer avec un mot" },
            { "label": "Attendre" }
        ],
        "recommend": { "option": "a", "why": "Les critères sont tenus." }
    })
}

async fn world(h: &Harness) -> (Project, AgentMeta) {
    let p = project(h).await;
    let a = put_agent(h, &p, "pagination", 1);
    (p, a)
}

fn served(h: &Harness, p: &Project, page: &str, n: u32) -> String {
    std::fs::read_to_string(
        crate::pages::version_dir(h.core.data.root(), &p.id, page, n).join("index.html"),
    )
    .unwrap()
}

fn proposal(options: &[&str]) -> Proposal {
    Proposal {
        options: options.iter().map(|o| o.to_string()).collect(),
        note: None,
    }
}

#[tokio::test]
async fn show_page_leaves_its_request_in_the_box_and_the_page_draws_its_options() {
    let h = harness("page-asks-show");
    let (p, a) = world(&h).await;
    let c = as_agent(&h, &a.id).await;
    let v = read(
        &c,
        "show_page",
        json!({ "title": "Revue de POR-41", "template": "review", "data": review(), "ask": verdict() }),
    )
    .await;
    assert_eq!(v["page"], "p-1");
    let ask = v["ask"].as_str().unwrap().to_string();
    // The same deposit as ask_user's: in the box, by the agent, of its project.
    let kept = h.core.ask(&ask).unwrap();
    assert_eq!(kept.kind, AskKind::Choice);
    assert_eq!(kept.state, AskState::Open);
    assert_eq!(kept.author.agent.as_deref(), Some(a.id.as_str()));
    assert_eq!(kept.project_id, p.id);
    assert_eq!(kept.options.len(), 3);
    // The page carries it, and draws its options with its key.
    assert_eq!(h.core.page_ask("p-1").as_deref(), Some(ask.as_str()));
    let html = served(&h, &p, "p-1", 1);
    assert!(html.contains(&format!("data-ask=\"{ask}\"")), "{html}");
    for o in ["a", "b", "c"] {
        assert!(html.contains(&format!("data-option=\"{o}\"")), "{o}");
    }
    assert!(html.contains("Fusionner dans main"));
    // The box shows the page with it (« Ouvrir la page », its thumbnail).
    h.core.settings.write().mcp_enabled = true;
    let view = h.core.inbox_list(None);
    let w = view.asks.iter().find(|x| x.ask.id == ask).unwrap();
    let page = w.page.as_ref().unwrap();
    assert_eq!(
        (page.page.as_str(), page.template.as_str(), page.version),
        ("p-1", "review", 1)
    );
    // get_page names it.
    let g = read(&c, "get_page", json!({ "page": "p-1" })).await;
    assert_eq!(g["ask"], json!(ask));
}

#[tokio::test]
async fn a_proposal_is_checked_against_the_request_and_answers_nothing() {
    let h = harness("page-asks-propose");
    let (_p, a) = world(&h).await;
    let c = as_agent(&h, &a.id).await;
    let v = read(
        &c,
        "show_page",
        json!({ "title": "Revue", "template": "review", "data": review(), "ask": verdict() }),
    )
    .await;
    let ask = v["ask"].as_str().unwrap().to_string();
    // An option of the request: kept for the band, nothing answered.
    assert!(h.core.page_propose("p-1", proposal(&["a"])));
    let row = h.core.page_row("p-1").unwrap();
    let kept = row.proposal.clone().unwrap();
    assert_eq!(
        (kept.ask.as_str(), kept.options),
        (ask.as_str(), vec!["a".to_string()])
    );
    assert_eq!(h.core.ask(&ask).unwrap().state, AskState::Open);
    assert!(h.core.ask(&ask).unwrap().answer.is_none());
    // An option it does not have, two for a choice of one, none: refused, the last kept as it was.
    for wrong in [&["z"][..], &["a", "b"], &[], &["yes"]] {
        assert!(!h.core.page_propose("p-1", proposal(wrong)), "{wrong:?}");
    }
    assert_eq!(
        h.core.page_row("p-1").unwrap().proposal.unwrap().options,
        ["a"]
    );
    // Another page, or one that carries nothing: refused.
    assert!(!h.core.page_propose("p-2", proposal(&["a"])));
    // « Annuler » lets it go.
    assert!(h.core.page_drop_proposal("p-1"));
    assert!(h.core.page_row("p-1").unwrap().proposal.is_none());
    // A note comes on its lines.
    assert!(h.core.page_propose(
        "p-1",
        Proposal {
            options: vec!["B".into()],
            note: Some("  garde\u{7}la courbe \n".into()),
        }
    ));
    let kept = h.core.page_row("p-1").unwrap().proposal.unwrap();
    assert_eq!(kept.options, ["b"]);
    assert_eq!(kept.note.as_deref(), Some("garde la courbe"));
    // The user's answer (the band sends the box's command): the proposal goes, the page is told,
    // and nothing more is proposed.
    h.core
        .ask_answer(
            &ask,
            AnswerIn {
                options: vec!["b".into()],
                ..Default::default()
            },
            None,
            vec![],
        )
        .unwrap();
    assert!(h.core.page_row("p-1").unwrap().proposal.is_none());
    assert_eq!(
        h.core.pages.lock().told.get("p-1").cloned(),
        Some(vec!["b".to_string()])
    );
    assert!(!h.core.page_propose("p-1", proposal(&["a"])));
    // Taken back within its 10 seconds: the page is told it has no answer, and may propose again.
    h.core.ask_reopen(&ask).unwrap();
    assert_eq!(h.core.pages.lock().told.get("p-1").cloned(), Some(vec![]));
    assert!(h.core.page_propose("p-1", proposal(&["c"])));
}

#[tokio::test]
async fn a_go_ahead_is_never_drawn_in_the_page_nor_proposed_from_it() {
    let h = harness("page-asks-gate");
    let (p, a) = world(&h).await;
    let c = as_agent(&h, &a.id).await;
    let v = read(
        &c,
        "show_page",
        json!({
            "title": "Publier la 2.2.15",
            "template": "review",
            "data": review(),
            "ask": { "kind": "confirm", "title": "Publier la 2.2.15 ?", "gate": true, "yesLabel": "Publier" }
        }),
    )
    .await;
    let ask = v["ask"].as_str().unwrap().to_string();
    assert!(h.core.ask(&ask).unwrap().gate);
    // Not drawn: no option in the page.
    let html = served(&h, &p, "p-1", 1);
    assert!(!html.contains("data-option"), "{html}");
    assert!(!html.contains("data-ask"), "{html}");
    // Never proposed: yes, no, anything.
    for o in ["yes", "no", "a"] {
        assert!(!h.core.page_propose("p-1", proposal(&[o])), "{o}");
    }
    assert!(h.core.page_row("p-1").unwrap().proposal.is_none());
    assert_eq!(h.core.ask(&ask).unwrap().state, AskState::Open);
    // A confirm that is no go-ahead: its yes and no are drawn and proposed.
    let v = read(
        &c,
        "show_page",
        json!({
            "title": "Garder le tri",
            "template": "review",
            "data": review(),
            "ask": { "kind": "confirm", "title": "Garder le tri ?", "yesLabel": "Garder" }
        }),
    )
    .await;
    assert_eq!(v["page"], "p-2");
    let html = served(&h, &p, "p-2", 1);
    assert!(
        html.contains("data-option=\"yes\"") && html.contains("Garder"),
        "{html}"
    );
    assert!(h.core.page_propose("p-2", proposal(&["yes"])));
    assert!(!h.core.page_propose("p-2", proposal(&["yes", "no"])));
}

#[tokio::test]
async fn a_version_keeps_its_request_and_a_new_one_replaces_it() {
    let h = harness("page-asks-version");
    let (p, a) = world(&h).await;
    let c = as_agent(&h, &a.id).await;
    let v = read(
        &c,
        "show_page",
        json!({ "title": "Revue", "template": "review", "data": review(), "ask": verdict() }),
    )
    .await;
    let first = v["ask"].as_str().unwrap().to_string();
    // A version without ask: the same request, drawn again.
    let v = read(
        &c,
        "show_page",
        json!({ "title": "Revue", "template": "review", "data": review(), "page": "p-1" }),
    )
    .await;
    assert_eq!(v["ask"], json!(first));
    assert!(served(&h, &p, "p-1", 2).contains(&format!("data-ask=\"{first}\"")));
    // A version with another: the first withdrawn, never two for the same page.
    let v = read(
        &c,
        "show_page",
        json!({ "title": "Revue", "template": "review", "data": review(), "page": "p-1", "ask": verdict() }),
    )
    .await;
    let second = v["ask"].as_str().unwrap().to_string();
    assert_ne!(second, first);
    assert_eq!(h.core.ask(&first).unwrap().state, AskState::Withdrawn);
    assert_eq!(h.core.ask(&second).unwrap().state, AskState::Open);
    assert_eq!(h.core.page_ask("p-1").as_deref(), Some(second.as_str()));
}

#[tokio::test]
async fn a_request_that_does_not_fit_shows_no_page_and_leaves_nothing() {
    let h = harness("page-asks-refused");
    let (_p, a) = world(&h).await;
    let c = as_agent(&h, &a.id).await;
    // One option is no choice; a confirm takes none; an option's files are the page's.
    for ask in [
        json!({ "kind": "choice", "title": "Un ?", "options": [{ "label": "Seul" }] }),
        json!({ "kind": "confirm", "title": "Oui ?", "options": [{ "label": "A" }, { "label": "B" }] }),
        json!({ "kind": "choice", "title": "Choisis", "options": [
            { "label": "A", "files": [{ "path": "a.png" }] }, { "label": "B" }
        ] }),
    ] {
        refused(
            &c,
            "show_page",
            json!({ "title": "Revue", "template": "review", "data": review(), "ask": ask }),
        )
        .await;
    }
    assert!(h.core.asks_all().is_empty());
    assert!(h.core.page_row("p-1").is_none());
    // The page refused (a template without its data): its request goes with it.
    refused(
        &c,
        "show_page",
        json!({ "title": "Revue", "template": "review", "ask": verdict() }),
    )
    .await;
    assert!(h
        .core
        .asks_all()
        .iter()
        .all(|x| x.state == AskState::Withdrawn));
}

#[test]
fn the_answer_told_to_a_page_is_a_fixed_script_with_ids_alone() {
    let s = crate::pages::answer_script(&["a".into(), "yes".into()]);
    assert!(s.contains("([\"a\",\"yes\"])"), "{s}");
    assert!(s.contains("escouade:answered"));
    // Anything but an id is left out.
    let s = crate::pages::answer_script(&["a\"]);alert(1);//".into(), "</script>".into()]);
    assert!(s.ends_with("([]);"), "{s}");
    assert!(!s.contains("alert"));
}
