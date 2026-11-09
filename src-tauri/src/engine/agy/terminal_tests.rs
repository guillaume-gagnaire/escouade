use super::*;
use crate::engine::terminal::HookServer;
use crate::engine::Program;
use crate::model::AgentMeta;

const HOOK: &str = if cfg!(windows) {
    r"C:\Program Files\Escouade\escouade-hook.exe"
} else {
    "/opt/escouade/escouade-hook"
};

const TOKEN: &str = "jeton-secret-de-l-agent";

fn launch_of(
    meta: &AgentMeta,
    home: Option<&Path>,
    fence: &crate::fence::Fence,
    access: &Escouade,
) -> TerminalArgs {
    AGY_TERMINAL.launch(&TerminalLaunch {
        meta,
        session: None,
        remote: None,
        private_file: Path::new("/data/tui/a1.json"),
        home,
        hooks: HookServer {
            url: "http://127.0.0.1:4120/hook",
            token_var: "ESCOUADE_HOOK_TOKEN",
            url_var: "ESCOUADE_HOOK_URL",
            command: Some(Path::new(HOOK)),
        },
        access,
        fence,
        program: &Program::Native(PathBuf::new()),
    })
}

fn granted() -> Escouade {
    Escouade::Granted {
        url: "http://127.0.0.1:4121/mcp".into(),
        token: TOKEN.into(),
        file: None,
    }
}

fn pair(a: &[String], k: &str) -> Option<String> {
    a.windows(2).find(|w| w[0] == k).map(|w| w[1].clone())
}

fn env_of<'a>(t: &'a TerminalArgs, k: &str) -> Option<&'a str> {
    t.env.iter().rfind(|(n, _)| n == k).map(|(_, v)| v.as_str())
}

fn file_of(t: &TerminalArgs, path: &Path) -> Option<Value> {
    t.files
        .iter()
        .find(|(p, _)| p == path)
        .map(|(_, b)| serde_json::from_slice(b).unwrap())
}

const CWD: &str = if cfg!(windows) {
    r"C:\x\repo"
} else {
    "/x/repo"
};

/// A conversation of the agent's home with a transcript.
fn written(home: &Path, id: &str) {
    let t = transcript(home, id);
    std::fs::create_dir_all(t.parent().unwrap()).unwrap();
    std::fs::write(&t, "{}\n").unwrap();
}

#[test]
fn a_written_conversation_is_resumed_in_its_home_with_its_model_mode_trust_hooks_and_server() {
    let home = crate::paths::test_dir("esc272-agy-launch");
    written(&home, "c1");
    let meta = AgentMeta {
        session_id: Some("c1".into()),
        model: "gemini-3.8-flash".into(),
        effort: "low".into(),
        mode: "acceptEdits".into(),
        cwd: CWD.into(),
        ..Default::default()
    };
    let t = launch_of(
        &meta,
        Some(&home),
        &crate::fence::Fence::default(),
        &granted(),
    );
    assert_eq!(pair(&t.args, "--conversation").as_deref(), Some("c1"));
    assert_eq!(
        pair(&t.args, "--model").as_deref(),
        Some("gemini-3.8-flash")
    );
    assert_eq!(pair(&t.args, "--effort").as_deref(), Some("low"));
    // « Édits auto » is no mode of `agy`'s: its interface asks what no rule allows (ESC-376).
    assert!(pair(&t.args, "--mode").is_none(), "{:?}", t.args);
    // Its interface, not its stream.
    assert!(!t
        .args
        .iter()
        .any(|a| a.starts_with("-p") || a.contains("stream-json")));
    // The token of Escouade's server is in no argument nor variable: in its MCP config.
    assert!(!t.args.iter().any(|a| a.contains(TOKEN)), "{:?}", t.args);
    assert!(!t.env.iter().any(|(_, v)| v.contains(TOKEN)));
    let mcp = file_of(&t, &super::super::mcp_file(&home)).expect("its MCP config");
    assert_eq!(
        mcp["mcpServers"]["escouade"]["serverUrl"],
        "http://127.0.0.1:4121/mcp"
    );
    assert_eq!(
        mcp["mcpServers"]["escouade"]["headers"]["Authorization"],
        format!("Bearer {TOKEN}")
    );
    // Its home, its account's address kept off its screen, Escouade's hooks' server.
    let home_text = home.to_string_lossy().into_owned();
    assert_eq!(env_of(&t, "USERPROFILE"), Some(home_text.as_str()));
    assert_eq!(env_of(&t, "HOME"), Some(home_text.as_str()));
    assert_eq!(env_of(&t, "AGY_CLI_HIDE_ACCOUNT_INFO"), Some("1"));
    assert_eq!(
        env_of(&t, "ESCOUADE_HOOK_URL"),
        Some("http://127.0.0.1:4120/hook")
    );
    // Its settings: the rules of its mode, the trust of its folder.
    let settings = file_of(&t, &settings_file(&home)).expect("its settings");
    assert_eq!(settings["trustedWorkspaces"], json!([CWD]));
    let allow = settings["permissions"]["allow"].as_array().unwrap();
    assert!(allow.is_empty(), "{allow:?}");
    // Its first screens passed.
    let onboarding = file_of(&t, &onboarding_file(&home)).expect("its onboarding");
    assert_eq!(onboarding["onboardingComplete"], true);
    // Its hooks: escouade-hook <event> for each event Escouade reads.
    let hooks = file_of(&t, &hooks_file(&home)).expect("its hooks");
    let spec = &hooks["escouade"];
    for event in EVENTS {
        let handler = if event.ends_with("ToolUse") {
            assert_eq!(spec[event][0]["matcher"], "*");
            &spec[event][0]["hooks"][0]
        } else {
            &spec[event][0]
        };
        let command = handler["command"].as_str().unwrap();
        if cfg!(windows) {
            assert_eq!(command, format!("escouade-hook {event}"));
        } else {
            assert_eq!(command, format!("'{HOOK}' {event}"));
        }
    }
    if cfg!(windows) {
        let path = env_of(&t, "PATH").expect("its PATH");
        assert!(path.starts_with(r"C:\Program Files\Escouade"), "{path}");
    }
}

