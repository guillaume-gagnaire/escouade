//! The docs in the repository (ESC-256): the formats read back as they were written, a decision
//! that was accepted never rewritten, the paths kept inside the project (no link, no way up), the
//! documents' text kept outside the repository, a spec found in the project, and the branch of the
//! docs: the current one, or a fixed one in a worktree Escouade holds and never merges.

use crate::core_tests::{git, harness};
use crate::docs::*;
use crate::paths::{make_dir_link, make_file_link, test_dir};
use std::path::{Path, PathBuf};

fn space(name: &str) -> (Space, PathBuf) {
    let root = test_dir(name);
    let text = test_dir(&format!("{name}-text"));
    (Space::new(&root, Some(text)), root)
}

/// One folder however it is spelt: git answers `C:/…/runneradmin/…`, the temporary folder
/// `C:…RUNNER~1…`.
pub(crate) fn same(a: &Path, b: &Path) -> bool {
    crate::paths::same_folder(&a.to_string_lossy(), &b.to_string_lossy())
}

/// Removes a link to a folder made by `make_dir_link`: a file for Unix, a folder for Windows.
fn remove_dir_link(link: &Path) {
    #[cfg(unix)]
    std::fs::remove_file(link).unwrap();
    #[cfg(windows)]
    std::fs::remove_dir(link).unwrap();
}

pub(crate) fn proposal(title: &str) -> Decision {
    Decision {
        title: title.into(),
        context: "Les clients payent en plusieurs fois.".into(),
        decision: "Trois échéances au plus.".into(),
        consequences: "Un échéancier à suivre.".into(),
        ..Default::default()
    }
}

#[test]
fn a_page_reads_back_as_it_was_written() {
    let p = Page {
        path: "paiement/recu-fiscal".into(),
        title: "Reçu fiscal « à l’envoi »: \"oui\"".into(),
        subject: "paiement".into(),
        sources: vec![
            "documents/cahier.md".into(),
            "https://exemple.fr/a?b=1".into(),
        ],
        tickets: vec!["ESC-12".into()],
        decisions: vec![
            Ref::Number(3),
            Ref::Id("tarif-famille".into()),
            Ref::Number(7),
        ],
        compared: "2026-10-28T10:00:00+02:00".into(),
        body: "# Reçu\n\nLe reçu part par e-mail.\n\n---\n\n- un : deux\n- trois".into(),
        ..Default::default()
    };
    // Without gaps, draft nor order, the header has none of them (a page reads as it always did).
    assert!(!page_write(&p).contains("gaps:") && !page_write(&p).contains("draft:"));
    let text = page_write(&p);
    assert!(text.starts_with("---\ntitle: "), "{text}");
    assert_eq!(page_read("paiement/recu-fiscal", &text), p);
    // Written again, the same bytes.
    assert_eq!(page_write(&page_read(&p.path, &text)), text);
    // Never compared: null.
    let never = Page {
        compared: String::new(),
        ..p.clone()
    };
    assert!(page_write(&never).contains("compared: null"));
    assert_eq!(page_read(&p.path, &page_write(&never)), never);
}

#[test]
fn a_page_written_by_hand_without_a_header_is_still_a_page() {
    let p = page_read("guide/demarrer", "# Démarrer\n\nOn lance.\n");
    assert_eq!(p.title, "Démarrer");
    assert_eq!(p.subject, "guide");
    assert_eq!(p.body, "# Démarrer\n\nOn lance.");
    let lone = page_read("notes", "Juste du texte.");
    assert_eq!(
        (lone.title.as_str(), lone.subject.as_str()),
        ("notes", "general")
    );
}

#[test]
fn a_page_path_is_a_slug_per_segment_and_never_leaves_the_folder() {
    assert_eq!(
        page_path("Paiement/Reçu fiscal.md").as_deref(),
        Some("paiement/recu-fiscal")
    );
    assert_eq!(page_path("a/b/c/d").as_deref(), Some("a/b/c/d"));
    for bad in [
        "",
        "  ",
        "../x",
        "a/../b",
        "/abs",
        "a\\b",
        "C:/x",
        "a//b",
        "a/b/c/d/e",
        "con",
        "a/./b",
    ] {
        assert_eq!(page_path(bad), None, "{bad}");
    }
    assert_eq!(
        document_path("Contrats/2026/cahier v2.pdf").as_deref(),
        Some("Contrats/2026/cahier v2.pdf")
    );
    for bad in ["", "../x", "a/../x", "a\\b", "C:/x", "a//b", "/etc/passwd"] {
        assert_eq!(document_path(bad), None, "{bad}");
    }
}

#[test]
fn the_spec_is_written_by_subject_and_read_searched_and_listed() {
    let (s, root) = space("docs-spec");
    let (page, added, removed) = s
        .spec_write("paiement/recu-fiscal", |p| {
            p.title = "Reçu fiscal".into();
            p.body = "Le reçu part par e-mail.\nIl est en PDF.".into();
            p.tickets = vec!["ESC-1".into()];
        })
        .unwrap();
    assert_eq!((page.subject.as_str(), added, removed), ("paiement", 2, 0));
    assert!(root
        .join(".escouade/spec/paiement/recu-fiscal.md")
        .is_file());
    // Changed: the fields left alone stay, the lines are counted.
    let (page, added, removed) = s
        .spec_write("paiement/recu-fiscal", |p| {
            p.body = "Le reçu part par e-mail.\nIl est signé.\nIl est en PDF.".into();
        })
        .unwrap();
    assert_eq!((added, removed), (1, 0));
    assert_eq!(page.tickets, vec!["ESC-1".to_string()]);
    assert_eq!(s.spec_read("Paiement/Reçu fiscal").unwrap(), page);
    s.spec_write("paiement/echeances", |p| p.body = "Trois échéances.".into())
        .unwrap();
    s.spec_write("compte/profil", |p| p.body = "# Profil\nLe nom.".into())
        .unwrap();
    let tree = s.spec_tree();
    let shape: Vec<(String, Vec<String>)> = tree
        .iter()
        .map(|t| {
            (
                t.name.clone(),
                t.pages.iter().map(|p| p.path.clone()).collect(),
            )
        })
        .collect();
    assert_eq!(
        shape,
        vec![
            ("compte".to_string(), vec!["compte/profil".to_string()]),
            (
                "paiement".to_string(),
                vec![
                    "paiement/echeances".to_string(),
                    "paiement/recu-fiscal".to_string()
                ]
            ),
        ]
    );
    assert_eq!(tree[0].pages[0].title, "Profil");
    // A search: all the words, any case, title first.
    let hits = s.spec_search("REÇU signé", 10);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, "paiement/recu-fiscal");
    assert!(hits[0].lines.iter().any(|l| l.contains("signé")));
    assert!(s.spec_search("inconnu", 10).is_empty());
    assert!(s.spec_read("paiement/rien").is_err());
    assert!(s.spec_read("../../etc/passwd").is_err());
    assert!(s.spec_write("a/../b", |_| {}).is_err());
}

