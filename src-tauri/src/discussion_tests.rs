//! The discussions of the Docs tab (ESC-261): their instructions and first message, what they
//! propose kept page by page and decision by decision, and the tools refused or let through.

use crate::discussion::{
    allowed_tool, first_message, holds, instructions, refused_tool, Discussion, Kind, Start,
};
use crate::docs::{Decision, Page};
use crate::i18n::Lang;
use crate::model::EngineId;

#[test]
fn each_role_is_told_it_never_changes_the_code_and_writes_the_docs_by_the_tools() {
    for kind in [Kind::Discussion, Kind::Brainstorm] {
        for lang in [Lang::Fr, Lang::En] {
            let text = instructions(lang, kind, "Portail");
            assert!(text.contains("Portail"), "{text}");
            assert!(text.contains("write_spec_page") && text.contains("propose_decision"));
            assert!(text.contains("search_documents"));
            // The tools by their names in Escouade's server.
            assert!(text.contains("read_spec_page") && !text.contains("get_spec_page"));
        }
        assert!(instructions(Lang::Fr, kind, "P").contains("Tu ne modifies jamais le code"));
        assert!(instructions(Lang::En, kind, "P").contains("You never change the code"));
    }
    let writer = instructions(Lang::Fr, Kind::Discussion, "P");
    let product = instructions(Lang::Fr, Kind::Brainstorm, "P");
    assert!(writer.contains("rédacteur de spec"));
    assert!(product.contains("BMAD") && product.contains("RGPD") && product.contains("tickets"));
    assert!(!writer.contains("BMAD"));
}

#[test]
fn the_first_message_is_the_subject_with_the_documents_joined() {
    let mut s = Start {
        kind: Kind::Discussion,
        subject: " Que dit la spec sur le paiement ? ".into(),
        ..Default::default()
    };
    assert_eq!(
        first_message(Lang::Fr, &s),
        "Que dit la spec sur le paiement ?"
    );
    s.kind = Kind::Brainstorm;
    s.subject = "Tarif famille".into();
    s.documents = vec!["Comptes rendus/cr-atelier.pdf".into()];
    let text = first_message(Lang::Fr, &s);
    assert!(
        text.starts_with("Sujet du brainstorm : Tarif famille"),
        "{text}"
    );
    assert!(text.contains("- Comptes rendus/cr-atelier.pdf"));
    // A message of its own (a gesture of the spec) goes in place of the subject.
    s.message = Some("Mets la page à jour.".into());
    assert!(first_message(Lang::En, &s).starts_with("Mets la page à jour."));
}

fn page(path: &str, body: &str) -> Page {
    Page {
        path: path.into(),
        title: "T".into(),
        body: body.into(),
        ..Default::default()
    }
}

#[test]
fn a_page_proposed_again_keeps_the_text_it_had_before_the_first_change() {
    let mut d = Discussion::default();
    d.propose_page(
        Some("Aucune notification.".into()),
        page("notif/n", "Une relance."),
    );
    let c = d.propose_page(
        Some("Une relance.".into()),
        page("notif/n", "Une relance.\nDeux."),
    );
    assert_eq!(c.before.as_deref(), Some("Aucune notification."));
    assert_eq!((c.added, c.removed), (2, 1));
    d.propose_page(None, page("relances/r", "Trois relances."));
    assert_eq!(d.pages.len(), 2);
    assert!(d.page("relances/r").unwrap().before.is_none());
}

#[test]
fn a_decision_of_the_same_title_is_revised_and_carries_the_discussions_origin() {
    let mut d = Discussion {
        kind: Kind::Brainstorm,
        subject: "Relances".into(),
        ..Default::default()
    };
    let one = |t: &str, what: &str| Decision {
        title: t.into(),
        decision: what.into(),
        ..Default::default()
    };
    assert_eq!(d.propose_decision(one("E-mail seulement", "a")), 1);
    assert_eq!(d.propose_decision(one("Pas de SMS", "b")), 2);
    assert_eq!(d.propose_decision(one(" e-mail SEULEMENT", "c")), 1);
    assert_eq!(d.decisions.len(), 2);
    assert_eq!(d.decisions[0].decision, "c");
    assert_eq!(
        (
            d.decisions[0].origin.as_str(),
            d.decisions[0].origin_ref.as_str()
        ),
        ("brainstorm", "Relances")
    );
}

