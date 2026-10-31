//! GitHub Copilot CLI beside Claude Code, through the core (ESC-39): the real run of its adapter,
//! by hand. The fake's runs are the bench (`engine/copilot/bench.rs`) and the e2e
//! (`tests/e2e/copilot.spec.ts`).

use crate::core::AgentOptions;
use crate::core_tests::{harness, Harness};
use crate::model::*;
use serde_json::{json, Value};

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
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    while std::time::Instant::now() < deadline {
        if pred(h) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    panic!("no {what}: {:#}", Value::Array(h.items(id)));
}

/// Real run (ESC-39), by hand: the real Copilot CLI 1.0.98 (`ESCOUADE_REAL_COPILOT`, its
/// `copilot.exe`), a `COPILOT_HOME` of its own and a token in the environment of this process
/// alone (`GH_TOKEN`, never printed): `cargo test --lib real_copilot -- --ignored --nocapture`.
/// Its credits are counted: four short turns. It converses, asks a command (allowed), a question
/// (answered), then plans and is sent back; it prints the conversation, tokens never in it.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn real_copilot() {
    let real = std::env::var("ESCOUADE_REAL_COPILOT")
        .expect("ESCOUADE_REAL_COPILOT: the path of the real copilot.exe");
    let home = std::env::var("COPILOT_HOME").expect("COPILOT_HOME: a folder of the trial's");
    assert!(
        !home.contains(".copilot"),
        "a COPILOT_HOME of the trial's, never the user's"
    );
    let h = harness("esc39-real");
    let (p, _) = h.project(false).await;
    h.set_settings(|s| {
        s.accounts.push(Account {
            id: "principal-copilot".into(),
            name: "Principal".into(),
            engine: EngineId::Copilot,
            config_dir: String::new(),
            claude_path: real.clone(),
            active: true,
            projects: Vec::new(),
            daily_budget: None,
            ..Default::default()
        });
    });
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::Copilot),
                model: Some("gpt-6-luna".into()),
                mode: Some("default".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    let turns = |h: &Harness| h.items(&id).iter().filter(|i| i["kind"] == "turn").count();
    let send = |text: &str| {
        let (c, id, text) = (h.core.clone(), id.clone(), text.to_string());
        async move { c.send_message(&id, text, vec![]).await.unwrap() }
    };
    let ended = |h: &Harness, n: usize| turns(h) >= n && !h.agent(&id).status.is_active();

    send("Réponds seulement : A").await;
    wait_long(&h, &id, "the first reply", 300, |h| ended(h, 1)).await;
    assert!(texts(&h, &id).iter().any(|t| t.contains('A')));
    assert_eq!(h.agent(&id).model, "gpt-6-luna");

    send("Avec l'outil powershell, lance exactement `git status --short`, puis dis en une phrase ce qu'il a affiché.").await;
    wait_long(&h, &id, "the permission", 300, |h| {
        waiting(h, &id, "permission").is_some() || ended(h, 2)
    })
    .await;
    let card = waiting(&h, &id, "permission").expect("git status asks");
    println!("PERMISSION {card:#}");
    assert_eq!(card["toolName"], "Bash");
    h.core
        .answer_permission(&id, card["id"].as_str().unwrap(), "allow", None)
        .unwrap();
    wait_long(&h, &id, "the command's turn", 300, |h| ended(h, 2)).await;
    assert!(h
        .items(&id)
        .iter()
        .any(|i| i["kind"] == "tool" && i["name"] == "Bash" && i["status"] == "ok"));

    send("Pose-moi une question avec l'outil ask_user : « Quelle couleur ? », avec les choix rouge et bleu. Puis réponds seulement par la couleur choisie.").await;
    wait_long(&h, &id, "the question", 300, |h| {
        waiting(h, &id, "question").is_some()
    })
    .await;
    let card = waiting(&h, &id, "question").unwrap();
    println!("QUESTION {card:#}");
    let q = card["questions"][0]["question"]
        .as_str()
        .unwrap()
        .to_string();
    h.core
        .answer_question(&id, card["id"].as_str().unwrap(), json!({ q: "bleu" }))
        .unwrap();
    wait_long(&h, &id, "the question's turn", 300, |h| ended(h, 3)).await;
    assert!(texts(&h, &id)
        .last()
        .is_some_and(|t| t.to_lowercase().contains("bleu")));

    h.core
        .set_agent_options(&id, None, None, Some("plan".into()))
        .await
        .unwrap();
    send("Prépare un plan de deux étapes pour ajouter un fichier b.txt, sans rien modifier, puis demande sa validation avec exit_plan_mode, sans poser de question.").await;
    wait_long(&h, &id, "the plan", 300, |h| {
        waiting(h, &id, "permission").is_some_and(|c| c["toolName"] == "ExitPlanMode")
            || ended(h, 4)
    })
    .await;
    // Reads in its folder go without a card: only the plan is asked.
    let card = waiting(&h, &id, "permission").expect("the plan to approve");
    println!("PLAN {card:#}");
    assert_eq!(card["toolName"], "ExitPlanMode");
    // Sent back, Copilot plans again and asks again (seen on the real run of 2026-10-18, as on
    // ESC-23's): each plan is sent back until its turn ends.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(300);
    while !ended(&h, 4) && std::time::Instant::now() < deadline {
        if let Some(card) = waiting(&h, &id, "permission") {
            h.core
                .answer_permission(
                    &id,
                    card["id"].as_str().unwrap(),
                    "deny",
                    Some("Pas maintenant : arrête-toi là, sans rien faire de plus.".into()),
                )
                .unwrap();
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    wait_long(&h, &id, "the plan's turn", 30, |h| ended(h, 4)).await;

    let a = h.agent(&id);
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
                .take(100)
                .collect::<String>()
                .replace('\n', " ")
        );
    }
    println!(
        "TOKENS {} COST {:.5} $ SESSION {:?}",
        a.tokens, a.cost, a.session_id
    );
    assert!(a.tokens > 0 && a.cost > 0.0);
    h.core.archive_agent(&id, true).await.ok();
}

