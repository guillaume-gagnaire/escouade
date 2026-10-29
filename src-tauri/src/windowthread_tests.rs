//! The window's thread (ESC-290): the commands off it on their lanes, the slow ones and the pauses
//! in the log without a path nor a content.

use crate::windowthread::*;
use parking_lot::{Condvar, Mutex};
use serde_json::json;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::ipc::{CallbackFn, InvokeBody, InvokeResponse};
use tauri::webview::InvokeRequest;

#[test]
fn a_family_shares_a_lane_and_any_other_command_has_its_own() {
    assert_eq!(lane_of("term_write"), "term");
    assert_eq!(lane_of("term_resize"), "term");
    assert_eq!(lane_of("attach_chunk"), "attach");
    assert_eq!(lane_of("conversation_listen"), "voice");
    assert_eq!(lane_of("voice_start"), "voice");
    assert_eq!(lane_of("get_code_index"), "get_code_index");
    assert_eq!(lane_of("get_conversation"), "get_conversation");
}

#[test]
fn the_log_gets_the_ids_of_a_call_never_a_path_nor_a_text() {
    let body = InvokeBody::Json(json!({
        "projectId": "3f2a9c81d0e4",
        "agentId": "a1b2c3",
        "path": "/home/someone/kDrive/secret/notes.md",
        "text": "the user's message",
        "id": "/home/someone/file name.txt",
    }));
    let ids = ids_of(&body);
    assert_eq!(ids, " (project 3f2a9c81d0e4 agent a1b2c3)");
    // A project passed whole: its id only.
    let whole = InvokeBody::Json(json!({
        "project": { "id": "p1", "path": "/home/someone/remedy", "name": "remedy" },
    }));
    assert_eq!(ids_of(&whole), " (project p1)");
    assert_eq!(ids_of(&InvokeBody::Json(json!({}))), "");
    assert_eq!(ids_of(&InvokeBody::Raw(b"/home/x".to_vec())), "");
}

#[test]
fn a_command_over_250_ms_is_written_once_in_30_s_with_the_count_between() {
    let lanes = Lanes::new(Arc::default());
    assert_eq!(lanes.slow_line("stats", SLOW, ""), None);
    assert_eq!(
        lanes
            .slow_line("stats", Duration::from_millis(812), " (project p1)")
            .as_deref(),
        Some("slow command: stats took 812 ms (project p1)")
    );
    assert_eq!(lanes.slow_line("stats", Duration::from_secs(2), ""), None);
    assert_eq!(lanes.slow_line("stats", Duration::from_secs(2), ""), None);
    // Another command is its own.
    assert!(lanes
        .slow_line("get_conversation", Duration::from_secs(1), "")
        .is_some());
}

#[test]
fn what_runs_is_told_the_longest_first_and_forgotten_once_done() {
    let running = Running::default();
    let a = running.enter("get_code_index");
    std::thread::sleep(Duration::from_millis(30));
    {
        let _b = running.enter("stats");
        let told = running.told();
        assert!(told.starts_with("get_code_index "), "{told}");
        assert!(told.contains(", stats "), "{told}");
    }
    assert!(!running.told().contains("stats"));
    drop(a);
    assert_eq!(running.told(), "");
}

/// A window's thread of the test: runs the jobs posted to it, and stops for `stall` when asked.
fn window_thread() -> (mpsc::Sender<Job>, std::thread::JoinHandle<()>) {
    let (tx, rx) = mpsc::channel::<Job>();
    let handle = std::thread::spawn(move || {
        while let Ok(job) = rx.recv() {
            job();
        }
    });
    (tx, handle)
}

#[test]
fn a_pause_of_the_window_thread_is_told_with_the_commands_running_then_its_length() {
    let (post, thread) = window_thread();
    let running = Arc::new(Running::default());
    let told: Arc<Mutex<Vec<Pause>>> = Arc::default();
    // The window stops for 400 ms while a command runs on its lane.
    let lane_command = running.clone();
    let stall = post.clone();
    let blocker = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        let _r = lane_command.enter("get_workflow_tracks");
        stall
            .send(Box::new(|| std::thread::sleep(Duration::from_millis(400))))
            .unwrap();
        std::thread::sleep(Duration::from_millis(500));
    });
    let (r, t) = (running.clone(), told.clone());
    let started = Instant::now();
    let watcher = std::thread::spawn(move || {
        watch(
            |job| started.elapsed() < Duration::from_millis(900) && post.send(job).is_ok(),
            &r,
            |p| t.lock().push(p),
            Duration::from_millis(20),
            Duration::from_millis(150),
        )
    });
    watcher.join().unwrap();
    blocker.join().unwrap();
    drop(thread);
    let told = told.lock().clone();
    assert_eq!(told.len(), 2, "{told:?}");
    let Pause::Began { running } = &told[0] else {
        panic!("{told:?}")
    };
    assert!(running.starts_with("get_workflow_tracks "), "{running}");
    let Pause::Ended { lasted } = told[1] else {
        panic!("{told:?}")
    };
    assert!(lasted >= Duration::from_millis(250), "{lasted:?}");
    assert!(told[1].line().starts_with("window thread: paused "));
    assert_eq!(
        Pause::Began {
            running: String::new()
        }
        .line(),
        "window thread: no answer for 1000 ms, no command running"
    );
}

