//! Importing a spec the project has (ESC-258): cut by its titles, nothing written before the
//! user agrees, nothing overwritten, nothing read through a link; and a page's gaps with the code,
//! its draft and its place read back as written.

use crate::docs::*;
use crate::docsimport::cut;
use crate::paths::{make_file_link, test_dir};

fn space(name: &str) -> (Space, std::path::PathBuf) {
    let root = test_dir(name);
    (Space::new(&root, None), root)
}

const SPEC: &str = "# Portail des adhérents\n\nLe portail sert aux adhérents.\n\n## Adhésion en ligne\n\nIntro de l’adhésion.\n\n### Formulaire\n\nLe formulaire a 3 étapes.\n\n### Pièces justificatives\n\nUn justificatif de domicile.\n\n```md\n## pas un titre\n```\n\n## Paiement\n\nCarte ou prélèvement.\n";

#[test]
fn a_spec_file_is_cut_by_its_titles_subject_by_subject() {
    let d = cut(SPEC, "docs/SPEC.md", None, "Introduction");
    let got: Vec<(&str, &str, u32)> = d
        .iter()
        .map(|d| (d.subject.as_str(), d.title.as_str(), d.lines))
        .collect();
    assert_eq!(
        got,
        vec![
            // The lone top title names the introduction, it is not its text.
            ("Introduction", "Portail des adhérents", 1),
            // A subject's text before its first page is a page of its own name.
            ("Adhésion en ligne", "Adhésion en ligne", 1),
            ("Adhésion en ligne", "Formulaire", 1),
            // A title in a code block is code.
            ("Adhésion en ligne", "Pièces justificatives", 4),
            // A subject without titles under it is one page.
            ("Paiement", "Paiement", 1),
        ]
    );
    assert!(d.iter().all(|d| d.source == "docs/SPEC.md"));
    assert_eq!(d[2].body, "Le formulaire a 3 étapes.");
}

#[test]
fn a_spec_without_titles_is_one_page() {
    let d = cut("Juste du texte.\n", "SPEC.md", None, "Introduction");
    assert_eq!(d.len(), 1);
    assert_eq!(
        (d[0].subject.as_str(), d[0].title.as_str()),
        ("Introduction", "Introduction")
    );
    // Front matter is left out.
    let d = cut(
        "---\ntitle: x\n---\n\n## A\n\na\n\n## B\n\nb\n",
        "SPEC.md",
        None,
        "Introduction",
    );
    assert_eq!(
        d.iter().map(|d| d.title.as_str()).collect::<Vec<_>>(),
        vec!["A", "B"]
    );
}

#[test]
fn the_import_writes_nothing_until_asked_then_keeps_the_order_and_cites_its_source() {
    let (s, root) = space("import-file");
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(root.join("docs/SPEC.md"), SPEC).unwrap();
    let plan = s.import_plan("docs/SPEC.md").unwrap();
    assert_eq!(plan.len(), 5);
    assert_eq!(plan[2].path, "adhesion-en-ligne/formulaire");
    // Nothing written by the plan.
    assert!(!root.join(".escouade").exists());
    // Only the pages agreed to.
    let only = vec![plan[2].path.clone(), plan[4].path.clone()];
    let done = s.import("docs/SPEC.md", Some(&only)).unwrap();
    assert_eq!(done.written, only);
    let page = s.spec_read("adhesion-en-ligne/formulaire").unwrap();
    assert_eq!(page.subject, "Adhésion en ligne");
    assert_eq!(page.sources, vec!["docs/SPEC.md".to_string()]);
    assert_eq!(page.body, "Le formulaire a 3 étapes.");
    // Then everything: what is there stays as it is, said skipped.
    s.spec_write("adhesion-en-ligne/formulaire", |p| {
        p.body = "Changé à la main.".into()
    })
    .unwrap();
    let again = s.import("docs/SPEC.md", None).unwrap();
    assert_eq!(again.skipped, only);
    assert_eq!(again.written.len(), 3);
    assert_eq!(
        s.spec_read("adhesion-en-ligne/formulaire").unwrap().body,
        "Changé à la main."
    );
    // The tree keeps the spec's order, not the alphabet's, and names the subjects as the spec did.
    let tree = s.spec_tree();
    let names: Vec<&str> = tree.iter().map(|t| t.title.as_str()).collect();
    assert_eq!(names, vec!["Introduction", "Adhésion en ligne", "Paiement"]);
    let pages: Vec<&str> = tree[1].pages.iter().map(|p| p.title.as_str()).collect();
    assert_eq!(
        pages,
        vec!["Adhésion en ligne", "Formulaire", "Pièces justificatives"]
    );
}

#[test]
fn a_spec_folder_gives_a_subject_per_file() {
    let (s, root) = space("import-folder");
    std::fs::create_dir_all(root.join("specs/sous")).unwrap();
    std::fs::write(
        root.join("specs/paiement.md"),
        "# Paiement\n\n## Moyens\n\nCarte.\n\n## En plusieurs fois\n\n3 fois.\n",
    )
    .unwrap();
    std::fs::write(root.join("specs/sous/back-office.md"), "Les exports.\n").unwrap();
    let plan = s.import_plan("specs").unwrap();
    let got: Vec<(&str, &str)> = plan
        .iter()
        .map(|d| (d.path.as_str(), d.source.as_str()))
        .collect();
    assert_eq!(
        got,
        vec![
            ("paiement/moyens", "specs/paiement.md"),
            ("paiement/en-plusieurs-fois", "specs/paiement.md"),
            ("back-office/back-office", "specs/sous/back-office.md"),
        ]
    );
}

