//! The squad lead's marks on the box « À toi » (ESC-557) through a real MCP client (rmcp's):
//! `claim_inbox_view` takes a ticket to test in hand (it no longer counts as waiting for the user) and
//! gives it back; `settle_inbox_view` ticks one settled another way into its trace; the refusals: an
//! agent that is not the lead, the setting « Personne », its own ticket, a go-ahead, a task, the docs
//! to share, a review the host asks of the user, a stopped request ticked.

use super::tools_tests::{as_agent, external, project, put, put_agent, read, refused, ticket};
use crate::core_tests::{harness, Harness};
use crate::model::*;
use serde_json::json;

/// The project `p` with `chef` as its squad lead, the server on, the lead allowed to answer.
fn lead_of(h: &Harness, p: &Project, chef: &AgentMeta) {
    h.core.settings.write().mcp_enabled = true;
    h.core.settings.write().agents_answer = AgentsAnswer::Chef;
    h.core
        .projects
        .write()
        .iter_mut()
        .find(|x| x.id == p.id)
        .unwrap()
        .chef = Some(chef.id.clone());
}

#[tokio::test]
async fn the_lead_takes_a_ticket_in_hand_then_settles_it_and_the_user_puts_it_back() {
    let h = harness("mcp-inbox-marks");
    let p = project(&h).await;
    let chef = put_agent(&h, &p, "overseer-fable", 1);
    let dev = put_agent(&h, &p, "dev", 2);
    lead_of(&h, &p, &chef);
    let mut t = ticket(&p, "PUB-1", "Le ticket à tester", Column::Review, 1);
    t.agent_id = Some(dev.id.clone());
    t.review_at = Some(10);
    let t = put(&h, t);
    let key = format!("ticket:{}", t.id);
    assert_eq!(h.core.inbox_views(None).await.total, 1);

    let c = as_agent(&h, &chef.id).await;
    let got = read(
        &c,
        "claim_inbox_view",
        json!({ "ticket": "pub-1", "note": "vérification en cours, chaîne de tests" }),
    )
    .await;
    assert_eq!(got["waiting"], 0);
    assert_eq!(got["lines"][0]["key"], key);
    assert_eq!(got["lines"][0]["source"], "ticketReview");
    assert_eq!(got["lines"][0]["mark"]["kind"], "claimed");
    assert_eq!(got["lines"][0]["mark"]["by"], "overseer-fable");
    assert_eq!(
        got["lines"][0]["mark"]["note"],
        "vérification en cours, chaîne de tests"
    );
    // Given back, then taken again and settled with how.
    let back = read(
        &c,
        "claim_inbox_view",
        json!({ "ticket": "PUB-1", "release": true }),
    )
    .await;
    assert_eq!(back["waiting"], 1);
    assert!(back["lines"][0]["mark"].is_null());
    read(
        &c,
        "claim_inbox_view",
        json!({ "ticket": "PUB-1", "note": "je vérifie" }),
    )
    .await;
    let done = read(
        &c,
        "settle_inbox_view",
        json!({ "ticket": "PUB-1", "note": "fusionné à la main sur main en 79fe7aec" }),
    )
    .await;
    assert_eq!(done["lines"][0]["mark"]["kind"], "settled");
    assert_eq!(done["waiting"], 0);
    let v = h.core.inbox_views(None).await;
    let line = v.views.iter().find(|v| v.key == key).unwrap();
    assert_eq!(
        line.mark.as_ref().unwrap().note,
        "fusionné à la main sur main en 79fe7aec"
    );
    // The user puts it back.
    h.core.inbox_view_reopen(&key).await.unwrap();
    assert_eq!(h.core.inbox_views(None).await.total, 1);
    // The log has the call, never its note.
    let logged = serde_json::to_string(&h.core.mcp.activity.entries()).unwrap();
    assert!(!logged.contains("79fe7aec"), "{logged}");
}

