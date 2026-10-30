//! Integration tests of « Vue Vibe » (`tui`, `engine/vibe/terminal.rs`, ESC-268): the fake `vibe`
//! plays Mistral Vibe's interface in a real terminal (tests/fixtures/fake-vibe-tui.mjs) and writes
//! the journal of its session's store, which the core reads as it comes, from one segment to the
//! next; its `vibe-acp` takes the session back in Escouade's view, from the same store.

use crate::core::AgentOptions;
use crate::core_tests::{harness, plain_text, Harness};
use crate::engine::vibe::bench::fake_vibe;
use crate::model::*;
use crate::pty::TermInfo;
use parking_lot::Mutex;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Mistral Vibe's Principal on the fake `vibe`, and an agent of the project in « Demander ».
async fn vibe_agent(h: &Harness) -> (String, PathBuf) {
    let (p, repo) = h.project(false).await;
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "principal-vibe".into(),
            name: "Principal".into(),
            engine: EngineId::VIBE,
            claude_path: fake_vibe(),
            active: true,
            ..Default::default()
        });
    });
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::VIBE),
                model: Some("mistral-medium-3.5".into()),
                mode: Some("default".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    (id, repo)
}

/// The interface of the agent opened as the window does, and what its terminal shows, once it
/// shows `first`.
async fn open_vibe(h: &Harness, id: &str, first: &str) -> (TermInfo, Arc<Mutex<String>>) {
    let screen = Arc::new(Mutex::new(String::new()));
    let sink = screen.clone();
    let info = h
        .core
        .open_agent_terminal(id, (100, 30), false, move |b| {
            sink.lock().push_str(&String::from_utf8_lossy(&b))
        })
        .await
        .unwrap();
    wait_screen(h, &info, &screen, first).await;
    (info, screen)
}