#[test]
fn a_window_thread_that_answers_is_never_told() {
    let (post, _thread) = window_thread();
    let told: Arc<Mutex<Vec<Pause>>> = Arc::default();
    let t = told.clone();
    let started = Instant::now();
    watch(
        |job| started.elapsed() < Duration::from_millis(300) && post.send(job).is_ok(),
        &Running::default(),
        |p| t.lock().push(p),
        Duration::from_millis(10),
        Duration::from_millis(150),
    );
    assert!(told.lock().is_empty());
}

/// The gate the slow command of the mock app waits behind.
static GATE: (Mutex<bool>, Condvar) = (Mutex::new(false), Condvar::new());

#[tauri::command]
fn slow_command() -> String {
    let mut open = GATE.0.lock();
    while !*open {
        GATE.1.wait(&mut open);
    }
    "slow".into()
}

#[tauri::command]
fn fast_command() -> String {
    "fast".into()
}

fn request(cmd: &str) -> InvokeRequest {
    InvokeRequest {
        cmd: cmd.into(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: if cfg!(windows) {
            "http://tauri.localhost"
        } else {
            "tauri://localhost"
        }
        .parse()
        .unwrap(),
        body: InvokeBody::default(),
        headers: Default::default(),
        invoke_key: tauri::test::INVOKE_KEY.to_string(),
    }
}

/// A command whose own part takes long (simulated: it waits behind `GATE`) is called from the
/// window's thread, then a quick one: the window's thread goes on at once, and the quick one
/// answers while the slow one still runs. Through Tauri's own dispatch of the calls (mock runtime),
/// with the handler `lib.rs` gives the app.
#[test]
fn a_slow_command_neither_holds_the_window_thread_nor_the_other_commands() {
    let lanes = Lanes::new(Arc::default());
    let app = tauri::test::mock_builder()
        .invoke_handler(off_the_window(
            lanes.clone(),
            tauri::generate_handler![slow_command, fast_command],
        ))
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let (answers, answered) = mpsc::channel::<(String, String)>();
    // The window's thread: it calls both, and says when it is free again.
    let (free, freed) = mpsc::channel::<()>();
    let w = webview.clone();
    std::thread::spawn(move || {
        for cmd in ["slow_command", "fast_command"] {
            let answers = answers.clone();
            let name = cmd.to_string();
            w.as_ref().clone().on_message(
                request(cmd),
                Box::new(move |_w, _cmd, response, _c, _e| {
                    let InvokeResponse::Ok(body) = response else {
                        panic!("{name} failed")
                    };
                    let text: String = body.deserialize().unwrap();
                    let _ = answers.send((name, text));
                }),
            );
        }
        let _ = free.send(());
    });
    let within = Duration::from_secs(5);
    freed
        .recv_timeout(within)
        .expect("the window's thread waited for the slow command");
    assert_eq!(
        answered.recv_timeout(within).unwrap(),
        ("fast_command".to_string(), "fast".to_string()),
        "the quick command waited for the slow one"
    );
    assert!(lanes.running.told().starts_with("slow_command "));
    *GATE.0.lock() = true;
    GATE.1.notify_all();
    assert_eq!(
        answered.recv_timeout(within).unwrap(),
        ("slow_command".to_string(), "slow".to_string())
    );
}

/// The calls of one lane keep their order: a terminal's keystrokes reach it as typed.
#[test]
fn the_calls_of_a_lane_run_in_order() {
    let lanes = Lanes::new(Arc::default());
    let seen: Arc<Mutex<Vec<u32>>> = Arc::default();
    let (done, finished) = mpsc::channel();
    for i in 0..200u32 {
        let (seen, done) = (seen.clone(), done.clone());
        lanes.submit(
            "term",
            Box::new(move || {
                if i % 50 == 0 {
                    std::thread::sleep(Duration::from_millis(5));
                }
                seen.lock().push(i);
                if i == 199 {
                    done.send(()).unwrap();
                }
            }),
        );
    }
    finished.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(*seen.lock(), (0..200).collect::<Vec<_>>());
}

/// An unknown command is refused, as Tauri refuses it, instead of never answering.
#[test]
fn an_unknown_command_is_refused() {
    let app = tauri::test::mock_builder()
        .invoke_handler(off_the_window(
            Lanes::new(Arc::default()),
            tauri::generate_handler![fast_command],
        ))
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let got = tauri::test::get_ipc_response(&webview, request("nope"));
    assert!(got.is_err(), "{got:?}");
    let got = tauri::test::get_ipc_response(&webview, request("fast_command"));
    assert_eq!(got.unwrap().deserialize::<String>().unwrap(), "fast");
}