#[tokio::test]
async fn only_the_lead_marks_the_box_never_its_own_ticket_nor_what_stays_the_users() {
    let h = harness("mcp-inbox-marks-refused");
    let p = project(&h).await;
    let chef = put_agent(&h, &p, "overseer-fable", 1);
    let dev = put_agent(&h, &p, "dev", 2);
    lead_of(&h, &p, &chef);
    let mut t = ticket(&p, "PUB-1", "Le ticket à tester", Column::Review, 1);
    t.agent_id = Some(dev.id.clone());
    put(&h, t);
    let mut own = ticket(&p, "PUB-2", "Celui du chef", Column::Review, 2);
    own.agent_id = Some(chef.id.clone());
    put(&h, own);
    let claim = json!({ "ticket": "PUB-1", "note": "je vérifie" });

    // An agent that is not the lead.
    let other = put_agent(&h, &p, "autre", 3);
    let d = as_agent(&h, &dev.id).await;
    let o = as_agent(&h, &other.id).await;
    let e = refused(&o, "claim_inbox_view", claim.clone()).await;
    assert!(e.contains("Seul le chef d’escouade"), "{e}");
    // Its own ticket, by the ticket's agent.
    let e = refused(&d, "settle_inbox_view", claim.clone()).await;
    assert!(e.contains("son propre ticket"), "{e}");
    // « Personne »: the lead neither.
    let c = as_agent(&h, &chef.id).await;
    h.core.settings.write().agents_answer = AgentsAnswer::Nobody;
    let e = refused(&c, "settle_inbox_view", claim.clone()).await;
    assert!(e.contains("Réglages › Agents › Piloter Escouade"), "{e}");
    h.core.settings.write().agents_answer = AgentsAnswer::Chef;
    // Its own ticket.
    let e = refused(
        &c,
        "settle_inbox_view",
        json!({ "ticket": "PUB-2", "note": "fait" }),
    )
    .await;
    assert!(e.contains("son propre ticket"), "{e}");
    // A go-ahead and a task, the user's alone.
    read(
        &d,
        "ask_user",
        json!({ "kind": "confirm", "title": "Publier la 2.2.6 ?", "gate": true }),
    )
    .await;
    read(
        &d,
        "ask_user",
        json!({ "kind": "task", "title": "Essaie sur ton téléphone", "steps": ["Installer", "Ouvrir"] }),
    )
    .await;
    let e = refused(
        &c,
        "settle_inbox_view",
        json!({ "ask": "a-1", "note": "publié" }),
    )
    .await;
    assert!(e.contains("feu vert est le geste de l’utilisateur"), "{e}");
    let e = refused(
        &c,
        "claim_inbox_view",
        json!({ "ask": "a-2", "note": "je le fais" }),
    )
    .await;
    assert!(e.contains("Une tâche est ce que l’utilisateur seul"), "{e}");
    // The docs to share, a review the host asks of the user.
    let e = refused(
        &c,
        "settle_inbox_view",
        json!({ "docs": true, "note": "partagé" }),
    )
    .await;
    assert!(e.contains("Partager les docs reste le geste"), "{e}");
    let e = refused(
        &c,
        "settle_inbox_view",
        json!({ "pullRequest": 7, "note": "relu" }),
    )
    .await;
    assert!(e.contains("reste la sienne"), "{e}");
    // A request an agent stopped for is answered, not ticked; nothing waiting, nothing to take.
    let e = refused(
        &c,
        "settle_inbox_view",
        json!({ "agent": "dev", "note": "fait" }),
    )
    .await;
    assert!(e.contains("pas une coche"), "{e}");
    let e = refused(
        &c,
        "claim_inbox_view",
        json!({ "agent": "dev", "note": "je regarde" }),
    )
    .await;
    assert!(e.contains("n’est arrêté sur aucune demande"), "{e}");
    // One line, and one only.
    let e = refused(&c, "claim_inbox_view", json!({ "note": "x" })).await;
    assert!(e.contains("une seule"), "{e}");
    // A ticket that does not wait for the user.
    put(&h, ticket(&p, "PUB-3", "En cours", Column::Doing, 3));
    let e = refused(
        &c,
        "claim_inbox_view",
        json!({ "ticket": "PUB-3", "note": "x" }),
    )
    .await;
    assert!(e.contains("n’attend pas l’utilisateur"), "{e}");
    // Nothing of it was marked.
    assert!(h.core.inbox_snoozes.lock().marks.is_empty());
    // The lead, at last; and Claude outside Escouade, as for the other answers.
    read(&c, "claim_inbox_view", claim.clone()).await;
    let x = external(&h).await;
    let got = read(
        &x,
        "settle_inbox_view",
        json!({ "ticket": "PUB-1", "note": "fusionné à la main" }),
    )
    .await;
    assert_eq!(got["lines"][0]["mark"]["by"], "Claude · client MCP");
}

#[tokio::test]
async fn a_request_withdrawn_keeps_the_note_the_user_reads_never_a_secret() {
    let h = harness("mcp-inbox-withdraw-note");
    let p = project(&h).await;
    let me = put_agent(&h, &p, "design", 1);
    h.core.settings.write().mcp_enabled = true;
    let c = as_agent(&h, &me.id).await;
    read(
        &c,
        "ask_user",
        json!({ "kind": "answer", "title": "Quel nom pour le module ?" }),
    )
    .await;
    let e = refused(
        &c,
        "withdraw_ask",
        json!({ "ask": "a-1", "reason": "ghp_abcdefghijklmnopqrstuvwxyz0123456789" }),
    )
    .await;
    assert!(e.contains("secret"), "{e}");
    assert_eq!(
        h.core.ask("a-1").unwrap().state,
        crate::asks::AskState::Open
    );
    let done = read(
        &c,
        "withdraw_ask",
        json!({ "ask": "a-1", "reason": "tranché dans la conversation du chef : B" }),
    )
    .await;
    assert_eq!(done["state"], "withdrawn");
    let a = h.core.ask("a-1").unwrap();
    assert_eq!(
        a.reason.as_deref(),
        Some("tranché dans la conversation du chef : B")
    );
    assert!(!a.seen, "the user is shown it once");
    assert_eq!(a.history.last().unwrap().what, "withdrawn");
}
