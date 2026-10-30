//! Integration tests of « Vue Antigravity » (`tui`, `engine/agy/terminal.rs`, ESC-272): the fake
//! `agy` plays Antigravity CLI's interface in a real terminal (tests/fixtures/fake-agy-tui.mjs) and
//! writes the transcript of the agent's conversation, which the core reads as it comes; its `agy -p`
//! takes the conversation back in Escouade's view, from the same store.

use crate::core::AgentOptions;
use crate::core_tests::{harness, plain_text, Harness};
use crate::engine::agy::bench::fake_agy;
use crate::model::*;
use crate::pty::TermInfo;
use parking_lot::Mutex;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Antigravity's Principal on the fake `agy`, and an agent of the project in « Demander ».
async fn agy_agent(h: &Harness) -> (String, PathBuf) {
    let (p, repo) = h.project(false).await;
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "principal-agy".into(),
            name: "Principal".into(),
            engine: EngineId::AGY,
            claude_path: fake_agy(),
            active: true,
            ..Default::default()
        });
    });
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::AGY),
                model: Some("gemini-3.8-flash".into()),
                effort: Some("low".into()),
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
async fn open_agy(h: &Harness, id: &str, first: &str) -> (TermInfo, Arc<Mutex<String>>) {
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

/// The hooks file of the agent's home.
fn hooks_of(home: &Path) -> PathBuf {
    home.join(".gemini").join("config").join("hooks.json")
}

#[tokio::test(flavor = "multi_thread")]
async fn agys_view_resumes_the_agents_conversation_follows_its_turns_and_hands_it_back() {
    let h = harness("tui-agy-view");
    h.core.set_mcp_enabled(true).unwrap();
    h.wait("the server", |h| h.core.mcp.status().running).await;
    let (id, repo) = agy_agent(&h).await;
    let turns = |h: &Harness| items_of(h, &id, "turn").len();
    let home = h.core.data.agent_home(&id);

    // A first turn in Escouade's view, by `agy -p`.
    h.core
        .send_message(&id, "Premier".into(), Vec::new())
        .await
        .unwrap();
    wait_conv(&h, &id, "the stream's first turn", |h| {
        h.agent(&id).status == AgentStatus::Done && turns(h) == 1
    })
    .await;
    let session = h.agent(&id).session_id.unwrap();

    // Into the Antigravity view: its interface resumes the same conversation, from the same
    // store, in the agent's own home, with Escouade's server in its MCP config, never on its
    // command line; its first screens passed.
    h.core.set_agent_view(&id, true).await.unwrap();
    let (info, screen) = open_agy(&h, &id, &format!("fake-agy-tui {session}")).await;
    let launch = jsonl(&home.join("fake-tui-launches.jsonl")).pop().unwrap();
    let argv: Vec<String> = serde_json::from_value(launch["argv"].clone()).unwrap();
    let flag = |k: &str| argv.windows(2).find(|w| w[0] == k).map(|w| w[1].clone());
    assert_eq!(
        flag("--conversation").as_deref(),
        Some(session.as_str()),
        "{argv:?}"
    );
    assert_eq!(flag("--model").as_deref(), Some("gemini-3.8-flash"));
    assert_eq!(flag("--effort").as_deref(), Some("low"));
    assert_eq!(launch["home"], home.to_string_lossy().as_ref());
    assert_eq!(launch["hideAccount"], "1");
    let server = &launch["mcp"]["mcpServers"]["escouade"];
    let header = server["headers"]["Authorization"]
        .as_str()
        .expect("its MCP config");
    let token = header.strip_prefix("Bearer ").unwrap();
    assert!(!token.is_empty());
    assert!(!argv.iter().any(|a| a.contains(token)), "{argv:?}");
    assert!(launch["hooks"]["escouade"]["Stop"].is_array());
    let cwd = h.agent(&id).cwd;
    assert!(launch["settings"]["trustedWorkspaces"]
        .as_array()
        .unwrap()
        .iter()
        .any(|w| w == cwd.as_str()));
    assert_eq!(launch["onboarding"]["onboardingComplete"], true);
    assert!(!plain_text(&screen.lock()).contains("Do you trust"));

    // A message: the card says it works, then that it is done; the conversation is the
    // transcript's.
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

    // A command: shown under Claude Code's name, done; its hooks ran.
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
    assert!(
        hooks.contains(&"PreToolUse".to_string()) && hooks.contains(&"Stop".to_string()),
        "{hooks:?}"
    );

    // A question of its own in the interface: the card waits, until it is answered there.
    h.core
        .send_message(&id, "question".into(), Vec::new())
        .await
        .unwrap();
    wait_conv(&h, &id, "waiting", |h| {
        h.agent(&id).status == AgentStatus::Waiting
    })
    .await;
    // Its transcript tells the call before its screen shows the question, as `agy`'s.
    let shown = Instant::now();
    while !plain_text(&screen.lock()).contains("Bleu ou rouge") {
        assert!(
            shown.elapsed() < Duration::from_secs(30),
            "no question on its screen"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    h.core.pty.write(&info.id, b"\r").unwrap();
    wait_conv(&h, &id, "the question's turn", |h| {
        h.agent(&id).status == AgentStatus::Done && turns(h) == 4
    })
    .await;

    // Interrupted from the window (Escape): the turn ends with what it wrote.
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
    wait_conv(&h, &id, "the stopped turn", |h| {
        h.agent(&id).status != AgentStatus::Running && turns(h) == 5
    })
    .await;

    // Back to Escouade's view: the same conversation, resumed by `agy -p`, whose hooks are gone
    // from its home; nothing was lost.
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
    assert!(!hooks_of(&home).exists());
    assert_eq!(
        users(&h, &id),
        [
            "Premier",
            "Bonjour la vue",
            "commande",
            "question",
            "lent",
            "Et maintenant ?"
        ]
    );
    // Nothing of Escouade's in the project.
    assert!(!repo.join(".agents").exists());

    // Into the view again: the same conversation, nothing read twice.
    h.core.set_agent_view(&id, true).await.unwrap();
    open_agy(&h, &id, &format!("fake-agy-tui {session}")).await;
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert_eq!(users(&h, &id).len(), 6, "nothing read twice");
    h.core.mcp.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_agent_in_agys_view_takes_the_conversation_its_interface_makes_from_its_hooks() {
    let h = harness("tui-agy-new");
    h.core.settings.write().terminal_view = true;
    let (id, _repo) = agy_agent(&h).await;
    assert!(h.agent(&id).terminal);
    open_agy(&h, &id, "fake-agy-tui").await;
    let home = h.core.data.agent_home(&id);
    let launch = jsonl(&home.join("fake-tui-launches.jsonl")).pop().unwrap();
    let argv: Vec<String> = serde_json::from_value(launch["argv"].clone()).unwrap();
    assert!(!argv.iter().any(|a| a == "--conversation"), "{argv:?}");
    // Escouade's server refused (its switch off): none given.
    assert!(launch["mcp"]["mcpServers"]["escouade"].is_null());
    h.core
        .send_message(&id, "Salut".into(), Vec::new())
        .await
        .unwrap();
    wait_conv(&h, &id, "its turn", |h| {
        h.agent(&id).status == AgentStatus::Done && !items_of(h, &id, "turn").is_empty()
    })
    .await;
    let session = h.agent(&id).session_id.expect("told by its hook");
    assert_eq!(users(&h, &id), ["Salut"]);
    // Back in Escouade's view, `agy -p` resumes that conversation.
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
