//! Tests of what a plugin of a third party may do without a review (audit SEC-01, ESC-589): « N'installer que des
//! versions vérifiées » on a folder, off and on, the folder in development apart; the setting turned on at the update of
//! an installation without a plugin of a third party, and kept with one; the harness a third party describes asking at
//! its first launch, its whole command said, kept by its fingerprint, asked again when its line changes, refused, and
//! refused by « Harness des plugins tiers : Refuser ».

use crate::core::{AgentOptions, Core};
use crate::core_tests::{harness, Harness};
use crate::i18n::Lang;
use crate::model::*;
use crate::paths::{test_dir, DataDir};
use crate::third_party::LaunchHeld;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tauri::test::mock_app;

const OWNER: &str = "dev.example.fake-acp";

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
}

/// The fake ACP agent's program: its `.cmd` under Windows, its shell script elsewhere.
fn fake_acp_path() -> String {
    let name = if cfg!(windows) {
        "fake-acp.cmd"
    } else {
        "fake-acp"
    };
    fixtures()
        .join("acp-bin")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

/// The manifest of the fake ACP agent's plugin (`tests/fixtures/plugins/fake-acp`), changed by `change`, in a folder of
/// its own.
fn fake_acp_folder(name: &str, change: impl FnOnce(&mut Value)) -> PathBuf {
    let text = std::fs::read_to_string(
        fixtures()
            .join("plugins")
            .join("fake-acp")
            .join("escouade-plugin.json"),
    )
    .unwrap();
    let mut m: Value = serde_json::from_str(&text).unwrap();
    change(&mut m);
    let dir = test_dir(name).join("fake-acp");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("escouade-plugin.json"),
        serde_json::to_string_pretty(&m).unwrap(),
    )
    .unwrap();
    dir
}

/// The fake ACP agent's plugin installed from a folder (a third party's), its harness in the registry, an account of
/// it on the fake program: its engine.
fn install_fake_acp(h: &Harness, name: &str, change: impl FnOnce(&mut Value)) -> EngineId {
    let dir = fake_acp_folder(name, change);
    h.core.plugins.install_folder(&dir, Lang::Fr).unwrap();
    h.core.sync_harnesses();
    let engine = crate::plugins::harness::described_id(OWNER, "fake-acp");
    let mut s = h.core.settings.read().clone();
    if !s.accounts.iter().any(|a| a.engine == engine) {
        let mut a = crate::accounts::principal_of(engine);
        a.claude_path = fake_acp_path();
        s.accounts.push(a);
        h.core.save_settings(s).unwrap();
    }
    engine
}

