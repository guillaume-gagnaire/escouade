//! The spec the tickets keep (ESC-263): their protocol asks for it; on a fixed branch of the docs,
//! what a ticket's agent writes waits in a copy and is poured on the branch at the validation of
//! its ticket, never before, without undoing what the team wrote meanwhile.

use crate::core_tests::harness;
use crate::docs::*;
use crate::docs_tests::{proposal, same, set_docs_branch};
use crate::docsticket::{pour, seed, Poured};
use crate::engine::Engine;
use crate::paths::test_dir;

fn spaces(name: &str) -> (Space, Space, Space) {
    let into = Space::new(test_dir(&format!("{name}-into")), None);
    into.spec_write("paiement/echeances", |p| p.body = "Trois échéances.".into())
        .unwrap();
    into.spec_write("paiement/recu", |p| p.body = "Par e-mail.".into())
        .unwrap();
    into.spec_write("compte/connexion", |p| p.body = "Par mot de passe.".into())
        .unwrap();
    into.decision_propose(None, proposal("Trois échéances"))
        .unwrap();
    into.decision_accept(
        &Ref::Id("trois-echeances".into()),
        "Léa",
        &Register::default(),
        None,
    )
    .unwrap();
    // A proposal of the team, still open.
    into.decision_propose(None, proposal("Tarif famille"))
        .unwrap();
    let base = Space::new(test_dir(&format!("{name}-base")), None);
    let work = Space::new(test_dir(&format!("{name}-work")), None);
    seed(&into, &base).unwrap();
    seed(&into, &work).unwrap();
    (into, base, work)
}

#[test]
fn only_what_the_ticket_changed_is_poured_and_the_teams_writes_stay() {
    let (into, base, work) = spaces("dt-pour");
    // The ticket: a page changed, one made, one removed, a decision.
    work.spec_write("paiement/echeances", |p| {
        p.body = "Quatre échéances.".into()
    })
    .unwrap();
    work.spec_write("paiement/acompte", |p| {
        p.body = "Un acompte de 30 %.".into()
    })
    .unwrap();
    work.raw_remove(SPEC, "compte/connexion.md").unwrap();
    work.decision_propose(None, proposal("Quatre échéances"))
        .unwrap();
    // The team meanwhile, on the branch: another page.
    into.spec_write("paiement/recu", |p| p.body = "Par e-mail, en PDF.".into())
        .unwrap();
    let poured = pour(&work, &base, &into).unwrap();
    assert_eq!(
        poured,
        Poured {
            pages: vec!["paiement/acompte".into(), "paiement/echeances".into()],
            removed: vec!["compte/connexion".into()],
            decisions: vec![Ref::Id("quatre-echeances".into())],
        }
    );
    assert_eq!(
        into.spec_read("paiement/echeances").unwrap().body,
        "Quatre échéances."
    );
    assert_eq!(
        into.spec_read("paiement/acompte").unwrap().body,
        "Un acompte de 30 %."
    );
    assert!(into.spec_read("compte/connexion").is_err());
    // What the team wrote is not undone by the ticket's older copy.
    assert_eq!(
        into.spec_read("paiement/recu").unwrap().body,
        "Par e-mail, en PDF."
    );
    let mine = into.decision(&Ref::Id("quatre-echeances".into())).unwrap();
    assert_eq!(
        (mine.title.as_str(), mine.number),
        ("Quatre échéances", None)
    );
}

#[test]
fn a_new_decision_whose_number_the_branch_took_gets_the_next_one() {
    let (into, base, work) = spaces("dt-renumber");
    let mine = work
        .decision_propose(None, proposal("Quatre échéances"))
        .unwrap();
    assert_eq!(mine.number, None);
    work.spec_write("paiement/echeances", |p| {
        p.body = "Quatre échéances.".into();
        p.decisions = vec![Ref::Number(1), mine.reference()];
    })
    .unwrap();
    // Accepted in the copy without looking at the branch, where the team accepted its own 2
    // meanwhile.
    let mine = work
        .decision_accept(&mine.reference(), "Léa", &Register::default(), None)
        .unwrap();
    assert_eq!(mine.number, Some(2));
    into.decision_propose(None, proposal("Relances par e-mail"))
        .unwrap();
    into.decision_accept(
        &Ref::Id("relances-par-e-mail".into()),
        "Léa",
        &Register::default(),
        None,
    )
    .unwrap();
    let poured = pour(&work, &base, &into).unwrap();
    assert_eq!(poured.decisions, vec![Ref::Number(3)]);
    let titles: Vec<(Option<u32>, String)> = into
        .decisions()
        .into_iter()
        .map(|d| (d.number, d.title))
        .collect();
    assert_eq!(
        titles,
        vec![
            (Some(1), "Trois échéances".to_string()),
            (Some(2), "Relances par e-mail".to_string()),
            (Some(3), "Quatre échéances".to_string()),
            (None, "Tarif famille".to_string()),
        ]
    );
    // The page names the number the decision has on the branch.
    assert_eq!(
        into.spec_read("paiement/echeances").unwrap().decisions,
        vec![Ref::Number(1), Ref::Number(3)]
    );
}

