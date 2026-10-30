//! Mistral Vibe through the core (ESC-267): its accounts by API key, what a turn costs, its rate
//! limit and the resume, the texts written by `vibe -p`. The fake's runs of its adapter are the
//! bench (`engine/vibe/bench.rs`), `agent/vibe_tests.rs` and the e2e (`tests/e2e/vibe.spec.ts`).

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

/// Vibe's Principal (the machine's key), on the fake.
fn machine_account(h: &Harness) {
    let program = crate::engine::fake_vibe();
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "principal-vibe".into(),
            name: "Principal".into(),
            engine: EngineId::VIBE,
            claude_path: program,
            active: true,
            ..Default::default()
        });
    });
}

/// A Vibe account added by its API key, on the fake, its key given; its id.
async fn key_account(h: &Harness, name: &str, key: Option<&str>) -> String {
    let made = h
        .core
        .create_account(EngineId::VIBE, name, &[], crate::accounts::ShareMode::Link)
        .unwrap();
    let program = crate::engine::fake_vibe();
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
                engine: Some(EngineId::VIBE),
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

/// Each Vibe account added by its key runs as that key (`MISTRAL_API_KEY`), the machine's as Vibe
/// goes in a terminal; the key is in the keychain, never in the settings' file; one without its
/// key starts nothing (it would run as the machine's), and its key goes with it.
#[tokio::test]
async fn two_vibe_accounts_by_key_and_the_machines_never_mix() {
    let h = harness("esc267-keys");
    let (p, _) = h.project(false).await;
    machine_account(&h);
    let pro = key_account(&h, "Pro", Some("cle-mistral-pro-00000001")).await;
    let team = key_account(&h, "Équipe", Some("cle-mistral-equipe-0002")).await;
    let none = key_account(&h, "Sans clé", None).await;
    let agents = [
        (agent_on(&h, &p.id, &pro).await, "Compte : clé …0001"),
        (agent_on(&h, &p.id, &team).await, "Compte : clé …0002"),
        (
            agent_on(&h, &p.id, "principal-vibe").await,
            "Compte : clé de la machine",
        ),
    ];
    for (id, said) in &agents {
        h.turn(id, "compte").await;
        assert!(texts(&h, id).iter().any(|t| t == said), "{said}");
    }
    let keychain = crate::integrations::secrets::memory_of(&h.core.data);
    assert_eq!(
        keychain.entry(&format!("vibe:{pro}")).as_deref(),
        Some("cle-mistral-pro-00000001")
    );
    let saved = std::fs::read_to_string(h.core.data.settings_file()).unwrap_or_default();
    assert!(!saved.contains("cle-mistral"), "{saved}");
    let status = h.core.claude_account_status(&pro).await.unwrap();
    assert!(status.connected);
    assert_eq!(status.email.as_deref(), Some("clé …0001"));
    assert!(!h.core.claude_account_status(&none).await.unwrap().connected);
    assert!(
        h.core
            .claude_account_status("principal-vibe")
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
            String::from("Mistral Vibe clé d’API")
        }
    };
    assert!(
        said.contains("clé d’API") && said.contains("Mistral Vibe"),
        "{said}"
    );
    assert!(!texts(&h, &lost).iter().any(|t| t.contains("machine")));
    // A key that is none, or one given to the machine's account: refused.
    let short = h.core.set_account_key(&pro, "trop courte").await;
    assert!(format!("{:#}", short.unwrap_err()).contains("Mistral"));
    assert!(h
        .core
        .set_account_key("principal-vibe", "cle-mistral-0000000000000")
        .await
        .is_err());
    // Removed, its key leaves the keychain.
    h.core.delete_agent(&agents[1].0, false).await.unwrap();
    h.core.remove_claude_account(&team).unwrap();
    assert_eq!(keychain.entry(&format!("vibe:{team}")), None);
    h.core.stop_for_good();
}

