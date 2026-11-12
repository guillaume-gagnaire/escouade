//! « Vue Copilot »: the launch of its interface (`terminal/launch`) and the reads of an account by a server of its own
//! (`account/read`), as the core's tests of its view had them before it left the core (ESC-359).

use super::*;

fn params() -> TerminalParams {
    TerminalParams {
        agent: "tui-a1".into(),
        cwd: "/x/repo".into(),
        hook_url: "http://127.0.0.1:4120/hook".into(),
        hook_token_var: "ESCOUADE_HOOK_TOKEN".into(),
        hook_url_var: "ESCOUADE_HOOK_URL".into(),
        private_file: "/data/tui/a1.json".into(),
        ..TerminalParams::default()
    }
}

fn pair(a: &[String], k: &str) -> Option<String> {
    a.windows(2).find(|w| w[0] == k).map(|w| w[1].clone())
}

#[test]
fn a_new_session_is_named_a_known_one_resumed_with_the_agents_options() {
    let p = TerminalParams {
        new_session: Some("u-1".into()),
        model: "gpt-6-luna".into(),
        effort: "high".into(),
        mode: "auto".into(),
        hook_command: Some("/opt/escouade/escouade-hook".into()),
        ..params()
    };
    let told = launch(&p);
    let new = &told.launch;
    assert_eq!(new.args[..2], ["--session-id", "u-1"]);
    for flag in [
        "--no-auto-update",
        "--disable-builtin-mcps",
        "--disallow-temp-dir",
        "--allow-tool=write",
        "--allow-tool=shell(git commit)",
    ] {
        assert!(new.args.iter().any(|a| a == flag), "{flag}: {:?}", new.args);
    }
    assert_eq!(pair(&new.args, "--model").as_deref(), Some("gpt-6-luna"));
    assert_eq!(
        pair(&new.args, "--reasoning-effort").as_deref(),
        Some("high")
    );
    assert_eq!(pair(&new.args, "--mode").as_deref(), Some("autopilot"));
    assert!(new.args.iter().any(|a| a.starts_with("--secret-env-vars=")));
    // Its plugin, beside its private file: the hooks of each event.
    assert_eq!(
        pair(&new.args, "--plugin-dir").as_deref(),
        Some("/data/tui/a1.copilot-plugin")
    );
    let paths: Vec<&str> = new.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "/data/tui/a1.copilot-plugin/plugin.json",
            "/data/tui/a1.copilot-plugin/hooks/hooks.json"
        ]
    );
    let hooks: Value = serde_json::from_str(&new.files[1].text).unwrap();
    assert_eq!(hooks["version"], 1);
    assert_eq!(
        hooks["hooks"]["agentStop"][0]["bash"],
        "'/opt/escouade/escouade-hook' agentStop"
    );
    assert_eq!(new.env[0].name, "ESCOUADE_HOOK_URL");
    // Escouade's server refused: turned off by name.
    assert_eq!(
        pair(&new.args, "--disable-mcp-server").as_deref(),
        Some("escouade")
    );
    // A model named: its account's list read before it starts.
    assert_eq!(
        told.before.as_ref().map(|b| b.read.as_str()),
        Some("listed")
    );
    assert!(told.warn.is_none());

    let resumed = launch(&TerminalParams {
        session: Some("s1".into()),
        model: "auto".into(),
        ..params()
    });
    assert_eq!(resumed.launch.args[..2], ["--resume", "s1"]);
    assert!(!resumed
        .launch
        .args
        .iter()
        .any(|a| a == "--model" || a == "--plugin-dir"));
    assert!(resumed.launch.files.is_empty() && resumed.before.is_none());
    // Without escouade-hook, the conversation says it will not follow the interface.
    assert!(resumed.warn.is_some());
}

#[test]
fn escouades_server_by_its_file_its_token_in_a_variable_and_an_own_account_its_own_sign_in() {
    let p = TerminalParams {
        escouade: Some(escouade_plugin_protocol::engine::Grant {
            url: "http://127.0.0.1:4121/mcp".into(),
            token: "jeton".into(),
        }),
        escouade_file: Some("/data/mcp/a1.json".into()),
        env_names: vec!["COPILOT_HOME".into()],
        read_dirs: vec!["/data/files/a1".into()],
        ..params()
    };
    let l = launch(&p).launch;
    assert_eq!(
        pair(&l.args, "--additional-mcp-config").as_deref(),
        Some("@/data/mcp/a1.json")
    );
    assert!(!l.args.iter().any(|a| a.contains("jeton")));
    assert!(l
        .env
        .iter()
        .any(|v| v.name == TOKEN_VAR && v.value == "jeton"));
    assert!(l.args.iter().any(|a| a == "--add-dir=/data/files/a1"));
    for var in INHERITED_TOKENS {
        assert!(
            l.env.iter().any(|v| v.name == var && v.value.is_empty()),
            "{var}"
        );
    }
}