#[test]
fn a_decision_reads_back_as_it_was_written() {
    let d = Decision {
        number: Some(12),
        id: "trois-echeances-au-plus".into(),
        title: "Trois échéances au plus".into(),
        date: "2026-10-28".into(),
        state: State::Accepted,
        origin: "brainstorm".into(),
        origin_ref: "Paiement en plusieurs fois".into(),
        pages: vec!["paiement/echeances".into()],
        tickets: vec!["ESC-9".into()],
        documents: vec!["cahier/paiement.pdf".into()],
        supersedes: vec![4],
        superseded_by: None,
        changes: vec![Change {
            page: "paiement/echeances".into(),
            added: 12,
            removed: 3,
        }],
        accepted_at: "2026-10-28T11:00:00+02:00".into(),
        accepted_by: "Léa".into(),
        context: "Les clients payent en plusieurs fois.\n\nSouvent.".into(),
        decision: "Trois échéances au plus.".into(),
        consequences: "Un échéancier à suivre.".into(),
    };
    let text = decision_write(&d);
    assert_eq!(decision_read(&text).unwrap(), d);
    assert_eq!(d.file_name(), "0012-trois-echeances-au-plus.md");
    // The headings may be in either language.
    let en = text
        .replace("## Contexte", "## Context")
        .replace("## Décision", "## Decision")
        .replace("## Conséquences", "## Consequences");
    assert_eq!(decision_read(&en).unwrap(), d);
    assert!(decision_read("pas d’en-tête").is_err());
    assert!(decision_read("---\ntitle: \"x\"\nnumber: \"x\"\n---\n").is_err());
    // A proposal has no number (ESC-303), and its file none either.
    let p = Decision {
        number: None,
        state: State::Proposed,
        ..d.clone()
    };
    let text = decision_write(&p);
    assert!(text.contains("number: null\n"), "{text}");
    assert_eq!(decision_read(&text).unwrap(), p);
    assert_eq!(p.file_name(), "trois-echeances-au-plus.md");
    assert_eq!(p.reference(), Ref::Id("trois-echeances-au-plus".into()));
}

#[test]
fn a_decision_is_cited_by_its_number_or_its_id() {
    assert_eq!(Ref::of("0004"), Some(Ref::Number(4)));
    assert_eq!(Ref::of("#4"), Some(Ref::Number(4)));
    assert_eq!(
        Ref::of("Tarif famille"),
        Some(Ref::Id("tarif-famille".into()))
    );
    assert_eq!(Ref::of("  "), None);
    assert_eq!(Ref::Id("12".into()).plain(), Some(Ref::Number(12)));
    assert_eq!(Ref::Number(4).to_string(), "0004");
    // As JSON: a number, or a string.
    let refs: Vec<Ref> = serde_json::from_str("[3, \"tarif-famille\"]").unwrap();
    assert_eq!(refs, vec![Ref::Number(3), Ref::Id("tarif-famille".into())]);
    assert_eq!(
        serde_json::to_string(&refs).unwrap(),
        "[3,\"tarif-famille\"]"
    );
    // An id of digits alone would read as a number.
    assert_eq!(decision_id("2026"), "d-2026");
    assert_eq!(decision_id("2 tickets"), "2-tickets");
}

#[test]
fn decisions_are_proposed_without_a_number_and_numbered_when_a_human_accepts_them() {
    let (s, root) = space("docs-decisions");
    assert_eq!(s.next_number(), 1);
    let one = s
        .decision_propose(None, proposal("Trois échéances"))
        .unwrap();
    let two = s
        .decision_propose(None, proposal("Reçu par e-mail"))
        .unwrap();
    // No number yet (ESC-303): an id, and a file without a number.
    assert_eq!((one.number, two.number), (None, None));
    assert_eq!(
        (one.id.as_str(), two.id.as_str()),
        ("trois-echeances", "recu-par-e-mail")
    );
    assert_eq!(one.state, State::Proposed);
    let dir = root.join(".escouade/decisions");
    assert!(dir.join("trois-echeances.md").is_file());
    assert_eq!(s.next_number(), 1, "a proposal takes no number");
    // The same title again: another id.
    let twin = s
        .decision_propose(None, proposal("Trois échéances"))
        .unwrap();
    assert_eq!(twin.id, "trois-echeances-2");
    // A proposal still open is revised, under its id, its date and its file.
    let mut again = proposal("Trois échéances au plus");
    again.accepted_by = "faux".into();
    let first = Ref::Id("trois-echeances".into());
    let revised = s.decision_propose(Some(&first), again).unwrap();
    assert_eq!(
        (revised.id.as_str(), revised.number, revised.date),
        ("trois-echeances", None, one.date)
    );
    assert_eq!(s.decisions().len(), 3);
    assert_eq!(s.decision(&first).unwrap().title, "Trois échéances au plus");
    assert!(s.decision(&first).unwrap().accepted_by.is_empty());
    assert!(s.decision(&Ref::Number(9)).is_err());
    assert!(s.decision(&Ref::Id("rien".into())).is_err());
    // A page cites it by its id.
    s.spec_write("paiement/echeances", |p| {
        p.body = "# Échéances".into();
        p.decisions = vec![first.clone(), Ref::Id("recu-par-e-mail".into())];
    })
    .unwrap();
    // A human accepts: it takes the next number, its file is renamed, the page cites its number.
    let accepted = s
        .decision_accept(&first, "Léa", &Register::default(), None)
        .unwrap();
    assert_eq!(accepted.number, Some(1));
    assert_eq!(accepted.state, State::Accepted);
    assert_eq!(accepted.accepted_by, "Léa");
    assert!(!accepted.accepted_at.is_empty());
    assert!(dir.join("0001-trois-echeances.md").is_file());
    assert!(!dir.join("trois-echeances.md").exists());
    assert_eq!(s.decision(&Ref::Number(1)).unwrap(), accepted);
    // Its id still finds it.
    assert_eq!(s.decision(&first).unwrap(), accepted);
    assert_eq!(
        s.spec_read("paiement/echeances").unwrap().decisions,
        vec![Ref::Number(1), Ref::Id("recu-par-e-mail".into())]
    );
    // Accepting again changes nothing.
    assert_eq!(
        s.decision_accept(&Ref::Number(1), "Autre", &Register::default(), None)
            .unwrap(),
        accepted
    );
    // Against a register further on (the branch it goes to): after it, and the ticket noted.
    let r = Ref::Id("recu-par-e-mail".into());
    let next = s
        .decision_accept(
            &r,
            "Léa",
            &Register {
                next: 7,
                taken: vec![],
            },
            Some("ESC-9"),
        )
        .unwrap();
    assert_eq!(next.number, Some(7));
    assert_eq!(next.tickets, vec!["ESC-9".to_string()]);
    assert!(dir.join("0007-recu-par-e-mail.md").is_file());
    assert_eq!(
        s.spec_read("paiement/echeances").unwrap().decisions,
        vec![Ref::Number(1), Ref::Number(7)]
    );
    assert_eq!(s.next_number(), 8);
}

