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

#[test]
fn the_quota_of_an_account_is_what_get_usage_tells_once_initialize_is_answered() {
    let step = read(&AccountRead {
        read: "quota".into(),
        ..AccountRead::default()
    });
    // A process given no turn and no session, as for its models: Claude Code reads its own sign-in.
    let l = step.launch.expect("a process");
    assert!(has(&l.args, "--no-session-persistence") && has(&l.args, "--strict-mcp-config"));
    assert!(l.env.is_empty() && l.files.is_empty());
    assert_eq!(step.writes.len(), 1);
    assert!(step.writes[0].contains("\"subtype\":\"initialize\""));
    let answer = |id: &str, response: Value| {
        json!({ "type": "control_response", "response": {
            "subtype": "success", "request_id": id, "response": response,
        } })
        .to_string()
    };
    // `initialize` answered: `get_usage` is asked, nothing told yet.
    let asked = lines(&AccountLines {
        read: "quota".into(),
        lines: vec![
            "{\"type\":\"system\"}".into(),
            answer("ccm_1", json!({ "models": [] })),
        ],
        ..AccountLines::default()
    });
    assert_eq!((asked.done.as_ref(), asked.error.as_ref()), (None, None));
    assert_eq!(asked.writes.len(), 1);
    let usage: Value = serde_json::from_str(&asked.writes[0]).unwrap();
    assert_eq!(usage["request"]["subtype"], "get_usage");
    assert_eq!(usage["request_id"], "ccm_2");
    // Its rate_limits as told, `limits` included or not: the core tells a reading from what the process saw.
    let windows =
        json!({ "limits": null, "five_hour": { "utilization": 40.0, "resets_at": null } });
    let done = lines(&AccountLines {
        read: "quota".into(),
        state: asked.state.clone(),
        lines: vec![answer("ccm_2", json!({ "rate_limits": windows.clone() }))],
        ..AccountLines::default()
    });
    assert_eq!(done.done, Some(windows));
    // Nothing to tell, a refusal, or the process gone: the read fails, the last values stand.
    let none = lines(&AccountLines {
        read: "quota".into(),
        lines: vec![answer("ccm_2", json!({ "rate_limits": null }))],
        ..AccountLines::default()
    });
    assert!(none.error.is_some());
    let refused = json!({ "type": "control_response", "response": {
        "subtype": "error", "request_id": "ccm_2", "error": "Not logged in",
    } });
    let refused = lines(&AccountLines {
        read: "quota".into(),
        lines: vec![refused.to_string()],
        ..AccountLines::default()
    });
    assert_eq!(refused.error.as_deref(), Some("get_usage: Not logged in"));
    let gone = lines(&AccountLines {
        read: "quota".into(),
        ended: true,
        ..AccountLines::default()
    });
    assert!(gone.error.is_some());
    // A read of its models takes no `get_usage` answer for its own.
    let models = lines(&AccountLines {
        read: "models".into(),
        lines: vec![answer("ccm_2", json!({ "rate_limits": {} }))],
        ..AccountLines::default()
    });
    assert_eq!((models.done, models.error), (None, None));
}

/// ESC-553: the experimental features go in the `env` of the interface's private settings, where Claude Code reads Code
/// Mode (« Code Mode (experimental) is on » with it there, 2.1.329), never on its stream's launch; what is not passed is
/// said.
#[test]
fn the_experimental_features_go_in_the_private_settings_of_the_interface_alone() {
    use crate::experimental::{key, Context, AGENT_TEAMS, CODE_MODE};
    let settings = json!({ key::CODE_MODE: true, key::AGENT_TEAMS: true })
        .as_object()
        .cloned()
        .unwrap();
    let c = Context {
        settings,
        version: Some("2.1.329".into()),
        ..Context::default()
    };
    let t = launch_with(&params(), &c);
    let file: Value = serde_json::from_str(&t.launch.files[0].text).unwrap();
    assert_eq!(file["env"], json!({ CODE_MODE: "1", AGENT_TEAMS: "1" }));
    assert!(file["hooks"]["PreToolUse"].is_array());
    // Never a variable of the process's, nor an argument.
    assert!(t.launch.env.is_empty());
    assert!(!t.launch.args.iter().any(|a| a.contains("EXPERIMENTAL")));
    assert_eq!(t.warn, None);
    // Without the settings (a core of before), nothing.
    let before = launch(&params());
    let file: Value = serde_json::from_str(&before.launch.files[0].text).unwrap();
    assert!(file.get("env").is_none());
    // Refused, its remote control kept, and the reason said.
    let remote = launch_with(
        &TerminalParams {
            remote: Some("demo".into()),
            lang: "en".into(),
            ..params()
        },
        &c,
    );
    let file: Value = serde_json::from_str(&remote.launch.files[0].text).unwrap();
    assert_eq!(file["env"], json!({ AGENT_TEAMS: "1" }));
    assert!(has(&remote.launch.args, "--remote-control=demo"));
    assert!(remote
        .warn
        .as_deref()
        .is_some_and(|w| w.starts_with("Code Mode not passed to the interface: Remote Control")));
}

/// ESC-553: the plugin reads what the core tells beyond the published protocol from the parameters of `terminal/launch`.
#[test]
fn the_context_is_read_from_the_parameters_of_terminal_launch() {
    let mut v = serde_json::to_value(params()).unwrap();
    v["settings"] = json!({ "experimental.codeMode": true });
    v["version"] = json!("2.1.329");
    v["gateway"] = json!("CLAUDE_CODE_USE_BEDROCK");
    v["terminalOnly"] = json!(true);
    let c: crate::experimental::Context = serde_json::from_value(v).unwrap();
    assert_eq!(c.settings["experimental.codeMode"], true);
    assert_eq!(c.version.as_deref(), Some("2.1.329"));
    assert_eq!(c.gateway.as_deref(), Some("CLAUDE_CODE_USE_BEDROCK"));
    assert!(c.terminal_only);
}
