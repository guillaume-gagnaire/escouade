//! The MCP server on a free port, against a real MCP client (rmcp's) and raw HTTP requests.

use super::activity::{self, ActivityEntry, Outcome, KEPT};
use super::tools::Tools;
use super::*;
use crate::core_tests::{harness, Harness};
use crate::model::{Project, Settings};
use crate::paths::{test_dir, DataDir};
use rmcp::model::CallToolRequestParams;
use rmcp::service::RunningService;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::{RoleClient, ServiceExt};
use serde_json::{json, Value};
use tauri::test::{mock_app, MockRuntime};

/// The tools the server offers, sorted: none yet. M2 and M4 add theirs here, knowingly. None may
/// accept a permission, answer for the user, run a command or the tests, merge or delete.
const EXPOSED: &[&str] = &[];

/// The test's own tool, beside them in tests (`tools::Tools::test_router`).
const TEST_TOOL: &str = "whoami";

/// What a tool's name may never hold.
const FORBIDDEN: &[&str] = &[
    "permission",
    "approve",
    "accept",
    "allow",
    "answer",
    "command",
    "bash",
    "shell",
    "exec",
    "run",
    "test",
    "merge",
    "delete",
    "remove",
    "discard",
    "kill",
];

/// The `initialize` of Claude Code 2.1.289 (the version of the handshake it falls back on).
const INITIALIZE: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"claude-code","version":"2.1.289"}}}"#;

fn url(port: u16) -> String {
    format!("http://127.0.0.1:{port}/mcp")
}

/// The harness's server started: its port and the token of Claude outside Escouade.
fn started(h: &Harness) -> (u16, String) {
    let port = h.core.mcp.start(0).unwrap();
    (port, h.core.mcp.external_token().unwrap())
}

/// A real MCP client, initialized with `token`.
async fn client(
    port: u16,
    token: &str,
) -> Result<RunningService<RoleClient, ()>, Box<rmcp::service::ClientInitializeError>> {
    // rmcp's client is on reqwest 0.13, built here without a TLS provider of its own (another
    // crate asks for `rustls-no-provider`): it would panic without the process's (plain HTTP all
    // the same).
    let _ = rustls::crypto::ring::default_provider().install_default();
    let config = StreamableHttpClientTransportConfig::with_uri(url(port)).auth_header(token);
    ().serve(StreamableHttpClientTransport::from_config(config))
        .await
        .map_err(Box::new)
}

fn http() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().build().unwrap()
}

/// A raw POST of `body` to the server, with `headers` besides what MCP asks: its status and its
/// text.
async fn post(port: u16, headers: &[(&str, &str)], body: &str) -> (u16, String) {
    let mut req = http()
        .post(url(port))
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream");
    for (name, value) in headers {
        req = req.header(*name, *value);
    }
    let resp = req.body(body.to_string()).send().await.unwrap();
    (resp.status().as_u16(), resp.text().await.unwrap())
}

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

fn refusals(h: &Harness) -> Vec<ActivityEntry> {
    h.core
        .mcp
        .activity
        .entries()
        .into_iter()
        .filter(|e| e.outcome == "refused")
        .collect()
}