#[test]
fn a_proposal_numbered_before_keeps_its_number_unless_another_has_it() {
    let (s, root) = space("docs-decisions-before");
    let dir = root.join(".escouade/decisions");
    std::fs::create_dir_all(&dir).unwrap();
    let old = |n: u32, title: &str, state: State| Decision {
        number: Some(n),
        title: title.into(),
        state,
        date: "2026-10-29".into(),
        ..Default::default()
    };
    // Written before ESC-303: no id in the header, a number in the name; two of them 4.
    let write = |name: &str, d: &Decision| {
        let text = decision_write(d).replace("id: \"\"\n", "");
        std::fs::write(dir.join(name), text).unwrap();
    };
    write(
        "0004-un-nom-d-agent.md",
        &old(4, "Un nom d'agent", State::Accepted),
    );
    write(
        "0004-un-badge-retire.md",
        &old(4, "Un badge retiré", State::Proposed),
    );
    write("0005-l-index.md", &old(5, "L'index", State::Proposed));
    let before: Vec<String> = ["0004-un-nom-d-agent.md", "0004-un-badge-retire.md"]
        .iter()
        .map(|f| std::fs::read_to_string(dir.join(f)).unwrap())
        .collect();
    // Read as they are, their id from their name.
    let ids: Vec<(Option<u32>, String)> = s
        .decisions()
        .into_iter()
        .map(|d| (d.number, d.id))
        .collect();
    assert_eq!(
        ids,
        vec![
            (Some(4), "un-badge-retire".to_string()),
            (Some(4), "un-nom-d-agent".to_string()),
            (Some(5), "l-index".to_string()),
        ]
    );
    // The 5 is its own: kept.
    let five = s
        .decision_accept(
            &Ref::Id("l-index".into()),
            "Léa",
            &Register::default(),
            None,
        )
        .unwrap();
    assert_eq!(five.number, Some(5));
    assert!(dir.join("0005-l-index.md").is_file());
    // The 4 another has: the next one, and the page it names follows.
    s.spec_write("store/badges", |p| {
        p.body = "# Badges".into();
        p.decisions = vec![Ref::Number(4)];
    })
    .unwrap();
    let mut badge = s.decision(&Ref::Id("un-badge-retire".into())).unwrap();
    badge.pages = vec!["store/badges".into()];
    s.decision_propose(Some(&Ref::Id(badge.id.clone())), badge)
        .unwrap();
    let six = s
        .decision_accept(
            &Ref::Id("un-badge-retire".into()),
            "Léa",
            &Register::default(),
            None,
        )
        .unwrap();
    assert_eq!(six.number, Some(6));
    assert!(dir.join("0006-un-badge-retire.md").is_file());
    assert!(!dir.join("0004-un-badge-retire.md").exists());
    assert_eq!(
        s.spec_read("store/badges").unwrap().decisions,
        vec![Ref::Number(6)]
    );
    // The accepted one is untouched.
    assert_eq!(
        std::fs::read_to_string(dir.join("0004-un-nom-d-agent.md")).unwrap(),
        before[0]
    );
}

#[test]
fn an_accepted_decision_cannot_be_rewritten_a_new_one_replaces_it() {
    let (s, _) = space("docs-immutable");
    s.decision_propose(None, proposal("Trois échéances"))
        .unwrap();
    let accepted = s
        .decision_accept(
            &Ref::Id("trois-echeances".into()),
            "Léa",
            &Register::default(),
            None,
        )
        .unwrap();
    let one = Ref::Number(1);
    // Neither a revision, nor a new proposal under its number.
    let refused = s
        .decision_propose(Some(&one), proposal("Quatre échéances"))
        .unwrap_err()
        .to_string();
    assert!(
        refused.contains("décision 0001") || refused.contains("Decision 0001"),
        "{refused}"
    );
    assert_eq!(s.decision(&one).unwrap(), accepted);
    // The rule itself: any change of what was decided is a rewrite.
    let mut tampered = accepted.clone();
    tampered.decision = "Autre chose".into();
    assert!(rewrites(&accepted, &tampered));
    let mut moved = accepted.clone();
    moved.title = "Autre titre".into();
    assert!(rewrites(&accepted, &moved));
    let mut back = accepted.clone();
    back.state = State::Proposed;
    assert!(rewrites(&accepted, &back));
    // The one change it allows: being replaced.
    let mut replaced = accepted.clone();
    replaced.state = State::Superseded;
    replaced.superseded_by = Some(2);
    assert!(!rewrites(&accepted, &replaced));
    // A proposal is replaced by nothing: it is revised.
    s.decision_propose(None, proposal("Reçu par e-mail"))
        .unwrap();
    s.decision_accept(
        &Ref::Id("recu-par-e-mail".into()),
        "Léa",
        &Register::default(),
        None,
    )
    .unwrap();
    s.decision_propose(None, proposal("Relances")).unwrap();
    assert!(s
        .decision_replace(9, proposal("Reçu par courrier"))
        .is_err());
    // The replacement: proposed without a number, the old one stands until a human accepts it.
    let new = s
        .decision_replace(1, proposal("Quatre échéances au plus"))
        .unwrap();
    assert_eq!(
        (new.number, new.state, new.supersedes.clone()),
        (None, State::Proposed, vec![1])
    );
    assert_eq!(s.decision(&one).unwrap().state, State::Accepted);
    let taken = s
        .decision_accept(&new.reference(), "Léa", &Register::default(), None)
        .unwrap();
    assert_eq!(taken.number, Some(3));
    let old = s.decision(&one).unwrap();
    assert_eq!((old.state, old.superseded_by), (State::Superseded, Some(3)));
    assert_eq!(old.decision, accepted.decision, "what it said is untouched");
    // Superseded: neither revised, replaced nor accepted again.
    assert!(s.decision_propose(Some(&one), proposal("x")).is_err());
    assert!(s.decision_replace(1, proposal("x")).is_err());
    assert!(s
        .decision_accept(&one, "Léa", &Register::default(), None)
        .is_err());
}

