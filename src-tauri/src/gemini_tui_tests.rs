//! Integration tests of « Vue Gemini » (`tui`, `engine/gemini/terminal.rs`, ESC-275): the fake
//! `gemini` plays Gemini CLI's interface in a real terminal (tests/fixtures/fake-gemini-tui.mjs) and
//! runs its hooks, which tell the core the agent's turns; its `gemini --acp` takes the session back
//! in Escouade's view, from the same store.

use crate::core::AgentOptions;
use crate::core_tests::{harness, plain_text, Harness};
use crate::engine::gemini::bench::fake_gemini;
use crate::model::*;
use crate::pty::TermInfo;
use parking_lot::Mutex;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Gemini's account of the machine on the fake `gemini`, and an agent of the project in « Demander ».
async fn gemini_agent(h: &Harness) -> (String, std::path::PathBuf) {
    let (p, repo) = h.project(false).await;
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "principal-gemini".into(),
            name: "Principal".into(),
            engine: EngineId::GEMINI,
            claude_path: fake_gemini(),
            active: true,
            ..Default::default()
        });
    });
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::GEMINI),
                model: Some("gemini-3.8-flash".into()),
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
async fn open_gemini(h: &Harness, id: &str, first: &str) -> (TermInfo, Arc<Mutex<String>>) {
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

/// The agent's settings, in its home.
fn settings_of(home: &Path) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(home.join(".gemini").join("settings.json")).unwrap_or_default(),
    )
    .unwrap_or(Value::Null)
}

#[tokio::test(flavor = "multi_thread")]
async fn geminis_view_resumes_the_agents_session_follows_its_turns_by_its_hooks_and_hands_it_back()
{
    let h = harness("tui-gemini-view");
    h.core.set_mcp_enabled(true).unwrap();
    h.wait("the server", |h| h.core.mcp.status().running).await;
    let (id, repo) = gemini_agent(&h).await;
    let turns = |h: &Harness| items_of(h, &id, "turn").len();
    let home = h.core.data.agent_home(&id);

    // A first turn in Escouade's view, by `gemini --acp`.
    h.core
        .send_message(&id, "Premier".into(), Vec::new())
        .await
        .unwrap();
    wait_conv(&h, &id, "the process's first turn", |h| {
        h.agent(&id).status == AgentStatus::Done && turns(h) == 1
    })
    .await;
    let session = h.agent(&id).session_id.unwrap();

    // Into the Gemini view: its interface resumes the same session, from the same store, in the
    // agent's own home, with Escouade's server in its settings, never on its command line.
    h.core.set_agent_view(&id, true).await.unwrap();
    let (info, screen) = open_gemini(&h, &id, &format!("fake-gemini-tui {session}")).await;
    let launch = jsonl(&home.join("fake-tui-launches.jsonl")).pop().unwrap();
    let argv: Vec<String> = serde_json::from_value(launch["argv"].clone()).unwrap();
    let flag = |k: &str| argv.windows(2).find(|w| w[0] == k).map(|w| w[1].clone());
    assert_eq!(
        flag("--resume").as_deref(),
        Some(session.as_str()),
        "{argv:?}"
    );
    // Its router (`auto`), its default, is not named.
    let model = h.agent(&id).model;
    let named = (model != "auto").then_some(model.as_str());
    assert_eq!(flag("--model").as_deref(), named, "{argv:?}");
    assert_eq!(flag("--approval-mode").as_deref(), Some("default"));
    assert_eq!(launch["home"], home.to_string_lossy().as_ref());
    assert_eq!(launch["trust"], true);
    let settings = &launch["settings"];
    let header = settings["mcpServers"]["escouade"]["headers"]["Authorization"]
        .as_str()
        .expect("its settings");
    let token = header.strip_prefix("Bearer ").unwrap();
    assert!(!token.is_empty());
    assert!(!argv.iter().any(|a| a.contains(token)), "{argv:?}");
    assert_eq!(settings["hooksConfig"]["enabled"], true);
    for event in crate::engine::gemini::terminal::EVENTS {
        assert!(settings["hooks"][event].is_array(), "{event}");
    }
    let allowed = settings["security"]["environmentVariableRedaction"]["allowed"].clone();
    assert!(allowed
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == crate::tui::TOKEN_VAR));

    // A message: the card says it works, then that it is done; the conversation is its hooks'.
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

    // A command: shown under Claude Code's name, done.
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

    // A permission in its interface: the card waits, until it is answered there.
    h.core
        .send_message(&id, "permission".into(), Vec::new())
        .await
        .unwrap();
    wait_conv(&h, &id, "waiting", |h| {
        h.agent(&id).status == AgentStatus::Waiting
    })
    .await;
    let shown = Instant::now();
    while !plain_text(&screen.lock()).contains("Allow execution") {
        assert!(shown.elapsed() < Duration::from_secs(30), "no permission");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    h.core.pty.write(&info.id, b"\r").unwrap();
    wait_conv(&h, &id, "the permission's turn", |h| {
        h.agent(&id).status == AgentStatus::Done && turns(h) == 4
    })
    .await;

    // Interrupted from the window (Escape): no hook tells it, the turn ends all the same.
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

    // Back to Escouade's view: the same session, resumed by `gemini --acp`, whose settings no
    // longer have its view's hooks nor Escouade's server.
    h.core.set_agent_view(&id, false).await.unwrap();
    h.core
        .send_message(&id, "Et maintenant ?".into(), Vec::new())
        .await
        .unwrap();
    wait_conv(&h, &id, "the process's turn", |h| {
        h.agent(&id).status == AgentStatus::Done && turns(h) == 6
    })
    .await;
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(session.as_str()));
    let after = settings_of(&home);
    assert!(after["hooks"].is_null() && after["mcpServers"]["escouade"].is_null());
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
    assert!(!repo.join(".gemini").exists());
    h.core.mcp.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_agent_in_geminis_view_starts_a_session_under_the_id_escouade_gives_it() {
    let h = harness("tui-gemini-new");
    h.core.settings.write().terminal_view = true;
    let (id, _repo) = gemini_agent(&h).await;
    assert!(h.agent(&id).terminal);
    open_gemini(&h, &id, "fake-gemini-tui").await;
    let home = h.core.data.agent_home(&id);
    let launch = jsonl(&home.join("fake-tui-launches.jsonl")).pop().unwrap();
    let argv: Vec<String> = serde_json::from_value(launch["argv"].clone()).unwrap();
    let named = argv
        .windows(2)
        .find(|w| w[0] == "--session-id")
        .map(|w| w[1].clone())
        .expect("a session named by Escouade");
    assert!(!argv.iter().any(|a| a == "--resume"), "{argv:?}");
    // Escouade's server refused (its switch off): none given.
    assert!(launch["settings"]["mcpServers"]["escouade"].is_null());
    h.core
        .send_message(&id, "Salut".into(), Vec::new())
        .await
        .unwrap();
    wait_conv(&h, &id, "its turn", |h| {
        h.agent(&id).status == AgentStatus::Done && !items_of(h, &id, "turn").is_empty()
    })
    .await;
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(named.as_str()));
    assert_eq!(users(&h, &id), ["Salut"]);
    // Back in Escouade's view, `gemini --acp` resumes that session.
    h.core.set_agent_view(&id, false).await.unwrap();
    h.core
        .send_message(&id, "Et là ?".into(), Vec::new())
        .await
        .unwrap();
    wait_conv(&h, &id, "the process's turn", |h| {
        h.agent(&id).status == AgentStatus::Done && items_of(h, &id, "turn").len() == 2
    })
    .await;
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(named.as_str()));
}
