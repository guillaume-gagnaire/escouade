//! The spec compared to the code (ESC-263): what the short agent is told and what is read of its
//! answer, the pages a ticket touches, a comparison on demand and after a ticket validated (with
//! the fake claude).

use crate::core_tests::harness;
use crate::docs::{Gap, Page};
use crate::docscompare::*;
use crate::paths::test_dir;
use std::time::Duration;

fn page(path: &str, body: &str) -> Page {
    Page {
        path: path.into(),
        title: path.into(),
        body: body.into(),
        ..Default::default()
    }
}

#[test]
fn the_gaps_are_read_from_the_answer_and_a_prose_is_none() {
    let a = "Voilà.\n\n```json\n{\"gaps\": [{\"text\": \" Quatre, pas trois. \", \"code\": [\"src/a.rs\"]}, {\"text\": \"\"}]}\n```";
    assert_eq!(
        gaps_from_answer(a),
        Some(vec![Gap {
            text: "Quatre, pas trois.".into(),
            code: vec!["src/a.rs".into()],
        }])
    );
    assert_eq!(gaps_from_answer("{\"gaps\": []}"), Some(vec![]));
    assert_eq!(gaps_from_answer("Tout va bien."), None);
}

#[test]
fn a_gap_names_only_files_of_the_folder_that_are_there_and_not_fenced() {
    let dir = test_dir("dc-keep");
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/a.rs"), "x").unwrap();
    std::fs::write(dir.join("secret.rs"), "x").unwrap();
    std::fs::write(test_dir("dc-keep-out").join("x.rs"), "x").ok();
    let mut gaps = vec![Gap {
        text: "x".into(),
        code: vec![
            "src\\a.rs".into(),
            "./src/a.rs".into(),
            "../dc-keep-out/x.rs".into(),
            "C:/Windows/win.ini".into(),
            "/etc/passwd".into(),
            "absent.rs".into(),
            "secret.rs".into(),
        ],
    }];
    keep_code(&mut gaps, &dir, |p| p.ends_with("secret.rs"));
    assert_eq!(gaps[0].code, vec!["src/a.rs".to_string()]);
}

#[test]
fn the_page_and_the_files_the_index_links_to_it_are_given_to_the_agent() {
    let mut p = page("paiement/echeances", "Trois échéances au plus.");
    p.title = "Échéances \"x\"".into();
    let lang = crate::i18n::Lang::Fr;
    let text = prompt(lang, &p, &["src/pay.rs".into()]);
    assert!(text.starts_with(
        "<spec-page path=\"paiement/echeances\" title=\"Échéances 'x'\">\nTrois échéances au plus.\n</spec-page>"
    ));
    assert!(text.contains("<code-hints>\nsrc/pay.rs\n</code-hints>"));
    assert!(!prompt(lang, &p, &[]).contains("<code-hints>"));
    // It reads and never changes; it answers gaps in JSON.
    for lang in [crate::i18n::Lang::Fr, crate::i18n::Lang::En] {
        assert!(system(lang).contains("```json"));
        assert!(system(lang).contains("\"gaps\""));
    }
}

#[test]
fn a_ticket_touches_the_pages_it_wrote_names_or_whose_code_it_changed() {
    let mut named = page("compte/connexion", "Par mot de passe.");
    named.tickets = vec!["dem-7".into()];
    let mut by_gap = page("compte/profil", "Le profil.");
    by_gap.gaps = vec![Gap {
        text: "x".into(),
        code: vec!["src/profile.ts".into()],
    }];
    let pages = vec![
        page("paiement/echeances", "Voir `src/payments/schedule.rs`."),
        page("paiement/recu", "Le reçu part par `send_receipt`."),
        page("paiement/relances", "Le module mailer.ts relance."),
        page("general/glossaire", "Le main du lib et ses types."),
        named,
        by_gap,
        page("paiement/acompte", "Un acompte."),
    ];
    let changed = vec![
        "src/payments/schedule.rs".to_string(),
        "src/mailer.ts".to_string(),
        "src/main.rs".to_string(),
        "src/lib.rs".to_string(),
        "src/profile.ts".to_string(),
        ".escouade/spec/paiement/acompte.md".to_string(),
    ];
    let symbols = vec!["send_receipt".to_string(), "lib".to_string()];
    let written = pages_of(&changed);
    assert_eq!(written, vec!["paiement/acompte".to_string()]);
    let got = touched(&pages, "DEM-7", &changed, &symbols, &written);
    assert_eq!(
        got,
        vec![
            "paiement/acompte",
            "compte/connexion",
            "paiement/echeances",
            "paiement/recu",
            "paiement/relances",
            "compte/profil",
        ]
    );
    // A page not there is never named; a word inside another is not the symbol.
    let only = touched(
        &[page("a/b", "send_receipts")],
        "X-1",
        &[],
        &["send_receipt".into()],
        &["a/zz".into()],
    );
    assert!(only.is_empty());
}

