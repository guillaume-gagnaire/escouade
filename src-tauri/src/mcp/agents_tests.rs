//! Escouade's own agents and its MCP server: what their processes are started with (the fake
//! `claude` logs it), the config file and the token each one is given, and when they go.

use super::tests::{answer_text, bearer, client, post, INITIALIZE, TEST_TOOL};
use crate::core_tests::{harness, Harness};
use crate::engine::Process;
use crate::engine::{Engine, Escouade};
use crate::model::*;
use rmcp::model::CallToolRequestParams;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// What a launch was given of Escouade: its `--mcp-config` file, and whether Escouade's server was
/// kept out of it (`kept_out`).
fn escouade_flags(argv: &[String]) -> (Option<PathBuf>, bool) {
    let config = argv
        .iter()
        .position(|a| a == "--mcp-config")
        .map(|i| PathBuf::from(&argv[i + 1]));
    (config, kept_out(argv))
}

/// Escouade's server kept out of a launch, whatever config declares it: its tools refused
/// (`--disallowedTools`), and the server itself, which would otherwise be connected and give the
/// agent its instructions with none of its tools (ESC-91, seen on Claude Code 2.1.300), denied by
/// the one `--settings` file (only the last one given counts).
fn kept_out(argv: &[String]) -> bool {
    let tools_refused = argv
        .windows(2)
        .any(|w| w[0] == "--disallowedTools" && w[1] == "mcp__escouade");
    let settings: Vec<&String> = argv
        .windows(2)
        .filter(|w| w[0] == "--settings")
        .map(|w| &w[1])
        .collect();
    let server_denied = match settings.as_slice() {
        [file] => read_json(Path::new(file.as_str()))["deniedMcpServers"]
            .as_array()
            .is_some_and(|d| d.contains(&json!({ "serverName": "escouade" }))),
        _ => false,
    };
    tools_refused && server_denied
}

/// « Claude et les agents peuvent piloter Escouade » turned on or off, as the window does it: the
/// one switch that gives the server to Claude outside Escouade and to every agent of every project
/// (ESC-91).
fn drives(h: &Harness, on: bool) {
    h.core.set_mcp_enabled(on).unwrap();
}

/// A new agent of `p`, its first process started and ready (so it has logged its launch).
async fn started_agent(h: &Harness, p: &Project) -> String {
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.ensure_process(&id).await.unwrap();
    id
}

/// Once `p` has exited.
async fn gone(h: &Harness, p: &Process) {
    h.wait("the process to exit", |_| !p.is_alive()).await;
}

/// The agent's process stopped as the idle stop does it (detached, its input closed), once gone.
async fn stop(h: &Harness, id: &str) {
    let p = h
        .core
        .agent(id)
        .unwrap()
        .lock()
        .detach()
        .expect("a running process");
    p.close_input();
    gone(h, &p).await;
}

/// The agent's process stopped, if one is up, with no start in flight (the warm-up of a new agent
/// can come late under load): whatever starts next postdates what the test set before this.
async fn settled(h: &Harness, id: &str) {
    let lock = h.core.spawn_lock(id);
    let _no_start = lock.lock().await;
    let live = h.core.agent(id).unwrap().lock().detach();
    if let Some(p) = live {
        p.close_input();
        gone(h, &p).await;
    }
}

fn read_json(file: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap()
}

/// The token an agent's config file gives it.
fn token_in(config: &Path) -> String {
    read_json(config)["mcpServers"]["escouade"]["headers"]["Authorization"]
        .as_str()
        .and_then(|h| h.strip_prefix("Bearer "))
        .expect("a bearer token")
        .to_string()
}

/// Whether the server lets `token` in.
async fn admitted(port: u16, token: &str) -> bool {
    let (status, _) = post(port, &[("authorization", &bearer(token))], INITIALIZE).await;
    status == 200
}

