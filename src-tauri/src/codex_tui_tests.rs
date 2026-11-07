//! Integration tests of « Vue Codex » (`tui`, `engine/codex/terminal.rs`): the fake `codex` plays
//! Codex's interface in a real terminal (tests/fixtures/fake-codex-tui.mjs), runs the hooks of its
//! options (`escouade-hook`, trusted by its options too) and writes its session's rollout, which
//! the core reads as it comes; its app-server takes the session back in Escouade's view.

use crate::core::AgentOptions;
use crate::core_tests::{harness, plain_text, Harness};
use crate::model::*;
use crate::pty::TermInfo;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The fake `codex` its interface is started as: under Windows its program (`examples/fake_codex.rs`,
/// which `cargo test` builds beside the tests), elsewhere its shell script.
fn fake_codex() -> String {
    if !cfg!(windows) {
        return crate::engine::fake_codex();
    }
    let exe = std::env::current_exe().unwrap();
    let found = exe
        .ancestors()
        .skip(1)
        .take(3)
        .map(|d| d.join("examples").join("fake_codex.exe"))
        .find(|p| p.is_file())
        .expect("examples/fake_codex.exe is not built beside the tests: `cargo test` builds it, `cargo test --lib` does not");
    found.to_string_lossy().into_owned()
}

/// A Codex account of its own, its `CODEX_HOME` in the test's folder, on the fake `codex`; with
/// `trusted`, its config already trusts the project `repo` (as Codex's app-server writes it).
fn codex_account(h: &Harness, repo: &Path, trusted: bool) -> PathBuf {
    let home = h.dir.join("codex-cx");
    std::fs::create_dir_all(&home).unwrap();
    if trusted {
        let config = format!(
            "model = \"gpt-6-luna\"\n\n[projects.'{}']\ntrust_level = \"trusted\"\n",
            repo.display()
        );
        std::fs::write(home.join("config.toml"), config).unwrap();
    }
    let mut s = h.core.settings.read().clone();
    s.accounts.push(Account {
        id: "cx".into(),
        name: "Codex".into(),
        engine: EngineId::CODEX,
        config_dir: home.to_string_lossy().into(),
        claude_path: fake_codex(),
        ..Default::default()
    });
    h.core.save_settings(s).unwrap();
    home
}

/// An agent on Codex, in the Codex view, and the `CODEX_HOME` of its account.
async fn codex_agent(h: &Harness, trusted: bool) -> (String, PathBuf) {
    let (id, home) = new_codex_agent(h, trusted).await;
    h.core.set_agent_view(&id, true).await.unwrap();
    (id, home)
}

/// An agent on Codex, in Escouade's view, and the `CODEX_HOME` of its account.
async fn new_codex_agent(h: &Harness, trusted: bool) -> (String, PathBuf) {
    let (p, repo) = h.project(false).await;
    let home = codex_account(h, &repo, trusted);
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::CODEX),
                account: Some("cx".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    (id, home)
}

/// The interface of the agent opened as the window does, until it shows `first`: its terminal.
async fn open_codex(h: &Harness, id: &str, first: &str) -> TermInfo {
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
    info
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

fn items_of(h: &Harness, id: &str, kind: &str) -> Vec<Value> {
    let a = h.core.agent(id).unwrap();
    let mut rt = a.lock();
    rt.conv
        .items()
        .into_iter()
        .filter(|i| i["kind"] == kind)
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn codexs_view_follows_its_turns_and_goes_back_to_escouade_with_its_conversation() {
    let h = harness("tui-codex-view");
    let (id, home) = new_codex_agent(&h, true).await;
    let config_before = std::fs::read_to_string(home.join("config.toml")).unwrap();
    let turns = |h: &Harness| items_of(h, &id, "turn").len();
    let users = |h: &Harness| -> Vec<String> {
        items_of(h, &id, "user")
            .iter()
            .map(|u| u["text"].as_str().unwrap().to_string())
            .collect()
    };

    // A first turn in Escouade's view, on the app-server's thread.
    h.core
        .send_message(&id, "Premier".into(), Vec::new())
        .await
        .unwrap();
    h.wait("the stream's first turn over", |h| {
        h.agent(&id).status == AgentStatus::Done && turns(h) == 1
    })
    .await;
    let session = h.agent(&id).session_id.unwrap();

    // Into the Codex view: its interface resumes that thread, in a process of its own, with
    // Escouade's hooks for it alone and their trust, the address of the hooks' server and their
    // token in its environment.
    h.core.set_agent_view(&id, true).await.unwrap();
    open_codex(&h, &id, "fake-codex-tui").await;
    let launch = last_launch(&home);
    let argv: Vec<String> = serde_json::from_value(launch["argv"].clone()).unwrap();
    assert_eq!(
        argv[..3],
        ["resume", session.as_str(), "--no-daemon"],
        "{argv:?}"
    );
    assert!(
        argv.windows(2).any(|w| w == ["-m", "gpt-6-luna"]),
        "{argv:?}"
    );
    assert!(argv.iter().any(|a| a.starts_with("hooks.Stop=")));
    assert!(argv.iter().any(|a| a.starts_with("hooks.state=")));
    assert_eq!(
        (&launch["hookUrl"], &launch["hookToken"]),
        (&json!(true), &json!(true))
    );

    // A message: the card says it works, then that it is done; the conversation is the rollout's.
    h.core
        .send_message(&id, "Bonjour".into(), Vec::new())
        .await
        .unwrap();
    h.wait("the turn over", |h| {
        h.agent(&id).status == AgentStatus::Done && turns(h) == 2
    })
    .await;
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(session.as_str()));
    assert_eq!(users(&h), ["Premier", "Bonjour"]);
    assert!(items_of(&h, &id, "text")
        .iter()
        .any(|t| t["text"] == "Fini : Bonjour"));
    // Every hook it ran was trusted.
    let hooks = jsonl(&home.join("fake-tui-hooks.jsonl"));
    assert!(hooks.iter().any(|x| x["event"] == "Stop"), "{hooks:?}");
    assert!(hooks.iter().all(|x| x["trusted"] == true), "{hooks:?}");

    // A permission: the card says it waits, then works again once answered in the terminal.
    h.core
        .send_message(&id, "permission ?".into(), Vec::new())
        .await
        .unwrap();
    h.wait("waiting for the permission", |h| {
        h.agent(&id).status == AgentStatus::Waiting
    })
    .await;
    let term = {
        let a = h.core.agent(&id).unwrap();
        let rt = a.lock();
        rt.tui.as_ref().unwrap().term_id.clone()
    };
    h.core.pty.write(&term, b"\r").unwrap();
    h.wait("its second turn over", |h| {
        h.agent(&id).status == AgentStatus::Done && turns(h) == 3
    })
    .await;
    // Its command comes from the rollout, which may be read a little after the Stop hook.
    h.wait("its command done", |h| {
        items_of(h, &id, "tool")
            .iter()
            .any(|t| t["name"] == "Bash" && t["status"] == "ok")
    })
    .await;

    // Interrupted from the window (Escape): the turn ends as interrupted. Sent once its message
    // is read from the rollout: the interface waits for it then.
    h.core
        .send_message(&id, "lent".into(), Vec::new())
        .await
        .unwrap();
    h.wait("working on its message", |h| {
        h.agent(&id).status == AgentStatus::Running && users(h).last().is_some_and(|u| u == "lent")
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

    // Back to Escouade's view: the same thread resumed by the app-server, the conversation kept.
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
        users(&h),
        [
            "Premier",
            "Bonjour",
            "permission ?",
            "lent",
            "Et maintenant ?"
        ]
    );

    // Into the Codex view again: `codex resume <session>`, nothing read twice.
    h.core.set_agent_view(&id, true).await.unwrap();
    open_codex(&h, &id, "fake-codex-tui").await;
    let launch = last_launch(&home);
    assert_eq!(launch["argv"][0], "resume");
    assert_eq!(launch["argv"][1], session.as_str());
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert_eq!(users(&h).len(), 5);
    assert_eq!(turns(&h), 5);

    // The user's config of Codex is as it was: Escouade's hooks were options of its process.
    assert_eq!(
        std::fs::read_to_string(home.join("config.toml")).unwrap(),
        config_before
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn codexs_question_about_the_folder_is_left_to_the_user() {
    let h = harness("tui-codex-trust");
    let (id, home) = codex_agent(&h, false).await;
    let info = open_codex(&h, &id, "fake-codex-trust").await;
    // Nothing is pasted while Codex asks: the text would answer its question.
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
    // The user answers in the terminal (Codex writes it in its config); then the message goes.
    h.core.pty.write(&info.id, b"\r").unwrap();
    h.wait("the folder trusted", |_| {
        std::fs::read_to_string(home.join("config.toml")).is_ok_and(|c| c.contains("trust_level"))
    })
    .await;
    assert!(!h.core.tui_asks_trust(&id).unwrap());
    h.core
        .send_message(&id, "Bonjour".into(), Vec::new())
        .await
        .unwrap();
    h.wait("the turn over", |h| !items_of(h, &id, "turn").is_empty())
        .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_codex_session_with_no_rollout_is_given_up_and_a_new_one_starts() {
    let h = harness("tui-codex-lost");
    let (id, home) = codex_agent(&h, true).await;
    h.core.agent(&id).unwrap().lock().meta.session_id = Some("perdu-1".into());
    open_codex(&h, &id, "fake-codex-tui").await;
    // Not resumed: Codex would quit at once on a session it does not find.
    assert_eq!(h.agent(&id).session_id, None);
    assert_eq!(last_launch(&home)["argv"][0], "--no-daemon");
}

#[tokio::test(flavor = "multi_thread")]
async fn escouades_hooks_touch_no_codex_session_started_outside_escouade() {
    let h = harness("tui-codex-outside");
    let (id, home) = codex_agent(&h, true).await;
    open_codex(&h, &id, "fake-codex-tui").await;
    h.core
        .send_message(&id, "Bonjour".into(), Vec::new())
        .await
        .unwrap();
    h.wait("the turn over", |h| !items_of(h, &id, "turn").is_empty())
        .await;
    // Nothing of Escouade's hooks is left in Codex's folder for its other sessions.
    let config = std::fs::read_to_string(home.join("config.toml")).unwrap();
    assert!(!config.contains("hooks"), "{config}");
    assert!(!home.join("hooks.json").exists());
    let ran = jsonl(&home.join("fake-tui-hooks.jsonl")).len();

    // The user's own `codex`, in the same folder and project: none of Escouade's hooks runs.
    let cwd = h.agent(&id).cwd;
    let out = std::process::Command::new(fake_codex())
        .current_dir(&cwd)
        .env("CODEX_HOME", &home)
        .env_remove("ESCOUADE_HOOK_TOKEN")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .and_then(|mut c| {
            use std::io::Write;
            // Its input closed after the message: it ends once the turn is over.
            c.stdin.take().unwrap().write_all(b"Bonjour\r")?;
            c.wait_with_output()
        })
        .unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).contains("fake-turn Bonjour"));
    assert_eq!(jsonl(&home.join("fake-tui-hooks.jsonl")).len(), ran);
    assert_eq!(items_of(&h, &id, "user").len(), 1);
}

/// Real run (ESC-47): « Vue Codex » with the real `codex` that `ESCOUADE_REAL_CODEX` names, on the
/// machine's own Codex folder (`~/.codex`): a turn of its model in its interface, then one back in
/// Escouade's view, on the same thread. By hand only (`cargo test --lib real_codex_view --
/// --ignored --nocapture`): save `~/.codex/config.toml` before and put it back after (Codex writes
/// the trust of the test's folder there), and take the thread it prints out of `~/.codex/sessions`
/// and `session_index.jsonl` (docs/TODO-1.8.md, « Vue Codex »).
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn real_codex_view() {
    use crate::engine::codex::terminal::{hook_args, list_hooks, CODEX_TERMINAL};
    use crate::engine::TerminalView;
    let program = std::env::var("ESCOUADE_REAL_CODEX").expect("ESCOUADE_REAL_CODEX");
    let h = harness("tui-codex-real");
    let (p, repo) = h.project(false).await;
    let home = dirs::home_dir().unwrap().join(".codex");

    // The trust Escouade gives its hooks is the one Codex wants: with it, its `hooks/list` says
    // each of them trusted.
    let real = crate::engine::Program::Native(PathBuf::from(&program));
    let hooks = hook_args(&crate::engine::terminal::hook_command().expect("escouade-hook"));
    let env = vec![(
        "CODEX_HOME".to_string(),
        home.to_string_lossy().into_owned(),
    )];
    let cwd = repo.to_string_lossy().into_owned();
    let trust = CODEX_TERMINAL
        .prepare(&real, &cwd, &hooks, &env)
        .await
        .unwrap();
    let listed = list_hooks(&real, &cwd, &[hooks, trust].concat(), &env)
        .await
        .unwrap();
    let ours: Vec<&Value> = listed["data"][0]["hooks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|x| x["source"] == "sessionFlags")
        .collect();
    println!(
        "Escouade's hooks: {:?}",
        ours.iter()
            .map(|x| (&x["eventName"], &x["trustStatus"]))
            .collect::<Vec<_>>()
    );
    assert_eq!(ours.len(), crate::engine::codex::terminal::EVENTS.len());
    assert!(ours.iter().all(|x| x["trustStatus"] == "trusted"));

    let mut s = h.core.settings.read().clone();
    s.accounts.push(Account {
        id: "cx".into(),
        name: "Codex".into(),
        engine: EngineId::CODEX,
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
                engine: Some(EngineId::CODEX),
                account: Some("cx".into()),
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
    // Its first screen: the question about the folder (answered here, as the user would), or
    // its prompt.
    let start = Instant::now();
    let mut answered = false;
    loop {
        let text = plain_text(&screen.lock());
        if !answered && screen.lock().contains("\x1b[6n") {
            h.core.pty.write(&info.id, b"\x1b[1;1R").unwrap();
            answered = true;
        }
        if text.contains("Trust and continue") {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let refused = h.core.send_message(&id, "x".into(), Vec::new()).await;
            println!("message while it asks: {refused:?}");
            assert!(refused.is_err());
            h.core.pty.write(&info.id, b"\r").unwrap();
            break;
        }
        if text.contains("Ask Codex") && start.elapsed() > Duration::from_secs(15) {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(60), "{text}");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_secs(3)).await;
    let statuses = Arc::new(Mutex::new(Vec::new()));
    let send_and_wait = |text: &'static str| {
        let (h, id, statuses) = (&h, id.clone(), statuses.clone());
        async move {
            let turns = items_of(h, &id, "turn").len();
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
            let start = Instant::now();
            while items_of(h, &id, "turn").len() == turns {
                let st = h.agent(&id).status;
                if statuses.lock().last() != Some(&st) {
                    statuses.lock().push(st);
                }
                assert!(start.elapsed() < Duration::from_secs(180), "no end of turn");
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    };
    send_and_wait("Réponds seulement : A").await;
    let session = h.agent(&id).session_id.clone().unwrap();
    println!("session {session}");
    println!("statuses in its interface: {:?}", statuses.lock());
    println!(
        "conversation: {:#?}",
        h.core.agent(&id).unwrap().lock().conv.items()
    );
    assert!(statuses.lock().contains(&AgentStatus::Running));
    assert_eq!(h.agent(&id).status, AgentStatus::Done);

    h.core.set_agent_view(&id, false).await.unwrap();
    statuses.lock().clear();
    send_and_wait("Réponds seulement : B").await;
    println!("statuses in Escouade's view: {:?}", statuses.lock());
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(session.as_str()));
    let texts: Vec<Value> = items_of(&h, &id, "text")
        .iter()
        .map(|t| t["text"].clone())
        .collect();
    println!("texts: {texts:?}");
    assert_eq!(items_of(&h, &id, "user").len(), 2);
    assert_eq!(json!(texts.last()), json!("B"));
}

/// An agent in the Claude Code view handed over to Codex (« Continuer sur Codex », ESC-120): the
/// handover file goes over the stream, then, its turn ended, the agent is in the Codex view on its
/// new session. One in Escouade's view stays there.
#[tokio::test(flavor = "multi_thread")]
async fn an_agent_handed_over_from_the_claude_code_view_goes_on_in_the_codex_view() {
    let h = harness("tui-codex-handover");
    let (p, repo) = h.project(false).await;
    crate::core_tests::second_account(&h);
    let home = codex_account(&h, &repo, true);
    let mut s = h.core.settings.read().clone();
    s.engine_order = vec![EngineId::CODEX];
    h.core.save_settings(s).unwrap();
    let in_view = crate::core_tests::agent_on(&h, &p, "pro").await.id;
    let in_escouade = crate::core_tests::agent_on(&h, &p, "pro").await.id;
    h.core.set_agent_view(&in_view, true).await.unwrap();
    assert_eq!(
        h.core.handover_targets(&in_view).unwrap(),
        [EngineId::CODEX]
    );
    let handed = |h: &Harness, id: &str| {
        let turns = items_of(h, id, "turn");
        turns.last().is_some_and(|t| t["engine"] == "codex") && !h.agent(id).status.is_active()
    };

    h.core
        .continue_on_engine(&in_view, EngineId::CODEX)
        .await
        .unwrap();
    // The file goes over the stream: Escouade's view meanwhile.
    let m = h.agent(&in_view);
    assert_eq!((m.engine, m.account.as_str()), (EngineId::CODEX, "cx"));
    assert!(!m.terminal);
    h.wait("the turn of the handover file", |h| handed(h, &in_view))
        .await;
    h.wait("the Codex view", |h| h.agent(&in_view).terminal)
        .await;
    let session = h.agent(&in_view).session_id.expect("its new session");
    // Its interface resumes the session the handover file started.
    open_codex(&h, &in_view, "fake-codex-tui").await;
    let launch = last_launch(&home);
    assert_eq!(launch["argv"][0], "resume");
    assert_eq!(launch["argv"][1], session.as_str());
    assert_eq!(items_of(&h, &in_view, "turn").len(), 1);

    // In Escouade's view before: it stays there.
    h.core
        .continue_on_engine(&in_escouade, EngineId::CODEX)
        .await
        .unwrap();
    h.wait("the other's handover turn", |h| handed(h, &in_escouade))
        .await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let o = h.agent(&in_escouade);
    assert_eq!((o.engine, o.terminal), (EngineId::CODEX, false));
}

/// ESC-205: in the Codex view, `/compact` goes to its interface as it was typed, which runs it
/// itself: Escouade neither asks its app-server for it nor refuses it.
#[tokio::test]
async fn compact_in_the_codex_view_goes_to_its_interface_as_typed() {
    let h = harness("tui-codex-compact");
    let (id, home) = new_codex_agent(&h, true).await;
    h.core
        .send_message(&id, "Premier".into(), Vec::new())
        .await
        .unwrap();
    h.wait("the stream's first turn over", |h| {
        h.agent(&id).status == AgentStatus::Done && items_of(h, &id, "turn").len() == 1
    })
    .await;
    h.core.set_agent_view(&id, true).await.unwrap();
    open_codex(&h, &id, "fake-codex-tui").await;
    h.core
        .send_message(&id, "/compact".into(), Vec::new())
        .await
        .unwrap();
    h.wait("/compact typed in its interface", |h| {
        items_of(h, &id, "text")
            .iter()
            .any(|t| t["text"] == "Fini : /compact")
    })
    .await;
    let asked = std::fs::read_to_string(home.join("requests.jsonl")).unwrap_or_default();
    assert!(!asked.contains("thread/compact/start"), "{asked}");
    h.core.stop_for_good();
}
