//! Antigravity CLI through the core (ESC-271): its accounts (the machine's Google sign-in, another by
//! its Gemini API key), its usage limit and the resume, the move to another account, the texts
//! written by `agy -p`. The fake's runs of its adapter are the bench (`engine/agy/bench.rs`),
//! `agent/agy_tests.rs` and the e2e (`tests/e2e/agy.spec.ts`).

use crate::core::AgentOptions;
use crate::core_tests::{harness, Harness};
use crate::model::*;
use serde_json::Value;
use std::time::Duration;

fn texts(h: &Harness, id: &str) -> Vec<String> {
    h.items(id)
        .iter()
        .filter(|i| i["kind"] == "text" && i["parent"].is_null())
        .filter_map(|i| i["text"].as_str().map(str::to_string))
        .collect()
}

fn turns(h: &Harness, id: &str) -> Vec<Value> {
    h.items(id)
        .into_iter()
        .filter(|i| i["kind"] == "turn")
        .collect()
}

/// Antigravity's Principal (the machine's Google sign-in), on the fake.
fn machine_account(h: &Harness) {
    let program = crate::engine::fake_agy();
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "principal-agy".into(),
            name: "Principal".into(),
            engine: EngineId::AGY,
            claude_path: program,
            active: true,
            ..Default::default()
        });
    });
}

/// An Antigravity account added by its Gemini API key, on the fake, its key given; its id.
async fn key_account(h: &Harness, name: &str, key: Option<&str>) -> String {
    let made = h
        .core
        .create_account(EngineId::AGY, name, &[], crate::accounts::ShareMode::Link)
        .unwrap();
    let program = crate::engine::fake_agy();
    h.set_settings(|s| {
        if let Some(a) = s.accounts.iter_mut().find(|a| a.id == made.id) {
            a.claude_path = program.clone();
        }
    });
    if let Some(key) = key {
        h.core.set_account_key(&made.id, key).await.unwrap();
    }
    made.id
}

