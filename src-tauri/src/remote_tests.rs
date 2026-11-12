//! The Remote Control for every agent without a process kept for each (ESC-417, issue #26): only
//! the agents that want their process start for it, those that cannot be linked never do, an
//! account that refused it for good is not asked again, the processes it keeps have a cap, and
//! « Serveurs MCP des agents : seulement Escouade » gives Claude Code `--strict-mcp-config`.

use crate::accounts::PRINCIPAL;
use crate::core::{over_remote_cap, remote_refusal_is_final, remote_wants_process, AgentOptions};
use crate::core_tests::{harness, Harness};
use crate::model::*;
use std::path::Path;
use std::time::Duration;

impl Harness {
    /// The agent with no process, as after a restart of the app (creating one warms its process
    /// up: it is let start, then stopped).
    async fn cold(&self, id: &str) {
        self.core.ensure_process(id).await.unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
        if let Some(p) = self.core.agent(id).unwrap().lock().detach() {
            p.close_input();
        }
    }

    /// The agent as the last run left it: Remote Control in force from the global setting, its
    /// status and its last activity `ago` ms ago.
    fn left_linked(&self, id: &str, status: AgentStatus, ago: i64) {
        let a = self.core.agent(id).unwrap();
        let mut rt = a.lock();
        rt.meta.remote_control = true;
        rt.meta.remote_source = Some(RemoteSource::Global);
        rt.meta.status = status;
        rt.meta.last_activity = now_ms() - ago;
    }

    /// The app starts with « Remote control pour tous les agents » on, and its starts are over.
    async fn app_starts(&self) {
        self.core.settings.write().remote_control_all = true;
        for t in self.core.start_remote_agents() {
            t.await.unwrap();
        }
    }

    fn linked(&self, id: &str) -> bool {
        self.core.agent(id).unwrap().lock().remote_linked
    }