#[test]
fn a_model_its_account_does_not_list_goes_as_auto_without_an_effort() {
    let args = |model: &str| -> Vec<String> {
        [
            "--resume",
            "s1",
            "--model",
            model,
            "--reasoning-effort",
            "high",
            "--allow-all",
        ]
        .map(String::from)
        .to_vec()
    };
    // A Free account lists none (a token) or `auto` alone (`copilot login`), tried (ESC-122).
    for listed in [vec![], vec!["auto".to_string()]] {
        assert_eq!(
            model_of_account(args("gpt-6-luna"), &listed),
            ["--resume", "s1", "--model", "auto", "--allow-all"]
        );
    }
    let paid = ["auto", "gpt-6-luna", "gpt-5-mini"].map(String::from);
    assert_eq!(
        model_of_account(args("gpt-6-luna"), &paid),
        args("gpt-6-luna")
    );
    assert_eq!(model_of_account(args("auto"), &[]), args("auto"));
    // Read by a server: its answer to `models.list` fits the arguments.
    let r = AccountRead {
        read: "listed".into(),
        input: json!({ "args": args("gpt-6-luna") }),
        home: Some("/h/free".into()),
        ..AccountRead::default()
    };
    let step = read(&r);
    let answer = AccountLines {
        read: "listed".into(),
        state: step.state,
        lines: vec![
            json!({ "jsonrpc": "2.0", "id": 2, "result": { "models": [{ "id": "auto" }] } })
                .to_string(),
        ],
        ..AccountLines::default()
    };
    assert_eq!(
        lines(&answer).done.unwrap()["args"],
        json!(["--resume", "s1", "--model", "auto", "--allow-all"])
    );
}

#[test]
fn an_agent_on_auto_goes_without_an_effort() {
    let a = launch(&TerminalParams {
        new_session: Some("u-1".into()),
        model: "auto".into(),
        effort: "medium".into(),
        ..params()
    })
    .launch
    .args;
    assert!(
        !a.iter()
            .any(|x| x == "--model" || x == "--reasoning-effort"),
        "{a:?}"
    );
}

#[test]
fn each_hook_runs_escouade_hook_with_its_event_quoted_for_its_shell() {
    let hooks = hooks_file(r"C:\Program Files\Esc'ouade\escouade-hook.exe");
    let entry = &hooks["hooks"]["notification"][0];
    assert_eq!(entry["type"], "command");
    assert_eq!(
        entry["powershell"],
        r"& 'C:\Program Files\Esc''ouade\escouade-hook.exe' notification"
    );
    assert_eq!(
        entry["bash"],
        r"'C:\Program Files\Esc'\''ouade\escouade-hook.exe' notification"
    );
    assert_eq!(hooks["hooks"].as_object().unwrap().len(), EVENTS.len());
    // A Windows path keeps its separator.
    assert_eq!(
        plugin_dir(r"C:\data\tui\a1.json"),
        r"C:\data\tui\a1.copilot-plugin"
    );
}

#[test]
fn its_quota_is_read_once_signed_in_as_the_account_and_a_session_copied_up_to_an_event() {
    let step = read(&AccountRead {
        read: "quota".into(),
        ..AccountRead::default()
    });
    assert!(step
        .launch
        .unwrap()
        .args
        .iter()
        .any(|a| a == "--disable-mcp-server"));
    let auth = AccountLines {
        read: "quota".into(),
        state: step.state.clone(),
        lines: vec![json!({ "id": 2, "result": { "isAuthenticated": false } }).to_string()],
        ..AccountLines::default()
    };
    assert_eq!(lines(&auth).done, Some(json!({ "notSignedIn": true })));

    let fork = read(&AccountRead {
        read: "fork".into(),
        input: json!({ "of": "s1", "at": "e2" }),
        ..AccountRead::default()
    });
    assert!(fork.writes[1].contains("sessions.readPersistedEvents"));
    let events = AccountLines {
        read: "fork".into(),
        state: fork.state.clone(),
        lines: vec![json!({ "id": 2, "result": { "events": [{ "id": "e1" }, { "id": "e2" }, { "id": "e3" }] } }).to_string()],
        ..AccountLines::default()
    };
    let next = lines(&events);
    assert!(
        next.writes[0].contains("\"toEventId\":\"e3\""),
        "{:?}",
        next.writes
    );
    let copied = AccountLines {
        read: "fork".into(),
        state: next.state,
        lines: vec![json!({ "id": 3, "result": { "sessionId": "s2" } }).to_string()],
        ..AccountLines::default()
    };
    assert_eq!(lines(&copied).done, Some(json!({ "session": "s2" })));
}

#[test]
fn each_choice_at_the_limit_of_its_credits_gives_copilot_its_action_in_either_language() {
    use crate::words::{limit_answer, limit_choices};
    let data = json!({ "requestId": "lim1", "usedAiCredits": 5.2, "maxAiCredits": 5 });
    for lang in ["fr", "en"] {
        let labels: Vec<String> = limit_choices(lang, &data)
            .into_iter()
            .map(|(l, _)| l)
            .collect();
        assert_eq!(labels.len(), 3);
        for (label, action) in labels.iter().zip(["add", "unset", "cancel"]) {
            assert_eq!(limit_answer(&data, label)["action"], action, "{lang}");
        }
        assert_eq!(limit_answer(&data, &labels[0])["additionalAiCredits"], 5.0);
    }
    assert_eq!(limit_answer(&data, "autre chose")["action"], "cancel");
}