#[test]
fn what_claude_codes_process_is_given_of_escouade_is_as_before() {
    let claude: &dyn Engine = &crate::engine::CLAUDE;
    let denied = |file: Option<&str>| Escouade::Denied {
        file: file.map(PathBuf::from),
    };
    assert_eq!(
        claude.escouade_launch(&denied(None)).args,
        ["--disallowedTools", "mcp__escouade"]
    );
    assert_eq!(
        claude
            .escouade_launch(&denied(Some("C:/data/mcp-refused.json")))
            .args,
        [
            "--settings",
            "C:/data/mcp-refused.json",
            "--disallowedTools",
            "mcp__escouade"
        ]
    );
    let granted = Escouade::Granted {
        url: "http://127.0.0.1:4321/mcp".into(),
        token: "s3cr3t".into(),
        file: Some(PathBuf::from("C:/data/mcp/a1.json")),
    };
    // A path, not the JSON itself: a `.cmd` launcher takes 8,191 characters at most.
    let launch = claude.escouade_launch(&granted);
    assert_eq!(launch.args, ["--mcp-config", "C:/data/mcp/a1.json"]);
    assert!(launch.env.is_empty());
    // The file names the server as `claude mcp add --transport http` would, with the token.
    let file: Value = serde_json::from_slice(
        &claude
            .grant_file("http://127.0.0.1:4321/mcp", "s3cr3t")
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        file,
        json!({ "mcpServers": { "escouade": {
            "type": "http",
            "url": "http://127.0.0.1:4321/mcp",
            "headers": { "Authorization": "Bearer s3cr3t" },
        } } })
    );
    // Refused, the server itself is kept out too, by the settings it is started with: its tools
    // refused alone, Claude Code still connects it and gives the agent its instructions.
    assert_eq!(
        claude.refusal(),
        Some(json!({ "deniedMcpServers": [{ "serverName": "escouade" }] }))
    );
}

/// What every engine must keep: the token of a process given Escouade's server is never on its
/// command line (seen by every process of the user, and written by the app's log), only in the
/// private file it is given or in its environment; and a refused process gets no token at all.
fn keeps_the_token_off_the_command_line(engine: &dyn Engine) {
    let (url, token) = ("http://127.0.0.1:4321/mcp", "t0k3n-0f-th3-ag3nt");
    let file = engine
        .grant_file(url, token)
        .map(|_| PathBuf::from("C:/data/mcp/a1.json"));
    let granted = Escouade::Granted {
        url: url.into(),
        token: token.into(),
        file: file.clone(),
    };
    let launch = engine.escouade_launch(&granted);
    let name = engine.name();
    assert!(
        !launch.args.iter().any(|a| a.contains(token)),
        "{name}: {launch:?}"
    );
    // Somewhere it reaches all the same: its file, or a variable.
    let in_file = engine
        .grant_file(url, token)
        .is_some_and(|b| String::from_utf8_lossy(&b).contains(token));
    let in_env = launch.env.iter().any(|(_, v)| v.contains(token));
    assert!(in_file || in_env, "{name}: the token reaches nothing");
    if in_file {
        let file = file.unwrap().to_string_lossy().into_owned();
        assert!(launch.args.contains(&file), "{name}: {launch:?}");
    }
    for refused in [
        Escouade::Denied { file: None },
        Escouade::Denied {
            file: Some(PathBuf::from("C:/data/mcp-refused.json")),
        },
    ] {
        let launch = engine.escouade_launch(&refused);
        assert!(
            !launch
                .args
                .iter()
                .chain(launch.env.iter().map(|(_, v)| v))
                .any(|a| a.contains(token)),
            "{name}: {launch:?}"
        );
    }
}

#[test]
fn no_engine_puts_the_token_on_the_command_line() {
    for engine in gives_the_server() {
        keeps_the_token_off_the_command_line(engine);
    }
}

/// The engines that give their agents Escouade's server at the launch of their process: all but
/// OpenCode, whose server is shared by the agents of an account and gives each agent's folder its
/// entry by its API (`engine/opencode/process.rs`, `give_escouade`; tested in `opencode_tests`),
/// and Cursor CLI and Mistral Vibe, whose session is given it on its input (`mcpServers` of
/// `session/new`; tested in `engine/cursor` and `agent/vibe_tests.rs`).
fn gives_the_server() -> Vec<&'static dyn Engine> {
    crate::engine::registry::all()
        .into_iter()
        .filter(|e| {
            !matches!(
                e.id(),
                EngineId::OPENCODE
                    | EngineId::CURSOR
                    | EngineId::VIBE
                    | EngineId::AGY
                    | EngineId::GEMINI
            )
        })
        .collect()
}

#[test]
fn an_agents_access_never_prints_its_token() {
    let granted = Escouade::Granted {
        url: "http://127.0.0.1:4321/mcp".into(),
        token: "s3cr3t".into(),
        file: Some(PathBuf::from("C:/data/mcp/a1.json")),
    };
    // The pretty form too (`{:#?}`): a log or a failed assertion writes either.
    for shown in [format!("{granted:?}"), format!("{granted:#?}")] {
        assert!(!shown.contains("s3cr3t"), "{shown}");
        assert!(
            shown.contains("Granted") && shown.contains("a1.json"),
            "{shown}"
        );
    }
    assert_eq!(
        format!("{:?}", Escouade::Denied { file: None }),
        "Denied { file: None }"
    );
    // Nor what a process is started with, where an engine may pass the token in a variable.
    let launch = crate::engine::mcp::Launch {
        args: vec!["--x".into()],
        env: vec![("ESCOUADE_MCP_TOKEN".into(), "s3cr3t".into())],
    };
    let shown = format!("{launch:?}");
    assert!(
        !shown.contains("s3cr3t") && shown.contains("ESCOUADE_MCP_TOKEN"),
        "{shown}"
    );
}