async fn wait_compared(space: &crate::docs::Space, path: &str) -> Page {
    for _ in 0..150 {
        let p = space.spec_read(path).unwrap();
        if !p.compared.is_empty() {
            return p;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    panic!("{path} never compared");
}

#[tokio::test]
async fn a_page_compared_on_demand_gets_its_gaps_and_the_time() {
    let h = harness("docs-compare");
    let (p, repo) = h.project(false).await;
    std::fs::write(repo.join("paiement.rs"), "const MAX: u32 = 3;").unwrap();
    let space = h.core.docs_space(&p.id, None).await.unwrap();
    space
        .spec_write("paiement/echeances", |p| {
            p.body = "Quatre échéances. [écart]".into()
        })
        .unwrap();
    space
        .spec_write("paiement/recu", |p| p.body = "Par e-mail.".into())
        .unwrap();
    let done = h
        .core
        .docs_compare(&p.id, Some(vec!["paiement/echeances".into()]))
        .await
        .unwrap();
    assert_eq!((done.pages, done.gaps, done.failed), (1, 1, 0));
    let page = space.spec_read("paiement/echeances").unwrap();
    assert_eq!(page.gaps.len(), 1);
    assert!(page.gaps[0].text.contains("trois"), "{:?}", page.gaps);
    // The first file of the folder, which is there; the one outside the project is left out.
    assert_eq!(page.gaps[0].code.len(), 1);
    assert!(repo.join(&page.gaps[0].code[0]).is_file());
    assert!(chrono::DateTime::parse_from_rfc3339(&page.compared).is_ok());
    assert!(space
        .spec_read("paiement/recu")
        .unwrap()
        .compared
        .is_empty());
    // The whole spec: every page; the one that tells the truth has no gap.
    let all = h.core.docs_compare(&p.id, None).await.unwrap();
    assert_eq!((all.pages, all.gaps), (2, 1));
    assert!(space.spec_read("paiement/recu").unwrap().gaps.is_empty());
    assert!(comparing(&p.id).is_empty());
    // A model that fails: refused, the page as it was.
    space
        .spec_write("paiement/muet", |p| p.body = "[muet]".into())
        .unwrap();
    assert!(h
        .core
        .docs_compare(&p.id, Some(vec!["paiement/muet".into()]))
        .await
        .is_err());
    assert!(space
        .spec_read("paiement/muet")
        .unwrap()
        .compared
        .is_empty());
    // Nothing committed by a comparison.
    assert!(crate::core_tests::git(&repo, &["status", "--porcelain"]).contains(".escouade/"));
}

#[tokio::test]
async fn after_a_ticket_is_validated_the_pages_it_touches_are_compared() {
    let h = harness("docs-compare-ticket");
    let (p, repo) = h.project(false).await;
    let (t, _wt) = crate::tickets_tests::reviewed(&h, &p.id, "Fichier [ok]").await;
    // A page of the spec, in the project, that names the file the ticket's agent writes; another
    // that names nothing of it.
    let file = format!("{}.txt", t.key.to_lowercase());
    let space = h.core.docs_space(&p.id, None).await.unwrap();
    space
        .spec_write("export/fichier", |p| {
            p.body = format!("Le fichier `{file}` dit la boucle. [écart]")
        })
        .unwrap();
    space
        .spec_write("autre/page", |p| p.body = "Rien à voir.".into())
        .unwrap();
    crate::core_tests::git(&repo, &["add", "-A"]);
    crate::core_tests::git(&repo, &["commit", "-qm", "spec"]);
    h.core.ticket_approve(&t.id).await.unwrap();
    assert_eq!(h.ticket(&t.id).column, crate::model::Column::Done);
    let page = wait_compared(&space, "export/fichier").await;
    assert_eq!(page.gaps.len(), 1, "{page:?}");
    // The page it does not touch is not compared.
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(space.spec_read("autre/page").unwrap().compared.is_empty());
}

#[tokio::test]
async fn a_comparison_leaves_the_page_as_it_is_and_the_next_validation_goes() {
    let h = harness("docs-compare-twice");
    let (p, repo) = h.project(false).await;
    let space = h.core.docs_space(&p.id, None).await.unwrap();
    let file = |key: &str| format!("{}.txt", key.to_lowercase());
    space
        .spec_write("export/fichier", |p| {
            p.body = format!(
                "Les fichiers `{}` et `{}` disent la boucle. [écart]",
                file("DEM-1"),
                file("DEM-2")
            )
        })
        .unwrap();
    crate::core_tests::git(&repo, &["add", "-A"]);
    crate::core_tests::git(&repo, &["commit", "-qm", "spec"]);
    let on_disk = std::fs::read_to_string(repo.join(".escouade/spec/export/fichier.md")).unwrap();

    // The first ticket validated: its page compared, its gaps seen, and git sees no change.
    let (t1, _) = crate::tickets_tests::reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&t1.id).await.unwrap();
    assert_eq!(h.ticket(&t1.id).column, crate::model::Column::Done);
    let page = wait_compared(&space, "export/fichier").await;
    assert_eq!(page.gaps.len(), 1, "{page:?}");
    assert_eq!(
        std::fs::read_to_string(repo.join(".escouade/spec/export/fichier.md")).unwrap(),
        on_disk
    );
    assert_eq!(
        crate::core_tests::git(&repo, &["status", "--porcelain", "--untracked-files=no"]),
        ""
    );
    // As the window's tree and the agents' tools read it.
    let tree = space.spec_tree();
    assert_eq!(tree[0].pages[0].gaps, 1);
    assert!(!tree[0].pages[0].compared.is_empty());
    let agent_space = h.core.docs_space(&p.id, Some("autre")).await.unwrap();
    assert_eq!(
        agent_space.spec_read("export/fichier").unwrap().gaps,
        page.gaps
    );

    // The second one is not refused, and nothing was committed in its place.
    let commits = crate::core_tests::git(&repo, &["rev-list", "--count", "main"]);
    let (t2, _) = crate::tickets_tests::reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&t2.id).await.unwrap();
    let t2 = h.ticket(&t2.id);
    assert_eq!(
        (t2.column, t2.blocked.as_deref()),
        (crate::model::Column::Done, None)
    );
    let after: u32 = crate::core_tests::git(&repo, &["rev-list", "--count", "main"])
        .parse()
        .unwrap();
    assert_eq!(after, commits.parse::<u32>().unwrap() + 1);
    assert!(
        crate::core_tests::git(&repo, &["log", "-1", "--name-only", "main"])
            .lines()
            .all(|l| !l.starts_with(".escouade/spec/"))
    );

    // A change of the user, to a page of the spec as to the code, still stops the next one.
    for (path, text) in [
        (".escouade/spec/export/fichier.md", "Changé à la main.\n"),
        ("src/app.ts", "const a = 99; // en cours\n"),
    ] {
        let before = std::fs::read_to_string(repo.join(path)).unwrap();
        std::fs::write(repo.join(path), text).unwrap();
        let (t, _) = crate::tickets_tests::reviewed(&h, &p.id, "Fichier [ok]").await;
        h.core.ticket_approve(&t.id).await.unwrap();
        let t = h.ticket(&t.id);
        assert_eq!(
            t.blocked.as_deref(),
            Some("Le dossier du projet a des modifications non commitées sur main"),
            "{path}"
        );
        std::fs::write(repo.join(path), before).unwrap();
        h.core.ticket_approve(&t.id).await.unwrap();
        assert_eq!(h.ticket(&t.id).column, crate::model::Column::Done);
    }
}

