//! Escouade's own agents and its MCP server: what their processes are started with (the fake
//! `claude` logs it), the config file and the token each one is given, and when they go.

use super::tests::{answer_text, bearer, client, post, INITIALIZE, TEST_TOOL};
use super::AgentAccess;
use crate::claude::ClaudeProcess;
use crate::core_tests::{harness, Harness};
use crate::model::*;
use rmcp::model::CallToolRequestParams;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// What a launch was given of Escouade: its `--mcp-config` file, and whether Escouade's tools were
/// refused to it.
fn escouade_flags(argv: &[String]) -> (Option<PathBuf>, bool) {
    let config = argv
        .iter()
        .position(|a| a == "--mcp-config")
        .map(|i| PathBuf::from(&argv[i + 1]));
    let denied = argv
        .windows(2)
        .any(|w| w[0] == "--disallowedTools" && w[1] == "mcp__escouade");
    (config, denied)
}

/// « Les agents peuvent utiliser Escouade » set on the project, as the window saves it.
fn lets_agents(h: &Harness, p: &Project, allowed: bool) {
    let p = h.core.project(&p.id).unwrap();
    h.core
        .update_project(Project {
            agents_use_escouade: allowed,
            ..p
        })
        .unwrap();
}

/// A new agent of `p`, its first process started and ready (so it has logged its launch).
async fn started_agent(h: &Harness, p: &Project) -> String {
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.ensure_process(&id).await.unwrap();
    id
}

/// Once `p` has exited.
async fn gone(h: &Harness, p: &Arc<ClaudeProcess>) {
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
fn what_an_agents_process_is_given_of_escouade_never_holds_its_token() {
    assert!(AgentAccess::None.args().is_empty());
    assert_eq!(
        AgentAccess::Denied.args(),
        ["--disallowedTools", "mcp__escouade"]
    );
    let granted = AgentAccess::Granted {
        config: PathBuf::from("C:/data/mcp/a1.json"),
        token: "s3cr3t".into(),
    };
    // A path, not the JSON itself: a `.cmd` launcher takes 8,191 characters at most.
    assert_eq!(granted.args(), ["--mcp-config", "C:/data/mcp/a1.json"]);
}

#[tokio::test]
async fn an_agent_reaches_escouade_with_a_token_of_its_own_only_where_its_project_lets_it() {
    let h = harness("mcp-agents-launch");
    let (p, r) = h.project(false).await;
    // Nothing wants the server and the project does not let its agents use it: no server to
    // reach, and Escouade's tools refused to the agent all the same (an entry of the user's config
    // of Claude Code may declare them).
    let id = started_agent(&h, &p).await;
    assert!(!h.core.mcp.status().running);
    assert_eq!(escouade_flags(&h.launches(&r).pop().unwrap()), (None, true));
    stop(&h, &id).await;

    // Its project lets its agents use Escouade: the server runs, and the agent's next process
    // reaches it with a token of its own, in a private file.
    lets_agents(&h, &p, true);
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

    // The server stopped though the project lets its agents (it failed, say): nothing to reach,
    // started as it always was.
    h.core.mcp.stop();
    settled(&h, &id).await;
    h.turn(&id, "Toujours là ?").await;
    assert_eq!(
        escouade_flags(&h.launches(&r).pop().unwrap()),
        (None, false)
    );
    assert!(!config.exists());
    stop(&h, &id).await;

    // The project does not let them, and the server is stopped: nothing runs, yet an entry of the
    // same name may still be declared in the user's config of Claude Code (it was while Claude
    // drove Escouade), which the agent would inherit: its tools are refused all the same.
    lets_agents(&h, &p, false);
    assert!(!h.core.mcp.status().running);
    settled(&h, &id).await;
    h.turn(&id, "Sans serveur").await;
    assert_eq!(escouade_flags(&h.launches(&r).pop().unwrap()), (None, true));
    stop(&h, &id).await;

    // The project no longer lets them, the server running all the same (Claude may drive
    // Escouade): Escouade's tools are refused to the agent, those of an entry of the same name in
    // the user's config of Claude Code too.
    h.core.settings.write().mcp_enabled = true;
    lets_agents(&h, &p, false);
    assert!(h.core.mcp.status().running);
    settled(&h, &id).await;
    h.turn(&id, "Encore").await;
    assert_eq!(escouade_flags(&h.launches(&r).pop().unwrap()), (None, true));
    assert!(!config.exists());
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_process_that_ends_once_the_next_one_started_leaves_the_next_ones_token() {
    let h = harness("mcp-agents-restart");
    let (p, _) = h.project(false).await;
    lets_agents(&h, &p, true);
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
    h.core.settings.write().mcp_enabled = true;
    lets_agents(&h, &p, true);
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
    h.core.remove_project(&p.id).await.unwrap();
    assert!(!cb.exists());
    assert!(!admitted(port, &tb).await);
    h.core.mcp.stop();
}

#[tokio::test]
async fn no_agent_config_outlives_the_app() {
    let h = harness("mcp-agents-quit");
    let (p, _) = h.project(false).await;
    lets_agents(&h, &p, true);
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
    h.core.settings.write().mcp_enabled = true;
    lets_agents(&h, &p, true);
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
        async move { core.remove_project(&pid).await }
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