async fn agent_on(h: &Harness, project: &str, account: &str) -> String {
    h.core
        .create_agent_with(
            project,
            AgentOptions {
                engine: Some(EngineId::AGY),
                mode: Some("default".into()),
                account: Some(account.into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id
}

/// Each Antigravity account added by its key runs as that key (`GEMINI_API_KEY` and `modelProvider:
/// gemini` in its agent's settings), the machine's as its Google sign-in (no key, none of the
/// app's); the key is in the keychain, never in the settings' file nor on a command line; one
/// without its key starts nothing (it would run as the machine's), and its key goes with it.
#[tokio::test]
async fn antigravity_accounts_by_key_and_the_machines_never_mix() {
    let h = harness("esc271-keys");
    let (p, _) = h.project(false).await;
    machine_account(&h);
    let pro = key_account(&h, "Pro", Some("AIzaCleGeminiPro00000001")).await;
    let team = key_account(&h, "Équipe", Some("AIzaCleGeminiEquipe0002")).await;
    let none = key_account(&h, "Sans clé", None).await;
    let agents = [
        (agent_on(&h, &p.id, &pro).await, "Compte : clé …0001"),
        (agent_on(&h, &p.id, &team).await, "Compte : clé …0002"),
        (
            agent_on(&h, &p.id, "principal-agy").await,
            "Compte : Google de la machine",
        ),
    ];
    for (id, said) in &agents {
        h.turn(id, "compte").await;
        assert!(texts(&h, id).iter().any(|t| t == said), "{said}");
    }
    let keychain = crate::integrations::secrets::memory_of(&h.core.data);
    assert_eq!(
        keychain.entry(&format!("agy:{pro}")).as_deref(),
        Some("AIzaCleGeminiPro00000001")
    );
    let saved = std::fs::read_to_string(h.core.data.settings_file()).unwrap_or_default();
    assert!(!saved.contains("AIzaCle"), "{saved}");
    let status = h.core.claude_account_status(&pro).await.unwrap();
    assert!(status.connected);
    assert_eq!(status.email.as_deref(), Some("clé …0001"));
    assert!(!h.core.claude_account_status(&none).await.unwrap().connected);
    // The machine's: as far as can be told in the tests (its keychain never looked at).
    assert!(
        h.core
            .claude_account_status("principal-agy")
            .await
            .unwrap()
            .connected
    );
    // Without its key: refused, saying where to give it.
    let lost = agent_on(&h, &p.id, &none).await;
    let said = match h.core.send_message(&lost, "compte".into(), vec![]).await {
        Err(e) => format!("{e:#}"),
        Ok(_) => {
            h.wait("the refusal", |h| {
                h.items(&lost)
                    .iter()
                    .any(|i| i.to_string().contains("clé d’API"))
            })
            .await;
            String::from("Antigravity clé d’API")
        }
    };
    assert!(
        said.contains("clé d’API") && said.contains("Antigravity"),
        "{said}"
    );
    assert!(!texts(&h, &lost).iter().any(|t| t.contains("machine")));
    // A key that is none, or one given to the machine's account: refused.
    let short = h.core.set_account_key(&pro, "trop courte").await;
    assert!(format!("{:#}", short.unwrap_err()).contains("Gemini"));
    assert!(h
        .core
        .set_account_key("principal-agy", "AIzaCleGemini0000000000")
        .await
        .is_err());
    // Removed, its key leaves the keychain.
    h.core.delete_agent(&agents[1].0, false).await.unwrap();
    h.core.remove_claude_account(&team).unwrap();
    assert_eq!(keychain.entry(&format!("agy:{team}")), None);
    h.core.stop_for_good();
}

/// The machine's account is added (ESC-128: never by itself) before it is signed in, its sign-in
/// then made by `agy` itself in a terminal; its new agents go with its defaults.
#[tokio::test]
async fn the_machines_antigravity_account_is_added_by_hand_and_its_defaults_hold() {
    let h = harness("esc271-machine");
    let (p, _) = h.project(false).await;
    assert!(!crate::accounts::has_any(
        &h.core.settings.read(),
        EngineId::AGY
    ));
    let program = crate::engine::fake_agy();
    let machine = h.core.machine_account(EngineId::AGY).await;
    assert!(!machine.added && machine.signs_in && machine.signs_in_later);
    let made = h.core.add_machine_account(EngineId::AGY).unwrap();
    assert_eq!(made.id, "principal-agy");
    assert!(h.core.machine_account(EngineId::AGY).await.added);
    // Its sign-in: `agy` itself, its interface (no argument), never a key of the app's.
    assert!(crate::accounts::store(EngineId::AGY).login.is_empty());
    let env = crate::accounts::launch_env_with(&made, &|v| v == "GEMINI_API_KEY");
    assert!(
        env.contains(&("GEMINI_API_KEY".into(), String::new())),
        "{env:?}"
    );
    // Its defaults, as Réglages › Comptes sets them.
    let mut a = made.clone();
    a.claude_path = program;
    a.default_model = "gemini-3.8-flash".into();
    a.default_effort = "high".into();
    a.default_mode = "acceptEdits".into();
    h.core.update_claude_account(a).unwrap();
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::AGY),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    h.turn(&id, "modèle").await;
    assert!(
        texts(&h, &id)
            .iter()
            .any(|t| t == "Modèle : gemini-3.8-flash, effort : high, mode : accept-edits"),
        "{:?}",
        texts(&h, &id)
    );
    // Removed like any: none left, no agent of Antigravity starts.
    h.core.delete_agent(&id, false).await.unwrap();
    h.core.remove_claude_account("principal-agy").unwrap();
    assert!(!crate::accounts::has_any(
        &h.core.settings.read(),
        EngineId::AGY
    ));
    h.core.stop_for_good();
}

/// Its quota met ends the turn « limité », its resume planned an hour later (`agy` tells no reset);
/// the agent then goes on by itself, on its account.
#[tokio::test]
async fn the_quota_met_ends_the_turn_limited_and_the_agent_goes_on_by_itself() {
    let h = harness("esc271-limit");
    let (p, _) = h.project(false).await;
    machine_account(&h);
    let id = agent_on(&h, &p.id, "principal-agy").await;
    let before = now_ms();
    h.turn(&id, "limite").await;
    let turn = turns(&h, &id).pop().unwrap();
    assert_eq!(turn["limited"], true, "{turn}");
    assert!(
        turn["error"]
            .as_str()
            .unwrap()
            .contains("Limite d’usage d’Antigravity atteinte : You have exhausted your quota"),
        "{turn}"
    );
    // Planned once the turn's end is applied (just after the agent is seen idle).
    h.wait("its resume planned", |h| h.agent(&id).resume_at.is_some())
        .await;
    let at = h.agent(&id).resume_at.unwrap();
    let margin = crate::core::RESUME_MARGIN_MS;
    let hour = crate::engine::agy::QUOTA_RESUME_MS;
    assert!(
        at >= before + hour + margin && at <= now_ms() + hour + 60_000,
        "{at}"
    );
    h.core.resume_due().await;
    assert!(h.agent(&id).resume_at.is_some(), "not due yet");
    h.core.agent(&id).unwrap().lock().meta.resume_at = Some(now_ms() - 1_000);
    h.core.resume_due().await;
    h.wait("it goes on", |h| {
        texts(h, &id)
            .iter()
            .any(|t| t.starts_with("Antigravity a reçu : "))
    })
    .await;
    assert_eq!(h.agent(&id).resume_at, None);
    // The model's capacity: tried again as it says (30 s).
    let before = now_ms();
    h.turn(&id, "capacité").await;
    assert_eq!(turns(&h, &id).pop().unwrap()["limited"], true);
    // Planned once the turn's end is applied (just after the agent is seen idle).
    h.wait("its resume planned", |h| h.agent(&id).resume_at.is_some())
        .await;
    let at = h.agent(&id).resume_at.unwrap();
    assert!(
        at >= before + 30_000 + margin && at <= now_ms() + 30_000 + margin,
        "{at}"
    );
    h.core.stop_for_good();
}

/// « Reprendre sur <compte> » (and the move at the limit, « Changer de compte à »): the agent's
/// conversation stays in its own home, only its key changes; nothing to copy.
#[tokio::test]
async fn an_antigravity_agent_goes_on_on_another_account_with_its_conversation() {
    let h = harness("esc271-move");
    let (p, _) = h.project(false).await;
    machine_account(&h);
    let pro = key_account(&h, "Pro", Some("AIzaCleGeminiPro00000001")).await;
    let id = agent_on(&h, &p.id, "principal-agy").await;
    h.turn(&id, "compte").await;
    let session = h.agent(&id).session_id.clone().expect("its conversation");
    h.core.resume_on_account(&id, &pro).await.unwrap();
    h.wait("the resume's turn", |h| {
        turns(h, &id).len() >= 2 && !h.agent(&id).status.is_active()
    })
    .await;
    h.turn(&id, "compte").await;
    assert_eq!(h.agent(&id).account, pro);
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(session.as_str()));
    assert_eq!(texts(&h, &id).last().unwrap(), "Compte : clé …0001");
    h.core.stop_for_good();
}

/// The texts written can go to Antigravity (« Modèle des textes rédigés »): one `agy -p`, its
/// question on its input, every tool refused by its settings, in a home of its own removed once
/// answered, on the account's key; a key refused is an error, never an empty name.
#[tokio::test]
async fn the_texts_written_go_to_agy_p_without_a_tool() {
    let dir = crate::paths::test_dir("esc271-writer");
    std::fs::create_dir_all(&dir).unwrap();
    let log = dir.join("fake-agy.jsonl");
    let program =
        crate::engine::Engine::locate(&crate::engine::AGY, &crate::engine::fake_agy(), "").unwrap();
    let ask = |key: &str, prompt: &'static str| crate::engine::OneShot {
        program: program.clone(),
        env: vec![
            ("FAKE_AGY_LOG".into(), log.to_string_lossy().into_owned()),
            ("GEMINI_API_KEY".into(), key.into()),
        ],
        weight: crate::engine::Weight::Light,
        cwd: &dir,
        system: "Nomme l’agent.",
        prompt,
        limit: Duration::from_secs(30),
        model: String::new(),
    };
    let name = crate::engine::oneshot::ask(
        &crate::engine::AGY,
        ask("AIzaCleGeminiEcrit00009", "<tache>Corriger la TVA</tache>"),
    )
    .await
    .unwrap();
    assert_eq!(name, "corriger-la-agy");
    let said = std::fs::read_to_string(&log).unwrap();
    let launch: Value = said
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .find(|v| v["launch"].is_object())
        .unwrap()["launch"]
        .clone();
    let argv: Vec<&str> = launch["argv"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(argv[0], "-p=");
    assert!(argv
        .windows(2)
        .any(|w| w == ["--model", "gemini-3.8-flash-low"]));
    // No tool: all refused by its settings.
    let deny: Vec<&str> = launch["deny"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    for rule in ["command(*)", "write_file(*)", "read_file(*)", "mcp(*)"] {
        assert!(deny.contains(&rule), "{deny:?}");
    }
    // Its key by its environment and its settings, never its arguments.
    assert_eq!(launch["geminiKey"], "AIzaCleGeminiEcrit00009");
    assert_eq!(launch["provider"], "gemini");
    assert!(!argv.iter().any(|a| a.contains("AIzaCle")));
    let home = launch["home"].as_str().unwrap();
    assert!(!std::path::Path::new(home).exists(), "removed: {home}");
    // A key Antigravity refuses: an error.
    let refused = crate::engine::oneshot::ask(
        &crate::engine::AGY,
        ask("cle-refusee-00000000001", "un message de commit"),
    )
    .await;
    assert!(refused.is_err());
}

/// Real run (ESC-271), by hand: a text written by the real Antigravity CLI (`ESCOUADE_TEST_REAL_AGY`,
/// `%LOCALAPPDATA%\agy\bin\agy.exe`), on the machine's Google sign-in, every tool refused:
/// `cargo test --lib real_agy_writes -- --ignored --nocapture`. One short question (Gemini Flash
/// Low, about 0.2 % of its weekly quota), in a home of its own: it prints the answer and its time.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "the real Antigravity CLI: run by hand"]
async fn real_agy_writes_a_text() {
    let real = std::env::var("ESCOUADE_TEST_REAL_AGY").expect("ESCOUADE_TEST_REAL_AGY");
    let program = crate::engine::Engine::locate(&crate::engine::AGY, &real, "").unwrap();
    let dir = crate::paths::test_dir("esc271-real-writer");
    std::fs::create_dir_all(&dir).unwrap();
    let started = std::time::Instant::now();
    let answer = crate::engine::oneshot::ask(
        &crate::engine::AGY,
        crate::engine::OneShot {
            program,
            // Its Principal's: no key of the machine's.
            env: vec![
                ("GEMINI_API_KEY".into(), String::new()),
                ("GOOGLE_API_KEY".into(), String::new()),
            ],
            weight: crate::engine::Weight::Light,
            cwd: &dir,
            system: "Tu nommes des agents de code. Réponds par le nom seul, trois mots au plus, sans ponctuation.",
            prompt: "Corriger le calcul de la TVA dans la facture",
            limit: Duration::from_secs(90),
            model: String::new(),
        },
    )
    .await
    .unwrap();
    println!("answer in {:?}: {answer}", started.elapsed());
    assert!(!answer.trim().is_empty() && answer.len() < 200, "{answer}");
}
