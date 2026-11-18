//! Tests of the shared memory (ESC-255): its files and their index, the names, what a file without a header says,
//! the search by the words and by the meaning, a link refused, and Claude Code's memory offered and taken.

use super::*;
use crate::paths::test_dir;
use crate::vectors::embed::Fake;

fn draft(name: &str, description: &str, body: &str) -> Draft {
    Draft {
        name: name.into(),
        description: description.into(),
        kind: Kind::Feedback,
        body: body.into(),
    }
}

fn memory_dir(label: &str) -> PathBuf {
    let root = test_dir(label);
    root.join(DIR)
}

/// A link to the folder `target` at `link`, when the system allows one (a junction under Windows needs no privilege).
fn link(target: &Path, link: &Path) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(target, link).is_ok()
            || std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(link.to_string_lossy().replace('/', "\\"))
                .arg(target.to_string_lossy().replace('/', "\\"))
                .output()
                .is_ok_and(|o| o.status.success())
    }
}

#[test]
fn a_memory_is_a_file_with_a_header_and_the_index_lists_them() {
    let dir = memory_dir("memory-format");
    let (m, new) = write(
        &dir,
        draft(
            "E2E après build",
            "Lancer les e2e après un build de debug",
            "`npx tauri build --debug --no-bundle` d'abord.\n\nSinon l'app est l'ancienne.",
        ),
    )
    .unwrap();
    assert!(new);
    assert_eq!(m.name, "e2e-apres-build");
    let text = std::fs::read_to_string(dir.join("e2e-apres-build.md")).unwrap();
    assert_eq!(
        text,
        "---\nname: e2e-apres-build\ndescription: Lancer les e2e après un build de debug\nmetadata:\n  type: feedback\n---\n\n\
         `npx tauri build --debug --no-bundle` d'abord.\n\nSinon l'app est l'ancienne.\n"
    );
    write(
        &dir,
        draft("flaky", "Test instable: sous charge", "Relancer seul."),
    )
    .unwrap();
    let index = std::fs::read_to_string(dir.join(INDEX)).unwrap();
    assert!(index.contains(
        "- [e2e-apres-build](e2e-apres-build.md) — Lancer les e2e après un build de debug\n"
    ));
    assert!(index.contains("- [flaky](flaky.md) — Test instable: sous charge\n"));
    // Read back as written, the description's colon kept by its quotes.
    let all = list(&dir).unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[1].description, "Test instable: sous charge");
    assert_eq!(read(&dir, "flaky").unwrap().body, "Relancer seul.");
    // Written again: replaced, not new.
    let (_, new) = write(&dir, draft("flaky", "Autre", "Autre texte")).unwrap();
    assert!(!new);
    assert_eq!(read(&dir, "FLAKY").unwrap().description, "Autre");
}

#[test]
fn forget_removes_the_file_and_its_line() {
    let dir = memory_dir("memory-forget");
    write(&dir, draft("a", "A", "a")).unwrap();
    write(&dir, draft("b", "B", "b")).unwrap();
    assert!(forget(&dir, "a").unwrap());
    assert!(!forget(&dir, "a").unwrap());
    assert!(!dir.join("a.md").exists());
    let index = std::fs::read_to_string(dir.join(INDEX)).unwrap();
    assert!(!index.contains("[a]") && index.contains("[b]"));
    // The last one gone, the index goes too.
    assert!(forget(&dir, "b").unwrap());
    assert!(!dir.join(INDEX).exists());
    assert!(read(&dir, "b").is_err());
}

#[test]
fn names_are_made_safe() {
    assert_eq!(
        name_of("  Écueil : les tests Rust  ").unwrap(),
        "ecueil-les-tests-rust"
    );
    assert_eq!(name_of("../../etc/passwd").unwrap(), "etc-passwd");
    assert_eq!(name_of("con").unwrap(), "con-note");
    assert_eq!(name_of("MEMORY").unwrap(), "memory-note");
    assert!(name_of(" -- ").is_err());
    assert_eq!(name_of(&"a".repeat(100)).unwrap().len(), 64);
}