/// What a tool's call answered, as text.
fn answer_text(result: &rmcp::model::CallToolResult) -> String {
    serde_json::to_value(result).unwrap()["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string()
}

fn names(tools: &[rmcp::model::Tool]) -> Vec<String> {
    let mut names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();
    names.sort();
    names
}

#[tokio::test]
async fn claude_outside_escouade_initializes_and_lists_the_tools_with_its_token() {
    let h = harness("mcp-initialize");
    let (port, token) = started(&h);
    assert!(PORTS.contains(&port), "{port}");
    let c = client(port, &token).await.unwrap();
    let info = c.peer_info().unwrap();
    assert_eq!(info.server_info.as_ref().unwrap().name, "escouade");
    assert!(info.capabilities.tools.is_some());
    let mut listed = names(&c.list_all_tools().await.unwrap());
    listed.retain(|n| n != TEST_TOOL);
    assert_eq!(listed, EXPOSED);
    let who = c
        .call_tool(CallToolRequestParams::new(TEST_TOOL))
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&answer_text(&who)).unwrap(),
        json!({ "caller": "external" })
    );
    // The call is in the activity log, under the name the window shows.
    let last = h.core.mcp.activity.entries().pop().unwrap();
    assert_eq!(
        (last.caller.as_str(), last.tool.as_str(), last.outcome),
        ("Claude (hors Escouade)", TEST_TOOL, "ok")
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_server_offers_exactly_its_tools_and_none_that_accepts_runs_merges_or_deletes() {
    // The router the app ships, without the test's tool.
    let shipped = names(&Tools::<MockRuntime>::tool_router().list_all());
    assert_eq!(shipped, EXPOSED);
    let h = harness("mcp-tools");
    let (port, token) = started(&h);
    let c = client(port, &token).await.unwrap();
    let listed = names(&c.list_all_tools().await.unwrap());
    let mut expected: Vec<String> = EXPOSED.iter().map(|s| s.to_string()).collect();
    expected.push(TEST_TOOL.to_string());
    expected.sort();
    assert_eq!(listed, expected);
    for name in &listed {
        for word in FORBIDDEN {
            assert!(!name.contains(word), "{name} holds {word}");
        }
    }
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_request_without_a_token_or_with_an_unknown_one_is_refused_and_logged() {
    let h = harness("mcp-no-token");
    let (port, token) = started(&h);
    // No token, another scheme, an unknown token, the token of another run: 401.
    let wrong = "d3JvbmctdG9rZW4td3JvbmctdG9rZW4td3JvbmctdG9r";
    for auth in [
        None,
        Some(format!("Basic {token}")),
        Some(bearer(wrong)),
        Some(bearer(&token[..token.len() - 1])),
        Some("Bearer".to_string()),
    ] {
        let headers: Vec<(&str, &str)> = auth
            .as_deref()
            .map(|a| vec![("authorization", a)])
            .unwrap_or_default();
        let (status, text) = post(port, &headers, INITIALIZE).await;
        assert_eq!(status, 401, "{auth:?}: {text}");
    }
    // The real client cannot even initialize.
    assert!(client(port, wrong).await.is_err());
    // The token passes, its scheme in any case.
    let (status, text) = post(
        port,
        &[("authorization", &format!("bearer {token}"))],
        INITIALIZE,
    )
    .await;
    assert_eq!(status, 200, "{text}");
    // Each refusal is logged, from an unknown client, without the token it brought.
    let refused = refusals(&h);
    assert!(refused.len() >= 6, "{refused:?}");
    let messages: Vec<&str> = refused
        .iter()
        .map(|e| e.message.as_deref().unwrap())
        .collect();
    assert!(
        messages.contains(&"Requête refusée : pas de jeton"),
        "{messages:?}"
    );
    assert!(
        messages.contains(&"Requête refusée : jeton inconnu"),
        "{messages:?}"
    );
    for e in &refused {
        assert_eq!((e.caller.as_str(), e.tool.as_str()), ("Client inconnu", ""));
        assert_eq!(e.summary, "POST /mcp");
        let shown = serde_json::to_string(e).unwrap();
        assert!(!shown.contains(wrong) && !shown.contains(&token), "{shown}");
    }
    // The window was told of each one as it came.
    let told = h
        .events
        .lock()
        .iter()
        .filter(|e| e["type"] == "mcpActivity" && e["entry"]["outcome"] == "refused")
        .count();
    assert_eq!(told, refused.len());
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_foreign_host_or_a_web_pages_origin_is_refused_whatever_the_token() {
    let h = harness("mcp-host-origin");
    let (port, token) = started(&h);
    let auth = bearer(&token);
    let other = if port == *PORTS.end() {
        port - 1
    } else {
        port + 1
    };
    for host in [
        format!("evil.example:{port}"),
        format!("127.0.0.1:{other}"),
        "localhost".to_string(),
        format!("127.0.0.1.evil.example:{port}"),
    ] {
        let (status, text) = post(
            port,
            &[("host", &host), ("authorization", &auth)],
            INITIALIZE,
        )
        .await;
        assert_eq!(status, 403, "{host}: {text}");
    }
    let (status, text) = post(
        port,
        &[("origin", "http://localhost"), ("authorization", &auth)],
        INITIALIZE,
    )
    .await;
    assert_eq!(status, 403, "{text}");
    // The server's own names pass.
    for host in [format!("localhost:{port}"), format!("127.0.0.1:{port}")] {
        let (status, text) = post(
            port,
            &[("host", &host), ("authorization", &auth)],
            INITIALIZE,
        )
        .await;
        assert_eq!(status, 200, "{host}: {text}");
    }
    let messages: Vec<String> = refusals(&h)
        .into_iter()
        .map(|e| e.message.unwrap())
        .collect();
    assert!(
        messages.contains(&format!(
            "Requête refusée : en-tête Host inattendu (evil.example:{port})"
        )),
        "{messages:?}"
    );
    assert!(
        messages.contains(
            &"Requête refusée : en-tête Origin d’une page web (http://localhost)".to_string()
        ),
        "{messages:?}"
    );
    // Another path than the endpoint's, with the token: refused too, the caller known.
    let resp = http()
        .post(format!("http://127.0.0.1:{port}/other"))
        .header("authorization", &auth)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 404);
    let last = refusals(&h).pop().unwrap();
    assert_eq!(
        (last.caller.as_str(), last.message.as_deref()),
        (
            "Claude (hors Escouade)",
            Some("Requête refusée : chemin inconnu (/other)")
        )
    );
    h.core.mcp.stop();
}

#[tokio::test]
async fn claude_codes_discover_probe_its_initialize_and_an_unknown_method_get_json_rpc_answers() {
    let h = harness("mcp-protocol");
    let (port, token) = started(&h);
    let auth = bearer(&token);
    // Claude Code 2.1.289 first probes with `server/discover` (2026-07-28).
    let discover = json!({
        "jsonrpc": "2.0",
        "id": 0,
        "method": "server/discover",
        "params": { "_meta": {
            "io.modelcontextprotocol/protocolVersion": "2026-07-28",
            "io.modelcontextprotocol/clientInfo": { "name": "claude-code", "version": "2.1.289" },
            "io.modelcontextprotocol/clientCapabilities": {}
        } }
    });
    let (status, text) = post(
        port,
        &[
            ("authorization", &auth),
            ("mcp-protocol-version", "2026-07-28"),
            ("mcp-method", "server/discover"),
        ],
        &discover.to_string(),
    )
    .await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["id"], 0, "{answer}");
    let versions = answer["result"]["supportedVersions"].as_array().unwrap();
    assert!(versions.contains(&json!("2025-11-25")), "{answer}");
    // Then its `initialize`, in JSON.
    let (status, text) = post(port, &[("authorization", &auth)], INITIALIZE).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        answer["result"]["protocolVersion"], "2025-11-25",
        "{answer}"
    );
    assert_eq!(
        answer["result"]["serverInfo"]["name"], "escouade",
        "{answer}"
    );
    // A method the server does not know: a JSON-RPC error, not a broken answer.
    let unknown = r#"{"jsonrpc":"2.0","id":7,"method":"escouade/nothing","params":{}}"#;
    let (_, text) = post(
        port,
        &[
            ("authorization", &auth),
            ("mcp-protocol-version", "2025-11-25"),
        ],
        unknown,
    )
    .await;
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        (&answer["id"], &answer["error"]["code"]),
        (&json!(7), &json!(-32601)),
        "{answer}"
    );
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_tool_called_as_claude_code_calls_it_without_initialize_knows_its_caller() {
    // Claude Code 2.1.296, its `server/discover` answered, goes on in 2026-07-28 (no
    // `initialize`, the version in each request): so it was seen in a trial.
    let h = harness("mcp-protocol-2026");
    let (port, _) = started(&h);
    let token = h.core.mcp.register_agent("a1");
    let call = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": TEST_TOOL,
            "arguments": {},
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientInfo": { "name": "claude-code", "version": "2.1.296" },
                "io.modelcontextprotocol/clientCapabilities": {}
            }
        }
    });
    let (status, text) = post(
        port,
        &[
            ("authorization", &bearer(&token)),
            ("mcp-protocol-version", "2026-07-28"),
            ("mcp-method", "tools/call"),
            ("mcp-name", TEST_TOOL),
        ],
        &call.to_string(),
    )
    .await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    let said = answer["result"]["content"][0]["text"].as_str().unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(said).unwrap(),
        json!({ "caller": "agent", "agent": "a1" })
    );
    // No agent of that id any more: the log names it by its id.
    let last = h.core.mcp.activity.entries().pop().unwrap();
    assert_eq!(
        (last.caller.as_str(), last.tool.as_str()),
        ("a1", TEST_TOOL)
    );
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agents_token_is_taken_for_that_agent_until_it_is_forgotten() {
    let h = harness("mcp-agent-token");
    let (p, _) = h.project(false).await;
    let agent = h.core.create_agent(&p.id, None).await.unwrap().meta;
    let (port, external) = started(&h);
    let token = h.core.mcp.register_agent(&agent.id);
    assert_ne!(token, external);
    let c = client(port, &token).await.unwrap();
    let who = c
        .call_tool(CallToolRequestParams::new(TEST_TOOL))
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&answer_text(&who)).unwrap(),
        json!({ "caller": "agent", "agent": agent.id })
    );
    let last = h.core.mcp.activity.entries().pop().unwrap();
    assert_eq!(
        (last.caller.as_str(), last.outcome),
        (agent.name.as_str(), "ok")
    );
    c.cancel().await.unwrap();
    // A new one in place of the first: the first is refused.
    let renewed = h.core.mcp.register_agent(&agent.id);
    let (status, _) = post(port, &[("authorization", &bearer(&token))], INITIALIZE).await;
    assert_eq!(status, 401);
    let (status, _) = post(port, &[("authorization", &bearer(&renewed))], INITIALIZE).await;
    assert_eq!(status, 200);
    // Forgotten: refused; the external token still passes.
    h.core.mcp.forget_agent(&agent.id);
    let (status, _) = post(port, &[("authorization", &bearer(&renewed))], INITIALIZE).await;
    assert_eq!(status, 401);
    let (status, _) = post(port, &[("authorization", &bearer(&external))], INITIALIZE).await;
    assert_eq!(status, 200);
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_stopped_server_refuses_connections_and_the_app_stops_it_when_it_quits() {
    let h = harness("mcp-stop");
    let (port, token) = started(&h);
    let (status, _) = post(port, &[("authorization", &bearer(&token))], INITIALIZE).await;
    assert_eq!(status, 200);
    h.core.mcp.stop();
    let err = http().post(url(port)).send().await.unwrap_err();
    assert!(err.is_connect(), "{err:?}");
    assert!(!h.core.mcp.status().running);
    // Started again: on the same port, which it saved.
    h.core.settings.write().mcp_enabled = true;
    h.core.sync_mcp();
    let port = h.core.mcp.status().port;
    assert_eq!(h.core.settings.read().mcp_port, port);
    let (status, _) = post(port, &[("authorization", &bearer(&token))], INITIALIZE).await;
    assert_eq!(status, 200);
    h.core.mcp.stop();
    h.core.sync_mcp();
    assert_eq!(h.core.mcp.status().port, port);
    // The app quits: stopped, and never started again.
    h.core.shutdown();
    assert!(!h.core.mcp.status().running);
    let err = http().post(url(port)).send().await.unwrap_err();
    assert!(err.is_connect(), "{err:?}");
    h.core.sync_mcp();
    assert!(!h.core.mcp.status().running);
}