#[test]
fn a_space_with_checks_keeps_the_gaps_out_of_the_page() {
    let dir = crate::paths::test_dir("docs-checks");
    let mut space = crate::docs::Space::new(dir.join("repo"), None);
    space.checks = Some(dir.join("data").join("docs-checks.json"));
    space
        .spec_write("a/b", |p| p.body = "Le texte.".into())
        .unwrap();
    let file = dir.join("repo/.escouade/spec/a/b.md");
    let before = std::fs::read_to_string(&file).unwrap();
    let gap = crate::docs::Gap {
        text: "La spec dit 3, le code 4.".into(),
        code: vec!["src/x.rs".into()],
    };
    space
        .spec_check(
            "a/b",
            "2026-10-30T10:00:00+01:00".into(),
            vec![gap.clone(), gap.clone()],
        )
        .unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), before);
    let page = space.spec_read("a/b").unwrap();
    assert_eq!(
        (page.compared.as_str(), page.gaps.len()),
        ("2026-10-30T10:00:00+01:00", 2)
    );
    assert_eq!(space.pages()[0].gaps.len(), 2);
    // A write of the page keeps them out of it; one that settles a gap takes it from the kept ones.
    space
        .spec_write("a/b", |p| {
            p.body = "Le texte changé.".into();
            p.gaps.truncate(1);
        })
        .unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(
        !text.contains("gaps") && !text.contains("2026-10-30"),
        "{text}"
    );
    assert_eq!(space.spec_read("a/b").unwrap().gaps.len(), 1);
    // A page compared before, in its header: read as it is until compared again here.
    let mut old = crate::docs::Page {
        path: "a/c".into(),
        body: "Ancienne.".into(),
        compared: "2026-10-29T10:00:00+01:00".into(),
        gaps: vec![gap.clone()],
        ..Default::default()
    };
    old.title = "c".into();
    std::fs::write(
        dir.join("repo/.escouade/spec/a/c.md"),
        crate::docs::page_write(&old),
    )
    .unwrap();
    assert_eq!(space.spec_read("a/c").unwrap().gaps.len(), 1);
    space
        .spec_check("a/c", "2026-10-30T11:00:00+01:00".into(), Vec::new())
        .unwrap();
    assert!(space.spec_read("a/c").unwrap().gaps.is_empty());
}