#[test]
fn a_conversation_without_transcript_is_not_given_its_settings_are_kept_and_a_fence_is_refused() {
    let home = crate::paths::test_dir("esc272-agy-unwritten");
    // What the user set in its settings, and a trust already there.
    let settings = settings_file(&home);
    std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
    std::fs::write(
        &settings,
        json!({ "theme": "dark", "trustedWorkspaces": [CWD] }).to_string(),
    )
    .unwrap();
    let meta = AgentMeta {
        session_id: Some("c-empty".into()),
        model: "gemini-3.8-flash-low".into(),
        effort: "high".into(),
        mode: "bypassPermissions".into(),
        cwd: CWD.into(),
        ..Default::default()
    };
    let private = if cfg!(windows) {
        r"C:\secret"
    } else {
        "/secret"
    };
    let fence = crate::fence::Fence::new(vec![PathBuf::from(private)]);
    let denied = Escouade::Denied { file: None };
    let t = launch_of(&meta, Some(&home), &fence, &denied);
    assert!(pair(&t.args, "--conversation").is_none(), "{:?}", t.args);
    // Its effort is part of the model's id.
    assert!(pair(&t.args, "--effort").is_none());
    // « Tout accepter » is its rules', not a mode.
    assert!(pair(&t.args, "--mode").is_none());
    let doc = file_of(&t, &settings).unwrap();
    assert_eq!(doc["theme"], "dark");
    assert_eq!(doc["trustedWorkspaces"], json!([CWD]));
    // With a fence, no rule `allow`: its interface asks (ESC-376).
    assert_eq!(doc["permissions"]["allow"], json!([]));
    let deny = doc["permissions"]["deny"].as_array().unwrap();
    assert!(
        deny.iter().any(|r| r.as_str().unwrap().contains("secret")),
        "{deny:?}"
    );
    // Escouade's server refused: not in its MCP config.
    let mcp = file_of(&t, &super::super::mcp_file(&home)).unwrap();
    assert!(mcp["mcpServers"].get("escouade").is_none(), "{mcp}");
}

#[test]
fn its_keys_its_lost_conversation_and_its_hooks_taken_away() {
    let env = vec![("PATH".to_string(), "x".to_string())];
    let masked = AGY_TERMINAL.masked_env(&env);
    assert_eq!(
        masked,
        [
            ("GEMINI_API_KEY".to_string(), String::new()),
            ("GOOGLE_API_KEY".to_string(), String::new())
        ]
    );
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

#[test]
fn its_rules_are_its_plugins_and_what_the_2_2_0_kept_of_always_is_erased_unread() {
    let home = crate::paths::test_dir("esc376-agy-view-rules");
    let always = always_file(&home);
    std::fs::write(&always, r#"["command(planted)","command(*)"]"#).unwrap();
    let settings = settings_file(&home);
    std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
    std::fs::write(
        &settings,
        r#"{"toolPermission":"auto","agentMode":"accept-edits","permissions":{"allow":["command(*)"]}}"#,
    )
    .unwrap();
    for (mode, fenced, allowed) in [
        ("default", false, false),
        ("acceptEdits", false, false),
        ("plan", false, false),
        ("bypassPermissions", true, false),
        ("bypassPermissions", false, true),
    ] {
        let meta = AgentMeta {
            mode: mode.into(),
            cwd: CWD.into(),
            ..Default::default()
        };
        let fence = if fenced {
            crate::fence::Fence::new(vec![PathBuf::from(if cfg!(windows) {
                r"C:\secret"
            } else {
                "/secret"
            })])
        } else {
            crate::fence::Fence::default()
        };
        let t = launch_of(&meta, Some(&home), &fence, &granted());
        assert!(!always.exists(), "{mode}: escouade-always.json erased");
        let doc = file_of(&t, &settings).unwrap();
        let allow = doc["permissions"]["allow"].as_array().unwrap();
        assert_eq!(!allow.is_empty(), allowed, "{mode} {fenced}: {allow:?}");
        assert!(!allow.contains(&json!("command(planted)")), "{allow:?}");
        assert!(doc.get("toolPermission").is_none() && doc.get("agentMode").is_none());
        assert!(pair(&t.args, "--mode").is_none_or(|m| m == "plan"));
    }
}

#[test]
fn its_temporary_folder_is_the_agents_own_as_its_plugin_has_it() {
    // `agy` 1.5.3 writes in its temporary folder with no rule nor question (ESC-388): not the user's `%TEMP%` (ESC-389).
    let home = crate::paths::test_dir("esc389-agy-temp");
    let meta = AgentMeta {
        mode: "default".into(),
        cwd: CWD.into(),
        ..Default::default()
    };
    let t = launch_of(
        &meta,
        Some(&home),
        &crate::fence::Fence::default(),
        &granted(),
    );
    let tmp = home.join("tmp").to_string_lossy().into_owned();
    for var in crate::engine::agy::TEMP_VARS {
        assert_eq!(env_of(&t, var), Some(tmp.as_str()), "{var}");
        assert_eq!(t.env.iter().filter(|(k, _)| k == var).count(), 1, "{var}");
    }
    // The folder is made with the launch's files.
    assert!(t.files.iter().any(|(p, _)| *p == temp_mark(&home)));
    assert_eq!(temp_mark(&home).parent(), Some(home.join("tmp").as_path()));
}
