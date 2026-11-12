//! « Vue Codex »: the launch of its interface (`terminal/launch`) and the reads of an account by an app-server of its own
//! (`account/read`), as the core's tests had them before they left the core (ESC-359).

use super::*;
use escouade_plugin_protocol::engine::Grant;

fn params() -> TerminalParams {
    TerminalParams {
        agent: "tui-a1".into(),
        cwd: "/x/repo".into(),
        model: "gpt-6-luna".into(),
        effort: "high".into(),
        mode: "default".into(),
        hook_url: "http://127.0.0.1:4120/hook".into(),
        hook_token_var: "ESCOUADE_HOOK_TOKEN".into(),
        hook_url_var: "ESCOUADE_HOOK_URL".into(),
        hook_command: Some("/opt/escouade/escouade-hook".into()),
        lang: "fr".into(),
        ..TerminalParams::default()
    }
}

fn has(a: &[String], x: &str) -> bool {
    a.iter().any(|y| y == x)
}

#[test]
fn a_session_is_resumed_or_copied_with_its_model_effort_mode_and_hooks() {
    let resumed = args(&TerminalParams {
        session: Some("s1".into()),
        append_prompt: Some("ne pas redire".into()),
        ..params()
    });
    assert_eq!(resumed[..2], ["resume", "s1"]);
    assert!(has(&resumed, "--no-daemon"));
    assert!(resumed.windows(2).any(|w| w == ["-m", "gpt-6-luna"]));
    assert!(has(&resumed, "model_reasoning_effort=\"high\""));
    assert!(resumed
        .windows(4)
        .any(|w| w == ["-a", "on-request", "-s", "workspace-write"]));
    assert!(has(&resumed, "tools.update_plan.enabled=true"));
    // A resumed session keeps the instructions it had.
    assert!(!resumed
        .iter()
        .any(|a| a.starts_with("developer_instructions")));
    let new = args(&TerminalParams {
        append_prompt: Some("le ticket\nESC-1".into()),
        ..params()
    });
    assert!(has(&new, "developer_instructions=\"le ticket ESC-1\""));
    let copy = args(&TerminalParams {
        fork_of: Some("s0".into()),
        ..params()
    });
    assert_eq!(copy[..2], ["fork", "s0"]);
    let hooks = hooks_among(&new);
    assert_eq!(hooks.len(), EVENTS.len() * 2);
    assert!(
        hooks[1].starts_with("hooks.SessionStart=[{hooks=[{type=\"command\",command="),
        "{}",
        hooks[1]
    );
    // Its modes, a discussion read only, a fence by Escouade's profile in place of the sandbox.
    assert!(has(
        &args(&TerminalParams {
            mode: "auto".into(),
            ..params()
        }),
        "--approve-for-me"
    ));
    let bypass = args(&TerminalParams {
        mode: "bypassPermissions".into(),
        ..params()
    });
    assert!(has(&bypass, "--dangerously-bypass-approvals-and-sandbox"));
    let read_only = args(&TerminalParams {
        read_only: true,
        ..params()
    });
    assert!(read_only.windows(2).any(|w| w == ["-s", "read-only"]));
    let fenced = args(&TerminalParams {
        fence: vec!["C:\\secret".into()],
        ..params()
    });
    assert!(!has(&fenced, "-s"));
    assert!(
        fenced.iter().any(|a| a.contains("\"C:/secret\"=\"deny\"")),
        "{fenced:?}"
    );
}

#[test]
fn escouades_server_by_its_options_and_the_trust_of_its_hooks_read_before_it_starts() {
    let granted = launch(&TerminalParams {
        escouade: Some(Grant {
            url: "http://127.0.0.1:4121/mcp".into(),
            token: "jeton".into(),
        }),
        ..params()
    });
    let l = &granted.launch;
    assert!(has(
        &l.args,
        "mcp_servers.escouade.url=http://127.0.0.1:4121/mcp"
    ));
    assert!(!l.args.iter().any(|a| a.contains("jeton")));
    assert!(l
        .env
        .iter()
        .any(|v| v.name == TOKEN_VAR && v.value == "jeton"));
    assert!(l.env.iter().any(|v| v.name == "ESCOUADE_HOOK_URL"));
    let before = granted.before.expect("the trust of its hooks");
    assert_eq!(before.read, "hooks");
    assert!(before.input["failed"].as_str().unwrap().contains("{error}"));
    // Refused, with no entry of the user's: nothing passed (an entry of `enabled` alone keeps Codex from starting).
    let denied = launch(&params()).launch;
    assert!(!denied
        .args
        .iter()
        .any(|a| a.contains("mcp_servers.escouade")));
    // Without escouade-hook: said, nothing to trust.
    let unhooked = launch(&TerminalParams {
        hook_command: None,
        ..params()
    });
    assert!(unhooked.before.is_none() && unhooked.warn.is_some());

    let step = read(&AccountRead {
        read: "hooks".into(),
        input: json!({ "args": new_args(), "cwd": "/x/repo" }),
        ..AccountRead::default()
    });
    let server = step.launch.clone().unwrap();
    assert_eq!(server.args[0], "app-server");
    assert!(server.args.iter().any(|a| a.starts_with("hooks.Stop=")));
    assert!(step.writes[2].contains("\"hooks/list\""));
    let listed = json!({ "id": 2, "result": { "data": [{ "hooks": [
        { "source": "sessionFlags", "key": "k1", "currentHash": "h1" },
        { "source": "user", "trustStatus": "untrusted", "key": "k2", "currentHash": "h2" },
    ] }] } });
    let done = lines(&AccountLines {
        read: "hooks".into(),
        state: step.state,
        lines: vec![listed.to_string()],
        ..AccountLines::default()
    })
    .done
    .unwrap();
    let all = done["args"].as_array().unwrap();
    assert_eq!(
        all.last().unwrap(),
        "hooks.state={\"k1\"={trusted_hash=\"h1\"}}"
    );
}

fn new_args() -> Vec<String> {
    args(&params())
}

#[test]
fn the_models_are_every_page_and_the_quota_its_windows() {
    let step = read(&AccountRead {
        read: "models".into(),
        ..AccountRead::default()
    });
    assert_eq!(step.launch.as_ref().unwrap().args, ["app-server"]);
    let page = |id: u64, model: &str, next: Option<&str>| {
        json!({ "id": id, "result": { "data": [{ "id": model, "displayName": model }], "nextCursor": next } })
            .to_string()
    };
    let first = lines(&AccountLines {
        read: "models".into(),
        state: step.state,
        lines: vec![page(2, "one", Some("c2"))],
        ..AccountLines::default()
    });
    assert!(first.writes[0].contains("\"cursor\":\"c2\""));
    let last = lines(&AccountLines {
        read: "models".into(),
        state: first.state,
        lines: vec![page(3, "two", None)],
        ..AccountLines::default()
    });
    let models = last.done.unwrap();
    assert_eq!(models[0]["id"], "one");
    assert_eq!(models[1]["id"], "two");

    let quota = lines(&AccountLines {
        read: "quota".into(),
        lines: vec![json!({ "id": 2, "result": { "rateLimits": { "primary": {
            "usedPercent": 12.5, "windowDurationMins": 300, "resetsAt": 1000
        } } } })
        .to_string()],
        ..AccountLines::default()
    });
    assert_eq!(
        quota.done,
        Some(json!({ "meters": [{ "minutes": 300, "pct": 12.5, "resetsAt": 1_000_000 }] }))
    );
}
