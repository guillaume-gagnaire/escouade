//! Cursor CLI beside Claude Code, through the core (ESC-42): the real run of its adapter, by hand,
//! and what the core gives its agents. The fake's runs are the bench (`engine/cursor/bench.rs`) and
//! the e2e (`tests/e2e/cursor.spec.ts`).

use crate::core::AgentOptions;
use crate::core_tests::{harness, Harness};
use crate::model::*;
use serde_json::Value;
use std::time::Duration;

/// The card waiting of `kind` (`permission`, `question`) in the agent's conversation.
fn waiting(h: &Harness, id: &str, kind: &str) -> Option<Value> {
    h.items(id).into_iter().find(|i| {
        i["kind"] == kind
            && i["decision"].is_null()
            && i["answers"].is_null()
            && i["cancelled"] != true
    })
}

fn texts(h: &Harness, id: &str) -> Vec<String> {
    h.items(id)
        .iter()
        .filter(|i| i["kind"] == "text" && i["parent"].is_null())
        .filter_map(|i| i["text"].as_str().map(str::to_string))
        .collect()
}

async fn wait_long(h: &Harness, id: &str, what: &str, secs: u64, pred: impl Fn(&Harness) -> bool) {
    let deadline = std::time::Instant::now() + Duration::from_secs(secs);
    while std::time::Instant::now() < deadline {
        if pred(h) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    panic!("no {what}: {:#}", Value::Array(h.items(id)));
}

/// Cursor's Principal, on `program`.
fn cursor_account(h: &Harness, program: &str) {
    let program = program.to_string();
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "principal-cursor".into(),
            name: "Principal".into(),
            engine: EngineId::CURSOR,
            config_dir: String::new(),
            claude_path: program,
            active: true,
            ..Default::default()
        });
    });
}

