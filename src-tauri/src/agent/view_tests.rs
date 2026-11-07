//! A view whose plugin does not read it: a module that does not answer, a plugin turned off, and what starts anew.

use super::*;
use crate::plugins::wasm::{self, Instance};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// What a turn of the module that loops is given.
const TURN: Duration = Duration::from_millis(300);

fn agent(plugin: Option<&'static str>) -> AgentRt {
    let dir = crate::paths::test_dir(&format!("esc352-view-{}", new_id()));
    let meta = AgentMeta {
        id: "a1".into(),
        engine: EngineId::COPILOT,
        cwd: "C:\\wt".into(),
        session_id: Some("s1".into()),
        ..Default::default()
    };
    let mut a = AgentRt::new(meta, &dir);
    a.start_tui();
    a.view.plugin = plugin;
    a
}

/// An instance that loops at what it is asked: `probe/spin` of the fixture, until its turn is over.
fn looping() -> Result<(Instance, Option<&'static str>), String> {
    let file = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures/plugins/reactor/plugin.wasm");
    let bytes = std::fs::read(file).map_err(|e| e.to_string())?;
    let module = wasm::reactor(&bytes)?;
    let limits = wasm::Limits {
        turn: TURN,
        ..wasm::Limits::default()
    };
    let instance =
        Instance::start_with(&module, &BTreeMap::new(), limits, None).map_err(|f| f.to_string())?;
    Ok((instance, Some("probe/spin")))
}

fn typed(id: &str, text: &str) -> Value {
    json!({ "type": "user.message", "id": id, "data": { "content": text, "messageId": id } })
}

fn notices(a: &mut AgentRt) -> Vec<String> {
    a.conv
        .items()
        .iter()
        .filter(|i| i["kind"] == "notice")
        .filter_map(|i| i["text"].as_str().map(str::to_string))
        .collect()
}

#[test]
fn a_module_that_loops_holds_the_agent_once_then_lets_its_lines_go_by() {
    const PLUGIN: &str = "test.loops";
    let made = Arc::new(AtomicUsize::new(0));
    let count = made.clone();
    crate::plugins::view::stand_in(
        PLUGIN,
        Box::new(move || {
            count.fetch_add(1, Ordering::SeqCst);
            looping()
        }),
    );
    let mut a = agent(Some(PLUGIN));
    let mut fx = Effects::default();
    // The first line takes its two tries, each its whole turn, and says why.
    let at = Instant::now();
    a.view_line(&typed("m1", "Un"), &mut fx);
    assert!(at.elapsed() >= TURN * 2, "{:?}", at.elapsed());
    assert_eq!(made.load(Ordering::SeqCst), 2);
    assert_eq!(notices(&mut a).len(), 1);
    // The next ones do not wait for it, by far, nor call it.
    let at = Instant::now();
    for i in 0..100 {
        a.view_line(&typed(&format!("n{i}"), "Deux"), &mut fx);
    }
    assert!(at.elapsed() < TURN, "{:?}", at.elapsed());
    assert_eq!(made.load(Ordering::SeqCst), 2);
    // The pause over: one try, and no more words.
    a.view.paused = Some(Instant::now());
    a.view_line(&typed("m3", "Trois"), &mut fx);
    // One instance more is one try: a clock would count the module's start too, slow on a runner.
    assert_eq!(made.load(Ordering::SeqCst), 3);
    assert_eq!(notices(&mut a).len(), 1);
    assert!(a.view.paused.is_some());
}

#[test]
fn a_plugin_off_is_said_once_and_the_view_goes_on_once_it_is_on() {
    const PLUGIN: &str = "test.switched";
    let on = Arc::new(AtomicBool::new(false));
    let is_on = on.clone();
    crate::plugins::view::stand_in(
        PLUGIN,
        Box::new(move || {
            if !is_on.load(Ordering::SeqCst) {
                return Err(crate::plugins::view::off());
            }
            let copilot = crate::plugins::builtin::COPILOT;
            Ok((crate::plugins::view::module_of_the_tests(copilot)?, None))
        }),
    );
    let mut a = agent(Some(PLUGIN));
    let mut fx = Effects::default();
    for i in 0..5 {
        a.view_line(&typed(&format!("m{i}"), "Pas lu"), &mut fx);
    }
    assert_eq!(
        notices(&mut a),
        ["Le plugin de Copilot CLI n’a pas lu ce que sa vue a écrit (il est coupé ou n’est pas installé)."]
    );
    assert!(!a.conv.items().iter().any(|i| i["kind"] == "user"));
    // Still off once the pause is over: not said again.
    a.view.paused = Some(Instant::now());
    a.view_line(&typed("m5", "Pas lu"), &mut fx);
    assert_eq!(notices(&mut a).len(), 1);
    // Turned on again: the view reads what comes next.
    on.store(true, Ordering::SeqCst);
    a.view.paused = Some(Instant::now());
    a.view_line(&typed("m6", "Lu"), &mut fx);
    let users: Vec<Value> = a.conv.items();
    let users: Vec<&Value> = users.iter().filter(|i| i["kind"] == "user").collect();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0]["text"], "Lu");
    assert!(a.view.paused.is_none());
    assert_eq!(notices(&mut a).len(), 1);
}

#[test]
fn a_turn_ends_though_its_plugin_cannot_read_its_end() {
    const PLUGIN: &str = "test.off-at-the-end";
    crate::plugins::view::stand_in(PLUGIN, Box::new(|| Err(crate::plugins::view::off())));
    let mut a = agent(Some(PLUGIN));
    let mut fx = Effects::default();
    a.set_status(AgentStatus::Running, &mut fx);
    a.tui_error = Some("perdu".into());
    a.end_tui_turn(None, &mut fx);
    assert!(!a.meta.status.is_active(), "{:?}", a.meta.status);
    assert_eq!(fx.turn_end, Some(TurnEnd::Error("perdu".into())));
}

#[test]
fn another_conversation_does_not_take_up_the_question_of_the_last_one() {
    let mut a = agent(None);
    let mut fx = Effects::default();
    let asked = json!({
        crate::tui::EVENT_KEY: "permissionRequest", "sessionId": "s1", "toolName": "powershell",
        "toolInput": { "command": "ls" },
    });
    a.copilot_hook(&asked, &mut fx);
    assert!(a.tui_asked.is_some());
    // `/new`: its own session, and what it waits for is its own.
    let waits = json!({
        crate::tui::EVENT_KEY: "notification", "sessionId": "s2",
        "notification_type": "permission_prompt", "message": "Lancer npm test ?",
    });
    a.copilot_hook(&waits, &mut fx);
    assert_eq!(a.tui_asked, None);
    assert_eq!(
        fx.notify.take().map(|n| n.body).as_deref(),
        Some("Lancer npm test ?")
    );
}

#[test]
fn a_plugin_that_never_had_a_view_gets_no_thread_when_it_stops() {
    const PLUGIN: &str = "test.never-a-view";
    crate::plugins::view::forget(PLUGIN);
    assert!(!crate::plugins::view::has_thread(PLUGIN));
}