    fn refused(&self, account: &str) -> Option<String> {
        crate::accounts::find(&self.core.settings.read(), account)
            .and_then(|a| a.remote_refused.clone())
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

#[test]
fn only_an_active_agent_or_one_active_lately_wants_its_process() {
    let now = 1_000_000_000;
    let meta = |status: AgentStatus, ago_min: i64| AgentMeta {
        status,
        last_activity: now - ago_min * 60_000,
        ..Default::default()
    };
    // Active: whatever its last activity.
    assert!(remote_wants_process(
        &meta(AgentStatus::Running, 600),
        now,
        30
    ));
    assert!(remote_wants_process(
        &meta(AgentStatus::Waiting, 600),
        now,
        30
    ));
    // Idle: within the delay of the idle ones only.
    assert!(remote_wants_process(&meta(AgentStatus::Idle, 5), now, 30));
    assert!(!remote_wants_process(&meta(AgentStatus::Idle, 31), now, 30));
    // Done (or stopped on an error): never, however recent.
    assert!(!remote_wants_process(&meta(AgentStatus::Done, 0), now, 30));
    assert!(!remote_wants_process(&meta(AgentStatus::Error, 0), now, 30));
    // No delay (« Jamais »): an idle one does, the cap bounds them.
    assert!(remote_wants_process(&meta(AgentStatus::Idle, 6000), now, 0));
}

#[test]
fn only_a_missing_subscription_is_a_refusal_for_good() {
    assert!(remote_refusal_is_final(
        "Remote Control requires a claude.ai subscription"
    ));
    assert!(!remote_refusal_is_final("control request timed out"));
    assert!(!remote_refusal_is_final("Codex ne le permet pas"));
}

#[test]
fn over_the_cap_the_least_recently_active_go_first() {
    let kept = vec![(30, "c"), (10, "a"), (40, "d"), (20, "b")];
    assert_eq!(over_remote_cap(kept.clone(), 2), ["a", "b"]);
    assert_eq!(over_remote_cap(kept.clone(), 4), Vec::<&str>::new());
    assert_eq!(over_remote_cap(kept, 0), ["a", "b", "c", "d"]);
    assert_eq!(Settings::default().remote_keep_max, 8);
}

/// Criterion 1: at the app's launch, an agent done or idle for long does not start for the Remote
/// Control (issue #26: 61 of the 65 started were done); an active one does, and one active lately.
/// The others keep their Remote Control and their remote session, which their next process
/// takes back.
#[tokio::test]
async fn the_app_starting_starts_no_agent_done_or_idle_for_the_remote_control() {
    let h = harness("esc417-startup");
    let (p, r) = h.project(false).await;
    let mut ids = Vec::new();
    for _ in 0..4 {
        let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
        h.cold(&id).await;
        ids.push(id);
    }
    let [running, lately, done, idle] = ids.try_into().unwrap();
    h.left_linked(&running, AgentStatus::Running, 3 * 3_600_000);
    h.left_linked(&lately, AgentStatus::Idle, 10_000);
    h.left_linked(&done, AgentStatus::Done, 10_000);
    // Idle longer than « Arrêter les agents inactifs après » (1 minute in the tests).
    h.left_linked(&idle, AgentStatus::Idle, 3 * 60_000);
    h.core.agent(&done).unwrap().lock().meta.remote_session = Some("cse_done".into());

    h.app_starts().await;
    assert!(h.alive(&running) && h.linked(&running));
    assert!(h.alive(&lately) && h.linked(&lately));
    assert!(!h.alive(&done));
    assert!(!h.alive(&idle));
    for id in [&done, &idle] {
        let m = h.agent(id);
        assert!(m.remote_control, "{id} keeps its remote control");
        assert_eq!(m.remote_source, Some(RemoteSource::Global));
    }
    assert_eq!(h.agent(&done).remote_session.as_deref(), Some("cse_done"));

    // Its next process (a message, a ticket, a resume) links it, to the same session.
    h.core.ensure_process(&done).await.unwrap();
    let req = h.remote_requests(&r).pop().unwrap();
    assert_eq!(req["reattach_session_id"], "cse_done");
    assert!(h.linked(&done));
}

/// Criterion 5: the processes the Remote Control keeps have a cap; over it, the least recently
/// active stop, at the launch as later, their remote session kept.
#[tokio::test]
async fn the_remote_control_keeps_no_more_processes_than_its_cap() {
    let h = harness("esc417-cap");
    let (p, _) = h.project(false).await;
    h.core.settings.write().remote_keep_max = 1;
    let mut ids = Vec::new();
    for _ in 0..3 {
        let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
        h.cold(&id).await;
        ids.push(id);
    }
    let [oldest, middle, newest] = ids.clone().try_into().unwrap();
    h.left_linked(&oldest, AgentStatus::Idle, 30_000);
    h.left_linked(&middle, AgentStatus::Idle, 20_000);
    h.left_linked(&newest, AgentStatus::Idle, 10_000);
    h.app_starts().await;
    assert!(h.alive(&newest) && h.linked(&newest));
    assert!(!h.alive(&middle) && !h.alive(&oldest));

    // Later, on the timer of the idle ones: three linked, a cap of two.
    h.core.settings.write().remote_keep_max = 2;
    for id in [&oldest, &middle] {
        h.core.ensure_process(id).await.unwrap();
    }
    h.wait("the three linked", |h| ids.iter().all(|id| h.linked(id)))
        .await;
    let sessions: Vec<_> = ids.iter().map(|id| h.agent(id).remote_session).collect();
    for (id, ago) in [(&oldest, 30_000), (&middle, 20_000), (&newest, 10_000)] {
        h.core.agent(id).unwrap().lock().meta.last_activity = now_ms() - ago;
    }
    h.core.stop_idle_processes();
    assert!(!h.alive(&oldest));
    assert!(h.alive(&middle) && h.alive(&newest));
    let m = h.agent(&oldest);
    assert!(m.remote_control);
    assert_eq!(m.remote_session, sessions[0]);
    // An active one is never stopped for the cap, nor counted.
    h.core.settings.write().remote_keep_max = 0;
    h.core.agent(&middle).unwrap().lock().meta.status = AgentStatus::Running;
    h.core.stop_idle_processes();
    assert!(h.alive(&middle));
    assert!(!h.alive(&newest));
}

/// ESC-431: at the launch, the places under the cap go to the most recently active, however long
/// each took to get its settings applied: the newest, slower to get there, still has the only one,
/// whether it held its Remote Control from the last run or comes in with the setting.
#[tokio::test]
async fn the_places_under_the_cap_go_to_the_most_recently_active_whatever_their_speed() {
    let h = harness("esc431-order");
    let (p, _) = h.project(false).await;
    h.core.settings.write().remote_keep_max = 1;
    for held in [true, false] {
        let mut ids = Vec::new();
        for _ in 0..3 {
            let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
            h.cold(&id).await;
            ids.push(id);
        }
        let [oldest, middle, newest] = ids.clone().try_into().unwrap();
        for (id, ago) in [(&oldest, 30_000), (&middle, 20_000), (&newest, 10_000)] {
            h.left_linked(id, AgentStatus::Idle, ago);
            if !held {
                let a = h.core.agent(id).unwrap();
                let mut rt = a.lock();
                rt.meta.remote_control = false;
                rt.meta.remote_source = None;
            }
        }
        h.core
            .remote_apply_delay
            .lock()
            .insert(newest.clone(), Duration::from_millis(600));
        h.core.settings.write().remote_control_all = true;
        for t in h.core.start_remote(ids.clone()) {
            t.await.unwrap();
        }
        assert!(h.alive(&newest), "held {held}: the newest started");
        assert!(
            !h.alive(&middle) && !h.alive(&oldest),
            "held {held}: none other"
        );
        for id in &ids {
            if let Some(p) = h.core.agent(id).unwrap().lock().detach() {
                p.close_input();
            }
            h.core.agent(id).unwrap().lock().meta.status = AgentStatus::Done;
        }
    }
}

/// Criterion 3: a harness that cannot be linked to claude.ai starts no process for the Remote
/// Control (issue #26: `codex app-server` started to fail with « Codex ne le permet pas »).
#[tokio::test]
async fn a_codex_agent_has_no_process_started_for_the_remote_control() {
    let h = harness("esc417-codex");
    let (p, _) = h.project(false).await;
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "principal-codex".into(),
            name: "Codex".into(),
            engine: EngineId::CODEX,
            claude_path: crate::engine::fake_codex(),
            ..Default::default()
        })
    });
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::CODEX),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    h.cold(&id).await;
    h.core.agent(&id).unwrap().lock().meta.last_activity = now_ms();
    assert!(!h.core.remote_can_link(&h.core.agent(&id).unwrap()));

    // The setting turned on, then the app starting again.
    h.core.settings.write().remote_control_all = true;
    for t in h.core.apply_remote_to(None) {
        t.await.unwrap();
    }
    assert!(!h.alive(&id));
    h.app_starts().await;
    assert!(!h.alive(&id));
    // Started for a message, it is not asked: nothing says it failed.
    h.core.ensure_process(&id).await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!h.items(&id).iter().any(|i| i["kind"] == "notice"
        && i["text"]
            .as_str()
            .is_some_and(|t| t.starts_with("Remote control indisponible"))));
}