#[test]
fn the_documents_go_in_folders_their_text_stays_outside_the_repository() {
    let (s, root) = space("docs-documents");
    let a = s
        .document_add(
            "cahier des charges",
            "paiement.md",
            "# Paiement\nTrois fois.".as_bytes(),
        )
        .unwrap();
    assert_eq!(a.path, "cahier des charges/paiement.md");
    assert_eq!((a.kind.as_str(), a.readable), ("text", true));
    // A name taken is not overwritten.
    let b = s
        .document_add("cahier des charges", "paiement.md", b"autre")
        .unwrap();
    assert_eq!(b.path, "cahier des charges/paiement-2.md");
    assert_eq!(
        std::fs::read_to_string(root.join(".escouade/documents/cahier des charges/paiement.md"))
            .unwrap(),
        "# Paiement\nTrois fois."
    );
    let pdf = s.document_add("", "plan.pdf", b"%PDF-1.4").unwrap();
    assert_eq!(
        (pdf.kind.as_str(), pdf.readable, pdf.folder.as_str()),
        ("pdf", true, "")
    );
    let all = s.documents("").unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(s.documents("cahier des charges").unwrap().len(), 2);
    // The text: read, kept in the data folder, never in the repository.
    let t = s
        .document_text("cahier des charges/paiement.md", 0, 1000)
        .unwrap();
    assert_eq!(t.text.as_deref(), Some("# Paiement\nTrois fois."));
    let kept = std::fs::read_dir(s.text_dir.as_ref().unwrap())
        .unwrap()
        .count();
    assert_eq!(kept, 1);
    assert!(!root
        .join(".escouade")
        .join("documents")
        .join("paiement.txt")
        .exists());
    let walk = |dir: &Path| -> Vec<String> {
        fn go(d: &Path, out: &mut Vec<String>) {
            for e in std::fs::read_dir(d).unwrap().flatten() {
                if e.path().is_dir() {
                    go(&e.path(), out);
                } else {
                    out.push(e.file_name().to_string_lossy().to_string());
                }
            }
        }
        let mut out = Vec::new();
        go(dir, &mut out);
        out
    };
    assert!(walk(&root).iter().all(|n| !n.ends_with(".txt")));
    // A long text is read by parts.
    let long = "a".repeat(250);
    s.document_add("", "long.txt", long.as_bytes()).unwrap();
    let first = s.document_text("long.txt", 0, 100).unwrap();
    assert_eq!((first.total, first.truncated), (250, true));
    let rest = s.document_text("long.txt", 200, 100).unwrap();
    assert_eq!((rest.text.unwrap().len(), rest.truncated), (50, false));
    // A format whose text is not extracted says so.
    let none = s.document_text("plan.pdf", 0, 100).unwrap();
    assert_eq!((none.text, none.kind.as_str()), (None, "pdf"));
    assert!(s.document_text("../x.md", 0, 10).is_err());
    assert!(s.document_text("absent.md", 0, 10).is_err());
    assert!(s.document_add("../dehors", "x.md", b"x").is_err());
}

#[test]
fn a_link_in_the_docs_is_never_followed_and_nothing_leaves_the_project() {
    let (s, root) = space("docs-links");
    let outside = test_dir("docs-links-outside");
    std::fs::write(
        outside.join("secret.md"),
        "---\ntitle: \"Secret\"\n---\n\nnon",
    )
    .unwrap();
    std::fs::create_dir_all(root.join(".escouade")).unwrap();

    // `.escouade/spec` itself a link to a folder outside.
    if make_dir_link(&outside, &root.join(".escouade/spec")) {
        assert!(s.spec_tree().is_empty());
        assert!(s.spec_read("secret").is_err());
        assert!(s.spec_write("a/b", |p| p.body = "x".into()).is_err());
        assert!(s.spec_search("non", 5).is_empty());
        assert!(
            !outside.join("a").exists(),
            "nothing written through the link"
        );
        remove_dir_link(&root.join(".escouade/spec"));
    }
    // A subject folder that is a link.
    std::fs::create_dir_all(root.join(".escouade/spec")).unwrap();
    if make_dir_link(&outside, &root.join(".escouade/spec/paiement")) {
        assert!(s.spec_tree().is_empty());
        assert!(s.spec_read("paiement/secret").is_err());
        assert!(s
            .spec_write("paiement/nouveau", |p| p.body = "x".into())
            .is_err());
        assert!(!outside.join("nouveau.md").exists());
        remove_dir_link(&root.join(".escouade/spec/paiement"));
    }
    // A page that is a link to a file outside.
    std::fs::create_dir_all(root.join(".escouade/spec/a")).unwrap();
    if make_file_link(
        &outside.join("secret.md"),
        &root.join(".escouade/spec/a/lien.md"),
    ) {
        assert!(s.spec_tree().is_empty());
        assert!(s.spec_read("a/lien").is_err());
        assert!(s.spec_write("a/lien", |p| p.body = "x".into()).is_err());
        assert!(std::fs::read_to_string(outside.join("secret.md"))
            .unwrap()
            .contains("non"));
    }
    // The decisions' folder, the documents' folder.
    if make_dir_link(&outside, &root.join(".escouade/documents")) {
        assert!(s.documents("").is_err());
        assert!(s.document_add("", "x.md", b"x").is_err());
        assert!(s.document_text("secret.md", 0, 10).is_err());
        assert!(!outside.join("x.md").exists());
        remove_dir_link(&root.join(".escouade/documents"));
    }
    if make_dir_link(&outside, &root.join(".escouade/decisions")) {
        assert!(s.decisions().is_empty());
        assert!(s.decision_propose(None, proposal("x")).is_err());
        remove_dir_link(&root.join(".escouade/decisions"));
    }
    // `.escouade` itself a link.
    let real = root.join("real-escouade");
    std::fs::rename(root.join(".escouade"), &real).unwrap();
    if make_dir_link(&outside, &root.join(".escouade")) {
        assert!(s.spec_write("a/b", |p| p.body = "x".into()).is_err());
        assert!(s.spec_tree().is_empty());
        assert!(!outside.join("spec").exists());
    }
}