async fn cursor_agent(h: &Harness, project: &str, model: &str) -> String {
    h.core
        .create_agent_with(
            project,
            AgentOptions {
                engine: Some(EngineId::CURSOR),
                model: Some(model.into()),
                mode: Some("default".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id
}

#[tokio::test]
async fn each_cursor_agent_runs_in_a_configuration_folder_of_its_own_removed_with_it() {
    let h = harness("esc42-agent-home");
    let (p, _) = h.project(false).await;
    cursor_account(&h, &crate::engine::fake_cursor());
    let a = cursor_agent(&h, &p.id, "default").await;
    let b = cursor_agent(&h, &p.id, "default").await;
    for id in [&a, &b] {
        h.core
            .send_message(id, "bonjour".into(), vec![])
            .await
            .unwrap();
    }
    let ended = |h: &Harness, id: &str| {
        h.items(id).iter().any(|i| i["kind"] == "turn") && !h.agent(id).status.is_active()
    };
    wait_long(&h, &a, "a's turn", 30, |h| ended(h, &a)).await;
    wait_long(&h, &b, "b's turn", 30, |h| ended(h, &b)).await;
    assert!(texts(&h, &a).iter().any(|t| t == "Cursor a reçu : bonjour"));
    // Each its own folder, its session kept there: never the user's ~/.cursor, nor the other's.
    let (home_a, home_b) = (h.core.data.agent_home(&a), h.core.data.agent_home(&b));
    assert_ne!(home_a, home_b);
    let session = h.agent(&a).session_id.unwrap();
    assert!(home_a
        .join("acp-sessions")
        .join(format!("{session}.json"))
        .is_file());
    assert!(!home_b
        .join("acp-sessions")
        .join(format!("{session}.json"))
        .exists());
    h.core.delete_agent(&a, false).await.unwrap();
    assert!(!home_a.exists(), "gone with its agent");
    assert!(home_b.exists());
}

/// A Cursor account added by its API key (ESC-43), on the fake, its key given; its id.
async fn key_account(h: &Harness, name: &str, key: Option<&str>) -> String {
    let made = h
        .core
        .create_account(
            EngineId::CURSOR,
            name,
            &[],
            crate::accounts::ShareMode::Link,
        )
        .unwrap();
    let program = crate::engine::fake_cursor();
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
                engine: Some(EngineId::CURSOR),
                model: Some("default".into()),
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

/// ESC-43: each Cursor account added by its key runs as that key, the machine's as its sign-in;
/// the key is in the keychain, never in the settings' file; one without its key starts nothing
/// (it would run as the machine's sign-in), and its key goes with it.
#[tokio::test]
async fn two_cursor_accounts_by_key_and_the_machines_never_mix() {
    let h = harness("esc43-keys");
    let (p, _) = h.project(false).await;
    cursor_account(&h, &crate::engine::fake_cursor());
    let pro = key_account(&h, "Pro", Some("cle-essai-pro-0000000001")).await;
    let team = key_account(&h, "Équipe", Some("cle-essai-equipe-000002")).await;
    let none = key_account(&h, "Sans clé", None).await;
    let agents = [
        (agent_on(&h, &p.id, &pro).await, "Compte : clé …0001"),
        (agent_on(&h, &p.id, &team).await, "Compte : clé …0002"),
        (
            agent_on(&h, &p.id, "principal-cursor").await,
            "Compte : connexion de la machine",
        ),
    ];
    for (id, _) in &agents {
        h.core
            .send_message(id, "compte".into(), vec![])
            .await
            .unwrap();
    }
    for (id, said) in &agents {
        wait_long(&h, id, said, 30, |h| texts(h, id).iter().any(|t| t == said)).await;
    }
    // The keychain holds the keys; the file of the settings none.
    let keychain = crate::integrations::secrets::memory_of(&h.core.data);
    assert_eq!(
        keychain.entry(&format!("cursor:{pro}")).as_deref(),
        Some("cle-essai-pro-0000000001")
    );
    let saved = std::fs::read_to_string(h.core.data.settings_file()).unwrap_or_default();
    assert!(!saved.contains("cle-essai"), "{saved}");
    // Its status says it by its end only.
    let status = h.core.claude_account_status(&pro).await.unwrap();
    assert!(status.connected);
    assert_eq!(status.email.as_deref(), Some("clé …0001"));
    assert!(!h.core.claude_account_status(&none).await.unwrap().connected);
    // Without its key: refused, saying where to give it.
    let lost = agent_on(&h, &p.id, &none).await;
    let refused = h.core.send_message(&lost, "compte".into(), vec![]).await;
    let said = match refused {
        Err(e) => format!("{e:#}"),
        Ok(_) => {
            wait_long(&h, &lost, "the refusal", 30, |h| {
                h.items(&lost)
                    .iter()
                    .any(|i| i.to_string().contains("clé d’API"))
            })
            .await;
            String::from("clé d’API")
        }
    };
    assert!(said.contains("clé d’API"), "{said}");
    assert!(!texts(&h, &lost)
        .iter()
        .any(|t| t.contains("connexion de la machine")));
    // A key that is none: refused, the one kept left as it was.
    assert!(h.core.set_account_key(&pro, "trop courte").await.is_err());
    assert!(h
        .core
        .set_account_key("principal-cursor", "cle-essai-0000000000000")
        .await
        .is_err());
    // Removed, its key leaves the keychain.
    h.core.delete_agent(&agents[1].0, false).await.unwrap();
    h.core.remove_claude_account(&team).unwrap();
    assert_eq!(keychain.entry(&format!("cursor:{team}")), None);
}

/// ESC-43: the key Cursor refuses ends the turn in error, saying to give it again; the limit of the
/// month ends it « limité » on the amber card, with no resume planned (no reset is known); a write a
/// hook refused fails, saying Cursor wrote nothing.
#[tokio::test]
async fn a_refused_key_the_limit_and_a_blocked_write_are_said_on_the_conversation() {
    let h = harness("esc43-refusals");
    let (p, _) = h.project(false).await;
    cursor_account(&h, &crate::engine::fake_cursor());
    let refused = key_account(&h, "Refusée", Some("cle-refusee-00000000001")).await;
    let a = agent_on(&h, &p.id, &refused).await;
    let b = agent_on(&h, &p.id, "principal-cursor").await;
    h.core
        .send_message(&a, "bonjour".into(), vec![])
        .await
        .unwrap();
    h.core
        .send_message(&b, "limite".into(), vec![])
        .await
        .unwrap();
    let turn = |h: &Harness, id: &str| h.items(id).into_iter().find(|i| i["kind"] == "turn");
    wait_long(&h, &a, "a's turn", 30, |h| turn(h, &a).is_some()).await;
    wait_long(&h, &b, "b's turn", 30, |h| turn(h, &b).is_some()).await;
    let ta = turn(&h, &a).unwrap();
    assert_eq!(ta["isError"], true);
    assert_eq!(ta["limited"], false);
    assert!(
        ta["error"].as_str().unwrap().contains("Changer la clé"),
        "{ta}"
    );
    let tb = turn(&h, &b).unwrap();
    assert_eq!(tb["limited"], true);
    assert!(
        tb["error"].as_str().unwrap().contains("Limite d’usage"),
        "{tb}"
    );
    assert_eq!(h.agent(&b).resume_at, None);
    // A write refused by a hook: its tool fails, saying so.
    wait_long(&h, &b, "b idle", 30, |h| !h.agent(&b).status.is_active()).await;
    h.core
        .send_message(&b, "bloquée".into(), vec![])
        .await
        .unwrap();
    wait_long(&h, &b, "the write refused", 30, |h| {
        h.items(&b).iter().any(|i| {
            i["kind"] == "tool" && i["status"] == "error" && i.to_string().contains("hook")
        })
    })
    .await;
}

/// ESC-43: the texts written can go to Cursor CLI (« Modèle des textes rédigés »): one question to an
/// `agent acp` of its own, in its mode Ask (it changes nothing), in a configuration folder of its own
/// removed once answered; on the account's key when it has one.
#[tokio::test]
async fn the_texts_written_go_to_cursor_in_its_mode_ask_on_its_account() {
    let dir = crate::paths::test_dir("esc43-writer");
    std::fs::create_dir_all(&dir).unwrap();
    let program =
        crate::engine::Engine::locate(&crate::engine::CURSOR, &crate::engine::fake_cursor(), "")
            .unwrap();
    let ask = |env: Vec<(String, String)>, prompt: &'static str| crate::engine::OneShot {
        program: program.clone(),
        env,
        weight: crate::engine::Weight::Light,
        cwd: &dir,
        system: "Réponds court.",
        prompt,
        limit: Duration::from_secs(30),
        model: String::new(),
    };
    let answer = crate::engine::oneshot::ask(&crate::engine::CURSOR, ask(vec![], "modèle"))
        .await
        .unwrap();
    assert_eq!(answer, "Je tourne sur default[], en mode ask.");
    let keyed = vec![(
        "CURSOR_API_KEY".to_string(),
        "cle-essai-redacteur-0009".to_string(),
    )];
    let answer = crate::engine::oneshot::ask(&crate::engine::CURSOR, ask(keyed, "compte"))
        .await
        .unwrap();
    assert_eq!(answer, "Compte : clé …0009");
}

/// Real run (ESC-42), by hand: the real Cursor CLI (`ESCOUADE_REAL_CURSOR`, its `agent.cmd`) on the
/// user's account, signed in already (`agent login`, never done here): `cargo test --lib
/// real_cursor -- --ignored --nocapture`. The account pays: four short turns on Auto. It converses,
/// asks a command, allowed « Pour cet agent » (its rule in the agent's own folder, never in
/// `~/.cursor/cli-config.json`), writes a file beside its folder (its permission, with its diff),
/// then is stopped and resumed by `session/load`; it prints the conversation.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn real_cursor() {
    let real =
        std::env::var("ESCOUADE_REAL_CURSOR").expect("ESCOUADE_REAL_CURSOR: the path of agent.cmd");
    let h = harness("esc42-real");
    let (p, _) = h.project(false).await;
    cursor_account(&h, &real);
    let id = cursor_agent(&h, &p.id, "default").await;
    let turns = |h: &Harness| h.items(&id).iter().filter(|i| i["kind"] == "turn").count();
    let send = |text: &str| {
        let (c, id, text) = (h.core.clone(), id.clone(), text.to_string());
        async move { c.send_message(&id, text, vec![]).await.unwrap() }
    };
    let ended = |h: &Harness, n: usize| turns(h) >= n && !h.agent(&id).status.is_active();

    send("Réponds seulement : A").await;
    wait_long(&h, &id, "the first reply", 300, |h| ended(h, 1)).await;
    assert!(texts(&h, &id).iter().any(|t| t.contains('A')));
    let session = h.agent(&id).session_id.clone().expect("its session");
    let home = h.core.data.agent_home(&id);
    println!("SESSION {session} HOME {}", home.display());

    send("Avec ton outil shell, lance exactement `git status --short`, puis dis en une phrase ce qu'il a affiché.").await;
    wait_long(&h, &id, "the permission", 300, |h| {
        waiting(h, &id, "permission").is_some() || ended(h, 2)
    })
    .await;
    let card = waiting(&h, &id, "permission").expect("git status asks");
    println!("PERMISSION {card:#}");
    assert_eq!(card["toolName"], "Bash");
    assert_eq!(card["alwaysScope"], "agent");
    h.core
        .answer_permission(&id, card["id"].as_str().unwrap(), "always", None)
        .unwrap();
    wait_long(&h, &id, "the command's turn", 300, |h| ended(h, 2)).await;
    // Only its rules: the file holds the account's name and e-mail too.
    let config: Value =
        serde_json::from_str(&std::fs::read_to_string(home.join("cli-config.json")).unwrap())
            .unwrap();
    let rules = config["permissions"]["allow"].to_string();
    println!("RULES {rules}");
    assert!(rules.contains("Shell(git status"), "{rules}");

    send("Avec ton outil d'édition de fichiers (pas le shell), crée le fichier ../esc42-dehors.txt qui contient la ligne « bonjour », puis dis seulement : fait.").await;
    wait_long(&h, &id, "the write", 300, |h| {
        waiting(h, &id, "permission").is_some() || ended(h, 3)
    })
    .await;
    if let Some(card) = waiting(&h, &id, "permission") {
        println!("WRITE {card:#}");
        h.core
            .answer_permission(&id, card["id"].as_str().unwrap(), "allow", None)
            .unwrap();
    }
    wait_long(&h, &id, "the write's turn", 300, |h| ended(h, 3)).await;

    // Stopped as an idle agent is, then resumed in another process.
    let old = h.core.agent(&id).unwrap().lock().detach().unwrap();
    old.close_input();
    assert!(
        old.wait_exit(Duration::from_secs(10)).await,
        "killed shortly after its input closed"
    );
    send("Sans outil, en une phrase : quel fichier as-tu créé au tour précédent ?").await;
    wait_long(&h, &id, "the resumed turn", 300, |h| ended(h, 4)).await;
    assert_eq!(h.agent(&id).session_id.as_deref(), Some(session.as_str()));

    for i in h.items(&id) {
        let text = i["text"].as_str().unwrap_or_default();
        println!(
            "{} {} {} {}",
            i["kind"],
            i["name"]
                .as_str()
                .or(i["toolName"].as_str())
                .unwrap_or_default(),
            i["status"]
                .as_str()
                .or(i["decision"].as_str())
                .unwrap_or_default(),
            text.chars()
                .take(140)
                .collect::<String>()
                .replace('\n', " ")
        );
    }
    h.core.archive_agent(&id, true).await.ok();
}

/// Real run (ESC-43), by hand: a text written by the real Cursor CLI (`ESCOUADE_REAL_CURSOR`, its
/// `agent.cmd`), as « Modèle des textes rédigés » asks it, on the machine's sign-in: `cargo test
/// --lib real_cursor_writes -- --ignored --nocapture`. One short question on Auto, in its mode Ask,
/// in a configuration folder of its own (removed after): it prints the answer.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn real_cursor_writes_a_text() {
    let real =
        std::env::var("ESCOUADE_REAL_CURSOR").expect("ESCOUADE_REAL_CURSOR: the path of agent.cmd");
    let program = crate::engine::Engine::locate(&crate::engine::CURSOR, &real, "").unwrap();
    let dir = crate::paths::test_dir("esc43-real-writer");
    std::fs::create_dir_all(&dir).unwrap();
    let started = std::time::Instant::now();
    let answer = crate::engine::oneshot::ask(
        &crate::engine::CURSOR,
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

/// The menu the account `id` keeps (`Core::account_models`): its models' ids.
fn menu_of(core: &crate::core::Core<tauri::test::MockRuntime>, id: &str) -> Vec<String> {
    core.account_models
        .read()
        .get(id)
        .map(|m| m.catalog.iter().map(|m| m.id.clone()).collect())
        .unwrap_or_default()
}

#[tokio::test]
async fn the_models_of_a_cursor_account_are_read_without_an_agent_and_kept_after_a_restart() {
    let h = harness("esc186-cursor-models");
    cursor_account(&h, &crate::engine::fake_cursor());
    assert!(menu_of(&h.core, "principal-cursor").is_empty());
    h.core.read_account_models("principal-cursor").await;
    // Those of `cursor/list_available_models`, with their names and efforts; Auto by default.
    assert_eq!(
        menu_of(&h.core, "principal-cursor"),
        ["default", "claude-opus-5-5", "gpt-5.5", "composer-2.5"]
    );
    let told = h.core.account_models.read()["principal-cursor"].clone();
    let opus = told
        .catalog
        .iter()
        .find(|m| m.id == "claude-opus-5-5")
        .unwrap();
    assert_eq!(opus.label, "Claude Opus 5.5");
    assert_eq!(opus.efforts, ["low", "medium", "high"]);
    assert!(told.catalog[0].default);
    // No agent, no session.
    assert!(h.core.agents.read().is_empty());
    h.core.save_now();
    let app = tauri::test::mock_app();
    let (reloaded, _rx) = crate::core::Core::load(app.handle().clone(), h.core.data.clone());
    assert_eq!(menu_of(&reloaded, "principal-cursor").len(), 4);
    h.core.stop_for_good();
}

/// ESC-186, by hand: the models of the real Cursor CLI (`ESCOUADE_TEST_REAL_CURSOR`, its launcher),
/// read as Escouade reads them without an agent: an `agent acp` in a folder of its own, never the
/// user's `cli-config.json` (to be checked byte for byte after).
#[tokio::test]
#[ignore = "the real Cursor CLI: run by hand"]
async fn real_cursor_models() {
    use crate::engine::Engine;
    let program = std::env::var("ESCOUADE_TEST_REAL_CURSOR").expect("ESCOUADE_TEST_REAL_CURSOR");
    let program = crate::engine::Program::Native(program.into());
    let started = std::time::Instant::now();
    let listed = crate::engine::CURSOR
        .list_models(program, Vec::new(), &std::env::temp_dir())
        .await
        .unwrap()
        .expect("models listed");
    eprintln!(
        "{} models in {} ms, {} with efforts, default {:?}",
        listed.len(),
        started.elapsed().as_millis(),
        listed.iter().filter(|m| !m.efforts.is_empty()).count(),
        listed.iter().find(|m| m.default).map(|m| m.id.clone())
    );
    assert!(listed.len() > 1);
}

/// ESC-205: Cursor CLI's ACP has no compaction request, and its model would take `/compact` for a
/// message: refused in Escouade's view with what does it (its interface's `/summarize`), the
/// composer keeping the text, and nothing sent.
#[tokio::test]
async fn compact_typed_to_a_cursor_agent_is_refused_with_where_it_works() {
    let h = harness("esc205-cursor");
    let (p, _) = h.project(false).await;
    cursor_account(&h, &crate::engine::fake_cursor());
    let id = cursor_agent(&h, &p.id, "default").await;
    h.core
        .send_message(&id, "bonjour".into(), vec![])
        .await
        .unwrap();
    wait_long(&h, &id, "its turn", 30, |h| {
        h.items(&id).iter().any(|i| i["kind"] == "turn") && !h.agent(&id).status.is_active()
    })
    .await;
    let e = h
        .core
        .send_message(&id, "/compact".into(), vec![])
        .await
        .unwrap_err();
    assert_eq!(
        format!("{e:#}"),
        "Cursor CLI ne compacte pas depuis Escouade : tape /summarize dans sa vue terminal."
    );
    assert!(!h.items(&id).iter().any(|i| i["text"] == "/compact"));
    assert!(!h.agent(&id).status.is_active());
    h.core.stop_for_good();
}
