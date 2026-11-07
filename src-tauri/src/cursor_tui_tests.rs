//! Integration tests of « Vue Cursor » (`tui`, `engine/cursor/terminal.rs`, ESC-49): the fake
//! `agent` plays Cursor's interface in a real terminal (tests/fixtures/fake-cursor-tui.mjs) and
//! writes its transcript, which the core reads as it comes; its `agent acp` takes the session back
//! in Escouade's view. Cursor keeps the two conversations apart: each side is told what was said in
//! the other (`handover::relay`).

use crate::core::AgentOptions;
use crate::core_tests::{harness, plain_text, Harness};
use crate::engine::fake_cursor;
use crate::model::*;
use crate::pty::TermInfo;
use parking_lot::Mutex;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Cursor's Principal on the fake `agent`, and an agent of the project on Auto.
async fn cursor_agent(h: &Harness) -> (String, PathBuf) {
    let (p, repo) = h.project(false).await;
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "principal-cursor".into(),
            name: "Principal".into(),
            engine: EngineId::CURSOR,
            claude_path: fake_cursor(),
            active: true,
            ..Default::default()
        });
    });
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::CURSOR),
                model: Some("default".into()),
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
async fn open_cursor(h: &Harness, id: &str, first: &str) -> (TermInfo, Arc<Mutex<String>>) {
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

fn git_status(repo: &Path) -> String {
    let out = std::process::Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all"])
        .current_dir(repo)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[tokio::test(flavor = "multi_thread")]
async fn cursors_view_follows_its_turns_and_each_side_is_told_what_was_said_in_the_other() {
    let h = harness("tui-cursor-view");
    let (id, repo) = cursor_agent(&h).await;
    let turns = |h: &Harness| items_of(h, &id, "turn").len();
    let home = h.core.data.agent_home(&id);

    // A first turn in Escouade's view, on its ACP session.
    h.core
        .send_message(&id, "Premier".into(), Vec::new())
        .await
        .unwrap();
    h.wait("the stream's first turn over", |h| {
        h.agent(&id).status == AgentStatus::Done && turns(h) == 1
    })
    .await;
    let session = h.agent(&id).session_id.unwrap();

    // Into the Cursor view: its interface opens on the session's id, in the agent's own config
    // folder, never Git Bash; the folder is new to it, so it asks about its trust first, and no
    // message is pasted meanwhile.
    h.core.set_agent_view(&id, true).await.unwrap();
    let (info, screen) = open_cursor(&h, &id, "fake-cursor-trust").await;
    let launch = jsonl(&home.join("fake-tui-launches.jsonl")).pop().unwrap();
    let argv: Vec<String> = serde_json::from_value(launch["argv"].clone()).unwrap();
    let flag = |k: &str| argv.windows(2).find(|w| w[0] == k).map(|w| w[1].clone());
    assert_eq!(
        flag("--resume").as_deref(),
        Some(session.as_str()),
        "{argv:?}"
    );
    assert_eq!(flag("--model").as_deref(), Some("auto"));
    assert_eq!(launch["configDir"], home.to_string_lossy().as_ref());
    assert!(launch["msystem"].as_str().unwrap_or_default().is_empty());
    // The conversation so far, in a rule of its folder, kept out of git.
    let relay = launch["relay"].as_str().expect("the relay rule");
    assert!(relay.contains("alwaysApply: true"), "{relay}");
    assert!(
        relay.contains("Premier") && relay.contains("Cursor a reçu : Premier"),
        "{relay}"
    );
    let rule = repo
        .join(".cursor")
        .join("rules")
        .join("escouade-relais.mdc");
    assert!(rule.is_file());
    assert_eq!(git_status(&repo), "", "the rule is kept out of git");
    let refused = h
        .core
        .send_message(&id, "Trop tôt".into(), Vec::new())
        .await
        .unwrap_err();
    assert!(refused.to_string().contains("Cursor"), "{refused}");
    h.core.pty.write(&info.id, b"a").unwrap();
    wait_screen(&h, &info, &screen, "fake-cursor-tui").await;

    // A message: the card says it works, then that it is done; the conversation is the
    // transcript's.
    h.core
        .send_message(&id, "Bonjour la vue".into(), Vec::new())
        .await
        .unwrap();
    h.wait("the interface's turn over", |h| {
        h.agent(&id).status == AgentStatus::Done && turns(h) == 2
    })
    .await;
    assert_eq!(users(&h, &id), ["Premier", "Bonjour la vue"]);
    assert!(items_of(&h, &id, "text")
        .iter()
        .any(|t| t["text"] == "Fini : Bonjour la vue"));

    // A command: shown done, under Claude Code's name.
    h.core
        .send_message(&id, "commande".into(), Vec::new())
        .await
        .unwrap();
    h.wait("its command shown", |h| {
        turns(h) == 3
            && items_of(h, &id, "tool")
                .iter()
                .any(|t| t["name"] == "Bash" && t["input"]["command"] == "npm test")
    })
    .await;

    // Interrupted from the window (Ctrl+C): the turn ends as interrupted.
    h.core
        .send_message(&id, "lent".into(), Vec::new())
        .await
        .unwrap();
    h.wait("working on its message", |h| {
        h.agent(&id).status == AgentStatus::Running
            && users(h, &id).last().is_some_and(|u| u == "lent")
    })
    .await;
    h.core.interrupt(&id).await.unwrap();
    h.wait("interrupted", |h| {
        items_of(h, &id, "turn")
            .last()
            .is_some_and(|t| t["interrupted"] == true)
    })
    .await;

    // Back to Escouade's view: the rule is gone, and what was said in the interface goes with the
    // next message to the session, which resumes.
    h.core.set_agent_view(&id, false).await.unwrap();
    assert!(!rule.exists());
    // The folders made for it go with it.
    assert!(!repo.join(".cursor").exists());
    assert!(h.agent(&id).relay_made.is_empty());
    let told = h.agent(&id).relay.expect("what to tell the session");
    assert!(told.contains("Bonjour la vue") && told.contains("Fini : Bonjour la vue"));
    assert!(!told.contains("Premier"), "only what came after: {told}");
    h.core
        .send_message(&id, "Et maintenant ?".into(), Vec::new())
        .await
        .unwrap();
    wait_conv(&h, &id, "stream's turn", |h| {
        h.agent(&id).status == AgentStatus::Done && turns(h) == 5
    })
    .await;
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(session.as_str()));
    assert!(h.agent(&id).relay.is_none(), "told once");
    let kept: Value = serde_json::from_str(
        &std::fs::read_to_string(home.join("acp-sessions").join(format!("{session}.json")))
            .unwrap(),
    )
    .unwrap();
    let prompt = kept["history"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|u| u["sessionUpdate"] == "user_message_chunk")
        .filter_map(|u| u["content"]["text"].as_str())
        .next_back()
        .unwrap()
        .to_string();
    assert!(prompt.contains("<relais_escouade>"), "{prompt}");
    assert!(prompt.contains("Bonjour la vue") && prompt.ends_with("Et maintenant ?"));
    assert_eq!(
        users(&h, &id),
        [
            "Premier",
            "Bonjour la vue",
            "commande",
            "lent",
            "Et maintenant ?"
        ]
    );

    // Into the view again: the same conversation of the interface, the trust remembered, and the
    // rule tells what was said up to now, the interface's turns among them.
    h.core.set_agent_view(&id, true).await.unwrap();
    open_cursor(&h, &id, "fake-cursor-tui").await;
    let launch = jsonl(&home.join("fake-tui-launches.jsonl")).pop().unwrap();
    let relay = launch["relay"].as_str().unwrap();
    assert!(relay.contains("Et maintenant ?") && relay.contains("Bonjour la vue"));
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert_eq!(users(&h, &id).len(), 5, "nothing read twice");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_agent_in_cursors_view_names_its_conversation_and_writes_no_rule() {
    let h = harness("tui-cursor-new");
    h.core.settings.write().terminal_view = true;
    let (id, repo) = cursor_agent(&h).await;
    assert!(h.agent(&id).terminal);
    let (info, screen) = open_cursor(&h, &id, "fake-cursor-trust").await;
    h.core.pty.write(&info.id, b"a").unwrap();
    wait_screen(&h, &info, &screen, "fake-cursor-tui").await;
    let session = h.agent(&id).session_id.expect("named by Escouade");
    // Nothing said yet: no rule.
    assert!(!repo.join(".cursor").exists());
    h.core
        .send_message(&id, "Salut".into(), Vec::new())
        .await
        .unwrap();
    h.wait("its turn over", |h| {
        h.agent(&id).status == AgentStatus::Done && !items_of(h, &id, "turn").is_empty()
    })
    .await;
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(session.as_str()));
    assert_eq!(users(&h, &id), ["Salut"]);
    // Back in Escouade's view, its ACP session is a new one (Cursor's interface and ACP keep their
    // conversations apart): it is told what was said.
    h.core.set_agent_view(&id, false).await.unwrap();
    assert!(h.agent(&id).relay.unwrap().contains("Salut"));
    h.core
        .send_message(&id, "Et là ?".into(), Vec::new())
        .await
        .unwrap();
    h.wait("the stream's turn over", |h| {
        h.agent(&id).status == AgentStatus::Done && items_of(h, &id, "turn").len() == 2
    })
    .await;
    assert!(h.agent(&id).relay.is_none());
    assert!(items_of(&h, &id, "text")
        .iter()
        .any(|t| t["text"].as_str().is_some_and(|x| x.contains("Et là ?"))));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_relay_left_by_an_app_that_stopped_goes_at_the_next_start_and_a_folder_of_the_users_stays(
) {
    let h = harness("tui-cursor-stale");
    let (id, repo) = cursor_agent(&h).await;
    h.core
        .send_message(&id, "Premier".into(), Vec::new())
        .await
        .unwrap();
    h.wait("its first turn over", |h| {
        h.agent(&id).status == AgentStatus::Done && !items_of(h, &id, "turn").is_empty()
    })
    .await;
    // The user's own rules: their folder is theirs.
    let rules = repo.join(".cursor").join("rules");
    std::fs::create_dir_all(&rules).unwrap();
    std::fs::write(rules.join("mine.mdc"), "x").unwrap();
    h.core.set_agent_view(&id, true).await.unwrap();
    open_cursor(&h, &id, "fake-cursor-trust").await;
    let rule = rules.join("escouade-relais.mdc");
    assert!(rule.is_file());
    assert!(
        h.agent(&id).relay_made.is_empty(),
        "nothing made: {:?}",
        h.agent(&id).relay_made
    );
    h.core.set_agent_view(&id, false).await.unwrap();
    assert!(!rule.exists());
    assert!(rules.join("mine.mdc").is_file());

    // Its own folders, made for the relay; then the app stops with the view open: the rule stays,
    // and the agent comes back out of the view (as after a restart in Escouade's view).
    std::fs::remove_dir_all(repo.join(".cursor")).unwrap();
    h.core.set_agent_view(&id, true).await.unwrap();
    open_cursor(&h, &id, "fake-cursor-trust").await;
    assert!(rule.is_file());
    assert_eq!(h.agent(&id).relay_made.len(), 2);
    {
        let a = h.core.agent(&id).unwrap();
        let mut rt = a.lock();
        let mut fx = crate::agent::Effects::default();
        rt.stop_tui(&mut fx);
        rt.meta.terminal = false;
    }
    assert!(rule.is_file(), "left behind");
    // Its next start takes it away, and the folders it made.
    h.core
        .send_message(&id, "Ensuite".into(), Vec::new())
        .await
        .unwrap();
    assert!(!rule.exists());
    assert!(!repo.join(".cursor").exists());
    assert!(h.agent(&id).relay_made.is_empty());
}

/// The last text the agent said.
fn last_text(h: &Harness, id: &str) -> String {
    items_of(h, id, "text")
        .last()
        .and_then(|t| t["text"].as_str().map(str::to_string))
        .unwrap_or_default()
}

/// Real run (ESC-49), by hand: « Vue Cursor » with the real Cursor CLI (`ESCOUADE_REAL_CURSOR`, its
/// `agent.cmd`) on the user's account, signed in already (`agent login`, never done here), its
/// folder of the workspaces the real one (`ESCOUADE_CURSOR_PROJECTS`: `~/.cursor/projects`, where
/// the interface writes its transcript and the trust of the test's folder, taken away at the end):
/// `cargo test --lib real_cursor_view -- --ignored --nocapture`. The account pays: three short
/// turns on Auto. A word given in Escouade's view is known in the interface (the rule Escouade
/// writes), and a word given in the interface is known back in Escouade's view (the relay).
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn real_cursor_view() {
    let real =
        std::env::var("ESCOUADE_REAL_CURSOR").expect("ESCOUADE_REAL_CURSOR: the path of agent.cmd");
    let projects = PathBuf::from(
        std::env::var(crate::engine::cursor::terminal::PROJECTS_VAR)
            .expect("ESCOUADE_CURSOR_PROJECTS: ~/.cursor/projects"),
    );
    let h = harness("esc49-real");
    let (p, repo) = h.project(false).await;
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "principal-cursor".into(),
            name: "Principal".into(),
            engine: EngineId::CURSOR,
            claude_path: real,
            active: true,
            ..Default::default()
        });
    });
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::CURSOR),
                model: Some("default".into()),
                mode: Some("default".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    let done = |h: &Harness, n: usize| {
        !h.agent(&id).status.is_active() && items_of(h, &id, "turn").len() == n
    };
    let long = |secs| async move { tokio::time::sleep(Duration::from_secs(secs)).await };

    // In Escouade's view, a word to keep.
    h.core
        .send_message(
            &id,
            "Retiens le mot de code KIWI. Réponds seulement : OK".into(),
            vec![],
        )
        .await
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(120);
    while !done(&h, 1) {
        assert!(
            Instant::now() < deadline,
            "{:#}",
            Value::Array(h.items(&id))
        );
        long(1).await;
    }
    println!("ACP: {}", last_text(&h, &id));

    // Into the Cursor view: its question about the folder answered as the user would.
    h.core.set_agent_view(&id, true).await.unwrap();
    let screen = Arc::new(Mutex::new(String::new()));
    let sink = screen.clone();
    let info = h
        .core
        .open_agent_terminal(&id, (120, 40), false, move |b| {
            sink.lock().push_str(&String::from_utf8_lossy(&b))
        })
        .await
        .unwrap();
    let start = Instant::now();
    let mut answered = false;
    loop {
        if !answered && screen.lock().contains("\x1b[6n") {
            h.core.pty.write(&info.id, b"\x1b[1;1R").unwrap();
            answered = true;
        }
        let text = plain_text(&screen.lock());
        if text.contains("Trust this workspace") {
            let refused = h.core.send_message(&id, "x".into(), Vec::new()).await;
            println!("message while it asks: {refused:?}");
            assert!(refused.is_err());
            tokio::time::sleep(Duration::from_millis(800)).await;
            h.core.pty.write(&info.id, b"a").unwrap();
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(60), "{text}");
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let start = Instant::now();
    while !plain_text(&screen.lock()).contains("Auto") || start.elapsed() < Duration::from_secs(8) {
        assert!(start.elapsed() < Duration::from_secs(60));
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    // The word given in Escouade's view, known in the interface; a word to keep given there.
    let mut sent = false;
    for _ in 0..5 {
        if h.core
            .send_message(
                &id,
                "Quel est le mot de code ? Retiens aussi le mot de passe ANANAS. Réponds seulement par le mot de code, sans outil.".into(),
                vec![],
            )
            .await
            .is_ok()
        {
            sent = true;
            break;
        }
        long(2).await;
    }
    assert!(sent);
    let deadline = Instant::now() + Duration::from_secs(120);
    while !done(&h, 2) {
        assert!(
            Instant::now() < deadline,
            "{} {:#}",
            plain_text(&screen.lock()),
            Value::Array(h.items(&id))
        );
        long(1).await;
    }
    let said = last_text(&h, &id);
    println!("interface: {said}");
    assert!(said.contains("KIWI"), "{said}");

    // Back in Escouade's view: the word given in the interface, known.
    h.core.set_agent_view(&id, false).await.unwrap();
    h.core
        .send_message(
            &id,
            "Quel mot de passe t'ai-je donné dans l'interface de Cursor ? Réponds seulement par ce mot."
                .into(),
            vec![],
        )
        .await
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(120);
    while !done(&h, 3) {
        assert!(
            Instant::now() < deadline,
            "{:#}",
            Value::Array(h.items(&id))
        );
        long(1).await;
    }
    let said = last_text(&h, &id);
    println!("ACP back: {said}");
    assert!(said.contains("ANANAS"), "{said}");
    assert_eq!(git_status(&repo), "");

    // What the trial left in the folder of the workspaces: its own folder's, taken away.
    let key = crate::engine::cursor::terminal::project_key(&repo.to_string_lossy());
    let own = projects.join(&key);
    println!("taking away {}", own.display());
    let _ = std::fs::remove_dir_all(own);
}