#[tokio::test]
async fn a_port_taken_at_the_start_gives_another_free_one_saved_and_told() {
    let h = harness("mcp-port-taken");
    let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let busy = taken.local_addr().unwrap().port();
    {
        let mut s = h.core.settings.write();
        s.mcp_enabled = true;
        s.mcp_port = busy;
    }
    h.core.sync_mcp();
    let status = h.core.mcp.status();
    assert!(status.running, "{status:?}");
    assert_ne!(status.port, busy);
    assert!(PORTS.contains(&status.port), "{status:?}");
    // Saved, in memory and on disk, for the next start (and the declaration in Claude).
    assert_eq!(h.core.settings.read().mcp_port, status.port);
    let saved: Settings =
        serde_json::from_slice(&std::fs::read(h.core.data.settings_file()).unwrap()).unwrap();
    assert_eq!(saved.mcp_port, status.port);
    // The window was told.
    let told: Vec<Value> = h
        .events
        .lock()
        .iter()
        .filter(|e| e["type"] == "mcpStatus")
        .cloned()
        .collect();
    assert_eq!(
        told.last().unwrap()["status"],
        json!({ "running": true, "port": status.port, "error": null })
    );
    let token = h.core.mcp.external_token().unwrap();
    let (code, _) = post(
        status.port,
        &[("authorization", &bearer(&token))],
        INITIALIZE,
    )
    .await;
    assert_eq!(code, 200);
    drop(taken);
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_server_runs_while_claude_may_drive_escouade_or_a_project_lets_its_agents() {
    let h = harness("mcp-lifecycle");
    // The settings of before: off, no port yet.
    h.core.sync_mcp();
    assert_eq!(
        h.core.mcp.status(),
        McpStatus {
            running: false,
            port: 0,
            error: None
        }
    );
    let on = Settings {
        mcp_enabled: true,
        ..h.core.settings.read().clone()
    };
    h.core.save_settings(on.clone()).unwrap();
    let status = h.core.mcp.status();
    assert!(status.running);
    let port = status.port;
    assert_eq!(h.core.settings.read().mcp_port, port);
    // A save from the window, its copy older (no port): the port stays.
    h.core.save_settings(on.clone()).unwrap();
    assert_eq!(h.core.mcp.status(), status);
    assert_eq!(h.core.settings.read().mcp_port, port);
    let off = Settings {
        mcp_enabled: false,
        ..on
    };
    h.core.save_settings(off).unwrap();
    assert_eq!(
        h.core.mcp.status(),
        McpStatus {
            running: false,
            port,
            error: None
        }
    );
    // A project whose agents may use Escouade: it runs, on its port, until none does.
    let (p, _) = h.project(false).await;
    assert!(!p.agents_use_escouade);
    h.core
        .update_project(Project {
            agents_use_escouade: true,
            ..p.clone()
        })
        .unwrap();
    assert!(h.core.mcp.status().running);
    assert_eq!(h.core.mcp.status().port, port);
    assert!(h.core.project(&p.id).unwrap().agents_use_escouade);
    h.core
        .update_project(Project {
            agents_use_escouade: false,
            ..p.clone()
        })
        .unwrap();
    assert!(!h.core.mcp.status().running);
    // Removed while it lets them: the server stops with it.
    h.core
        .update_project(Project {
            agents_use_escouade: true,
            ..p.clone()
        })
        .unwrap();
    assert!(h.core.mcp.status().running);
    h.core.remove_project(&p.id).unwrap();
    assert!(!h.core.mcp.status().running);
}

#[tokio::test]
async fn settings_and_projects_saved_before_the_mcp_server_leave_it_off() {
    let s: Settings = serde_json::from_value(json!({ "sound": false })).unwrap();
    assert_eq!((s.mcp_enabled, s.mcp_port), (false, 0));
    let d = Settings::default();
    assert_eq!((d.mcp_enabled, d.mcp_port), (false, 0));
    let v = serde_json::to_value(Settings {
        mcp_enabled: true,
        mcp_port: 47123,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        (&v["mcpEnabled"], &v["mcpPort"]),
        (&json!(true), &json!(47123))
    );
    let p: Project = serde_json::from_value(
        json!({ "id": "p1", "name": "demo", "path": "C:/demo", "color": "red" }),
    )
    .unwrap();
    assert!(!p.agents_use_escouade);
    assert_eq!(
        serde_json::to_value(Project {
            agents_use_escouade: true,
            ..p
        })
        .unwrap()["agentsUseEscouade"],
        json!(true)
    );
    // A data folder of 1.6, its project in it: the core loads it with the server off.
    let data = DataDir::new(test_dir("mcp-old-settings"));
    data.ensure().unwrap();
    std::fs::write(data.settings_file(), r#"{ "sound": false }"#).unwrap();
    std::fs::write(
        data.state_file(),
        r#"{ "projects": [{ "id": "p1", "name": "demo", "path": "C:/demo", "color": "red" }] }"#,
    )
    .unwrap();
    let app = mock_app();
    let (core, _rx) = Core::load(app.handle().clone(), data);
    assert_eq!(core.projects.read().len(), 1);
    assert!(!core.mcp_wanted());
    core.sync_mcp();
    assert_eq!(
        core.mcp.status(),
        McpStatus {
            running: false,
            port: 0,
            error: None
        }
    );
}

#[test]
fn the_activity_log_keeps_the_last_200_entries() {
    let log = activity::Activity::default();
    let entry = |i: usize| ActivityEntry {
        at: i as i64,
        caller: "Claude (hors Escouade)".into(),
        tool: "list_projects".into(),
        summary: String::new(),
        outcome: "ok",
        message: None,
    };
    for i in 0..KEPT + 5 {
        log.push(entry(i));
    }
    let kept = log.entries();
    assert_eq!(kept.len(), KEPT);
    assert_eq!((kept[0].at, kept[KEPT - 1].at), (5, (KEPT + 4) as i64));
    assert_eq!(KEPT, 200);
}

#[tokio::test]
async fn an_entry_names_its_caller_and_keeps_its_texts_short_on_one_line() {
    let h = harness("mcp-record");
    activity::record(
        &h.core,
        Some(&Caller::Agent("gone".into())),
        "get_ticket",
        &format!("ticket: {}\nnext", "x".repeat(400)),
        Outcome::Error("introuvable".into()),
    );
    let e = h.core.mcp.activity.entries().pop().unwrap();
    // An agent that is gone: its id.
    assert_eq!(e.caller, "gone");
    assert_eq!(
        (e.outcome, e.message.as_deref()),
        ("error", Some("introuvable"))
    );
    assert!(!e.summary.contains('\n'));
    assert_eq!(e.summary.chars().count(), 300);
    assert!(e.summary.ends_with('…'));
    let told = h.events.lock().last().cloned().unwrap();
    assert_eq!(told["type"], "mcpActivity");
    assert_eq!(told["entry"]["tool"], "get_ticket");
    assert_eq!(told["entry"]["outcome"], "error");
}

#[test]
fn a_token_is_32_random_bytes_in_base64url() {
    let a = new_token();
    let b = new_token();
    assert_eq!(a.len(), 43, "{a}");
    assert!(a
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    assert_ne!(a, b);
}

#[tokio::test]
async fn the_external_token_is_kept_in_the_keychain_and_the_same_after_a_restart() {
    let h = harness("mcp-token-kept");
    let (_, token) = started(&h);
    h.core.mcp.stop();
    let store = crate::integrations::secrets::memory_of(&h.core.data);
    assert_eq!(
        store.entry(crate::integrations::secrets::MCP_ENTRY),
        Some(token.clone())
    );
    // Another core on the same data folder (the app started again): the same token.
    let app = mock_app();
    let (again, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    assert_eq!(again.mcp.external_token().unwrap(), token);
}