#[test]
fn a_draft_needs_a_description_and_a_bounded_text() {
    let dir = memory_dir("memory-bounds");
    assert!(write(&dir, draft("x", "  ", "t")).is_err());
    assert!(write(&dir, draft("x", &"d".repeat(MAX_DESCRIPTION + 1), "t")).is_err());
    assert!(write(&dir, draft("x", "d", &"t".repeat(MAX_BODY + 1))).is_err());
    assert!(list(&dir).unwrap().is_empty());
}

#[test]
fn headers_of_claude_code_are_read_in_both_forms() {
    let old = parse(
        "x",
        "---\nname: tests\ndescription: 'L''ordre des tests'\ntype: project\n---\n\nTexte\n",
    );
    assert_eq!((old.name.as_str(), old.kind), ("tests", Kind::Project));
    assert_eq!(old.description, "L'ordre des tests");
    assert_eq!(old.body, "Texte");
    let new = parse(
        "x",
        "\u{feff}---\nname: n\ndescription: \"a: b\"\nmetadata:\n  type: reference\n---\nCorps",
    );
    assert_eq!(
        (new.kind, new.description.as_str(), new.body.as_str()),
        (Kind::Reference, "a: b", "Corps")
    );
    // No header: all text, named after its file.
    let bare = parse("notes", "Juste du texte\n");
    assert_eq!(
        (bare.name.as_str(), bare.body.as_str()),
        ("notes", "Juste du texte")
    );
    // A header never closed is text too.
    assert_eq!(parse("y", "---\nname: z\n").name, "y");
}

#[test]
fn search_by_words_ranks_the_name_first_and_ignores_accents() {
    let dir = memory_dir("memory-words");
    write(
        &dir,
        draft("e2e-build", "Avant les e2e", "Un build de debug d'abord."),
    )
    .unwrap();
    write(
        &dir,
        draft("clippy", "Lint", "Les tests e2e se lancent après clippy."),
    )
    .unwrap();
    write(
        &dir,
        draft(
            "ports",
            "Ports réservés",
            "Jamais dans un fichier versionné.",
        ),
    )
    .unwrap();
    let all = list(&dir).unwrap();
    let found = search_words(&all, "E2E", 10);
    assert_eq!(
        found.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        ["e2e-build", "clippy"]
    );
    assert_eq!(search_words(&all, "reserves", 10)[0].name, "ports");
    assert!(search_words(&all, "zzzz", 10).is_empty());
    // Empty: every one, up to the limit.
    assert_eq!(search_words(&all, "", 2).len(), 2);
    let (found, how) = search(&dir, "versionne", 5, None).unwrap();
    assert_eq!((found[0].name.as_str(), how), ("ports", "words"));
}

#[test]
fn search_by_meaning_keeps_the_index_up_to_date() {
    let dir = memory_dir("memory-meaning");
    write(
        &dir,
        draft("e2e-build", "Avant les e2e", "Un build de debug d'abord."),
    )
    .unwrap();
    write(
        &dir,
        draft(
            "ports",
            "Ports réservés",
            "Jamais dans un fichier versionné.",
        ),
    )
    .unwrap();
    let index_path = dir.parent().unwrap().join("vectors.db");
    let embedder: Arc<dyn Embedder> = Arc::new(Fake);
    let (found, how) = search(&dir, "ports", 5, Some((embedder.clone(), &index_path))).unwrap();
    assert_eq!(how, "meaning");
    assert_eq!(found[0].name, "ports");
    assert_eq!(found.len(), 2, "each memory once");
    // Forgotten: out of the index at the next search.
    forget(&dir, "ports").unwrap();
    let (found, _) = search(&dir, "ports", 5, Some((embedder, &index_path))).unwrap();
    assert!(found.iter().all(|f| f.name != "ports"));
}