#[test]
fn writes_and_commands_are_refused_and_escouades_tools_for_the_docs_let_through() {
    // Codex's « Permissions » asks for more than its sandbox gives.
    for t in [
        "Edit",
        "Write",
        "MultiEdit",
        "NotebookEdit",
        "Bash",
        "Permissions",
    ] {
        assert!(refused_tool(t) && !allowed_tool(t), "{t}");
    }
    // What reads, and what writes the docs (as proposals).
    for t in [
        "mcp__escouade__write_spec_page",
        "mcp__escouade__propose_decision",
        "mcp__escouade__replace_decision",
        "mcp__escouade__read_spec_page",
        "mcp__escouade__get_spec_tree",
        "mcp__escouade__search_documents",
        "mcp__escouade__read_document",
        "mcp__escouade__list_decisions",
        "mcp__escouade__search_memory",
        "mcp__escouade__index_search",
        "mcp__escouade__get_board",
    ] {
        assert!(allowed_tool(t) && !refused_tool(t), "{t}");
    }
    // Escouade's other tools act on the board, the agents or the shared files: the user's to
    // allow, as the rest.
    for t in [
        "mcp__escouade__create_ticket",
        "mcp__escouade__start_ticket",
        "mcp__escouade__approve_ticket",
        "mcp__escouade__send_message",
        "mcp__escouade__stop_agent",
        "mcp__escouade__share_docs",
        "mcp__escouade__add_document",
        "mcp__escouade__write_memory",
        "mcp__escouade__forget_memory",
        "WebFetch",
        "Read",
        "mcp__other__read_x",
    ] {
        assert!(!allowed_tool(t) && !refused_tool(t), "{t}");
    }
}

#[test]
fn a_discussion_runs_only_on_a_harness_that_keeps_it_from_writing() {
    for e in [
        EngineId::Claude,
        EngineId::Codex,
        EngineId::Opencode,
        EngineId::Copilot,
    ] {
        assert!(holds(e), "{e:?}");
    }
    // They write the files of their folder without going through Escouade.
    assert!(!holds(EngineId::Cursor));
    assert!(!holds(EngineId::Vibe));
}

#[test]
fn a_discussion_reads_back_as_it_was_saved() {
    let mut d = Discussion {
        kind: Kind::Brainstorm,
        subject: "S".into(),
        ..Default::default()
    };
    d.propose_page(None, page("a/b", "x"));
    let v = serde_json::to_value(&d).unwrap();
    assert_eq!(v["kind"], "brainstorm");
    assert!(v.get("poured").is_none());
    assert_eq!(serde_json::from_value::<Discussion>(v).unwrap(), d);
    // An older state without the field: no discussion.
    let meta: crate::model::AgentMeta = serde_json::from_str(r#"{"id":"a"}"#).unwrap();
    assert!(meta.discussion.is_none());
}

/// ESC-289: a discussion is never in the terminal view, even with « Vue Claude Code » on: its first
/// message goes, and its writes stay refused by Escouade in its conversation.
#[tokio::test]
async fn a_discussion_never_goes_to_the_terminal_view() {
    let h = crate::core_tests::harness("discussion-no-terminal");
    let (p, _) = h.project(false).await;
    h.core.settings.write().terminal_view = true;
    let view = h
        .core
        .discussion_start(
            &p.id,
            Start {
                kind: Kind::Discussion,
                subject: "Première spec depuis le code".into(),
                message: Some("Lis le code.".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let meta = h.agent(&view.meta.id);
    assert!(meta.discussion.is_some());
    assert!(!meta.terminal && !meta.attached, "{meta:?}");
    // Its first message went to its harness, in the conversation.
    h.wait("its first message sent", |h| {
        h.items(&view.meta.id)
            .iter()
            .any(|i| i["text"].as_str() == Some("Lis le code."))
    })
    .await;
    // An ordinary agent still goes to the terminal view.
    let other = h.core.create_agent(&p.id, None).await.unwrap();
    assert!(h.agent(&other.meta.id).terminal);
    // On the docs branch too: in its worktree, its first message sent.
    crate::docs_tests::set_docs_branch(&h, &p.id, "docs");
    let on_branch = h
        .core
        .discussion_start(
            &p.id,
            Start {
                kind: Kind::Discussion,
                subject: "Première spec, branche des docs".into(),
                message: Some("Lis le code, sur la branche des docs.".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let meta = h.agent(&on_branch.meta.id);
    assert!(!meta.terminal && !meta.attached);
    assert_ne!(meta.cwd, p.path);
    h.wait("its first message sent on the docs branch", |h| {
        h.items(&on_branch.meta.id)
            .iter()
            .any(|i| i["text"].as_str() == Some("Lis le code, sur la branche des docs."))
    })
    .await;
}

/// ESC-289: a first message refused once its agent is made (here, its harness cannot be started)
/// leaves no agent behind: it is removed, and the refusal goes back to the window, which keeps what
/// was chosen to start again.
#[tokio::test]
async fn a_discussion_whose_first_message_is_refused_leaves_no_agent() {
    let h = crate::core_tests::harness("discussion-refused");
    let (p, _) = h.project(false).await;
    h.core.settings.write().claude_path = h
        .dir
        .join("nowhere")
        .join("claude.exe")
        .to_string_lossy()
        .into_owned();
    let refused = h
        .core
        .discussion_start(
            &p.id,
            Start {
                kind: Kind::Discussion,
                subject: "Première spec depuis le code".into(),
                message: Some("Lis le code.".into()),
                ..Default::default()
            },
        )
        .await;
    assert!(refused.is_err());
    // Made, then removed.
    let events = h.events.lock().clone();
    let removed: Vec<_> = events
        .iter()
        .filter(|e| e["type"] == "agentRemoved")
        .collect();
    assert_eq!(removed.len(), 1, "{events:?}");
    assert!(h.core.agent(removed[0]["id"].as_str().unwrap()).is_err());
}