#[test]
fn a_spec_the_project_already_has_is_offered_as_a_candidate() {
    let (s, root) = space("docs-detect");
    assert!(s.detect().is_empty());
    std::fs::create_dir_all(root.join("docs/guide")).unwrap();
    std::fs::write(root.join("docs/SPEC.md"), "# Spec\n").unwrap();
    std::fs::write(root.join("docs/guide/a.md"), "# A\n").unwrap();
    std::fs::write(root.join("docs/image.png"), "x").unwrap();
    std::fs::write(root.join("SPEC.md"), "# Racine\n").unwrap();
    // The own folder of the docs is never a candidate.
    std::fs::create_dir_all(root.join(".escouade/spec/a")).unwrap();
    std::fs::write(root.join(".escouade/spec/a/b.md"), "x").unwrap();
    let found: Vec<(String, String, usize)> = s
        .detect()
        .into_iter()
        .map(|c| (c.path, c.kind, c.files))
        .collect();
    assert_eq!(
        found,
        vec![
            ("SPEC.md".to_string(), "file".to_string(), 1),
            ("docs/SPEC.md".to_string(), "file".to_string(), 1),
            ("docs".to_string(), "folder".to_string(), 2),
        ]
    );
    // A spec folder.
    let (s2, root2) = space("docs-detect-folder");
    std::fs::create_dir_all(root2.join("specs")).unwrap();
    std::fs::write(root2.join("specs/paiement.md"), "# P\n").unwrap();
    let found = s2.detect();
    assert_eq!(
        (found[0].path.as_str(), found[0].kind.as_str()),
        ("specs", "folder")
    );
    // A link is not a candidate.
    let (s3, root3) = space("docs-detect-link");
    let outside = test_dir("docs-detect-link-outside");
    std::fs::write(outside.join("a.md"), "x").unwrap();
    if make_dir_link(&outside, &root3.join("docs")) {
        assert!(s3.detect().is_empty());
    }
}

pub(crate) fn set_docs_branch(
    h: &crate::core_tests::Harness,
    id: &str,
    branch: &str,
) -> crate::model::Project {
    let mut p = h.core.project(id).unwrap();
    p.docs_branch = branch.into();
    h.core
        .update_project(p)
        .map(|_| h.core.project(id).unwrap())
        .unwrap()
}

#[tokio::test]
async fn with_the_current_branch_the_docs_are_where_the_caller_works() {
    let h = harness("docs-current");
    let (p, repo) = h.project(false).await;
    assert!(p.docs_branch.is_empty());
    // The window: the project's folder.
    let s = h.core.docs_space(&p.id, None).await.unwrap();
    assert!(same(&s.root, &repo), "{:?} {:?}", s.root, repo);
    // An agent in a folder of its own: its folder, so a ticket's spec follows its branch.
    let wt = h.dir.join("wt");
    std::fs::create_dir_all(&wt).unwrap();
    let meta = crate::model::AgentMeta {
        id: "a-alice".into(),
        project_id: p.id.clone(),
        name: "Alice".into(),
        cwd: wt.to_string_lossy().to_string(),
        ..Default::default()
    };
    let rt = crate::agent::AgentRt::new(meta.clone(), &h.core.data.conversations());
    h.core.agents.write().insert(
        meta.id.clone(),
        std::sync::Arc::new(parking_lot::Mutex::new(rt)),
    );
    let s = h.core.docs_space(&p.id, Some(&meta.id)).await.unwrap();
    assert!(same(&s.root, &wt), "{:?} {:?}", s.root, wt);
    s.spec_write("a/b", |p| p.body = "x".into()).unwrap();
    assert!(wt.join(".escouade/spec/a/b.md").is_file());
    assert!(!repo.join(".escouade").exists());
    // An agent of another project does not take the folder of this one's agent.
    let other = h.core.docs_space(&p.id, Some("inconnu")).await.unwrap();
    assert!(same(&other.root, &repo), "{:?} {:?}", other.root, repo);
}

#[tokio::test]
async fn a_fixed_branch_has_a_worktree_of_its_own_that_escouade_never_merges() {
    let h = harness("docs-fixed");
    let (p, repo) = h.project(false).await;
    let before = git(&repo, &["rev-parse", "HEAD"]);
    let branch_before = git(&repo, &["rev-parse", "--abbrev-ref", "HEAD"]);
    let p = set_docs_branch(&h, &p.id, "docs");
    assert_eq!(p.docs_branch, "docs");
    let s = h.core.docs_space(&p.id, None).await.unwrap();
    // A worktree of its own, in Escouade's data, not in the project.
    assert!(!same(&s.root, &repo), "{:?} {:?}", s.root, repo);
    assert!(
        s.root.starts_with(h.core.data.project_files(&p.id)),
        "{:?}",
        s.root
    );
    assert_eq!(git(&s.root, &["rev-parse", "--abbrev-ref", "HEAD"]), "docs");
    s.spec_write("paiement/echeances", |p| p.body = "Trois échéances.".into())
        .unwrap();
    s.decision_propose(None, proposal("Trois échéances"))
        .unwrap();
    s.document_add("", "cahier.md", b"# Cahier").unwrap();
    // Where the docs live: the worktree. Not in the project's checkout.
    assert!(s
        .root
        .join(".escouade/spec/paiement/echeances.md")
        .is_file());
    assert!(!repo.join(".escouade").exists());
    // The same worktree the next time.
    let again = h.core.docs_space(&p.id, None).await.unwrap();
    assert!(same(&again.root, &s.root), "{:?} {:?}", again.root, s.root);
    assert_eq!(
        again.spec_read("paiement/echeances").unwrap().body,
        "Trois échéances."
    );
    // Nothing merged, nothing committed: the project's branch is where it was, the docs branch
    // has no commit of its own, the files wait to be shared.
    assert_eq!(git(&repo, &["rev-parse", "HEAD"]), before);
    assert_eq!(
        git(&repo, &["rev-parse", "--abbrev-ref", "HEAD"]),
        branch_before
    );
    assert_eq!(git(&repo, &["rev-parse", "docs"]), before);
    assert_eq!(
        git(&repo, &["log", "--oneline", "--all"]).lines().count(),
        1
    );
    let status = git(&s.root, &["status", "--porcelain"]);
    assert!(status.contains(".escouade/"), "{status}");
    assert!(
        git(&repo, &["status", "--porcelain"]).is_empty()
            || !git(&repo, &["status", "--porcelain"]).contains(".escouade")
    );
    // Back to the current branch: the docs are where the caller works again.
    let p = set_docs_branch(&h, &p.id, "");
    let back = h.core.docs_space(&p.id, None).await.unwrap();
    assert!(same(&back.root, &repo), "{:?} {:?}", back.root, repo);
}

