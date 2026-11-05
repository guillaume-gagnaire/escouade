//! Integration tests of « Vue Claude Code » (`tui`): the fake `claude` plays Claude Code's
//! interface in a real terminal, calls the hooks of its settings over HTTP and writes its
//! transcript, which the core reads as it comes.

use crate::attachfiles_tests::copy;
use crate::core::AgentOptions;
use crate::core_tests::{agent_on, harness, plain_text, second_account, Harness};
use crate::model::*;
use crate::pty::TermInfo;
use serde_json::{json, Value};
use std::sync::Arc;

/// An agent on a Claude account of its own (the fake keeps its transcripts in its folder), in the
/// Claude Code view, its interface started.
async fn agent_in_tui(h: &Harness) -> String {
    let (p, _) = h.project(false).await;
    second_account(h);
    let id = agent_on(h, &p, "pro").await.id;
    h.core.set_agent_view(&id, true).await.unwrap();
    open_interface(h, &id).await;
    up(h, &id).await;
    id
}

/// Waits until the registry of Claude Code's sessions lists the agent's interface: it is up, and
/// reads its input. A Ctrl+C sent before (it prints its first screen first) is lost.
async fn up(h: &Harness, id: &str) {
    let session = h.agent(id).session_id.unwrap();
    h.wait("its interface registered", |h| {
        std::fs::read_dir(registry_dir(h))
            .into_iter()
            .flatten()
            .flatten()
            .any(|f| std::fs::read_to_string(f.path()).is_ok_and(|c| c.contains(&session)))
    })
    .await;
}

