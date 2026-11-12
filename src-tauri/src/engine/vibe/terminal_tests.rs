use super::*;

/// A session of the agent's `VIBE_HOME` with a segment of its journal.
fn written_session(home: &Path, session: &str) {
    let journal = session_dir(home, session).join("journal");
    std::fs::create_dir_all(&journal).unwrap();
    std::fs::write(journal.join("0000000000000001.jsonl"), "{}\n").unwrap();
}

#[test]
fn its_log_is_the_latest_segment_of_the_sessions_journal_which_moves() {
    let home = crate::paths::test_dir("esc268-vibe-log");
    assert!(VIBE_TERMINAL.find_log(&home, "s1").is_none());
    written_session(&home, "s1");
    let journal = session_dir(&home, "s1").join("journal");
    std::fs::write(journal.join("0000000000000023.jsonl"), "").unwrap();
    assert_eq!(
        VIBE_TERMINAL.find_log(&home, "s1"),
        Some(journal.join("0000000000000023.jsonl"))
    );
    assert!(VIBE_TERMINAL.log_moves());
    assert!(VIBE_TERMINAL.find_log(&home, "../s1").is_none());
}

#[test]
fn its_interface_is_vibe_beside_vibe_acp_and_a_fake_plays_both() {
    let dir = crate::paths::test_dir("esc268-vibe-program");
    let ext = if cfg!(windows) { ".exe" } else { "" };
    let acp = dir.join(format!("vibe-acp{ext}"));
    std::fs::write(&acp, "").unwrap();
    // Alone, kept as it is.
    assert_eq!(
        interface_of(&Program::Native(acp.clone())),
        Program::Native(acp.clone())
    );
    let vibe = dir.join(format!("vibe{ext}"));
    std::fs::write(&vibe, "").unwrap();
    assert_eq!(interface_of(&Program::Native(acp)), Program::Native(vibe));
    let fake = dir.join("fake-vibe.cmd");
    assert_eq!(
        interface_of(&Program::Native(fake.clone())),
        Program::Native(fake)
    );
}

#[test]
fn the_hooks_of_its_view_are_taken_away_and_quit_and_interrupt_keys_are_its_own() {
    let home = crate::paths::test_dir("esc268-vibe-forget");
    std::fs::write(home.join(HOOKS), "x").unwrap();
    forget_hooks(&home);
    assert!(!home.join(HOOKS).exists());
    assert_eq!(VIBE_TERMINAL.interrupt_key(), b"\x1b");
    assert_eq!(VIBE_TERMINAL.quit_key(), b"\x03");
    assert!(VIBE_TERMINAL
        .says_session_lost(b"Error: Failed to resume session: Session not found: 1111"));
    assert!(!VIBE_TERMINAL.names_its_session());
}
