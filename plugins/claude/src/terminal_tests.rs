//! « Vue Claude Code »: the launch of its interface (`terminal/launch`) and the models of an account (`account/read`), as
//! the core's tests of its view had them before it left the core (ESC-359).

use super::*;
use escouade_plugin_protocol::engine::Grant;

fn params() -> TerminalParams {
    TerminalParams {
        agent: "tui-a1".into(),
        cwd: "/x/repo".into(),
        model: "sonnet".into(),
        effort: "high".into(),
        mode: "default".into(),
        new_session: Some("s-new".into()),
        hook_url: "http://127.0.0.1:4120/hook".into(),
        hook_token_var: "ESCOUADE_HOOK_TOKEN".into(),
        private_file: "/data/tui/a1.json".into(),
        ..TerminalParams::default()
    }
}

fn has(a: &[String], x: &str) -> bool {
    a.iter().any(|y| y == x)
}

#[test]
fn a_new_agent_starts_a_session_of_its_own_and_a_known_one_is_resumed() {
    let a = args(&params());
    assert!(has(&a, "--session-id=s-new"));
    assert!(a
        .windows(2)
        .any(|w| w == ["--settings", "/data/tui/a1.json"]));
    assert!(a.windows(2).any(|w| w == ["--permission-mode", "default"]));
    assert!(a.windows(2).any(|w| w == ["--effort", "high"]));
    // Nothing of the stream's.
    assert!(!a.iter().any(|x| x.contains("stream-json") || x == "-p"));
    let resumed = args(&TerminalParams {
        session: Some("s1".into()),
        new_session: None,
        ..params()
    });
    assert!(has(&resumed, "--resume=s1"));
    assert!(!resumed.iter().any(|x| x.starts_with("--session-id")));
    // A copy forks its original's into its own.
    let copy = args(&TerminalParams {
        fork_of: Some("s0".into()),
        fork_at: Some("e1".into()),
        new_session: Some("s2".into()),
        ..params()
    });
    for x in [
        "--resume=s0",
        "--fork-session",
        "--resume-session-at=e1",
        "--session-id=s2",
    ] {
        assert!(has(&copy, x), "{x} in {copy:?}");
    }
    // Haiku takes no effort.
    let haiku = args(&TerminalParams {
        model: "haiku".into(),
        ..params()
    });
    assert!(!has(&haiku, "--effort"));
}

#[test]
fn an_interface_with_remote_control_starts_with_its_name_in_claude_ai() {
    let a = args(&TerminalParams {
        remote: Some("demo · agent-1".into()),
        ..params()
    });
    // One argument, the name glued to the flag: a name is never taken for an option of its own.
    assert!(has(&a, "--remote-control=demo · agent-1"), "{a:?}");
    assert!(!has(&a, "--remote-control"), "{a:?}");
    let a = args(&TerminalParams {
        remote: Some("-demo · agent-1".into()),
        ..params()
    });
    assert!(has(&a, "--remote-control=-demo · agent-1"), "{a:?}");
    assert!(!has(&a, "-demo · agent-1"), "{a:?}");
    assert!(!args(&params())
        .iter()
        .any(|x| x.starts_with("--remote-control")));
}

#[test]
fn its_settings_are_its_hooks_and_escouades_server_its_file_or_its_tools_refused() {
    let p = TerminalParams {
        escouade: Some(Grant {
            url: "http://127.0.0.1:4121/mcp".into(),
            token: "jeton".into(),
        }),
        escouade_file: Some("/data/mcp/a1.json".into()),
        read_dirs: vec!["/data/files/a1".into()],
        append_prompt: Some("Le ticket\n  ESC-1".into()),
        ..params()
    };
    let l = launch(&p).launch;
    assert!(l
        .args
        .windows(2)
        .any(|w| w == ["--mcp-config", "/data/mcp/a1.json"]));
    assert!(has(&l.args, "--add-dir=/data/files/a1"));
    assert!(l
        .args
        .windows(2)
        .any(|w| w == ["--append-system-prompt", "Le ticket ESC-1"]));
    assert!(!l.args.iter().any(|a| a.contains("jeton")));
    assert_eq!(l.files.len(), 1);
    assert_eq!(l.files[0].path, "/data/tui/a1.json");
    let settings: Value = serde_json::from_str(&l.files[0].text).unwrap();
    for event in EVENTS {
        let hook = &settings["hooks"][event][0]["hooks"][0];
        assert_eq!(hook["type"], "http");
        assert_eq!(hook["url"], "http://127.0.0.1:4120/hook");
        assert_eq!(
            hook["headers"]["Authorization"],
            "Bearer $ESCOUADE_HOOK_TOKEN"
        );
    }
    let denied = launch(&params()).launch;
    assert!(denied
        .args
        .windows(2)
        .any(|w| w == ["--disallowedTools", "mcp__escouade"]));
}

#[test]
fn the_models_of_an_account_are_what_initialize_tells_by_a_process_given_no_turn() {
    let step = read(&AccountRead {
        read: "models".into(),
        ..AccountRead::default()
    });
    let l = step.launch.expect("a process");
    assert!(has(&l.args, "--no-session-persistence") && has(&l.args, "--strict-mcp-config"));
    assert!(step.writes[0].contains("\"subtype\":\"initialize\""));
    let answer = json!({ "type": "control_response", "response": {
        "subtype": "success", "request_id": "ccm_1",
        "response": { "models": [{ "value": "opus" }] },
    } });
    let done = lines(&AccountLines {
        read: "models".into(),
        lines: vec!["{\"type\":\"system\"}".into(), answer.to_string()],
        ..AccountLines::default()
    });
    assert_eq!(done.done, Some(json!([{ "value": "opus" }])));
    let gone = lines(&AccountLines {
        read: "models".into(),
        ended: true,
        ..AccountLines::default()
    });
    assert!(gone.error.is_some());
}