/// A turn costs what its session's cost grew by (Vibe tells the session's so far, and a session
/// loaded again says what it spent before): its card, the agent, its account's day and the stats.
#[tokio::test]
async fn a_vibe_turn_costs_what_its_session_grew_by() {
    let h = harness("esc267-cost");
    let (p, _) = h.project(false).await;
    machine_account(&h);
    let id = agent_on(&h, &p.id, "principal-vibe").await;
    h.turn(&id, "bonjour").await;
    h.turn(&id, "encore").await;
    // A new process on the same session (`session/load`): its first usage is what it spent before.
    let old = h.core.agent(&id).unwrap().lock().detach().unwrap();
    old.close_input();
    assert!(old.wait_exit(Duration::from_secs(10)).await);
    h.turn(&id, "repris").await;
    assert!(texts(&h, &id).iter().any(|t| t == "Vibe a reçu : repris"));
    let all = turns(&h, &id);
    assert_eq!(all.len(), 3);
    for t in &all {
        assert!((t["cost"].as_f64().unwrap() - 0.0123).abs() < 1e-9, "{t}");
        assert_eq!(t["tokens"], 9393 + 73, "{t}");
        assert_eq!(t["engine"], "vibe");
    }
    let a = h.agent(&id);
    assert!((a.cost - 3.0 * 0.0123).abs() < 1e-9, "{}", a.cost);
    let usage = h.core.usage.lock().clone();
    let day = usage.account("principal-vibe").unwrap().today_cost;
    assert!((day - 3.0 * 0.0123).abs() < 1e-9, "{day}");
    let stats = h.core.stats_view_by("7d", None, Some("vibe"));
    assert!((stats.cost - 3.0 * 0.0123).abs() < 1e-9);
    assert_eq!(stats.tokens, 3 * (9393 + 73));
    h.core.stop_for_good();
}

/// Vibe's rate limit ends the turn « limité », its resume planned a minute later; it then goes on
/// by itself, on its account.
#[tokio::test]
async fn the_rate_limit_ends_the_turn_limited_and_the_agent_goes_on_by_itself() {
    let h = harness("esc267-limit");
    let (p, _) = h.project(false).await;
    machine_account(&h);
    let id = agent_on(&h, &p.id, "principal-vibe").await;
    let before = now_ms();
    h.turn(&id, "limite").await;
    let turn = turns(&h, &id).pop().unwrap();
    assert_eq!(turn["limited"], true, "{turn}");
    assert!(
        turn["error"]
            .as_str()
            .unwrap()
            .contains("Limite de débit de Mistral"),
        "{turn}"
    );
    let at = h.agent(&id).resume_at.expect("its resume planned");
    let margin = crate::core::RESUME_MARGIN_MS;
    assert!(
        at >= before + 60_000 + margin && at <= now_ms() + 120_000,
        "{at}"
    );
    h.core.resume_due().await;
    assert!(h.agent(&id).resume_at.is_some(), "not due yet");
    h.core.agent(&id).unwrap().lock().meta.resume_at = Some(now_ms() - 1_000);
    h.core.resume_due().await;
    h.wait("it goes on", |h| {
        texts(h, &id)
            .iter()
            .any(|t| t.starts_with("Vibe a reçu : "))
    })
    .await;
    assert_eq!(h.agent(&id).resume_at, None);
    h.core.stop_for_good();
}

/// « Reprendre sur <compte> » (and the move at the limit): the agent's session stays in its own
/// `VIBE_HOME`, only its key changes; nothing to copy.
#[tokio::test]
async fn a_vibe_agent_goes_on_on_another_account_with_its_session() {
    let h = harness("esc267-move");
    let (p, _) = h.project(false).await;
    machine_account(&h);
    let pro = key_account(&h, "Pro", Some("cle-mistral-pro-00000001")).await;
    let team = key_account(&h, "Équipe", Some("cle-mistral-equipe-0002")).await;
    let id = agent_on(&h, &p.id, &pro).await;
    h.turn(&id, "compte").await;
    let session = h.agent(&id).session_id.clone().expect("its session");
    h.core.resume_on_account(&id, &team).await.unwrap();
    h.wait("the resume's turn", |h| {
        turns(h, &id).len() >= 2 && !h.agent(&id).status.is_active()
    })
    .await;
    h.turn(&id, "compte").await;
    assert_eq!(h.agent(&id).account, team);
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(session.as_str()));
    assert_eq!(texts(&h, &id).last().unwrap(), "Compte : clé …0002");
    h.core.stop_for_good();
}

