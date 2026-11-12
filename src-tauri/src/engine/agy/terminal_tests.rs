use super::*;

#[test]
fn its_lost_conversation_and_its_hooks_taken_away() {
    // What 1.4.9 printed of an unknown conversation, before going on in a new one.
    assert!(AGY_TERMINAL.says_session_lost(
        b"warning: conversation \"0b6c1b52-1111-4222-8333-944455556666\" not found\r\n"
    ));
    assert!(!AGY_TERMINAL.says_session_lost(b"Antigravity CLI 1.4.9"));
    assert_eq!(AGY_TERMINAL.find_log(Path::new("/h"), "../x"), None);
    let home = crate::paths::test_dir("esc272-agy-forget");
    let hooks = hooks_file(&home);
    std::fs::create_dir_all(hooks.parent().unwrap()).unwrap();
    std::fs::write(&hooks, "{}").unwrap();
    forget_hooks(&home);
    assert!(!hooks.exists());
}
