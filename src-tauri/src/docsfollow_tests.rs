//! A spec imported into Escouade (ESC-278): the paths it came from are kept with the docs, once each,
//! the old file is never touched, and every agent launched on a project whose spec is in Escouade is
//! told to follow Escouade's docs alone, whatever its harness.

use crate::core::AgentOptions;
use crate::core_tests::harness;
use crate::docs::*;
use crate::model::{Account, EngineId};
use crate::paths::test_dir;

const OLD: &str = "# Portail\n\nIntro.\n\n## Paiement\n\nCarte.\n\n## Reçu\n\nPar e-mail.\n";

fn imported_space(name: &str) -> (Space, std::path::PathBuf) {
    let root = test_dir(name);
    std::fs::write(root.join("SPEC.md"), OLD).unwrap();
    let s = Space::new(&root, None);
    s.import("SPEC.md", None).unwrap();
    (s, root)
}

#[test]
fn the_imports_are_kept_with_the_docs_once_each_and_the_old_file_stays_as_it_was() {
    let (s, root) = imported_space("follow-imports");
    let kept = s.imported();
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].path, "SPEC.md");
    assert_eq!(kept[0].at.len(), 10, "{}", kept[0].at);
    // With the spec, in a file the team shares by git.
    assert!(root.join(".escouade/spec/.imports.json").is_file());
    assert!(crate::docsshare::is_docs_path(
        ".escouade/spec/.imports.json"
    ));
    // Again, and another path: no duplicate.
    s.import("SPEC.md", None).unwrap();
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(
        root.join("docs/regles.md"),
        "# Règles\n\n## Délais\n\nTrente jours.\n",
    )
    .unwrap();
    s.import("docs", None).unwrap();
    s.import("docs", None).unwrap();
    let paths: Vec<String> = s.imported().into_iter().map(|i| i.path).collect();
    assert_eq!(paths, vec!["SPEC.md", "docs"]);
    // The imports file is not a page.
    assert!(s.pages().iter().all(|p| !p.path.contains("imports")));
    // The original: byte for byte.
    assert_eq!(std::fs::read_to_string(root.join("SPEC.md")).unwrap(), OLD);
}

#[test]
fn the_rule_names_the_old_files_and_is_short_and_absent_without_a_spec() {
    let (s, _) = imported_space("follow-rule");
    for lang in [crate::i18n::Lang::Fr, crate::i18n::Lang::En] {
        let rule = s.follow_rule(lang).unwrap();
        assert!(rule.contains("SPEC.md"), "{rule}");
        for tool in [
            "get_spec_tree",
            "read_spec_page",
            "search_spec",
            "list_decisions",
            "read_decision",
        ] {
            assert!(rule.contains(tool), "{rule}");
        }
        assert!(!rule.contains('\n') && rule.len() < 600, "{rule}");
    }
    // A spec written in the tab, never imported: the rule, without an old file to name.
    let written = Space::new(test_dir("follow-written"), None);
    assert!(written.follow_rule(crate::i18n::Lang::En).is_none());
    written.spec_write("a/b", |p| p.body = "x".into()).unwrap();
    let rule = written.follow_rule(crate::i18n::Lang::En).unwrap();
    assert!(
        rule.contains("get_spec_tree") && !rule.contains("Old spec"),
        "{rule}"
    );
    // A project with no spec in Escouade: nothing, even with a SPEC.md in the folder.
    let root = test_dir("follow-none");
    std::fs::write(root.join("SPEC.md"), OLD).unwrap();
    assert!(Space::new(&root, None)
        .follow_rule(crate::i18n::Lang::Fr)
        .is_none());
}

fn codex_account(h: &crate::core_tests::Harness) {
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "principal-codex".into(),
            name: "principal-codex".into(),
            engine: EngineId::CODEX,
            claude_path: crate::engine::fake_codex(),
            active: true,
            ..Default::default()
        });
    });
}

async fn told(
    h: &crate::core_tests::Harness,
    project: &str,
    engine: Option<EngineId>,
    ticket: bool,
) -> String {
    let t = crate::model::Ticket {
        key: "DEM-7".into(),
        title: "Quatre échéances".into(),
        ..Default::default()
    };
    let protocol = ticket
        .then(|| crate::board::protocol_prompt_for(crate::i18n::Lang::Fr, &t, None, false, None));
    let id = h
        .core
        .create_agent_with(
            project,
            AgentOptions {
                engine,
                append_prompt: protocol,
                ticket_id: ticket.then(|| "t1".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    let said = h.core.agent(&id).unwrap().lock().meta.append_prompt.clone();
    said.unwrap_or_default()
}

#[tokio::test]
async fn every_agent_of_a_project_with_its_spec_in_escouade_is_told_to_follow_it_on_two_harness() {
    let h = harness("follow-agents");
    codex_account(&h);
    let (p, repo) = h.project(false).await;
    // No spec in Escouade yet: nothing is added, whatever the agent.
    for (engine, ticket) in [(None, false), (None, true), (Some(EngineId::CODEX), false)] {
        let said = told(&h, &p.id, engine, ticket).await;
        assert!(!said.contains("get_spec_tree"), "{said}");
    }
    std::fs::write(repo.join("SPEC.md"), OLD).unwrap();
    h.core
        .docs_space(&p.id, None)
        .await
        .unwrap()
        .import("SPEC.md", None)
        .unwrap();
    let rule = h
        .core
        .docs_space(&p.id, None)
        .await
        .unwrap()
        .follow_rule(crate::i18n::Lang::Fr)
        .unwrap();
    // A ticket's agent (its protocol first, its spec rule in it), and agents with no ticket, on Claude Code and Codex.
    let ticket = told(&h, &p.id, None, true).await;
    assert!(
        ticket.contains(&rule) && ticket.contains("DEM-7"),
        "{ticket}"
    );
    assert!(ticket.contains(&crate::board::spec_rule(crate::i18n::Lang::Fr)));
    for engine in [None, Some(EngineId::CODEX)] {
        let free = told(&h, &p.id, engine, false).await;
        assert!(free.contains(&rule) && free.contains("SPEC.md"), "{free}");
    }
    // The old file is as it was.
    assert_eq!(std::fs::read_to_string(repo.join("SPEC.md")).unwrap(), OLD);
}