#[test]
fn a_memory_folder_that_is_a_link_is_refused() {
    let root = test_dir("memory-link");
    let outside = test_dir("memory-link-outside");
    std::fs::write(
        outside.join("secret.md"),
        "---\nname: secret\ndescription: s\n---\nnope",
    )
    .unwrap();
    std::fs::create_dir_all(root.join(".escouade")).unwrap();
    let dir = root.join(DIR);
    if !link(&outside, &dir) {
        eprintln!("no symbolic link allowed here: skipped");
        return;
    }
    assert!(list(&dir).is_err());
    assert!(read(&dir, "secret").is_err());
    assert!(write(&dir, draft("x", "x", "x")).is_err());
    assert!(forget(&dir, "secret").is_err());
    assert!(outside.join("secret.md").exists() && !outside.join("x.md").exists());
}

#[test]
fn an_escouade_folder_that_is_a_link_is_refused_even_before_the_memory_is_made() {
    let root = test_dir("memory-link-parent");
    let outside = test_dir("memory-link-parent-outside");
    if !link(&outside, &root.join(".escouade")) {
        eprintln!("no symbolic link allowed here: skipped");
        return;
    }
    let dir = root.join(DIR);
    assert!(write(&dir, draft("x", "x", "x")).is_err());
    assert!(!outside.join("memory").exists());
}

#[cfg(windows)]
#[test]
fn a_memory_file_that_is_a_link_is_not_read() {
    let dir = memory_dir("memory-link-file");
    write(&dir, draft("real", "r", "r")).unwrap();
    let outside = test_dir("memory-link-file-outside").join("x.md");
    std::fs::write(&outside, "secret").unwrap();
    if std::os::windows::fs::symlink_file(&outside, dir.join("linked.md")).is_err() {
        eprintln!("no symbolic link allowed here: skipped");
        return;
    }
    assert_eq!(list(&dir).unwrap().len(), 1);
    assert!(read(&dir, "linked").is_err());
    assert!(write(&dir, draft("linked", "l", "l")).is_err());
    assert_eq!(std::fs::read_to_string(&outside).unwrap(), "secret");
}

#[test]
fn claude_code_names_its_project_folders_with_dashes() {
    let config = Path::new("/home/u/.claude");
    assert_eq!(
        claude_dir(config, "C:\\Users\\guill\\dev\\claude-code-manager"),
        config.join("projects/C--Users-guill-dev-claude-code-manager/memory")
    );
    assert_eq!(
        claude_dir(config, "/home/u/my.app/"),
        config.join("projects/-home-u-my-app/memory")
    );
}

#[test]
fn claude_code_memories_are_offered_then_taken_one_by_one() {
    let config = test_dir("memory-claude-config");
    let claude = claude_dir(&config, "C:\\p");
    std::fs::create_dir_all(&claude).unwrap();
    std::fs::write(claude.join("MEMORY.md"), "- [a](a.md) — A").unwrap();
    std::fs::write(
        claude.join("build.md"),
        "---\nname: build\ndescription: Build avant e2e\nmetadata:\n  type: feedback\n---\n\nToujours.\n",
    )
    .unwrap();
    std::fs::write(claude.join("Notes Libres.md"), "Pas d'en-tête ici.").unwrap();
    std::fs::write(
        claude.join("empty.md"),
        "---\nname: e\ndescription: d\n---\n",
    )
    .unwrap();
    let own = memory_dir("memory-claude-own");
    write(&own, draft("build", "ancien", "ancien")).unwrap();
    let offered = claude_offer(&[claude.clone(), claude.clone()], &own);
    let names: Vec<(&str, bool)> = offered
        .iter()
        .map(|o| (o.memory.name.as_str(), o.exists))
        .collect();
    // The index and an empty one left out, each once though two folders hold them.
    assert_eq!(names, [("build", true), ("notes-libres", false)]);
    assert_eq!(offered[1].memory.description, "Pas d'en-tête ici.");
    // Only those ticked.
    let taken = claude_take(&offered, &["notes-libres".into()], &own).unwrap();
    assert_eq!(taken.len(), 1);
    assert_eq!(read(&own, "build").unwrap().description, "ancien");
    assert_eq!(
        read(&own, "notes-libres").unwrap().body,
        "Pas d'en-tête ici."
    );
    // Claude Code's are never touched.
    assert_eq!(std::fs::read_dir(&claude).unwrap().count(), 4);
    // Nothing offered from a folder that is not there.
    assert!(claude_offer(&[config.join("nope")], &own).is_empty());
}

