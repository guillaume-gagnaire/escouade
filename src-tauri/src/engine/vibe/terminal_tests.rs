use super::*;
use crate::engine::terminal::HookServer;
use crate::engine::vibe::VIBE;
use crate::model::AgentMeta;

const HOOK: &str = if cfg!(windows) {
    r"C:\Program Files\Escouade\escouade-hook.exe"
} else {
    "/opt/escouade/escouade-hook"
};

fn launch_of(meta: &AgentMeta, home: Option<&Path>, fence: &crate::fence::Fence) -> TerminalArgs {
    let access = Escouade::Denied { file: None };
    VIBE_TERMINAL.launch(&TerminalLaunch {
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
        access: &access,
        fence,
        program: &Program::Native(PathBuf::new()),
    })
}

fn pair(a: &[String], k: &str) -> Option<String> {
    a.windows(2).find(|w| w[0] == k).map(|w| w[1].clone())
}

fn env_of<'a>(t: &'a TerminalArgs, k: &str) -> Option<&'a str> {
    t.env.iter().rfind(|(n, _)| n == k).map(|(_, v)| v.as_str())
}

/// A session of the agent's `VIBE_HOME` with a segment of its journal.
fn written_session(home: &Path, session: &str) {
    let journal = session_dir(home, session).join("journal");
    std::fs::create_dir_all(&journal).unwrap();
    std::fs::write(journal.join("0000000000000001.jsonl"), "{}\n").unwrap();
}

#[test]
fn a_written_session_is_resumed_in_the_agents_mode_with_its_hooks_trust_and_no_telemetry() {
    let home = crate::paths::test_dir("esc268-vibe-launch");
    written_session(&home, "s1");
    let meta = AgentMeta {
        session_id: Some("s1".into()),
        model: "mistral-medium-3.5".into(),
        mode: "acceptEdits".into(),
        cwd: if cfg!(windows) {
            r"C:\x\repo"
        } else {
            "/x/repo"
        }
        .into(),
        ..Default::default()
    };
    let t = launch_of(&meta, Some(&home), &crate::fence::Fence::default());
    assert_eq!(pair(&t.args, "--resume").as_deref(), Some("s1"));
    assert_eq!(pair(&t.args, "--agent").as_deref(), Some("accept-edits"));
    assert_eq!(env_of(&t, "VIBE_ENABLE_TELEMETRY"), Some("false"));
    assert_eq!(env_of(&t, "VIBE_ACTIVE_MODEL"), Some("mistral-medium-3.5"));
    assert_eq!(
        env_of(&t, "ESCOUADE_HOOK_URL"),
        Some("http://127.0.0.1:4120/hook")
    );
    // Its hooks and the trust of its folder, in the agent's VIBE_HOME.
    let file = |name: &str| {
        t.files
            .iter()
            .find(|(p, _)| *p == home.join(name))
            .map(|(_, b)| String::from_utf8(b.clone()).unwrap())
    };
    let hooks = file(HOOKS).expect("its hooks");
    let parsed: toml_edit::DocumentMut = hooks.parse().unwrap();
    let kinds: Vec<&str> = parsed["hooks"]
        .as_array_of_tables()
        .unwrap()
        .iter()
        .map(|h| h["type"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["pre_tool", "post_tool", "post_agent"]);
    let command = parsed["hooks"]
        .as_array_of_tables()
        .unwrap()
        .get(0)
        .unwrap()["command"]
        .as_str()
        .unwrap()
        .to_string();
    if cfg!(windows) {
        // By its name, which Git Bash and PowerShell both find: its folder first on the PATH.
        assert_eq!(command, "escouade-hook");
        let path = env_of(&t, "PATH").expect("its PATH");
        assert!(path.starts_with(r"C:\Program Files\Escouade"), "{path}");
        assert_eq!(
            env_of(&t, crate::engine::terminal::NO_CURRENT_DIR),
            Some("1")
        );
    } else {
        assert_eq!(command, format!("'{HOOK}'"));
    }
    let trust = file(TRUSTED_FOLDERS).expect("the trust of its folder");
    assert!(trust.contains("repo"), "{trust}");
    // No private project: its config is not written.
    assert!(file("config.toml").is_none());
}

#[test]
fn a_session_its_store_does_not_have_is_not_given_and_bypass_stays_ask_with_a_fence() {
    let home = crate::paths::test_dir("esc268-vibe-unwritten");
    // A session ACP opened and never wrote: not in the store, its interface would say « Session
    // not found ».
    let meta = AgentMeta {
        session_id: Some("s-empty".into()),
        mode: "bypassPermissions".into(),
        cwd: "/x/repo".into(),
        ..Default::default()
    };
    let t = launch_of(&meta, Some(&home), &crate::fence::Fence::default());
    assert!(pair(&t.args, "--resume").is_none(), "{:?}", t.args);
    assert_eq!(pair(&t.args, "--agent").as_deref(), Some("auto-approve"));
    // A private project's paths: refused to its file tools, and never auto-approve, which passes
    // over them.
    let private = if cfg!(windows) {
        r"C:\secret"
    } else {
        "/secret"
    };
    let fence = crate::fence::Fence::new(vec![PathBuf::from(private)]);
    let t = launch_of(&meta, Some(&home), &fence);
    assert_eq!(pair(&t.args, "--agent").as_deref(), Some("ask"));
    let config = t
        .files
        .iter()
        .find(|(p, _)| *p == home.join("config.toml"))
        .map(|(_, b)| String::from_utf8(b.clone()).unwrap())
        .expect("the refusals in its config");
    assert!(
        config.contains("[tools.read_file]") && config.contains("secret"),
        "{config}"
    );
    // The plan of Escouade asks, as « Demander ».
    let plan = AgentMeta {
        mode: "plan".into(),
        ..meta.clone()
    };
    let t = launch_of(&plan, Some(&home), &crate::fence::Fence::default());
    assert_eq!(pair(&t.args, "--agent").as_deref(), Some("ask"));
}

#[test]
fn escouades_server_goes_by_its_variables_never_its_command_line() {
    let granted = Escouade::Granted {
        url: "http://127.0.0.1:4121/mcp".into(),
        token: "jeton-secret".into(),
        file: None,
    };
    let launch = VIBE_TERMINAL.escouade(&VIBE, &granted, Path::new("/home"));
    assert!(launch.args.is_empty());
    assert!(!launch.args.iter().any(|a| a.contains("jeton-secret")));
    let servers: Value = serde_json::from_str(
        &launch
            .env
            .iter()
            .find(|(k, _)| k == "VIBE_MCP_SERVERS")
            .unwrap()
            .1,
    )
    .unwrap();
    assert_eq!(servers[0]["name"], "escouade");
    assert_eq!(servers[0]["transport"], "http");
    assert_eq!(servers[0]["url"], "http://127.0.0.1:4121/mcp");
    assert_eq!(servers[0]["api_key_env"], MCP_TOKEN_VAR);
    assert!(!servers.to_string().contains("jeton-secret"));
    assert!(launch
        .env
        .iter()
        .any(|(k, v)| k == MCP_TOKEN_VAR && v == "jeton-secret"));
    let denied = VIBE_TERMINAL.escouade(&VIBE, &Escouade::Denied { file: None }, Path::new("/h"));
    assert!(denied.args.is_empty() && denied.env.is_empty());
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
