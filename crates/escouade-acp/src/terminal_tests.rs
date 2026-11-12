//! The launch of the interfaces of Cursor CLI, Mistral Vibe and Gemini CLI (`terminal/launch`), as the core's tests of
//! their views had it before it left the core (ESC-359).

use super::*;
use escouade_plugin_protocol::engine::Grant;

const HOOK_WIN: &str = r"C:\Program Files\Escouade\escouade-hook.exe";
const HOOK_UNIX: &str = "/opt/escouade/escouade-hook";

fn params() -> TerminalParams {
    TerminalParams {
        agent: "tui-a1".into(),
        cwd: "/x/repo".into(),
        hook_url: "http://127.0.0.1:4120/hook".into(),
        hook_token_var: "ESCOUADE_HOOK_TOKEN".into(),
        hook_url_var: "ESCOUADE_HOOK_URL".into(),
        hook_command: Some(HOOK_UNIX.into()),
        private_file: "/data/tui/a1.json".into(),
        ..TerminalParams::default()
    }
}

fn pair(a: &[String], k: &str) -> Option<String> {
    a.windows(2).find(|w| w[0] == k).map(|w| w[1].clone())
}

fn env_of<'a>(l: &'a Launch, k: &str) -> Option<&'a str> {
    l.env
        .iter()
        .rfind(|v| v.name == k)
        .map(|v| v.value.as_str())
}

fn file_of<'a>(l: &'a Launch, end: &str) -> Option<&'a str> {
    l.files
        .iter()
        .find(|f| f.path.replace('\\', "/").ends_with(end))
        .map(|f| f.text.as_str())
}

#[test]
fn vibe_resumes_a_session_its_store_has_in_the_agents_mode_with_its_hooks_and_no_telemetry() {
    let p = TerminalParams {
        session: Some("s1".into()),
        resumes: true,
        model: "mistral-medium-3.5".into(),
        mode: "acceptEdits".into(),
        home: Some("/h/vibe".into()),
        read_dirs: vec!["/data/files/a1".into()],
        ..params()
    };
    let l = vibe(&p);
    assert_eq!(pair(&l.args, "--resume").as_deref(), Some("s1"));
    assert_eq!(pair(&l.args, "--agent").as_deref(), Some("accept-edits"));
    assert!(l.args.iter().any(|a| a == "--add-dir=/data/files/a1"));
    assert_eq!(env_of(&l, "VIBE_ENABLE_TELEMETRY"), Some("false"));
    assert_eq!(env_of(&l, "VIBE_ACTIVE_MODEL"), Some("mistral-medium-3.5"));
    assert_eq!(
        env_of(&l, "ESCOUADE_HOOK_URL"),
        Some("http://127.0.0.1:4120/hook")
    );
    let hooks = file_of(&l, "/h/vibe/hooks.toml").expect("its hooks");
    for kind in ["pre_tool", "post_tool", "post_agent"] {
        assert!(hooks.contains(&format!("type = \"{kind}\"")), "{hooks}");
    }
    assert!(
        hooks.contains(&format!("command = \"'{HOOK_UNIX}'\"")),
        "{hooks}"
    );
    // Under Windows by its name, its folder first on the PATH.
    let win = vibe(&TerminalParams {
        hook_command: Some(HOOK_WIN.into()),
        path: Some(r"C:\Windows".into()),
        ..p.clone()
    });
    assert!(file_of(&win, "hooks.toml")
        .unwrap()
        .contains("command = \"escouade-hook\""));
    assert_eq!(
        env_of(&win, "PATH"),
        Some(r"C:\Program Files\Escouade;C:\Windows")
    );
}

#[test]
fn vibe_does_not_resume_a_session_it_does_not_have_and_bypass_stays_ask_with_a_fence() {
    let p = TerminalParams {
        session: Some("s-empty".into()),
        mode: "bypassPermissions".into(),
        ..params()
    };
    let l = vibe(&p);
    assert!(pair(&l.args, "--resume").is_none(), "{:?}", l.args);
    assert_eq!(pair(&l.args, "--agent").as_deref(), Some("auto-approve"));
    let fenced = vibe(&TerminalParams {
        fence: vec!["/secret".into()],
        ..p.clone()
    });
    assert_eq!(pair(&fenced.args, "--agent").as_deref(), Some("ask"));
    let plan = vibe(&TerminalParams {
        mode: "plan".into(),
        ..p
    });
    assert_eq!(pair(&plan.args, "--agent").as_deref(), Some("ask"));
}

#[test]
fn vibe_gets_escouades_server_by_its_variables_never_its_command_line() {
    let p = TerminalParams {
        escouade: Some(Grant {
            url: "http://127.0.0.1:4121/mcp".into(),
            token: "jeton-secret".into(),
        }),
        ..params()
    };
    let l = vibe(&p);
    assert!(!l.args.iter().any(|a| a.contains("jeton-secret")));
    let servers: Value = serde_json::from_str(env_of(&l, "VIBE_MCP_SERVERS").unwrap()).unwrap();
    assert_eq!(servers[0]["name"], "escouade");
    assert_eq!(servers[0]["transport"], "http");
    assert_eq!(servers[0]["url"], "http://127.0.0.1:4121/mcp");
    assert_eq!(servers[0]["api_key_env"], VIBE_TOKEN_VAR);
    assert!(!servers.to_string().contains("jeton-secret"));
    assert_eq!(env_of(&l, VIBE_TOKEN_VAR), Some("jeton-secret"));
    let denied = vibe(&params());
    assert!(env_of(&denied, "VIBE_MCP_SERVERS").is_none());
}