#[test]
fn escouades_tools_are_named_mcp_escouade_tool_whatever_the_engine() {
    use crate::engine::mcp::{tool_name, SERVER};
    assert_eq!(
        tool_name(SERVER, "create_ticket"),
        "mcp__escouade__create_ticket"
    );
    assert_eq!(
        tool_name("github", "create_issue"),
        "mcp__github__create_issue"
    );
    // What refuses them to Claude Code is the start of those names.
    assert_eq!(crate::engine::claude_all_tools(), "mcp__escouade");
    for engine in gives_the_server() {
        // Every engine names the server `escouade` where it gives it: its tools then take the
        // names the window reads (the permission card, the labels, the search).
        let file = engine.grant_file("http://127.0.0.1:1/mcp", "t");
        let launch = engine.escouade_launch(&Escouade::Granted {
            url: "http://127.0.0.1:1/mcp".into(),
            token: "t".into(),
            file: None,
        });
        let named = file.is_some_and(|b| String::from_utf8_lossy(&b).contains("\"escouade\""))
            || launch.args.iter().any(|a| a.contains("escouade"));
        assert!(named, "{}", engine.name());
    }
}

#[tokio::test]
async fn an_agent_reaches_escouade_with_a_token_of_its_own_while_claude_may_drive_it() {
    let h = harness("mcp-agents-launch");
    let (p, r) = h.project(false).await;
    // The switch off: no server to reach, and Escouade's tools refused to the agent all the same
    // (an entry of the user's config of Claude Code may declare them).
    let id = started_agent(&h, &p).await;
    assert!(!h.core.mcp.status().running);
    assert_eq!(escouade_flags(&h.launches(&r).pop().unwrap()), (None, true));
    stop(&h, &id).await;

    // « Claude et les agents peuvent piloter Escouade » on: the server runs, and the agent's next process reaches
    // it with a token of its own, in a private file. Nothing to allow in its project.
    drives(&h, true);
    settled(&h, &id).await;
    let port = h.core.mcp.status().port;
    h.turn(&id, "Bonjour").await;
    let argv = h.launches(&r).pop().unwrap();
    let (config, denied) = escouade_flags(&argv);
    let status = h.core.mcp.status();
    let config = config.unwrap_or_else(|| panic!("no --mcp-config: {argv:?} {status:?}"));
    assert!(!denied, "{argv:?}");
    assert_eq!(config, h.core.data.mcp_agent_config(&id));
    let token = token_in(&config);
    assert_eq!(
        read_json(&config),
        json!({ "mcpServers": { "escouade": {
            "type": "http",
            "url": format!("http://127.0.0.1:{port}/mcp"),
            "headers": { "Authorization": format!("Bearer {token}") },
        } } })
    );
    assert_ne!(token, h.core.mcp.external_token().unwrap());
    // Never on the command line (nor in the app's log, which writes it).
    assert!(!argv.iter().any(|a| a.contains(&token)), "{argv:?}");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&config).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    // The server takes it for the agent.
    let c = client(port, &token).await.unwrap();
    let who = c
        .call_tool(CallToolRequestParams::new(TEST_TOOL))
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&answer_text(&who)).unwrap(),
        json!({ "caller": "agent", "agent": id })
    );
    c.cancel().await.unwrap();

    // Its process stops: its file and its token go.
    stop(&h, &id).await;
    h.wait("its config to go", |_| !config.exists()).await;
    assert!(!admitted(port, &token).await);

    // The server stopped though the switch is on (it failed, say): nothing to reach, and
    // Escouade's tools refused all the same. An entry of the same name in the user's config of
    // Claude Code would be inherited, and the agent would act as « Claude (hors Escouade) ».
    h.core.mcp.stop();
    settled(&h, &id).await;
    assert_eq!(
        h.core.agent_access(&id, &crate::engine::CLAUDE),
        Escouade::Denied { file: None }
    );
    h.turn(&id, "Toujours là ?").await;
    assert_eq!(escouade_flags(&h.launches(&r).pop().unwrap()), (None, true));
    assert!(!config.exists());
    stop(&h, &id).await;

    // The switch off again: the server stops, yet an entry of the same name may still be declared
    // in the user's config of Claude Code (its removal failed, or the user wrote it), which the
    // agent would inherit: its tools are refused all the same.
    drives(&h, true);
    assert!(h.core.mcp.status().running);
    drives(&h, false);
    assert!(!h.core.mcp.status().running);
    settled(&h, &id).await;
    h.turn(&id, "Sans serveur").await;
    assert_eq!(escouade_flags(&h.launches(&r).pop().unwrap()), (None, true));
    assert!(!config.exists());
}