async fn agent_on(h: &Harness, project: &str, engine: EngineId) -> String {
    h.core
        .create_agent_with(
            project,
            AgentOptions {
                engine: Some(engine),
                select: true,
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id
}

/// Why the agent `id` does not start.
async fn refusal_of(h: &Harness, id: &str) -> String {
    match h.core.ensure_process(id).await {
        Ok(_) => panic!("the agent started"),
        Err(e) => e.to_string(),
    }
}

fn notices(h: &Harness, id: &str) -> Vec<String> {
    h.core
        .agent(id)
        .unwrap()
        .lock()
        .conv
        .items()
        .into_iter()
        .filter(|i| i["kind"] == "notice")
        .filter_map(|i| i["text"].as_str().map(str::to_string))
        .collect()
}

/// A third party's harness asks at its first launch: its agent does not start, said; the window reads its whole command
/// (the program found, its arguments, its variables) and whether it waits; « Autoriser » on a line read before it
/// changed is refused; agreed, the line is kept by its fingerprint and the agent starts, and the next start asks nothing.
#[tokio::test(flavor = "multi_thread")]
async fn a_third_party_harness_asks_at_its_first_launch_then_starts_and_asks_no_more() {
    let h = harness("sec01-first");
    let engine = install_fake_acp(&h, "sec01-first-plugin", |m| {
        m["contributes"]["harnesses"][0]["acp"]["env"] = json!({ "FAKE_ACP_COLOR": "bleu" });
    });
    let (p, _) = h.project(false).await;
    let id = agent_on(&h, &p.id, engine).await;
    let refused = refusal_of(&h, &id).await;
    assert!(refused.contains("attend ton accord"), "{refused}");
    assert!(!h.alive(&id));
    assert_eq!(
        h.core.harness_launches().get(&engine),
        Some(&LaunchHeld::Asks)
    );
    let ask = h.core.launch_ask(engine, Some(&id)).unwrap().unwrap();
    assert_eq!(ask.held, Some(LaunchHeld::Asks));
    assert_eq!(ask.plugin_name, "Faux ACP");
    assert_eq!(ask.line.plugin, OWNER);
    assert!(
        Path::new(&ask.line.program).ends_with(Path::new(&fake_acp_path()).file_name().unwrap()),
        "{}",
        ask.line.program
    );
    assert_eq!(ask.line.args, ["acp"]);
    assert_eq!(ask.line.env.len(), 1);
    assert_eq!(ask.line.env[0].name, "FAKE_ACP_COLOR");
    assert_eq!(ask.line.escouade_env, ["FAKE_ACP_HOME"]);
    // Read before it changed: refused, to be read again.
    let stale = h
        .core
        .agree_to_harness(engine, Some(&id), "0000", true)
        .unwrap_err()
        .to_string();
    assert!(stale.contains("a changé"), "{stale}");
    // Agreed: kept in the plugin's choices, the agent starts.
    h.core
        .agree_to_harness(engine, Some(&id), &ask.line.fingerprint, true)
        .unwrap();
    let kept = h.core.plugins.choices(OWNER).unwrap().harnesses_approved;
    assert_eq!(
        kept["fake-acp"],
        std::slice::from_ref(&ask.line.fingerprint)
    );
    assert!(h.core.harness_launches().is_empty());
    let after = h.core.launch_ask(engine, Some(&id)).unwrap().unwrap();
    assert_eq!(after.held, None);
    h.core.ensure_process(&id).await.unwrap();
    assert!(h.alive(&id));
    // A second agent of the same harness: nothing asked.
    let second = agent_on(&h, &p.id, engine).await;
    h.core.ensure_process(&second).await.unwrap();
    assert!(h.alive(&second));
}

/// Stops the process of the agent `id`, as it runs.
fn stop(h: &Harness, id: &str) {
    if let Some(proc) = h.core.agent(id).unwrap().lock().detach() {
        proc.kill();
    }
}

/// An update of the plugin that changes its harness's line (an argument added) asks again; refused, nothing is kept and
/// the conversation says the agent does not start. An agent alive at the update keeps its process, on the line it was
/// started on, until it stops: its next start asks (ESC-697). The warm-up « Autoriser » starts is waited for, and its
/// process stopped after the update: nothing here depends on when the warm-up comes.
#[tokio::test(flavor = "multi_thread")]
async fn a_line_changed_by_an_update_asks_again_and_a_refusal_is_said() {
    let h = harness("sec01-update");
    let engine = install_fake_acp(&h, "sec01-update-v1", |_| {});
    let (p, _) = h.project(false).await;
    let id = agent_on(&h, &p.id, engine).await;
    let first = h.core.launch_ask(engine, Some(&id)).unwrap().unwrap();
    h.core
        .agree_to_harness(engine, Some(&id), &first.line.fingerprint, true)
        .unwrap();
    assert_eq!(
        h.core.launch_ask(engine, Some(&id)).unwrap().unwrap().held,
        None
    );
    // The warm-up « Autoriser » starts, waited for: alive, by it or by this start, whichever takes the lock first.
    h.core.ensure_process(&id).await.unwrap();
    assert!(h.alive(&id));
    // The update: a new version whose line says one more argument.
    install_fake_acp(&h, "sec01-update-v2", |m| {
        m["version"] = json!("1.1.0");
        m["contributes"]["harnesses"][0]["acp"]["args"] = json!(["acp", "--plus"]);
    });
    let next = h.core.launch_ask(engine, Some(&id)).unwrap().unwrap();
    assert_ne!(next.line.fingerprint, first.line.fingerprint);
    assert_eq!(next.line.args, ["acp", "--plus"]);
    assert_eq!(next.held, Some(LaunchHeld::Asks));
    // Alive at the update: its process is not stopped by it, and a start finds it running.
    assert!(h.alive(&id));
    h.core.ensure_process(&id).await.unwrap();
    // Stopped, it does not start again on the new line without the user's agreement.
    stop(&h, &id);
    let refused = refusal_of(&h, &id).await;
    assert!(refused.contains("attend ton accord"), "{refused}");
    // Refused by the user: nothing kept, said in its conversation, and it still does not start.
    h.core
        .agree_to_harness(engine, Some(&id), &next.line.fingerprint, false)
        .unwrap();
    assert!(
        notices(&h, &id)
            .iter()
            .any(|n| n.contains("Tu as refusé de lancer Faux ACP")),
        "{:?}",
        notices(&h, &id)
    );
    assert_eq!(
        h.core.launch_ask(engine, Some(&id)).unwrap().unwrap().held,
        Some(LaunchHeld::Asks)
    );
    assert!(h.core.ensure_process(&id).await.is_err());
    assert!(!h.alive(&id));
}

/// The race of ESC-697: a start begun before an update of the plugin holds the harness it found then, whose line (the
/// old one) stays among those kept. Ended after the update, it does not run that line: the line of the plugin installed
/// now is read again as its program is about to run, and a line changed asks the user's agreement again.
#[tokio::test(flavor = "multi_thread")]
async fn a_start_begun_before_an_update_does_not_run_the_old_line_after_it() {
    let h = harness("sec01-race");
    let engine = install_fake_acp(&h, "sec01-race-v1", |_| {});
    let (p, _) = h.project(false).await;
    let id = agent_on(&h, &p.id, engine).await;
    let first = h.core.launch_ask(engine, Some(&id)).unwrap().unwrap();
    h.core
        .agree_to_harness(engine, Some(&id), &first.line.fingerprint, true)
        .unwrap();
    // Its warm-up out of the way: started, then stopped.
    h.core.ensure_process(&id).await.unwrap();
    stop(&h, &id);
    // The start begun before the update: the harness of then, its account's program.
    let before = crate::engine::by_id(engine);
    let settings = h.core.settings.read().clone();
    let account = settings
        .accounts
        .iter()
        .find(|a| a.engine == engine)
        .unwrap()
        .clone();
    let program = crate::accounts::program(&account, &settings).unwrap();
    let meta = h.core.agent(&id).unwrap().lock().meta.clone();
    // The update meanwhile, its line changed.
    install_fake_acp(&h, "sec01-race-v2", |m| {
        m["version"] = json!("1.1.0");
        m["contributes"]["harnesses"][0]["acp"]["args"] = json!(["acp", "--plus"]);
    });
    assert!(!std::ptr::addr_eq(before, crate::engine::by_id(engine)));
    // The start ends after it.
    let (tx, rx) = tokio::sync::oneshot::channel();
    let spawn = crate::engine::Spawn::of(&meta, program, crate::accounts::launch_env(&account));
    let proc = before
        .spawn(
            spawn,
            Box::new(|_| {}),
            Box::new(move |_, why| {
                let _ = tx.send(why);
            }),
        )
        .unwrap_or_else(|e| panic!("refused before its launch, not where the race is: {e:#}"));
    match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
        Ok(why) => {
            let why = why.unwrap();
            assert!(why.contains("attend ton accord"), "{why}");
        }
        Err(_) => {
            proc.kill();
            panic!("the old line ran after the update");
        }
    }
}

/// « Harness des plugins tiers : Refuser »: none runs, agreed or not, and « Autoriser » is refused; the harnesses of
/// Escouade's own hand ask nothing.
#[tokio::test(flavor = "multi_thread")]
async fn refuse_runs_no_third_party_harness_and_the_first_hand_asks_nothing() {
    let h = harness("sec01-refuse");
    let engine = install_fake_acp(&h, "sec01-refuse-plugin", |_| {});
    let (p, _) = h.project(false).await;
    let id = agent_on(&h, &p.id, engine).await;
    let ask = h.core.launch_ask(engine, Some(&id)).unwrap().unwrap();
    h.core
        .agree_to_harness(engine, Some(&id), &ask.line.fingerprint, true)
        .unwrap();
    let mut s = h.core.settings.read().clone();
    s.third_party_harnesses = ThirdPartyHarnesses::Refuse;
    h.core.save_settings(s).unwrap();
    let refused = refusal_of(&h, &id).await;
    assert!(refused.contains("« Refuser »"), "{refused}");
    assert_eq!(
        h.core.harness_launches().get(&engine),
        Some(&LaunchHeld::Refused)
    );
    assert!(h
        .core
        .agree_to_harness(engine, Some(&id), &ask.line.fingerprint, true)
        .is_err());
    // Claude Code, of Escouade's own hand: no line, no question.
    assert!(h.core.launch_ask(EngineId::CLAUDE, None).unwrap().is_none());
    assert!(!h.core.harness_launches().contains_key(&EngineId::CLAUDE));
}

/// The data of an installation whose settings were saved by a version before ESC-589 (`pluginsVerifiedOnly` off, not
/// settled), with the plugin of `third` installed when given.
fn before_589(name: &str, third: Option<&Path>) -> DataDir {
    let data = DataDir::new(test_dir(name).join("data"));
    data.ensure().unwrap();
    std::fs::write(
        data.settings_file(),
        serde_json::to_vec(&json!({ "sound": false, "pluginsVerifiedOnly": false })).unwrap(),
    )
    .unwrap();
    if let Some(dir) = third {
        let plugins = crate::plugins::Plugins::load(data.clone(), "2.2.0");
        plugins.install_folder(dir, Lang::Fr).unwrap();
    }
    data
}

fn verified_only_at_start(data: DataDir) -> (bool, bool) {
    let app = mock_app();
    let (core, _rx) = Core::<tauri::test::MockRuntime>::load(app.handle().clone(), data.clone());
    let s = core.settings.read().clone();
    let saved: Value =
        serde_json::from_slice(&std::fs::read(data.settings_file()).unwrap()).unwrap();
    assert_eq!(
        saved["pluginsVerifiedSettled"],
        json!(true),
        "written settled"
    );
    (s.plugins_verified_only, s.plugins_verified_settled)
}

/// At the update: « N'installer que des versions vérifiées » turned on for an installation with no plugin of a third
/// party; one that has one keeps its choice; once settled, the user's choice stays.
#[tokio::test(flavor = "multi_thread")]
async fn the_setting_turns_on_at_the_update_of_an_installation_without_a_third_party_plugin() {
    assert_eq!(
        verified_only_at_start(before_589("sec01-settle-none", None)),
        (true, true)
    );
    let third = fake_acp_folder("sec01-settle-third-plugin", |_| {});
    assert_eq!(
        verified_only_at_start(before_589("sec01-settle-third", Some(&third))),
        (false, true)
    );
    // Settled, then turned off by the user: off it stays.
    let data = DataDir::new(test_dir("sec01-settle-kept").join("data"));
    data.ensure().unwrap();
    std::fs::write(
        data.settings_file(),
        serde_json::to_vec(&json!({
            "sound": false, "pluginsVerifiedOnly": false, "pluginsVerifiedSettled": true
        }))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(verified_only_at_start(data), (false, true));
}

/// « N'installer que des versions vérifiées » on a folder: off, it is set aside as before; on, a folder the store does
/// not have verified is refused with why and the gesture that installs it anyway. A folder in development is another
/// gesture, which the setting does not hold.
#[tokio::test(flavor = "multi_thread")]
async fn a_folder_is_refused_with_the_setting_on_and_accepted_off_and_development_is_apart() {
    let h = harness("sec01-folder");
    let dir = fake_acp_folder("sec01-folder-plugin", |_| {});
    let mut s = h.core.settings.read().clone();
    s.plugins_verified_only = false;
    h.core.save_settings(s).unwrap();
    let p = h.core.plugins.inspect_folder(&dir, Lang::Fr).unwrap();
    let kept = h.core.held_to_store(p, Lang::Fr).await.unwrap();
    assert!(kept.matches_store.is_none());
    h.core.plugins.discard(&kept.token);

    let mut s = h.core.settings.read().clone();
    s.plugins_verified_only = true;
    h.core.save_settings(s).unwrap();
    let p = h.core.plugins.inspect_folder(&dir, Lang::Fr).unwrap();
    let token = p.token.clone();
    let e = h.core.held_to_store(p, Lang::Fr).await.unwrap_err();
    assert!(
        e.message.contains("Faux ACP v1.0.0 n'est pas installé"),
        "{}",
        e.message
    );
    assert!(e.message.contains("coupe-le"), "{}", e.message);
    assert!(h.core.plugins.staged(&token).is_none());
    // In development: read in place, the setting does not hold it.
    let dev = h.core.plugins.inspect_dev(&dir, Lang::Fr).unwrap();
    assert!(dev.matches_store.is_none());
    h.core.plugins.discard(&dev.token);
}