/// Criterion 3: an account that refused the Remote Control for good (no claude.ai subscription)
/// is not asked again, at the next launch nor at the next process, and the card says it once;
/// its settings or its sign-in changed, it is asked again.
#[tokio::test]
async fn an_account_with_no_subscription_is_not_asked_again() {
    let h = harness("esc417-refused");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.cold(&id).await;
    h.fake_flag(&r, "remote-refused");
    h.core.settings.write().remote_control_all = true;
    for t in h.core.apply_remote_to(None) {
        t.await.unwrap();
    }
    let refused = h.refused(PRINCIPAL).expect("the refusal kept");
    assert!(refused.contains("claude.ai subscription"), "{refused}");
    let told = |h: &Harness| {
        h.items(&id)
            .iter()
            .filter(|i| {
                i["kind"] == "notice"
                    && i["text"]
                        .as_str()
                        .is_some_and(|t| t.starts_with("Remote control indisponible"))
            })
            .count()
    };
    assert_eq!(told(&h), 1);
    let asked = h.remote_requests(&r).len();
    // Kept in the settings' file, for the next run.
    let file: Settings =
        serde_json::from_slice(&std::fs::read(h.core.data.settings_file()).unwrap()).unwrap();
    assert!(crate::accounts::find(&file, PRINCIPAL)
        .unwrap()
        .remote_refused
        .is_some());

    // The next launch starts nothing for it; a message's process asks nothing.
    h.cold(&id).await;
    h.app_starts().await;
    assert!(!h.alive(&id));
    h.core.ensure_process(&id).await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(h.remote_requests(&r).len(), asked);
    assert_eq!(told(&h), 1);
    assert!(h.agent(&id).remote_control);

    // A window's save of the same settings keeps it, though its copy does not have it.
    let mut copy = h.core.settings.read().clone();
    for a in &mut copy.accounts {
        a.remote_refused = None;
    }
    h.core.save_settings(copy).unwrap();
    assert!(h.refused(PRINCIPAL).is_some());
    // Its settings changed: asked again at its next process, and linked once it may be.
    h.fake_unflag(&r, "remote-refused");
    h.set_settings(|s| s.accounts[0].name = "Moi".into());
    assert_eq!(h.refused(PRINCIPAL), None);
    h.cold(&id).await;
    h.core.ensure_process(&id).await.unwrap();
    h.wait("linked", |h| h.linked(&id)).await;
    assert!(h.remote_requests(&r).len() > asked);
}