#[test]
fn a_proposal_accepted_in_the_copy_leaves_no_file_without_a_number_on_the_branch() {
    let (into, base, work) = spaces("dt-numbered");
    let id = Ref::Id("tarif-famille".into());
    let on = into.register();
    let d = work
        .decision_accept(&id, "Léa", &on, Some("DEM-7"))
        .unwrap();
    assert_eq!(d.number, Some(2));
    let poured = pour(&work, &base, &into).unwrap();
    assert_eq!(poured.decisions, vec![Ref::Number(2)]);
    let dir = into.root.join(".escouade/decisions");
    assert!(dir.join("0002-tarif-famille.md").is_file());
    assert!(!dir.join("tarif-famille.md").exists());
    let on_branch = into.decision(&id).unwrap();
    assert_eq!(
        (on_branch.number, on_branch.state, on_branch.tickets),
        (Some(2), State::Accepted, vec!["DEM-7".to_string()])
    );
    assert_eq!(into.decisions().len(), 2);
}

#[test]
fn the_protocol_of_a_ticket_asks_to_keep_the_spec_and_leave_a_decision() {
    let t = crate::model::Ticket {
        key: "DEM-7".into(),
        title: "Quatre échéances".into(),
        ..Default::default()
    };
    for lang in [crate::i18n::Lang::Fr, crate::i18n::Lang::En] {
        let p = crate::board::protocol_prompt_for(lang, &t, None, false, None);
        let rule = crate::board::spec_rule(lang);
        assert!(p.contains(&rule), "{p}");
        for tool in ["write_spec_page", "propose_decision", "search_spec"] {
            assert!(rule.contains(tool), "{rule}");
        }
        // On one line, as the protocol always is.
        assert!(!p.contains('\n'));
    }
    assert!(crate::board::spec_rule(crate::i18n::Lang::Fr).contains("change le comportement"));
    // Within Claude Code's room still, with the criteria.
    let room = crate::engine::CLAUDE.instructions_room();
    let p = crate::board::protocol_prompt_for(crate::i18n::Lang::Fr, &t, Some(4100), false, room);
    assert!(p.contains(&crate::board::spec_rule(crate::i18n::Lang::Fr)));
}

#[tokio::test]
async fn on_a_fixed_branch_a_tickets_spec_is_poured_when_the_ticket_is_validated() {
    let h = harness("docs-ticket-pour");
    let (p, repo) = h.project(false).await;
    let p = set_docs_branch(&h, &p.id, "docs");
    let branch = h.core.docs_space(&p.id, None).await.unwrap();
    branch
        .spec_write("paiement/echeances", |p| p.body = "Trois échéances.".into())
        .unwrap();
    branch.document_add("", "cahier.md", b"# Cahier").unwrap();
    let (t, _wt) = crate::tickets_tests::reviewed(&h, &p.id, "Fichier [ok]").await;
    let agent = t.agent_id.clone().unwrap();
    // Its agent works on a copy of the branch's docs, in Escouade's folder, not on the branch.
    let s = h.core.docs_space(&p.id, Some(&agent)).await.unwrap();
    assert!(!same(&s.root, &branch.root));
    assert!(s.root.starts_with(h.core.data.project_files(&p.id)));
    assert!(!s.root.starts_with(&repo));
    assert_eq!(
        s.spec_read("paiement/echeances").unwrap().body,
        "Trois échéances."
    );
    // The branch's documents are its own.
    assert_eq!(s.documents("").unwrap().len(), 1);
    s.spec_write("paiement/echeances", |p| {
        p.body = "Quatre échéances.".into()
    })
    .unwrap();
    let mut d = proposal("Quatre échéances");
    d.origin = "ticket".into();
    d.origin_ref = t.key.clone();
    s.decision_propose(None, d).unwrap();
    // Never before the validation: the branch still says what is merged.
    assert_eq!(
        branch.spec_read("paiement/echeances").unwrap().body,
        "Trois échéances."
    );
    assert!(branch.decisions().is_empty());
    // The window and the other agents see the branch.
    let window = h.core.docs_space(&p.id, None).await.unwrap();
    assert!(same(&window.root, &branch.root));
    h.core.ticket_approve(&t.id).await.unwrap();
    assert_eq!(h.ticket(&t.id).column, crate::model::Column::Done);
    // Validated: on the branch, the decision accepted; the copy gone; nothing committed there.
    assert_eq!(
        branch.spec_read("paiement/echeances").unwrap().body,
        "Quatre échéances."
    );
    let ds = branch.decisions();
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0].state, State::Accepted);
    assert!(!h.core.ticket_docs_dir(&p.id, &t.key).exists());
    let status = crate::core_tests::git(&branch.root, &["status", "--porcelain"]);
    assert!(status.contains(".escouade/"), "{status}");
    // Nothing of the docs in the ticket's merge.
    assert!(!repo.join(".escouade").exists());
}

#[tokio::test]
async fn a_ticket_deleted_takes_its_copy_of_the_docs_with_it() {
    let h = harness("docs-ticket-drop");
    let (p, _repo) = h.project(false).await;
    let p = set_docs_branch(&h, &p.id, "docs");
    let (t, _wt) = crate::tickets_tests::reviewed(&h, &p.id, "Fichier [ok]").await;
    let agent = t.agent_id.clone().unwrap();
    let s = h.core.docs_space(&p.id, Some(&agent)).await.unwrap();
    s.spec_write("a/b", |p| p.body = "x".into()).unwrap();
    assert!(h.core.ticket_docs_dir(&p.id, &t.key).is_dir());
    h.core.ticket_delete(&t.id).await.unwrap();
    assert!(!h.core.ticket_docs_dir(&p.id, &t.key).exists());
    let branch = h.core.docs_space(&p.id, None).await.unwrap();
    assert!(branch.spec_read("a/b").is_err());
}