// ----- ESC-40: its accounts, their credits, the texts it writes -----

use std::path::PathBuf;
use std::time::Duration;

/// Copilot's Principal (inactive: never started here) and an account « Work » of its own folder
/// (`<test dir>/work`), both on the fake `copilot`; `fake` is what the fake reads of « Work »
/// (`fake.json`: `auth`, `free`, `credits`).
fn copilot_accounts(h: &Harness, fake: Value) -> PathBuf {
    let work = h.dir.join("work");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::write(work.join("fake.json"), fake.to_string()).unwrap();
    h.set_settings(|s| {
        for (id, dir, active) in [
            ("principal-copilot", String::new(), false),
            ("work", work.to_string_lossy().into_owned(), true),
        ] {
            s.accounts.push(Account {
                id: id.into(),
                name: id.into(),
                engine: EngineId::Copilot,
                config_dir: dir,
                claude_path: crate::engine::fake_copilot(),
                active,
                projects: Vec::new(),
                daily_budget: None,
                ..Default::default()
            });
        }
    });
    work
}

/// The lines of the file `name` the fake wrote in the folder `dir`.
fn noted(dir: &std::path::Path, name: &str) -> Vec<Value> {
    std::fs::read_to_string(dir.join(name))
        .unwrap_or_default()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn argv(launch: &Value) -> Vec<String> {
    launch["argv"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap().to_string())
        .collect()
}

/// The first of the month after now, at midnight UTC, in epoch ms.
fn next_month() -> i64 {
    use chrono::{Datelike, TimeZone};
    let now = chrono::Utc::now();
    let (y, m) = if now.month() == 12 {
        (now.year() + 1, 1)
    } else {
        (now.year(), now.month() + 1)
    };
    chrono::Utc
        .with_ymd_and_hms(y, m, 1, 0, 0, 0)
        .unwrap()
        .timestamp_millis()
}

async fn copilot_agent(h: &Harness, p: &Project) -> String {
    h.core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::Copilot),
                account: Some("work".into()),
                model: Some("gpt-6-luna".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id
}

#[test]
fn a_new_copilot_account_shares_its_mcp_servers_instructions_agents_and_skills_and_never_its_sign_in(
) {
    use crate::accounts::{create_of, launch_env, principal_of, shareable_of, ShareMode};
    let dir = crate::paths::test_dir("esc40-copilot-share");
    let principal = dir.join(".copilot");
    std::fs::create_dir_all(principal.join("skills").join("revue")).unwrap();
    std::fs::write(
        principal.join("skills").join("revue").join("SKILL.md"),
        "# Revue",
    )
    .unwrap();
    std::fs::create_dir_all(principal.join("agents")).unwrap();
    std::fs::write(principal.join("agents").join("relu.agent.md"), "# Relu").unwrap();
    std::fs::write(principal.join("mcp-config.json"), r#"{"mcpServers":{}}"#).unwrap();
    std::fs::write(principal.join("copilot-instructions.md"), "Sois bref.").unwrap();
    // Its sign-in, its trusted folders and its sessions.
    std::fs::write(
        principal.join("config.json"),
        r#"{"loggedInUsers":[{"host":"https://github.com","login":"moi"}]}"#,
    )
    .unwrap();
    std::fs::write(principal.join("settings.json"), "{}").unwrap();
    std::fs::create_dir_all(principal.join("session-state")).unwrap();
    assert_eq!(
        shareable_of(EngineId::Copilot, &principal),
        [
            "mcp-config.json",
            "copilot-instructions.md",
            "agents",
            "skills"
        ]
    );
    let mut s = Settings::default();
    s.accounts.push(principal_of(EngineId::Copilot));
    let root = dir.join("data").join("copilot");
    for refused in ["config.json", "settings.json", "session-state"] {
        let e = create_of(
            EngineId::Copilot,
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
    let share = shareable_of(EngineId::Copilot, &principal);
    let made = create_of(
        EngineId::Copilot,
        &s,
        &root,
        &principal,
        "Travail",
        &share,
        ShareMode::Copy,
    )
    .unwrap();
    let own = PathBuf::from(&made.config_dir);
    assert_eq!(own, std::path::absolute(root.join("travail")).unwrap());
    assert_eq!(
        std::fs::read_to_string(own.join("copilot-instructions.md")).unwrap(),
        "Sois bref."
    );
    assert!(own.join("mcp-config.json").is_file());
    assert!(own.join("agents").join("relu.agent.md").is_file());
    assert!(own.join("skills").join("revue").join("SKILL.md").is_file());
    // Not signed in as Principal's user: a folder of its own, without its config.json.
    assert!(!own.join("config.json").exists());
    assert_eq!(crate::accounts::copilot_sign_in(&own), None);
    assert_eq!(
        launch_env(&made),
        [("COPILOT_HOME".to_string(), made.config_dir.clone())]
    );
    assert!(launch_env(&principal_of(EngineId::Copilot)).is_empty());
}

#[test]
fn a_copilot_sign_in_is_the_user_its_config_json_names_never_a_token() {
    use crate::accounts::copilot_sign_in;
    let dir = crate::paths::test_dir("esc40-copilot-auth");
    assert_eq!(copilot_sign_in(&dir), None);
    let write = |text: &str| std::fs::write(dir.join("config.json"), text).unwrap();
    // As Copilot CLI writes it: comments first; never signed in.
    write("// User settings belong in settings.json.\n// This file is managed automatically.\n{\n  \"firstLaunchAt\": \"2026-10-17T18:49:58.805Z\"\n}\n");
    assert_eq!(copilot_sign_in(&dir), None);
    // Signed in: its last user, else the first of its list.
    write("// managed\n{\"loggedInUsers\":[{\"host\":\"https://github.com\",\"login\":\"un\"},{\"host\":\"https://github.com\",\"login\":\"deux\"}],\"lastLoggedInUser\":{\"host\":\"https://github.com\",\"login\":\"deux\"}}");
    assert_eq!(copilot_sign_in(&dir).as_deref(), Some("deux"));
    write(r#"{"loggedInUsers":[{"host":"https://github.com","login":"un"}]}"#);
    assert_eq!(copilot_sign_in(&dir).as_deref(), Some("un"));
    // Without a keychain, its token is in the file too: never read as the user.
    write(
        r#"{"logged_in_users":["trois"],"copilot_tokens":{"https://github.com:trois":"gho_secret"}}"#,
    );
    assert_eq!(copilot_sign_in(&dir).as_deref(), Some("trois"));
    write(r#"{"loggedInUsers":[]}"#);
    assert_eq!(copilot_sign_in(&dir), None);
}

#[tokio::test]
async fn a_copilot_account_is_added_then_signed_in_by_copilot_login_in_a_terminal_of_its_folder() {
    use crate::accounts::ShareMode;
    let h = harness("esc40-copilot-login");
    let work = h
        .core
        .create_account(EngineId::Copilot, "Travail", &[], ShareMode::Link)
        .unwrap();
    let dir = PathBuf::from(&work.config_dir);
    assert_eq!(
        dir,
        std::path::absolute(h.core.data.accounts_of(EngineId::Copilot).join("travail")).unwrap()
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
    h.core
        .update_claude_account(Account {
            claude_path: crate::engine::fake_copilot(),
            ..work
        })
        .unwrap();
    let status = h.core.claude_account_status("travail").await.unwrap();
    assert!(!status.connected);
    let out = std::sync::Arc::new(parking_lot::Mutex::new(String::new()));
    let sink = out.clone();
    let info = h
        .core
        .claude_login("travail", (200, 30), move |b| {
            sink.lock().push_str(&String::from_utf8_lossy(&b))
        })
        .unwrap();
    assert_eq!(
        (info.name.as_str(), info.shell.as_str()),
        ("Travail", "copilot cli")
    );
    let start = std::time::Instant::now();
    let mut answered = false;
    while !dir.join("config.json").is_file() {
        // The console asks where its cursor is (ConPTY) before it shows anything.
        if !answered && out.lock().contains("[6n") {
            h.core.pty.write(&info.id, b"[1;1R").unwrap();
            answered = true;
        }
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "no sign-in: {:?}",
            out.lock()
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    // `copilot login`, in the account's folder: its user, never a token. Once it is signed in, the
    // app reads the account's models (`--server`), which may be noted after it: not the last one.
    let launches = noted(&dir, "launches.jsonl");
    assert!(
        launches.iter().any(|l| argv(l) == ["login"]),
        "no `copilot login` among {launches:?}"
    );
    let status = h.core.claude_account_status("travail").await.unwrap();
    assert!(status.connected);
    assert_eq!(status.email.as_deref(), Some("@copilot-user"));
    assert_eq!(status.stamp.as_deref(), Some("copilot-user"));
    h.core.stop_for_good();
}

#[tokio::test]
async fn the_credits_of_a_copilot_account_without_an_agent_are_read_by_a_server_of_its_own() {
    let h = harness("esc40-quota-apart");
    let work = copilot_accounts(&h, json!({ "credits": 25 }));
    h.core.refresh_usage().await;
    let usage = h.core.usage.lock().clone();
    let read = usage.account("work").unwrap();
    assert_eq!(read.meters.len(), 1, "{:?}", read.meters);
    // The month's credits, 75 % used; `account.getQuota` tells the time of its reading as their
    // end: the first of next month instead.
    assert_eq!((read.meters[0].minutes, read.meters[0].pct), (None, 75.0));
    assert_eq!(read.meters[0].resets_at, Some(next_month()));
    assert!(read.connected);
    // A server started for that and stopped: no session, Escouade's server kept out.
    let started = noted(&work, "launches.jsonl");
    assert_eq!(started.len(), 1);
    let a = argv(&started[0]);
    assert!(a.contains(&"--server".to_string()), "{a:?}");
    assert!(a.contains(&"--disable-mcp-server".to_string()), "{a:?}");
    let calls: Vec<String> = noted(&work, "calls.jsonl")
        .iter()
        .map(|c| c["method"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(calls, ["connect", "auth.getStatus", "account.getQuota"]);
    // Read again within five minutes: not asked again.
    h.core.refresh_usage().await;
    assert_eq!(noted(&work, "launches.jsonl").len(), 1);
    h.core.stop_for_good();
}

#[tokio::test]
async fn an_account_of_its_own_on_the_github_clis_sign_in_is_not_signed_in() {
    let h = harness("esc40-gh-cli");
    let (p, _) = h.project(false).await;
    // Copilot took the GitHub CLI's account by itself: the same for every account.
    copilot_accounts(&h, json!({ "auth": "gh-cli" }));
    h.core.refresh_usage().await;
    assert!(!h.core.usage.lock().account("work").unwrap().connected);
    let id = copilot_agent(&h, &p).await;
    let e = h
        .core
        .send_message(&id, "bonjour".into(), vec![])
        .await
        .unwrap_err();
    assert!(format!("{e:#}").contains("n'a pas pu démarrer"), "{e:#}");
    // Why, in its conversation: its process stopped on it.
    h.wait("its reason", |h| {
        h.items(&id).iter().any(|i| {
            i.to_string().contains("copilot login") && i.to_string().contains("Réglages › Comptes")
        })
    })
    .await;
    // Nothing was said on that sign-in.
    assert!(!h
        .items(&id)
        .iter()
        .any(|i| i["kind"] == "text" && i["text"].as_str().unwrap_or("").contains("a reçu")));
    h.core.stop_for_good();
}

#[tokio::test]
async fn a_copilot_turn_tells_its_accounts_credits_and_their_end_stops_it_until_they_come_back() {
    let h = harness("esc40-credits");
    let (p, _) = h.project(false).await;
    copilot_accounts(&h, json!({ "credits": 3 }));
    let id = copilot_agent(&h, &p).await;
    h.turn(&id, "bonjour").await;
    // Each call says the month's credits: 97 % used, over « Pause au-delà du quota » (95 %).
    let usage = h.core.usage.lock().clone();
    let read = usage.account("work").unwrap();
    assert_eq!((read.meters[0].minutes, read.meters[0].pct), (None, 97.0));
    assert!(crate::accounts::over_threshold(
        &usage.accounts,
        "work",
        95,
        now_ms()
    ));
    let pause =
        crate::board::account_pause(&usage.accounts, "work", 95, &Default::default(), now_ms())
            .expect("the autopilot paused");
    assert_eq!(pause.reason, PauseReason::Credits);
    // A turn stopped by its credits spent: limited, its resume planned when they are given again.
    h.turn(&id, "limite").await;
    let turn = h
        .items(&id)
        .into_iter()
        .rfind(|i| i["kind"] == "turn")
        .unwrap();
    assert_eq!(turn["limited"], true);
    let at = h.agent(&id).resume_at.expect("its resume planned");
    assert!(at >= next_month(), "after the reset: {at}");
    // Its cost: the nano-AIU of its calls (0.227 credit, $0.00227 a turn).
    let cost = h.agent(&id).cost;
    assert!((cost - 2.0 * 227_227_500.0 / 1e11).abs() < 1e-9, "{cost}");
    // The statistics keep them as Copilot's, and narrow to it.
    let all = h.core.stats_view_by("day", None, None);
    assert!(
        all.engines.contains(&"copilot".to_string()),
        "{:?}",
        all.engines
    );
    let copilot = h.core.stats_view_by("day", None, Some("copilot"));
    assert!((copilot.cost - cost).abs() < 1e-9, "{}", copilot.cost);
    assert_eq!(copilot.tokens, 2 * (200 + 1000 + 40));
    let claude = h.core.stats_view_by("day", None, Some("claude"));
    assert_eq!(claude.tokens, 0);
    h.core.stop_for_good();
}

#[tokio::test]
async fn the_sessions_ceiling_of_credits_is_answered_from_its_card() {
    let h = harness("esc40-ceiling");
    let (p, _) = h.project(false).await;
    copilot_accounts(&h, json!({}));
    let id = copilot_agent(&h, &p).await;
    h.core
        .send_message(&id, "plafond".into(), vec![])
        .await
        .unwrap();
    h.wait("the card", |h| waiting(h, &id, "question").is_some())
        .await;
    let card = waiting(&h, &id, "question").unwrap();
    let q = &card["questions"][0];
    let text = q["question"].as_str().unwrap().to_string();
    let lift = q["options"][1]["label"].as_str().unwrap().to_string();
    h.core
        .answer_question(&id, card["id"].as_str().unwrap(), json!({ text: lift }))
        .unwrap();
    h.wait("the turn's end", |h| {
        texts(h, &id).iter().any(|t| t == "Plafond : unset.")
    })
    .await;
    h.core.stop_for_good();
}

#[tokio::test]
async fn the_texts_written_by_copilot_go_through_copilot_p_without_tools_on_the_account_of_the_agent(
) {
    let h = harness("esc40-writer");
    let (p, _) = h.project(false).await;
    let work = copilot_accounts(&h, json!({}));
    h.set_settings(|s| s.writer_engine = Some(EngineId::Copilot));
    let id = copilot_agent(&h, &p).await;
    let answer = h
        .core
        .one_shot("Nomme la tâche.", "ajouter un bouton", Some(&id), None)
        .await
        .unwrap();
    assert_eq!(answer, "Copilot (gpt-5-mini) a rédigé : ajouter un bouton");
    let asked = noted(&work, "prompts.jsonl");
    assert_eq!(asked.len(), 1);
    let a = argv(&asked[0]);
    for x in [
        "--output-format",
        "json",
        "--disable-builtin-mcps",
        "--no-custom-instructions",
        "--available-tools=escouade-aucun-outil",
    ] {
        assert!(a.iter().any(|y| y == x), "{x}: {a:?}");
    }
    // The question never on the command line: from the input, its instructions first.
    assert!(!a.iter().any(|y| y.contains("ajouter un bouton")), "{a:?}");
    let question = asked[0]["question"].as_str().unwrap();
    assert!(
        question.starts_with("<instructions>\nNomme la tâche."),
        "{question}"
    );
    // Its session is gone from the account's folder.
    let session = a
        .iter()
        .find_map(|y| y.strip_prefix("--session-id="))
        .unwrap();
    assert!(!work.join("session-state").join(session).exists());
    // A Free account takes no model but `auto`: asked again on it.
    std::fs::write(work.join("fake.json"), r#"{"free":true}"#).unwrap();
    let answer = h
        .core
        .one_shot("Nomme.", "autre chose", Some(&id), None)
        .await
        .unwrap();
    assert_eq!(answer, "Copilot (auto) a rédigé : autre chose");
    h.core.stop_for_good();
}

#[tokio::test]
async fn an_agent_on_auto_goes_without_an_effort_which_copilot_refuses_with_it() {
    // ESC-52, tried for real: a signed-in Principal's default model is « auto », and the ticket's
    // agent went with the app's effort: Copilot CLI refused its session (« Reasoning effort is not
    // supported when using the "auto" model »), and the ticket never began.
    let h = harness("esc52-copilot-auto");
    let (p, _) = h.project(false).await;
    let home = copilot_accounts(&h, json!({ "autoModel": true }));
    // A first agent: its models are known from then on, « auto » first.
    let first = copilot_agent(&h, &p).await;
    h.turn(&first, "bonjour").await;
    // Then one on a model of Claude Code's (a ticket of a Kanban on Sonnet) goes on Copilot's first.
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::Copilot),
                account: Some("work".into()),
                model: Some("sonnet".into()),
                effort: Some("medium".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    assert_eq!(h.agent(&id).model, "auto");
    h.turn(&id, "bonjour").await;
    let calls = noted(&home, "calls.jsonl");
    let create = calls
        .iter()
        .rfind(|c| c["method"] == "session.create")
        .unwrap_or_else(|| panic!("{calls:?}"));
    assert_eq!(create["params"]["model"], "auto", "{create}");
    assert!(
        create["params"].get("reasoningEffort").is_none(),
        "{create}"
    );
    assert!(
        texts(&h, &id).iter().any(|t| t.contains("a reçu")),
        "{:?}",
        h.items(&id)
    );
    // On another model, its effort goes with it again.
    assert!(!crate::engine::by_id(EngineId::Copilot).supports_effort("auto"));
    assert!(crate::engine::by_id(EngineId::Copilot).supports_effort("gpt-6-luna"));
    h.core.stop_for_good();
}

#[tokio::test]
async fn a_session_copilot_does_not_open_says_why_in_the_conversation() {
    // ESC-52, tried for real: `session.create` refused, the message failed with « voir le détail
    // dans la conversation », and the conversation said nothing.
    let h = harness("esc52-copilot-refused");
    let (p, _) = h.project(false).await;
    copilot_accounts(
        &h,
        json!({ "sessionError": "Request session.create failed with message: nope." }),
    );
    let id = copilot_agent(&h, &p).await;
    assert!(h
        .core
        .send_message(&id, "bonjour".into(), vec![])
        .await
        .is_err());
    h.wait("its reason", |h| {
        h.items(&id).iter().any(|i| {
            let i = i.to_string();
            i.contains("n’a pas ouvert la session") && i.contains("nope.")
        })
    })
    .await;
    h.core.stop_for_good();
}

/// ESC-156 with Copilot CLI: an agent of Copilot, named by Copilot (« Modèle des textes rédigés »),
/// gets its worktree at its first message under that name, and its session takes it.
#[tokio::test]
async fn a_copilot_agent_gets_its_worktree_at_its_first_message_under_the_name_copilot_gives() {
    let h = harness("esc156-copilot");
    let (p, _) = h.project_as_new(true).await;
    let work = copilot_accounts(&h, json!({}));
    h.set_settings(|s| s.writer_engine = Some(EngineId::Copilot));
    let id = copilot_agent(&h, &p).await;
    assert!(h.agent(&id).worktree_pending);
    h.turn(&id, "Ajoute un bouton").await;
    let m = h.agent(&id);
    assert_eq!(m.name, "ajoute-un-copilot");
    let wt = m.worktree.unwrap();
    assert_eq!(
        PathBuf::from(&wt.path),
        p.worktrees_root().join("ajoute-un-copilot")
    );
    assert_eq!(wt.branch, "escouade/ajoute-un-copilot");
    h.wait("its session renamed", |_| {
        noted(&work, "calls.jsonl").iter().any(|c| {
            c["method"] == "session.name.set" && c["params"]["name"] == "ajoute-un-copilot"
        })
    })
    .await;
    h.core.stop_for_good();
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
async fn the_models_of_a_copilot_account_are_read_without_an_agent_and_kept_after_a_restart() {
    let h = harness("esc186-copilot-models");
    let work = copilot_accounts(&h, json!({}));
    assert!(menu_of(&h.core, "work").is_empty());
    h.core.read_account_models("work").await;
    // Those of `models.list`, with what the menu says of them.
    assert_eq!(
        menu_of(&h.core, "work"),
        ["gpt-6-luna", "claude-sonnet-4.5", "gpt-5-mini"]
    );
    let told = h.core.account_models.read()["work"].clone();
    let sonnet = told
        .catalog
        .iter()
        .find(|m| m.id == "claude-sonnet-4.5")
        .unwrap();
    assert_eq!(sonnet.multiplier.as_deref(), Some("3"));
    let luna = &told.catalog[0];
    assert!(luna.default && luna.images);
    assert_eq!(luna.efforts, ["low", "medium", "high"]);
    // A server of its own, stopped: no session, nothing spent.
    assert_eq!(noted(&work, "launches.jsonl").len(), 1);
    let calls: Vec<String> = noted(&work, "calls.jsonl")
        .iter()
        .map(|c| c["method"].as_str().unwrap().to_string())
        .collect();
    assert!(calls.contains(&"models.list".to_string()), "{calls:?}");
    assert!(
        !calls.iter().any(|c| c.starts_with("session.")),
        "{calls:?}"
    );
    // Opening the menu within a day reads nothing again.
    h.core.refresh_account_models("work").await;
    assert_eq!(noted(&work, "launches.jsonl").len(), 1);
    // Kept by account through a restart.
    h.core.save_now();
    let app = tauri::test::mock_app();
    let (reloaded, _rx) = crate::core::Core::load(app.handle().clone(), h.core.data.clone());
    assert_eq!(
        menu_of(&reloaded, "work"),
        ["gpt-6-luna", "claude-sonnet-4.5", "gpt-5-mini"]
    );
    h.core.stop_for_good();
}

#[tokio::test]
async fn a_free_copilot_account_keeps_the_models_it_has_today_never_auto() {
    let h = harness("esc186-copilot-free");
    copilot_accounts(&h, json!({ "free": true }));
    h.core.read_account_models("work").await;
    // `models.list` gives none on Free (ESC-122): its declared models, as an agent of it has them.
    assert_eq!(menu_of(&h.core, "work"), ["gpt-6-luna", "gpt-5-mini"]);
    h.core.stop_for_good();
}

/// ESC-186, by hand: the models of the real `copilot` of the machine (`ESCOUADE_TEST_REAL_COPILOT`,
/// its program), read as Escouade reads them without an agent (`models.list` of a server of its
/// own, its usual `~/.copilot`). Its files are to be checked byte for byte after.
#[tokio::test]
#[ignore = "the real copilot: run by hand"]
async fn real_copilot_models() {
    use crate::engine::Engine;
    let program = std::env::var("ESCOUADE_TEST_REAL_COPILOT").expect("ESCOUADE_TEST_REAL_COPILOT");
    let program = crate::engine::Program::Native(program.into());
    let started = std::time::Instant::now();
    let listed = crate::engine::COPILOT
        .list_models(program, Vec::new(), &std::env::temp_dir())
        .await
        .unwrap()
        .expect("models listed");
    let ids: Vec<String> = listed
        .iter()
        .map(|m| format!("{} ×{}", m.id, m.multiplier.as_deref().unwrap_or("?")))
        .collect();
    eprintln!(
        "{} models in {} ms: {ids:?}",
        ids.len(),
        started.elapsed().as_millis()
    );
    assert!(!listed.is_empty());
}

/// ESC-205: `/compact` typed to an agent of Copilot is no message to its model. Copilot compacts
/// its session (`session.history.compact`, the instruction after the command given as what to
/// keep), the conversation says when it starts and when it ends, the context gauge goes down, and
/// the agent is at rest again once Copilot answered.
#[tokio::test]
async fn compact_typed_to_a_copilot_agent_compacts_its_session_with_its_instruction() {
    let h = harness("esc205-copilot");
    let (p, _) = h.project(false).await;
    let work = copilot_accounts(&h, json!({}));
    let id = copilot_agent(&h, &p).await;
    h.turn(&id, "bonjour").await;
    let context = |h: &Harness| h.core.agent(&id).unwrap().lock().context_tokens;
    let before = context(&h);
    assert!(before > 0);
    h.core
        .send_message(&id, "/compact garde les décisions".into(), vec![])
        .await
        .unwrap();
    h.wait("the compaction over", |h| {
        !h.agent(&id).status.is_active()
            && h.items(&id)
                .iter()
                .any(|i| i["text"] == "Contexte compacté")
    })
    .await;
    let calls = noted(&work, "calls.jsonl");
    let compact: Vec<&Value> = calls
        .iter()
        .filter(|c| c["method"] == "session.history.compact")
        .collect();
    assert_eq!(compact.len(), 1, "{calls:?}");
    assert_eq!(
        compact[0]["params"]["customInstructions"],
        "garde les décisions"
    );
    let sends = calls
        .iter()
        .filter(|c| c["method"] == "session.send")
        .count();
    assert_eq!(sends, 1, "the model was told /compact");
    let shown: Vec<String> = h
        .items(&id)
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
    assert_eq!(
        shown[shown.len() - 3..],
        [
            "user: /compact garde les décisions",
            "notice: Compaction du contexte…",
            "notice: Contexte compacté",
        ],
        "{shown:?}"
    );
    assert!(context(&h) < before, "{before} → {}", context(&h));
    h.core.stop_for_good();
}