#[test]
fn a_refusal_goes_with_a_change_of_the_accounts_settings_only() {
    let refused = Account {
        id: "a".into(),
        remote_refused: Some("no subscription".into()),
        ..Default::default()
    };
    let mut same = vec![Account {
        remote_refused: None,
        ..refused.clone()
    }];
    crate::accounts::keep_remote_refusals(std::slice::from_ref(&refused), &mut same);
    assert_eq!(same[0].remote_refused.as_deref(), Some("no subscription"));
    let mut changed = vec![Account {
        config_dir: "/elsewhere".into(),
        ..refused.clone()
    }];
    crate::accounts::keep_remote_refusals(std::slice::from_ref(&refused), &mut changed);
    assert_eq!(changed[0].remote_refused, None);
}

/// Criterion 6: « Serveurs MCP des agents : seulement Escouade » gives Claude Code
/// `--strict-mcp-config`: the servers of the user's configuration, of the project's `.mcp.json`
/// and of the claude.ai account are not loaded; « Tous les miens » (by default) loads them.
#[tokio::test]
async fn escouade_alone_as_the_agents_mcp_servers_gives_claude_code_strict_mcp_config() {
    let h = harness("esc417-mcp");
    let (p, r) = h.project(false).await;
    assert_eq!(Settings::default().agents_mcp, AgentsMcp::All);
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.cold(&id).await;
    let strict = |h: &Harness, r: &Path| {
        h.launches(r)
            .last()
            .unwrap()
            .contains(&"--strict-mcp-config".to_string())
    };
    h.core.ensure_process(&id).await.unwrap();
    assert!(!strict(&h, &r));
    h.cold(&id).await;
    h.set_settings(|s| s.agents_mcp = AgentsMcp::Escouade);
    h.core.ensure_process(&id).await.unwrap();
    let launch = h.launches(&r).last().unwrap().clone();
    assert!(strict(&h, &r), "{launch:?}");
    // After the plugin's arguments: `--mcp-config` takes several values and stops at it.
    if let Some(at) = launch.iter().position(|a| a == "--mcp-config") {
        let strict_at = launch
            .iter()
            .position(|a| a == "--strict-mcp-config")
            .unwrap();
        assert!(strict_at > at + 1, "{launch:?}");
    }
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(h.core.data.settings_file()).unwrap()).unwrap();
    assert_eq!(saved["agentsMcp"], "escouade");
    assert_eq!(saved["remoteKeepMax"], 8);
}
