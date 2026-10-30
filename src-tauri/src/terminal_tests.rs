//! « Vue <harness> » behind its interface (`engine::terminal`): what each harness declares, and
//! `escouade-hook`, the command a harness that runs its hooks as commands is given, posting to
//! Escouade's own hooks' server.

use crate::core_tests::{agent_on, harness, plain_text, second_account, Harness};
use crate::engine::terminal::{bracketed, hook_command, HookServer, TerminalLaunch};
use crate::engine::{self, Engine, Escouade};
use crate::model::*;
use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn a_harness_has_a_terminal_view_exactly_when_its_caps_say_so() {
    for id in [
        EngineId::CLAUDE,
        EngineId::CODEX,
        EngineId::COPILOT,
        EngineId::CURSOR,
        EngineId::OPENCODE,
    ] {
        let e = engine::by_id(id);
        // In the process's place, or joined to it (OpenCode), never both.
        assert_eq!(
            e.terminal().is_some() || e.attaches(),
            e.caps().terminal_view,
            "{id:?}"
        );
        assert!(!(e.terminal().is_some() && e.attaches()), "{id:?}");
    }
    assert!(engine::by_id(EngineId::OPENCODE).attaches());
    let minimal = &crate::engine::minimal::MINIMAL;
    assert!(minimal.terminal().is_none() && !minimal.attaches() && !minimal.caps().terminal_view);
}

#[test]
fn claude_codes_view_pastes_sends_and_interrupts_as_before() {
    let view = engine::by_id(EngineId::CLAUDE).terminal().unwrap();
    assert_eq!(view.shell(), "claude");
    // Line breaks pasted as a terminal pastes them (xterm.js too): \r. Claude Code drops a \n
    // pasted in its interface under Windows (ESC-12, its real one).
    assert_eq!(
        view.paste("a\nb\r\nc\x1bd"),
        b"\x1b[200~a\rb\rcd\x1b[201~".to_vec()
    );
    assert_eq!(bracketed("x"), view.paste("x"));
    assert_eq!(view.submit(), b"\r");
    assert_eq!(view.interrupt_key(), b"\x1b");
    assert_eq!(view.quit_key(), b"\x03");
    assert!(view.image_types().contains(&"image/png"));
    assert!(view.names_its_session());
    assert!(view.says_session_lost(b"No conversation found with session ID: s1"));
    assert!(!view.says_session_lost(b"Welcome to Claude Code"));
    assert_eq!(
        view.read_dir_args(std::path::Path::new("/d")),
        vec!["--add-dir=/d"]
    );
}

#[test]
fn claude_codes_launch_is_its_arguments_and_its_settings_file() {
    let view = engine::by_id(EngineId::CLAUDE).terminal().unwrap();
    let meta = AgentMeta {
        model: "sonnet".into(),
        effort: "high".into(),
        mode: "default".into(),
        ..Default::default()
    };
    let file = std::path::Path::new("/data/tui/a1.json");
    let access = Escouade::Denied { file: None };
    let launch = view.launch(&TerminalLaunch {
        meta: &meta,
        session: Some("s-new"),
        remote: None,
        private_file: file,
        home: None,
        hooks: HookServer {
            url: "http://127.0.0.1:4120/hook",
            token_var: "ESCOUADE_HOOK_TOKEN",
            url_var: "ESCOUADE_HOOK_URL",
            command: None,
        },
        access: &access,
        fence: &crate::fence::Fence::default(),
        program: &engine::Program::Native(std::path::PathBuf::new()),
    });
    assert_eq!(
        launch.args,
        crate::tui::tui_args(&meta, Some("s-new"), None, file)
    );
    let settings: serde_json::Value = serde_json::from_slice(&launch.file.unwrap()).unwrap();
    assert_eq!(
        settings,
        crate::tui::tui_settings(4120, engine::by_id(EngineId::CLAUDE), &access)
    );
    // Its hooks are HTTP calls: nothing in its environment for a command.
    assert!(launch.env.is_empty());
}

