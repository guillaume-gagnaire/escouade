//! Codex beside Claude Code, through the core (ESC-38): its accounts, their quota, the texts it
//! writes and Escouade's server given to its agents. Its agents run the fake `codex`
//! (tests/fixtures/fake-codex.mjs).

use crate::core::AgentOptions;
use crate::core_tests::{harness, Harness};
use crate::model::*;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

/// Codex's Principal and an account « Work » of its own folder (`<test dir>/work`), both on the fake
/// `codex`.
fn codex_accounts(h: &Harness) {
    h.set_settings(|s| {
        for (id, dir) in [
            ("principal-codex", String::new()),
            ("work", h.dir.join("work").to_string_lossy().into_owned()),
        ] {
            s.accounts.push(Account {
                id: id.into(),
                name: id.into(),
                engine: EngineId::CODEX,
                config_dir: dir,
                claude_path: crate::engine::fake_codex(),
                active: true,
                projects: Vec::new(),
                daily_budget: None,
                ..Default::default()
            });
        }
    });
}

/// What the fake `codex exec` noted in the folder of the account « Work ».
fn execs(h: &Harness) -> Vec<Value> {
    std::fs::read_to_string(h.dir.join("work").join("exec.jsonl"))
        .unwrap_or_default()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[tokio::test]
async fn the_texts_written_by_codex_go_through_codex_exec_on_the_account_of_the_agent() {
    let h = harness("esc38-writer");
    let (p, _) = h.project(false).await;
    codex_accounts(&h);
    h.set_settings(|s| s.writer_engine = Some(EngineId::CODEX));
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
    h.core.agent(&id).unwrap().lock().meta.account = "work".into();
    let answer = h
        .core
        .one_shot("Nomme la tâche.", "ajouter un bouton", Some(&id), None)
        .await
        .unwrap();
    assert_eq!(answer, "Réponse de Codex : ajouter un bouton");
    // On its account's folder, ephemeral, read only, its instructions before its question.
    let asked = execs(&h);
    assert_eq!(asked.len(), 1);
    let argv: Vec<&str> = asked[0]["argv"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap())
        .collect();
    for a in [
        "exec",
        "--ephemeral",
        "--ignore-user-config",
        "read-only",
        "--json",
    ] {
        assert!(argv.contains(&a), "{a}: {argv:?}");
    }
    let question = asked[0]["question"].as_str().unwrap();
    assert!(
        question.starts_with("<instructions>\nNomme la tâche."),
        "{question}"
    );
    // Its refusal says Codex's reason.
    let e = h
        .core
        .one_shot("Nomme.", "échoue", Some(&id), None)
        .await
        .unwrap_err();
    assert_eq!(format!("{e:#}"), "Le modèle a refusé.");
    // A question about no agent of Codex: as a new agent of Codex would go, never Claude Code's.
    let other = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let answer = h
        .core
        .one_shot("Nomme.", "autre chose", Some(&other), None)
        .await
        .unwrap();
    assert_eq!(answer, "Réponse de Codex : autre chose");
    h.core.stop_for_good();
}

#[test]
fn a_new_codex_account_shares_its_settings_skills_and_instructions_and_never_its_sign_in() {
    use crate::accounts::{create_of, launch_env, principal_of, shareable_of, ShareMode};
    let dir = crate::paths::test_dir("esc38-codex-share");
    let principal = dir.join(".codex");
    std::fs::create_dir_all(principal.join("skills").join("revue")).unwrap();
    std::fs::write(
        principal.join("skills").join("revue").join("SKILL.md"),
        "# Revue",
    )
    .unwrap();
    std::fs::write(
        principal.join("config.toml"),
        "[windows]\nsandbox = \"elevated\"\n",
    )
    .unwrap();
    std::fs::write(principal.join("AGENTS.md"), "Sois bref.").unwrap();
    std::fs::write(principal.join("auth.json"), "{\"tokens\":{}}").unwrap();
    std::fs::create_dir_all(principal.join("sessions")).unwrap();
    // Offered: its settings, instructions and skills; never its sign-in nor its sessions.
    assert_eq!(
        shareable_of(EngineId::CODEX, &principal),
        ["config.toml", "AGENTS.md", "skills"]
    );
    let mut s = Settings::default();
    s.accounts.push(principal_of(EngineId::CODEX));
    let root = dir.join("data").join("codex");
    for refused in ["auth.json", "sessions", "../x"] {
        let e = create_of(
            EngineId::CODEX,
            &s,
            &root,
            &principal,
            "Refusé",
            &[refused.to_string()],
            ShareMode::Copy,
        )
        .unwrap_err();
        assert!(
            e.to_string().contains("ne se partage pas"),
            "{refused}: {e}"
        );
    }
    let share: Vec<String> = shareable_of(EngineId::CODEX, &principal);
    let made = create_of(
        EngineId::CODEX,
        &s,
        &root,
        &principal,
        "Travail",
        &share,
        ShareMode::Copy,
    )
    .unwrap();
    assert_eq!(
        (made.id.as_str(), made.engine),
        ("travail", EngineId::CODEX)
    );
    let own = std::path::PathBuf::from(&made.config_dir);
    assert_eq!(own, std::path::absolute(root.join("travail")).unwrap());
    assert_eq!(
        std::fs::read_to_string(own.join("AGENTS.md")).unwrap(),
        "Sois bref."
    );
    assert!(own.join("config.toml").is_file());
    assert!(own.join("skills").join("revue").join("SKILL.md").is_file());
    assert!(!own.join("auth.json").exists());
    assert!(!own.join("sessions").exists());
    // Its processes get its folder by CODEX_HOME; the Principal is launched without it.
    assert_eq!(
        launch_env(&made),
        [("CODEX_HOME".to_string(), made.config_dir.clone())]
    );
    assert!(launch_env(&principal_of(EngineId::CODEX)).is_empty());
}

#[test]
fn a_codex_sign_in_is_read_from_its_auth_json_its_email_from_its_id_token() {
    use base64::Engine as _;
    let dir = crate::paths::test_dir("esc38-codex-auth");
    assert_eq!(crate::accounts::codex_sign_in(&dir), None);
    let b64 = |v: Value| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(v.to_string());
    let token = |email: &str| {
        format!(
            "{}.{}.sig",
            b64(serde_json::json!({ "alg": "RS256" })),
            b64(serde_json::json!({ "email": email }))
        )
    };
    let write = |id_token: &str| {
        let auth = serde_json::json!({ "auth_mode": "chatgpt", "OPENAI_API_KEY": null,
            "tokens": { "id_token": id_token, "access_token": "a", "refresh_token": "r", "account_id": "c" } });
        std::fs::write(dir.join("auth.json"), auth.to_string()).unwrap();
    };
    write(&token("moi@example.com"));
    let one = crate::accounts::codex_sign_in(&dir).unwrap();
    assert_eq!(one.email.as_deref(), Some("moi@example.com"));
    let stamp = one.stamp.unwrap();
    assert_eq!(stamp.len(), 16);
    assert!(!stamp.contains("sig"));
    // Signed in again: another stamp.
    write(&token("autre@example.com"));
    assert_ne!(
        crate::accounts::codex_sign_in(&dir).unwrap().stamp.unwrap(),
        stamp
    );
    // An API key: signed in, no email.
    std::fs::write(dir.join("auth.json"), r#"{"OPENAI_API_KEY":"sk-x"}"#).unwrap();
    assert_eq!(crate::accounts::codex_sign_in(&dir).unwrap().email, None);
    std::fs::write(dir.join("auth.json"), "{}").unwrap();
    assert_eq!(crate::accounts::codex_sign_in(&dir), None);
}

#[tokio::test]
async fn a_codex_account_is_added_then_signed_in_by_codex_login_in_a_terminal_of_its_folder() {
    use crate::accounts::ShareMode;
    let h = harness("esc38-codex-login");
    // A new account alone: the first of Codex brings no Principal (ESC-128).
    let work = h
        .core
        .create_account(EngineId::CODEX, "Travail", &[], ShareMode::Link)
        .unwrap();
    let dir = std::path::PathBuf::from(&work.config_dir);
    assert_eq!(
        dir,
        std::path::absolute(h.core.data.accounts_of(EngineId::CODEX).join("travail")).unwrap()
    );
    let ids: Vec<String> = h
        .core
        .settings
        .read()
        .accounts
        .iter()
        .map(|a| a.id.clone())
        .collect();
    assert_eq!(ids, ["principal", "travail"]);
    let work = h
        .core
        .update_claude_account(Account {
            claude_path: crate::engine::fake_codex(),
            ..work
        })
        .unwrap()
        .into_iter()
        .find(|a| a.id == "travail")
        .unwrap();
    let status = h.core.claude_account_status("travail").await.unwrap();
    assert!(!status.connected);
    assert_eq!(status.dir, work.config_dir);
    let out = Arc::new(parking_lot::Mutex::new(String::new()));
    let sink = out.clone();
    let info = h
        .core
        .claude_login("travail", (200, 30), move |b| {
            sink.lock().push_str(&String::from_utf8_lossy(&b))
        })
        .unwrap();
    assert_eq!(
        (info.name.as_str(), info.shell.as_str()),
        ("Travail", "codex")
    );
    let start = std::time::Instant::now();
    let mut answered = false;
    while !dir.join("auth.json").is_file() {
        if !answered && out.lock().contains("\x1b[6n") {
            h.core.pty.write(&info.id, b"\x1b[1;1R").unwrap();
            answered = true;
        }
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "no sign-in: {:?}",
            out.lock()
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let status = h.core.claude_account_status("travail").await.unwrap();
    assert!(status.connected);
    assert_eq!(status.email.as_deref(), Some("codex@example.com"));
    // What `codex login` said is in its terminal.
    while !crate::core_tests::plain_text(&out.lock()).contains("Successfully logged in") {
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "{:?}",
            out.lock()
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    h.core.stop_for_good();
}

/// The account « Work » signed in (an `auth.json` of the fake's), its quota `limits` when given.
fn signed_in(h: &Harness, limits: Option<Value>) {
    let dir = h.dir.join("work");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("auth.json"),
        r#"{"OPENAI_API_KEY":null,"tokens":{"id_token":"a.e30.s","account_id":"c"}}"#,
    )
    .unwrap();
    if let Some(limits) = limits {
        let quota = serde_json::json!({ "rateLimits": limits });
        std::fs::write(dir.join("quota.json"), quota.to_string()).unwrap();
    }
}

/// The launches of the fake `codex` with the account « Work » (`CODEX_HOME`).
fn launches(h: &Harness) -> Vec<Value> {
    std::fs::read_to_string(h.dir.join("work").join("launches.jsonl"))
        .unwrap_or_default()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[tokio::test]
async fn the_quota_of_a_codex_account_without_an_agent_is_read_by_an_app_server_of_its_own() {
    let h = harness("esc38-quota-apart");
    codex_accounts(&h);
    // The free plan's single window: 30 days.
    let resets = chrono::Utc::now().timestamp() + 86_400;
    signed_in(
        &h,
        Some(serde_json::json!({
            "primary": { "usedPercent": 40, "windowDurationMins": 43200, "resetsAt": resets },
            "secondary": null, "rateLimitReachedType": null,
        })),
    );
    h.core.refresh_usage().await;
    let usage = h.core.usage.lock().clone();
    let work = usage.account("work").unwrap();
    assert_eq!(work.meters.len(), 1, "{:?}", work.meters);
    assert_eq!(
        (work.meters[0].minutes, work.meters[0].pct),
        (Some(43200), 40.0)
    );
    assert_eq!(work.meters[0].resets_at, Some(resets * 1000));
    assert!(work.connected);
    // Started for that and gone: nothing of Escouade's server, no thread.
    let started = launches(&h);
    assert_eq!(started.len(), 1);
    assert_eq!(started[0]["argv"], serde_json::json!(["app-server"]));
    // Its Principal has no sign-in here: said so, no process started.
    let main = usage.account("principal-codex").unwrap();
    assert!(!main.connected);
    // Read again within five minutes: not asked again.
    h.core.refresh_usage().await;
    assert_eq!(launches(&h).len(), 1);
    h.core.stop_for_good();
}

#[tokio::test]
async fn a_codex_turn_stopped_by_the_limit_waits_for_the_reset_then_goes_on_by_itself() {
    let h = harness("esc38-limit");
    let (p, _) = h.project(false).await;
    codex_accounts(&h);
    signed_in(&h, None);
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::CODEX),
                account: Some("work".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    h.turn(&id, "limite").await;
    let turn = h
        .items(&id)
        .into_iter()
        .rfind(|i| i["kind"] == "turn")
        .unwrap();
    assert_eq!(turn["limited"], true);
    // Its resume is planned after the reset Codex told of its full window (`account/rateLimits/
    // updated`, in an hour with the fake), not from the quota the core reads besides: a reading
    // asked before the turn may be answered after it.
    let at = h.agent(&id).resume_at.expect("its resume planned");
    assert!(at >= now_ms() + 3_500_000, "after the reset: {at}");
    // Not due yet: nothing sent.
    h.core.resume_due().await;
    assert!(h.agent(&id).resume_at.is_some());
    // Its reset has come: it goes on by itself, on Codex.
    h.core.agent(&id).unwrap().lock().meta.resume_at = Some(now_ms() - 1_000);
    h.core.resume_due().await;
    h.wait("it goes on", |h| {
        h.items(&id)
            .iter()
            .any(|i| i["kind"] == "text" && i["text"] == "Bonjour, tu as dit : continue")
    })
    .await;
    assert_eq!(h.agent(&id).resume_at, None);
    h.core.stop_for_good();
}

#[tokio::test]
async fn new_codex_agents_go_to_the_next_codex_account_once_one_is_over_the_threshold() {
    let h = harness("esc38-switch");
    let (p, _) = h.project(false).await;
    codex_accounts(&h);
    let new_codex = |h: &Harness| {
        let core = h.core.clone();
        let p = p.id.clone();
        async move {
            core.create_agent_with(
                &p,
                AgentOptions {
                    engine: Some(EngineId::CODEX),
                    ..Default::default()
                },
            )
            .await
            .unwrap()
            .meta
        }
    };
    assert_eq!(new_codex(&h).await.account, "principal-codex");
    // The Principal of Codex over the threshold (Claude Code's untouched): « Work », and said so.
    let full = |pct: f64| {
        crate::usage::Reading::Windows(vec![QuotaMeter::window(
            300,
            RateWindow {
                pct,
                resets_at: Some(now_ms() + 3_600_000),
            },
        )])
    };
    h.core.record_usage("principal-codex", full(100.0));
    assert_eq!(h.core.usage.lock().current[&EngineId::CODEX], "work");
    assert_eq!(h.core.usage.lock().current[&EngineId::CLAUDE], "principal");
    assert!(h.events.lock().iter().any(|e| e["type"] == "toast"));
    let a = new_codex(&h).await;
    assert_eq!(a.account, "work");
    // Its process gets the folder of « Work »; a new Claude Code agent stays on Claude Code's.
    signed_in(&h, None);
    h.turn(&a.id, "bonjour").await;
    let started = launches(&h);
    assert_eq!(
        started.last().unwrap()["codexHome"],
        h.dir.join("work").to_string_lossy().as_ref()
    );
    assert_eq!(
        h.core.create_agent(&p.id, None).await.unwrap().meta.account,
        "principal"
    );
    h.core.stop_for_good();
}

/// ESC-156 with Codex: an agent of Codex, named by Codex (« Modèle des textes rédigés »), gets its
/// worktree at its first message under that name, and its thread takes it.
#[tokio::test]
async fn a_codex_agent_gets_its_worktree_at_its_first_message_under_the_name_codex_gives() {
    let h = harness("esc156-codex");
    let (p, _) = h.project_as_new(true).await;
    codex_accounts(&h);
    h.set_settings(|s| s.writer_engine = Some(EngineId::CODEX));
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::CODEX),
                account: Some("work".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    assert!(h.agent(&id).worktree_pending);
    h.turn(&id, "Ajoute un bouton").await;
    let m = h.agent(&id);
    assert_eq!(m.name, "ajoute-un-codex");
    let wt = m.worktree.unwrap();
    assert_eq!(
        std::path::PathBuf::from(&wt.path),
        p.worktrees_root().join("ajoute-un-codex")
    );
    assert_eq!(wt.branch, "escouade/ajoute-un-codex");
    let renamed = |h: &Harness| {
        std::fs::read_to_string(h.dir.join("work").join("requests.jsonl"))
            .unwrap_or_default()
            .lines()
            .filter_map(|l| serde_json::from_str::<Value>(l).ok())
            .any(|r| {
                r["in"]["method"] == "thread/name/set"
                    && r["in"]["params"]["name"] == "ajoute-un-codex"
            })
    };
    h.wait("its thread renamed", renamed).await;
    h.core.stop_for_good();
}

/// What the fake `codex` of the account « Work » was sent (`requests.jsonl`): the methods, in order.
fn sent(h: &Harness) -> Vec<String> {
    std::fs::read_to_string(h.dir.join("work").join("requests.jsonl"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter_map(|r| r["in"]["method"].as_str().map(str::to_string))
        .collect()
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
async fn the_models_of_a_codex_account_are_read_on_every_page_without_an_agent_and_kept_after_a_restart(
) {
    let h = harness("esc186-codex-models");
    codex_accounts(&h);
    signed_in(&h, None);
    // Before anything read: nothing but the menu's stand-in (GPT-6-Luna, ESC-186).
    assert!(menu_of(&h.core, "work").is_empty());
    h.core.read_account_models("work").await;
    // Each page of `model/list` (the fake gives one model a page), the hidden one left out.
    assert_eq!(menu_of(&h.core, "work"), ["gpt-6-luna", "gpt-5.6-terra"]);
    // An app-server of its own, gone after: no thread, no turn.
    let started = launches(&h);
    assert_eq!(started.len(), 1);
    assert_eq!(started[0]["argv"], serde_json::json!(["app-server"]));
    let methods = sent(&h);
    assert_eq!(
        methods,
        ["initialize", "initialized", "model/list", "model/list"]
    );
    // The window is told, and the harness's list follows (a new agent's model, the writer's).
    assert!(h
        .events
        .lock()
        .iter()
        .any(|e| e["type"] == "accountModels" && e["account"] == "work"));
    assert_eq!(h.core.engine_models.read()[&EngineId::CODEX].len(), 2);
    // Opening the menu within a day reads nothing again; a day after, it reads again.
    h.core.refresh_account_models("work").await;
    assert_eq!(launches(&h).len(), 1);
    h.core
        .account_models
        .write()
        .get_mut("work")
        .unwrap()
        .read_at -= 2 * 86_400_000;
    h.core.refresh_account_models("work").await;
    assert_eq!(launches(&h).len(), 2);
    // Kept by account through a restart.
    h.core.save_now();
    let app = tauri::test::mock_app();
    let (reloaded, _rx) = crate::core::Core::load(app.handle().clone(), h.core.data.clone());
    assert_eq!(menu_of(&reloaded, "work"), ["gpt-6-luna", "gpt-5.6-terra"]);
    assert!(menu_of(&reloaded, "principal-codex").is_empty());
    h.core.stop_for_good();
}

#[tokio::test]
async fn a_codex_agent_lists_every_page_of_model_list_for_its_account() {
    let h = harness("esc186-codex-agent-models");
    let (p, _) = h.project(false).await;
    codex_accounts(&h);
    signed_in(&h, None);
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::CODEX),
                account: Some("work".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    h.turn(&id, "Bonjour").await;
    assert_eq!(menu_of(&h.core, "work"), ["gpt-6-luna", "gpt-5.6-terra"]);
    let listed: Vec<String> = h.core.engine_models.read()[&EngineId::CODEX]
        .iter()
        .map(|m| m.id.clone())
        .collect();
    assert_eq!(listed, ["gpt-6-luna", "gpt-5.6-terra"]);
    h.core.stop_for_good();
}

/// ESC-186, by hand: the models of the real `codex` of the machine (`ESCOUADE_TEST_REAL_CODEX`, its
/// program), read as Escouade reads them without an agent (every page of `model/list`, its own
/// `~/.codex`: nothing but `initialize` and `model/list` asked). Its `config.toml` is to be copied
/// before and checked byte for byte after (the app-server may write in it, ESC-37).
#[tokio::test]
#[ignore = "the real codex: run by hand"]
async fn real_codex_models() {
    let program = std::env::var("ESCOUADE_TEST_REAL_CODEX").expect("ESCOUADE_TEST_REAL_CODEX");
    let program = crate::engine::Program::Native(program.into());
    let started = std::time::Instant::now();
    let listed = crate::engine::by_id(EngineId::CODEX)
        .list_models(program, Vec::new(), &std::env::temp_dir())
        .await
        .unwrap()
        .expect("models listed");
    let ids: Vec<&str> = listed.iter().map(|m| m.id.as_str()).collect();
    eprintln!(
        "{} models in {} ms: {ids:?}",
        ids.len(),
        started.elapsed().as_millis()
    );
    assert!(!listed.is_empty());
    assert_eq!(listed.iter().filter(|m| m.default).count(), 1);
}

#[test]
fn codexs_stand_in_model_serves_only_until_its_account_listed_its_own() {
    let h = harness("esc186-stand-in");
    // Nothing listed yet: GPT-6-Luna, the one it declares.
    assert_eq!(
        h.core.model_for(EngineId::CODEX, "gpt-6-luna".into()),
        "gpt-6-luna"
    );
    // Its account lists others (the real one of ESC-186: GPT-5.6-Terra by default, GPT-5.6-Luna).
    let told = |id: &str, default: bool| crate::engine::registry::ToldModel {
        from_config: false,
        id: id.into(),
        label: id.into(),
        efforts: Vec::new(),
        default_effort: None,
        images: false,
        default,
        provider: None,
        multiplier: None,
        price: None,
    };
    h.core.engine_models.write().insert(
        EngineId::CODEX,
        vec![told("gpt-5.6-terra", true), told("gpt-5.6-luna", false)],
    );
    assert_eq!(
        h.core.model_for(EngineId::CODEX, "gpt-6-luna".into()),
        "gpt-5.6-terra"
    );
    assert_eq!(
        h.core.model_for(EngineId::CODEX, "gpt-5.6-luna".into()),
        "gpt-5.6-luna"
    );
}

#[test]
fn the_model_of_an_accounts_config_toml_comes_after_those_listed_when_they_lack_it() {
    use crate::engine::codex::{config_model, with_config_model};
    let dir = crate::paths::test_dir("esc186-config-model");
    let told = |id: &str| crate::engine::registry::ToldModel {
        from_config: false,
        id: id.into(),
        label: id.into(),
        efforts: vec!["medium".into()],
        default_effort: None,
        images: false,
        default: false,
        provider: None,
        multiplier: None,
        price: None,
    };
    let listed = || vec![told("gpt-5.6-terra"), told("gpt-5.6-luna")];
    let ids = |m: &[crate::engine::registry::ToldModel]| -> Vec<String> {
        m.iter().map(|m| m.id.clone()).collect()
    };
    // No config.toml, or one without `model`: those listed alone.
    assert_eq!(
        ids(&with_config_model(listed(), &dir)),
        ["gpt-5.6-terra", "gpt-5.6-luna"]
    );
    std::fs::write(
        dir.join("config.toml"),
        "model_reasoning_effort = \"high\"\n",
    )
    .unwrap();
    assert_eq!(with_config_model(listed(), &dir).len(), 2);
    // A model it does not list: after them, from the config, with no effort of its own.
    std::fs::write(
        dir.join("config.toml"),
        "model = \"gpt-5.6-sol\"\nmodel_reasoning_effort = \"high\"\n\n[profiles.x]\nmodel = \"autre\"\n",
    )
    .unwrap();
    let menu = with_config_model(listed(), &dir);
    assert_eq!(ids(&menu), ["gpt-5.6-terra", "gpt-5.6-luna", "gpt-5.6-sol"]);
    assert!(menu[2].from_config && menu[2].efforts.is_empty() && !menu[2].default);
    assert!(!menu[0].from_config);
    // One it lists: not twice.
    std::fs::write(dir.join("config.toml"), "model = \"gpt-5.6-luna\"\n").unwrap();
    let menu = with_config_model(listed(), &dir);
    assert_eq!(ids(&menu), ["gpt-5.6-terra", "gpt-5.6-luna"]);
    assert!(menu.iter().all(|m| !m.from_config));
    assert_eq!(config_model("pas du toml = ="), None);
    assert_eq!(config_model("model = \"\""), None);
}

#[tokio::test]
async fn the_menu_of_a_codex_account_has_the_model_its_config_toml_names() {
    let h = harness("esc186-config-menu");
    codex_accounts(&h);
    signed_in(&h, None);
    std::fs::write(
        h.dir.join("work").join("config.toml"),
        "model = \"gpt-5.6-sol\"\n",
    )
    .unwrap();
    h.core.read_account_models("work").await;
    assert_eq!(
        menu_of(&h.core, "work"),
        ["gpt-6-luna", "gpt-5.6-terra", "gpt-5.6-sol"]
    );
    // The harness-wide list keeps those listed alone: the config is the account's.
    assert_eq!(h.core.engine_models.read()[&EngineId::CODEX].len(), 2);
    h.core.stop_for_good();
}

/// The context gauge of the agent `id`: the tokens of its context.
fn context_of(h: &Harness, id: &str) -> u64 {
    h.core.agent(id).unwrap().lock().context_tokens
}

/// ESC-205: `/compact` typed to an agent of Codex is no message to its model. Codex compacts its
/// thread (`thread/compact/start`), the conversation says when it starts and when it ends, and the
/// context gauge goes down. During a turn it is refused with its reason, and so is an instruction
/// after it: Codex's request takes none.
#[tokio::test]
async fn compact_typed_to_a_codex_agent_compacts_its_thread_and_not_through_its_model() {
    let h = harness("esc205-compact");
    let (p, _) = h.project(false).await;
    codex_accounts(&h);
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::CODEX),
                account: Some("work".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    h.turn(&id, "Bonjour").await;
    h.turn(&id, "Encore").await;
    let before = context_of(&h, &id);
    assert!(before > 0);
    h.turn(&id, "/compact").await;
    let methods = sent(&h);
    assert_eq!(
        methods
            .iter()
            .filter(|m| *m == "thread/compact/start")
            .count(),
        1,
        "{methods:?}"
    );
    assert_eq!(
        methods.iter().filter(|m| *m == "turn/start").count(),
        2,
        "{methods:?}"
    );
    let items = h.items(&id);
    let texts: Vec<String> = items
        .iter()
        .filter(|i| i["kind"] == "user" || i["kind"] == "notice")
        .map(|i| {
            format!(
                "{}: {}",
                i["kind"].as_str().unwrap(),
                i["text"].as_str().unwrap()
            )
        })
        .collect();
    let tail = &texts[texts.len() - 3..];
    assert_eq!(
        tail,
        [
            "user: /compact",
            "notice: Compaction du contexte…",
            "notice: Contexte compacté",
        ],
        "{texts:?}"
    );
    assert!(
        !items
            .iter()
            .any(|i| i["kind"] == "text" && i["text"].as_str().unwrap().contains("/compact")),
        "the model was told /compact"
    );
    let after = context_of(&h, &id);
    assert!(after < before, "{before} → {after}");
    // An instruction after it: Codex takes none, the composer keeps the text.
    let e = h
        .core
        .send_message(&id, "/compact garde les décisions".into(), vec![])
        .await
        .unwrap_err();
    assert!(format!("{e:#}").contains("instruction"), "{e:#}");
    // During a turn: refused, with its reason.
    h.core
        .send_message(&id, "lent 20s".into(), vec![])
        .await
        .unwrap();
    h.wait("the slow turn", |h| h.agent(&id).status.is_active())
        .await;
    let e = h
        .core
        .send_message(&id, "/compact".into(), vec![])
        .await
        .unwrap_err();
    assert!(format!("{e:#}").contains("tour"), "{e:#}");
    h.core.interrupt(&id).await.unwrap();
    h.wait("the turn interrupted", |h| !h.agent(&id).status.is_active())
        .await;
    let methods = sent(&h);
    assert_eq!(
        methods
            .iter()
            .filter(|m| *m == "thread/compact/start")
            .count(),
        1,
        "{methods:?}"
    );
    h.core.stop_for_good();
}

/// ESC-205, by hand: `/compact` with the real `codex` that `ESCOUADE_REAL_CODEX` names (0.163.0
/// or later), on the machine's own Codex folder (`~/.codex`): two short turns, then the
/// compaction, its start and its end shown and the context gauge lower. `cargo test --lib
/// real_codex_compact -- --ignored --nocapture`; save `~/.codex/config.toml` before and check it
/// byte for byte after, and take the thread it prints out of `~/.codex/sessions` and
/// `session_index.jsonl`.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "the real codex: run by hand"]
async fn real_codex_compact() {
    let program = std::env::var("ESCOUADE_REAL_CODEX").expect("ESCOUADE_REAL_CODEX");
    let h = harness("esc205-real");
    let (p, _) = h.project(false).await;
    let home = dirs::home_dir().unwrap().join(".codex");
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
    for text in [
        "Réponds seulement par le mot : bonjour.",
        "Réponds seulement par le mot : merci.",
    ] {
        h.turn(&id, text).await;
    }
    let before = context_of(&h, &id);
    let started = std::time::Instant::now();
    h.turn(&id, "/compact").await;
    let after = context_of(&h, &id);
    let shown: Vec<String> = h
        .items(&id)
        .iter()
        .filter(|i| ["user", "notice", "text"].contains(&i["kind"].as_str().unwrap_or("")))
        .map(|i| {
            format!(
                "{}: {}",
                i["kind"].as_str().unwrap(),
                i["text"].as_str().unwrap_or("")
            )
        })
        .collect();
    println!("thread: {:?}", h.agent(&id).session_id);
    println!(
        "context: {before} → {after}, compaction in {} ms",
        started.elapsed().as_millis()
    );
    println!("{shown:#?}");
    assert!(shown.contains(&"notice: Compaction du contexte…".to_string()));
    assert!(shown.contains(&"notice: Contexte compacté".to_string()));
    assert!(after < before, "{before} → {after}");
    h.core.stop_for_good();
}