#[tokio::test]
async fn a_project_that_let_its_agents_use_escouade_before_the_one_switch_gives_them_nothing_alone()
{
    // A project of 1.7 saved with « Les agents peuvent utiliser Escouade » on, the switch of the
    // app off: its agents lose the server (ESC-91: the switch of the app alone gives it).
    let h = harness("mcp-agents-old-project");
    let (p, r) = h.project(false).await;
    let saved = serde_json::to_value(h.core.project(&p.id).unwrap()).unwrap();
    let mut old = saved.clone();
    old["agentsUseEscouade"] = json!(true);
    h.core
        .update_project(serde_json::from_value(old).unwrap())
        .unwrap();
    assert!(!h.core.mcp_wanted());
    let id = started_agent(&h, &p).await;
    assert!(!h.core.mcp.status().running);
    assert_eq!(escouade_flags(&h.launches(&r).pop().unwrap()), (None, true));
    // Nor is it written back.
    assert!(serde_json::to_value(h.core.project(&p.id).unwrap())
        .unwrap()
        .get("agentsUseEscouade")
        .is_none());
    stop(&h, &id).await;
}

#[tokio::test]
async fn a_process_that_ends_once_the_next_one_started_leaves_the_next_ones_token() {
    let h = harness("mcp-agents-restart");
    let (p, _) = h.project(false).await;
    drives(&h, true);
    let port = h.core.mcp.status().port;
    let id = started_agent(&h, &p).await;
    let config = h.core.data.mcp_agent_config(&id);
    let first = token_in(&config);
    assert!(admitted(port, &first).await);
    // Stopped for being idle, then a message at once: the next process starts before the first
    // one is gone, with a token of its own.
    let old = h.core.agent(&id).unwrap().lock().detach().unwrap();
    h.core.ensure_process(&id).await.unwrap();
    let second = token_in(&config);
    assert_ne!(first, second);
    assert!(!admitted(port, &first).await);
    old.close_input();
    gone(&h, &old).await;
    // The first one's end handled (just after it is seen gone): the second one's stay.
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(config.exists());
    assert_eq!(token_in(&config), second);
    assert!(admitted(port, &second).await);
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agents_token_and_file_go_at_once_when_it_is_deleted_or_its_project_closed() {
    let h = harness("mcp-agents-removed");
    let (p, _) = h.project(false).await;
    // The server stays when the project goes: Claude may drive Escouade.
    drives(&h, true);
    let port = h.core.mcp.status().port;
    let (a, b) = (started_agent(&h, &p).await, started_agent(&h, &p).await);
    let (ca, cb) = (
        h.core.data.mcp_agent_config(&a),
        h.core.data.mcp_agent_config(&b),
    );
    let (ta, tb) = (token_in(&ca), token_in(&cb));
    assert!(admitted(port, &ta).await && admitted(port, &tb).await);
    h.core.delete_agent(&a, false).await.unwrap();
    assert!(!ca.exists());
    assert!(!admitted(port, &ta).await);
    assert!(admitted(port, &tb).await);
    h.core.remove_project(&p.id, false).await.unwrap();
    assert!(!cb.exists());
    assert!(!admitted(port, &tb).await);
    h.core.mcp.stop();
}

#[tokio::test]
async fn no_agent_config_outlives_the_app() {
    let h = harness("mcp-agents-quit");
    let (p, _) = h.project(false).await;
    drives(&h, true);
    let id = started_agent(&h, &p).await;
    let config = h.core.data.mcp_agent_config(&id);
    assert!(config.exists());
    // The app quits: the agents' tokens die with it, their files go.
    h.core.shutdown();
    assert!(!config.exists());
    // One a crash left behind goes when the app starts again, before any agent does.
    let left = h.core.data.mcp_agent_config("crashed");
    std::fs::create_dir_all(left.parent().unwrap()).unwrap();
    std::fs::write(&left, "{}").unwrap();
    h.core.start_mcp();
    assert!(!left.exists());
}

#[tokio::test]
async fn a_project_closed_while_an_agents_process_starts_leaves_neither_that_process_nor_its_token()
{
    let h = harness("mcp-agents-closing");
    let (p, _) = h.project(false).await;
    // The server stays up when the project goes.
    drives(&h, true);
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let agent = h.core.agent(&id).unwrap();
    let config = h.core.data.mcp_agent_config(&id);
    // A start is under way (the warm-up of a new agent is one): it holds the agent's start lock.
    let lock = h.core.spawn_lock(&id);
    let guard = lock.lock().await;
    let live = agent.lock().detach();
    if let Some(p) = live {
        p.close_input();
        gone(&h, &p).await;
    }
    let start = tokio::spawn({
        let (core, id) = (h.core.clone(), id.clone());
        async move { core.ensure_process(&id).await }
    });
    tokio::time::sleep(Duration::from_millis(300)).await;
    let closing = tokio::spawn({
        let (core, pid) = (h.core.clone(), p.id.clone());
        async move { core.remove_project(&pid, false).await }
    });
    tokio::time::sleep(Duration::from_millis(300)).await;
    // The close waits for the start, as the deletion of an agent does: the agent is still there.
    assert!(h.core.agent(&id).is_ok());
    drop(guard);
    start.await.unwrap().unwrap();
    closing.await.unwrap().unwrap();
    // The process the start made went with the agent, so did its token and its file.
    assert!(h.core.agent(&id).is_err());
    assert!(agent.lock().proc.is_none());
    assert!(!h.core.mcp.tokens.agents.read().contains_key(&id));
    assert!(!config.exists());
    h.core.mcp.stop();
}

/// What the window is shown of the agent: « Outils d'Escouade au prochain démarrage ».
fn escouade_next(h: &Harness, id: &str) -> bool {
    h.core.agent(id).unwrap().lock().view().escouade_next
}

#[tokio::test]
async fn an_agent_started_before_the_switch_says_it_gets_escouade_at_its_next_start_and_restarts_on_its_session(
) {
    let h = harness("mcp-agents-next-start");
    let (p, r) = h.project(false).await;
    let id = started_agent(&h, &p).await;
    h.turn(&id, "Bonjour").await;
    let session = h.agent(&id).session_id.expect("a session");
    // The switch off, nothing to come: its next process would be refused Escouade all the same.
    assert!(!escouade_next(&h, &id));

    // Turned on: its process, started without Escouade's server, says it gets it at its next
    // start, and the window is told.
    drives(&h, true);
    assert!(escouade_next(&h, &id));
    assert!(h.events.lock().iter().any(|e| e["type"] == "agent"
        && e["agent"]["id"] == id.as_str()
        && e["agent"]["escouadeNext"] == true));
    // Turned off again before it restarted: nothing to come any more.
    drives(&h, false);
    assert!(!escouade_next(&h, &id));
    drives(&h, true);
    assert!(escouade_next(&h, &id));

    // « Relancer », at rest: its process stops and the next one resumes the same session, with the
    // server as itself; nothing left to come.
    let before = h.launches(&r).len();
    h.core.restart_process(&id).await.unwrap();
    let launches = h.launches(&r);
    assert_eq!(launches.len(), before + 1, "{launches:?}");
    let argv = launches.last().unwrap();
    assert!(argv.contains(&format!("--resume={session}")), "{argv:?}");
    let (config, kept) = escouade_flags(argv);
    assert!(config.is_some() && !kept, "{argv:?}");
    assert!(!escouade_next(&h, &id));
    assert!(h
        .core
        .agent(&id)
        .unwrap()
        .lock()
        .proc
        .as_ref()
        .unwrap()
        .is_alive());
    // Its conversation goes on, on the same session.
    h.turn(&id, "Toujours là ?").await;
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(session.as_str()));
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agent_without_a_process_or_at_work_is_not_restarted_under_its_feet() {
    let h = harness("mcp-agents-restart-busy");
    let (p, r) = h.project(false).await;
    let id = started_agent(&h, &p).await;
    // Stopped for being idle (no start in flight left, the warm-up of a new agent can come late):
    // nothing runs without Escouade, nothing to say.
    settled(&h, &id).await;
    drives(&h, true);
    assert!(!escouade_next(&h, &id));
    // At work: a turn, a question waiting for an answer, the setup of its worktree, or a subagent
    // (or a workflow) going on in the background after its turn, which the restart would kill:
    // refused, its process left as it is.
    h.core.ensure_process(&id).await.unwrap();
    let live = h.core.agent(&id).unwrap().lock().proc.clone().unwrap();
    let background = || crate::plan::PlanState {
        agents: vec![crate::plan::SubAgent {
            id: "t1".into(),
            background: true,
            ..Default::default()
        }],
        ..Default::default()
    };
    type MakesBusy = Box<dyn Fn(&mut crate::agent::AgentRt)>;
    let busy: [(&str, MakesBusy); 4] = [
        ("turn", Box::new(|rt| rt.meta.status = AgentStatus::Running)),
        (
            "question",
            Box::new(|rt| rt.meta.status = AgentStatus::Waiting),
        ),
        (
            "setup",
            Box::new(|rt| rt.setup = Some("1/2 · npm ci".into())),
        ),
        (
            "background",
            Box::new(move |rt| rt.meta.plan = Some(background())),
        ),
    ];
    for (what, set) in busy {
        set(&mut h.core.agent(&id).unwrap().lock());
        let before = h.launches(&r).len();
        let refused = h.core.restart_process(&id).await.unwrap_err();
        assert!(
            refused.to_string().contains("travaille"),
            "{what}: {refused:#}"
        );
        assert_eq!(h.launches(&r).len(), before, "{what}");
        let now = h.core.agent(&id).unwrap().lock().proc.clone().unwrap();
        assert!(Arc::ptr_eq(&live, &now) && now.is_alive(), "{what}");
        let agent = h.core.agent(&id).unwrap();
        let mut rt = agent.lock();
        rt.meta.status = AgentStatus::Done;
        rt.setup = None;
        rt.meta.plan = None;
    }
    // Archived, or in the Claude Code view (its interface is restarted by the window): refused.
    for (what, set) in [("archived", true), ("terminal", false)] {
        {
            let agent = h.core.agent(&id).unwrap();
            let mut rt = agent.lock();
            rt.meta.archived = set;
            rt.meta.terminal = !set;
        }
        assert!(h.core.restart_process(&id).await.is_err(), "{what}");
        let now = h.core.agent(&id).unwrap().lock().proc.clone().unwrap();
        assert!(Arc::ptr_eq(&live, &now) && now.is_alive(), "{what}");
    }
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_refusal_file_is_written_back_when_it_is_not_as_it_should_be() {
    let h = harness("mcp-agents-refusal-file");
    let refusal = crate::engine::CLAUDE.refusal().unwrap();
    let file = h.core.mcp_refusal_file(&refusal).unwrap();
    assert_eq!(read_json(&file), refusal);
    // Changed by hand (or cut short): written again at the next refused start.
    std::fs::write(&file, "{}").unwrap();
    assert_eq!(h.core.mcp_refusal_file(&refusal).unwrap(), file);
    assert_eq!(read_json(&file), refusal);
    // Many starts at once (warm-ups, the autopilot): each one gets the file, whole.
    std::fs::remove_file(&file).unwrap();
    let starts: Vec<_> = (0..8)
        .map(|_| {
            let (core, refusal) = (h.core.clone(), refusal.clone());
            std::thread::spawn(move || core.mcp_refusal_file(&refusal).map(|_| ()))
        })
        .collect();
    for s in starts {
        s.join().unwrap().unwrap();
    }
    assert_eq!(read_json(&file), refusal);
}

#[tokio::test]
async fn turned_off_the_agents_already_running_lose_the_tools_at_once_and_get_them_back_by_a_restart_only(
) {
    let h = harness("mcp-agents-turned-off");
    let (p, _) = h.project(false).await;
    drives(&h, true);
    let id = started_agent(&h, &p).await;
    let config = h.core.data.mcp_agent_config(&id);
    let token = token_in(&config);
    assert!(admitted(h.core.mcp.status().port, &token).await);
    assert!(!escouade_next(&h, &id));

    // Off: its token is forgotten at once, with its file, as the token of Claude outside Escouade.
    drives(&h, false);
    assert!(!config.exists());
    assert_eq!(h.core.mcp.caller(&token), None);
    assert!(!escouade_next(&h, &id));

    // On again, in the same session: the token it was started with opens nothing any more, and its
    // conversation says its next start gives it the tools.
    drives(&h, true);
    let port = h.core.mcp.status().port;
    assert!(!admitted(port, &token).await);
    assert!(escouade_next(&h, &id));
    // « Relancer »: a token of its own again.
    h.core.restart_process(&id).await.unwrap();
    let fresh = token_in(&config);
    assert_ne!(fresh, token);
    assert!(admitted(port, &fresh).await);
    assert!(!escouade_next(&h, &id));
    h.core.mcp.stop();
}

/// The launches of the fake `codex` with the folder `dir` (`<CODEX_HOME>/launches.jsonl`).
fn codex_launches(dir: &Path) -> Vec<Value> {
    std::fs::read_to_string(dir.join("launches.jsonl"))
        .unwrap_or_default()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[tokio::test]
async fn a_codex_agent_is_given_the_server_by_its_options_and_its_token_by_its_environment_or_refused_it(
) {
    let h = harness("esc38-mcp-codex");
    let (p, _) = h.project(false).await;
    let home = h.dir.join("codex-work");
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "codex-work".into(),
            name: "Codex".into(),
            engine: EngineId::CODEX,
            config_dir: home.to_string_lossy().into(),
            claude_path: crate::engine::fake_codex(),
            active: true,
            projects: Vec::new(),
            daily_budget: None,
            ..Default::default()
        })
    });
    let id = h
        .core
        .create_agent_with(
            &p.id,
            crate::core::AgentOptions {
                engine: Some(EngineId::CODEX),
                account: Some("codex-work".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    let argv_of = |l: &Value| -> Vec<String> {
        l["argv"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a.as_str().unwrap().to_string())
            .collect()
    };
    // « Claude et les agents peuvent piloter Escouade » on: its URL among the options, its token in
    // the environment alone, and the server lets it in.
    drives(&h, true);
    settled(&h, &id).await;
    let port = h.core.mcp.status().port;
    h.turn(&id, "bonjour").await;
    let launch = codex_launches(&home).pop().unwrap();
    let argv = argv_of(&launch);
    assert!(
        argv.contains(&format!(
            "mcp_servers.escouade.url=http://127.0.0.1:{port}/mcp"
        )),
        "{argv:?}"
    );
    assert!(
        argv.contains(&"mcp_servers.escouade.bearer_token_env_var=ESCOUADE_MCP_TOKEN".to_string())
    );
    let token = launch["escouadeToken"]
        .as_str()
        .expect("a token")
        .to_string();
    assert!(argv.iter().all(|a| !a.contains(&token)), "{argv:?}");
    assert!(admitted(port, &token).await);
    // On, the switch declared the server in the account's own config too (Codex is installed,
    // ESC-119); off, it takes it out.
    assert!(std::fs::read_to_string(home.join("config.toml"))
        .unwrap()
        .contains("[mcp_servers.escouade]"));
    // Off: nothing of the server, and no token.
    drives(&h, false);
    h.wait("the entry taken out of the account's config", |_| {
        !std::fs::read_to_string(home.join("config.toml"))
            .unwrap_or_default()
            .contains("[mcp_servers.escouade]")
    })
    .await;
    settled(&h, &id).await;
    h.turn(&id, "encore").await;
    let launch = codex_launches(&home).pop().unwrap();
    assert!(launch["escouadeToken"].is_null());
    assert!(!argv_of(&launch).iter().any(|a| a.contains("mcp_servers")));
    // An entry `escouade` of the user's own config of Codex is turned off.
    std::fs::write(
        home.join("config.toml"),
        "[mcp_servers.escouade]\nurl = \"http://127.0.0.1:1/mcp\"\n",
    )
    .unwrap();
    settled(&h, &id).await;
    h.turn(&id, "et encore").await;
    let argv = argv_of(&codex_launches(&home).pop().unwrap());
    assert!(
        argv.contains(&"mcp_servers.escouade.enabled=false".to_string()),
        "{argv:?}"
    );
    h.core.stop_for_good();
}

/// The methods the last process of the fake `copilot` of the folder `dir` received (those after
/// its `connect`), with their parameters.
fn copilot_last_calls(dir: &Path) -> Vec<Value> {
    let all = copilot_noted(dir, "calls.jsonl");
    let from = all
        .iter()
        .rposition(|c| c["method"] == "connect")
        .unwrap_or(0);
    all[from..].to_vec()
}

/// What the fake `copilot` of the folder `dir` wrote in its file `name`.
fn copilot_noted(dir: &Path, name: &str) -> Vec<Value> {
    std::fs::read_to_string(dir.join(name))
        .unwrap_or_default()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[tokio::test]
async fn a_copilot_agent_is_given_the_server_by_a_file_naming_its_token_or_refused_it() {
    let h = harness("esc40-mcp-copilot");
    let (p, _) = h.project(false).await;
    let home = h.dir.join("copilot-work");
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "copilot-work".into(),
            name: "Copilot".into(),
            engine: EngineId::COPILOT,
            config_dir: home.to_string_lossy().into(),
            claude_path: crate::engine::fake_copilot(),
            active: true,
            projects: Vec::new(),
            daily_budget: None,
            ..Default::default()
        })
    });
    let id = h
        .core
        .create_agent_with(
            &p.id,
            crate::core::AgentOptions {
                engine: Some(EngineId::COPILOT),
                account: Some("copilot-work".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    let argv_of = |l: &Value| -> Vec<String> {
        l["argv"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a.as_str().unwrap().to_string())
            .collect()
    };
    // « Claude et les agents peuvent piloter Escouade » on: its entry in a private file, which
    // names the token; the token in its environment alone; Copilot reaches the server with it.
    drives(&h, true);
    settled(&h, &id).await;
    let port = h.core.mcp.status().port;
    h.turn(&id, "bonjour").await;
    let launch = copilot_noted(&home, "launches.jsonl").pop().unwrap();
    let argv = argv_of(&launch);
    let config = h.core.data.mcp_agent_config(&id);
    let at = argv
        .iter()
        .position(|a| a == "--additional-mcp-config")
        .unwrap_or_else(|| panic!("{argv:?}"));
    assert_eq!(argv[at + 1], format!("@{}", config.to_string_lossy()));
    let token = launch["escouadeToken"]
        .as_str()
        .expect("a token")
        .to_string();
    assert!(argv.iter().all(|a| !a.contains(&token)), "{argv:?}");
    let file = std::fs::read_to_string(&config).unwrap();
    assert!(!file.contains(&token), "{file}");
    assert!(file.contains("Bearer ${ESCOUADE_MCP_TOKEN}"), "{file}");
    let reached = copilot_noted(&home, "mcp.jsonl").pop().unwrap();
    assert_eq!(
        (reached["name"].as_str(), reached["status"].as_u64()),
        (Some("escouade"), Some(200))
    );
    assert_eq!(
        reached["url"],
        format!("http://127.0.0.1:{port}/mcp").as_str()
    );
    assert!(admitted(port, &token).await);
    // Given, the session keeps its entry on.
    let calls = copilot_last_calls(&home);
    assert!(!calls.iter().any(|c| c["method"] == "session.mcp.disable"));
    // Off: no file, no token, and the server turned off by its name, for the session too (an
    // entry `escouade` of the user's mcp-config.json is inherited otherwise).
    drives(&h, false);
    settled(&h, &id).await;
    h.turn(&id, "encore").await;
    let launch = copilot_noted(&home, "launches.jsonl").pop().unwrap();
    assert!(launch["escouadeToken"].is_null());
    let argv = argv_of(&launch);
    assert!(
        !argv.iter().any(|a| a == "--additional-mcp-config"),
        "{argv:?}"
    );
    let at = argv
        .iter()
        .position(|a| a == "--disable-mcp-server")
        .unwrap_or_else(|| panic!("{argv:?}"));
    assert_eq!(argv[at + 1], "escouade");
    let calls = copilot_last_calls(&home);
    assert!(calls
        .iter()
        .any(|c| c["method"] == "session.mcp.disable" && c["params"]["serverName"] == "escouade"));
    h.core.stop_for_good();
}

/// ESC-43: a Cursor CLI agent is given Escouade's server by its session (`mcpServers` of
/// `session/new`, its token in the header, never on its command line), and reaches it as itself;
/// the switch off, its session is given none.
#[tokio::test]
async fn a_cursor_agent_reaches_escouade_by_the_server_its_session_is_given() {
    let h = harness("esc43-cursor-mcp");
    let (p, _) = h.project(false).await;
    let program = crate::engine::fake_cursor();
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "principal-cursor".into(),
            name: "Principal".into(),
            engine: EngineId::CURSOR,
            claude_path: program.clone(),
            active: true,
            ..Default::default()
        });
    });
    let said = |h: &Harness, id: &str, start: &str| {
        h.items(id)
            .iter()
            .filter(|i| i["kind"] == "text")
            .filter_map(|i| i["text"].as_str().map(str::to_string))
            .find(|t| t.starts_with(start))
    };
    let agent = |h: &Harness| {
        let (c, p) = (h.core.clone(), p.id.clone());
        async move {
            c.create_agent_with(
                &p,
                crate::core::AgentOptions {
                    engine: Some(EngineId::CURSOR),
                    model: Some("default".into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap()
            .meta
            .id
        }
    };
    drives(&h, true);
    let id = agent(&h).await;
    h.core
        .send_message(&id, format!("outil escouade {TEST_TOOL}"), vec![])
        .await
        .unwrap();
    h.wait("its answer", |h| said(h, &id, "Escouade a").is_some())
        .await;
    let answer = said(&h, &id, "Escouade a").unwrap();
    let who: Value = serde_json::from_str(answer.trim_start_matches("Escouade a répondu : "))
        .unwrap_or_else(|_| panic!("{answer}"));
    assert_eq!(who, json!({ "caller": "agent", "agent": id }));

    // The switch off: a new agent's session is given no server.
    drives(&h, false);
    let other = agent(&h).await;
    h.core
        .send_message(&other, format!("outil escouade {TEST_TOOL}"), vec![])
        .await
        .unwrap();
    h.wait("its answer", |h| {
        said(h, &other, "Pas de serveur").is_some()
    })
    .await;
}