/// The texts written can go to Mistral Vibe (« Modèle des textes rédigés »): one `vibe -p`, its
/// question on its input, no tool, one turn within a price, in a `VIBE_HOME` of its own removed
/// once answered, on the account's key; a key refused is an error, never an empty name.
#[tokio::test]
async fn the_texts_written_go_to_vibe_p_without_a_tool_within_a_price() {
    let dir = crate::paths::test_dir("esc267-writer");
    std::fs::create_dir_all(&dir).unwrap();
    let log = dir.join("fake-vibe.jsonl");
    let program =
        crate::engine::Engine::locate(&crate::engine::VIBE, &crate::engine::fake_vibe(), "")
            .unwrap();
    let ask = |key: &str, prompt: &'static str| crate::engine::OneShot {
        program: program.clone(),
        env: vec![
            ("FAKE_VIBE_LOG".into(), log.to_string_lossy().into_owned()),
            ("MISTRAL_API_KEY".into(), key.into()),
        ],
        weight: crate::engine::Weight::Light,
        cwd: &dir,
        system: "Nomme l’agent.",
        prompt,
        limit: Duration::from_secs(30),
        model: String::new(),
    };
    let name = crate::engine::oneshot::ask(
        &crate::engine::VIBE,
        ask("cle-mistral-ecrit-0009", "<tache>Corriger la TVA</tache>"),
    )
    .await
    .unwrap();
    assert_eq!(name, "corriger-la-vibe");
    let said = std::fs::read_to_string(&log).unwrap();
    let call: Value = said
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .find(|v| v["print"].is_object())
        .unwrap()["print"]
        .clone();
    let argv: Vec<&str> = call["argv"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(argv, crate::engine::vibe::print_args());
    assert!(argv.windows(2).any(|w| w == ["--enabled-tools", "none"]));
    assert!(argv.windows(2).any(|w| w[0] == "--max-price"));
    // Its key by its environment, never its arguments; its question by its input.
    assert_eq!(call["apiKey"], "cle-mistral-ecrit-0009");
    assert!(!said.contains("\"argv\":[\"cle"));
    assert!(call["question"]
        .as_str()
        .unwrap()
        .starts_with("<instructions>\nNomme l’agent.\n</instructions>"));
    let home = call["home"].as_str().unwrap();
    assert!(!std::path::Path::new(home).exists(), "removed: {home}");
    assert!(!argv.iter().any(|a| a.contains("cle-mistral")));
    // A key Vibe refuses: an error.
    let refused = crate::engine::oneshot::ask(
        &crate::engine::VIBE,
        ask("cle-refusee-00000000001", "un message de commit"),
    )
    .await;
    assert!(refused.is_err());
}

/// Real run (ESC-267), by hand: a text written by the real Mistral Vibe (`ESCOUADE_TEST_REAL_VIBE`,
/// the Windows path of `~/.local/bin/vibe-acp.exe`; its `vibe.exe` beside it answers), on the
/// machine's key: `cargo test --lib real_vibe_writes -- --ignored --nocapture`. One short question
/// (about 0.01 $), in a `VIBE_HOME` of its own: it prints the answer and its time.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "the real Mistral Vibe: run by hand"]
async fn real_vibe_writes_a_text() {
    let real = std::env::var("ESCOUADE_TEST_REAL_VIBE").expect("ESCOUADE_TEST_REAL_VIBE");
    let program = crate::engine::Engine::locate(&crate::engine::VIBE, &real, "").unwrap();
    let dir = crate::paths::test_dir("esc267-real-writer");
    std::fs::create_dir_all(&dir).unwrap();
    let started = std::time::Instant::now();
    let answer = crate::engine::oneshot::ask(
        &crate::engine::VIBE,
        crate::engine::OneShot {
            program,
            env: Vec::new(),
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