#[test]
fn only_a_spec_escouade_found_is_read_and_never_through_a_link() {
    let (s, root) = space("import-refused");
    std::fs::write(root.join("README.md"), "# Lisez-moi\n").unwrap();
    assert!(s.import_plan("README.md").is_err());
    assert!(s.import_plan("../SPEC.md").is_err());
    // A SPEC.md that is a link is not offered, so not read.
    let outside = test_dir("import-refused-outside");
    std::fs::write(outside.join("secret.md"), "# Secret\n").unwrap();
    if make_file_link(&outside.join("secret.md"), &root.join("SPEC.md")) {
        assert!(s.detect().is_empty());
        assert!(s.import_plan("SPEC.md").is_err());
    }
}

#[test]
fn a_page_keeps_its_gaps_its_draft_and_its_place() {
    let (s, _) = space("page-gaps");
    s.spec_write("paiement/en-plusieurs-fois", |p| {
        p.body = "Trois fois au plus.".into();
        p.gaps = vec![Gap {
            text: "La spec dit 3 fois au plus ; le code accepte 4 échéances.".into(),
            code: vec!["src/payment.ts".into()],
        }];
        p.draft = true;
        p.order = 2;
    })
    .unwrap();
    let p = s.spec_read("paiement/en-plusieurs-fois").unwrap();
    assert_eq!(p.gaps.len(), 1);
    assert_eq!(p.gaps[0].code, vec!["src/payment.ts".to_string()]);
    assert!(p.draft);
    assert_eq!(p.order, 2);
    // Written again by an edit of its text, they stay.
    s.spec_write("paiement/en-plusieurs-fois", |p| {
        p.body = "Quatre fois.".into()
    })
    .unwrap();
    let info = &s.spec_tree()[0].pages[0];
    assert_eq!((info.gaps, info.draft), (1, true));
    // A gap with no text is none.
    let text = "---\ntitle: \"A\"\ngaps: [{\"text\":\"  \"}]\n---\n\nx\n";
    assert!(page_read("a/b", text).gaps.is_empty());
}

#[test]
fn the_last_changes_of_the_spec_are_read_from_git_log() {
    let log = "\u{1e}1761650000\u{1f}Claire\u{1f}Docs : Paiement\n\n.escouade/spec/paiement/moyens.md\n.escouade/spec/paiement/en-plusieurs-fois.md\n\u{1e}1761600000\u{1f}Guillaume\u{1f}init\n\n.escouade/spec/a/b.md\n";
    let m = crate::docsshare::parse_recent(log);
    assert_eq!(m.len(), 2);
    assert_eq!(m[0].author, "Claire");
    assert_eq!(
        m[0].pages,
        vec![
            "paiement/moyens".to_string(),
            "paiement/en-plusieurs-fois".to_string()
        ]
    );
    assert_eq!(m[1].when, 1761600000);
}

/// ESC-653: a spec file over a page's cap (2 MiB) is not read, nor a folder whose files pass
/// 32 MiB together, nor a folder reached through a link (a junction under Windows).
#[test]
fn an_import_is_read_within_its_caps_and_never_through_a_linked_folder() {
    let (s, root) = space("import-caps");
    std::fs::write(root.join("SPEC.md"), vec![b'a'; MAX_TEXT as usize + 1]).unwrap();
    let err = s.import_plan("SPEC.md").unwrap_err().to_string();
    assert!(err.contains("SPEC.md"), "{err}");
    std::fs::write(root.join("SPEC.md"), SPEC).unwrap();
    assert_eq!(s.import_plan("SPEC.md").unwrap().len(), 5);
    // 17 files at the cap of a page each: past the cap of an import.
    let specs = root.join("specs");
    std::fs::create_dir_all(&specs).unwrap();
    let page = [b"# T\n\n".as_slice(), &vec![b'a'; MAX_TEXT as usize - 5]].concat();
    for i in 0..17 {
        std::fs::write(specs.join(format!("p{i:02}.md")), &page).unwrap();
    }
    assert!(s.import_plan("specs").is_err());
    for i in 1..17 {
        std::fs::remove_file(specs.join(format!("p{i:02}.md"))).unwrap();
    }
    assert_eq!(s.import_plan("specs").unwrap().len(), 1);
    // A folder linked from the spec's folder is not walked.
    let outside = test_dir("import-caps-outside");
    std::fs::write(outside.join("secret.md"), "# Secret\n\nsecret\n").unwrap();
    assert!(crate::paths::make_dir_link(&outside, &specs.join("out")));
    let plan = s.import_plan("specs").unwrap();
    assert!(plan.iter().all(|d| !d.body.contains("secret")), "{plan:?}");
    #[cfg(windows)]
    std::fs::remove_dir(specs.join("out")).unwrap();
}