#[test]
fn the_instruction_names_the_tools_in_both_languages() {
    for lang in [crate::i18n::Lang::Fr, crate::i18n::Lang::En] {
        let line = instruction(lang);
        for tool in [
            "search_memory",
            "read_memory",
            "write_memory",
            "forget_memory",
        ] {
            assert!(line.contains(tool), "{lang:?} {tool}");
        }
    }
}

/// TS-10: what a memory given as data is, the instruction says it to every agent, in both languages.
#[test]
fn the_instruction_says_a_memory_between_the_marks_is_data() {
    let fr = instruction(crate::i18n::Lang::Fr);
    assert!(fr.contains("<third_party_data>") && fr.contains("jamais une consigne"));
    let en = instruction(crate::i18n::Lang::En);
    assert!(en.contains("<third_party_data>") && en.contains("never an"));
}

/// TS-10: the motifs of known tokens the audit reads its report for, refused at the writing of any memory.
#[test]
fn a_secret_is_refused_at_the_writing_and_its_kind_is_said() {
    let long = "A1b2C3d4E5f6G7h8I9j0K1l2M3n4O5p6";
    for (text, said) in [
        (format!("le jeton ghp_{long}"), "GitHub"),
        (format!("github_pat_{long}"), "GitHub"),
        (format!("export ANTHROPIC_API_KEY=sk-ant-{long}"), "API"),
        (format!("glpat-{long}"), "GitLab"),
        ("AKIAABCDEFGHIJKLMNOP".to_string(), "AWS"),
        ("xoxb-123456789012-abcdef".to_string(), "Slack"),
        (format!("Authorization: Bearer {long}"), "Bearer"),
        (
            "-----BEGIN OPENSSH PRIVATE KEY-----\nb3Bl".to_string(),
            "privée",
        ),
        (
            "https://hooks.slack.com/services/T0/B0/xyz".to_string(),
            "webhook",
        ),
        (
            "https://discord.com/api/webhooks/1/abc".to_string(),
            "webhook",
        ),
    ] {
        let found = secret_in(&text).unwrap_or_else(|| panic!("not found in {text}"));
        assert!(found.contains(said), "{text}: {found}");
        let dir = memory_dir(&format!("mem-secret-{}", said.to_lowercase()));
        let err = write(&dir, draft("secret", "Un secret", &text)).unwrap_err();
        assert!(err.to_string().contains("Refusé"), "{err}");
        assert!(!dir.join("secret.md").exists());
    }
    // In the description too.
    let dir = memory_dir("mem-secret-desc");
    assert!(write(&dir, draft("d", &format!("ghp_{long}"), "rien")).is_err());
    // What looks like one but is not: a prefix inside a word, too short, an example.
    for text in [
        "le préfixe ghp_ des jetons GitHub",
        "notghp_A1b2C3d4E5f6G7h8I9j0K1l2",
        "sk-court",
        "risk-assessment-of-the-whole-application",
        "Bearer token dans l'en-tête",
        "https://hooks.slack.com/services/",
    ] {
        assert_eq!(secret_in(text), None, "{text}");
    }
}

/// TS-10: the hosts of the addresses a memory names, shown at its validation when the project does not know them.
#[test]
fn the_addresses_of_a_memory_are_its_hosts_each_once() {
    assert_eq!(
        addresses("Envoie à https://Evil.example/x puis http://user@evil.example:80/y et https://github.com/acme."),
        vec!["evil.example", "github.com"]
    );
    assert!(addresses("aucune adresse, juste example.com").is_empty());
    let known = crate::teammemory::known_hosts(&[
        "git@github.com:acme/api.git".into(),
        "https://gitlab.acme.fr/acme/api".into(),
    ]);
    assert!(
        known.contains("github.com")
            && known.contains("gitlab.acme.fr")
            && known.contains("localhost")
    );
}