#[tokio::test]
async fn a_fixed_branch_already_checked_out_is_used_where_it_is() {
    let h = harness("docs-fixed-here");
    let (p, repo) = h.project(false).await;
    let current = git(&repo, &["rev-parse", "--abbrev-ref", "HEAD"]);
    // The branch of the docs is the one checked out (`main` itself).
    let p = set_docs_branch(&h, &p.id, &current);
    let s = h.core.docs_space(&p.id, None).await.unwrap();
    assert!(same(&s.root, &repo), "{:?} {:?}", s.root, repo);
    // A branch that exists on the remote only is made from it.
    git(&repo, &["branch", "wiki"]);
    let p = set_docs_branch(&h, &p.id, "wiki");
    let s = h.core.docs_space(&p.id, None).await.unwrap();
    assert!(!same(&s.root, &repo), "{:?} {:?}", s.root, repo);
    assert_eq!(git(&s.root, &["rev-parse", "--abbrev-ref", "HEAD"]), "wiki");
    // Its folder gone: made again.
    let gone = s.root.clone();
    git(
        &repo,
        &["worktree", "remove", "--force", &gone.to_string_lossy()],
    );
    let s = h.core.docs_space(&p.id, None).await.unwrap();
    assert!(s.root.is_dir());
}

#[tokio::test]
async fn a_docs_branch_that_is_not_a_branch_name_is_refused() {
    let h = harness("docs-bad-branch");
    let (p, _) = h.project(false).await;
    for bad in ["-x", "a b", "a..b", "x.lock", "a:b", "/x", "x/", "@{u}"] {
        let mut q = h.core.project(&p.id).unwrap();
        q.docs_branch = bad.into();
        assert!(h.core.update_project(q).is_err(), "{bad}");
    }
    assert!(h.core.project(&p.id).unwrap().docs_branch.is_empty());
}

#[tokio::test]
async fn a_human_accepts_a_decision_by_the_core_with_their_name() {
    let h = harness("docs-accept");
    let (p, _) = h.project(false).await;
    {
        let mut projects = h.core.projects.write();
        projects
            .iter_mut()
            .find(|x| x.id == p.id)
            .unwrap()
            .user_name = "Léa".into();
    }
    let s = h.core.docs_space(&p.id, None).await.unwrap();
    s.decision_propose(None, proposal("Trois échéances"))
        .unwrap();
    let r = Ref::Id("trois-echeances".into());
    let d = h.core.docs_accept_decision(&p.id, &r).await.unwrap();
    assert_eq!((d.state, d.accepted_by.as_str()), (State::Accepted, "Léa"));
    assert_eq!(d.number, Some(1));
    assert!(h
        .core
        .docs_accept_decision(&p.id, &Ref::Number(9))
        .await
        .is_err());
}

#[tokio::test]
async fn a_large_file_asks_for_a_confirmation_before_it_goes_in_the_repository() {
    let h = harness("docs-large");
    let (p, repo) = h.project(false).await;
    let src = h.dir.join("gros.bin");
    let f = std::fs::File::create(&src).unwrap();
    f.set_len(LARGE_FILE + 1).unwrap();
    drop(f);
    let refused = h
        .core
        .docs_drop(&p.id, Some("videos"), &src, false)
        .await
        .unwrap_err()
        .to_string();
    assert!(
        refused.contains("50") || refused.contains("Mo") || refused.contains("MB"),
        "{refused}"
    );
    assert!(!repo.join(".escouade/documents/videos").exists());
    let ok = h
        .core
        .docs_drop(&p.id, Some("videos"), &src, true)
        .await
        .unwrap();
    assert_eq!(ok.file.path, "videos/gros.bin");
    // A small one needs nothing, and a missing one is an error.
    let small = h.dir.join("petit.md");
    std::fs::write(&small, "# Petit").unwrap();
    assert!(h.core.docs_drop(&p.id, None, &small, false).await.is_ok());
    assert!(h
        .core
        .docs_drop(&p.id, None, Path::new("/nulle/part.md"), false)
        .await
        .is_err());
}

#[tokio::test]
async fn the_decision_a_tickets_agent_left_is_accepted_when_the_ticket_is_validated() {
    let h = harness("docs-ticket-accept");
    let (p, repo) = h.project(false).await;
    {
        let mut projects = h.core.projects.write();
        projects
            .iter_mut()
            .find(|x| x.id == p.id)
            .unwrap()
            .user_name = "Léa".into();
    }
    let (t, wt) = crate::tickets_tests::reviewed(&h, &p.id, "Fichier [ok]").await;
    // Where its agent writes the docs: its own worktree (the branch of the docs is the current one).
    let agent = t.agent_id.clone().unwrap();
    let s = h.core.docs_space(&p.id, Some(&agent)).await.unwrap();
    assert!(same(&s.root, &wt), "{:?} {:?}", s.root, wt);
    let mut own = proposal("Trois échéances");
    own.origin = "ticket".into();
    own.origin_ref = t.key.clone();
    s.decision_propose(None, own).unwrap();
    // Another ticket's, and one of a brainstorm, wait for a human.
    let mut other = proposal("Relances par e-mail");
    other.origin = "ticket".into();
    other.origin_ref = "DEM-99".into();
    s.decision_propose(None, other).unwrap();
    s.decision_propose(None, proposal("Tarif famille")).unwrap();
    h.core.ticket_approve(&t.id).await.unwrap();
    assert_eq!(h.ticket(&t.id).column, crate::model::Column::Done);
    // Merged with the ticket, accepted by the one who validated it.
    let merged = h.core.docs_space(&p.id, None).await.unwrap();
    assert!(same(&merged.root, &repo));
    let states: Vec<(Option<u32>, String, State, String)> = merged
        .decisions()
        .into_iter()
        .map(|d| (d.number, d.id, d.state, d.accepted_by))
        .collect();
    assert_eq!(
        states,
        vec![
            (
                Some(1),
                "trois-echeances".to_string(),
                State::Accepted,
                "Léa".to_string()
            ),
            (
                None,
                "relances-par-e-mail".to_string(),
                State::Proposed,
                String::new()
            ),
            (
                None,
                "tarif-famille".to_string(),
                State::Proposed,
                String::new()
            ),
        ]
    );
    // In the commit of the ticket, not left to share.
    assert_eq!(git(&repo, &["status", "--porcelain"]), "");
}

