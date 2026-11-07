//! Integration tests of « Vue Copilot » (`tui`, `engine/copilot/terminal.rs`): the fake `copilot`
//! plays Copilot's interface in a real terminal (tests/fixtures/fake-copilot-tui.mjs), runs the
//! hooks of the plugin Escouade gives it (`escouade-hook <event>`) and writes its session's
//! `events.jsonl`, which the core reads as it comes; its server takes the session back in
//! Escouade's view.

use crate::core::AgentOptions;
use crate::core_tests::{harness, plain_text, Harness};
use crate::engine::fake_copilot;
use crate::model::*;
use crate::pty::TermInfo;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A Copilot account of its own, its `COPILOT_HOME` in the test's folder, on the fake `copilot`;
/// with `trusted`, its config already trusts the project `repo` (« Yes, and remember this folder »).
fn copilot_account(h: &Harness, repo: &Path, trusted: bool) -> PathBuf {
    let home = h.dir.join("copilot-cp");
    std::fs::create_dir_all(&home).unwrap();
    if trusted {
        let config =
            json!({ "firstLaunchAt": "2026-10-18T09:59:47.522Z", "trustedFolders": [repo] });
        std::fs::write(
            home.join("config.json"),
            format!("// User settings belong in settings.json.\n// This file is managed automatically.\n{config:#}\n"),
        )
        .unwrap();
    }
    let mut s = h.core.settings.read().clone();
    s.accounts.push(Account {
        id: "cp".into(),
        name: "Copilot".into(),
        engine: EngineId::COPILOT,
        config_dir: home.to_string_lossy().into(),
        claude_path: fake_copilot(),
        ..Default::default()
    });
    h.core.save_settings(s).unwrap();
    home
}

/// An agent on Copilot, in Escouade's view, and the `COPILOT_HOME` of its account.
async fn new_copilot_agent(h: &Harness, trusted: bool) -> (String, PathBuf) {
    let (p, repo) = h.project(false).await;
    let home = copilot_account(h, &repo, trusted);
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::COPILOT),
                account: Some("cp".into()),
                model: Some("gpt-6-luna".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    (id, home)
}

/// An agent on Copilot, in the Copilot view.
/// Created in it, as with the setting « Vue terminal » on (ESC-125): created in Escouade's view,
/// its server starts at once and opens a session, which its interface would resume, or not,
/// whether its switch came first (`a_new_agent_whose_server_opened_its_session_resumes_it_in_copilots_view`).
async fn copilot_agent(h: &Harness, trusted: bool) -> (String, PathBuf) {
    h.core.settings.write().terminal_view = true;
    let (id, home) = new_copilot_agent(h, trusted).await;
    assert!(h.agent(&id).terminal);
    assert!(!h.alive(&id), "no server for an agent born in the view");
    (id, home)
}

/// The interface of the agent opened as the window does, until it shows `first`: its terminal.
async fn open_copilot(h: &Harness, id: &str, first: &str) -> TermInfo {
    open_copilot_screen(h, id, first).await.0
}