async fn wait_screen(h: &Harness, info: &TermInfo, screen: &Mutex<String>, what: &str) {
    let start = Instant::now();
    let mut answered = false;
    while !plain_text(&screen.lock()).contains(what) {
        // As xterm.js does: the pseudo-console (Windows) asks where the cursor is first.
        if !answered && screen.lock().contains("\x1b[6n") {
            h.core.pty.write(&info.id, b"\x1b[1;1R").unwrap();
            answered = true;
        }
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "{what} not in {:?}",
            screen.lock()
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn jsonl(file: &Path) -> Vec<Value> {
    std::fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn items_of(h: &Harness, id: &str, kind: &str) -> Vec<Value> {
    h.items(id)
        .into_iter()
        .filter(|i| i["kind"] == kind)
        .collect()
}

fn users(h: &Harness, id: &str) -> Vec<String> {
    items_of(h, id, "user")
        .iter()
        .map(|u| u["text"].as_str().unwrap().to_string())
        .collect()
}

/// `Harness::wait`, the conversation and the agent told when it times out.
async fn wait_conv(h: &Harness, id: &str, what: &str, pred: impl Fn(&Harness) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        if pred(h) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!(
        "no {what}: {:?} {:#}",
        h.agent(id).status,
        Value::Array(h.items(id))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn vibes_view_resumes_the_agents_session_follows_its_turns_and_hands_it_back() {
    let h = harness("tui-vibe-view");
    h.core.set_mcp_enabled(true).unwrap();
    h.wait("the server", |h| h.core.mcp.status().running).await;
    let (id, _repo) = vibe_agent(&h).await;
    let turns = |h: &Harness| items_of(h, &id, "turn").len();
    let home = h.core.data.agent_home(&id);

    // A first turn in Escouade's view, on its ACP session.
    h.core
        .send_message(&id, "Premier".into(), Vec::new())
        .await
        .unwrap();
    wait_conv(&h, &id, "the stream's first turn", |h| {
        h.agent(&id).status == AgentStatus::Done && turns(h) == 1
    })
    .await;
    let session = h.agent(&id).session_id.unwrap();

    // Into the Vibe view: its interface resumes the same session, from the same store, in the
    // agent's own VIBE_HOME, with Escouade's server in its variables, never its command line.
    h.core.set_agent_view(&id, true).await.unwrap();
    let (info, screen) = open_vibe(&h, &id, &format!("fake-vibe-tui {session}")).await;
    let launch = jsonl(&home.join("fake-tui-launches.jsonl")).pop().unwrap();
    let argv: Vec<String> = serde_json::from_value(launch["argv"].clone()).unwrap();
    let flag = |k: &str| argv.windows(2).find(|w| w[0] == k).map(|w| w[1].clone());
    assert_eq!(
        flag("--resume").as_deref(),
        Some(session.as_str()),
        "{argv:?}"
    );
    assert_eq!(flag("--agent").as_deref(), Some("ask"));
    assert_eq!(launch["home"], home.to_string_lossy().as_ref());
    assert_eq!(launch["telemetry"], "false");
    assert_eq!(launch["activeModel"], "mistral-medium-3.5");
    let servers: Value =
        serde_json::from_str(launch["mcpServers"].as_str().expect("its MCP servers")).unwrap();
    assert_eq!(servers[0]["name"], "escouade");
    assert_eq!(servers[0]["api_key_env"], "ESCOUADE_MCP_TOKEN");
    let token = launch["mcpToken"]
        .as_str()
        .expect("the token in its variable");
    assert!(!token.is_empty());
    assert!(!argv.iter().any(|a| a.contains(token)), "{argv:?}");
    assert!(!servers.to_string().contains(token));
    // Its hooks and the trust of its folder, in its VIBE_HOME.
    assert!(launch["hooks"].as_str().unwrap().contains("post_agent"));
    assert!(launch["trusted"].as_str().unwrap().contains("trusted = ["));
    assert!(home.join("hooks.toml").is_file());

    // A message: the card says it works, then that it is done; the conversation is the journal's.
    h.core
        .send_message(&id, "Bonjour la vue".into(), Vec::new())
        .await
        .unwrap();
    wait_conv(&h, &id, "the interface's turn", |h| {
        h.agent(&id).status == AgentStatus::Done && turns(h) == 2
    })
    .await;
    assert_eq!(users(&h, &id), ["Premier", "Bonjour la vue"]);
    assert!(items_of(&h, &id, "text")
        .iter()
        .any(|t| t["text"] == "Fini : Bonjour la vue"));

    // A command, in the next segment of its journal: shown under Claude Code's name, done; its
    // hooks ran.
    h.core
        .send_message(&id, "commande".into(), Vec::new())
        .await
        .unwrap();
    wait_conv(&h, &id, "its command", |h| {
        turns(h) == 3
            && items_of(h, &id, "tool").iter().any(|t| {
                t["name"] == "Bash" && t["input"]["command"] == "npm test" && t["status"] == "ok"
            })
    })
    .await;
    let hooks: Vec<String> = jsonl(&home.join("fake-tui-hooks.jsonl"))
        .iter()
        .map(|e| e["event"].as_str().unwrap().to_string())
        .collect();
    assert!(hooks.contains(&"pre_tool".to_string()) && hooks.contains(&"post_agent".to_string()));

    // A permission asked in the interface: the card waits, until it is answered there.
    h.core
        .send_message(&id, "permission".into(), Vec::new())
        .await
        .unwrap();
    wait_conv(&h, &id, "waiting", |h| {
        h.agent(&id).status == AgentStatus::Waiting
    })
    .await;
    assert!(plain_text(&screen.lock()).contains("fake-vibe-permission"));
    h.core.pty.write(&info.id, b"\r").unwrap();
    wait_conv(&h, &id, "the permission's turn", |h| {
        h.agent(&id).status == AgentStatus::Done && turns(h) == 4
    })
    .await;

    // Interrupted from the window (Escape): the turn ends as interrupted.
    h.core
        .send_message(&id, "lent".into(), Vec::new())
        .await
        .unwrap();
    wait_conv(&h, &id, "working on its message", |h| {
        h.agent(&id).status == AgentStatus::Running
            && users(h, &id).last().is_some_and(|u| u == "lent")
    })
    .await;
    h.core.interrupt(&id).await.unwrap();
    wait_conv(&h, &id, "interrupted", |h| {
        items_of(h, &id, "turn")
            .last()
            .is_some_and(|t| t["interrupted"] == true)
    })
    .await;

    // Back to Escouade's view: the same session, resumed by its ACP process, whose hooks are gone
    // from its VIBE_HOME; nothing was lost.
    h.core.set_agent_view(&id, false).await.unwrap();
    h.core
        .send_message(&id, "Et maintenant ?".into(), Vec::new())
        .await
        .unwrap();
    wait_conv(&h, &id, "the stream's turn", |h| {
        h.agent(&id).status == AgentStatus::Done && turns(h) == 6
    })
    .await;
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(session.as_str()));
    assert!(!home.join("hooks.toml").exists());
    assert_eq!(
        users(&h, &id),
        [
            "Premier",
            "Bonjour la vue",
            "commande",
            "permission",
            "lent",
            "Et maintenant ?"
        ]
    );

    // Into the view again: the same session, nothing read twice.
    h.core.set_agent_view(&id, true).await.unwrap();
    open_vibe(&h, &id, &format!("fake-vibe-tui {session}")).await;
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert_eq!(users(&h, &id).len(), 6, "nothing read twice");
    h.core.mcp.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_agent_in_vibes_view_takes_the_session_its_interface_makes_from_its_hooks() {
    let h = harness("tui-vibe-new");
    h.core.settings.write().terminal_view = true;
    let (id, _repo) = vibe_agent(&h).await;
    assert!(h.agent(&id).terminal);
    open_vibe(&h, &id, "fake-vibe-tui").await;
    let home = h.core.data.agent_home(&id);
    let launch = jsonl(&home.join("fake-tui-launches.jsonl")).pop().unwrap();
    let argv: Vec<String> = serde_json::from_value(launch["argv"].clone()).unwrap();
    assert!(!argv.iter().any(|a| a == "--resume"), "{argv:?}");
    // Escouade's server refused (its switch off): none given.
    assert!(launch["mcpServers"].is_null());
    h.core
        .send_message(&id, "Salut".into(), Vec::new())
        .await
        .unwrap();
    wait_conv(&h, &id, "its turn", |h| {
        h.agent(&id).status == AgentStatus::Done && !items_of(h, &id, "turn").is_empty()
    })
    .await;
    let session = h.agent(&id).session_id.expect("told by its hook");
    assert!(home
        .join("logs")
        .join("session")
        .join("unified")
        .join(&session)
        .is_dir());
    assert_eq!(users(&h, &id), ["Salut"]);
    // Back in Escouade's view, its ACP process resumes that session.
    h.core.set_agent_view(&id, false).await.unwrap();
    h.core
        .send_message(&id, "Et là ?".into(), Vec::new())
        .await
        .unwrap();
    wait_conv(&h, &id, "the stream's turn", |h| {
        h.agent(&id).status == AgentStatus::Done && items_of(h, &id, "turn").len() == 2
    })
    .await;
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(session.as_str()));
}