/// ESC-303: two tickets in two worktrees propose a decision each, at the same time; validated one
/// after the other, they get two numbers, after main's register, and what cites them follows: the
/// pages, the decision one of them replaces, the ticket that implements the other.
#[tokio::test]
async fn two_tickets_in_parallel_get_two_numbers_for_their_decisions_when_validated() {
    let h = harness("docs-two-tickets");
    let (p, repo) = h.project(false).await;
    {
        let mut projects = h.core.projects.write();
        projects
            .iter_mut()
            .find(|x| x.id == p.id)
            .unwrap()
            .user_name = "Léa".into();
    }
    // On main, committed: decision 1 accepted, and the page that follows it.
    let main = h.core.docs_space(&p.id, None).await.unwrap();
    main.decision_propose(None, proposal("Trois échéances"))
        .unwrap();
    main.decision_accept(
        &Ref::Id("trois-echeances".into()),
        "Léa",
        &Register::default(),
        None,
    )
    .unwrap();
    main.spec_write("paiement/echeances", |x| {
        x.body = "Trois échéances.".into();
        x.decisions = vec![Ref::Number(1)];
    })
    .unwrap();
    let one =
        std::fs::read_to_string(repo.join(".escouade/decisions/0001-trois-echeances.md")).unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "docs"]);
    // Two tickets at work at once, each in its worktree.
    let (a, wa) = crate::tickets_tests::reviewed(&h, &p.id, "Un [ok]").await;
    let (b, wb) = crate::tickets_tests::reviewed(&h, &p.id, "Deux [ok]").await;
    let sa = h
        .core
        .docs_space(&p.id, a.agent_id.as_deref())
        .await
        .unwrap();
    let sb = h
        .core
        .docs_space(&p.id, b.agent_id.as_deref())
        .await
        .unwrap();
    assert!(same(&sa.root, &wa) && same(&sb.root, &wb));
    // A replaces decision 1, B proposes a new one: neither has a number, each cites its id.
    let mut ra = proposal("Quatre échéances");
    ra.origin = "ticket".into();
    ra.origin_ref = a.key.clone();
    let ra = sa.decision_replace(1, ra).unwrap();
    sa.spec_write("paiement/echeances", |x| {
        x.body = "Quatre échéances.".into();
        x.decisions = vec![Ref::Number(1), ra.reference()];
    })
    .unwrap();
    let mut rb = proposal("Tarif famille");
    rb.origin = "ticket".into();
    rb.origin_ref = b.key.clone();
    let rb = sb.decision_propose(None, rb).unwrap();
    sb.spec_write("adherent/tarif", |x| {
        x.body = "-20 % dès le troisième membre.".into();
        x.decisions = vec![rb.reference()];
    })
    .unwrap();
    assert_eq!((ra.number, rb.number), (None, None));
    assert!(wa.join(".escouade/decisions/quatre-echeances.md").is_file());
    assert!(wb.join(".escouade/decisions/tarif-famille.md").is_file());
    // A ticket to come implements B's proposal, by its id.
    let mut board = h.core.project(&p.id).unwrap().board;
    board.autopilot = false;
    h.core.board_set(&p.id, board).unwrap();
    let c = h
        .core
        .ticket_create(
            &p.id,
            crate::tickets::TicketDraft {
                title: "Appliquer le tarif".into(),
                decisions: Some(vec![rb.reference()]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    // Validated one after the other.
    h.core.ticket_approve(&a.id).await.unwrap();
    assert_eq!(
        h.ticket(&a.id).column,
        crate::model::Column::Done,
        "{:?}",
        h.ticket(&a.id).blocked
    );
    h.core.ticket_approve(&b.id).await.unwrap();
    assert_eq!(
        h.ticket(&b.id).column,
        crate::model::Column::Done,
        "{:?}",
        h.ticket(&b.id).blocked
    );
    // On main: 1, 2 and 3, no two alike, no file without a number.
    let mut files: Vec<String> = git(
        &repo,
        &[
            "ls-tree",
            "--name-only",
            "main",
            "--",
            ".escouade/decisions/",
        ],
    )
    .lines()
    .map(|l| l.rsplit('/').next().unwrap().to_string())
    .collect();
    files.sort();
    assert_eq!(
        files,
        vec![
            "0001-trois-echeances.md",
            "0002-quatre-echeances.md",
            "0003-tarif-famille.md",
        ]
    );
    let merged = h.core.docs_space(&p.id, None).await.unwrap();
    type Row = (Option<u32>, String, State, Option<u32>, Vec<String>);
    let all: Vec<Row> = merged
        .decisions()
        .into_iter()
        .map(|d| (d.number, d.id, d.state, d.superseded_by, d.tickets))
        .collect();
    assert_eq!(
        all,
        vec![
            (
                Some(1),
                "trois-echeances".to_string(),
                State::Superseded,
                Some(2),
                vec![]
            ),
            (
                Some(2),
                "quatre-echeances".to_string(),
                State::Accepted,
                None,
                vec![a.key.clone()]
            ),
            (
                Some(3),
                "tarif-famille".to_string(),
                State::Accepted,
                None,
                vec![b.key.clone()]
            ),
        ]
    );
    assert_eq!(
        merged.decision(&Ref::Number(2)).unwrap().supersedes,
        vec![1]
    );
    // What 1 decided is untouched, but for who replaced it.
    let now =
        std::fs::read_to_string(repo.join(".escouade/decisions/0001-trois-echeances.md")).unwrap();
    assert_eq!(
        now.replace("state: \"superseded\"", "state: \"accepted\"")
            .replace("superseded_by: 2", "superseded_by: null"),
        one
    );
    // The pages cite their numbers.
    assert_eq!(
        merged.spec_read("paiement/echeances").unwrap().decisions,
        vec![Ref::Number(1), Ref::Number(2)]
    );
    assert_eq!(
        merged.spec_read("adherent/tarif").unwrap().decisions,
        vec![Ref::Number(3)]
    );
    // The ticket that implements B's.
    assert_eq!(h.ticket(&c.id).decisions, vec![Ref::Number(3)]);
    // In the commits of the tickets, not left to share.
    assert_eq!(
        git(&repo, &["status", "--porcelain", "--untracked-files=no"]),
        ""
    );
}

/// What happened to ESC-302 (32c0a0f1): two tickets had each proposed a decision 4, as Escouade
/// numbered them before ESC-303; the second validation accepted the other's 4 and committed its own
/// as a second `0004-…`, still proposed. A validation now finds its decisions by their own id, and
/// one whose number main has under another takes the next.
#[tokio::test]
async fn a_validation_accepts_its_own_decision_never_another_of_the_same_number() {
    let h = harness("docs-same-number");
    let (p, repo) = h.project(false).await;
    {
        let mut projects = h.core.projects.write();
        projects
            .iter_mut()
            .find(|x| x.id == p.id)
            .unwrap()
            .user_name = "Léa".into();
    }
    // On main: 1 to 3, accepted and committed.
    let main = h.core.docs_space(&p.id, None).await.unwrap();
    for title in ["Un", "Deux", "Trois"] {
        let d = main.decision_propose(None, proposal(title)).unwrap();
        main.decision_accept(&d.reference(), "Léa", &Register::default(), None)
            .unwrap();
    }
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "docs"]);
    let dir = |root: &Path| root.join(".escouade/decisions");
    let before: Vec<(String, String)> = ["0001-un.md", "0002-deux.md", "0003-trois.md"]
        .iter()
        .map(|f| {
            (
                f.to_string(),
                std::fs::read_to_string(dir(&repo).join(f)).unwrap(),
            )
        })
        .collect();
    let (a, wa) = crate::tickets_tests::reviewed(&h, &p.id, "Un [ok]").await;
    let (b, wb) = crate::tickets_tests::reviewed(&h, &p.id, "Deux [ok]").await;
    // Each proposes a 4, as before ESC-303 (a number in its header and its name, no id).
    let legacy = |root: &Path, key: &str, title: &str, page: &str| {
        let d = Decision {
            number: Some(4),
            title: title.into(),
            date: "2026-10-30".into(),
            state: State::Proposed,
            origin: "ticket".into(),
            origin_ref: key.into(),
            pages: vec![page.into()],
            ..Default::default()
        };
        let name = format!("0004-{}.md", decision_id(title));
        let text = decision_write(&d).replace("id: \"\"\n", "");
        std::fs::write(dir(root).join(name), text).unwrap();
    };
    legacy(&wa, &a.key, "Un nom d'agent", "agents/noms");
    legacy(&wb, &b.key, "Un badge retiré", "store/badges");
    let sb = h
        .core
        .docs_space(&p.id, b.agent_id.as_deref())
        .await
        .unwrap();
    sb.spec_write("store/badges", |x| {
        x.body = "Un badge retiré s'annonce.".into();
        x.decisions = vec![Ref::Number(4)];
    })
    .unwrap();
    // A validated first: its 4 is its own.
    h.core.ticket_approve(&a.id).await.unwrap();
    assert_eq!(h.ticket(&a.id).column, crate::model::Column::Done);
    let a_file = std::fs::read_to_string(dir(&repo).join("0004-un-nom-d-agent.md")).unwrap();
    assert!(a_file.contains("state: \"accepted\""), "{a_file}");
    // B takes main in (as the chef did): its worktree has A's 4 and its own.
    git(&wb, &["add", "-A"]);
    git(&wb, &["commit", "-qm", "wip"]);
    git(&wb, &["merge", "-q", "--no-edit", "main"]);
    assert!(dir(&wb).join("0004-un-nom-d-agent.md").is_file());
    h.core.ticket_approve(&b.id).await.unwrap();
    assert_eq!(
        h.ticket(&b.id).column,
        crate::model::Column::Done,
        "{:?}",
        h.ticket(&b.id).blocked
    );
    // Only its own accepted, as 5; A's untouched; 1 to 3 untouched; no two 4s.
    let mut files: Vec<String> = git(
        &repo,
        &[
            "ls-tree",
            "--name-only",
            "main",
            "--",
            ".escouade/decisions/",
        ],
    )
    .lines()
    .map(|l| l.rsplit('/').next().unwrap().to_string())
    .collect();
    files.sort();
    assert_eq!(
        files,
        vec![
            "0001-un.md",
            "0002-deux.md",
            "0003-trois.md",
            "0004-un-nom-d-agent.md",
            "0005-un-badge-retire.md",
        ]
    );
    for (f, text) in &before {
        assert_eq!(
            &std::fs::read_to_string(dir(&repo).join(f)).unwrap(),
            text,
            "{f}"
        );
    }
    assert_eq!(
        std::fs::read_to_string(dir(&repo).join("0004-un-nom-d-agent.md")).unwrap(),
        a_file
    );
    let merged = h.core.docs_space(&p.id, None).await.unwrap();
    let five = merged.decision(&Ref::Number(5)).unwrap();
    assert_eq!(
        (five.id.as_str(), five.state, five.origin_ref.as_str()),
        ("un-badge-retire", State::Accepted, b.key.as_str())
    );
    assert_eq!(
        merged.spec_read("store/badges").unwrap().decisions,
        vec![Ref::Number(5)]
    );
}
#[test]
fn a_decision_from_the_window_needs_only_its_fields() {
    // « Proposer une décision » sends a title and its parts, nothing of its number, state or links.
    let d: Decision = serde_json::from_value(serde_json::json!({
        "title": "Tarif famille",
        "context": "Les familles paient plein tarif.",
        "decision": "-20 % dès le troisième membre.",
        "consequences": "",
    }))
    .unwrap();
    assert_eq!(
        (d.title.as_str(), d.state, d.number, d.supersedes.len()),
        ("Tarif famille", State::Proposed, None, 0)
    );
}
