//! The core in a process without a window (ESC-451, `docs/REMOTE.md`, « L'ordre proposé », the
//! first short trial): no Tauri app, no window, no display under Linux. It creates a project, runs
//! an agent of the fake `claude` (tests/fixtures) to the end of a turn, and what the desktop would
//! show comes out of its host as events.

use crate::core_tests::{fake_cli, repo};
use crate::headless::{Headless, InstanceLock, LOCK_FILE};
use crate::host::{HeadlessHost, Host, HostEvent, Opened};
use crate::model::*;
use crate::paths::{test_dir, DataDir};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A data folder with the settings of the core's tests: the fake `claude`, Claude Code's Principal.
pub(crate) fn data(name: &str) -> (std::path::PathBuf, DataDir) {
    let dir = test_dir(name);
    let data = DataDir::new(dir.join("data"));
    data.ensure().unwrap();
    let settings = Settings {
        claude_path: fake_cli(),
        // What the desktop would chime and show: out of the host, as events.
        sound: true,
        os_notifications: true,
        idle_stop_minutes: 1,
        accounts: vec![crate::accounts::principal()],
        ..Default::default()
    };
    std::fs::write(data.settings_file(), serde_json::to_vec(&settings).unwrap()).unwrap();
    (dir, data)
}

/// Waits for `pred`, 60 s at most (the whole suite runs hundreds of fakes at once).
pub(crate) async fn wait(what: &str, pred: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        if pred() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(pred(), "timed out waiting for {what}");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_core_without_a_window_runs_an_agent_of_the_fake_claude_to_the_end_of_a_turn() {
    // As on a VPS or in a container: no display at all (nextest runs a test in a process of its
    // own, so the variables go for this one only).
    #[cfg(target_os = "linux")]
    {
        std::env::remove_var("DISPLAY");
        std::env::remove_var("WAYLAND_DISPLAY");
    }
    let (dir, data) = data("headless-turn");
    let root = data.root().to_path_buf();
    let h = Headless::start(data).unwrap();
    // The data folder is this process's: a second core on it is refused.
    assert!(root.join(LOCK_FILE).is_file());
    assert!(InstanceLock::acquire(&root).is_err());
    // The events of the interface, as a client would get them.
    let events = Arc::new(parking_lot::Mutex::new(Vec::<UiEvent>::new()));
    let sink = events.clone();
    h.core.hub.subscribe(Box::new(move |e| sink.lock().push(e)));
    let mut shown = h.host.subscribe();
    assert_eq!(h.core.host.version(), env!("CARGO_PKG_VERSION"));

    let r = repo(&dir);
    let p = h
        .core
        .create_project(
            &r.to_string_lossy(),
            "demo",
            "oklch(0.72 0.12 48)",
            false,
            None,
            true,
        )
        .await
        .unwrap();
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&a, "Bonjour".into(), vec![])
        .await
        .unwrap();
    let core = h.core.clone();
    let id = a.clone();
    let status = move || core.agent(&id).unwrap().lock().meta.status;
    wait("the end of the turn", || status() == AgentStatus::Done).await;
    let id = a;
    assert!(h
        .core
        .agent(&id)
        .unwrap()
        .lock()
        .conv
        .items()
        .iter()
        .any(|i| i["text"] == "Bonjour, tu as dit : Bonjour"));
    // The interface was told, by the hub: the agent done.
    wait("the agent's view", || {
        events.lock().iter().any(|e| {
            matches!(e, UiEvent::Agent { agent } if agent.meta.id == id && agent.meta.status == AgentStatus::Done)
        })
    })
    .await;
    // And what the desktop would show of the end of the turn came out of the host.
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut got = Vec::new();
    while !got.iter().any(|e| matches!(e, HostEvent::Notify { .. })) {
        let left = deadline.saturating_duration_since(Instant::now());
        match tokio::time::timeout(left, shown.recv()).await {
            Ok(Some(e)) => got.push(e),
            _ => panic!("no notification out of the host: {got:?}"),
        }
    }
    assert!(got.contains(&HostEvent::Chime), "{got:?}");
    assert!(got.contains(&HostEvent::Flash), "{got:?}");
    assert!(got
        .iter()
        .any(|e| matches!(e, HostEvent::Notify { title, .. } if title.starts_with("demo · "))));

    h.stop();
    // The folder is free again once the core has stopped.
    InstanceLock::acquire(&root).unwrap();
}

#[test]
fn the_lock_of_a_data_folder_goes_with_its_holder() {
    let root = test_dir("headless-lock").join("data");
    let first = InstanceLock::acquire(&root).unwrap();
    assert_eq!(first.path(), root.join(LOCK_FILE));
    let refused = InstanceLock::acquire(&root).unwrap_err().to_string();
    assert!(refused.contains(&root.display().to_string()), "{refused}");
    drop(first);
    let _again = InstanceLock::acquire(&root).unwrap();
}

#[tokio::test]
async fn a_host_without_a_window_publishes_what_the_desktop_shows_to_each_subscriber() {
    let host = HeadlessHost::default();
    let (mut one, mut two) = (host.subscribe(), host.subscribe());
    host.chime();
    host.notify("t", "b", Box::new(|| {}));
    host.show_waiting(2, crate::i18n::Lang::Fr);
    host.open(Opened::Url("https://escouade.dev".into()))
        .unwrap();
    let all = [
        HostEvent::Chime,
        HostEvent::Notify {
            id: 1,
            title: "t".into(),
            body: "b".into(),
        },
        HostEvent::Waiting { count: 2 },
        HostEvent::Open(Opened::Url("https://escouade.dev".into())),
    ];
    for want in &all {
        assert_eq!(one.recv().await.as_ref(), Some(want));
        assert_eq!(two.recv().await.as_ref(), Some(want));
    }
    // A subscriber that went is forgotten, the others still get the events.
    drop(one);
    host.flash();
    assert_eq!(two.recv().await, Some(HostEvent::Flash));
    // Nothing else a desktop has: no one looks at it, no clipboard.
    assert!(!host.attended());
    assert!(!host.window_visible());
    assert!(host.clipboard_image().await.is_err());
}