/// An agent on a Claude account of its own, in the Claude Code view, its interface up.
async fn agent_in_tui(h: &Harness) -> String {
    let (p, _) = h.project(false).await;
    second_account(h);
    let id = agent_on(h, &p, "pro").await.id;
    h.core.set_agent_view(&id, true).await.unwrap();
    let screen = std::sync::Arc::new(parking_lot::Mutex::new(String::new()));
    let sink = screen.clone();
    let info = h
        .core
        .open_agent_terminal(&id, (100, 30), false, move |b| {
            sink.lock().push_str(&String::from_utf8_lossy(&b))
        })
        .await
        .unwrap();
    let start = std::time::Instant::now();
    let mut answered = false;
    while !plain_text(&screen.lock()).contains("fake-claude-tui") {
        if !answered && screen.lock().contains("\x1b[6n") {
            h.core.pty.write(&info.id, b"\x1b[1;1R").unwrap();
            answered = true;
        }
        assert!(start.elapsed() < std::time::Duration::from_secs(60));
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    id
}

/// Runs `escouade-hook` with `event` on its input and `env` in its environment.
fn run_hook(event: &serde_json::Value, env: &[(&str, &str)]) {
    let program = hook_command().expect(
        "escouade-hook is not built beside the tests: `cargo test` builds it (its integration tests need it), `cargo test --lib` does not",
    );
    let mut c = Command::new(program);
    c.env_remove("ESCOUADE_HOOK_TOKEN")
        .env_remove("ESCOUADE_HOOK_URL")
        .envs(env.iter().copied())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = c.spawn().unwrap();
    let _ = child
        .stdin
        .take()
        .unwrap()
        .write_all(event.to_string().as_bytes());
    assert_eq!(child.wait().unwrap().code(), Some(0));
}

#[tokio::test(flavor = "multi_thread")]
async fn escouade_hook_tells_escouade_what_the_interface_does() {
    let h = harness("tui-escouade-hook");
    let id = agent_in_tui(&h).await;
    let session = h.agent(&id).session_id.unwrap();
    let url = format!("http://127.0.0.1:{}/hook", h.core.hooks_port().unwrap());
    let token = h.core.hooks.grant(&id);
    let event = |name: &str| {
        serde_json::json!({
            "hook_event_name": name, "session_id": session,
            "tool_name": "Bash", "tool_input": { "command": "echo tui" },
        })
    };
    // Without the token, nothing reaches Escouade.
    run_hook(&event("PermissionRequest"), &[("ESCOUADE_HOOK_URL", &url)]);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert_ne!(h.agent(&id).status, AgentStatus::Waiting);
    // With it, the permission it waits for shows on its card, and its answer ends the wait.
    let env = [
        ("ESCOUADE_HOOK_URL", url.as_str()),
        ("ESCOUADE_HOOK_TOKEN", &token),
    ];
    run_hook(&event("PermissionRequest"), &env);
    h.wait("the agent waits for its permission", |h| {
        h.agent(&id).status == AgentStatus::Waiting
    })
    .await;
    run_hook(&event("PostToolUse"), &env);
    h.wait("the agent runs again", |h| {
        h.agent(&id).status == AgentStatus::Running
    })
    .await;
}

/// What the hooks' server answers (status line and body) to `event` posted with `token`.
fn post_hook(port: u16, token: &str, event: &serde_json::Value) -> (String, String) {
    use std::io::Read;
    let body = event.to_string();
    let req = format!(
        "POST /hook HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .unwrap();
    s.write_all(req.as_bytes()).unwrap();
    let mut out = String::new();
    let _ = s.read_to_string(&mut out);
    let (head, body) = out.split_once("\r\n\r\n").unwrap_or((&out, ""));
    (
        head.lines().next().unwrap_or_default().to_string(),
        body.trim().to_string(),
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn the_hook_points_a_search_for_a_symbol_to_the_index_and_lets_a_text_search_go() {
    let h = harness("tui-index-hint");
    let id = agent_in_tui(&h).await;
    let project_id = h.agent(&id).project_id;
    let port = h.core.hooks_port().unwrap();
    let token = h.core.hooks.grant(&id);
    let session = h.agent(&id).session_id.unwrap();
    let search = |tool: &str, input: serde_json::Value| {
        post_hook(
            port,
            &token,
            &serde_json::json!({ "hook_event_name": "PreToolUse", "session_id": session,
                "tool_name": tool, "tool_input": input }),
        )
    };
    let symbol = serde_json::json!({ "pattern": "parse_config" });
    let bash = serde_json::json!({ "command": "rg -n 'fn load_all' src" });
    let text = serde_json::json!({ "pattern": "failed to connect" });

    // The index is off: nothing is said, whatever is searched.
    assert_eq!(
        search("Grep", symbol.clone()),
        ("HTTP/1.1 200 OK".into(), "{}".into())
    );
    h.core
        .set_code_index(&project_id, Some(true))
        .await
        .unwrap();

    // On: a search for a symbol, by the Grep tool or a shell, gets a pointer to the index, as
    // context only: no decision, so the search runs.
    for (tool, input) in [("Grep", symbol), ("Bash", bash)] {
        let (status, body) = search(tool, input);
        assert!(status.contains("200"), "{status}");
        let answer: serde_json::Value = serde_json::from_str(&body).unwrap();
        let out = &answer["hookSpecificOutput"];
        assert_eq!(out["hookEventName"], "PreToolUse", "{body}");
        assert!(out["additionalContext"]
            .as_str()
            .unwrap()
            .contains("index_search"));
        assert!(out.get("permissionDecision").is_none() && answer.get("decision").is_none());
    }
    // A text search, or another tool, goes by with nothing said.
    assert_eq!(search("Grep", text).1, "{}");
    assert_eq!(
        search("Bash", serde_json::json!({ "command": "npm test" })).1,
        "{}"
    );
    assert_eq!(
        search(
            "Read",
            serde_json::json!({ "file_path": "/tmp/parse_config" })
        )
        .1,
        "{}"
    );
    // The other events say nothing either.
    let (_, body) = post_hook(
        port,
        &token,
        &serde_json::json!({ "hook_event_name": "PostToolUse", "session_id": session,
            "tool_name": "Grep", "tool_input": { "pattern": "parse_config" } }),
    );
    assert_eq!(body, "{}");
}
