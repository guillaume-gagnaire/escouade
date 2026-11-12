//! « Vue Gemini » alone: its arguments, settings and sessions (ESC-275).

use super::*;

#[test]
fn its_interface_says_when_it_does_not_find_its_session() {
    assert!(
        GEMINI_TERMINAL.says_session_lost(b"Error resuming session: Invalid session identifier")
    );
    assert!(!GEMINI_TERMINAL.says_session_lost(b"fake-gemini-tui"));
}

#[test]
fn its_sessions_are_found_by_their_short_id_in_any_project() {
    let home = crate::paths::test_dir("esc275-sessions");
    let chats = home
        .join(".gemini")
        .join("tmp")
        .join("abcdef")
        .join("chats");
    std::fs::create_dir_all(&chats).unwrap();
    let id = "9f2c1a7b-1111-2222-3333-444455556666";
    assert!(session_file(&home, id).is_none());
    let file = chats.join("session-2026-10-29T10-00-9f2c1a7b.jsonl");
    std::fs::write(&file, "{}\n").unwrap();
    assert_eq!(session_file(&home, id), Some(file));
    assert!(session_file(&home, "../etc/passwd").is_none());
    assert!(session_file(&home, "short").is_none());
}

#[test]
fn its_process_takes_away_what_its_view_left_in_its_settings() {
    let doc = json!({
        "ui": { "theme": "x" },
        "hooks": { "SessionStart": [] },
        "hooksConfig": { "enabled": true },
        "mcpServers": { "mine": { "command": "m" }, "escouade": { "httpUrl": "u" } },
    });
    // Its process starts: its view's hooks and Escouade's server go, the user's server stays.
    let mut back = doc.clone();
    forget_view(&mut back);
    assert!(back["hooks"].is_null() && back["hooksConfig"].is_null());
    assert!(back["mcpServers"]["escouade"].is_null());
    assert_eq!(back["mcpServers"]["mine"]["command"], "m");
    let mut alone = json!({ "mcpServers": { "escouade": { "httpUrl": "u" } } });
    forget_view(&mut alone);
    assert!(alone.get("mcpServers").is_none());
}