/// The interface of the agent (in the Claude Code view) opened as the window does, its first
/// screen waited for: its terminal, and what that has shown.
async fn open_interface(h: &Harness, id: &str) -> (TermInfo, Arc<parking_lot::Mutex<String>>) {
    let screen = Arc::new(parking_lot::Mutex::new(String::new()));
    let sink = screen.clone();
    let info = h
        .core
        .open_agent_terminal(id, (100, 30), false, move |b| {
            sink.lock().push_str(&String::from_utf8_lossy(&b))
        })
        .await
        .unwrap();
    let start = std::time::Instant::now();
    let mut answered = false;
    while !plain_text(&screen.lock()).contains("fake-claude-tui") {
        // As xterm.js does in the window: the pseudo-console (Windows) asks where the cursor is
        // before it runs anything.
        if !answered && screen.lock().contains("\x1b[6n") {
            h.core.pty.write(&info.id, b"\x1b[1;1R").unwrap();
            answered = true;
        }
        assert!(
            start.elapsed() < std::time::Duration::from_secs(60),
            "the interface's first screen not in {:?}",
            screen.lock()
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    (info, screen)
}

#[tokio::test(flavor = "multi_thread")]
async fn remote_control_goes_with_an_agent_into_the_claude_code_view_and_back() {
    let h = harness("tui-remote");
    let (p, r) = h.project(false).await;
    second_account(&h);
    let id = agent_on(&h, &p, "pro").await.id;
    h.core.set_remote_control(&id, true).await.unwrap();
    let m = h.agent(&id);
    let session = m.remote_session.clone().unwrap();
    let name = format!("demo · {}", m.name);

    // Not refused: its stream's process stops, after its remote session was closed for good (the
    // interface will have a session of its own), and Remote Control stays in force.
    let asked = h.fake_log(&r, "control").len();
    h.core.set_agent_view(&id, true).await.unwrap();
    assert!(!h.alive(&id));
    {
        let a = h.core.agent(&id).unwrap();
        let rt = a.lock();
        assert!(!rt.remote_linked);
        assert_eq!(rt.remote_state, None);
        assert!(rt.meta.remote_control);
        assert_eq!(rt.meta.remote_source, Some(RemoteSource::Hand));
        assert_eq!(
            (&rt.meta.remote_session, &rt.meta.remote_url),
            (&None, &None)
        );
    }
    let closing: Vec<Value> = h.fake_log(&r, "control").split_off(asked);
    assert_eq!(
        closing
            .iter()
            .map(|q| q["enabled"].clone())
            .collect::<Vec<_>>(),
        [json!(false), json!(true), json!(false)],
        "{closing:?}"
    );
    assert_eq!(closing[1]["reattach_session_id"], session.as_str());
    assert_eq!(h.fake_log(&r, "archived"), [json!({ "bridge": session })]);

    // Its interface starts with Remote Control, under the name of its session in claude.ai.
    open_interface(&h, &id).await;
    let launch = h.launches(&r).pop().unwrap();
    assert!(
        launch.contains(&format!("--remote-control={name}")),
        "{launch:?}"
    );

    // Back to the stream: its next process opens a session of its own (the interface's was closed
    // on its side, and the stream's was closed when it left).
    h.core.set_agent_view(&id, false).await.unwrap();
    h.wait("the stream linked again", |h| {
        h.core.agent(&id).unwrap().lock().remote_linked
    })
    .await;
    let req = h.fake_log(&r, "control").pop().unwrap();
    assert_eq!(req["enabled"], true);
    assert!(req.get("reattach_session_id").is_none(), "{req}");
}

/// Waits until the agent's interface takes a message (a window showed it, or it said it is up).
async fn ready(h: &Harness, id: &str) {
    h.wait("its interface ready", |h| {
        h.core.agent(id).unwrap().lock().interface_ready()
    })
    .await;
}

/// The agent has its interface running.
fn interface_runs(h: &Harness, id: &str) -> bool {
    h.core.agent(id).unwrap().lock().tui.is_some()
}

/// The terminal of the agent's interface.
fn terminal_of(h: &Harness, id: &str) -> String {
    h.core
        .agent(id)
        .unwrap()
        .lock()
        .tui
        .as_ref()
        .unwrap()
        .term_id
        .clone()
}

/// A window taking the agent's terminal over: what it was sent.
async fn show(h: &Harness, id: &str) -> (TermInfo, Arc<parking_lot::Mutex<String>>) {
    let screen = Arc::new(parking_lot::Mutex::new(String::new()));
    let sink = screen.clone();
    let info = h
        .core
        .open_agent_terminal(id, (100, 30), false, move |b| {
            sink.lock().push_str(&String::from_utf8_lossy(&b))
        })
        .await
        .unwrap();
    (info, screen)
}

/// When (ms) the fake interface read each Ctrl+C in `cwd`.
fn ctrl_c(h: &Harness, cwd: &std::path::Path) -> Vec<i64> {
    h.fake_log(cwd, "quit")
        .iter()
        .map(|q| q["at"].as_i64().unwrap())
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn going_back_to_escouades_view_quits_the_interface_with_two_ctrl_c() {
    let h = harness("tui-quit-back");
    let id = agent_in_tui(&h).await;
    let cwd = std::path::PathBuf::from(h.agent(&id).cwd);
    let term = terminal_of(&h, &id);
    let started = std::time::Instant::now();
    h.core.set_agent_view(&id, false).await.unwrap();
    // Two, 700 ms apart: Claude Code closes on them and archives its remote session. (The pause
    // is the app's: the interface reads both as it can, however loaded the machine.)
    let at = ctrl_c(&h, &cwd);
    assert_eq!(at.len(), 2, "{at:?}");
    assert!(started.elapsed() >= std::time::Duration::from_millis(700));
    // It left by itself (its own code, not a kill's), and it is gone before the stream resumes its
    // session.
    assert!(!h.core.pty.exists(&term));
    assert!(h.core.agent(&id).unwrap().lock().tui.is_none());
    h.wait("its exit told", |h| exit_code(h, &term).is_some())
        .await;
    assert_eq!(exit_code(&h, &term), Some(Some(0)));
}

/// What the window was told of the end of the terminal `term`: None while it was told nothing,
/// else the code of the program (None if it was killed).
fn exit_code(h: &Harness, term: &str) -> Option<Option<u64>> {
    h.events
        .lock()
        .iter()
        .find(|e| e["type"] == "terminalExit" && e["id"] == term)
        .map(|e| e["code"].as_u64())
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interface_that_does_not_quit_on_two_ctrl_c_is_killed_after_five_seconds() {
    let h = harness("tui-quit-stuck");
    let (p, r) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = agent_on(&h, &p, "pro").await.id;
    h.fake_flag(&r, "tui-ignore-quit");
    open_interface(&h, &id).await;
    up(&h, &id).await;
    let term = terminal_of(&h, &id);
    let started = std::time::Instant::now();
    h.core.set_agent_view(&id, false).await.unwrap();
    assert_eq!(ctrl_c(&h, &r).len(), 2);
    assert!(started.elapsed() >= std::time::Duration::from_millis(4900));
    assert!(!h.core.pty.exists(&term));
}

#[tokio::test(flavor = "multi_thread")]
async fn archiving_or_deleting_an_agent_quits_its_interface_with_two_ctrl_c() {
    let h = harness("tui-quit-archive-delete");
    let (p, r) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let archived = agent_on(&h, &p, "pro").await.id;
    let deleted = agent_on(&h, &p, "pro").await.id;
    open_interface(&h, &archived).await;
    open_interface(&h, &deleted).await;
    up(&h, &archived).await;
    up(&h, &deleted).await;
    let (first, second) = (terminal_of(&h, &archived), terminal_of(&h, &deleted));

    h.core.archive_agent(&archived, true).await.unwrap();
    assert_eq!(ctrl_c(&h, &r).len(), 2);
    assert!(!h.core.pty.exists(&first));
    assert!(h.core.pty.exists(&second));

    h.core.delete_agent(&deleted, false).await.unwrap();
    assert_eq!(ctrl_c(&h, &r).len(), 4);
    assert!(!h.core.pty.exists(&second));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_users_closing_of_a_terminal_is_immediate_and_sends_no_ctrl_c() {
    let h = harness("tui-quit-not-for-the-window");
    let id = agent_in_tui(&h).await;
    let cwd = std::path::PathBuf::from(h.agent(&id).cwd);
    let term = terminal_of(&h, &id);
    h.core.kill_terminal(&term);
    h.wait("the interface over", |h| !interface_runs(h, &id))
        .await;
    assert!(!h.core.pty.exists(&term));
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(ctrl_c(&h, &cwd).is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interface_quit_by_the_app_soon_after_it_started_with_remote_control_is_no_failure() {
    let h = harness("tui-quit-no-fallback");
    let (p, r) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = agent_on(&h, &p, "pro").await.id;
    h.core.set_remote_control(&id, true).await.unwrap();
    let term = terminal_of(&h, &id);
    ready(&h, &id).await;
    // Turned off soon after: no window showed the interface, it quits.
    h.core.set_remote_control(&id, false).await.unwrap();
    h.wait("its terminal closed", |h| !h.core.pty.exists(&term))
        .await;
    assert_eq!(ctrl_c(&h, &r).len(), 2);
    // Its end is nothing to be told, and starts no other interface.
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    assert!(unavailable(&h, &id).is_empty());
    assert!(!h.core.agent(&id).unwrap().lock().remote_fallback);
    assert_eq!(interface_launches(&h, &r).len(), 1);
    assert!(!interface_runs(&h, &id));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_app_starting_starts_the_interface_of_the_claude_code_agents_a_setting_covers() {
    let h = harness("tui-remote-startup");
    let (p, r) = h.project(false).await;
    let other = h.second_project().await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let covered = agent_on(&h, &p, "pro").await;
    let apart = agent_on(&h, &other, "pro").await;
    assert!(covered.terminal && apart.terminal);
    // The project's setting as the last run left it: no agent has it yet.
    h.core
        .projects
        .write()
        .iter_mut()
        .find(|x| x.id == p.id)
        .unwrap()
        .remote_control = true;

    h.core.start_remote_agents();
    h.wait("its interface", |h| interface_runs(h, &covered.id))
        .await;
    // No window asked for it: it runs by itself, with Remote Control under its name.
    let name = format!("demo · {}", covered.name);
    h.wait("its launch", |h| !h.launches(&r).is_empty()).await;
    let launch = h.launches(&r).pop().unwrap();
    assert!(
        launch.contains(&format!("--remote-control={name}")),
        "{launch:?}"
    );
    let m = h.agent(&covered.id);
    assert_eq!(m.remote_source, Some(RemoteSource::Project));
    // The other one is not covered: no interface.
    h.settle_remote().await;
    assert!(!interface_runs(&h, &apart.id));
    assert!(h.launches(std::path::Path::new(&other.path)).is_empty());
    assert!(!h.agent(&apart.id).remote_control);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_window_opening_an_interface_that_runs_takes_it_over_without_starting_another() {
    let h = harness("tui-attach");
    let (p, r) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = agent_on(&h, &p, "pro").await.id;
    h.remote_all(true);
    h.wait("its interface", |h| interface_runs(h, &id)).await;
    let term = terminal_of(&h, &id);
    // It printed its first screen with no window to show it.
    h.wait("its first screen", |h| {
        let run = h.core.agent(&id).unwrap();
        let out = run.lock().tui.as_ref().unwrap().output.kept();
        plain_text(&String::from_utf8_lossy(&out)).contains("fake-claude-tui")
    })
    .await;
    let launched = h.launches(&r).len();

    let (info, screen) = show(&h, &id).await;
    assert_eq!(info.id, term);
    assert!(h.agent(&id).terminal);
    // Its screen first.
    assert!(plain_text(&screen.lock()).contains("fake-claude-tui"));
    // Then what it writes.
    h.core
        .send_message(&id, "bonjour".into(), Vec::new())
        .await
        .unwrap();
    h.wait("the turn on its screen", |_| {
        plain_text(&screen.lock()).contains("fake-turn bonjour")
    })
    .await;
    // The window is reloaded and asks again: the same terminal, its screen as it is now.
    let (again, screen) = show(&h, &id).await;
    assert_eq!(again.id, term);
    let seen = plain_text(&screen.lock());
    assert!(
        seen.contains("fake-claude-tui") && seen.contains("fake-turn bonjour"),
        "{seen:?}"
    );
    assert_eq!(h.launches(&r).len(), launched);
    assert_eq!(terminal_of(&h, &id), term);
}

#[tokio::test(flavor = "multi_thread")]
async fn remote_control_ending_stops_an_interface_no_window_showed_and_leaves_one_that_was() {
    let h = harness("tui-remote-ends");
    let (p, r) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let unseen = agent_on(&h, &p, "pro").await.id;
    let seen = agent_on(&h, &p, "pro").await.id;
    let working = agent_on(&h, &p, "pro").await.id;
    h.remote_all(true);
    for id in [&unseen, &seen, &working] {
        h.wait("its interface", |h| interface_runs(h, id)).await;
    }
    show(&h, &seen).await;
    // Up and reading its input: a Ctrl+C sent while it starts would be lost.
    for id in [&unseen, &seen, &working] {
        ready(&h, id).await;
    }
    // A turn under way is not cut.
    h.core.agent(&working).unwrap().lock().meta.status = AgentStatus::Running;
    let (term, kept) = (terminal_of(&h, &unseen), terminal_of(&h, &seen));

    h.remote_all(false);
    h.wait("the interface stopped", |h| !interface_runs(h, &unseen))
        .await;
    // Each agent's turn of the setting is its own task: waited for, not assumed over.
    for id in [&unseen, &seen, &working] {
        h.wait("the remote control out", |h| !h.agent(id).remote_control)
            .await;
    }
    assert!(!h.core.hooks.holds(&unseen));
    assert!(h.agent(&unseen).terminal);
    // Quit as its user would, with two Ctrl+C, so that it archives its remote session.
    h.wait("its terminal closed", |h| !h.core.pty.exists(&term))
        .await;
    assert_eq!(ctrl_c(&h, &r).len(), 2);
    // Shown once: it goes on, with the choice for its next start.
    h.settle_remote().await;
    assert!(interface_runs(&h, &seen) && h.core.pty.exists(&kept));
    assert!(interface_runs(&h, &working));

    // By hand: turned on, its interface is started again; turned off, stopped again.
    h.core.set_remote_control(&unseen, true).await.unwrap();
    assert!(interface_runs(&h, &unseen));
    assert_eq!(h.agent(&unseen).remote_source, Some(RemoteSource::Hand));
    h.core.set_remote_control(&unseen, false).await.unwrap();
    assert!(!interface_runs(&h, &unseen));
    // Turned off by hand, it is set apart whether a setting covered it or not.
    assert!(h.agent(&unseen).remote_off);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interface_running_a_turn_when_remote_control_ends_stops_with_the_turn() {
    let h = harness("tui-remote-ends-late");
    let (p, _) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let stops = agent_on(&h, &p, "pro").await.id;
    let stays = agent_on(&h, &p, "pro").await.id;
    h.remote_all(true);
    for id in [&stops, &stays] {
        ready(&h, id).await;
        h.core
            .send_message(id, "lent".into(), Vec::new())
            .await
            .unwrap();
        h.wait("its turn", |h| h.agent(id).status == AgentStatus::Running)
            .await;
    }

    h.remote_all(false);
    for id in [&stops, &stays] {
        h.wait("the remote control out", |h| !h.agent(id).remote_control)
            .await;
    }
    // A turn is not cut.
    h.settle_remote().await;
    assert!(interface_runs(&h, &stops) && interface_runs(&h, &stays));
    // One of them gets Remote Control back, by hand, before its turn ends.
    h.core.set_remote_control(&stays, true).await.unwrap();

    for id in [&stops, &stays] {
        h.core.interrupt(id).await.unwrap();
        h.wait("its turn over", |h| {
            h.agent(id).status != AgentStatus::Running
        })
        .await;
    }
    // Over, with no window and no Remote Control: it stops. The other one holds it.
    let term = terminal_of(&h, &stays);
    h.wait("the interface stopped", |h| !interface_runs(h, &stops))
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert!(interface_runs(&h, &stays) && h.core.pty.exists(&term));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_view_tells_whether_the_interface_that_runs_was_started_with_remote_control() {
    let h = harness("tui-interface-remote");
    let (p, _) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let with = agent_on(&h, &p, "pro").await.id;
    let without = agent_on(&h, &p, "pro").await.id;
    let idle = agent_on(&h, &p, "pro").await.id;
    let started = |id: &str| h.core.agent(id).unwrap().lock().view().interface_remote;
    // No interface runs.
    assert_eq!(started(&idle), None);
    // Started with Remote Control on.
    h.core.set_remote_control(&with, true).await.unwrap();
    assert_eq!(started(&with), Some(true));
    // Started without it.
    open_interface(&h, &without).await;
    assert_eq!(started(&without), Some(false));
    // Turned on while it runs: it has not got it before its next start.
    h.core.set_remote_control(&without, true).await.unwrap();
    assert!(h.agent(&without).remote_control);
    assert_eq!(started(&without), Some(false));
    // Stopped: none runs.
    h.core.set_agent_view(&without, false).await.unwrap();
    assert_eq!(started(&without), None);
}

#[tokio::test(flavor = "multi_thread")]
async fn turning_remote_control_on_or_off_by_hand_says_whether_the_interface_waits_for_its_next_start(
) {
    let h = harness("tui-rc-next-start");
    let (p, _) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = agent_on(&h, &p, "pro").await.id;
    let started = |h: &Harness| h.core.agent(&id).unwrap().lock().view().interface_remote;
    let set = |on: bool| {
        let (core, id) = (h.core.clone(), id.clone());
        async move { core.set_remote_control(&id, on).await.unwrap() }
    };
    // No interface runs: it is started with the change, which is made, nothing waits.
    assert!(!set(true).await);
    assert_eq!(started(&h), Some(true));
    // Off: no window showed it, it stops.
    assert!(!set(false).await);
    assert!(!interface_runs(&h, &id));

    // A window shows an interface started without Remote Control: turned on, it has it from its
    // next start.
    open_interface(&h, &id).await;
    assert_eq!(started(&h), Some(false));
    assert!(set(true).await);
    assert!(h.agent(&id).remote_control);
    // Turned off again, what runs has none either: nothing waits.
    assert!(!set(false).await);

    // Started with it, shown, turned off: it keeps it until its next start.
    assert!(set(true).await);
    h.core.kill_terminal(&terminal_of(&h, &id));
    h.wait("the interface over", |h| !interface_runs(h, &id))
        .await;
    open_interface(&h, &id).await;
    assert_eq!(started(&h), Some(true));
    assert!(set(false).await);
    assert!(!h.agent(&id).remote_control);
    // And turned on again, it has it: nothing waits.
    assert!(!set(true).await);

    // In Escouade's view the stream holds it at once: nothing waits.
    h.core.settings.write().terminal_view = false;
    let stream = agent_on(&h, &p, "pro").await.id;
    assert!(!h.agent(&stream).terminal);
    assert!(!h.core.set_remote_control(&stream, true).await.unwrap());
    assert!(!h.core.set_remote_control(&stream, false).await.unwrap());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_window_taking_an_interface_over_shows_it_under_the_lock_that_finds_it() {
    let h = harness("tui-take-over");
    let (p, _) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = agent_on(&h, &p, "pro").await.id;
    h.remote_all(true);
    h.wait("its interface", |h| interface_runs(h, &id)).await;
    let handle = h.core.agent(&id).unwrap();
    // Found by what starts it: nobody shows it.
    let (_, output) = h.core.running_interface(&handle, false).unwrap();
    assert!(!output.shown());
    // Found by a window: shown at once, with nothing between that could stop it.
    let (_, output) = h.core.running_interface(&handle, true).unwrap();
    assert!(output.shown());
    handle.lock().meta.remote_control = false;
    h.core.stop_unseen_interface(&handle);
    assert!(interface_runs(&h, &id));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interface_without_remote_control_starts_without_it() {
    let h = harness("tui-no-remote");
    let (p, r) = h.project(false).await;
    second_account(&h);
    let id = agent_on(&h, &p, "pro").await.id;
    h.core.set_agent_view(&id, true).await.unwrap();
    open_interface(&h, &id).await;
    let launch = h.launches(&r).pop().unwrap();
    assert!(
        !launch.iter().any(|a| a == "--remote-control"),
        "{launch:?}"
    );
}

fn texts(h: &Harness, id: &str, kind: &str) -> Vec<String> {
    h.items(id)
        .iter()
        .filter(|i| i["kind"] == kind)
        .map(|i| i["text"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// What the agent is told about its Remote Control being unavailable.
fn unavailable(h: &Harness, id: &str) -> Vec<String> {
    texts(h, id, "notice")
        .into_iter()
        .filter(|t| t.starts_with("Remote control indisponible : "))
        .collect()
}

/// The arguments of each launch of Claude Code's interface in `cwd` (the stream's are left out).
fn interface_launches(h: &Harness, cwd: &std::path::Path) -> Vec<Vec<String>> {
    h.launches(cwd)
        .into_iter()
        .filter(|a| !a.iter().any(|x| x == "stream-json"))
        .collect()
}

fn asks_remote_control(args: &[String]) -> bool {
    args.iter().any(|a| a.starts_with("--remote-control"))
}

/// The remote control of the agent, with the interface in the Claude Code view started for it in
/// force by hand (no setting), without waiting for anything to run.
fn hold_remote_control(h: &Harness, id: &str) {
    let a = h.core.agent(id).unwrap();
    let mut rt = a.lock();
    rt.meta.remote_control = true;
    rt.meta.remote_source = Some(RemoteSource::Hand);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interface_that_closes_right_after_starting_with_remote_control_is_started_again_without_it(
) {
    let h = harness("tui-rc-fallback");
    let (p, r) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = agent_on(&h, &p, "pro").await.id;
    // Claude Code closes as soon as it is started with Remote Control.
    h.fake_flag(&r, "tui-rc-exit");
    h.remote_all(true);
    // No window showed it: started again, in the background, without the option.
    h.wait("the interface started again without it", |h| {
        interface_launches(h, &r).len() >= 2
            && h.core.agent(&id).unwrap().lock().view().interface_remote == Some(false)
    })
    .await;
    let launches = interface_launches(&h, &r);
    assert_eq!(launches.len(), 2, "{launches:?}");
    assert!(asks_remote_control(&launches[0]), "{:?}", launches[0]);
    assert!(!asks_remote_control(&launches[1]), "{:?}", launches[1]);
    // Told, once; Remote Control stays in force for the agent.
    assert_eq!(
        unavailable(&h, &id),
        ["Remote control indisponible : l’interface de Claude Code s’est fermée juste après son lancement avec le remote control ; elle est relancée sans lui."]
    );
    assert!(h.agent(&id).remote_control);
    // The one started again, without the option, says it is up by the registry alone.
    ready(&h, &id).await;
    // What the settings do again starts nothing.
    h.settle_remote().await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert_eq!(interface_launches(&h, &r).len(), 2);

    // Only once: the interface started without the option closing soon after starts nothing.
    h.core.pty.kill(&terminal_of(&h, &id));
    h.wait("the interface over", |h| !interface_runs(h, &id))
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    assert_eq!(interface_launches(&h, &r).len(), 2);
    assert_eq!(unavailable(&h, &id).len(), 1);
}

/// An interface started with Remote Control that a window shows ends soon after (`typed`: the
/// user types `/exit` on the box about the folder, else Claude Code closes with code 1 by itself):
/// the user saw it, and may have ended it themselves. No failure of the option, nothing told,
/// nothing started again; the next launch asks for the option as before.
async fn a_shown_interface_ends_soon_after_it_started(name: &str, typed: bool) {
    let h = harness(name);
    let (p, r) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = agent_on(&h, &p, "pro").await.id;
    h.fake_flag(&r, if typed { "tui-welcome" } else { "tui-rc-exit" });
    hold_remote_control(&h, &id);
    // The window opens it: started with Remote Control, shown, closed at once.
    let (info, _) = open_interface(&h, &id).await;
    if typed {
        h.core.pty.write(&info.id, b"/exit\r").unwrap();
    }
    h.wait("the interface over", |h| !interface_runs(h, &id))
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    assert!(unavailable(&h, &id).is_empty());
    assert!(!h.core.agent(&id).unwrap().lock().remote_fallback);
    // Nothing starts it again by itself: « Relancer » is the window's.
    assert_eq!(interface_launches(&h, &r).len(), 1);
    h.fake_unflag(&r, "tui-rc-exit");
    // Which asks for the option as the first launch did.
    open_interface(&h, &id).await;
    let launches = interface_launches(&h, &r);
    assert_eq!(launches.len(), 2);
    assert!(asks_remote_control(&launches[0]));
    assert!(asks_remote_control(&launches[1]), "{:?}", launches[1]);
    assert!(h.agent(&id).remote_control);
    assert_eq!(
        h.core.agent(&id).unwrap().lock().view().interface_remote,
        Some(true)
    );
    assert!(unavailable(&h, &id).is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interface_a_window_showed_that_closes_by_itself_soon_after_starting_is_no_failure_of_remote_control(
) {
    a_shown_interface_ends_soon_after_it_started("tui-rc-shown-closes", false).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interface_a_window_showed_that_its_user_exits_soon_after_starting_is_no_failure_of_remote_control(
) {
    a_shown_interface_ends_soon_after_it_started("tui-rc-shown-exit", true).await;
}

/// A wait of a view's test that times out tells what to read the failure by (ESC-125): the screen
/// of the agent's interface, whether its process runs, and the end of the fake's logs.
#[tokio::test(flavor = "multi_thread")]
async fn a_wait_that_times_out_tells_the_screen_the_interfaces_state_and_the_fakes_logs() {
    let h = harness("tui-wait-report");
    let (p, r) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = agent_on(&h, &p, "pro").await.id;
    let (info, _) = open_interface(&h, &id).await;
    h.core.pty.write(&info.id, b"bonjour").unwrap();
    h.wait("the line typed", |h| h.views_report().contains("> bonjour"))
        .await;
    let said = h.timed_out("nothing");
    assert!(
        said.starts_with("timed out waiting for nothing\n"),
        "{said}"
    );
    assert!(said.contains(&format!("--- agent {id} (")), "{said}");
    assert!(
        said.contains(&format!("interface {} (running, started ", info.id)),
        "{said}"
    );
    let screen = &said[said.find("screen (").unwrap()..];
    assert!(screen.contains("  | fake-claude-tui"), "{said}");
    assert!(screen.contains("> bonjour\n"), "{said}");
    // The fake's launch log, named after the repository it ran in.
    let key: String = r
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    assert!(
        said.contains(&format!("fake-claude-{key}.jsonl (")),
        "{said}"
    );
    assert!(said.contains("\"argv\""), "{said}");

    // Its process gone: said so, and how it ended.
    h.core.pty.write(&info.id, b"\x15/exit\r").unwrap();
    h.wait("the interface over", |h| !interface_runs(h, &id))
        .await;
    let said = h.views_report();
    assert!(
        said.contains("no interface running, last exit Some("),
        "{said}"
    );
}

/// A key typed as soon as the interface shows its first screen is read as typed, however long the
/// rest of its start takes (ESC-125: on a loaded runner, `/exit` reached the fake interface while
/// its terminal was still cooked, which turned its Enter into a line feed: it never ended).
#[tokio::test(flavor = "multi_thread")]
async fn a_key_typed_as_soon_as_the_first_screen_shows_is_read_as_typed() {
    let h = harness("tui-typed-at-once");
    let (p, r) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = agent_on(&h, &p, "pro").await.id;
    h.fake_flag(&r, "tui-slow-start");
    let (info, _) = open_interface(&h, &id).await;
    h.core.pty.write(&info.id, b"/exit\r").unwrap();
    h.wait("the interface over", |h| !interface_runs(h, &id))
        .await;
    assert_eq!(
        h.core
            .agent(&id)
            .unwrap()
            .lock()
            .tui_exit
            .as_ref()
            .and_then(|e| e.code),
        Some(0)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interface_shown_after_it_started_in_the_background_is_no_failure_when_it_closes() {
    let h = harness("tui-rc-shown-late");
    let (p, r) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = agent_on(&h, &p, "pro").await.id;
    // Started on the box about the folder, a window takes it over, then its user exits it.
    h.fake_flag(&r, "tui-welcome");
    h.core.set_remote_control(&id, true).await.unwrap();
    first_screen(&h, &id).await;
    show(&h, &id).await;
    let term = terminal_of(&h, &id);
    h.core.pty.write(&term, b"/exit\r").unwrap();
    h.wait("the interface over", |h| !interface_runs(h, &id))
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    assert!(unavailable(&h, &id).is_empty());
    assert!(!h.core.agent(&id).unwrap().lock().remote_fallback);
    assert_eq!(interface_launches(&h, &r).len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_link_of_an_interface_started_with_remote_control_is_read_from_its_transcript() {
    let h = harness("tui-rc-link");
    let (p, r) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = agent_on(&h, &p, "pro").await.id;
    let state = |h: &Harness| h.core.agent(&id).unwrap().lock().view().remote_state;
    assert_eq!(state(&h), None);
    h.core.set_remote_control(&id, true).await.unwrap();
    // Linked by its registry or its `bridge-session` entry; its link comes with the
    // `bridge_status` entry it writes next, read a moment later under load.
    h.wait("the interface linked, with its link", |h| {
        state(h).as_deref() == Some("connected") && h.agent(&id).remote_url.is_some()
    })
    .await;
    // The link it wrote: the interface's own, not the stream's session.
    let m = h.agent(&id);
    let url = m.remote_url.unwrap();
    assert!(
        url.starts_with("https://claude.ai/code/session_fake_tui_"),
        "{url}"
    );
    assert_eq!(m.remote_session, None);
    // It is in the view the window gets.
    let view = serde_json::to_value(h.core.agent(&id).unwrap().lock().view()).unwrap();
    assert_eq!(view["remoteState"], "connected");
    assert_eq!(view["remoteUrl"], url.as_str());
    assert!(unavailable(&h, &id).is_empty());

    // The user closes it: its link is over with it, and the next interface knows nothing of it
    // until it tells it. Closing it soon after it started is no failure of Remote Control.
    let term = terminal_of(&h, &id);
    h.core.kill_terminal(&term);
    h.wait("the interface over", |h| !interface_runs(h, &id))
        .await;
    assert_eq!(state(&h), None);
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    assert!(unavailable(&h, &id).is_empty());
    assert!(!h.core.agent(&id).unwrap().lock().remote_fallback);
    assert_eq!(interface_launches(&h, &r).len(), 1);
    open_interface(&h, &id).await;
    assert_eq!(interface_launches(&h, &r).len(), 2);
    h.wait("the interface linked again", |h| {
        state(h).as_deref() == Some("connected")
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_remote_control_that_does_not_open_is_told_once_and_a_late_link_connects_it() {
    let h = harness("tui-rc-unavailable");
    let (p, r) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = agent_on(&h, &p, "pro").await.id;
    // Up, but Remote Control never opens (an account with no claude.ai subscription, a network or
    // an organization that refuses it): Claude Code says nothing of it in its terminal.
    h.fake_flag(&r, "tui-rc-hold");
    h.core.set_remote_control(&id, true).await.unwrap();
    let state = |h: &Harness| h.core.agent(&id).unwrap().lock().view().remote_state;
    h.wait("the verdict", |h| {
        state(h).as_deref() == Some("unavailable")
    })
    .await;
    assert_eq!(unavailable(&h, &id), [NO_LINK]);
    // It runs (it did not close), with no link, and it takes a message: it is up.
    assert!(interface_runs(&h, &id));
    assert_eq!(h.agent(&id).remote_url, None);
    say_in_tui(&h, &id, "bonjour").await;
    assert_eq!(texts(&h, &id, "text"), ["Fini : bonjour"]);
    // Remote Control stays in force for the agent: its next start tries again.
    assert!(h.agent(&id).remote_control);
    // Told once.
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    assert_eq!(unavailable(&h, &id).len(), 1);
    assert_eq!(state(&h).as_deref(), Some("unavailable"));

    // The link comes after all: connected, and what fails after that is news again.
    h.fake_unflag(&r, "tui-rc-hold");
    h.wait("the late link", |h| {
        state(h).as_deref() == Some("connected")
    })
    .await;
    assert!(!h.core.agent(&id).unwrap().lock().remote_told);
    assert!(h.agent(&id).remote_url.is_some());
    assert_eq!(unavailable(&h, &id).len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interface_that_writes_no_link_in_its_remote_control_entry_is_unavailable_at_once() {
    let h = harness("tui-rc-inactive");
    let (p, r) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = agent_on(&h, &p, "pro").await.id;
    // Claude Code runs but the link failed: its entry says what it did not do.
    h.fake_flag(&r, "tui-rc-inactive");
    h.core.set_remote_control(&id, true).await.unwrap();
    let state = |h: &Harness| h.core.agent(&id).unwrap().lock().view().remote_state;
    h.wait("the verdict", |h| {
        state(h).as_deref() == Some("unavailable")
    })
    .await;
    assert_eq!(
        unavailable(&h, &id),
        ["Remote control indisponible : Claude Code n’a pas pu ouvrir la session distante (« /remote-control is not active »)."]
    );
    assert_eq!(h.agent(&id).remote_url, None);
    assert!(interface_runs(&h, &id) && h.agent(&id).remote_control);
    // The entry makes it ready, and told once: the waiting for a link says no more.
    ready(&h, &id).await;
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    assert_eq!(unavailable(&h, &id).len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_link_the_registry_alone_tells_connects_the_interface() {
    let h = harness("tui-rc-registry-only");
    let (p, r) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = agent_on(&h, &p, "pro").await.id;
    // Its session carries a `bridgeSessionId` in the registry; its transcript says nothing.
    h.fake_flag(&r, "tui-rc-registry-only");
    h.core.set_remote_control(&id, true).await.unwrap();
    let state = |h: &Harness| h.core.agent(&id).unwrap().lock().view().remote_state;
    h.wait("connected", |h| state(h).as_deref() == Some("connected"))
        .await;
    // Linked, with no address to give.
    assert_eq!(h.agent(&id).remote_url, None);
    // Nothing is said of a verdict, however long it waits.
    tokio::time::sleep(std::time::Duration::from_millis(1800)).await;
    assert_eq!(state(&h).as_deref(), Some("connected"));
    assert!(unavailable(&h, &id).is_empty());
}

/// The refusal of a message to an interface that is not ready.
const NOT_READY: &str = "L’interface de Claude Code de cet agent n’est pas encore prête (premier lancement ou dossier à approuver) : ouvre l’agent pour la voir.";

/// What the agent is told when its interface, up, never opened Remote Control.
const NO_LINK: &str = "Remote control indisponible : l’interface de Claude Code ne l’a pas ouvert (compte sans abonnement claude.ai, réseau, proxy ou règle de l’organisation).";

/// Sends `text` to the agent until its interface takes it (60 s at most), the refusals met on the
/// way being the one of an interface that is not ready.
async fn send_when_ready(h: &Harness, id: &str, text: &str) {
    let started = std::time::Instant::now();
    loop {
        match h.core.send_message(id, text.into(), Vec::new()).await {
            Ok(()) => return,
            Err(e) => {
                assert_eq!(e.to_string(), NOT_READY);
                assert!(
                    started.elapsed() < std::time::Duration::from_secs(60),
                    "{e:#}"
                );
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        }
    }
}

/// The agent's interface, started with no window, has printed its first screen.
async fn first_screen(h: &Harness, id: &str) {
    h.wait("its first screen", |h| {
        let run = h.core.agent(id).unwrap();
        let kept = run.lock().tui.as_ref().map(|t| t.output.kept());
        plain_text(&String::from_utf8_lossy(&kept.unwrap_or_default())).contains("fake-claude-tui")
    })
    .await;
}

/// The folder where the Claude Code of the test's second account registers its interfaces.
fn registry_dir(h: &Harness) -> std::path::PathBuf {
    h.dir.join("claude-pro").join("sessions")
}

/// An agent whose interface, started with no window, sits on its welcome screen (until the
/// `tui-welcome` file of the fake in the returned folder goes): it refuses a message, as the text
/// typed would choose an option there.
async fn background_interface_on_its_welcome_screen(
    name: &str,
) -> (Harness, String, std::path::PathBuf) {
    let h = harness(name);
    let (p, r) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = agent_on(&h, &p, "pro").await.id;
    h.fake_flag(&r, "tui-welcome");
    h.core.set_remote_control(&id, true).await.unwrap();
    first_screen(&h, &id).await;
    (h, id, r)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_message_to_a_background_interface_waits_until_the_registry_lists_its_session() {
    let (h, id, r) = background_interface_on_its_welcome_screen("tui-ready-registry").await;
    // Typed there, the text would choose an option of the box.
    for _ in 0..2 {
        let refusal = h
            .core
            .send_message(&id, "bonjour".into(), Vec::new())
            .await
            .unwrap_err()
            .to_string();
        assert_eq!(refusal, NOT_READY);
    }
    assert!(texts(&h, &id, "user").is_empty());
    assert!(!h.core.agent(&id).unwrap().lock().interface_ready());
    assert!(std::fs::read_dir(registry_dir(&h)).is_err());
    // Past the screen: it registers its session, and takes the message.
    h.fake_unflag(&r, "tui-welcome");
    send_when_ready(&h, &id, "bonjour").await;
    // Its link comes in its own time after it is up.
    h.wait("connected", |h| {
        h.core
            .agent(&id)
            .unwrap()
            .lock()
            .view()
            .remote_state
            .as_deref()
            == Some("connected")
    })
    .await;
    h.wait("the turn's end", |h| {
        h.items(&id).iter().any(|i| i["kind"] == "turn")
    })
    .await;
    assert_eq!(texts(&h, &id, "text"), ["Fini : bonjour"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_registry_makes_an_interface_started_without_the_option_ready_too() {
    // The agent holds Remote Control but its interface starts without the option: nothing but the
    // registry says it is up.
    let h = harness("tui-ready-no-option");
    let (p, r) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = agent_on(&h, &p, "pro").await.id;
    h.fake_flag(&r, "tui-welcome");
    hold_remote_control(&h, &id);
    h.core.agent(&id).unwrap().lock().remote_fallback = true;
    h.core.start_interface(&id).await.unwrap();
    first_screen(&h, &id).await;
    assert!(!asks_remote_control(
        &interface_launches(&h, &r).pop().unwrap()
    ));
    assert!(!h.core.agent(&id).unwrap().lock().interface_ready());
    h.fake_unflag(&r, "tui-welcome");
    ready(&h, &id).await;
    assert_eq!(h.core.agent(&id).unwrap().lock().view().remote_state, None);
    say_in_tui(&h, &id, "bonjour").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_registry_of_other_sessions_or_half_written_makes_no_interface_ready() {
    let (h, id, r) = background_interface_on_its_welcome_screen("tui-ready-registry-junk").await;
    let session = h.agent(&id).session_id.unwrap();
    let dir = registry_dir(&h);
    std::fs::create_dir_all(&dir).unwrap();
    let entry = |sid: &str, entrypoint: &str| {
        json!({ "pid": 1, "sessionId": sid, "kind": "interactive", "entrypoint": entrypoint })
            .to_string()
    };
    // The stream's process of the same session, another session's interface, a file being written,
    // one that holds no JSON, one that is empty, a folder, and the key file of a box not answered.
    std::fs::write(dir.join("901.json"), entry(&session, "sdk-cli")).unwrap();
    std::fs::write(dir.join("902.json"), entry("another-session", "cli")).unwrap();
    std::fs::write(dir.join("903.json"), r#"{"pid":3,"sessionId":""#).unwrap();
    std::fs::write(dir.join("904.json"), "not json").unwrap();
    std::fs::write(dir.join("905.json"), "").unwrap();
    std::fs::create_dir_all(dir.join("907.json")).unwrap();
    std::fs::write(dir.join("906.0a1b2c.key"), entry(&session, "cli")).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    assert!(!h.core.agent(&id).unwrap().lock().interface_ready());
    assert_eq!(
        h.core
            .send_message(&id, "bonjour".into(), Vec::new())
            .await
            .unwrap_err()
            .to_string(),
        NOT_READY
    );
    // Its own entry does.
    h.fake_unflag(&r, "tui-welcome");
    ready(&h, &id).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_message_to_an_interface_a_window_showed_before_is_not_held_back() {
    let (h, id, _) = background_interface_on_its_welcome_screen("tui-shown-before").await;
    assert!(h
        .core
        .send_message(&id, "bonjour".into(), Vec::new())
        .await
        .is_err());
    // A window takes it over later: it is shown from then on.
    show(&h, &id).await;
    say_in_tui(&h, &id, "bonjour").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_background_interface_that_starts_normally_takes_a_message_once_it_is_up() {
    let h = harness("tui-ready");
    let (p, _) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = agent_on(&h, &p, "pro").await.id;
    h.core.set_remote_control(&id, true).await.unwrap();
    send_when_ready(&h, &id, "bonjour").await;
    h.wait("the turn's end", |h| {
        h.items(&id).iter().any(|i| i["kind"] == "turn")
    })
    .await;
    assert_eq!(texts(&h, &id, "text"), ["Fini : bonjour"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interface_a_window_showed_takes_a_message_whatever_it_has_said() {
    let h = harness("tui-shown-not-ready");
    let (p, _) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = agent_on(&h, &p, "pro").await.id;
    // Started by the window, with no Remote Control: it never says anything of it.
    open_interface(&h, &id).await;
    assert_eq!(h.core.agent(&id).unwrap().lock().view().remote_state, None);
    // The user sees the screen: what is typed there is on them.
    say_in_tui(&h, &id, "bonjour").await;
    assert_eq!(texts(&h, &id, "text"), ["Fini : bonjour"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_message_goes_through_the_interface_and_its_turn_comes_back_from_the_transcript() {
    let h = harness("tui-turn");
    let id = agent_in_tui(&h).await;
    let a = h.agent(&id);
    assert!(a.terminal);
    let session = a
        .session_id
        .clone()
        .expect("a session of its own from the start");
    // No stream's process: the interface has the session.
    assert!(!h.alive(&id));
    assert!(h.core.agent(&id).unwrap().lock().tui.is_some());

    h.core
        .send_message(&id, "bonjour".into(), Vec::new())
        .await
        .unwrap();
    // The turn's row, then its status: the row can be shown a moment before the agent is told done (the CI's Windows,
    // 2026-11-05: `Running`).
    h.wait("the turn's end", |h| {
        h.items(&id).iter().any(|i| i["kind"] == "turn") && h.agent(&id).status == AgentStatus::Done
    })
    .await;
    let user = h
        .items(&id)
        .into_iter()
        .find(|i| i["kind"] == "user")
        .unwrap();
    assert_eq!(user["text"], "bonjour");
    assert_eq!(user["origin"], "terminal");
    assert_eq!(texts(&h, &id, "text"), ["Fini : bonjour"]);
    let a = h.agent(&id);
    assert_eq!(a.prompts, 1);
    assert!(a.cost > 0.0, "counted at list prices");
    assert_eq!(a.session_id.as_deref(), Some(session.as_str()));
    // Where the read stopped is kept, for the next start.
    let (file, offset) = a.transcript_read.clone().unwrap();
    assert!(file.ends_with(&format!("{session}.jsonl")));
    assert_eq!(offset, std::fs::metadata(&file).unwrap().len());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_message_on_several_lines_reaches_the_interface_whole_in_one_turn() {
    let h = harness("tui-lines");
    let id = agent_in_tui(&h).await;
    say_in_tui(&h, &id, "un\ndeux\r\ntrois").await;
    assert_eq!(texts(&h, &id, "user"), ["un\ndeux\ntrois"]);
    assert_eq!(texts(&h, &id, "text"), ["Fini : un\ndeux\ntrois"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_message_waits_for_the_interface_to_take_pastes_again_after_it_is_up() {
    // As the real one under Windows: listed in the registry, then bracketed paste off for a while,
    // when a line break pasted would send the message's first line alone.
    let h = harness("tui-paste-late");
    let (p, r) = h.project(false).await;
    second_account(&h);
    let id = agent_on(&h, &p, "pro").await.id;
    h.fake_flag(&r, "tui-paste-late");
    h.core.set_agent_view(&id, true).await.unwrap();
    open_interface(&h, &id).await;
    up(&h, &id).await;
    say_in_tui(&h, &id, "un\ndeux").await;
    assert_eq!(texts(&h, &id, "user"), ["un\ndeux"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_permission_asked_in_the_interface_is_told_and_answered_there() {
    let h = harness("tui-permission");
    let id = agent_in_tui(&h).await;
    h.core
        .send_message(&id, "permission".into(), Vec::new())
        .await
        .unwrap();
    h.wait("the agent waiting", |h| {
        h.agent(&id).status == AgentStatus::Waiting
    })
    .await;
    assert!(
        h.core.alerts.lock().iter().any(|a| a.contains("echo tui")),
        "{:?}",
        h.core.alerts.lock()
    );
    // A message would go to its dialog: refused.
    let refused = h.core.send_message(&id, "et ça ?".into(), Vec::new()).await;
    assert!(refused.is_err());
    // Answered in its terminal (Enter: « Yes »).
    let term = h
        .core
        .agent(&id)
        .unwrap()
        .lock()
        .view()
        .terminal_id
        .unwrap();
    h.core.pty.write(&term, b"\r").unwrap();
    h.wait("the turn's end", |h| {
        h.agent(&id).status == AgentStatus::Done
    })
    .await;
    let tool = h
        .items(&id)
        .into_iter()
        .find(|i| i["kind"] == "tool")
        .unwrap();
    assert_eq!(
        (tool["name"].as_str(), tool["status"].as_str()),
        (Some("Bash"), Some("ok"))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interrupt_from_the_window_is_escape_in_the_interface() {
    let h = harness("tui-interrupt");
    let id = agent_in_tui(&h).await;
    h.core
        .send_message(&id, "lent".into(), Vec::new())
        .await
        .unwrap();
    h.wait("the turn running", |h| {
        h.agent(&id).status == AgentStatus::Running
    })
    .await;
    h.core.interrupt(&id).await.unwrap();
    h.wait("the turn interrupted", |h| {
        h.items(&id)
            .iter()
            .any(|i| i["kind"] == "turn" && i["interrupted"] == true)
    })
    .await;
}

/// `text` sent to the agent's interface, then cancelled before its answer by Escape (Escouade's
/// button, or the key typed in its terminal), until its turn ends as interrupted.
async fn cancel_in_tui(h: &Harness, id: &str, text: &str, from_terminal: bool) {
    h.core
        .send_message(id, text.into(), Vec::new())
        .await
        .unwrap();
    h.wait("the turn running", |h| {
        h.agent(id).status == AgentStatus::Running
    })
    .await;
    if from_terminal {
        h.core
            .pty
            .write_from_window(&terminal_of(h, id), b"\x1b")
            .unwrap();
    } else {
        h.core.interrupt(id).await.unwrap();
    }
    h.wait("the turn interrupted", |h| {
        h.items(id)
            .iter()
            .any(|i| i["kind"] == "turn" && i["interrupted"] == true)
    })
    .await;
    assert!(!h.agent(id).status.is_active());
}

/// What the fake interface found in its prompt line at each Ctrl+C it read.
fn ctrl_c_lines(h: &Harness, id: &str) -> Vec<Value> {
    let cwd = std::path::PathBuf::from(h.agent(id).cwd);
    h.fake_log(&cwd, "quit")
        .into_iter()
        .map(|q| q["line"].clone())
        .collect()
}

/// What the conversation says of a cancelled message left in the interface's prompt.
fn left_in_prompt(h: &Harness, id: &str) -> Vec<String> {
    texts(h, id, "notice")
        .into_iter()
        .filter(|n| n.contains("remis dans sa saisie"))
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_turn_its_interface_cancels_before_any_answer_ends_when_its_registry_says_idle() {
    // Escape before the first answer: Claude Code puts the message back in its prompt, writes
    // nothing more in its transcript and calls no hook; its registry file says it idle.
    let h = harness("tui-cancelled");
    let id = agent_in_tui(&h).await;
    cancel_in_tui(&h, &id, "pense", false).await;
    // The next message is a turn of its own, ended by its transcript: the prompt, which held
    // Escouade's cancelled message alone, was emptied first by a single Ctrl+C (ESC-118).
    say_in_tui(&h, &id, "bonjour").await;
    assert_eq!(texts(&h, &id, "text"), ["Fini : bonjour"]);
    assert_eq!(texts(&h, &id, "user"), ["pense", "bonjour"]);
    assert_eq!(ctrl_c_lines(&h, &id), [json!("pense")]);
    assert!(left_in_prompt(&h, &id).is_empty());
    let turns: Vec<Value> = h
        .items(&id)
        .into_iter()
        .filter(|i| i["kind"] == "turn")
        .collect();
    assert_eq!(turns.len(), 2);
    assert_eq!(turns[1]["interrupted"], false);
    // Once only: the message after is pasted as it is.
    say_in_tui(&h, &id, "merci").await;
    assert_eq!(ctrl_c_lines(&h, &id).len(), 1);
    assert_eq!(texts(&h, &id, "text"), ["Fini : bonjour", "Fini : merci"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_message_cancelled_by_escape_typed_in_the_terminal_is_emptied_all_the_same() {
    // Escape types no text: the prompt holds Escouade's message alone still.
    let h = harness("tui-cancelled-key");
    let id = agent_in_tui(&h).await;
    cancel_in_tui(&h, &id, "pense", true).await;
    say_in_tui(&h, &id, "bonjour").await;
    assert_eq!(texts(&h, &id, "text"), ["Fini : bonjour"]);
    assert_eq!(ctrl_c_lines(&h, &id), [json!("pense")]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_draft_typed_in_the_terminal_after_a_cancelled_message_is_kept_and_said() {
    // The user typed after the cancelled message Claude Code put back: nothing is emptied, the
    // next message is added to the prompt, and the conversation says it.
    let h = harness("tui-cancelled-draft");
    let id = agent_in_tui(&h).await;
    cancel_in_tui(&h, &id, "pense", false).await;
    h.core
        .pty
        .write_from_window(&terminal_of(&h, &id), b" et ceci")
        .unwrap();
    say_in_tui(&h, &id, "bonjour").await;
    assert!(ctrl_c_lines(&h, &id).is_empty());
    assert_eq!(texts(&h, &id, "text"), ["Fini : pense et cecibonjour"]);
    assert_eq!(
        left_in_prompt(&h, &id).len(),
        1,
        "{:?}",
        texts(&h, &id, "notice")
    );
    // Said once: the prompt was sent with it.
    say_in_tui(&h, &id, "merci").await;
    assert_eq!(left_in_prompt(&h, &id).len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_draft_typed_before_escouades_message_is_kept_when_that_message_is_cancelled() {
    // The prompt held the user's draft when Escouade pasted its message: the cancelled message
    // Claude Code puts back holds the draft too, which is not emptied.
    let h = harness("tui-cancelled-before");
    let id = agent_in_tui(&h).await;
    h.core
        .pty
        .write_from_window(&terminal_of(&h, &id), b"brouillon ")
        .unwrap();
    cancel_in_tui(&h, &id, "pense", false).await;
    say_in_tui(&h, &id, "bonjour").await;
    assert!(ctrl_c_lines(&h, &id).is_empty());
    assert_eq!(texts(&h, &id, "text"), ["Fini : brouillon pensebonjour"]);
    assert_eq!(left_in_prompt(&h, &id).len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_turn_whose_registry_says_idle_before_its_transcript_ends_it_is_not_cut_short() {
    // Idle in the registry while the turn's end is not read yet (it comes within the grace): the
    // turn ends by its transcript, not interrupted.
    let h = harness("tui-idle-early");
    let id = agent_in_tui(&h).await;
    say_in_tui(&h, &id, "tard").await;
    let turns: Vec<Value> = h
        .items(&id)
        .into_iter()
        .filter(|i| i["kind"] == "turn")
        .collect();
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0]["interrupted"], false);
    assert_eq!(texts(&h, &id, "text"), ["Fini : tard"]);
}

/// The turns of the agent's conversation.
fn turns_of(h: &Harness, id: &str) -> Vec<Value> {
    h.items(id)
        .into_iter()
        .filter(|i| i["kind"] == "turn")
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_local_command_is_one_message_and_its_output_ends_its_turn() {
    // `/compact` as Claude Code 2.1.302 writes it: the text typed and the command, under one
    // prompt, then its output, and no end of a turn.
    let h = harness("tui-compact");
    let id = agent_in_tui(&h).await;
    let start = std::time::Instant::now();
    say_in_tui(&h, &id, "/compact").await;
    // Its output ended it, not the registry's idle (`IDLE_ENDS_TURN_MS`).
    assert!(start.elapsed() < std::time::Duration::from_secs(2));
    assert_eq!(texts(&h, &id, "user"), ["/compact"]);
    let notices = texts(&h, &id, "notice");
    assert!(
        notices.iter().any(|n| n == "Contexte compacté"),
        "{notices:?}"
    );
    // Its output under it, without its tags nor its ANSI codes (ESC-154).
    let output = h
        .items(&id)
        .into_iter()
        .find(|i| i["kind"] == "notice" && i["level"] == "output");
    assert_eq!(
        output.as_ref().map(|o| o["text"].clone()),
        Some(json!("Compacted (ctrl+o to see full summary)")),
        "{notices:?}"
    );
    no_local_command_tag(&h, &id);
    let turns = turns_of(&h, &id);
    assert_eq!(turns.len(), 1);
    assert_eq!(
        (&turns[0]["interrupted"], &turns[0]["isError"]),
        (&json!(false), &json!(false))
    );
    assert_eq!(h.agent(&id).status, AgentStatus::Done);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_local_command_that_fails_ends_its_turn_with_its_error() {
    let h = harness("tui-compact-fails");
    let id = agent_in_tui(&h).await;
    say_in_tui(&h, &id, "/compact echec").await;
    assert_eq!(texts(&h, &id, "user"), ["/compact echec"]);
    let turns = turns_of(&h, &id);
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0]["isError"], true, "{turns:?}");
    assert!(
        turns[0]["error"]
            .as_str()
            .is_some_and(|e| e.contains("Error during compaction")),
        "{turns:?}"
    );
    // The error on its card only, not again as a notice (ESC-154).
    let notices = texts(&h, &id, "notice");
    assert!(
        !notices
            .iter()
            .any(|n| n.contains("Error during compaction")),
        "{notices:?}"
    );
    no_local_command_tag(&h, &id);
}

/// No item of the conversation of `id` shows a tag of a local command (ESC-154).
fn no_local_command_tag(h: &Harness, id: &str) {
    for i in h.items(id) {
        let text = i["text"].as_str().unwrap_or_default();
        for tag in ["<local-command", "<command-", "<bash-", "\u{1b}"] {
            assert!(!text.contains(tag), "{i}");
        }
    }
}

/// `text` typed in the interface, until its turn comes back from the transcript.
async fn say_in_tui(h: &Harness, id: &str, text: &str) {
    let turns = |h: &Harness| h.items(id).iter().filter(|i| i["kind"] == "turn").count();
    let before = turns(h);
    h.core
        .send_message(id, text.into(), Vec::new())
        .await
        .unwrap();
    h.wait("the turn's end", |h| {
        turns(h) > before && !h.agent(id).status.is_active()
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn in_the_claude_code_view_commands_do_not_name_the_agent_but_the_next_ordinary_message_does()
{
    let h = harness("tui-name-after-commands");
    h.core.settings.write().claude_language = "en".into();
    let id = agent_in_tui(&h).await;
    for command in ["/clear", "/impeccable"] {
        say_in_tui(&h, &id, command).await;
        let a = h.agent(&id);
        assert_eq!((a.named, a.name.as_str()), (false, "agent-1"), "{command}");
        assert!(!h.core.agent(&id).unwrap().lock().naming_asked, "{command}");
    }
    say_in_tui(&h, &id, "Create the hello file").await;
    h.wait("named", |h| h.agent(&id).named).await;
    assert_eq!(h.agent(&id).name, "create-the-fake-en");
}

#[tokio::test(flavor = "multi_thread")]
async fn in_the_claude_code_view_the_automatic_resume_never_names_the_agent() {
    let h = harness("tui-name-not-by-continue");
    h.core.settings.write().claude_language = "en".into();
    let id = agent_in_tui(&h).await;
    say_in_tui(&h, &id, "/impeccable").await;
    // « continue », typed by the app itself after the usage limit: its entry in the transcript is
    // read like any other, and is no task.
    let turns = |h: &Harness| h.items(&id).iter().filter(|i| i["kind"] == "turn").count();
    let before = turns(&h);
    h.core.send_continue(&id).await.unwrap();
    h.wait("the resumed turn's end", |h| {
        turns(h) > before && !h.agent(&id).status.is_active()
    })
    .await;
    assert_eq!(texts(&h, &id, "user"), ["/impeccable", "continue"]);
    let a = h.agent(&id);
    assert_eq!((a.named, a.name.as_str()), (false, "agent-1"));
    assert!(!h.core.agent(&id).unwrap().lock().naming_asked);
    say_in_tui(&h, &id, "Create the hello file").await;
    h.wait("named", |h| h.agent(&id).named).await;
    assert_eq!(h.agent(&id).name, "create-the-fake-en");
}

#[tokio::test(flavor = "multi_thread")]
async fn in_the_claude_code_view_a_skill_with_arguments_names_the_agent_from_them() {
    let h = harness("tui-name-from-skill");
    h.core.settings.write().claude_language = "en".into();
    let id = agent_in_tui(&h).await;
    say_in_tui(&h, &id, "/impeccable propose un nouveau design").await;
    h.wait("named", |h| h.agent(&id).named).await;
    assert_eq!(h.agent(&id).name, "impeccable-propose-fake-en");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_claude_code_view_starts_without_its_own_update() {
    // Stopped with its tree when the view closes: an update it started would be cut (ESC-133).
    let h = harness("tui-no-autoupdater");
    let id = agent_in_tui(&h).await;
    let cwd = std::path::PathBuf::from(h.agent(&id).cwd);
    let launch = h
        .launch_log(&cwd)
        .into_iter()
        .rfind(|l| !l["argv"].to_string().contains("stream-json"))
        .expect("the interface's launch");
    assert_eq!(launch["autoUpdater"], serde_json::json!("1"), "{launch}");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interface_refused_escouades_tools_is_started_without_its_server() {
    // « Claude et les agents peuvent piloter Escouade » off: the interface's tools of Escouade are
    // refused, and the server itself denied in the one settings file it is given (its hooks'),
    // else an entry of the user's config would still give it the server's instructions (ESC-91).
    let h = harness("tui-escouade-refused");
    let id = agent_in_tui(&h).await;
    let cwd = std::path::PathBuf::from(h.agent(&id).cwd);
    let launch = h
        .launch_log(&cwd)
        .into_iter()
        .rfind(|l| !l["argv"].to_string().contains("stream-json"))
        .expect("the interface's launch");
    let argv: Vec<String> = launch["argv"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|a| a.as_str().map(str::to_string))
        .collect();
    assert!(
        argv.windows(2)
            .any(|w| w == ["--disallowedTools", "mcp__escouade"]),
        "{argv:?}"
    );
    let settings: Vec<&String> = argv
        .windows(2)
        .filter(|w| w[0] == "--settings")
        .map(|w| &w[1])
        .collect();
    assert_eq!(settings.len(), 1, "{argv:?}");
    let s: serde_json::Value =
        serde_json::from_slice(&std::fs::read(settings[0]).unwrap()).unwrap();
    assert!(s["hooks"]["PermissionRequest"].is_array(), "{s}");
    assert_eq!(
        s["deniedMcpServers"],
        serde_json::json!([{ "serverName": "escouade" }])
    );
    // Turned on: the interface says it gets Escouade at its next start.
    let next = |h: &Harness| h.core.agent(&id).unwrap().lock().view().escouade_next;
    assert!(!next(&h));
    h.core.set_mcp_enabled(true).unwrap();
    assert!(next(&h));
    h.core.mcp.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interface_given_escouade_has_its_token_in_its_file_never_on_its_command_line() {
    let h = harness("tui-escouade-granted");
    h.core.set_mcp_enabled(true).unwrap();
    h.wait("the server", |h| h.core.mcp.status().running).await;
    let id = agent_in_tui(&h).await;
    let cwd = std::path::PathBuf::from(h.agent(&id).cwd);
    let argv = h
        .launches(&cwd)
        .into_iter()
        .rfind(|argv| !argv.iter().any(|a| a == "stream-json"))
        .expect("the interface's launch");
    let at = argv
        .iter()
        .position(|a| a == "--mcp-config")
        .unwrap_or_else(|| panic!("no --mcp-config: {argv:?}"));
    let file = std::path::PathBuf::from(&argv[at + 1]);
    assert_eq!(file, h.core.data.mcp_agent_config(&id));
    let config: serde_json::Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    let token = config["mcpServers"]["escouade"]["headers"]["Authorization"]
        .as_str()
        .and_then(|a| a.strip_prefix("Bearer "))
        .expect("a bearer token")
        .to_string();
    assert!(!argv.iter().any(|a| a.contains(&token)), "{argv:?}");
    assert!(!argv.iter().any(|a| a == "--disallowedTools"), "{argv:?}");
    h.core.mcp.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_interface_starts_with_the_folder_of_the_files_joined_to_its_agent_readable() {
    let h = harness("tui-add-dir");
    let id = agent_in_tui(&h).await;
    let cwd = std::path::PathBuf::from(h.agent(&id).cwd);
    let launch = h
        .launches(&cwd)
        .into_iter()
        .rfind(|argv| !argv.iter().any(|a| a == "stream-json"))
        .expect("the interface's launch");
    let folder = h.core.data.agent_files(&id);
    let wanted = format!("--add-dir={}", folder.display());
    assert_eq!(
        launch.iter().filter(|a| a.starts_with("--add-dir")).count(),
        1,
        "{launch:?}"
    );
    assert!(launch.contains(&wanted), "{launch:?}");
    // Made before it starts.
    assert!(folder.is_dir());
    // The window copies a file dropped on its terminal there, as for a message, and types the path
    // its `claude` reads it by.
    let file = copy(&h, &id, "a.zip", b"zip");
    assert!(std::path::Path::new(&file.path).starts_with(&folder));
    assert_eq!(file.claude_path, file.path);
    // The files of a message stay refused in this view: they are typed in its terminal.
    let e = h
        .core
        .send_message_files(&id, "x".into(), vec![], vec![file])
        .await
        .unwrap_err();
    assert!(e.to_string().contains("terminal"), "{e}");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interface_killed_and_replaced_by_relancer_leaves_the_new_one_its_hooks_token() {
    let h = harness("tui-relaunch-hooks");
    let id = agent_in_tui(&h).await;
    let run = |h: &Harness| {
        let agent = h.core.agent(&id).unwrap();
        let rt = agent.lock();
        let t = rt.tui.as_ref().expect("an interface");
        (t.gen, t.term_id.clone())
    };
    let (old_gen, old_term) = run(&h);
    assert!(h.core.hooks.holds(&id));
    // « Relancer » in the Claude Code view: the window kills the interface, then opens another one,
    // which takes its place and gets a token of its own.
    h.core.pty.kill(&old_term);
    let info = h
        .core
        .open_agent_terminal(&id, (100, 30), true, |_| {})
        .await
        .unwrap();
    let (new_gen, new_term) = run(&h);
    assert_eq!(new_term, info.id);
    assert!(new_gen > old_gen);
    assert!(h.core.hooks.holds(&id));
    // The old interface's end, however late it comes: the new one keeps its token and its run.
    h.core.tui_ended(&id, old_gen);
    assert!(h.core.hooks.holds(&id));
    assert_eq!(run(&h).0, new_gen);
    // Its own end does take them.
    h.core.tui_ended(&id, new_gen);
    assert!(!h.core.hooks.holds(&id));
    assert!(h.core.agent(&id).unwrap().lock().tui.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn back_in_escouades_view_the_stream_resumes_the_same_session() {
    let h = harness("tui-back");
    let id = agent_in_tui(&h).await;
    h.core
        .send_message(&id, "bonjour".into(), Vec::new())
        .await
        .unwrap();
    h.wait("the turn's end", |h| {
        h.agent(&id).status == AgentStatus::Done
    })
    .await;
    let session = h.agent(&id).session_id.unwrap();
    let cwd = std::path::PathBuf::from(h.agent(&id).cwd);
    // The warm-up when the agent was made started one already, with no session to resume.
    let streams = |h: &Harness| {
        h.launch_log(&cwd)
            .into_iter()
            .filter(|l| l["argv"].to_string().contains("stream-json"))
            .collect::<Vec<_>>()
    };
    let before = streams(&h).len();
    h.core.set_agent_view(&id, false).await.unwrap();
    assert!(!h.agent(&id).terminal);
    assert!(h.core.agent(&id).unwrap().lock().tui.is_none());
    h.wait("the stream's process", |h| streams(h).len() > before)
        .await;
    let last = streams(&h).pop().unwrap();
    let args: Vec<&str> = last["argv"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|a| a.as_str())
        .collect();
    assert!(
        args.contains(&format!("--resume={session}").as_str()),
        "{args:?}"
    );
    assert!(args.contains(&"stream-json"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_tickets_agent_never_goes_to_the_claude_code_view() {
    let h = harness("tui-ticket");
    let (p, _) = h.project(false).await;
    let a = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                ticket_id: Some("t1".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta;
    assert!(h.core.set_agent_view(&a.id, true).await.is_err());
    assert!(!h.agent(&a.id).terminal);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_hook_without_the_token_of_an_interface_is_refused() {
    let h = harness("tui-token");
    let _id = agent_in_tui(&h).await;
    let port = h.core.hooks_port().unwrap();
    let call = |auth: Option<&str>| {
        let mut req = format!(
            "POST /hook HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n"
        );
        if let Some(a) = auth {
            req.push_str(&format!("Authorization: Bearer {a}\r\n"));
        }
        req.push_str("\r\n{}");
        use std::io::{Read, Write};
        let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        s.set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .unwrap();
        s.write_all(req.as_bytes()).unwrap();
        let mut out = String::new();
        let _ = s.read_to_string(&mut out);
        out.lines().next().unwrap_or_default().to_string()
    };
    assert!(call(None).contains("401"));
    assert!(call(Some("not-a-token")).contains("401"));
}

/// The first bytes of an image of each format Claude Code reads.
const PNG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
const JPEG: [u8; 4] = [0xff, 0xd8, 0xff, 0xe0];
const GIF: &[u8] = b"GIF89a";
const WEBP: &[u8] = b"RIFF\x24\0\0\0WEBPVP8 ";

fn b64(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[tokio::test(flavor = "multi_thread")]
async fn an_image_pasted_in_the_interface_is_written_to_a_file_whose_path_is_typed_there() {
    let h = harness("tui-paste");
    let id = agent_in_tui(&h).await;
    let pastes = h.dir.join("data").join("tui").join("pastes");
    // Claude Code keeps the image in its transcript and resumes without the file: it serves the
    // paste, then the model, told its path, in the turns that follow. Kept a day: older ones go as
    // the next one comes.
    std::fs::create_dir_all(&pastes).unwrap();
    let aged = |name: &str, hours: u64| {
        let f = pastes.join(name);
        std::fs::write(&f, b"old").unwrap();
        std::fs::File::options()
            .write(true)
            .open(&f)
            .unwrap()
            .set_modified(
                std::time::SystemTime::now() - std::time::Duration::from_secs(hours * 3600),
            )
            .unwrap();
        f
    };
    let (old, recent) = (aged("old.png", 25), aged("recent.png", 23));

    let path = h.core.paste_image(&id, "image/png", &b64(&PNG)).unwrap();
    let file = std::path::Path::new(&path);
    assert_eq!(file.parent(), Some(pastes.as_path()));
    assert_eq!(file.extension().unwrap(), "png");
    assert_eq!(std::fs::read(file).unwrap(), PNG);
    assert!(!old.exists());
    assert!(recent.exists());

    // Each paste its own file, named after what it holds (a file copied under a wrong name).
    for (head, extension) in [(&JPEG[..], "jpg"), (GIF, "gif"), (WEBP, "webp")] {
        let other = h.core.paste_image(&id, "image/png", &b64(head)).unwrap();
        assert!(other.ends_with(&format!(".{extension}")), "{other}");
        assert_ne!(other, path);
    }
    let written = std::fs::read_dir(&pastes).unwrap().count();

    // Refused, and nothing written: a type Claude Code does not read, content that is no image of
    // those (whatever its type says), more than 18 MB (all the files of a message in the composer).
    for refused in ["application/pdf", "image/svg+xml"] {
        assert!(
            h.core.paste_image(&id, refused, &b64(&PNG)).is_err(),
            "{refused}"
        );
    }
    let e = h
        .core
        .paste_image(&id, "image/png", &b64(b"%PDF-1.7"))
        .unwrap_err();
    assert!(e.to_string().contains("PNG, JPEG, GIF"), "{e}");
    assert!(h
        .core
        .paste_image(&id, "image/png", "pas du base64")
        .is_err());
    let mut big = PNG.to_vec();
    big.resize((18 << 20) + 1, 0);
    let e = h
        .core
        .paste_image(&id, "image/png", &b64(&big))
        .unwrap_err();
    assert!(e.to_string().contains("18 Mo"), "{e}");
    assert_eq!(std::fs::read_dir(&pastes).unwrap().count(), written);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_image_is_pasted_only_where_claude_codes_interface_runs() {
    use base64::Engine;
    let h = harness("tui-paste-none");
    let (p, _) = h.project(false).await;
    let id = agent_on(&h, &p, "principal").await.id;
    let data = base64::engine::general_purpose::STANDARD.encode([1u8]);
    let e = h.core.paste_image(&id, "image/png", &data).unwrap_err();
    assert!(e.to_string().contains("vue Claude Code"), "{e}");
    // In the view, its interface stopped (« Claude Code s’est arrêté », « Relancer »).
    h.core.set_agent_view(&id, true).await.unwrap();
    let e = h.core.paste_image(&id, "image/png", &data).unwrap_err();
    assert!(e.to_string().contains("relance"), "{e}");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_images_pasted_go_with_their_agent_and_old_ones_at_the_next_start() {
    let h = harness("tui-paste-gone");
    let id = agent_in_tui(&h).await;
    let mine = h.core.paste_image(&id, "image/png", &b64(&PNG)).unwrap();
    let pastes = h.dir.join("data").join("tui").join("pastes");
    let other = pastes.join("autre-4f2a.png");
    std::fs::write(&other, b"png").unwrap();
    let old = pastes.join("autre-ancienne.png");
    std::fs::write(&old, b"png").unwrap();
    std::fs::File::options()
        .write(true)
        .open(&old)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(25 * 3600))
        .unwrap();

    h.core.delete_agent(&id, false).await.unwrap();
    assert!(!std::path::Path::new(&mine).exists());
    assert!(other.exists());

    // Nobody pastes any more: the next start clears what is older than a day.
    let app = tauri::test::mock_app();
    let (_reloaded, _rx) = crate::core::Core::load(app.handle().clone(), h.core.data.clone());
    assert!(!old.exists());
    assert!(other.exists());
}

/// Starts the agent's interface as the window does, on a screen of its own.
async fn open_tui(
    h: &Harness,
    id: &str,
    manual: bool,
) -> anyhow::Result<(crate::pty::TermInfo, Arc<parking_lot::Mutex<String>>)> {
    let screen = Arc::new(parking_lot::Mutex::new(String::new()));
    let sink = screen.clone();
    let info = h
        .core
        .open_agent_terminal(id, (100, 30), manual, move |b| {
            sink.lock().push_str(&String::from_utf8_lossy(&b))
        })
        .await?;
    Ok((info, screen))
}

/// Waits until `done` is true of what the interface drew (its text, with no escape sequence) and of
/// the harness. The pseudo-console (Windows) asks where the cursor is before it runs anything:
/// answered, as xterm.js does in the window.
async fn until(
    h: &Harness,
    term: &str,
    screen: &Arc<parking_lot::Mutex<String>>,
    what: &str,
    done: impl Fn(&Harness, &str) -> bool,
) {
    let start = std::time::Instant::now();
    let mut answered = false;
    loop {
        if done(h, &plain_text(&screen.lock())) {
            return;
        }
        if !answered && screen.lock().contains("\x1b[6n") {
            let _ = h.core.pty.write(term, b"\x1b[1;1R");
            answered = true;
        }
        assert!(
            start.elapsed() < std::time::Duration::from_secs(60),
            "{what} not in {:?}",
            screen.lock()
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

/// The interface is up: it drew its first screen.
async fn alive(h: &Harness, term: &str, screen: &Arc<parking_lot::Mutex<String>>) {
    until(
        h,
        term,
        screen,
        "the interface's first screen",
        |_, text| text.contains("fake-claude-tui"),
    )
    .await;
}

/// The interface is over, and the agent knows how it ended.
async fn over(
    h: &Harness,
    id: &str,
    term: &str,
    screen: &Arc<parking_lot::Mutex<String>>,
) -> TuiExit {
    until(h, term, screen, "the interface's end", |h, _| {
        let agent = h.core.agent(id).unwrap();
        let rt = agent.lock();
        rt.tui.is_none() && rt.tui_exit.is_some()
    })
    .await;
    let exit = h.core.agent(id).unwrap().lock().tui_exit.clone();
    exit.unwrap()
}

/// The notices of the agent's conversation at `level`.
fn notices(h: &Harness, id: &str, level: &str) -> Vec<String> {
    h.items(id)
        .iter()
        .filter(|i| i["kind"] == "notice" && i["level"] == level)
        .map(|i| i["text"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// The arguments of each interface the fake `claude` launched in the agent's folder (not its streams).
fn agent_interface_launches(h: &Harness, id: &str) -> Vec<Vec<String>> {
    interface_launches(h, std::path::Path::new(&h.agent(id).cwd))
}

fn session_args(launches: &[Vec<String>], arg: &str) -> bool {
    launches.iter().any(|l| l.iter().any(|a| a == arg))
}

/// An agent of the account "pro", in the Claude Code view and its interface not started, which
/// has `meta` changed by `f`.
async fn agent_to_open(h: &Harness, p: &Project, f: impl FnOnce(&mut AgentMeta)) -> String {
    let id = agent_on(h, p, "pro").await.id;
    h.core.set_agent_view(&id, true).await.unwrap();
    f(&mut h.core.agent(&id).unwrap().lock().meta);
    id
}

#[tokio::test(flavor = "multi_thread")]
async fn a_session_with_no_transcript_is_given_up_and_the_interface_starts_a_new_one() {
    // An agent that never sent a message has a session id (set at its first start) and no
    // transcript: Claude Code quits on `--resume` of it, and the window looped (ESC-106).
    let h = harness("tui-session-missing");
    let (p, _) = h.project(false).await;
    second_account(&h);
    struct Case {
        session: Option<&'static str>,
        /// The session of the original, for a copy that has none of its own yet.
        original: Option<&'static str>,
        /// How many messages it sent.
        prompts: u32,
        /// The notice its conversation is told, if any.
        told: Option<fn() -> String>,
    }
    let cases = [
        Case {
            session: Some("aaaaaaaa-0000-4000-8000-000000000001"),
            original: None,
            prompts: 0,
            told: None,
        },
        Case {
            session: Some("aaaaaaaa-0000-4000-8000-000000000002"),
            original: None,
            prompts: 3,
            told: Some(crate::tui::session_lost_notice),
        },
        // A copy always has a conversation to lose: its original's.
        Case {
            session: None,
            original: Some("aaaaaaaa-0000-4000-8000-000000000003"),
            prompts: 0,
            told: Some(crate::tui::original_lost_notice),
        },
    ];
    for Case {
        session,
        original,
        prompts,
        told,
    } in cases
    {
        let id = agent_to_open(&h, &p, |m| {
            m.session_id = session.map(str::to_string);
            m.fork_of = original.map(str::to_string);
            m.fork_at = original.map(|_| "e1".to_string());
            m.prompts = prompts;
        })
        .await;
        let (info, screen) = open_tui(&h, &id, false).await.unwrap();
        alive(&h, &info.id, &screen).await;
        let a = h.agent(&id);
        let new = a.session_id.clone().expect("a session of its own");
        let old = session.or(original).unwrap();
        assert_ne!(new, old);
        assert_eq!((a.fork_of, a.fork_at, a.last_entry), (None, None, None));
        let launches = agent_interface_launches(&h, &id);
        assert!(session_args(&launches, &format!("--session-id={new}")));
        for arg in [format!("--resume={old}"), "--fork-session".to_string()] {
            assert!(!session_args(&launches, &arg), "{arg} in {launches:?}");
        }
        // Nothing is said of a session there was nothing to lose of.
        assert_eq!(
            notices(&h, &id, "warn"),
            told.map(|text| text()).into_iter().collect::<Vec<_>>()
        );
        assert!(notices(&h, &id, "info").is_empty());
        assert_eq!(h.agent(&id).session_id.as_deref(), Some(new.as_str()));
        assert!(h.core.agent(&id).unwrap().lock().tui_exit.is_none());
    }
}

/// The arguments of each stream process the fake `claude` launched in the agent's folder.
fn agent_stream_launches(h: &Harness, id: &str) -> Vec<Vec<String>> {
    h.launches(std::path::Path::new(&h.agent(id).cwd))
        .into_iter()
        .filter(|a| a.iter().any(|x| x == "stream-json"))
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_lost_session_is_told_once_though_the_stream_and_the_interface_start_again_after() {
    // The session is lost: the interface starts a new one, which it names and has not written yet
    // (no message sent in it). The stream warmed up on the way back to Escouade's view, and the
    // interface started again, find no transcript of it either: nothing of it is lost, nothing
    // more is said (ESC-113: « une nouvelle session sera démarrée au prochain message » came too).
    let h = harness("tui-session-lost-once");
    let (p, _) = h.project(false).await;
    second_account(&h);
    let id = agent_to_open(&h, &p, |m| {
        m.session_id = Some("cccccccc-0000-4000-8000-000000000001".into());
        m.prompts = 3;
    })
    .await;
    let (info, screen) = open_tui(&h, &id, false).await.unwrap();
    alive(&h, &info.id, &screen).await;
    let named = h.agent(&id).session_id.unwrap();
    let told = vec![crate::tui::session_lost_notice()];
    assert_eq!(notices(&h, &id, "warn"), told);

    // Back to Escouade's view: the stream is warmed up at once, as the interface quits.
    let before = agent_stream_launches(&h, &id).len();
    h.core.set_agent_view(&id, false).await.unwrap();
    h.wait("the stream started", |h| {
        agent_stream_launches(h, &id).len() > before
            && h.core.agent(&id).unwrap().lock().proc.is_some()
    })
    .await;
    let streams = agent_stream_launches(&h, &id)[before..].to_vec();
    assert!(
        !session_args(&streams, &format!("--resume={named}")),
        "{streams:?}"
    );
    assert_eq!(notices(&h, &id, "warn"), told);

    // And into the Claude Code view again: the interface starts on the stream's session.
    h.core.set_agent_view(&id, true).await.unwrap();
    let (info, screen) = open_tui(&h, &id, false).await.unwrap();
    alive(&h, &info.id, &screen).await;
    assert_eq!(notices(&h, &id, "warn"), told);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interface_started_again_in_the_session_it_named_and_never_wrote_says_nothing() {
    let h = harness("tui-session-unwritten");
    let (p, _) = h.project(false).await;
    second_account(&h);
    let id = agent_to_open(&h, &p, |m| m.prompts = 3).await;
    let (info, screen) = open_tui(&h, &id, false).await.unwrap();
    alive(&h, &info.id, &screen).await;
    let named = h.agent(&id).session_id.unwrap();
    // Closed with no message sent in it, then opened again.
    h.core.kill_terminal(&info.id);
    h.wait("its interface over", |h| !interface_runs(h, &id))
        .await;
    let (info, screen) = open_tui(&h, &id, true).await.unwrap();
    alive(&h, &info.id, &screen).await;
    assert_ne!(h.agent(&id).session_id.unwrap(), named);
    assert!(notices(&h, &id, "warn").is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_session_left_in_another_accounts_folder_is_copied_and_resumed() {
    let h = harness("tui-session-other-account");
    let (p, _) = h.project(false).await;
    second_account(&h);
    let third = Account {
        id: "third".into(),
        name: "Tiers".into(),
        config_dir: h.dir.join("claude-third").to_string_lossy().into(),
        ..Default::default()
    };
    let mut s = h.core.settings.read().clone();
    s.accounts.push(third.clone());
    h.core.save_settings(s).unwrap();
    let session = "bbbbbbbb-0000-4000-8000-000000000001";
    let left = h
        .dir
        .join("claude-third")
        .join("projects")
        .join("-work-demo");
    std::fs::create_dir_all(&left).unwrap();
    std::fs::write(
        left.join(format!("{session}.jsonl")),
        "{\"type\":\"summary\"}\n",
    )
    .unwrap();

    let id = agent_to_open(&h, &p, |m| {
        m.session_id = Some(session.into());
        m.prompts = 2;
    })
    .await;
    let (info, screen) = open_tui(&h, &id, false).await.unwrap();
    alive(&h, &info.id, &screen).await;

    // Copied into the agent's account, and resumed: the same session, told.
    let copied = h
        .dir
        .join("claude-pro")
        .join("projects")
        .join("-work-demo")
        .join(format!("{session}.jsonl"));
    assert_eq!(
        std::fs::read_to_string(&copied).unwrap(),
        "{\"type\":\"summary\"}\n"
    );
    assert!(left.join(format!("{session}.jsonl")).exists());
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(session));
    assert!(session_args(
        &agent_interface_launches(&h, &id),
        &format!("--resume={session}")
    ));
    assert!(notices(&h, &id, "warn").is_empty());
    let told = notices(&h, &id, "info");
    assert_eq!(told.len(), 1, "{told:?}");
    assert!(
        told[0].contains("Tiers") && told[0].contains("Pro"),
        "{told:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_session_that_cannot_be_copied_from_another_account_is_given_up_with_its_reason() {
    let h = harness("tui-session-not-copied");
    let (p, _) = h.project(false).await;
    second_account(&h);
    let third = Account {
        id: "third".into(),
        name: "Tiers".into(),
        config_dir: h.dir.join("claude-third").to_string_lossy().into(),
        ..Default::default()
    };
    let mut s = h.core.settings.read().clone();
    s.accounts.push(third);
    h.core.save_settings(s).unwrap();
    let session = "bbbbbbbb-0000-4000-8000-000000000002";
    let left = h
        .dir
        .join("claude-third")
        .join("projects")
        .join("-work-demo");
    std::fs::create_dir_all(&left).unwrap();
    std::fs::write(left.join(format!("{session}.jsonl")), "{}\n").unwrap();
    // A file where the copy needs a folder: it cannot be written.
    std::fs::create_dir_all(h.dir.join("claude-pro")).unwrap();
    std::fs::write(h.dir.join("claude-pro").join("projects"), "in the way").unwrap();

    let id = agent_to_open(&h, &p, |m| {
        m.session_id = Some(session.into());
        m.prompts = 2;
    })
    .await;
    let (info, screen) = open_tui(&h, &id, false).await.unwrap();
    alive(&h, &info.id, &screen).await;

    let new = h.agent(&id).session_id.expect("a session of its own");
    assert_ne!(new, session);
    let warns = notices(&h, &id, "warn");
    assert_eq!(warns.len(), 1, "{warns:?}");
    assert!(
        warns[0].contains("Tiers") && warns[0].contains("Pro"),
        "{warns:?}"
    );
    assert!(!session_args(
        &agent_interface_launches(&h, &id),
        &format!("--resume={session}")
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interface_that_quits_on_a_session_it_does_not_find_gives_it_up_and_the_next_start_is_new(
) {
    // The check before the start found the transcript, and Claude Code does not see it (a
    // transcript it cannot read, another way of naming the folder): the interface says so and
    // quits at once. The agent reads that from its output.
    let h = harness("tui-session-refused");
    let id = agent_in_tui(&h).await;
    say_in_tui(&h, &id, "bonjour").await;
    let session = h.agent(&id).session_id.unwrap();
    let prompts = h.agent(&id).prompts;
    let term = |h: &Harness| h.core.agent(&id).unwrap().lock().view().terminal_id;

    // Left by its user (/exit, code 0) a few seconds after it started: no quick exit.
    h.core.pty.write(&term(&h).unwrap(), b"/exit\r").unwrap();
    h.wait("the interface's end", |h| {
        h.core.agent(&id).unwrap().lock().tui_exit.is_some()
    })
    .await;
    let exit = h.core.agent(&id).unwrap().lock().view().tui_exit.unwrap();
    assert_eq!(
        exit,
        TuiExit {
            code: Some(0),
            quick: false,
            session_lost: false
        }
    );
    assert!(term(&h).is_none());
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(session.as_str()));

    // Started again, the fake refuses every `--resume` (the marker), the session being there.
    std::fs::write(h.dir.join("claude-pro").join("fake-no-conversation"), "").unwrap();
    let (info, screen) = open_tui(&h, &id, false).await.unwrap();
    let exit = over(&h, &id, &info.id, &screen).await;
    assert_eq!(
        exit,
        TuiExit {
            code: Some(1),
            quick: true,
            session_lost: true
        }
    );
    assert!(session_args(
        &agent_interface_launches(&h, &id),
        &format!("--resume={session}")
    ));
    let a = h.agent(&id);
    assert_eq!((a.session_id, a.fork_of, a.last_entry), (None, None, None));
    assert_eq!(a.prompts, prompts, "its conversation stays");
    assert_eq!(
        notices(&h, &id, "warn"),
        [crate::tui::session_lost_notice()]
    );
    // The window reads it from the agent's view.
    let view = h.core.agent(&id).unwrap().lock().view();
    assert_eq!(view.tui_exit, Some(exit));
    assert!(view.terminal_id.is_none());

    // Started again (once, as the window does): a new session, which lives.
    let (info, screen) = open_tui(&h, &id, false).await.unwrap();
    alive(&h, &info.id, &screen).await;
    let new = h.agent(&id).session_id.expect("a session of its own");
    assert_ne!(new, session);
    assert!(session_args(
        &agent_interface_launches(&h, &id),
        &format!("--session-id={new}")
    ));
    assert!(h.core.agent(&id).unwrap().lock().view().tui_exit.is_none());
    assert_eq!(notices(&h, &id, "warn").len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interface_that_crashes_as_it_starts_is_not_started_again_by_itself_more_than_once() {
    let h = harness("tui-crash-loop");
    let (p, _) = h.project(false).await;
    second_account(&h);
    let id = agent_to_open(&h, &p, |_| {}).await;
    std::fs::create_dir_all(h.dir.join("claude-pro")).unwrap();
    let crash = h.dir.join("claude-pro").join("fake-tui-crash");
    std::fs::write(&crash, "").unwrap();
    let quick_exits = || h.core.agent(&id).unwrap().lock().tui_quick_exits;

    // The window opens the view, and starts the interface (automatic); it stops as soon as it started.
    for n in 1..=2 {
        let (info, screen) = open_tui(&h, &id, false).await.unwrap();
        let exit = over(&h, &id, &info.id, &screen).await;
        assert_eq!(
            exit,
            TuiExit {
                code: Some(1),
                quick: true,
                session_lost: false
            }
        );
        assert_eq!(quick_exits(), n);
    }
    // The third is refused, and says why; nothing is started.
    let launched = agent_interface_launches(&h, &id).len();
    let refused = open_tui(&h, &id, false).await.unwrap_err().to_string();
    assert!(refused.contains('2'), "{refused}");
    assert!(
        refused.contains("Relancer") || refused.contains("Restart"),
        "{refused}"
    );
    assert_eq!(agent_interface_launches(&h, &id).len(), launched);
    assert!(h.core.agent(&id).unwrap().lock().tui.is_none());

    // « Relancer » goes through, and begins the count again.
    let (info, screen) = open_tui(&h, &id, true).await.unwrap();
    over(&h, &id, &info.id, &screen).await;
    assert_eq!(quick_exits(), 1);
    // Fixed since: the window's start goes, and the interface lives.
    std::fs::remove_file(&crash).unwrap();
    let (info, screen) = open_tui(&h, &id, false).await.unwrap();
    alive(&h, &info.id, &screen).await;
    // Switching to the view starts the count over too.
    h.core.pty.kill(&info.id);
    h.wait("the interface gone", |h| {
        h.core.agent(&id).unwrap().lock().tui.is_none()
    })
    .await;
    h.core.set_agent_view(&id, false).await.unwrap();
    h.core.set_agent_view(&id, true).await.unwrap();
    assert_eq!(quick_exits(), 0);
}

/// The interface launches in `cwd` stay `launched` for a while: nothing starts another one, and
/// the agent has none running, its quick exits in a row still `exits`.
async fn nothing_starts_it_again(
    h: &Harness,
    id: &str,
    cwd: &std::path::Path,
    (launched, exits): (usize, u32),
    way: &str,
) {
    // Time for a start to launch the fake `claude` and for it to write its log.
    tokio::time::sleep(std::time::Duration::from_millis(900)).await;
    let launches = interface_launches(h, cwd);
    assert_eq!(
        launches.len(),
        launched,
        "started again by {way}: {launches:?}"
    );
    assert!(!interface_runs(h, id), "{way} left an interface running");
    assert_eq!(
        h.core.agent(id).unwrap().lock().tui_quick_exits,
        exits,
        "{way} changed the count of quick exits"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interface_that_quits_at_once_is_started_again_by_itself_once_whatever_the_way() {
    // Remote Control makes the first start (no window shows it); the interface quits as soon as it
    // started, and so does the one started again without the option (the fallback). From then on
    // nothing starts it by itself, by whichever way, but « Relancer » (ESC-93 and ESC-106).
    let h = harness("tui-quick-exits-every-way");
    let (p, r) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = agent_on(&h, &p, "pro").await.id;
    std::fs::create_dir_all(h.dir.join("claude-pro")).unwrap();
    std::fs::write(h.dir.join("claude-pro").join("fake-tui-crash"), "").unwrap();
    let quick_exits = |h: &Harness| h.core.agent(&id).unwrap().lock().tui_quick_exits;

    // The setting starts it with Remote Control, then the fallback starts it without: two starts.
    h.remote_all(true);
    h.wait("the two starts over", |h| {
        interface_launches(h, &r).len() >= 2 && quick_exits(h) == 2 && !interface_runs(h, &id)
    })
    .await;
    let launches = interface_launches(&h, &r);
    assert_eq!(launches.len(), 2, "{launches:?}");
    assert!(asks_remote_control(&launches[0]), "{:?}", launches[0]);
    assert!(!asks_remote_control(&launches[1]), "{:?}", launches[1]);
    assert_eq!(unavailable(&h, &id).len(), 1);
    assert!(h.agent(&id).remote_control);
    let stays = (2, 2);

    // The start of the interface in the background.
    h.core.start_interface(&id).await.unwrap();
    nothing_starts_it_again(&h, &id, &r, stays, "start_interface").await;
    // The warm-up.
    h.core.warm(&id);
    nothing_starts_it_again(&h, &id, &r, stays, "the warm-up").await;
    // The agents of the last run, started with the app.
    h.core.start_remote_agents();
    nothing_starts_it_again(&h, &id, &r, stays, "the start with the app").await;
    // The setting turned off, then on again: it applies to the agent as to one that has none.
    h.remote_all(false);
    h.wait("the setting cut", |h| !h.agent(&id).remote_control)
        .await;
    h.remote_all(true);
    h.settle_remote().await;
    nothing_starts_it_again(&h, &id, &r, stays, "the setting").await;
    assert!(h.agent(&id).remote_control);
    // The window's own start (it opened the view) is refused, and says why.
    let refused = open_tui(&h, &id, false).await.unwrap_err().to_string();
    assert!(refused.contains('2'), "{refused}");
    assert!(
        refused.contains("Relancer") || refused.contains("Restart"),
        "{refused}"
    );
    nothing_starts_it_again(&h, &id, &r, stays, "the window").await;
    // None of this was told as a failure again.
    assert_eq!(unavailable(&h, &id).len(), 1);

    // « Relancer » goes through (one more start, which quits as soon as it started too, and begins
    // the count again); nothing starts it again after it either.
    let (info, screen) = open_tui(&h, &id, true).await.unwrap();
    over(&h, &id, &info.id, &screen).await;
    nothing_starts_it_again(&h, &id, &r, (3, 1), "« Relancer »").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn removing_the_project_stops_its_agents_interfaces_and_clears_their_pasted_images() {
    let h = harness("tui-paste-project");
    let id = agent_in_tui(&h).await;
    let project = h.agent(&id).project_id;
    let cwd = std::path::PathBuf::from(h.agent(&id).cwd);
    let mine = h.core.paste_image(&id, "image/png", &b64(&PNG)).unwrap();
    assert!(h.core.pty.list().iter().any(|t| t.shell == "claude"));

    h.core.remove_project(&project, false).await.unwrap();
    assert!(!std::path::Path::new(&mine).exists());
    assert!(
        !h.core.pty.list().iter().any(|t| t.shell == "claude"),
        "Claude Code's interface still runs"
    );
    // It quit as its user would.
    assert_eq!(ctrl_c(&h, &cwd).len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn turning_remote_control_off_keeps_the_link_of_an_interface_a_window_showed_until_its_next_launch(
) {
    let h = harness("tui-rc-off-keeps-link");
    let (p, _) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = agent_on(&h, &p, "pro").await.id;
    let state = |h: &Harness| h.core.agent(&id).unwrap().lock().view().remote_state;
    h.core.set_remote_control(&id, true).await.unwrap();
    h.wait("the interface linked", |h| {
        state(h).as_deref() == Some("connected")
    })
    .await;
    show(&h, &id).await;

    // Off for Escouade, but the interface a window showed runs on with its link: that is what the
    // state tells, until it starts again.
    assert!(h.core.set_remote_control(&id, false).await.unwrap());
    assert!(!h.agent(&id).remote_control);
    assert!(interface_runs(&h, &id));
    assert_eq!(state(&h).as_deref(), Some("connected"));
    h.core.kill_terminal(&terminal_of(&h, &id));
    h.wait("the interface over", |h| !interface_runs(h, &id))
        .await;
    assert_eq!(state(&h), None);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_setting_starts_three_interfaces_at_most_until_they_are_up() {
    let h = harness("tui-remote-throttle");
    let (p, r) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let mut ids = Vec::new();
    for _ in 0..4 {
        ids.push(agent_on(&h, &p, "pro").await.id);
    }
    // None gets past its welcome screen: each keeps the place it started in.
    h.fake_flag(&r, "tui-welcome");
    h.remote_all(true);
    h.wait("three interfaces", |h| interface_launches(h, &r).len() >= 3)
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    assert_eq!(interface_launches(&h, &r).len(), 3);
    assert_eq!(
        ids.iter().filter(|id| interface_runs(&h, id)).count(),
        3,
        "the fourth waits for a place"
    );
    // One is up: its place goes to the next.
    h.fake_unflag(&r, "tui-welcome");
    h.wait("the fourth", |h| interface_launches(h, &r).len() == 4)
        .await;
    for id in &ids {
        ready(&h, id).await;
    }
    h.settle_remote().await;
    assert_eq!(interface_launches(&h, &r).len(), 4);
}

/// The launches of the stream's process (not of an interface) in `cwd`.
fn stream_launches(h: &Harness, cwd: &std::path::Path) -> usize {
    h.launches(cwd)
        .iter()
        .filter(|a| a.iter().any(|x| x == "stream-json"))
        .count()
}

#[tokio::test(flavor = "multi_thread")]
async fn quitting_an_interface_does_not_hold_the_launch_lock_and_no_process_starts_before_it_is_gone(
) {
    let h = harness("tui-quit-lock");
    let (p, r) = h.project(false).await;
    second_account(&h);
    h.core.settings.write().terminal_view = true;
    let id = agent_on(&h, &p, "pro").await.id;
    // It holds out against the two Ctrl+C: it is killed five seconds after the second.
    h.fake_flag(&r, "tui-ignore-quit");
    open_interface(&h, &id).await;
    up(&h, &id).await;
    let term = terminal_of(&h, &id);
    let streams = stream_launches(&h, &r);

    let (core, agent) = (h.core.clone(), id.clone());
    let back = tokio::spawn(async move { core.set_agent_view(&agent, false).await });
    h.wait("both Ctrl+C sent", |h| ctrl_c(h, &r).len() == 2)
        .await;
    // It quits (for five seconds still): the lock that serializes the launches is free.
    assert!(h.core.pty.exists(&term));
    assert!(h.core.spawn_lock(&id).try_lock().is_ok());
    // A start asked meanwhile waits for the interface to be gone: never two processes on one
    // session.
    let (core, agent) = (h.core.clone(), id.clone());
    let started = tokio::spawn(async move { core.ensure_process(&agent).await.map(|_| ()) });
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    assert!(h.core.pty.exists(&term));
    assert_eq!(stream_launches(&h, &r), streams);
    back.await.unwrap().unwrap();
    started.await.unwrap().unwrap();
    assert!(!h.core.pty.exists(&term));
    h.wait("the stream's process", |h| stream_launches(h, &r) > streams)
        .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_place_among_the_starts_is_kept_until_the_interface_is_up_but_for_a_while_at_most() {
    let (h, id, r) = background_interface_on_its_welcome_screen("tui-place-kept").await;
    let agent = h.core.agent(&id).unwrap();
    // Not up: its place is kept for as long as it is given, no less.
    let waited = std::time::Instant::now();
    h.core
        .wait_interface_ready(&agent, std::time::Duration::from_millis(700))
        .await;
    assert!(waited.elapsed() >= std::time::Duration::from_millis(700));
    assert!(!agent.lock().interface_ready());
    // Up: given back as soon as it is, whatever it was given.
    h.fake_unflag(&r, "tui-welcome");
    tokio::time::timeout(
        std::time::Duration::from_secs(60),
        h.core
            .wait_interface_ready(&agent, std::time::Duration::from_secs(3600)),
    )
    .await
    .expect("given back once the interface is up");
    assert!(agent.lock().interface_ready());
    // Gone: nothing to wait for.
    h.core.kill_terminal(&terminal_of(&h, &id));
    h.wait("the interface over", |h| !interface_runs(h, &id))
        .await;
    tokio::time::timeout(
        std::time::Duration::from_secs(60),
        h.core
            .wait_interface_ready(&agent, std::time::Duration::from_secs(3600)),
    )
    .await
    .expect("given back once the interface is gone");
}

/// What Claude Code 2.1.300 wrote of a real subagent (`tests/fixtures`), and its call and task.
const SIDECHAIN: &str = include_str!("../../tests/fixtures/real-sidechain.jsonl");
const SIDECHAIN_META: &str = include_str!("../../tests/fixtures/real-sidechain.meta.json");
const SUB_CALL: &str = "toolu_01SdqbgNzZvBJ1FgjQC3Gg6L";
const SUB_TASK: &str = "a3c004cb08b483eb9";

/// The folder where Claude Code keeps a session's subagents' transcripts, next to its own, written
/// in the account `config_dir` (`projects/<folder>/<session>/subagents`); the transcript itself empty.
fn subagents_folder(config_dir: &str, session: &str) -> std::path::PathBuf {
    let folder = std::path::Path::new(config_dir)
        .join("projects")
        .join("-work-demo");
    let subs = folder.join(session).join("subagents");
    std::fs::create_dir_all(&subs).unwrap();
    std::fs::write(folder.join(format!("{session}.jsonl")), "").unwrap();
    subs
}

#[tokio::test(flavor = "multi_thread")]
async fn the_conversation_of_a_subagent_is_read_from_its_own_transcript_in_the_claude_code_view() {
    let h = harness("tui-sidechain");
    let (p, _) = h.project(false).await;
    let pro = second_account(&h);
    let id = agent_on(&h, &p, "pro").await.id;
    // The stream view has the thread in its conversation: nothing to read there.
    assert!(h.core.subagent_transcript(&id, SUB_CALL, None).is_err());
    h.core.set_agent_view(&id, true).await.unwrap();
    let session = "55555555-2222-3333-4444-555555555555";
    h.core.agent(&id).unwrap().lock().meta.session_id = Some(session.into());

    // No file yet: nothing, not an error (and no cursor: the next read looks for it again).
    let none = h.core.subagent_transcript(&id, SUB_CALL, None);
    let none = none.unwrap().unwrap();
    assert!(none.items.is_empty() && none.cursor.is_none() && none.ended.is_none());

    // Found by the meta file that names its call, in the account's folder.
    let subs = subagents_folder(&pro.config_dir, session);
    std::fs::write(subs.join(format!("agent-{SUB_TASK}.jsonl")), SIDECHAIN).unwrap();
    std::fs::write(
        subs.join(format!("agent-{SUB_TASK}.meta.json")),
        SIDECHAIN_META,
    )
    .unwrap();
    let items = thread(&h, &id, SUB_CALL);
    assert_eq!(items.len(), 4);
    assert!(items.iter().all(|i| i["parent"] == SUB_CALL));
    assert_eq!(items[0]["name"], "Write");
    assert_eq!(items[0]["status"], "ok");
    assert_eq!(thread(&h, &id, "toolu_nobody").len(), 0);
    assert!(h
        .core
        .subagent_transcript("nobody", SUB_CALL, None)
        .is_err());

    // Found by its task, which the row of the plan tells, without a meta file. (The path found
    // before is kept: forgotten here, for it to be looked for again.)
    std::fs::remove_file(subs.join(format!("agent-{SUB_TASK}.meta.json"))).unwrap();
    h.core.forget_sidechains(&id);
    assert!(thread(&h, &id, SUB_CALL).is_empty());
    h.core.agent(&id).unwrap().lock().meta.plan = Some(crate::plan::PlanState {
        agents: vec![crate::plan::SubAgent {
            id: SUB_CALL.into(),
            task_id: Some(SUB_TASK.into()),
            ..Default::default()
        }],
        ..Default::default()
    });
    assert_eq!(thread(&h, &id, SUB_CALL).len(), 4);

    // The transcript the interface is read from is the one it looks beside, wherever it is.
    let elsewhere = h.dir.join("elsewhere");
    let there = elsewhere.join("s9").join("subagents");
    std::fs::create_dir_all(&there).unwrap();
    std::fs::write(elsewhere.join("s9.jsonl"), "").unwrap();
    std::fs::write(
        there.join(format!("agent-{SUB_TASK}.jsonl")),
        SIDECHAIN.lines().take(4).collect::<Vec<_>>().join("\n"),
    )
    .unwrap();
    h.core.agent(&id).unwrap().lock().meta.transcript_read =
        Some((elsewhere.join("s9.jsonl").to_string_lossy().into(), 0));
    h.core.forget_sidechains(&id);
    assert_eq!(thread(&h, &id, SUB_CALL).len(), 1);
}

/// What a first read (no cursor) of the thread of `call` gives: it always gives something.
fn thread(h: &Harness, id: &str, call: &str) -> Vec<serde_json::Value> {
    h.core
        .subagent_transcript(id, call, None)
        .unwrap()
        .expect("a first read gives the thread")
        .items
}

/// An agent in the Claude Code view whose transcript is `<h.dir>/<name>/s9.jsonl`, and the folder
/// of its subagents' beside it.
async fn agent_with_subagents(h: &Harness, name: &str) -> (String, std::path::PathBuf) {
    let (p, _) = h.project(false).await;
    second_account(h);
    let id = agent_on(h, &p, "pro").await.id;
    h.core.set_agent_view(&id, true).await.unwrap();
    let root = h.dir.join(name);
    let subs = root.join("s9").join("subagents");
    std::fs::create_dir_all(&subs).unwrap();
    std::fs::write(root.join("s9.jsonl"), "").unwrap();
    h.core.agent(&id).unwrap().lock().meta.transcript_read =
        Some((root.join("s9.jsonl").to_string_lossy().into(), 0));
    (id, subs)
}

/// The agent's plan has a row for each of `rows` (call, task, background), running, and its
/// conversation the call of each.
fn run_subagents(h: &Harness, id: &str, rows: &[(&str, Option<&str>, bool)]) {
    let handle = h.core.agent(id).unwrap();
    let mut rt = handle.lock();
    for (call, ..) in rows {
        rt.conv.apply(&ConvOp::Append {
            item: serde_json::json!({
                "kind": "tool", "id": call, "name": "Agent", "status": "ok",
                "input": { "description": "Lent", "prompt": "Attends." }, "ts": 1,
            }),
        });
    }
    rt.meta.plan = Some(crate::plan::PlanState {
        agents: rows
            .iter()
            .map(|(call, task, background)| crate::plan::SubAgent {
                id: call.to_string(),
                title: "Lent".into(),
                task_id: task.map(str::to_string),
                background: *background,
                started_at: 1,
                ..Default::default()
            })
            .collect(),
        launched: rows.len() as u32,
        ..Default::default()
    });
}

/// The status of the row of `call` in the agent's plan.
fn row_status(h: &Harness, id: &str, call: &str) -> crate::plan::RunStatus {
    h.agent(id)
        .plan
        .unwrap()
        .agents
        .iter()
        .find(|a| a.id == call)
        .unwrap()
        .status
}

/// How many times the window was sent the agent `id`.
fn agent_events(h: &Harness, id: &str) -> usize {
    h.events
        .lock()
        .iter()
        .filter(|e| e["type"] == "agent" && e["agent"]["id"] == id)
        .count()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_transcript_is_read_again_only_when_it_changed_and_the_path_found_is_kept() {
    let h = harness("tui-sidechain-cursor");
    let (id, subs) = agent_with_subagents(&h, "cursor").await;
    let file = subs.join(format!("agent-{SUB_TASK}.jsonl"));
    let meta = subs.join(format!("agent-{SUB_TASK}.meta.json"));
    let lines: Vec<&str> = SIDECHAIN.lines().collect();
    std::fs::write(&file, lines[..6].join("\n") + "\n").unwrap();
    std::fs::write(&meta, format!(r#"{{"toolUseId":"{SUB_CALL}"}}"#)).unwrap();
    let read = |since| h.core.subagent_transcript(&id, SUB_CALL, since).unwrap();

    // The first read, which finds it by its meta file: all of it, and the cursor to give back.
    let first = read(None).unwrap();
    assert_eq!(first.items.len(), 2);
    let cursor = first.cursor.unwrap();
    assert_eq!(cursor.len, std::fs::metadata(&file).unwrap().len());
    // Nothing changed since: nothing to say.
    assert!(read(first.cursor).is_none());

    // The path is kept: the meta files are not looked at again, whatever becomes of them.
    std::fs::write(&meta, "{}").unwrap();
    assert_eq!(read(None).unwrap().items.len(), 2);
    std::fs::rename(&meta, subs.join("elsewhere.json")).unwrap();
    assert_eq!(read(None).unwrap().items.len(), 2);
    assert_eq!(h.core.sidechains.lock().len(), 1);

    // It grows: the new items, and the cursor of what was read.
    let mut text = std::fs::read_to_string(&file).unwrap();
    text.push_str(&(lines[6..10].join("\n") + "\n"));
    std::fs::write(&file, text).unwrap();
    let grown = read(first.cursor).unwrap();
    let names: Vec<_> = grown
        .items
        .iter()
        .map(|i| i["name"].as_str().unwrap_or("-"))
        .collect();
    assert_eq!(names, ["Write", "Bash", "-", "Bash"]);
    assert!(grown.cursor.unwrap().len > cursor.len);
    assert!(read(grown.cursor).is_none());

    // The file is gone: nothing (and no cursor), until it is found again, where it is now.
    std::fs::remove_file(&file).unwrap();
    let gone = read(grown.cursor).unwrap();
    assert!(gone.items.is_empty() && gone.cursor.is_none());
    assert_eq!(
        h.core.sidechains.lock().len(),
        0,
        "the path it had is forgotten"
    );
    std::fs::write(subs.join("agent-other.jsonl"), first_four()).unwrap();
    std::fs::write(
        subs.join("agent-other.meta.json"),
        format!(r#"{{"toolUseId":"{SUB_CALL}"}}"#),
    )
    .unwrap();
    let moved = read(grown.cursor).unwrap();
    assert_eq!(moved.items.len(), 1);
    assert_eq!(h.core.sidechains.lock().len(), 1);
}

fn first_four() -> String {
    SIDECHAIN.lines().take(4).collect::<Vec<_>>().join("\n") + "\n"
}

/// An agent whose subagent `SUB_CALL` was read once, so that its path is kept.
async fn agent_with_a_path_kept(h: &Harness, name: &str) -> String {
    let (id, subs) = agent_with_subagents(h, name).await;
    std::fs::write(subs.join(format!("agent-{SUB_TASK}.jsonl")), first_four()).unwrap();
    std::fs::write(
        subs.join(format!("agent-{SUB_TASK}.meta.json")),
        SIDECHAIN_META,
    )
    .unwrap();
    assert_eq!(thread(h, &id, SUB_CALL).len(), 1);
    assert!(h.core.sidechains.lock().has(&id, SUB_CALL));
    id
}

#[tokio::test(flavor = "multi_thread")]
async fn the_path_kept_of_a_subagent_goes_with_its_agent() {
    let h = harness("tui-sidechain-path-agent");
    let id = agent_with_a_path_kept(&h, "path-agent").await;
    h.core.delete_agent(&id, false).await.unwrap();
    assert_eq!(h.core.sidechains.lock().len(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_path_kept_of_a_subagent_goes_with_its_project() {
    let h = harness("tui-sidechain-path-project");
    let id = agent_with_a_path_kept(&h, "path-project").await;
    let project = h.agent(&id).project_id;
    h.core.remove_project(&project, false).await.unwrap();
    assert_eq!(h.core.sidechains.lock().len(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_view_of_a_subagent_puts_the_model_of_its_transcript_on_its_row_once() {
    use crate::plan::RunStatus;
    let h = harness("tui-sidechain-model");
    let (id, subs) = agent_with_subagents(&h, "model").await;
    run_subagents(&h, &id, &[(SUB_CALL, Some(SUB_TASK), true)]);
    let file = subs.join(format!("agent-{SUB_TASK}.jsonl"));
    std::fs::write(
        &file,
        SIDECHAIN.lines().take(10).collect::<Vec<_>>().join("\n") + "\n",
    )
    .unwrap();
    std::fs::write(
        subs.join(format!("agent-{SUB_TASK}.meta.json")),
        format!(r#"{{"toolUseId":"{SUB_CALL}"}}"#),
    )
    .unwrap();
    let model = || h.agent(&id).plan.unwrap().agents[0].model.clone();
    // Its call named none: the row has none until its view is opened.
    assert_eq!(model(), None);
    let sent = agent_events(&h, &id);
    let first = h
        .core
        .subagent_transcript(&id, SUB_CALL, None)
        .unwrap()
        .unwrap();
    assert_eq!(first.model.as_deref(), Some("claude-haiku-5-5"));
    assert_eq!(model().as_deref(), Some("claude-haiku-5-5"));
    assert_eq!(row_status(&h, &id, SUB_CALL), RunStatus::Running);
    // The window is sent the agent once.
    assert_eq!(agent_events(&h, &id), sent + 1);
    // Read again (the window asks every 2 s), its model the same: nothing more is sent.
    h.core.subagent_transcript(&id, SUB_CALL, None).unwrap();
    assert_eq!(agent_events(&h, &id), sent + 1);
    // Once it ends (stopped in the interface), its call keeps the model the plan may forget.
    std::fs::write(&file, SIDECHAIN).unwrap();
    h.core.subagent_transcript(&id, SUB_CALL, None).unwrap();
    let call = h
        .items(&id)
        .into_iter()
        .find(|i| i["id"] == SUB_CALL)
        .unwrap();
    assert_eq!(call["run"]["model"], "claude-haiku-5-5");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_subagent_stopped_in_the_interface_ends_its_row_when_its_transcript_is_read() {
    use crate::agent::sidechain::Ended;
    use crate::plan::RunStatus;
    let h = harness("tui-sidechain-ended");
    let (id, subs) = agent_with_subagents(&h, "ended").await;
    run_subagents(&h, &id, &[(SUB_CALL, Some(SUB_TASK), true)]);
    let file = subs.join(format!("agent-{SUB_TASK}.jsonl"));
    // Not over: its meta file does not say, and its transcript ends on a tool's result.
    std::fs::write(
        &file,
        SIDECHAIN.lines().take(10).collect::<Vec<_>>().join("\n") + "\n",
    )
    .unwrap();
    std::fs::write(
        subs.join(format!("agent-{SUB_TASK}.meta.json")),
        format!(r#"{{"toolUseId":"{SUB_CALL}"}}"#),
    )
    .unwrap();
    let running = h
        .core
        .subagent_transcript(&id, SUB_CALL, None)
        .unwrap()
        .unwrap();
    assert_eq!(running.ended, None);
    assert_eq!(row_status(&h, &id, SUB_CALL), RunStatus::Running);
    let sent = agent_events(&h, &id);

    // The user stops it in the interface: the interruption comes in its transcript.
    std::fs::write(&file, SIDECHAIN).unwrap();
    let read = h
        .core
        .subagent_transcript(&id, SUB_CALL, running.cursor)
        .unwrap()
        .unwrap();
    assert_eq!(read.ended, Some(Ended::Stopped));
    let row = h.agent(&id).plan.unwrap().agents[0].clone();
    assert_eq!(row.status, RunStatus::Stopped);
    assert!(row.ended_at.is_some_and(|at| at >= row.started_at));
    // What it was is kept with its call, and the window is sent the agent: the sidebar changes.
    let call = h
        .items(&id)
        .into_iter()
        .find(|i| i["id"] == SUB_CALL)
        .unwrap();
    assert_eq!(call["run"]["status"], "stopped");
    assert_eq!(agent_events(&h, &id), sent + 1);
    assert!(h.events.lock().iter().any(|e| e["type"] == "agent"
        && e["agent"]["id"] == id.as_str()
        && e["agent"]["plan"]["agents"][0]["status"] == "stopped"));
    // Read again: nothing changed, nothing sent.
    assert!(h
        .core
        .subagent_transcript(&id, SUB_CALL, read.cursor)
        .unwrap()
        .is_none());
    assert_eq!(agent_events(&h, &id), sent + 1);
    // And once it ended, however it is read, nothing more is sent.
    h.core
        .subagent_transcript(&id, SUB_CALL, None)
        .unwrap()
        .unwrap();
    assert_eq!(agent_events(&h, &id), sent + 1);

    // One that failed: the API gave up on it.
    let mut failed: serde_json::Value =
        serde_json::from_str(SIDECHAIN.lines().nth(7).unwrap()).unwrap();
    failed["isApiErrorMessage"] = serde_json::json!(true);
    std::fs::write(
        subs.join("agent-late.jsonl"),
        format!("{}\n{failed}\n", first_four().trim_end()),
    )
    .unwrap();
    std::fs::write(
        subs.join("agent-late.meta.json"),
        r#"{"toolUseId":"toolu_late"}"#,
    )
    .unwrap();
    run_subagents(
        &h,
        &id,
        &[(SUB_CALL, Some(SUB_TASK), true), ("toolu_late", None, true)],
    );
    let read = h
        .core
        .subagent_transcript(&id, "toolu_late", None)
        .unwrap()
        .unwrap();
    assert_eq!(read.ended, Some(Ended::Failed));
    assert_eq!(row_status(&h, &id, "toolu_late"), RunStatus::Failed);
    assert_eq!(
        row_status(&h, &id, SUB_CALL),
        RunStatus::Running,
        "not that one"
    );

    // A row that ended otherwise (its hand-back came first) stays as it was; the window is told
    // nothing (its model, which the transcript tells too, it had already).
    let sent = agent_events(&h, &id);
    {
        let a = h.core.agent(&id).unwrap();
        let mut rt = a.lock();
        let row = &mut rt.meta.plan.as_mut().unwrap().agents[0];
        row.status = RunStatus::Done;
        row.model = Some("claude-haiku-5-5".into());
    }
    let read = h
        .core
        .subagent_transcript(&id, SUB_CALL, None)
        .unwrap()
        .unwrap();
    assert_eq!(read.ended, Some(Ended::Stopped));
    assert_eq!(row_status(&h, &id, SUB_CALL), RunStatus::Done);
    assert_eq!(agent_events(&h, &id), sent);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_end_of_a_turn_ends_the_background_subagents_the_user_stopped_in_the_interface() {
    use crate::plan::RunStatus;
    let h = harness("tui-sidechain-stop-hook");
    let id = agent_in_tui(&h).await;
    let session = h.agent(&id).session_id.unwrap();
    let pro = h
        .core
        .settings
        .read()
        .accounts
        .last()
        .unwrap()
        .config_dir
        .clone();
    // The interface writes its transcript with its first message (as Claude Code does); its subagents' are beside it.
    say_in_tui(&h, &id, "bonjour").await;
    h.wait("the transcript", |_| {
        crate::tui::find_transcript(std::path::Path::new(&pro), &session).is_some()
    })
    .await;
    let main = crate::tui::find_transcript(std::path::Path::new(&pro), &session).unwrap();
    let subs = main.parent().unwrap().join(&session).join("subagents");
    std::fs::create_dir_all(&subs).unwrap();
    // A subagent's files: its transcript, and its meta file that says whether the user stopped it.
    let files = |task: &str, stopped: bool| {
        std::fs::write(subs.join(format!("agent-{task}.jsonl")), first_four()).unwrap();
        std::fs::write(
            subs.join(format!("agent-{task}.meta.json")),
            format!(r#"{{"toolUseId":"call-{task}","stoppedByUser":{stopped}}}"#),
        )
        .unwrap();
    };
    files("stopped", true);
    files("running", false);
    files("foreground", true);
    files("over", true);
    // One whose task is not told yet, found by its meta file; one with no file at all.
    files("notask", true);
    run_subagents(
        &h,
        &id,
        &[
            ("call-stopped", Some("stopped"), true),
            ("call-running", Some("running"), true),
            ("call-foreground", Some("foreground"), false),
            ("call-over", Some("over"), true),
            ("call-notask", None, true),
            ("call-nofile", Some("nofile"), true),
        ],
    );
    h.core
        .agent(&id)
        .unwrap()
        .lock()
        .meta
        .plan
        .as_mut()
        .unwrap()
        .agents[3]
        .status = RunStatus::Done;
    let sent = agent_events(&h, &id);

    // Not at any hook: at the end of a turn.
    h.core.on_tui_hook(
        &id,
        serde_json::json!({ "hook_event_name": "PostToolUse", "session_id": session }),
    );
    assert_eq!(row_status(&h, &id, "call-stopped"), RunStatus::Running);
    h.core.on_tui_hook(
        &id,
        serde_json::json!({ "hook_event_name": "Stop", "session_id": session }),
    );
    let status = |call: &str| row_status(&h, &id, call);
    assert_eq!(status("call-stopped"), RunStatus::Stopped);
    assert_eq!(status("call-notask"), RunStatus::Stopped);
    assert_eq!(status("call-running"), RunStatus::Running);
    // A subagent in the foreground ends with the turn of its agent, as it always did.
    assert_eq!(status("call-foreground"), RunStatus::Running);
    assert_eq!(status("call-over"), RunStatus::Done);
    assert_eq!(status("call-nofile"), RunStatus::Running);
    let call = h
        .items(&id)
        .into_iter()
        .find(|i| i["id"] == "call-stopped")
        .unwrap();
    assert_eq!(call["run"]["status"], "stopped");
    assert!(agent_events(&h, &id) > sent);
    // The paths found are kept, for the next end of a turn.
    assert!(h.core.sidechains.lock().len() >= 2);
}

/// The real `claude` of `ESCOUADE_REAL_CLAUDE` in « Vue Claude Code », on the machine's own
/// account (Principal: no `CLAUDE_CONFIG_DIR`, the home folder `ESCOUADE_TEST_REAL_HOME` names),
/// on Haiku, up and its prompt ready: the agent, its terminal and what that showed.
async fn real_interface(
    name: &str,
) -> (Harness, String, TermInfo, Arc<parking_lot::Mutex<String>>) {
    use std::time::{Duration, Instant};
    let program = std::env::var("ESCOUADE_REAL_CLAUDE").expect("ESCOUADE_REAL_CLAUDE");
    let home = std::env::var("ESCOUADE_TEST_REAL_HOME").expect("ESCOUADE_TEST_REAL_HOME");
    let home = std::path::Path::new(&home).join(".claude");
    let h = harness(name);
    let (p, repo) = h.project(false).await;
    println!("repo {}", repo.display());
    // Principal, as the app runs it: no `CLAUDE_CONFIG_DIR`, its own Claude Code.
    let mut s = h.core.settings.read().clone();
    s.claude_path = program;
    s.mcp_enabled = false;
    h.core.save_settings(s).unwrap();
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                model: Some("haiku".into()),
                mode: Some("default".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    h.core.set_agent_view(&id, true).await.unwrap();
    let screen = Arc::new(parking_lot::Mutex::new(String::new()));
    let sink = screen.clone();
    let info = h
        .core
        .open_agent_terminal(&id, (120, 40), false, move |b| {
            sink.lock().push_str(&String::from_utf8_lossy(&b))
        })
        .await
        .unwrap();
    // Its first screen: the box about the folder (answered as the user would), then its prompt.
    let start = Instant::now();
    let (mut answered, mut trusted) = (false, false);
    let session = h.agent(&id).session_id.unwrap();
    let registered = || {
        std::fs::read_dir(home.join("sessions"))
            .into_iter()
            .flatten()
            .flatten()
            .any(|f| std::fs::read_to_string(f.path()).is_ok_and(|c| c.contains(&session)))
    };
    while !registered() {
        let text = plain_text(&screen.lock());
        if !answered && screen.lock().contains("\x1b[6n") {
            h.core.pty.write(&info.id, b"\x1b[1;1R").unwrap();
            answered = true;
        }
        // « No, exit » chosen first, then « Yes, I trust this folder » (`plain_text` drops the
        // spaces).
        if !trusted && text.contains("Itrustthisfolder") {
            tokio::time::sleep(Duration::from_millis(500)).await;
            h.core.pty.write(&info.id, b"\x1b[B").unwrap();
            tokio::time::sleep(Duration::from_millis(300)).await;
            h.core.pty.write(&info.id, b"\r").unwrap();
            trusted = true;
        }
        assert!(start.elapsed() < Duration::from_secs(60), "{text}");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    println!("ready after {:?}, trusted: {trusted}", start.elapsed());
    (h, id, info, screen)
}

/// Real run (ESC-12): « Vue Claude Code » with the real `claude` that `ESCOUADE_REAL_CLAUDE`
/// names, on the machine's own account (Principal: no `CLAUDE_CONFIG_DIR`, the home folder
/// `ESCOUADE_TEST_REAL_HOME` names), on Haiku: a message pasted on two lines as soon as the
/// interface is up, `/compact`, Escape during a turn, a subagent, then `/clear`, each followed in
/// Escouade as its interface writes it. About 0.13 $ a run. By hand only (`cargo test --lib
/// real_claude_view -- --ignored --nocapture`, out of any Claude Code session:
/// `env -u CLAUDECODE -u CLAUDE_CODE_CHILD_SESSION`), the folder of its sessions taken out of
/// `~/.claude/projects` after (docs/TODO-1.8.md, « Vue Claude Code »).
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn real_claude_view() {
    use std::time::{Duration, Instant};
    let (h, id, _, screen) = real_interface("tui-claude-real").await;
    let session = h.agent(&id).session_id.unwrap();
    let turns = |h: &Harness| h.items(&id).iter().filter(|i| i["kind"] == "turn").count();
    let say = |text: &'static str| {
        let (h, id, screen) = (&h, id.clone(), &screen);
        async move {
            let before = turns(h);
            h.core
                .send_message(&id, text.into(), Vec::new())
                .await
                .unwrap();
            let start = Instant::now();
            while turns(h) == before || h.agent(&id).status.is_active() {
                assert!(
                    start.elapsed() < Duration::from_secs(180),
                    "no end of turn for {text:?}: {:#?}\n{}",
                    h.items(&id),
                    plain_text(&screen.lock())
                );
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    };
    let last = |kind: &str| {
        h.items(&id)
            .into_iter()
            .rev()
            .find(|i| i["kind"] == kind)
            .unwrap_or_default()
    };

    // 1. A message on two lines, pasted: one message, its two lines, one turn.
    say("Réponds seulement : A\nPuis rien d'autre.").await;
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(session.as_str()));
    println!("session {session}");
    assert_eq!(
        last("user")["text"],
        "Réponds seulement : A\nPuis rien d'autre."
    );
    assert_eq!(texts(&h, &id, "user").len(), 1);
    println!(
        "1. answer {:?}, cost {}",
        last("text")["text"],
        h.agent(&id).cost
    );
    assert_eq!(h.agent(&id).status, AgentStatus::Done);

    // 2. `/compact`: told once, its turn ended by its output, and the next message answered.
    say("/compact").await;
    println!(
        "2. after /compact: {:#?}",
        &h.items(&id)[h.items(&id).len().saturating_sub(4)..]
    );
    assert!(notices_of(&h, &id).iter().any(|n| n.contains("compact")));
    assert_eq!(
        texts(&h, &id, "user")
            .iter()
            .filter(|u| *u == "/compact")
            .count(),
        1
    );
    assert_eq!(last("turn")["interrupted"], false);
    say("Réponds seulement : B").await;
    println!("2. answer {:?}", last("text")["text"]);
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(session.as_str()));

    // 3. Escape during a turn: it ends, interrupted (by the transcript, or by the registry when
    // it was cancelled before its first answer).
    let before = turns(&h);
    h.core
        .send_message(
            &id,
            "Écris un long poème de quarante strophes sur la mer, sans outil.".into(),
            Vec::new(),
        )
        .await
        .unwrap();
    let start = Instant::now();
    while h.agent(&id).status != AgentStatus::Running {
        assert!(start.elapsed() < Duration::from_secs(60), "never running");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    tokio::time::sleep(Duration::from_secs(3)).await;
    h.core.interrupt(&id).await.unwrap();
    let start = Instant::now();
    while turns(&h) == before {
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "no end after Escape: {:#?}\n{}",
            h.items(&id),
            plain_text(&screen.lock())
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    println!("3. turn {}, status {:?}", last("turn"), h.agent(&id).status);
    assert_eq!(last("turn")["interrupted"], true);
    assert!(!h.agent(&id).status.is_active());
    // Cancelled before its first answer, the message may be back in the interface's prompt:
    // Escouade empties it before the next one (ESC-118, `real_claude_view_cancelled`).
    tokio::time::sleep(Duration::from_secs(1)).await;
    let tail: String = plain_text(&screen.lock()).chars().rev().take(600).collect();
    let tail: String = tail.chars().rev().collect();
    println!("3. screen after: {tail}");

    // 4. A subagent: its row in the plan, and its thread from its own transcript.
    say(
        "Lance un seul sous-agent avec l'outil Agent (subagent_type general-purpose, model haiku), \
         dont la consigne est : « Réponds seulement OK, sans aucun outil. » Puis réponds seulement : C",
    )
    .await;
    assert!(last("user")["text"]
        .as_str()
        .is_some_and(|u| u.starts_with("Lance un seul sous-agent")));
    let plan = h.agent(&id).plan;
    println!("4. plan {plan:#?}");
    let row = plan
        .expect("a plan")
        .agents
        .first()
        .cloned()
        .expect("a row");
    assert_eq!(row.status, crate::plan::RunStatus::Done);
    let thread = thread(&h, &id, &row.id);
    println!("4. thread {thread:#?}");
    assert!(!thread.is_empty());

    // 5. `/clear`: another session, followed; the next message is answered in it.
    say("/clear").await;
    say("Réponds seulement : D").await;
    let cleared = h.agent(&id).session_id.clone().unwrap();
    println!("5. session {cleared}, answer {:?}", last("text")["text"]);
    assert_ne!(cleared, session);
    assert!(h.agent(&id).plan.is_none_or(|p| p.agents.is_empty()));
    println!("cost {}", h.agent(&id).cost);
    println!("screen:\n{}", plain_text(&screen.lock()));
}

/// Real run (ESC-118): a message cancelled by Escape before its answer, which Claude Code puts
/// back in its prompt; the next one Escouade sends arrives alone (the prompt emptied by one
/// Ctrl+C, the interface still up). Then a draft typed in the terminal after another cancelled
/// message: kept, the next message added to it, and the conversation says so. On Haiku, a few
/// cents. By hand only, as `real_claude_view`.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn real_claude_view_cancelled() {
    let (h, id, info, screen) = real_interface("tui-claude-real-cancel").await;
    let tail = || {
        let all = plain_text(&screen.lock());
        let tail: String = all.chars().rev().take(400).collect();
        tail.chars().rev().collect::<String>()
    };
    let last = |kind: &str| last_item(&h, &id, kind);
    let pause = || tokio::time::sleep(std::time::Duration::from_millis(500));

    // 1. Escouade's message alone in the prompt: emptied, the next one arrives alone.
    real_cancel(
        &h,
        &id,
        "Écris les nombres de un à deux cents en toutes lettres, sans outil.",
    )
    .await;
    pause().await;
    println!("1. screen after the cancel: {}", tail());
    real_say(&h, &id, "Réponds seulement : A").await;
    println!(
        "1. user {:?}, answer {:?}",
        last("user")["text"],
        last("text")["text"]
    );
    assert_eq!(last("user")["text"], "Réponds seulement : A");
    assert!(h.core.pty.exists(&info.id), "the interface quit");
    assert!(notices_of(&h, &id).iter().all(|n| !n.contains("remis")));

    // 2. A draft typed after the cancel: kept, the next message added to it, and said.
    real_cancel(
        &h,
        &id,
        "Écris les nombres de un à deux cents en toutes lettres, sans outil.",
    )
    .await;
    pause().await;
    h.core
        .pty
        .write_from_window(&info.id, " Puis réponds seulement : B".as_bytes())
        .unwrap();
    pause().await;
    println!("2. screen with the draft: {}", tail());
    real_say(&h, &id, " Non, réponds seulement : C").await;
    println!(
        "2. user {:?}, answer {:?}",
        last("user")["text"],
        last("text")["text"]
    );
    assert!(notices_of(&h, &id).iter().any(|n| n.contains("remis")));
    let user = last("user")["text"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(
        user.contains(" B") && user.ends_with("Non, réponds seulement : C"),
        "{user}"
    );
    println!("cost {}", h.agent(&id).cost);
}

/// The last item of the agent's conversation of that kind (null for none).
fn last_item(h: &Harness, id: &str, kind: &str) -> Value {
    h.items(id)
        .into_iter()
        .rev()
        .find(|i| i["kind"] == kind)
        .unwrap_or_default()
}

/// Waits for the end of a turn of the real interface, after `before` turns.
async fn real_turn_after(h: &Harness, id: &str, before: usize) {
    let turns = |h: &Harness| h.items(id).iter().filter(|i| i["kind"] == "turn").count();
    let start = std::time::Instant::now();
    while turns(h) == before || h.agent(id).status.is_active() {
        assert!(
            start.elapsed() < std::time::Duration::from_secs(120),
            "no end of turn: {:#?}",
            h.items(id)
        );
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

/// `text` sent to the real interface, until its turn ends.
async fn real_say(h: &Harness, id: &str, text: &str) {
    let before = h.items(id).iter().filter(|i| i["kind"] == "turn").count();
    h.core
        .send_message(id, text.into(), Vec::new())
        .await
        .unwrap();
    real_turn_after(h, id, before).await;
}

/// `text` sent to the real interface, then Escape as soon as its turn runs: before its first
/// answer, so the registry ends it (`IDLE_ENDS_TURN_MS`).
async fn real_cancel(h: &Harness, id: &str, text: &str) {
    use std::time::{Duration, Instant};
    let before = h.items(id).iter().filter(|i| i["kind"] == "turn").count();
    h.core
        .send_message(id, text.into(), Vec::new())
        .await
        .unwrap();
    let start = Instant::now();
    while h.agent(id).status != AgentStatus::Running {
        assert!(start.elapsed() < Duration::from_secs(60), "never running");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    h.core.interrupt(id).await.unwrap();
    real_turn_after(h, id, before).await;
    let cancelled = h
        .core
        .agent(id)
        .unwrap()
        .lock()
        .tui
        .as_ref()
        .and_then(|t| t.cancelled);
    println!("cancelled {cancelled:?}, turn {}", last_item(h, id, "turn"));
    assert_eq!(last_item(h, id, "turn")["interrupted"], true);
    assert!(
        cancelled.is_some(),
        "ended by its transcript, not cancelled before its answer"
    );
}

/// The notices of the agent's conversation, in lower case.
fn notices_of(h: &Harness, id: &str) -> Vec<String> {
    h.items(id)
        .into_iter()
        .filter(|i| i["kind"] == "notice")
        .map(|i| i["text"].as_str().unwrap_or_default().to_lowercase())
        .collect()
}

/// ESC-124, tried for real: the models Escouade reads of the `claude` of Principal, without a turn
/// (`Core::read_claude_models`), are those its `/model` shows. Run by hand, as `real_claude_view`
/// (`ESCOUADE_REAL_CLAUDE`, `ESCOUADE_TEST_REAL_HOME`, out of any Claude Code session); nothing is
/// asked of the model: `initialize` alone, then `/model` opened and closed.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn real_claude_models() {
    use std::time::{Duration, Instant};
    let program = std::env::var("ESCOUADE_REAL_CLAUDE").expect("ESCOUADE_REAL_CLAUDE");
    let home = std::env::var("ESCOUADE_TEST_REAL_HOME").expect("ESCOUADE_TEST_REAL_HOME");
    let home = std::path::Path::new(&home).join(".claude");
    let h = harness("tui-claude-real-models");
    let (p, _) = h.project(false).await;
    let mut s = h.core.settings.read().clone();
    s.claude_path = program;
    s.mcp_enabled = false;
    h.core.save_settings(s).unwrap();
    let start = Instant::now();
    h.core.read_claude_models("principal").await;
    let told = h.core.account_models.read()["principal"].clone();
    println!("read in {:?}, version {}", start.elapsed(), told.version);
    for m in &told.catalog {
        println!(
            "  {} = {:?}{} efforts {:?}, auto {:?}",
            m.id,
            m.label,
            if m.default { " (default)" } else { "" },
            m.efforts,
            crate::engine::claude_told_auto(&told.models, &m.id)
        );
    }
    assert!(told.catalog.len() >= 4, "{told:#?}");
    // Its `/model`, in its own interface.
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                model: Some("haiku".into()),
                mode: Some("default".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    h.core.set_agent_view(&id, true).await.unwrap();
    let screen = Arc::new(parking_lot::Mutex::new(String::new()));
    let sink = screen.clone();
    let info = h
        .core
        .open_agent_terminal(&id, (140, 60), false, move |b| {
            sink.lock().push_str(&String::from_utf8_lossy(&b))
        })
        .await
        .unwrap();
    let session = h.agent(&id).session_id.unwrap();
    let registered = || {
        std::fs::read_dir(home.join("sessions"))
            .into_iter()
            .flatten()
            .flatten()
            .any(|f| std::fs::read_to_string(f.path()).is_ok_and(|c| c.contains(&session)))
    };
    let (mut answered, mut trusted) = (false, false);
    let start = Instant::now();
    while !registered() {
        let text = plain_text(&screen.lock());
        if !answered && screen.lock().contains("\x1b[6n") {
            h.core.pty.write(&info.id, b"\x1b[1;1R").unwrap();
            answered = true;
        }
        if !trusted && text.contains("Itrustthisfolder") {
            tokio::time::sleep(Duration::from_millis(500)).await;
            h.core.pty.write(&info.id, b"\x1b[B").unwrap();
            tokio::time::sleep(Duration::from_millis(300)).await;
            h.core.pty.write(&info.id, b"\r").unwrap();
            trusted = true;
        }
        assert!(start.elapsed() < Duration::from_secs(60), "{text}");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_secs(8)).await;
    let before = screen.lock().len();
    h.core.pty.write(&info.id, b"/model").unwrap();
    tokio::time::sleep(Duration::from_millis(800)).await;
    h.core.pty.write(&info.id, b"\r").unwrap();
    // Its picker, once the interface is ready for it (its hooks of the start run first).
    let opened = |s: &str| {
        let bare: String = plain_text(s).split_whitespace().collect();
        bare.contains("Selectmodel")
    };
    let start = Instant::now();
    while !opened(&screen.lock()[before..]) {
        let text = plain_text(&screen.lock()[before..]);
        assert!(start.elapsed() < Duration::from_secs(60), "{text}");
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    tokio::time::sleep(Duration::from_secs(1)).await;
    let picker = plain_text(&screen.lock()[before..]);
    println!("/model:\n{picker}");
    h.core.pty.write(&info.id, b"\x1b").unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    h.core.pty.kill(&info.id);
    // Every model read is in it, by the name of the menu, in its order after « Default »; the last
    // ones below its fold (« … +3 models »).
    let bare: String = picker.chars().filter(|c| !c.is_whitespace()).collect();
    let menu = &bare[bare.find("Selectmodel").expect("the picker")..];
    let below: usize = menu
        .split_once("…+")
        .and_then(|(_, rest)| rest.split_once("models"))
        .and_then(|(n, _)| n.parse().ok())
        .unwrap_or(0);
    let shown = told.catalog.len() - below;
    let mut at = menu.find("Default(recommended)").expect("Default");
    for (i, m) in told.catalog.iter().take(shown).enumerate() {
        let entry = format!("{}.{}", i + 2, m.label.replace(' ', ""));
        let found = menu[at..].find(&entry);
        assert!(found.is_some(), "{entry} not in /model after {at}: {menu}");
        at += found.unwrap();
    }
    println!(
        "/model: Default, then the {shown} first of the {} read, {below} below its fold",
        told.catalog.len()
    );
}

/// ESC-138: a command the agent sends to the background in the Claude Code view, Remote Control or
/// not, is listed from its launch (its result in the transcript), runs on after the turn (the card
/// says « Attend », not « Terminé »), and ends with the notification Claude Code replays.
async fn background_command_in_the_view(remote: bool) {
    use crate::plan::RunStatus;
    let h = harness(if remote { "tui-bg-remote" } else { "tui-bg" });
    let (p, _) = h.project(false).await;
    second_account(&h);
    let id = agent_on(&h, &p, "pro").await.id;
    if remote {
        h.core.set_remote_control(&id, true).await.unwrap();
    }
    h.core.set_agent_view(&id, true).await.unwrap();
    open_interface(&h, &id).await;
    up(&h, &id).await;
    let row = |h: &Harness| {
        h.agent(&id).plan.and_then(|p| {
            p.bg_tasks
                .iter()
                .find(|t| t.title == "Wait until PR 148 checks finish")
                .cloned()
        })
    };
    h.core
        .send_message(&id, "commande de fond 4s".into(), Vec::new())
        .await
        .unwrap();
    // Its turn is over, the command runs on: listed, running.
    h.wait("the turn over with the command running", |h| {
        h.agent(&id).status == AgentStatus::Done
            && row(h).is_some_and(|r| r.status == RunStatus::Running)
    })
    .await;
    let started = row(&h).unwrap().started_at;
    assert!(h.agent(&id).plan.unwrap().runs_something());
    // It ends with its notification, after the time it ran.
    h.wait("the command over", |h| {
        row(h).is_some_and(|r| r.status == RunStatus::Done)
    })
    .await;
    let r = row(&h).unwrap();
    assert!(
        r.ended_at.unwrap() - started >= 3_000,
        "listed from its launch: {started} → {:?}",
        r.ended_at
    );
    assert!(r.summary.unwrap().contains("exit code 0"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_background_command_in_the_claude_code_view_is_listed_from_its_launch() {
    background_command_in_the_view(false).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_background_command_in_the_claude_code_view_with_remote_control_is_listed_from_its_launch(
) {
    background_command_in_the_view(true).await;
}