#[test]
fn gemini_resumes_a_session_it_has_and_starts_one_under_its_id_otherwise() {
    let p = TerminalParams {
        session: Some("abc12345-x".into()),
        resumes: true,
        model: "gemini-3.8-flash".into(),
        mode: "acceptEdits".into(),
        hook_command: None,
        ..params()
    };
    assert_eq!(
        gemini(&p).args,
        [
            "--resume",
            "abc12345-x",
            "--model",
            "gemini-3.8-flash",
            "--approval-mode",
            "auto_edit"
        ]
    );
    let new = TerminalParams {
        session: None,
        new_session: Some("abc12345-x".into()),
        resumes: false,
        model: "auto".into(),
        mode: "bypassPermissions".into(),
        read_dirs: vec!["/data/files/a1".into()],
        ..p.clone()
    };
    assert_eq!(
        gemini(&new).args,
        [
            "--session-id",
            "abc12345-x",
            "--approval-mode",
            "yolo",
            "--include-directories",
            "/data/files/a1"
        ]
    );
    let plan = TerminalParams {
        session: None,
        mode: "plan".into(),
        model: String::new(),
        ..p
    };
    assert_eq!(gemini(&plan).args, ["--approval-mode", "plan"]);
    assert_eq!(
        env_of(&gemini(&plan), "GEMINI_CLI_TRUST_WORKSPACE"),
        Some("true")
    );
}

#[test]
fn gemini_settings_get_its_hooks_and_escouades_server_the_users_kept() {
    let before = r#"{"ui":{"theme":"x"},"mcpServers":{"mine":{"command":"m"}}}"#;
    let doc = gemini_settings(
        before,
        Some(HOOK_UNIX),
        Some(("http://127.0.0.1:4100/mcp", "secret")),
        &["ESCOUADE_HOOK_URL", "ESCOUADE_HOOK_TOKEN"],
    );
    assert_eq!(doc["ui"]["theme"], "x");
    assert_eq!(doc["hooksConfig"]["enabled"], true);
    for event in GEMINI_EVENTS {
        let line = doc["hooks"][event][0]["hooks"][0]["command"]
            .as_str()
            .unwrap();
        assert!(
            line.contains("escouade-hook") && line.ends_with(event),
            "{line}"
        );
        assert_eq!(doc["hooks"][event][0]["hooks"][0]["timeout"], HOOK_LIMIT_MS);
    }
    assert_eq!(
        doc["mcpServers"]["escouade"]["headers"]["Authorization"],
        "Bearer secret"
    );
    assert_eq!(doc["mcpServers"]["mine"]["command"], "m");
    assert_eq!(
        doc["security"]["environmentVariableRedaction"]["allowed"],
        json!(["ESCOUADE_HOOK_URL", "ESCOUADE_HOOK_TOKEN"])
    );
    let denied = gemini_settings(&doc.to_string(), None, None, &[]);
    assert!(denied["mcpServers"]["escouade"].is_null());
    // In the agent's home, with its hooks.
    let l = gemini(&TerminalParams {
        home: Some("/h/gemini".into()),
        ..params()
    });
    let text = file_of(&l, "/h/gemini/.gemini/settings.json").expect("its settings");
    assert!(text.contains("escouade-hook"), "{text}");
}

#[test]
fn cursor_resumes_by_its_id_with_the_agents_model_and_mode_and_no_update() {
    let p = TerminalParams {
        session: Some("s1".into()),
        model: "claude-opus-5-5[context=300k,effort=medium,fast=false]".into(),
        mode: "plan".into(),
        ..params()
    };
    let l = cursor(&p);
    assert_eq!(l.args[0], "--disable-auto-update");
    assert_eq!(pair(&l.args, "--resume").as_deref(), Some("s1"));
    assert_eq!(pair(&l.args, "--model").as_deref(), Some("claude-opus-5-5"));
    assert_eq!(pair(&l.args, "--mode").as_deref(), Some("plan"));
    assert!(!l.args.iter().any(|x| x == "--force"));
    // Never Git Bash's shell, nothing written, no folder added to its workspace.
    assert_eq!(env_of(&l, "MSYSTEM"), Some(""));
    assert_eq!(env_of(&l, "EXEPATH"), Some(""));
    assert!(l.files.is_empty());
}

#[test]
fn cursor_new_agent_on_auto_in_everything_accepted_starts_the_session_escouade_names() {
    let p = TerminalParams {
        new_session: Some("u-1".into()),
        model: crate::cursor::AUTO.into(),
        mode: "bypassPermissions".into(),
        ..params()
    };
    let l = cursor(&p);
    assert_eq!(pair(&l.args, "--resume").as_deref(), Some("u-1"));
    assert_eq!(pair(&l.args, "--model").as_deref(), Some("auto"));
    assert!(l.args.iter().any(|x| x == "--force"));
    for mode in ["default", "acceptEdits"] {
        let a = cursor(&TerminalParams {
            mode: mode.into(),
            ..params()
        })
        .args;
        assert!(
            !a.iter()
                .any(|x| x == "--force" || x == "--mode" || x == "--model"),
            "{a:?}"
        );
    }
}
