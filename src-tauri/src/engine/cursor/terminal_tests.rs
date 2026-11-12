use super::*;

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("escouade-{name}-{}", crate::model::new_id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_folder_is_named_as_cursor_names_it_in_its_projects() {
    // As Cursor CLI 2026.10.22 named them (~/.cursor/projects, ESC-24 and ESC-49).
    assert_eq!(
        project_key(r"C:\Users\user\AppData\Local\Temp\esc24\wt-agent1"),
        "C-Users-user-AppData-Local-Temp-esc24-wt-agent1"
    );
    assert_eq!(
        project_key("/home/u/dev/my.app/.claude/worktrees/x_1/"),
        "home-u-dev-my-app-claude-worktrees-x-1"
    );
}

#[test]
fn its_transcript_is_found_in_any_workspace_once_written_and_its_trust_by_its_file() {
    let projects = temp("cursor-projects");
    let cwd = r"C:\work\repo";
    assert!(asks_trust(&projects, cwd));
    let dir = projects.join(project_key(cwd));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(".workspace-trusted"), "{}").unwrap();
    assert!(!asks_trust(&projects, cwd));
    assert!(asks_trust(&projects, r"C:\work\other"));

    assert_eq!(find_transcript(&projects, "s1"), None);
    let file = dir.join("agent-transcripts").join("s1").join("s1.jsonl");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "{}\n").unwrap();
    assert_eq!(find_transcript(&projects, "s1"), Some(file));
    // Never a path for a session.
    assert_eq!(find_transcript(&projects, "../s1"), None);
    assert_eq!(find_transcript(&projects, ""), None);
}

#[test]
fn the_relay_is_a_rule_always_applied_in_the_agents_folder() {
    let path = CURSOR_TERMINAL.relay_path(Path::new("C:/work/wt")).unwrap();
    assert_eq!(
        path,
        Path::new("C:/work/wt")
            .join(".cursor")
            .join("rules")
            .join("escouade-relais.mdc")
    );
    let file = String::from_utf8(CURSOR_TERMINAL.relay_file("  # Avant\n\nUn mot.\n")).unwrap();
    assert!(file.starts_with("---\n"), "{file}");
    assert!(
        file.contains("\nalwaysApply: true\n---\n\n# Avant\n\nUn mot.\n"),
        "{file}"
    );
}

#[test]
fn a_message_waits_while_it_may_be_on_its_question_about_the_folder() {
    // `paste_refusal` reads `projects_dir`: in the unit tests a folder of no workspace's.
    let cwd = temp("cursor-untrusted").to_string_lossy().into_owned();
    let refused = CURSOR_TERMINAL.paste_refusal(Path::new("."), &cwd, false);
    assert!(refused.is_some_and(|r| r.contains("Cursor")));
    // Past it (it wrote in its transcript since it started): nothing is refused.
    assert!(CURSOR_TERMINAL
        .paste_refusal(Path::new("."), &cwd, true)
        .is_none());
}