/// `open_copilot`, with what its terminal shows.
async fn open_copilot_screen(h: &Harness, id: &str, first: &str) -> (TermInfo, Arc<Mutex<String>>) {
    let screen = Arc::new(Mutex::new(String::new()));
    let sink = screen.clone();
    let info = h
        .core
        .open_agent_terminal(id, (100, 30), false, move |b| {
            sink.lock().push_str(&String::from_utf8_lossy(&b))
        })
        .await
        .unwrap();
    let start = Instant::now();
    let mut answered = false;
    while !plain_text(&screen.lock()).contains(first) {
        // As xterm.js does: the pseudo-console (Windows) asks where the cursor is first.
        if !answered && screen.lock().contains("\x1b[6n") {
            h.core.pty.write(&info.id, b"\x1b[1;1R").unwrap();
            answered = true;
        }
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "{first} not in {:?}",
            screen.lock()
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    (info, screen)
}

fn jsonl(file: &Path) -> Vec<Value> {
    std::fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

/// The last launch of the fake interface: its arguments and what its environment had.
fn last_launch(home: &Path) -> Value {
    jsonl(&home.join("fake-tui-launches.jsonl")).pop().unwrap()
}

fn argv_of(launch: &Value) -> Vec<String> {
    serde_json::from_value(launch["argv"].clone()).unwrap()
}

fn flag(argv: &[String], name: &str) -> Option<String> {
    argv.windows(2).find(|w| w[0] == name).map(|w| w[1].clone())
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

fn term_of(h: &Harness, id: &str) -> String {
    let a = h.core.agent(id).unwrap();
    let rt = a.lock();
    rt.tui.as_ref().unwrap().term_id.clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn copilots_view_follows_its_turns_and_goes_back_to_escouade_with_its_conversation() {
    let h = harness("tui-copilot-view");
    let (id, home) = new_copilot_agent(&h, true).await;
    let turns = |h: &Harness| items_of(h, &id, "turn").len();

    // A first turn in Escouade's view, on the server's session.
    h.core
        .send_message(&id, "Premier".into(), Vec::new())
        .await
        .unwrap();
    h.wait("the stream's first turn over", |h| {
        h.agent(&id).status == AgentStatus::Done && turns(h) == 1
    })
    .await;
    let session = h.agent(&id).session_id.unwrap();

    // Into the Copilot view: its interface resumes that session, with Escouade's hooks in a plugin
    // of its own, the address of the hooks' server and their token in its environment.
    h.core.set_agent_view(&id, true).await.unwrap();
    open_copilot(&h, &id, "fake-copilot-tui").await;
    let launch = last_launch(&home);
    let argv = argv_of(&launch);
    assert_eq!(argv[..2], ["--resume", session.as_str()], "{argv:?}");
    assert_eq!(flag(&argv, "--model").as_deref(), Some("gpt-6-luna"));
    let plugin = PathBuf::from(flag(&argv, "--plugin-dir").expect("its plugin"));
    assert!(plugin.join("hooks").join("hooks.json").is_file());
    assert_eq!(
        (&launch["hookUrl"], &launch["hookToken"]),
        (&json!(true), &json!(true))
    );

    // A message: the card says it works, then that it is done; the conversation is the log's.
    h.core
        .send_message(&id, "Bonjour".into(), Vec::new())
        .await
        .unwrap();
    h.wait("the turn over", |h| {
        h.agent(&id).status == AgentStatus::Done && turns(h) == 2
    })
    .await;
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(session.as_str()));
    assert_eq!(users(&h, &id), ["Premier", "Bonjour"]);
    assert!(items_of(&h, &id, "text")
        .iter()
        .any(|t| t["text"] == "Fini : Bonjour"));
    // Its hooks were the plugin's.
    let hooks = jsonl(&home.join("fake-tui-hooks.jsonl"));
    assert!(hooks.iter().any(|x| x["event"] == "agentStop"), "{hooks:?}");
    assert!(hooks.iter().all(|x| x["from"] == "plugin"), "{hooks:?}");

    // A permission: the card says it waits, then works again once answered in the terminal.
    h.core
        .send_message(&id, "permission ?".into(), Vec::new())
        .await
        .unwrap();
    h.wait("waiting for the permission", |h| {
        h.agent(&id).status == AgentStatus::Waiting
    })
    .await;
    h.core.pty.write(&term_of(&h, &id), b"\r").unwrap();
    h.wait("its second turn over", |h| {
        h.agent(&id).status == AgentStatus::Done && turns(h) == 3
    })
    .await;
    h.wait("its command done", |h| {
        items_of(h, &id, "tool")
            .iter()
            .any(|t| t["name"] == "Bash" && t["status"] == "ok")
    })
    .await;

    // Interrupted from the window (Escape): the turn ends as interrupted. Sent once its message
    // is read from the log: the interface waits for it then.
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
    assert!(!h.agent(&id).status.is_active());

    // Back to Escouade's view: the same session resumed by the server, the conversation kept.
    h.core.set_agent_view(&id, false).await.unwrap();
    h.core
        .send_message(&id, "Et maintenant ?".into(), Vec::new())
        .await
        .unwrap();
    h.wait("the stream's turn over", |h| {
        h.agent(&id).status == AgentStatus::Done && turns(h) == 5
    })
    .await;
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(session.as_str()));
    assert_eq!(
        users(&h, &id),
        [
            "Premier",
            "Bonjour",
            "permission ?",
            "lent",
            "Et maintenant ?"
        ]
    );

    // Into the Copilot view again: `--resume <session>`, nothing read twice.
    h.core.set_agent_view(&id, true).await.unwrap();
    open_copilot(&h, &id, "fake-copilot-tui").await;
    let argv = argv_of(&last_launch(&home));
    assert_eq!(argv[..2], ["--resume", session.as_str()]);
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert_eq!(users(&h, &id).len(), 5);
    assert_eq!(turns(&h), 5);
}

#[tokio::test(flavor = "multi_thread")]
async fn on_a_free_account_copilots_view_opens_on_auto_without_a_word_about_its_model() {
    // ESC-122, seen for real: an agent of a Free account on GPT-6 Luna (the menu's first, its account
    // listing none) opened its view with `--model gpt-6-luna`, and Copilot said in red « ✗ Model
    // "gpt-6-luna" from --model flag is not available. Using "auto" instead. ».
    let h = harness("tui-copilot-free");
    let (p, repo) = h.project(false).await;
    let home = copilot_account(&h, &repo, true);
    std::fs::write(home.join("fake.json"), r#"{"free": true}"#).unwrap();
    // Born in the view (« Vue terminal »): created in Escouade's, its server would open a session first, which the
    // interface then resumes instead of naming its own (the race of ESC-125, seen here on Linux).
    h.core.settings.write().terminal_view = true;
    // With an effort, which `auto` does not take.
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::COPILOT),
                account: Some("cp".into()),
                model: Some("gpt-6-luna".into()),
                effort: Some("high".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    assert!(h.agent(&id).terminal);
    assert!(!h.alive(&id), "no server for an agent born in the view");
    assert_eq!(h.agent(&id).model, "gpt-6-luna");
    let refused = "from --model flag is not available";

    // A new session in its view.
    let (_, screen) = open_copilot_screen(&h, &id, "fake-copilot-tui").await;
    h.wait("its session named", |h| {
        let argv = argv_of(&last_launch(&home));
        argv[0] == "--session-id" && h.agent(&id).session_id.as_deref() == Some(argv[1].as_str())
    })
    .await;
    let argv = argv_of(&last_launch(&home));
    assert_eq!(flag(&argv, "--model").as_deref(), Some("auto"), "{argv:?}");
    assert!(!argv.iter().any(|a| a == "--reasoning-effort"), "{argv:?}");
    h.core
        .send_message(&id, "Bonjour".into(), Vec::new())
        .await
        .unwrap();
    h.wait("the turn over", |h| {
        h.agent(&id).status == AgentStatus::Done && !items_of(h, &id, "turn").is_empty()
    })
    .await;
    assert!(
        !plain_text(&screen.lock()).contains(refused),
        "{}",
        plain_text(&screen.lock())
    );

    // Back in Escouade's view, then in Copilot's again: its session resumed, on `auto` too.
    h.core.set_agent_view(&id, false).await.unwrap();
    h.core.set_agent_view(&id, true).await.unwrap();
    let (_, screen) = open_copilot_screen(&h, &id, "fake-copilot-tui").await;
    let argv = argv_of(&last_launch(&home));
    assert_eq!(argv[0], "--resume", "{argv:?}");
    assert_eq!(flag(&argv, "--model").as_deref(), Some("auto"), "{argv:?}");
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(
        !plain_text(&screen.lock()).contains(refused),
        "{}",
        plain_text(&screen.lock())
    );
    // The agent keeps its model: Escouade's view would run it, were the account to list it.
    assert_eq!(h.agent(&id).model, "gpt-6-luna");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_deleted_agent_leaves_no_folder_of_its_plugin_in_its_copilot_account() {
    // ESC-122, seen after ESC-52: each plugin of `--plugin-dir` left an empty folder of its data in
    // the account's `plugin-data/_direct`, never removed by Copilot.
    let h = harness("tui-copilot-forget");
    let (id, home) = copilot_agent(&h, true).await;
    let direct = home.join("plugin-data").join("_direct");
    // Another plugin's, with its data: never touched.
    std::fs::create_dir_all(direct.join("other")).unwrap();
    std::fs::write(direct.join("other").join("data.json"), "{}").unwrap();
    open_copilot(&h, &id, "fake-copilot-tui").await;
    let plugin = PathBuf::from(flag(&argv_of(&last_launch(&home)), "--plugin-dir").unwrap());
    let ours = direct.join(crate::engine::copilot::terminal::plugin_data_id(&plugin));
    assert!(
        ours.is_dir(),
        "{:?}",
        std::fs::read_dir(&direct).unwrap().collect::<Vec<_>>()
    );

    h.core.delete_agent(&id, false).await.unwrap();
    assert!(!ours.exists());
    assert!(!plugin.exists());
    assert!(direct.join("other").join("data.json").is_file());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_agent_in_copilots_view_names_its_session_and_a_question_waits() {
    let h = harness("tui-copilot-new");
    let (id, home) = copilot_agent(&h, true).await;
    open_copilot(&h, &id, "fake-copilot-tui").await;
    // Its session named by Escouade before any hook (that of its last launch: the window's may
    // follow a first one in the background).
    h.wait("its session named", |h| {
        let argv = argv_of(&last_launch(&home));
        argv[0] == "--session-id" && h.agent(&id).session_id.as_deref() == Some(argv[1].as_str())
    })
    .await;
    let argv = argv_of(&last_launch(&home));

    h.core
        .send_message(&id, "Une question ?".into(), Vec::new())
        .await
        .unwrap();
    h.wait("waiting for the answer", |h| {
        h.agent(&id).status == AgentStatus::Waiting
    })
    .await;
    h.core.pty.write(&term_of(&h, &id), b"Bleu\r").unwrap();
    h.wait("the turn over", |h| {
        h.agent(&id).status == AgentStatus::Done && !items_of(h, &id, "turn").is_empty()
    })
    .await;
    assert_eq!(users(&h, &id), ["Une question ?"]);
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(argv[1].as_str()));
}

/// What the CI met (ESC-125): a new agent created in Escouade's view has its server started at
/// once, which opens a session; switched to Copilot's view after that, its interface resumes that
/// session instead of naming one.
#[tokio::test(flavor = "multi_thread")]
async fn a_new_agent_whose_server_opened_its_session_resumes_it_in_copilots_view() {
    let h = harness("tui-copilot-server-first");
    let (id, home) = new_copilot_agent(&h, true).await;
    h.wait("its server's session", |h| {
        h.agent(&id).session_id.is_some()
    })
    .await;
    let session = h.agent(&id).session_id.unwrap();
    h.core.set_agent_view(&id, true).await.unwrap();
    open_copilot(&h, &id, "fake-copilot-tui").await;
    let argv = argv_of(&last_launch(&home));
    assert_eq!(argv[..2], ["--resume", session.as_str()], "{argv:?}");
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(session.as_str()));
}

#[tokio::test(flavor = "multi_thread")]
async fn copilots_question_about_the_folder_is_left_to_the_user() {
    let h = harness("tui-copilot-trust");
    let (id, home) = copilot_agent(&h, false).await;
    let info = open_copilot(&h, &id, "fake-copilot-trust").await;
    // Nothing is pasted while Copilot asks: the text would answer its question.
    let refused = h
        .core
        .send_message(&id, "Bonjour".into(), Vec::new())
        .await
        .unwrap_err();
    assert!(
        refused.to_string().contains("fais confiance à ce dossier"),
        "{refused:#}"
    );
    assert!(items_of(&h, &id, "user").is_empty());
    // The band of its view says it, above the terminal.
    assert!(h.core.tui_asks_trust(&id).unwrap());
    // « 1. Yes », for this process alone: nothing written, so Escouade still cannot tell.
    h.core.pty.write(&info.id, b"\r").unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(h
        .core
        .send_message(&id, "x".into(), Vec::new())
        .await
        .is_err());
    // The user's own message, typed in the terminal: then Escouade's go.
    h.core.pty.write(&info.id, b"A toi\r").unwrap();
    h.wait("the user's turn over", |h| {
        !items_of(h, &id, "turn").is_empty()
    })
    .await;
    assert!(!h.core.tui_asks_trust(&id).unwrap());
    h.core
        .send_message(&id, "Bonjour".into(), Vec::new())
        .await
        .unwrap();
    h.wait("the second turn over", |h| {
        items_of(h, &id, "turn").len() == 2
    })
    .await;
    assert_eq!(users(&h, &id), ["A toi", "Bonjour"]);
    assert!(!std::fs::read_to_string(home.join("config.json"))
        .unwrap_or_default()
        .contains("trustedFolders"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_folder_remembered_lets_the_messages_go_at_once() {
    let h = harness("tui-copilot-remember");
    let (id, home) = copilot_agent(&h, false).await;
    let info = open_copilot(&h, &id, "fake-copilot-trust").await;
    // « 2. Yes, and remember this folder »: written in its config.
    h.core.pty.write(&info.id, b"\x1b[B").unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    h.core.pty.write(&info.id, b"\r").unwrap();
    h.wait("the folder remembered", |_| {
        std::fs::read_to_string(home.join("config.json"))
            .is_ok_and(|c| c.contains("trustedFolders"))
    })
    .await;
    h.core
        .send_message(&id, "Bonjour".into(), Vec::new())
        .await
        .unwrap();
    h.wait("the turn over", |h| !items_of(h, &id, "turn").is_empty())
        .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_copilot_session_with_no_log_is_given_up_and_a_new_one_starts() {
    let h = harness("tui-copilot-lost");
    let (id, home) = copilot_agent(&h, true).await;
    h.core.agent(&id).unwrap().lock().meta.session_id = Some("perdu-1".into());
    open_copilot(&h, &id, "fake-copilot-tui").await;
    // Not resumed: Copilot would quit at once on a session it does not find.
    let argv = argv_of(&last_launch(&home));
    assert_eq!(argv[0], "--session-id");
    assert_ne!(h.agent(&id).session_id.as_deref(), Some("perdu-1"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_copy_in_copilots_view_resumes_a_copy_made_first() {
    let h = harness("tui-copilot-copy");
    let (id, home) = new_copilot_agent(&h, true).await;
    h.core
        .send_message(&id, "Premier".into(), Vec::new())
        .await
        .unwrap();
    h.wait("the first turn over", |h| {
        h.agent(&id).status == AgentStatus::Done && !items_of(h, &id, "turn").is_empty()
    })
    .await;
    let original = h.agent(&id).session_id.unwrap();
    let copy = h.core.duplicate_agent(&id).await.unwrap().meta.id;
    h.core.set_agent_view(&copy, true).await.unwrap();
    open_copilot(&h, &copy, "fake-copilot-tui").await;
    let argv = argv_of(&last_launch(&home));
    assert_eq!(argv[0], "--resume");
    assert_ne!(argv[1], original);
    let meta = h.agent(&copy);
    assert_eq!(meta.session_id.as_deref(), Some(argv[1].as_str()));
    assert_eq!(meta.fork_of, None);
    // The copy has the original's conversation, which its server kept.
    let kept: Value = serde_json::from_str(
        &std::fs::read_to_string(home.join("session-state").join(format!("{}.json", argv[1])))
            .unwrap(),
    )
    .unwrap();
    assert!(kept["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["type"] == "user.message" && e["data"]["content"] == "Premier"));
}

#[tokio::test(flavor = "multi_thread")]
async fn escouades_hooks_touch_no_copilot_session_started_outside_escouade() {
    let h = harness("tui-copilot-outside");
    let (id, home) = copilot_agent(&h, true).await;
    open_copilot(&h, &id, "fake-copilot-tui").await;
    h.core
        .send_message(&id, "Bonjour".into(), Vec::new())
        .await
        .unwrap();
    h.wait("the turn over", |h| !items_of(h, &id, "turn").is_empty())
        .await;
    // Nothing of Escouade's hooks in Copilot's folder, which the user's other sessions read.
    assert!(!home.join("hooks").exists());
    assert!(!home.join("settings.json").exists());
    assert!(!home.join("installed-plugins").exists());
    let ran = jsonl(&home.join("fake-tui-hooks.jsonl")).len();
    assert!(ran > 0);

    // The user's own `copilot`, in the same folder and project: none of Escouade's hooks runs.
    let cwd = h.agent(&id).cwd;
    let mut cmd = if cfg!(windows) {
        let mut c = std::process::Command::new("cmd");
        c.arg("/c").arg(fake_copilot());
        c
    } else {
        std::process::Command::new(fake_copilot())
    };
    let out = cmd
        .current_dir(&cwd)
        .env("COPILOT_HOME", &home)
        .env_remove("ESCOUADE_HOOK_TOKEN")
        .env_remove("ESCOUADE_HOOK_URL")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .and_then(|mut c| {
            use std::io::Write;
            let mut input = c.stdin.take().unwrap();
            input.write_all(b"Bonjour\r")?;
            std::thread::sleep(Duration::from_millis(1500));
            input.write_all(b"/quit\r")?;
            drop(input);
            c.wait_with_output()
        })
        .unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).contains("fake-turn Bonjour"));
    assert_eq!(jsonl(&home.join("fake-tui-hooks.jsonl")).len(), ran);
    assert_eq!(items_of(&h, &id, "user").len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn escouade_hook_does_nothing_without_the_token_its_interface_has() {
    // A hook of Escouade's reached by another process of Copilot's (the user's, with GH_TOKEN in
    // its environment, but not the interface's token): it posts nothing.
    let h = harness("tui-copilot-guard");
    let (id, _home) = copilot_agent(&h, true).await;
    open_copilot(&h, &id, "fake-copilot-tui").await;
    let hook = crate::engine::terminal::hook_command().expect("escouade-hook beside the tests");
    let port = h.core.hooks_port().unwrap();
    let out = std::process::Command::new(&hook)
        .arg("userPromptSubmitted")
        .env("ESCOUADE_HOOK_URL", format!("http://127.0.0.1:{port}/hook"))
        .env_remove("ESCOUADE_HOOK_TOKEN")
        .env("GH_TOKEN", "not-a-token")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            use std::io::Write;
            let session = h.agent(&id).session_id.unwrap();
            c.stdin.take().unwrap().write_all(
                json!({ "sessionId": session, "prompt": "x" })
                    .to_string()
                    .as_bytes(),
            )?;
            c.wait()
        })
        .unwrap();
    assert!(out.success());
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(h.agent(&id).status, AgentStatus::Idle);
}

/// Real run (ESC-48), by hand: « Vue Copilot » with the real Copilot CLI 1.0.98
/// (`ESCOUADE_REAL_COPILOT`, its `copilot.exe`), a `COPILOT_HOME` of the trial's and a token in the
/// environment of this process alone (`GH_TOKEN`, never printed): `cargo test --lib
/// real_copilot_view -- --ignored --nocapture`. Its credits are counted: three short turns in its
/// interface (a reply, a command allowed there, a command interrupted), one back in Escouade's view.
/// `echo` alone asks nothing: Copilot runs it without a word (its `permissionRequest` hook comes all
/// the same, no `notification`), `Start-Sleep` asks.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn real_copilot_view() {
    let program = std::env::var("ESCOUADE_REAL_COPILOT")
        .expect("ESCOUADE_REAL_COPILOT: the path of the real copilot.exe");
    let home = PathBuf::from(
        std::env::var("COPILOT_HOME").expect("COPILOT_HOME: a folder of the trial's"),
    );
    assert!(
        !home.to_string_lossy().contains(".copilot"),
        "a COPILOT_HOME of the trial's, never the user's"
    );
    let h = harness("tui-copilot-real");
    let (p, _repo) = h.project(false).await;
    let mut s = h.core.settings.read().clone();
    s.accounts.push(Account {
        id: "cp".into(),
        name: "Copilot".into(),
        engine: EngineId::COPILOT,
        config_dir: home.to_string_lossy().into(),
        claude_path: program,
        ..Default::default()
    });
    h.core.save_settings(s).unwrap();
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::COPILOT),
                account: Some("cp".into()),
                model: Some("gpt-6-luna".into()),
                mode: Some("default".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
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
    // Its first screen: the question about the folder, answered here as the user would
    // (« Yes, and remember this folder »), Escouade refusing to paste meanwhile.
    let start = Instant::now();
    let mut answered = false;
    loop {
        let text = plain_text(&screen.lock());
        if !answered && screen.lock().contains("\x1b[6n") {
            h.core.pty.write(&info.id, b"\x1b[1;1R").unwrap();
            answered = true;
        }
        if text.contains("Confirm folder trust") {
            tokio::time::sleep(Duration::from_millis(800)).await;
            let refused = h.core.send_message(&id, "x".into(), Vec::new()).await;
            println!("message while it asks: {refused:?}");
            assert!(refused.is_err());
            h.core.pty.write(&info.id, b"\x1b[B").unwrap();
            tokio::time::sleep(Duration::from_millis(500)).await;
            h.core.pty.write(&info.id, b"\r").unwrap();
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(60), "{text}");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_secs(4)).await;
    let statuses = Arc::new(Mutex::new(Vec::new()));
    let watch = |h: &Harness, statuses: &Mutex<Vec<AgentStatus>>| {
        let st = h.agent(&id).status;
        if statuses.lock().last() != Some(&st) {
            statuses.lock().push(st);
        }
    };
    let send = |text: &'static str| {
        let (h, id) = (&h, id.clone());
        async move {
            let start = Instant::now();
            loop {
                match h.core.send_message(&id, text.into(), Vec::new()).await {
                    Ok(()) => break,
                    Err(e) => {
                        println!("not sent yet: {e:#}");
                        assert!(start.elapsed() < Duration::from_secs(60));
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        }
    };
    let turns = |h: &Harness| items_of(h, &id, "turn").len();
    let until = |what: &'static str, secs: u64, pred: &dyn Fn(&Harness) -> bool| {
        let start = Instant::now();
        let (h, statuses) = (&h, statuses.clone());
        let ok = loop {
            watch(h, &statuses);
            if pred(h) {
                break true;
            }
            if start.elapsed() > Duration::from_secs(secs) {
                break false;
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        assert!(
            ok,
            "no {what}: {}",
            plain_text(&screen.lock())
                .chars()
                .rev()
                .take(2000)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
        );
    };

    send("Réponds seulement : A").await;
    until("end of the first turn", 180, &|h| turns(h) == 1);
    let session = h.agent(&id).session_id.clone().unwrap();
    println!("session {session}");
    println!("statuses of the first turn: {:?}", statuses.lock());
    assert!(statuses.lock().contains(&AgentStatus::Running));
    assert_eq!(h.agent(&id).status, AgentStatus::Done);

    statuses.lock().clear();
    send(
        "Lance exactement `Start-Sleep -Seconds 1; echo tui` avec powershell, puis réponds : fini.",
    )
    .await;
    until("the permission waiting", 180, &|h| {
        h.agent(h_id(h, &id)).status == AgentStatus::Waiting
    });
    tokio::time::sleep(Duration::from_millis(800)).await;
    h.core.pty.write(&term_of(&h, &id), b"\r").unwrap();
    until("end of the second turn", 180, &|h| turns(h) == 2);
    println!("statuses of the second turn: {:?}", statuses.lock());
    assert!(statuses.lock().contains(&AgentStatus::Waiting));

    statuses.lock().clear();
    send("Lance exactement `Start-Sleep -Seconds 60; echo fini` avec powershell.").await;
    until("its permission", 180, &|h| {
        h.agent(h_id(h, &id)).status == AgentStatus::Waiting
    });
    tokio::time::sleep(Duration::from_millis(800)).await;
    h.core.pty.write(&term_of(&h, &id), b"\r").unwrap();
    tokio::time::sleep(Duration::from_secs(5)).await;
    h.core.interrupt(&id).await.unwrap();
    until("the interruption", 60, &|h| turns(h) == 3);
    println!("statuses of the third turn: {:?}", statuses.lock());
    let last = items_of(&h, &id, "turn").pop().unwrap();
    println!("its end: {last}");

    h.core.set_agent_view(&id, false).await.unwrap();
    statuses.lock().clear();
    send("Réponds seulement : B").await;
    until("end of the turn in Escouade's view", 180, &|h| {
        turns(h) == 4
    });
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(session.as_str()));
    let users: Vec<String> = users(&h, &id);
    let texts: Vec<Value> = items_of(&h, &id, "text")
        .iter()
        .map(|t| t["text"].clone())
        .collect();
    let tools: Vec<Value> = items_of(&h, &id, "tool")
        .iter()
        .map(|t| json!([t["name"], t["input"]["command"], t["status"]]))
        .collect();
    println!("users: {users:?}\ntexts: {texts:?}\ntools: {tools:?}");
    println!(
        "cost: {} tokens: {}",
        h.agent(&id).cost,
        h.agent(&id).tokens
    );
    assert_eq!(users.len(), 4);
    assert_eq!(json!(texts.last()), json!("B"));
    assert!(last["interrupted"] == true, "{last}");
}

/// The agent's id, for the closures of `real_copilot_view`.
fn h_id<'a>(_h: &Harness, id: &'a str) -> &'a str {
    id
}

/// Real run (ESC-122), by hand: « Vue Copilot » of an agent of a Free account on GPT-6 Luna (with
/// an effort), with the real Copilot CLI (`ESCOUADE_REAL_COPILOT`), on its Principal: a home of
/// the trial's (`ESCOUADE_TEST_REAL_HOME`), whose `.copilot` is the real one's `COPILOT_HOME`, and
/// the token of `gh auth token` (`GH_TOKEN`, never printed) in the environment of this process
/// alone, an account of Escouade's own emptying them. `cargo test --lib
/// real_copilot_view_on_a_free_account -- --ignored --nocapture`. Its interface opens on a
/// new session, then resumes it after a short turn (its only credits): never a word about its
/// model.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn real_copilot_view_on_a_free_account() {
    let program = std::env::var("ESCOUADE_REAL_COPILOT")
        .expect("ESCOUADE_REAL_COPILOT: the path of the real copilot.exe");
    let trial = PathBuf::from(
        std::env::var("ESCOUADE_TEST_REAL_HOME").expect("ESCOUADE_TEST_REAL_HOME: a trial's home"),
    );
    assert!(
        dirs::home_dir().is_none_or(|h| h != trial),
        "a home of the trial's, never the user's"
    );
    let home = PathBuf::from(std::env::var("COPILOT_HOME").expect("COPILOT_HOME"));
    assert_eq!(home, trial.join(".copilot"), "its Principal's folder");
    assert!(std::env::var("GH_TOKEN").is_ok_and(|t| !t.is_empty()));
    let h = harness("tui-copilot-real-free");
    let (p, repo) = h.project(false).await;
    // Its folder trusted, as « Yes, and remember this folder » writes it: no question first.
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(
        home.join("config.json"),
        json!({ "trustedFolders": [repo] }).to_string(),
    )
    .unwrap();
    let mut s = h.core.settings.read().clone();
    s.accounts.push(Account {
        id: crate::accounts::principal_id(EngineId::COPILOT),
        name: "Principal".into(),
        engine: EngineId::COPILOT,
        claude_path: program,
        ..Default::default()
    });
    h.core.save_settings(s).unwrap();
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::COPILOT),
                account: Some(crate::accounts::principal_id(EngineId::COPILOT)),
                model: Some("gpt-6-luna".into()),
                effort: Some("medium".into()),
                mode: Some("default".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    h.core.set_agent_view(&id, true).await.unwrap();
    let refused = "from --model flag is not available";
    // Its interface opened until it waits for a prompt (its footer), then a moment more.
    let open = || {
        let (h, id) = (&h, id.clone());
        async move {
            let screen = Arc::new(Mutex::new(String::new()));
            let sink = screen.clone();
            let started = Instant::now();
            let info = h
                .core
                .open_agent_terminal(&id, (140, 40), false, move |b| {
                    sink.lock().push_str(&String::from_utf8_lossy(&b))
                })
                .await
                .unwrap();
            println!("opened in {:?}", started.elapsed());
            let mut answered = false;
            while !plain_text(&screen.lock()).contains("/ commands") {
                if !answered && screen.lock().contains("\x1b[6n") {
                    h.core.pty.write(&info.id, b"\x1b[1;1R").unwrap();
                    answered = true;
                }
                assert!(
                    started.elapsed() < Duration::from_secs(90),
                    "{}",
                    plain_text(&screen.lock())
                );
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            tokio::time::sleep(Duration::from_secs(6)).await;
            screen
        }
    };
    let tail = |screen: &Mutex<String>| {
        let text = plain_text(&screen.lock());
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        lines[lines.len().saturating_sub(12)..].join("\n")
    };
    // A new session.
    let screen = open().await;
    println!("new session:\n{}", tail(&screen));
    assert!(!plain_text(&screen.lock()).contains(refused));

    // A short turn, then Escouade's view and Copilot's again: its session resumed.
    let start = Instant::now();
    while let Err(e) = h
        .core
        .send_message(
            &id,
            "Réponds seulement par le mot : prêt.".into(),
            Vec::new(),
        )
        .await
    {
        println!("not sent yet: {e:#}");
        assert!(start.elapsed() < Duration::from_secs(60));
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    h.wait("its turn over", |h| {
        !items_of(h, &id, "turn").is_empty() && !h.agent(&id).status.is_active()
    })
    .await;
    let session = h.agent(&id).session_id.clone().unwrap();
    println!("session {session}");
    h.core.set_agent_view(&id, false).await.unwrap();
    h.core.set_agent_view(&id, true).await.unwrap();
    let screen = open().await;
    // Its history comes once the session is read.
    let start = Instant::now();
    while !plain_text(&screen.lock()).contains("prêt") {
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "{}",
            tail(&screen)
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    println!("resumed:\n{}", tail(&screen));
    assert!(!plain_text(&screen.lock()).contains(refused));
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(session.as_str()));
    println!("cost: {}", h.agent(&id).cost);

    // Deleted: the folder Copilot made for the data of its plugin goes with it.
    let plugin = crate::engine::copilot::terminal::plugin_dir(
        &h.core.data.root().join("tui").join(format!("{id}.json")),
    );
    let direct = home.join("plugin-data").join("_direct");
    let ours = direct.join(crate::engine::copilot::terminal::plugin_data_id(&plugin));
    assert!(
        ours.is_dir(),
        "{:?}",
        std::fs::read_dir(&direct).map(|d| d.flatten().map(|e| e.file_name()).collect::<Vec<_>>())
    );
    h.core.delete_agent(&id, false).await.unwrap();
    assert!(!ours.exists());
    println!(
        "left in plugin-data/_direct: {:?}",
        std::fs::read_dir(&direct).map(|d| d.count())
    );
}

/// An agent in the Claude Code view handed over to Copilot (« Continuer sur Copilot CLI »,
/// ESC-120): its turn of the handover file ended, it is in the Copilot view on its new session.
#[tokio::test(flavor = "multi_thread")]
async fn an_agent_handed_over_from_the_claude_code_view_goes_on_in_the_copilot_view() {
    let h = harness("tui-copilot-handover");
    let (p, repo) = h.project(false).await;
    crate::core_tests::second_account(&h);
    let home = copilot_account(&h, &repo, true);
    let mut s = h.core.settings.read().clone();
    s.engine_order = vec![EngineId::COPILOT];
    s.handover_models
        .insert(EngineId::COPILOT, "gpt-6-luna".into());
    h.core.save_settings(s).unwrap();
    let id = crate::core_tests::agent_on(&h, &p, "pro").await.id;
    h.core.set_agent_view(&id, true).await.unwrap();

    h.core
        .continue_on_engine(&id, EngineId::COPILOT)
        .await
        .unwrap();
    assert!(!h.agent(&id).terminal);
    h.wait("the turn of the handover file", |h| {
        items_of(h, &id, "turn")
            .last()
            .is_some_and(|t| t["engine"] == "copilot")
            && !h.agent(&id).status.is_active()
    })
    .await;
    h.wait("the Copilot view", |h| h.agent(&id).terminal).await;
    let m = h.agent(&id);
    assert_eq!(m.engine, EngineId::COPILOT);
    let session = m.session_id.expect("its new session");
    open_copilot(&h, &id, "fake-copilot-tui").await;
    let argv = argv_of(&last_launch(&home));
    assert_eq!(argv[..2], ["--resume", session.as_str()], "{argv:?}");
}
